//! Intrinsic cohesive discretization: every interior tetrahedral face may break.
use super::{Body, Material, Vec3, cross, dot, sub};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct CohesiveMesh {
    pub body: Body,
    /// Original mesh vertex for each duplicated solver vertex. Replicate boundary
    /// constraints, but partition original nodal loads among copies to retain force.
    pub source_nodes: Vec<usize>,
    /// Solver vertices per source cell, preserving source cell and local order.
    pub cell_nodes: Vec<[usize; 4]>,
    source_node_count: usize,
}
impl CohesiveMesh {
    fn copy_counts(&self) -> Result<Vec<u32>, &'static str> {
        if self.source_nodes.len() != self.body.rest.len() {
            return Err("invalid cohesive source mapping length");
        }
        let mut counts = vec![0_u32; self.source_node_count];
        for &source in &self.source_nodes {
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
impl Body {
    /// Duplicate cell vertices and bond every two-sided interior face. The finite
    /// cohesive stiffness adds compliance even before damage. Crack paths follow
    /// mesh faces; this is intrinsic insertion, not arbitrary within-cell cracking.
    /// # Errors
    /// Invalid source mesh, >32 cells (128 duplicated nodes), nonmanifold faces,
    /// overlapping face neighbors, or invalid cohesive geometry.
    pub fn with_cohesive_faces(
        rest: &[Vec3],
        cells: &[([usize; 4], Material)],
        cohesive: crate::cohesive::Material,
    ) -> Result<CohesiveMesh, &'static str> {
        if cells.len() > 32 {
            return Err("cohesive mesh exceeds duplicated vertex limit");
        }
        // Validate source connectivity and geometry before expanding its topology.
        Self::new(rest.to_vec(), cells.to_vec())?;
        let mut source_nodes = Vec::with_capacity(4 * cells.len());
        let mut cell_nodes = Vec::with_capacity(cells.len());
        let mut faces = BTreeMap::<[usize; 3], Vec<(usize, usize)>>::new();
        for (cell, (nodes, _)) in cells.iter().enumerate() {
            cell_nodes.push(std::array::from_fn(|local| 4 * cell + local));
            source_nodes.extend(nodes);
            for opposite in 0..4 {
                let mut face = [0; 3];
                let mut count = 0;
                for (local, &node) in nodes.iter().enumerate() {
                    if local != opposite {
                        face[count] = node;
                        count += 1;
                    }
                }
                face.sort_unstable();
                let owners = faces.entry(face).or_default();
                owners.push((cell, opposite));
                if owners.len() > 2 {
                    return Err("nonmanifold cohesive source face");
                }
            }
        }
        let expanded = source_nodes.iter().map(|&node| rest[node]).collect();
        let expanded_cells = cell_nodes
            .iter()
            .zip(cells)
            .map(|(&nodes, (_, material))| (nodes, *material))
            .collect();
        let mut body = Self::new(expanded, expanded_cells)?;
        for (mut face, owners) in faces {
            if owners.len() != 2 {
                continue;
            }
            let (minus_cell, opposite) = owners[0];
            let (plus_cell, _) = owners[1];
            let [a, b, c] = face.map(|node| rest[node]);
            let edges = [
                sub(b, a),
                sub(c, a),
                sub(rest[cells[minus_cell].0[opposite]], a),
            ];
            let scale = edges.iter().flatten().fold(0_f64, |m, v| m.max(v.abs()));
            let scaled = edges.map(|v| v.map(|x| x / scale));
            if dot(cross(scaled[0], scaled[1]), scaled[2]) > 0. {
                face.swap(1, 2);
            }
            let mapped = |cell: usize| -> Result<[usize; 3], &'static str> {
                let mut result = [0; 3];
                for (i, node) in face.iter().enumerate() {
                    let local = cells[cell]
                        .0
                        .iter()
                        .position(|n| n == node)
                        .ok_or("missing cohesive source face node")?;
                    result[i] = cell_nodes[cell][local];
                }
                Ok(result)
            };
            body.add_interface(mapped(minus_cell)?, mapped(plus_cell)?, cohesive)?;
        }
        Ok(CohesiveMesh {
            body,
            source_nodes,
            cell_nodes,
            source_node_count: rest.len(),
        })
    }
}
