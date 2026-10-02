//! Bounded import dependencies and deterministic rebuild ordering.
use crate::AssetId;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyError {
    Capacity,
    InvalidId,
    UnknownAsset(AssetId),
    Cycle,
}
impl std::fmt::Display for DependencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "asset dependency error: {self:?}")
    }
}
impl std::error::Error for DependencyError {}
#[derive(Debug)]
pub struct AssetDependencies {
    edges: BTreeMap<AssetId, BTreeSet<AssetId>>,
    max_assets: usize,
    max_edges: usize,
}
impl AssetDependencies {
    #[must_use]
    pub fn new(max_assets: usize, max_edges: usize) -> Self {
        Self {
            edges: BTreeMap::new(),
            max_assets,
            max_edges,
        }
    }
    /// Declares an asset before assigning dependencies. Repeated declarations are no-ops.
    /// # Errors
    /// Rejects empty IDs and capacity overflow before mutation.
    pub fn declare(&mut self, asset: AssetId) -> Result<(), DependencyError> {
        if asset.0.is_empty() {
            return Err(DependencyError::InvalidId);
        }
        if !self.edges.contains_key(&asset) && self.edges.len() >= self.max_assets {
            return Err(DependencyError::Capacity);
        }
        self.edges.entry(asset).or_default();
        Ok(())
    }
    /// Replaces dependencies atomically. All dependency IDs must be declared.
    /// # Errors
    /// Rejects unknown IDs, edge capacity overflow and cycles without modifying the graph.
    pub fn set(
        &mut self,
        asset: &AssetId,
        dependencies: impl IntoIterator<Item = AssetId>,
    ) -> Result<(), DependencyError> {
        let old = self
            .edges
            .get(asset)
            .ok_or_else(|| DependencyError::UnknownAsset(asset.clone()))?;
        let dependencies: BTreeSet<_> = dependencies.into_iter().collect();
        let total: usize = self.edges.values().map(BTreeSet::len).sum();
        if total - old.len() + dependencies.len() > self.max_edges {
            return Err(DependencyError::Capacity);
        }
        let mut pending: Vec<_> = dependencies.iter().cloned().collect();
        let mut visited = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if &current == asset {
                return Err(DependencyError::Cycle);
            }
            let next = self
                .edges
                .get(&current)
                .ok_or_else(|| DependencyError::UnknownAsset(current.clone()))?;
            if visited.insert(current) {
                pending.extend(next.iter().cloned());
            }
        }
        self.edges.insert(asset.clone(), dependencies);
        Ok(())
    }
    /// Direct compiled dependencies for preparing immutable worker snapshots.
    #[must_use]
    pub fn dependencies(&self, asset: &AssetId) -> Option<&BTreeSet<AssetId>> {
        self.edges.get(asset)
    }
    /// Finds changed resources and every transitive dependent. Orders dependencies
    /// before dependents, using lexical ID order for otherwise independent nodes.
    /// This computes a rebuild plan; the caller dispatches actual imports.
    /// # Errors
    /// Rejects undeclared changed IDs and internal graph cycles.
    pub fn affected(
        &self,
        changed: impl IntoIterator<Item = AssetId>,
    ) -> Result<Vec<AssetId>, DependencyError> {
        let mut affected: BTreeSet<_> = changed.into_iter().collect();
        for asset in &affected {
            if !self.edges.contains_key(asset) {
                return Err(DependencyError::UnknownAsset(asset.clone()));
            }
        }
        loop {
            let additions: Vec<_> = self
                .edges
                .iter()
                .filter(|(asset, deps)| {
                    !affected.contains(*asset) && deps.iter().any(|dep| affected.contains(dep))
                })
                .map(|(asset, _)| asset.clone())
                .collect();
            if additions.is_empty() {
                break;
            }
            affected.extend(additions);
        }
        let mut result = Vec::with_capacity(affected.len());
        let mut completed = BTreeSet::new();
        while !affected.is_empty() {
            let ready: Vec<_> = affected
                .iter()
                .filter(|asset| {
                    self.edges[*asset]
                        .iter()
                        .all(|dep| !affected.contains(dep) || completed.contains(dep))
                })
                .cloned()
                .collect();
            if ready.is_empty() {
                return Err(DependencyError::Cycle);
            }
            for asset in ready {
                affected.remove(&asset);
                completed.insert(asset.clone());
                result.push(asset);
            }
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn id(name: &str) -> AssetId {
        AssetId(name.into())
    }
    #[test]
    fn diamond_rebuilds_each_dependent_once_in_dependency_order() {
        let mut graph = AssetDependencies::new(5, 8);
        for asset in ["texture", "material", "mesh", "scene", "other"] {
            graph.declare(id(asset)).unwrap();
        }
        graph.set(&id("material"), [id("texture")]).unwrap();
        graph.set(&id("mesh"), [id("texture")]).unwrap();
        graph
            .set(&id("scene"), [id("material"), id("mesh")])
            .unwrap();
        assert_eq!(
            graph.affected([id("texture")]).unwrap(),
            vec![id("texture"), id("material"), id("mesh"), id("scene")]
        );
        assert_eq!(
            graph.affected([id("mesh")]).unwrap(),
            vec![id("mesh"), id("scene")]
        );
    }
    #[test]
    fn failed_edits_preserve_graph_and_limits() {
        let mut graph = AssetDependencies::new(2, 1);
        graph.declare(id("a")).unwrap();
        graph.declare(id("b")).unwrap();
        graph.set(&id("b"), [id("a")]).unwrap();
        assert_eq!(
            graph.set(&id("a"), [id("b")]),
            Err(DependencyError::Capacity)
        );
        assert_eq!(graph.set(&id("b"), [id("b")]), Err(DependencyError::Cycle));
        assert!(graph.set(&id("b"), [id("missing")]).is_err());
        assert_eq!(graph.affected([id("a")]).unwrap(), vec![id("a"), id("b")]);
        assert_eq!(graph.declare(id("c")), Err(DependencyError::Capacity));
    }
    #[test]
    fn indirect_cycle_is_rejected_without_losing_existing_dependencies() {
        let mut graph = AssetDependencies::new(3, 4);
        for name in ["a", "b", "c"] {
            graph.declare(id(name)).unwrap();
        }
        graph.set(&id("b"), [id("a")]).unwrap();
        graph.set(&id("c"), [id("b")]).unwrap();
        assert_eq!(graph.set(&id("a"), [id("c")]), Err(DependencyError::Cycle));
        assert_eq!(
            graph.affected([id("a")]).unwrap(),
            vec![id("a"), id("b"), id("c")]
        );
    }
}
