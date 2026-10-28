//! Ordered model import/watch fence; capture stays detached from GPU publication.
#![allow(dead_code)]
use crate::*;
use voxy_assets::{PollWorkerError, SourcePollerCheckpoint};
#[derive(Debug)]
pub(crate) enum Barrier {
    Draining,
    Requested,
    Ready {
        epoch: Arc<()>,
        watcher: Option<SourcePollerCheckpoint>,
        delay: Duration,
    },
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelResourcesCheckpoint {
    version: u32,
    catalog: Vec<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    preview_models: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preview: Option<crate::animation_preview::checkpoint::PreviewCheckpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preview_shared_gpu_owner: Option<voxy_scene::ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preview_shared_runtime_owner: Option<voxy_scene::ObjectId>,
    manifest: Option<String>,
    import_worker: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    playback_sources: Vec<crate::animation_sources::Binding>,
    #[serde(with = "asset_checkpoint_wire::ids")]
    outputs: Vec<AssetId>,
    #[serde(with = "asset_checkpoint_wire::ids")]
    queued: Vec<AssetId>,
    #[serde(with = "asset_checkpoint_wire::ids")]
    current: Vec<AssetId>,
    watcher: Option<Watcher>,
    delay: Duration,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Watcher(#[serde(with = "asset_checkpoint_wire::watcher")] SourcePollerCheckpoint);
#[derive(Clone, Copy)]
pub(crate) struct RestoreLimits {
    pub catalog: import::catalog_checkpoint::Limits,
    pub max_bytes: usize,
    pub max_outputs: usize,
    pub max_queued: usize,
    pub max_sources: usize,
    pub max_file_bytes: usize,
}
struct PreviewCapture {
    checkpoint: Option<crate::animation_preview::checkpoint::PreviewCheckpoint>,
    gpu_owner: Option<voxy_scene::ObjectId>,
    runtime_owner: Option<voxy_scene::ObjectId>,
}
pub(crate) struct PreparedModelResources {
    pub resources: import::catalog_checkpoint::Prepared,
    pub preview_models: Vec<Arc<voxy_model::ModelAsset>>,
    pending_preview: Option<crate::animation_preview::checkpoint::PreparedPreview>,
    preview_shared_gpu_owner: Option<voxy_scene::ObjectId>,
    preview_shared_runtime_owner: Option<voxy_scene::ObjectId>,
    pub preview: Option<crate::animation_preview::Preview>,
    pub recipe: InputRecipe,
    pub outputs: BTreeSet<AssetId>,
    pub queued: BTreeSet<AssetId>,
    pub current: Option<AssetId>,
    pub imports: Option<AssetImportWorker<import::EditorAsset>>,
    pub watcher: Option<SourcePollWorker>,
    pub delay: Duration,
    pub playback_sources: crate::animation_sources::PreparedSources,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelAnimationCheckpoint {
    resources: ModelResourcesCheckpoint,
    runtime: crate::animation_runtime::checkpoint::AnimationRuntimeCheckpoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gpu: Option<crate::gpu_source_bindings::GpuSourceBindings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    accepted_frames: Option<crate::animated_models::checkpoint::AcceptedFrames>,
}
pub(crate) struct PreparedModelAnimation {
    pub resources: PreparedModelResources,
    pub animations: crate::animation_runtime::AnimationRuntime,
    pub gpu: Option<crate::gpu_source_bindings::PreparedBindings>,
    pub accepted_frames:
        Option<BTreeMap<NodeId, crate::animated_models::checkpoint::PreparedFrame>>,
}
pub(crate) struct PendingModelAnimation {
    origin: Option<super::restore_guard::SceneAuthoringGuard>,
    model_origin: Option<super::restore_guard::ModelRuntimeGuard>,
    resources: PreparedModelResources,
    animations: crate::animation_runtime::AnimationRuntime,
    gpu: crate::gpu_model::restore::PendingModels,
}
pub(crate) struct ReadyModelAnimation {
    origin: Option<super::restore_guard::SceneAuthoringGuard>,
    model_origin: Option<super::restore_guard::ModelRuntimeGuard>,
    pub resources: PreparedModelResources,
    pub animations: crate::animation_runtime::AnimationRuntime,
    pub gpu: crate::gpu_model::restore::ReadyModels,
}
impl PreparedModelAnimation {
    pub(crate) fn stage_gpu_for_app(
        self,
        app: &App,
        scene: &SceneGraph,
        target: crate::gpu_model::batch::UploadTarget<'_>,
    ) -> Result<PendingModelAnimation, EditorError> {
        app.require_model_checkpoint()?;
        let origin = super::restore_guard::SceneAuthoringGuard::capture(app);
        let model_origin = super::restore_guard::ModelRuntimeGuard::capture(app)?;
        let mut pending = self.stage_gpu(scene, target)?;
        pending.origin = Some(origin);
        pending.model_origin = Some(model_origin);
        Ok(pending)
    }
    pub(crate) fn stage_gpu(
        self,
        scene: &SceneGraph,
        target: crate::gpu_model::batch::UploadTarget<'_>,
    ) -> Result<PendingModelAnimation, EditorError> {
        let Self {
            resources,
            animations,
            gpu,
            accepted_frames,
        } = self;
        let bindings = gpu.ok_or("model bundle has no GPU source roles")?;
        let gpu = target.stage_restore(scene, &bindings, accepted_frames.unwrap_or_default())?;
        Ok(PendingModelAnimation {
            origin: None,
            model_origin: None,
            resources,
            animations,
            gpu,
        })
    }
}
impl PendingModelAnimation {
    pub(crate) async fn validate(self) -> Result<ReadyModelAnimation, EditorError> {
        let gpu = self.gpu.validate().await?;
        Ok(ReadyModelAnimation {
            origin: self.origin,
            model_origin: self.model_origin,
            resources: self.resources,
            animations: self.animations,
            gpu,
        })
    }
}
impl ReadyModelAnimation {
    /// Recheck native model clock/publication/index/queue/fence observations.
    pub(crate) fn validate_model_origin(&self, app: &App) -> Result<(), EditorError> {
        app.require_model_checkpoint()?;
        self.model_origin
            .as_ref()
            .ok_or("model preparation has no runtime/resource observation")?
            .validate(app)
    }
    /// This checks the scene/authoring part only. The enclosing transaction
    /// must also call validate_model_origin and guard UI/audio/host state
    /// and the GPU destination.
    pub(crate) fn validate_scene_authoring(&self, app: &App) -> Result<(), EditorError> {
        app.require_model_checkpoint()?;
        self.origin
            .as_ref()
            .ok_or("model preparation has no App scene/authoring observation")?
            .validate(app)
    }
}
impl ModelAnimationCheckpoint {
    pub(crate) fn to_bytes(&self, max_bytes: usize) -> Result<Vec<u8>, EditorError> {
        envelope(self, *b"VOXYMAC1", max_bytes)
    }
    pub(crate) fn from_bytes(bytes: &[u8], max_bytes: usize) -> Result<Self, EditorError> {
        decode_envelope(bytes, *b"VOXYMAC1", max_bytes)
    }
    pub(crate) fn prepare(
        &self,
        scene: &SceneGraph,
        resolve: impl Fn(&voxy_scene::ObjectId) -> Option<NodeId>,
        root: &std::path::Path,
        limits: RestoreLimits,
        max_runtime_bytes: usize,
    ) -> Result<PreparedModelAnimation, EditorError> {
        self.to_bytes(limits.max_bytes)?;
        if let Some(preview) = &self.resources.preview {
            preview.to_json(max_runtime_bytes)?;
        }
        let mut resources = self.resources.prepare(root, limits)?;
        let gpu = self
            .gpu
            .as_ref()
            .map(|gpu| {
                gpu.prepare(
                    scene,
                    &resolve,
                    &resources.resources.retained,
                    limits.catalog.assets.max_references,
                )
            })
            .transpose()?;
        if gpu.is_none() && !resources.resources.retained.is_empty() {
            return Err("GPU publications have no stable role bindings".into());
        }
        let accepted_frames = self
            .accepted_frames
            .as_ref()
            .map(|saved| {
                saved.prepare(
                    scene,
                    &resolve,
                    gpu.as_ref().ok_or("accepted frames have no GPU roles")?,
                    max_runtime_bytes,
                )
            })
            .transpose()?;
        let mut animations = self.runtime.prepare(
            scene,
            &resolve,
            |key| {
                let index = key.strip_prefix("model:")?.parse::<usize>().ok()?;
                if format!("model:{index}") != key {
                    return None;
                }
                resources.resources.models.get(index).cloned()
            },
            |key| resources.playback_sources.resolve(key),
            max_runtime_bytes,
        )?;
        resources.bind_preview(
            scene,
            &resolve,
            &mut animations,
            accepted_frames.as_ref(),
            max_runtime_bytes,
        )?;
        validate_animation_resources(
            &animations,
            &resources,
            max_runtime_bytes,
            limits.catalog.assets.models.animation.max_references,
        )?;
        Ok(PreparedModelAnimation {
            resources,
            animations,
            gpu,
            accepted_frames,
        })
    }
}
fn validate_animation_resources(
    animations: &crate::animation_runtime::AnimationRuntime,
    resources: &PreparedModelResources,
    max_runtime_bytes: usize,
    max_sources: usize,
) -> Result<(), EditorError> {
    let mut roots = animations.checkpoint_model_roots();
    for model in &resources.preview_models {
        if !roots.iter().any(|root| Arc::ptr_eq(root, model)) {
            roots.push(model.clone());
        }
    }
    let actual: std::collections::HashSet<_> = roots.iter().map(Arc::as_ptr).collect();
    let expected: std::collections::HashSet<_> =
        resources.resources.models.iter().map(Arc::as_ptr).collect();
    if actual != expected || roots.len() != resources.resources.models.len() {
        return Err("animation model root bindings differ".into());
    }
    let sources = animations.checkpoint_sources(max_runtime_bytes, max_sources)?;
    if !sources.matches(&resources.playback_sources) {
        return Err("animation source root bindings differ".into());
    }
    Ok(())
}

pub(crate) mod play;
fn envelope<T: serde::Serialize>(
    state: &T,
    magic: [u8; 8],
    max_bytes: usize,
) -> Result<Vec<u8>, EditorError> {
    let body = serde_json::to_vec(state)?;
    if body.len().checked_add(40).is_none_or(|n| n > max_bytes) {
        return Err("model resource archive exceeds policy".into());
    }
    let mut bytes = magic.to_vec();
    bytes.extend_from_slice(blake3::hash(&body).as_bytes());
    bytes.extend_from_slice(&body);
    Ok(bytes)
}
fn decode_envelope<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    magic: [u8; 8],
    max_bytes: usize,
) -> Result<T, EditorError> {
    if bytes.len() < 40
        || bytes.len() > max_bytes
        || &bytes[..8] != magic.as_slice()
        || blake3::hash(&bytes[40..]).as_bytes() != &bytes[8..40]
    {
        return Err("invalid model resource archive".into());
    }
    Ok(serde_json::from_slice(&bytes[40..])?)
}
impl ModelResourcesCheckpoint {
    pub(crate) fn to_bytes(&self, max_bytes: usize) -> Result<Vec<u8>, EditorError> {
        envelope(self, *b"VOXYEMR1", max_bytes)
    }
    pub(crate) fn from_bytes(bytes: &[u8], max_bytes: usize) -> Result<Self, EditorError> {
        decode_envelope(bytes, *b"VOXYEMR1", max_bytes)
    }
    /// Validate every detached participant before creating idle workers. This never
    /// rereads a manifest or model; later scans/imports observe the current filesystem.
    pub(crate) fn prepare(
        &self,
        root: &std::path::Path,
        limits: RestoreLimits,
    ) -> Result<PreparedModelResources, EditorError> {
        self.to_bytes(limits.max_bytes)?;
        if self.version != 1 || self.current.len() > 1 || self.delay > Duration::from_millis(200) {
            return Err("invalid model resource state".into());
        }
        let ids = |values: &[AssetId], max| -> Result<BTreeSet<AssetId>, EditorError> {
            if values.len() > max
                || values.iter().any(|id| id.0.is_empty())
                || values.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return Err("invalid model resource identities".into());
            }
            Ok(values.iter().cloned().collect())
        };
        let outputs = ids(&self.outputs, limits.max_outputs)?;
        let queued = ids(&self.queued, limits.max_queued)?;
        let current = self.current.first().cloned();
        let recipe = match &self.manifest {
            None => InputRecipe::Direct,
            Some(path) => InputRecipe::Manifest(SourcePath::new(path.clone())?.observation_id()),
        };
        let mut resources = import::catalog_checkpoint::prepare(&self.catalog, limits.catalog)?;
        if self.preview_models.len() > 2
            || self
                .preview_models
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != self.preview_models.len()
        {
            return Err("invalid preview model root roles".into());
        }
        let preview_models = self
            .preview_models
            .iter()
            .map(|index| {
                resources
                    .models
                    .get(*index)
                    .cloned()
                    .ok_or_else(|| EditorError::from("missing preview model root"))
            })
            .collect::<Result<Vec<_>, EditorError>>()?;
        if current
            .as_ref()
            .is_some_and(|id| !outputs.contains(id) && resources.catalog.snapshot(id).is_none())
        {
            return Err(
                "current model has neither an available source nor an archived publication".into(),
            );
        }
        let pending_preview = self
            .preview
            .as_ref()
            .map(|saved| {
                saved.prepare_unbound(
                    |key| {
                        let index = key.strip_prefix("model:")?.parse::<usize>().ok()?;
                        if format!("model:{index}") != key {
                            return None;
                        }
                        resources.models.get(index).cloned()
                    },
                    limits.max_bytes,
                )
            })
            .transpose()?;
        if let Some(preview) = &pending_preview {
            let roots = preview.model_roots();
            if roots.len() != preview_models.len()
                || roots
                    .iter()
                    .zip(&preview_models)
                    .any(|(a, b)| !Arc::ptr_eq(a, b))
            {
                return Err("preview model role bindings differ".into());
            }
        } else if self.preview_shared_gpu_owner.is_some()
            || self.preview_shared_runtime_owner.is_some()
        {
            return Err("shared preview frame has no preview participant".into());
        }
        let playback_sources = crate::animation_sources::prepare(
            &self.playback_sources,
            &resources.clips,
            std::mem::take(&mut resources.properties),
            limits.catalog.assets.models.animation.max_references,
        )?;
        if let Some(Watcher(watcher)) = &self.watcher {
            watcher.prepare(limits.max_sources, limits.max_file_bytes)?;
            let mut observed: BTreeSet<_> = resources.sources.source_ids().into_iter().collect();
            if let InputRecipe::Manifest(manifest) = &recipe {
                observed.insert(manifest.clone());
            }
            if observed.len() > watcher.max_sources {
                return Err(
                    "restored watcher source budget cannot cover model dependencies".into(),
                );
            }
            // Every accepted publication is joined to its native source output.
            // Failed-attempt paths without bytes remain legitimate unavailable
            // watches; retired GPU-only publications need no filesystem watcher.
            for output in resources.sources.checkpoint().outputs {
                if let Some(asset) = resources.catalog.snapshot(&output.output)
                    && asset.inputs().observations().values().any(|input| {
                        input
                            .as_ref()
                            .is_ok_and(|snapshot| snapshot.bytes.len() > watcher.max_file_bytes)
                    })
                {
                    return Err(
                        "restored watcher byte budget cannot cover accepted model inputs".into(),
                    );
                }
            }
        }
        // Resolve both providers before spawning either thread. No input read occurs.
        let provider = self
            .watcher
            .as_ref()
            .map(|_| FileInputs::new(root))
            .transpose()?;
        let imports = self
            .import_worker
            .then(|| recipe.spawn_worker(root))
            .transpose()?;
        let watcher = match (&self.watcher, provider) {
            (Some(Watcher(saved)), Some(provider)) => Some(SourcePollWorker::new_with_checkpoint(
                provider,
                saved,
                limits.max_sources,
                limits.max_file_bytes,
            )?),
            _ => None,
        };
        Ok(PreparedModelResources {
            resources,
            preview_models,
            pending_preview,
            preview_shared_gpu_owner: self.preview_shared_gpu_owner.clone(),
            preview_shared_runtime_owner: self.preview_shared_runtime_owner.clone(),
            preview: None,
            recipe,
            outputs,
            queued,
            current,
            imports,
            watcher,
            delay: self.delay,
            playback_sources,
        })
    }
}
impl PreparedModelResources {
    pub(crate) fn bind_preview(
        &mut self,
        scene: &SceneGraph,
        resolve: impl Fn(&voxy_scene::ObjectId) -> Option<NodeId>,
        runtime: &mut crate::animation_runtime::AnimationRuntime,
        frames: Option<&BTreeMap<NodeId, crate::animated_models::checkpoint::PreparedFrame>>,
        max: usize,
    ) -> Result<(), EditorError> {
        let Some(saved) = self.pending_preview.take() else {
            if !self.preview_models.is_empty()
                || self.preview_shared_gpu_owner.is_some()
                || self.preview_shared_runtime_owner.is_some()
            {
                return Err("preview roots have no accepted preview participant".into());
            }
            return Ok(());
        };
        let mut preview = saved.bind_resolved(scene, &resolve)?;
        let runtime_owner = self
            .preview_shared_runtime_owner
            .as_ref()
            .map(|id| {
                resolve(id).ok_or_else(|| EditorError::from("missing shared preview runtime role"))
            })
            .transpose()?;
        if let Some(owner) = runtime_owner {
            let frame = runtime
                .checkpoint_frame_roots()
                .into_iter()
                .find(|(node, _)| *node == owner)
                .map(|(_, frame)| frame)
                .ok_or("missing shared preview runtime frame")?;
            preview.restore_shared_frame(frame, max)?;
        }
        if let Some(id) = &self.preview_shared_gpu_owner {
            let owner = resolve(id).ok_or("missing shared preview GPU role")?;
            let frame = frames
                .and_then(|frames| frames.get(&owner))
                .ok_or("missing shared preview GPU frame")?;
            preview.restore_shared_frame(frame.frame.clone(), max)?;
            if let Some(owner) = runtime_owner {
                runtime.restore_shared_frame(owner, frame.frame.clone(), max)?;
            }
        }
        self.preview = Some(preview);
        Ok(())
    }
}
impl App {
    pub(crate) fn preview_runtime_frame_role(
        &self,
        identify: impl Fn(NodeId) -> Option<voxy_scene::ObjectId>,
    ) -> Result<Option<voxy_scene::ObjectId>, EditorError> {
        let Some(preview) = &self.authoring.animation_preview else {
            return Ok(None);
        };
        for (owner, frame) in self.play.animations.checkpoint_frame_roots() {
            if Arc::ptr_eq(&frame, preview.accepted_frame()) {
                return identify(owner)
                    .map(Some)
                    .ok_or_else(|| "unidentified shared preview runtime role".into());
            }
        }
        Ok(None)
    }
    pub(crate) fn capture_preview_checkpoint(
        &self,
        identify: impl Fn(NodeId) -> Option<voxy_scene::ObjectId>,
        max: usize,
    ) -> Result<Option<crate::animation_preview::checkpoint::PreviewCheckpoint>, EditorError> {
        let Some(preview) = &self.authoring.animation_preview else {
            return Ok(None);
        };
        let id = identify(preview.owner).ok_or("unidentified preview owner")?;
        let view = voxy_scene::SceneCheckpointView::new(&self.scene, [(id, preview.owner)])?;
        let models = self.checkpoint_resource_model_roots();
        preview
            .checkpoint(
                &view,
                |model| {
                    models
                        .iter()
                        .position(|root| Arc::ptr_eq(root, model))
                        .map(|index| format!("model:{index}"))
                },
                max,
            )
            .map(Some)
    }
    pub(crate) fn preview_render_frame_role(
        &self,
        animated: &crate::animated_models::AnimatedModels,
        identify: impl Fn(NodeId) -> Option<voxy_scene::ObjectId>,
    ) -> Result<Option<voxy_scene::ObjectId>, EditorError> {
        let Some(preview) = &self.authoring.animation_preview else {
            return Ok(None);
        };
        for (owner, _) in animated.source_publication_owners()? {
            if animated
                .accepted_frame(owner)
                .is_some_and(|(_, frame)| Arc::ptr_eq(frame, preview.accepted_frame()))
            {
                return identify(owner)
                    .map(Some)
                    .ok_or_else(|| "unidentified shared preview GPU role".into());
            }
        }
        Ok(None)
    }
    pub(crate) fn capture_model_animation_checkpoint(
        &self,
        identify: impl Fn(NodeId) -> Option<voxy_scene::ObjectId>,
        limits: import::catalog_checkpoint::Limits,
        max_runtime_bytes: usize,
    ) -> Result<ModelAnimationCheckpoint, EditorError> {
        self.require_model_checkpoint()?;
        let live = self.gpu_publication_roots()?;
        self.capture_model_animation_with_sources(identify, limits, max_runtime_bytes, live)
    }
    fn capture_model_animation_with_sources(
        &self,
        identify: impl Fn(NodeId) -> Option<voxy_scene::ObjectId>,
        limits: import::catalog_checkpoint::Limits,
        max_runtime_bytes: usize,
        live: Option<crate::gpu_source_bindings::LiveSources>,
    ) -> Result<ModelAnimationCheckpoint, EditorError> {
        self.require_model_checkpoint()?;
        let gpu = live
            .as_ref()
            .map(|live| {
                crate::gpu_source_bindings::GpuSourceBindings::capture(
                    live,
                    &identify,
                    limits.assets.max_references,
                )
            })
            .transpose()?;
        let accepted_frames = self
            .graphics
            .as_ref()
            .map(|graphics| {
                graphics
                    .animated_models
                    .checkpoint_accepted_frames(&identify, max_runtime_bytes)
            })
            .transpose()?;
        let models = self.checkpoint_resource_model_roots();
        let sources = std::cell::RefCell::new(self.play.animations.checkpoint_sources(
            max_runtime_bytes,
            limits.assets.models.animation.max_references,
        )?);
        let runtime = self.play.animations.checkpoint(
            &self.scene,
            &identify,
            |model| {
                models
                    .iter()
                    .position(|m| Arc::ptr_eq(m, model))
                    .map(|i| format!("model:{i}"))
            },
            |clip, properties| {
                sources.borrow_mut().intern(
                    clip,
                    properties,
                    limits.assets.models.animation.max_references,
                )
            },
            max_runtime_bytes,
        )?;
        let retained = live.map(|live| live.publications()).unwrap_or_default();
        let (watcher, delay) = self.require_model_checkpoint()?;
        let preview = self.capture_preview_checkpoint(&identify, max_runtime_bytes)?;
        let preview_gpu = self
            .graphics
            .as_ref()
            .map(|graphics| self.preview_render_frame_role(&graphics.animated_models, &identify))
            .transpose()?
            .flatten();
        let resources = self.capture_model_resources_at_barrier(
            limits,
            watcher,
            delay,
            &retained,
            PreviewCapture {
                checkpoint: preview,
                gpu_owner: preview_gpu,
                runtime_owner: self.preview_runtime_frame_role(&identify)?,
            },
        )?;
        if resources.playback_sources != sources.into_inner().bindings {
            return Err("animation sources changed during capture".into());
        }
        Ok(ModelAnimationCheckpoint {
            resources,
            runtime,
            gpu,
            accepted_frames,
        })
    }
    pub(crate) fn begin_model_checkpoint(&mut self) -> Result<(), EditorError> {
        if self.resource_checkpoint.is_some()
            || self.model_relink.is_some()
            || self.model_relink_draft.is_some()
        {
            return Err("model checkpoint already requested or model relink is active".into());
        }
        self.resource_checkpoint = Some(Barrier::Draining);
        Ok(())
    }
    /// Drains normal invalidation-before-publication processing without dispatching IO.
    pub(crate) fn poll_model_checkpoint(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if self.resource_checkpoint.is_none() {
            return Err("model checkpoint not requested".into());
        }
        if matches!(self.resource_checkpoint, Some(Barrier::Ready { .. })) {
            return Ok(true);
        }
        self.tick()?;
        if self.scan_active
            || self.pending_model.is_some()
            || self.catalog.pending() != 0
            || self.imports.as_ref().is_some_and(|w| w.is_busy())
        {
            return Ok(false);
        }
        let watcher = if let Some(watcher) = &mut self.watcher {
            if matches!(self.resource_checkpoint, Some(Barrier::Draining)) {
                match watcher.request_checkpoint() {
                    Ok(()) => self.resource_checkpoint = Some(Barrier::Requested),
                    Err(PollWorkerError::Busy) => return Ok(false),
                    Err(error) => return Err(error.into()),
                }
            }
            let Some(saved) = watcher.try_checkpoint()? else {
                return Ok(false);
            };
            Some(saved)
        } else {
            None
        };
        self.resource_checkpoint = Some(Barrier::Ready {
            epoch: Arc::new(()),
            watcher,
            delay: Duration::from_millis(200).saturating_sub(self.last_scan.elapsed()),
        });
        Ok(true)
    }
    pub(crate) fn capture_model_resources(
        &self,
        limits: import::catalog_checkpoint::Limits,
    ) -> Result<ModelResourcesCheckpoint, EditorError> {
        let (watcher, delay) = self.require_model_checkpoint()?;
        let retained = self
            .gpu_publication_roots()?
            .map(|live| live.publications())
            .unwrap_or_default();
        let identify = |node| {
            self.instances
                .iter()
                .position(|owner| *owner == node)
                .map(|index| self.object_ids[index].clone())
        };
        let preview = self.capture_preview_checkpoint(identify, limits.max_bytes)?;
        let preview_gpu = self
            .graphics
            .as_ref()
            .map(|graphics| self.preview_render_frame_role(&graphics.animated_models, identify))
            .transpose()?
            .flatten();
        self.capture_model_resources_at_barrier(
            limits,
            watcher,
            delay,
            &retained,
            PreviewCapture {
                checkpoint: preview,
                gpu_owner: preview_gpu,
                runtime_owner: self.preview_runtime_frame_role(identify)?,
            },
        )
    }
    fn gpu_publication_roots(
        &self,
    ) -> Result<Option<crate::gpu_source_bindings::LiveSources>, EditorError> {
        let Some(graphics) = &self.graphics else {
            return Ok(None);
        };
        let mut models = Vec::new();
        for (id, model) in &graphics.models {
            models.push((
                id.clone(),
                model
                    .publication
                    .clone()
                    .ok_or("GPU model has no retained CPU publication")?,
            ));
        }
        let owners = graphics.animated_models.source_publication_owners()?;
        Ok(Some(crate::gpu_source_bindings::LiveSources {
            models,
            owners,
        }))
    }
    fn require_model_checkpoint(
        &self,
    ) -> Result<(Option<&SourcePollerCheckpoint>, Duration), EditorError> {
        let Some(Barrier::Ready { watcher, delay, .. }) = &self.resource_checkpoint else {
            return Err("model checkpoint barrier not ready".into());
        };
        if self.scan_active
            || self.pending_model.is_some()
            || self.catalog.pending() != 0
            || self.imports.as_ref().is_some_and(|w| w.is_busy())
        {
            return Err("model checkpoint changed after barrier".into());
        }
        Ok((watcher.as_ref(), *delay))
    }
    /// Runtime roots retain their original indices; preview appends only new
    /// allocations. Catalog and external roots use the same native asset graph.
    pub(crate) fn checkpoint_resource_model_roots(&self) -> Vec<Arc<voxy_model::ModelAsset>> {
        let mut models = self.play.animations.checkpoint_model_roots();
        if let Some(preview) = &self.authoring.animation_preview {
            for model in preview.model_roots() {
                if !models.iter().any(|root| Arc::ptr_eq(root, &model)) {
                    models.push(model);
                }
            }
        }
        models
    }
    fn capture_model_resources_at_barrier(
        &self,
        limits: import::catalog_checkpoint::Limits,
        watcher: Option<&SourcePollerCheckpoint>,
        delay: Duration,
        retained: &[Arc<ImportedAsset<import::EditorAsset>>],
        preview: PreviewCapture,
    ) -> Result<ModelResourcesCheckpoint, EditorError> {
        let playback = self.play.animations.checkpoint_sources(
            limits.max_bytes,
            limits.assets.models.animation.max_references,
        )?;
        let properties: Vec<_> = playback
            .properties
            .iter()
            .map(|p| p.as_ref().clone())
            .collect();
        let models = self.checkpoint_resource_model_roots();
        let preview_models = self
            .authoring
            .animation_preview
            .as_ref()
            .map(|preview| {
                preview
                    .model_roots()
                    .iter()
                    .map(|model| {
                        models
                            .iter()
                            .position(|root| Arc::ptr_eq(root, model))
                            .ok_or_else(|| EditorError::from("preview model root disappeared"))
                    })
                    .collect::<Result<Vec<_>, EditorError>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(ModelResourcesCheckpoint {
            version: 1,
            preview_models,
            preview: preview.checkpoint,
            preview_shared_gpu_owner: preview.gpu_owner,
            preview_shared_runtime_owner: preview.runtime_owner,
            catalog: import::catalog_checkpoint::capture_with_retained(
                &self.catalog,
                &self.sources,
                &models,
                &playback.clips,
                &properties,
                retained,
                limits,
            )?,
            manifest: match &self.import_recipe {
                InputRecipe::Direct => None,
                InputRecipe::Manifest(id) => Some(id.0.clone()),
            },
            import_worker: self.imports.is_some(),
            playback_sources: playback.bindings,
            outputs: self.available.iter().cloned().collect(),
            queued: self.reload.iter().cloned().collect(),
            current: self.id.iter().cloned().collect(),
            watcher: self.watcher.as_ref().and(watcher).cloned().map(Watcher),
            delay,
        })
    }
    /// Cancellation also consumes an issued watcher snapshot before releasing the fence.
    pub(crate) fn resume_model_checkpoint(&mut self) -> Result<bool, EditorError> {
        if matches!(self.resource_checkpoint, Some(Barrier::Requested)) {
            let watcher = self
                .watcher
                .as_mut()
                .ok_or("model watcher disappeared during checkpoint")?;
            if watcher.try_checkpoint()?.is_none() {
                return Ok(false);
            }
        }
        if let Some(Barrier::Ready { delay, .. }) = &self.resource_checkpoint {
            self.last_scan = Instant::now()
                .checked_sub(Duration::from_millis(200).saturating_sub(*delay))
                .ok_or("model poll delay cannot be restored")?;
        }
        self.resource_checkpoint = None;
        Ok(true)
    }
}
#[cfg(test)]
mod tests;
