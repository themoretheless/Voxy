//! Native dyadic planning and sampled error policy for experimental GGX patches.
//! Sampled success is not a bound over a whole cell or the full material domain.
use super::{GgxEnergyPatchData, GgxEnergyPatchError};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, PartialEq)]
pub enum GgxEnergyRefinementError {
    InvalidRegion,
    OverlappingRegions,
    InvalidLevel,
    UnrepresentableStencil,
    DuplicateProbe,
    InvalidMomentCount,
    ConflictingStencil,
    EmptyObservations,
    ObservationMismatch(usize),
    InvalidOracle(usize),
    InvalidGpu(usize),
    InvalidPolicy,
    Patch(GgxEnergyPatchError),
}
impl std::fmt::Display for GgxEnergyRefinementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "experimental GGX refinement: {self:?}")
    }
}
impl std::error::Error for GgxEnergyRefinementError {}
#[derive(Debug, Clone)]
pub struct GgxEnergyRefinementProbe {
    pub uv: [f32; 2],
    pub bounds: [f32; 4],
    pub view_nodes: [f32; 6],
    pub roughness_nodes: [f32; 6],
    pub node_indices: [usize; 36],
}
#[derive(Debug, Clone)]
pub struct GgxEnergyRefinementPlan {
    refinement: u32,
    nodes: Vec<[f32; 2]>,
    probes: Vec<GgxEnergyRefinementProbe>,
}
fn valid_level(level: u32) -> bool {
    matches!(level, 1 | 2 | 4 | 8)
}
fn axis(
    value: f32,
    level: u32,
    positive: bool,
) -> Result<([f32; 6], f64), GgxEnergyRefinementError> {
    if !value.is_finite() || !(0. ..=1.).contains(&value) || !valid_level(level) {
        return Err(GgxEnergyRefinementError::InvalidRegion);
    }
    let v = f64::from(value);
    let distance = if v == 0. || v == 1. {
        1.
    } else {
        v.min(1. - v)
    };
    // Exact binary exponent, rather than a rounded logarithm near powers of two.
    let exponent = ((distance.to_bits() >> 52) & 0x7ff) as i32 - 1023;
    let step = (2f64.powi(exponent) / 8.).min(1. / 64.) / f64::from(level);
    let origin =
        ((v / step).floor() - 2.).clamp(if positive { 1. } else { 0. }, (1. / step).round() - 5.);
    let nodes = std::array::from_fn(|i| ((origin + i as f64) * step) as f32);
    for i in 0..6 {
        if !nodes[i].is_finite()
            || !(0. ..=1.).contains(&nodes[i])
            || (i > 0 && nodes[i] <= nodes[i - 1])
            || (positive && (nodes[i] * nodes[i]) * (nodes[i] * nodes[i]) <= 0.)
        {
            return Err(GgxEnergyRefinementError::UnrepresentableStencil);
        }
    }
    Ok((nodes, step))
}
fn cell(value: f32, step: f64) -> [f32; 2] {
    let left = (f64::from(value) / step)
        .floor()
        .mul_add(step, 0.)
        .min(1. - step);
    [left as f32, (left + step) as f32]
}
impl GgxEnergyRefinementPlan {
    /// Seed positive-roughness physical cells. The exact zero-roughness boundary
    /// is handled analytically by atlas lookup, not by these sampled patches.
    pub fn regions_from_seeds(
        seeds: &[[f32; 2]],
    ) -> Result<Vec<[f32; 4]>, GgxEnergyRefinementError> {
        if seeds.is_empty() {
            return Err(GgxEnergyRefinementError::InvalidRegion);
        }
        seeds
            .iter()
            .map(|&[nv, r]| {
                if !(r > 0.) {
                    return Err(GgxEnergyRefinementError::InvalidRegion);
                }
                let (_, dx) = axis(nv, 1, false)?;
                let (_, dy) = axis(r, 1, true)?;
                let x = cell(nv, dx);
                let y = cell(r, dy);
                Ok([x[0], y[0], x[1], y[1]])
            })
            .collect()
    }
    /// Generate fixed interior probes and shared bit-identified GPU nodes.
    /// No tolerance, density or probability mass is reduced during refinement.
    pub fn try_new(
        regions: &[[f32; 4]],
        refinement: u32,
    ) -> Result<Self, GgxEnergyRefinementError> {
        if !valid_level(refinement) {
            return Err(GgxEnergyRefinementError::InvalidLevel);
        }
        if regions.is_empty() {
            return Err(GgxEnergyRefinementError::InvalidRegion);
        }
        for (i, b) in regions.iter().enumerate() {
            if !b.iter().all(|v| v.is_finite() && (0. ..=1.).contains(v))
                || b[0] >= b[2]
                || b[1] <= 0.
                || b[1] >= b[3]
            {
                return Err(GgxEnergyRefinementError::InvalidRegion);
            }
            for a in &regions[..i] {
                if a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3] {
                    return Err(GgxEnergyRefinementError::OverlappingRegions);
                }
            }
        }
        let mut nodes = Vec::new();
        let mut identities = HashMap::new();
        let mut probes = Vec::new();
        let mut probe_ids = HashSet::new();
        for b in regions {
            for fy in [0.125, 0.5, 0.875] {
                for fx in [0.125, 0.5, 0.875] {
                    let uv = [
                        (f64::from(b[0]) + fx * f64::from(b[2] - b[0])) as f32,
                        (f64::from(b[1]) + fy * f64::from(b[3] - b[1])) as f32,
                    ];
                    if !probe_ids.insert(uv.map(f32::to_bits)) {
                        return Err(GgxEnergyRefinementError::DuplicateProbe);
                    }
                    let (xs, dx) = axis(uv[0], refinement, false)?;
                    let (ys, dy) = axis(uv[1], refinement, true)?;
                    let x = cell(uv[0], dx);
                    let y = cell(uv[1], dy);
                    let bounds = [x[0], y[0], x[1], y[1]];
                    if x[0] < xs[0] || x[1] > xs[5] || y[0] < ys[0] || y[1] > ys[5] {
                        return Err(GgxEnergyRefinementError::UnrepresentableStencil);
                    }
                    let mut indices = [0; 36];
                    for row in 0..6 {
                        for col in 0..6 {
                            let coordinate = [xs[col], ys[row]];
                            let key = coordinate.map(f32::to_bits);
                            indices[row * 6 + col] = *identities.entry(key).or_insert_with(|| {
                                let id = nodes.len();
                                nodes.push(coordinate);
                                id
                            });
                        }
                    }
                    probes.push(GgxEnergyRefinementProbe {
                        uv,
                        bounds,
                        view_nodes: xs,
                        roughness_nodes: ys,
                        node_indices: indices,
                    });
                }
            }
        }
        Ok(Self {
            refinement,
            nodes,
            probes,
        })
    }
    #[must_use]
    pub fn refinement(&self) -> u32 {
        self.refinement
    }
    #[must_use]
    pub fn nodes(&self) -> &[[f32; 2]] {
        &self.nodes
    }
    #[must_use]
    pub fn probes(&self) -> &[GgxEnergyRefinementProbe] {
        &self.probes
    }
    /// Structural conversion only; sampled error assessment remains separate.
    pub fn experimental_patch_data(
        &self,
        moments: &[[f32; 4]],
    ) -> Result<Vec<GgxEnergyPatchData>, GgxEnergyRefinementError> {
        if moments.len() != self.nodes.len() {
            return Err(GgxEnergyRefinementError::InvalidMomentCount);
        }
        let mut result = Vec::new();
        let mut seen = HashMap::new();
        for (i, p) in self.probes.iter().enumerate() {
            if let Some(&previous) = seen.get(&p.bounds.map(f32::to_bits)) {
                let a: &GgxEnergyRefinementProbe = &self.probes[previous];
                if a.node_indices != p.node_indices {
                    return Err(GgxEnergyRefinementError::ConflictingStencil);
                }
                continue;
            }
            let data = std::array::from_fn(|j| moments[p.node_indices[j]]);
            result.push(
                GgxEnergyPatchData::try_new(p.bounds, p.view_nodes, p.roughness_nodes, data)
                    .map_err(GgxEnergyRefinementError::Patch)?,
            );
            seen.insert(p.bounds.map(f32::to_bits), i);
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy)]
pub struct GgxEnergyRefinementObservation {
    pub uv: [f32; 2],
    pub gpu: [f32; 4],
    pub reference: [f64; 3],
    pub uncertainty: [f64; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgxEnergyRefinementDecision {
    Refine { next: u32 },
    SampledPass,
    BudgetExhausted,
}
#[derive(Debug, Clone)]
pub struct GgxEnergyRefinementAssessment {
    decision: GgxEnergyRefinementDecision,
    max_relative_errors: [f64; 3],
    failed_samples: usize,
}
impl GgxEnergyRefinementAssessment {
    #[must_use]
    pub fn decision(&self) -> GgxEnergyRefinementDecision {
        self.decision
    }
    #[must_use]
    pub fn max_relative_errors(&self) -> [f64; 3] {
        self.max_relative_errors
    }
    #[must_use]
    pub fn failed_samples(&self) -> usize {
        self.failed_samples
    }
}
#[derive(Debug, Clone, Copy)]
pub struct GgxEnergyRefinementPolicy {
    target: f64,
    maximum: u32,
}
pub(super) fn validate_reference(
    reference: [f64; 3],
    uncertainty: [f64; 3],
    index: usize,
) -> Result<(), GgxEnergyRefinementError> {
    if !reference
        .iter()
        .all(|v| v.is_finite() && (0. ..=1.).contains(v))
        || (reference.iter().sum::<f64>() - 1.).abs() > 1e-12
        || (0..3).any(|j| {
            !uncertainty[j].is_finite()
                || uncertainty[j] < 0.
                || uncertainty[j] > reference[j] * 1e-6 + 1e-34
        })
    {
        return Err(GgxEnergyRefinementError::InvalidOracle(index));
    }
    Ok(())
}
impl GgxEnergyRefinementPolicy {
    /// The internal target may tighten, but never relax, the external 1e-4 gate.
    pub fn try_new(target: f64, maximum: u32) -> Result<Self, GgxEnergyRefinementError> {
        if !target.is_finite() || !(target > 0. && target <= 1e-4) || !valid_level(maximum) {
            return Err(GgxEnergyRefinementError::InvalidPolicy);
        }
        Ok(Self { target, maximum })
    }
    pub fn assess(
        &self,
        plan: &GgxEnergyRefinementPlan,
        observations: &[GgxEnergyRefinementObservation],
    ) -> Result<GgxEnergyRefinementAssessment, GgxEnergyRefinementError> {
        if observations.is_empty() {
            return Err(GgxEnergyRefinementError::EmptyObservations);
        }
        if plan.refinement > self.maximum || observations.len() != plan.probes.len() {
            return Err(GgxEnergyRefinementError::InvalidLevel);
        }
        let mut maximum = [0f64; 3];
        let mut failures = 0;
        for (i, (p, o)) in plan.probes.iter().zip(observations).enumerate() {
            if p.uv.map(f32::to_bits) != o.uv.map(f32::to_bits) {
                return Err(GgxEnergyRefinementError::ObservationMismatch(i));
            }
            validate_reference(o.reference, o.uncertainty, i)?;
            if !o
                .gpu
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
            {
                return Err(GgxEnergyRefinementError::InvalidGpu(i));
            }
            let mut failed = o.gpu[3] != 1. || (o.gpu[0] + o.gpu[1] + o.gpu[2] - 1.).abs() > 5e-6;
            for (j, max) in maximum.iter_mut().enumerate() {
                let observed = f64::from(o.gpu[j]);
                let expected = o.reference[j];
                let error = if expected == 0. {
                    observed.abs()
                } else {
                    (observed - expected).abs() / expected
                };
                *max = max.max(error);
                failed |= error >= 1e-4 || (expected == 0. && observed != 0.);
            }
            failures += usize::from(failed);
        }
        let decision = if failures == 0 && maximum.into_iter().all(|e| e < self.target) {
            GgxEnergyRefinementDecision::SampledPass
        } else if plan.refinement < self.maximum {
            GgxEnergyRefinementDecision::Refine {
                next: plan.refinement * 2,
            }
        } else {
            GgxEnergyRefinementDecision::BudgetExhausted
        };
        Ok(GgxEnergyRefinementAssessment {
            decision,
            max_relative_errors: maximum,
            failed_samples: failures,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plan(level: u32) -> GgxEnergyRefinementPlan {
        let b = GgxEnergyRefinementPlan::regions_from_seeds(&[[0.001, 0.005]]).unwrap();
        GgxEnergyRefinementPlan::try_new(&b, level).unwrap()
    }
    fn observations(p: &GgxEnergyRefinementPlan) -> Vec<GgxEnergyRefinementObservation> {
        p.probes
            .iter()
            .map(|p| GgxEnergyRefinementObservation {
                uv: p.uv,
                gpu: [0.5, 0.25, 0.25, 1.],
                reference: [0.5, 0.25, 0.25],
                uncertainty: [0.; 3],
            })
            .collect()
    }
    #[test]
    fn fixed_probes_and_shared_nodes() {
        let a = plan(1);
        let b = plan(2);
        assert_eq!(a.probes.len(), 9);
        assert_eq!(
            a.probes
                .iter()
                .map(|p| p.uv.map(f32::to_bits))
                .collect::<Vec<_>>(),
            b.probes
                .iter()
                .map(|p| p.uv.map(f32::to_bits))
                .collect::<Vec<_>>()
        );
        assert!(a.nodes.len() < 9 * 36);
    }
    #[test]
    fn rejects_bad_regions_and_unrepresentable_axes() {
        assert!(GgxEnergyRefinementPlan::regions_from_seeds(&[[f32::NAN, 0.5]]).is_err());
        assert!(GgxEnergyRefinementPlan::regions_from_seeds(&[[0.5, 1e-20]]).is_err());
        assert!(GgxEnergyRefinementPlan::try_new(&[[0.2, 0.2, 0.8, 0.8]; 2], 1).is_err());
        assert!(
            GgxEnergyRefinementPlan::try_new(
                &[[f32::from_bits(1f32.to_bits() - 1), 0.2, 1., 0.8]],
                1
            )
            .is_err()
        );
    }
    #[test]
    fn strict_policy_and_gpu_flag() {
        let p = plan(1);
        let mut o = observations(&p);
        let policy = GgxEnergyRefinementPolicy::try_new(1e-5, 8).unwrap();
        assert_eq!(
            policy.assess(&p, &o).unwrap().decision(),
            GgxEnergyRefinementDecision::SampledPass
        );
        o[0].gpu[3] = 0.;
        assert_eq!(
            policy.assess(&p, &o).unwrap().decision(),
            GgxEnergyRefinementDecision::Refine { next: 2 }
        );
        assert_eq!(
            GgxEnergyRefinementPolicy::try_new(1e-5, 1)
                .unwrap()
                .assess(&p, &o)
                .unwrap()
                .decision(),
            GgxEnergyRefinementDecision::BudgetExhausted
        );
        assert!(GgxEnergyRefinementPolicy::try_new(0.01, 8).is_err());
    }
    #[test]
    fn rejects_oracle_and_probe_mismatch() {
        let p = plan(1);
        let policy = GgxEnergyRefinementPolicy::try_new(1e-5, 8).unwrap();
        let mut o = observations(&p);
        o[0].uncertainty[0] = 1e-3;
        assert!(policy.assess(&p, &o).is_err());
        let mut o = observations(&p);
        o[0].uv[0] = 0.;
        assert!(policy.assess(&p, &o).is_err());
        assert!(policy.assess(&p, &[]).is_err());
    }
    #[test]
    fn preserves_exact_zero_requirement() {
        let p = plan(1);
        let policy = GgxEnergyRefinementPolicy::try_new(1e-5, 8).unwrap();
        let mut o = observations(&p);
        for v in &mut o {
            v.reference = [0.75, 0.25, 0.];
            v.gpu = [0.75, 0.25, 0., 1.];
        }
        o[0].gpu = [0.75, 0.249999, 1e-6, 1.];
        assert_eq!(
            policy.assess(&p, &o).unwrap().decision(),
            GgxEnergyRefinementDecision::Refine { next: 2 }
        );
    }
    #[test]
    fn structural_conversion_deduplicates_cells() {
        let p = plan(2);
        assert!(p.experimental_patch_data(&[]).is_err());
        let patches = p
            .experimental_patch_data(&vec![[0.5, 0.25, 0.25, 1.]; p.nodes.len()])
            .unwrap();
        assert!(patches.len() <= p.probes.len());
    }
}
