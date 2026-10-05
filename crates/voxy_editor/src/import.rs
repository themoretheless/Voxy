//! Project-scoped model import; external reads enter dependency observations.
use glam::{Mat4, Vec3};
use voxy_assets::{AssetId, FileInputs, ImportInputs, SourcePath};
use voxy_render::{
    ImageAsset, ImageLimits, ObjAsset, ObjLimits, SceneMesh, SceneVertex, TextureFilter,
    TextureSampling, TextureWrap,
};
use voxy_scene::Transform;
#[cfg(test)]
mod animated_tests;

#[derive(Clone, Debug)]
pub(crate) struct ImportedNode {
    pub name: String,
    pub parent: Option<usize>,
    pub local: Transform,
    pub mesh: Option<SceneMesh>,
    pub image: Option<usize>,
    pub sampling: TextureSampling,
    pub use_mips: bool,
    pub geometry_key: Option<(usize, usize)>,
}
#[derive(Clone, Debug)]
pub(crate) struct EditorAsset {
    pub mesh: SceneMesh,
    pub lod: Option<std::sync::Arc<voxy_render::CertifiedLodIndexSet>>,
    pub animated: Option<std::sync::Arc<voxy_render::ModelAsset>>,
    pub skinned_lod: Option<std::sync::Arc<voxy_render::SkinnedLodMesh>>,
    pub nodes: Vec<ImportedNode>,
    pub images: Vec<ImageAsset>,
}
impl From<ObjAsset> for EditorAsset {
    fn from(value: ObjAsset) -> Self {
        Self {
            mesh: value.mesh,
            lod: None,
            animated: None,
            skinned_lod: None,
            nodes: vec![],
            images: vec![],
        }
    }
}
impl EditorAsset {
    pub fn mesh_for(&self, part: Option<u32>) -> Option<&SceneMesh> {
        match part {
            None => Some(&self.mesh),
            Some(index) => self.nodes.get(index as usize)?.mesh.as_ref(),
        }
    }
}
fn external(source: &SourcePath, uri: &str) -> Result<AssetId, String> {
    if uri.contains(':') || uri.contains('%') || uri.contains('\\') {
        return Err("unsupported glTF URI".into());
    }
    let parent = std::path::Path::new(&source.observation_id().0)
        .parent()
        .unwrap_or(std::path::Path::new(""))
        .to_owned();
    let path = parent.join(uri);
    let name = path.to_str().ok_or("non-UTF8 glTF URI")?;
    Ok(SourcePath::new(name.to_owned())
        .map_err(|e| e.to_string())?
        .observation_id())
}
fn observe(
    provider: &FileInputs,
    inputs: &mut ImportInputs,
    id: AssetId,
) -> Result<Vec<u8>, String> {
    inputs
        .read(id, |id, limit| {
            provider.read(id, limit.min(16 * 1024 * 1024))
        })
        .map(|snapshot| snapshot.bytes.to_vec())
        .map_err(|e| format!("{e:?}"))
}
/// Bounded import: static material nodes/textures or explicit skeletal/clip data
/// through the renderer's model parser. Skeletal previews use the bind pose;
/// unsupported material maps, morphs and interpolation reject in that parser.
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
pub(crate) fn load(
    source: &SourcePath,
    provider: &FileInputs,
    inputs: &mut ImportInputs,
) -> Result<EditorAsset, String> {
    let bytes = observe(provider, inputs, source.observation_id())?;
    let id = source.observation_id();
    let extension = std::path::Path::new(&id.0)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "vmodel" {
        return load_lod_model(source, provider, inputs, &bytes);
    }
    if extension == "obj" {
        return ObjAsset::parse(
            std::str::from_utf8(&bytes).map_err(|e| e.to_string())?,
            ObjLimits {
                source_bytes: 16 * 1024 * 1024,
                attributes: 65_536,
                vertices: 65_536,
                triangles: 65_536,
            },
        )
        .map(Into::into)
        .map_err(|e| e.to_string());
    }
    if extension != "gltf" && extension != "glb" {
        return Err("expected OBJ, glTF, GLB or VMODEL".into());
    }
    let gltf = gltf::Gltf::from_slice(&bytes).map_err(|e| e.to_string())?;
    if gltf.extensions_required().next().is_some() {
        return Err("required glTF extensions unsupported".into());
    }
    if gltf.nodes().count() > 128 || gltf.buffers().count() > 16 || gltf.images().count() > 16 {
        return Err("glTF object budget exceeded".into());
    }
    let mut buffers = Vec::new();
    for buffer in gltf.buffers() {
        let data = match buffer.source() {
            gltf::buffer::Source::Bin => gltf.blob.clone().ok_or("GLB missing BIN chunk")?,
            gltf::buffer::Source::Uri(uri) => observe(provider, inputs, external(source, uri)?)?,
        };
        if data.len() < buffer.length() {
            return Err("truncated glTF buffer".into());
        }
        buffers.push(data);
    }
    // Reader utilities assume valid accessor ranges; check every view/accessor first.
    for view in gltf.views() {
        if view
            .offset()
            .checked_add(view.length())
            .is_none_or(|end| end > buffers[view.buffer().index()].len())
        {
            return Err("glTF view outside buffer".into());
        }
    }
    for accessor in gltf.accessors() {
        if accessor.sparse().is_some() {
            return Err("sparse accessors unsupported".into());
        }
        let view = accessor.view().ok_or("accessor missing view")?;
        let stride = view.stride().unwrap_or(accessor.size());
        let end = accessor
            .count()
            .checked_sub(1)
            .and_then(|n| n.checked_mul(stride))
            .and_then(|n| n.checked_add(accessor.offset()))
            .and_then(|n| n.checked_add(accessor.size()));
        if end.is_none_or(|end| end > view.length()) {
            return Err("glTF accessor outside view".into());
        }
    }
    let images = if gltf
        .buffers()
        .all(|buffer| matches!(buffer.source(), gltf::buffer::Source::Bin))
        && gltf
            .images()
            .all(|image| matches!(image.source(), gltf::image::Source::View { .. }))
    {
        voxy_render::ModelAsset::decode_embedded_images(
            &bytes,
            ImageLimits {
                source_bytes: 16 * 1024 * 1024,
                dimension: 2048,
                pixel_bytes: 32 * 1024 * 1024,
            },
            16,
        )
        .map_err(|error| error.to_string())?
    } else {
        let mut images = Vec::new();
        for image in gltf.images() {
            let data = match image.source() {
                gltf::image::Source::View { view, .. } => buffers[view.buffer().index()]
                    [view.offset()..view.offset() + view.length()]
                    .to_vec(),
                gltf::image::Source::Uri { uri, .. } => {
                    observe(provider, inputs, external(source, uri)?)?
                }
            };
            images.push(
                ImageAsset::decode(
                    &data,
                    ImageLimits {
                        source_bytes: 16 * 1024 * 1024,
                        dimension: 2048,
                        pixel_bytes: 32 * 1024 * 1024
                            - u64::try_from(
                                images
                                    .iter()
                                    .map(|image: &ImageAsset| image.rgba().len())
                                    .sum::<usize>(),
                            )
                            .map_err(|e| e.to_string())?,
                    },
                )
                .map_err(|e| e.to_string())?,
            );
        }
        images
    };
    if images.iter().map(|image| image.rgba().len()).sum::<usize>() > 32 * 1024 * 1024 {
        return Err("decoded image budget exceeded".into());
    }
    if gltf.animations().next().is_some() || gltf.skins().next().is_some() {
        return load_animated_model(&bytes, &buffers, images);
    }
    let raw: Vec<_> = gltf.nodes().collect();
    let mut parents = vec![None; raw.len()];
    for node in &raw {
        for child in node.children() {
            if parents[child.index()].replace(node.index()).is_some() {
                return Err("glTF node has multiple parents".into());
            }
        }
    }
    let mut order = Vec::new();
    while order.len() < raw.len() {
        let before = order.len();
        for (index, parent) in parents.iter().enumerate() {
            if !order.contains(&index) && parent.is_none_or(|p| order.contains(&p)) {
                order.push(index);
            }
        }
        if before == order.len() {
            return Err("cyclic glTF hierarchy".into());
        }
    }
    let mut remap = vec![0; raw.len()];
    for (index, &raw_index) in order.iter().enumerate() {
        remap[raw_index] = index;
    }
    let mut nodes: Vec<ImportedNode> = Vec::new();
    let mut primitive_nodes = Vec::new();
    let mut worlds: Vec<Mat4> = Vec::new();
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();
    for raw_index in order {
        let node = &raw[raw_index];
        let (translation, rotation, scale) = node.transform().decomposed();
        let local = Transform {
            translation: Vec3::from_array(translation),
            rotation: glam::Quat::from_array(rotation),
            scale: Vec3::from_array(scale),
        };
        let matrix =
            Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation);
        if !matrix.is_finite()
            || !matrix.abs_diff_eq(Mat4::from_cols_array_2d(&node.transform().matrix()), 1e-4)
        {
            return Err("glTF transform must be finite TRS".into());
        }
        let parent = parents[raw_index].map(|parent| remap[parent]);
        let world = parent.map_or(matrix, |parent| worlds[parent] * matrix);
        worlds.push(world);
        let mut imported = ImportedNode {
            name: node.name().unwrap_or("glTF node").to_owned(),
            parent,
            local,
            mesh: None,
            image: None,
            sampling: TextureSampling::default(),
            use_mips: false,
            geometry_key: None,
        };
        if let Some(mesh) = node.mesh() {
            let count = mesh.primitives().count();
            if count == 0 {
                return Err("empty glTF mesh".into());
            }
            if raw.len() + primitive_nodes.len() + if count > 1 { count } else { 0 } > 128 {
                return Err("glTF primitive node budget exceeded".into());
            }
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles
                    || primitive.morph_targets().next().is_some()
                {
                    return Err("triangle primitives without morphs required".into());
                }
                let mut part = imported.clone();
                part.geometry_key = Some((mesh.index(), primitive.index()));
                let material = primitive.material();
                let pbr = material.pbr_metallic_roughness();
                if pbr.metallic_roughness_texture().is_some()
                    || material.normal_texture().is_some()
                    || material.occlusion_texture().is_some()
                    || material.emissive_texture().is_some()
                    || material.alpha_mode() != gltf::material::AlphaMode::Opaque
                {
                    return Err("only opaque base-color materials supported".into());
                }
                if let Some(texture) = pbr.base_color_texture() {
                    if texture.tex_coord() != 0 {
                        return Err("base color requires TEXCOORD_0".into());
                    }
                    part.image = Some(texture.texture().source().index());
                    let sampler = texture.texture().sampler();
                    let wrap = |mode| match mode {
                        gltf::texture::WrappingMode::ClampToEdge => TextureWrap::Clamp,
                        gltf::texture::WrappingMode::Repeat => TextureWrap::Repeat,
                        gltf::texture::WrappingMode::MirroredRepeat => TextureWrap::Mirror,
                    };
                    part.use_mips = !matches!(
                        sampler.min_filter(),
                        Some(gltf::texture::MinFilter::Nearest | gltf::texture::MinFilter::Linear)
                    );
                    part.sampling = TextureSampling {
                        wrap_u: wrap(sampler.wrap_s()),
                        wrap_v: wrap(sampler.wrap_t()),
                        min_filter: match sampler.min_filter() {
                            Some(
                                gltf::texture::MinFilter::Nearest
                                | gltf::texture::MinFilter::NearestMipmapNearest
                                | gltf::texture::MinFilter::NearestMipmapLinear,
                            ) => TextureFilter::Nearest,
                            _ => TextureFilter::Linear,
                        },
                        mag_filter: if sampler.mag_filter()
                            == Some(gltf::texture::MagFilter::Nearest)
                        {
                            TextureFilter::Nearest
                        } else {
                            TextureFilter::Linear
                        },
                        mipmap_filter: Some(match sampler.min_filter() {
                            Some(
                                gltf::texture::MinFilter::NearestMipmapNearest
                                | gltf::texture::MinFilter::LinearMipmapNearest,
                            ) => TextureFilter::Nearest,
                            _ => TextureFilter::Linear,
                        }),
                        anisotropy: 1,
                    };
                }
                let position_count = primitive
                    .get(&gltf::Semantic::Positions)
                    .ok_or("missing positions")?
                    .count();
                if position_count > 65_536
                    || all_vertices.len() + position_count > 65_536
                    || primitive
                        .indices()
                        .is_some_and(|accessor| accessor.count() > 196_608)
                {
                    return Err("glTF geometry budget exceeded".into());
                }
                let reader = primitive.reader(|buffer| Some(buffers[buffer.index()].as_slice()));
                let positions: Vec<_> = reader
                    .read_positions()
                    .ok_or("missing glTF positions")?
                    .collect();
                if positions.len() > 65_536
                    || all_vertices
                        .len()
                        .checked_add(positions.len())
                        .is_none_or(|count| count > 65_536)
                {
                    return Err("glTF vertex budget exceeded".into());
                }
                let uvs: Vec<_> = reader.read_tex_coords(0).map_or_else(
                    || vec![[0.; 2]; positions.len()],
                    |uv| uv.into_f32().collect(),
                );
                if uvs.len() != positions.len()
                    || (part.image.is_some() && reader.read_tex_coords(0).is_none())
                {
                    return Err("glTF UV count mismatch or missing texture UV".into());
                }
                let colors: Vec<_> = reader.read_colors(0).map_or_else(
                    || vec![[1.; 4]; positions.len()],
                    |colors| colors.into_rgba_f32().collect(),
                );
                if colors.len() != positions.len() {
                    return Err("glTF color count mismatch".into());
                }
                let factor = pbr.base_color_factor();
                let vertices: Vec<_> = positions
                    .into_iter()
                    .zip(uvs)
                    .zip(colors)
                    .map(|((position, uv), color)| SceneVertex {
                        position,
                        uv,
                        color: std::array::from_fn(|i| color[i] * factor[i]),
                    })
                    .collect();
                let indices: Vec<_> = reader.read_indices().map_or_else(
                    || (0..vertices.len() as u32).collect(),
                    |indices| indices.into_u32().collect(),
                );
                if all_indices
                    .len()
                    .checked_add(indices.len())
                    .is_none_or(|count| count > 196_608)
                {
                    return Err("glTF index budget exceeded".into());
                }
                part.mesh = Some(
                    SceneMesh::new(vertices.clone(), indices.clone()).map_err(|e| e.to_string())?,
                );
                let base = all_vertices.len() as u32;
                all_vertices.extend(vertices.into_iter().map(|vertex| {
                    SceneVertex {
                        position: world
                            .transform_point3(Vec3::from_array(vertex.position))
                            .to_array(),
                        ..vertex
                    }
                }));
                all_indices.extend(indices.into_iter().map(|index| index + base));
                if count == 1 {
                    imported = part;
                } else {
                    part.parent = Some(remap[raw_index]);
                    part.local = Transform::default();
                    part.name = format!("{}/primitive-{}", imported.name, primitive.index());
                    primitive_nodes.push(part);
                }
            }
        }
        nodes.push(imported);
    }
    nodes.extend(primitive_nodes);
    Ok(EditorAsset {
        mesh: SceneMesh::new(all_vertices, all_indices).map_err(|e| e.to_string())?,
        lod: None,
        animated: None,
        skinned_lod: None,
        nodes,
        images,
    })
}

fn load_animated_model(
    bytes: &[u8],
    buffers: &[Vec<u8>],
    images: Vec<ImageAsset>,
) -> Result<EditorAsset, String> {
    let buffers: Vec<_> = buffers.iter().map(Vec::as_slice).collect();
    let model = voxy_render::ModelAsset::parse(
        bytes,
        &buffers,
        voxy_render::ModelLimits {
            bytes: 32 * 1024 * 1024,
            vertices: 65_536,
            indices: 196_608,
            keys: 65_536,
        },
    )
    .map_err(|error| format!("animated model import: {error}"))?;
    if model.primitives.len() > 128 {
        return Err("animated primitive budget exceeded".into());
    }
    let pose = model.skeleton.bind_pose();
    let meshes = model
        .scene_meshes(&pose)
        .map_err(|error| error.to_string())?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    for mesh in meshes {
        let offset = u32::try_from(vertices.len()).map_err(|error| error.to_string())?;
        vertices.extend_from_slice(mesh.vertices());
        if let Some(stream) = mesh.authored_normals() {
            normals.extend_from_slice(stream);
        } else {
            normals.resize(normals.len() + mesh.vertices().len(), [0.; 3]);
        }
        indices.extend(mesh.indices().iter().map(|index| index + offset));
    }
    Ok(EditorAsset {
        mesh: SceneMesh::new(vertices, indices)
            .and_then(|mesh| mesh.with_normals(normals))
            .map_err(|error| error.to_string())?,
        animated: Some(std::sync::Arc::new(model)),
        skinned_lod: None,
        lod: None,
        nodes: Vec::new(),
        images,
    })
}

/// Explicit versioned model recipe; neither filename probing nor recursive recipes.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LodModelRecipe {
    version: u32,
    base: String,
    certificate: String,
}
fn load_lod_model(
    source: &SourcePath,
    provider: &FileInputs,
    inputs: &mut ImportInputs,
    bytes: &[u8],
) -> Result<EditorAsset, String> {
    if bytes.len() > 4096 {
        return Err("model recipe byte budget exceeded".into());
    }
    let recipe: LodModelRecipe = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if recipe.version != 1 {
        return Err("unsupported model recipe version".into());
    }
    let base_id = external(source, &recipe.base)?;
    let extension = std::path::Path::new(&base_id.0)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or("LOD model base missing extension")?
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "obj" | "gltf" | "glb") {
        return Err("LOD model base must be OBJ, glTF or GLB".into());
    }
    let base_source = SourcePath::new(base_id.0).map_err(|e| e.to_string())?;
    let mut asset = load(&base_source, provider, inputs)?;
    let certificate = observe(provider, inputs, external(source, &recipe.certificate)?)?;
    let limits = voxy_render::LodArchiveLimits {
        bytes: 16 * 1024 * 1024,
        positions: 65_536,
        levels: 8,
        indices: 1_572_864,
        cells: 262_144,
    };
    if let Some(model) = &asset.animated {
        if model.primitives.len() != 1 {
            return Err("skeletal LOD requires one primitive".into());
        }
        let voxy_render::ModelGeometry::Skinned(mesh) = &model.primitives[0].geometry else {
            return Err("animated LOD requires a skinned primitive".into());
        };
        let source = voxy_render::decode_skinned_lod_archive(
            std::sync::Arc::new(mesh.clone()),
            &certificate,
            limits,
        )
        .map_err(|error| format!("skeletal LOD verification: {error}"))?;
        // Validate the bind palette now, before source/dependency publication.
        let palette = model
            .skin_matrices(&model.skeleton.bind_pose())
            .map_err(|error| error.to_string())?;
        source
            .prepare(&palette, Mat4::IDENTITY)
            .map_err(|error| error.to_string())?;
        asset.skinned_lod = Some(std::sync::Arc::new(source));
        return Ok(asset);
    }
    let artifact = voxy_render::decode_lod_archive(&certificate, limits)
        .map_err(|e| format!("LOD certificate verification: {e:?}"))?;
    voxy_render::SceneRenderer::certified_lod_mesh_allocation_bytes(&asset.mesh, &artifact)
        .map_err(|e| format!("LOD base identity: {e:?}"))?;
    asset.lod = Some(std::sync::Arc::new(artifact));
    Ok(asset)
}

#[cfg(test)]
mod lod_model_tests {
    use super::*;
    #[test]
    fn model_certificate_identity_and_dependency_publication() {
        let root = std::env::temp_dir().join(format!("voxy-lod-model-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let obj = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        std::fs::write(root.join("base.obj"), obj).unwrap();
        let base = ObjAsset::parse(obj, ObjLimits::default()).unwrap();
        let positions: Vec<_> = base.mesh.vertices().iter().map(|v| v.position).collect();
        let indices = base.mesh.indices();
        let surface = voxy_render::LodSurface {
            positions: &positions,
            indices,
        };
        let witnesses =
            voxy_render::generate_subdivided_lod_witnesses(surface, surface, 0, 100).unwrap();
        let bytes = voxy_render::encode_lod_archive(
            &positions,
            indices,
            &[voxy_render::CertifiedLodSubdivisionVariant {
                indices: indices.to_vec(),
                source_to_variant: witnesses.source_to_approximation,
                variant_to_source: witnesses.approximation_to_source,
            }],
            voxy_render::LodArchiveLimits {
                bytes: 4096,
                positions: 3,
                levels: 2,
                indices: 6,
                cells: 2,
            },
        )
        .unwrap();
        std::fs::write(root.join("proof.lod"), &bytes).unwrap();
        std::fs::write(
            root.join("mesh.vmodel"),
            br#"{"version":1,"base":"base.obj","certificate":"proof.lod"}"#,
        )
        .unwrap();
        let source = SourcePath::new("mesh.vmodel").unwrap();
        let provider = FileInputs::new(&root).unwrap();
        let mut inputs = ImportInputs::new(3, 32 * 1024);
        let asset = load(&source, &provider, &mut inputs).unwrap();
        assert_eq!(inputs.observations().len(), 3);
        assert_eq!(asset.lod.as_ref().unwrap().indices().levels().len(), 2);
        let mut app = crate::App::new(&root.join("mesh.vmodel"), false).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.catalog.snapshot(&app.id).is_none() {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let last_good = app.catalog.snapshot(&app.id).unwrap();
        std::fs::write(root.join("proof.lod"), b"changed").unwrap();
        while app.failed_at.is_none() {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(std::sync::Arc::ptr_eq(
            &last_good,
            &app.catalog.snapshot(&app.id).unwrap()
        ));

        assert!(
            inputs
                .finish(asset, |id, limit| provider.read(id, limit))
                .is_err()
        );
        assert!(load(&source, &provider, &mut ImportInputs::new(3, 32 * 1024)).is_err());
        std::fs::write(root.join("proof.lod"), &bytes).unwrap();
        while std::sync::Arc::ptr_eq(&last_good, &app.catalog.snapshot(&app.id).unwrap()) {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(app.catalog.snapshot(&app.id).unwrap().value().lod.is_some());
        app.stop_workers().unwrap();
        std::fs::write(root.join("base.obj"), obj.replace("v 1 0 0", "v 2 0 0")).unwrap();
        assert!(
            load(&source, &provider, &mut ImportInputs::new(3, 32 * 1024))
                .unwrap_err()
                .contains("LOD base identity")
        );
        std::fs::write(
            root.join("mesh.vmodel"),
            br#"{"version":2,"base":"base.obj","certificate":"proof.lod"}"#,
        )
        .unwrap();
        assert!(load(&source, &provider, &mut ImportInputs::new(3, 32 * 1024)).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
