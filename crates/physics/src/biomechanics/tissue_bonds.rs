//! Objective discrete tissue attachments; not continuum bonding or contact.
use super::{Body, Vec3, add, dot, scale, sub};
#[derive(Clone, Copy, Debug)]
pub struct TissueBond {
    pub nodes: [usize; 2],
    pub stiffness_n_m: f64,
    pub reference_length_m: f64,
}
#[derive(Clone, Debug)]
pub struct TissueAssembly {
    pub body: Body,
    pub node_ranges: Vec<std::ops::Range<usize>>,
    pub cell_ranges: Vec<std::ops::Range<usize>>,
}
impl Body {
    /// Combine tissues in their existing world coordinates and retain cell laws,
    /// states, cavities, dead loads and supports. Regions retain original identities.
    /// # Errors
    /// Empty input, uniform-pressure storage, mixed dry/cell-stored parts,
    /// installed embedded skin contact requiring rebinding, or invalid state.
    /// Viscous branch memories and trial time are retained by cloning whole elements.
    pub fn assemble_tissues(parts: &[Body]) -> Result<TissueAssembly, &'static str> {
        if parts.iter().any(|p| p.embedded_contact.is_some()) {
            return Err("tissue assembly requires rebinding embedded skin contact");
        }
        let cell_stored = parts.iter().any(|p| !p.cell_pore_fluids.is_empty());
        if parts.is_empty()
            || parts.iter().any(|p| {
                p.pore_fluid.is_some()
                    || (cell_stored && p.cell_pore_fluids.len() != p.elements.len())
            })
        {
            return Err("unsupported tissue assembly state");
        }
        let law = parts[0].surface_contact_law;
        if parts.iter().any(|p| p.surface_contact_law != law) {
            return Err("mixed surface contact laws in tissue assembly");
        }
        let mut rest = Vec::new();
        let mut pins = Vec::new();
        let mut cells = Vec::new();
        let mut ranges = Vec::new();
        let mut cell_ranges = Vec::new();
        for part in parts {
            let offset = rest.len();
            cell_ranges.push(cells.len()..cells.len() + part.elements.len());
            ranges.push(offset..offset + part.rest.len());
            rest.extend_from_slice(&part.rest);
            pins.extend_from_slice(&part.pinned);
            for e in &part.elements {
                cells.push((e.nodes.map(|i| i + offset), e.material.clone()));
            }
        }
        let mut body = Body::new(rest, pins, cells)?;
        body.elements.clear();
        body.cavities.clear();
        body.tissue_bonds.clear();
        body.tissue_gaps.clear();
        body.surface_contacts.clear();
        body.surface_contact_law = law;
        body.positions.clear();
        body.forces.clear();
        body.diagonal.clear();
        body.cell_pore_fluids.clear();
        for (part, range) in parts.iter().zip(&ranges) {
            let offset = range.start;
            body.positions.extend_from_slice(&part.positions);
            body.forces.extend_from_slice(&part.forces);
            body.diagonal.extend_from_slice(&part.diagonal);
            body.cell_pore_fluids
                .extend_from_slice(&part.cell_pore_fluids);
            for e in &part.elements {
                let mut e = e.clone();
                e.nodes = e.nodes.map(|i| i + offset);
                body.elements.push(e);
            }
            for c in &part.cavities {
                let mut c = c.clone();
                c.faces = c.faces.iter().map(|f| f.map(|i| i + offset)).collect();
                body.cavities.push(c);
            }
            for contact in &part.surface_contacts {
                let mut contact = contact.clone();
                contact.faces = contact
                    .faces
                    .iter()
                    .map(|face| face.map(|i| i + offset))
                    .collect();
                body.surface_contacts.push(contact);
            }
            for gap in &part.tissue_gaps {
                let mut gap = *gap;
                gap.nodes = gap.nodes.map(|i| i + offset);
                body.tissue_gaps.push(gap);
            }
            for b in &part.tissue_bonds {
                let mut b = *b;
                b.nodes = b.nodes.map(|i| i + offset);
                body.tissue_bonds.push(b);
            }
        }
        body.evaluate(&body.positions)?;
        body.pore_preconditioner()?;
        Ok(TissueAssembly {
            body,
            node_ranges: ranges,
            cell_ranges,
        })
    }
    /// Install a batch of rest-length springs transactionally. Reference lengths
    /// come from reference geometry; no artificial stress-free rebasing is performed.
    /// # Errors
    /// Invalid/duplicate pair, collapsed reference/current bond or overflow.
    pub fn add_tissue_bonds(&mut self, pairs: &[([usize; 2], f64)]) -> Result<(), &'static str> {
        let mut next = self.clone();
        let mut seen: std::collections::BTreeSet<_> = next
            .tissue_bonds
            .iter()
            .map(|b| (b.nodes[0].min(b.nodes[1]), b.nodes[0].max(b.nodes[1])))
            .collect();
        for &(nodes, k) in pairs {
            if nodes.iter().any(|&i| i >= self.rest.len())
                || nodes[0] == nodes[1]
                || !k.is_finite()
                || k <= 0.
                || !seen.insert((nodes[0].min(nodes[1]), nodes[0].max(nodes[1])))
            {
                return Err("invalid or duplicate tissue bond");
            }
            let d = sub(self.rest[nodes[1]], self.rest[nodes[0]]);
            let length = dot(d, d).sqrt();
            if !length.is_finite() || length <= 1e-12 {
                return Err("collapsed tissue bond reference");
            }
            next.tissue_bonds.push(TissueBond {
                nodes,
                stiffness_n_m: k,
                reference_length_m: length,
            });
        }
        next.evaluate(&next.positions)?;
        next.pore_preconditioner()?;
        *self = next;
        Ok(())
    }
    #[must_use]
    pub fn tissue_bonds(&self) -> &[TissueBond] {
        &self.tissue_bonds
    }
    pub(super) fn bond_energy_gradient(
        &self,
        positions: &[Vec3],
        gradient: &mut [Vec3],
    ) -> Result<f64, &'static str> {
        let mut energy = 0.;
        for b in &self.tissue_bonds {
            let [i, j] = b.nodes;
            let d = sub(positions[j], positions[i]);
            let length = dot(d, d).sqrt();
            if !length.is_finite() || length <= 1e-12 {
                return Err("collapsed tissue attachment");
            }
            let extension = length - b.reference_length_m;
            energy += 0.5 * b.stiffness_n_m * extension * extension;
            let force = scale(d, b.stiffness_n_m * extension / length);
            gradient[i] = sub(gradient[i], force);
            gradient[j] = add(gradient[j], force);
        }
        Ok(energy)
    }
}
impl super::ClitoralComplex {
    /// Release independent glans/bulb supports and attach their basal caps to the
    /// corpus/crus meshes through explicit discrete springs. Only crus roots stay fixed.
    /// # Errors
    /// Invalid spring stiffness/neighbor count, invalid assembly or failed attachment.
    pub fn coupled(
        &self,
        stiffness_n_m: f64,
        neighbors: usize,
    ) -> Result<TissueAssembly, &'static str> {
        if !stiffness_n_m.is_finite() || stiffness_n_m <= 0. || !(3..=8).contains(&neighbors) {
            return Err("invalid clitoral attachment controls");
        }
        let mut parts = vec![
            self.corpora_crura[0].clone(),
            self.corpora_crura[1].clone(),
            self.glans.clone(),
            self.vestibular_bulbs[0].clone(),
            self.vestibular_bulbs[1].clone(),
        ];
        let roots: Vec<Vec<usize>> = parts
            .iter()
            .skip(2)
            .map(|p| {
                p.pinned
                    .iter()
                    .enumerate()
                    .filter_map(|(i, fixed)| fixed.then_some(i))
                    .collect()
            })
            .collect();
        for p in parts.iter_mut().skip(2) {
            p.pinned.fill(false);
        }
        let mut assembly = Body::assemble_tissues(&parts)?;
        let corpus_surface: std::collections::BTreeSet<_> = parts[..2]
            .iter()
            .zip(&assembly.node_ranges[..2])
            .flat_map(|(p, range)| {
                p.surface()
                    .into_iter()
                    .flatten()
                    .map(move |node| node + range.start)
            })
            .collect();
        
        let mut bonds = Vec::new();
        // === OPTIMIZATION #18: Quickselect вместо полного сортирования O(n log n) → O(n) ===
        
        for (part, roots) in roots.iter().enumerate() {
            for &root in roots {
                let node = assembly.node_ranges[part + 2].start + root;
                let p = assembly.body.rest[node];
                
                // Compute all distances first
                // === OPTIMIZATION #18: Quickselect O(n) вместо sort O(n log n) ===
let mut candidates: Vec<(f64, usize)> = corpus_surface
                    .iter()
                    .copied()
                    .filter_map(|i| {
                        let d = sub(p, assembly.body.rest[i]);
                        let distance = dot(d, d);
                        (distance > 1e-24).then_some((distance, i))
                    })
                    .collect();
                
                // Early termination: if we have fewer than neighbors candidates, fail fast
                if candidates.len() < neighbors {
                    return Err("insufficient corpus attachment nodes");
                }
                
                // Use selection algorithm instead of full sort for top-k nearest neighbors
                // Quickselect/partition is O(n) average vs O(n log n) for full sort
                if candidates.len() > neighbors {
                    // Partition so that first 'neighbors' are the smallest distances
                    candidates.select_nth_unstable_by::<_, Ordering>(|a: &(f64, usize), b: &(f64, usize)| a.0.total_cmp(&b.0));
                }
                
                // Take the k nearest (already at front after select_nth)
                for &(target_dist, target) in candidates.iter().take(neighbors) {
                    // Skip if too far (distance cutoff optimization)
                    if target_dist > 0.1 {
                        continue;
                    }
                    bonds.push(([node, target], stiffness_n_m / neighbors as f64));
                }
            }
        }
        
        assembly.body.add_tissue_bonds(&bonds)?;
        Ok(assembly)
    }
}
