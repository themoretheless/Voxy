//! Render-ready immutable imported geometry; authoring and physics remain separate owners.
#[path = "archive_refine_crossover.rs"]
mod refine_crossover;
pub use refine_crossover::RefineCrossover;
#[path = "archive_compute_gi.rs"]
mod compute_gi;
mod draw;
mod probe_timing;
mod settings;
mod types;
mod world_rays;

pub use compute_gi::restir_jacobian_shift_det;
pub use probe_timing::ArchiveProbeBake;
pub use types::*;

use crate::{Bc7ImageAsset, ComputeMemoryBudget, ComputeStorage};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use probe_timing::Timing;
use std::sync::Arc;
use types::{archive_light_z_masks, display_mode, fully_covering_transmission};
use world_rays::validate_street_ray_env_config;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Camera {
    vp: [[f32; 4]; 4],
    shadow: [[f32; 4]; 4],
    inverse_vp: [[f32; 4]; 4],
    eye: [f32; 4],
    sun: [f32; 4],
    sun_colour: [f32; 4],
    viewport: [f32; 4],
    weather: [f32; 4],
    atmosphere: [f32; 4],
    wind: [f32; 4],
    /// Jittered view-projection of the frame whose opaque radiance is in `background`.
    previous_vp: [[f32; 4]; 4],
    previous_inverse_vp: [[f32; 4]; 4],
    /// [surface wetness, puddle coverage, history valid, vertical pixel angular size].
    surface: [f32; 4],
}

#[derive(Debug)]
struct Material {
    bind: wgpu::BindGroup,
    _uniform: ComputeStorage,
    transparent: bool,
    fully_covering: bool,
    history_guides: bool,
    double_sided: bool,
    masked: bool,
}

#[derive(Debug)]
struct Mesh {
    culling: Option<crate::ArchiveInstanceCulling>,
    frustum: crate::archive_frustum::ArchiveFrustumGroups,
    scratch_ids: Vec<usize>,
    scratch_instances: Vec<ArchiveInstance>,
    vertices: ComputeStorage,
    indices: ComputeStorage,
    instances: ComputeStorage,
    visible_instances: ComputeStorage,
    source_instances: Vec<ArchiveInstance>,
    spheres: Vec<(Vec3, f32)>,
    visible_ids: Vec<usize>,
    visible_count: u32,
    count: u32,
    copies: u32,
    material: usize,
    center: Vec3,
}

#[derive(Debug)]
pub struct ArchiveSceneRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    meshes: Vec<Mesh>,
    draw_order: Vec<usize>,
    materials: Vec<Material>,
    _textures: Vec<Arc<voxy_gpu_core::ManagedTexture>>,
    _lights: ComputeStorage,
    camera: Arc<ComputeStorage>,
    camera_bind: wgpu::BindGroup,
    opaque: wgpu::RenderPipeline,
    opaque_double: wgpu::RenderPipeline,
    masked: wgpu::RenderPipeline,
    masked_double: wgpu::RenderPipeline,
    transparent: wgpu::RenderPipeline,
    transparent_double: wgpu::RenderPipeline,
    covered_transparent: wgpu::RenderPipeline,
    covered_transparent_double: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    depth_pipeline: wgpu::RenderPipeline,
    depth_double: wgpu::RenderPipeline,
    timing: Option<Timing>,
    refine_alternates: Option<[wgpu::RenderPipeline; 8]>,
    refine_crossover: Option<RefineCrossover>,
    refine_cache_base: bool,
    gi_stage_crossover: bool,
    benchmark_frame: u64,
    /// Timestamp producer identity, not its later CPU collection frame.
    /// Each tuple is (producer ordinal, assigned label B, candidate enabled).
    pub gpu_frame_stamps: Vec<(u64, bool, bool)>,
    pub gpu_times_ms: Vec<f64>,
    /// [opaque shading, transparent shading] GPU intervals, excluding presentation.
    pub gpu_shading_times_ms: Vec<[f64; 2]>,
    /// Optional [full-float32 guide rasterization, opaque GI compute] intervals.
    pub gpu_gi_times_ms: Vec<Option<[f64; 2]>>,
    /// Optional exact opaque reflection compute interval.
    pub gpu_ssr_times_ms: Vec<Option<f64>>,
    ray_queue_frame_count: u64,
    exposure: f32,
    exposure_bias_ev: f32,
    adapted_exposure_scale: f32,
    sky_strength: f32,
    _environment: crate::ImportedEnvironment,
    environment_bindings: crate::environment_lighting::EnvironmentBindings,
    environment_enabled: bool,
    _sky: Arc<voxy_gpu_core::ManagedTexture>,
    post: wgpu::RenderPipeline,
    post_linear: wgpu::RenderPipeline,
    output_format: wgpu::TextureFormat,
    antialiasing: crate::ArchiveAntialiasing,
    temporal: Option<crate::archive_temporal::ArchiveTemporal>,
    post_bind: wgpu::BindGroup,
    bloom: crate::bloom::Bloom,
    display_parameters: ComputeStorage,
    hdr_output: bool,
    linear_output: bool,
    vertical_fov: f32,
    camera_up: Vec3,
    tone_mapper: ArchiveToneMapper,
    camera_lens: crate::archive_temporal::ArchiveCameraLens,
    display_headroom: f32,
    spatial_antialiasing: bool,
    frame_bind: wgpu::BindGroup,
    probe_sh: ComputeStorage,
    probe_bake_owner: Arc<()>,
    probe_grid: ComputeStorage,
    hdr: Arc<voxy_gpu_core::ManagedTexture>,
    hdr_view: wgpu::TextureView,
    background: Arc<voxy_gpu_core::ManagedTexture>,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    transmission_depth: Option<wgpu::Texture>,
    transmission_depth_view: Option<wgpu::TextureView>,
    history_depth: Arc<voxy_gpu_core::ManagedTexture>,
    depth_bounds: crate::archive_depth_bounds::ArchiveDepthBounds,
    pixel_depth_bounds: Option<crate::archive_depth_bounds::ArchiveDepthBounds>,
    compute_gi: Option<compute_gi::ArchiveComputeGi>,
    _gi_output: Arc<voxy_gpu_core::ManagedTexture>,
    _ssr_output: Arc<voxy_gpu_core::ManagedTexture>,
    shadow: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    shadow_matrix: Mat4,
    weather: crate::weather::Weather,
    weather_time: f32,
    surface_wetness: Option<f32>,
    adapted_wetness: f32,
    puddle_coverage: f32,
    previous_vp: Option<Mat4>,
    shadow_bounds: (Vec3, Vec3),
    sun: Vec3,
    sun_colour: [f32; 4],
    width: u32,
    height: u32,
    shadow_ready: bool,
    camera_layout: wgpu::BindGroupLayout,
    water: Option<crate::archive_water::ArchiveWater>,
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    data: &[u8],
    usage: wgpu::BufferUsages,
) -> Result<ComputeStorage, crate::ComputeError> {
    let [b] = ComputeMemoryBudget::for_device(device).allocate_buffers([
        voxy_gpu_core::ManagedBufferDescriptor {
            label,
            size: data.len() as u64,
            contents: Some(data),
            usage,
        },
    ])?;
    Ok(b)
}

fn target(
    device: &wgpu::Device,
    w: u32,
    h: u32,
) -> Result<Arc<voxy_gpu_core::ManagedTexture>, crate::ComputeError> {
    ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
        label: Some("imported scene linear HDR"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn depth(device: &wgpu::Device, w: u32, h: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("imported scene depth"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

impl ArchiveSceneRenderer {
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        images: Vec<Bc7ImageAsset>,
        sources: Vec<ArchiveMaterialSource>,
        geometry: Vec<ArchiveMeshSource>,
        lights: &[ArchiveLight],
        sun: Vec3,
        sun_colour: [f32; 4],
        bounds: (Vec3, Vec3),
    ) -> Result<Self, crate::ComputeError> {
        Self::new_with_images(
            device,
            queue,
            format,
            width,
            height,
            images.into_iter().map(ArchiveImageSource::Bc7).collect(),
            sources,
            geometry,
            lights,
            sun,
            sun_colour,
            bounds,
        )
    }

    /// Render imported or generated RGBA images with the same material/shadow pipeline.
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn new_with_images(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        images: Vec<ArchiveImageSource>,
        sources: Vec<ArchiveMaterialSource>,
        geometry: Vec<ArchiveMeshSource>,
        lights: &[ArchiveLight],
        sun: Vec3,
        sun_colour: [f32; 4],
        bounds: (Vec3, Vec3),
    ) -> Result<Self, crate::ComputeError> {
        Self::new_with_environment(
            device, queue, format, width, height, images, sources, geometry, lights, sun,
            sun_colour, bounds, None,
        )
    }

    #[expect(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn new_with_environment(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        images: Vec<ArchiveImageSource>,
        sources: Vec<ArchiveMaterialSource>,
        geometry: Vec<ArchiveMeshSource>,
        lights: &[ArchiveLight],
        sun: Vec3,
        sun_colour: [f32; 4],
        bounds: (Vec3, Vec3),
        environment: Option<ArchiveEnvironmentSource>,
    ) -> Result<Self, crate::ComputeError> {
        validate_street_ray_env_config(device)?;
        let init_started = std::time::Instant::now();
        let init_trace = |stage: &str| {
            if std::env::var_os("VOXY_STREET_INIT_TRACE").is_some() {
                eprintln!(
                    "ARCHIVE INIT stage={stage} elapsed_ms={:.3}",
                    init_started.elapsed().as_secs_f64() * 1000.
                );
            }
        };
        init_trace("environment_begin");
        let environment_enabled = environment.is_some();
        let (panorama, background_panorama, intensity) = match environment {
            Some(source) => (source.lighting, Some(source.background), source.intensity),
            None => (
                crate::HdrImageAsset::from_rgb(2, 1, vec![0.; 6], crate::ImageLimits::default())
                    .map_err(|e| crate::ComputeError::Validation(e.to_string()))?,
                None,
                1.,
            ),
        };
        if !intensity.is_finite() || intensity <= 0. {
            return Err(crate::ComputeError::Validation(
                "invalid environment intensity".into(),
            ));
        }
        let sky = if let Some(panorama) = background_panorama {
            ArchiveImageSource::Float {
                width: panorama.width(),
                height: panorama.height(),
                pixels: panorama
                    .rgb()
                    .chunks_exact(3)
                    .flat_map(|p| [p[0], p[1], p[2], 1.])
                    .collect(),
            }
            .upload(device, queue)?
        } else {
            ArchiveImageSource::Rgba(
                crate::ImageAsset::from_rgba(
                    1,
                    1,
                    vec![0, 0, 0, 255],
                    crate::ImageLimits::default(),
                )
                .map_err(|e| crate::ComputeError::Validation(e.to_string()))?,
            )
            .upload(device, queue)?
        };
        let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let environment = if environment_enabled {
            crate::ImportedEnvironment::from_panorama_full_precision(
                device,
                queue,
                &panorama,
                1024,
                crate::ImageLimits {
                    pixel_bytes: 512 * 1024 * 1024,
                    ..Default::default()
                },
            )
        } else {
            crate::ImportedEnvironment::from_panorama(
                device,
                queue,
                &panorama,
                1,
                crate::ImageLimits::default(),
            )
        }
        .map_err(|e| crate::ComputeError::Validation(e.to_string()))?;
        init_trace("environment_resources_ready");
        let environment_bindings = crate::environment_lighting::EnvironmentBindings::new(
            device,
            environment.specular(),
            environment.dfg(),
            Some(environment.diffuse()),
            intensity,
            None,
        )
        .map_err(|e| crate::ComputeError::Validation(e.to_string()))?;
        let mut preparation = device.create_command_encoder(&Default::default());
        environment.encode(&mut preparation);
        queue.submit([preparation.finish()]);
        let frag = wgpu::ShaderStages::FRAGMENT;
        let unfilterable_float = wgpu::TextureSampleType::Float { filterable: false };
        let filterable_float = wgpu::TextureSampleType::Float { filterable: true };
        let depth_sample = wgpu::TextureSampleType::Depth;
        let camera_layout = voxy_gpu_core::BindGroupLayoutBuilder::with_capacity(
            Some("archive camera and lights"),
            2,
        )
        .uniform_buffer(0, wgpu::ShaderStages::VERTEX_FRAGMENT)
        .uniform_buffer(1, frag)
        .build(device);
        let mut material_builder = voxy_gpu_core::BindGroupLayoutBuilder::with_capacity(
            Some("authored four-map PBR material"),
            6,
        )
        .uniform_buffer(0, frag);
        for i in 1..=4 {
            material_builder = material_builder.texture_2d(i, frag, filterable_float);
        }
        let material_layout = material_builder
            .sampler(5, frag, wgpu::SamplerBindingType::Filtering)
            .build(device);
        let frame_layout = voxy_gpu_core::BindGroupLayoutBuilder::with_capacity(
            Some("opaque transmission and sun shadows"),
            10,
        )
        .texture_2d(0, frag, filterable_float)
        .texture_2d(9, frag, unfilterable_float)
        .texture_2d(8, frag, unfilterable_float)
        .texture_2d(1, frag, depth_sample)
        .texture_2d(3, frag, depth_sample)
        .texture_2d(4, frag, depth_sample)
        .texture_2d(7, frag, unfilterable_float)
        .storage_buffer(5, frag, true)
        .uniform_buffer(6, frag)
        .sampler(2, frag, wgpu::SamplerBindingType::Comparison)
        .build(device);
        let post_layout =
            voxy_gpu_core::BindGroupLayoutBuilder::with_capacity(Some("archive HDR display"), 9)
                .texture_2d(0, frag, filterable_float)
                .uniform_buffer(1, frag)
                .texture_2d(2, frag, depth_sample)
                .uniform_buffer(3, frag)
                .texture_2d(4, frag, filterable_float)
                .sampler(5, frag, wgpu::SamplerBindingType::Filtering)
                .texture_2d(6, frag, filterable_float)
                .texture_2d(7, frag, depth_sample)
                .sampler(8, frag, wgpu::SamplerBindingType::Comparison)
                .build(device);
        init_trace("scene_shader_begin");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("archive PBR"),
            source: wgpu::ShaderSource::Wgsl(archive_shader_source().into()),
        });
        init_trace("scene_shader_ready");
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("archive scene layout"),
            bind_group_layouts: &[
                Some(&camera_layout),
                Some(&material_layout),
                Some(&frame_layout),
                Some(&environment_bindings.layout),
            ],
            immediate_size: 0,
        });
        let vertex_attrs =
            wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,2=>Float32x4,3=>Float32x2];
        let instance_attrs = wgpu::vertex_attr_array![4=>Float32x4,5=>Float32x4,6=>Float32x4,7=>Float32x4,8=>Float32x4,9=>Float32x4,10=>Float32x4];
        let vertex_buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: 48,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attrs,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: 112,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance_attrs,
            }),
        ];
        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&camera_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let refine_crossover = std::env::var("VOXY_STREET_REFINE_CROSSOVER")
            .ok()
            .map(|s| RefineCrossover::parse(&s))
            .transpose()?;
        let refine_cache_base = std::env::var_os("VOXY_STREET_REFINE_CACHE").is_some();
        let gi_stage_crossover = std::env::var_os("VOXY_STREET_GI_STAGE_CROSSOVER").is_some();
        let compute_requested = std::env::var_os("VOXY_STREET_COMPUTE_GI").is_some();
        if gi_stage_crossover && (refine_crossover.is_none() || !compute_requested) {
            return Err(crate::ComputeError::Validation(
                "GI stage crossover requires a schedule and compute GI".into(),
            ));
        }
        if refine_crossover.is_some()
            && (refine_cache_base
                || (compute_requested && !gi_stage_crossover)
                || [
                    "VOXY_STREET_PAIRED_DDA",
                    "VOXY_STREET_RAY_QUEUE",
                    "VOXY_STREET_RAY_HIERARCHY",
                    "VOXY_STREET_RAY_STAGES",
                    "VOXY_STREET_COMPUTE_SSR",
                ]
                .iter()
                .any(|key| std::env::var_os(key).is_some()))
        {
            return Err(crate::ComputeError::Validation(
                "crossover requires the original GI algorithm without other ray experiments".into(),
            ));
        }
        let compute_variant = std::cell::Cell::new(compute_requested && !gi_stage_crossover);
        let refine_variant = std::cell::Cell::new(refine_cache_base);
        let pipeline = |blend: bool,
                        double: bool,
                        shadow: bool,
                        predepth: bool,
                        masked: bool,
                        covered: bool| {
            let pipeline_started = std::time::Instant::now();
            if std::env::var_os("VOXY_STREET_INIT_TRACE").is_some() {
                eprintln!(
                    "ARCHIVE INIT pipeline_begin blend={blend} double={double} shadow={shadow} predepth={predepth} masked={masked}"
                );
            }
            let targets = [Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: if blend {
                    Some(wgpu::BlendState::ALPHA_BLENDING)
                } else {
                    None
                },
                write_mask: wgpu::ColorWrites::ALL,
            })];
            let result = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("archive instanced material pipeline"),
                layout: Some(if shadow || predepth {
                    &shadow_layout
                } else {
                    &layout
                }),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(if shadow { "vs_shadow" } else { "vs_main" }),
                    compilation_options: Default::default(),
                    buffers: &vertex_buffers,
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: if double { None } else { Some(wgpu::Face::Back) },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(shadow || predepth),
                    depth_compare: Some(if !shadow && !predepth && (!blend || covered) {
                        wgpu::CompareFunction::Equal
                    } else {
                        wgpu::CompareFunction::LessEqual
                    }),
                    stencil: Default::default(),
                    bias: if shadow {
                        wgpu::DepthBiasState {
                            constant: 2,
                            slope_scale: 2.,
                            clamp: 0.,
                        }
                    } else {
                        Default::default()
                    },
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if shadow || predepth {
                        "fs_shadow"
                    } else if masked {
                        "fs_main"
                    } else {
                        "fs_opaque"
                    }),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[
                            (
                                "ARCHIVE_DDA_INTERVAL_CACHE",
                                if std::env::var_os("VOXY_STREET_DDA_INTERVAL_CACHE").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_RAY_FORCE_FALLBACK",
                                if std::env::var_os("VOXY_STREET_RAY_FORCE_FALLBACK").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_DDA_PLANE_CACHE",
                                if std::env::var_os("VOXY_STREET_DDA_PLANE_CACHE").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_DDA_POINT_CACHE",
                                if std::env::var_os("VOXY_STREET_DDA_POINT_CACHE").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_COMPUTE_SSR",
                                if !blend
                                    && !shadow
                                    && !predepth
                                    && std::env::var_os("VOXY_STREET_COMPUTE_GI").is_some()
                                    && std::env::var_os("VOXY_STREET_COMPUTE_SSR").is_some()
                                {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_COMPUTE_GI",
                                if !blend && !shadow && !predepth && compute_variant.get() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_COMPUTE_GI_EXACT_SAMPLE",
                                f64::from(u8::from(gi_stage_crossover)),
                            ),
                            (
                                "ARCHIVE_EARLY_GI",
                                if std::env::var_os("VOXY_STREET_EARLY_GI").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_DDA_TILE_EDGE_CACHE",
                                if std::env::var("VOXY_STREET_DDA_TILE_EDGE_CACHE").as_deref()
                                    == Ok("1")
                                {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_DDA_EDGE_CACHE",
                                if std::env::var_os("VOXY_STREET_DDA_EDGE_CACHE").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_RAY_BOUNDS",
                                if std::env::var_os("VOXY_STREET_RAY_REFERENCE").is_some() {
                                    0.
                                } else {
                                    1.
                                },
                            ),
                            (
                                "ARCHIVE_DDA_HIERARCHY",
                                if std::env::var_os("VOXY_STREET_DDA_HIERARCHY").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_DDA_FAR_BOUND",
                                if std::env::var_os("VOXY_STREET_DDA_FAR_BOUND").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_DDA_CELL_BOUNDS",
                                if std::env::var_os("VOXY_STREET_DDA_CELL_BOUNDS").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_DDA_TILES",
                                if std::env::var_os("VOXY_STREET_PAIRED_DDA_TILES").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_PAIRED_DDA",
                                if std::env::var_os("VOXY_STREET_PAIRED_DDA").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "WGPU_METAL_PRECISE_MATH",
                                if std::env::var_os("VOXY_STREET_PAIRED_DDA").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_LIGHT_Z_MASKS",
                                if std::env::var_os("VOXY_STREET_LIGHT_Z_MASKS").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_RAY_DEPTH_REUSE",
                                if std::env::var_os("VOXY_STREET_RAY_DEPTH_REUSE").is_some() {
                                    1.
                                } else {
                                    0.
                                },
                            ),
                            (
                                "ARCHIVE_REFINE_CACHE",
                                if refine_variant.get() { 1. } else { 0. },
                            ),
                        ],
                        ..Default::default()
                    },
                    targets: if shadow || predepth { &[] } else { &targets },
                }),
                multiview_mask: None,
                cache: None,
            });
            if std::env::var_os("VOXY_STREET_INIT_TRACE").is_some() {
                eprintln!(
                    "ARCHIVE INIT pipeline_ready blend={blend} double={double} shadow={shadow} predepth={predepth} masked={masked} milliseconds={:.3}",
                    pipeline_started.elapsed().as_secs_f64() * 1000.
                );
            }
            result
        };
        let opaque = pipeline(false, false, false, false, false, false);
        let opaque_double = pipeline(false, true, false, false, false, false);
        let masked = pipeline(false, false, false, false, true, false);
        let masked_double = pipeline(false, true, false, false, true, false);
        let transparent = pipeline(true, false, false, false, false, false);
        let transparent_double = pipeline(true, true, false, false, false, false);
        let covered_transparent = pipeline(true, false, false, false, false, true);
        let covered_transparent_double = pipeline(true, true, false, false, false, true);
        let shadow_pipeline = pipeline(false, true, true, false, false, false);
        let depth_pipeline = pipeline(false, false, false, true, false, false);
        let depth_double = pipeline(false, true, false, true, false, false);

        let refine_alternates = refine_crossover.map(|mode| {
            refine_variant.set(mode.cache_b() && !gi_stage_crossover);
            compute_variant.set(if gi_stage_crossover {
                mode.cache_b()
            } else {
                compute_requested
            });
            let result = [
                pipeline(false, false, false, false, false, false),
                pipeline(false, false, false, false, true, false),
                pipeline(false, true, false, false, false, false),
                pipeline(false, true, false, false, true, false),
                pipeline(true, false, false, false, false, false),
                pipeline(true, true, false, false, false, false),
                pipeline(true, false, false, false, false, true),
                pipeline(true, true, false, false, false, true),
            ];
            refine_variant.set(refine_cache_base);
            compute_variant.set(compute_requested && !gi_stage_crossover);
            result
        });
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("archive display transform"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("display_transform.wgsl"),
                    include_str!("archive_environment.wgsl"),
                    include_str!("archive_post.wgsl")
                )
                .into(),
            ),
        });
        let post_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&post_layout), Some(&environment_bindings.layout)],
            immediate_size: 0,
        });
        let create_post = |format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("archive HDR display pass"),
                layout: Some(&post_pl),
                vertex: wgpu::VertexState {
                    module: &post_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &post_shader,
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
            })
        };
        let post = create_post(format);
        let post_linear = create_post(wgpu::TextureFormat::Rgba16Float);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("authored scene anisotropic sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        init_trace("material_upload_begin");
        let mut textures = Vec::new();
        for image in images {
            textures.push(image.upload(device, queue)?);
            init_trace("material_upload_next");
        }
        init_trace("material_upload_ready");
        // Absent glTF maps have specified neutral values; these are not replacements for source images.
        let mut defaults = Vec::new();
        for colour in [[255, 255, 255, 255], [128, 128, 255, 255]] {
            let t = ComputeMemoryBudget::for_device(device).allocate_texture(
                &wgpu::TextureDescriptor {
                    label: Some("absent glTF map identity"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
            )?;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &t,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &colour,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            defaults.push(t);
        }
        let mut materials = Vec::new();
        for source in sources {
            let uniform = buffer(
                device,
                "authored PBR factors",
                bytemuck::bytes_of(&source.parameters),
                wgpu::BufferUsages::UNIFORM,
            )?;
            let views: Vec<_> = source
                .images
                .iter()
                .enumerate()
                .map(|(slot, index)| {
                    index
                        .map_or(&defaults[usize::from(slot == 2)], |i| &textures[i])
                        .create_view(&wgpu::TextureViewDescriptor {
                            format: index.map(|i| {
                                if slot == 0
                                    || slot == 3
                                    || (slot == 1 && source.parameters.workflow[0] < 0.5)
                                {
                                    textures[i].format().add_srgb_suffix()
                                } else {
                                    textures[i].format().remove_srgb_suffix()
                                }
                            }),
                            ..Default::default()
                        })
                })
                .collect();
            let mut entries = vec![wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }];
            for (i, view) in views.iter().enumerate() {
                entries.push(wgpu::BindGroupEntry {
                    binding: i as u32 + 1,
                    resource: wgpu::BindingResource::TextureView(view),
                });
            }
            entries.push(wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Sampler(&sampler),
            });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("authored material maps"),
                layout: &material_layout,
                entries: &entries,
            });
            materials.push(Material {
                bind,
                _uniform: uniform,
                transparent: source.parameters.surface[2] > 0.
                    || source.parameters.surface[3] == -2.,
                fully_covering: fully_covering_transmission(&source),
                history_guides: source.parameters.surface[2] == 0.,
                double_sided: source.double_sided,
                masked: source.parameters.surface[3] >= 0.,
            });
        }
        textures.extend(defaults);
        let camera = Arc::new(buffer(
            device,
            "archive frame camera",
            bytemuck::bytes_of(&Camera::zeroed()),
            wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | if std::env::var_os("VOXY_STREET_RAY_CAPTURE_DIR").is_some() {
                    wgpu::BufferUsages::COPY_SRC
                } else {
                    wgpu::BufferUsages::empty()
                },
        )?);
        if lights.len() > 128 {
            return Err(crate::ComputeError::Validation(
                "archive light uniform capacity exceeded; no lights dropped".into(),
            ));
        }
        let mut light_data = vec![0u8; 16 + 128 * std::mem::size_of::<ArchiveLight>()];
        light_data[..4].copy_from_slice(&(lights.len() as u32).to_le_bytes());
        light_data[16..16 + std::mem::size_of_val(lights)]
            .copy_from_slice(bytemuck::cast_slice(lights));
        let masks = archive_light_z_masks(lights);
        light_data.extend_from_slice(bytemuck::cast_slice(&masks));
        let light_buffer = buffer(
            device,
            "unchanged authored punctual light constants",
            &light_data,
            wgpu::BufferUsages::UNIFORM,
        )?;
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &camera_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
        });
        let meshes = geometry
            .into_iter()
            .filter(|s| !s.instances.is_empty())
            .map(|s| {
                let mut low = Vec3::splat(f32::INFINITY);
                let mut high = Vec3::splat(f32::NEG_INFINITY);
                for v in &s.vertices {
                    let p = Vec3::from_array(v.position);
                    low = low.min(p);
                    high = high.max(p);
                }
                let centre = (low + high) * 0.5;
                let radius = (high - low).length() * 0.5;
                let spheres: Vec<(Vec3, f32)> = s
                    .instances
                    .iter()
                    .map(|instance| {
                        let m = Mat4::from_cols_array_2d(&instance.world);
                        let scale = (m.x_axis.truncate().length_squared()
                            + m.y_axis.truncate().length_squared()
                            + m.z_axis.truncate().length_squared())
                        .sqrt();
                        let c = m.transform_point3(centre);
                        (
                            c,
                            radius * scale
                                + 64.
                                    * f32::EPSILON
                                    * (1. + c.abs().max_element() + radius * scale),
                        )
                    })
                    .collect();
                let instances = buffer(
                    device,
                    "scene instances",
                    bytemuck::cast_slice(&s.instances),
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
                )?;
                let visible_instances = buffer(
                    device,
                    "visible authored instances",
                    bytemuck::cast_slice(&s.instances),
                    wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                )?;
                let culling = if std::env::var_os("VOXY_STREET_GPU_CULLING").is_some()
                    && s.instances.len() >= 1024
                    && !materials[s.material].transparent
                {
                    Some(crate::ArchiveInstanceCulling::new(
                        device,
                        &instances,
                        &visible_instances,
                        &spheres,
                        s.indices.len() as u32,
                    )?)
                } else {
                    None
                };
                Ok(Mesh {
                    culling,
                    frustum: crate::archive_frustum::ArchiveFrustumGroups::new(&spheres),
                    scratch_ids: Vec::with_capacity(s.instances.len()),
                    scratch_instances: Vec::new(),
                    vertices: buffer(
                        device,
                        "authored vertices",
                        bytemuck::cast_slice(&s.vertices),
                        wgpu::BufferUsages::VERTEX,
                    )?,
                    indices: buffer(
                        device,
                        "authored indices",
                        bytemuck::cast_slice(&s.indices),
                        wgpu::BufferUsages::INDEX,
                    )?,
                    instances,
                    visible_instances,
                    visible_ids: (0..s.instances.len()).collect(),
                    visible_count: s.instances.len() as u32,
                    source_instances: s.instances.clone(),
                    spheres,
                    count: s.indices.len() as u32,
                    copies: s.instances.len() as u32,
                    material: s.material,
                    center: s.center,
                })
            })
            .collect::<Result<Vec<_>, crate::ComputeError>>()?;
        let centre = (bounds.0 + bounds.1) * 0.5;
        let extent = ((bounds.1 - bounds.0).length() * 0.44).max(1.);
        #[allow(deprecated)]
        let shadow_matrix =
            Mat4::orthographic_rh(-extent, extent, -extent, extent, 0.1, extent * 4.)
                * Mat4::look_at_rh(centre + sun * extent * 2., centre, Vec3::Y);
        let hdr = target(device, width, height)?;
        let hdr_view = hdr.create_view(&Default::default());
        let background = target(device, width, height)?;
        let depth_texture = depth(device, width, height);
        let depth_view = depth_texture.create_view(&Default::default());
        let depth_bounds = if std::env::var_os("VOXY_STREET_DDA_HIERARCHY").is_some() {
            crate::archive_depth_bounds::ArchiveDepthBounds::new_hierarchy(
                device,
                &depth_texture,
                width,
                height,
            )?
        } else {
            crate::archive_depth_bounds::ArchiveDepthBounds::new(
                device,
                &depth_texture,
                width,
                height,
            )?
        };
        let pixel_depth_bounds = if std::env::var_os("VOXY_STREET_RAY_HIERARCHY").is_some() {
            Some(
                crate::archive_depth_bounds::ArchiveDepthBounds::new_pixel_hierarchy(
                    device,
                    &depth_texture,
                    width,
                    height,
                )?,
            )
        } else {
            None
        };
        let history_depth =
            ComputeMemoryBudget::for_device(device).allocate_texture(&wgpu::TextureDescriptor {
                label: Some("opaque radiance history depth"),
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
            })?;
        let shadow = depth(device, 8192, 8192);
        let shadow_view = shadow.create_view(&Default::default());
        let comparison = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        // Irradiance probes start disabled (origin.w = 0) until baked.
        let probe_sh = buffer(
            device,
            "irradiance probe SH",
            &vec![0; crate::probe_gi::MAX_PROBES * crate::probe_gi::PROBE_BYTES as usize],
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        )?;
        let probe_grid = buffer(
            device,
            "irradiance probe grid",
            &[0; std::mem::size_of::<crate::probe_gi::ProbeGrid>()],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        )?;
        let compute_enabled = std::env::var_os("VOXY_STREET_COMPUTE_GI").is_some();
        let gi_output = compute_gi::output_texture(
            device,
            if compute_enabled { width } else { 1 },
            if compute_enabled { height } else { 1 },
        )?;
        let ssr_enabled = compute_enabled && std::env::var_os("VOXY_STREET_COMPUTE_SSR").is_some();
        let ssr_output = compute_gi::output_texture(
            device,
            if ssr_enabled { width } else { 1 },
            if ssr_enabled { height } else { 1 },
        )?;
        let compute_gi = if compute_enabled {
            Some(compute_gi::ArchiveComputeGi::new(
                device,
                &shader,
                &shadow_layout,
                &layout,
                &vertex_buffers,
                &camera,
                &background,
                &depth_texture,
                &history_depth,
                &depth_bounds.texture,
                pixel_depth_bounds
                    .as_ref()
                    .map_or(&*depth_bounds.texture, |b| &*b.texture),
                &gi_output,
                &ssr_output,
                ssr_enabled,
                width,
                height,
            )?)
        } else {
            None
        };
        let phase_chunks = compute_gi
            .as_ref()
            .and_then(compute_gi::ArchiveComputeGi::stages_configuration)
            .map_or(0, |cfg| cfg[0] as u32);
        let mut timing = Timing::new(device, compute_enabled, ssr_enabled, phase_chunks)?;
        if let Some(timing) = &mut timing {
            timing.skip_inactive_gi = gi_stage_crossover;
        }
        let frame_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &frame_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &background.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&comparison),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(
                        &history_depth.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: probe_sh.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: probe_grid.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(
                        &depth_bounds.texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(
                        &gi_output.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(
                        &ssr_output.create_view(&Default::default()),
                    ),
                },
            ],
        });
        let hdr_output = format == wgpu::TextureFormat::Rgba16Float;
        let display_parameters = buffer(
            device,
            "street HDR display parameters",
            bytemuck::cast_slice(&[
                display_mode(hdr_output, ArchiveToneMapper::default()),
                1.,
                1.,
                0.,
            ]),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        )?;
        let bloom = crate::bloom::Bloom::new(device, width, height)?;
        let post_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &post_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: display_parameters.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(
                        &sky.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&sky_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&bloom.output()),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&comparison),
                },
            ],
        });
        let transmission_depth = (std::env::var_os("VOXY_STREET_TRANSMISSION_DEPTH").is_some()
            && materials.iter().any(|m| m.fully_covering))
        .then(|| depth(device, width, height));
        let transmission_depth_view = transmission_depth
            .as_ref()
            .map(|t| t.create_view(&Default::default()));
        let draw_order = Vec::with_capacity(meshes.len());
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            meshes,
            draw_order,
            materials,
            _textures: textures,
            _lights: light_buffer,
            camera,
            camera_bind,
            compute_gi,
            _gi_output: gi_output,
            _ssr_output: ssr_output,
            opaque,
            opaque_double,
            masked,
            masked_double,
            transparent,
            transparent_double,
            covered_transparent,
            covered_transparent_double,
            shadow_pipeline,
            depth_pipeline,
            depth_double,
            timing,
            refine_alternates,
            refine_crossover,
            refine_cache_base,
            gi_stage_crossover,
            benchmark_frame: 0,
            gpu_frame_stamps: Vec::new(),
            gpu_times_ms: Vec::new(),
            gpu_shading_times_ms: Vec::new(),
            gpu_gi_times_ms: Vec::new(),
            gpu_ssr_times_ms: Vec::new(),
            ray_queue_frame_count: 0,
            exposure: std::env::var("VOXY_STREET_EXPOSURE")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite() && *v > 0.)
                .unwrap_or(if environment_enabled {
                    1. / 600.
                } else {
                    1. / 1200.
                }),
            exposure_bias_ev: 0.,
            adapted_exposure_scale: 1.,
            sky_strength: 4.,
            _environment: environment,
            environment_bindings,
            environment_enabled,
            _sky: sky,
            post,
            post_linear,
            output_format: format,
            antialiasing: crate::ArchiveAntialiasing::Spatial,
            temporal: None,
            post_bind,
            bloom,
            display_parameters,
            hdr_output,
            linear_output: false,
            vertical_fov: 65_f32.to_radians(),
            camera_up: Vec3::Y,
            tone_mapper: ArchiveToneMapper::default(),
            camera_lens: Default::default(),
            display_headroom: 1.,
            spatial_antialiasing: true,
            frame_bind,
            probe_sh,
            probe_bake_owner: Arc::new(()),
            probe_grid,
            hdr,
            hdr_view,
            background,
            depth: depth_texture,
            depth_view,
            transmission_depth,
            transmission_depth_view,
            history_depth,
            depth_bounds,
            pixel_depth_bounds,
            shadow,
            shadow_view,
            shadow_matrix,
            weather: Default::default(),
            weather_time: 0.,
            surface_wetness: None,
            adapted_wetness: 0.,
            puddle_coverage: 0.45,
            previous_vp: None,
            shadow_bounds: bounds,
            sun,
            sun_colour,
            width,
            height,
            shadow_ready: false,
            camera_layout,
            water: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transmission_depth_requires_exact_full_coverage() {
        let mut source = ArchiveMaterialSource {
            parameters: ArchiveMaterial::metallic_roughness([1.; 4], 0., 0.1, 0.9, -2.),
            images: [None; 4],
            double_sided: false,
        };
        assert!(fully_covering_transmission(&source));
        source.parameters.diffuse[3] = f32::from_bits(1_f32.to_bits() - 1);
        assert!(!fully_covering_transmission(&source));
        source.parameters.diffuse[3] = 1.;
        source.images[0] = Some(0);
        assert!(!fully_covering_transmission(&source));
        source.parameters.workflow[1] = 1.5;
        assert!(!fully_covering_transmission(&source));
        source.parameters.workflow[1] = f32::from_bits(1.5_f32.to_bits() + 1);
        assert!(fully_covering_transmission(&source));
        source.parameters.surface[3] = 0.;
        assert!(!fully_covering_transmission(&source));
        source.parameters.surface[3] = -1.;
        source.parameters.diffuse[3] = 0.;
        assert!(fully_covering_transmission(&source)); // fs_opaque returns alpha one.
        source.parameters.surface[2] = 0.;
        assert!(!fully_covering_transmission(&source));
    }

    #[test]
    fn scene_shader_validates() {
        let source = archive_shader_source();
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
    #[test]
    fn camera_layout_matches_shader() {
        // 5 matrices and 8 vectors, as declared by archive_scene.wgsl.
        assert_eq!(std::mem::size_of::<Camera>(), 5 * 64 + 8 * 16);
    }
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn reflective_presets_route_to_expected_passes() {
        assert_eq!(ArchiveMaterial::MIRROR.surface[2], 0.);
        assert!(ArchiveMaterial::GLASS.surface[2] > 0.);
        assert_eq!(ArchiveMaterial::GLASS.surface[3], -2.);
    }
}

#[cfg(test)]
#[path = "archive_vertex_invariance_tests.rs"]
mod vertex_invariance_tests;

#[cfg(test)]
#[path = "archive_derivative_control_flow_tests.rs"]
mod derivative_control_flow_tests;
