//! Deferred native reload: GPU admission precedes the exclusive CPU/GPU swap.
use crate::*;
use gpu_model::batch::{ReadyModelBatch, RejectedModelBatch, UploadTarget};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use voxy_assets::{AssetTicket, FailedImport};
type Error = Box<dyn std::error::Error>;
type ResultAsset = Result<ImportedAsset<import::EditorAsset>, FailedImport<String>>;
pub(crate) struct Pending {
    ticket: AssetTicket,
    source: Arc<ImportedAsset<import::EditorAsset>>,
    device: wgpu::Device,
    cache: (u64, u64, u64, u64),
    cache_identity: Arc<()>,
    geometry: u64,
    checks: Pin<Box<dyn Future<Output = Result<ReadyModelBatch, RejectedModelBatch>>>>,
}
impl std::fmt::Debug for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingModelReload")
            .field("ticket", &self.ticket)
            .field("cache", &self.cache)
            .field("geometry", &self.geometry)
            .finish_non_exhaustive()
    }
}
fn take_source(
    mut assets: BTreeMap<AssetId, Arc<ImportedAsset<import::EditorAsset>>>,
    id: &AssetId,
) -> Result<ImportedAsset<import::EditorAsset>, Error> {
    let source = assets
        .remove(id)
        .ok_or("GPU candidate lost CPU publication")?;
    Arc::try_unwrap(source).map_err(|_| "GPU candidate CPU publication unexpectedly shared".into())
}
struct GpuDestination<'a> {
    models: &'a mut BTreeMap<AssetId, ModelGraphics>,
    cache: &'a mut gpu_model::ResidencyCache,
    errors: &'a mut BTreeMap<AssetId, gpu_model::DeferredUpload>,
}
struct SourceWatchAdmission<'a> {
    max_sources: usize,
    max_file_bytes: usize,
    required_source: Option<&'a AssetId>,
}
fn fail_dependency_budget(
    catalog: &mut AssetCatalog<ImportedAsset<import::EditorAsset>>,
    ticket: &AssetTicket,
    reason: &str,
) -> Result<bool, PublicationError> {
    catalog
        .complete(
            ticket,
            Err(format!("model source dependency budget exceeded ({reason}); prior model and watches retained")),
        )
        .map_err(PublicationError::Asset)?;
    Ok(false)
}
fn complete_transaction(
    catalog: &mut AssetCatalog<ImportedAsset<import::EditorAsset>>,
    sources: &mut SourceDependencies,
    ticket: &AssetTicket,
    mut result: ResultAsset,
    mut gpu: Option<ReadyModelBatch>,
    target: Option<GpuDestination<'_>>,
    source_watch: Option<SourceWatchAdmission<'_>>,
) -> Result<bool, PublicationError> {
    if gpu.as_ref().is_some_and(|g| {
        target.is_none() || g.models.len() != 1 || !g.models.contains_key(ticket.asset())
    }) {
        result = result.and_then(|source| {
            Err(source.into_failed("GPU publication destination or candidate changed".into()))
        });
        gpu = None;
    }
    if let Some(policy) = source_watch {
        let (inputs, outcome) = match &result {
            Ok(asset) => (asset.inputs(), voxy_assets::ImportOutcome::Published),
            Err(failed) => (&failed.inputs, voxy_assets::ImportOutcome::Failed),
        };
        if inputs.observations().values().any(|input| {
            input
                .as_ref()
                .is_ok_and(|snapshot| snapshot.bytes.len() > policy.max_file_bytes)
        }) {
            return fail_dependency_budget(catalog, ticket, "watcher source-byte limit");
        }
        match sources.check_record_sources(
            ticket.asset(),
            inputs,
            outcome,
            policy.max_sources,
            policy.required_source,
        ) {
            Ok(()) => {}
            Err(voxy_assets::DependencyError::Capacity) => {
                return fail_dependency_budget(
                    catalog,
                    ticket,
                    "source index or watcher count limit",
                );
            }
            Err(error) => return Err(PublicationError::Sources(error)),
        }
    }
    let successful = result.is_ok();
    match catalog.complete_observed(sources, ticket, result) {
        Ok(()) => {}
        Err(PublicationError::Sources(voxy_assets::DependencyError::Capacity)) => {
            return fail_dependency_budget(catalog, ticket, "source index capacity");
        }
        Err(error) => return Err(error),
    }
    if successful && let (Some(mut gpu), Some(target)) = (gpu, target) {
        let publication = catalog.snapshot(ticket.asset());
        for model in gpu.models.values_mut() {
            model.publication.clone_from(&publication);
        }
        target.models.append(&mut gpu.models);
        *target.cache = gpu.cache;
        target.errors.remove(ticket.asset());
    }
    Ok(successful)
}
impl App {
    pub(crate) fn can_retry_selected_model(&self) -> bool {
        self.imports.is_some()
            && self.play.playing.is_none()
            && self.resource_checkpoint.is_none()
            && self.restore_barrier.is_none()
            && self.model_relink.is_none()
            && self.model_relink_draft.is_none()
            && self.retarget_draft.is_none()
            && self.instances.get(self.selected).is_some_and(|node| {
                self.scene
                    .component::<ModelInstance>(*node)
                    .ok()
                    .flatten()
                    .is_some_and(|model| {
                        self.available.contains(&model.asset)
                            && matches!(
                                self.catalog.status(&model.asset),
                                Some(voxy_assets::AssetStatus::Failed(_))
                            )
                    })
            })
    }
    pub(crate) fn retry_selected_model(&mut self) -> Result<(), Error> {
        if !self.can_retry_selected_model() {
            return Err("model import retry unavailable in the current editor state".into());
        }
        let node = self.instances[self.selected];
        let asset = self
            .scene
            .component::<ModelInstance>(node)?
            .ok_or("selected model disappeared")?
            .asset
            .clone();
        self.catalog.invalidate(std::slice::from_ref(&asset))?;
        self.reload.insert(asset);
        Ok(())
    }
    pub(crate) fn validate_reload_candidate(
        &self,
        output: &AssetId,
        candidate: &import::EditorAsset,
    ) -> Result<(), Error> {
        let parts: Vec<_> = self
            .scene
            .components::<ModelPart>()
            .filter(|(node, _)| {
                self.scene
                    .component::<ModelInstance>(*node)
                    .ok()
                    .flatten()
                    .is_some_and(|model| &model.asset == output)
            })
            .map(|(_, part)| part.node)
            .collect();
        let invalid = parts.iter().any(|part| {
            *part != u32::MAX
                && usize::try_from(*part).map_or(true, |index| index >= candidate.nodes.len())
        });
        let changed = !parts.is_empty()
            && self.catalog.snapshot(output).is_some_and(|previous| {
                previous.value().nodes.len() != candidate.nodes.len()
                    || previous
                        .value()
                        .nodes
                        .iter()
                        .zip(&candidate.nodes)
                        .any(|(a, b)| a.parent != b.parent || a.name != b.name)
            });
        if invalid || changed {
            return Err(
                "imported hierarchy changed; preserve last good model and reimport explicitly"
                    .into(),
            );
        }
        for (owner, controller) in self.scene.components::<ModelLocomotion>() {
            let source = self
                .scene
                .component::<ModelRetarget>(owner)?
                .map(|profile| AssetId(profile.source.clone()))
                .or_else(|| {
                    self.scene
                        .component::<ModelInstance>(owner)
                        .ok()
                        .flatten()
                        .map(|model| model.asset.clone())
                });
            if source.as_ref() == Some(output) {
                let model = candidate
                    .animated
                    .as_ref()
                    .ok_or("locomotion import rejected: locomotion requires an animated model")?;
                controller
                    .validate_model(model)
                    .map_err(|e| format!("locomotion import rejected: {e}"))?;
            }
        }
        Ok(())
    }
    pub(crate) fn stage_or_complete_reload(
        &mut self,
        ticket: AssetTicket,
        result: ResultAsset,
    ) -> Result<(), Error> {
        let required = result.as_ref().ok().is_some_and(|asset| {
            self.scene
                .active_components::<ModelInstance>()
                .any(|(node, model)| {
                    &model.asset == ticket.asset()
                        && asset
                            .value()
                            .mesh_for(
                                self.scene
                                    .component::<ModelPart>(node)
                                    .ok()
                                    .flatten()
                                    .map(|part| part.node),
                            )
                            .is_some()
                })
        });
        if !required || self.graphics.is_none() {
            return self.complete_model_reload(&ticket, result, None);
        }
        let source = result.map_err(|_| "validated model result changed")?;
        let assets = BTreeMap::from([(ticket.asset().clone(), Arc::new(source))]);
        let source = assets[ticket.asset()].clone();
        let graphics = self.graphics.as_ref().ok_or("model graphics disappeared")?;
        let geometry = graphics.geometry_bytes();
        let target = UploadTarget {
            renderer: &graphics.renderer,
            device: &graphics.device,
            queue: &graphics.queue,
            cache: &graphics.residency_cache,
            live_geometry: geometry,
            replaced_geometry: graphics
                .models
                .get(ticket.asset())
                .map_or(0, ModelGraphics::geometry_bytes),
        };
        match target.stage(assets) {
            Ok(pending) => {
                self.pending_model = Some(Pending {
                    ticket,
                    source,
                    device: graphics.device.clone(),
                    cache: graphics.residency_cache.state(),
                    cache_identity: graphics.residency_cache.identity(),
                    geometry,
                    checks: Box::pin(pending.validate()),
                });
                Ok(())
            }
            Err(rejected) => {
                drop(source);
                let source = take_source(rejected.assets, ticket.asset())?;
                self.complete_model_reload(
                    &ticket,
                    Err(source.into_failed(format!("GPU import rejected: {}", rejected.error))),
                    None,
                )
            }
        }
    }
    pub(crate) fn poll_pending_model(&mut self) -> Result<(), Error> {
        let Some(mut pending) = self.pending_model.take() else {
            return Ok(());
        };
        if let Err(error) = pending.device.poll(wgpu::PollType::Poll) {
            self.pending_model = Some(pending);
            return Err(error.into());
        }
        let result = match pending
            .checks
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Pending => {
                self.pending_model = Some(pending);
                return Ok(());
            }
            Poll::Ready(result) => result,
        };
        let ticket = pending.ticket;
        drop(pending.checks);
        drop(pending.source);
        match result {
            Err(rejected) => {
                let source = take_source(rejected.assets, ticket.asset())?;
                self.complete_model_reload(
                    &ticket,
                    Err(source.into_failed(format!("GPU import rejected: {}", rejected.error))),
                    None,
                )
            }
            Ok(mut ready) => {
                let source = take_source(std::mem::take(&mut ready.assets), ticket.asset())?;
                let stable = self.graphics.as_ref().is_some_and(|g| {
                    g.device == pending.device
                        && Arc::ptr_eq(&g.residency_cache.identity(), &pending.cache_identity)
                        && g.residency_cache.state() == pending.cache
                        && g.geometry_bytes() == pending.geometry
                });
                let failure = if stable {
                    self.validate_reload_candidate(ticket.asset(), source.value())
                        .err()
                        .map(|e| e.to_string())
                } else {
                    Some("GPU destination changed during model preparation".into())
                };
                if let Some(error) = failure {
                    return self.complete_model_reload(
                        &ticket,
                        Err(source.into_failed(error)),
                        None,
                    );
                }
                self.complete_model_reload(&ticket, Ok(source), Some(ready))
            }
        }
    }
    pub(crate) fn cancel_pending_model(&mut self) -> Result<(), Error> {
        if let Some(Pending {
            ticket,
            source,
            checks,
            ..
        }) = self.pending_model.take()
        {
            drop(checks);
            let source =
                Arc::try_unwrap(source).map_err(|_| "cancelled GPU publication still shared")?;
            self.complete_model_reload(
                &ticket,
                Err(source.into_failed("GPU publication cancelled during shutdown".into())),
                None,
            )?;
        }
        Ok(())
    }
    fn complete_model_reload(
        &mut self,
        ticket: &AssetTicket,
        result: ResultAsset,
        gpu: Option<ReadyModelBatch>,
    ) -> Result<(), Error> {
        let output = ticket.asset();
        let source_watch = self.watcher.as_ref().map(|watcher| SourceWatchAdmission {
            max_sources: watcher.source_capacity(),
            max_file_bytes: watcher.source_byte_capacity(),
            required_source: match &self.import_recipe {
                InputRecipe::Manifest(id) => Some(id),
                InputRecipe::Direct => None,
            },
        });
        let target = self.graphics.as_mut().map(|g| GpuDestination {
            models: &mut g.models,
            cache: &mut g.residency_cache,
            errors: &mut g.residency_errors,
        });
        match complete_transaction(
            &mut self.catalog,
            &mut self.sources,
            ticket,
            result,
            gpu,
            target,
            source_watch,
        ) {
            Ok(true) => {
                self.recovered |= self.failed_at.is_some();
                self.last_publication = Some(Instant::now());
                if let Some(window) = &self.window {
                    window.set_title("Voxy — model loaded");
                }
                if let Some(asset) = self.catalog.snapshot(output) {
                    println!(
                        "MODEL PUBLISHED frames={} first_x={} asset={}",
                        self.frames,
                        asset.value().mesh.vertices()[0].position[0],
                        output.0
                    );
                }
                Ok(())
            }
            Ok(false) => {
                self.failed_at.get_or_insert(self.frames);
                if let Some(window) = &self.window {
                    window.set_title("Voxy — import failed; showing previous model");
                }
                println!(
                    "MODEL FAILED frames={} last_good={}",
                    self.frames,
                    self.catalog.snapshot(output).is_some()
                );
                Ok(())
            }
            Err(PublicationError::Asset(AssetError::StaleTicket)) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "model_reload/gpu_app_tests.rs"]
mod gpu_app_tests;
