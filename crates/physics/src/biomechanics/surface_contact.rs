//! Nonadjacent triangle barriers and conservative advancement of straight steps.
use super::surface_distance::{
    PreparedTriangle, separation_lower_bound, triangle_distance, triangle_pair_path_is_open,
};
use super::{Body, Vec3, add, cross, dot, scale, sub};
#[derive(Clone, Debug)]
pub struct TissueSurfaceContact {
    pub faces: Vec<[usize; 3]>,
    pub minimum_distance_m: f64,
    pub activation_gap_m: f64,
    /// Discrete per-triangle-pair coefficient; not a mesh-independent material fit.
    pub pair_stiffness_n_m: f64,
}
/// Experimental primitive law is uncalibrated and retains the geometric path guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceContactLaw {
    TriangleMinimum,
    ExperimentalPrimitiveSum,
}
/// Canonical diagnostic stencil. Group identity preserves configured exclusions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SurfacePrimitive {
    VertexFace {
        group: usize,
        vertex: usize,
        face: [usize; 3],
    },
    EdgeEdge {
        group: usize,
        edges: [[usize; 2]; 2],
    },
}
impl TissueSurfaceContact {
    fn candidates(&self, start: &[Vec3], end: Option<&[Vec3]>, margin: f64) -> Vec<(usize, usize)> {
        // === OPTIMIZATION #1-2: Sweep-and-prune с порогом для O(n²) ===
        const SWEEP_THRESHOLD: usize = 64;
        
        let mut bounds: Vec<_> = self
            .faces
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let mut lo = [f64::INFINITY; 3];
                let mut hi = [f64::NEG_INFINITY; 3];
                for &node in f {
                    for k in 0..3 {
                        lo[k] = lo[k].min(start[node][k]);
                        hi[k] = hi[k].max(start[node][k]);
                        if let Some(end) = end {
                            lo[k] = lo[k].min(end[node][k]);
                            hi[k] = hi[k].max(end[node][k]);
                        }
                    }
                }
                for k in 0..3 {
                    lo[k] -= 0.5 * margin;
                    hi[k] += 0.5 * margin;
                }
                (i, lo, hi)
            })
            .collect();
        
        // Sort by X coordinate only (sweep line optimization)
        bounds.sort_unstable_by(|a, b| a.1[0].total_cmp(&b.1[0]));
        
        let n = bounds.len();
        
        // === OPTIMIZATION #3: Pre-reserve capacity based on expected density ===
        let capacity = if n < SWEEP_THRESHOLD {
            (n * n) / 4 // Conservative estimate for small N
        } else {
            n.min(500) // Cap for large scenes
        };
        let mut pairs = Vec::with_capacity(capacity);
        
        if n < SWEEP_THRESHOLD {
            // Small scene: simple nested loop (cache-friendly)
            for i in 0..n {
                let (a, lo_a, hi_a) = bounds[i];
                for j in i + 1..n {
                    let (b, lo_b, hi_b) = bounds[j];
                    
                    // Quick rejection: X-axis separation due to sorting
                    if lo_b[0] > hi_a[0] {
                        break; // Can stop early!
                    }
                    
                    // 3D AABB overlap check (fast fail)
                    if lo_a[1] > hi_b[1] || lo_b[1] > hi_a[1] ||
                       lo_a[2] > hi_b[2] || lo_b[2] > hi_a[2] {
                        continue;
                    }
                    
                    // Shared vertex check (most common rejection)
                    if !self.faces[a].iter().any(|v| self.faces[b].contains(v)) {
                        pairs.push((a, b));
                    }
                }
            }
        } else {
            // Large scene: sweep-and-prune with early exits
            for i in 0..n {
                let (a, lo_a, hi_a) = bounds[i];
                
                // Optimization: track active window to avoid scanning entire array
                let start_j = if i > 0 { i.saturating_sub(8) } else { 0 };
                
                for j in start_j..n {
                    let (b, lo_b, hi_b) = bounds[j];
                    
                    // Early termination when X separated
                    if lo_b[0] > hi_a[0] {
                        break;
                    }
                    
                    // Fast 3D AABB test
                    if lo_a[1] <= hi_b[1] && lo_b[1] <= hi_a[1] &&
                       lo_a[2] <= hi_b[2] && lo_b[2] <= hi_a[2] {
                        
                        if !self.faces[a].iter().any(|v| self.faces[b].contains(v)) {
                            pairs.push((a, b));
                        }
                    }
                }
            }
        }
        
        // Deduplicate via BTreeSet if too many pairs (unlikely but safe)
        if pairs.len() > n * n / 4 {
            use std::collections::BTreeSet;
            let mut seen = BTreeSet::new();
            pairs.retain(|&(a, b)| {
                let key = (a.min(b), a.max(b));
                seen.insert(key)
            });
        }
        
        pairs
    }
    fn energy_gradient(&self, x: &[Vec3], gradient: &mut [Vec3]) -> Result<f64, &'static str> {
        self.energy_gradient_impl::<true>(x, gradient)
    }
    fn energy_gradient_impl<const CULL: bool>(
        &self,
        x: &[Vec3],
        gradient: &mut [Vec3],
    ) -> Result<f64, &'static str> {
        for face in &self.faces {
            let [a, b, c] = face.map(|i| x[i]);
            let n = cross(sub(b, a), sub(c, a));
            if !dot(n, n).is_finite() || dot(n, n) <= 1e-30 {
                return Err("degenerate contact triangle");
            }
        }
        let prepared: Vec<_> = if CULL {
            self.faces
                .iter()
                .map(|face| PreparedTriangle::new(face.map(|i| x[i])))
                .collect()
        } else {
            Vec::new()
        };
        let mut energy = 0.;
        for (a, b) in self.candidates(x, None, self.minimum_distance_m + self.activation_gap_m) {
            if CULL
                && prepared[a].separation_lower_bound(&prepared[b])
                    >= self.minimum_distance_m + self.activation_gap_m
            {
                continue;
            }
            let fa = self.faces[a];
            let fb = self.faces[b];
            let a = fa.map(|i| x[i]);
            let b = fb.map(|i| x[i]);
            let closest = triangle_distance(a, b)?;
            let (pair_energy, derivative) = barrier_response(
                closest.distance,
                self.minimum_distance_m,
                self.activation_gap_m,
                self.pair_stiffness_n_m,
            )?;
            energy += pair_energy;
            if derivative == 0. {
                continue;
            }
            let g = scale(closest.delta, derivative / closest.distance);
            for k in 0..3 {
                gradient[fa[k]] = add(gradient[fa[k]], scale(g, closest.a[k]));
                gradient[fb[k]] = sub(gradient[fb[k]], scale(g, closest.b[k]));
            }
        }
        Ok(energy)
    }
    fn path_is_open(&self, start: &[Vec3], end: &[Vec3]) -> bool {
        self.path_is_open_impl::<true>(start, end)
    }
    fn path_is_open_impl<const CULL: bool>(&self, start: &[Vec3], end: &[Vec3]) -> bool {
        for (a, b) in self.candidates(start, Some(end), self.minimum_distance_m) {
            let fa = self.faces[a];
            let fb = self.faces[b];
            if !triangle_pair_path_is_open::<CULL>(
                fa.map(|node| start[node]),
                fa.map(|node| end[node]),
                fb.map(|node| start[node]),
                fb.map(|node| end[node]),
                self.minimum_distance_m,
            ) {
                return false;
            }
        }
        true
    }
}
// Shared triangle-minimum barrier for internal and prescribed external surfaces.
// Coefficients are admitted by the owning contact configuration.
pub(super) fn barrier_response(
    distance: f64,
    minimum: f64,
    activation: f64,
    stiffness: f64,
) -> Result<(f64, f64), &'static str> {
    barrier_response_gap(distance - minimum, activation, stiffness)
}
pub(super) fn barrier_response_gap(
    gap: f64,
    activation: f64,
    stiffness: f64,
) -> Result<(f64, f64), &'static str> {
    if !gap.is_finite() || gap <= 0. {
        return Err("closed surface contact gap");
    }
    if gap >= activation {
        return Ok((0., 0.));
    }
    let offset = gap - activation;
    let logarithm = if gap < 0.5 * activation {
        (gap / activation).ln()
    } else {
        (offset / activation).ln_1p()
    };
    let energy = -stiffness * offset * offset * logarithm;
    let derivative = -stiffness * (2. * offset * logarithm + offset * offset / gap);
    if !energy.is_finite() || !derivative.is_finite() {
        return Err("surface barrier response overflow");
    }
    Ok((energy, derivative))
}

type PrimitiveGeometry = (usize, [usize; 4], Vec3, [f64; 4], f64, f64, [Vec3; 4]);
pub(super) fn barrier_curvature(gap: f64, activation: f64, stiffness: f64) -> f64 {
    let offset = gap - activation;
    let logarithm = if gap < 0.5 * activation {
        (gap / activation).ln()
    } else {
        (offset / activation).ln_1p()
    };
    -stiffness * (2. * logarithm + 4. * offset / gap - (offset / gap).powi(2))
}
impl Body {
    /// Experimental mesh-wide unique primitive barrier and gradient.
    /// Uses reference-based edge mollification including its product derivative.
    /// Does not replace the solver law; endpoint-feature deduplication and mesh
    /// weighting still require verification before calibrated use.
    pub fn surface_primitive_energy_at(
        &self,
        x: &[Vec3],
    ) -> Result<(f64, Vec<Vec3>), &'static str> {
        self.evaluate(x)?;
        self.primitive_energy_gradient(x)
    }
    fn primitive_geometry(
        &self,
        x: &[Vec3],
        stencil: SurfacePrimitive,
    ) -> Result<PrimitiveGeometry, &'static str> {
        Ok(match stencil {
            SurfacePrimitive::VertexFace {
                group,
                vertex,
                face,
            } => {
                let (delta, w, d) =
                    super::surface_distance::vertex_face_closest(x[vertex], face.map(|i| x[i]));
                (
                    group,
                    [vertex, face[0], face[1], face[2]],
                    delta,
                    [1., -w[0], -w[1], -w[2]],
                    d,
                    1.,
                    [[0.; 3]; 4],
                )
            }
            SurfacePrimitive::EdgeEdge { group, edges } => {
                let nodes = [edges[0][0], edges[0][1], edges[1][0], edges[1][1]];
                let positions = nodes.map(|i| x[i]);
                let (delta, w, d) = super::surface_distance::edge_edge_closest(positions);
                let (m, mg) =
                    super::edge_contact_mollifier(nodes.map(|i| self.rest[i]), positions)?;
                (group, nodes, delta, w, d, m, mg)
            }
        })
    }
    /// Positive lumped normal-barrier curvature in N/m per node. This is an
    /// approximate solver preconditioner, not the full Hessian or tissue stiffness.
    /// Omits distance/mollifier Hessians and does not alter energy or gradients.
    pub fn surface_primitive_curvature_at(&self, x: &[Vec3]) -> Result<Vec<f64>, &'static str> {
        self.evaluate(x)?;
        self.primitive_curvature(x)
    }
    pub(super) fn primitive_curvature(&self, x: &[Vec3]) -> Result<Vec<f64>, &'static str> {
        let mut diagonal = vec![0.; x.len()];
        for stencil in self.primitive_stencils_unchecked(x) {
            let (group, nodes, _, weights, d, m, _) = self.primitive_geometry(x, stencil)?;
            let c = &self.surface_contacts[group];
            let gap = d - c.minimum_distance_m;
            if gap <= 0. {
                return Err("closed primitive curvature gap");
            }
            if gap >= c.activation_gap_m {
                continue;
            }
            let curvature = m * barrier_curvature(gap, c.activation_gap_m, c.pair_stiffness_n_m);
            if !curvature.is_finite() || curvature < 0. {
                return Err("primitive curvature overflow");
            }
            for i in 0..4 {
                diagonal[nodes[i]] += curvature * weights[i] * weights[i];
            }
        }
        if diagonal.iter().any(|d| !d.is_finite()) {
            return Err("primitive curvature accumulation overflow");
        }
        Ok(diagonal)
    }
    fn primitive_energy_gradient(&self, x: &[Vec3]) -> Result<(f64, Vec<Vec3>), &'static str> {
        let stencils = self.primitive_stencils_unchecked(x);
        let mut energy = 0.;
        let mut gradient = vec![[0.; 3]; x.len()];
        for stencil in stencils {
            let (group, nodes, delta, weights, d, m, mg) = self.primitive_geometry(x, stencil)?;
            let c = &self.surface_contacts[group];
            let gap = d - c.minimum_distance_m;
            if gap <= 0. {
                return Err("closed primitive barrier gap");
            }
            if gap >= c.activation_gap_m {
                continue;
            }
            let offset = gap - c.activation_gap_m;
            let logarithm = if gap < 0.5 * c.activation_gap_m {
                (gap / c.activation_gap_m).ln()
            } else {
                (offset / c.activation_gap_m).ln_1p()
            };
            let barrier = -c.pair_stiffness_n_m * offset * offset * logarithm;
            let derivative =
                -c.pair_stiffness_n_m * (2. * offset * logarithm + offset * offset / gap);
            energy += m * barrier;
            let g = scale(delta, m * derivative / d);
            for i in 0..4 {
                gradient[nodes[i]] = add(
                    gradient[nodes[i]],
                    add(scale(g, weights[i]), scale(mg[i], barrier)),
                );
            }
        }
        if !energy.is_finite() || gradient.iter().flatten().any(|v| !v.is_finite()) {
            return Err("primitive energy overflow");
        }
        Ok((energy, gradient))
    }
    /// Broad-phase primitive candidates, deduplicated over neighboring faces.
    /// This is diagnostic topology, not a new energy law. Uses existing exclusions.
    pub fn surface_primitive_stencils_at(
        &self,
        x: &[Vec3],
    ) -> Result<Vec<SurfacePrimitive>, &'static str> {
        self.evaluate(x)?;
        Ok(self.primitive_stencils_unchecked(x))
    }
    fn primitive_stencils_unchecked(&self, x: &[Vec3]) -> Vec<SurfacePrimitive> {
        let mut set = std::collections::BTreeSet::new();
        for (group, contact) in self.surface_contacts.iter().enumerate() {
            for (a, b) in contact.candidates(
                x,
                None,
                contact.minimum_distance_m + contact.activation_gap_m,
            ) {
                let mut a = contact.faces[a];
                let mut b = contact.faces[b];
                a.sort_unstable();
                b.sort_unstable();
                for vertex in a {
                    set.insert(SurfacePrimitive::VertexFace {
                        group,
                        vertex,
                        face: b,
                    });
                }
                for vertex in b {
                    set.insert(SurfacePrimitive::VertexFace {
                        group,
                        vertex,
                        face: a,
                    });
                }
                for i in 0..3 {
                    for j in 0..3 {
                        let mut ea = [a[i], a[(i + 1) % 3]];
                        let mut eb = [b[j], b[(j + 1) % 3]];
                        ea.sort_unstable();
                        eb.sort_unstable();
                        let mut edges = [ea, eb];
                        edges.sort_unstable();
                        set.insert(SurfacePrimitive::EdgeEdge { group, edges });
                    }
                }
            }
        }
        set.into_iter().collect()
    }
    /// Select a contact energy atomically without changing the geometry guard.
    /// Primitive stiffness is discrete and not interchangeable with a tissue fit.
    pub fn set_surface_contact_law(&mut self, law: SurfaceContactLaw) -> Result<(), &'static str> {
        let mut next = self.clone();
        next.surface_contact_law = law;
        next.evaluate(&next.positions)?;
        *self = next;
        Ok(())
    }
    #[must_use]
    pub fn surface_contact_law(&self) -> SurfaceContactLaw {
        self.surface_contact_law
    }
    /// Replace explicitly selected surface groups atomically. Shared-vertex face
    /// pairs are excluded. Separate groups do not interact across their boundary.
    /// # Errors
    /// Invalid/duplicate faces, coefficients, current contact or overflow.
    pub fn set_surface_contacts(
        &mut self,
        contacts: Vec<TissueSurfaceContact>,
    ) -> Result<(), &'static str> {
        let mut seen = std::collections::BTreeSet::new();
        for contact in &contacts {
            if contact.faces.is_empty()
                || [
                    contact.minimum_distance_m,
                    contact.activation_gap_m,
                    contact.pair_stiffness_n_m,
                ]
                .iter()
                .any(|v| !v.is_finite() || *v <= 0.)
            {
                return Err("invalid surface contact controls");
            }
            for face in &contact.faces {
                let mut key = *face;
                key.sort_unstable();
                if key.iter().any(|&i| i >= self.rest.len())
                    || key.windows(2).any(|p| p[0] == p[1])
                    || !seen.insert(key)
                {
                    return Err("invalid or duplicate surface contact face");
                }
            }
        }
        let mut next = self.clone();
        next.surface_contacts = contacts;
        next.evaluate(&next.positions)?;
        *self = next;
        Ok(())
    }
    #[must_use]
    pub fn surface_contacts(&self) -> &[TissueSurfaceContact] {
        &self.surface_contacts
    }
    /// Minimum distance over configured nonadjacent face pairs. Returns None
    /// when no eligible pair exists. Adjacent and cross-group pairs are excluded.
    /// # Errors
    /// Degenerate geometry or nonfinite distance.
    pub fn minimum_surface_contact_distance(&self) -> Result<Option<f64>, &'static str> {
        let mut minimum = f64::INFINITY;
        for contact in &self.surface_contacts {
            for (i, a) in contact.faces.iter().enumerate() {
                for b in &contact.faces[i + 1..] {
                    if a.iter().any(|v| b.contains(v)) {
                        continue;
                    }
                    let closest = triangle_distance(
                        a.map(|i| self.positions[i]),
                        b.map(|i| self.positions[i]),
                    )?;
                    minimum = minimum.min(closest.distance);
                }
            }
        }
        Ok(minimum.is_finite().then_some(minimum))
    }
    /// Active nonadjacent triangle pairs at a validated trial state. Each row
    /// contains group index, two node triples, distance and barrier energy.
    /// This is a diagnostic; it neither commits geometry nor certifies equilibrium.
    pub fn active_surface_pairs_at(
        &self,
        x: &[Vec3],
    ) -> Result<Vec<(usize, [usize; 3], [usize; 3], f64, f64)>, &'static str> {
        self.evaluate(x)?;
        let mut rows = Vec::new();
        for (group, contact) in self.surface_contacts.iter().enumerate() {
            for (a, b) in contact.candidates(
                x,
                None,
                contact.minimum_distance_m + contact.activation_gap_m,
            ) {
                let fa = contact.faces[a];
                let fb = contact.faces[b];
                let closest = triangle_distance(fa.map(|i| x[i]), fb.map(|i| x[i]))?;
                let gap = closest.distance - contact.minimum_distance_m;
                if gap >= contact.activation_gap_m {
                    continue;
                }
                if gap <= 0. {
                    return Err("closed surface contact gap");
                }
                let offset = gap - contact.activation_gap_m;
                let logarithm = if gap < 0.5 * contact.activation_gap_m {
                    (gap / contact.activation_gap_m).ln()
                } else {
                    (offset / contact.activation_gap_m).ln_1p()
                };
                rows.push((
                    group,
                    fa,
                    fb,
                    closest.distance,
                    -contact.pair_stiffness_n_m * offset * offset * logarithm,
                ));
            }
        }
        Ok(rows)
    }
    /// Closest-point barycentric weights and distance for a diagnostic face pair.
    /// Validates the trial state and node indices; does not commit geometry.
    pub fn surface_pair_closest_at(
        &self,
        x: &[Vec3],
        a: [usize; 3],
        b: [usize; 3],
    ) -> Result<([f64; 3], [f64; 3], f64), &'static str> {
        self.evaluate(x)?;
        if a.iter().chain(b.iter()).any(|i| *i >= x.len()) {
            return Err("invalid diagnostic face index");
        }
        let closest = triangle_distance(a.map(|i| x[i]), b.map(|i| x[i]))?;
        Ok((closest.a, closest.b, closest.distance))
    }
    /// Experimental sum of six point/triangle and nine edge/edge barriers for
    /// one pair. No mesh deduplication or parallel-edge mollification: diagnostic
    /// only, not the body's contact law or a complete IPC implementation.
    pub fn surface_pair_primitive_barrier_at(
        &self,
        x: &[Vec3],
        a: [usize; 3],
        b: [usize; 3],
        clearance: f64,
        activation: f64,
        stiffness: f64,
    ) -> Result<(f64, [Vec3; 6]), &'static str> {
        self.surface_pair_closest_at(x, a, b)?;
        if [clearance, activation, stiffness]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("invalid diagnostic barrier parameters");
        }
        let mut energy = 0.;
        let mut gradient = [[0.; 3]; 6];
        for c in
            super::surface_distance::triangle_primitive_distances(a.map(|i| x[i]), b.map(|i| x[i]))?
        {
            let gap = c.distance - clearance;
            if gap <= 0. {
                return Err("closed primitive contact");
            }
            if gap >= activation {
                continue;
            }
            let offset = gap - activation;
            let logarithm = if gap < 0.5 * activation {
                (gap / activation).ln()
            } else {
                (offset / activation).ln_1p()
            };
            energy -= stiffness * offset * offset * logarithm;
            let derivative = -stiffness * (2. * offset * logarithm + offset * offset / gap);
            let g = scale(c.delta, derivative / c.distance);
            for k in 0..3 {
                gradient[k] = add(gradient[k], scale(g, c.a[k]));
                gradient[k + 3] = sub(gradient[k + 3], scale(g, c.b[k]));
            }
        }
        Ok((energy, gradient))
    }
    pub(super) fn surface_energy_gradient(
        &self,
        x: &[Vec3],
        gradient: &mut [Vec3],
    ) -> Result<f64, &'static str> {
        if self.surface_contact_law == SurfaceContactLaw::ExperimentalPrimitiveSum {
            // Preserve exact triangle gap/degeneracy rejection independently of
            // mollified energies. Never recurse through Body::evaluate here.
            let mut guard_gradient = vec![[0.; 3]; x.len()];
            for contact in &self.surface_contacts {
                contact.energy_gradient(x, &mut guard_gradient)?;
            }
            let (energy, primitive_gradient) = self.primitive_energy_gradient(x)?;
            for (g, p) in gradient.iter_mut().zip(primitive_gradient) {
                *g = add(*g, p);
            }
            return Ok(energy);
        }
        let mut energy = 0.;
        for contact in &self.surface_contacts {
            energy += contact.energy_gradient(x, gradient)?;
        }
        Ok(energy)
    }
    pub(super) fn surface_path_is_open(&self, start: &[Vec3], end: &[Vec3]) -> bool {
        self.surface_contacts
            .iter()
            .all(|contact| contact.path_is_open(start, end))
    }
}

#[cfg(test)]
mod pruning_tests {
    use super::*;
    #[test]
    fn normal_barrier_curvature_matches_energy_second_difference() {
        let h = 5e-5;
        let k = 1.;
        for gap in [6e-8, 3e-7, 1e-6, 1e-5, 4e-5] {
            let energy = |g: f64| -k * (g - h).powi(2) * (g / h).ln();
            let step = gap * 1e-4;
            let fd = (energy(gap + step) - 2. * energy(gap) + energy(gap - step)) / (step * step);
            let curvature = barrier_curvature(gap, h, k);
            assert!(curvature > 0.);
            assert!(
                (fd - curvature).abs() / curvature < 1e-5,
                "gap={gap} fd={fd} c={curvature}"
            );
        }
    }

    #[test]
    fn pruning_preserves_unpruned_energy_gradients_and_contact_errors() {
        let mut seed = 11_u64;
        let mut random = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 11) as f64 / (1_u64 << 53) as f64 - 0.5) * 0.02
        };
        let contact = TissueSurfaceContact {
            faces: vec![[0, 1, 2], [3, 4, 5]],
            minimum_distance_m: 1e-6,
            activation_gap_m: 0.001,
            pair_stiffness_n_m: 10.,
        };
        for _ in 0..10000 {
            let x: Vec<Vec3> = (0..6).map(|_| std::array::from_fn(|_| random())).collect();
            let mut a = vec![[0.; 3]; 6];
            let mut b = a.clone();
            let optimized = contact.energy_gradient_impl::<true>(&x, &mut a);
            let baseline = contact.energy_gradient_impl::<false>(&x, &mut b);
            assert_eq!(optimized, baseline);
            if optimized.is_ok() {
                assert_eq!(a, b);
            }
        }
    }
    #[test]
    fn path_pruning_rejects_degenerate_candidate_before_distance_certificate() {
        let contact = TissueSurfaceContact {
            faces: vec![[0, 1, 2], [3, 4, 5]],
            minimum_distance_m: 1e-6,
            activation_gap_m: 0.001,
            pair_stiffness_n_m: 10.,
        };
        let x = vec![
            [0., 0., 0.],
            [0.01, 0., 0.01],
            [0., 0.01, 0.01],
            [0.002, 0.002, 0.009],
            [0.003, 0.002, 0.009],
            [0.004, 0.002, 0.009],
        ];
        assert!(!contact.path_is_open_impl::<false>(&x, &x));
        assert!(!contact.path_is_open_impl::<true>(&x, &x));
    }
    #[test]
    fn path_pruning_never_accepts_a_path_rejected_by_exact_feature_guard() {
        let mut seed = 37_u64;
        let mut random = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 11) as f64 / (1_u64 << 53) as f64 - 0.5) * 0.02
        };
        let contact = TissueSurfaceContact {
            faces: vec![[0, 1, 2], [3, 4, 5]],
            minimum_distance_m: 1e-6,
            activation_gap_m: 0.001,
            pair_stiffness_n_m: 10.,
        };
        let mut accepted = 0;
        let mut rejected = 0;
        for _ in 0..10000 {
            let start: Vec<Vec3> = (0..6).map(|_| std::array::from_fn(|_| random())).collect();
            let end: Vec<Vec3> = (0..6).map(|_| std::array::from_fn(|_| random())).collect();
            let optimized = contact.path_is_open_impl::<true>(&start, &end);
            let baseline = contact.path_is_open_impl::<false>(&start, &end);
            assert!(
                !optimized || baseline,
                "pruning must not permit a rejected trajectory"
            );
            if optimized {
                accepted += 1;
            } else {
                rejected += 1;
            }
        }
        assert!(accepted > 0 && rejected > 0);
        eprintln!("trajectory_pruning samples=10000 accepted={accepted} rejected={rejected}");
    }
    #[test]
    #[ignore = "requires VOXY_CONTACT_BENCH_OBJ frozen OBJ fixture"]
    fn frozen_wall_pruning_work_and_time() {
        let path = std::env::var("VOXY_CONTACT_BENCH_OBJ").expect("set VOXY_CONTACT_BENCH_OBJ");
        let text = std::fs::read_to_string(&path).unwrap();
        let mut x = Vec::new();
        let mut faces = Vec::new();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            match fields.next() {
                Some("v") => x.push(std::array::from_fn(|_| {
                    fields.next().unwrap().parse::<f64>().unwrap()
                })),
                Some("f") => faces.push(std::array::from_fn(|_| {
                    fields.next().unwrap().parse::<usize>().unwrap() - 1
                })),
                _ => {}
            }
        }
        let contact = TissueSurfaceContact {
            faces,
            minimum_distance_m: 1e-6,
            activation_gap_m: 5e-5,
            pair_stiffness_n_m: 1.,
        };
        let candidates = contact.candidates(
            &x,
            None,
            contact.minimum_distance_m + contact.activation_gap_m,
        );
        let culled = candidates
            .iter()
            .filter(|&&(a, b)| {
                separation_lower_bound(
                    contact.faces[a].map(|i| x[i]),
                    contact.faces[b].map(|i| x[i]),
                ) >= contact.minimum_distance_m + contact.activation_gap_m
            })
            .count();
        let mut a = vec![[0.; 3]; x.len()];
        let mut b = a.clone();
        assert_eq!(
            contact.energy_gradient_impl::<true>(&x, &mut a).unwrap(),
            contact.energy_gradient_impl::<false>(&x, &mut b).unwrap()
        );
        assert_eq!(a, b);
        let mut timings = Vec::new();
        for optimized in [false, true] {
            let start = std::time::Instant::now();
            for _ in 0..100 {
                let mut g = vec![[0.; 3]; x.len()];
                let e = if optimized {
                    contact.energy_gradient_impl::<true>(&x, &mut g)
                } else {
                    contact.energy_gradient_impl::<false>(&x, &mut g)
                }
                .unwrap();
                std::hint::black_box((e, g));
            }
            timings.push(start.elapsed().as_secs_f64());
        }
        println!(
            "contact_pruning_bench path={path} faces={} candidates={} culled={} baseline_s={} optimized_s={}",
            contact.faces.len(),
            candidates.len(),
            culled,
            timings[0],
            timings[1]
        );
    }
}
