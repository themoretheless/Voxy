//! Single import worker with owner-thread publication and bounded outstanding work.
use crate::{
    AssetId, AssetImport, AssetTicket, FailedImport, FileInputs, ImportInputs, ImportedAsset,
    RebuildClaim, SourcePath,
};
use std::{
    collections::BTreeMap,
    sync::Arc,
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread::JoinHandle,
};
/// Immutable compiled versions supplied alongside the import ticket.
pub type DependencySnapshots<T> = BTreeMap<AssetId, Arc<ImportedAsset<T>>>;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportWorkerError {
    Busy,
    Disconnected,
    DependencyInputsRequired,
    Capacity,
    ClaimMismatch,
    LocationRequired,
}
impl std::fmt::Display for ImportWorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "import worker error: {self:?}")
    }
}
impl std::error::Error for ImportWorkerError {}
#[derive(Debug)]
pub struct ImportCompletion<T> {
    pub ticket: AssetTicket,
    pub rebuild: Option<RebuildClaim>,
    pub source: Option<SourcePath>,
    pub result: Result<ImportedAsset<T>, FailedImport<String>>,
}
#[derive(Debug)]
struct WorkerJob<T> {
    import: AssetImport<ImportedAsset<T>>,
    rebuild: Option<RebuildClaim>,
    source: Option<SourcePath>,
}
/// A typed decoder runs on one dedicated thread. One unconsumed completion is
/// allowed. Captured input limits do not bound decoded output memory or CPU time.
/// A decoder panic disconnects the worker; the owner retains its original ticket
/// and must settle/retry that catalog request. No panic recovery is promised.
#[derive(Debug)]
pub struct AssetImportWorker<T> {
    requests: SyncSender<WorkerJob<T>>,
    results: Receiver<ImportCompletion<T>>,
    thread: JoinHandle<()>,
    pending: bool,
    max_dependencies: usize,
    requires_location: bool,
}
impl<T: Send + Sync + 'static> AssetImportWorker<T> {
    /// Creates a file-backed decoder. All source reads must use the supplied
    /// observations for provenance; successful decoding revalidates those files.
    /// This convenience decoder accepts dependency-free tickets only.
    /// # Errors
    /// Returns OS thread creation errors.
    pub fn new(
        provider: FileInputs,
        max_inputs: usize,
        max_input_bytes: usize,
        mut decoder: impl FnMut(&AssetId, &FileInputs, &mut ImportInputs) -> Result<T, String>
        + Send
        + 'static,
    ) -> std::io::Result<Self> {
        Self::new_with_dependencies(
            provider,
            max_inputs,
            max_input_bytes,
            0,
            move |asset, provider, inputs, _| decoder(asset, provider, inputs),
        )
    }
    /// Supplies immutable compiled inputs captured by `AssetCatalog::prepare_import`.
    /// Limits bound file observations and dependency snapshot counts separately.
    /// # Errors
    /// Returns OS thread creation errors.
    pub fn new_with_dependencies(
        provider: FileInputs,
        max_inputs: usize,
        max_input_bytes: usize,
        max_dependencies: usize,
        mut decoder: impl FnMut(
            &AssetId,
            &FileInputs,
            &mut ImportInputs,
            &DependencySnapshots<T>,
        ) -> Result<T, String>
        + Send
        + 'static,
    ) -> std::io::Result<Self> {
        Self::spawn(
            provider,
            max_inputs,
            max_input_bytes,
            max_dependencies,
            false,
            move |asset, _, provider, inputs, dependencies| {
                decoder(asset, provider, inputs, dependencies)
            },
        )
    }
    /// A decoder whose source location is captured independently of the logical ID.
    /// # Errors
    /// Returns OS thread creation errors.
    pub fn new_with_locations(
        provider: FileInputs,
        max_inputs: usize,
        max_input_bytes: usize,
        max_dependencies: usize,
        mut decoder: impl FnMut(
            &AssetId,
            &SourcePath,
            &FileInputs,
            &mut ImportInputs,
            &DependencySnapshots<T>,
        ) -> Result<T, String>
        + Send
        + 'static,
    ) -> std::io::Result<Self> {
        Self::spawn(
            provider,
            max_inputs,
            max_input_bytes,
            max_dependencies,
            true,
            move |asset, source, provider, inputs, dependencies| {
                let source = source.ok_or("source location required")?;
                decoder(asset, source, provider, inputs, dependencies)
            },
        )
    }
    fn spawn(
        provider: FileInputs,
        max_inputs: usize,
        max_input_bytes: usize,
        max_dependencies: usize,
        requires_location: bool,
        mut decoder: impl FnMut(
            &AssetId,
            Option<&SourcePath>,
            &FileInputs,
            &mut ImportInputs,
            &DependencySnapshots<T>,
        ) -> Result<T, String>
        + Send
        + 'static,
    ) -> std::io::Result<Self> {
        let (requests, work) = mpsc::sync_channel::<WorkerJob<T>>(1);
        let (output, results) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("voxy-asset-import".into())
            .spawn(move || {
                while let Ok(job) = work.recv() {
                    let WorkerJob {
                        import: job,
                        rebuild,
                        source,
                    } = job;
                    let ticket = job.ticket;
                    let attempt =
                        ImportInputs::new(max_inputs, max_input_bytes).decode_observed(|inputs| {
                            decoder(
                                &ticket.asset,
                                source.as_ref(),
                                &provider,
                                inputs,
                                &job.dependencies,
                            )
                        });
                    let result = match attempt {
                        Ok((value, inputs)) => inputs
                            .finish_observed(value, |id, limit| provider.read(id, limit))
                            .map_err(|rejected| FailedImport {
                                error: format!("{:?}", rejected.error),
                                inputs: rejected.inputs,
                            }),
                        Err(failure) => Err(failure),
                    };
                    if output
                        .send(ImportCompletion {
                            ticket,
                            rebuild,
                            source,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self {
            requests,
            results,
            thread,
            pending: false,
            max_dependencies,
            requires_location,
        })
    }
    /// Copies a ticket without taking ownership of the owner's recovery handle.
    /// Ticket currency is checked by the catalog at publication, not by this worker.
    /// # Errors
    /// Rejects outstanding work or a disconnected thread without enqueueing a job.
    pub fn submit(&mut self, ticket: &AssetTicket) -> Result<(), ImportWorkerError> {
        if !ticket.dependencies.is_empty() {
            return Err(ImportWorkerError::DependencyInputsRequired);
        }
        self.enqueue(
            AssetImport {
                ticket: ticket.clone(),
                dependencies: BTreeMap::new(),
            },
            None,
            None,
        )
    }
    /// Copies a prepared ticket and its immutable dependency snapshots.
    /// # Errors
    /// Rejects missing/unexpected dependency IDs, capacity, busy or stopped workers.
    pub fn submit_import(
        &mut self,
        job: &AssetImport<ImportedAsset<T>>,
    ) -> Result<(), ImportWorkerError> {
        self.submit_prepared(job, None, None)
    }
    /// Carries the exact rebuild attempt token through the worker completion.
    /// # Errors
    /// Rejects resource/claim mismatch and all prepared submission failures.
    pub fn submit_rebuild(
        &mut self,
        claim: &RebuildClaim,
        job: &AssetImport<ImportedAsset<T>>,
    ) -> Result<(), ImportWorkerError> {
        if claim.asset() != &job.ticket.asset {
            return Err(ImportWorkerError::ClaimMismatch);
        }
        self.submit_prepared(job, Some(claim.clone()), None)
    }
    /// Captures a location for this prepared import; caller invalidates tickets on relocation.
    /// # Errors
    /// Rejects invalid dependency input mappings, capacity, busy or stopped workers.
    pub fn submit_at(
        &mut self,
        job: &AssetImport<ImportedAsset<T>>,
        source: &SourcePath,
    ) -> Result<(), ImportWorkerError> {
        self.submit_prepared(job, None, Some(source.clone()))
    }
    /// Captures both the rebuild attempt and its current source location.
    /// # Errors
    /// Rejects mismatched claims and all prepared submission failures.
    pub fn submit_rebuild_at(
        &mut self,
        claim: &RebuildClaim,
        job: &AssetImport<ImportedAsset<T>>,
        source: &SourcePath,
    ) -> Result<(), ImportWorkerError> {
        if claim.asset() != &job.ticket.asset {
            return Err(ImportWorkerError::ClaimMismatch);
        }
        self.submit_prepared(job, Some(claim.clone()), Some(source.clone()))
    }
    fn submit_prepared(
        &mut self,
        job: &AssetImport<ImportedAsset<T>>,
        rebuild: Option<RebuildClaim>,
        source: Option<SourcePath>,
    ) -> Result<(), ImportWorkerError> {
        if self.pending {
            return Err(ImportWorkerError::Busy);
        }
        if job.dependencies.len() > self.max_dependencies {
            return Err(ImportWorkerError::Capacity);
        }
        if job.dependencies.len() != job.ticket.dependencies.len()
            || job
                .ticket
                .dependencies
                .iter()
                .any(|(id, _)| !job.dependencies.contains_key(id))
        {
            return Err(ImportWorkerError::DependencyInputsRequired);
        }
        self.enqueue(
            AssetImport {
                ticket: job.ticket.clone(),
                dependencies: job.dependencies.clone(),
            },
            rebuild,
            source,
        )
    }
    fn enqueue(
        &mut self,
        job: AssetImport<ImportedAsset<T>>,
        rebuild: Option<RebuildClaim>,
        source: Option<SourcePath>,
    ) -> Result<(), ImportWorkerError> {
        if self.pending {
            return Err(ImportWorkerError::Busy);
        }
        if self.requires_location && source.is_none() {
            return Err(ImportWorkerError::LocationRequired);
        }
        self.requests
            .try_send(WorkerJob {
                import: job,
                rebuild,
                source,
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => ImportWorkerError::Busy,
                mpsc::TrySendError::Disconnected(_) => ImportWorkerError::Disconnected,
            })?;
        self.pending = true;
        Ok(())
    }
    /// True until the owner consumes the submitted job's completion.
    #[must_use]
    pub const fn is_busy(&self) -> bool {
        self.pending
    }
    /// Takes a completed result without waiting, decoding or performing file IO.
    /// # Errors
    /// Reports worker disconnection; the owner must settle its outstanding ticket.
    pub fn try_result(&mut self) -> Result<Option<ImportCompletion<T>>, ImportWorkerError> {
        match self.results.try_recv() {
            Ok(result) => {
                self.pending = false;
                Ok(Some(result))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.pending = false;
                Err(ImportWorkerError::Disconnected)
            }
        }
    }
    /// Closes queues without blocking. Joining waits for active decoding and IO.
    /// Dropping the worker instead detaches its closing thread.
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AssetCatalog, AssetError, PublicationError, SourceDependencies};
    fn id(s: &str) -> AssetId {
        AssetId(s.into())
    }
    fn value(v: u32) -> ImportedAsset<u32> {
        ImportInputs::new(0, 0).finish(v, |_, _| panic!()).unwrap()
    }
    fn wait(worker: &mut AssetImportWorker<u32>) -> ImportCompletion<u32> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(result) = worker.try_result().unwrap() {
                return result;
            }
            assert!(std::time::Instant::now() < deadline, "worker timeout");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn worker_returns_exact_attempt_token_and_rejects_mismatched_resource() {
        let mut graph = crate::AssetDependencies::new(2, 0);
        graph.declare(id("a")).unwrap();
        graph.declare(id("b")).unwrap();
        let mut plan = crate::RebuildPlan::new(&graph, [id("a")], 1).unwrap();
        let claim = plan.claim_ready().unwrap().unwrap();
        let mut catalog = AssetCatalog::new(2, 2).unwrap();
        let wrong = catalog.prepare_import(id("b"), &[]).unwrap();
        let correct = catalog.prepare_import(id("a"), &[]).unwrap();
        let mut worker =
            AssetImportWorker::new(FileInputs::new(".").unwrap(), 0, 0, |_, _, _| Ok(42_u32))
                .unwrap();
        assert_eq!(
            worker.submit_rebuild(&claim, &wrong),
            Err(ImportWorkerError::ClaimMismatch)
        );
        worker.submit_rebuild(&claim, &correct).unwrap();
        let finished = wait(&mut worker);
        let returned = finished.rebuild.unwrap();
        assert_eq!(returned.asset(), &id("a"));
        let mut sources = SourceDependencies::new(1, 0);
        catalog
            .complete_observed(&mut sources, &finished.ticket, finished.result)
            .unwrap();
        plan.finish_claim(&returned, true).unwrap();
        assert!(plan.is_finished());
        catalog
            .complete(&wrong.ticket, Err("cancelled".into()))
            .unwrap();
        worker.close().join().unwrap();
    }

    #[test]
    fn worker_holds_captured_dependencies_and_owner_rejects_changed_revision() {
        let mut catalog = AssetCatalog::new(2, 2).unwrap();
        let mut sources = SourceDependencies::new(2, 0);
        let ticket = catalog.request(id("base")).unwrap();
        catalog
            .complete_observed(&mut sources, &ticket, Ok(value(1)))
            .unwrap();
        let (captured, observed) = mpsc::sync_channel(1);
        let (resume, barrier) = mpsc::sync_channel(1);
        let mut worker = AssetImportWorker::new_with_dependencies(
            FileInputs::new(".").unwrap(),
            0,
            0,
            1,
            move |_, _, _, dependencies| {
                let base = *dependencies[&id("base")].value();
                captured.send(base).unwrap();
                barrier.recv().unwrap();
                Ok(base + 1)
            },
        )
        .unwrap();
        let mut job = catalog
            .prepare_import(id("derived"), &[id("base")])
            .unwrap();
        assert_eq!(
            worker.submit(&job.ticket),
            Err(ImportWorkerError::DependencyInputsRequired)
        );
        worker.submit_import(&job).unwrap();
        job.dependencies.clear(); // Worker already owns its Arc snapshot references.
        assert_eq!(
            observed
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap(),
            1
        );
        let ticket = catalog.request(id("base")).unwrap();
        catalog
            .complete_observed(&mut sources, &ticket, Ok(value(2)))
            .unwrap();
        resume.send(()).unwrap();
        let finished = wait(&mut worker);
        assert_eq!(*finished.result.as_ref().unwrap().value(), 2);
        assert_eq!(
            catalog.complete_observed(&mut sources, &finished.ticket, finished.result),
            Err(PublicationError::Asset(AssetError::StaleTicket))
        );
        assert_eq!(catalog.pending(), 0);
        assert!(catalog.snapshot(&id("derived")).is_none());
        let job = catalog
            .prepare_import(id("derived"), &[id("base")])
            .unwrap();
        worker.submit_import(&job).unwrap();
        assert_eq!(
            observed
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap(),
            2
        );
        resume.send(()).unwrap();
        let finished = wait(&mut worker);
        catalog
            .complete_observed(&mut sources, &finished.ticket, finished.result)
            .unwrap();
        assert_eq!(*catalog.snapshot(&id("derived")).unwrap().value(), 3);
        worker.close().join().unwrap();
    }
}
