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
        use crate::compute_memory::ManagedBufferDescriptor;
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
                    wgpu::BufferUsages::UNIFORM
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
            usage: wgpu::BufferUsages::STORAGE,
        });
        if descriptors.iter().any(|d| {
            d.size > limits.max_buffer_size
                || (d.usage.contains(wgpu::BufferUsages::STORAGE)
                    && d.size > u64::from(limits.max_storage_buffer_binding_size))
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
        Ok(Self {
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
    pub fn upload(&self, queue: &wgpu::Queue, matrices: &[Mat4]) -> Result<(), SceneError> {
        if matrices.len() != self.joints as usize {
            return Err(SceneError::InvalidTransform);
        }
        let mut values = Vec::with_capacity(matrices.len() * 2);
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
                || self.maximum_position * 4. + translation.abs().max_element() as f64
                    > f32::MAX as f64 * 0.25
            {
                return Err(SceneError::InvalidTransform);
            }
            let dual =
                Quat::from_xyzw(translation.x, translation.y, translation.z, 0.) * rotation * 0.5;
            values.push(rotation.to_array());
            values.push(dual.to_array());
        }
        queue.write_buffer(&self.palette, 0, bytemuck::cast_slice(&values));
        self.initialized
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) -> Result<(), SceneError> {
        if !self.initialized.load(std::sync::atomic::Ordering::Acquire) {
            return Err(SceneError::InvalidTransform);
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("rigid surface skinning"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(self.count.div_ceil(64), 1, 1);
        Ok(())
    }
}
