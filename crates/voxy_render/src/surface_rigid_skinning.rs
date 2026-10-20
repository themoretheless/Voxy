//! Optional rigid dual-quaternion stage before physical surface displacements.
use crate::{SceneError, SceneGeometry};
use glam::{Mat4, Quat};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SurfaceRigidSkinWeight {
    pub joints: [u32; 4],
    pub weights: [f32; 4],
}
#[derive(Debug)]
pub(crate) struct SurfaceRigidSkinning {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    pub posed: crate::ComputeStorage,
    palette: crate::ComputeStorage,
    _inputs: Vec<crate::ComputeStorage>,
    joints: u32,
    count: u32,
    maximum_position: f64,
    initialized: std::sync::atomic::AtomicBool,
    shared: Option<SharedRigidPalette>,
}
#[derive(Debug)]
struct RigidSnapshot {
    joints: Vec<(Quat, glam::Vec3)>,
    offset: glam::Vec3,
}
#[derive(Debug)]
struct SharedRigidPalette {
    ring: crate::SharedUploadRing,
    device: wgpu::Device,
    groups: Vec<wgpu::BindGroup>,
    parameters: [u32; 8],
    parameter_offset: usize,
    snapshot: std::sync::Mutex<Option<RigidSnapshot>>,
}
fn dual(rotation: Quat, translation: glam::Vec3) -> Quat {
    Quat::from_xyzw(translation.x, translation.y, translation.z, 0.) * rotation * 0.5
}
fn encode_skin(
    pipeline: &wgpu::ComputePipeline,
    group: &wgpu::BindGroup,
    count: u32,
    encoder: &mut wgpu::CommandEncoder,
) {
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("rigid surface skinning"),
        timestamp_writes: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
}
impl SurfaceRigidSkinning {
    pub fn new(
        device: &wgpu::Device,
        rest: &wgpu::Buffer,
        normals: &[[f32; 3]],
        geometry: &SceneGeometry,
        weights: &[SurfaceRigidSkinWeight],
        joints: u32,
        trailing: Option<u32>,
        maximum_position: f64,
    ) -> Result<Self, SceneError> {
        use voxy_gpu_core::ManagedBufferDescriptor;
        let count = normals.len();
        let limits = device.limits();
        if count == 0
            || count > u32::MAX as usize
            || weights.len() > count
            || joints == 0
            || trailing.is_some_and(|i| i >= joints)
            || limits.max_storage_buffers_per_shader_stage < 6
            || normals.iter().flatten().any(|v| !v.is_finite())
            || weights.iter().any(|w| {
                w.joints.iter().any(|i| *i >= joints)
                    || w.weights.iter().any(|v| !v.is_finite() || *v < 0.)
                    || (w.weights.iter().sum::<f32>() - 1.).abs() > 1e-5
            })
        {
            return Err(SceneError::InvalidGeometry);
        }
        let parameters = [
            count as u32,
            weights.len() as u32,
            joints,
            trailing.unwrap_or(u32::MAX),
            0,
            0,
            0,
            0,
        ];
        let padding = [SurfaceRigidSkinWeight {
            joints: [0; 4],
            weights: [1., 0., 0., 0.],
        }];
        let data = [
            bytemuck::cast_slice(&parameters),
            bytemuck::cast_slice(normals),
            bytemuck::cast_slice(if weights.is_empty() {
                &padding
            } else {
                weights
            }),
        ];
        let mut descriptors: Vec<_> = data
            .iter()
            .enumerate()
            .map(|(i, data)| ManagedBufferDescriptor {
                label: "rigid surface skin input",
                size: data.len() as u64,
                contents: Some(data),
                usage: if i == 0 {
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
                } else {
                    wgpu::BufferUsages::STORAGE
                },
            })
            .collect();
        descriptors.push(ManagedBufferDescriptor {
            label: "rigid surface palette",
            size: u64::from(joints) * 32,
            contents: None,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        descriptors.push(ManagedBufferDescriptor {
            label: "rigid posed surface",
            size: count as u64 * 36,
            contents: None,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        if descriptors.iter().any(|d| {
            d.size > limits.max_buffer_size
                || (d.usage.contains(wgpu::BufferUsages::STORAGE)
                    && d.size > limits.max_storage_buffer_binding_size)
        }) {
            return Err(SceneError::GeometryCapacityExceeded);
        }
        let mut inputs = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffer_batch(&descriptors)
            .map_err(|_| SceneError::MemoryBudget)?;
        let posed = inputs.pop().unwrap();
        let palette = inputs.pop().unwrap();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rigid surface skinning"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface_rigid_skinning.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("rigid surface skinning"),
            layout: None,
            module: &shader,
            entry_point: Some("skin"),
            compilation_options: Default::default(),
            cache: None,
        });
        let buffers: [&wgpu::Buffer; 7] = [
            &inputs[0],
            rest,
            &inputs[1],
            &inputs[2],
            &palette,
            &posed,
            geometry.deformation_normals(),
        ];
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rigid surface bindings"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let palette_bytes = u64::from(joints) * 32;
        let alignment = u64::from(limits.min_uniform_buffer_offset_alignment);
        let parameter_offset = palette_bytes.div_ceil(alignment) * alignment;
        let shared = crate::SharedUploadRing::new(
            device,
            parameter_offset + 32,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::UNIFORM,
        )
        .map(|ring| {
            let groups = ring
                .buffers()
                .map(|buffer| {
                    let mut entries = entries.clone();
                    entries[0].resource = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer,
                        offset: parameter_offset,
                        size: std::num::NonZeroU64::new(32),
                    });
                    entries[4].resource = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(palette_bytes),
                    });
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("shared rigid palette bindings"),
                        layout: &pipeline.get_bind_group_layout(0),
                        entries: &entries,
                    })
                })
                .collect();
            SharedRigidPalette {
                ring,
                device: device.clone(),
                groups,
                parameters,
                parameter_offset: parameter_offset as usize,
                snapshot: std::sync::Mutex::new(None),
            }
        });
        Ok(Self {
            shared,
            pipeline,
            group,
            posed,
            palette,
            _inputs: inputs,
            joints,
            count: count as u32,
            maximum_position,
            initialized: std::sync::atomic::AtomicBool::new(false),
        })
    }
    pub fn replace_source_bound(&mut self, maximum: f64) {
        self.maximum_position = maximum;
        self.initialized
            .store(false, std::sync::atomic::Ordering::Release);
    }
    pub fn upload(&self, queue: &wgpu::Queue, matrices: &[Mat4]) -> Result<(), SceneError> {
        self.upload_with_offset(queue, matrices, glam::Vec3::ZERO)
    }
    pub fn upload_with_offset(
        &self,
        queue: &wgpu::Queue,
        matrices: &[Mat4],
        offset: glam::Vec3,
    ) -> Result<(), SceneError> {
        if !offset.is_finite() || offset.abs().max_element() as f64 > f32::MAX as f64 * 0.125 {
            return Err(SceneError::InvalidTransform);
        }
        if matrices.len() != self.joints as usize {
            return Err(SceneError::InvalidTransform);
        }
        let mut validated = Vec::with_capacity(if self.shared.is_some() {
            matrices.len()
        } else {
            0
        });
        let mut values = Vec::with_capacity(if self.shared.is_none() {
            matrices.len() * 2
        } else {
            0
        });
        for matrix in matrices {
            if !matrix.is_finite() {
                return Err(SceneError::InvalidTransform);
            }
            let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
            if !scale.abs_diff_eq(glam::Vec3::ONE, 1e-4)
                || !rotation.is_finite()
                || !rotation.is_normalized()
                || !Mat4::from_rotation_translation(rotation, translation)
                    .abs_diff_eq(*matrix, 1e-4)
                || self.maximum_position * 4. + f64::from(translation.abs().max_element())
                    > f64::from(f32::MAX) * 0.25
            {
                return Err(SceneError::InvalidTransform);
            }
            // Match the native rigid palette constructor before forming the dual part.
            let rotation = rotation.normalize();
            if self.shared.is_some() {
                validated.push((rotation, translation));
            } else {
                values.push(rotation.to_array());
                values.push(dual(rotation, translation).to_array());
            }
        }
        if let Some(shared) = &self.shared {
            // Snapshot caller-owned transforms; the final dual-quaternion bytes
            // are produced only in the leased shader buffer during encode.
            *shared.snapshot.lock().expect("rigid snapshot poisoned") = Some(RigidSnapshot {
                joints: validated,
                offset,
            });
        } else {
            queue.write_buffer(&self.palette, 0, bytemuck::cast_slice(&values));
            queue.write_buffer(
                &self._inputs[0],
                16,
                bytemuck::cast_slice(&offset.extend(0.).to_array()),
            );
        }
        self.initialized
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) -> Result<(), SceneError> {
        if !self.initialized.load(std::sync::atomic::Ordering::Acquire) {
            return Err(SceneError::InvalidTransform);
        }
        if let Some(shared) = &self.shared {
            let snapshot = shared.snapshot.lock().expect("rigid snapshot poisoned");
            let snapshot = snapshot.as_ref().ok_or(SceneError::InvalidTransform)?;
            let used = shared
                .ring
                .encode(
                    encoder,
                    |bytes| shared.produce(snapshot, bytes),
                    |index, _, encoder| {
                        encode_skin(&self.pipeline, &shared.groups[index], self.count, encoder);
                    },
                )
                .map_err(|_| SceneError::InvalidTransform)?;
            if used {
                return Ok(());
            }
            // Overflow stages a unique immutable snapshot and copies it in this
            // encoder, before its dispatch. Queue writes into a reusable target
            // would corrupt multiple still-unsubmitted overflow snapshots.
            let staging = shared.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shared palette overflow staging"),
                size: shared.parameter_offset as u64 + 32,
                usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: true,
            });
            {
                let mut mapped = staging
                    .slice(..)
                    .get_mapped_range_mut()
                    .map_err(|_| SceneError::InvalidTransform)?;
                shared.produce(snapshot, mapped.slice(..));
            }
            staging.unmap();
            encoder.copy_buffer_to_buffer(
                &staging,
                0,
                &self.palette,
                0,
                u64::from(self.joints) * 32,
            );
            encoder.copy_buffer_to_buffer(
                &staging,
                shared.parameter_offset as u64,
                &self._inputs[0],
                0,
                32,
            );
        }
        encode_skin(&self.pipeline, &self.group, self.count, encoder);
        Ok(())
    }
}
impl SharedRigidPalette {
    fn produce(&self, snapshot: &RigidSnapshot, bytes: wgpu::WriteOnly<'_, [u8]>) {
        // Write generated shader values directly, with no packed Vec or
        // queue.write_buffer/staging/blit on this path.
        let (palette, parameters) = bytes.split_at(self.parameter_offset);
        let palette = palette
            .into_slice(..snapshot.joints.len() * 32)
            .into_chunks::<32>()
            .0;
        for (row, &(rotation, translation)) in palette.into_iter().zip(&snapshot.joints) {
            row.write(bytemuck::cast([
                rotation.to_array(),
                dual(rotation, translation).to_array(),
            ]));
        }
        let mut header = self.parameters;
        for (to, value) in header[4..]
            .iter_mut()
            .zip(snapshot.offset.extend(0.).to_array())
        {
            *to = value.to_bits();
        }
        parameters
            .into_chunks::<32>()
            .0
            .into_iter()
            .next()
            .unwrap()
            .write(bytemuck::cast(header));
    }
}

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "surface_rigid_shared_tests.rs"]
pub(crate) mod shared_tests;
