//! CC0 scanned surfaces on independently authored original street geometry.
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
pub fn apply(scene: &mut StreetScene) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var_os("VOXY_STREET_MATERIALS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/environments/original-street/materials")
        });
    let mut sets = Vec::new();
    for asset in ["plastered_stone_wall", "brick_wall_001", "bark_willow"] {
        let mut ids = [0; 3];
        for (i, channel) in ["Diffuse", "arm", "nor_gl"].iter().enumerate() {
            ids[i] = scene.images.len();
            scene
                .images
                .push(load(&root.join(format!("{asset}-{channel}.png")), i == 0)?);
        }
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
            material.images = [Some(ids[0]), Some(ids[1]), Some(ids[2]), None];
            material.parameters.workflow[1] = 1.;
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
        "ORIGINAL STREET MATERIALS provider=PolyHaven license=CC0 scans=3 maps={} source_resolution=4k gpu=linear_rgba32float",
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
