//! Update only the RR specular hit-distance guide after tracing primary surfaces.
use crate::{RayReconstructionGuides, ReconstructionGuideError};
#[derive(Debug)]
pub struct ReconstructionDistancePass {
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
    target: wgpu::TextureView,
}
impl ReconstructionDistancePass {
    /// Keep the normal/albedo/depth guides intact while replacing hit distance.
    /// Source must be distinct, single-sample `R32Float` on this device and describe
    /// the same primary surfaces. For R16 output, distances must not exceed 65504.
    /// Zero remains the miss convention. No shader distance is inferred from depth.
    /// # Errors
    /// Rejects aliasing, mismatched dimensions, format or resource usages.
    pub fn new(
        device: &wgpu::Device,
        guides: &RayReconstructionGuides,
        source: &wgpu::Texture,
    ) -> Result<Self, ReconstructionGuideError> {
        let target = guides.specular_hit_distance();
        if source.size() != target.size() {
            return Err(ReconstructionGuideError::InvalidDimensions);
        }
        if source == target
            || source.format() != wgpu::TextureFormat::R32Float
            || source.dimension() != wgpu::TextureDimension::D2
            || source.sample_count() != 1
            || source.depth_or_array_layers() != 1
            || !source
                .usage()
                .contains(wgpu::TextureUsages::TEXTURE_BINDING)
            || !target
                .usage()
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            return Err(ReconstructionGuideError::UnsupportedFormats);
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RR specular distance update"),
            source: wgpu::ShaderSource::Wgsl(include_str!("reconstruction_distance.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RR specular distance update"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target.format(),
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
        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("traced RR hit distances"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&source_view),
            }],
        });
        Ok(Self {
            pipeline,
            bindings,
            target: target.create_view(&wgpu::TextureViewDescriptor::default()),
        })
    }
    /// Encode after ray tracing and before native RR preparation; no depth writes.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RR specular guide update"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.draw(0..3, 0..1);
    }
}
