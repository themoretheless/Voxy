//! Source-to-output invalidation, independent of compiled asset dependency order.
use crate::{AssetId, DependencyError, ImportInputs};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportOutcome {
    Published,
    Failed,
}
#[derive(Default, Debug)]
struct Reads {
    published: BTreeSet<AssetId>,
    attempted: BTreeSet<AssetId>,
}
impl Reads {
    fn count(&self) -> usize {
        self.published.union(&self.attempted).count()
    }
}
/// A single owner records accepted import outcomes after ticket validation.
/// Failed attempts retain the last published inputs and replace earlier failures.
/// No OS watcher or job dispatcher is owned here.
#[derive(Debug)]
pub struct SourceDependencies {
    outputs: BTreeMap<AssetId, Reads>,
    max_outputs: usize,
    max_edges: usize,
}
impl SourceDependencies {
    #[must_use]
    pub fn new(max_outputs: usize, max_edges: usize) -> Self {
        Self {
            outputs: BTreeMap::new(),
            max_outputs,
            max_edges,
        }
    }
    /// Records every observed source, including missing/failed reads.
    /// Call only for the current attempt: this index does not validate tickets.
    /// # Errors
    /// Rejects invalid output IDs and capacity overflow without changing mappings.
    pub fn record(
        &mut self,
        output: AssetId,
        inputs: &ImportInputs,
        outcome: ImportOutcome,
    ) -> Result<(), DependencyError> {
        if output.0.is_empty() {
            return Err(DependencyError::InvalidId);
        }
        if !self.outputs.contains_key(&output) && self.outputs.len() >= self.max_outputs {
            return Err(DependencyError::Capacity);
        }
        let observed: BTreeSet<_> = inputs.observations().keys().cloned().collect();
        let replacement = match outcome {
            ImportOutcome::Published => Reads {
                published: observed,
                attempted: BTreeSet::new(),
            },
            ImportOutcome::Failed => Reads {
                published: self
                    .outputs
                    .get(&output)
                    .map(|r| r.published.clone())
                    .unwrap_or_default(),
                attempted: observed,
            },
        };
        let existing = self.outputs.get(&output).map_or(0, Reads::count);
        let total: usize = self.outputs.values().map(Reads::count).sum();
        if total - existing + replacement.count() > self.max_edges {
            return Err(DependencyError::Capacity);
        }
        self.outputs.insert(output, replacement);
        Ok(())
    }
    /// Returns sorted, deduplicated outputs that read any changed source.
    /// Feed these outputs into `AssetDependencies::affected` for transitive imports.
    #[must_use]
    pub fn affected(&self, changed: impl IntoIterator<Item = AssetId>) -> Vec<AssetId> {
        let changed: BTreeSet<_> = changed.into_iter().collect();
        self.outputs
            .iter()
            .filter(|(_, r)| {
                r.published
                    .union(&r.attempted)
                    .any(|id| changed.contains(id))
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
    /// Sorted distinct sources, including failed reads from current attempts.
    #[must_use]
    pub fn source_ids(&self) -> Vec<AssetId> {
        self.outputs
            .values()
            .flat_map(|r| r.published.union(&r.attempted).cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    /// Removes all source observations for a deleted output.
    pub fn remove(&mut self, output: &AssetId) {
        self.outputs.remove(output);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn id(s: &str) -> AssetId {
        AssetId(s.into())
    }
    fn inputs(source: &str, failed: bool) -> ImportInputs {
        let mut inputs = ImportInputs::new(1, 1);
        let _ = inputs.read(id(source), |_, _| {
            if failed {
                Err("missing".into())
            } else {
                Ok(vec![1])
            }
        });
        inputs
    }
    #[test]
    fn failures_watch_missing_inputs_and_preserve_last_good_sources() {
        let mut index = SourceDependencies::new(2, 4);
        index
            .record(
                id("mesh"),
                &inputs("geometry", false),
                ImportOutcome::Published,
            )
            .unwrap();
        index
            .record(id("mesh"), &inputs("include", true), ImportOutcome::Failed)
            .unwrap();
        assert_eq!(
            index.affected([id("include"), id("geometry")]),
            vec![id("mesh")]
        );
        index
            .record(id("mesh"), &inputs("other", true), ImportOutcome::Failed)
            .unwrap();
        assert!(index.affected([id("include")]).is_empty());
        assert_eq!(
            index.affected([id("geometry"), id("other")]),
            vec![id("mesh")]
        );
        index
            .record(
                id("mesh"),
                &inputs("replacement", false),
                ImportOutcome::Published,
            )
            .unwrap();
        assert!(index.affected([id("geometry"), id("other")]).is_empty());
        assert_eq!(index.affected([id("replacement")]), vec![id("mesh")]);
        index.remove(&id("mesh"));
        assert!(index.affected([id("replacement")]).is_empty());
    }
    #[test]
    fn capacity_rejection_preserves_mapping_and_compiled_plan_is_transitive() {
        let mut index = SourceDependencies::new(1, 1);
        index
            .record(
                id("mesh"),
                &inputs("geometry", false),
                ImportOutcome::Published,
            )
            .unwrap();
        assert_eq!(
            index.record(id("mesh"), &inputs("include", true), ImportOutcome::Failed),
            Err(DependencyError::Capacity)
        );
        assert_eq!(
            index.record(
                id("scene"),
                &inputs("geometry", false),
                ImportOutcome::Published
            ),
            Err(DependencyError::Capacity)
        );
        let mut graph = crate::AssetDependencies::new(2, 1);
        graph.declare(id("mesh")).unwrap();
        graph.declare(id("scene")).unwrap();
        graph.set(&id("scene"), [id("mesh")]).unwrap();
        assert_eq!(
            graph.affected(index.affected([id("geometry")])).unwrap(),
            vec![id("mesh"), id("scene")]
        );
        assert!(index.affected([id("unknown")]).is_empty());
    }
}
