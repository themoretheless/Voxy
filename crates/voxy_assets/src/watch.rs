//! Bounded round-robin content polling. Run file IO outside the frame loop.
use crate::{AssetId, DependencyError, SourceDependencies};
#[derive(Clone, Debug, Eq, PartialEq)]
enum Fingerprint {
    Content([u8; 32]),
    Unavailable,
}
/// Polls only registered sources; no directory walk, OS events or worker ownership.
/// Unknown sources emit one initial invalidation, avoiding a silent baseline race.
/// Unavailable states coalesce all provider errors; recovery emits an invalidation.
#[derive(Debug)]
pub struct SourcePoller {
    sources: Vec<(AssetId, Option<Fingerprint>)>,
    cursor: usize,
    max_sources: usize,
    max_file_bytes: usize,
}
impl SourcePoller {
    #[must_use]
    pub fn new(max_sources: usize, max_file_bytes: usize) -> Self {
        Self {
            sources: Vec::new(),
            cursor: 0,
            max_sources,
            max_file_bytes,
        }
    }
    /// Seeds the first comparison from a completed observed import. This avoids
    /// unnecessary reloads while still detecting changes since that import.
    /// # Errors
    /// Rejects invalid IDs, source count and observed input byte limits.
    pub fn with_observations(
        max_sources: usize,
        max_file_bytes: usize,
        observations: &std::collections::BTreeMap<
            AssetId,
            Result<crate::InputSnapshot, crate::InputError>,
        >,
    ) -> Result<Self, DependencyError> {
        if observations.len() > max_sources
            || observations.values().any(|value| {
                value
                    .as_ref()
                    .is_ok_and(|input| input.bytes.len() > max_file_bytes)
            })
        {
            return Err(DependencyError::Capacity);
        }
        if observations.keys().any(|id| id.0.is_empty()) {
            return Err(DependencyError::InvalidId);
        }
        Ok(Self {
            sources: observations
                .iter()
                .map(|(id, input)| {
                    (
                        id.clone(),
                        Some(input.as_ref().map_or(Fingerprint::Unavailable, |input| {
                            Fingerprint::Content(input.digest)
                        })),
                    )
                })
                .collect(),
            cursor: 0,
            max_sources,
            max_file_bytes,
        })
    }
    /// Adopts the current source index, retaining observations for surviving IDs.
    /// # Errors
    /// Rejects source capacity overflow before modifying polling state.
    pub fn reconcile(&mut self, index: &SourceDependencies) -> Result<(), DependencyError> {
        self.reconcile_ids(index.source_ids())
    }
    pub(crate) fn reconcile_ids(&mut self, ids: Vec<AssetId>) -> Result<(), DependencyError> {
        if ids.len() > self.max_sources {
            return Err(DependencyError::Capacity);
        }
        if self.sources.iter().map(|(id, _)| id).eq(ids.iter()) {
            return Ok(());
        }
        let sources = ids
            .into_iter()
            .map(|id| {
                let previous = self
                    .sources
                    .binary_search_by(|(old, _)| old.cmp(&id))
                    .ok()
                    .and_then(|i| self.sources[i].1.clone());
                (id, previous)
            })
            .collect();
        self.sources = sources;
        self.cursor = 0;
        Ok(())
    }
    /// Checks at most `max_checks` distinct sources, advancing fairly across calls.
    /// The provider must enforce the supplied per-file byte limit while reading.
    /// Returned oversized buffers are unavailable defensively. Read/hash time is
    /// not bounded by wall-clock duration; dispatch this call on an IO worker.
    /// Changes that revert between polls are unobservable. Results are sorted IDs.
    pub fn poll(
        &mut self,
        max_checks: usize,
        mut reader: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
    ) -> Vec<AssetId> {
        let mut changed = Vec::new();
        for _ in 0..max_checks.min(self.sources.len()) {
            let (id, previous) = &mut self.sources[self.cursor];
            let current = match reader(id, self.max_file_bytes) {
                Ok(bytes) if bytes.len() <= self.max_file_bytes => {
                    Fingerprint::Content(*blake3::hash(&bytes).as_bytes())
                }
                _ => Fingerprint::Unavailable,
            };
            if previous.as_ref() != Some(&current) {
                changed.push(id.clone());
            }
            *previous = Some(current);
            self.cursor = (self.cursor + 1) % self.sources.len();
        }
        changed.sort();
        changed
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ImportInputs, ImportOutcome};
    #[test]
    fn observed_baseline_does_not_invalidate_unchanged_source_or_hide_drift() {
        let mut inputs = ImportInputs::new(1, 1);
        inputs
            .read(AssetId("tone".into()), |_, _| Ok(vec![1]))
            .unwrap();
        let mut poller = SourcePoller::with_observations(1, 1, inputs.observations()).unwrap();
        assert!(poller.poll(1, |_, _| Ok(vec![1])).is_empty());
        assert_eq!(
            poller.poll(1, |_, _| Ok(vec![2])),
            vec![AssetId("tone".into())]
        );
        assert_eq!(
            poller.poll(1, |_, _| Err("removed".into())),
            vec![AssetId("tone".into())]
        );
        assert!(
            poller
                .poll(1, |_, _| Err("still removed".into()))
                .is_empty()
        );
        assert_eq!(
            poller.poll(1, |_, _| Ok(vec![1])),
            vec![AssetId("tone".into())]
        );
        assert!(SourcePoller::with_observations(0, 1, inputs.observations()).is_err());
    }
    fn id(s: &str) -> AssetId {
        AssetId(s.into())
    }
    fn index(names: &[&str]) -> SourceDependencies {
        let mut inputs = ImportInputs::new(names.len(), names.len());
        for name in names {
            inputs.read(id(name), |_, _| Ok(vec![1])).unwrap();
        }
        let mut index = SourceDependencies::new(1, names.len());
        index
            .record(id("output"), &inputs, ImportOutcome::Published)
            .unwrap();
        index
    }
    #[test]
    fn bounded_round_robin_detects_content_deletion_and_recovery() {
        let index = index(&["a", "b"]);
        let mut poller = SourcePoller::new(2, 1);
        poller.reconcile(&index).unwrap();
        assert!(poller.poll(0, |_, _| panic!()).is_empty());
        assert_eq!(
            poller.poll(1, |id, limit| {
                assert_eq!(id.0, "a");
                assert_eq!(limit, 1);
                Ok(vec![1])
            }),
            vec![id("a")]
        );
        poller.reconcile(&index).unwrap(); // Reconcile does not restart an unchanged scan.
        assert_eq!(
            poller.poll(1, |source, _| {
                assert_eq!(source.0, "b");
                Err("missing".into())
            }),
            vec![id("b")]
        );
        assert!(
            poller
                .poll(2, |source, _| if source.0 == "a" {
                    Ok(vec![1])
                } else {
                    Err("permission changed".into())
                })
                .is_empty()
        );
        assert_eq!(poller.poll(2, |_, _| Ok(vec![2])), vec![id("a"), id("b")]);
        assert_eq!(
            poller.poll(2, |_, _| Ok(vec![2, 3])),
            vec![id("a"), id("b")]
        );
        assert!(poller.poll(2, |_, _| Err("too large".into())).is_empty());
    }
    #[test]
    fn reconciliation_preserves_survivors_and_capacity_rejection_is_atomic() {
        let mut poller = SourcePoller::new(2, 1);
        poller.reconcile(&index(&["a", "b"])).unwrap();
        poller.poll(2, |_, _| Ok(vec![1]));
        assert_eq!(
            poller.reconcile(&index(&["a", "b", "c"])),
            Err(DependencyError::Capacity)
        );
        assert!(poller.poll(2, |_, _| Ok(vec![1])).is_empty());
        poller.reconcile(&index(&["b", "c"])).unwrap();
        assert_eq!(poller.poll(9, |_, _| Ok(vec![1])), vec![id("c")]);
        poller.reconcile(&SourceDependencies::new(0, 0)).unwrap();
        assert!(poller.poll(1, |_, _| panic!()).is_empty());
    }
}
