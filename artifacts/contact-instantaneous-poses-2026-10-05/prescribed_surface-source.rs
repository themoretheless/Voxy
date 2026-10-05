//! Externally prescribed triangle surfaces reuse the internal contact geometry/law.
use super::surface_contact::{barrier_curvature, barrier_response};
#[cfg(test)]
use super::surface_distance::triangle_pair_path_is_open;
use super::surface_distance::{
    PreparedTriangle, trajectory_point, triangle_distance, triangle_pair_path_rejection_time,
};
use super::{Vec3, add, cross, dot, scale, sub};
use crate::triangle_index::{TriangleBounds, TriangleIndex};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct PrescribedTriangleSurface {
    positions: Arc<[Vec3]>,
    faces: Arc<[[usize; 3]]>,
    contact_faces: Arc<[bool]>,
    minimum: f64,
    activation: f64,
    stiffness: f64,
    index: Arc<TriangleIndex>,
    prepared: Arc<[PreparedTriangle]>,
    swept_from: Option<(Arc<[Vec3]>, Arc<TriangleIndex>)>,
}
/// Positive normal-barrier block for a frozen closest feature. This is a
/// preconditioner, not the full Hessian of deforming triangle distance.
#[derive(Clone, Debug)]
pub struct PrescribedContactStencil {
    pub body_face: [usize; 3],
    pub obstacle_face: [usize; 3],
    pub obstacle_face_index: usize,
    pub body_weights: [f64; 3],
    pub obstacle_weights: [f64; 3],
    pub normal: Vec3,
    pub normal_curvature_n_m: f64,
}
impl PrescribedContactStencil {
    /// Apply the PSD normal block to simultaneous body/obstacle displacement.
    /// Translating both features equally gives zero action.
    #[must_use]
    pub fn apply(
        &self,
        body_displacement: [Vec3; 3],
        obstacle_displacement: [Vec3; 3],
    ) -> ([Vec3; 3], [Vec3; 3]) {
        let mut relative = [0.; 3];
        for i in 0..3 {
            relative = add(
                relative,
                sub(
                    scale(body_displacement[i], self.body_weights[i]),
                    scale(obstacle_displacement[i], self.obstacle_weights[i]),
                ),
            );
        }
        let value = scale(
            self.normal,
            self.normal_curvature_n_m * dot(self.normal, relative),
        );
        (
            self.body_weights.map(|weight| scale(value, weight)),
            self.obstacle_weights.map(|weight| scale(value, -weight)),
        )
    }
}
/// Five-point Gauss averages along simultaneous linear feature trajectories.
#[derive(Clone, Debug)]
pub struct PrescribedContactPathResponse {
    pub body_gradient_n: Vec<Vec3>,
    pub obstacle_gradient_n: Vec<Vec3>,
    /// Contact contribution to the midpoint unknown's line-search objective.
    /// Its gradient is the averaged body gradient; not stored physical energy.
    pub midpoint_objective_j: f64,
}
/// Nearest active geometric pair, retaining original source-face identities.
#[derive(Clone, Debug)]
pub struct PrescribedContactFeature {
    pub distance_m: f64,
    pub gap_m: f64,
    pub body_face: [usize; 3],
    pub obstacle_face_index: usize,
    pub body_weights: [f64; 3],
    pub obstacle_weights: [f64; 3],
}
#[derive(Clone, Debug)]
pub struct PrescribedSurfaceResponse {
    pub potential_j: f64,
    /// Potential derivatives, not physical forces; force is minus gradient.
    pub body_gradient_n: Vec<Vec3>,
    /// Opposite feature forces distributed to the obstacle's vertices.
    pub obstacle_gradient_n: Vec<Vec3>,
}
pub(super) struct PreparedPrescribedMotion<'a> {
    start: &'a PrescribedTriangleSurface,
    end: &'a PrescribedTriangleSurface,
    index: Arc<TriangleIndex>,
}
impl PreparedPrescribedMotion<'_> {
    pub(super) fn rejection_time(
        &self,
        body_start: &[Vec3],
        body_end: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<Option<f64>, &'static str> {
        self.start
            .path_rejection_time_indexed(self.end, body_start, body_end, faces, &self.index)
    }
}
pub(super) struct PreparedPrescribedContactPath {
    samples: Vec<(f64, f64, PrescribedTriangleSurface)>,
    boundaries: Vec<(f64, PrescribedTriangleSurface)>,
}
impl PreparedPrescribedContactPath {
    // Endpoint energy differences are an independent quadrature error
    // estimator only. They never replace force-integrated actuator work.
    pub(super) fn refine_by_work(
        &self,
        start: &[Vec3],
        end: &[Vec3],
        faces: &[[usize; 3]],
        tolerance: f64,
    ) -> Result<Option<Vec<f64>>, &'static str> {
        if start.len() != end.len() {
            return Err("body contact vertex count changed");
        }
        let body_pose = |time: f64| -> Vec<Vec3> {
            if time == 0. {
                return start.to_vec();
            }
            if time == 1. {
                return end.to_vec();
            }
            start
                .iter()
                .zip(end)
                .map(|(a, b)| trajectory_point(*a, *b, time))
                .collect()
        };
        let energies = self
            .boundaries
            .iter()
            .map(|(time, surface)| {
                surface
                    .response(&body_pose(*time), faces)
                    .map(|r| r.potential_j)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let obstacle_start = self.boundaries[0].1.positions();
        let obstacle_end = self.boundaries.last().unwrap().1.positions();
        let mut defects = Vec::new();
        for (panel, samples) in self.samples.chunks_exact(5).enumerate() {
            let mut work = 0.;
            for (time, weight, surface) in samples {
                let response = surface.response(&body_pose(*time), faces)?;
                work += weight
                    * (response
                        .body_gradient_n
                        .iter()
                        .zip(start.iter().zip(end))
                        .map(|(g, (a, b))| dot(*g, sub(*b, *a)))
                        .sum::<f64>()
                        + response
                            .obstacle_gradient_n
                            .iter()
                            .zip(obstacle_start.iter().zip(obstacle_end))
                            .map(|(g, (a, b))| dot(*g, sub(*b, *a)))
                            .sum::<f64>());
            }
            defects.push((energies[panel + 1] - energies[panel] - work).abs());
        }
        if defects.iter().any(|v| !v.is_finite()) {
            return Err("contact quadrature error overflow");
        }
        if defects.iter().sum::<f64>() <= tolerance || self.boundaries.len() >= 129 {
            return Ok(None);
        }
        let worst = defects
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .unwrap()
            .0;
        let left = self.boundaries[worst].0;
        let right = self.boundaries[worst + 1].0;
        let middle = left + 0.5 * (right - left);
        if middle <= left || middle >= right {
            return Ok(None);
        }
        let mut knots: Vec<_> = self.boundaries.iter().map(|b| b.0).collect();
        knots.insert(worst + 1, middle);
        Ok(Some(knots))
    }
    pub(super) fn refine_near(&self, time: f64) -> Option<Vec<f64>> {
        if !time.is_finite() || !(0. ..=1.).contains(&time) || self.boundaries.len() >= 129 {
            return None;
        }
        let panel = self
            .boundaries
            .windows(2)
            .position(|b| b[0].0 <= time && time <= b[1].0)?;
        let left = self.boundaries[panel].0;
        let right = self.boundaries[panel + 1].0;
        let middle = left + 0.5 * (right - left);
        if middle <= left || middle >= right {
            return None;
        }
        let mut knots: Vec<_> = self.boundaries.iter().map(|b| b.0).collect();
        knots.insert(panel + 1, middle);
        Some(knots)
    }
    pub(super) fn normal_stencils(
        &self,
        start: &[Vec3],
        end: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<Vec<PrescribedContactStencil>, &'static str> {
        if start.len() != end.len() {
            return Err("body contact vertex count changed");
        }
        validate_faces(start, faces)?;
        validate_faces(end, faces)?;
        let mut result = Vec::new();
        for &(time, weight, ref surface) in &self.samples {
            let body: Vec<_> = start
                .iter()
                .zip(end)
                .map(|(a, b)| trajectory_point(*a, *b, time))
                .collect();
            for mut block in surface.normal_stencils(&body, faces)? {
                // Endpoint = 2*midpoint-start; sampled positions have derivative
                // 2*time with respect to the midpoint unknown.
                block.normal_curvature_n_m *= 2. * time * weight;
                result.push(block);
            }
        }
        Ok(result)
    }
    pub(super) fn response(
        &self,
        start: &[Vec3],
        end: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<PrescribedContactPathResponse, &'static str> {
        if start.len() != end.len() {
            return Err("body contact vertex count changed");
        }
        validate_faces(start, faces)?;
        validate_faces(end, faces)?;
        let mut result = PrescribedContactPathResponse {
            body_gradient_n: vec![[0.; 3]; start.len()],
            obstacle_gradient_n: vec![[0.; 3]; self.samples[0].2.positions.len()],
            midpoint_objective_j: 0.,
        };
        for &(time, weight, ref surface) in &self.samples {
            let body: Vec<_> = start
                .iter()
                .zip(end)
                .map(|(a, b)| trajectory_point(*a, *b, time))
                .collect();
            let response = surface.response(&body, faces)?;
            result.midpoint_objective_j += 0.5 * weight / time * response.potential_j;
            for (sum, gradient) in result
                .body_gradient_n
                .iter_mut()
                .zip(response.body_gradient_n)
            {
                *sum = add(*sum, scale(gradient, weight));
            }
            for (sum, gradient) in result
                .obstacle_gradient_n
                .iter_mut()
                .zip(response.obstacle_gradient_n)
            {
                *sum = add(*sum, scale(gradient, weight));
            }
        }
        if !result.midpoint_objective_j.is_finite()
            || result
                .body_gradient_n
                .iter()
                .chain(&result.obstacle_gradient_n)
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err("prescribed path response overflow");
        }
        Ok(result)
    }
}

fn validate_faces(positions: &[Vec3], faces: &[[usize; 3]]) -> Result<(), &'static str> {
    if positions.is_empty()
        || positions.len() > 65536
        || faces.is_empty()
        || faces.len() > 131072
        || positions.iter().flatten().any(|value| !value.is_finite())
    {
        return Err("invalid prescribed surface geometry budget");
    }
    let mut seen = std::collections::BTreeSet::new();
    for face in faces {
        let mut key = *face;
        key.sort_unstable();
        if key[2] >= positions.len() || key.windows(2).any(|p| p[0] == p[1]) || !seen.insert(key) {
            return Err("invalid or duplicate prescribed surface triangle");
        }
        let [a, b, c] = face.map(|node| positions[node]);
        let normal = cross(sub(b, a), sub(c, a));
        let area = dot(normal, normal);
        if !area.is_finite() || area <= 1e-30 {
            return Err("degenerate contact triangle");
        }
    }
    Ok(())
}
impl PrescribedTriangleSurface {
    /// Immutable surface geometry and discrete per-triangle-pair barrier controls.
    /// No mesh-independent material calibration is implied by the coefficient.
    /// # Errors
    /// Nonfinite/invalid controls, budgets, indices, duplicate or degenerate faces.
    pub fn new(
        positions: Vec<Vec3>,
        faces: Vec<[usize; 3]>,
        minimum_distance_m: f64,
        activation_gap_m: f64,
        pair_stiffness_n_m: f64,
    ) -> Result<Self, &'static str> {
        if [minimum_distance_m, activation_gap_m, pair_stiffness_n_m]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
            || !(minimum_distance_m + activation_gap_m).is_finite()
        {
            return Err("invalid prescribed surface contact controls");
        }
        validate_faces(&positions, &faces)?;
        let triangles: Vec<_> = faces
            .iter()
            .map(|face| face.map(|node| positions[node]))
            .collect();
        let index = Arc::new(TriangleIndex::new(&triangles));
        let prepared = triangles
            .into_iter()
            .map(PreparedTriangle::new)
            .collect::<Vec<_>>()
            .into();
        Ok(Self {
            index,
            prepared,
            swept_from: None,
            positions: positions.into(),
            contact_faces: vec![true; faces.len()].into(),
            faces: faces.into(),
            minimum: minimum_distance_m,
            activation: activation_gap_m,
            stiffness: pair_stiffness_n_m,
        })
    }
    /// Stages the next pose without mutating the admitted surface or its identity.
    /// # Errors
    /// Changed vertex count or invalid/degenerate next geometry.
    pub fn with_positions(&self, positions: Vec<Vec3>) -> Result<Self, &'static str> {
        self.stage_positions::<true>(positions)
    }
    // Quadrature samples are immutable instantaneous poses. Their callers do
    // not query motion from the source pose, so avoid the unused swept BVH.
    fn stage_positions<const SWEPT: bool>(
        &self,
        positions: Vec<Vec3>,
    ) -> Result<Self, &'static str> {
        if positions.len() != self.positions.len() {
            return Err("prescribed surface vertex count changed");
        }
        validate_faces(&positions, &self.faces)?;
        let triangles: Vec<_> = self
            .faces
            .iter()
            .map(|face| face.map(|node| positions[node]))
            .collect();
        let mut index = (*self.index).clone();
        index.refit(&triangles);
        let swept_from = if SWEPT {
            let mut swept = (*self.index).clone();
            swept.refit_bounds(
                &self
                    .faces
                    .iter()
                    .map(|face| {
                        TriangleBounds::swept(
                            face.map(|node| self.positions[node]),
                            face.map(|node| positions[node]),
                        )
                    })
                    .collect::<Vec<_>>(),
            );
            Some((self.positions.clone(), Arc::new(swept)))
        } else {
            None
        };
        let prepared = triangles
            .into_iter()
            .map(PreparedTriangle::new)
            .collect::<Vec<_>>()
            .into();
        Ok(Self {
            index: Arc::new(index),
            prepared,
            swept_from,
            positions: positions.into(),
            ..self.clone()
        })
    }
    #[must_use]
    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }
    #[must_use]
    pub fn faces(&self) -> &[[usize; 3]] {
        &self.faces
    }
    /// Author an immutable contact domain using original triangle indices.
    /// False faces remain in geometry but do not exert contact or constrain CCD.
    /// This creates a new contact owner; bind it explicitly before animation.
    /// # Errors
    /// Wrong mask length or empty contact domain.
    pub fn with_contact_faces(&self, enabled: Vec<bool>) -> Result<Self, &'static str> {
        if enabled.len() != self.faces.len() || !enabled.iter().any(|&v| v) {
            return Err("invalid prescribed surface contact domain");
        }
        Ok(Self {
            contact_faces: enabled.into(),
            ..self.clone()
        })
    }
    #[must_use]
    pub fn contact_faces(&self) -> &[bool] {
        &self.contact_faces
    }
    pub(super) fn same_owner(&self, other: &Self) -> Result<(), &'static str> {
        if !Arc::ptr_eq(&self.faces, &other.faces)
            || !Arc::ptr_eq(&self.contact_faces, &other.contact_faces)
            || self.minimum != other.minimum
            || self.activation != other.activation
            || self.stiffness != other.stiffness
        {
            return Err("prescribed surface identity or contact law changed");
        }
        Ok(())
    }
    /// Average contact gradients over a linear body/obstacle path.
    /// Quadrature never infers work from endpoint energy. CCD is caller-owned.
    /// # Errors
    /// Changed owner, invalid geometry, closed sampled gap or overflow.
    pub fn path_response(
        &self,
        next: &Self,
        start: &[Vec3],
        end: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<PrescribedContactPathResponse, &'static str> {
        self.prepare_path(next)?.response(start, end, faces)
    }
    pub(super) fn prepare_path(
        &self,
        next: &Self,
    ) -> Result<PreparedPrescribedContactPath, &'static str> {
        self.prepare_path_panels(next, 1)
    }
    pub(super) fn prepare_path_panels(
        &self,
        next: &Self,
        panels: usize,
    ) -> Result<PreparedPrescribedContactPath, &'static str> {
        if ![1, 2, 4, 8, 16].contains(&panels) {
            return Err("invalid contact quadrature panels");
        }
        let knots: Vec<_> = (0..=panels).map(|i| i as f64 / panels as f64).collect();
        self.prepare_path_partition(next, &knots)
    }
    pub(super) fn prepare_path_partition(
        &self,
        next: &Self,
        knots: &[f64],
    ) -> Result<PreparedPrescribedContactPath, &'static str> {
        self.same_owner(next)?;
        if knots.len() < 2
            || knots.len() > 129
            || knots[0] != 0.
            || knots[knots.len() - 1] != 1.
            || knots.windows(2).any(|p| !p[0].is_finite() || p[0] >= p[1])
        {
            return Err("invalid contact quadrature partition");
        }
        let pose = |time: f64| {
            if time == 0. {
                return Ok(self.clone());
            }
            if time == 1. {
                return Ok(next.clone());
            }
            self.stage_positions::<false>(
                self.positions
                    .iter()
                    .zip(next.positions.iter())
                    .map(|(a, b)| trajectory_point(*a, *b, time))
                    .collect(),
            )
        };
        let mut samples = Vec::new();
        for interval in knots.windows(2) {
            let width = interval[1] - interval[0];
            for (local_time, local_weight) in [
                (0.046910077030668, 0.11846344252809454),
                (0.23076534494715845, 0.23931433524968325),
                (0.5, 0.28444444444444444),
                (0.7692346550528415, 0.23931433524968325),
                (0.953089922969332, 0.11846344252809454),
            ] {
                let time = interval[0] + width * local_time;
                samples.push((time, width * local_weight, pose(time)?));
            }
        }
        let boundaries = knots
            .iter()
            .map(|&time| Ok((time, pose(time)?)))
            .collect::<Result<Vec<_>, &'static str>>()?;
        Ok(PreparedPrescribedContactPath {
            samples,
            boundaries,
        })
    }
    /// Normal barrier blocks for a contact-aware nonlinear preconditioner.
    /// Closest-feature and normal derivatives are intentionally not included.
    /// # Errors
    /// Invalid geometry, closed separation or overflowing curvature.
    pub fn normal_stencils(
        &self,
        body: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<Vec<PrescribedContactStencil>, &'static str> {
        self.normal_stencils_impl::<true>(body, faces)
    }
    fn normal_stencils_impl<const PRUNED: bool>(
        &self,
        body: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<Vec<PrescribedContactStencil>, &'static str> {
        validate_faces(body, faces)?;
        let mut stencils = Vec::new();
        for &face in faces {
            let triangle = face.map(|node| body[node]);
            let prepared = PreparedTriangle::new(triangle);
            let mut candidates = Vec::new();
            self.index.query_conservative(
                TriangleBounds::triangle(triangle),
                self.minimum + self.activation,
                &mut candidates,
            );
            candidates.sort_unstable();
            for index in candidates {
                if !self.contact_faces[index] {
                    continue;
                }
                // Same conservative lower bound as force evaluation. Only
                // inactive barriers are skipped; exact active feature order and
                // errors still come from the original distance/law evaluation.
                if PRUNED
                    && prepared.separation_lower_bound(&self.prepared[index])
                        >= self.minimum + self.activation
                {
                    continue;
                }
                let obstacle_face = self.faces[index];
                let closest =
                    triangle_distance(triangle, obstacle_face.map(|node| self.positions[node]))?;
                let (_, derivative) = barrier_response(
                    closest.distance,
                    self.minimum,
                    self.activation,
                    self.stiffness,
                )?;
                if derivative == 0. {
                    continue;
                }
                let curvature = barrier_curvature(
                    closest.distance - self.minimum,
                    self.activation,
                    self.stiffness,
                );
                if !curvature.is_finite() || curvature < 0. {
                    return Err("prescribed contact curvature overflow");
                }
                stencils.push(PrescribedContactStencil {
                    body_face: face,
                    obstacle_face,
                    obstacle_face_index: index,
                    body_weights: closest.a,
                    obstacle_weights: closest.b,
                    normal: scale(closest.delta, 1. / closest.distance),
                    normal_curvature_n_m: curvature,
                });
            }
        }
        Ok(stencils)
    }
    /// Inspect the nearest enabled pair inside the barrier activation range.
    /// This read-only query also reports closed gaps without modifying geometry.
    /// # Errors
    /// Invalid body triangles or closest-feature geometry.
    pub fn nearest_active_contact(
        &self,
        body: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<Option<PrescribedContactFeature>, &'static str> {
        validate_faces(body, faces)?;
        let mut nearest: Option<PrescribedContactFeature> = None;
        for &face in faces {
            let triangle = face.map(|node| body[node]);
            let mut candidates = Vec::new();
            self.index.query_conservative(
                TriangleBounds::triangle(triangle),
                self.minimum + self.activation,
                &mut candidates,
            );
            candidates.sort_unstable();
            for index in candidates {
                if !self.contact_faces[index] {
                    continue;
                }
                let closest = triangle_distance(
                    triangle,
                    self.faces[index].map(|node| self.positions[node]),
                )?;
                if closest.distance >= self.minimum + self.activation
                    || nearest
                        .as_ref()
                        .is_some_and(|old| old.distance_m <= closest.distance)
                {
                    continue;
                }
                nearest = Some(PrescribedContactFeature {
                    distance_m: closest.distance,
                    gap_m: closest.distance - self.minimum,
                    body_face: face,
                    obstacle_face_index: index,
                    body_weights: closest.a,
                    obstacle_weights: closest.b,
                });
            }
        }
        Ok(nearest)
    }
    /// Build an uncommitted, mass-weighted normal separation guess.
    /// The caller must still certify its complete motion and solve equilibrium.
    pub(super) fn restore_separation_guess(
        &self,
        body: &[Vec3],
        faces: &[[usize; 3]],
        pinned: &[bool],
        masses: &[f64],
    ) -> Result<Vec<Vec3>, &'static str> {
        if body.len() != pinned.len()
            || body.len() != masses.len()
            || masses.iter().any(|m| !m.is_finite() || *m <= 0.)
        {
            return Err("invalid contact guess masses");
        }
        let mut guess = body.to_vec();
        let target_gap = self.activation * 0.001;
        for _ in 0..32 {
            let Some(feature) = self.nearest_active_contact(&guess, faces)? else {
                return Ok(guess);
            };
            if feature.gap_m >= target_gap * 0.5 {
                return Ok(guess);
            }
            let closest = triangle_distance(
                feature.body_face.map(|i| guess[i]),
                self.faces[feature.obstacle_face_index].map(|i| self.positions[i]),
            )?;
            if closest.distance <= 0. {
                return Err("intersecting contact guess has no normal");
            }
            let normal = scale(closest.delta, 1. / closest.distance);
            let denominator: f64 = feature
                .body_face
                .iter()
                .zip(feature.body_weights)
                .filter(|(node, _)| !pinned[**node])
                .map(|(&node, w)| w * w / masses[node])
                .sum();
            if !denominator.is_finite() || denominator <= 0. {
                return Err("contact guess has no free feature");
            }
            let multiplier = (target_gap - feature.gap_m) / denominator;
            for (node, weight) in feature.body_face.into_iter().zip(feature.body_weights) {
                if !pinned[node] {
                    guess[node] = add(
                        guess[node],
                        scale(normal, multiplier * weight / masses[node]),
                    );
                }
            }
            if guess.iter().flatten().any(|v| !v.is_finite()) {
                return Err("contact guess overflow");
            }
        }
        Err("contact guess restoration nonconvergence")
    }
    /// Cross-surface triangle-minimum barrier, with both feature gradients.
    /// Uses the same closest features and barrier as Body's surface contact.
    /// # Errors
    /// Invalid body triangles, closed separation gap or nonfinite responses.
    pub fn response(
        &self,
        body: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<PrescribedSurfaceResponse, &'static str> {
        self.response_impl::<true>(body, faces)
    }
    fn response_impl<const INDEXED: bool>(
        &self,
        body: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<PrescribedSurfaceResponse, &'static str> {
        validate_faces(body, faces)?;
        let mut response = PrescribedSurfaceResponse {
            potential_j: 0.,
            body_gradient_n: vec![[0.; 3]; body.len()],
            obstacle_gradient_n: vec![[0.; 3]; self.positions.len()],
        };
        for face in faces {
            let triangle = face.map(|node| body[node]);
            let prepared = PreparedTriangle::new(triangle);
            let mut candidates = Vec::new();
            if INDEXED {
                self.index.query_conservative(
                    TriangleBounds::triangle(triangle),
                    self.minimum + self.activation,
                    &mut candidates,
                );
            } else {
                candidates.extend(0..self.faces.len());
            }
            candidates.sort_unstable();
            for index in candidates {
                if !self.contact_faces[index] {
                    continue;
                }
                let obstacle = &self.faces[index];
                if prepared.separation_lower_bound(&self.prepared[index])
                    >= self.minimum + self.activation
                {
                    continue;
                }
                let closest =
                    triangle_distance(triangle, obstacle.map(|node| self.positions[node]))?;
                let (energy, derivative) = barrier_response(
                    closest.distance,
                    self.minimum,
                    self.activation,
                    self.stiffness,
                )?;
                response.potential_j += energy;
                if derivative == 0. {
                    continue;
                }
                let gradient = scale(closest.delta, derivative / closest.distance);
                for corner in 0..3 {
                    let node = face[corner];
                    response.body_gradient_n[node] = add(
                        response.body_gradient_n[node],
                        scale(gradient, closest.a[corner]),
                    );
                    let node = obstacle[corner];
                    response.obstacle_gradient_n[node] = sub(
                        response.obstacle_gradient_n[node],
                        scale(gradient, closest.b[corner]),
                    );
                }
            }
        }
        if !response.potential_j.is_finite()
            || response
                .body_gradient_n
                .iter()
                .chain(&response.obstacle_gradient_n)
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err("prescribed surface contact response overflow");
        }
        Ok(response)
    }
    /// Certify minimum separation during simultaneous linear vertex motion.
    /// Unknown conservative-advancement intervals return false for subdivision.
    /// # Errors
    /// Changed obstacle identity/law or invalid endpoint geometry.
    pub fn path_is_open(
        &self,
        next: &Self,
        body_start: &[Vec3],
        body_end: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<bool, &'static str> {
        self.path_rejection_time(next, body_start, body_end, faces)
            .map(|time| time.is_none())
    }
    pub(super) fn path_rejection_time(
        &self,
        next: &Self,
        body_start: &[Vec3],
        body_end: &[Vec3],
        faces: &[[usize; 3]],
    ) -> Result<Option<f64>, &'static str> {
        self.prepare_motion(next)?
            .rejection_time(body_start, body_end, faces)
    }
    pub(super) fn prepare_motion<'a>(
        &'a self,
        next: &'a Self,
    ) -> Result<PreparedPrescribedMotion<'a>, &'static str> {
        self.same_owner(next)?;
        let index = if Arc::ptr_eq(&self.positions, &next.positions) {
            self.index.clone()
        } else if let Some((source, index)) = &next.swept_from {
            if Arc::ptr_eq(source, &self.positions) {
                index.clone()
            } else {
                self.swept_index(next)
            }
        } else {
            self.swept_index(next)
        };
        Ok(PreparedPrescribedMotion {
            start: self,
            end: next,
            index,
        })
    }
    fn path_rejection_time_indexed(
        &self,
        next: &Self,
        body_start: &[Vec3],
        body_end: &[Vec3],
        faces: &[[usize; 3]],
        index: &TriangleIndex,
    ) -> Result<Option<f64>, &'static str> {
        if body_start.len() != body_end.len() {
            return Err("body contact vertex count changed");
        }
        validate_faces(body_start, faces)?;
        validate_faces(body_end, faces)?;
        for face in faces {
            let mut candidates = Vec::new();
            index.query_conservative(
                TriangleBounds::swept(
                    face.map(|node| body_start[node]),
                    face.map(|node| body_end[node]),
                ),
                self.minimum,
                &mut candidates,
            );
            candidates.sort_unstable();
            for index in candidates {
                if !self.contact_faces[index] {
                    continue;
                }
                let obstacle = &self.faces[index];
                if let Some(time) = triangle_pair_path_rejection_time::<true>(
                    face.map(|node| body_start[node]),
                    face.map(|node| body_end[node]),
                    obstacle.map(|node| self.positions[node]),
                    obstacle.map(|node| next.positions[node]),
                    self.minimum,
                ) {
                    if std::env::var_os("VOXY_CCD_REJECTION_TRACE").is_some() {
                        eprintln!(
                            "PRESCRIBED_CCD_REJECTION body_face={face:?} obstacle_face_index={index}"
                        );
                    }
                    return Ok(Some(time));
                }
            }
        }
        Ok(None)
    }
    fn swept_index(&self, next: &Self) -> Arc<TriangleIndex> {
        let mut index = (*self.index).clone();
        index.refit_bounds(
            &self
                .faces
                .iter()
                .map(|face| {
                    TriangleBounds::swept(
                        face.map(|node| self.positions[node]),
                        face.map(|node| next.positions[node]),
                    )
                })
                .collect::<Vec<_>>(),
        );
        Arc::new(index)
    }
    /// Independent trapezoidal actuator work from obstacle vertex gradients.
    /// # Errors
    /// Changed identity/law, incomplete/nonfinite gradients or overflowing work.
    pub fn motion_work(
        &self,
        next: &Self,
        before: &[Vec3],
        after: &[Vec3],
    ) -> Result<f64, &'static str> {
        self.same_owner(next)?;
        if before.len() != self.positions.len()
            || after.len() != self.positions.len()
            || before.iter().chain(after).flatten().any(|v| !v.is_finite())
        {
            return Err("invalid prescribed surface work gradients");
        }
        let mut work = 0.;
        for node in 0..self.positions.len() {
            let average =
                std::array::from_fn(|axis| 0.5 * before[node][axis] + 0.5 * after[node][axis]);
            work += dot(average, sub(next.positions[node], self.positions[node]));
        }
        if !work.is_finite() {
            return Err("prescribed surface actuator work overflow");
        }
        Ok(work)
    }
}

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn prepared_motion_keeps_the_actual_source_when_cached_source_differs() {
        let source = PrescribedTriangleSurface::new(
            vec![[-1., -1., 1.], [1., -1., 1.], [0., 1., 1.]],
            vec![[0, 1, 2]],
            0.0001,
            0.003,
            100.,
        )
        .unwrap();
        let actual = source
            .with_positions(
                source
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], -0.001])
                    .collect(),
            )
            .unwrap();
        let next = source
            .with_positions(
                source
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], 0.001])
                    .collect(),
            )
            .unwrap();
        let prepared = actual.prepare_motion(&next).unwrap();
        let saved_index = prepared.index.clone();
        for height in [-0.005, 0., 0.005] {
            let body = [[0.1, 0.1, height], [0.15, 0.1, height], [0.1, 0.15, height]];
            let result = prepared.rejection_time(&body, &body, &[[0, 1, 2]]).unwrap();
            assert_eq!(
                result,
                actual
                    .path_rejection_time(&next, &body, &body, &[[0, 1, 2]])
                    .unwrap()
            );
            assert_eq!(result.is_some(), height == 0.);
            assert!(Arc::ptr_eq(&prepared.index, &saved_index));
        }
        assert!(
            source
                .prepare_motion(
                    &PrescribedTriangleSurface::new(
                        source.positions().to_vec(),
                        source.faces().to_vec(),
                        0.0001,
                        0.003,
                        100.
                    )
                    .unwrap()
                )
                .is_err()
        );
    }
    #[test]
    fn rejection_location_refines_only_its_interval() {
        let surface = PrescribedTriangleSurface::new(
            vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
            vec![[0, 1, 2]],
            0.0001,
            0.003,
            100.,
        )
        .unwrap();
        let path = surface
            .prepare_path_partition(&surface, &[0., 0.5, 0.75, 1.])
            .unwrap();
        assert_eq!(
            path.refine_near(0.99).unwrap(),
            vec![0., 0.5, 0.75, 0.875, 1.]
        );
        assert_eq!(
            path.refine_near(0.625).unwrap(),
            vec![0., 0.5, 0.625, 0.75, 1.]
        );
        assert!(path.refine_near(f64::NAN).is_none());
        assert!(path.refine_near(-0.1).is_none());
        assert!(path.refine_near(1.1).is_none());
        let body = [[0.1, 0.1, 0.001], [0.15, 0.1, 0.025], [0.1, 0.15, 0.025]];
        assert!(
            surface
                .path_rejection_time(&surface, &body, &body, &[[0, 1, 2]])
                .unwrap()
                .is_none()
        );
        let crossing = surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], 0.002])
                    .collect(),
            )
            .unwrap();
        let time = surface
            .path_rejection_time(&crossing, &body, &body, &[[0, 1, 2]])
            .unwrap()
            .unwrap();
        assert!((time - 0.45).abs() < 1e-7, "rejection time {time}");
        assert!(
            !surface
                .path_is_open(&crossing, &body, &body, &[[0, 1, 2]])
                .unwrap()
        );
    }
    #[test]
    fn adaptive_partition_resolves_endpoint_barrier_without_redefining_work() {
        let surface = PrescribedTriangleSurface::new(
            vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
            vec![[0, 1, 2]],
            0.0001,
            0.003,
            100.,
        )
        .unwrap();
        let movement = 41.958e-9;
        let next = surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], movement])
                    .collect(),
            )
            .unwrap();
        let body = [
            [0.1, 0.1, 0.000100042],
            [0.15, 0.1, 0.025],
            [0.1, 0.15, 0.025],
        ];
        let faces = [[0, 1, 2]];
        let delta = next.response(&body, &faces).unwrap().potential_j
            - surface.response(&body, &faces).unwrap().potential_j;
        let mut knots = vec![0., 1.];
        let mut admitted = false;
        for _ in 0..32 {
            let path = surface.prepare_path_partition(&next, &knots).unwrap();
            let response = path.response(&body, &body, &faces).unwrap();
            let work: f64 = response
                .obstacle_gradient_n
                .iter()
                .map(|g| g[2] * movement)
                .sum();
            if (delta - work).abs() < 1e-10 {
                admitted = true;
                break;
            }
            knots = path
                .refine_by_work(&body, &body, &faces, 2.5e-11)
                .unwrap()
                .unwrap();
        }
        assert!(
            admitted,
            "unresolved endpoint barrier with {} panels",
            knots.len() - 1
        );
        assert!(knots.len() > 2 && knots.len() < 33);
        assert!(knots.windows(2).any(|p| p[1] - p[0] < 1e-3));
        for invalid in [
            vec![],
            vec![0., 0., 1.],
            vec![0., f64::NAN, 1.],
            vec![0., 0.5],
        ] {
            assert!(surface.prepare_path_partition(&next, &invalid).is_err());
        }
        let held = surface.prepare_path_partition(&surface, &[0., 1.]).unwrap();
        assert!(
            held.refine_by_work(&body, &body, &faces, 1e-12)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn path_normal_blocks_match_midpoint_force_derivative_in_narrow_gap() {
        let surface = PrescribedTriangleSurface::new(
            vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
            vec![[0, 1, 2]],
            0.0001,
            0.003,
            100.,
        )
        .unwrap();
        let next = surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], 0.000000138])
                    .collect(),
            )
            .unwrap();
        let path = surface.prepare_path(&next).unwrap();
        let body = [
            [0.1, 0.1, 0.000100435],
            [0.15, 0.1, 0.025],
            [0.1, 0.15, 0.025],
        ];
        let blocks = path.normal_stencils(&body, &body, &[[0, 1, 2]]).unwrap();
        assert_eq!(blocks.len(), 5);
        let action: f64 = blocks
            .iter()
            .map(|block| {
                block
                    .apply([[0., 0., 1.]; 3], [[0.; 3]; 3])
                    .0
                    .iter()
                    .map(|g| g[2])
                    .sum::<f64>()
            })
            .sum();
        let h = 1e-11;
        let plus = body.map(|p| [p[0], p[1], p[2] + h]);
        let minus = body.map(|p| [p[0], p[1], p[2] - h]);
        let derivative = (path
            .response(&body, &plus, &[[0, 1, 2]])
            .unwrap()
            .body_gradient_n
            .iter()
            .map(|g| g[2])
            .sum::<f64>()
            - path
                .response(&body, &minus, &[[0, 1, 2]])
                .unwrap()
                .body_gradient_n
                .iter()
                .map(|g| g[2])
                .sum::<f64>())
            / (2. * h);
        assert!((action / (2. * derivative) - 1.).abs() < 1e-6);
        assert!(blocks.iter().all(|block| block.normal_curvature_n_m > 0.));
        assert!(
            path.normal_stencils(&body, &plus[..2], &[[0, 1, 2]])
                .is_err()
        );
    }
    #[test]
    fn prepared_quadrature_poses_remain_bound_to_their_original_motion() {
        let surface = PrescribedTriangleSurface::new(
            vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
            vec![[0, 1, 2]],
            0.001,
            0.03,
            100.,
        )
        .unwrap();
        let next = surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], 0.001])
                    .collect(),
            )
            .unwrap();
        let prepared = surface.prepare_path(&next).unwrap();
        let different = next
            .with_positions(
                next.positions()
                    .iter()
                    .map(|p| [p[0], p[1], 0.002])
                    .collect(),
            )
            .unwrap();
        let start = [[0.1, 0.1, 0.015], [0.15, 0.1, 0.04], [0.1, 0.15, 0.04]];
        for i in 0..32 {
            let end = start.map(|p| [p[0] + i as f64 * 0.001, p[1], p[2] + 0.0001]);
            let cached = prepared.response(&start, &end, &[[0, 1, 2]]).unwrap();
            let fresh = surface
                .path_response(&next, &start, &end, &[[0, 1, 2]])
                .unwrap();
            assert_eq!(cached.body_gradient_n, fresh.body_gradient_n);
            assert_eq!(cached.obstacle_gradient_n, fresh.obstacle_gradient_n);
            assert_eq!(cached.midpoint_objective_j, fresh.midpoint_objective_j);
            assert_ne!(
                cached.midpoint_objective_j,
                surface
                    .path_response(&different, &start, &end, &[[0, 1, 2]])
                    .unwrap()
                    .midpoint_objective_j
            );
        }
    }
    #[test]
    fn motion_cache_from_different_source_cannot_hide_crossing() {
        let triangle = |z| vec![[0., 0., z], [1., 0., z], [0., 1., z]];
        let start =
            PrescribedTriangleSurface::new(triangle(-1.), vec![[0, 1, 2]], 0.001, 0.1, 3.).unwrap();
        let unrelated_source = start.with_positions(triangle(2.)).unwrap();
        let end = unrelated_source.with_positions(triangle(3.)).unwrap();
        let body = triangle(0.);
        assert!(
            !start
                .path_is_open(&end, &body, &body, &[[0, 1, 2]])
                .unwrap()
        );
        assert!(
            unrelated_source
                .path_is_open(&end, &body, &body, &[[0, 1, 2]])
                .unwrap()
        );
    }
    #[test]
    fn indexed_motion_admission_matches_all_pairs_for_endpoint_trajectories() {
        let mut points = Vec::new();
        let mut faces = Vec::new();
        for i in 0..64 {
            let x = i as f64 * 2.;
            let n = points.len();
            points.extend([[x, 0., 0.], [x + 0.8, 0., 0.], [x, 0.8, 0.]]);
            faces.push([n, n + 1, n + 2]);
        }
        let start = PrescribedTriangleSurface::new(points, faces, 0.001, 0.1, 3.).unwrap();
        let next = start
            .with_positions(
                start
                    .positions()
                    .iter()
                    .map(|p| [p[0] + 0.2, p[1], p[2] + 0.1])
                    .collect(),
            )
            .unwrap();
        for i in 0..1000 {
            let x = (i % 80) as f64 * 1.7;
            let z = if i % 2 == 0 { -0.2 } else { 0.3 };
            let body = [[x, 0.1, z], [x + 0.3, 0.1, z], [x, 0.4, z]];
            let end = body.map(|p| [p[0] + 0.05, p[1], 0.4]);
            let full = start.faces.iter().all(|face| {
                triangle_pair_path_is_open::<true>(
                    body,
                    end,
                    face.map(|n| start.positions[n]),
                    face.map(|n| next.positions[n]),
                    start.minimum,
                )
            });
            assert_eq!(
                start
                    .path_is_open(&next, &body, &end, &[[0, 1, 2]])
                    .unwrap(),
                full,
                "trajectory {i}"
            );
        }
    }
    #[test]
    fn indexed_energy_and_both_gradients_match_full_scan_after_refit() {
        let mut points = Vec::new();
        let mut faces = Vec::new();
        for i in 0..128 {
            let x = i as f64 * 0.4;
            let n = points.len();
            points.extend([[x, 0., 0.], [x + 0.2, 0., 0.], [x, 0.2, 0.]]);
            faces.push([n, n + 1, n + 2]);
        }
        let surface = PrescribedTriangleSurface::new(points, faces, 0.001, 0.1, 3.).unwrap();
        let next = surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0] + 0.01, p[1], p[2] + 0.005])
                    .collect(),
            )
            .unwrap();
        for obstacle in [&surface, &next] {
            for i in 0..128 {
                let x = i as f64 * 0.4;
                let body = [
                    [x + 0.02, 0.02, 0.03],
                    [x + 0.12, 0.02, 0.04],
                    [x + 0.02, 0.12, 0.05],
                ];
                let indexed = obstacle.response_impl::<true>(&body, &[[0, 1, 2]]).unwrap();
                let full = obstacle
                    .response_impl::<false>(&body, &[[0, 1, 2]])
                    .unwrap();
                assert_eq!(indexed.potential_j, full.potential_j);
                assert_eq!(indexed.body_gradient_n, full.body_gradient_n);
                assert_eq!(indexed.obstacle_gradient_n, full.obstacle_gradient_n);
            }
        }
    }
}

#[cfg(test)]
mod local_displacement_precision_tests {
    use super::*;
    #[test]
    fn local_path_retains_contact_change_below_world_coordinate_resolution() {
        let world_start = vec![
            [0., 0., 1.0001000000001],
            [0.02, 0., 1.05],
            [0., 0.02, 1.05],
        ];
        let origin = world_start[0];
        let surface = PrescribedTriangleSurface::new(
            vec![[-1., -1., 1.], [1., -1., 1.], [0., 1., 1.]],
            vec![[0, 1, 2]],
            0.0001,
            0.003,
            100.,
        )
        .unwrap();
        let shifted = surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|&p| sub(p, origin))
                    .collect(),
            )
            .unwrap();
        let local_start: Vec<_> = world_start.iter().map(|&p| sub(p, origin)).collect();
        let mut local_end = local_start.clone();
        local_end[0][2] += 2e-18;
        let mut world_end = world_start.clone();
        world_end[0][2] += 2e-18;
        assert_eq!(world_end, world_start);
        let path = shifted.prepare_path_partition(&shifted, &[0., 1.]).unwrap();
        let before = path
            .response(&local_start, &local_start, &[[0, 1, 2]])
            .unwrap();
        let after = path
            .response(&local_start, &local_end, &[[0, 1, 2]])
            .unwrap();
        assert!(after.body_gradient_n[0][2].abs() < before.body_gradient_n[0][2].abs());
        assert!(after.midpoint_objective_j < before.midpoint_objective_j);
        assert!(
            after
                .body_gradient_n
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
    }
}

#[cfg(test)]
mod restored_guess_tests {
    use super::*;
    fn surface() -> PrescribedTriangleSurface {
        PrescribedTriangleSurface::new(
            vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
            vec![[0, 1, 2]],
            0.0001,
            0.003,
            100.,
        )
        .unwrap()
    }
    #[test]
    fn shallow_closed_guess_restores_separation_without_moving_pins_or_source() {
        let surface = surface();
        let body = vec![
            [-0.1, -0.1, 0.00009995],
            [0.1, -0.1, 0.00009995],
            [0., 0.1, 0.00009995],
            [2., 2., 2.],
        ];
        let saved = body.clone();
        let source = surface.positions().to_vec();
        assert!(surface.response(&body, &[[0, 1, 2]]).is_err());
        let restored = surface
            .restore_separation_guess(
                &body,
                &[[0, 1, 2]],
                &[false, false, false, true],
                &[1., 2., 3., 4.],
            )
            .unwrap();
        assert!(surface.response(&restored, &[[0, 1, 2]]).is_ok());
        assert!(
            surface
                .nearest_active_contact(&restored, &[[0, 1, 2]])
                .unwrap()
                .unwrap()
                .gap_m
                >= 1.5e-6
        );
        assert_eq!(restored[3], body[3]);
        assert_eq!(body, saved);
        assert_eq!(surface.positions(), source);
    }
    #[test]
    fn closed_fully_pinned_feature_cannot_be_restored() {
        let surface = surface();
        let body = vec![
            [-0.1, -0.1, 0.00009995],
            [0.1, -0.1, 0.00009995],
            [0., 0.1, 0.00009995],
        ];
        assert_eq!(
            surface
                .restore_separation_guess(&body, &[[0, 1, 2]], &[true; 3], &[1.; 3])
                .unwrap_err(),
            "contact guess has no free feature"
        );
    }
}

#[cfg(test)]
mod normal_pruning_tests {
    use super::*;
    fn fixture(offset: f64) -> (PrescribedTriangleSurface, Vec<Vec3>) {
        let triangle = [[-1., -1., -2.], [1., -1., 0.], [0., 1., 1.]];
        let n = scale([1., 1., -1.], 1. / 3_f64.sqrt());
        let shift = [offset, -offset, offset];
        let body: Vec<_> = triangle.map(|p| add(p, shift)).to_vec();
        let mut positions = Vec::new();
        let mut faces = Vec::new();
        for i in 0..256 {
            let gap = if i == 0 {
                0.001
            } else {
                0.01 + i as f64 * 0.0001
            };
            let base = positions.len();
            positions.extend(triangle.map(|p| add(add(p, shift), scale(n, gap))));
            faces.push([base, base + 1, base + 2]);
        }
        (
            PrescribedTriangleSurface::new(positions, faces, 0.0001, 0.003, 100.).unwrap(),
            body,
        )
    }
    #[test]
    fn conservative_normal_pruning_matches_unpruned_features_and_errors() {
        for offset in [0., 1e6] {
            let (surface, body) = fixture(offset);
            for gap in [
                -0.0021000001,
                -0.0021,
                -0.0020999999,
                0.0040999999,
                0.0041,
                0.0041000001,
                0.,
                0.0000999999,
                0.0001,
                0.001,
                0.0030999999,
                0.0031,
                0.0031000001,
                0.1,
            ] {
                let moved: Vec<_> = body
                    .iter()
                    .map(|p| add(*p, scale([1., 1., -1.], gap / 3_f64.sqrt())))
                    .collect();
                let a = surface.normal_stencils_impl::<true>(&moved, &[[0, 1, 2]]);
                let b = surface.normal_stencils_impl::<false>(&moved, &[[0, 1, 2]]);
                match (a, b) {
                    (Ok(a), Ok(b)) => {
                        assert_eq!(a.len(), b.len());
                        for (a, b) in a.iter().zip(b) {
                            assert_eq!(a.body_face, b.body_face);
                            assert_eq!(a.obstacle_face, b.obstacle_face);
                            assert_eq!(a.obstacle_face_index, b.obstacle_face_index);
                            assert_eq!(
                                a.body_weights.map(f64::to_bits),
                                b.body_weights.map(f64::to_bits)
                            );
                            assert_eq!(
                                a.obstacle_weights.map(f64::to_bits),
                                b.obstacle_weights.map(f64::to_bits)
                            );
                            assert_eq!(a.normal.map(f64::to_bits), b.normal.map(f64::to_bits));
                            assert_eq!(
                                a.normal_curvature_n_m.to_bits(),
                                b.normal_curvature_n_m.to_bits()
                            );
                        }
                    }
                    (Err(a), Err(b)) => assert_eq!(a, b),
                    other => panic!("pruning changed contact admission: {other:?}"),
                }
            }
        }
    }
    #[test]
    #[ignore = "manual normal-pruning microbenchmark"]
    fn normal_pruning_microbenchmark() {
        use std::{hint::black_box, time::Instant};
        let (surface, body) = fixture(0.);
        for _ in 0..5 {
            let start = Instant::now();
            for _ in 0..500 {
                black_box(
                    surface
                        .normal_stencils_impl::<false>(&body, &[[0, 1, 2]])
                        .unwrap(),
                );
            }
            let exhaustive = start.elapsed();
            let start = Instant::now();
            for _ in 0..500 {
                black_box(surface.normal_stencils(&body, &[[0, 1, 2]]).unwrap());
            }
            eprintln!(
                "NORMAL_PRUNING_BENCH unpruned_ns={} pruned_ns={}",
                exhaustive.as_nanos(),
                start.elapsed().as_nanos()
            );
        }
    }
}

#[cfg(test)]
mod instantaneous_pose_tests {
    use super::*;
    #[test]
    #[ignore = "manual instantaneous-pose preparation benchmark"]
    fn instantaneous_pose_microbenchmark() {
        use std::{hint::black_box, time::Instant};
        let mut positions = Vec::new();
        let mut faces = Vec::new();
        for i in 0..4096 {
            let x = (i % 64) as f64 * 0.01;
            let y = (i / 64) as f64 * 0.01;
            let base = positions.len();
            positions.extend([[x, y, 0.], [x + 0.005, y, 0.], [x, y + 0.005, 0.]]);
            faces.push([base, base + 1, base + 2]);
        }
        let source = PrescribedTriangleSurface::new(positions, faces, 0.0001, 0.003, 100.).unwrap();
        let positions: Vec<_> = source
            .positions()
            .iter()
            .map(|p| [p[0], p[1], 0.001])
            .collect();
        for _ in 0..5 {
            let start = Instant::now();
            for _ in 0..25 {
                black_box(source.with_positions(positions.clone()).unwrap());
            }
            let swept = start.elapsed();
            let start = Instant::now();
            for _ in 0..25 {
                black_box(source.stage_positions::<false>(positions.clone()).unwrap());
            }
            eprintln!(
                "INSTANT_POSE_BENCH swept_ns={} instant_ns={}",
                swept.as_nanos(),
                start.elapsed().as_nanos()
            );
        }
    }
    #[test]
    fn instantaneous_samples_preserve_force_normal_and_motion_admission() {
        let source = PrescribedTriangleSurface::new(
            vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
            vec![[0, 1, 2]],
            0.0001,
            0.003,
            100.,
        )
        .unwrap();
        let body = [[0.1, 0.1, 0.001], [0.15, 0.1, 0.002], [0.1, 0.15, 0.002]];
        let faces = [[0, 1, 2]];
        for z in [-0.005, 0., 0.0002, 0.002, 0.005] {
            let positions: Vec<_> = source.positions().iter().map(|p| [p[0], p[1], z]).collect();
            let public = source.with_positions(positions.clone()).unwrap();
            let sample = source.stage_positions::<false>(positions).unwrap();
            assert!(public.swept_from.is_some());
            assert!(sample.swept_from.is_none());
            source.same_owner(&sample).unwrap();
            assert_eq!(public.positions(), sample.positions());
            assert_eq!(
                format!("{:?}", public.response(&body, &faces)),
                format!("{:?}", sample.response(&body, &faces))
            );
            assert_eq!(
                format!("{:?}", public.normal_stencils(&body, &faces)),
                format!("{:?}", sample.normal_stencils(&body, &faces))
            );
            assert_eq!(
                source.path_rejection_time(&public, &body, &body, &faces),
                source.path_rejection_time(&sample, &body, &body, &faces)
            );
        }
        let mut invalid = source.positions().to_vec();
        invalid[1] = invalid[0];
        assert_eq!(
            source.with_positions(invalid.clone()).unwrap_err(),
            source.stage_positions::<false>(invalid).unwrap_err()
        );
    }
}
