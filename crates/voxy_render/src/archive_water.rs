//! Enclosed water bodies (pools, ponds) for [`crate::ArchiveSceneRenderer`], driven by
//! the ocean's FFT wave field. The water pass runs after opaque shading and before
//! transparents: it refracts the copied opaque frame with absorption and caustics,
//! reflects the screen and sky, writes depth, and fogs the view when the eye is under
//! the surface of a body.
use crate::ocean::{WaveSimulation, WaveSpectrum};
use glam::Vec3;

/// An axis-aligned water volume: the basin interior from `min` (floor corner) to `max`,
/// whose `y` is the water level. The basin itself is ordinary scene geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArchiveWaterBody {
    pub min: Vec3,
    pub max: Vec3,
    /// Beer-Lambert absorption per metre (linear RGB).
    pub absorption: [f32; 3],
    /// Fraction of the ambient light scattered back towards the eye (water colour).
    pub scatter: [f32; 3],
    /// Scale of the vertical wave displacement; small for sheltered water.
    pub displacement: f32,
    /// 0 disables caustics, 1 is physically scaled focusing.
    pub caustics: f32,
}

impl ArchiveWaterBody {
    /// Clear chlorinated pool water.
    pub fn pool(min: Vec3, max: Vec3) -> Self {
        Self {
            min,
            max,
            absorption: [0.42, 0.075, 0.035],
            scatter: [0.006, 0.030, 0.034],
            displacement: 1.0,
            caustics: 1.0,
        }
    }

    fn contains_eye(&self, eye: Vec3) -> bool {
        eye.x > self.min.x
            && eye.x < self.max.x
            && eye.z > self.min.z
            && eye.z < self.max.z
            && eye.y > self.min.y
            && eye.y < self.max.y
    }
}

/// Gentle wind ripples on a sheltered pool: a short fetch and a faint slosh.
pub fn pool_waves() -> WaveSpectrum {
    WaveSpectrum {
        cascades: [9.7, 4.2, 1.8, 0.77, 0.33, 0.14],
        wind_speed: 4.0,
        wind_direction: 70.0,
        fetch_m: 25.0,
        swell_height: 0.012,
        swell_direction: 100.0,
        swell_length: 2.4,
        swell_spread: 6.0,
        choppiness: 0.6,
        short_cut: 0.004,
    }
}

pub(crate) const SOURCE: &str = concat!(include_str!("archive_environment.wgsl"), include_str!("archive_water.wgsl"));

const GRID_SPACING: f32 = 0.08;
/// Waves fade out this far from the basin walls.
const EDGE_FALLOFF: f32 = 0.25;
const LOOP: f32 = 600.0;

#[derive(Debug)]
struct Body {
    source: ArchiveWaterBody,
    uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

#[derive(Debug)]
pub(crate) struct ArchiveWater {
    waves: WaveSimulation,
    surface: wgpu::RenderPipeline,
    fog: wgpu::RenderPipeline,
    bodies: Vec<Body>,
    depth_copy: wgpu::Texture,
    time: f32,
    step: f32,
    size: [u32; 2],
}

fn layout_entry(binding: u32, ty: wgpu::BindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty,
        count: None,
    }
}

fn texture_type(dimension: wgpu::TextureViewDimension, sample_type: wgpu::TextureSampleType) -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type,
        view_dimension: dimension,
        multisampled: false,
    }
}

pub(crate) struct WaterTargets<'a> {
    pub camera_layout: &'a wgpu::BindGroupLayout,
    pub environment_layout: &'a wgpu::BindGroupLayout,
    pub background: &'a wgpu::Texture,
    pub shadow: &'a wgpu::Texture,
    pub size: [u32; 2],
}

pub(crate) struct WaterFrame<'a> {
    pub hdr: &'a wgpu::Texture,
    pub background: &'a wgpu::Texture,
    pub depth: &'a wgpu::Texture,
    pub camera: &'a wgpu::BindGroup,
    pub environment: &'a wgpu::BindGroup,
    pub eye: Vec3,
    /// View frustum planes (inside where `dot(plane, (p, 1)) >= 0`).
    pub planes: [glam::Vec4; 6],
}

impl ArchiveWater {
    pub(crate) fn new(
        device: &wgpu::Device,
        targets: &WaterTargets,
        bodies: &[ArchiveWaterBody],
        spectrum: WaveSpectrum,
    ) -> Self {
        use wgpu::{TextureSampleType as S, TextureViewDimension as D};
        let waves = WaveSimulation::new(device, spectrum);
        let float = S::Float { filterable: true };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("archive water"),
            entries: &[
                layout_entry(
                    0,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                layout_entry(1, texture_type(D::D2Array, float)),
                layout_entry(2, texture_type(D::D2Array, float)),
                layout_entry(3, texture_type(D::D2Array, float)),
                layout_entry(4, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
                layout_entry(5, texture_type(D::D2, S::Float { filterable: false })),
                layout_entry(6, texture_type(D::D2, S::Depth)),
                layout_entry(7, texture_type(D::D2, S::Depth)),
                layout_entry(8, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison)),
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("archive water"),
            source: wgpu::ShaderSource::Wgsl(
                SOURCE.into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("archive water"),
            bind_group_layouts: &[Some(targets.camera_layout), Some(&layout), Some(targets.environment_layout)],
            immediate_size: 0,
        });
        let target = [Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba16Float,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let grid = [Some(wgpu::VertexBufferLayout {
            array_stride: 8,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2],
        })];
        let pipeline = |label, vs, fs, buffers: &[Option<wgpu::VertexBufferLayout>], depth: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers,
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: depth.then(|| wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &target,
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let surface = pipeline("archive water surface", "vs_surface", "fs_surface", &grid, true);
        let fog = pipeline("archive underwater view", "vs_fullscreen", "fs_underwater", &[], false);

        let [width, height] = targets.size;
        let depth_copy = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("archive water opaque depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let wave_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("archive water waves"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("archive water sun shadow"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let background = targets.background.create_view(&Default::default());
        let depth_view = depth_copy.create_view(&Default::default());
        let shadow = targets.shadow.create_view(&Default::default());
        let bodies = bodies
            .iter()
            .map(|source| {
                let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("archive water body"),
                    size: 96,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                use wgpu::BindingResource as R;
                let resources = [
                    uniforms.as_entire_binding(),
                    R::TextureView(&waves.displacement),
                    R::TextureView(&waves.derivatives),
                    R::TextureView(&waves.moments),
                    R::Sampler(&wave_sampler),
                    R::TextureView(&background),
                    R::TextureView(&depth_view),
                    R::TextureView(&shadow),
                    R::Sampler(&shadow_sampler),
                ];
                let entries: Vec<_> = resources
                    .into_iter()
                    .enumerate()
                    .map(|(binding, resource)| wgpu::BindGroupEntry {
                        binding: binding as u32,
                        resource,
                    })
                    .collect();
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("archive water body"),
                    layout: &layout,
                    entries: &entries,
                });
                let size = source.max - source.min;
                let nx = ((size.x / GRID_SPACING).ceil() as u32).max(1);
                let nz = ((size.z / GRID_SPACING).ceil() as u32).max(1);
                let mut points = Vec::with_capacity(((nx + 1) * (nz + 1)) as usize);
                for j in 0..=nz {
                    for i in 0..=nx {
                        points.push([
                            source.min.x + size.x * i as f32 / nx as f32,
                            source.min.z + size.z * j as f32 / nz as f32,
                        ]);
                    }
                }
                let mut indices = Vec::with_capacity((nx * nz * 6) as usize);
                for j in 0..nz {
                    for i in 0..nx {
                        let a = i + j * (nx + 1);
                        let (b, c) = (a + 1, a + nx + 1);
                        indices.extend_from_slice(&[a, c, c + 1, a, c + 1, b]);
                    }
                }
                let buffer = |label, contents: &[u8], usage| {
                    wgpu::util::DeviceExt::create_buffer_init(
                        device,
                        &wgpu::util::BufferInitDescriptor { label: Some(label), contents, usage },
                    )
                };
                Body {
                    source: *source,
                    uniforms,
                    group,
                    vertices: buffer("archive water grid", bytemuck::cast_slice(&points), wgpu::BufferUsages::VERTEX),
                    indices: buffer("archive water indices", bytemuck::cast_slice(&indices), wgpu::BufferUsages::INDEX),
                    count: indices.len() as u32,
                }
            })
            .collect();
        Self {
            waves,
            surface,
            fog,
            bodies,
            depth_copy,
            time: 0.0,
            step: 0.0,
            size: targets.size,
        }
    }

    pub(crate) fn advance(&mut self, dt: f32) {
        self.step = dt;
        self.time = (self.time + dt) % LOOP;
    }

    fn copy(&self, encoder: &mut wgpu::CommandEncoder, frame: &WaterFrame) {
        let extent = wgpu::Extent3d {
            width: self.size[0],
            height: self.size[1],
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_texture(frame.hdr.as_image_copy(), frame.background.as_image_copy(), extent);
        encoder.copy_texture_to_texture(frame.depth.as_image_copy(), self.depth_copy.as_image_copy(), extent);
    }

    /// Encodes the wave update, the water surfaces and, with the eye under a surface,
    /// the underwater view. `frame.hdr` holds the opaque radiance on entry.
    pub(crate) fn encode(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, frame: &WaterFrame) {
        if self.bodies.is_empty() {
            return;
        }
        let visible = |s: &ArchiveWaterBody| {
            let (lo, hi) = (s.min, s.max + Vec3::Y * 0.1);
            frame.planes.iter().all(|p| {
                let corner = Vec3::new(
                    if p.x > 0. { hi.x } else { lo.x },
                    if p.y > 0. { hi.y } else { lo.y },
                    if p.z > 0. { hi.z } else { lo.z },
                );
                p.truncate().dot(corner) + p.w >= 0.
            })
        };
        // Off-screen water costs nothing; the waves simply resume when it returns.
        if !self.bodies.iter().any(|b| visible(&b.source) || b.source.contains_eye(frame.eye)) {
            return;
        }
        let mut submerged = None;
        let inv_l = self.waves_inverse_lengths();
        for (index, body) in self.bodies.iter().enumerate() {
            let s = &body.source;
            let inside = s.contains_eye(frame.eye);
            if inside {
                submerged = Some(index);
            }
            let mut u = [0.0f32; 24];
            u[..4].copy_from_slice(&[s.min.x, s.min.y, s.min.z, EDGE_FALLOFF]);
            u[4..8].copy_from_slice(&[s.max.x, s.max.y, s.max.z, f32::from(u8::from(inside))]);
            u[8..12].copy_from_slice(&[s.absorption[0], s.absorption[1], s.absorption[2], s.displacement]);
            u[12..16].copy_from_slice(&[s.scatter[0], s.scatter[1], s.scatter[2], s.caustics]);
            u[16..22].copy_from_slice(&inv_l);
            queue.write_buffer(&body.uniforms, 0, bytemuck::cast_slice(&u));
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("archive water waves"),
                timestamp_writes: None,
            });
            self.waves.encode(queue, &mut pass, self.time, self.step);
        }
        self.copy(encoder, frame);
        let hdr = frame.hdr.create_view(&Default::default());
        let depth = frame.depth.create_view(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("archive water surfaces"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &hdr,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.surface);
            pass.set_bind_group(0, frame.camera, &[]);
            pass.set_bind_group(2, frame.environment, &[]);
            for body in &self.bodies {
                pass.set_bind_group(1, &body.group, &[]);
                pass.set_vertex_buffer(0, body.vertices.slice(..));
                pass.set_index_buffer(body.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..body.count, 0, 0..1);
            }
        }
        if let Some(index) = submerged {
            self.copy(encoder, frame);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("archive underwater view"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &hdr,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.fog);
            pass.set_bind_group(0, frame.camera, &[]);
            pass.set_bind_group(1, &self.bodies[index].group, &[]);
            pass.set_bind_group(2, frame.environment, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    fn waves_inverse_lengths(&self) -> [f32; 6] {
        self.waves.spectrum_lengths().map(|l| 1.0 / l)
    }
}
