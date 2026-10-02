//! Automatic exposed-node to exposed-face sweeps between accepted components.
use super::{QuadraticBody, QuadraticFace, QuadraticSweep, QuadraticSweepLimits, Vec3};
#[derive(Clone, Debug)]
pub struct QuadraticFragmentSweep {
    pub source_node: usize,
    pub source_component: usize,
    pub target_component: usize,
    pub target_face: QuadraticFace,
    /// Witness or unresolved interval. Separated queries are omitted.
    pub sweep: QuadraticSweep,
}
impl QuadraticBody {
    /// Search every exposed boundary node against every exposed face on other
    /// accepted components along supplied linear trajectories. Returns witnesses
    /// and unresolved queries; an empty list means all queried pairs separated.
    /// This does not cover edge-edge crossings or locate first impact time.
    /// Topology remains fixed during the sweep; no state is changed.
    pub fn fragment_sweeps_at(
        &self,
        end: &[Vec3],
        clearance_m: f64,
        limits: QuadraticSweepLimits,
        max_queries: usize,
    ) -> Result<Vec<QuadraticFragmentSweep>, &'static str> {
        if end.len() != self.positions.len()
            || end.iter().flatten().any(|x| !x.is_finite())
            || !clearance_m.is_finite()
            || clearance_m < 0.
            || max_queries == 0
            || !limits.minimum_time_fraction.is_finite()
            || limits.minimum_time_fraction <= 0.
            || limits.minimum_time_fraction > 1.
            || limits.max_intervals == 0
            || limits.max_intervals > 65536
            || !limits.closest.distance_tolerance_m.is_finite()
            || limits.closest.distance_tolerance_m <= 0.
            || limits.closest.max_patches == 0
            || limits.closest.max_patches > 65536
            || limits.closest.max_depth > 24
        {
            return Err("invalid fragment sweep request");
        }
        let fragments = self.fragment_nodes();
        let mut owner = vec![usize::MAX; end.len()];
        for (i, nodes) in fragments.iter().enumerate() {
            for &n in nodes {
                owner[n] = i;
            }
        }
        let faces = self.exposed_faces_at(&self.positions)?;
        let sources: std::collections::BTreeSet<_> = faces.iter().flat_map(|f| f.nodes).collect();
        let mut count = 0usize;
        for &node in &sources {
            for face in &faces {
                if owner[node] != owner[face.nodes[0]] {
                    count = count
                        .checked_add(1)
                        .ok_or("fragment sweep query overflow")?;
                    if count > max_queries {
                        return Err("fragment sweep query limit");
                    }
                }
            }
        }
        let mut result = Vec::new();
        for node in sources {
            for face in &faces {
                let target = owner[face.nodes[0]];
                if owner[node] == target {
                    continue;
                }
                let sweep = face.swept_point_clearance_at(
                    &self.positions,
                    end,
                    self.positions[node],
                    end[node],
                    clearance_m,
                    limits,
                )?;
                if !matches!(sweep, QuadraticSweep::Separated { .. }) {
                    result.push(QuadraticFragmentSweep {
                        source_node: node,
                        source_component: owner[node],
                        target_component: target,
                        target_face: *face,
                        sweep,
                    });
                }
            }
        }
        Ok(result)
    }
}

#[derive(Clone, Debug)]
pub struct QuadraticFragmentFirstCandidate {
    pub source_node: usize,
    pub source_component: usize,
    pub target_component: usize,
    pub target_face: QuadraticFace,
    pub clearance: super::QuadraticFirstClearance,
}
#[derive(Clone, Debug)]
pub struct QuadraticFragmentFirstClearance {
    /// Global separated-prefix lower bound and earliest feasible witness upper
    /// bound (1 if no feasible witness exists). Unresolved pairs lower this
    /// prefix bound even if another pair has a converged later witness.
    pub time_interval: [f64; 2],
    pub earliest_witness_index: Option<usize>,
    pub candidates: Vec<QuadraticFragmentFirstCandidate>,
    pub converged: bool,
}
impl QuadraticBody {
    /// Aggregate earliest-clearance bounds over exposed node/face pairs.
    /// No state is advanced. max_queries bounds pair enumeration;
    /// max_prefix_searches bounds each candidate's refinement separately.
    pub fn first_fragment_clearance_at(
        &self,
        end: &[Vec3],
        clearance_m: f64,
        limits: QuadraticSweepLimits,
        max_queries: usize,
        time_tolerance: f64,
        max_prefix_searches: usize,
    ) -> Result<Option<QuadraticFragmentFirstClearance>, &'static str> {
        if !time_tolerance.is_finite()
            || time_tolerance <= 0.
            || time_tolerance > 1.
            || max_prefix_searches == 0
            || max_prefix_searches > 65536
        {
            return Err("invalid fragment first-clearance limits");
        }
        let possible = self.fragment_sweeps_at(end, clearance_m, limits, max_queries)?;
        let mut candidates = Vec::new();
        for pair in possible {
            if let Some(clearance) = pair.target_face.first_point_clearance_at(
                &self.positions,
                end,
                self.positions[pair.source_node],
                end[pair.source_node],
                clearance_m,
                limits,
                time_tolerance,
                max_prefix_searches,
            )? {
                candidates.push(QuadraticFragmentFirstCandidate {
                    source_node: pair.source_node,
                    source_component: pair.source_component,
                    target_component: pair.target_component,
                    target_face: pair.target_face,
                    clearance,
                });
            }
        }
        if candidates.is_empty() {
            return Ok(None);
        }
        let lower = candidates
            .iter()
            .map(|r| r.clearance.time_interval[0])
            .fold(1_f64, f64::min);
        let earliest = candidates
            .iter()
            .enumerate()
            .filter(|(_, r)| r.clearance.witness.is_some())
            .min_by(|(_, a), (_, b)| {
                a.clearance.time_interval[1].total_cmp(&b.clearance.time_interval[1])
            })
            .map(|(i, _)| i);
        let upper = earliest.map_or(1., |i| candidates[i].clearance.time_interval[1]);
        if lower > upper {
            return Err("inconsistent fragment clearance bounds");
        }
        Ok(Some(QuadraticFragmentFirstClearance {
            time_interval: [lower, upper],
            earliest_witness_index: earliest,
            converged: earliest.is_some() && upper - lower <= time_tolerance,
            candidates,
        }))
    }
}
