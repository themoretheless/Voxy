//! Render pipeline construction for `ArchiveSceneRenderer`, keeping `archive_scene.rs`
//! under the line budget without changing shader or render-state configuration.

use super::{RefineCrossover, legacy_render_mode, opt_in_default};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PassKind {
    Main,
    Shadow,
    Predepth,
    PredepthNormal,
}

#[derive(Clone, Copy)]
struct PipelineSpec {
    blend: bool,
    double: bool,
    kind: PassKind,
    masked: bool,
    covered: bool,
}

pub(super) struct ScenePipelineConfig {
    pub refine_crossover: Option<RefineCrossover>,
    pub refine_cache_base: bool,
    pub light_z_masks: bool,
    pub ray_depth_reuse: bool,
    pub gi_stage_crossover: bool,
    pub compute_requested: bool,
    pub fast_gi_enabled: bool,
    pub fast_shading_enabled: bool,
}

impl ScenePipelineConfig {
    pub(super) fn from_env() -> Result<Self, crate::ComputeError> {
        let refine_crossover = std::env::var("VOXY_STREET_REFINE_CROSSOVER")
            .ok()
            .map(|s| RefineCrossover::parse(&s))
            .transpose()?;
        let qual_mode = refine_crossover.is_some()
            || std::env::var_os("VOXY_STREET_RAY_REFERENCE").is_some()
            || std::env::var_os("VOXY_STREET_GI_PRECISION_PROOF").is_some();
        let flag = |var: &str| {
            if qual_mode {
                std::env::var_os(var).is_some()
            } else {
                opt_in_default(var)
            }
        };
        let refine_cache_base = flag("VOXY_STREET_REFINE_CACHE");
        let light_z_masks = flag("VOXY_STREET_LIGHT_Z_MASKS");
        let ray_depth_reuse = flag("VOXY_STREET_RAY_DEPTH_REUSE");
        let gi_stage_crossover = std::env::var_os("VOXY_STREET_GI_STAGE_CROSSOVER").is_some();
        let compute_requested = std::env::var_os("VOXY_STREET_COMPUTE_GI").is_some();
        if gi_stage_crossover && (refine_crossover.is_none() || !compute_requested) {
            return Err(crate::ComputeError::Validation(
                "GI stage crossover requires a schedule and compute GI".into(),
            ));
        }
        if refine_crossover.is_some()
            && (std::env::var_os("VOXY_STREET_REFINE_CACHE").is_some()
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
        let fast_gi_enabled =
            !qual_mode && !compute_requested && opt_in_default("VOXY_STREET_FAST_GI");
        let fast_shading_enabled = !qual_mode && opt_in_default("VOXY_STREET_FAST_SHADING");
        Ok(Self {
            refine_crossover,
            refine_cache_base,
            light_z_masks,
            ray_depth_reuse,
            gi_stage_crossover,
            compute_requested,
            fast_gi_enabled,
            fast_shading_enabled,
        })
    }

    pub(super) fn fast_gi_constants(&self) -> [(&'static str, f64); 4] {
        [
            ("ARCHIVE_RAY_BOUNDS", 1.),
            (
                "ARCHIVE_RAY_DEPTH_REUSE",
                f64::from(u8::from(self.ray_depth_reuse)),
            ),
            (
                "ARCHIVE_REFINE_CACHE",
                f64::from(u8::from(self.refine_cache_base)),
            ),
            (
                "ARCHIVE_FAST_SHADING",
                f64::from(u8::from(self.fast_shading_enabled)),
            ),
        ]
    }

    pub(super) fn bypass_1x_supersample() -> bool {
        !legacy_render_mode()
            && std::env::var_os("VOXY_STREET_HDR_STAGE_CAPTURE").is_none()
            && std::env::var_os("VOXY_STREET_HDR_CHECK_DIR").is_none()
    }
}

pub(super) struct PipelineInputs<'a> {
    pub device: &'a wgpu::Device,
    pub shader: &'a wgpu::ShaderModule,
    pub main_layout: &'a wgpu::PipelineLayout,
    pub shadow_layout: &'a wgpu::PipelineLayout,
    pub vertex_buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    pub config: &'a ScenePipelineConfig,
}

pub(super) struct ScenePipelines {
    pub opaque: wgpu::RenderPipeline,
    pub opaque_double: wgpu::RenderPipeline,
    pub masked: wgpu::RenderPipeline,
    pub masked_double: wgpu::RenderPipeline,
    pub transparent: wgpu::RenderPipeline,
    pub transparent_double: wgpu::RenderPipeline,
    pub covered_transparent: wgpu::RenderPipeline,
    pub covered_transparent_double: wgpu::RenderPipeline,
    pub shadow_pipeline: wgpu::RenderPipeline,
    pub depth_opaque: wgpu::RenderPipeline,
    pub depth_opaque_double: wgpu::RenderPipeline,
    pub depth_pipeline: wgpu::RenderPipeline,
    pub depth_double: wgpu::RenderPipeline,
    pub prepass_pipeline: wgpu::RenderPipeline,
    pub prepass_double: wgpu::RenderPipeline,
    pub prepass_masked: wgpu::RenderPipeline,
    pub prepass_masked_double: wgpu::RenderPipeline,
    pub refine_alternates: Option<[wgpu::RenderPipeline; 8]>,
}

fn env_set(var: &str) -> f64 {
    f64::from(u8::from(std::env::var_os(var).is_some()))
}

fn fragment_constants(
    cfg: &ScenePipelineConfig,
    spec: PipelineSpec,
    compute_variant: bool,
    refine_variant: bool,
) -> Vec<(&'static str, f64)> {
    let shadow = spec.kind == PassKind::Shadow;
    let predepth = matches!(spec.kind, PassKind::Predepth | PassKind::PredepthNormal);
    let main_unblended = spec.kind == PassKind::Main && !spec.blend;
    let ssr = main_unblended
        && std::env::var_os("VOXY_STREET_COMPUTE_GI").is_some()
        && std::env::var_os("VOXY_STREET_COMPUTE_SSR").is_some();
    let compute_gi = main_unblended && compute_variant;
    let edge_cache = std::env::var("VOXY_STREET_DDA_TILE_EDGE_CACHE").as_deref() == Ok("1");
    let ray_bounds = std::env::var_os("VOXY_STREET_RAY_REFERENCE").is_none();
    vec![
        (
            "ARCHIVE_DDA_INTERVAL_CACHE",
            env_set("VOXY_STREET_DDA_INTERVAL_CACHE"),
        ),
        (
            "ARCHIVE_RAY_FORCE_FALLBACK",
            env_set("VOXY_STREET_RAY_FORCE_FALLBACK"),
        ),
        (
            "ARCHIVE_DDA_PLANE_CACHE",
            env_set("VOXY_STREET_DDA_PLANE_CACHE"),
        ),
        (
            "ARCHIVE_DDA_POINT_CACHE",
            env_set("VOXY_STREET_DDA_POINT_CACHE"),
        ),
        ("ARCHIVE_COMPUTE_SSR", f64::from(u8::from(ssr))),
        ("ARCHIVE_COMPUTE_GI", f64::from(u8::from(compute_gi))),
        (
            "ARCHIVE_COMPUTE_GI_EXACT_SAMPLE",
            f64::from(u8::from(cfg.gi_stage_crossover)),
        ),
        ("ARCHIVE_EARLY_GI", env_set("VOXY_STREET_EARLY_GI")),
        (
            "ARCHIVE_DDA_TILE_EDGE_CACHE",
            f64::from(u8::from(edge_cache)),
        ),
        (
            "ARCHIVE_DDA_EDGE_CACHE",
            env_set("VOXY_STREET_DDA_EDGE_CACHE"),
        ),
        ("ARCHIVE_RAY_BOUNDS", f64::from(u8::from(ray_bounds))),
        (
            "ARCHIVE_DDA_HIERARCHY",
            env_set("VOXY_STREET_DDA_HIERARCHY"),
        ),
        (
            "ARCHIVE_DDA_FAR_BOUND",
            env_set("VOXY_STREET_DDA_FAR_BOUND"),
        ),
        (
            "ARCHIVE_DDA_CELL_BOUNDS",
            env_set("VOXY_STREET_DDA_CELL_BOUNDS"),
        ),
        ("ARCHIVE_DDA_TILES", env_set("VOXY_STREET_PAIRED_DDA_TILES")),
        ("ARCHIVE_PAIRED_DDA", env_set("VOXY_STREET_PAIRED_DDA")),
        ("WGPU_METAL_PRECISE_MATH", env_set("VOXY_STREET_PAIRED_DDA")),
        (
            "ARCHIVE_LIGHT_Z_MASKS",
            f64::from(u8::from(cfg.light_z_masks)),
        ),
        (
            "ARCHIVE_RAY_DEPTH_REUSE",
            f64::from(u8::from(cfg.ray_depth_reuse)),
        ),
        ("ARCHIVE_REFINE_CACHE", f64::from(u8::from(refine_variant))),
        (
            "ARCHIVE_FAST_GI_AO",
            f64::from(u8::from(
                main_unblended && !shadow && !predepth && cfg.fast_gi_enabled,
            )),
        ),
        (
            "ARCHIVE_FAST_SHADING",
            f64::from(u8::from(
                spec.kind == PassKind::Main && cfg.fast_shading_enabled,
            )),
        ),
    ]
}

fn depth_stencil_state(spec: PipelineSpec) -> wgpu::DepthStencilState {
    let shadow = spec.kind == PassKind::Shadow;
    let predepth = matches!(spec.kind, PassKind::Predepth | PassKind::PredepthNormal);
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: Some(shadow || predepth),
        depth_compare: Some(if !shadow && !predepth && (!spec.blend || spec.covered) {
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
    }
}

fn create_pipeline(
    inputs: &PipelineInputs<'_>,
    spec: PipelineSpec,
    compute_variant: bool,
    refine_variant: bool,
) -> wgpu::RenderPipeline {
    let shadow = spec.kind == PassKind::Shadow;
    let predepth = matches!(spec.kind, PassKind::Predepth | PassKind::PredepthNormal);
    let predepth_normal = spec.kind == PassKind::PredepthNormal;
    let started = std::time::Instant::now();
    if std::env::var_os("VOXY_STREET_INIT_TRACE").is_some() {
        eprintln!(
            "ARCHIVE INIT pipeline_begin blend={} double={} shadow={shadow} predepth={predepth} masked={}",
            spec.blend, spec.double, spec.masked
        );
    }
    let main_targets = [Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Rgba16Float,
        blend: spec.blend.then_some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let predepth_targets = [Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Rgba8Unorm,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let targets: &[Option<wgpu::ColorTargetState>] = if predepth_normal {
        &predepth_targets
    } else if shadow || predepth {
        &[]
    } else {
        &main_targets
    };
    let constants = fragment_constants(inputs.config, spec, compute_variant, refine_variant);
    let entry_point = if predepth_normal {
        if spec.masked {
            "fs_predepth"
        } else {
            "fs_predepth_opaque"
        }
    } else if shadow || predepth {
        "fs_shadow"
    } else if spec.masked {
        "fs_main"
    } else {
        "fs_opaque"
    };
    let result = inputs
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("archive instanced material pipeline"),
            layout: Some(if shadow || predepth {
                inputs.shadow_layout
            } else {
                inputs.main_layout
            }),
            vertex: wgpu::VertexState {
                module: inputs.shader,
                entry_point: Some(if shadow { "vs_shadow" } else { "vs_main" }),
                compilation_options: Default::default(),
                buffers: inputs.vertex_buffers,
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: (!spec.double).then_some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(depth_stencil_state(spec)),
            multisample: Default::default(),
            fragment: (spec.kind != PassKind::Predepth || spec.masked).then_some(
                wgpu::FragmentState {
                    module: inputs.shader,
                    entry_point: Some(entry_point),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &constants,
                        ..Default::default()
                    },
                    targets,
                },
            ),
            multiview_mask: None,
            cache: None,
        });
    if std::env::var_os("VOXY_STREET_INIT_TRACE").is_some() {
        eprintln!(
            "ARCHIVE INIT pipeline_ready blend={} double={} shadow={shadow} predepth={predepth} masked={} milliseconds={:.3}",
            spec.blend,
            spec.double,
            spec.masked,
            started.elapsed().as_secs_f64() * 1000.
        );
    }
    result
}

impl ScenePipelines {
    pub(super) fn build(inputs: &PipelineInputs<'_>) -> Self {
        let cfg = inputs.config;
        let compute_base = cfg.compute_requested && !cfg.gi_stage_crossover;
        let make = |blend, double, kind, masked, covered, cv, rv| {
            create_pipeline(
                inputs,
                PipelineSpec {
                    blend,
                    double,
                    kind,
                    masked,
                    covered,
                },
                cv,
                rv,
            )
        };
        let std_main = |blend, double, masked, covered| {
            make(
                blend,
                double,
                PassKind::Main,
                masked,
                covered,
                compute_base,
                cfg.refine_cache_base,
            )
        };
        let pre = |double, kind, masked| {
            make(
                false,
                double,
                kind,
                masked,
                false,
                compute_base,
                cfg.refine_cache_base,
            )
        };
        let refine_alternates = cfg.refine_crossover.map(|mode| {
            let rv = mode.cache_b() && !cfg.gi_stage_crossover;
            let cv = if cfg.gi_stage_crossover {
                mode.cache_b()
            } else {
                cfg.compute_requested
            };
            [
                make(false, false, PassKind::Main, false, false, cv, rv),
                make(false, false, PassKind::Main, true, false, cv, rv),
                make(false, true, PassKind::Main, false, false, cv, rv),
                make(false, true, PassKind::Main, true, false, cv, rv),
                make(true, false, PassKind::Main, false, false, cv, rv),
                make(true, true, PassKind::Main, false, false, cv, rv),
                make(true, false, PassKind::Main, false, true, cv, rv),
                make(true, true, PassKind::Main, false, true, cv, rv),
            ]
        });
        Self {
            opaque: std_main(false, false, false, false),
            opaque_double: std_main(false, true, false, false),
            masked: std_main(false, false, true, false),
            masked_double: std_main(false, true, true, false),
            transparent: std_main(true, false, false, false),
            transparent_double: std_main(true, true, false, false),
            covered_transparent: std_main(true, false, false, true),
            covered_transparent_double: std_main(true, true, false, true),
            shadow_pipeline: pre(true, PassKind::Shadow, false),
            depth_opaque: pre(false, PassKind::Predepth, false),
            depth_opaque_double: pre(true, PassKind::Predepth, false),
            depth_pipeline: pre(false, PassKind::Predepth, true),
            depth_double: pre(true, PassKind::Predepth, true),
            prepass_pipeline: pre(false, PassKind::PredepthNormal, false),
            prepass_double: pre(true, PassKind::PredepthNormal, false),
            prepass_masked: pre(false, PassKind::PredepthNormal, true),
            prepass_masked_double: pre(true, PassKind::PredepthNormal, true),
            refine_alternates,
        }
    }
}
