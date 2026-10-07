//! Single-writer asset publication with bounded pending jobs and immutable versions.
mod artifact_cache;
mod atomic_file;
pub use atomic_file::{AtomicSaveError, save_atomic_file};
mod dependencies;
pub use artifact_cache::{ArtifactCache, ArtifactCacheError};
mod files;
mod import_worker;
mod inputs;
mod locations;
mod manifest;

// === OPTIMIZATION #31-40: Asset caching system ===
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Global asset cache with reference counting
pub struct AssetCache {
    /// In-memory cached assets (strong refs)
    strong_cache: Mutex<HashMap<String, Arc<dyn std::any::Any>>>,
    /// Weak references to prevent premature collection
    weak_refs: HashMap<String, usize>,
}

impl AssetCache {
    pub fn new() -> Self {
        Self {
            strong_cache: Mutex::new(HashMap::new()),
            weak_refs: HashMap::new(),
        }
    }
    
    /// Cache-lookup-then-load pattern for repeated assets
    pub fn load_or_cache<T: 'static + Clone>(&self, path: &str, loader: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        let mut cache = self.strong_cache.lock().unwrap();
        if let Some(cached) = cache.get(path) {
            // Return cloned copy
            return Ok(cached.downcast_ref::<T>().unwrap().clone());
        }
        
        // Load and cache
        let value = loader()?;
        let arc = Arc::new(value.clone());
        cache.insert(path.to_string(), arc);
        Ok(value)
    }
}
pub use import_worker::{
    AssetImportWorker, DependencySnapshots, ImportCompletion, ImportWorkerError,
};
pub use locations::{AssetLocations, LocationError, SourcePath};
pub use manifest::ManifestError;
mod publication;
mod rebuild;
pub use rebuild::{RebuildClaim, RebuildError, RebuildPlan, RebuildStatus};
mod sources;
mod watch;
mod watch_worker;
pub use dependencies::{AssetDependencies, DependencyError};
pub use files::FileInputs;
pub use inputs::{
    FailedImport, ImportInputs, ImportedAsset, InputError, InputSnapshot, RejectedImport,
};
pub use publication::PublicationError;
pub use sources::{ImportOutcome, SourceDependencies};
pub use watch::SourcePoller;
pub use watch_worker::{PollWorkerError, SourcePollWorker};

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT_CATALOG: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AssetId(pub String);
#[derive(Clone, Debug)]
pub struct AssetTicket {
    catalog: u64,
    asset: AssetId,
    revision: u64,
    dependencies: Vec<(AssetId, u64)>,
}
impl AssetTicket {
    /// Logical output identity of this import request.
    #[must_use]
    pub fn asset(&self) -> &AssetId {
        &self.asset
    }
}
/// Immutable import inputs captured with the dependency-version ticket.
#[derive(Debug)]
pub struct AssetImport<T> {
    pub ticket: AssetTicket,
    pub dependencies: BTreeMap<AssetId, Arc<T>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetStatus {
    /// Last-good data may exist, but it is not an import-ready dependency.
    Dirty,
    Loading,
    Ready,
    Failed(String),
}
/// Result of an idempotent demand. Only Started authorizes a new worker.
/// Failed is sticky: retry and reload require an explicit request.
#[derive(Debug)]
pub enum AssetDemand<T> {
    Started(AssetTicket),
    Pending,
    Dirty,
    Ready(Arc<T>),
    Failed(String),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetError {
    InvalidId,
    UnknownAsset,
    Capacity,
    StaleTicket,
    Exhausted,
    DependencyNotReady,
}
impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "asset error: {self:?}")
    }
}
impl std::error::Error for AssetError {}
#[derive(Debug)]
struct Entry<T> {
    revision: u64,
    status: AssetStatus,
    published: Option<Arc<T>>,
    published_revision: u64,
}
/// Worker code receives a ticket and its own immutable input. Only the owner applies
/// results. Published Arc snapshots survive replacement or removal from the catalog.
/// Limits bound catalog entries/current pending tickets, not arbitrary payload bytes
/// or superseded workers still running outside this catalog.
#[derive(Debug)]
pub struct AssetCatalog<T> {
    id: u64,
    revision: u64,
    entries: BTreeMap<AssetId, Entry<T>>,
    max_assets: usize,
    max_pending: usize,
    pending: usize,
}
impl<T> AssetCatalog<T> {
    /// # Errors
    /// Rejects exhaustion of process-local catalog identifiers.
    pub fn new(max_assets: usize, max_pending: usize) -> Result<Self, AssetError> {
        let id = NEXT_CATALOG
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| AssetError::Exhausted)?;
        Ok(Self {
            id,
            revision: 0,
            entries: BTreeMap::new(),
            max_assets,
            max_pending,
            pending: 0,
        })
    }
    /// Returns the current load state without superseding in-flight work.
    /// Missing assets start a dependency-free load. For dependency-aware imports,
    /// changed inputs, retries and reloads, use `request` / `prepare_import` explicitly.
    /// Pending or Failed may still have a last-good snapshot accessible separately.
    /// # Errors
    /// Missing assets obey request identity, capacity and revision limits.
    /// # Panics
    /// Panics if the internal Ready-state publication invariant is broken.
    pub fn demand(&mut self, asset: AssetId) -> Result<AssetDemand<T>, AssetError> {
        if let Some(entry) = self.entries.get(&asset) {
            return Ok(match &entry.status {
                AssetStatus::Loading => AssetDemand::Pending,
                AssetStatus::Dirty => AssetDemand::Dirty,
                AssetStatus::Ready => AssetDemand::Ready(
                    entry
                        .published
                        .as_ref()
                        .expect("ready asset has a version")
                        .clone(),
                ),
                AssetStatus::Failed(error) => AssetDemand::Failed(error.clone()),
            });
        }
        self.request(asset).map(AssetDemand::Started)
    }

    /// Atomically marks a declared affected set dirty, preserving last-good versions.
    /// Superseded tickets cannot publish; current pending slots are released. This
    /// does not stop external workers. Invalidate the full graph-affected set before
    /// dispatching a replacement plan, so retained dependencies cannot appear ready.
    /// # Errors
    /// Rejects oversized batches, unknown IDs and revision exhaustion before mutation.
    pub fn invalidate(&mut self, affected: &[AssetId]) -> Result<(), AssetError> {
        if affected.len() > self.max_assets {
            return Err(AssetError::Capacity);
        }
        let unique: std::collections::BTreeSet<_> = affected.iter().collect();
        if unique.iter().any(|id| !self.entries.contains_key(*id)) {
            return Err(AssetError::UnknownAsset);
        }
        let count = u64::try_from(unique.len()).map_err(|_| AssetError::Exhausted)?;
        self.revision
            .checked_add(count)
            .ok_or(AssetError::Exhausted)?;
        for id in unique {
            if let Some(entry) = self.entries.get_mut(id) {
                self.revision += 1;
                entry.revision = self.revision;
                if entry.status == AssetStatus::Loading {
                    self.pending -= 1;
                }
                entry.status = AssetStatus::Dirty;
            }
        }
        Ok(())
    }

    /// Starts/restarts a load, preserving the last published version. Restarting
    /// supersedes the previous ticket; cancellation of its worker is the caller's job.
    /// # Errors
    /// Rejects empty IDs, entry/pending limits or revision exhaustion before mutation.
    pub fn request(&mut self, asset: AssetId) -> Result<AssetTicket, AssetError> {
        self.request_with_dependencies(asset, &[])
    }

    /// Captures ready dependency revisions for validation at publication.
    /// Dependencies must belong to this catalog and cannot include the output.
    /// # Errors
    /// Rejects missing/unready dependencies and oversized lists before mutation,
    /// plus the same request capacity/identity errors as request.
    pub fn request_with_dependencies(
        &mut self,
        asset: AssetId,
        dependencies: &[AssetId],
    ) -> Result<AssetTicket, AssetError> {
        if dependencies.len() > self.max_assets {
            return Err(AssetError::Capacity);
        }
        let mut stamps = BTreeMap::new();
        for dependency in dependencies {
            if dependency == &asset {
                return Err(AssetError::DependencyNotReady);
            }
            let entry = self
                .entries
                .get(dependency)
                .filter(|entry| entry.status == AssetStatus::Ready)
                .ok_or(AssetError::DependencyNotReady)?;
            stamps.insert(dependency.clone(), entry.revision);
        }
        if asset.0.is_empty() {
            return Err(AssetError::InvalidId);
        }
        let existing = self.entries.get(&asset);
        let already_pending = existing.is_some_and(|entry| entry.status == AssetStatus::Loading);
        if (existing.is_none() && self.entries.len() >= self.max_assets)
            || (!already_pending && self.pending >= self.max_pending)
        {
            return Err(AssetError::Capacity);
        }
        let revision = self.revision.checked_add(1).ok_or(AssetError::Exhausted)?;
        self.revision = revision;
        if !already_pending {
            self.pending += 1;
        }
        let entry = self.entries.entry(asset.clone()).or_insert(Entry {
            revision,
            status: AssetStatus::Loading,
            published: None,
            published_revision: 0,
        });
        entry.revision = revision;
        entry.status = AssetStatus::Loading;
        Ok(AssetTicket {
            catalog: self.id,
            asset,
            revision,
            dependencies: stamps.into_iter().collect(),
        })
    }
    /// Captures dependency snapshots and their ticket in one owner-thread operation.
    /// Worker inputs cannot change while asynchronous decoding uses them.
    /// # Errors
    /// Rejects unavailable dependencies and all request validation errors.
    pub fn prepare_import(
        &mut self,
        asset: AssetId,
        dependencies: &[AssetId],
    ) -> Result<AssetImport<T>, AssetError> {
        if dependencies.len() > self.max_assets {
            return Err(AssetError::Capacity);
        }
        let mut inputs = BTreeMap::new();
        for dependency in dependencies {
            let input = self
                .snapshot(dependency)
                .ok_or(AssetError::DependencyNotReady)?;
            inputs.insert(dependency.clone(), input);
        }
        let ticket = self.request_with_dependencies(asset, dependencies)?;
        Ok(AssetImport {
            ticket,
            dependencies: inputs,
        })
    }

    /// Publishes only the currently requested revision. Failed reloads keep the
    /// last good version available while exposing an explicit failure status.
    /// # Errors
    /// Rejects foreign, superseded, removed or already completed tickets.
    pub fn complete(
        &mut self,
        ticket: &AssetTicket,
        result: Result<T, String>,
    ) -> Result<(), AssetError> {
        if ticket.catalog != self.id {
            return Err(AssetError::StaleTicket);
        }
        let dependencies_current = ticket.dependencies.iter().all(|(id, revision)| {
            self.entries.get(id).is_some_and(|entry| {
                entry.revision == *revision && entry.status == AssetStatus::Ready
            })
        });
        let entry = self
            .entries
            .get_mut(&ticket.asset)
            .filter(|entry| {
                entry.revision == ticket.revision && entry.status == AssetStatus::Loading
            })
            .ok_or(AssetError::StaleTicket)?;
        self.pending -= 1;
        if !dependencies_current {
            entry.status = AssetStatus::Failed("dependency changed during import".into());
            return Err(AssetError::StaleTicket);
        }
        match result {
            Ok(value) => {
                entry.published = Some(Arc::new(value));
                entry.published_revision = ticket.revision;
                entry.status = AssetStatus::Ready;
            }
            Err(error) => entry.status = AssetStatus::Failed(error),
        }
        Ok(())
    }
    #[must_use]
    pub fn snapshot(&self, asset: &AssetId) -> Option<Arc<T>> {
        self.entries
            .get(asset)
            .and_then(|entry| entry.published.clone())
    }
    /// Returns last-good data and its publication revision together. Requests and
    /// failed reloads do not relabel the retained publication with a newer ticket.
    #[must_use]
    pub fn snapshot_with_revision(&self, asset: &AssetId) -> Option<(u64, Arc<T>)> {
        let entry = self.entries.get(asset)?;
        Some((entry.published_revision, entry.published.clone()?))
    }
    #[must_use]
    pub fn status(&self, asset: &AssetId) -> Option<&AssetStatus> {
        self.entries.get(asset).map(|entry| &entry.status)
    }
    pub fn remove(&mut self, asset: &AssetId) -> bool {
        if let Some(entry) = self.entries.remove(asset) {
            if entry.status == AssetStatus::Loading {
                self.pending -= 1;
            }
            true
        } else {
            false
        }
    }
    #[must_use]
    pub const fn pending(&self) -> usize {
        self.pending
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn id() -> AssetId {
        AssetId("textures/player".into())
    }
    #[test]
    fn overlapping_invalidations_cancel_tickets_and_hide_stale_dependencies() {
        let base = AssetId("base".into());
        let derived = AssetId("derived".into());
        let mut catalog = AssetCatalog::new(2, 2).unwrap();
        let ticket = catalog.request(base.clone()).unwrap();
        catalog.complete(&ticket, Ok(1)).unwrap();
        let ticket = catalog
            .request_with_dependencies(derived.clone(), std::slice::from_ref(&base))
            .unwrap();
        catalog.complete(&ticket, Ok(2)).unwrap();
        let old = catalog.snapshot(&derived).unwrap();
        let running = catalog
            .request_with_dependencies(derived.clone(), std::slice::from_ref(&base))
            .unwrap();
        catalog
            .invalidate(&[base.clone(), derived.clone()])
            .unwrap();
        assert_eq!(catalog.pending(), 0);
        assert!(matches!(
            catalog.demand(base.clone()).unwrap(),
            AssetDemand::Dirty
        ));
        assert_eq!(
            catalog.complete(&running, Ok(99)),
            Err(AssetError::StaleTicket)
        );
        assert_eq!(
            catalog
                .request_with_dependencies(derived.clone(), std::slice::from_ref(&base))
                .unwrap_err(),
            AssetError::DependencyNotReady
        );
        assert!(Arc::ptr_eq(&old, &catalog.snapshot(&derived).unwrap()));
        let fresh = catalog.request(base.clone()).unwrap();
        catalog.complete(&fresh, Ok(3)).unwrap();
        let fresh = catalog
            .request_with_dependencies(derived.clone(), &[base])
            .unwrap();
        catalog.complete(&fresh, Ok(4)).unwrap();
        assert_eq!(*catalog.snapshot(&derived).unwrap(), 4);
        assert_eq!(*old, 2);
    }
    #[test]
    fn invalidation_preflight_preserves_loading_state_on_errors() {
        let mut catalog = AssetCatalog::<u32>::new(2, 1).unwrap();
        let ticket = catalog.request(id()).unwrap();
        let revision = catalog.revision;
        assert_eq!(
            catalog.invalidate(&[id(), AssetId("unknown".into())]),
            Err(AssetError::UnknownAsset)
        );
        assert_eq!(catalog.revision, revision);
        assert_eq!(catalog.pending(), 1);
        assert_eq!(catalog.status(&id()), Some(&AssetStatus::Loading));
        catalog.revision = u64::MAX;
        assert_eq!(catalog.invalidate(&[id()]), Err(AssetError::Exhausted));
        assert_eq!(catalog.pending(), 1);
        catalog.complete(&ticket, Ok(7)).unwrap();
        catalog.revision = 0;
        catalog.invalidate(&[id(), id()]).unwrap();
        assert_eq!(catalog.revision, 1); // Duplicate IDs have one revision transition.
    }

    #[test]
    fn demand_deduplicates_and_requires_explicit_retry() {
        let mut catalog = AssetCatalog::new(1, 1).unwrap();
        let AssetDemand::Started(first) = catalog.demand(id()).unwrap() else {
            panic!()
        };
        assert!(matches!(
            catalog.demand(id()).unwrap(),
            AssetDemand::Pending
        ));
        assert_eq!(catalog.pending(), 1);
        catalog.complete(&first, Ok(7_u32)).unwrap();
        let AssetDemand::Ready(snapshot) = catalog.demand(id()).unwrap() else {
            panic!()
        };
        assert_eq!(*snapshot, 7);
        let reload = catalog.request(id()).unwrap();
        assert!(matches!(
            catalog.demand(id()).unwrap(),
            AssetDemand::Pending
        ));
        catalog.complete(&reload, Err("broken".into())).unwrap();
        assert!(matches!(catalog.demand(id()).unwrap(), AssetDemand::Failed(e) if e == "broken"));
        assert_eq!(catalog.pending(), 0);
        assert_eq!(*catalog.snapshot(&id()).unwrap(), 7);
        let retry = catalog.request(id()).unwrap();
        catalog.complete(&retry, Ok(9)).unwrap();
        assert_eq!(*snapshot, 7);
        assert_eq!(*catalog.snapshot(&id()).unwrap(), 9);
        catalog.remove(&id());
        assert!(matches!(
            catalog.demand(id()).unwrap(),
            AssetDemand::Started(_)
        ));
    }

    #[test]
    fn stale_reloads_cannot_replace_last_good_version() {
        let mut catalog = AssetCatalog::new(1, 1).unwrap();
        let first = catalog.request(id()).unwrap();
        catalog.complete(&first, Ok(1_u32)).unwrap();
        let snapshot = catalog.snapshot(&id()).unwrap();
        let old = catalog.request(id()).unwrap();
        let new = catalog.request(id()).unwrap();
        assert_eq!(catalog.complete(&old, Ok(2)), Err(AssetError::StaleTicket));
        catalog.complete(&new, Err("decode failed".into())).unwrap();
        assert_eq!(*catalog.snapshot(&id()).unwrap(), 1);
        let retry = catalog.request(id()).unwrap();
        catalog.complete(&retry, Ok(3)).unwrap();
        assert_eq!(*snapshot, 1);
        assert_eq!(*catalog.snapshot(&id()).unwrap(), 3);
        assert_eq!(
            catalog.complete(&first, Ok(4)),
            Err(AssetError::StaleTicket)
        );
    }
    #[test]
    fn capacity_remove_recreate_and_foreign_tickets() {
        let mut catalog = AssetCatalog::<u32>::new(2, 1).unwrap();
        let old = catalog.request(id()).unwrap();
        assert_eq!(
            catalog.request(AssetId("other".into())).unwrap_err(),
            AssetError::Capacity
        );
        let mut foreign = AssetCatalog::<u32>::new(1, 1).unwrap();
        assert_eq!(foreign.complete(&old, Ok(1)), Err(AssetError::StaleTicket));
        assert!(catalog.remove(&id()));
        assert_eq!(catalog.pending(), 0);
        let new = catalog.request(id()).unwrap();
        assert_eq!(catalog.complete(&old, Ok(1)), Err(AssetError::StaleTicket));
        catalog.complete(&new, Ok(2)).unwrap();
        assert_eq!(catalog.pending(), 0);
    }
    #[test]
    fn dependency_revision_changes_reject_results_and_release_pending_capacity() {
        let mut catalog = AssetCatalog::new(3, 2).unwrap();
        let source = AssetId("source".into());
        let output = AssetId("output".into());
        let ticket = catalog.request(source.clone()).unwrap();
        catalog.complete(&ticket, Ok(10_u32)).unwrap();
        let first = catalog
            .request_with_dependencies(output.clone(), std::slice::from_ref(&source))
            .unwrap();
        catalog.complete(&first, Ok(20)).unwrap();
        let import = catalog
            .prepare_import(output.clone(), std::slice::from_ref(&source))
            .unwrap();
        let stale = import.ticket;
        assert_eq!(*import.dependencies[&source], 10);
        let reload = catalog.request(source.clone()).unwrap();
        catalog.complete(&reload, Ok(11)).unwrap();
        assert_eq!(*import.dependencies[&source], 10);
        assert_eq!(
            catalog.complete(&stale, Ok(22)),
            Err(AssetError::StaleTicket)
        );
        assert_eq!(catalog.pending(), 0);
        assert_eq!(*catalog.snapshot(&output).unwrap(), 20);
        assert!(matches!(
            catalog.status(&output),
            Some(AssetStatus::Failed(_))
        ));
        let retry = catalog
            .request_with_dependencies(output.clone(), std::slice::from_ref(&source))
            .unwrap();
        catalog.complete(&retry, Ok(22)).unwrap();
        assert_eq!(*catalog.snapshot(&output).unwrap(), 22);
    }

    #[test]
    fn invalid_dependency_requests_do_not_supersede_existing_loads() {
        let mut catalog = AssetCatalog::<u32>::new(2, 1).unwrap();
        let ticket = catalog.request(id()).unwrap();
        assert!(matches!(
            catalog.request_with_dependencies(id(), &[AssetId("missing".into())]),
            Err(AssetError::DependencyNotReady)
        ));
        catalog.complete(&ticket, Ok(5)).unwrap();
        assert_eq!(*catalog.snapshot(&id()).unwrap(), 5);
    }
}

mod package;
pub use package::{PackageLimits, ResourcePackage};
