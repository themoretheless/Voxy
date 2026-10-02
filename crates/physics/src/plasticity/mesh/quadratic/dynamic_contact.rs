//! Explicit selected moving-surface pairs; exposure follows trial crack damage.
use super::{FiniteQuadraticDynamics, QuadraticDynamics};
use crate::plasticity::mesh::{
    QuadraticFace, QuadraticSurfaceContact, QuadraticSurfaceContactEvaluation,
};
#[derive(Clone, Debug)]
pub(super) struct SurfaceContact {
    law: QuadraticSurfaceContact,
    pairs: Vec<(QuadraticFace, QuadraticFace)>,
    automatic: bool,
}
fn key(mut nodes: [usize; 6]) -> [usize; 6] {
    nodes.sort_unstable();
    nodes
}
impl QuadraticDynamics {
    /// Install selected boundary-face pairs or remove the surface law. Returns
    /// parameter work (change of contact potential at fixed accepted geometry).
    /// Only exposed sides interact; fully bonded internal faces are inactive.
    /// Pair node sets select authoritative reference faces and areas.
    /// # Errors
    /// Invalid/repeated/shared-node pairs, >4096 pairs, missing reference boundary,
    /// unresolved contact geometry or overflow. Accepted state is unchanged.
    pub fn set_surface_contact(
        &mut self,
        law: Option<QuadraticSurfaceContact>,
        pairs: &[([usize; 6], [usize; 6])],
    ) -> Result<f64, &'static str> {
        let mut candidate = self.clone();
        if let Some(law) = law {
            if pairs.len() > 4096 {
                return Err("quadratic contact pair limit");
            }
            let faces = self.body.reference_faces()?;
            let mut selected = Vec::new();
            let mut keys = std::collections::BTreeSet::new();
            for &(a, b) in pairs {
                let a = key(a);
                let b = key(b);
                let pair = if a < b { (a, b) } else { (b, a) };
                if a.iter().any(|n| b.contains(n)) || !keys.insert(pair) {
                    return Err("invalid repeated quadratic contact pair");
                }
                let first = *faces
                    .iter()
                    .find(|f| key(f.nodes) == a)
                    .ok_or("missing quadratic contact boundary")?;
                let second = *faces
                    .iter()
                    .find(|f| key(f.nodes) == b)
                    .ok_or("missing quadratic contact boundary")?;
                selected.push((first, second));
            }
            candidate.surface_contact = Some(SurfaceContact {
                law,
                pairs: selected,
                automatic: false,
            });
        } else {
            if !pairs.is_empty() {
                return Err("surface contact pairs require a law");
            }
            candidate.surface_contact = None;
        }
        let change = candidate.energy()?.surface_contact_j - self.energy()?.surface_contact_j;
        if !change.is_finite() {
            return Err("quadratic surface parameter work overflow");
        }
        *self = candidate;
        Ok(change)
    }
    /// Enable nearest-surface contact between distinct trial fracture components.
    /// Exposure and component membership are recomputed at each force evaluation.
    /// No same-component self-contact is inserted. Returns parameter work.
    /// # Errors
    /// Unresolved geometry or overflow; accepted state and previous law are retained.
    pub fn set_automatic_surface_contact(
        &mut self,
        law: Option<QuadraticSurfaceContact>,
    ) -> Result<f64, &'static str> {
        let mut candidate = self.clone();
        candidate.surface_contact = law.map(|law| SurfaceContact {
            law,
            pairs: Vec::new(),
            automatic: true,
        });
        let work = candidate.energy()?.surface_contact_j - self.energy()?.surface_contact_j;
        if !work.is_finite() {
            return Err("quadratic surface parameter work overflow");
        }
        *self = candidate;
        Ok(work)
    }
    // T6 sum of absolute shape weights is <=7/4: each negative corner
    // contribution has magnitude <=1/8 and all six weights sum to one.
    // Subtract a common moving anchor so uniform translation does not reduce dt.
    pub(super) fn check_surface_motion(&self, next: &Self) -> Result<(), &'static str> {
        let Some(contact) = &self.surface_contact else {
            return Ok(());
        };
        let pairs: Vec<(Vec<usize>, Vec<usize>)> = if contact.automatic {
            let mut trial = next.body.clone();
            trial.interfaces = next
                .body
                .cohesive_trials_at(&next.body.positions)?
                .into_iter()
                .map(|t| t.candidate)
                .collect();
            let fragments = trial.fragment_nodes();
            let mut pairs = Vec::new();
            for i in 0..fragments.len() {
                for j in i + 1..fragments.len() {
                    pairs.push((fragments[i].clone(), fragments[j].clone()));
                }
            }
            pairs
        } else {
            contact
                .pairs
                .iter()
                .map(|(a, b)| (a.nodes.to_vec(), b.nodes.to_vec()))
                .collect()
        };
        let displacement = |node: usize| -> [f64; 3] {
            std::array::from_fn(|axis| {
                next.body.positions[node][axis] - self.body.positions[node][axis]
            })
        };
        for (first, second) in pairs {
            let anchor = displacement(first[0]);
            let bound = |nodes: &[usize]| -> f64 {
                nodes
                    .iter()
                    .map(|&node| {
                        let movement = displacement(node);
                        let relative: [f64; 3] = std::array::from_fn(|i| movement[i] - anchor[i]);
                        relative[0].hypot(relative[1]).hypot(relative[2])
                    })
                    .fold(0_f64, f64::max)
            };
            let relative_motion = 1.75 * (bound(&first) + bound(&second));
            if !relative_motion.is_finite() || relative_motion > 0.25 * contact.law.thickness_m() {
                return Err("quadratic surface motion limit reached");
            }
        }
        Ok(())
    }
    pub(super) fn check_surface_sweep(&self, next: &Self) -> Result<(), &'static str> {
        let Some(contact) = &self.surface_contact else {
            return Ok(());
        };
        let Some((clearance, limits)) = contact.law.sweep_guard() else {
            return Ok(());
        };
        let faces = self.body.exposed_faces_at(&self.body.positions)?;
        let mut surfaces = Vec::new();
        if contact.automatic {
            let mut trial = next.body.clone();
            trial.interfaces = next
                .body
                .cohesive_trials_at(&next.body.positions)?
                .into_iter()
                .map(|t| t.candidate)
                .collect();
            let fragments = trial.fragment_nodes();
            let mut owner = vec![0; self.body.positions.len()];
            for (i, nodes) in fragments.iter().enumerate() {
                for &node in nodes {
                    owner[node] = i;
                }
            }
            let mut groups = vec![Vec::new(); fragments.len()];
            for face in faces {
                groups[owner[face.nodes[0]]].push(face);
            }
            for i in 0..groups.len() {
                for j in i + 1..groups.len() {
                    if !groups[i].is_empty() && !groups[j].is_empty() {
                        surfaces.push((groups[i].clone(), groups[j].clone()));
                    }
                }
            }
        } else {
            let exposed: std::collections::BTreeSet<_> =
                faces.iter().map(|f| key(f.nodes)).collect();
            for &(a, b) in &contact.pairs {
                if exposed.contains(&key(a.nodes)) && exposed.contains(&key(b.nodes)) {
                    surfaces.push((vec![a], vec![b]));
                }
            }
        }
        for (first, second) in surfaces {
            for (sources, targets) in [(&first, &second), (&second, &first)] {
                let nodes: std::collections::BTreeSet<_> =
                    sources.iter().flat_map(|f| f.nodes).collect();
                for node in nodes {
                    for target in targets {
                        match target.swept_point_clearance_at(
                            &self.body.positions,
                            &next.body.positions,
                            self.body.positions[node],
                            next.body.positions[node],
                            clearance,
                            limits,
                        )? {
                            crate::plasticity::mesh::QuadraticSweep::Separated { .. } => {}
                            crate::plasticity::mesh::QuadraticSweep::WithinClearance { .. } => {
                                return Err("quadratic surface sweep clearance reached");
                            }
                            crate::plasticity::mesh::QuadraticSweep::Unresolved { .. } => {
                                return Err("quadratic surface sweep unresolved");
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn surface_contact_evaluation(
        &self,
    ) -> Result<Option<QuadraticSurfaceContactEvaluation>, &'static str> {
        let Some(contact) = &self.surface_contact else {
            return Ok(None);
        };
        let faces = self.body.exposed_faces_at(&self.body.positions)?;
        let mut surfaces = Vec::new();
        if contact.automatic {
            let mut trial = self.body.clone();
            trial.interfaces = self
                .body
                .cohesive_trials_at(&self.body.positions)?
                .into_iter()
                .map(|t| t.candidate)
                .collect();
            let fragments = trial.fragment_nodes();
            let mut owner = vec![0; self.body.positions.len()];
            for (component, nodes) in fragments.iter().enumerate() {
                for &node in nodes {
                    owner[node] = component;
                }
            }
            let mut grouped = vec![Vec::new(); fragments.len()];
            for face in faces {
                grouped[owner[face.nodes[0]]].push(face);
            }
            for i in 0..grouped.len() {
                for j in i + 1..grouped.len() {
                    if !grouped[i].is_empty() && !grouped[j].is_empty() {
                        surfaces.push((grouped[i].clone(), grouped[j].clone()));
                    }
                }
            }
        } else {
            let exposed: std::collections::BTreeSet<_> =
                faces.iter().map(|f| key(f.nodes)).collect();
            for &(first, second) in &contact.pairs {
                if exposed.contains(&key(first.nodes)) && exposed.contains(&key(second.nodes)) {
                    surfaces.push((vec![first], vec![second]));
                }
            }
        }
        let mut total = QuadraticSurfaceContactEvaluation {
            energy_j: 0.,
            forces_n: vec![[0.; 3]; self.body.positions.len()],
            search_energy_error_bound_j: 0.,
            active_samples: 0,
        };
        for (first, second) in surfaces {
            let response = contact
                .law
                .evaluate_surfaces(&self.body.positions, &first, &second)?;
            total.energy_j += response.energy_j;
            total.search_energy_error_bound_j += response.search_energy_error_bound_j;
            total.active_samples += response.active_samples;
            for (total, force) in total.forces_n.iter_mut().zip(response.forces_n) {
                for axis in 0..3 {
                    total[axis] += force[axis];
                }
            }
        }
        if !total.energy_j.is_finite()
            || !total.search_energy_error_bound_j.is_finite()
            || total.forces_n.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("quadratic surface pair assembly overflow");
        }
        Ok(Some(total))
    }
}
impl FiniteQuadraticDynamics {
    /// Enable/remove automatic contact between distinct trial fragments.
    /// # Errors
    /// Unresolved surface contact or overflow; accepted state is unchanged.
    pub fn set_automatic_surface_contact(
        &mut self,
        law: Option<QuadraticSurfaceContact>,
    ) -> Result<f64, &'static str> {
        self.inner.set_automatic_surface_contact(law)
    }

    /// Install/remove selected exposed surface pairs, returning fixed-geometry
    /// parameter work. Exposure follows cohesive trial damage.
    /// # Errors
    /// Invalid pairs or contact evaluation; accepted state is unchanged.
    pub fn set_surface_contact(
        &mut self,
        law: Option<QuadraticSurfaceContact>,
        pairs: &[([usize; 6], [usize; 6])],
    ) -> Result<f64, &'static str> {
        self.inner.set_surface_contact(law, pairs)
    }
}
