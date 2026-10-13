//! Inherit accepted body colors on independently tessellated attached surfaces.
use crate::{ComputeError, SceneGeometry};
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SurfaceColorBinding {
    pub sources: [u32; 3],
    pub target: u32,
    pub weights: [f32; 3],
    pub padding: f32,
}
#[derive(Debug)]
pub struct SurfaceColorTransfer {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    groups: u32,
}
impl SurfaceColorTransfer {
    /// Sources lie in a read-only prefix; unique targets lie outside it. This
    /// excludes intra-dispatch dependencies and write races by construction.
    pub async fn new(
        device: &wgpu::Device,
        geometry: &SceneGeometry,
        source_count: u32,
        mapping: &[SurfaceColorBinding],
    ) -> Result<Self, ComputeError> {
        if !geometry.belongs_to(device) {
            return Err(ComputeError::DeviceMismatch);
        }
        let limits = device.limits();
        let mut seen = std::collections::HashSet::new();
        let target = geometry.deformation_vertices();
        if source_count == 0
            || source_count as usize > geometry.deformation_vertex_capacity()
            || mapping.is_empty()
            || mapping.len() > u32::MAX as usize
            || mapping.len().div_ceil(64) > limits.max_compute_workgroups_per_dimension as usize
            || limits.max_storage_buffers_per_shader_stage < 2
            || limits.max_compute_workgroup_size_x < 64
            || limits.max_compute_invocations_per_workgroup < 64
            || !target.usage().contains(wgpu::BufferUsages::STORAGE)
            || [target.size(), std::mem::size_of_val(mapping) as u64]
                .iter()
                .any(|s| {
                    *s > limits.max_buffer_size
                        || *s > u64::from(limits.max_storage_buffer_binding_size)
                })
            || mapping.iter().any(|m| {
                m.target < source_count
                    || m.target as usize >= geometry.deformation_vertex_capacity()
                    || m.target > (u32::MAX - 8) / 9
                    || m.sources
                        .iter()
                        .any(|i| *i >= source_count || *i > (u32::MAX - 8) / 9)
                    || !seen.insert(m.target)
                    || m.weights.iter().any(|w| !w.is_finite() || *w < 0.)
                    || (m.weights.iter().sum::<f32>() - 1.).abs() > 1e-5
            })
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bindings = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface inherited color bindings"),
            contents: bytemuck::cast_slice(mapping),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface inherited colors"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface_color_transfer.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("surface inherited colors"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("surface inherited color streams"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bindings.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: target.as_entire_binding(),
                },
            ],
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
    /// Encode after body illumination transfer and before drawing.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(self.groups, 1, 1);
    }
}
