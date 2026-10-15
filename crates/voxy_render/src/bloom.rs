//! Dual Kawase bloom mip chain for the archive display pass.
use crate::{ComputeError, ComputeMemoryBudget};
use std::sync::Arc;

const LEVELS: usize = 6;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[derive(Debug)]
pub(crate) struct Bloom {
    levels: Vec<Arc<voxy_gpu_core::ManagedTexture>>,
    params: Vec<wgpu::Buffer>,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
}

impl Bloom {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self, ComputeError> {
        let mut sizes = Vec::new();
        let (mut w, mut h) = (width, height);
        for _ in 0..LEVELS {
            w = (w / 2).max(1);
            h = (h / 2).max(1);
            sizes.push((w, h));
        }
        let budget = ComputeMemoryBudget::for_device(device);
        let levels = sizes
            .iter()
            .map(|&(w, h)| {
                budget.allocate_texture(&wgpu::TextureDescriptor {
                    label: Some("bloom level"),
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: FORMAT,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        // One constant uniform per pass: texel size of the level it reads.
        // Downsample i reads level i-1 (or the source); upsample i reads i+1.
        let uniform = |size: (u32, u32), first: bool| {
            use wgpu::util::DeviceExt;
            let values = [1. / size.0 as f32, 1. / size.1 as f32, f32::from(u8::from(first)), 0.];
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bloom pass parameters"),
                contents: bytemuck::cast_slice(&values),
                usage: wgpu::BufferUsages::UNIFORM,
            })
        };
        let mut params = vec![uniform((width, height), true)];
        params.extend((1..LEVELS).map(|i| uniform(sizes[i - 1], false)));
        params.extend((0..LEVELS).map(|i| uniform(sizes[(i + 1).min(LEVELS - 1)], false)));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
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
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom bilinear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dual Kawase bloom"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bloom.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry: &str, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FORMAT,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::REPLACE,
        };
        Ok(Self {
            down: pipeline("fs_down", None),
            up: pipeline("fs_up", Some(additive)),
            levels,
            params,
            layout,
            sampler,
        })
    }

    /// The half-resolution result: the sum of all levels, `LEVELS` times the
    /// mean radiance, so readers divide by `levels()`.
    pub fn output(&self) -> wgpu::TextureView {
        self.levels[0].create_view(&Default::default())
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn levels() -> f32 {
        LEVELS as f32
    }

    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
    ) {
        let views: Vec<_> = self.levels.iter().map(|t| t.create_view(&Default::default())).collect();
        let mut pass = |pass_index: usize,
                        pipeline: &wgpu::RenderPipeline,
                        input: &wgpu::TextureView,
                        output: &wgpu::TextureView,
                        load: wgpu::LoadOp<wgpu::Color>| {
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom pass"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(input) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: self.params[pass_index].as_entire_binding() },
                ],
            });
            let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            render.set_pipeline(pipeline);
            render.set_bind_group(0, &bind, &[]);
            render.draw(0..3, 0..1);
        };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
        pass(0, &self.down, source, &views[0], clear);
        for i in 1..LEVELS {
            pass(i, &self.down, &views[i - 1], &views[i], clear);
        }
        for i in (0..LEVELS - 1).rev() {
            pass(LEVELS + i, &self.up, &views[i + 1], &views[i], wgpu::LoadOp::Load);
        }
    }
}
