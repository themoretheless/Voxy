//! Headless project bootstrap using the existing asset and scene documents.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const LIMIT: usize = 8 * 1024 * 1024;
pub const PROJECT_FILE: &str = "project.voxy";
#[derive(Debug, Serialize, Deserialize)]
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
    pub scene: voxy_scene::SceneDocument,
    pub assets: voxy_assets::AssetLocations,
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
    let path = path.canonicalize()?;
    let root = path
        .parent()
        .ok_or_else(|| ProjectError::Invalid("project parent missing".into()))?
        .to_owned();
    let descriptor: ProjectDescriptor = serde_json::from_str(&text(&path)?)?;
    if descriptor.version != 1 || descriptor.name.trim().is_empty() {
        return Err(ProjectError::Invalid(
            "unsupported version or empty name".into(),
        ));
    }
    let manifest = resolve(&root, &root, &descriptor.asset_manifest)?;
    let assets = voxy_assets::AssetLocations::from_json(&text(&manifest)?, 100_000, LIMIT, LIMIT)
        .map_err(|e| ProjectError::Invalid(format!("asset manifest: {e:?}")))?;
    for (_, source) in assets.bindings() {
        resolve(&root, &root, Path::new(source.as_str()))?;
    }
    let scene_path = resolve(&root, &root, &descriptor.startup_scene)?;
    let scene = voxy_scene::SceneDocument::from_json(&text(&scene_path)?)
        .map_err(|e| ProjectError::Invalid(e.to_string()))?;
    scene
        .load(registry, 100_000)
        .map_err(|e| ProjectError::Invalid(e.to_string()))?;
    Ok(OpenProject {
        root,
        descriptor,
        scene,
        assets,
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
    fs::create_dir(directory)?;
    let mut owned = Vec::new();
    let result = (|| {
        let descriptor = ProjectDescriptor {
            version: 1,
            name: name.into(),
            asset_manifest: "assets.json".into(),
            startup_scene: "main.scene.json".into(),
        };
        let scene = voxy_scene::SceneDocument {
            version: 1,
            objects: Vec::new(),
        };
        let assets = voxy_assets::AssetLocations::new(100_000, LIMIT);
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
                .to_json(LIMIT)
                .map_err(|e| ProjectError::Invalid(format!("manifest: {e:?}")))?,
        )?;
        write(
            "main.scene.json",
            &scene
                .to_json()
                .map_err(|e| ProjectError::Invalid(e.to_string()))?,
        )?;
        write(
            ".project.pending",
            &serde_json::to_string_pretty(&descriptor)?,
        )?;
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
}
