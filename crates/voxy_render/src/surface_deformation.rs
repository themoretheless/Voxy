//! Fixed-topology displacement transfer into the exact buffer used for drawing.
use crate::{SceneError, SceneGeometry, SceneMesh};
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SurfaceDeformationWeight {
    pub control: u32,
    pub weight: f32,
}
#[derive(Debug)]
pub struct SurfaceDeformation {
    device: wgpu::Device,
    topology: Vec<u32>,
    geometry: SceneGeometry,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    controls: crate::ComputeStorage,
    _inputs: Vec<crate::ComputeStorage>,
    count: u32,
    control_count: u32,
    maximum_row_weight: f64,
    maximum_rest_position: f64,
    initialized: bool,
    normals: crate::surface_normal_transport::SurfaceNormalTransport,
    rigid_skin: Option<crate::surface_rigid_skinning::SurfaceRigidSkinning>,
}
pub(crate) fn validate(
    device: &wgpu::Device,
    mesh: &SceneMesh,
    offsets: &[u32],
    weights: &[SurfaceDeformationWeight],
    control_count: u32,
) -> Result<(), SceneError> {
    let count = mesh.vertices().len();
    let limits = device.limits();
    if count == 0
        || count > u32::MAX as usize
        || control_count == 0
        || offsets.len() != count + 1
        || offsets[0] != 0
        || offsets[count] as usize != weights.len()
        || offsets.windows(2).any(|p| p[0] > p[1])
        || weights
            .iter()
            .any(|w| w.control >= control_count || !w.weight.is_finite())
        || limits.max_storage_buffers_per_shader_stage < 5
        || limits.max_compute_invocations_per_workgroup < 64
        || limits.max_compute_workgroup_size_x < 64
        || count.div_ceil(64) > limits.max_compute_workgroups_per_dimension as usize
    {
        return Err(SceneError::InvalidGeometry);
    }
    let sizes = [
        count as u64 * 36,
        offsets.len() as u64 * 4,
        (weights.len() as u64 * 8).max(8),
        u64::from(control_count) * 16,
    ];
    if sizes
        .iter()
        .any(|s| *s > limits.max_storage_buffer_binding_size || *s > limits.max_buffer_size)
    {
        return Err(SceneError::GeometryCapacityExceeded);
    }
    Ok(())
}
impl SurfaceDeformation {
    pub(crate) fn new(
        device: &wgpu::Device,
        mesh: &SceneMesh,
        geometry: SceneGeometry,
        offsets: &[u32],
        weights: &[SurfaceDeformationWeight],
        control_count: u32,
    ) -> Result<Self, SceneError> {
        use voxy_gpu_core::ManagedBufferDescriptor;
        let count = mesh.vertices().len() as u32;
        let parameters = [count, control_count, 0, 0];
        let padding = [SurfaceDeformationWeight {
            control: 0,
            weight: 0.,
        }];
        let data = [
            bytemuck::cast_slice(&parameters),
            bytemuck::cast_slice(mesh.vertices()),
            bytemuck::cast_slice(offsets),
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
                label: "surface deformation immutable input",
                size: data.len() as u64,
                contents: Some(data),
                usage: if i == 0 {
                    wgpu::BufferUsages::UNIFORM
                } else if i == 1 {
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
                } else {
                    wgpu::BufferUsages::STORAGE
                },
            })
            .collect();
        descriptors.push(ManagedBufferDescriptor {
            label: "surface displacement controls",
            size: u64::from(control_count) * 16,
            contents: None,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let mut inputs = crate::ComputeMemoryBudget::for_device(device)
            .allocate_buffer_batch(&descriptors)
            .map_err(|_| SceneError::MemoryBudget)?;
        let controls = inputs.pop().expect("admitted control buffer");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface displacement transfer"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface_deformation.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("surface displacement transfer"),
            layout: None,
            module: &shader,
            entry_point: Some("deform"),
            compilation_options: Default::default(),
            cache: None,
        });
        let mut entries: Vec<_> = inputs
            .iter()
            .enumerate()
            .map(|(binding, b)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: controls.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 5,
            resource: geometry.deformation_vertices().as_entire_binding(),
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("surface displacement bindings"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let deforming_limit = if mesh.vertices().len() > 100_000 {
            offsets
                .windows(2)
                .rposition(|row| row[1] - row[0] > 1)
                .map_or(mesh.vertices().len().min(100_000), |last| last + 1)
        } else {
            mesh.vertices().len()
        };
        let normals = crate::surface_normal_transport::SurfaceNormalTransport::new_with_limit(
            device,
            mesh,
            &geometry,
            deforming_limit,
        )?;
        let maximum_row_weight = offsets
            .windows(2)
            .map(|row| {
                weights[row[0] as usize..row[1] as usize]
                    .iter()
                    .map(|w| f64::from(w.weight).abs())
                    .sum::<f64>()
            })
            .fold(0_f64, f64::max);
        let maximum_rest_position = mesh
            .vertices()
            .iter()
            .flat_map(|v| v.position)
            .map(|v| f64::from(v).abs())
            .fold(0_f64, f64::max);
        Ok(Self {
            device: device.clone(),
            topology: mesh.indices().to_vec(),
            geometry,
            pipeline,
            group,
            controls,
            _inputs: inputs,
            count,
            control_count,
            maximum_row_weight,
            maximum_rest_position,
            initialized: false,
            normals,
            rigid_skin: None,
        })
    }
    /// Replace the unskinned source without rebuilding resident topology or
    /// bind-space normal data. Re-upload pose and controls before encoding.
    pub fn upload_source_mesh(
        &mut self,
        queue: &wgpu::Queue,
        source: &SceneMesh,
    ) -> Result<(), SceneError> {
        if source.vertices().len() != self.count as usize
            || source.indices() != self.topology.as_slice()
        {
            return Err(SceneError::InvalidGeometry);
        }
        let maximum = source
            .vertices()
            .iter()
            .flat_map(|v| v.position)
            .map(|v| f64::from(v).abs())
            .fold(0f64, f64::max);
        if !maximum.is_finite() || maximum > f32::MAX as f64 * 0.015625 {
            return Err(SceneError::InvalidGeometry);
        }
        queue.write_buffer(&self._inputs[1], 0, bytemuck::cast_slice(source.vertices()));
        self.maximum_rest_position = maximum;
        self.initialized = false;
        if let Some(skin) = &mut self.rigid_skin {
            skin.replace_source_bound(maximum);
        }
        Ok(())
    }
    /// Set the bind-space surface used to transport authored normals.
    /// Position skinning may start from a facially deformed source while normals
    /// must retain the original reference. Replacement is atomic on admission.
    pub fn set_normal_reference(
        &mut self,
        device: &wgpu::Device,
        reference: &SceneMesh,
    ) -> Result<(), SceneError> {
        if *device != self.device
            || reference.vertices().len() != self.count as usize
            || reference.indices() != self.topology.as_slice()
        {
            return Err(SceneError::InvalidGeometry);
        }
        let normals = crate::surface_normal_transport::SurfaceNormalTransport::new(
            device,
            reference,
            &self.geometry,
        )?;
        self.normals = normals;
        Ok(())
    }
    /// Skin immutable source positions on the GPU before adding physical
    /// displacements. A trailing rigid joint avoids per-fibre weight storage.
    /// Normal transport still follows the final displaced surface.
    pub fn enable_rigid_skinning(
        &mut self,
        device: &wgpu::Device,
        normals: &[[f32; 3]],
        weights: &[crate::SurfaceRigidSkinWeight],
        joints: u32,
        trailing_joint: Option<u32>,
    ) -> Result<(), SceneError> {
        if self.rigid_skin.is_some() || normals.len() != self.count as usize {
            return Err(SceneError::InvalidGeometry);
        }
        let skin = crate::surface_rigid_skinning::SurfaceRigidSkinning::new(
            device,
            &self._inputs[1],
            normals,
            &self.geometry,
            weights,
            joints,
            trailing_joint,
            self.maximum_rest_position,
        )?;
        let buffers: [&wgpu::Buffer; 6] = [
            &self._inputs[0],
            &skin.posed,
            &self._inputs[2],
            &self._inputs[3],
            &self.controls,
            self.geometry.deformation_vertices(),
        ];
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(binding, b)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skinned surface displacement"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        self.group = group;
        self.rigid_skin = Some(skin);
        Ok(())
    }
    /// Upload a validated rigid palette; vertex deformation stays resident.
    /// Upload a local bone palette followed by actor translation. All inputs
    /// are admitted before either GPU buffer is updated.
    /// Intermediate rigid positions, before physical-shell displacement.
    pub fn rigid_vertex_buffer(&self) -> Option<&wgpu::Buffer> {
        self.rigid_skin.as_ref().map(|skin| &*skin.posed)
    }
    pub fn upload_rigid_pose_with_offset(
        &self,
        queue: &wgpu::Queue,
        matrices: &[glam::Mat4],
        offset: glam::Vec3,
    ) -> Result<(), SceneError> {
        self.rigid_skin
            .as_ref()
            .ok_or(SceneError::InvalidTransform)?
            .upload_with_offset(queue, matrices, offset)
    }
    pub fn upload_rigid_pose(
        &self,
        queue: &wgpu::Queue,
        matrices: &[glam::Mat4],
    ) -> Result<(), SceneError> {
        self.rigid_skin
            .as_ref()
            .ok_or(SceneError::InvalidTransform)?
            .upload(queue, matrices)
    }
    /// Upload only simulation-control displacements, in metres.
    /// Positions are computed from the immutable rest mesh on every encode.
    pub fn update_controls(
        &mut self,
        queue: &wgpu::Queue,
        deltas: &[[f32; 4]],
    ) -> Result<(), SceneError> {
        self.write_controls(queue, deltas)?;
        self.initialized = true;
        Ok(())
    }
    /// Refresh initialized control storage inside an acquired-frame hook.
    pub fn upload_controls(
        &self,
        queue: &wgpu::Queue,
        deltas: &[[f32; 4]],
    ) -> Result<(), SceneError> {
        if !self.initialized {
            return Err(SceneError::InvalidGeometry);
        }
        self.write_controls(queue, deltas)
    }
    fn write_controls(&self, queue: &wgpu::Queue, deltas: &[[f32; 4]]) -> Result<(), SceneError> {
        if deltas.len() != self.control_count as usize
            || deltas.iter().flatten().any(|v| !v.is_finite())
        {
            return Err(SceneError::InvalidGeometry);
        }
        let maximum_displacement = deltas
            .iter()
            .flat_map(|d| &d[..3])
            .map(|v| f64::from(*v).abs())
            .fold(0_f64, f64::max);
        if self.maximum_rest_position + self.maximum_row_weight * maximum_displacement
            > f64::from(f32::MAX) * 0.5
        {
            return Err(SceneError::NonFiniteVertex);
        }
        queue.write_buffer(&self.controls, 0, bytemuck::cast_slice(deltas));
        Ok(())
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) -> Result<(), SceneError> {
        if !self.initialized {
            return Err(SceneError::InvalidGeometry);
        }
        if let Some(skin) = &self.rigid_skin {
            skin.encode(encoder)?;
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("surface displacement transfer"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(self.count.div_ceil(64), 1, 1);
        drop(pass);
        self.normals.encode(encoder);
        Ok(())
    }
    /// Optimize a fixed prefix of deforming triangles. Vertices after the prefix
    /// must only undergo rigid translation, or use the rigid skinning stage
    /// which rotates their authored normals before this transport pass.
    pub fn set_deforming_normal_prefix(&mut self, count: u32) -> Result<(), SceneError> {
        if count == 0 || count > self.count {
            return Err(SceneError::InvalidGeometry);
        }
        self.normals.set_prefix(count);
        Ok(())
    }
    /// The same resident vertex allocation is consumed by the scene renderer.
    /// Authored normals are transported through the posed triangles on the GPU.
    /// Baked material attributes remain unchanged.
    #[must_use]
    pub fn geometry(&self) -> &SceneGeometry {
        &self.geometry
    }
    /// Diagnostic readback source; production rendering requires no copy.
    #[must_use]
    pub fn normal_buffer(&self) -> &wgpu::Buffer {
        self.geometry.deformation_normals()
    }
    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        self.geometry.deformation_vertices()
    }
}
