//! A single bounded file-poll worker; the owner decides when to submit scans.
use crate::{AssetId, DependencyError, FileInputs, SourceDependencies, SourcePoller};
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
}
/// Exactly one submitted scan may remain unconsumed. File IO and hashing occur
/// on the worker; owner calls only copy bounded IDs and use nonblocking channels.
/// Dropping closes channels and detaches the thread, which exits after current IO.
/// Use `close` to obtain a join handle for shutdown outside frame-critical code.
#[derive(Debug)]
pub struct SourcePollWorker {
    requests: SyncSender<Scan>,
    results: Receiver<Result<Vec<AssetId>, DependencyError>>,
    thread: JoinHandle<()>,
    pending: bool,
    max_sources: usize,
}
impl SourcePollWorker {
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
        let (requests, work) = mpsc::sync_channel::<Scan>(1);
        let (output, results) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("voxy-source-poll".into())
            .spawn(move || {
                while let Ok(scan) = work.recv() {
                    let result = poller.reconcile_ids(scan.sources).map(|()| {
                        poller.poll(scan.max_checks, |id, limit| provider.read(id, limit))
                    });
                    if output.send(result).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            requests,
            results,
            thread,
            pending: false,
            max_sources,
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
        if self.pending {
            return Err(PollWorkerError::Busy);
        }
        let sources = index.source_ids();
        if sources.len() > self.max_sources {
            return Err(PollWorkerError::Capacity);
        }
        self.requests
            .try_send(Scan {
                sources,
                max_checks,
            })
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
    /// Closes both queues without blocking. Joining waits for any active file IO.
    #[must_use]
    pub fn close(self) -> JoinHandle<()> {
        let Self {
            requests,
            results,
            thread,
            ..
        } = self;
        drop(requests);
        drop(results);
        thread
    }
}
