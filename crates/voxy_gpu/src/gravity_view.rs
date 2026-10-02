use crate::{GravityComputeError, GravityJob};
use voxy_render::ComputeError;

/// Draws committed resident bodies as small squares in clip space.
#[derive(Debug)]
pub struct GravityView {
    pipeline: wgpu::RenderPipeline,
    binding: wgpu::BindGroup,
    count: u32,
}
impl GravityView {
    /// Uses the job's original device; the view retains its buffer binding.
    /// # Errors
    /// Reports shader, binding or target format validation failure.
    pub async fn new(
        device: &wgpu::Device,
        job: &GravityJob,
        format: wgpu::TextureFormat,
    ) -> Result<Self, GravityComputeError> {
        Self::from_buffer(device, job.buffer(), job.body_count(), format).await
    }
    /// Binds render-only body storage produced by a graphics/compute integration.
    /// The caller supplies eight header u32 words followed by eight f32/u32 words
    /// per body (position xyz, mass, velocity xyz, padding). This does not import
    /// CUDA memory; platform code must supply a wgpu buffer on this device and
    /// complete external ownership transitions before encoding a draw.
    /// # Errors
    /// Rejects empty/overflowing body counts, missing storage usage, short buffers
    /// and shader/device/binding/format validation errors.
    pub async fn from_buffer(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        body_count: u32,
        format: wgpu::TextureFormat,
    ) -> Result<Self, GravityComputeError> {
        let words = body_count
            .checked_mul(8)
            .and_then(|words| words.checked_add(8))
            .ok_or(GravityComputeError::Budget)?;
        let bytes = u64::from(words) * 4;
        if body_count == 0
            || buffer.size() < bytes
            || !buffer.usage().contains(wgpu::BufferUsages::STORAGE)
        {
            return Err(GravityComputeError::InvalidInput);
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("resident gravity drawing"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gravity_view.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(bytes),
                },
                count: None,
            }],
        });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        if let Some(error) = scope.pop().await {
            return Err(GravityComputeError::Compute(ComputeError::Validation(
                error.to_string(),
            )));
        }
        Ok(Self {
            pipeline,
            binding,
            count: body_count,
        })
    }
    /// Records drawing after the job's encoded physics steps, on the same encoder.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("resident gravity view"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.binding, &[]);
        pass.draw(0..6, 0..self.count);
    }
}
