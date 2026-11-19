//! Project archives launch from memory. Legacy model archives use private materialization.
use crate::{ModelSource, ViewportMode};
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use voxy_assets::{
    AssetId, PROJECT_LAUNCH_SOURCE, PROJECT_PACKAGE_LIMITS, ProjectPackageLaunch, ResourcePackage,
    SourcePath,
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyLaunch {
    version: u32,
    scene: String,
    model: String,
    manifest: Option<String>,
}
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Launch {
    Legacy(LegacyLaunch),
    Project(ProjectPackageLaunch),
}
struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
impl Project {
    fn create() -> Result<Self, std::io::Error> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(std::io::Error::other)?
            .as_nanos();
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let index = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "voxy-packaged-game-{}-{nonce}-{index}",
            std::process::id()
        ));
        let builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        let builder = {
            let mut builder = builder;
            builder.mode(0o700);
            builder
        };
        builder.create(&path)?;
        Ok(Self(path))
    }
}
/// Resolve the package next to a portable executable or inside its macOS bundle.
/// # Errors
/// Returns an error when the executable path has no parent.
pub fn bundled_game_package(executable: &Path) -> std::io::Result<PathBuf> {
    let directory = executable
        .parent()
        .ok_or_else(|| std::io::Error::other("game executable parent missing"))?;
    #[cfg(target_os = "macos")]
    if directory.file_name() == Some(std::ffi::OsStr::new("MacOS"))
        && let Some(contents) = directory.parent()
        && contents.file_name() == Some(std::ffi::OsStr::new("Contents"))
        && contents.parent().and_then(Path::extension) == Some(std::ffi::OsStr::new("app"))
    {
        return Ok(contents.join("Resources/game.vpak"));
    }
    Ok(directory.join("game.vpak"))
}

/// Runs a verified package without requiring its original project sources.
/// # Errors
/// Rejects damaged packages, invalid launch paths, IO, imports and runtime failures.
pub fn run_packaged_game(
    path: &Path,
    mode: ViewportMode,
) -> Result<(), Box<dyn std::error::Error>> {
    run_packaged_configured(path, mode, None, None, None, None, None)
}
/// Runs an isolated verified package with application-owned UI callback setup.
/// Package assets declare action names; executable callbacks come from the host.
/// # Errors
/// Reports package, callback registration, resource and runtime failures.
pub fn run_packaged_game_with_ui_actions(
    path: &Path,
    mode: ViewportMode,
    setup: voxy_gameplay::UiActionSetup,
) -> Result<(), Box<dyn std::error::Error>> {
    run_packaged_configured(path, mode, Some(setup), None, None, None, None)
}
/// Run a package with the same application component codecs used to build it.
/// # Errors
/// Reports package, registration, import and runtime failures.
pub fn run_packaged_game_with_components(
    path: &Path,
    mode: ViewportMode,
    registry: voxy_scene::ComponentRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    run_packaged_configured(path, mode, None, Some(registry), None, None, None)
}
/// Run a project package using the same executable game host as its editor/build.
/// # Errors
/// Reports invalid packages, unsupported legacy hosts and game runtime failures.
pub fn run_packaged_game_with_host(
    path: &Path,
    mode: ViewportMode,
    host: crate::GameHost,
) -> Result<(), Box<dyn std::error::Error>> {
    run_packaged_configured(
        path,
        mode,
        host.ui_actions,
        Some(host.components),
        host.behaviors,
        Some(host.behavior_checkpoints),
        Some(host.ui_action_checkpoints),
    )
}
fn run_packaged_configured(
    path: &Path,
    mode: ViewportMode,
    setup: Option<voxy_gameplay::UiActionSetup>,
    registry: Option<voxy_scene::ComponentRegistry>,
    behaviors: Option<crate::BehaviorSetup>,
    behavior_checkpoints: Option<std::sync::Arc<voxy_scene::BehaviorCheckpointRegistry>>,
    ui_action_checkpoints: Option<std::sync::Arc<voxy_gameplay::UiActionCheckpointRegistry>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !matches!(
        mode,
        ViewportMode::Game | ViewportMode::GameCheck | ViewportMode::GameSmoke
    ) {
        return Err("packaged game requires a game mode".into());
    }
    let (archive, launch) = read_archive(path)?;
    if let Launch::Project(_) = launch {
        return crate::project_runtime::run_packaged_project_host(
            path,
            mode,
            crate::GameHost {
                components: match registry {
                    Some(registry) => registry,
                    None => crate::editor_component_registry()?,
                },
                ui_actions: setup,
                behaviors,
                behavior_checkpoints: behavior_checkpoints.unwrap_or_default(),
                ui_action_checkpoints: ui_action_checkpoints
                    .unwrap_or_else(|| std::sync::Arc::new(crate::ui_action_codecs::registry())),
            },
            archive,
        );
    }
    let MaterializedPackage {
        project,
        launch,
        scene,
        bootstrap,
        ..
    } = MaterializedPackage::from_archive(archive, launch)?;
    let Launch::Legacy(launch) = launch else {
        unreachable!("project archives are launched directly");
    };
    if behaviors.is_some() {
        return Err("behavior host requires a project package".into());
    }
    if behavior_checkpoints
        .as_ref()
        .is_some_and(|registry| !registry.is_empty())
    {
        return Err("behavior checkpoint codecs require a project package".into());
    }
    if ui_action_checkpoints
        .as_ref()
        .is_some_and(|registry| registry.schemas().any(|schema| schema != "voxy.ui.hide.v1"))
    {
        return Err("UI callback checkpoint codecs require a project package".into());
    }
    let source = if launch.manifest.is_some() {
        ModelSource::Manifest {
            path: project.0.join(bootstrap.as_str()),
            asset: AssetId(launch.model),
        }
    } else {
        ModelSource::File(project.0.join(bootstrap.as_str()))
    };
    crate::run_model_viewport_configured_registry(
        &source,
        mode,
        Some(
            &project
                .0
                .join(scene.ok_or("legacy scene missing")?.as_str()),
        ),
        setup,
        registry,
    )
}

fn read_archive(
    path: &Path,
) -> Result<(std::sync::Arc<ResourcePackage>, Launch), Box<dyn std::error::Error>> {
    let limits = PROJECT_PACKAGE_LIMITS;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(u64::try_from(limits.max_document_bytes)? + 1)
        .read_to_end(&mut bytes)?;
    let package = ResourcePackage::from_bytes(&bytes, limits)?;
    let launch =
        serde_json::from_slice(&package.read(&AssetId(PROJECT_LAUNCH_SOURCE.into()), 65_536)?)?;
    Ok((std::sync::Arc::new(package), launch))
}

pub(super) fn read_project_archive(
    path: &Path,
) -> Result<std::sync::Arc<ResourcePackage>, Box<dyn std::error::Error>> {
    let (archive, launch) = read_archive(path)?;
    if !matches!(launch, Launch::Project(_)) {
        return Err("headless sessions require a project package".into());
    }
    Ok(archive)
}
/// Owns extracted sources until the last game/import worker has stopped.
pub(super) struct MaterializedPackage {
    project: Project,
    launch: Launch,
    scene: Option<SourcePath>,
    bootstrap: SourcePath,
    #[cfg(test)]
    pub(super) archive: std::sync::Arc<ResourcePackage>,
}
impl MaterializedPackage {
    #[cfg(test)]
    pub(super) fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let (archive, launch) = read_archive(path)?;
        Self::from_archive(archive, launch)
    }

    fn from_archive(
        package: std::sync::Arc<ResourcePackage>,
        launch: Launch,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let limits = PROJECT_PACKAGE_LIMITS;
        let (scene, bootstrap) = match &launch {
            Launch::Legacy(launch) => {
                if launch.version != 1 {
                    return Err("unsupported package launch version".into());
                }
                (
                    Some(SourcePath::new(launch.scene.clone())?),
                    SourcePath::new(
                        launch
                            .manifest
                            .clone()
                            .unwrap_or_else(|| launch.model.clone()),
                    )?,
                )
            }
            Launch::Project(launch) => (None, launch.descriptor_path()?),
        };
        // Bootstrap files define the root for the ordinary project/model loader.
        if bootstrap.as_str().contains('/') {
            return Err("package bootstrap must be at its root".into());
        }
        let project = Project::create()?;
        for source in package.paths() {
            let output = project.0.join(source);
            std::fs::create_dir_all(output.parent().ok_or("invalid packaged source path")?)?;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)?;
            std::io::Write::write_all(
                &mut file,
                &package.read(&AssetId(source.into()), limits.max_payload_bytes)?,
            )?;
        }
        Ok(Self {
            project,
            launch,
            scene,
            bootstrap,
            #[cfg(test)]
            archive: package,
        })
    }
    #[cfg(test)]
    pub(super) fn project_path(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        if !matches!(self.launch, Launch::Project(_)) {
            return Err("headless sessions require a project package".into());
        }
        Ok(self.project.0.join(self.bootstrap.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_assets::PackageLimits;
    #[test]
    fn launch_rejects_traversal_and_nested_bootstrap_before_running() {
        let project = Project::create().unwrap();
        let file = project.0.join("game.vpak");
        for launch in [
            serde_json::json!({"version":1,"scene":"../outside","model":"mesh.obj","manifest":null}),
            serde_json::json!({"version":1,"scene":"scene.json","model":"mesh","manifest":"nested/assets.json"}),
            serde_json::json!({"version":2,"project":"../project.voxy"}),
            serde_json::json!({"version":2,"project":"nested/project.voxy"}),
            serde_json::json!({"version":3,"project":"project.voxy"}),
            serde_json::json!({"version":2,"project":"project.voxy","unexpected":true}),
        ] {
            let bytes = serde_json::to_vec(&launch).unwrap();
            let limits = PackageLimits {
                max_entries: 1,
                max_payload_bytes: 1024,
                max_document_bytes: 4096,
            };
            let package = ResourcePackage::capture(
                &[SourcePath::new("__voxy_game.json").unwrap()],
                limits,
                |_, _| Ok(bytes.clone()),
            )
            .unwrap();
            std::fs::write(&file, package.value().to_bytes(limits).unwrap()).unwrap();
            assert!(run_packaged_game(&file, ViewportMode::GameCheck).is_err());
        }
        let path = project.0.clone();
        drop(project);
        assert!(!path.exists());
    }
}
