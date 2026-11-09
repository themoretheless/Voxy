//! Device-owned HDR environment convolution with GGX NDF importance sampling.
use crate::RendererError;
use wgpu::util::DeviceExt;
#[derive(Debug)]
pub struct GgxEnvironmentPrefilter {
    input: wgpu::Texture,
    device: wgpu::Device,
    output: wgpu::Texture,
    view: wgpu::TextureView,
    passes: Vec<(wgpu::TextureView, wgpu::BindGroup)>,
    pipeline: wgpu::RenderPipeline,
}
impl GgxEnvironmentPrefilter {
    /// Allocate input/output six-face linear HDR cubes and retained GGX pass bindings.
    /// Input order is +X,-X,+Y,-Y,+Z,-Z. Initialize all input faces before encode.
    /// Output mip roughness is level/(levels-1); mip zero is an unfiltered lookup.
    /// # Errors
    /// Rejects zero dimensions and dimensions exceeding device limits.
    pub fn new(device: &wgpu::Device, size: u32) -> Result<Self, RendererError> {
        Self::with_samples(device, size, 64)
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
        Self::with_samples_format(device, size, samples, wgpu::TextureFormat::Rgba16Float)
    }
    /// Internal full-precision import path; caller validates format/filter support.
    pub(crate) fn with_samples_format(
        device: &wgpu::Device,
        size: u32,
        samples: u32,
        format: wgpu::TextureFormat,
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
        let levels = size.ilog2() + 1;
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
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        };
        let input = allocate("GGX source environment", 1);
        let output = allocate("GGX filtered environment", levels);
        let cube = |texture: &wgpu::Texture| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            })
        };
        let source = cube(&input);
        let view = cube(&output);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("GGX environment sampler"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GGX environment layout"),
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
                let roughness = if levels == 1 {
                    0.0
                } else {
                    level as f32 / (levels - 1) as f32
                };
                let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("GGX face/mip parameters"),
                    contents: bytemuck::cast_slice(&[
                        face,
                        (size >> level).max(1),
                        roughness.to_bits(),
                        samples,
                    ]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("retained GGX cube pass"),
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
            label: Some("GGX environment convolution"),
            source: wgpu::ShaderSource::Wgsl(include_str!("environment_ggx.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GGX cube pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GGX cube prefilter"),
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
                    format,
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
    /// Encode all six faces/all roughness levels; resources are reused, no submission.
    /// Encoder must belong to the owning device (wgpu validation).
    /// This normalized NDF convolution is the environment term, not the full BRDF:
    /// DFG integration, multiple-scattering compensation and local visibility are separate.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        for (view, group) in &self.passes {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GGX environment face/mip"),
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

/// Evaluates the Burley/Heitz anisotropic Trowbridge-Reitz (GGX) Normal Distribution Function
/// `D(h) = 1 / (pi * alpha_x * alpha_y * ((h_x / alpha_x)^2 + (h_y / alpha_y)^2 + h_z^2)^2)`
/// in tangent space (`h_z = N dot H`).
#[must_use]
pub fn anisotropic_ggx_ndf(h_local: [f32; 3], alpha_x: f32, alpha_y: f32) -> f32 {
    if h_local.iter().any(|c| !c.is_finite())
        || !alpha_x.is_finite()
        || !alpha_y.is_finite()
        || h_local[2] <= 0.0
    {
        return 0.0;
    }
    let ax = alpha_x.max(1e-4);
    let ay = alpha_y.max(1e-4);
    let tx = h_local[0] / ax;
    let ty = h_local[1] / ay;
    let tz = h_local[2];
    let d = tx * tx + ty * ty + tz * tz;
    let denom = std::f32::consts::PI * ax * ay * d * d;
    if denom <= 0.0 || !denom.is_finite() {
        0.0
    } else {
        1.0 / denom
    }
}

/// Evaluates the Smith height-correlated anisotropic masking-shadowing function
/// `G2(v, l) = 1 / (1 + Lambda(v) + Lambda(l))` in tangent space.
#[must_use]
pub fn smith_height_correlated_aniso_g2(
    v_local: [f32; 3],
    l_local: [f32; 3],
    alpha_x: f32,
    alpha_y: f32,
) -> f32 {
    if v_local.iter().chain(&l_local).any(|c| !c.is_finite())
        || !alpha_x.is_finite()
        || !alpha_y.is_finite()
        || v_local[2] <= 0.0
        || l_local[2] <= 0.0
    {
        return 0.0;
    }
    let ax = alpha_x.max(1e-4);
    let ay = alpha_y.max(1e-4);
    let lambda = |w: [f32; 3]| {
        let wz2 = (w[2] * w[2]).max(1e-8);
        let slope2 = (ax * ax * w[0] * w[0] + ay * ay * w[1] * w[1]) / wz2;
        0.5 * ((1.0 + slope2).sqrt() - 1.0)
    };
    (1.0 / (1.0 + lambda(v_local) + lambda(l_local))).clamp(0.0, 1.0)
}

/// Samples a microfacet normal `h_local` from the Heitz (2018) GGX Distribution of Visible Normals
/// (VNDF) in tangent space via hemisphere stretching and orthonormal disk projection.
#[must_use]
pub fn sample_ggx_vndf_isotropic(v_local: [f32; 3], alpha: f32, u1: f32, u2: f32) -> [f32; 3] {
    if v_local.iter().any(|c| !c.is_finite())
        || !alpha.is_finite()
        || !u1.is_finite()
        || !u2.is_finite()
    {
        return [0.0, 0.0, 1.0];
    }
    let a = alpha.clamp(1e-4, 1.0);
    let vh_raw = [a * v_local[0], a * v_local[1], v_local[2].max(1e-4)];
    let vh_len = (vh_raw[0] * vh_raw[0] + vh_raw[1] * vh_raw[1] + vh_raw[2] * vh_raw[2]).sqrt();
    let vh = [vh_raw[0] / vh_len, vh_raw[1] / vh_len, vh_raw[2] / vh_len];

    let lensq = vh[0] * vh[0] + vh[1] * vh[1];
    let t1 = if lensq > 1e-7 {
        let inv = lensq.sqrt().recip();
        [-vh[1] * inv, vh[0] * inv, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let t2 = [
        vh[1] * t1[2] - vh[2] * t1[1],
        vh[2] * t1[0] - vh[0] * t1[2],
        vh[0] * t1[1] - vh[1] * t1[0],
    ];

    let r = u1.clamp(0.0, 1.0).sqrt();
    let phi = 2.0 * std::f32::consts::PI * u2.clamp(0.0, 1.0);
    let t1_p = r * phi.cos();
    let s = 0.5 * (1.0 + vh[2]);
    let t2_p = (1.0 - s) * (1.0 - t1_p * t1_p).max(0.0).sqrt() + s * (r * phi.sin());
    let tz_p = (1.0 - t1_p * t1_p - t2_p * t2_p).max(0.0).sqrt();

    let nh = [
        t1_p * t1[0] + t2_p * t2[0] + tz_p * vh[0],
        t1_p * t1[1] + t2_p * t2[1] + tz_p * vh[1],
        t1_p * t1[2] + t2_p * t2[2] + tz_p * vh[2],
    ];
    let ne_raw = [a * nh[0], a * nh[1], nh[2].max(0.0)];
    let ne_len = (ne_raw[0] * ne_raw[0] + ne_raw[1] * ne_raw[1] + ne_raw[2] * ne_raw[2]).sqrt();
    if ne_len <= 1e-7 {
        [0.0, 0.0, 1.0]
    } else {
        [ne_raw[0] / ne_len, ne_raw[1] / ne_len, ne_raw[2] / ne_len]
    }
}

#[cfg(test)]
mod vndf_tests {
    use super::*;

    #[test]
    fn anisotropic_ggx_and_vndf_sample_remain_normalized_and_upper_hemisphere() {
        let d_peak = anisotropic_ggx_ndf([0.0, 0.0, 1.0], 0.2, 0.4);
        let d_off = anisotropic_ggx_ndf([0.5, 0.0, 0.866_025_4], 0.2, 0.4);
        assert!(d_peak > d_off && d_off > 0.0);

        let g2 = smith_height_correlated_aniso_g2([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], 0.3, 0.5);
        assert!((g2 - 1.0).abs() < 1e-5);

        let h = sample_ggx_vndf_isotropic([0.3, -0.2, 0.932_737_9], 0.25, 0.4, 0.7);
        let len = (h[0] * h[0] + h[1] * h[1] + h[2] * h[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-5);
        assert!(h[2] >= 0.0);
    }
}
