//! Scene composition uses the same bounded source observations as model import.
use crate::{InputRecipe, scene_limits};
#[cfg(test)]
use crate::model_registry;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use voxy_assets::{AssetId, AssetLocations, FileInputs, ImportInputs, ImportedAsset, SourcePath};
use voxy_scene::{DocumentError, PrefabLimits, PrefabSceneDocument, SceneDocument};
pub(super) struct AuthoringProject {
    pub(super) registry: std::sync::Arc<voxy_scene::ComponentRegistry>,
    root: PathBuf,
    provider: FileInputs,
    manifest: Option<AssetId>,
}
impl std::fmt::Debug for AuthoringProject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthoringProject")
            .field("root", &self.root)
            .field("manifest", &self.manifest)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct AuthoredScene {
    pub(super) source: PrefabSceneDocument,
    pub(super) expanded: SceneDocument,
    pub(super) dependencies: std::collections::BTreeMap<String, PrefabSceneDocument>,
}
impl AuthoringProject {
    #[cfg(test)]
    pub(super) fn new(
        root: &Path,
        recipe: &InputRecipe,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_registry(root, recipe, std::sync::Arc::new(model_registry()?))
    }
    pub(super) fn with_registry(
        root: &Path,
        recipe: &InputRecipe,
        registry: std::sync::Arc<voxy_scene::ComponentRegistry>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            registry,
            root: root.canonicalize()?,
            provider: FileInputs::new(root)?,
            manifest: match recipe {
                InputRecipe::Manifest(id) => Some(id.clone()),
                InputRecipe::Direct(_) => None,
            },
        })
    }
    pub(super) fn worker_project(&self) -> Result<Self, String> {
        Ok(Self {
            registry: self.registry.clone(),
            root: self.root.clone(),
            provider: FileInputs::new(&self.root).map_err(|error| error.to_string())?,
            manifest: self.manifest.clone(),
        })
    }
    pub(super) fn input_provider(&self) -> Result<FileInputs, String> {
        FileInputs::new(&self.root).map_err(|error| error.to_string())
    }
    pub(super) fn decode_ui_font(
        &self,
        asset: &str,
        inputs: &mut ImportInputs,
    ) -> Result<voxy_text::TextFont, String> {
        let locations = self.project_locations(inputs)?;
        let source = Self::resolve_project_input(asset, locations.as_ref())?;
        voxy_gameplay::decode_ui_font_observed(inputs, source.observation_id(), |id, limit| {
            self.provider.read(id, limit.min(4 * 1024 * 1024))
        })
    }
    #[cfg(test)]
    pub(super) fn validate_import_inputs(&self, inputs: &ImportInputs) -> Result<(), String> {
        inputs
            .validate(|id, limit| self.provider.read(id, limit))
            .map_err(|error| format!("project inputs changed: {error:?}"))
    }
    #[cfg(test)]
    pub(super) fn import_ui_font(
        &self,
        asset: &str,
    ) -> Result<ImportedAsset<voxy_text::TextFont>, String> {
        let mut inputs = ImportInputs::new(2, 4 * 1024 * 1024 + 65_536);
        let font = self.decode_ui_font(asset, &mut inputs)?;
        inputs
            .finish(font, |id, limit| self.provider.read(id, limit))
            .map_err(|error| format!("UI font inputs: {error:?}"))
    }
    pub(super) fn import_audio(
        &self,
        asset: &str,
        target_rate: u32,
        settings_asset: Option<&str>,
    ) -> Result<ImportedAsset<voxy_audio::Clip>, String> {
        let mut inputs = ImportInputs::new(3, 8 * 1024 * 1024);
        let clip = self.decode_audio(asset, target_rate, settings_asset, &mut inputs)?;
        inputs
            .finish(clip, |id, limit| self.provider.read(id, limit))
            .map_err(|e| format!("audio inputs: {e:?}"))
    }
    pub(super) fn audio_watcher(
        &self,
        observations: &std::collections::BTreeMap<
            AssetId,
            Result<voxy_assets::InputSnapshot, voxy_assets::InputError>,
        >,
    ) -> Result<voxy_assets::SourcePollWorker, String> {
        self.source_watcher(observations, 768, 8 * 1024 * 1024)
    }
    pub(super) fn source_watcher(
        &self,
        observations: &std::collections::BTreeMap<
            AssetId,
            Result<voxy_assets::InputSnapshot, voxy_assets::InputError>,
        >,
        capacity: usize,
        max_bytes: usize,
    ) -> Result<voxy_assets::SourcePollWorker, String> {
        voxy_assets::SourcePollWorker::new_with_observations(
            FileInputs::new(&self.root).map_err(|e| e.to_string())?,
            capacity,
            max_bytes,
            observations,
        )
        .map_err(|e| e.to_string())
    }
    pub(super) fn audio_worker(
        &self,
        rate: u32,
        recipes: std::collections::BTreeMap<AssetId, Option<String>>,
    ) -> Result<voxy_assets::AssetImportWorker<voxy_audio::Clip>, String> {
        let project = self.worker_project()?;
        voxy_assets::AssetImportWorker::new(
            FileInputs::new(&self.root).map_err(|e| e.to_string())?,
            3,
            8 * 1024 * 1024,
            move |id, _, inputs| {
                let settings = recipes.get(id).ok_or("missing audio import recipe")?;
                project.decode_audio(&id.0, rate, settings.as_deref(), inputs)
            },
        )
        .map_err(|e| e.to_string())
    }
    fn decode_audio(
        &self,
        asset: &str,
        target_rate: u32,
        settings_asset: Option<&str>,
        inputs: &mut ImportInputs,
    ) -> Result<voxy_audio::Clip, String> {
        let locations = self.project_locations(inputs)?;
        let resolve = |id: &str| Self::resolve_project_input(id, locations.as_ref());
        let source = resolve(asset)?;
        let settings = if let Some(id) = settings_asset {
            let observed = inputs
                .read(resolve(id)?.observation_id(), |id, limit| {
                    self.provider.read(id, limit.min(4096))
                })
                .map_err(|e| format!("audio settings: {e:?}"))?;
            voxy_gameplay::AudioImportConfig::from_json(&observed.bytes)?.settings(target_rate)?
        } else {
            voxy_gameplay::AudioImportSettings {
                target_rate,
                max_input_bytes: 8 * 1024 * 1024,
                max_frames: 480_000,
                max_filter_evaluations: 480_000 * 65,
            }
        };
        let cache =
            voxy_assets::ArtifactCache::new(self.root.join(".voxy-cache"), 480_000 * 8 + 16).ok();
        voxy_gameplay::decode_wav_observed(
            inputs,
            source.observation_id(),
            settings,
            cache.as_ref(),
            |id, limit| self.provider.read(id, limit),
        )
    }
    fn project_locations(
        &self,
        inputs: &mut ImportInputs,
    ) -> Result<Option<AssetLocations>, String> {
        let locations = if let Some(manifest) = &self.manifest {
            let observed = inputs
                .read(manifest.clone(), |id, limit| {
                    self.provider.read(id, limit.min(65_536))
                })
                .map_err(|e| format!("project manifest: {e:?}"))?;
            Some(
                AssetLocations::from_json(
                    std::str::from_utf8(&observed.bytes).map_err(|e| e.to_string())?,
                    128,
                    65_536,
                    65_536,
                )
                .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        Ok(locations)
    }
    fn resolve_project_input(
        id: &str,
        locations: Option<&AssetLocations>,
    ) -> Result<SourcePath, String> {
        if let Some(locations) = locations {
            locations
                .source(&AssetId(id.into()))
                .cloned()
                .ok_or_else(|| format!("unknown project input {id}"))
        } else {
            SourcePath::new(id).map_err(|e| e.to_string())
        }
    }
    pub(super) fn audio_settings_draft(
        &self,
        id: &str,
    ) -> Result<super::audio_settings::AudioSettingsDraft, String> {
        let mut inputs = ImportInputs::new(2, 69_632);
        let locations = self.project_locations(&mut inputs)?;
        let source = Self::resolve_project_input(id, locations.as_ref())?.observation_id();
        let observed = inputs
            .read(source.clone(), |id, limit| {
                self.provider.read(id, limit.min(4096))
            })
            .map_err(|e| format!("audio settings: {e:?}"))?;
        let config = voxy_gameplay::AudioImportConfig::from_json(&observed.bytes)?;
        let imported = inputs
            .finish(config, |id, limit| self.provider.read(id, limit))
            .map_err(|e| format!("settings changed: {e:?}"))?;
        let observations = imported
            .inputs()
            .observations()
            .iter()
            .map(|(id, input)| {
                Ok((
                    id.0.clone(),
                    input
                        .as_ref()
                        .map_err(|e| format!("settings input: {e:?}"))?
                        .digest,
                ))
            })
            .collect::<Result<_, String>>()?;
        Ok(super::audio_settings::AudioSettingsDraft {
            config,
            source: source.0,
            observations,
        })
    }
    pub(super) fn save_audio_settings(
        &self,
        draft: &super::audio_settings::AudioSettingsDraft,
        written: &std::collections::BTreeMap<String, [u8; 32]>,
    ) -> Result<[u8; 32], String> {
        draft.config.settings(48000)?;
        if !draft.observations.iter().any(|(id, _)| id == &draft.source) {
            return Err("missing settings observation".into());
        }
        for (id, original) in &draft.observations {
            let limit = if id == &draft.source { 4096 } else { 65_536 };
            let bytes = self.provider.read(&AssetId(id.clone()), limit)?;
            let expected = if id == &draft.source {
                written.get(id).unwrap_or(original)
            } else {
                original
            };
            if blake3::hash(&bytes).as_bytes() != expected {
                return Err(format!(
                    "import settings input {id} changed externally; reopen settings"
                ));
            }
        }
        let path = self
            .root
            .join(&draft.source)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(&self.root) {
            return Err("settings path escapes project".into());
        }
        let bytes = serde_json::to_vec_pretty(&draft.config).map_err(|e| e.to_string())?;
        let digest = *blake3::hash(&bytes).as_bytes();
        draft.config.save_file(&path)?;
        Ok(digest)
    }
    pub(super) fn load(
        &self,
        path: &Path,
    ) -> Result<ImportedAsset<AuthoredScene>, Box<dyn std::error::Error>> {
        let absolute = path.canonicalize()?;
        // Legacy standalone scene paths remain valid; a composed scene must be
        // project scoped because its dependency observations use project IDs.
        let relative = absolute.strip_prefix(&self.root)?;
        let id =
            SourcePath::new(relative.to_str().ok_or("scene path must be UTF-8")?)?.observation_id();
        let mut inputs = ImportInputs::new(130, 16 * 1024 * 1024);
        let snapshot = inputs
            .read(id, |id, limit| self.provider.read(id, limit.min(scene_limits::DOCUMENT_BYTES)))
            .map_err(|error| format!("scene input: {error:?}"))?;
        let source: PrefabSceneDocument = serde_json::from_slice(&snapshot.bytes)?;
        self.expand_inputs(source, inputs)
    }
    pub(super) fn prepare(
        &self,
        source: PrefabSceneDocument,
        path: Option<&Path>,
    ) -> Result<ImportedAsset<AuthoredScene>, Box<dyn std::error::Error>> {
        let mut inputs = ImportInputs::new(130, 16 * 1024 * 1024);
        if let Some(path) = path.filter(|path| path.exists()) {
            let absolute = path.canonicalize()?;
            let relative = absolute.strip_prefix(&self.root)?;
            let id = SourcePath::new(relative.to_str().ok_or("scene path must be UTF-8")?)?
                .observation_id();
            inputs
                .read(id, |id, limit| self.provider.read(id, limit.min(scene_limits::DOCUMENT_BYTES)))
                .map_err(|error| format!("scene input: {error:?}"))?;
        }
        self.expand_inputs(source, inputs)
    }
    fn expand_inputs(
        &self,
        source: PrefabSceneDocument,
        mut inputs: ImportInputs,
    ) -> Result<ImportedAsset<AuthoredScene>, Box<dyn std::error::Error>> {
        let locations = if source.instances.is_empty() {
            None
        } else {
            self.manifest
                .as_ref()
                .map(|manifest| {
                    let snapshot = inputs
                        .read(manifest.clone(), |id, limit| {
                            self.provider.read(id, limit.min(65_536))
                        })
                        .map_err(|error| format!("prefab manifest: {error:?}"))?;
                    AssetLocations::from_json(
                        std::str::from_utf8(&snapshot.bytes).map_err(|error| error.to_string())?,
                        128,
                        65_536,
                        65_536,
                    )
                    .map_err(|error| error.to_string())
                })
                .transpose()?
        };
        let mut dependencies = std::collections::BTreeMap::new();
        let expanded = source.expand(
            &self.registry,
            PrefabLimits {
                max_objects: scene_limits::OBJECTS,
                max_instances: 128,
                max_depth: 16,
            },
            |asset| {
                let path = if let Some(locations) = &locations {
                    locations
                        .source(&AssetId(asset.into()))
                        .cloned()
                        .ok_or_else(|| {
                            DocumentError::Invalid(format!("unknown prefab asset {asset}"))
                        })?
                } else {
                    SourcePath::new(asset)
                        .map_err(|error| DocumentError::Invalid(error.to_string()))?
                };
                let snapshot = inputs
                    .read(path.observation_id(), |id, limit| {
                        self.provider.read(id, limit.min(scene_limits::DOCUMENT_BYTES))
                    })
                    .map_err(|error| DocumentError::Invalid(format!("prefab input: {error:?}")))?;
                let document: PrefabSceneDocument = serde_json::from_slice(&snapshot.bytes)?;
                dependencies.insert(asset.to_owned(), document.clone());
                Ok(document)
            },
        )?;
        inputs
            .finish(
                AuthoredScene {
                    source,
                    expanded: expanded.document,
                    dependencies,
                },
                |id, limit| self.provider.read(id, limit),
            )
            .map_err(|error| format!("scene changed during load: {error:?}").into())
    }
    fn create_prefab(
        &self,
        source: &PrefabSceneDocument,
        expected: &SceneDocument,
    ) -> Result<AssetId, Box<dyn std::error::Error>> {
        let mut inputs = ImportInputs::new(1, 65_536);
        let mut locations = self
            .manifest
            .as_ref()
            .map(|manifest| {
                let input = inputs
                    .read(manifest.clone(), |id, limit| self.provider.read(id, limit))
                    .map_err(|error| format!("prefab manifest: {error:?}"))?;
                AssetLocations::from_json(
                    std::str::from_utf8(&input.bytes).map_err(|error| error.to_string())?,
                    128,
                    65_536,
                    65_536,
                )
                .map_err(|error| error.to_string())
            })
            .transpose()?;
        let candidate = self.prepare(source.clone(), None)?;
        let mut actual = candidate.value().expanded.clone();
        let mut expected = expected.clone();
        actual.objects.sort_by(|a, b| a.id.cmp(&b.id));
        expected.objects.sort_by(|a, b| a.id.cmp(&b.id));
        if actual != expected {
            return Err("prefab export changed selected objects or references".into());
        }
        let bytes = serde_json::to_vec_pretty(&candidate.value().source)?;
        if bytes.len() > scene_limits::DOCUMENT_BYTES {
            return Err("prefab exceeds document budget".into());
        }
        let (asset, path) = (1..=4096)
            .find_map(|number| {
                let name = format!("prefab-{number}.prefab");
                let asset = AssetId(name.clone());
                let path = self.root.join(&name);
                (!path.exists()
                    && locations
                        .as_ref()
                        .is_none_or(|locations| locations.source(&asset).is_none()))
                .then_some((asset, path))
            })
            .ok_or("prefab filename capacity exhausted")?;
        if let Some(locations) = &mut locations {
            locations.bind(asset.clone(), SourcePath::new(asset.0.clone())?)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        // The new file is an independent asset. A failed manifest publication leaves
        // it recoverable and never overwrites a pre-existing file or the scene.
        if let (Some(locations), Some(manifest)) = (locations, &self.manifest) {
            inputs
                .validate(|id, limit| self.provider.read(id, limit))
                .map_err(|error| {
                    format!(
                        "prefab retained at {}; manifest changed: {error:?}",
                        path.display()
                    )
                })?;
            locations
                .save_manifest(&self.root.join(&manifest.0), 65_536)
                .map_err(|error| {
                    format!(
                        "prefab retained at {}; manifest publication failed: {error}",
                        path.display()
                    )
                })?;
        }
        Ok(asset)
    }
    pub(super) fn prefabs(&self) -> Result<Vec<AssetId>, Box<dyn std::error::Error>> {
        self.asset_choices("prefab")
    }
    pub(super) fn audio_assets(&self) -> Result<Vec<AssetId>, Box<dyn std::error::Error>> {
        self.asset_choices("wav")
    }
    fn asset_choices(&self, extension: &str) -> Result<Vec<AssetId>, Box<dyn std::error::Error>> {
        let mut assets = Vec::new();
        if let Some(manifest) = &self.manifest {
            let input = self
                .provider
                .read(manifest, 65_536)
                .map_err(|error| format!("asset manifest: {error:?}"))?;
            let locations =
                AssetLocations::from_json(std::str::from_utf8(&input)?, 128, 65_536, 65_536)?;
            assets.extend(
                locations
                    .bindings()
                    .filter(|(_, source)| {
                        std::path::Path::new(source.as_str())
                            .extension()
                            .is_some_and(|ext| ext == extension)
                    })
                    .map(|(asset, _)| asset.clone()),
            );
        } else {
            for (index, entry) in std::fs::read_dir(&self.root)?.enumerate() {
                if index >= 2048 {
                    return Err("asset directory exceeds discovery limit".into());
                }
                let entry = entry?;
                if entry.file_type()?.is_file()
                    && entry.path().extension().is_some_and(|ext| ext == extension)
                {
                    let name = entry
                        .file_name()
                        .into_string()
                        .map_err(|_| "asset path must be UTF-8")?;
                    SourcePath::new(name.clone())?;
                    assets.push(AssetId(name));
                }
            }
        }
        if assets.len() > 128 {
            return Err("asset discovery capacity exceeded".into());
        }
        assets.sort();
        Ok(assets)
    }
    pub(super) fn save(
        &self,
        path: &Path,
        imported: &ImportedAsset<AuthoredScene>,
        snapshot: &AuthoredScene,
        source: &PrefabSceneDocument,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.validate(imported)?;
        // Undo can restore composition metadata from an earlier file load. Its
        // captured dependency revisions must still match before saving.
        let mut inputs = ImportInputs::new(130, 16 * 1024 * 1024);
        let locations = self
            .manifest
            .as_ref()
            .map(|manifest| {
                let input = inputs
                    .read(manifest.clone(), |id, limit| {
                        self.provider.read(id, limit.min(65_536))
                    })
                    .map_err(|error| format!("prefab manifest: {error:?}"))?;
                AssetLocations::from_json(
                    std::str::from_utf8(&input.bytes).map_err(|error| error.to_string())?,
                    128,
                    65_536,
                    65_536,
                )
                .map_err(|error| error.to_string())
            })
            .transpose()?;
        for (asset, expected) in &snapshot.dependencies {
            let path = if let Some(locations) = &locations {
                locations
                    .source(&AssetId(asset.clone()))
                    .cloned()
                    .ok_or("historical prefab dependency is missing")?
            } else {
                SourcePath::new(asset)?
            };
            let input = inputs
                .read(path.observation_id(), |id, limit| {
                    self.provider.read(id, limit.min(scene_limits::DOCUMENT_BYTES))
                })
                .map_err(|error| format!("prefab source: {error:?}"))?;
            let current: PrefabSceneDocument = serde_json::from_slice(&input.bytes)?;
            if &current != expected {
                return Err("historical prefab source changed; reload before saving".into());
            }
        }
        inputs
            .validate(|id, limit| self.provider.read(id, limit))
            .map_err(|error| format!("prefab inputs changed during save: {error:?}"))?;
        voxy_scene::save_prefab_scene_file(
            path,
            source,
            &self.registry,
            PrefabLimits {
                max_objects: scene_limits::OBJECTS,
                max_instances: 128,
                max_depth: 16,
            },
            scene_limits::DOCUMENT_BYTES,
            |asset| {
                snapshot.dependencies.get(asset).cloned().ok_or_else(|| {
                    DocumentError::Invalid(format!("unobserved prefab dependency {asset}"))
                })
            },
        )?;
        Ok(())
    }
    pub(super) fn source_path(
        &self,
        path: &Path,
    ) -> Result<SourcePath, Box<dyn std::error::Error>> {
        let absolute = path.canonicalize()?;
        Ok(SourcePath::new(
            absolute
                .strip_prefix(&self.root)?
                .to_str()
                .ok_or("package path must be UTF-8")?,
        )?)
    }
    pub(super) fn capture_package(
        &self,
        paths: &[SourcePath],
        launch: &[u8],
    ) -> Result<ImportedAsset<voxy_assets::ResourcePackage>, Box<dyn std::error::Error>> {
        let mut all = paths.to_vec();
        let launch_path = SourcePath::new("__voxy_game.json")?;
        if all.contains(&launch_path) {
            return Err("reserved package launch path is already a source".into());
        }
        all.push(launch_path);
        voxy_assets::ResourcePackage::capture(
            &all,
            voxy_assets::PackageLimits {
                max_entries: 4096,
                max_payload_bytes: 64 * 1024 * 1024,
                max_document_bytes: 256 * 1024 * 1024,
            },
            |id, limit| {
                if id.0 == "__voxy_game.json" {
                    if launch.len() > limit {
                        return Err("package launch budget exceeded".into());
                    }
                    Ok(launch.to_vec())
                } else {
                    self.provider.read(id, limit)
                }
            },
        )
        .map_err(Into::into)
    }
    pub(super) fn smoke_dependency(
        &self,
        scene: &ImportedAsset<AuthoredScene>,
    ) -> Result<(PathBuf, Vec<u8>), Box<dyn std::error::Error>> {
        let (id, observation) = scene
            .inputs()
            .observations()
            .iter()
            .find(|(id, _)| id.0.ends_with(".prefab"))
            .ok_or("prefab acceptance requires a .prefab dependency")?;
        let snapshot = observation
            .as_ref()
            .map_err(|error| format!("prefab observation: {error:?}"))?;
        Ok((self.root.join(&id.0), snapshot.bytes.to_vec()))
    }
    pub(super) fn validate(
        &self,
        scene: &ImportedAsset<AuthoredScene>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        scene
            .inputs()
            .validate(|id, limit| self.provider.read(id, limit))
            .map_err(|error| {
                format!("scene dependencies changed; reload before saving: {error:?}").into()
            })
    }
}

/// Keep complete linked instances; copy partial instance selections as independent
/// authored rows. This leaves source assets untouched and retains nested links.
fn export_linked_subtree(
    snapshot: &AuthoredScene,
    edited: &SceneDocument,
    selected: &std::collections::BTreeSet<voxy_scene::ObjectId>,
    registry: &voxy_scene::ComponentRegistry,
) -> Result<PrefabSceneDocument, Box<dyn std::error::Error>> {
    let baseline =
        snapshot.source.instance_baseline(
            &registry,
            PrefabLimits {
                max_objects: scene_limits::OBJECTS,
                max_instances: 128,
                max_depth: 16,
            },
            |asset| {
                snapshot.dependencies.get(asset).cloned().ok_or_else(|| {
                    DocumentError::Invalid(format!("missing historical prefab {asset}"))
                })
            },
        )?;
    let mut source = snapshot
        .source
        .capture_edits(&baseline, edited, &registry, scene_limits::OBJECTS)?;
    let authored: std::collections::BTreeSet<_> = snapshot
        .source
        .objects
        .iter()
        .map(|object| &object.id)
        .collect();
    let live: std::collections::BTreeSet<_> =
        edited.objects.iter().map(|object| &object.id).collect();
    let mut retained = std::collections::BTreeSet::new();
    source.instances.retain_mut(|instance| {
        let prefix = format!("{}:{}", instance.id.0.len(), instance.id.0);
        let owned: Vec<_> = baseline
            .objects
            .iter()
            .filter(|object| {
                !authored.contains(&object.id)
                    && object.id.0.starts_with(&prefix)
                    && live.contains(&object.id)
            })
            .map(|object| &object.id)
            .collect();
        if owned.is_empty() || !owned.iter().all(|id| selected.contains(*id)) {
            return false;
        }
        retained.extend(owned.into_iter().cloned());
        if instance
            .parent
            .as_ref()
            .is_some_and(|parent| !selected.contains(parent))
        {
            instance.parent = None;
        }
        true
    });
    source.objects = edited
        .objects
        .iter()
        .filter(|object| selected.contains(&object.id) && !retained.contains(&object.id))
        .cloned()
        .collect();
    Ok(source)
}

impl crate::App {
    pub(super) fn create_prefab_from_selection(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(observed) = &self.authoring.authoring_source {
            self.authoring.authoring_project.validate(observed)?;
        }
        let document = self.authoring_document()?;
        let root = self
            .object_ids
            .get(self.selected)
            .ok_or("select a subtree to create prefab")?
            .clone();
        let mut selected = std::collections::BTreeSet::from([root.clone()]);
        loop {
            let before = selected.len();
            for object in &document.objects {
                if object
                    .parent
                    .as_ref()
                    .is_some_and(|parent| selected.contains(parent))
                {
                    selected.insert(object.id.clone());
                }
            }
            if before == selected.len() {
                break;
            }
        }
        let mut edited = document.clone();
        edited
            .objects
            .iter_mut()
            .find(|object| object.id == root)
            .ok_or("missing selected root")?
            .parent = None;
        let expected = SceneDocument {
            version: edited.version,
            objects: edited
                .objects
                .iter()
                .filter(|object| selected.contains(&object.id))
                .cloned()
                .collect(),
        };
        let metadata = self.authoring.history.as_ref().ok_or("missing history")?.metadata();
        let source = if metadata.is_null() {
            PrefabSceneDocument {
                version: 1,
                objects: expected.objects.clone(),
                instances: Vec::new(),
            }
        } else {
            let snapshot: AuthoredScene = serde_json::from_value(metadata.clone())?;
            export_linked_subtree(
                &snapshot,
                &edited,
                &selected,
                &self.authoring.authoring_project.registry,
            )?
        };
        let asset = self.authoring.authoring_project.create_prefab(&source, &expected)?;
        if let Some(observed) = &self.authoring.authoring_source {
            let refreshed = self.authoring
                .authoring_project
                .prepare(observed.value().source.clone(), self.authoring.scene_path.as_deref())?;
            self.authoring.authoring_source = Some(refreshed);
        }
        self.authoring.prefab_assets = self.authoring.authoring_project.prefabs()?;
        self.authoring.prefab_choice = self.authoring
            .prefab_assets
            .iter()
            .position(|id| *id == asset)
            .ok_or("created prefab missing from inventory")?;
        self.update_edit_title();
        Ok(())
    }
    pub(super) fn place_prefab(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let asset = self.authoring
            .prefab_assets
            .get(self.authoring.prefab_choice)
            .ok_or("choose a prefab asset")?
            .clone();
        let registry = self.authoring.authoring_project.registry.clone();
        let edited = self.authoring_document()?;
        let history = self.authoring.history.as_ref().ok_or("missing history")?;
        if let Some(observed) = &self.authoring.authoring_source {
            self.authoring.authoring_project.validate(observed)?;
        }
        let snapshot = if history.metadata().is_null() {
            None
        } else {
            Some(serde_json::from_value::<AuthoredScene>(
                history.metadata().clone(),
            )?)
        };
        let mut source = if let Some(snapshot) = &snapshot {
            let baseline = snapshot.source.instance_baseline(
                &registry,
                PrefabLimits {
                    max_objects: scene_limits::OBJECTS,
                    max_instances: 128,
                    max_depth: 16,
                },
                |asset| {
                    snapshot.dependencies.get(asset).cloned().ok_or_else(|| {
                        DocumentError::Invalid(format!("missing historical prefab {asset}"))
                    })
                },
            )?;
            snapshot
                .source
                .capture_edits(&baseline, &edited, &registry, scene_limits::OBJECTS)?
        } else {
            PrefabSceneDocument {
                version: 1,
                objects: edited.objects,
                instances: Vec::new(),
            }
        };
        let mut next = self.authoring.next_object_id;
        let id = loop {
            let id = voxy_scene::ObjectId(format!("prefab-{next}"));
            next = next.checked_add(1).ok_or("prefab identity exhausted")?;
            if !source.instances.iter().any(|instance| instance.id == id)
                && !source.objects.iter().any(|object| object.id == id)
            {
                break id;
            }
        };
        let prefix = format!("{}:{}", id.0.len(), id.0);
        source.instances.push(voxy_scene::PrefabInstance {
            id,
            asset: asset.0,
            parent: None,
            overrides: std::collections::BTreeMap::new(),
        });
        let candidate = self.authoring
            .authoring_project
            .prepare(source, self.authoring.scene_path.as_deref())?;
        if let Some(snapshot) = &snapshot {
            for (asset, expected) in &snapshot.dependencies {
                if candidate.value().dependencies.get(asset) != Some(expected) {
                    return Err("historical prefab source changed; reload before placing".into());
                }
            }
        }
        let document = &candidate.value().expanded;
        self.validate_authoring_document(document)?;
        if let Some(observed) = &self.authoring.authoring_source {
            self.authoring.authoring_project.validate(observed)?;
        }
        let selected = document.objects.iter().position(|object| {
            object.id.0.starts_with(&prefix) && !self.object_ids.contains(&object.id)
        });
        self.authoring.history
            .as_mut()
            .ok_or("missing history")?
            .commit_with_metadata(
                document.clone(),
                serde_json::to_value(candidate.value())?,
                &registry,
            )?;
        self.restore_authoring()?;
        self.selected = selected.unwrap_or(self.selected);
        self.authoring.next_object_id = next;
        self.authoring.authoring_source = Some(candidate);
        self.update_edit_title();
        Ok(())
    }
    /// Revert one linked instance without changing siblings or its source asset.
    pub(super) fn revert_prefab(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let history = self.authoring.history.as_ref().ok_or("missing history")?;
        if history.metadata().is_null() {
            return Err("selected object is not a prefab instance".into());
        }
        let snapshot: AuthoredScene = serde_json::from_value(history.metadata().clone())?;
        self.authoring.authoring_project.validate(
            self.authoring.authoring_source
                .as_ref()
                .ok_or("missing observed authoring publication")?,
        )?;
        let observed = self.authoring
            .authoring_source
            .as_ref()
            .ok_or("missing observed authoring publication")?;
        for (asset, document) in &snapshot.dependencies {
            if observed.value().dependencies.get(asset) != Some(document) {
                return Err("historical prefab source changed; reload before reverting".into());
            }
        }
        let selected = self
            .object_ids
            .get(self.selected)
            .ok_or("select an instance object")?;
        if snapshot
            .source
            .objects
            .iter()
            .any(|object| &object.id == selected)
        {
            return Err("selected object is not a prefab instance".into());
        }
        let instance = snapshot
            .source
            .instances
            .iter()
            .position(|instance| {
                let prefix = voxy_scene::instance_object_id(
                    &instance.id,
                    &voxy_scene::ObjectId(String::new()),
                );
                // The final zero is the local ID length; retain the framed instance prefix.
                selected
                    .0
                    .starts_with(prefix.0.strip_suffix("0:").unwrap_or(&prefix.0))
            })
            .ok_or("selected object is not a prefab instance")?;
        let registry = self.authoring.authoring_project.registry.clone();
        let limits = PrefabLimits {
            max_objects: scene_limits::OBJECTS,
            max_instances: 128,
            max_depth: 16,
        };
        let resolve =
            |asset: &str| {
                snapshot.dependencies.get(asset).cloned().ok_or_else(|| {
                    DocumentError::Invalid(format!("missing historical prefab {asset}"))
                })
            };
        let baseline = snapshot
            .source
            .instance_baseline(&registry, limits, resolve)?;
        if !baseline.objects.iter().any(|object| &object.id == selected) {
            return Err("selected object is not a source instance object".into());
        }
        let edited = self.authoring_document()?;
        let mut source = snapshot
            .source
            .capture_edits(&baseline, &edited, &registry, scene_limits::OBJECTS)?;
        source.instances[instance].overrides.clear();
        let expanded = source.expand(&registry, limits, resolve)?.document;
        self.validate_authoring_document(&expanded)?;
        let metadata = serde_json::to_value(AuthoredScene {
            source,
            expanded: expanded.clone(),
            dependencies: snapshot.dependencies,
        })?;
        self.authoring.history
            .as_mut()
            .ok_or("missing history")?
            .commit_with_metadata(expanded, metadata, &registry)?;
        self.restore_authoring()?;
        self.update_edit_title();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::App;
    use winit::keyboard::KeyCode;
    #[test]
    fn subtree_export_keeps_nested_links_overrides_and_authored_parent() {
        let object = serde_json::json!({"id":"root","parent":null,"name":"Root","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{}});
        let leaf: PrefabSceneDocument = serde_json::from_value(
            serde_json::json!({"version":1,"objects":[object],"instances":[]}),
        )
        .unwrap();
        let nested: PrefabSceneDocument = serde_json::from_value(serde_json::json!({"version":1,"objects":[],"instances":[{"id":"child","asset":"leaf","parent":null,"overrides":{}}]})).unwrap();
        let mut host = leaf.objects[0].clone();
        host.id = voxy_scene::ObjectId("host".into());
        let source: PrefabSceneDocument = serde_json::from_value(serde_json::json!({"version":1,"objects":[host],"instances":[{"id":"first","asset":"nested","parent":"host","overrides":{}},{"id":"second","asset":"nested","parent":"host","overrides":{}}]})).unwrap();
        let dependencies =
            std::collections::BTreeMap::from([("leaf".into(), leaf), ("nested".into(), nested)]);
        let expanded = source
            .expand(
                &model_registry().unwrap(),
                PrefabLimits::default(),
                |asset| Ok(dependencies[asset].clone()),
            )
            .unwrap()
            .document;
        let snapshot = AuthoredScene {
            source,
            expanded: expanded.clone(),
            dependencies,
        };
        let mut edited = expanded;
        edited.objects[1].translation[0] = 3.;
        edited.objects[2].name = "Sibling".into();
        let selected = edited
            .objects
            .iter()
            .map(|object| object.id.clone())
            .collect();
        let exported =
            export_linked_subtree(&snapshot, &edited, &selected, &model_registry().unwrap())
                .unwrap();
        assert_eq!(exported.objects.len(), 1);
        assert_eq!(exported.instances.len(), 2);
        assert!(
            exported
                .instances
                .iter()
                .all(|instance| instance.asset == "nested" && !instance.overrides.is_empty())
        );
        assert_eq!(
            exported
                .expand(
                    &model_registry().unwrap(),
                    PrefabLimits::default(),
                    |asset| Ok(snapshot.dependencies[asset].clone())
                )
                .unwrap()
                .document,
            edited
        );
        // Exporting a linked root outside its authored parent detaches it without
        // flattening nested source links or changing source definitions.
        let mut detached = edited.clone();
        detached.objects[1].parent = None;
        let selected = std::collections::BTreeSet::from([detached.objects[1].id.clone()]);
        let exported =
            export_linked_subtree(&snapshot, &detached, &selected, &model_registry().unwrap())
                .unwrap();
        assert!(exported.objects.is_empty());
        assert_eq!(exported.instances.len(), 1);
        let result = exported
            .expand(
                &model_registry().unwrap(),
                PrefabLimits::default(),
                |asset| Ok(snapshot.dependencies[asset].clone()),
            )
            .unwrap()
            .document;
        assert_eq!(result.objects, vec![detached.objects[1].clone()]);
    }
    #[test]
    fn flat_project_scene_save_rejects_external_edits_and_refreshes_own_revision() {
        let root = std::env::temp_dir().join(format!("voxy-flat-save-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let model = root.join("mesh.obj");
        std::fs::write(&model, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        let mut app = App::new(&model, false).unwrap();
        let path = root.join("scene.json");
        let original = app.authoring_document().unwrap();
        std::fs::write(&path, original.to_json().unwrap()).unwrap();
        app.configure_scene(&path).unwrap();
        assert!(app.authoring.authoring_source.is_some());
        assert!(app.authoring.history.as_ref().unwrap().metadata().is_null());
        app.scene.set_name(app.instances[0], "First save").unwrap();
        app.save_authoring().unwrap();
        app.scene.set_name(app.instances[0], "Second save").unwrap();
        app.save_authoring().unwrap();
        let retained = app.authoring_document().unwrap();
        let mut external = retained.clone();
        external.objects[0].name = "External edit".into();
        let bytes = external.to_json().unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(app.save_authoring().is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), bytes);
        assert_eq!(app.authoring_document().unwrap(), retained);
        app.load_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), external);
        app.save_authoring().unwrap();
        app.stop_workers().unwrap();
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn editor_saves_member_override_and_inherits_other_motion_fields_on_reload() {
        let root = std::env::temp_dir().join(format!("voxy-member-save-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let model = root.join("mesh.obj");
        std::fs::write(&model, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        let mut app = App::new(&model, false).unwrap();
        let mut leaf = app.authoring_document().unwrap();
        leaf.objects[0].components.insert(
            "game.angular-motion.v1".into(),
            serde_json::json!({"axis":[0,0,1],"radians_per_second":1}),
        );
        let leaf_path = root.join("leaf.prefab");
        std::fs::write(&leaf_path, leaf.to_json().unwrap()).unwrap();
        let scene_path = root.join("scene.json");
        std::fs::write(&scene_path, serde_json::json!({"version":1,"objects":[],"instances":[{"id":"outer","asset":"leaf.prefab","parent":null,"overrides":{}}]}).to_string()).unwrap();
        app.configure_scene(&scene_path).unwrap();
        let mut edited = app.authoring_document().unwrap();
        edited.objects[0]
            .components
            .get_mut("game.angular-motion.v1")
            .unwrap()["radians_per_second"] = serde_json::json!(2);
        app.authoring.history
            .as_mut()
            .unwrap()
            .commit(edited, &model_registry().unwrap())
            .unwrap();
        app.restore_authoring().unwrap();
        app.save_authoring().unwrap();
        let saved: PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(&scene_path).unwrap()).unwrap();
        let overrides = saved.instances[0].overrides.values().next().unwrap();
        assert!(overrides.components.is_empty());
        assert_eq!(
            overrides.component_members["game.angular-motion.v1"].len(),
            1
        );
        leaf.objects[0]
            .components
            .get_mut("game.angular-motion.v1")
            .unwrap()["axis"] = serde_json::json!([0, 1, 0]);
        std::fs::write(&leaf_path, leaf.to_json().unwrap()).unwrap();
        app.load_authoring().unwrap();
        let loaded = app.authoring_document().unwrap();
        assert_eq!(
            loaded.objects[0].components["game.angular-motion.v1"]["axis"],
            serde_json::json!([0.0, 1.0, 0.0])
        );
        assert_eq!(
            loaded.objects[0].components["game.angular-motion.v1"]["radians_per_second"],
            serde_json::json!(2.0)
        );
        app.inspector = crate::InspectorMode::Behavior;
        let overridden = app.prefab_overridden_fields(&loaded).unwrap();
        assert_eq!(overridden, std::collections::BTreeSet::from([3]));
        let mut panels = crate::panels::Panels::new().unwrap();
        panels.overridden_fields = overridden;
        panels
            .build(
                &loaded,
                0,
                glam::Vec2::new(800.0, 600.0),
                false,
                None,
                0,
                false,
                crate::InspectorMode::Behavior,
                "Texture: none",
                "Prefab",
                None,
            )
            .unwrap();
        panels.frame_outcome(voxy_render::RenderOutcome::Presented);
        let rect = panels
            .regions
            .iter()
            .find(|(_, action)| *action == crate::panels::Action::ResetField(3))
            .unwrap()
            .0;
        let action = panels
            .hit(glam::Vec2::new(
                rect[0] + rect[2] / 2.0,
                rect[1] + rect[3] / 2.0,
            ))
            .unwrap();
        assert_eq!(action, crate::panels::Action::ResetField(3));
        app.panel_action(action).unwrap();
        let reset = app.authoring_document().unwrap();
        assert_eq!(
            reset.objects[0].components["game.angular-motion.v1"]["radians_per_second"],
            serde_json::json!(1.0)
        );
        assert_eq!(
            reset.objects[0].components["game.angular-motion.v1"]["axis"],
            loaded.objects[0].components["game.angular-motion.v1"]["axis"]
        );
        assert!(app.prefab_overridden_fields(&reset).unwrap().is_empty());
        app.history_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), loaded);
        app.history_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), reset);
        app.save_authoring().unwrap();
        let clean: PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(&scene_path).unwrap()).unwrap();
        assert!(clean.instances[0].overrides.is_empty());
        let mut legacy = clean.clone();
        legacy.instances[0].overrides.insert(
            leaf.objects[0].id.clone(),
            voxy_scene::ObjectOverride {
                components: std::collections::BTreeMap::from([(
                    "game.angular-motion.v1".into(),
                    Some(serde_json::json!({"axis":[1,0,0],"radians_per_second":2})),
                )]),
                ..voxy_scene::ObjectOverride::default()
            },
        );
        std::fs::write(&scene_path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        app.load_authoring().unwrap();
        app.panel_action(crate::panels::Action::ResetField(3))
            .unwrap();
        app.save_authoring().unwrap();
        let split: PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(&scene_path).unwrap()).unwrap();
        let split = split.instances[0].overrides.values().next().unwrap();
        assert!(split.components.is_empty());
        assert_eq!(
            split.component_members["game.angular-motion.v1"]
                .keys()
                .collect::<Vec<_>>(),
            vec!["/axis"]
        );
        leaf.objects[0]
            .components
            .get_mut("game.angular-motion.v1")
            .unwrap()["radians_per_second"] = serde_json::json!(3);
        std::fs::write(&leaf_path, leaf.to_json().unwrap()).unwrap();
        app.load_authoring().unwrap();
        let retained = app.authoring_document().unwrap();
        assert_eq!(
            retained.objects[0].components["game.angular-motion.v1"]["radians_per_second"],
            serde_json::json!(3.0)
        );
        assert_eq!(
            retained.objects[0].components["game.angular-motion.v1"]["axis"],
            serde_json::json!([1.0, 0.0, 0.0])
        );
        app.inspector = crate::InspectorMode::Components(0);
        let members = crate::component_fields::fields(&retained.objects[0]).unwrap();
        let speed = members
            .iter()
            .position(|field| field.path == "/radians_per_second")
            .unwrap();
        app.edit_component_field(speed, "4").unwrap();
        assert!(
            app.prefab_overridden_fields(&app.authoring_document().unwrap())
                .unwrap()
                .contains(&speed)
        );
        app.panel_action(crate::panels::Action::ResetField(speed))
            .unwrap();
        assert_eq!(app.authoring_document().unwrap(), retained);
        let axis = members
            .iter()
            .position(|field| field.path == "/axis/0")
            .unwrap();
        app.panel_action(crate::panels::Action::ResetField(axis))
            .unwrap();
        assert_eq!(
            app.authoring_document().unwrap().objects[0].components["game.angular-motion.v1"]["axis"],
            serde_json::json!([0.0, 1.0, 0.0])
        );
        app.history_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), retained);
        app.save_authoring().unwrap();
        app.load_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), retained);
        app.inspector = crate::InspectorMode::Behavior;
        let metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        std::fs::write(&leaf_path, "external invalid source").unwrap();
        assert!(
            app.panel_action(crate::panels::Action::ResetField(3))
                .is_err()
        );
        assert_eq!(app.authoring_document().unwrap(), retained);
        assert_eq!(app.authoring.history.as_ref().unwrap().metadata(), &metadata);
        app.stop_workers().unwrap();
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }

    // One causal file/history/reload sequence verifies publication across boundaries.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn editor_nested_prefab_edits_save_undo_and_dependency_failure_retain_scene() {
        let root = std::env::temp_dir().join(format!("voxy-editor-prefabs-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("mesh.obj"),
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        let object = serde_json::json!({"id":"root","parent":null,"name":"Model","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"editor.model.v1":"mesh"}});
        let leaf = serde_json::json!({"version":1,"objects":[object],"instances":[]});
        std::fs::write(root.join("leaf.prefab"), leaf.to_string()).unwrap();
        std::fs::write(root.join("nested.prefab"), serde_json::json!({"version":1,"objects":[],"instances":[{"id":"child","asset":"leaf","parent":null,"overrides":{}}]}).to_string()).unwrap();
        std::fs::write(root.join("assets.json"), serde_json::json!({"version":1,"assets":[{"asset":"mesh","source":"mesh.obj"},{"asset":"leaf","source":"leaf.prefab"},{"asset":"nested","source":"nested.prefab"}]}).to_string()).unwrap();
        let scene_path = root.join("scene.json");
        std::fs::write(&scene_path, serde_json::json!({"version":1,"objects":[],"instances":[{"id":"first","asset":"nested","parent":null,"overrides":{}},{"id":"second","asset":"nested","parent":null,"overrides":{}}]}).to_string()).unwrap();
        let mut app =
            App::from_manifest(&root.join("assets.json"), AssetId("mesh".into()), false).unwrap();
        assert_eq!(app.available.len(), 1);
        app.configure_scene(&scene_path).unwrap();
        assert_eq!(app.instances.len(), 2);
        assert_eq!(
            app.authoring.authoring_source
                .as_ref()
                .unwrap()
                .inputs()
                .observations()
                .len(),
            4
        );
        let identities = app.object_ids.clone();
        let original = app.authoring_document().unwrap();
        let mut edited = original.clone();
        edited.objects[0].translation[0] = 3.0;
        edited.objects[0].name = "Edited prefab".into();
        app.authoring.history
            .as_mut()
            .unwrap()
            .commit(edited.clone(), &model_registry().unwrap())
            .unwrap();
        app.restore_authoring().unwrap();
        app.save_authoring().unwrap();
        let saved: PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(&scene_path).unwrap()).unwrap();
        assert!(saved.objects.is_empty());
        assert_eq!(saved.instances.len(), 2);
        assert_eq!(saved.instances[0].overrides.len(), 1);
        assert!(saved.instances[1].overrides.is_empty());
        app.load_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), edited);
        assert_eq!(app.object_ids, identities);
        // Play mutates a detached graph; Stop must preserve both the authoring
        // rows and prefab source metadata, without inserting a history entry.
        let before_play_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        let before_play_bytes = std::fs::read(&scene_path).unwrap();
        let authoring_handle = app.instances[0];
        app.toggle_play().unwrap();
        assert!(app.play.playing.is_some());
        assert!(app.scene.name(authoring_handle).is_err());
        let runtime_handle = app.instances[0];
        app.scene.set_name(runtime_handle, "Runtime only").unwrap();
        app.scene.set_active(runtime_handle, false).unwrap();
        assert_ne!(app.authoring_document().unwrap(), edited);
        app.edit_key(KeyCode::F5).unwrap();
        assert_eq!(std::fs::read(&scene_path).unwrap(), before_play_bytes);
        app.toggle_play().unwrap();
        assert!(app.play.playing.is_none());
        assert!(app.scene.name(runtime_handle).is_err());
        assert_eq!(app.authoring_document().unwrap(), edited);
        assert_eq!(app.object_ids, identities);
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &before_play_metadata
        );
        assert_eq!(std::fs::read(&scene_path).unwrap(), before_play_bytes);
        app.history_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), original);
        app.history_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), edited);
        app.selected = 0;
        app.panel_action(crate::panels::Action::RevertPrefab)
            .unwrap();
        assert_eq!(app.authoring_document().unwrap(), original);
        app.save_authoring().unwrap();
        let reverted: PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(&scene_path).unwrap()).unwrap();
        assert!(
            reverted
                .instances
                .iter()
                .all(|instance| instance.overrides.is_empty())
        );
        app.history_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), edited);
        app.history_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), original);
        app.history_key(KeyCode::KeyZ).unwrap();
        app.save_authoring().unwrap();
        let retained = app.authoring_document().unwrap();
        std::fs::write(root.join("leaf.prefab"), b"broken source").unwrap();
        let history_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        assert!(
            app.panel_action(crate::panels::Action::RevertPrefab)
                .is_err()
        );
        assert_eq!(app.authoring.history.as_ref().unwrap().metadata(), &history_metadata);
        assert_eq!(app.authoring_document().unwrap(), retained);
        assert!(app.load_authoring().is_err());
        assert_eq!(app.authoring_document().unwrap(), retained);
        let before = std::fs::read(&scene_path).unwrap();
        assert!(app.save_authoring().is_err());
        assert_eq!(std::fs::read(&scene_path).unwrap(), before);
        std::fs::write(root.join("leaf.prefab"), leaf.to_string()).unwrap();
        app.load_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), retained);
        // Delete through the existing editor command, save a tombstone, then undo
        // and save again. This must restore the linked instance, not a baked node.
        app.selected = 0;
        app.edit_key(KeyCode::Delete).unwrap();
        let deleted = app.authoring_document().unwrap();
        assert_eq!(deleted.objects.len(), 1);
        app.save_authoring().unwrap();
        let tombstone: PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(&scene_path).unwrap()).unwrap();
        assert!(
            tombstone.instances[0]
                .overrides
                .values()
                .any(|value| value.deleted)
        );
        assert!(tombstone.objects.is_empty());
        app.history_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), retained);
        app.save_authoring().unwrap();
        app.load_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), retained);
        assert!(
            app.authoring.authoring_source
                .as_ref()
                .unwrap()
                .value()
                .source
                .objects
                .is_empty()
        );
        // A different composition with identical flattened rows still creates a
        // metadata undo entry. Reload/saving must not erase the old source links.
        let old_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        let mut replacement = app.authoring
            .authoring_source
            .as_ref()
            .unwrap()
            .value()
            .source
            .clone();
        replacement.objects = retained.objects.clone();
        replacement.instances.clear();
        std::fs::write(&scene_path, serde_json::to_vec(&replacement).unwrap()).unwrap();
        app.load_authoring().unwrap();
        assert!(app.authoring.history.as_ref().unwrap().metadata().is_null());
        app.history_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring.history.as_ref().unwrap().metadata(), &old_metadata);
        app.save_authoring().unwrap();
        let restored_links: PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(&scene_path).unwrap()).unwrap();
        assert_eq!(restored_links.instances.len(), 2);
        assert!(restored_links.objects.is_empty());
        let mut siblings = app.authoring_document().unwrap();
        siblings.objects[1].name = "Independent sibling edit".into();
        app.authoring.history
            .as_mut()
            .unwrap()
            .commit(siblings.clone(), &model_registry().unwrap())
            .unwrap();
        app.restore_authoring().unwrap();
        app.selected = 0;
        app.panel_action(crate::panels::Action::RevertPrefab)
            .unwrap();
        let isolated = app.authoring_document().unwrap();
        assert_eq!(isolated.objects[0], original.objects[0]);
        assert_eq!(isolated.objects[1], siblings.objects[1]);
        app.history_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), siblings);
        app.panel_action(crate::panels::Action::PlacePrefab)
            .unwrap();
        let placed = app.authoring_document().unwrap();
        assert_eq!(placed.objects.len(), siblings.objects.len() + 1);
        let placed_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        assert_eq!(
            placed_metadata["source"]["instances"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        app.history_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), siblings);
        app.history_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), placed);
        app.save_authoring().unwrap();
        app.load_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), placed);
        let retained_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        let retained_bytes = std::fs::read(&scene_path).unwrap();
        std::fs::write(root.join("leaf.prefab"), b"broken source").unwrap();
        assert!(
            app.panel_action(crate::panels::Action::PlacePrefab)
                .is_err()
        );
        assert_eq!(app.authoring_document().unwrap(), placed);
        assert_eq!(app.authoring.history.as_ref().unwrap().metadata(), &retained_metadata);
        assert_eq!(std::fs::read(&scene_path).unwrap(), retained_bytes);
        std::fs::write(root.join("leaf.prefab"), leaf.to_string()).unwrap();
        let before_creation = app.authoring_document().unwrap();
        let before_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        app.panel_action(crate::panels::Action::CreatePrefab)
            .unwrap();
        assert_eq!(app.authoring_document().unwrap(), before_creation);
        assert_eq!(app.authoring.history.as_ref().unwrap().metadata(), &before_metadata);
        let created = app.authoring.prefab_assets[app.authoring.prefab_choice].clone();
        let template: PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(root.join(&created.0)).unwrap()).unwrap();
        assert!(template.objects.is_empty());
        assert_eq!(template.instances.len(), 1);
        let exported = app.authoring.authoring_project.prepare(template, None).unwrap();
        assert_eq!(exported.value().expanded.objects.len(), 1);
        assert!(exported.value().expanded.objects[0].parent.is_none());
        app.panel_action(crate::panels::Action::PlacePrefab)
            .unwrap();
        assert_eq!(
            app.authoring_document().unwrap().objects.len(),
            before_creation.objects.len() + 1
        );
        app.save_authoring().unwrap();
        let created_scene = app.authoring_document().unwrap();
        app.load_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), created_scene);
        app.history_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before_creation);
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }
}
