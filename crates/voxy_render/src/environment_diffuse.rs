//! Cosine-weighted HDR diffuse convolution. Output stores irradiance divided by pi.
use crate::RendererError;
use wgpu::util::DeviceExt;
#[derive(Debug)]
pub struct DiffuseEnvironmentConvolution {
    input: wgpu::Texture,
    device: wgpu::Device,
    output: wgpu::Texture,
    view: wgpu::TextureView,
    passes: Vec<(wgpu::TextureView, wgpu::BindGroup)>,
    pipeline: wgpu::RenderPipeline,
}
impl DiffuseEnvironmentConvolution {
    /// Allocate input/output six-face linear HDR cubes and retained diffuse pass bindings.
    /// Default budget is 256 cosine-weighted samples per output texel.
    /// Input order is +X,-X,+Y,-Y,+Z,-Z. Initialize all input faces before encode.
    /// Output has one level and stores E/pi; multiply by diffuse albedo without another pi factor.
    /// # Errors
    /// Rejects zero dimensions and dimensions exceeding device limits.
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
        let levels = 1_u32;
        let allocate = |label, mip_level_count| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 6,
                },
                mip_level_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        };
        let input = allocate("diffuse source environment", 1);
        let output = allocate("diffuse filtered environment", levels);
        let cube = |texture: &wgpu::Texture| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            })
        };
        let source = cube(&input);
        let view = cube(&output);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("diffuse environment sampler"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("diffuse environment layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
        });
        let mut passes = Vec::new();
        for level in 0..levels {
            for face in 0..6 {
                let roughness = 0.0_f32;
                let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("diffuse face/mip parameters"),
                    contents: bytemuck::cast_slice(&[
                        face,
                        (size >> level).max(1),
                        roughness.to_bits(),
                        samples,
                    ]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("retained diffuse cube pass"),
                    layout: &layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&source),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: params.as_entire_binding(),
                        },
                    ],
                });
                let target = output.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    base_array_layer: face,
                    array_layer_count: Some(1),
                    ..Default::default()
                });
                passes.push((target, group));
            }
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("diffuse environment convolution"),
            source: wgpu::ShaderSource::Wgsl(include_str!("environment_diffuse.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("diffuse cube pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("diffuse cube prefilter"),
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
            input,
            output,
            view,
            passes,
            pipeline,
        })
    }
    #[must_use]
    pub fn input(&self) -> &wgpu::Texture {
        &self.input
    }
    #[must_use]
    pub fn belongs_to(&self, device: &wgpu::Device) -> bool {
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
    /// Encode all six diffuse faces; resources are reused, no submission.
    /// Encoder must belong to the owning device (wgpu validation).
    /// Produces E/pi, excluding material albedo, Fresnel and local visibility.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        for (view, group) in &self.passes {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("diffuse environment face/mip"),
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
