//! Detached resource + accepted-animation GPU restore; App owns final publication.
#![allow(dead_code)]
use super::*;
use crate::{
    animated_models::{AnimatedModels, Request, checkpoint::PreparedFrame},
    gpu_source_bindings::PreparedBindings,
};
use std::{future::Future, pin::Pin};
use voxy_scene::{NodeId, SceneGraph, SceneId};
struct Destination {
    scene_change: voxy_scene::SceneChangeToken,
    device: wgpu::Device,
    queue: wgpu::Queue,
    cache_identity: Arc<()>,
    cache_state: (u64, u64, u64, u64),
    image_bytes: u64,
    live_geometry: u64,
    replaced_geometry: u64,
}
pub(crate) struct ReadyModels {
    destination: Destination,
    pub scene: SceneId,
    pub models: BTreeMap<voxy_assets::AssetId, ModelGraphics>,
    pub animated: AnimatedModels,
    pub cache: ResidencyCache,
}
pub(crate) struct PendingModels {
    candidate: ReadyModels,
    checks: Pin<Box<dyn Future<Output = Result<(), String>>>>,
}
impl batch::UploadTarget<'_> {
    pub(crate) fn stage_restore(
        self,
        scene: &SceneGraph,
        bindings: &PreparedBindings,
        frames: BTreeMap<NodeId, PreparedFrame>,
    ) -> Result<PendingModels, String> {
        self.stage_restore_with(scene, bindings, frames, || Ok(()))
    }
    fn stage_restore_with(
        self,
        scene: &SceneGraph,
        bindings: &PreparedBindings,
        frames: BTreeMap<NodeId, PreparedFrame>,
        after_upload: impl FnOnce() -> Result<(), String>,
    ) -> Result<PendingModels, String> {
        if scene.identity() != bindings.scene || !frames.keys().eq(bindings.owners.keys()) {
            return Err("GPU restore scene or accepted owner inventory differs".into());
        }
        let destination = Destination {
            scene_change: scene.change_token(),
            device: self.device.clone(),
            queue: self.queue.clone(),
            cache_identity: self.cache.identity(),
            cache_state: self.cache.state(),
            image_bytes: self.cache.live_bytes(),
            live_geometry: self.live_geometry,
            replaced_geometry: self.replaced_geometry,
        };
        let oom = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let internal = self.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let staged = (|| {
            let retained = self
                .live_geometry
                .checked_sub(self.replaced_geometry)
                .ok_or("invalid restore geometry inventory")?;
            let mut cache = self.cache.fork_for_upload();
            cache.geometry_live = self.live_geometry;
            let mut models = BTreeMap::new();
            let mut materials = BTreeMap::new();
            let mut geometry = 0_u64;
            for (id, publication) in &bindings.models {
                let mut model = ModelGraphics::upload_on(
                    self.renderer,
                    self.device,
                    self.queue,
                    publication.value(),
                    &mut cache,
                )
                .map_err(|e| e.to_string())?;
                geometry = geometry
                    .checked_add(model.geometry_bytes())
                    .ok_or("restore geometry overflow")?;
                cache.geometry_live = self
                    .live_geometry
                    .checked_add(geometry)
                    .ok_or("restore peak geometry overflow")?;
                materials.insert(
                    Arc::as_ptr(publication) as usize,
                    AnimatedMaterials {
                        textures: model.animated_textures.clone(),
                        storage: model._images.clone(),
                    },
                );
                model.publication = Some(publication.clone());
                models.insert(id.clone(), model);
            }
            let other_live = self
                .live_geometry
                .checked_add(geometry)
                .ok_or("restore peak geometry overflow")?;
            let mut animated = AnimatedModels::new(self.renderer).map_err(|e| e.to_string())?;
            for (node, state) in frames {
                scene.local(node).map_err(|e| e.to_string())?;
                let publication = &bindings.owners[&node];
                if publication
                    .value()
                    .animated
                    .as_ref()
                    .is_none_or(|m| !Arc::ptr_eq(m, &state.model))
                {
                    return Err("accepted frame differs from restored source model".into());
                }
                let key = Arc::as_ptr(publication) as usize;
                if let std::collections::btree_map::Entry::Vacant(entry) = materials.entry(key) {
                    entry.insert(
                        upload_animated_materials(
                            self.renderer,
                            self.device,
                            self.queue,
                            publication.value(),
                            &mut cache,
                        )
                        .map_err(|e| e.to_string())?,
                    );
                }
                let material = &materials[&key];
                let request = Request {
                    owner: node,
                    publication: Some(publication.clone()),
                    model: state.model,
                    settings: state.settings,
                    frame: Some(state.frame),
                    lod: state.lod.as_ref().map(|lod| lod.source.clone()),
                    textures: material.textures.clone(),
                    texture_storage: material.storage.clone(),
                };
                animated
                    .restore_accepted_owner(
                        self.renderer,
                        self.device,
                        self.queue,
                        request,
                        state.ticks,
                        other_live,
                        cache.geometry_budget,
                    )
                    .map_err(|e| e.to_string())?;
                if let Some(lod) = state.lod {
                    animated
                        .restore_accepted_lod(
                            self.renderer,
                            self.device,
                            node,
                            lod,
                            other_live,
                            cache.geometry_budget,
                        )
                        .map_err(|e| e.to_string())?;
                }
            }
            geometry = geometry
                .checked_add(animated.allocation_bytes())
                .ok_or("restore geometry overflow")?;
            cache.geometry_live = retained
                .checked_add(geometry)
                .ok_or("restore geometry overflow")?;
            after_upload()?;
            Ok::<_, String>(ReadyModels {
                destination,
                scene: scene.identity(),
                models,
                animated,
                cache,
            })
        })();
        // Pop synchronously before returning, including every failure path.
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
        staged.map(|candidate| PendingModels { candidate, checks })
    }
}
impl ReadyModels {
    /// Recheck immediately before the enclosing exclusive App commit.
    /// The App must additionally guard its CPU scene/history/publication epoch.
    pub(crate) fn validate_destination(
        &self,
        scene: &SceneGraph,
        target: &batch::UploadTarget<'_>,
    ) -> Result<(), String> {
        let d = &self.destination;
        if scene.change_token() != d.scene_change
            || scene.identity() != self.scene
            || target.device != &d.device
            || target.queue != &d.queue
            || !Arc::ptr_eq(&target.cache.identity(), &d.cache_identity)
            || target.cache.state() != d.cache_state
            || target.cache.live_bytes() != d.image_bytes
            || target.live_geometry != d.live_geometry
            || target.replaced_geometry != d.replaced_geometry
        {
            return Err("GPU restore destination changed after preparation".into());
        }
        Ok(())
    }
}
impl PendingModels {
    pub(crate) async fn validate(self) -> Result<ReadyModels, String> {
        self.checks.await?;
        Ok(self.candidate)
    }
}

#[cfg(test)]
#[path = "gpu_model_restore/tests.rs"]
mod tests;

pub(crate) struct UiUploadBase<'a> {
    pub cache: &'a ResidencyCache,
    pub peak_geometry: u64,
    pub final_geometry: u64,
    _validated: (),
}
pub(crate) struct PublishedModelOwners {
    pub scene: SceneId,
    pub models: BTreeMap<voxy_assets::AssetId, ModelGraphics>,
    pub animated: AnimatedModels,
}
impl ReadyModels {
    pub(crate) fn ui_upload_base<'a>(
        &'a self,
        scene: &SceneGraph,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &ResidencyCache,
        live_geometry: u64,
    ) -> Result<UiUploadBase<'a>, String> {
        self.validate_destination(
            scene,
            &batch::UploadTarget {
                renderer,
                device,
                queue,
                cache,
                live_geometry,
                replaced_geometry: self.destination.replaced_geometry,
            },
        )?;
        let retained = self
            .destination
            .live_geometry
            .checked_sub(self.destination.replaced_geometry)
            .ok_or("invalid model restore inventory")?;
        let added =
            self.models
                .values()
                .try_fold(self.animated.allocation_bytes(), |sum, model| {
                    sum.checked_add(model.geometry_bytes())
                        .ok_or("prepared model geometry overflow")
                })?;
        if retained.checked_add(added) != Some(self.cache.geometry_live)
            || self.cache.budget != cache.budget
            || self.cache.geometry_budget != cache.geometry_budget
        {
            return Err("prepared model inventory or cache policy changed".into());
        }
        Ok(UiUploadBase {
            cache: &self.cache,
            peak_geometry: self
                .destination
                .live_geometry
                .checked_add(added)
                .ok_or("combined model/UI geometry overflow")?,
            final_geometry: self.cache.geometry_live,
            _validated: (),
        })
    }
    /// The shared cache is published exactly once by the enclosing UI group.
    pub(crate) fn into_owners(self) -> PublishedModelOwners {
        PublishedModelOwners {
            scene: self.scene,
            models: self.models,
            animated: self.animated,
        }
    }
}
