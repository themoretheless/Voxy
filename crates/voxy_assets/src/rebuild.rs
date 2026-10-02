//! Owner-driven dependency-aware rebuild scheduling, independent of worker runtime.
use crate::{AssetDependencies, AssetId, DependencyError};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_CLAIM: AtomicU64 = AtomicU64::new(1);
/// Identifies one dispatched attempt, including across plan replacement or deferral.
#[derive(Clone, Debug)]
pub struct RebuildClaim {
    serial: u64,
    asset: AssetId,
}
impl RebuildClaim {
    #[must_use]
    pub const fn asset(&self) -> &AssetId {
        &self.asset
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RebuildStatus {
    Pending,
    Running,
    Published,
    Failed,
    Blocked,
}
#[derive(Debug, Eq, PartialEq)]
pub enum RebuildError {
    Dependency(DependencyError),
    InvalidLimit,
    Exhausted,
    StaleClaim,
    InvalidTransition(AssetId),
}
impl std::fmt::Display for RebuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rebuild error: {self:?}")
    }
}
impl std::error::Error for RebuildError {}
#[derive(Debug)]
struct Job {
    dependencies: BTreeSet<AssetId>,
    status: RebuildStatus,
    claim: Option<u64>,
}
/// A bounded DAG snapshot for one change batch. Report successful owner publication,
/// not merely decoder success. Dependencies outside this plan must be ready in the
/// catalog when preparing imports. New changes require a new plan and ticket rules.
#[derive(Debug)]
pub struct RebuildPlan {
    jobs: BTreeMap<AssetId, Job>,
    max_running: usize,
    running: usize,
}
impl RebuildPlan {
    /// # Errors
    /// Rejects zero concurrency, undeclared changes and invalid dependency graphs.
    pub fn new(
        graph: &AssetDependencies,
        changed: impl IntoIterator<Item = AssetId>,
        max_running: usize,
    ) -> Result<Self, RebuildError> {
        if max_running == 0 {
            return Err(RebuildError::InvalidLimit);
        }
        let ids = graph.affected(changed).map_err(RebuildError::Dependency)?;
        let jobs = ids
            .into_iter()
            .map(|id| {
                let dependencies = graph.dependencies(&id).cloned().unwrap_or_default();
                (
                    id,
                    Job {
                        dependencies,
                        status: RebuildStatus::Pending,
                        claim: None,
                    },
                )
            })
            .collect();
        Ok(Self {
            jobs,
            max_running,
            running: 0,
        })
    }
    /// Builds a replacement batch from new changes and every unfinished old job.
    /// Published/failed/blocked jobs are retried only when reached by those roots.
    /// The old plan is unchanged. Before dispatch, invalidate all resident outputs
    /// in the replacement and discard old completions using catalog ticket checks.
    /// Draining old workers remains the owner's responsibility.
    /// # Errors
    /// Rejects undeclared changed/unfinished IDs and invalid current graphs.
    pub fn replacement(
        &self,
        graph: &AssetDependencies,
        changed: impl IntoIterator<Item = AssetId>,
    ) -> Result<Self, RebuildError> {
        let unfinished = self
            .jobs
            .iter()
            .filter(|(_, job)| {
                matches!(job.status, RebuildStatus::Pending | RebuildStatus::Running)
            })
            .map(|(id, _)| id.clone());
        Self::new(
            graph,
            changed.into_iter().chain(unfinished),
            self.max_running,
        )
    }
    /// Sorted outputs to invalidate before dispatching this batch.
    #[must_use]
    pub fn outputs(&self) -> Vec<AssetId> {
        self.jobs.keys().cloned().collect()
    }
    /// Claims one ready job, respecting concurrency and published predecessors.
    /// For asynchronous work use `claim_ready` and token-checked settlement.
    /// This ID-only interface is for local owner-controlled scheduling.
    /// If worker submission is rejected, return that claim with `defer`.
    pub fn next_ready(&mut self) -> Option<AssetId> {
        if self.running >= self.max_running {
            return None;
        }
        let ready = self
            .jobs
            .iter()
            .find(|(_, job)| {
                job.status == RebuildStatus::Pending
                    && job.dependencies.iter().all(|dependency| {
                        self.jobs
                            .get(dependency)
                            .is_none_or(|job| job.status == RebuildStatus::Published)
                    })
            })
            .map(|(id, _)| id.clone())?;
        self.jobs.get_mut(&ready)?.status = RebuildStatus::Running;
        self.running += 1;
        Some(ready)
    }
    /// Claims ready work with a unique attempt token for asynchronous completion.
    /// # Errors
    /// Reports process-local token exhaustion without retaining a running claim.
    pub fn claim_ready(&mut self) -> Result<Option<RebuildClaim>, RebuildError> {
        let Some(asset) = self.next_ready() else {
            return Ok(None);
        };
        let Ok(serial) =
            NEXT_CLAIM.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
        else {
            self.defer(&asset)?;
            return Err(RebuildError::Exhausted);
        };
        if let Some(job) = self.jobs.get_mut(&asset) {
            job.claim = Some(serial);
        }
        Ok(Some(RebuildClaim { serial, asset }))
    }
    /// Returns an asynchronous claim after backpressure.
    /// # Errors
    /// Rejects old/foreign/already settled tokens without changing any job.
    pub fn defer_claim(&mut self, claim: &RebuildClaim) -> Result<(), RebuildError> {
        self.validate_claim(claim)?;
        self.defer(&claim.asset)
    }
    /// Reports publication for exactly one asynchronous attempt.
    /// # Errors
    /// Rejects old/foreign/already settled tokens without changing any job.
    pub fn finish_claim(
        &mut self,
        claim: &RebuildClaim,
        published: bool,
    ) -> Result<(), RebuildError> {
        self.validate_claim(claim)?;
        self.finish(&claim.asset, published)
    }
    fn validate_claim(&self, claim: &RebuildClaim) -> Result<(), RebuildError> {
        if self.jobs.get(&claim.asset).is_some_and(|job| {
            job.status == RebuildStatus::Running && job.claim == Some(claim.serial)
        }) {
            Ok(())
        } else {
            Err(RebuildError::StaleClaim)
        }
    }
    /// Releases a claim after backpressure without marking it failed.
    /// # Errors
    /// Rejects jobs that are absent or not running, preserving state.
    pub fn defer(&mut self, id: &AssetId) -> Result<(), RebuildError> {
        self.transition(id, RebuildStatus::Pending)
    }
    /// Marks owner publication success/failure. Failure blocks all pending dependents.
    /// # Errors
    /// Rejects jobs that are absent or not running, preserving state.
    pub fn finish(&mut self, id: &AssetId, published: bool) -> Result<(), RebuildError> {
        self.transition(
            id,
            if published {
                RebuildStatus::Published
            } else {
                RebuildStatus::Failed
            },
        )?;
        loop {
            let blocked: Vec<_> = self
                .jobs
                .iter()
                .filter(|(_, job)| {
                    job.status == RebuildStatus::Pending
                        && job.dependencies.iter().any(|dependency| {
                            self.jobs.get(dependency).is_some_and(|job| {
                                matches!(job.status, RebuildStatus::Failed | RebuildStatus::Blocked)
                            })
                        })
                })
                .map(|(id, _)| id.clone())
                .collect();
            if blocked.is_empty() {
                break;
            }
            for id in blocked {
                if let Some(job) = self.jobs.get_mut(&id) {
                    job.status = RebuildStatus::Blocked;
                }
            }
        }
        Ok(())
    }
    fn transition(&mut self, id: &AssetId, status: RebuildStatus) -> Result<(), RebuildError> {
        let job = self
            .jobs
            .get_mut(id)
            .filter(|job| job.status == RebuildStatus::Running)
            .ok_or_else(|| RebuildError::InvalidTransition(id.clone()))?;
        job.status = status;
        job.claim = None;
        self.running -= 1;
        Ok(())
    }
    #[must_use]
    pub fn status(&self, id: &AssetId) -> Option<RebuildStatus> {
        self.jobs.get(id).map(|job| job.status)
    }
    /// True when every planned output is published, failed or blocked.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.jobs
            .values()
            .all(|job| !matches!(job.status, RebuildStatus::Pending | RebuildStatus::Running))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn id(s: &str) -> AssetId {
        AssetId(s.into())
    }
    fn graph() -> AssetDependencies {
        let mut graph = AssetDependencies::new(5, 5);
        for s in ["base", "material", "mesh", "scene", "other"] {
            graph.declare(id(s)).unwrap();
        }
        graph.set(&id("material"), [id("base")]).unwrap();
        graph.set(&id("mesh"), [id("base")]).unwrap();
        graph
            .set(&id("scene"), [id("material"), id("mesh")])
            .unwrap();
        graph
    }
    #[test]
    fn diamond_waits_for_publication_and_failure_blocks_dependents() {
        let mut plan = RebuildPlan::new(&graph(), [id("base")], 2).unwrap();
        assert_eq!(plan.next_ready(), Some(id("base")));
        assert!(plan.next_ready().is_none());
        plan.finish(&id("base"), true).unwrap();
        assert_eq!(plan.next_ready(), Some(id("material")));
        assert_eq!(plan.next_ready(), Some(id("mesh")));
        assert!(plan.next_ready().is_none());
        plan.finish(&id("mesh"), true).unwrap();
        assert!(plan.next_ready().is_none());
        plan.finish(&id("material"), false).unwrap();
        assert_eq!(plan.status(&id("scene")), Some(RebuildStatus::Blocked));
        assert!(plan.is_finished());
        assert_eq!(plan.status(&id("other")), None);
    }
    #[test]
    fn replacement_merges_unfinished_work_and_new_changes_without_retrying_failures() {
        let graph = graph();
        let mut old = RebuildPlan::new(&graph, [id("base")], 2).unwrap();
        old.next_ready().unwrap();
        old.finish(&id("base"), true).unwrap();
        old.next_ready().unwrap();
        let mut replacement = old.replacement(&graph, [id("other"), id("other")]).unwrap();
        assert_eq!(
            replacement.outputs(),
            vec![id("material"), id("mesh"), id("other"), id("scene")]
        );
        assert_eq!(old.status(&id("material")), Some(RebuildStatus::Running));
        assert_eq!(replacement.next_ready(), Some(id("material")));
        assert_eq!(replacement.next_ready(), Some(id("mesh")));
        assert!(replacement.next_ready().is_none());
        let restarted = old.replacement(&graph, [id("base")]).unwrap();
        assert_eq!(restarted.status(&id("base")), Some(RebuildStatus::Pending));
        assert!(old.replacement(&graph, [id("unknown")]).is_err());
        old.finish(&id("material"), false).unwrap();
        let mesh = old.next_ready().unwrap();
        old.finish(&mesh, true).unwrap();
        let unrelated = old.replacement(&graph, [id("other")]).unwrap();
        assert_eq!(unrelated.outputs(), vec![id("other")]);
    }
    #[test]
    fn replacing_active_batch_invalidates_old_ticket_before_new_dispatch() {
        use crate::{AssetCatalog, AssetError, AssetStatus};
        let graph = graph();
        let mut catalog = AssetCatalog::new(5, 2).unwrap();
        for name in ["base", "material", "mesh", "scene", "other"] {
            let ticket = catalog.request(id(name)).unwrap();
            catalog.complete(&ticket, Ok(1_u32)).unwrap();
        }
        let mut old = RebuildPlan::new(&graph, [id("base")], 1).unwrap();
        catalog.invalidate(&old.outputs()).unwrap();
        let running = old.next_ready().unwrap();
        let ticket = catalog.request(running).unwrap();
        let mut replacement = old.replacement(&graph, [id("other")]).unwrap();
        catalog.invalidate(&replacement.outputs()).unwrap();
        assert_eq!(catalog.pending(), 0);
        assert_eq!(
            catalog.complete(&ticket, Ok(99)),
            Err(AssetError::StaleTicket)
        );
        assert_eq!(catalog.status(&id("scene")), Some(&AssetStatus::Dirty));
        while let Some(output) = replacement.next_ready() {
            let dependencies: Vec<_> = graph
                .dependencies(&output)
                .unwrap()
                .iter()
                .cloned()
                .collect();
            let import = catalog
                .prepare_import(output.clone(), &dependencies)
                .unwrap();
            let value = import
                .dependencies
                .values()
                .map(|value| **value)
                .sum::<u32>()
                + 2;
            catalog.complete(&import.ticket, Ok(value)).unwrap();
            replacement.finish(&output, true).unwrap();
        }
        assert!(replacement.is_finished());
        assert_eq!(*catalog.snapshot(&id("scene")).unwrap(), 10);
        assert_eq!(catalog.pending(), 0);
    }

    #[test]
    fn attempt_tokens_reject_replacement_deferral_and_duplicate_completion() {
        let graph = graph();
        let mut old = RebuildPlan::new(&graph, [id("base")], 1).unwrap();
        let obsolete = old.claim_ready().unwrap().unwrap();
        let mut replacement = old.replacement(&graph, [id("base")]).unwrap();
        let first = replacement.claim_ready().unwrap().unwrap();
        assert_eq!(
            replacement.finish_claim(&obsolete, true),
            Err(RebuildError::StaleClaim)
        );
        assert_eq!(
            replacement.status(&id("base")),
            Some(RebuildStatus::Running)
        );
        replacement.defer_claim(&first).unwrap();
        let retry = replacement.claim_ready().unwrap().unwrap();
        assert_eq!(
            replacement.finish_claim(&first, false),
            Err(RebuildError::StaleClaim)
        );
        assert_eq!(
            replacement.defer_claim(&obsolete),
            Err(RebuildError::StaleClaim)
        );
        replacement.finish_claim(&retry, true).unwrap();
        assert_eq!(
            replacement.finish_claim(&retry, true),
            Err(RebuildError::StaleClaim)
        );
        assert_eq!(
            replacement.status(&id("base")),
            Some(RebuildStatus::Published)
        );
        assert_eq!(old.status(&id("base")), Some(RebuildStatus::Running));
        assert_eq!(retry.asset(), &id("base"));
    }

    #[test]
    fn deferred_claims_and_invalid_transitions_preserve_order() {
        let mut plan = RebuildPlan::new(&graph(), [id("mesh")], 1).unwrap();
        assert!(plan.finish(&id("scene"), true).is_err());
        assert_eq!(plan.next_ready(), Some(id("mesh")));
        plan.defer(&id("mesh")).unwrap();
        assert_eq!(plan.next_ready(), Some(id("mesh")));
        plan.finish(&id("mesh"), true).unwrap();
        assert_eq!(plan.next_ready(), Some(id("scene")));
        plan.finish(&id("scene"), true).unwrap();
        assert!(plan.is_finished());
        assert!(matches!(
            RebuildPlan::new(&graph(), [id("base")], 0),
            Err(RebuildError::InvalidLimit)
        ));
    }
}
