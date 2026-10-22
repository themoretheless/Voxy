//! Open-ocean water variant: Tessendorf FFT waves (six 256² cascades, 1103 m to 0.26 m),
//! CDLOD surface mesh out to 160 km, LEAN-mapped sun glitter and sky reflection,
//! Jacobian whitecaps in camera-centred foam clipmaps, a Hillaire-style atmosphere with
//! two cloud layers, and HDR post (4x MSAA, bloom, AgX).
//!
//! Ported from web-ocean by Maksymilian Dendura (MIT, see `LICENSE-web-ocean`). The WGSL
//! is the original shader code with the JavaScript template constants resolved; the only
//! functional change is that the composite pass can target an sRGB view.
//!
//! The renderer owns every pass and resolves straight into a caller-provided view, so it
//! drops into `SceneSurface::render_custom` or an offscreen texture.

mod cdlod;

use cdlod::{Cdlod, GRID, MAX_NODES, R0};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
use voxy_water::spectrum::LOOP;
use voxy_water::{WaveSimulation, WaveSpectrum};

const CASCADES: u32 = 6;
const SKY_W: u32 = 256;
const SKY_H: u32 = 144;
const ENV_SIZE: u32 = 256;
const ENV_MIPS: u32 = 9;
const FOAM_SIZE: u32 = 512;
const FOAM_MIPS: u32 = 10;
const FOAM_RES: u32 = 1024;
const FOAM_WORLD_MIPS: u32 = FOAM_RES.ilog2() + 1;
const FOAM_LEVEL_SIZE: [f32; 2] = [128.0, 1024.0];
const BLOOM_LEVELS: u32 = 8;
const MSAA: u32 = 4;
const HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const CASC_L: [f32; CASCADES as usize] = [1103.0, 211.3, 39.7, 7.43, 1.393, 0.2617];

/// Artist-facing ocean parameters. Angles are in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OceanSettings {
    /// Wind speed at 10 m (m/s).
    pub wind_speed: f32,
    /// Direction the wind waves travel (90 = towards +Z).
    pub wind_direction: f32,
    pub fetch_km: f32,
    /// Swell significant height (m).
    pub swell_height: f32,
    pub swell_direction: f32,
    /// Swell peak wavelength (m).
    pub swell_length: f32,
    /// `cos^2s` spreading exponent of the swell.
    pub swell_spread: f32,
    pub choppiness: f32,
    /// Water body brightness (coastal sediment / plankton).
    pub turbidity: f32,
    pub foam: f32,
    pub sun_elevation: f32,
    /// 0 looks along -Z.
    pub sun_azimuth: f32,
    pub exposure: f32,
    /// Aerosol (Mie) multiplier.
    pub haze: f32,
    /// Low cumulus coverage.
    pub clouds: f32,
    /// High altostratus deck coverage.
    pub cloud_deck: f32,
    pub bloom: f32,
}

impl Default for OceanSettings {
    fn default() -> Self {
        Self {
            wind_speed: 7.0,
            wind_direction: 118.0,
            fetch_km: 90.0,
            swell_height: 1.8,
            swell_direction: 94.0,
            swell_length: 120.0,
            swell_spread: 48.0,
            choppiness: 1.25,
            turbidity: 1.6,
            foam: 1.0,
            sun_elevation: 2.6,
            sun_azimuth: 4.0,
            exposure: 1.35,
            haze: 3.0,
            clouds: 0.40,
            cloud_deck: 0.36,
            bloom: 0.10,
        }
    }
}

impl OceanSettings {
    fn spectrum(&self) -> WaveSpectrum {
        WaveSpectrum {
            cascades: CASC_L,
            wind_speed: self.wind_speed,
            wind_direction: self.wind_direction,
            fetch_m: self.fetch_km * 1000.0,
            swell_height: self.swell_height,
            swell_direction: self.swell_direction,
            swell_length: self.swell_length,
            swell_spread: self.swell_spread,
            choppiness: self.choppiness,
            short_cut: 0.0012,
        }
    }
}

/// Free camera: world position in metres (+Y up), yaw/pitch in radians.
/// Yaw 0 looks along -Z, positive yaw turns towards +X.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OceanCamera {
    pub position: glam::Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y: f32,
}

impl Default for OceanCamera {
    fn default() -> Self {
        Self {
            position: glam::Vec3::new(0.0, 2.4, 0.0),
            yaw: 0.0,
            pitch: (-5.5f32).to_radians(),
            fov_y: 58f32.to_radians(),
        }
    }
}

impl OceanCamera {
    /// Unit forward, right and up vectors.
    pub fn basis(&self) -> [glam::Vec3; 3] {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let forward = glam::Vec3::new(cp * sy, sp, -cp * cy);
        let right = glam::Vec3::new(cy, 0.0, sy);
        [forward, right, right.cross(forward)]
    }
}

macro_rules! wgsl {
    ($($file:literal),+ $(,)?) => {
        concat!($(include_str!($file)),+)
    };
}

/// Pipeline sources assembled from the shared WGSL fragments.
mod shaders {
    pub(super) const TRANSMITTANCE: &str = wgsl!("atmosphere.wgsl", "sky_transmittance.wgsl");
    pub(super) const MULTISCATTER: &str = wgsl!("atmosphere.wgsl", "sky_multiscatter.wgsl");
    pub(super) const SKY_VIEW: &str = wgsl!("frame.wgsl", "atmosphere.wgsl", "sky_view.wgsl");
    pub(super) const ENV_CUBE: &str = wgsl!(
        "frame.wgsl",
        "atmosphere.wgsl",
        "noise.wgsl",
        "sky_render.wgsl",
        "sky_clouds.wgsl",
        "env_cube.wgsl"
    );
    pub(super) const SUN_VISIBILITY: &str = wgsl!(
        "frame.wgsl",
        "atmosphere.wgsl",
        "noise.wgsl",
        "sky_render.wgsl",
        "sky_clouds.wgsl",
        "sun_visibility.wgsl"
    );
    pub(super) const HEIGHT_PROBE: &str = wgsl!("frame.wgsl", "height_probe.wgsl");
    pub(super) const SURFACE: &str = wgsl!(
        "frame.wgsl",
        "atmosphere.wgsl",
        "noise.wgsl",
        "sky_render.wgsl",
        "surface.wgsl"
    );
    pub(super) const SKY: &str = wgsl!(
        "frame.wgsl",
        "atmosphere.wgsl",
        "noise.wgsl",
        "sky_render.wgsl",
        "sky_clouds.wgsl",
        "sky_background.wgsl"
    );
    pub(super) const BLOOM_DOWN: &str = wgsl!("fullscreen.wgsl", "bloom_down.wgsl");
    pub(super) const BLOOM_UP: &str = wgsl!("fullscreen.wgsl", "bloom_up.wgsl");
    pub(super) const COMPOSITE: &str = wgsl!("fullscreen.wgsl", "composite.wgsl");
    pub(super) const FOAM_TEXTURE: &str =
        wgsl!("noise.wgsl", "periodic_noise.wgsl", "foam_texture.wgsl");
}

#[derive(Debug)]
struct ScreenTargets {
    size: [u32; 2],
    color_ms: wgpu::TextureView,
    depth_ms: wgpu::TextureView,
    hdr: wgpu::TextureView,
    bloom: Vec<wgpu::TextureView>,
    down: Vec<wgpu::BindGroup>,
    up: Vec<wgpu::BindGroup>,
    composite: wgpu::BindGroup,
}

#[derive(Debug)]
struct Pipelines {
    foam_update: wgpu::ComputePipeline,
    array16_mips: wgpu::ComputePipeline,
    transmittance: wgpu::ComputePipeline,
    multiscatter: wgpu::ComputePipeline,
    sky_view: wgpu::ComputePipeline,
    env: wgpu::ComputePipeline,
    cube_mips: wgpu::ComputePipeline,
    sun_visibility: wgpu::ComputePipeline,
    probe: wgpu::ComputePipeline,
    ocean: wgpu::RenderPipeline,
    sky: wgpu::RenderPipeline,
    bloom_down: wgpu::RenderPipeline,
    bloom_up: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
}

#[derive(Debug)]
struct BindGroups {
    foam_update: [wgpu::BindGroup; 2],
    foam_mips: [Vec<wgpu::BindGroup>; 2],
    transmittance: wgpu::BindGroup,
    multiscatter: wgpu::BindGroup,
    sky_view: wgpu::BindGroup,
    env: wgpu::BindGroup,
    cube_mips: Vec<wgpu::BindGroup>,
    sun_visibility: wgpu::BindGroup,
    probe: wgpu::BindGroup,
    ocean: [wgpu::BindGroup; 2],
    sky: wgpu::BindGroup,
}

/// Self-contained ocean, sky and post-processing renderer.
#[derive(Debug)]
pub struct OceanRenderer {
    pipelines: Pipelines,
    groups: BindGroups,
    screen: ScreenTargets,
    output_srgb: bool,
    waves: WaveSimulation,
    foam_ubo: wgpu::Buffer,
    atmo_ubo: wgpu::Buffer,
    frame_ubo: wgpu::Buffer,
    composite_ubo: wgpu::Buffer,
    bloom_ubos: Vec<wgpu::Buffer>,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    probe_out: wgpu::Buffer,
    probe_read: wgpu::Buffer,
    /// 0 = in flight, 1 = mapped, 2 = mapping failed.
    probe_state: Arc<AtomicU8>,
    probe_pending: bool,
    water_height: f32,
    linear_sampler: wgpu::Sampler,
    cdlod: Cdlod,
    settings: OceanSettings,
    need_lut: bool,
    env_full: u32,
    foam_first: bool,
    foam_current: usize,
    foam_snap: [i32; 4],
    sim_time: f32,
    frame_index: u32,
}

fn compute_pipeline(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    })
}

struct RenderPipelineSpec<'a> {
    label: &'a str,
    source: &'a str,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
    depth: Option<wgpu::CompareFunction>,
    instanced: bool,
    samples: u32,
}

fn render_pipeline(device: &wgpu::Device, spec: &RenderPipelineSpec) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(spec.label),
        source: wgpu::ShaderSource::Wgsl(spec.source.into()),
    });
    let instance_layout = Some(wgpu::VertexBufferLayout {
        array_stride: 16,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 0,
            shader_location: 0,
        }],
    });
    let buffers = if spec.instanced {
        std::slice::from_ref(&instance_layout)
    } else {
        &[]
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(spec.label),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers,
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: spec.depth.map(|compare| wgpu::DepthStencilState {
            format: DEPTH,
            depth_write_enabled: Some(compare == wgpu::CompareFunction::Greater),
            depth_compare: Some(compare),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: spec.samples,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: spec.format,
                blend: spec.blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    resources: &[wgpu::BindingResource],
) -> wgpu::BindGroup {
    let entries: Vec<_> = resources
        .iter()
        .enumerate()
        .map(|(binding, resource)| wgpu::BindGroupEntry {
            binding: binding as u32,
            resource: resource.clone(),
        })
        .collect();
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &entries,
    })
}

fn view(
    texture: &wgpu::Texture,
    mip: Option<u32>,
    dimension: wgpu::TextureViewDimension,
) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(dimension),
        base_mip_level: mip.unwrap_or(0),
        mip_level_count: mip.map(|_| 1),
        ..Default::default()
    })
}

fn texture(
    device: &wgpu::Device,
    label: &str,
    size: [u32; 3],
    format: wgpu::TextureFormat,
    mips: u32,
    samples: u32,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: size[2],
        },
        mip_level_count: mips,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

fn sampler(
    device: &wgpu::Device,
    address: wgpu::AddressMode,
    mipmaps: bool,
    anisotropy: u16,
) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: address,
        address_mode_v: address,
        address_mode_w: address,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: if mipmaps {
            wgpu::MipmapFilterMode::Linear
        } else {
            wgpu::MipmapFilterMode::Nearest
        },
        anisotropy_clamp: anisotropy,
        ..Default::default()
    })
}

fn groups_for(n: u32) -> u32 {
    n.div_ceil(8)
}

impl OceanRenderer {
    /// Builds every pipeline and simulation resource; `output_format` is the final view
    /// format passed to [`Self::encode`].
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        use wgpu::{
            BindingResource as R, BufferUsages as BU, TextureUsages as TU,
            TextureViewDimension as D,
        };
        let pipelines = Pipelines {
            foam_update: compute_pipeline(
                device,
                "ocean foam update",
                include_str!("foam_update.wgsl"),
            ),
            array16_mips: compute_pipeline(
                device,
                "ocean foam mips",
                include_str!("array16_mips.wgsl"),
            ),
            transmittance: compute_pipeline(device, "sky transmittance", shaders::TRANSMITTANCE),
            multiscatter: compute_pipeline(device, "sky multiscatter", shaders::MULTISCATTER),
            sky_view: compute_pipeline(device, "sky view", shaders::SKY_VIEW),
            env: compute_pipeline(device, "sky environment cube", shaders::ENV_CUBE),
            cube_mips: compute_pipeline(device, "sky cube mips", include_str!("cube_mips.wgsl")),
            sun_visibility: compute_pipeline(device, "sun visibility", shaders::SUN_VISIBILITY),
            probe: compute_pipeline(device, "ocean height probe", shaders::HEIGHT_PROBE),
            ocean: render_pipeline(
                device,
                &RenderPipelineSpec {
                    label: "ocean surface",
                    source: shaders::SURFACE,
                    format: HDR,
                    blend: None,
                    depth: Some(wgpu::CompareFunction::Greater),
                    instanced: true,
                    samples: MSAA,
                },
            ),
            sky: render_pipeline(
                device,
                &RenderPipelineSpec {
                    label: "ocean sky",
                    source: shaders::SKY,
                    format: HDR,
                    blend: None,
                    depth: Some(wgpu::CompareFunction::Equal),
                    instanced: false,
                    samples: MSAA,
                },
            ),
            bloom_down: render_pipeline(
                device,
                &RenderPipelineSpec {
                    label: "ocean bloom down",
                    source: shaders::BLOOM_DOWN,
                    format: HDR,
                    blend: None,
                    depth: None,
                    instanced: false,
                    samples: 1,
                },
            ),
            bloom_up: render_pipeline(
                device,
                &RenderPipelineSpec {
                    label: "ocean bloom up",
                    source: shaders::BLOOM_UP,
                    format: HDR,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    depth: None,
                    instanced: false,
                    samples: 1,
                },
            ),
            composite: render_pipeline(
                device,
                &RenderPipelineSpec {
                    label: "ocean composite",
                    source: shaders::COMPOSITE,
                    format: output_format,
                    blend: None,
                    depth: None,
                    instanced: false,
                    samples: 1,
                },
            ),
        };
        let p = &pipelines;

        // simulation
        let settings = OceanSettings::default();
        let waves = WaveSimulation::new(device, settings.spectrum(), 1);
        let (disp_all, deriv_all, moments_all) =
            (&waves.displacement, &waves.derivatives, &waves.moments);
        let level = |t: &wgpu::Texture, m: u32| view(t, Some(m), D::D2Array);
        let bg = |pipe: &wgpu::BindGroupLayout, label, resources: &[R]| {
            bind_group(device, pipe, label, resources)
        };
        let layout_c = |pipe: &wgpu::ComputePipeline| pipe.get_bind_group_layout(0);

        // foam micro-structure (generated once)
        let foam_detail = texture(
            device,
            "ocean foam detail",
            [FOAM_SIZE, FOAM_SIZE, 1],
            wgpu::TextureFormat::Rgba8Unorm,
            FOAM_MIPS,
            1,
            TU::TEXTURE_BINDING | TU::STORAGE_BINDING,
        );
        {
            let generate = compute_pipeline(device, "ocean foam texture", shaders::FOAM_TEXTURE);
            let mips = compute_pipeline(
                device,
                "ocean foam texture mips",
                include_str!("rgba8_mips.wgsl"),
            );
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                let base = view(&foam_detail, Some(0), D::D2);
                pass.set_pipeline(&generate);
                pass.set_bind_group(
                    0,
                    &bg(
                        &layout_c(&generate),
                        "ocean foam texture",
                        &[R::TextureView(&base)],
                    ),
                    &[],
                );
                pass.dispatch_workgroups(FOAM_SIZE / 8, FOAM_SIZE / 8, 1);
                pass.set_pipeline(&mips);
                for m in 1..FOAM_MIPS {
                    let s = FOAM_SIZE >> m;
                    let (src, dst) = (
                        view(&foam_detail, Some(m - 1), D::D2),
                        view(&foam_detail, Some(m), D::D2),
                    );
                    pass.set_bind_group(
                        0,
                        &bg(
                            &layout_c(&mips),
                            "ocean foam texture mip",
                            &[R::TextureView(&src), R::TextureView(&dst)],
                        ),
                        &[],
                    );
                    pass.dispatch_workgroups(groups_for(s), groups_for(s), 1);
                }
            }
            queue.submit([encoder.finish()]);
        }

        // world-space foam clipmaps (ping-pong)
        let foam_world = [0, 1].map(|i| {
            texture(
                device,
                if i == 0 {
                    "ocean foam world 0"
                } else {
                    "ocean foam world 1"
                },
                [FOAM_RES, FOAM_RES, 2],
                HDR,
                FOAM_WORLD_MIPS,
                1,
                TU::TEXTURE_BINDING | TU::STORAGE_BINDING,
            )
        });
        let foam_ubo = buffer(device, "ocean foam ubo", 144, BU::UNIFORM | BU::COPY_DST);
        let sim_sampler = sampler(device, wgpu::AddressMode::Repeat, true, 1);
        let foam_update = [0usize, 1].map(|i| {
            let (prev, dst) = (level(&foam_world[1 - i], 0), level(&foam_world[i], 0));
            bg(
                &layout_c(&p.foam_update),
                "ocean foam update",
                &[
                    foam_ubo.as_entire_binding(),
                    R::TextureView(&prev),
                    R::TextureView(&dst),
                    R::TextureView(disp_all),
                    R::TextureView(deriv_all),
                    R::Sampler(&sim_sampler),
                    R::Sampler(&sim_sampler),
                ],
            )
        });
        let foam_mips = [0usize, 1].map(|i| {
            (1..FOAM_WORLD_MIPS)
                .map(|m| {
                    let (src, dst) = (level(&foam_world[i], m - 1), level(&foam_world[i], m));
                    bg(
                        &layout_c(&p.array16_mips),
                        "ocean foam mip",
                        &[R::TextureView(&src), R::TextureView(&dst)],
                    )
                })
                .collect()
        });

        // atmosphere
        let atmo_ubo = buffer(
            device,
            "ocean atmosphere ubo",
            16,
            BU::UNIFORM | BU::COPY_DST,
        );
        let lut = |label, w, h| {
            texture(
                device,
                label,
                [w, h, 1],
                HDR,
                1,
                1,
                TU::TEXTURE_BINDING | TU::STORAGE_BINDING,
            )
        };
        let trans_lut = lut("sky transmittance", 256, 64);
        let ms_lut = lut("sky multiscatter", 32, 32);
        let sky_lut = lut("sky view", SKY_W, SKY_H);
        let env = texture(
            device,
            "sky environment cube",
            [ENV_SIZE, ENV_SIZE, 6],
            HDR,
            ENV_MIPS,
            1,
            TU::TEXTURE_BINDING | TU::STORAGE_BINDING,
        );
        let env_cube = view(&env, None, D::Cube);
        let clamp_sampler = sampler(device, wgpu::AddressMode::ClampToEdge, true, 1);
        let repeat_sampler = sampler(device, wgpu::AddressMode::Repeat, true, 16);
        let frame_ubo = buffer(device, "ocean frame ubo", 384, BU::UNIFORM | BU::COPY_DST);
        let sun_vis = buffer(device, "sun visibility", 16, BU::STORAGE);
        let trans_view = view(&trans_lut, None, D::D2);
        let ms_view = view(&ms_lut, None, D::D2);
        let sky_view = view(&sky_lut, None, D::D2);
        let env0 = level(&env, 0);
        let transmittance = bg(
            &layout_c(&p.transmittance),
            "sky transmittance",
            &[atmo_ubo.as_entire_binding(), R::TextureView(&trans_view)],
        );
        let multiscatter = bg(
            &layout_c(&p.multiscatter),
            "sky multiscatter",
            &[
                atmo_ubo.as_entire_binding(),
                R::TextureView(&trans_view),
                R::Sampler(&clamp_sampler),
                R::TextureView(&ms_view),
            ],
        );
        let sky_view_group = bg(
            &layout_c(&p.sky_view),
            "sky view",
            &[
                frame_ubo.as_entire_binding(),
                R::TextureView(&trans_view),
                R::TextureView(&ms_view),
                R::Sampler(&clamp_sampler),
                R::TextureView(&sky_view),
            ],
        );
        let env_group = bg(
            &layout_c(&p.env),
            "sky environment cube",
            &[
                frame_ubo.as_entire_binding(),
                R::TextureView(&trans_view),
                R::TextureView(&sky_view),
                R::Sampler(&clamp_sampler),
                R::TextureView(&env0),
            ],
        );
        let sun_visibility = bg(
            &layout_c(&p.sun_visibility),
            "sun visibility",
            &[
                frame_ubo.as_entire_binding(),
                R::TextureView(&trans_view),
                R::TextureView(&sky_view),
                R::Sampler(&clamp_sampler),
                sun_vis.as_entire_binding(),
            ],
        );
        let cube_mips = (1..ENV_MIPS)
            .map(|m| {
                let (src, dst) = (level(&env, m - 1), level(&env, m));
                bg(
                    &layout_c(&p.cube_mips),
                    "sky cube mip",
                    &[R::TextureView(&src), R::TextureView(&dst)],
                )
            })
            .collect();

        // surface and sky passes
        let foam_detail_view = view(&foam_detail, None, D::D2);
        let ocean_layout = p.ocean.get_bind_group_layout(0);
        let ocean = [0usize, 1].map(|i| {
            let foam_all = view(&foam_world[i], None, D::D2Array);
            bg(
                &ocean_layout,
                "ocean surface",
                &[
                    frame_ubo.as_entire_binding(),
                    R::TextureView(&trans_view),
                    R::TextureView(&sky_view),
                    R::Sampler(&clamp_sampler),
                    R::TextureView(disp_all),
                    R::TextureView(deriv_all),
                    R::TextureView(moments_all),
                    R::TextureView(&foam_detail_view),
                    R::TextureView(&env_cube),
                    R::Sampler(&repeat_sampler),
                    R::TextureView(&foam_all),
                    sun_vis.as_entire_binding(),
                ],
            )
        });
        let sky = bg(
            &p.sky.get_bind_group_layout(0),
            "ocean sky",
            &[
                frame_ubo.as_entire_binding(),
                R::TextureView(&trans_view),
                R::TextureView(&sky_view),
                R::Sampler(&clamp_sampler),
            ],
        );

        // water height under the camera, read back a few frames late
        let probe_out = buffer(device, "ocean probe out", 16, BU::STORAGE | BU::COPY_SRC);
        let probe_read = buffer(device, "ocean probe read", 16, BU::MAP_READ | BU::COPY_DST);
        let probe = bg(
            &layout_c(&p.probe),
            "ocean height probe",
            &[
                frame_ubo.as_entire_binding(),
                R::TextureView(disp_all),
                R::Sampler(&sim_sampler),
                probe_out.as_entire_binding(),
            ],
        );

        // CDLOD grid
        let mut indices = Vec::with_capacity((GRID * GRID * 6) as usize);
        for j in 0..GRID {
            for i in 0..GRID {
                let a = (i + j * (GRID + 1)) as u16;
                let b = a + 1;
                let c = a + GRID as u16 + 1;
                let d = c + 1;
                indices.extend_from_slice(&[a, c, d, a, d, b]);
            }
        }
        let index_buffer = buffer(
            device,
            "ocean grid indices",
            (indices.len() * 2) as u64,
            BU::INDEX | BU::COPY_DST,
        );
        queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));
        let instance_buffer = buffer(
            device,
            "ocean grid instances",
            (MAX_NODES * 16) as u64,
            BU::VERTEX | BU::COPY_DST,
        );

        let linear_sampler = sampler(device, wgpu::AddressMode::ClampToEdge, false, 1);
        let composite_ubo = buffer(
            device,
            "ocean composite ubo",
            32,
            BU::UNIFORM | BU::COPY_DST,
        );
        let bloom_ubos = (0..BLOOM_LEVELS * 2)
            .map(|_| buffer(device, "ocean bloom ubo", 16, BU::UNIFORM | BU::COPY_DST))
            .collect::<Vec<_>>();
        let screen = Self::create_screen(
            device,
            queue,
            &pipelines,
            &linear_sampler,
            &bloom_ubos,
            &composite_ubo,
            width,
            height,
        );

        Self {
            groups: BindGroups {
                foam_update,
                foam_mips,
                transmittance,
                multiscatter,
                sky_view: sky_view_group,
                env: env_group,
                cube_mips,
                sun_visibility,
                probe,
                ocean,
                sky,
            },
            pipelines,
            screen,
            waves,
            output_srgb: output_format.is_srgb(),
            foam_ubo,
            atmo_ubo,
            frame_ubo,
            composite_ubo,
            bloom_ubos,
            index_buffer,
            instance_buffer,
            probe_out,
            probe_read,
            probe_state: Arc::new(AtomicU8::new(0)),
            probe_pending: false,
            water_height: 0.0,
            linear_sampler,
            cdlod: Cdlod::new(),
            settings,
            need_lut: true,
            env_full: 2,
            foam_first: true,
            foam_current: 0,
            foam_snap: [0; 4],
            sim_time: 0.0,
            frame_index: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn create_screen(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipelines: &Pipelines,
        linear_sampler: &wgpu::Sampler,
        bloom_ubos: &[wgpu::Buffer],
        composite_ubo: &wgpu::Buffer,
        width: u32,
        height: u32,
    ) -> ScreenTargets {
        use wgpu::{BindingResource as R, TextureUsages as TU, TextureViewDimension as D};
        let (w, h) = (width.max(1), height.max(1));
        let color_ms = texture(
            device,
            "ocean color msaa",
            [w, h, 1],
            HDR,
            1,
            MSAA,
            TU::RENDER_ATTACHMENT,
        );
        let depth_ms = texture(
            device,
            "ocean depth msaa",
            [w, h, 1],
            DEPTH,
            1,
            MSAA,
            TU::RENDER_ATTACHMENT,
        );
        let hdr = texture(
            device,
            "ocean hdr",
            [w, h, 1],
            HDR,
            1,
            1,
            TU::RENDER_ATTACHMENT | TU::TEXTURE_BINDING,
        );
        let (bw, bh) = ((w >> 1).max(1), (h >> 1).max(1));
        let bloom_levels = BLOOM_LEVELS.min(bw.max(bh).ilog2() + 1);
        let bloom = texture(
            device,
            "ocean bloom",
            [bw, bh, 1],
            HDR,
            bloom_levels,
            1,
            TU::RENDER_ATTACHMENT | TU::TEXTURE_BINDING,
        );
        let hdr_view = view(&hdr, None, D::D2);
        let bloom_views: Vec<_> = (0..bloom_levels)
            .map(|m| view(&bloom, Some(m), D::D2))
            .collect();
        let down_layout = pipelines.bloom_down.get_bind_group_layout(0);
        let up_layout = pipelines.bloom_up.get_bind_group_layout(0);
        let mut down = Vec::new();
        for m in 0..bloom_levels {
            let (sw, sh) = if m == 0 {
                (w, h)
            } else {
                ((bw >> (m - 1)).max(1), (bh >> (m - 1)).max(1))
            };
            let texel = [
                1.0 / sw as f32,
                1.0 / sh as f32,
                if m == 0 { 1.0 } else { 0.0 },
                0.0,
            ];
            let ubo = &bloom_ubos[m as usize];
            queue.write_buffer(ubo, 0, bytemuck::cast_slice(&texel));
            let src = if m == 0 {
                &hdr_view
            } else {
                &bloom_views[m as usize - 1]
            };
            down.push(bind_group(
                device,
                &down_layout,
                "ocean bloom down",
                &[
                    R::TextureView(src),
                    R::Sampler(linear_sampler),
                    ubo.as_entire_binding(),
                ],
            ));
        }
        let mut up = Vec::new();
        for m in (1..bloom_levels).rev() {
            let (sw, sh) = ((bw >> m).max(1), (bh >> m).max(1));
            let texel = [1.0 / sw as f32, 1.0 / sh as f32, 1.0, 1.0];
            let ubo = &bloom_ubos[(BLOOM_LEVELS + m) as usize];
            queue.write_buffer(ubo, 0, bytemuck::cast_slice(&texel));
            up.push(bind_group(
                device,
                &up_layout,
                "ocean bloom up",
                &[
                    R::TextureView(&bloom_views[m as usize]),
                    R::Sampler(linear_sampler),
                    ubo.as_entire_binding(),
                ],
            ));
        }
        let composite = bind_group(
            device,
            &pipelines.composite.get_bind_group_layout(0),
            "ocean composite",
            &[
                R::TextureView(&hdr_view),
                R::TextureView(&bloom_views[0]),
                R::Sampler(linear_sampler),
                composite_ubo.as_entire_binding(),
            ],
        );
        ScreenTargets {
            size: [w, h],
            color_ms: view(&color_ms, None, D::D2),
            depth_ms: view(&depth_ms, None, D::D2),
            hdr: hdr_view,
            bloom: bloom_views,
            down,
            up,
            composite,
        }
    }

    /// Recreates the screen-sized targets; a no-op when the size is unchanged.
    pub fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        if self.screen.size == [width.max(1), height.max(1)] {
            return;
        }
        self.screen = Self::create_screen(
            device,
            queue,
            &self.pipelines,
            &self.linear_sampler,
            &self.bloom_ubos,
            &self.composite_ubo,
            width,
            height,
        );
    }

    pub fn settings(&self) -> &OceanSettings {
        &self.settings
    }

    /// Applies new parameters; the spectrum, atmosphere LUTs and environment cube are
    /// rebuilt only when the parameters they depend on change.
    pub fn set_settings(&mut self, settings: OceanSettings) {
        if settings == self.settings {
            return;
        }
        self.waves.set_spectrum(settings.spectrum());
        self.need_lut |= settings.haze != self.settings.haze;
        self.env_full = 2;
        self.settings = settings;
    }

    /// Displaced water height under the camera, a few frames old.
    pub fn water_height(&self) -> f32 {
        self.water_height
    }

    /// Number of CDLOD nodes drawn last frame.
    pub fn node_count(&self) -> usize {
        self.cdlod.nodes.len()
    }

    fn view_projection(&self, camera: &OceanCamera) -> ([f32; 16], [glam::Vec3; 3], f32, f32) {
        let [w, h] = self.screen.size;
        let tan_y = (camera.fov_y * 0.5).tan();
        let tan_x = tan_y * w as f32 / h as f32;
        let near = 0.05;
        // camera-relative, reverse-Z, infinite far plane
        let mut proj = [0.0f32; 16];
        proj[0] = 1.0 / tan_x;
        proj[5] = 1.0 / tan_y;
        proj[11] = -1.0;
        proj[14] = near;
        let [f, r, u] = camera.basis();
        let view = [
            r.x, u.x, -f.x, 0.0, r.y, u.y, -f.y, 0.0, r.z, u.z, -f.z, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let view_proj = glam::Mat4::from_cols_array(&proj) * glam::Mat4::from_cols_array(&view);
        (view_proj.to_cols_array(), [f, r, u], tan_x, tan_y)
    }

    fn write_frame(
        &self,
        queue: &wgpu::Queue,
        camera: &OceanCamera,
        view_proj: &[f32; 16],
        basis: [glam::Vec3; 3],
        tan_x: f32,
        tan_y: f32,
    ) {
        let s = &self.settings;
        let [w, h] = self.screen.size;
        let [fw, r, u] = basis;
        let mut f = [0.0f32; 96];
        f[..16].copy_from_slice(view_proj);
        let p = camera.position;
        f[16..20].copy_from_slice(&[p.x, p.y, p.z, self.sim_time]);
        f[20..23].copy_from_slice(&(r * tan_x).to_array());
        f[24..27].copy_from_slice(&(u * tan_y).to_array());
        f[28..31].copy_from_slice(&fw.to_array());
        let (se, sa) = (s.sun_elevation.to_radians(), s.sun_azimuth.to_radians());
        f[32..36].copy_from_slice(&[sa.sin() * se.cos(), se.sin(), -sa.cos() * se.cos(), 14.0]);
        f[36..40].copy_from_slice(&[w as f32, h as f32, 1.0 / w as f32, 1.0 / h as f32]);
        for (c, l) in CASC_L.iter().enumerate() {
            f[40 + c] = *l;
            f[48 + c] = 1.0 / l;
        }
        let wd = s.wind_direction.to_radians();
        f[56..59].copy_from_slice(&[wd.cos(), wd.sin(), s.wind_speed]);
        f[64..68].copy_from_slice(&[
            R0 as f32,
            GRID as f32,
            FOAM_LEVEL_SIZE[0],
            FOAM_LEVEL_SIZE[1],
        ]);
        f[68..72].copy_from_slice(&[s.exposure, s.bloom, self.frame_index as f32, 0.0]);
        f[72..76].copy_from_slice(&[s.haze, p.y.max(10.0) * 0.001, s.cloud_deck, 0.9]);
        // deep water body scatter colour (greenish coastal water), unresolved slope variance
        let t = s.turbidity;
        f[76..80].copy_from_slice(&[0.016 * t, 0.080 * t, 0.096 * t, 0.0024]);
        f[80..84].copy_from_slice(&[0.035, 0.120, 0.085, 2.6]);
        f[84..88].copy_from_slice(&[0.36 + 0.12 * s.foam, 2.5, 1.0, 1.0]);
        let mie = 4.40e-3 * s.haze;
        f[88..92].copy_from_slice(&[
            (5.802e-3 + mie) * 1e-3,
            (13.558e-3 + mie) * 1e-3,
            (33.1e-3 + mie) * 1e-3,
            1.1e-4,
        ]);
        f[92..96].copy_from_slice(&[1.5, s.clouds, 2.2, self.sim_time]);
        queue.write_buffer(&self.frame_ubo, 0, bytemuck::cast_slice(&f));
    }

    fn write_foam(&mut self, queue: &wgpu::Queue, camera: &OceanCamera, dt: f32) {
        let s = &self.settings;
        let mut u = [0u32; 36];
        for l in 0..2 {
            let ts = FOAM_LEVEL_SIZE[l] / FOAM_RES as f32;
            let cx = (camera.position.x / ts).floor() as i32;
            let cz = (camera.position.z / ts).floor() as i32;
            // first frame: pretend the previous window was far away so everything is reset
            let (px, pz) = if self.foam_first {
                (cx + 100_000, cz + 100_000)
            } else {
                (self.foam_snap[l * 2], self.foam_snap[l * 2 + 1])
            };
            u[l * 4..l * 4 + 4].copy_from_slice(&[cx, cz, px, pz].map(|v| v as u32));
            self.foam_snap[l * 2] = cx;
            self.foam_snap[l * 2 + 1] = cz;
            u[8 + l] = ts.to_bits();
        }
        self.foam_first = false;
        let wd = s.wind_direction.to_radians();
        let drift = 0.03 * s.wind_speed;
        let floats = [
            (10, dt),
            (11, 0.22), // decay 1/s
            (12, 0.52 + 0.16 * s.foam),
            (13, 2.0),
            (14, 1.0),
            (32, wd.cos() * drift),
            (33, wd.sin() * drift),
        ];
        for (i, v) in floats {
            u[i] = f32::to_bits(v);
        }
        for (c, l) in CASC_L.iter().enumerate() {
            u[16 + c] = l.to_bits();
            u[24 + c] = (1.0 / l).to_bits();
        }
        queue.write_buffer(&self.foam_ubo, 0, bytemuck::cast_slice(&u));
    }

    fn poll_probe(&mut self) {
        if !self.probe_pending {
            return;
        }
        match self.probe_state.swap(0, Ordering::AcqRel) {
            1 => {
                if let Ok(range) = self.probe_read.get_mapped_range(..) {
                    self.water_height = bytemuck::pod_read_unaligned::<f32>(&range[..4]);
                }
                self.probe_read.unmap();
                self.probe_pending = false;
            }
            2 => self.probe_pending = false,
            _ => {}
        }
    }

    /// Advances the simulation by `dt` seconds and encodes the whole frame, ending with
    /// the tonemapped composite into `output` (the format given to [`Self::new`]).
    /// Call `device.poll` now and then (e.g. once per frame) so the height probe resolves.
    pub fn encode(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        camera: &OceanCamera,
        dt: f32,
    ) {
        self.poll_probe();
        self.sim_time += dt;
        if self.sim_time > LOOP {
            self.sim_time -= LOOP;
        }
        self.frame_index = self.frame_index.wrapping_add(1);
        let (view_proj, basis, tan_x, tan_y) = self.view_projection(camera);
        self.write_frame(queue, camera, &view_proj, basis, tan_x, tan_y);
        self.write_foam(queue, camera, dt);
        let p = camera.position;
        self.cdlod
            .select([f64::from(p.x), f64::from(p.y), f64::from(p.z)], &view_proj);
        let nodes = self.cdlod.nodes.len() as u32;
        if nodes > 0 {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.cdlod.nodes),
            );
        }

        let (pl, g) = (&self.pipelines, &self.groups);
        {
            let mut cp = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ocean simulation and sky"),
                timestamp_writes: None,
            });
            if self.need_lut {
                queue.write_buffer(
                    &self.atmo_ubo,
                    0,
                    bytemuck::cast_slice(&[self.settings.haze, 0.0, 0.0, 0.0]),
                );
                cp.set_pipeline(&pl.transmittance);
                cp.set_bind_group(0, &g.transmittance, &[]);
                cp.dispatch_workgroups(256 / 8, 64 / 8, 1);
                cp.set_pipeline(&pl.multiscatter);
                cp.set_bind_group(0, &g.multiscatter, &[]);
                cp.dispatch_workgroups(32 / 8, 32 / 8, 1);
                self.need_lut = false;
            }
            self.waves.encode(queue, &mut cp, self.sim_time, dt);
            // foam clipmaps
            self.foam_current = 1 - self.foam_current;
            cp.set_pipeline(&pl.foam_update);
            cp.set_bind_group(0, &g.foam_update[self.foam_current], &[]);
            cp.dispatch_workgroups(FOAM_RES / 8, FOAM_RES / 8, 2);
            cp.set_pipeline(&pl.array16_mips);
            for (m, group) in (1..FOAM_WORLD_MIPS).zip(&g.foam_mips[self.foam_current]) {
                let s = (FOAM_RES >> m).max(1);
                cp.set_bind_group(0, group, &[]);
                cp.dispatch_workgroups(groups_for(s), groups_for(s), 2);
            }
            cp.set_pipeline(&pl.probe);
            cp.set_bind_group(0, &g.probe, &[]);
            cp.dispatch_workgroups(1, 1, 1);
            cp.set_pipeline(&pl.sky_view);
            cp.set_bind_group(0, &g.sky_view, &[]);
            cp.dispatch_workgroups(groups_for(SKY_W), groups_for(SKY_H), 1);
            cp.set_pipeline(&pl.sun_visibility);
            cp.set_bind_group(0, &g.sun_visibility, &[]);
            cp.dispatch_workgroups(1, 1, 1);
            // one cube face per frame (all six on start-up / after parameter changes)
            cp.set_pipeline(&pl.env);
            cp.set_bind_group(0, &g.env, &[]);
            cp.dispatch_workgroups(
                ENV_SIZE / 8,
                ENV_SIZE / 8,
                if self.env_full > 0 { 6 } else { 1 },
            );
            self.env_full = self.env_full.saturating_sub(1);
            cp.set_pipeline(&pl.cube_mips);
            for (m, group) in (1..ENV_MIPS).zip(&g.cube_mips) {
                let s = (ENV_SIZE >> m).max(1);
                cp.set_bind_group(0, group, &[]);
                cp.dispatch_workgroups(groups_for(s), groups_for(s), 6);
            }
        }
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ocean main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.screen.color_ms,
                    depth_slice: None,
                    resolve_target: Some(&self.screen.hdr),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.screen.depth_ms,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            if nodes > 0 {
                rp.set_pipeline(&pl.ocean);
                rp.set_bind_group(0, &g.ocean[self.foam_current], &[]);
                rp.set_vertex_buffer(0, self.instance_buffer.slice(..));
                rp.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                rp.draw_indexed(0..GRID * GRID * 6, 0, 0..nodes);
            }
            rp.set_pipeline(&pl.sky);
            rp.set_bind_group(0, &g.sky, &[]);
            rp.draw(0..3, 0..1);
        }
        let fullscreen = |encoder: &mut wgpu::CommandEncoder,
                          label: &str,
                          target: &wgpu::TextureView,
                          load: wgpu::LoadOp<wgpu::Color>,
                          pipeline: &wgpu::RenderPipeline,
                          group: &wgpu::BindGroup| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
        for (target, group) in self.screen.bloom.iter().zip(&self.screen.down) {
            fullscreen(
                encoder,
                "ocean bloom down",
                target,
                clear,
                &pl.bloom_down,
                group,
            );
        }
        let levels = self.screen.bloom.len();
        for (i, group) in self.screen.up.iter().enumerate() {
            let target = &self.screen.bloom[levels - 2 - i];
            fullscreen(
                encoder,
                "ocean bloom up",
                target,
                wgpu::LoadOp::Load,
                &pl.bloom_up,
                group,
            );
        }
        let s = &self.settings;
        let composite = [
            s.exposure,
            s.bloom,
            (self.frame_index % 64) as f32,
            0.10,
            1.05,
            1.06,
            -0.19,
            if self.output_srgb { 1.0 } else { 0.0 },
        ];
        queue.write_buffer(&self.composite_ubo, 0, bytemuck::cast_slice(&composite));
        fullscreen(
            encoder,
            "ocean composite",
            output,
            clear,
            &pl.composite,
            &self.screen.composite,
        );

        if !self.probe_pending && self.frame_index.is_multiple_of(8) {
            encoder.copy_buffer_to_buffer(&self.probe_out, 0, &self.probe_read, 0, 16);
            let state = Arc::clone(&self.probe_state);
            encoder.map_buffer_on_submit(
                &self.probe_read,
                wgpu::MapMode::Read,
                ..,
                move |result| {
                    state.store(if result.is_ok() { 1 } else { 2 }, Ordering::Release);
                },
            );
            self.probe_pending = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use wgpu::naga::{
        front::wgsl,
        valid::{Capabilities, ValidationFlags, Validator},
    };

    /// Every assembled pipeline source parses and validates with the renderer's Naga.
    #[test]
    fn assembled_shaders_validate() {
        let water = crate::archive_water::source();
        let sources = [
            ("transmittance", super::shaders::TRANSMITTANCE),
            ("multiscatter", super::shaders::MULTISCATTER),
            ("sky view", super::shaders::SKY_VIEW),
            ("env cube", super::shaders::ENV_CUBE),
            ("sun visibility", super::shaders::SUN_VISIBILITY),
            ("height probe", super::shaders::HEIGHT_PROBE),
            ("surface", super::shaders::SURFACE),
            ("sky", super::shaders::SKY),
            ("bloom down", super::shaders::BLOOM_DOWN),
            ("bloom up", super::shaders::BLOOM_UP),
            ("composite", super::shaders::COMPOSITE),
            ("foam texture", super::shaders::FOAM_TEXTURE),
            ("foam update", include_str!("foam_update.wgsl")),
            ("foam mips", include_str!("array16_mips.wgsl")),
            ("foam texture mips", include_str!("rgba8_mips.wgsl")),
            ("cube mips", include_str!("cube_mips.wgsl")),
            ("archive water", water.as_str()),
        ];
        for (label, source) in sources {
            let module = wgsl::parse_str(source)
                .unwrap_or_else(|e| panic!("{label}: {}", e.emit_to_string(source)));
            Validator::new(ValidationFlags::all(), Capabilities::all())
                .validate(&module)
                .unwrap_or_else(|e| panic!("{label}: {}", e.emit_to_string(source)));
            assert!(
                !module.entry_points.is_empty(),
                "{label} has no entry points"
            );
        }
    }
}
