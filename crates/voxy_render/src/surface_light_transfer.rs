//! Transfer admitted illumination into the existing scene material color stream.
use crate::{ComputeError, SceneGeometry, SurfaceLightAdmission};
use wgpu::util::DeviceExt;

/// Immutable authored material and compact light-domain to draw-domain mapping.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SurfaceLightVertexBinding {
    pub albedo: [f32; 4],
    pub vertex: u32,
    pub light: u32,
    pub surface_fraction: f32,
    pub padding: u32,
}

#[derive(Debug)]
pub struct SurfaceLightTransfer {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    groups: u32,
}
impl SurfaceLightTransfer {
    /// Mapping is immutable and target vertices must be unique. Encode after
    /// admission and before drawing; no accepted result leaves authored colors intact.
    pub async fn new(
        device: &wgpu::Device,
        geometry: &SceneGeometry,
        admission: &SurfaceLightAdmission,
        mapping: &[SurfaceLightVertexBinding],
    ) -> Result<Self, ComputeError> {
        if !geometry.belongs_to(device) || !admission.belongs_to(device) {
            return Err(ComputeError::DeviceMismatch);
        }
        let limits = device.limits();
        let target = geometry.deformation_vertices();
        let mut seen = std::collections::HashSet::new();
        if mapping.is_empty()
            || mapping.len() > u32::MAX as usize
            || mapping.len().div_ceil(64) > limits.max_compute_workgroups_per_dimension as usize
            || limits.max_storage_buffers_per_shader_stage < 4
            || limits.max_compute_workgroup_size_x < 64
            || limits.max_compute_invocations_per_workgroup < 64
            || !target.usage().contains(wgpu::BufferUsages::STORAGE)
            || [
                std::mem::size_of_val(mapping) as u64,
                target.size(),
                admission.accepted_buffer().size(),
            ]
            .iter()
            .any(|s| {
                *s > limits.max_buffer_size
                    || *s > u64::from(limits.max_storage_buffer_binding_size)
            })
            || mapping.iter().any(|m| {
                m.vertex as usize >= geometry.deformation_vertex_capacity()
                    || m.vertex > (u32::MAX - 8) / 9
                    || m.light as usize >= admission.vertex_count()
                    || !seen.insert(m.vertex)
                    || !m.surface_fraction.is_finite()
                    || !(0.0..=1.0).contains(&m.surface_fraction)
                    || m.albedo.iter().any(|c| !c.is_finite() || *c < 0.0)
                    || m.albedo[3] > 1.0
            })
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bindings = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface material bindings"),
            contents: bytemuck::cast_slice(mapping),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let parameters = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface material transfer parameters"),
            contents: bytemuck::cast_slice(&[
                mapping.len() as u32,
                admission.vertex_count() as u32,
                0,
                0,
            ]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("accepted surface illumination transfer"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface_light_transfer.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("accepted surface illumination transfer"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let resources = [
            &bindings,
            admission.accepted_buffer(),
            admission.header_buffer(),
            target,
            &parameters,
        ];
        let entries: Vec<_> = resources
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("surface illumination draw streams"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        if let Some(error) = scope.pop().await {
            return Err(ComputeError::Validation(error.to_string()));
        }
        Ok(Self {
            pipeline,
            group,
            groups: (mapping.len() as u32).div_ceil(64),
        })
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(self.groups, 1, 1);
    }
}
