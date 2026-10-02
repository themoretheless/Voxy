//! Native owner publication; font IO and shaping stay on the existing importer.
use super::{Graphics, prefab_authoring::AuthoringProject, ui_draw, ui_text};
use std::sync::Arc;
use voxy_assets::{AssetCatalog, AssetId, AssetImportWorker};
use voxy_gameplay::SceneUiSnapshot;
use voxy_render::{ImageAsset, ImageLimits, SceneGeometry, SceneTexture};
use voxy_scene::{NodeId, SceneGraph};

#[derive(Debug)]
pub(super) struct GpuDraw {
    pub(super) owner: NodeId,
    pub(super) geometry: SceneGeometry,
    pub(super) texture: Option<Arc<SceneTexture>>,
}
#[derive(Debug, Default)]
pub(super) struct UiLive {
    attempted: Option<SceneUiSnapshot>,
    published: Option<SceneUiSnapshot>,
    presented: Option<SceneUiSnapshot>,
    worker: Option<AssetImportWorker<ui_text::UiTextPreparation>>,
    retired: Vec<std::thread::JoinHandle<()>>,
    deferred: Option<(u64, u64, u64, u64)>,
    sources: Option<voxy_assets::SourceDependencies>,
    watcher: Option<voxy_assets::SourcePollWorker>,
    scanning: bool,
    last_scan: Option<std::time::Instant>,
}
impl UiLive {
    pub(super) fn close(&mut self) -> Vec<std::thread::JoinHandle<()>> {
        self.attempted = None;
        self.published = None;
        self.presented = None;
        self.deferred = None;
        self.sources = None;
        self.scanning = false;
        self.last_scan = None;
        self.retired.extend(
            self.watcher
                .take()
                .map(voxy_assets::SourcePollWorker::close),
        );
        self.retired
            .extend(self.worker.take().map(AssetImportWorker::close));
        std::mem::take(&mut self.retired)
    }
    pub(super) fn focus_snapshot(&self) -> Option<&SceneUiSnapshot> {
        (self.published == self.presented)
            .then_some(self.published.as_ref())
            .flatten()
    }
    pub(super) fn presented(&self) -> Option<&SceneUiSnapshot> {
        self.presented.as_ref()
    }
    // Only call after a successful native presentation, never on GPU upload or
    // an occluded/skipped surface. Keep the old visible snapshot until that point.
    pub(super) fn frame_presented(&mut self) {
        if self.presented != self.published {
            self.presented.clone_from(&self.published);
        }
    }
    pub(super) fn ready(&self) -> bool {
        self.worker.is_none() && self.published.is_some() && self.published == self.attempted
    }
    fn reap(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut pending = Vec::new();
        let mut panicked = false;
        for worker in self.retired.drain(..) {
            if worker.is_finished() {
                panicked |= worker.join().is_err();
            } else {
                pending.push(worker);
            }
        }
        self.retired = pending;
        if panicked {
            Err("UI worker panicked".into())
        } else {
            Ok(())
        }
    }
    fn poll_sources(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(watcher) = &mut self.watcher else {
            return Ok(());
        };
        if let Some(changes) = watcher.try_result()? {
            self.scanning = false;
            if self
                .sources
                .as_ref()
                .is_some_and(|sources| !sources.affected(changes).is_empty())
            {
                self.attempted = None;
                self.deferred = None;
            }
        }
        if !self.scanning
            && self
                .last_scan
                .is_none_or(|last| last.elapsed() >= std::time::Duration::from_millis(200))
        {
            if let Some(sources) = &self.sources {
                watcher.request(sources, 32)?;
                self.scanning = true;
                self.last_scan = Some(std::time::Instant::now());
            }
        }
        Ok(())
    }
    pub(super) fn poll(
        &mut self,
        scene: &SceneGraph,
        project: &AuthoringProject,
        viewport: [f32; 2],
        graphics: &mut Graphics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        graphics.residency_cache.geometry_live = graphics.geometry_bytes();
        self.poll_gpu(
            scene,
            project,
            viewport,
            &graphics.renderer,
            graphics.host.device(),
            graphics.host.queue(),
            &mut graphics.residency_cache,
            &mut graphics.ui_draws,
            &mut graphics.ui_presented,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn poll_gpu(
        &mut self,
        scene: &SceneGraph,
        project: &AuthoringProject,
        viewport: [f32; 2],
        renderer: &voxy_render::SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &mut super::gpu_model::ResidencyCache,
        draws: &mut Vec<GpuDraw>,
        ui_presented: &mut bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.reap()?;
        self.poll_sources()?;
        if self.worker.is_none() && self.deferred.is_some_and(|state| state != cache.state()) {
            self.attempted = None;
            self.deferred = None;
        }

        let snapshot = voxy_gameplay::extract_scene_ui(scene, viewport, 128)?;
        if let Some(worker) = &mut self.worker {
            let Some(completion) = worker.try_result()? else {
                return Ok(());
            };
            self.retired
                .extend(self.worker.take().map(AssetImportWorker::close));
            if self.attempted.as_ref() == Some(&snapshot) {
                let inputs = match &completion.result {
                    Ok(prepared) => prepared.inputs(),
                    Err(failed) => &failed.inputs,
                };
                self.sources
                    .get_or_insert_with(|| voxy_assets::SourceDependencies::new(1, 32))
                    .record(
                        AssetId("game-ui-text".into()),
                        inputs,
                        voxy_assets::ImportOutcome::Failed,
                    )?;
                if self.watcher.is_none() {
                    self.watcher = Some(project.source_watcher(
                        inputs.observations(),
                        32,
                        16 * 1024 * 1024,
                    )?);
                }
                let staged = (|| {
                    let prepared = completion.result.map_err(|failed| failed.error)?;
                    let meshes = ui_draw::build(&snapshot, &prepared.value().runs)?;
                    let candidate = upload_on(renderer, device, queue, cache, &meshes)?;
                    self.sources
                        .as_mut()
                        .ok_or("missing UI source index")?
                        .record(
                            AssetId("game-ui-text".into()),
                            prepared.inputs(),
                            voxy_assets::ImportOutcome::Published,
                        )?;
                    Ok::<_, Box<dyn std::error::Error>>(candidate)
                })();
                let candidate = match staged {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        self.deferred = Some(cache.state());
                        return Err(error);
                    }
                };
                self.deferred = None;
                *draws = candidate;
                *ui_presented = false;
                self.published = Some(snapshot);
                println!("GAME UI GPU PUBLISHED draws={}", draws.len());
                return Ok(());
            }
        }
        if self.attempted.as_ref() != Some(&snapshot) {
            let mut worker =
                ui_text::UiTextPlan::from_snapshot(scene, snapshot.clone())?.worker(project)?;
            let mut catalog =
                AssetCatalog::<voxy_assets::ImportedAsset<ui_text::UiTextPreparation>>::new(1, 1)?;
            let ticket = catalog.request(AssetId("game-ui-text".into()))?;
            worker.submit(&ticket)?;
            self.attempted = Some(snapshot);
            self.worker = Some(worker);
        }
        Ok(())
    }
}
pub(super) fn upload_on(
    renderer: &voxy_render::SceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: &mut super::gpu_model::ResidencyCache,
    meshes: &[ui_draw::UiDrawMesh],
) -> Result<Vec<GpuDraw>, Box<dyn std::error::Error>> {
    let mut images = Vec::new();
    let mut indices = Vec::new();
    let mut runs = Vec::new();
    for mesh in meshes {
        let index = if let Some(run) = &mesh.atlas {
            if let Some(index) = runs.iter().position(|prior| Arc::ptr_eq(prior, run)) {
                Some(index)
            } else {
                let [width, height] = run.atlas().dimensions();
                let rgba = run
                    .atlas()
                    .alpha()
                    .iter()
                    .flat_map(|alpha| [255, 255, 255, *alpha])
                    .collect();
                images.push(ImageAsset::from_rgba(
                    u32::try_from(width)?,
                    u32::try_from(height)?,
                    rgba,
                    ImageLimits::default(),
                )?);
                runs.push(Arc::clone(run));
                Some(images.len() - 1)
            }
        } else {
            None
        };
        indices.push(index);
    }
    let additional: u64 = meshes
        .iter()
        .map(|mesh| voxy_render::SceneRenderer::mesh_allocation_bytes(&mesh.mesh))
        .sum();
    if additional > cache.geometry_budget.saturating_sub(cache.geometry_live) {
        return Err("GPU UI geometry peak budget exceeded".into());
    }
    cache.preflight_images(&images)?;
    let mut textures = Vec::new();
    for image in &images {
        textures.push(cache.get_or_upload_on(renderer, device, queue, image)?);
    }
    let mut candidate = Vec::new();
    for (mesh, index) in meshes.iter().zip(indices) {
        candidate.push(GpuDraw {
            owner: mesh.owner,
            geometry: renderer.upload_mesh(device, &mesh.mesh)?,
            texture: index.map(|index| Arc::clone(&textures[index])),
        });
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finished_panic_does_not_lose_unfinished_worker_ownership() {
        let (tx, rx) = std::sync::mpsc::channel();
        let pending = std::thread::spawn(move || {
            rx.recv().unwrap();
        });
        let failed = std::thread::spawn(|| panic!("UI worker failure"));
        while !failed.is_finished() {
            std::thread::yield_now();
        }
        let mut ui = UiLive {
            retired: vec![failed, pending],
            ..UiLive::default()
        };
        assert!(ui.reap().is_err());
        assert_eq!(ui.retired.len(), 1);
        let handles = ui.close();
        assert!(ui.retired.is_empty());
        tx.send(()).unwrap();
        for handle in handles {
            handle.join().unwrap();
        }
    }
}

#[cfg(test)]
mod presentation_tests {
    use super::*;
    #[test]
    fn upload_is_not_input_publication_until_native_frame_acknowledgement() {
        let first = SceneUiSnapshot {
            viewport: [100.0; 2],
            elements: Vec::new(),
        };
        let second = SceneUiSnapshot {
            viewport: [200.0; 2],
            elements: Vec::new(),
        };
        let mut ui = UiLive::default();
        ui.published = Some(first.clone());
        assert!(ui.presented().is_none());
        ui.frame_presented();
        assert_eq!(ui.presented(), Some(&first));
        ui.published = Some(second.clone());
        assert_eq!(ui.presented(), Some(&first));
        ui.frame_presented();
        assert_eq!(ui.presented(), Some(&second));
        assert!(ui.close().is_empty());
        assert!(ui.presented().is_none());
    }
}

#[cfg(test)]
mod source_reload_tests {
    use super::*;
    #[test]
    fn font_content_change_unavailability_and_recovery_invalidate_attempt_without_losing_visible_layout()
     {
        let root = std::env::temp_dir().join(format!(
            "voxy-ui-font-watch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("font.ttf"), b"first").unwrap();
        let mut inputs = voxy_assets::ImportInputs::new(1, 100);
        inputs
            .read(AssetId("font.ttf".into()), |_, _| Ok(b"first".to_vec()))
            .unwrap();
        let mut sources = voxy_assets::SourceDependencies::new(1, 32);
        sources
            .record(
                AssetId("game-ui-text".into()),
                &inputs,
                voxy_assets::ImportOutcome::Published,
            )
            .unwrap();
        let watcher = voxy_assets::SourcePollWorker::new_with_observations(
            voxy_assets::FileInputs::new(&root).unwrap(),
            32,
            100,
            inputs.observations(),
        )
        .unwrap();
        let layout = SceneUiSnapshot {
            viewport: [100.0; 2],
            elements: Vec::new(),
        };
        let mut ui = UiLive {
            sources: Some(sources),
            watcher: Some(watcher),
            attempted: Some(layout.clone()),
            published: Some(layout.clone()),
            presented: Some(layout.clone()),
            ..UiLive::default()
        };
        for replacement in [
            Some(b"changed".as_slice()),
            None,
            Some(b"restored".as_slice()),
        ] {
            if let Some(bytes) = replacement {
                std::fs::write(root.join("font.ttf"), bytes).unwrap();
            } else {
                std::fs::remove_file(root.join("font.ttf")).unwrap();
            }
            ui.attempted = Some(layout.clone());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while ui.attempted.is_some() {
                ui.last_scan = None;
                ui.poll_sources().unwrap();
                assert!(
                    std::time::Instant::now() < deadline,
                    "watcher failed to invalidate font change"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            assert_eq!(ui.published.as_ref(), Some(&layout));
            assert_eq!(ui.presented(), Some(&layout));
        }
        for handle in ui.close() {
            handle.join().unwrap();
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod gpu_reload_tests;
