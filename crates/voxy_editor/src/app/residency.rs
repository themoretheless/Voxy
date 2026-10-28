//! Asset polling and CPU/GPU residency synchronisation.
//!
//! Methods of [`App`] split out of lib.rs (plan step C4); they keep the same
//! bodies and private access through `crate::*`.

use crate::*;

impl App {
    pub(crate) fn poll_sources(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(watcher) = &mut self.watcher else {
            return Ok(());
        };
        let scan = watcher.try_observed_result()?;
        if let Some(scan) = scan {
            self.scan_active = false;
            let manifest_changed = matches!(&self.import_recipe,
                InputRecipe::Manifest(manifest) if scan.changes.contains(manifest));
            let inventory = scan.observation.and_then(|(id, bytes)| {
                if manifest_changed
                    && matches!(&self.import_recipe, InputRecipe::Manifest(manifest) if manifest == &id)
                {
                    bytes.ok().and_then(|bytes| import::model_inventory(&bytes).ok())
                } else { None }
            });
            let mut affected: BTreeSet<_> =
                self.sources.affected(scan.changes).into_iter().collect();
            if manifest_changed {
                // A first import may not have recorded its dependencies yet. Its
                // ticket must still be invalidated before accepting a completion.
                affected.extend(
                    self.required_cpu_model_assets()
                        .into_iter()
                        .filter(|id| self.catalog.status(id).is_some()),
                );
            }
            if !affected.is_empty() {
                self.catalog
                    .invalidate(&affected.iter().cloned().collect::<Vec<_>>())?;
                self.reload.extend(affected);
            }
            if let Some(inventory) = inventory {
                self.available = inventory;
            }
        }
        if self.resource_checkpoint.is_none()
            && !self.scan_active
            && self.last_scan.elapsed() >= Duration::from_millis(200)
        {
            let watcher = self.watcher.as_mut().ok_or("source watcher disappeared")?;
            match &self.import_recipe {
                InputRecipe::Manifest(manifest) => {
                    watcher.request_with_observation(&self.sources, manifest.clone(), 65536, 129)?
                }
                InputRecipe::Direct => watcher.request(&self.sources, 129)?,
            }
            self.scan_active = true;
            self.last_scan = Instant::now();
        }
        Ok(())
    }

    pub(crate) fn required_cpu_model_assets(&self) -> BTreeSet<AssetId> {
        self.scene
            .components::<ModelInstance>()
            .map(|(_, model)| model.asset.clone())
            .chain(
                self.scene
                    .components::<ModelRetarget>()
                    .map(|(_, profile)| AssetId(profile.source.clone())),
            )
            .chain(self.id.iter().cloned())
            .collect()
    }

    pub(crate) fn required_gpu_assets(&self) -> std::collections::BTreeSet<AssetId> {
        self.scene
            .active_components::<ModelInstance>()
            .filter(|(node, model)| {
                self.catalog.snapshot(&model.asset).is_none_or(|asset| {
                    asset
                        .value()
                        .mesh_for(
                            self.scene
                                .component::<ModelPart>(*node)
                                .ok()
                                .flatten()
                                .map(|part| part.node),
                        )
                        .is_some()
                })
            })
            .map(|(_, model)| model.asset.clone())
            .collect()
    }

    /// Visible scene references own residency; authoring and CPU catalog stay intact.
    pub(crate) fn synchronize_gpu_residency(&mut self) {
        let required = self.required_gpu_assets();
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        graphics.models.retain(|asset, _| required.contains(asset));
        graphics.residency_cache.geometry_live = graphics.geometry_bytes();
        graphics.residency_cache.prune();
        graphics
            .residency_errors
            .retain(|asset, _| required.contains(asset));
        for id in required {
            if graphics.models.contains_key(&id) && !graphics.residency_errors.contains_key(&id) {
                continue;
            }
            if let Some(asset) = self.catalog.snapshot(&id) {
                if graphics
                    .residency_errors
                    .get(&id)
                    .is_some_and(|deferred| deferred.blocks(&asset, &graphics.residency_cache))
                {
                    continue;
                }
                let model = ModelGraphics::upload_on(
                    &graphics.renderer,
                    &graphics.device,
                    &graphics.queue,
                    asset.value(),
                    &mut graphics.residency_cache,
                );
                match model {
                    Ok(mut model) => {
                        model.publication = Some(asset.clone());
                        graphics.models.insert(id.clone(), model);
                        graphics.residency_cache.geometry_live = graphics.geometry_bytes();
                        graphics.residency_errors.remove(&id);
                    }
                    Err(error) => {
                        let message = error.to_string();
                        if graphics
                            .residency_errors
                            .get(&id)
                            .map(|error| &error.message)
                            != Some(&message)
                        {
                            eprintln!("GPU MODEL DEFERRED asset={} error={error}", id.0);
                        }
                        graphics.residency_errors.insert(
                            id,
                            gpu_model::DeferredUpload::new(
                                &asset,
                                &graphics.residency_cache,
                                message,
                            ),
                        );
                    }
                }
            }
        }
    }
}
