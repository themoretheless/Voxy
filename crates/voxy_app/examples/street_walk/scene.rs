use glam::{DVec3, Vec3};
use physics::static_triangle_world::StaticTriangleWorld;
use std::{path::Path, sync::Arc};
use voxy_model::{SceneArchive, SceneArchiveTangents, SceneImportLimits};
use voxy_render::{Bc7ImageAsset, archive_scene::*};
pub struct StreetScene {
    pub archive: Option<SceneArchive>,
    pub images: Vec<ArchiveImageSource>,
    pub materials: Vec<ArchiveMaterialSource>,
    pub geometry: Vec<ArchiveMeshSource>,
    pub lights: Vec<ArchiveLight>,
    pub sun: Vec3,
    pub sun_colour: [f32; 4],
    pub bounds: (Vec3, Vec3),
    pub collision: StaticTriangleWorld,
    pub eye: Vec3,
    pub direction: Vec3,
    pub water: Vec<voxy_render::ArchiveWaterBody>,
    /// Eye and direction overlooking the pool, for `--spawn pool`.
    pub pool_view: Option<(Vec3, Vec3)>,
}
fn array<const N: usize>(v: &serde_json::Value, default: [f32; N]) -> [f32; N] {
    std::array::from_fn(|i| v[i].as_f64().map_or(default[i], |x| x as f32))
}
pub fn load(root: &Path) -> Result<StreetScene, Box<dyn std::error::Error>> {
    let source = std::fs::read(root.join("bistro.gltf"))?;
    let json: serde_json::Value = serde_json::from_slice(&source)?;
    let buffers = json["buffers"]
        .as_array()
        .ok_or("missing buffers")?
        .iter()
        .map(|b| {
            std::fs::read(root.join(b["uri"].as_str().ok_or("missing buffer URI")?))
                .map(Arc::<[u8]>::from)
                .map_err(Box::<dyn std::error::Error>::from)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut archive = SceneArchive::decode(&source, buffers, SceneImportLimits::default())?;
    let mut repairs = 0;
    for primitives in &mut archive.meshes {
        for p in primitives {
            repairs += p.repair_tangents(0)?;
        }
    }
    let mut images = Vec::new();
    let mut image_lookup = std::collections::BTreeMap::new();
    let mut texture_images = Vec::new();
    for texture in json["textures"].as_array().ok_or("missing textures")? {
        let id = texture["extensions"]["MSFT_texture_dds"]["source"]
            .as_u64()
            .ok_or("DDS source missing")? as usize;
        let index = if let Some(i) = image_lookup.get(&id) {
            *i
        } else {
            let uri = json["images"][id]["uri"]
                .as_str()
                .ok_or("missing image URI")?;
            let mut image = Bc7ImageAsset::decode_dds(
                std::fs::read(root.join(uri))?.into(),
                256 * 1024 * 1024,
                16384,
            )?;
            // Source roles define transfer functions. Sample raw unchanged BC7 values, decode colour maps in shader.
            image.format = wgpu::TextureFormat::Bc7RgbaUnorm;
            let i = images.len();
            images.push(ArchiveImageSource::Bc7(image));
            image_lookup.insert(id, i);
            i
        };
        texture_images.push(index);
    }
    let texture = |v: &serde_json::Value| -> Result<Option<usize>, Box<dyn std::error::Error>> {
        if v.is_null() {
            return Ok(None);
        }
        if v["texCoord"].as_u64().unwrap_or(0) != 0 {
            return Err("nonzero material UV set unsupported".into());
        }
        let i = v["index"].as_u64().ok_or("missing texture index")? as usize;
        Ok(Some(
            *texture_images.get(i).ok_or("texture index out of bounds")?,
        ))
    };
    let mut materials = Vec::new();
    for m in json["materials"].as_array().ok_or("missing materials")? {
        let spec = &m["extensions"]["KHR_materials_pbrSpecularGlossiness"];
        let mr = &m["pbrMetallicRoughness"];
        let workflow = f32::from(spec.is_null());
        let colour = if spec.is_null() {
            &mr["baseColorFactor"]
        } else {
            &spec["diffuseFactor"]
        };
        let spec_factor = array(&spec["specularFactor"], [1.; 3]);
        let alpha = match m["alphaMode"].as_str().unwrap_or("OPAQUE") {
            "MASK" => m["alphaCutoff"].as_f64().unwrap_or(0.5) as f32,
            "BLEND" => -2.,
            _ => -1.,
        };
        let emissive = array(&m["emissiveFactor"], [0.; 3]);
        materials.push(ArchiveMaterialSource {
            parameters: ArchiveMaterial {
                diffuse: array(colour, [1.; 4]),
                specular_gloss: [
                    spec_factor[0],
                    spec_factor[1],
                    spec_factor[2],
                    spec["glossinessFactor"].as_f64().unwrap_or(1.) as f32,
                ],
                emissive_normal: [
                    emissive[0],
                    emissive[1],
                    emissive[2],
                    m["normalTexture"]["scale"].as_f64().unwrap_or(1.) as f32,
                ],
                surface: [
                    mr["metallicFactor"].as_f64().unwrap_or(1.) as f32,
                    mr["roughnessFactor"].as_f64().unwrap_or(1.) as f32,
                    m["extensions"]["KHR_materials_transmission"]["transmissionFactor"]
                        .as_f64()
                        .unwrap_or(0.) as f32,
                    alpha,
                ],
                workflow: [workflow, f32::from(!m["normalTexture"].is_null()), 0., 0.],
            },
            images: [
                texture(if spec.is_null() {
                    &mr["baseColorTexture"]
                } else {
                    &spec["diffuseTexture"]
                })?,
                texture(if spec.is_null() {
                    &mr["metallicRoughnessTexture"]
                } else {
                    &spec["specularGlossinessTexture"]
                })?,
                texture(&m["normalTexture"])?,
                texture(&m["emissiveTexture"])?,
            ],
            double_sided: m["doubleSided"].as_bool().unwrap_or(false),
        });
    }
    let mut active = vec![false; archive.nodes.len()];
    let mut stack = archive.scene_roots[archive.default_scene.unwrap_or(0)].clone();
    while let Some(i) = stack.pop() {
        active[i] = true;
        stack.extend(archive.nodes[i].children.iter().copied());
    }
    let mut instances = vec![Vec::new(); archive.meshes.len()];
    let mut triangles = Vec::new();
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for (i, node) in archive.nodes.iter().enumerate() {
        if !active[i] {
            continue;
        }
        if let Some(mesh) = node.mesh {
            if node.skin.is_some() {
                return Err("street skin requires animated extraction".into());
            }
            instances[mesh].push(ArchiveInstance::new(node.world.as_mat4()));
            for primitive in &archive.meshes[mesh] {
                for ids in primitive.indices.chunks_exact(3) {
                    let t = std::array::from_fn(|j| {
                        node.world
                            .transform_point3(DVec3::from_array(
                                primitive.positions[ids[j] as usize].map(f64::from),
                            ))
                            .to_array()
                    });
                    for p in t {
                        let p = DVec3::from_array(p).as_vec3();
                        low = low.min(p);
                        high = high.max(p);
                    }
                    triangles.push(t);
                }
            }
        }
    }
    let mut geometry = Vec::new();
    for (mesh, primitives) in archive.meshes.iter().enumerate() {
        for p in primitives {
            let normals = p.normals.as_ref().ok_or("authored normals missing")?;
            let tangents = match p.tangents.as_ref() {
                Some(SceneArchiveTangents::Finite(v)) => v,
                _ => return Err("finite tangents missing".into()),
            };
            let uv = &p
                .texcoords
                .iter()
                .find(|(i, _)| *i == 0)
                .ok_or("UV0 missing")?
                .1;
            let vertices = (0..p.positions.len())
                .map(|i| ArchiveVertex {
                    position: p.positions[i],
                    normal: normals[i],
                    tangent: tangents[i],
                    uv: uv[i],
                })
                .collect();
            let centre = instances[mesh].first().map_or(Vec3::ZERO, |i| {
                glam::Mat4::from_cols_array_2d(&i.world).transform_point3(Vec3::ZERO)
            });
            geometry.push(ArchiveMeshSource {
                vertices,
                indices: p.indices.clone(),
                instances: instances[mesh].clone(),
                material: p.material.ok_or("authored material missing")?,
                center: centre,
            });
        }
    }
    let mut lights = Vec::new();
    let mut sun = Vec3::Y;
    let mut sun_colour = [1., 1., 1., 6830.];
    let mut eye = Vec3::new(-26.43, 3.156, 11.17);
    let mut direction = Vec3::NEG_Z;
    for (i, node) in archive.nodes.iter().enumerate() {
        if !active[i] {
            continue;
        }
        let raw = &json["nodes"][i];
        if raw["camera"].as_u64() == Some(0) {
            eye = node.world.transform_point3(DVec3::ZERO).as_vec3();
            direction = node
                .world
                .transform_vector3(DVec3::NEG_Z)
                .as_vec3()
                .normalize();
        }
        if let Some(id) = raw["extensions"]["KHR_lights_punctual"]["light"].as_u64() {
            let light = &json["extensions"]["KHR_lights_punctual"]["lights"][id as usize];
            let colour = array(&light["color"], [1.; 3]);
            let intensity = light["intensity"].as_f64().unwrap_or(1.) as f32;
            if light["type"] == "directional" {
                sun = node.world.transform_vector3(DVec3::Z).as_vec3().normalize();
                sun_colour = [colour[0], colour[1], colour[2], intensity];
            } else if light["type"] == "point" {
                let p = node.world.transform_point3(DVec3::ZERO).as_vec3();
                lights.push(ArchiveLight {
                    position_kind: [p.x, p.y, p.z, light["range"].as_f64().unwrap_or(0.) as f32],
                    colour_intensity: [colour[0], colour[1], colour[2], intensity],
                });
            } else {
                return Err("unsupported authored light".into());
            }
        }
    }
    eprintln!(
        "STREET source: {} images, {} materials, {} batches, {} full collision triangles, {repairs} repaired tangents; bounds {low:?}..{high:?}",
        images.len(),
        materials.len(),
        geometry.len(),
        triangles.len()
    );
    let collision = StaticTriangleWorld::new(triangles)?;
    Ok(StreetScene {
        archive: Some(archive),
        images,
        materials,
        geometry,
        lights,
        sun,
        sun_colour,
        bounds: (low, high),
        collision,
        eye,
        direction,
        water: Vec::new(),
        pool_view: None,
    })
}
