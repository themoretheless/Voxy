//! Validated mapping from duplicated solver nodes to source loads and constraints.
use super::Vec3;

#[derive(Debug)]
pub(super) struct SourceNodeMapping<'a> {
    pub source_nodes: &'a [usize],
    pub source_node_count: usize,
    pub solver_node_count: usize,
}
impl SourceNodeMapping<'_> {
    pub(super) fn copy_counts(&self) -> Result<Vec<u32>, &'static str> {
        if self.source_nodes.len() != self.solver_node_count {
            return Err("invalid cohesive source mapping length");
        }
        let mut counts = vec![0_u32; self.source_node_count];
        for &source in self.source_nodes {
            let count = counts
                .get_mut(source)
                .ok_or("invalid cohesive source node")?;
            *count += 1;
        }
        if counts.contains(&0) {
            return Err("missing cohesive source node");
        }
        Ok(counts)
    }
    /// Split each source nodal force equally among its coincident solver copies.
    /// Preserves reference force and moment. For face tractions or heterogeneous
    /// body forces, integrate loads per cell instead; uniform acceleration is
    /// already mass-weighted by the dynamic solver.
    /// # Errors
    /// Invalid mapping, source count, or nonfinite forces.
    pub fn split_nodal_forces(&self, forces: &[Vec3]) -> Result<Vec<Vec3>, &'static str> {
        let counts = self.copy_counts()?;
        if forces.len() != counts.len() || forces.iter().flatten().any(|v| !v.is_finite()) {
            return Err("invalid source nodal forces");
        }
        Ok(self
            .source_nodes
            .iter()
            .map(|&source| forces[source].map(|f| f / f64::from(counts[source])))
            .collect())
    }
    /// Replicate source prescribed displacements onto every solver copy.
    /// # Errors
    /// Invalid mapping, source count, or nonfinite prescribed displacement.
    pub fn expand_constraints(
        &self,
        prescribed: &[[Option<f64>; 3]],
    ) -> Result<Vec<[Option<f64>; 3]>, &'static str> {
        let counts = self.copy_counts()?;
        if prescribed.len() != counts.len()
            || prescribed
                .iter()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err("invalid source constraints");
        }
        Ok(self
            .source_nodes
            .iter()
            .map(|&source| prescribed[source])
            .collect())
    }
}
