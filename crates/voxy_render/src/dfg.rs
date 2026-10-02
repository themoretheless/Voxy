//! Single-scattering split-sum GGX BRDF integration lookup.
//! Coordinates are (NdotV, perceptual roughness); RG stores A,B for F0*A+B.
use crate::RendererError;
use wgpu::util::DeviceExt;
#[derive(Debug)]
pub struct GgxDfgLut {
    device: wgpu::Device,
    output: wgpu::Texture,
    view: wgpu::TextureView,
    passes: Vec<(wgpu::TextureView, wgpu::BindGroup)>,
    pipeline: wgpu::RenderPipeline,
}
impl GgxDfgLut {
    /// Allocate a square RGBA16Float LUT. Default integration budget is 256.
    /// Encode once before sampling; no submission or resource allocation in encode.
    /// # Errors
    /// Rejects zero/excessive dimensions.
    pub fn new(device: &wgpu::Device, size: u32) -> Result<Self, RendererError> {
        Self::with_samples(device, size, 256)
    }
    /// Set a fixed quality/work budget of 1..=4096 importance samples per output texel.
    /// Resources and the selected sample count remain immutable across encode calls.
    /// # Errors
    /// Rejects invalid sample counts before allocating, and invalid cube dimensions.
    pub fn with_samples(
        device: &wgpu::Device,
        size: u32,
        samples: u32,
    ) -> Result<Self, RendererError> {
        if !(1..=4096).contains(&samples) {
            return Err(RendererError::InvalidPrefilterSamples(samples));
        }
        let max_dimension = device.limits().max_texture_dimension_2d;
        if size == 0 || size > max_dimension {
            return Err(RendererError::InvalidSurfaceSize {
                width: size,
                height: size,
                max_dimension,
            });
        }
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GGX DFG LUT"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = output.create_view(&Default::default());
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GGX environment layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            }],
        });
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("DFG parameters"),
            contents: bytemuck::cast_slice(&[size, samples, 0_u32, 0]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("retained DFG integration"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: params.as_entire_binding(),
            }],
        });
        let passes = vec![(view.clone(), group)];
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GGX DFG integration"),
            source: wgpu::ShaderSource::Wgsl(include_str!("dfg.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GGX DFG pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GGX DFG prefilter"),
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
            device: device.clone(),
            output,
            view,
            passes,
            pipeline,
        })
    }
    pub(crate) fn belongs_to(&self, device: &wgpu::Device) -> bool {
        &self.device == device
    }
    #[must_use]
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
    /// Integrate the LUT into its retained target. Encoder ownership is validated by wgpu.
    /// This table uses height-correlated Smith visibility and Schlick Fresnel.
    /// Multiple-scattering compensation and local visibility are separate.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        for (view, group) in &self.passes {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GGX DFG integration"),
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
