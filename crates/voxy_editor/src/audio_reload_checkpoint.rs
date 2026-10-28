//! Nonblocking reload save barrier; no new IO is dispatched until resume.
use super::*;
use voxy_assets::{PollWorkerError, SourceDependenciesCheckpoint, SourcePollerCheckpoint};
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AudioReloadCheckpoint {
    pub(super) version: u32,
    #[serde(with = "wire::sources")]
    pub(super) sources: SourceDependenciesCheckpoint,
    #[serde(with = "wire::ids")]
    pub(super) outputs: Vec<AssetId>,
    #[serde(with = "wire::ids")]
    pub(super) queued: Vec<AssetId>,
    #[serde(with = "wire::watcher")]
    pub(super) watcher: SourcePollerCheckpoint,
    pub(super) delay: Duration,
    #[serde(with = "wire::recipes")]
    pub(crate) recipes: std::collections::BTreeMap<AssetId, Option<String>>,
}
#[derive(Debug)]
pub(super) enum Barrier {
    Draining,
    Requested,
    Ready(Box<AudioReloadCheckpoint>),
}
#[allow(dead_code)]
impl AudioReload {
    pub(crate) fn begin_checkpoint(&mut self) -> Result<(), EditorError> {
        if self.checkpoint.is_some() {
            return Err("audio reload checkpoint already requested".into());
        }
        self.checkpoint = Some(Barrier::Draining);
        Ok(())
    }
    /// Drain accepted completions in normal invalidation-before-publication order,
    /// then request the watcher snapshot. All calls remain nonblocking.
    pub(crate) fn poll_checkpoint(
        &mut self,
        catalog: &mut AssetCatalog<ImportedAsset<Clip>>,
    ) -> Result<Option<AudioReloadCheckpoint>, EditorError> {
        if self.checkpoint.is_none() {
            return Err("audio reload checkpoint not requested".into());
        }
        self.poll(catalog)?;
        if let Some(Barrier::Ready(saved)) = &self.checkpoint {
            return Ok(Some((**saved).clone()));
        }
        if self.importing || self.scanning || catalog.pending() != 0 {
            return Ok(None);
        }
        if matches!(self.checkpoint, Some(Barrier::Draining)) {
            match self.watcher.request_checkpoint() {
                Ok(()) => self.checkpoint = Some(Barrier::Requested),
                Err(PollWorkerError::Busy) => return Ok(None),
                Err(error) => return Err(error.into()),
            }
        }
        let Some(watcher) = self.watcher.try_checkpoint()? else {
            return Ok(None);
        };
        let delay = self.last_scan.map_or(Duration::ZERO, |last| {
            Duration::from_millis(200).saturating_sub(last.elapsed())
        });
        let saved = AudioReloadCheckpoint {
            version: 1,
            sources: self.sources.checkpoint(),
            outputs: self.outputs.iter().cloned().collect(),
            queued: self.queued.iter().cloned().collect(),
            watcher,
            delay,
            recipes: self.recipes.clone(),
        };
        self.checkpoint = Some(Barrier::Ready(Box::new(saved.clone())));
        Ok(Some(saved))
    }
    /// Cancels/releases a save fence without abandoning a watcher request.
    /// Returns false while its already-issued snapshot is still outstanding.
    pub(crate) fn resume_after_checkpoint(&mut self) -> Result<bool, EditorError> {
        if matches!(self.checkpoint, Some(Barrier::Requested))
            && self.watcher.try_checkpoint()?.is_none()
        {
            return Ok(false);
        }
        if let Some(Barrier::Ready(saved)) = &self.checkpoint {
            let elapsed = Duration::from_millis(200).saturating_sub(saved.delay);
            self.last_scan = Instant::now().checked_sub(elapsed);
        }
        self.checkpoint = None;
        Ok(true)
    }
}
#[allow(dead_code)]
impl AudioReloadCheckpoint {
    /// Validates before starting workers. Factories must reproduce the saved project
    /// import recipes and create the watcher from the provided exact poll baseline.
    pub(crate) fn prepare(
        &self,
        catalog: &AssetCatalog<ImportedAsset<Clip>>,
        make_worker: impl FnOnce() -> Result<AssetImportWorker<Clip>, EditorError>,
        make_watcher: impl FnOnce(&SourcePollerCheckpoint) -> Result<SourcePollWorker, EditorError>,
    ) -> Result<AudioReload, EditorError> {
        if self.version != 1
            || self.delay > Duration::from_millis(200)
            || catalog.pending() != 0
            || self.outputs.len() > 128
            || self.queued.len() > 128
        {
            return Err("invalid audio reload checkpoint header".into());
        }
        let collect = |ids: &[AssetId]| -> Result<BTreeSet<AssetId>, EditorError> {
            let mut set = BTreeSet::new();
            for id in ids {
                if id.0.is_empty() || !set.insert(id.clone()) {
                    return Err("invalid audio reload identity".into());
                }
            }
            Ok(set)
        };
        let outputs = collect(&self.outputs)?;
        let queued = collect(&self.queued)?;
        if self.recipes.keys().cloned().collect::<BTreeSet<_>>() != outputs
            || self
                .recipes
                .values()
                .flatten()
                .any(|id| id.is_empty() || id.len() > 1024 || id.chars().any(char::is_control))
            || !queued.is_subset(&outputs)
            || outputs.iter().any(|id| catalog.snapshot(id).is_none())
            || self
                .sources
                .outputs
                .iter()
                .any(|entry| !outputs.contains(&entry.output))
        {
            return Err("incompatible audio reload checkpoint outputs".into());
        }
        let sources = self.sources.prepare(128, 768)?;
        self.watcher.prepare(768, 8 * 1024 * 1024)?;
        let watcher = make_watcher(&self.watcher)?;
        let worker = match make_worker() {
            Ok(worker) => worker,
            Err(error) => {
                drop(watcher.close());
                return Err(error);
            }
        };
        let elapsed = Duration::from_millis(200).saturating_sub(self.delay);
        Ok(AudioReload {
            worker,
            watcher,
            sources,
            outputs,
            recipes: self.recipes.clone(),
            queued,
            importing: false,
            scanning: false,
            last_scan: Instant::now().checked_sub(elapsed),
            checkpoint: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::Path, sync::Arc};
    use voxy_assets::{FileInputs, ImportInputs, ImportOutcome};
    fn worker(path: &Path) -> Result<AssetImportWorker<Clip>, EditorError> {
        Ok(AssetImportWorker::new(
            FileInputs::new(path)?,
            1,
            4,
            |_, provider, inputs| {
                let captured = inputs
                    .read(AssetId("source.wav".into()), |id, limit| {
                        provider.read(id, limit)
                    })
                    .map_err(|e| format!("{e:?}"))?;
                Clip::new(48000, vec![[f32::from(captured.bytes[0]) / 255.; 2]; 4])
                    .map_err(|e| e.to_string())
            },
        )?)
    }
    #[test]
    fn save_barrier_drains_scan_retains_queue_and_restored_reload_publishes() {
        let path = std::env::temp_dir().join(format!(
            "voxy-audio-barrier-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(path.clone());
        std::fs::write(path.join("source.wav"), [1]).unwrap();
        let mut inputs = ImportInputs::new(1, 4);
        inputs
            .read(AssetId("source.wav".into()), |_, _| Ok(vec![1]))
            .unwrap();
        let mut sources = SourceDependencies::new(128, 768);
        let id = AssetId("tone".into());
        sources
            .record(id.clone(), &inputs, ImportOutcome::Published)
            .unwrap();
        let watcher = SourcePollWorker::new_with_observations(
            FileInputs::new(&path).unwrap(),
            768,
            8 * 1024 * 1024,
            inputs.observations(),
        )
        .unwrap();
        let imported = inputs
            .finish(Clip::new(48000, vec![[0.1; 2]; 4]).unwrap(), |_, _| {
                Ok(vec![1])
            })
            .unwrap();
        let mut catalog = AssetCatalog::new(128, 128).unwrap();
        let ticket = catalog.request(id.clone()).unwrap();
        catalog.complete(&ticket, Ok(imported)).unwrap();
        let mut original = AudioReload {
            worker: worker(&path).unwrap(),
            watcher,
            sources,
            outputs: BTreeSet::from([id.clone()]),
            recipes: std::collections::BTreeMap::from([(id.clone(), None)]),
            queued: BTreeSet::new(),
            importing: false,
            scanning: false,
            last_scan: None,
            checkpoint: None,
        };
        std::fs::write(path.join("source.wav"), [2]).unwrap();
        original.watcher.request(&original.sources, 768).unwrap();
        original.scanning = true;
        original.last_scan = Some(Instant::now());
        original.begin_checkpoint().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let saved = loop {
            if let Some(saved) = original.poll_checkpoint(&mut catalog).unwrap() {
                break saved;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        let json = serde_json::to_string(&saved).unwrap();
        let decoded: AudioReloadCheckpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        let saved = decoded;
        assert_eq!(saved.queued, vec![id.clone()]);
        assert!(!original.importing && !original.scanning && catalog.pending() == 0);
        for _ in 0..8 {
            original.poll(&mut catalog).unwrap();
        }
        assert_eq!(catalog.pending(), 0);
        let retained = catalog.snapshot(&id).unwrap();
        let catalog_record = catalog.checkpoint(|_| Some("old".into())).unwrap();
        let mut loaded_catalog = catalog_record
            .prepare(|_| Some(Arc::clone(&retained)), 128, 128)
            .unwrap();
        let mut loaded = saved
            .prepare(
                &loaded_catalog,
                || worker(&path),
                |state| {
                    Ok(SourcePollWorker::new_with_checkpoint(
                        FileInputs::new(&path)?,
                        state,
                        768,
                        8 * 1024 * 1024,
                    )?)
                },
            )
            .unwrap();
        assert_eq!(loaded.queued, original.queued);
        assert_eq!(loaded.sources.checkpoint(), original.sources.checkpoint());
        assert!(original.resume_after_checkpoint().unwrap());
        original.poll(&mut catalog).unwrap();
        assert!(original.importing);
        original.begin_checkpoint().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let completed = loop {
            if let Some(saved) = original.poll_checkpoint(&mut catalog).unwrap() {
                break saved;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        assert!(completed.queued.is_empty());
        assert_eq!(catalog.pending(), 0);
        assert!(original.resume_after_checkpoint().unwrap());
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            original.poll(&mut catalog).unwrap();
            loaded.poll(&mut loaded_catalog).unwrap();
            if matches!(catalog.status(&id), Some(voxy_assets::AssetStatus::Ready))
                && matches!(
                    loaded_catalog.status(&id),
                    Some(voxy_assets::AssetStatus::Ready)
                )
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let a = catalog.snapshot_with_revision(&id).unwrap();
        let b = loaded_catalog.snapshot_with_revision(&id).unwrap();
        assert_eq!(a.0, b.0);
        assert_eq!(
            a.1.value().to_pcm_bytes().unwrap(),
            b.1.value().to_pcm_bytes().unwrap()
        );
        assert_eq!(a.1.inputs().observations(), b.1.inputs().observations());
        for join in original.close().into_iter().chain(loaded.close()) {
            join.join().unwrap();
        }
    }
}

use crate::asset_checkpoint_wire as wire;
