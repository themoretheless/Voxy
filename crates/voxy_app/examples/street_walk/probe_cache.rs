//! Identify the exact inputs used to render the probe faces before GPU upload.
use crate::scene::StreetScene;
use std::path::PathBuf;
use voxy_render::archive_scene::{ArchiveEnvironmentSource, ArchiveImageSource};

pub(super) fn path(
    scene: &StreetScene,
    environment: Option<&ArchiveEnvironmentSource>,
    settings: &impl std::fmt::Debug,
) -> std::io::Result<Option<PathBuf>> {
    let Some(mut path) = std::env::var_os("VOXY_STREET_PROBE_CACHE") else {
        return Ok(None);
    };
    // Bake shader and pipeline changes must invalidate caches too.
    let mut hash = blake3::Hasher::new();
    hash.update_reader(std::fs::File::open(std::env::current_exe()?)?)?;
    let mut variables: Vec<_> = std::env::vars_os()
        .filter(|(name, _)| {
            let name = name.as_encoded_bytes();
            (name.starts_with(b"VOXY_") || name.starts_with(b"WGPU_"))
                && name != b"VOXY_STREET_PROBE_CACHE"
        })
        .collect();
    variables.sort_unstable();
    let mut add = |bytes: &[u8]| {
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    };
    add(format!("{settings:?}").as_bytes());
    add(&(variables.len() as u64).to_le_bytes());
    for (name, value) in variables {
        add(name.as_encoded_bytes());
        add(value.as_encoded_bytes());
    }
    hash_scene(add, scene, environment);
    path.push(".");
    path.push(hash.finalize().to_hex().as_str());
    path.push(".vprobes");
    Ok(Some(PathBuf::from(path)))
}

fn hash_scene(
    mut add: impl FnMut(&[u8]),
    scene: &StreetScene,
    environment: Option<&ArchiveEnvironmentSource>,
) {
    add(bytemuck::cast_slice(&scene.sun.to_array()));
    add(bytemuck::cast_slice(&scene.sun_colour));
    add(bytemuck::cast_slice(&scene.bounds.0.to_array()));
    add(bytemuck::cast_slice(&scene.bounds.1.to_array()));
    add(bytemuck::cast_slice(&scene.lights));
    add(format!("{:?}", scene.water).as_bytes());
    add(&(scene.geometry.len() as u64).to_le_bytes());
    for mesh in &scene.geometry {
        add(bytemuck::cast_slice(&mesh.vertices));
        add(bytemuck::cast_slice(&mesh.indices));
        add(bytemuck::cast_slice(&mesh.instances));
        add(&mesh.material.to_le_bytes());
        add(bytemuck::cast_slice(&mesh.center.to_array()));
    }
    add(&(scene.materials.len() as u64).to_le_bytes());
    for material in &scene.materials {
        add(bytemuck::bytes_of(&material.parameters));
        for image in material.images {
            add(&image.map_or(u64::MAX, |index| index as u64).to_le_bytes());
        }
        add(&[u8::from(material.double_sided)]);
    }
    add(&(scene.images.len() as u64).to_le_bytes());
    for image in &scene.images {
        match image {
            ArchiveImageSource::Bc7(image) => {
                add(format!("bc7 {:?} {} {}", image.format, image.width, image.height).as_bytes());
                add(&(image.mip_ranges.len() as u64).to_le_bytes());
                for mip in 0..image.mip_ranges.len() {
                    add(image.mip_bytes(mip).unwrap_or_default());
                }
            }
            ArchiveImageSource::Rgba(image) => {
                add(format!("rgba {} {}", image.width(), image.height()).as_bytes());
                add(image.rgba());
            }
            ArchiveImageSource::Float {
                width,
                height,
                pixels,
            } => {
                add(format!("float {width} {height}").as_bytes());
                add(bytemuck::cast_slice(pixels));
            }
        }
    }
    if let Some(environment) = environment {
        add(&environment.intensity.to_le_bytes());
        for image in [&environment.lighting, &environment.background] {
            add(&image.width().to_le_bytes());
            add(&image.height().to_le_bytes());
            add(bytemuck::cast_slice(image.rgb()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use voxy_render::archive_scene::{
        ArchiveLight, ArchiveMaterial, ArchiveMaterialSource, ArchiveMeshSource,
    };

    #[test]
    fn lighting_material_and_geometry_edits_invalidate_unchanged_probe_grid() {
        let mut scene = StreetScene {
            archive: None,
            images: vec![ArchiveImageSource::Float {
                width: 1,
                height: 1,
                pixels: vec![0.25; 4],
            }],
            materials: vec![ArchiveMaterialSource {
                parameters: ArchiveMaterial::MIRROR,
                images: [Some(0), None, None, None],
                double_sided: false,
            }],
            geometry: vec![ArchiveMeshSource {
                vertices: Vec::new(),
                indices: Vec::new(),
                instances: Vec::new(),
                material: 0,
                center: Vec3::ZERO,
            }],
            lights: vec![ArchiveLight {
                position_kind: [0.; 4],
                colour_intensity: [1.; 4],
            }],
            sun: Vec3::Y,
            sun_colour: [1.; 4],
            bounds: (Vec3::ZERO, Vec3::ONE),
            collision: physics::static_triangle_world::StaticTriangleWorld::new(vec![[
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 0., 1.],
            ]])
            .unwrap(),
            eye: Vec3::Y,
            direction: Vec3::Z,
            water: Vec::new(),
            pool_view: None,
        };
        let fingerprint = |scene: &StreetScene| {
            let mut hash = blake3::Hasher::new();
            hash_scene(
                |bytes| {
                    hash.update(&(bytes.len() as u64).to_le_bytes());
                    hash.update(bytes);
                },
                scene,
                None,
            );
            hash.finalize()
        };
        let original = fingerprint(&scene);
        assert_eq!(original, fingerprint(&scene));
        scene.lights[0].colour_intensity[3] = 2.;
        assert_ne!(original, fingerprint(&scene));
        scene.lights[0].colour_intensity[3] = 1.;
        scene.materials[0].double_sided = true;
        assert_ne!(original, fingerprint(&scene));
        scene.materials[0].double_sided = false;
        scene.geometry[0].center.x = 0.5;
        assert_ne!(original, fingerprint(&scene));
        scene.geometry[0].center.x = 0.;
        let ArchiveImageSource::Float { pixels, .. } = &mut scene.images[0] else {
            unreachable!()
        };
        pixels[0] = 0.5;
        assert_ne!(original, fingerprint(&scene));
        let with_texture_edit = fingerprint(&scene);
        scene.water.push(voxy_render::voxy_water::WaterBody::pool(
            Vec3::ZERO,
            Vec3::ONE,
        ));
        assert_ne!(with_texture_edit, fingerprint(&scene));
    }
}
