//! CC0 scanned surfaces on independently authored original street geometry.
#![allow(dead_code)]
use super::scene::StreetScene;
use std::path::{Path, PathBuf};
use voxy_render::archive_scene::ArchiveImageSource;

fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
fn load(path: &Path, colour: bool) -> Result<ArchiveImageSource, Box<dyn std::error::Error>> {
    let reader = image::ImageReader::open(path)?;
    let (width, height) = reader.into_dimensions()?;
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err("scanned material dimensions exceeded".into());
    }
    let mut pixels = image::open(path)?.into_rgba32f();
    if colour {
        for pixel in pixels.pixels_mut() {
            for c in &mut pixel.0[..3] {
                *c = srgb_to_linear(*c);
            }
        }
    }
    Ok(ArchiveImageSource::Float {
        width,
        height,
        pixels: pixels.into_raw(),
    })
}
/// Lossless channel packing for opaque, nonmetallic scans: RGB+AO and XYZ+roughness.
/// Every retained channel stays float32; dimensions and mip filtering are unchanged.
pub(crate) fn pack_set(
    colour: ArchiveImageSource,
    arm: ArchiveImageSource,
    normal: ArchiveImageSource,
) -> Result<[ArchiveImageSource; 2], Box<dyn std::error::Error>> {
    let ArchiveImageSource::Float {
        width,
        height,
        mut pixels,
    } = colour
    else {
        return Err("float colour required".into());
    };
    let ArchiveImageSource::Float {
        width: aw,
        height: ah,
        pixels: arm,
    } = arm
    else {
        return Err("float surface data required".into());
    };
    let ArchiveImageSource::Float {
        width: nw,
        height: nh,
        pixels: mut normal,
    } = normal
    else {
        return Err("float normal required".into());
    };
    if (width, height) != (aw, ah)
        || (width, height) != (nw, nh)
        || pixels.len() != arm.len()
        || pixels.len() != normal.len()
        || pixels.chunks_exact(4).any(|p| p[3] != 1.)
    {
        return Err("packing requires matching opaque maps".into());
    }
    for ((colour, normal), surface) in pixels
        .chunks_exact_mut(4)
        .zip(normal.chunks_exact_mut(4))
        .zip(arm.chunks_exact(4))
    {
        colour[3] = surface[0];
        normal[3] = surface[1];
    }
    Ok([
        ArchiveImageSource::Float {
            width,
            height,
            pixels,
        },
        ArchiveImageSource::Float {
            width,
            height,
            pixels: normal,
        },
    ])
}
pub fn apply(scene: &mut StreetScene) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var_os("VOXY_STREET_MATERIALS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/environments/original-street/materials")
        });
    let mut sets = Vec::new();
    for asset in ["plastered_stone_wall", "brick_wall_001", "bark_willow"] {
        let colour = load(&root.join(format!("{asset}-Diffuse.png")), true)?;
        let arm = load(&root.join(format!("{asset}-arm.png")), false)?;
        let normal = load(&root.join(format!("{asset}-nor_gl.png")), false)?;
        let ids = [scene.images.len(), scene.images.len() + 1];
        scene.images.extend(pack_set(colour, arm, normal)?);
        sets.push(ids);
    }
    for material in &mut scene.materials {
        let kind = material.parameters.workflow[2];
        let set = if kind == 2. {
            Some(0)
        } else if kind == 7. {
            Some(1)
        } else if kind == 8. {
            Some(2)
        } else {
            None
        };
        if let Some(set) = set {
            let ids = sets[set];
            if material.parameters.workflow[0] != 1.
                || material.parameters.surface[2] != 0.
                || material.parameters.surface[0] != 0.
                || material.parameters.diffuse[3] != 1.
                || material.parameters.emissive_normal[..3]
                    .iter()
                    .any(|v| *v != 0.)
            {
                return Err("packed scan material must remain opaque and nonmetallic".into());
            }
            material.images = [Some(ids[0]), None, Some(ids[1]), None];
            material.parameters.workflow[1] = 2.;
            material.parameters.workflow[2] = 0.;
            material.parameters.workflow[3] = if set == 2 { -1.5 } else { 0.5 };
            material.parameters.surface[1] = 1.;
            // Scan already owns its albedo; retain each house's independently authored tint.
            if set == 2 {
                material.parameters.diffuse = [1., 1., 1., 1.];
            }
        }
    }
    eprintln!(
        "ORIGINAL STREET MATERIALS provider=PolyHaven license=CC0 scans=3 source_maps=9 gpu_textures={} source_resolution=4k gpu=linear_rgba32float packing=lossless_channels",
        scene.images.len()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sixteen_bit_source_levels_survive_decode_without_eight_bit_quantization() {
        let path =
            std::env::temp_dir().join(format!("voxy-scan-fixture-{}.png", std::process::id()));
        let pixels = image::ImageBuffer::<image::Rgba<u16>, Vec<u16>>::from_raw(
            2,
            1,
            vec![1, 2, 32767, 65535, 3, 4, 32768, 65535],
        )
        .unwrap();
        pixels.save(&path).unwrap();
        let image = load(&path, false).unwrap();
        std::fs::remove_file(path).unwrap();
        let ArchiveImageSource::Float { pixels, .. } = image else {
            panic!("float material required")
        };
        assert_eq!(pixels[0], 1. / 65535.);
        assert_eq!(pixels[1], 2. / 65535.);
        assert_eq!(pixels[4], 3. / 65535.);
        assert!(pixels[2] < pixels[6]);
    }
    #[test]
    fn solar_split_keeps_background_and_integrated_irradiance() {
        let (w, h) = (256_u32, 128_u32);
        let mut pixels = vec![1_f32; (w * h * 3) as usize];
        let index = ((h / 4 * w + w / 2) * 3) as usize;
        pixels[index..index + 3].copy_from_slice(&[1000., 800., 600.]);
        let original = pixels.clone();
        let source =
            voxy_render::HdrImageAsset::from_rgb(w, h, pixels, voxy_render::ImageLimits::default())
                .unwrap();
        let sun = sun_direction(&source);
        let (indirect, irradiance) = separate_sun(&source, sun).unwrap();
        assert_eq!(
            source.rgb(),
            original.as_slice(),
            "display source cannot be clipped"
        );
        assert_eq!(&indirect.rgb()[index..index + 3], &[1., 1., 1.]);
        let theta = (h as f64 / 4. + 0.5) / h as f64 * std::f64::consts::PI;
        let area = std::f64::consts::TAU / w as f64 * std::f64::consts::PI / h as f64 * theta.sin();
        for (i, excess) in [999., 799., 599.].into_iter().enumerate() {
            assert!((irradiance[i] / (excess * area) - 1.).abs() < 1e-6);
        }
    }
    #[test]
    fn packed_channels_keep_every_mip_bit_exact() {
        let (w, h) = (32, 16);
        let build = |offset: u32| {
            (0..w * h)
                .flat_map(|i| {
                    [
                        ((i * 977 + offset) % 65536) as f32 / 65535.,
                        ((i * 919 + offset) % 65536) as f32 / 65535.,
                        ((i * 853 + offset) % 65536) as f32 / 65535.,
                        1.,
                    ]
                })
                .collect::<Vec<_>>()
        };
        let originals = [build(1), build(2), build(3)];
        let sources = originals.clone().map(|pixels| ArchiveImageSource::Float {
            width: w,
            height: h,
            pixels,
        });
        let [colour, arm, normal] = sources;
        let packed = pack_set(colour, arm, normal).unwrap().map(|s| {
            let ArchiveImageSource::Float { pixels, .. } = s else {
                panic!()
            };
            pixels
        });
        let mut original = originals.map(|p| image::Rgba32FImage::from_raw(w, h, p).unwrap());
        let mut packed = packed.map(|p| image::Rgba32FImage::from_raw(w, h, p).unwrap());
        loop {
            for (((c, a), n), (pc, pn)) in original[0]
                .pixels()
                .zip(original[1].pixels())
                .zip(original[2].pixels())
                .zip(packed[0].pixels().zip(packed[1].pixels()))
            {
                for channel in 0..3 {
                    assert_eq!(c[channel].to_bits(), pc[channel].to_bits());
                    assert_eq!(n[channel].to_bits(), pn[channel].to_bits());
                }
                assert_eq!(a[0].to_bits(), pc[3].to_bits());
                assert_eq!(a[1].to_bits(), pn[3].to_bits());
            }
            let (w, h) = original[0].dimensions();
            if w == 1 && h == 1 {
                break;
            }
            original = original.map(|p| {
                image::imageops::resize(
                    &p,
                    (w / 2).max(1),
                    (h / 2).max(1),
                    image::imageops::FilterType::Triangle,
                )
            });
            packed = packed.map(|p| {
                image::imageops::resize(
                    &p,
                    (w / 2).max(1),
                    (h / 2).max(1),
                    image::imageops::FilterType::Triangle,
                )
            });
        }
    }
    #[test]
    fn colour_transfer_preserves_endpoints_and_midtones() {
        assert_eq!(srgb_to_linear(0.), 0.);
        assert_eq!(srgb_to_linear(1.), 1.);
        assert!((srgb_to_linear(0.5) - 0.21404114).abs() < 1e-7);
    }
}

pub fn sky() -> Result<voxy_render::HdrImageAsset, Box<dyn std::error::Error>> {
    let root = std::env::var_os("VOXY_STREET_MATERIALS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/environments/original-street/materials")
        });
    Ok(voxy_render::HdrImageAsset::decode(
        &std::fs::read(root.join("daylight.hdr"))?,
        voxy_render::ImageLimits {
            source_bytes: 64 * 1024 * 1024,
            pixel_bytes: 512 * 1024 * 1024,
            ..Default::default()
        },
    )?)
}

/// Match the shadow-casting sun to the brightest upper-hemisphere HDR source.
pub fn sun_direction(panorama: &voxy_render::HdrImageAsset) -> glam::Vec3 {
    let width = panorama.width() as usize;
    let (pixel, _) = panorama
        .rgb()
        .chunks_exact(3)
        .take(width * panorama.height() as usize / 2)
        .enumerate()
        .max_by(|(_, a), (_, b)| (a[0] + a[1] + a[2]).total_cmp(&(b[0] + b[1] + b[2])))
        .expect("nonempty bounded HDR");
    let u = (pixel % width) as f32 + 0.5;
    let v = (pixel / width) as f32 + 0.5;
    let longitude = (u / panorama.width() as f32 - 0.5) * std::f32::consts::TAU;
    let latitude = v / panorama.height() as f32 * std::f32::consts::PI;
    glam::Vec3::new(
        latitude.sin() * longitude.cos(),
        latitude.cos(),
        latitude.sin() * longitude.sin(),
    )
    .normalize()
}

/// Keep the display sky intact. Move only excess solar radiance from the
/// indirect map into the shadow-casting directional source, preserving its
/// integrated RGB irradiance rather than clamping HDR highlights.
pub fn daylight() -> Result<
    (
        voxy_render::archive_scene::ArchiveEnvironmentSource,
        glam::Vec3,
        [f32; 4],
    ),
    Box<dyn std::error::Error>,
> {
    let background = sky()?;
    let sun = sun_direction(&background);
    let (lighting, irradiance) = separate_sun(&background, sun)?;
    let intensity = 500.;
    let rgb = irradiance.map(|c| (c * intensity as f64) as f32);
    let peak = rgb.into_iter().fold(0.0_f32, f32::max);
    let colour = if peak > 0. {
        [rgb[0] / peak, rgb[1] / peak, rgb[2] / peak, peak]
    } else {
        [1., 1., 1., 0.]
    };
    eprintln!(
        "DAYLIGHT solar_irradiance={rgb:?} source_background_unchanged=true indirect_float32=true"
    );
    Ok((
        voxy_render::archive_scene::ArchiveEnvironmentSource {
            lighting,
            background,
            intensity,
        },
        sun,
        colour,
    ))
}
fn separate_sun(
    source: &voxy_render::HdrImageAsset,
    sun: glam::Vec3,
) -> Result<(voxy_render::HdrImageAsset, [f64; 3]), Box<dyn std::error::Error>> {
    let w = source.width() as usize;
    let h = source.height() as usize;
    let longitude: Vec<_> = (0..w)
        .map(|x| (((x as f64 + 0.5) / w as f64 - 0.5) * std::f64::consts::TAU).sin_cos())
        .collect();
    let sun = sun.as_dvec3();
    let core = 0.016_f64.cos();
    let outer = 0.04_f64.cos();
    let mut ambient = [0_f64; 3];
    let mut ambient_weight = 0.;
    for y in 0..h {
        let (sy, cy) = ((y as f64 + 0.5) / h as f64 * std::f64::consts::PI).sin_cos();
        for (x, &(sl, cl)) in longitude.iter().enumerate() {
            let cosine = sy * cl * sun.x + cy * sun.y + sy * sl * sun.z;
            if cosine < core && cosine > outer {
                let p = &source.rgb()[(y * w + x) * 3..][..3];
                for c in 0..3 {
                    ambient[c] += p[c] as f64 * sy;
                }
                ambient_weight += sy;
            }
        }
    }
    if ambient_weight > 0. {
        for c in &mut ambient {
            *c /= ambient_weight;
        }
    }
    let mut pixels = source.rgb().to_vec();
    let mut irradiance = [0_f64; 3];
    let angular_area = std::f64::consts::TAU / w as f64 * std::f64::consts::PI / h as f64;
    for y in 0..h {
        let (sy, cy) = ((y as f64 + 0.5) / h as f64 * std::f64::consts::PI).sin_cos();
        for (x, &(sl, cl)) in longitude.iter().enumerate() {
            let cosine = sy * cl * sun.x + cy * sun.y + sy * sl * sun.z;
            if cosine > core {
                for c in 0..3 {
                    let index = (y * w + x) * 3 + c;
                    let excess = (pixels[index] as f64 - ambient[c]).max(0.);
                    pixels[index] = (pixels[index] as f64 - excess) as f32;
                    irradiance[c] += excess * angular_area * sy * cosine;
                }
            }
        }
    }
    Ok((
        voxy_render::HdrImageAsset::from_rgb(
            source.width(),
            source.height(),
            pixels,
            voxy_render::ImageLimits {
                pixel_bytes: 512 * 1024 * 1024,
                ..Default::default()
            },
        )?,
        irradiance,
    ))
}
