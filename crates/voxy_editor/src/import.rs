//! Project-scoped model import; external reads enter dependency observations.
use crate::EditorError;
use glam::{Mat4, Vec3};
use voxy_assets::{AssetId, FileInputs, ImportInputs, SourcePath};
use voxy_render::{
    ImageAsset, ObjAsset, ObjLimits, SceneMesh, TextureFilter, TextureSampling, TextureWrap,
};
#[cfg(test)]
use voxy_render::{ImageLimits, SceneVertex};
use voxy_scene::Transform;
#[cfg(test)]
mod animated_tests;
pub(crate) mod asset_checkpoint;
pub(crate) mod catalog_checkpoint;
pub(crate) mod node_checkpoint;
pub(crate) mod publication_checkpoint;

/// Per-source bound shared by model import and its content watcher.
pub(crate) const MODEL_INPUTS: usize = 34;
pub(crate) const MODEL_INPUT_BYTES: usize = 32 * 1024 * 1024;
pub(crate) const SOURCE_BYTES: usize = 16 * 1024 * 1024;

// Current common editor/build manifest policy; descriptor/scene budgets differ.
pub(crate) const MANIFEST_ENTRIES: usize = 128;
pub(crate) const MANIFEST_BYTES: usize = 64 * 1024;
pub(crate) fn manifest_locations(bytes: &[u8]) -> Result<voxy_assets::AssetLocations, EditorError> {
    voxy_assets::AssetLocations::from_json(
        std::str::from_utf8(bytes)?,
        MANIFEST_ENTRIES,
        MANIFEST_BYTES,
        MANIFEST_BYTES,
    )
    .map_err(EditorError::from)
}

pub(crate) fn model_inventory(
    bytes: &[u8],
) -> Result<std::collections::BTreeSet<AssetId>, EditorError> {
    let locations = manifest_locations(bytes)?;
    Ok(locations
        .bindings()
        .filter(|(_, source)| {
            std::path::Path::new(source.as_str())
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| {
                    matches!(
                        e.to_ascii_lowercase().as_str(),
                        "obj" | "gltf" | "glb" | "vmodel"
                    )
                })
        })
        .map(|(id, _)| id.clone())
        .collect())
}

pub(crate) use voxy_model::ImportedNode;
#[derive(Clone, Debug)]
pub(crate) struct EditorAsset {
    pub mesh: SceneMesh,
    pub lod: Option<std::sync::Arc<voxy_render::CertifiedLodIndexSet>>,
    pub animated: Option<std::sync::Arc<voxy_model::ModelAsset>>,
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
fn external(source: &SourcePath, uri: &str) -> Result<AssetId, EditorError> {
    Ok(source.sibling(uri)?.observation_id())
}
fn observe(
    provider: &FileInputs,
    inputs: &mut ImportInputs,
    id: AssetId,
) -> Result<Vec<u8>, EditorError> {
    inputs
        .read(id, |id, limit| provider.read(id, limit.min(SOURCE_BYTES)))
        .map(|snapshot| snapshot.bytes.to_vec())
        .map_err(|e| format!("{e:?}"))
        .map_err(Into::into)
}
/// Bounded import: static material nodes/textures or explicit skeletal/clip data
/// through the renderer's model parser. Skeletal previews use the bind pose;
/// unsupported material maps, morphs and interpolation reject in that parser.
pub(crate) fn load(
    source: &SourcePath,
    provider: &FileInputs,
    inputs: &mut ImportInputs,
) -> Result<EditorAsset, EditorError> {
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
            std::str::from_utf8(&bytes)?,
            ObjLimits {
                source_bytes: 16 * 1024 * 1024,
                attributes: 65_536,
                vertices: 65_536,
                triangles: 65_536,
            },
        )
        .map(Into::into)
        .map_err(EditorError::from);
    }
    if extension != "gltf" && extension != "glb" {
        return Err("expected OBJ, glTF, GLB or VMODEL".into());
    }
    match voxy_model::import_project_gltf(&bytes, |uri| {
        observe(provider, inputs, external(source, uri)?)
    })? {
        voxy_model::ImportedGltf::Static {
            mesh,
            nodes,
            images,
        } => Ok(EditorAsset {
            mesh,
            nodes,
            images,
            lod: None,
            animated: None,
            skinned_lod: None,
        }),
        voxy_model::ImportedGltf::Animated { buffers, images } => {
            load_animated_model(&bytes, &buffers, images)
        }
    }
}

fn load_animated_model(
    bytes: &[u8],
    buffers: &[Vec<u8>],
    images: Vec<ImageAsset>,
) -> Result<EditorAsset, EditorError> {
    let buffers: Vec<_> = buffers.iter().map(Vec::as_slice).collect();
    let model = voxy_model::ModelAsset::parse_animated(
        bytes,
        &buffers,
        voxy_model::ModelLimits {
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
    let meshes = model.scene_meshes_frame(&model.bind_frame()?)?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    for mesh in meshes {
        let offset = u32::try_from(vertices.len())?;
        vertices.extend_from_slice(mesh.vertices());
        if let Some(stream) = mesh.authored_normals() {
            normals.extend_from_slice(stream);
        } else {
            normals.resize(normals.len() + mesh.vertices().len(), [0.; 3]);
        }
        indices.extend(mesh.indices().iter().map(|index| index + offset));
    }
    Ok(EditorAsset {
        mesh: SceneMesh::new(vertices, indices).and_then(|mesh| mesh.with_normals(normals))?,
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
) -> Result<EditorAsset, EditorError> {
    if bytes.len() > 4096 {
        return Err("model recipe byte budget exceeded".into());
    }
    let recipe: LodModelRecipe = serde_json::from_slice(bytes)?;
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
    let base_source = SourcePath::new(base_id.0)?;
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
        if model.has_morph() {
            return Err("morph LOD needs a morph-aware error certificate".into());
        }
        if model.primitives.len() != 1 {
            return Err("skeletal LOD requires one primitive".into());
        }
        let voxy_model::ModelGeometry::Skinned(mesh) = &model.primitives[0].geometry else {
            return Err("animated LOD requires a skinned primitive".into());
        };
        let source = voxy_render::decode_skinned_lod_archive(
            std::sync::Arc::new(mesh.clone()),
            &certificate,
            limits,
        )
        .map_err(|error| format!("skeletal LOD verification: {error}"))?;
        // Validate the bind palette now, before source/dependency publication.
        let palette = model.skin_matrices(&model.skeleton.bind_pose())?;
        source.prepare(&palette, Mat4::IDENTITY)?;
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
        let whole_limits = asset_checkpoint::EditorAssetCheckpointLimits::default();
        let whole_checkpoint = asset_checkpoint::EditorAssetCheckpoint::capture(
            &[std::sync::Arc::new(asset.clone())],
            whole_limits,
        )
        .unwrap();
        std::fs::write(
            root.join("whole-static.voxyeda"),
            whole_checkpoint.to_bytes(whole_limits).unwrap(),
        )
        .unwrap();
        asset_checkpoint::retain_evidence(
            "static-lod.voxyeda",
            &whole_checkpoint.to_bytes(whole_limits).unwrap(),
        );
        let checkpoint_limits = voxy_render::LodArchiveLimits {
            bytes: 4096,
            positions: 3,
            levels: 2,
            indices: 6,
            cells: 2,
        };
        let checkpoint = asset
            .lod
            .as_ref()
            .unwrap()
            .checkpoint(checkpoint_limits)
            .unwrap();
        std::fs::write(
            root.join("accepted-lod.json"),
            checkpoint.to_json(checkpoint_limits.bytes).unwrap(),
        )
        .unwrap();
        let mut app = crate::App::new(&root.join("mesh.vmodel"), false).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app
            .catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .is_none()
        {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let last_good = app
            .catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .unwrap();
        std::fs::write(root.join("proof.lod"), b"changed").unwrap();
        while app.failed_at.is_none() {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(std::sync::Arc::ptr_eq(
            &last_good,
            &app.catalog
                .snapshot(app.id.as_ref().expect("model-only fixture"))
                .unwrap()
        ));

        assert!(
            inputs
                .finish(asset, |id, limit| provider.read(id, limit))
                .is_err()
        );
        assert!(load(&source, &provider, &mut ImportInputs::new(3, 32 * 1024)).is_err());
        std::fs::write(root.join("proof.lod"), &bytes).unwrap();
        while std::sync::Arc::ptr_eq(
            &last_good,
            &app.catalog
                .snapshot(app.id.as_ref().expect("model-only fixture"))
                .unwrap(),
        ) {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            app.catalog
                .snapshot(app.id.as_ref().expect("model-only fixture"))
                .unwrap()
                .value()
                .lod
                .is_some()
        );
        app.stop_workers().unwrap();
        std::fs::write(root.join("base.obj"), obj.replace("v 1 0 0", "v 2 0 0")).unwrap();
        assert!(
            load(&source, &provider, &mut ImportInputs::new(3, 32 * 1024))
                .unwrap_err()
                .to_string()
                .contains("LOD base identity")
        );
        std::fs::write(
            root.join("mesh.vmodel"),
            br#"{"version":2,"base":"base.obj","certificate":"proof.lod"}"#,
        )
        .unwrap();
        assert!(load(&source, &provider, &mut ImportInputs::new(3, 32 * 1024)).is_err());
        for filename in ["base.obj", "proof.lod", "mesh.vmodel"] {
            std::fs::remove_file(root.join(filename)).unwrap();
        }
        let restored_lod = voxy_render::CertifiedLodCheckpoint::from_json(
            &std::fs::read_to_string(root.join("accepted-lod.json")).unwrap(),
            checkpoint_limits.bytes,
        )
        .unwrap()
        .prepare(checkpoint_limits)
        .unwrap();
        assert_eq!(
            restored_lod.checkpoint(checkpoint_limits).unwrap(),
            checkpoint
        );
        let whole_assets = asset_checkpoint::EditorAssetCheckpoint::from_bytes(
            &std::fs::read(root.join("whole-static.voxyeda")).unwrap(),
            whole_limits,
        )
        .unwrap()
        .prepare(whole_limits)
        .unwrap();
        assert_eq!(
            asset_checkpoint::EditorAssetCheckpoint::capture(&whole_assets, whole_limits).unwrap(),
            whole_checkpoint
        );
        assert_eq!(
            whole_assets[0]
                .lod
                .as_ref()
                .unwrap()
                .checkpoint(checkpoint_limits)
                .unwrap(),
            checkpoint
        );
        let policy = voxy_render::LodPolicy {
            target_pixels: 1.0,
            hysteresis: 0.1,
        };
        for frame in 0..120 {
            let pixels = 0.05 + f64::from(frame) / 5.;
            assert_eq!(
                policy.select(
                    last_good.value().lod.as_ref().unwrap().indices().levels(),
                    1.0,
                    pixels,
                    None
                ),
                policy.select(restored_lod.indices().levels(), 1.0, pixels, None)
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
