use crate::{Bc7ImageAsset, ComputeMemoryBudget};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::sync::{Arc, OnceLock};

/// Original display panorama and separately prepared indirect-light panorama.
#[derive(Debug)]
pub struct ArchiveEnvironmentSource {
    pub lighting: crate::HdrImageAsset,
    pub background: crate::HdrImageAsset,
    pub intensity: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ArchiveVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ArchiveInstance {
    pub world: [[f32; 4]; 4],
    pub normal: [[f32; 4]; 3],
}

impl ArchiveInstance {
    pub fn new(world: Mat4) -> Self {
        let normal = world.inverse().transpose();
        Self {
            world: world.to_cols_array_2d(),
            normal: [
                normal.x_axis.to_array(),
                normal.y_axis.to_array(),
                normal.z_axis.to_array(),
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ArchiveMaterial {
    pub diffuse: [f32; 4],
    pub specular_gloss: [f32; 4],
    pub emissive_normal: [f32; 4],
    /// metallic, roughness, transmission, alpha cutoff (-1 opaque, -2 blend)
    pub surface: [f32; 4],
    /// 0 specular/glossiness, 1 metallic/roughness; reserved
    pub workflow: [f32; 4],
}

impl ArchiveMaterial {
    /// Metal/roughness parameters with no scanned maps or procedural surface.
    pub const fn metallic_roughness(
        colour: [f32; 4],
        metallic: f32,
        roughness: f32,
        transmission: f32,
        cutoff: f32,
    ) -> Self {
        Self {
            diffuse: colour,
            specular_gloss: [1.; 4],
            emissive_normal: [0., 0., 0., 1.],
            surface: [metallic, roughness, transmission, cutoff],
            workflow: [1., 0., 0., 0.],
        }
    }
    /// Silvered mirror: screen-traced near-ideal metal reflection.
    pub const MIRROR: Self = Self::metallic_roughness([0.97, 0.97, 0.96, 1.], 1., 0.02, 0., -1.);
    /// Clear window glass (F0 0.04): reflects the scene and refracts what is behind it.
    pub const GLASS: Self = Self::metallic_roughness([0.92, 0.96, 0.95, 0.1], 0., 0.02, 0.92, -2.);
    /// Dark standing-water sheet using the default dielectric F0.
    pub const WATER: Self = Self::metallic_roughness([0.25, 0.3, 0.3, 1.], 0., 0.03, 0., -1.);
}

/// Source pixels for an archive material; colour transfer is selected by the shader role.
#[derive(Debug)]
pub enum ArchiveImageSource {
    Bc7(Bc7ImageAsset),
    Rgba(crate::ImageAsset),
    /// Linear float32 pixels retain the precision of 16-bit scanned source maps.
    Float {
        width: u32,
        height: u32,
        pixels: Vec<f32>,
    },
}

impl ArchiveImageSource {
    /// Upload source pixels and every mip level; float data retains float32 precision.
    pub fn upload(
        self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Arc<voxy_gpu_core::ManagedTexture>, crate::ComputeError> {
        match self {
            Self::Bc7(image) => image.upload(device, queue),
            Self::Float {
                width,
                height,
                pixels,
            } => {
                if width == 0
                    || height == 0
                    || width > device.limits().max_texture_dimension_2d
                    || height > device.limits().max_texture_dimension_2d
                    || u64::from(width) * u64::from(height) * 4 != pixels.len() as u64
                    || pixels.iter().any(|p| !p.is_finite())
                    || !device
                        .features()
                        .contains(wgpu::Features::FLOAT32_FILTERABLE)
                {
                    return Err(crate::ComputeError::Validation(
                        "invalid float material or missing FLOAT32_FILTERABLE".into(),
                    ));
                }
                let mips = width.max(height).ilog2() + 1;
                let texture = ComputeMemoryBudget::for_device(device).allocate_texture(
                    &wgpu::TextureDescriptor {
                        label: Some("full precision scanned material"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: mips,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba32Float,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::COPY_DST
                            | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    },
                )?;
                let mut pixels =
                    image::Rgba32FImage::from_raw(width, height, pixels).ok_or_else(|| {
                        crate::ComputeError::Validation("invalid float pixels".into())
                    })?;
                for mip in 0..mips {
                    let (w, h) = pixels.dimensions();
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &texture,
                            mip_level: mip,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        bytemuck::cast_slice(pixels.as_raw()),
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(w * 16),
                            rows_per_image: Some(h),
                        },
                        wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                    );
                    if mip + 1 < mips {
                        pixels = image::imageops::resize(
                            &pixels,
                            (w / 2).max(1),
                            (h / 2).max(1),
                            image::imageops::FilterType::Triangle,
                        );
                    }
                }
                Ok(texture)
            }
            Self::Rgba(image) => {
                let width = image.width();
                let height = image.height();
                let mips = width.max(height).ilog2() + 1;
                let texture = ComputeMemoryBudget::for_device(device).allocate_texture(
                    &wgpu::TextureDescriptor {
                        label: Some("archive RGBA material image"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: mips,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[
                            wgpu::TextureFormat::Rgba8Unorm,
                            wgpu::TextureFormat::Rgba8UnormSrgb,
                        ],
                    },
                )?;
                let mut pixels = image::RgbaImage::from_raw(width, height, image.rgba().to_vec())
                    .ok_or_else(|| {
                    crate::ComputeError::Validation("invalid RGBA image".into())
                })?;
                for mip in 0..mips {
                    let w = pixels.width();
                    let h = pixels.height();
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &texture,
                            mip_level: mip,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        pixels.as_raw(),
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(w * 4),
                            rows_per_image: Some(h),
                        },
                        wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                    );
                    if mip + 1 < mips {
                        pixels = image::imageops::resize(
                            &pixels,
                            (w / 2).max(1),
                            (h / 2).max(1),
                            image::imageops::FilterType::Triangle,
                        );
                    }
                }
                Ok(texture)
            }
        }
    }
}

#[derive(Debug)]
pub struct ArchiveMaterialSource {
    pub parameters: ArchiveMaterial,
    /// diffuse, specular/glossiness OR metallic/roughness, normal, emissive
    pub images: [Option<usize>; 4],
    pub double_sided: bool,
}

#[derive(Debug)]
pub struct ArchiveMeshSource {
    pub vertices: Vec<ArchiveVertex>,
    pub indices: Vec<u32>,
    pub instances: Vec<ArchiveInstance>,
    pub material: usize,
    pub center: Vec3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ArchiveLight {
    pub position_kind: [f32; 4],
    pub colour_intensity: [f32; 4],
}

// Match the shader's final alpha exactly. Masked and texture-dependent blend
// coverage cannot enter this prepass; transmission alone has final alpha one.
pub(super) fn fully_covering_transmission(source: &ArchiveMaterialSource) -> bool {
    let p = &source.parameters;
    if p.surface[3] >= 0. {
        return false;
    }
    if p.surface[3] == -2. {
        p.diffuse[3] == 1. && (source.images[0].is_none() || p.workflow[1] > 1.5)
    } else {
        p.surface[2] > 0.
    }
}

/// SDR tone mapping curve applied by the archive display pass.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ArchiveToneMapper {
    /// Narkowicz ACES fit: punchy contrast, skews saturated highlights.
    Aces,
    /// AgX base: desaturates toward white in highlights like film.
    #[default]
    Agx,
}

/// Mode selector read by `display_transform` in display_transform.wgsl.
pub(super) fn display_mode(hdr_output: bool, tone_mapper: ArchiveToneMapper) -> f32 {
    match (hdr_output, tone_mapper) {
        (true, _) => 1.,
        (false, ArchiveToneMapper::Aces) => 0.,
        (false, ArchiveToneMapper::Agx) => 2.,
    }
}

/// Analytic-sky ambient relative to daylight: 1 with the sun 3 degrees up or
/// higher, falling log-linearly to 1e-4 at the end of nautical twilight.
pub(super) fn twilight_sky(sun_y: f32) -> f32 {
    let elevation = sun_y.clamp(-1., 1.).asin().to_degrees();
    10_f32.powf(-4. * ((3. - elevation) / 15.).clamp(0., 1.))
}

/// Saturation-based exposure: maximum luminance 1.2 * 2^EV100 maps to 1.
pub fn exposure_from_ev100(ev100: f32) -> f32 {
    1. / (1.2 * 2_f32.powf(ev100))
}

/// EV100 = log2(N^2 / t * 100 / S).
pub fn ev100_from_camera(aperture: f32, shutter_s: f32, iso: f32) -> f32 {
    (aperture * aperture / shutter_s * 100. / iso).log2()
}

/// Shader effects disabled for profiling: `VOXY_FX_OFF` bit mask, see
/// `fx_off` in archive_scene.wgsl.
pub(super) fn effects_off() -> f32 {
    static MASK: OnceLock<f32> = OnceLock::new();
    *MASK.get_or_init(|| {
        std::env::var("VOXY_FX_OFF")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0) as f32
    })
}

/// The same traversal implementation is qualified with R32Float fixtures and
/// instantiated directly over the renderer's full Depth32Float attachment.
#[doc(hidden)]
pub fn archive_shader_source() -> String {
    static BASE: OnceLock<String> = OnceLock::new();
    let base = BASE.get_or_init(|| {
        let traversal = [
            include_str!("../archive_depth_dda_paired.wgsl"),
            include_str!("../archive_depth_hierarchy.wgsl"),
        ]
        .concat()
        .replace("image:texture_2d<f32>", "image:texture_depth_2d")
        .replace(
            "textureLoad(image,vec2<i32>(x,y),0).x",
            "textureLoad(image,vec2<i32>(x,y),0)",
        );
        [
            include_str!("../bsdf.wgsl"),
            include_str!("../archive_environment.wgsl"),
            include_str!("../soft_shadow.wgsl"),
            include_str!("../probe_sampling.wgsl"),
            include_str!("../archive_history.wgsl"),
            include_str!("../archive_projection_pair.wgsl"),
            include_str!("../archive_ray_projection.wgsl"),
            &traversal,
            include_str!("../archive_ray_interval.wgsl"),
            include_str!("../archive_scene.wgsl"),
            include_str!("../archive_ray_stages.wgsl"),
        ]
        .concat()
    });
    let mut source = base.clone();
    if std::env::var_os("VOXY_STREET_RAY_PERSISTENT").is_some() {
        source.push_str(
            &include_str!("../archive_depth_dda_resumable.wgsl")
                .replace("image:texture_2d<f32>", "image:texture_depth_2d")
                .replace(
                    "textureLoad(image,vec2<i32>(x,y),0).x",
                    "textureLoad(image,vec2<i32>(x,y),0)",
                ),
        );
        source.push_str(include_str!("../archive_ray_persistent.wgsl"));
    }
    source
}

/// Remaps a dry dielectric roughness in `[0.04, 1.0]` and diffuse albedo factor under a thin
/// water film of `wetness` in `[0.0, 1.0]` (Sebastien Lagarde / porous medium darkening model).
///
/// Returns `(effective_roughness, porosity_albedo_scale)`.
#[must_use]
pub fn wet_roughness_remapping(dry_roughness: f32, wetness: f32, porosity: f32) -> (f32, f32) {
    let r = if dry_roughness.is_finite() {
        dry_roughness.clamp(0.04, 1.0)
    } else {
        0.5
    };
    let w = if wetness.is_finite() {
        wetness.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let p = if porosity.is_finite() {
        porosity.clamp(0.0, 1.0)
    } else {
        0.5
    };
    // Water film fills micro-cavities and smooths roughness toward ~0.06 at saturation:
    let wet_r = (r * (1.0 - 0.85 * w)).clamp(0.04, 1.0);
    // Internal total reflection in water-filled pores darkens porous stone/asphalt:
    let darken = 1.0 - 0.65 * p * w;
    (wet_r, darken.clamp(0.2, 1.0))
}

/// Advances porous surface wetness accumulation and evaporation over `dt` seconds.
/// Surfaces wet rapidly during rainfall (`0.45 / s`) and dry gradually after rain stops (`0.12 / s`).
#[must_use]
pub fn step_wetness_accumulation(current_wetness: f32, target_rain: f32, dt: f32) -> f32 {
    let cur = if current_wetness.is_finite() {
        current_wetness.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let tgt = if target_rain.is_finite() {
        target_rain.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if !dt.is_finite() || dt <= 0.0 {
        return cur;
    }
    let rate = if tgt >= cur { 0.45 } else { 0.12 };
    let alpha = 1.0 - (-dt * rate).exp();
    (cur + (tgt - cur) * alpha).clamp(0.0, 1.0)
}

/// Physiological temporal eye adaptation over `dt` seconds with asymmetric pupillary/photochemical rates:
/// dark-to-bright pupillary reflex adapts rapidly (`rate = 2.2 / s`, $\tau \approx 0.45\text{ s}$), while
/// bright-to-dark adaptation proceeds more gradually (`rate = 0.7 / s`, $\tau \approx 1.4\text{ s}$).
#[must_use]
pub fn step_eye_adaptation(current_scale: f32, target_scale: f32, dt: f32) -> f32 {
    let cur = if current_scale.is_finite() {
        current_scale.clamp(0.25, 4.0)
    } else {
        1.0
    };
    let tgt = if target_scale.is_finite() {
        target_scale.clamp(0.25, 4.0)
    } else {
        1.0
    };
    if !dt.is_finite() || dt <= 0.0 {
        return cur;
    }
    // Lower exposure scale = stopping down for bright light (fast pupillary constriction);
    // higher exposure scale = opening up for darker conditions (slower dark adaptation).
    let rate = if tgt < cur { 2.2 } else { 0.7 };
    let alpha = 1.0 - (-dt * rate).exp();
    (cur + (tgt - cur) * alpha).clamp(0.25, 4.0)
}

/// Selects an adaptive screen-space GI ray-march step budget in `[min_steps, max_steps]`
/// proportional to the available frame budget ratio `target_frame_ms / measured_frame_ms`.
#[must_use]
pub fn adaptive_gi_ray_budget(
    measured_frame_ms: f32,
    target_frame_ms: f32,
    min_steps: u32,
    max_steps: u32,
) -> u32 {
    let lo = min_steps.max(4);
    let hi = max_steps.max(lo);
    if !measured_frame_ms.is_finite()
        || !target_frame_ms.is_finite()
        || measured_frame_ms <= 0.0
        || target_frame_ms <= 0.0
    {
        return lo;
    }
    let ratio = (target_frame_ms / measured_frame_ms).clamp(0.25, 2.0);
    let t = ((ratio - 0.5) / 1.0).clamp(0.0, 1.0);
    let steps = (lo as f32 + (hi - lo) as f32 * t).round() as u32;
    steps.clamp(lo, hi)
}

#[cfg(test)]
mod exposure_tests {
    use super::*;

    #[test]
    fn sunny_sixteen_is_about_ev15() {
        let ev = ev100_from_camera(16., 1. / 100., 100.);
        assert!((ev - 14.64).abs() < 0.01, "{ev}");
    }

    #[test]
    fn ev100_round_trips_exposure() {
        for ev in [-2., 0., 8.97, 15.] {
            let e = exposure_from_ev100(ev);
            assert!(((1. / (1.2 * e)).log2() - ev).abs() < 1e-4);
        }
    }

    #[test]
    fn display_mode_selects_transform() {
        assert_eq!(display_mode(true, ArchiveToneMapper::Agx), 1.);
        assert_eq!(display_mode(false, ArchiveToneMapper::Aces), 0.);
        assert_eq!(display_mode(false, ArchiveToneMapper::Agx), 2.);
    }
}

#[cfg(test)]
mod n1_archive_scene_tests {
    use super::*;

    #[test]
    fn wet_roughness_remapping_smooths_and_darkens_porous_surfaces() {
        let (dry_r, dry_scale) = wet_roughness_remapping(0.8, 0.0, 0.7);
        let (wet_r, wet_scale) = wet_roughness_remapping(0.8, 1.0, 0.7);
        assert!(wet_r < dry_r);
        assert!(wet_scale < dry_scale);
        assert_eq!(adaptive_gi_ray_budget(4.0, 8.33, 8, 32), 32);
        assert_eq!(adaptive_gi_ray_budget(20.0, 8.33, 8, 32), 8);
    }

    #[test]
    fn step_wetness_accumulation_wets_faster_than_it_evaporates() {
        let wet_after_1s = step_wetness_accumulation(0.0, 1.0, 1.0);
        let dry_loss_after_1s = 1.0 - step_wetness_accumulation(1.0, 0.0, 1.0);
        assert!(wet_after_1s > 0.30, "wet_after_1s={wet_after_1s}");
        assert!(dry_loss_after_1s > 0.05 && dry_loss_after_1s < wet_after_1s);
    }

    #[test]
    fn step_eye_adaptation_constricts_faster_than_it_dilates() {
        let brighten_step = step_eye_adaptation(1.0, 1.5, 0.5) - 1.0;
        let darken_step = 1.5 - step_eye_adaptation(1.5, 1.0, 0.5);
        assert!(darken_step > brighten_step && brighten_step > 0.1);
    }
}
