//! A single bounded file-poll worker; the owner decides when to submit scans.
use crate::{
    AssetId, DependencyError, FileInputs, SourceDependencies, SourcePoller, SourcePollerCheckpoint,
};
use std::{
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread::JoinHandle,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PollWorkerError {
    Busy,
    Capacity,
    Disconnected,
}
impl std::fmt::Display for PollWorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "source poll worker error: {self:?}")
    }
}
impl std::error::Error for PollWorkerError {}
#[derive(Debug)]
struct Scan {
    sources: Vec<AssetId>,
    max_checks: usize,
    observation: Option<(AssetId, usize)>,
}
/// Completed bounded scan, optionally retaining one selected source's bytes from
/// the same read used for its fingerprint. The owner decides how to decode them.
#[derive(Debug)]
pub struct ObservedSourceScan {
    pub changes: Vec<AssetId>,
    pub observation: Option<(AssetId, Result<Vec<u8>, String>)>,
}
#[derive(Debug)]
enum Request {
    Scan(Scan),
    Checkpoint,
}
/// Exactly one submitted scan may remain unconsumed. File IO and hashing occur
/// on the worker; owner calls only copy bounded IDs and use nonblocking channels.
/// Dropping closes channels and detaches the thread, which exits after current IO.
/// Use `close` to obtain a join handle for shutdown outside frame-critical code.
#[derive(Debug)]
pub struct SourcePollWorker {
    requests: SyncSender<Request>,
    results: Receiver<Result<ObservedSourceScan, DependencyError>>,
    thread: JoinHandle<()>,
    checkpoint_results: Receiver<SourcePollerCheckpoint>,
    checkpoint_pending: bool,
    pending: bool,
    max_sources: usize,
    max_file_bytes: usize,
}
impl SourcePollWorker {
    /// Maximum distinct source IDs admitted by this worker's immutable policy.
    #[must_use]
    pub const fn source_capacity(&self) -> usize {
        self.max_sources
    }
    /// Maximum bytes read per source, retaining the saved policy after restore.
    #[must_use]
    pub const fn source_byte_capacity(&self) -> usize {
        self.max_file_bytes
    }

    /// # Errors
    /// Returns OS thread creation errors.
    pub fn new(
        provider: FileInputs,
        max_sources: usize,
        max_file_bytes: usize,
    ) -> std::io::Result<Self> {
        Self::spawn(
            provider,
            SourcePoller::new(max_sources, max_file_bytes),
            max_sources,
        )
    }
    /// Starts from successful/failed observed input fingerprints rather than an
    /// unknown baseline. IO remains on the worker and later new inputs invalidate.
    /// # Errors
    /// Rejects observed input bounds and returns OS thread creation errors.
    pub fn new_with_observations(
        provider: FileInputs,
        max_sources: usize,
        max_file_bytes: usize,
        observations: &std::collections::BTreeMap<
            AssetId,
            Result<crate::InputSnapshot, crate::InputError>,
        >,
    ) -> std::io::Result<Self> {
        let poller = SourcePoller::with_observations(max_sources, max_file_bytes, observations)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
        Self::spawn(provider, poller, max_sources)
    }
    fn spawn(
        provider: FileInputs,
        mut poller: SourcePoller,
        max_sources: usize,
    ) -> std::io::Result<Self> {
        let max_file_bytes = poller.source_byte_capacity();
        let (requests, work) = mpsc::sync_channel::<Request>(1);
        let (output, results) = mpsc::sync_channel(1);
        let (checkpoint_output, checkpoint_results) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("voxy-source-poll".into())
            .spawn(move || {
                while let Ok(request) = work.recv() {
                    match request {
                        Request::Scan(scan) => {
                            let result = poller.reconcile_ids(scan.sources).map(|()| {
                                let mut observation = None;
                                let changes =
                                    poller.poll_fingerprints(scan.max_checks, |id, limit| {
                                        if let Some((selected, selected_limit)) = &scan.observation
                                            && selected == id
                                        {
                                            let bytes =
                                                provider.read(id, limit.min(*selected_limit));
                                            let digest = bytes
                                                .as_ref()
                                                .map(|b| *blake3::hash(b).as_bytes())
                                                .map_err(Clone::clone);
                                            observation = Some((id.clone(), bytes));
                                            digest
                                        } else {
                                            provider.fingerprint(id, limit)
                                        }
                                    });
                                ObservedSourceScan {
                                    changes,
                                    observation,
                                }
                            });
                            if output.send(result).is_err() {
                                break;
                            }
                        }
                        Request::Checkpoint => {
                            if checkpoint_output.send(poller.checkpoint()).is_err() {
                                break;
                            }
                        }
                    }
                }
            })?;
        Ok(Self {
            requests,
            results,
            thread,
            pending: false,
            checkpoint_pending: false,
            checkpoint_results,
            max_sources,
            max_file_bytes,
        })
    }
    /// Submits a snapshot of source IDs. Apply returned changes against the current
    /// owner index; mappings may change while IO is running. Newly observed sources
    /// invalidate once. Polling cadence and import dispatch remain owner choices.
    /// # Errors
    /// Rejects an outstanding scan, source capacity overflow or a stopped worker.
    pub fn request(
        &mut self,
        index: &SourceDependencies,
        max_checks: usize,
    ) -> Result<(), PollWorkerError> {
        self.request_scan(index.source_ids(), max_checks, None)
    }
    /// Also watches a source absent from the dependency index, retaining its
    /// bounded bytes if round-robin polling selects it. No file IO occurs here.
    /// # Errors
    /// Rejects busy/stopped workers, invalid IDs and combined source capacity.
    pub fn request_with_observation(
        &mut self,
        index: &SourceDependencies,
        source: AssetId,
        max_bytes: usize,
        max_checks: usize,
    ) -> Result<(), PollWorkerError> {
        if source.0.is_empty() {
            return Err(PollWorkerError::Capacity);
        }
        let mut sources = index.source_ids();
        if let Err(position) = sources.binary_search(&source) {
            sources.insert(position, source.clone());
        }
        self.request_scan(sources, max_checks, Some((source, max_bytes)))
    }
    fn request_scan(
        &mut self,
        sources: Vec<AssetId>,
        max_checks: usize,
        observation: Option<(AssetId, usize)>,
    ) -> Result<(), PollWorkerError> {
        if self.pending || self.checkpoint_pending {
            return Err(PollWorkerError::Busy);
        }
        if sources.len() > self.max_sources {
            return Err(PollWorkerError::Capacity);
        }
        self.requests
            .try_send(Request::Scan(Scan {
                sources,
                max_checks,
                observation,
            }))
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => PollWorkerError::Busy,
                mpsc::TrySendError::Disconnected(_) => PollWorkerError::Disconnected,
            })?;
        self.pending = true;
        Ok(())
    }
    /// Retrieves a finished scan without waiting or performing filesystem IO.
    /// # Errors
    /// Reports worker disconnection or defensive worker-side capacity rejection.
    pub fn try_result(&mut self) -> Result<Option<Vec<AssetId>>, PollWorkerError> {
        self.try_observed_result()
            .map(|result| result.map(|scan| scan.changes))
    }
    /// Retrieves the selected bytes and changes from one completed background scan.
    /// # Errors
    /// Reports worker disconnection or worker-side capacity rejection.
    pub fn try_observed_result(&mut self) -> Result<Option<ObservedSourceScan>, PollWorkerError> {
        match self.results.try_recv() {
            Ok(result) => {
                self.pending = false;
                result.map(Some).map_err(|_| PollWorkerError::Capacity)
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.pending = false;
                Err(PollWorkerError::Disconnected)
            }
        }
    }
    /// Queue an ordered snapshot after any submitted scan, without doing file IO.
    /// New scans are fenced until the snapshot is consumed.
    /// # Errors
    /// Reports a full command queue, outstanding snapshot or worker disconnection.
    pub fn request_checkpoint(&mut self) -> Result<(), PollWorkerError> {
        if self.checkpoint_pending {
            return Err(PollWorkerError::Busy);
        }
        self.requests
            .try_send(Request::Checkpoint)
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => PollWorkerError::Busy,
                mpsc::TrySendError::Disconnected(_) => PollWorkerError::Disconnected,
            })?;
        self.checkpoint_pending = true;
        Ok(())
    }
    /// # Errors
    /// Reports worker disconnection. Scan results remain independently available.
    pub fn try_checkpoint(&mut self) -> Result<Option<SourcePollerCheckpoint>, PollWorkerError> {
        match self.checkpoint_results.try_recv() {
            Ok(saved) => {
                self.checkpoint_pending = false;
                Ok(Some(saved))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.checkpoint_pending = false;
                Err(PollWorkerError::Disconnected)
            }
        }
    }
    /// Creates a fresh worker from a validated baseline, without resetting its cursor.
    /// # Errors
    /// Rejects invalid checkpoint/policy and thread creation failure.
    pub fn new_with_checkpoint(
        provider: FileInputs,
        saved: &SourcePollerCheckpoint,
        max_sources: usize,
        max_file_bytes: usize,
    ) -> std::io::Result<Self> {
        let poller = saved
            .prepare(max_sources, max_file_bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
        Self::spawn(provider, poller, saved.max_sources)
    }
    /// Closes both queues without blocking. Joining waits for any active file IO.
    #[must_use]
    pub fn close(self) -> JoinHandle<()> {
        let Self {
            requests,
            results,
            thread,
            checkpoint_results,
            ..
        } = self;
        drop(requests);
        drop(results);
        drop(checkpoint_results);
        thread
    }
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    use crate::{ImportInputs, ImportOutcome};
    use std::time::{Duration, Instant};
    fn await_value<T>(mut get: impl FnMut() -> Result<Option<T>, PollWorkerError>) -> T {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(value) = get().unwrap() {
                return value;
            }
            assert!(Instant::now() < until, "worker observation timed out");
            std::thread::yield_now();
        }
    }
    #[test]
    fn queued_snapshot_fences_scans_keeps_results_and_restores_worker_baseline() {
        let path = std::env::temp_dir().join(format!(
            "voxy-watch-save-{}-{}",
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
        std::fs::write(path.join("a"), [1]).unwrap();
        let mut inputs = ImportInputs::new(2, 4);
        inputs
            .read(AssetId("a".into()), |_, _| Ok(vec![1]))
            .unwrap();
        let _ = inputs.read(AssetId("b".into()), |_, _| Err("missing".into()));
        let mut index = SourceDependencies::new(1, 2);
        index
            .record(AssetId("output".into()), &inputs, ImportOutcome::Failed)
            .unwrap();
        let mut original = SourcePollWorker::new(FileInputs::new(&path).unwrap(), 2, 4).unwrap();
        original.request(&index, 1).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match original.request_checkpoint() {
                Ok(()) => break,
                Err(PollWorkerError::Busy) => {
                    assert!(Instant::now() < until);
                    std::thread::yield_now();
                }
                Err(e) => panic!("snapshot request: {e}"),
            }
        }
        assert_eq!(original.request(&index, 1), Err(PollWorkerError::Busy));
        assert_eq!(original.request_checkpoint(), Err(PollWorkerError::Busy));
        let saved = await_value(|| original.try_checkpoint());
        assert_eq!(saved.cursor, 1);
        assert_eq!(
            await_value(|| original.try_result()),
            vec![AssetId("a".into())]
        );
        let mut restored =
            SourcePollWorker::new_with_checkpoint(FileInputs::new(&path).unwrap(), &saved, 2, 4)
                .unwrap();
        for tick in 0_u8..16 {
            if tick == 5 {
                std::fs::write(path.join("b"), [7]).unwrap();
            }
            std::fs::write(path.join("a"), [tick]).unwrap();
            original.request(&index, 1).unwrap();
            restored.request(&index, 1).unwrap();
            assert_eq!(
                await_value(|| original.try_result()),
                await_value(|| restored.try_result())
            );
        }
        original.request_checkpoint().unwrap();
        restored.request_checkpoint().unwrap();
        assert_eq!(
            await_value(|| original.try_checkpoint()),
            await_value(|| restored.try_checkpoint())
        );
        original.close().join().unwrap();
        restored.close().join().unwrap();
    }
    struct ObservationFixture(std::path::PathBuf);
    impl ObservationFixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "voxy-observed-source-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            std::fs::write(root.join("a"), b"a").unwrap();
            std::fs::write(root.join("manifest"), b"old").unwrap();
            Self(root)
        }
        fn provider(&self) -> FileInputs {
            FileInputs::new(&self.0).unwrap()
        }
        fn index(&self) -> SourceDependencies {
            let mut inputs = ImportInputs::new(1, 8);
            inputs
                .read(AssetId("a".into()), |id, limit| {
                    self.provider().read(id, limit)
                })
                .unwrap();
            let mut index = SourceDependencies::new(1, 4);
            index
                .record(AssetId("model".into()), &inputs, ImportOutcome::Published)
                .unwrap();
            index
        }
    }
    impl Drop for ObservationFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn queue_checkpoint(worker: &mut SourcePollWorker) {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match worker.request_checkpoint() {
                Ok(()) => break,
                Err(PollWorkerError::Busy) => {
                    assert!(Instant::now() < until);
                    std::thread::yield_now();
                }
                Err(error) => panic!("checkpoint failed: {error}"),
            }
        }
    }
    fn close_observer(worker: SourcePollWorker) {
        let thread = worker.close();
        let until = Instant::now() + Duration::from_secs(5);
        while !thread.is_finished() {
            assert!(Instant::now() < until, "closed observer did not exit");
            std::thread::yield_now();
        }
        thread.join().unwrap();
    }
    #[test]
    fn selected_source_bytes_match_checkpoint_digest_and_restore_round_robin() {
        let fixture = ObservationFixture::new();
        let index = fixture.index();
        let selected = AssetId("manifest".into());
        let mut worker = SourcePollWorker::new(fixture.provider(), 2, 8).unwrap();
        worker
            .request_with_observation(&index, selected.clone(), 8, 1)
            .unwrap();
        let first = await_value(|| worker.try_observed_result());
        assert_eq!(first.changes, vec![AssetId("a".into())]);
        assert!(first.observation.is_none());
        worker
            .request_with_observation(&index, selected.clone(), 8, 1)
            .unwrap();
        let second = await_value(|| worker.try_observed_result());
        assert_eq!(second.changes, vec![selected.clone()]);
        let (id, original) = second.observation.unwrap();
        let original = original.unwrap();
        assert_eq!(id, selected);
        assert_eq!(original, b"old");
        worker.request_checkpoint().unwrap();
        let saved = await_value(|| worker.try_checkpoint());
        assert_eq!(saved.cursor, 0);
        assert_eq!(
            saved
                .sources
                .iter()
                .find(|(id, _)| id == &selected)
                .unwrap()
                .1
                .as_ref(),
            Some(&crate::SourceFingerprint::Content(
                *blake3::hash(&original).as_bytes()
            ))
        );
        assert!(SourcePollWorker::new_with_checkpoint(fixture.provider(), &saved, 1, 8).is_err());
        let mut restored =
            SourcePollWorker::new_with_checkpoint(fixture.provider(), &saved, 2, 8).unwrap();
        std::fs::write(fixture.0.join("manifest"), b"new").unwrap();
        for observer in [&mut worker, &mut restored] {
            observer
                .request_with_observation(&index, selected.clone(), 8, 2)
                .unwrap();
            let next = await_value(|| observer.try_observed_result());
            assert_eq!(next.changes, vec![selected.clone()]);
            assert_eq!(next.observation.unwrap().1.unwrap(), b"new");
        }
        assert_eq!(original, b"old");
        worker.request_checkpoint().unwrap();
        restored.request_checkpoint().unwrap();
        assert_eq!(
            await_value(|| worker.try_checkpoint()),
            await_value(|| restored.try_checkpoint())
        );
        close_observer(worker);
        close_observer(restored);
    }
    #[test]
    fn selected_source_deduplicates_capacity_fences_snapshot_and_closes_pending_channels() {
        let fixture = ObservationFixture::new();
        let index = fixture.index();
        let mut worker = SourcePollWorker::new(fixture.provider(), 1, 8).unwrap();
        assert_eq!(
            worker.request_with_observation(&index, AssetId("manifest".into()), 8, 1),
            Err(PollWorkerError::Capacity)
        );
        assert_eq!(
            worker.request_with_observation(&index, AssetId(String::new()), 8, 1),
            Err(PollWorkerError::Capacity)
        );
        worker
            .request_with_observation(&index, AssetId("a".into()), 8, 1)
            .unwrap();
        assert_eq!(worker.request(&index, 1), Err(PollWorkerError::Busy));
        queue_checkpoint(&mut worker);
        assert_eq!(
            worker.request_with_observation(&index, AssetId("a".into()), 8, 1),
            Err(PollWorkerError::Busy)
        );
        let scan = await_value(|| worker.try_observed_result());
        assert_eq!(scan.observation.unwrap().1.unwrap(), b"a");
        assert_eq!(worker.request(&index, 1), Err(PollWorkerError::Busy));
        let saved = await_value(|| worker.try_checkpoint());
        assert_eq!(saved.sources.len(), 1);
        worker
            .request_with_observation(&index, AssetId("a".into()), 8, 1)
            .unwrap();
        let unchanged = await_value(|| worker.try_observed_result());
        assert!(unchanged.changes.is_empty());
        assert_eq!(unchanged.observation.unwrap().1.unwrap(), b"a");
        worker
            .request_with_observation(&index, AssetId("a".into()), 8, 1)
            .unwrap();
        queue_checkpoint(&mut worker);
        close_observer(worker);
        assert_eq!(std::fs::read(fixture.0.join("a")).unwrap(), b"a");
    }
    #[test]
    fn selected_source_enforces_its_byte_cap_and_reports_failure_recovery_without_imports() {
        let fixture = ObservationFixture::new();
        let index = SourceDependencies::new(1, 4);
        let selected = AssetId("manifest".into());
        let mut worker = SourcePollWorker::new(fixture.provider(), 1, 8).unwrap();
        worker
            .request_with_observation(&index, selected.clone(), 3, 0)
            .unwrap();
        let skipped = await_value(|| worker.try_observed_result());
        assert!(skipped.changes.is_empty() && skipped.observation.is_none());
        std::fs::write(fixture.0.join("manifest"), b"four").unwrap();
        worker
            .request_with_observation(&index, selected.clone(), 3, 1)
            .unwrap();
        let oversized = await_value(|| worker.try_observed_result());
        assert_eq!(oversized.changes, vec![selected.clone()]);
        assert_eq!(
            oversized.observation.unwrap().1.unwrap_err(),
            "input byte limit exceeded"
        );
        worker.request_checkpoint().unwrap();
        let failed = await_value(|| worker.try_checkpoint());
        assert_eq!(
            failed.sources[0].1,
            Some(crate::SourceFingerprint::Unavailable)
        );
        std::fs::write(fixture.0.join("manifest"), b"more").unwrap();
        worker
            .request_with_observation(&index, selected.clone(), 3, 1)
            .unwrap();
        let repeated = await_value(|| worker.try_observed_result());
        assert!(repeated.changes.is_empty() && repeated.observation.unwrap().1.is_err());
        for bytes in [Some(b"ok".as_slice()), None, Some(b"ok".as_slice())] {
            if let Some(bytes) = bytes {
                std::fs::write(fixture.0.join("manifest"), bytes).unwrap();
            } else {
                std::fs::remove_file(fixture.0.join("manifest")).unwrap();
            }
            worker
                .request_with_observation(&index, selected.clone(), 3, 1)
                .unwrap();
            let scan = await_value(|| worker.try_observed_result());
            assert_eq!(scan.changes, vec![selected.clone()]);
            let observed = scan.observation.unwrap().1;
            if let Some(bytes) = bytes {
                assert_eq!(observed.unwrap(), bytes);
            } else {
                assert!(observed.is_err());
            }
        }
        close_observer(worker);
    }
    #[test]
    fn restored_worker_frontend_keeps_saved_capacity_instead_of_caller_upper_bound() {
        let fixture = ObservationFixture::new();
        let index = fixture.index();
        let mut original = SourcePollWorker::new(fixture.provider(), 1, 8).unwrap();
        original.request(&index, 1).unwrap();
        await_value(|| original.try_result());
        original.request_checkpoint().unwrap();
        let saved = await_value(|| original.try_checkpoint());
        close_observer(original);
        let restored =
            SourcePollWorker::new_with_checkpoint(fixture.provider(), &saved, 4, 64).unwrap();
        let capacity = restored.source_capacity();
        let bytes = restored.source_byte_capacity();
        close_observer(restored);
        assert_eq!(bytes, saved.max_file_bytes);
        assert_eq!(
            capacity, saved.max_sources,
            "restored frontend relaxed saved worker quota"
        );
    }
}
