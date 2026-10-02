//! Retained linear HDR mip pyramid; a prerequisite for rough reflection filtering.
use crate::RendererError;
#[derive(Debug)]
pub struct HdrMipPyramid {
    texture: wgpu::Texture,
    views: Vec<wgpu::TextureView>,
    groups: Vec<wgpu::BindGroup>,
    pipeline: wgpu::RenderPipeline,
}
impl HdrMipPyramid {
    /// Allocate RGBA16Float levels and retain each downsample pass binding.
    /// # Errors
    /// Rejects zero dimensions and dimensions exceeding device texture limits.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self, RendererError> {
        let max_dimension = device.limits().max_texture_dimension_2d;
        if width == 0 || height == 0 || width > max_dimension || height > max_dimension {
            return Err(RendererError::InvalidSurfaceSize {
                width,
                height,
                max_dimension,
            });
        }
        let levels = width.max(height).ilog2() + 1;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("linear HDR mip pyramid"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let views: Vec<_> = (0..levels)
            .map(|level| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HDR mip source"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let groups = views
            .iter()
            .take(views.len() - 1)
            .map(|view| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("retained HDR mip binding"),
                    layout: &layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    }],
                })
            })
            .collect();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HDR mip area average"),
            source: wgpu::ShaderSource::Wgsl(include_str!("hdr_mips.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("HDR mip pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HDR mip downsample"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            texture,
            views,
            groups,
            pipeline,
        })
    }
    #[must_use]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    /// Base-level render attachment; initialize it before encoding downsampling.
    #[must_use]
    pub fn base_view(&self) -> &wgpu::TextureView {
        &self.views[0]
    }
    /// Encode all dependent levels in order, without submission or allocation.
    /// Each destination averages its integer source rectangle, including odd edges.
    /// Source and encoder must belong to this pyramid's device (wgpu validates ownership).
    /// This is an area pyramid, not a GGX angular prefilter.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        for (group, view) in self.groups.iter().zip(self.views.iter().skip(1)) {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("linear HDR mip pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
