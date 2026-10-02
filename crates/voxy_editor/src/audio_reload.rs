//! Common observed import/watch pipeline owned by one play session.
use super::{audio_play::AudioBudget, prefab_authoring::AuthoringProject};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
use voxy_assets::{
    AssetCatalog, AssetId, AssetImportWorker, ImportedAsset, SourceDependencies, SourcePollWorker,
};
use voxy_audio::Clip;
#[derive(Debug)]
pub(super) struct AudioReload {
    worker: AssetImportWorker<Clip>,
    watcher: SourcePollWorker,
    sources: SourceDependencies,
    outputs: BTreeSet<AssetId>,
    queued: BTreeSet<AssetId>,
    importing: bool,
    scanning: bool,
    last_scan: Option<Instant>,
}
impl AudioReload {
    pub(super) fn new(
        project: &AuthoringProject,
        worker: AssetImportWorker<Clip>,
        outputs: BTreeSet<AssetId>,
        catalog: &AssetCatalog<ImportedAsset<Clip>>,
    ) -> Result<Self, String> {
        let mut sources = SourceDependencies::new(128, 768);
        let mut observations = std::collections::BTreeMap::new();
        for id in &outputs {
            let asset = catalog
                .snapshot(id)
                .ok_or("missing initial audio publication")?;
            for (source, input) in asset.inputs().observations() {
                if observations
                    .get(source)
                    .is_some_and(|previous| previous != input)
                {
                    return Err("audio initial dependency revisions disagree".into());
                }
                observations.insert(source.clone(), input.clone());
            }
            sources
                .record(
                    id.clone(),
                    asset.inputs(),
                    voxy_assets::ImportOutcome::Published,
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(Self {
            worker,
            watcher: project.audio_watcher(&observations)?,
            sources,
            outputs,
            queued: BTreeSet::new(),
            importing: false,
            scanning: false,
            last_scan: None,
        })
    }
    pub(super) fn poll(
        &mut self,
        catalog: &mut AssetCatalog<ImportedAsset<Clip>>,
    ) -> Result<(), String> {
        // Known invalidations always precede accepting a candidate.
        if let Some(changes) = self.watcher.try_result().map_err(|e| e.to_string())? {
            self.scanning = false;
            let affected = self.sources.affected(changes);
            catalog.invalidate(&affected).map_err(|e| e.to_string())?;
            self.queued.extend(affected);
        }
        if self.importing
            && let Some(mut completion) = self.worker.try_result().map_err(|e| e.to_string())?
        {
            self.importing = false;
            if let Ok(candidate) = &completion.result
                && let Err(error) =
                    admit_replacement(&self.outputs, catalog, completion.ticket.asset(), candidate)
            {
                completion.result = completion
                    .result
                    .and_then(|candidate| Err(candidate.into_failed(error)));
            }
            if let Err(failure) = &completion.result {
                eprintln!(
                    "audio reload {}: {}",
                    completion.ticket.asset().0,
                    failure.error
                );
            }
            match catalog.complete_observed(
                &mut self.sources,
                &completion.ticket,
                completion.result,
            ) {
                Ok(())
                | Err(voxy_assets::PublicationError::Asset(voxy_assets::AssetError::StaleTicket)) =>
                    {}
                Err(error) => return Err(error.to_string()),
            }
        }
        if !self.importing
            && let Some(id) = self.queued.pop_first()
        {
            let ticket = catalog.request(id).map_err(|e| e.to_string())?;
            self.worker.submit(&ticket).map_err(|e| e.to_string())?;
            self.importing = true;
        }
        if !self.scanning
            && self
                .last_scan
                .is_none_or(|last| last.elapsed() >= Duration::from_millis(200))
        {
            self.watcher
                .request(&self.sources, 768)
                .map_err(|e| e.to_string())?;
            self.scanning = true;
            self.last_scan = Some(Instant::now());
        }
        Ok(())
    }
    pub(super) fn close(self) -> Vec<std::thread::JoinHandle<()>> {
        vec![self.worker.close(), self.watcher.close()]
    }
}
fn admit_replacement(
    outputs: &BTreeSet<AssetId>,
    catalog: &AssetCatalog<ImportedAsset<Clip>>,
    replacing: &AssetId,
    candidate: &ImportedAsset<Clip>,
) -> Result<(), String> {
    let mut resident = AudioBudget::default();
    let mut old_frames = 0_usize;
    for id in outputs {
        let asset = catalog
            .snapshot(id)
            .ok_or("missing retained audio publication")?;
        old_frames = old_frames
            .checked_add(asset.value().frame_count())
            .ok_or("audio peak budget overflow")?;
        if id != replacing {
            resident.admit(input_bytes(&asset)?, asset.value().frame_count())?;
        }
    }
    let peak = old_frames
        .checked_add(candidate.value().frame_count())
        .ok_or("audio peak budget overflow")?;
    if peak > 960_000 {
        return Err("audio replacement peak PCM budget exceeded".into());
    }
    resident.admit(input_bytes(candidate)?, candidate.value().frame_count())
}

fn input_bytes(asset: &ImportedAsset<Clip>) -> Result<usize, String> {
    asset
        .inputs()
        .observations()
        .values()
        .try_fold(0_usize, |total, input| {
            let input = input.as_ref().map_err(|e| format!("audio input: {e:?}"))?;
            total
                .checked_add(input.bytes.len())
                .ok_or_else(|| "audio input budget overflow".into())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn imported(frames: usize) -> ImportedAsset<Clip> {
        let mut inputs = voxy_assets::ImportInputs::new(1, 1);
        inputs
            .read(AssetId("source.wav".into()), |_, _| Ok(vec![1]))
            .unwrap();
        inputs
            .finish(
                Clip::new(48000, vec![[0.25; 2]; frames]).unwrap(),
                |_, _| Ok(vec![1]),
            )
            .unwrap()
    }
    #[test]
    fn replacement_budget_rejects_candidate_and_retains_old_publication() {
        let a = AssetId("a".into());
        let b = AssetId("b".into());
        let outputs = BTreeSet::from([a.clone(), b.clone()]);
        let mut catalog = AssetCatalog::new(2, 1).unwrap();
        for id in &outputs {
            let ticket = catalog.request(id.clone()).unwrap();
            catalog.complete(&ticket, Ok(imported(240_000))).unwrap();
        }
        let retained = catalog.snapshot_with_revision(&a).unwrap();
        assert!(admit_replacement(&outputs, &catalog, &a, &imported(240_000)).is_ok());
        let candidate = imported(240_001);
        let error = admit_replacement(&outputs, &catalog, &a, &candidate).unwrap_err();
        let ticket = catalog.request(a.clone()).unwrap();
        catalog.complete(&ticket, Err(error)).unwrap();
        let after = catalog.snapshot_with_revision(&a).unwrap();
        assert_eq!(after.0, retained.0);
        assert!(std::sync::Arc::ptr_eq(&after.1, &retained.1));
    }
}
