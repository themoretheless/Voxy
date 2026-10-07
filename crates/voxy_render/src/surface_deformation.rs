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
    if sizes.iter().any(|s| {
        *s > u64::from(limits.max_storage_buffer_binding_size) || *s > limits.max_buffer_size
    }) {
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
        use crate::compute_memory::ManagedBufferDescriptor;
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
        let normals =
            crate::surface_normal_transport::SurfaceNormalTransport::new(device, mesh, &geometry)?;
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
        })
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
    /// must only undergo rigid translation, so their resident normals stay valid.
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
