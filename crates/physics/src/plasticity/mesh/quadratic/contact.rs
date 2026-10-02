//! Symmetric finite-thickness frictionless T6 surface proximity potential.
use super::{QuadraticClosestLimits, QuadraticFace, Vec3};
const QUADRATURE: [(f64, f64, f64); 2] = [
    (
        0.445_948_490_915_965,
        0.108_103_018_168_070,
        0.223_381_589_678_011,
    ),
    (
        0.091_576_213_509_771,
        0.816_847_572_980_459,
        0.109_951_743_655_322,
    ),
];
#[derive(Clone, Copy, Debug)]
pub struct QuadraticSurfaceContact {
    thickness_m: f64,
    stiffness_n_m3: f64,
    closest: QuadraticClosestLimits,
    integration_depth: u8,
    sweep_guard: Option<(f64, super::QuadraticSweepLimits)>,
}
#[derive(Clone, Debug)]
pub struct QuadraticSurfaceContactEvaluation {
    pub energy_j: f64,
    pub forces_n: Vec<Vec3>,
    /// Distance-search contribution only; excludes surface quadrature error.
    pub search_energy_error_bound_j: f64,
    pub active_samples: usize,
}
impl QuadraticSurfaceContact {
    pub(super) fn thickness_m(&self) -> f64 {
        self.thickness_m
    }
    /// Reference-area penalty for positive surface thickness, stiffness in N/m³.
    /// This is a two-sided proximity layer, not signed solid penetration contact.
    /// # Errors
    /// Nonpositive/nonfinite thickness or stiffness, or invalid search limits.
    pub fn new(
        thickness_m: f64,
        stiffness_n_m3: f64,
        closest: QuadraticClosestLimits,
    ) -> Result<Self, &'static str> {
        if !thickness_m.is_finite()
            || thickness_m <= 0.
            || !stiffness_n_m3.is_finite()
            || stiffness_n_m3 <= 0.
            || !closest.distance_tolerance_m.is_finite()
            || closest.distance_tolerance_m <= 0.
            || closest.max_patches == 0
            || closest.max_patches > 65536
            || closest.max_depth > 24
        {
            return Err("invalid quadratic surface contact");
        }
        Ok(Self {
            thickness_m,
            stiffness_n_m3,
            closest,
            integration_depth: 0,
            sweep_guard: None,
        })
    }
    /// Require continuous node-to-face clearance during dynamic drift.
    /// Defaults to disabled. This guards nodes against exposed opposing faces;
    /// it does not cover edge-edge intersections or compute collision impulses.
    /// # Errors
    /// Invalid work/search limits or clearance outside (0, layer thickness).
    pub fn with_sweep_guard(
        mut self,
        clearance_m: f64,
        limits: super::QuadraticSweepLimits,
    ) -> Result<Self, &'static str> {
        let search = limits.closest;
        if !clearance_m.is_finite()
            || clearance_m <= 0.
            || clearance_m >= self.thickness_m
            || !limits.minimum_time_fraction.is_finite()
            || limits.minimum_time_fraction <= 0.
            || limits.minimum_time_fraction > 1.
            || limits.max_intervals == 0
            || limits.max_intervals > 65536
            || !search.distance_tolerance_m.is_finite()
            || search.distance_tolerance_m <= 0.
            || search.max_patches == 0
            || search.max_patches > 65536
            || search.max_depth > 24
        {
            return Err("invalid quadratic surface sweep guard");
        }
        self.sweep_guard = Some((clearance_m, limits));
        Ok(self)
    }
    pub(super) fn sweep_guard(&self) -> Option<(f64, super::QuadraticSweepLimits)> {
        self.sweep_guard
    }
    /// Uniform four-way subdivision depth for the positive six-point surface
    /// rule (default zero). This controls spatial integration, not closest search.
    /// # Errors
    /// Depth greater than five exceeds the bounded surface integration limit.
    pub fn with_integration_depth(mut self, depth: u8) -> Result<Self, &'static str> {
        if depth > 5 {
            return Err("quadratic surface integration limit");
        }
        self.integration_depth = depth;
        Ok(self)
    }
    /// Equal two-pass reference-area integration of 0.5*K*max(h-distance,0)².
    /// Forces are the negative energy gradient at unique converged projections.
    /// The positive six-point rule is applied on uniformly subdivided patches.
    /// Curved/partially active surfaces require integration-depth convergence.
    /// # Errors
    /// Shared/repeated nodes, invalid geometry, unresolved closest search,
    /// active samples within the distance-search tolerance (unresolved normal), or overflow.
    pub fn evaluate(
        &self,
        positions: &[Vec3],
        first: QuadraticFace,
        second: QuadraticFace,
    ) -> Result<QuadraticSurfaceContactEvaluation, &'static str> {
        self.evaluate_surfaces(positions, &[first], &[second])
    }
    /// Two-pass potential using the nearest point on each complete target surface.
    /// Faces within a surface may share nodes; the two surfaces must be disjoint.
    /// Each source quadrature sample interacts with one nearest target face,
    /// avoiding duplicate pressure from adjacent target faces.
    /// # Errors
    /// Invalid/duplicate faces, shared surface nodes, unresolved search or overflow.
    pub fn evaluate_surfaces(
        &self,
        positions: &[Vec3],
        first: &[QuadraticFace],
        second: &[QuadraticFace],
    ) -> Result<QuadraticSurfaceContactEvaluation, &'static str> {
        validate_surfaces(positions, first, second)?;
        let mut result = QuadraticSurfaceContactEvaluation {
            energy_j: 0.,
            forces_n: vec![[0.; 3]; positions.len()],
            search_energy_error_bound_j: 0.,
            active_samples: 0,
        };
        let samples = integration_samples(self.integration_depth);
        for (sources, targets) in [(first, second), (second, first)] {
            for source in sources {
                for &(l, weight) in &samples {
                    let (shape, _, _) = super::cohesive::basis(l);
                    let anchor = positions[source.nodes[0]];
                    let query = std::array::from_fn(|axis| {
                        anchor[axis]
                            + source
                                .nodes
                                .iter()
                                .zip(shape)
                                .map(|(&node, n)| (positions[node][axis] - anchor[axis]) * n)
                                .sum::<f64>()
                    });
                    let mut selected: Option<(QuadraticFace, super::QuadraticClosestPoint)> = None;
                    let mut lower = self.thickness_m;
                    for target in targets {
                        if aabb_distance(positions, *target, query)? >= self.thickness_m {
                            continue;
                        }
                        let candidate = target.closest_point_at(positions, query, self.closest)?;
                        if !candidate.converged {
                            return Err("unresolved quadratic contact projection");
                        }
                        lower = lower.min(candidate.lower_distance_m);
                        if selected
                            .as_ref()
                            .is_none_or(|(_, best)| candidate.distance_m < best.distance_m)
                        {
                            selected = Some((*target, candidate));
                        }
                    }
                    let Some((target, mut closest)) = selected else {
                        continue;
                    };
                    closest.lower_distance_m = lower.min(closest.distance_m);
                    let penetration = (self.thickness_m - closest.distance_m).max(0.);
                    let upper_penetration = (self.thickness_m - closest.lower_distance_m).max(0.);
                    let measure = 0.5 * source.reference_area_m2 * weight;
                    result.energy_j +=
                        0.5 * self.stiffness_n_m3 * measure * penetration * penetration;
                    result.search_energy_error_bound_j += 0.5
                        * self.stiffness_n_m3
                        * measure
                        * (upper_penetration * upper_penetration - penetration * penetration)
                            .max(0.);
                    if penetration == 0. {
                        continue;
                    }
                    if closest.distance_m <= self.closest.distance_tolerance_m {
                        return Err("coincident quadratic contact sample");
                    }
                    result.active_samples += 1;
                    let force: Vec3 = std::array::from_fn(|axis| {
                        self.stiffness_n_m3
                            * measure
                            * penetration
                            * (query[axis] - closest.point_m[axis])
                            / closest.distance_m
                    });
                    for (i, &source_weight) in shape.iter().enumerate() {
                        for (axis, &component) in force.iter().enumerate() {
                            result.forces_n[source.nodes[i]][axis] += source_weight * component;
                            result.forces_n[target.nodes[i]][axis] -=
                                closest.shape_weights[i] * component;
                        }
                    }
                }
            }
        }
        if !result.energy_j.is_finite()
            || !result.search_energy_error_bound_j.is_finite()
            || result.forces_n.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("quadratic surface contact overflow");
        }
        Ok(result)
    }
}

fn validate_surfaces(
    positions: &[Vec3],
    first: &[QuadraticFace],
    second: &[QuadraticFace],
) -> Result<(), &'static str> {
    if first.is_empty()
        || second.is_empty()
        || first.len() + second.len() > 4096
        || positions.iter().flatten().any(|x| !x.is_finite())
    {
        return Err("invalid quadratic contact surfaces");
    }
    let mut keys = std::collections::BTreeSet::new();
    for face in first.iter().chain(second) {
        let mut key = face.nodes;
        key.sort_unstable();
        if key.windows(2).any(|n| n[0] == n[1])
            || !keys.insert(key)
            || key.iter().any(|&n| n >= positions.len())
            || !face.reference_area_m2.is_finite()
            || face.reference_area_m2 <= 0.
        {
            return Err("invalid quadratic contact face");
        }
    }
    let nodes: std::collections::BTreeSet<_> = first.iter().flat_map(|f| f.nodes).collect();
    if second
        .iter()
        .flat_map(|f| f.nodes)
        .any(|n| nodes.contains(&n))
    {
        return Err("quadratic contact surfaces share nodes");
    }
    Ok(())
}
fn aabb_distance(
    positions: &[Vec3],
    face: QuadraticFace,
    query: Vec3,
) -> Result<f64, &'static str> {
    let mut controls: [[f64; 3]; 6] = face
        .nodes
        .map(|n| std::array::from_fn(|i| positions[n][i] - query[i]));
    for (edge, (a, b)) in [(0, 1), (1, 2), (0, 2)].into_iter().enumerate() {
        controls[3 + edge] = std::array::from_fn(|i| {
            2. * controls[3 + edge][i] - controls[a][i].midpoint(controls[b][i])
        });
    }
    if controls.iter().flatten().any(|x| !x.is_finite()) {
        return Err("quadratic contact bound overflow");
    }
    let distance: Vec3 = std::array::from_fn(|i| {
        let min = controls.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min);
        let max = controls
            .iter()
            .map(|p| p[i])
            .fold(f64::NEG_INFINITY, f64::max);
        if min > 0. {
            min
        } else if max < 0. {
            max
        } else {
            0.
        }
    });
    Ok(distance[0].hypot(distance[1]).hypot(distance[2]))
}

fn integration_patches(depth: u8) -> Vec<[Vec3; 3]> {
    let mut patches = vec![[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]];
    let midpoint = |a: Vec3, b: Vec3| std::array::from_fn(|i| a[i].midpoint(b[i]));
    for _ in 0..depth {
        patches = patches
            .into_iter()
            .flat_map(|[a, b, c]| {
                let ab = midpoint(a, b);
                let bc = midpoint(b, c);
                let ac = midpoint(a, c);
                [[a, ab, ac], [ab, b, bc], [ac, bc, c], [ab, bc, ac]]
            })
            .collect();
    }
    patches
}

fn integration_samples(depth: u8) -> Vec<(Vec3, f64)> {
    let weight = 4_f64.powi(-i32::from(depth));
    let mut samples = Vec::new();
    for patch in integration_patches(depth) {
        for (a, b, w) in QUADRATURE {
            for distinct in 0..3 {
                let local: Vec3 = std::array::from_fn(|i| if i == distinct { b } else { a });
                let l = std::array::from_fn(|axis| (0..3).map(|i| local[i] * patch[i][axis]).sum());
                samples.push((l, w * weight));
            }
        }
    }
    samples
}
