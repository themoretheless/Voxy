//! Water bodies for [`crate::ArchiveSceneRenderer`], built on `voxy_water`: basins, lake
//! and sea regions with breaking surf, rivers with rapids and waterfalls, and
//! interactive ripples, all on one FFT wave field. The pass runs after opaque shading
//! and before transparents: it refracts the copied opaque frame with absorption and
//! caustics, reflects the screen and sky, draws white water, writes depth, and fogs the
//! view when the eye is under a surface.
use glam::{Vec2, Vec3};
use voxy_water::{RippleField, WaterShape, WaterWorld, WaveSimulation, surface_mesh};

pub(crate) fn source() -> String {
    [
        include_str!("archive_environment.wgsl"),
        voxy_water::WATER_COMMON_WGSL,
        include_str!("archive_water.wgsl"),
    ]
    .concat()
}

/// Waves fade out this far from basin walls.
const EDGE_FALLOFF: f32 = 0.25;
/// Width of the ripple window that follows the eye (m) and the ripple celerity (m/s).
const RIPPLE_EXTENT: f32 = 24.0;
const RIPPLE_SPEED: f32 = 0.6;
const UNIFORM_FLOATS: usize = 40;
const WAVE_SEED: u32 = 1;

#[derive(Debug)]
struct Body {
    uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    bounds: (Vec3, Vec3),
    static_uniforms: [f32; UNIFORM_FLOATS],
}

#[derive(Debug)]
pub(crate) struct ArchiveWater {
    world: WaterWorld,
    waves: WaveSimulation,
    ripples: RippleField,
    surface: wgpu::RenderPipeline,
    fog: wgpu::RenderPipeline,
    bodies: Vec<Body>,
    depth_copy: wgpu::Texture,
    time: f32,
    pending_seconds: f64,
    size: [u32; 2],
    drawn_frames: u64,
    body_draws: u64,
    last_drawn_bodies: usize,
    last_submitted_time: f32,
}

fn layout_entry(binding: u32, ty: wgpu::BindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty,
        count: None,
    }
}

fn texture_type(
    dimension: wgpu::TextureViewDimension,
    sample_type: wgpu::TextureSampleType,
) -> wgpu::BindingType {
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

fn half_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    size: [u32; 2],
    texels: &[[f32; 4]],
) -> wgpu::Texture {
    let data: Vec<u16> = texels
        .iter()
        .flatten()
        .map(|v| half::f16::from_f32(*v).to_bits())
        .collect();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        bytemuck::cast_slice(&data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size[0] * 8),
            rows_per_image: None,
        },
        texture.size(),
    );
    texture
}

impl ArchiveWater {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        targets: &WaterTargets,
        world: &WaterWorld,
    ) -> Self {
        let spectrum = world.spectrum;
        use wgpu::{TextureSampleType as S, TextureViewDimension as D};
        let waves = WaveSimulation::new(device, spectrum, WAVE_SEED);
        let ripples = RippleField::new(device, RIPPLE_EXTENT, RIPPLE_SPEED);
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
                layout_entry(
                    4,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
                layout_entry(5, texture_type(D::D2, S::Float { filterable: false })),
                layout_entry(6, texture_type(D::D2, S::Depth)),
                layout_entry(7, texture_type(D::D2, S::Depth)),
                layout_entry(
                    8,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                ),
                layout_entry(9, texture_type(D::D2, float)),
                layout_entry(10, texture_type(D::D2, float)),
                layout_entry(
                    11,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("archive water"),
            source: wgpu::ShaderSource::Wgsl(source().into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("archive water"),
            bind_group_layouts: &[
                Some(targets.camera_layout),
                Some(&layout),
                Some(targets.environment_layout),
            ],
            immediate_size: 0,
        });
        let target = [Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba16Float,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let vertex = [Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<voxy_water::WaterVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x2, 3 => Float32, 4 => Float32],
        })];
        let pipeline =
            |label, vs, fs, buffers: &[Option<wgpu::VertexBufferLayout>], depth: bool| {
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
        let surface = pipeline(
            "archive water surface",
            "vs_surface",
            "fs_surface",
            &vertex,
            true,
        );
        let fog = pipeline(
            "archive underwater view",
            "vs_fullscreen",
            "fs_underwater",
            &[],
            false,
        );

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
        let sampler = |label, address| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: address,
                address_mode_v: address,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                anisotropy_clamp: 8,
                ..Default::default()
            })
        };
        let wave_sampler = sampler("archive water waves", wgpu::AddressMode::Repeat);
        let clamp_sampler = sampler("archive water shore", wgpu::AddressMode::ClampToEdge);
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
        let no_shore = half_texture(
            device,
            queue,
            "archive water no shore",
            [1, 1],
            &[[100.0, 1.0e4, 0.0, 0.0]],
        );
        let inv_l = spectrum.cascades.map(|l| 1.0 / l);
        let bodies = world
            .bodies
            .iter()
            .enumerate()
            .map(|(index, body)| {
                let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("archive water body"),
                    size: (UNIFORM_FLOATS * 4) as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let shore_texture = body.shore.as_ref().map(|s| {
                    half_texture(device, queue, "archive water shore", s.size, &s.texels())
                });
                let shore_view = shore_texture
                    .as_ref()
                    .unwrap_or(&no_shore)
                    .create_view(&Default::default());
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
                    R::TextureView(&shore_view),
                    R::TextureView(ripples.view()),
                    R::Sampler(&clamp_sampler),
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
                let mesh = surface_mesh(body, world.river_samples(index));
                let buffer = |label, contents: &[u8], usage| {
                    wgpu::util::DeviceExt::create_buffer_init(
                        device,
                        &wgpu::util::BufferInitDescriptor {
                            label: Some(label),
                            contents,
                            usage,
                        },
                    )
                };
                let (lo, hi) = body.bounds();
                let m = &body.material;
                let mut u = [0.0f32; UNIFORM_FLOATS];
                let edge = if matches!(body.shape, WaterShape::Basin { .. }) {
                    EDGE_FALLOFF
                } else {
                    0.0
                };
                u[..4].copy_from_slice(&[lo.x, lo.y, lo.z, edge]);
                u[4..8].copy_from_slice(&[hi.x, hi.y, hi.z, -1.0e9]);
                u[8..12].copy_from_slice(&[
                    m.absorption[0],
                    m.absorption[1],
                    m.absorption[2],
                    body.waves,
                ]);
                u[12..16].copy_from_slice(&[m.scatter[0], m.scatter[1], m.scatter[2], m.caustics]);
                u[16..22].copy_from_slice(&inv_l);
                if let Some(s) = &body.shore {
                    let origin = s.origin - Vec2::splat(s.cell * 0.5);
                    u[24..28].copy_from_slice(&[
                        origin.x,
                        origin.y,
                        s.cell * s.size[0] as f32,
                        s.cell * s.size[1] as f32,
                    ]);
                }
                let b = &body.breakers;
                u[28..32].copy_from_slice(&[
                    b.height,
                    b.period,
                    b.reach,
                    f32::from(u8::from(body.shore.is_some())),
                ]);
                u[36..40].copy_from_slice(&[
                    0.0,
                    m.foam,
                    f32::from(u8::from(matches!(body.shape, WaterShape::River(_)))),
                    0.0,
                ]);
                Body {
                    uniforms,
                    group,
                    vertices: buffer(
                        "archive water surface",
                        bytemuck::cast_slice(&mesh.vertices),
                        wgpu::BufferUsages::VERTEX,
                    ),
                    indices: buffer(
                        "archive water indices",
                        bytemuck::cast_slice(&mesh.indices),
                        wgpu::BufferUsages::INDEX,
                    ),
                    count: mesh.indices.len() as u32,
                    // room for waves, breakers and falls spray
                    bounds: (lo - Vec3::splat(0.5), hi + Vec3::splat(1.5)),
                    static_uniforms: u,
                }
            })
            .collect();
        Self {
            world: world.clone(),
            waves,
            ripples,
            surface,
            fog,
            bodies,
            depth_copy,
            time: 0.0,
            pending_seconds: 0.0,
            size: targets.size,
            drawn_frames: 0,
            body_draws: 0,
            last_drawn_bodies: 0,
            last_submitted_time: 0.,
        }
    }

    pub(crate) fn advance(&mut self, dt: f32) {
        assert!(
            dt.is_finite() && dt >= 0.,
            "finite nonnegative water timestep"
        );
        self.pending_seconds += f64::from(dt);
        self.time += dt;
    }

    /// CPU submission evidence, not a GPU-duration or visible-pixel count.
    pub(crate) fn render_stats(&self) -> serde_json::Value {
        serde_json::json!({"registered_bodies":self.bodies.len(),"last_drawn_bodies":self.last_drawn_bodies,
            "drawn_frames":self.drawn_frames,"body_draws":self.body_draws,"simulation_seconds":self.time,
            "simulation_time_bits":self.time.to_bits(),"last_submitted_time_bits":self.last_submitted_time.to_bits(),"wave_seed":WAVE_SEED,"ripple_encoded_seconds":self.ripples.encoded_seconds(),
            "ripple_pending_seconds":self.pending_seconds+self.ripples.pending_seconds(),
            "ripple_requested_seconds":self.pending_seconds+self.ripples.requested_seconds(),
            "ripple_encoded_ticks":self.ripples.encoded_ticks(),"ripple_tick_rate":self.ripples.tick_rate()})
    }
    pub(crate) fn time(&self) -> f32 {
        self.time
    }

    pub(crate) fn world(&self) -> &WaterWorld {
        &self.world
    }

    pub(crate) fn ripples(&mut self) -> &mut RippleField {
        &mut self.ripples
    }

    fn copy(&self, encoder: &mut wgpu::CommandEncoder, frame: &WaterFrame) {
        let extent = wgpu::Extent3d {
            width: self.size[0],
            height: self.size[1],
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_texture(
            frame.hdr.as_image_copy(),
            frame.background.as_image_copy(),
            extent,
        );
        encoder.copy_texture_to_texture(
            frame.depth.as_image_copy(),
            self.depth_copy.as_image_copy(),
            extent,
        );
    }

    /// Encodes the wave and ripple updates, the water surfaces and, with the eye under a
    /// surface, the underwater view. `frame.hdr` holds the opaque radiance on entry.
    pub(crate) fn encode(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &WaterFrame,
    ) {
        let visible = |(lo, hi): (Vec3, Vec3)| {
            frame.planes.iter().all(|p| {
                let corner = Vec3::new(
                    if p.x > 0. { hi.x } else { lo.x },
                    if p.y > 0. { hi.y } else { lo.y },
                    if p.z > 0. { hi.z } else { lo.z },
                );
                p.truncate().dot(corner) + p.w >= 0.
            })
        };
        let eye_water = self
            .world
            .sample(frame.eye, self.time)
            .filter(|s| s.submersion(frame.eye) > 0.0);
        let drawn: Vec<usize> = (0..self.bodies.len())
            .filter(|&i| visible(self.bodies[i].bounds) || eye_water.is_some_and(|s| s.body == i))
            .collect();
        self.last_drawn_bodies = drawn.len();
        // Off-screen water costs nothing; waves and ripples simply resume when it returns.
        if drawn.is_empty() {
            return;
        }
        self.drawn_frames += 1;
        self.body_draws += drawn.len() as u64;
        self.last_submitted_time = self.time;
        let eye_xz = Vec2::new(frame.eye.x, frame.eye.z);
        // ripples reflect off the walls of the basin the eye is over
        let mask = self.world.bodies.iter().find_map(|b| match b.shape {
            WaterShape::Basin { min, max }
                if eye_xz.x > min.x - 8.
                    && eye_xz.x < max.x + 8.
                    && eye_xz.y > min.z - 8.
                    && eye_xz.y < max.z + 8. =>
            {
                Some((Vec2::new(min.x, min.z), Vec2::new(max.x, max.z)))
            }
            _ => None,
        });
        self.ripples.set_mask(mask);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("archive water simulation"),
                timestamp_writes: None,
            });
            self.waves
                .encode(queue, &mut pass, self.time, self.pending_seconds as f32);
            self.ripples
                .encode_elapsed(queue, &mut pass, eye_xz, self.pending_seconds);
            self.pending_seconds = 0.;
        }
        let center = self.ripples.center();
        for &i in &drawn {
            let body = &self.bodies[i];
            let mut u = body.static_uniforms;
            if let Some(s) = eye_water.filter(|s| s.body == i) {
                u[7] = s.surface;
            }
            u[32..36].copy_from_slice(&[center.x, center.y, self.ripples.extent(), 1.0]);
            u[36] = self.time;
            queue.write_buffer(&body.uniforms, 0, bytemuck::cast_slice(&u));
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
            for &i in &drawn {
                let body = &self.bodies[i];
                pass.set_bind_group(1, &body.group, &[]);
                pass.set_vertex_buffer(0, body.vertices.slice(..));
                pass.set_index_buffer(body.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..body.count, 0, 0..1);
            }
        }
        if let Some(water) = eye_water {
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
            pass.set_bind_group(1, &self.bodies[water.body].group, &[]);
            pass.set_bind_group(2, frame.environment, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
