//! Detached complete GPU model batch. The App transaction owns the final swap.
#![allow(dead_code)]
use super::*;
use std::{future::Future, pin::Pin};
use voxy_assets::{AssetId, ImportedAsset};
type Assets = BTreeMap<AssetId, Arc<ImportedAsset<EditorAsset>>>;
pub(crate) struct ReadyModelBatch {
    pub(crate) assets: Assets,
    pub(crate) models: BTreeMap<AssetId, ModelGraphics>,
    pub(crate) cache: ResidencyCache,
}
pub(crate) struct PendingModelBatch {
    candidate: ReadyModelBatch,
    checks: Pin<Box<dyn Future<Output = Result<(), String>>>>,
}
pub(crate) struct RejectedModelBatch {
    pub(crate) assets: Assets,
    pub(crate) error: String,
}
pub(crate) struct UploadTarget<'a> {
    pub(crate) renderer: &'a SceneRenderer,
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) cache: &'a ResidencyCache,
    /// Actual allocation inventory, including the old models retained until swap.
    pub(crate) live_geometry: u64,
    pub(crate) replaced_geometry: u64,
}
impl UploadTarget<'_> {
    pub(crate) fn stage(self, assets: Assets) -> Result<PendingModelBatch, RejectedModelBatch> {
        self.stage_with(assets, || Ok(()))
    }
    fn stage_with(
        self,
        assets: Assets,
        after_upload: impl FnOnce() -> Result<(), String>,
    ) -> Result<PendingModelBatch, RejectedModelBatch> {
        let oom = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let internal = self.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let staged = (|| {
            let retained = self
                .live_geometry
                .checked_sub(self.replaced_geometry)
                .ok_or("invalid model geometry inventory")?;
            let mut cache = self.cache.fork_for_upload();
            cache.geometry_live = self.live_geometry;
            let mut models = BTreeMap::new();
            let mut candidate_bytes = 0_u64;
            for (id, asset) in &assets {
                let model = ModelGraphics::upload_on(
                    self.renderer,
                    self.device,
                    self.queue,
                    asset.value(),
                    &mut cache,
                )
                .map_err(|e| e.to_string())?;
                candidate_bytes = candidate_bytes
                    .checked_add(model.geometry_bytes())
                    .ok_or("model geometry inventory overflow")?;
                // Admission of the next model includes all earlier candidate allocations.
                cache.geometry_live = self
                    .live_geometry
                    .checked_add(candidate_bytes)
                    .ok_or("model peak geometry inventory overflow")?;
                models.insert(id.clone(), model);
            }
            cache.geometry_live = retained
                .checked_add(candidate_bytes)
                .ok_or("model geometry inventory overflow")?;
            after_upload()?;
            Ok::<_, String>((models, cache))
        })();
        // Always pop synchronously. Dropping a pending batch cannot strand scopes.
        let validation = validation.pop();
        let internal = internal.pop();
        let oom = oom.pop();
        let checks = Box::pin(async move {
            let messages: Vec<_> = [validation.await, internal.await, oom.await]
                .into_iter()
                .flatten()
                .map(|e| e.to_string())
                .collect();
            if messages.is_empty() {
                Ok(())
            } else {
                Err(messages.join("; "))
            }
        });
        match staged {
            Ok((models, cache)) => Ok(PendingModelBatch {
                candidate: ReadyModelBatch {
                    assets,
                    models,
                    cache,
                },
                checks,
            }),
            Err(error) => Err(RejectedModelBatch { assets, error }),
        }
    }
}
impl PendingModelBatch {
    pub(crate) async fn validate(self) -> Result<ReadyModelBatch, RejectedModelBatch> {
        match self.checks.await {
            Ok(()) => Ok(self.candidate),
            Err(error) => Err(RejectedModelBatch {
                assets: self.candidate.assets,
                error,
            }),
        }
    }
}

#[cfg(test)]
#[path = "gpu_model_batch/tests.rs"]
mod tests;
