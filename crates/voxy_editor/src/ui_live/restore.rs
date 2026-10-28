//! Exclusive native UI publication. GPU error scopes settle before the final
//! infallible owner/cache swap; the enclosing App still coordinates other owners.
#![allow(dead_code)]
use super::*;
use crate::{EditorError, gpu_model::ResidencyCache};
use checkpoint::PreparedUiLive;
use std::{future::Future, pin::Pin};

pub(super) struct UiGraphicsTarget<'a> {
    pub(super) cache: &'a mut ResidencyCache,
    pub(super) draws: &'a mut Vec<GpuDraw>,
    pub(super) focus: &'a mut Vec<GpuDraw>,
    pub(super) focus_key: &'a mut Option<ui_draw::FocusRing>,
    pub(super) focus_deferred: &'a mut Option<(ui_draw::FocusRing, (u64, u64, u64, u64))>,
    pub(super) presented: &'a mut bool,
    // Actual native allocation inventory, not a possibly stale cache observation.
    pub(super) geometry_live: u64,
}
pub(crate) struct UiPublicationPermit<'a> {
    live: &'a mut UiLive,
    target: UiGraphicsTarget<'a>,
    renderer: &'a voxy_render::SceneRenderer,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}
#[derive(Debug)]
pub(crate) struct RejectedUiPublication {
    pub(crate) error: String,
    pub(crate) prepared: Box<PreparedUiLive>,
}
impl RejectedUiPublication {
    pub(crate) fn discard(self) -> Option<std::thread::JoinHandle<()>> {
        self.prepared.close()
    }
}
struct Candidate {
    cache: ResidencyCache,
    draws: Vec<GpuDraw>,
}
pub(crate) struct PendingUiPublication<'a> {
    permit: UiPublicationPermit<'a>,
    prepared: PreparedUiLive,
    gpu: Candidate,
    checks: Pin<Box<dyn Future<Output = Result<(), String>>>>,
}
pub(crate) struct ReadyUiPublication<'a> {
    permit: UiPublicationPermit<'a>,
    prepared: PreparedUiLive,
    gpu: Candidate,
}
impl UiLive {
    /// Borrow native destinations until the candidate is accepted or discarded.
    /// Call outside frame encoding, at the enclosing App's load boundary.
    pub(crate) fn checkpoint_publication<'a>(
        &'a mut self,
        graphics: &'a mut Graphics,
    ) -> Result<UiPublicationPermit<'a>, EditorError> {
        self.publication_graphics(graphics, false)
    }
    /// Coordinated restore retains the completed source fence through staging.
    /// Exclusive borrowing prevents owner-side polling until publish or discard.
    pub(crate) fn checkpoint_publication_at_barrier<'a>(
        &'a mut self,
        graphics: &'a mut Graphics,
    ) -> Result<UiPublicationPermit<'a>, EditorError> {
        self.publication_graphics(graphics, true)
    }
    fn publication_graphics<'a>(
        &'a mut self,
        graphics: &'a mut Graphics,
        held_barrier: bool,
    ) -> Result<UiPublicationPermit<'a>, EditorError> {
        let geometry_live = graphics.geometry_bytes();
        let target = UiGraphicsTarget {
            cache: &mut graphics.residency_cache,
            draws: &mut graphics.ui_draws,
            focus: &mut graphics.ui_focus,
            focus_key: &mut graphics.ui_focus_key,
            focus_deferred: &mut graphics.ui_focus_deferred,
            presented: &mut graphics.ui_presented,
            geometry_live,
        };
        self.publication_on_mode(
            target,
            &graphics.renderer,
            &graphics.device,
            &graphics.queue,
            held_barrier,
        )
    }
    pub(super) fn publication_on<'a>(
        &'a mut self,
        target: UiGraphicsTarget<'a>,
        renderer: &'a voxy_render::SceneRenderer,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
    ) -> Result<UiPublicationPermit<'a>, EditorError> {
        self.publication_on_mode(target, renderer, device, queue, false)
    }
    fn publication_on_mode<'a>(
        &'a mut self,
        target: UiGraphicsTarget<'a>,
        renderer: &'a voxy_render::SceneRenderer,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        held_barrier: bool,
    ) -> Result<UiPublicationPermit<'a>, EditorError> {
        if held_barrier {
            if !matches!(self.checkpoint, Some(reload_checkpoint::Barrier::Ready(_)))
                || self.worker.is_some()
                || self.scanning
                || self.retired.iter().any(|worker| !worker.is_finished())
            {
                return Err("coordinated UI publication requires a completed barrier".into());
            }
        } else if self.checkpoint.is_some() {
            return Err("finish or cancel the UI save before loading".into());
        }
        // Active imports may remain owned by the destination until the swap.
        // Publication closes them and returns all old handles for host retirement.
        self.reap().map_err(|error| error.to_string())?;
        Ok(UiPublicationPermit {
            live: self,
            target,
            renderer,
            device,
            queue,
        })
    }
}
impl<'a> UiPublicationPermit<'a> {
    pub(crate) fn stage(
        self,
        prepared: PreparedUiLive,
    ) -> Result<PendingUiPublication<'a>, RejectedUiPublication> {
        self.stage_with(prepared, || Ok(()))
    }
    fn stage_with(
        self,
        prepared: PreparedUiLive,
        after_upload: impl FnOnce() -> Result<(), Box<dyn std::error::Error>>,
    ) -> Result<PendingUiPublication<'a>, RejectedUiPublication> {
        self.stage_with_base(prepared, None, after_upload)
    }
    fn stage_with_base(
        self,
        prepared: PreparedUiLive,
        base: Option<crate::gpu_model::restore::UiUploadBase<'_>>,
        after_upload: impl FnOnce() -> Result<(), Box<dyn std::error::Error>>,
    ) -> Result<PendingUiPublication<'a>, RejectedUiPublication> {
        let oom = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let internal = self.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let staged = (|| {
            let meshes = prepared.publications.published_meshes()?;
            let old_ui = self
                .target
                .draws
                .iter()
                .chain(self.target.focus.iter())
                .try_fold(0_u64, |sum, draw| {
                    sum.checked_add(draw.geometry.allocation_bytes())
                        .ok_or("UI geometry inventory overflow")
                })?;
            let (source, peak_geometry, final_geometry) = base.as_ref().map_or(
                (
                    &*self.target.cache,
                    self.target.geometry_live,
                    self.target.geometry_live,
                ),
                |base| (base.cache, base.peak_geometry, base.final_geometry),
            );
            let other_geometry = final_geometry
                .checked_sub(old_ui)
                .ok_or("inconsistent native UI geometry inventory")?;
            let mut cache = source.fork_for_upload();
            cache.geometry_live = peak_geometry;
            let draws = upload_into(self.renderer, self.device, self.queue, &mut cache, &meshes)?;
            cache.geometry_live = draws.iter().try_fold(other_geometry, |sum, draw| {
                sum.checked_add(draw.geometry.allocation_bytes())
                    .ok_or("UI geometry inventory overflow")
            })?;
            after_upload()?;
            Ok::<_, Box<dyn std::error::Error>>(Candidate { cache, draws })
        })();
        // Pop synchronously, before any await. Subsequent GPU work on this device
        // is outside these scopes; dropping a future cannot leave a scope open.
        let validation = validation.pop();
        let internal = internal.pop();
        let oom = oom.pop();
        let checks = Box::pin(async move {
            let errors = [validation.await, internal.await, oom.await];
            let messages: Vec<_> = errors
                .into_iter()
                .flatten()
                .map(|error| error.to_string())
                .collect();
            if messages.is_empty() {
                Ok(())
            } else {
                Err(messages.join("; "))
            }
        });
        match staged {
            Ok(gpu) => Ok(PendingUiPublication {
                permit: self,
                prepared,
                gpu,
                checks,
            }),
            Err(error) => Err(RejectedUiPublication {
                error: error.to_string(),
                prepared: Box::new(prepared),
            }),
        }
    }
}
impl<'a> PendingUiPublication<'a> {
    /// Does not block the calling thread. Host/device polling follows the normal
    /// wgpu event loop. Cancellation drops staged resources without changing live
    /// owners; explicit discard retains the idle watcher's teardown handle.
    pub(crate) async fn validate(self) -> Result<ReadyUiPublication<'a>, RejectedUiPublication> {
        let Self {
            permit,
            prepared,
            gpu,
            checks,
        } = self;
        match checks.await {
            Ok(()) => Ok(ReadyUiPublication {
                permit,
                prepared,
                gpu,
            }),
            Err(error) => Err(RejectedUiPublication {
                error,
                prepared: Box::new(prepared),
            }),
        }
    }
    pub(crate) fn discard(self) -> Option<std::thread::JoinHandle<()>> {
        self.prepared.close()
    }
}
impl ReadyUiPublication<'_> {
    /// All validation precedes this infallible publication. Old GPU handles are
    /// dropped after the swap; wgpu retains references used by submitted commands.
    /// Host must retain/join returned old worker handles outside the frame loop.
    pub(crate) fn publish(self) -> Vec<std::thread::JoinHandle<()>> {
        let Self {
            permit,
            prepared,
            gpu,
        } = self;
        let PreparedUiLive {
            scene: _,
            publications,
            reload,
        } = prepared;
        let publications::PreparedUiPublications {
            attempted,
            published,
            presented,
            published_text,
            presented_text,
        } = publications;
        let reload_checkpoint::PreparedUiReload {
            sources,
            watcher,
            last_scan,
            deferred,
        } = reload;
        let candidate = UiLive {
            attempted,
            published,
            presented,
            published_text,
            presented_text,
            worker: None,
            retired: Vec::new(),
            deferred: deferred.map(|_| gpu.cache.state()),
            sources,
            watcher,
            scanning: false,
            last_scan,
            checkpoint: None,
            awaiting_first_presentation: true,
        };
        let mut old = std::mem::replace(permit.live, candidate);
        *permit.target.draws = gpu.draws;
        permit.target.focus.clear();
        *permit.target.focus_key = None;
        *permit.target.focus_deferred = None;
        *permit.target.presented = false;
        *permit.target.cache = gpu.cache;
        old.close()
    }
    pub(crate) fn discard(self) -> Option<std::thread::JoinHandle<()>> {
        self.prepared.close()
    }
}

#[cfg(test)]
pub(super) mod tests;

pub(crate) struct RejectedModelsUi {
    pub ui: RejectedUiPublication,
    pub models: Box<crate::gpu_model::restore::ReadyModels>,
}
pub(crate) struct PendingModelsUi<'a> {
    ui: PendingUiPublication<'a>,
    models: crate::gpu_model::restore::ReadyModels,
}
pub(crate) struct ReadyModelsUi<'a> {
    ui: ReadyUiPublication<'a>,
    models: crate::gpu_model::restore::ReadyModels,
}
impl<'a> UiPublicationPermit<'a> {
    pub(crate) fn stage_after_models(
        self,
        prepared: PreparedUiLive,
        scene: &SceneGraph,
        models: crate::gpu_model::restore::ReadyModels,
    ) -> Result<PendingModelsUi<'a>, RejectedModelsUi> {
        if prepared.scene != scene.change_token() {
            return Err(RejectedModelsUi {
                ui: RejectedUiPublication {
                    error: "UI and model target scene observations differ".into(),
                    prepared: Box::new(prepared),
                },
                models: Box::new(models),
            });
        }
        let base = match models.ui_upload_base(
            scene,
            self.renderer,
            self.device,
            self.queue,
            self.target.cache,
            self.target.geometry_live,
        ) {
            Ok(base) => base,
            Err(error) => {
                return Err(RejectedModelsUi {
                    ui: RejectedUiPublication {
                        error,
                        prepared: Box::new(prepared),
                    },
                    models: Box::new(models),
                });
            }
        };
        match self.stage_with_base(prepared, Some(base), || Ok(())) {
            Ok(ui) => Ok(PendingModelsUi { ui, models }),
            Err(ui) => Err(RejectedModelsUi {
                ui,
                models: Box::new(models),
            }),
        }
    }
}
impl<'a> PendingModelsUi<'a> {
    pub(crate) async fn validate(self) -> Result<ReadyModelsUi<'a>, RejectedModelsUi> {
        match self.ui.validate().await {
            Ok(ui) => Ok(ReadyModelsUi {
                ui,
                models: self.models,
            }),
            Err(ui) => Err(RejectedModelsUi {
                ui,
                models: Box::new(self.models),
            }),
        }
    }
    pub(crate) fn discard(self) -> Option<std::thread::JoinHandle<()>> {
        self.ui.discard()
    }
}
impl ReadyModelsUi<'_> {
    /// The host installs returned model owners with its other infallible swaps.
    /// No second cache is returned that could overwrite the combined cache.
    pub(crate) fn publish(
        self,
    ) -> (
        crate::gpu_model::restore::PublishedModelOwners,
        Vec<std::thread::JoinHandle<()>>,
    ) {
        let owners = self.models.into_owners();
        (owners, self.ui.publish())
    }
    pub(crate) fn discard(self) -> Option<std::thread::JoinHandle<()>> {
        self.ui.discard()
    }
}

/// Native source observation for releasing a publication borrow while GPU checks
/// are pending. The enclosing coordinator must also retain/revalidate its epoch.
struct PublicationObservation {
    attempted: Option<SceneUiSnapshot>,
    published: Option<SceneUiSnapshot>,
    presented: Option<SceneUiSnapshot>,
    published_text: Option<Arc<voxy_assets::ImportedAsset<ui_text::UiTextPreparation>>>,
    presented_text: Option<Arc<voxy_assets::ImportedAsset<ui_text::UiTextPreparation>>>,
    awaiting: bool,
}
impl PublicationObservation {
    fn capture(live: &UiLive) -> Self {
        Self {
            attempted: live.attempted.clone(),
            published: live.published.clone(),
            presented: live.presented.clone(),
            published_text: live.published_text.clone(),
            presented_text: live.presented_text.clone(),
            awaiting: live.awaiting_first_presentation,
        }
    }
    fn matches(&self, live: &UiLive) -> bool {
        let same = |a: &Option<Arc<_>>, b: &Option<Arc<_>>| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        self.attempted == live.attempted
            && self.published == live.published
            && self.presented == live.presented
            && self.awaiting == live.awaiting_first_presentation
            && same(&self.published_text, &live.published_text)
            && same(&self.presented_text, &live.presented_text)
    }
}
pub(crate) struct PendingDetachedModelsUi {
    source: PublicationObservation,
    prepared: PreparedUiLive,
    gpu: Candidate,
    models: crate::gpu_model::restore::ReadyModels,
    checks: Pin<Box<dyn Future<Output = Result<(), String>>>>,
}
pub(crate) struct ReadyDetachedModelsUi {
    source: PublicationObservation,
    prepared: PreparedUiLive,
    gpu: Candidate,
    models: crate::gpu_model::restore::ReadyModels,
}
impl PendingModelsUi<'_> {
    pub(crate) fn detach_at_barrier(self) -> Result<PendingDetachedModelsUi, RejectedModelsUi> {
        let PendingUiPublication {
            permit,
            prepared,
            gpu,
            checks,
        } = self.ui;
        if !matches!(
            permit.live.checkpoint,
            Some(reload_checkpoint::Barrier::Ready(_))
        ) {
            return Err(RejectedModelsUi {
                ui: RejectedUiPublication {
                    error: "detached UI requires a completed source barrier".into(),
                    prepared: Box::new(prepared),
                },
                models: Box::new(self.models),
            });
        }
        let source = PublicationObservation::capture(permit.live);
        Ok(PendingDetachedModelsUi {
            source,
            prepared,
            gpu,
            models: self.models,
            checks,
        })
    }
}
impl PendingDetachedModelsUi {
    pub(crate) async fn validate(self) -> Result<ReadyDetachedModelsUi, RejectedModelsUi> {
        match self.checks.await {
            Ok(()) => Ok(ReadyDetachedModelsUi {
                source: self.source,
                prepared: self.prepared,
                gpu: self.gpu,
                models: self.models,
            }),
            Err(error) => Err(RejectedModelsUi {
                ui: RejectedUiPublication {
                    error,
                    prepared: Box::new(self.prepared),
                },
                models: Box::new(self.models),
            }),
        }
    }
    pub(crate) fn discard(self) -> Option<std::thread::JoinHandle<()>> {
        self.prepared.close()
    }
}
impl ReadyDetachedModelsUi {
    pub(crate) fn validate_destination(
        &self,
        scene: &SceneGraph,
        target: &crate::gpu_model::batch::UploadTarget<'_>,
    ) -> Result<(), String> {
        self.models.validate_destination(scene, target)
    }
    /// Caller revalidates its original coordinated source epoch before rebinding.
    pub(crate) fn rebind_at_barrier<'a>(
        self,
        permit: UiPublicationPermit<'a>,
        scene: &SceneGraph,
    ) -> Result<ReadyModelsUi<'a>, RejectedModelsUi> {
        let failure = if !matches!(
            permit.live.checkpoint,
            Some(reload_checkpoint::Barrier::Ready(_))
        ) || !self.source.matches(permit.live)
        {
            Some("UI source changed during detached GPU validation".into())
        } else if self.prepared.scene != scene.change_token() {
            Some("UI and model target scene observations differ".into())
        } else {
            self.models
                .ui_upload_base(
                    scene,
                    permit.renderer,
                    permit.device,
                    permit.queue,
                    permit.target.cache,
                    permit.target.geometry_live,
                )
                .err()
        };
        if let Some(error) = failure {
            return Err(RejectedModelsUi {
                ui: RejectedUiPublication {
                    error,
                    prepared: Box::new(self.prepared),
                },
                models: Box::new(self.models),
            });
        }
        Ok(ReadyModelsUi {
            ui: ReadyUiPublication {
                permit,
                prepared: self.prepared,
                gpu: self.gpu,
            },
            models: self.models,
        })
    }
    pub(crate) fn discard(self) -> Option<std::thread::JoinHandle<()>> {
        self.prepared.close()
    }
}
