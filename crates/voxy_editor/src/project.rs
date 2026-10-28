//! Headless project bootstrap using the existing asset and scene documents.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const LIMIT: usize = 8 * 1024 * 1024;
pub const PROJECT_FILE: &str = "project.voxy";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDescriptor {
    pub version: u32,
    pub name: String,
    pub asset_manifest: PathBuf,
    pub startup_scene: PathBuf,
}
#[derive(Debug)]
pub struct OpenProject {
    pub root: PathBuf,
    pub descriptor: ProjectDescriptor,
    /// Expanded runtime view; editing retains the original prefab composition.
    pub scene: voxy_scene::SceneDocument,
    pub assets: voxy_assets::AssetLocations,
    descriptor_path: PathBuf,
    descriptor_text: String,
}

impl OpenProject {
    /// Validate and atomically replace only the project descriptor. Scenes and
    /// manifests keep their own authoring transactions; this is not Save As.
    /// Reopen after external edits. Caller changes to public fields are not
    /// published until a candidate has passed validation.
    /// # Errors
    /// Rejects invalid dependencies, byte limits, external descriptor changes
    /// and IO. Uses trusted local paths, without a concurrent-writer CAS or
    /// directory crash-durability guarantee.
    pub fn save_descriptor(
        &mut self,
        descriptor: &ProjectDescriptor,
        registry: &voxy_scene::ComponentRegistry,
    ) -> Result<(), ProjectError> {
        let encoded = serde_json::to_string_pretty(descriptor)?;
        if encoded.len() > LIMIT {
            return Err(ProjectError::Invalid("document exceeds 8 MiB".into()));
        }
        let candidate = load_descriptor(&self.descriptor_path, &encoded, registry, false)?;
        if text(&self.descriptor_path)? != self.descriptor_text {
            return Err(ProjectError::Invalid(
                "project file changed externally; reopen before saving".into(),
            ));
        }
        voxy_assets::save_atomic_file(&self.descriptor_path, encoded.as_bytes(), LIMIT)
            .map_err(|error| ProjectError::Invalid(error.to_string()))?;
        *self = candidate;
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("project IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("project JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid project: {0}")]
    Invalid(String),
}
fn text(path: &Path) -> Result<String, ProjectError> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > LIMIT {
        return Err(ProjectError::Invalid("document exceeds 8 MiB".into()));
    }
    String::from_utf8(bytes).map_err(|_| ProjectError::Invalid("document is not UTF-8".into()))
}
fn resolve(root: &Path, base: &Path, relative: &Path) -> Result<PathBuf, ProjectError> {
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(ProjectError::Invalid(format!(
            "expected a project-relative path: {}",
            relative.display()
        )));
    }
    let resolved = base.join(relative).canonicalize()?;
    if !resolved.starts_with(root) || !resolved.is_file() {
        return Err(ProjectError::Invalid(format!(
            "path leaves project or is not a file: {}",
            relative.display()
        )));
    }
    Ok(resolved)
}
/// Validate dependencies and load the scene with the caller's component registry.
/// No window, GPU initialization or current-directory change is performed.
/// # Errors
/// Rejects malformed or oversized documents, unsupported versions, unavailable
/// sources, paths outside the project and scenes invalid for the supplied registry.
pub fn open(
    path: &Path,
    registry: &voxy_scene::ComponentRegistry,
) -> Result<OpenProject, ProjectError> {
    open_observed(path, registry).map(|(project, _)| project)
}

/// Opens an authoring project while allowing absent model source files. Scene,
/// manifest and prefab documents remain mandatory; game/build validation stays
/// strict through `open`. Missing models are reported by ordinary import owners.
/// # Errors
/// Rejects malformed documents, escaping paths and missing non-model sources.
pub fn open_for_editor(
    path: &Path,
    registry: &voxy_scene::ComponentRegistry,
) -> Result<OpenProject, ProjectError> {
    open_observed_policy(path, registry, true).map(|(project, _)| project)
}

pub(super) fn open_observed(
    path: &Path,
    registry: &voxy_scene::ComponentRegistry,
) -> Result<(OpenProject, String), ProjectError> {
    open_observed_policy(path, registry, false)
}
fn open_observed_policy(
    path: &Path,
    registry: &voxy_scene::ComponentRegistry,
    allow_missing_models: bool,
) -> Result<(OpenProject, String), ProjectError> {
    let path = path.canonicalize()?;
    let descriptor_text = text(&path)?;
    let project = load_descriptor(&path, &descriptor_text, registry, allow_missing_models)?;
    if text(&path)? != descriptor_text {
        return Err(ProjectError::Invalid(
            "project file changed during validation".into(),
        ));
    }
    Ok((project, descriptor_text))
}

fn load_descriptor(
    path: &Path,
    descriptor_text: &str,
    registry: &voxy_scene::ComponentRegistry,
    allow_missing_models: bool,
) -> Result<OpenProject, ProjectError> {
    let root = path
        .parent()
        .ok_or_else(|| ProjectError::Invalid("project parent missing".into()))?
        .to_owned();
    let descriptor: ProjectDescriptor = serde_json::from_str(descriptor_text)?;
    if descriptor.version != 1 || descriptor.name.trim().is_empty() {
        return Err(ProjectError::Invalid(
            "unsupported version or empty name".into(),
        ));
    }
    let manifest_path = resolve(&root, &root, &descriptor.asset_manifest)?;
    let scene_path = resolve(&root, &root, &descriptor.startup_scene)?;
    if manifest_path == path || scene_path == path || manifest_path == scene_path {
        return Err(ProjectError::Invalid(
            "descriptor, manifest and scene must be distinct files".into(),
        ));
    }
    let provider = voxy_assets::FileInputs::new(&root)
        .map_err(|error| ProjectError::Invalid(error.to_string()))?;
    let mut inputs = voxy_assets::ImportInputs::new(130, 16 * 1024 * 1024);
    let source_path = |path: &Path| {
        voxy_assets::SourcePath::new(
            path.to_str()
                .ok_or_else(|| ProjectError::Invalid("project source path must be UTF-8".into()))?,
        )
        .map_err(|error| ProjectError::Invalid(error.to_string()))
    };
    let manifest = inputs
        .read(
            source_path(&descriptor.asset_manifest)?.observation_id(),
            |id, limit| provider.read(id, limit.min(crate::import::MANIFEST_BYTES)),
        )
        .map_err(|error| ProjectError::Invalid(format!("manifest input: {error:?}")))?;
    let assets = crate::import::manifest_locations(&manifest.bytes)
        .map_err(|error| ProjectError::Invalid(format!("asset manifest: {error}")))?;
    for (_, source) in assets.bindings() {
        let source_path = Path::new(source.as_str());
        match resolve(&root, &root, source_path) {
            Ok(_) => {}
            Err(ProjectError::Io(error))
                if allow_missing_models
                    && error.kind() == std::io::ErrorKind::NotFound
                    && source_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| {
                            matches!(
                                e.to_ascii_lowercase().as_str(),
                                "obj" | "gltf" | "glb" | "vmodel"
                            )
                        }) =>
            {
                // Resolve the nearest existing ancestor, rejecting dangling links
                // and missing children under a directory outside the project.
                let mut ancestor = root.join(source_path);
                loop {
                    match ancestor.symlink_metadata() {
                        Ok(_) => {
                            let resolved = ancestor.canonicalize()?;
                            if !resolved.starts_with(&root) || !resolved.is_dir() {
                                return Err(ProjectError::Invalid(
                                    "missing model path leaves project".into(),
                                ));
                            }
                            break;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            if !ancestor.pop() {
                                return Err(ProjectError::Invalid("missing model ancestor".into()));
                            }
                        }
                        Err(error) => return Err(error.into()),
                    }
                }
            }
            Err(error) => return Err(error),
        }
    }
    resolve(&root, &root, &descriptor.startup_scene)?;
    let snapshot = inputs
        .read(
            source_path(&descriptor.startup_scene)?.observation_id(),
            |id, limit| provider.read(id, limit.min(crate::scene_limits::DOCUMENT_BYTES)),
        )
        .map_err(|error| ProjectError::Invalid(format!("scene input: {error:?}")))?;
    let source: voxy_scene::PrefabSceneDocument = serde_json::from_slice(&snapshot.bytes)?;
    let expanded = source
        .expand(registry, crate::scene_limits::PREFABS, |asset| {
            let path = assets
                .source(&voxy_assets::AssetId(asset.into()))
                .ok_or_else(|| {
                    voxy_scene::DocumentError::Invalid(format!("unknown prefab asset {asset}"))
                })?;
            let snapshot = inputs
                .read(path.observation_id(), |id, limit| {
                    provider.read(id, limit.min(crate::scene_limits::DOCUMENT_BYTES))
                })
                .map_err(|error| {
                    voxy_scene::DocumentError::Invalid(format!("prefab input: {error:?}"))
                })?;
            serde_json::from_slice(&snapshot.bytes).map_err(Into::into)
        })
        .map_err(|error| ProjectError::Invalid(error.to_string()))?;
    let publication = inputs
        .finish(expanded.document, |id, limit| provider.read(id, limit))
        .map_err(|error| {
            ProjectError::Invalid(format!(
                "project scene changed during validation: {error:?}"
            ))
        })?;
    let scene = publication.value().clone();
    if !allow_missing_models {
        for object in &scene.objects {
            if let Some(id) = object
                .components
                .get("editor.model.v1")
                .and_then(serde_json::Value::as_str)
                && assets.source(&voxy_assets::AssetId(id.into())).is_none()
            {
                return Err(ProjectError::Invalid(format!(
                    "unresolved model asset {id}"
                )));
            }
        }
    }
    Ok(OpenProject {
        root,
        descriptor,
        scene,
        assets,
        descriptor_path: path.to_owned(),
        descriptor_text: descriptor_text.to_owned(),
    })
}
/// Exclusively claim a NEW directory. Publish the descriptor last as a readiness
/// marker. On error remove only files created here; never remove foreign files.
/// The parent must already exist. This is not an atomic directory rename.
/// # Errors
/// Rejects an empty name, an existing destination or any write/publication error.
/// No crash-durability guarantee is made for the new directory entry.
pub fn create(directory: &Path, name: &str) -> Result<PathBuf, ProjectError> {
    if name.trim().is_empty() {
        return Err(ProjectError::Invalid("empty name".into()));
    }
    let descriptor = ProjectDescriptor {
        version: 1,
        name: name.into(),
        asset_manifest: "assets.json".into(),
        startup_scene: "main.scene.json".into(),
    };
    let encoded = serde_json::to_string_pretty(&descriptor)?;
    if encoded.len() > LIMIT {
        return Err(ProjectError::Invalid("document exceeds 8 MiB".into()));
    }
    fs::create_dir(directory)?;
    let mut owned = Vec::new();
    let result = (|| {
        let scene = voxy_scene::SceneDocument {
            version: 1,
            objects: Vec::new(),
        };
        let assets = voxy_assets::AssetLocations::new(
            crate::import::MANIFEST_ENTRIES,
            crate::import::MANIFEST_BYTES,
        );
        let mut write = |file: &str, content: &str| -> Result<(), ProjectError> {
            let path = directory.join(file);
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            owned.push(path);
            f.write_all(content.as_bytes())?;
            f.sync_all()?;
            Ok(())
        };
        write(
            "assets.json",
            &assets
                .to_json(crate::import::MANIFEST_BYTES)
                .map_err(|e| ProjectError::Invalid(format!("manifest: {e:?}")))?,
        )?;
        write(
            "main.scene.json",
            &scene
                .to_json()
                .map_err(|e| ProjectError::Invalid(e.to_string()))?,
        )?;
        write(".project.pending", &encoded)?;
        let published = directory.join(PROJECT_FILE);
        fs::hard_link(directory.join(".project.pending"), &published)?;
        owned.push(published.clone());
        fs::remove_file(directory.join(".project.pending"))?;
        Ok(published)
    })();
    if result.is_err() {
        for path in owned.iter().rev() {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_dir(directory);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes).unwrap();
            let path = std::env::temp_dir()
                .join(format!("voxy-project-{:032x}", u128::from_le_bytes(bytes)));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn empty_project_reopens_after_relocation_and_does_not_overwrite() {
        let t = Temp::new();
        let dir = t.0.join("game");
        let path = create(&dir, "Example").unwrap();
        let registry = voxy_scene::ComponentRegistry::default();
        assert!(open(&path, &registry).unwrap().scene.objects.is_empty());
        let before = fs::read(&path).unwrap();
        assert!(create(&dir, "Other").is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        fs::rename(&dir, t.0.join("moved")).unwrap();
        assert_eq!(
            open(&t.0.join("moved/project.voxy"), &registry)
                .unwrap()
                .descriptor
                .name,
            "Example"
        );
    }
    #[test]
    fn rejects_missing_dependencies_versions_unknown_fields_and_broken_scene() {
        let t = Temp::new();
        let path = create(&t.0.join("game"), "Game").unwrap();
        let registry = voxy_scene::ComponentRegistry::default();
        let original = fs::read_to_string(&path).unwrap();
        for invalid in [
            original.replace("\"version\": 1", "\"version\": 99"),
            original.replace("assets.json", "../outside.json"),
            original.replace("assets.json", "missing.json"),
            original.replace("\"name\"", "\"unknown\""),
        ] {
            fs::write(&path, invalid).unwrap();
            assert!(open(&path, &registry).is_err());
        }
        fs::write(&path, original).unwrap();
        fs::write(
            path.parent().unwrap().join("main.scene.json"),
            "{\"version\":2,\"objects\":[]}",
        )
        .unwrap();
        assert!(open(&path, &registry).is_err());
    }
    #[test]
    fn liquid_material_change_during_game_host_setup_rejects_package_publication()
    -> Result<(), Box<dyn std::error::Error>> {
        let t = Temp::new();
        let root = t.0.join("game");
        let path = create(&root, "Liquid build observation")?;
        let material = root.join("liquid.json");
        let original = br#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001,"optics":[3,0.5,0.2,1.333]}"#;
        fs::write(&material, original)?;
        fs::write(
            root.join("assets.json"),
            br#"{"version":1,"assets":[{"asset":"liquid","source":"liquid.json"}]}"#,
        )?;
        let source = voxy_gameplay::LiquidSource {
            pulses: vec![voxy_gameplay::LiquidPulse {
                start_s: 0.,
                duration_s: 1. / 60.,
                volume_m3: 0.001,
                speed_m_s: 0.,
            }],
            density_kg_m3: 1000.,
            particle_volume_m3: 0.001,
            nozzle_radius_m: 0.,
            direction: [0., 0., -1.],
            material_asset: "liquid".into(),
        };
        fs::write(
            root.join("main.scene.json"),
            serde_json::to_vec(&serde_json::json!({"version":1,"objects":[{
                "id":"jet","parent":null,"name":"Liquid source","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],
                "components":{"game.liquid-source.v1":serde_json::to_value(source)?}
            }]}))?,
        )?;
        let output = t.0.join("game.vpak");
        for replacement in [
            b"invalid material after runtime preparation".as_slice(),
            br#"{"rest_density":1000,"sound_speed":21,"viscosity":0.001,"optics":[3,0.5,0.2,1.333]}"#.as_slice(),
        ] {
            fs::write(&material, original)?;
            let changed = material.clone();
            let replacement = replacement.to_vec();
            let called = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let calls = called.clone();
            let mut host = crate::GameHost::new(crate::editor_component_registry()?);
            host.behaviors = Some(crate::BehaviorSetup::new(move |_, _| {
                calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                fs::write(&changed, &replacement)?;
                Ok(())
            }));
            let result = crate::export_project_package_with_host(&path, &output, host);
            assert_eq!(called.load(std::sync::atomic::Ordering::SeqCst), 1);
            let error = result.expect_err("build published material changed after native runtime preparation");
            assert!(error.to_string().contains("liquid.json changed during package export"), "{error}");
            assert!(!output.exists());
        }
        fs::write(&material, original)?;
        crate::export_project_package(&path, &output)?;
        fs::remove_dir_all(&root)?;
        crate::run_packaged_game(&output, crate::ViewportMode::GameCheck)?;
        Ok(())
    }
    #[test]
    fn project_manifest_admission_matches_editor_count_and_byte_policy()
    -> Result<(), Box<dyn std::error::Error>> {
        let t = Temp::new();
        let path = create(&t.0.join("game"), "Manifest budgets")?;
        let root = path.parent().unwrap();
        let registry = crate::editor_component_registry()?;
        let manifest = root.join("assets.json");
        let entries: Vec<_> = (0..128).map(|i| serde_json::json!({"asset":format!("data-{i}"),"source":format!("data-{i}.txt")})).collect();
        for i in 0..129 {
            fs::write(root.join(format!("data-{i}.txt")), [])?;
        }
        let encoded = serde_json::to_vec(&serde_json::json!({"version":1,"assets":entries}))?;
        let mut boundary = encoded.clone();
        boundary.resize(65536, b' ');
        fs::write(&manifest, &boundary)?;
        let mut opened = open(&path, &registry)?;
        let mut editor = crate::App::from_project(&path)?;
        editor.stop_workers()?;
        let original = fs::read(&path)?;
        let descriptor = opened.descriptor.clone();
        let mut excessive = entries;
        excessive.push(serde_json::json!({"asset":"data-128","source":"data-128.txt"}));
        fs::write(
            &manifest,
            serde_json::to_vec(&serde_json::json!({"version":1,"assets":excessive}))?,
        )?;
        assert!(
            open(&path, &registry).is_err(),
            "project validator accepted manifest count rejected by editor"
        );
        assert!(open_for_editor(&path, &registry).is_err());
        assert!(opened.save_descriptor(&descriptor, &registry).is_err());
        assert_eq!(fs::read(&path)?, original);
        boundary.push(b' ');
        fs::write(&manifest, &boundary)?;
        assert!(
            open(&path, &registry).is_err(),
            "project validator accepted manifest bytes rejected by editor"
        );
        assert!(open_for_editor(&path, &registry).is_err());
        assert!(opened.save_descriptor(&descriptor, &registry).is_err());
        assert_eq!(fs::read(&path)?, original);
        fs::write(&manifest, encoded)?;
        open(&path, &registry)?;
        Ok(())
    }
    #[test]
    fn rejects_corrupt_manifest_missing_asset_and_existing_file_destination() {
        let t = Temp::new();
        let path = create(&t.0.join("game"), "Game").unwrap();
        let manifest = path.parent().unwrap().join("assets.json");
        let registry = voxy_scene::ComponentRegistry::default();
        for invalid in [
            "not json",
            r#"{"version":99,"assets":[]}"#,
            r#"{"version":1,"assets":[{"asset":"missing","source":"absent.obj"}]}"#,
        ] {
            fs::write(&manifest, invalid).unwrap();
            assert!(open(&path, &registry).is_err());
        }
        let occupied = t.0.join("occupied");
        fs::write(&occupied, "preserve").unwrap();
        assert!(create(&occupied, "Game").is_err());
        assert_eq!(fs::read_to_string(occupied).unwrap(), "preserve");
        let empty = t.0.join("empty");
        assert!(create(&empty, " ").is_err());
        assert!(!empty.exists());
    }
    #[test]
    fn editor_project_bootstrap_edits_saves_undoes_and_reopens_without_model()
    -> Result<(), Box<dyn std::error::Error>> {
        let t = Temp::new();
        let path = create(&t.0.join("game"), "Game")?;
        let mut app = crate::App::from_project(&path)?;
        assert!(app.id.is_none());
        assert!(app.available.is_empty() && app.reload.is_empty());
        assert!(app.required_cpu_model_assets().is_empty());
        assert!(app.authoring_document()?.objects.is_empty());
        app.edit_key(winit::keyboard::KeyCode::KeyD)?;
        let authored = app.authoring_document()?;
        assert_eq!(authored.objects.len(), 1);
        assert!(authored.objects[0].name.starts_with("Object"));
        assert!(
            !authored.objects[0]
                .components
                .contains_key("editor.model.v1")
        );
        app.edit_key(winit::keyboard::KeyCode::KeyZ)?;
        assert!(app.authoring_document()?.objects.is_empty());
        app.edit_key(winit::keyboard::KeyCode::KeyY)?;
        assert_eq!(app.authoring_document()?, authored);
        app.save_authoring()?;
        app.stop_workers()?;
        fs::rename(t.0.join("game"), t.0.join("moved"))?;
        let mut reopened = crate::App::from_project(&t.0.join("moved/project.voxy"))?;
        assert_eq!(reopened.authoring_document()?, authored);
        assert!(reopened.required_cpu_model_assets().is_empty());
        reopened.stop_workers()?;
        Ok(())
    }
    #[test]
    fn nested_manifest_sources_follow_existing_project_root_contract()
    -> Result<(), Box<dyn std::error::Error>> {
        let t = Temp::new();
        let path = create(&t.0.join("game"), "Game")?;
        let root = path.parent().unwrap();
        fs::create_dir(root.join("assets"))?;
        fs::write(
            root.join("assets/test.obj"),
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )?;
        fs::write(
            root.join("assets/catalog.json"),
            r#"{"version":1,"assets":[{"asset":"triangle","source":"assets/test.obj"}]}"#,
        )?;
        let mut descriptor: ProjectDescriptor = serde_json::from_str(&fs::read_to_string(&path)?)?;
        descriptor.asset_manifest = "assets/catalog.json".into();
        fs::write(&path, serde_json::to_string(&descriptor)?)?;
        let mut app = crate::App::from_project(&path)?;
        assert!(
            app.available
                .contains(&voxy_assets::AssetId("triangle".into()))
        );
        assert!(app.id.is_none() && app.instances.is_empty());
        app.stop_workers()?;
        Ok(())
    }
    #[test]
    fn project_play_stop_and_standalone_share_fixed_motion()
    -> Result<(), Box<dyn std::error::Error>> {
        let t = Temp::new();
        let path = create(&t.0.join("game"), "Motion")?;
        let mut editor = crate::App::from_project(&path)?;
        editor.edit_key(winit::keyboard::KeyCode::KeyD)?;
        editor.scene.insert_component(
            editor.instances[0],
            voxy_gameplay::AngularMotion {
                axis: [0., 0., 1.],
                radians_per_second: 2.,
            },
        )?;
        editor.commit_authoring()?;
        editor.save_authoring()?;
        let authored = editor.authoring_document()?;
        let authoring_handle = editor.instances[0];
        editor.toggle_play()?;
        assert!(editor.scene.local(authoring_handle).is_err());
        let runtime_handle = editor.instances[0];
        for _ in 0..120 {
            editor.advance_game(1.0 / 60.0)?;
        }
        let played = editor.authoring_document()?;
        assert!(
            editor
                .scene
                .local(runtime_handle)?
                .rotation
                .angle_between(glam::Quat::from_rotation_z(4.))
                < 1e-3
        );
        assert_eq!(editor.play.simulation_ticks, 120);
        assert_eq!(
            editor.authoring.history.as_ref().unwrap().current(),
            &authored
        );
        editor.toggle_play()?;
        assert!(editor.scene.local(runtime_handle).is_err());
        assert_eq!(editor.authoring_document()?, authored);
        assert!(editor.play.simulation.is_none() && editor.play.angular_motion.is_none());
        editor.stop_workers()?;
        let mut game = crate::App::from_project(&path)?;
        assert!(crate::project_runtime::prepare_game(
            &mut game,
            crate::ViewportMode::GameCheck
        )?);
        assert!(game.standalone && game.play.playing.is_some());
        assert_eq!(game.play.simulation_ticks, 120);
        assert_eq!(game.authoring_document()?, played);
        game.stop_workers()?;
        crate::run_project(&path, crate::ViewportMode::GameCheck)?;
        assert!(crate::run_project(&path, crate::ViewportMode::AnimationSmoke).is_err());
        Ok(())
    }
    #[test]
    fn project_package_runs_without_source_tree_and_contains_import_closure()
    -> Result<(), Box<dyn std::error::Error>> {
        let t = Temp::new();
        let root = t.0.join("game");
        let path = create(&root, "Packaged Motion")?;
        let glb = include_bytes!("../../voxy_render/examples/assets/animated-triangle.glb");
        let parsed = gltf::Gltf::from_slice(glb)?;
        let json_len = u32::from_le_bytes(glb[12..16].try_into()?) as usize;
        let mut document: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len])?;
        document["buffers"][0]["uri"] = "motion.bin".into();
        fs::write(root.join("motion.gltf"), serde_json::to_vec(&document)?)?;
        fs::write(
            root.join("motion.bin"),
            parsed.blob.ok_or("fixture buffer missing")?,
        )?;
        fs::write(root.join("unused.txt"), "unused declared asset")?;
        fs::write(
            root.join("assets.json"),
            r#"{"version":1,"assets":[{"asset":"mesh","source":"motion.gltf"},{"asset":"unused","source":"unused.txt"}]}"#,
        )?;
        let mut editor = crate::App::from_project(&path)?;
        editor.edit_key(winit::keyboard::KeyCode::KeyD)?;
        editor.scene.insert_component(
            editor.instances[0],
            voxy_gameplay::AngularMotion {
                axis: [0., 0., 1.],
                radians_per_second: 2.,
            },
        )?;
        editor.commit_authoring()?;
        editor.save_authoring()?;
        editor.stop_workers()?;
        let output = t.0.join("game.vpak");
        crate::export_project_package(&path, &output)?;
        let original = fs::read(&output)?;
        assert!(crate::export_project_package(&path, &output).is_err());
        assert_eq!(fs::read(&output)?, original);
        let package = voxy_assets::ResourcePackage::from_bytes(
            &original,
            voxy_assets::PackageLimits {
                max_entries: 4096,
                max_payload_bytes: 64 * 1024 * 1024,
                max_document_bytes: 256 * 1024 * 1024,
            },
        )?;
        for source in [
            "project.voxy",
            "assets.json",
            "main.scene.json",
            "motion.gltf",
            "motion.bin",
            "unused.txt",
            "__voxy_game.json",
        ] {
            assert!(
                package.paths().any(|path| path == source),
                "missing {source}"
            );
        }
        fs::remove_dir_all(&root)?;
        assert!(!path.exists());
        let cwd = std::env::current_dir()?;
        crate::run_packaged_game(&output, crate::ViewportMode::GameCheck)?;
        assert_eq!(std::env::current_dir()?, cwd);
        let mut corrupt = original;
        let end = corrupt.len() - 1;
        corrupt[end] ^= 1;
        fs::write(&output, corrupt)?;
        assert!(crate::run_packaged_game(&output, crate::ViewportMode::GameCheck).is_err());
        Ok(())
    }
    #[test]
    fn package_includes_declared_uninstanced_model_dependencies_and_rejects_broken_inputs()
    -> Result<(), Box<dyn std::error::Error>> {
        let t = Temp::new();
        let root = t.0.join("game");
        let path = create(&root, "Deferred model")?;
        let glb = include_bytes!("../../voxy_render/examples/assets/animated-triangle.glb");
        let parsed = gltf::Gltf::from_slice(glb)?;
        let json_len = u32::from_le_bytes(glb[12..16].try_into()?) as usize;
        let mut document: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len])?;
        document["buffers"][0]["uri"] = "deferred.bin".into();
        fs::write(root.join("deferred.gltf"), serde_json::to_vec(&document)?)?;
        fs::write(
            root.join("assets.json"),
            r#"{"version":1,"assets":[{"asset":"deferred","source":"deferred.gltf"}]}"#,
        )?;
        let output = t.0.join("game.vpak");
        assert!(
            crate::export_project_package(&path, &output).is_err(),
            "missing uninstanced model dependency was packaged"
        );
        assert!(!output.exists());
        fs::write(root.join("deferred.bin"), b"truncated")?;
        assert!(crate::export_project_package(&path, &output).is_err());
        assert!(!output.exists());
        fs::write(
            root.join("deferred.bin"),
            parsed.blob.ok_or("fixture buffer missing")?,
        )?;
        crate::export_project_package(&path, &output)?;
        let package = voxy_assets::ResourcePackage::from_bytes(
            &fs::read(&output)?,
            voxy_assets::PackageLimits {
                max_entries: 4096,
                max_payload_bytes: 64 * 1024 * 1024,
                max_document_bytes: 256 * 1024 * 1024,
            },
        )?;
        assert!(package.paths().any(|path| path == "deferred.bin"));
        fs::remove_dir_all(&root)?;
        let materialized = crate::packaged_game::MaterializedPackage::open(&output)?;
        let manifest = materialized
            .project_path()?
            .parent()
            .unwrap()
            .join("assets.json");
        let mut model =
            crate::App::from_manifest(&manifest, voxy_assets::AssetId("deferred".into()), false)?;
        let id = voxy_assets::AssetId("deferred".into());
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while model.catalog.snapshot(&id).is_none() {
            model.tick()?;
            assert!(
                std::time::Instant::now() < until,
                "packaged deferred model did not load"
            );
            std::thread::yield_now();
        }
        assert!(
            model
                .catalog
                .snapshot(&id)
                .unwrap()
                .value()
                .animated
                .is_some()
        );
        model.stop_workers()?;
        crate::run_packaged_game(&output, crate::ViewportMode::GameCheck)?;
        Ok(())
    }
    #[test]
    fn project_custom_component_registry_survives_play_build_and_packaged_run()
    -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct GameTag {
            value: u32,
        }
        fn registry() -> Result<voxy_scene::ComponentRegistry, Box<dyn std::error::Error>> {
            let mut registry = crate::editor_component_registry()?;
            registry.register::<GameTag>("test.game-tag.v1")?;
            Ok(registry)
        }
        let t = Temp::new();
        let path = create(&t.0.join("game"), "Custom Game")?;
        let mut editor = crate::App::from_project_with_registry(&path, registry()?)?;
        editor.edit_key(winit::keyboard::KeyCode::KeyD)?;
        editor
            .scene
            .insert_component(editor.instances[0], GameTag { value: 42 })?;
        editor.commit_authoring()?;
        editor.save_authoring()?;
        let authored = editor.authoring_document()?;
        editor.toggle_play()?;
        assert_eq!(
            editor.scene.component::<GameTag>(editor.instances[0])?,
            Some(&GameTag { value: 42 })
        );
        editor.toggle_play()?;
        assert_eq!(editor.authoring_document()?, authored);
        editor.stop_workers()?;
        assert!(crate::run_project(&path, crate::ViewportMode::GameCheck).is_err());
        crate::run_project_with_components(&path, crate::ViewportMode::GameCheck, registry()?)?;
        let output = t.0.join("custom.vpak");
        assert!(crate::export_project_package(&path, &output).is_err());
        assert!(!output.exists());
        crate::export_project_package_with_components(&path, &output, registry()?)?;
        fs::remove_dir_all(path.parent().unwrap())?;
        assert!(crate::run_packaged_game(&output, crate::ViewportMode::GameCheck).is_err());
        crate::run_packaged_game_with_components(
            &output,
            crate::ViewportMode::GameCheck,
            registry()?,
        )?;
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn rejects_dependency_symlink_outside_project() {
        let t = Temp::new();
        let path = create(&t.0.join("game"), "Game").unwrap();
        let scene = path.parent().unwrap().join("main.scene.json");
        fs::rename(&scene, t.0.join("outside.json")).unwrap();
        std::os::unix::fs::symlink(t.0.join("outside.json"), scene).unwrap();
        assert!(open(&path, &voxy_scene::ComponentRegistry::default()).is_err());
    }

    #[test]
    fn descriptor_save_validates_before_publication_and_preserves_other_files() {
        let temp = Temp::new();
        let path = create(&temp.0.join("game"), "Original").unwrap();
        let registry = voxy_scene::ComponentRegistry::default();
        let mut project = open(&path, &registry).unwrap();
        let scene = fs::read(project.root.join("main.scene.json")).unwrap();
        let manifest = fs::read(project.root.join("assets.json")).unwrap();
        let mut candidate = project.descriptor.clone();
        for name in ["Renamed", "Saved again"] {
            candidate.name = name.into();
            project.save_descriptor(&candidate, &registry).unwrap();
            assert_eq!(open(&path, &registry).unwrap().descriptor, candidate);
        }
        let before = fs::read(&path).unwrap();
        for (name, scene_path) in [
            (" ", "main.scene.json"),
            ("Game", "missing.json"),
            ("Game", "../outside.json"),
            ("Game", "project.voxy"),
        ] {
            let mut invalid = candidate.clone();
            invalid.name = name.into();
            invalid.startup_scene = scene_path.into();
            assert!(project.save_descriptor(&invalid, &registry).is_err());
            assert_eq!(fs::read(&path).unwrap(), before);
            assert_eq!(project.descriptor, candidate);
        }
        assert_eq!(
            fs::read(project.root.join("main.scene.json")).unwrap(),
            scene
        );
        assert_eq!(
            fs::read(project.root.join("assets.json")).unwrap(),
            manifest
        );
    }

    #[test]
    fn descriptor_save_rejects_external_edits_and_accepts_reopen() {
        let temp = Temp::new();
        let path = create(&temp.0.join("game"), "Original").unwrap();
        let registry = voxy_scene::ComponentRegistry::default();
        let mut first = open(&path, &registry).unwrap();
        let mut second = open(&path, &registry).unwrap();
        let mut candidate = second.descriptor.clone();
        candidate.name = "External edit".into();
        second.save_descriptor(&candidate, &registry).unwrap();
        let bytes = fs::read(&path).unwrap();
        candidate.name = "Stale write".into();
        assert!(first.save_descriptor(&candidate, &registry).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(first.descriptor.name, "Original");
        let mut reopened = open(&path, &registry).unwrap();
        reopened.save_descriptor(&candidate, &registry).unwrap();
        assert_eq!(
            open(&path, &registry).unwrap().descriptor.name,
            "Stale write"
        );
    }

    #[test]
    fn oversized_project_names_never_publish_unreadable_projects() {
        let temp = Temp::new();
        let directory = temp.0.join("game");
        let too_large = "x".repeat(LIMIT);
        assert!(create(&directory, &too_large).is_err());
        assert!(!directory.exists());
        let path = create(&directory, "Game").unwrap();
        let registry = voxy_scene::ComponentRegistry::default();
        let mut project = open(&path, &registry).unwrap();
        let before = fs::read(&path).unwrap();
        let mut invalid = project.descriptor.clone();
        invalid.name = too_large;
        assert!(project.save_descriptor(&invalid, &registry).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn descriptor_save_can_select_nested_scene_and_survives_relocation() {
        let temp = Temp::new();
        let path = create(&temp.0.join("game"), "Game").unwrap();
        let registry = voxy_scene::ComponentRegistry::default();
        let mut project = open(&path, &registry).unwrap();
        fs::create_dir(project.root.join("scenes")).unwrap();
        fs::copy(
            project.root.join("main.scene.json"),
            project.root.join("scenes/start.json"),
        )
        .unwrap();
        let mut candidate = project.descriptor.clone();
        candidate.startup_scene = "scenes/start.json".into();
        project.save_descriptor(&candidate, &registry).unwrap();
        fs::rename(temp.0.join("game"), temp.0.join("moved")).unwrap();
        let reopened = open(&temp.0.join("moved/project.voxy"), &registry).unwrap();
        assert_eq!(reopened.descriptor, candidate);
        assert!(reopened.scene.objects.is_empty());
    }

    #[test]
    fn authoring_repeated_save_rejects_external_scene_changes_without_losing_edits()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = Temp::new();
        let path = create(&temp.0.join("game"), "Game")?;
        let mut app = crate::App::from_project(&path)?;
        app.edit_key(winit::keyboard::KeyCode::KeyD)?;
        app.save_authoring()?;
        app.edit_key(winit::keyboard::KeyCode::KeyD)?;
        app.save_authoring()?;
        let authored = app.authoring_document()?;
        let scene_path = path.parent().unwrap().join("main.scene.json");
        fs::write(&scene_path, br#"{"version":1,"objects":[]}"#)?;
        let external = fs::read(&scene_path)?;
        assert!(app.save_authoring().is_err());
        assert_eq!(fs::read(&scene_path)?, external);
        assert_eq!(app.authoring_document()?, authored);
        app.stop_workers()?;
        Ok(())
    }
    #[test]
    fn missing_model_editor_reopen_preserves_scene_and_recovers_without_weakening_build()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = Temp::new();
        let root = temp.0.join("game");
        let path = create(&root, "Missing model")?;
        let model = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        fs::write(root.join("mesh.obj"), model)?;
        fs::write(
            root.join("assets.json"),
            br#"{"version":1,"assets":[{"asset":"mesh","source":"mesh.obj"}]}"#,
        )?;
        let mut app = crate::App::from_project(&path)?;
        app.edit_key(winit::keyboard::KeyCode::KeyD)?;
        app.save_authoring()?;
        let authored = app.authoring_document()?;
        assert_eq!(authored.objects.len(), 1);
        app.stop_workers()?;
        fs::remove_file(root.join("mesh.obj"))?;
        let registry = crate::editor_component_registry()?;
        assert!(open(&path, &registry).is_err());
        assert_eq!(open_for_editor(&path, &registry)?.scene, authored);
        let output = temp.0.join("broken.vpak");
        assert!(crate::export_project_package(&path, &output).is_err());
        assert!(!output.exists());
        let mut cold = crate::App::from_project(&path)?;
        assert_eq!(cold.authoring_document()?, authored);
        assert!(cold.start_standalone().is_err());
        assert!(cold.play.playing.is_none());
        assert!(matches!(
            cold.catalog.status(&voxy_assets::AssetId("mesh".into())),
            Some(voxy_assets::AssetStatus::Failed(_))
        ));
        let notice = cold
            .selected_model_notice()
            .ok_or("missing failed-import diagnostic")?;
        assert!(notice.lines[0].contains("mesh"));
        assert!(notice.lines[2].contains("automatic reload"));
        cold.edit_key(winit::keyboard::KeyCode::ArrowRight)?;
        let edited = cold.authoring_document()?;
        assert_ne!(edited, authored);
        cold.save_authoring()?;
        cold.stop_workers()?;
        let mut reopened = crate::App::from_project(&path)?;
        assert_eq!(reopened.authoring_document()?, edited);
        let id = voxy_assets::AssetId("mesh".into());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !matches!(
            reopened.catalog.status(&id),
            Some(voxy_assets::AssetStatus::Failed(_))
        ) {
            reopened.tick()?;
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        fs::write(root.join("mesh.obj"), model)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while reopened.catalog.snapshot(&id).is_none() {
            reopened.tick()?;
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(reopened.authoring_document()?, edited);
        reopened.stop_workers()?;
        assert!(open(&path, &registry).is_ok());
        Ok(())
    }
    #[test]
    fn editor_missing_source_policy_keeps_non_models_and_escaping_paths_strict() {
        let temp = Temp::new();
        let root = temp.0.join("game");
        let path = create(&root, "Missing paths").unwrap();
        let registry = crate::editor_component_registry().unwrap();
        let write = |source| {
            fs::write(
                root.join("assets.json"),
                serde_json::to_vec(&serde_json::json!({
                    "version":1,"assets":[{"asset":"asset","source":source}]
                }))
                .unwrap(),
            )
            .unwrap()
        };
        write("missing/sub/model.obj");
        assert!(open_for_editor(&path, &registry).is_ok());
        for source in ["missing.wav", "missing.prefab", "../outside.obj"] {
            write(source);
            assert!(
                open_for_editor(&path, &registry).is_err(),
                "accepted {source}"
            );
        }
        #[cfg(unix)]
        {
            fs::create_dir(temp.0.join("outside")).unwrap();
            std::os::unix::fs::symlink(temp.0.join("outside"), root.join("link")).unwrap();
            write("link/missing.obj");
            assert!(open_for_editor(&path, &registry).is_err());
            std::os::unix::fs::symlink(
                temp.0.join("missing-outside.obj"),
                root.join("dangling.obj"),
            )
            .unwrap();
            write("dangling.obj");
            assert!(open_for_editor(&path, &registry).is_err());
        }
    }
    #[test]
    fn cold_unresolved_model_id_preserves_authoring_and_recovers_manifest_binding()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = Temp::new();
        let root = temp.0.join("game");
        let path = create(&root, "Unresolved ID")?;
        let manifest = br#"{"version":1,"assets":[{"asset":"mesh","source":"mesh.obj"}]}"#;
        fs::write(root.join("assets.json"), manifest)?;
        fs::write(
            root.join("mesh.obj"),
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )?;
        let mut initial = crate::App::from_project(&path)?;
        initial.edit_key(winit::keyboard::KeyCode::KeyD)?;
        initial.save_authoring()?;
        let original = initial.authoring_document()?;
        initial.stop_workers()?;
        fs::write(root.join("assets.json"), br#"{"version":1,"assets":[]}"#)?;
        let registry = crate::editor_component_registry()?;
        assert!(open(&path, &registry).is_err());
        let mut cold = crate::App::from_project(&path)?;
        assert!(cold.available.is_empty());
        assert_eq!(cold.authoring_document()?, original);
        let notice = cold
            .selected_model_notice()
            .ok_or("missing unresolved-model diagnostic")?;
        assert!(notice.lines[0].contains("mesh"));
        assert!(notice.lines[1].contains("reference kept"));
        let error = cold.toggle_play().unwrap_err().to_string();
        assert!(error.contains("mesh") && error.contains("unresolved"));
        assert!(cold.play.playing.is_none());
        assert_eq!(cold.authoring_document()?, original);
        cold.edit_key(winit::keyboard::KeyCode::ArrowRight)?;
        let edited = cold.authoring_document()?;
        assert_ne!(edited, original);
        cold.history_key(winit::keyboard::KeyCode::KeyZ)?;
        assert_eq!(cold.authoring_document()?, original);
        cold.history_key(winit::keyboard::KeyCode::KeyY)?;
        assert_eq!(cold.authoring_document()?, edited);
        cold.save_authoring()?;
        cold.stop_workers()?;
        let output = temp.0.join("unresolved.vpak");
        assert!(crate::export_project_package(&path, &output).is_err());
        assert!(!output.exists());
        let mut reopened = crate::App::from_project(&path)?;
        assert_eq!(reopened.authoring_document()?, edited);
        assert!(reopened.start_standalone().is_err());
        assert!(reopened.play.playing.is_none());
        fs::write(root.join("assets.json"), manifest)?;
        let id = voxy_assets::AssetId("mesh".into());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while reopened.catalog.snapshot(&id).is_none() {
            reopened.tick()?;
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(reopened.available.contains(&id));
        assert!(reopened.selected_model_notice().is_none());
        assert_eq!(reopened.authoring_document()?, edited);
        reopened.toggle_play()?;
        assert!(reopened.play.playing.is_some());
        reopened.toggle_play()?;
        assert_eq!(reopened.authoring_document()?, edited);
        reopened.stop_workers()?;
        assert!(open(&path, &registry).is_ok());
        Ok(())
    }
}
