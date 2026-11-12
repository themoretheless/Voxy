//! Platform-independent game host contract, builtin UI action codecs, and zero-extract `.vpak` project bootstrap.
//!
//! `voxy_player` decouples executable game runtime configuration (`GameHost`, `BehaviorSetup`,
//! builtin UI callbacks, and in-memory `.vpak` project loading) from `voxy_editor`, allowing
//! standalone desktop, browser (`voxy_web`), mobile (`voxy_mobile`), and XR (`voxy_xr`) targets
//! to load and validate packaged projects directly from memory without temporary filesystem extraction.

use std::sync::Arc;
use voxy_assets::{
    AssetId, AssetLocations, AssetSourceProvider, ImportInputs, PROJECT_PACKAGE_LIMITS,
    ProjectDescriptor, ResourcePackage, SourcePath,
};
use voxy_gameplay::{
    GameplayError, UiActionCheckpointRegistry, UiActionEvent, UiActionHandler, UiActionHandlers,
    UiActionSetup, UiActionState,
};
use voxy_scene::{
    BehaviorCheckpointRegistry, ComponentRegistry, DocumentError, LoadedScene, NodeId, ObjectId,
    PrefabLimits, PrefabSceneDocument, SceneCommand, SceneCommands, SceneDocument, SceneGraph,
    SceneSimulation,
};

type Setup = dyn FnMut(&mut SceneGraph, &mut SceneSimulation) -> Result<(), Box<dyn std::error::Error>>
    + Send;

/// Attaches fresh behavior instances to the detached runtime graph on every Play or launch.
/// Uses the existing bounded simulation and lifecycle; does not serialize executable code.
pub struct BehaviorSetup(Box<Setup>);

impl std::fmt::Debug for BehaviorSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BehaviorSetup").finish_non_exhaustive()
    }
}

impl BehaviorSetup {
    pub fn new(
        setup: impl FnMut(
            &mut SceneGraph,
            &mut SceneSimulation,
        ) -> Result<(), Box<dyn std::error::Error>>
        + Send
        + 'static,
    ) -> Self {
        Self(Box::new(setup))
    }

    /// Invokes the behavior setup closure on `scene` and `simulation`.
    /// # Errors
    /// Propagates any error returned by the setup closure.
    pub fn configure(
        &mut self,
        scene: &mut SceneGraph,
        simulation: &mut SceneSimulation,
    ) -> Result<(), Box<dyn std::error::Error>> {
        (self.0)(scene, simulation)
    }
}

/// Builtin UI callback that deactivates the owning scene node (`voxy.ui.hide` / `voxy.ui.hide.v1`).
struct HideAction;

impl UiActionHandler for HideAction {
    fn dispatch(
        &mut self,
        _: &SceneGraph,
        event: &UiActionEvent,
        commands: &mut SceneCommands,
    ) -> Result<(), GameplayError> {
        commands.push(SceneCommand::SetActive(event.owner, false))?;
        Ok(())
    }

    fn checkpoint(
        &self,
        _: &dyn Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<UiActionState, GameplayError> {
        UiActionState::new("voxy.ui.hide.v1".into(), Vec::new())
    }
}

/// Registers builtin engine UI callbacks (`voxy.ui.hide`) into `actions`.
/// # Errors
/// Returns [`GameplayError`] if registration fails.
pub fn register_builtin_ui_handlers(actions: &mut UiActionHandlers) -> Result<(), GameplayError> {
    actions.register_checkpointable("voxy.ui.hide".into(), HideAction)
}

/// Returns a [`UiActionCheckpointRegistry`] pre-populated with builtin engine UI action codecs.
#[must_use]
pub fn builtin_ui_action_checkpoint_registry() -> UiActionCheckpointRegistry {
    let mut registry = UiActionCheckpointRegistry::default();
    registry
        .register("voxy.ui.hide.v1", |payload, _| {
            if !payload.is_empty() {
                return Err("invalid hide callback payload".into());
            }
            Ok(Box::new(HideAction))
        })
        .expect("static builtin UI codec is valid and unique");
    registry
}

/// One host contract for editor Play, standalone player, package build, and package launch.
/// Register durable component codecs and supply fresh runtime behavior/UI setup.
#[derive(Debug)]
pub struct GameHost {
    /// Executable behavior state decoders, shared with all project launch paths.
    /// The resource package never serializes these factories.
    pub behavior_checkpoints: Arc<BehaviorCheckpointRegistry>,
    /// Explicit state decoders for opted-in UI callbacks, shared across launch paths.
    pub ui_action_checkpoints: Arc<UiActionCheckpointRegistry>,
    pub components: ComponentRegistry,
    pub behaviors: Option<BehaviorSetup>,
    pub ui_actions: Option<UiActionSetup>,
}

impl GameHost {
    #[must_use]
    pub fn new(components: ComponentRegistry) -> Self {
        Self {
            behavior_checkpoints: Default::default(),
            ui_action_checkpoints: Arc::new(builtin_ui_action_checkpoint_registry()),
            components,
            behaviors: None,
            ui_actions: None,
        }
    }
}

const DEFAULT_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_MANIFEST_BYTES: usize = 65_536;
const DEFAULT_MANIFEST_ENTRIES: usize = 128;

/// Zero-extract in-memory packaged project opened directly from a `.vpak` [`ResourcePackage`].
///
/// Validates the launch bootstrap (`__voxy_game.json`), project descriptor, asset manifest,
/// prefab expansion, and loaded [`LoadedScene`] without touching `std::env::temp_dir()`.
#[derive(Debug)]
pub struct InMemoryPackagedProject {
    pub bootstrap: SourcePath,
    pub descriptor: ProjectDescriptor,
    pub assets: AssetLocations,
    pub document: SceneDocument,
    pub loaded: LoadedScene,
    pub package: Arc<ResourcePackage>,
    pub provider: AssetSourceProvider,
}

impl InMemoryPackagedProject {
    /// Opens and validates a project `.vpak` from raw archive bytes completely in memory.
    /// # Errors
    /// Rejects malformed or oversized archives, invalid project/manifest/scene schemas,
    /// missing prefabs or model bindings, and component decode errors.
    pub fn from_vpak_bytes(bytes: &[u8], registry: &ComponentRegistry) -> Result<Self, String> {
        let package = Arc::new(ResourcePackage::from_bytes(bytes, PROJECT_PACKAGE_LIMITS)?);
        Self::from_package(package, registry)
    }

    /// Opens and validates a project directly from an in-memory [`ResourcePackage`].
    /// # Errors
    /// Rejects invalid launch bootstrap, descriptor, manifest, prefab, or scene documents.
    pub fn from_package(
        package: Arc<ResourcePackage>,
        registry: &ComponentRegistry,
    ) -> Result<Self, String> {
        let (bootstrap, descriptor) = package.open_project_descriptor()?;
        let source_path = |path: &std::path::Path| -> Result<SourcePath, String> {
            let text = path
                .to_str()
                .ok_or_else(|| "project source path must be UTF-8".to_string())?;
            SourcePath::new(text.to_owned()).map_err(|error| error.to_string())
        };
        let manifest_source = source_path(&descriptor.asset_manifest)?;
        let scene_source = source_path(&descriptor.startup_scene)?;
        if manifest_source == bootstrap
            || scene_source == bootstrap
            || manifest_source == scene_source
        {
            return Err("descriptor, manifest and scene must be distinct files".into());
        }

        let provider = AssetSourceProvider::from_package(Arc::clone(&package));
        let mut inputs = ImportInputs::new(130, 16 * 1024 * 1024);
        let manifest_snapshot = inputs
            .read(manifest_source.observation_id(), |id, limit| {
                provider.read(id, limit.min(DEFAULT_MANIFEST_BYTES))
            })
            .map_err(|error| format!("manifest input: {error:?}"))?;
        let manifest_str = std::str::from_utf8(&manifest_snapshot.bytes)
            .map_err(|error| format!("manifest UTF-8: {error}"))?;
        let assets = AssetLocations::from_json(
            manifest_str,
            DEFAULT_MANIFEST_ENTRIES,
            DEFAULT_MANIFEST_BYTES,
            DEFAULT_MANIFEST_BYTES,
        )
        .map_err(|error| format!("asset manifest: {error}"))?;

        for (_, source) in assets.bindings() {
            provider.fingerprint(
                &source.observation_id(),
                PROJECT_PACKAGE_LIMITS.max_payload_bytes,
            )?;
        }

        let scene_snapshot = inputs
            .read(scene_source.observation_id(), |id, limit| {
                provider.read(id, limit.min(DEFAULT_DOCUMENT_BYTES))
            })
            .map_err(|error| format!("scene input: {error:?}"))?;
        let prefab_scene: PrefabSceneDocument =
            serde_json::from_slice(&scene_snapshot.bytes).map_err(|error| error.to_string())?;
        let limits = PrefabLimits {
            max_objects: 4096,
            max_instances: 128,
            max_depth: 16,
        };
        let expanded = prefab_scene
            .expand(registry, limits, |asset| {
                let path = assets.source(&AssetId(asset.into())).ok_or_else(|| {
                    DocumentError::Invalid(format!("unknown prefab asset {asset}"))
                })?;
                let snapshot = inputs
                    .read(path.observation_id(), |id, limit| {
                        provider.read(id, limit.min(DEFAULT_DOCUMENT_BYTES))
                    })
                    .map_err(|error| DocumentError::Invalid(format!("prefab input: {error:?}")))?;
                serde_json::from_slice(&snapshot.bytes).map_err(Into::into)
            })
            .map_err(|error| error.to_string())?;
        let publication = inputs
            .finish(expanded.document, |id, limit| provider.read(id, limit))
            .map_err(|error| format!("packaged scene validation failed: {error:?}"))?;
        let document = publication.value().clone();
        for object in &document.objects {
            if let Some(id) = object
                .components
                .get("editor.model.v1")
                .and_then(serde_json::Value::as_str)
                && assets.source(&AssetId(id.into())).is_none()
            {
                return Err(format!("unresolved model asset {id}"));
            }
        }
        let loaded = document
            .load(registry, limits.max_objects)
            .map_err(|error| error.to_string())?;

        Ok(Self {
            bootstrap,
            descriptor,
            assets,
            document,
            loaded,
            package,
            provider,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use voxy_assets::{PROJECT_LAUNCH_SOURCE, PackageLimits};

    #[test]
    fn in_memory_packaged_project_opens_validates_and_runs_without_temp_dir_extraction() {
        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        files.insert(
            PROJECT_LAUNCH_SOURCE.into(),
            br#"{"version":2,"project":"project.voxy"}"#.to_vec(),
        );
        files.insert(
            "project.voxy".into(),
            br#"{"version":1,"name":"ZeroExtractDemo","asset_manifest":"assets.json","startup_scene":"main.scene.json"}"#.to_vec(),
        );
        files.insert(
            "assets.json".into(),
            br#"{"version":1,"assets":[{"asset":"hero","source":"models/hero.obj"}]}"#.to_vec(),
        );
        files.insert(
            "models/hero.obj".into(),
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
        );
        files.insert(
            "main.scene.json".into(),
            br#"{"version":1,"objects":[{"id":"obj-1","parent":null,"name":"Hero","active":true,"translation":[0.0,1.0,0.0],"rotation":[0.0,0.0,0.0,1.0],"scale":[1.0,1.0,1.0],"components":{"editor.model.v1":"hero"}}]}"#.to_vec(),
        );

        let paths: Vec<SourcePath> = files
            .keys()
            .map(|k| SourcePath::new(k.clone()).unwrap())
            .collect();
        let limits = PackageLimits {
            max_entries: 16,
            max_payload_bytes: 64 * 1024,
            max_document_bytes: 256 * 1024,
        };
        let captured = ResourcePackage::capture(&paths, limits, |id, _| {
            files
                .get(&id.0)
                .cloned()
                .ok_or_else(|| format!("missing {}", id.0))
        })
        .unwrap();
        let vpak_bytes = captured.value().to_bytes(limits).unwrap();

        let mut registry = ComponentRegistry::default();
        registry.register::<String>("editor.model.v1").unwrap();

        let project = InMemoryPackagedProject::from_vpak_bytes(&vpak_bytes, &registry).unwrap();
        assert_eq!(project.descriptor.name, "ZeroExtractDemo");
        assert_eq!(project.document.objects.len(), 1);
        assert_eq!(
            project
                .provider
                .read(&AssetId("models/hero.obj".into()), 1024)
                .unwrap(),
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n"
        );
    }
}
