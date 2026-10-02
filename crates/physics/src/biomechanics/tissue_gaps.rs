//! Selected-pair frictionless distance barriers, not complete surface collision.
use super::{Body, Vec3, add, dot, scale, sub};
#[derive(Clone, Copy, Debug)]
pub struct TissueGap {
    pub nodes: [usize; 2],
    pub minimum_distance_m: f64,
    pub activation_gap_m: f64,
    pub stiffness_n_m: f64,
}
impl Body {
    /// Install selected pair barriers atomically. Gap is distance minus clearance.
    /// # Errors
    /// Invalid/duplicate pairs, nonpositive gap, invalid coefficients or overflow.
    pub fn add_tissue_gaps(&mut self, gaps: &[TissueGap]) -> Result<(), &'static str> {
        let mut next = self.clone();
        let mut seen: std::collections::BTreeSet<_> = next
            .tissue_gaps
            .iter()
            .map(|g| (g.nodes[0].min(g.nodes[1]), g.nodes[0].max(g.nodes[1])))
            .collect();
        for gap in gaps {
            if gap.nodes.iter().any(|&i| i >= self.rest.len())
                || gap.nodes[0] == gap.nodes[1]
                || [
                    gap.minimum_distance_m,
                    gap.activation_gap_m,
                    gap.stiffness_n_m,
                ]
                .iter()
                .any(|x| !x.is_finite() || *x <= 0.)
                || !seen.insert((
                    gap.nodes[0].min(gap.nodes[1]),
                    gap.nodes[0].max(gap.nodes[1]),
                ))
            {
                return Err("invalid or duplicate tissue gap");
            }
            next.tissue_gaps.push(*gap);
        }
        next.evaluate(&next.positions)?;
        next.pore_preconditioner()?;
        *self = next;
        Ok(())
    }
    #[must_use]
    pub fn tissue_gaps(&self) -> &[TissueGap] {
        &self.tissue_gaps
    }
    pub(super) fn gap_energy_gradient(
        &self,
        positions: &[Vec3],
        gradient: &mut [Vec3],
    ) -> Result<f64, &'static str> {
        let mut energy = 0.;
        for g in &self.tissue_gaps {
            let [i, j] = g.nodes;
            let delta = sub(positions[j], positions[i]);
            let distance = dot(delta, delta).sqrt();
            let gap = distance - g.minimum_distance_m;
            if !gap.is_finite() || gap <= 0. {
                return Err("closed tissue gap");
            }
            if gap >= g.activation_gap_m {
                continue;
            }
            let offset = gap - g.activation_gap_m;
            let logarithm = if gap < 0.5 * g.activation_gap_m {
                (gap / g.activation_gap_m).ln()
            } else {
                (offset / g.activation_gap_m).ln_1p()
            };
            energy -= g.stiffness_n_m * offset * offset * logarithm;
            let derivative = -g.stiffness_n_m * (2. * offset * logarithm + offset * offset / gap);
            let v = scale(delta, derivative / distance);
            gradient[i] = sub(gradient[i], v);
            gradient[j] = add(gradient[j], v);
        }
        Ok(energy)
    }
    /// Exact closest approach of each selected pair along a straight nodal step.
    /// This guards the discrete integration/line-search path, not arbitrary surfaces.
    pub(super) fn gap_path_is_open(&self, start: &[Vec3], end: &[Vec3]) -> bool {
        self.surface_path_is_open(start, end)
            && self.tissue_gaps.iter().all(|g| {
                let [i, j] = g.nodes;
                let a = sub(start[j], start[i]);
                let b = sub(sub(end[j], end[i]), a);
                let bb = dot(b, b);
                let t = if bb > 0. {
                    (-dot(a, b) / bb).clamp(0., 1.)
                } else {
                    0.
                };
                let closest = add(a, scale(b, t));
                let squared = dot(closest, closest);
                squared.is_finite() && squared > g.minimum_distance_m * g.minimum_distance_m
            })
    }
}
