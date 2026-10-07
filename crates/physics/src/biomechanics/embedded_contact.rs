//! Contact geometry, forces and work for barycentrically embedded triangle skin.
//! Reuses the prescribed triangle law and CCD; no alternative contact law.
use super::{Body, PrescribedTriangleSurface, Vec3, dot, sub};
use crate::tissue_surface::{EmbeddedSurface, RelativeSurfaceLoads};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct RelativeSkinPose<'a> {
    pub nodes: &'a [Vec3],
    pub reference: &'a [Vec3],
    pub base: &'a [Vec3],
}
#[derive(Clone, Debug)]
pub struct EmbeddedTriangleContact {
    embedding: Arc<EmbeddedSurface>,
    rest: Vec<Vec3>,
    cells: Vec<[usize; 4]>,
    faces: Vec<[usize; 3]>,
    vertex_count: usize,
}
#[derive(Clone, Debug)]
pub struct EmbeddedContactResponse {
    pub potential_j: f64,
    pub skin_loads: RelativeSurfaceLoads,
    pub obstacle_forces_n: Vec<Vec3>,
}
#[derive(Clone, Debug)]
pub struct EmbeddedContactPathResponse {
    pub average_skin_loads: RelativeSurfaceLoads,
    pub average_obstacle_forces_n: Vec<Vec3>,
    pub midpoint_objective_j: f64,
    pub potential_change_j: f64,
    /// Signed finite-path work discrepancy; endpoint energy is an error estimator.
    pub work_defect_j: f64,
    pub quadrature_panels: usize,
}
impl EmbeddedTriangleContact {
    /// Bind authored reference-space skin, rejecting exterior vertices and bad triangles.
    /// # Errors
    /// Invalid tetrahedra, skin embedding or contact topology.
    pub fn new(
        rest: &[Vec3],
        cells: &[[usize; 4]],
        skin: &[Vec3],
        faces: Vec<[usize; 3]>,
    ) -> Result<Self, &'static str> {
        Self::new_mixed(rest, cells, skin, faces, &vec![true; skin.len()])
    }
    /// Bind a fixed mixed surface: tissue-owned vertices use the relative FEM
    /// displacement, other vertices retain their prescribed skeletal base pose.
    /// The same triangle law, CCD and force chain rule cover mixed triangles.
    /// # Errors
    /// Invalid topology/ownership or exterior tissue-owned vertices.
    pub fn new_mixed(
        rest: &[Vec3],
        cells: &[[usize; 4]],
        skin: &[Vec3],
        faces: Vec<[usize; 3]>,
        tissue_owned: &[bool],
    ) -> Result<Self, &'static str> {
        super::prescribed_surface::validate_faces(skin, &faces)?;
        Self::from_embedding(
            Arc::new(EmbeddedSurface::bind_relative(
                rest,
                cells,
                skin,
                tissue_owned,
            )?),
            faces,
        )
    }
    /// Reuse the exact immutable map used by the render skin. Authored reference
    /// geometry belongs to the embedding; no rebinding or extrapolation occurs.
    /// # Errors
    /// Invalid/degenerate triangle topology in the authored skin geometry.
    pub fn from_embedding(
        embedding: Arc<EmbeddedSurface>,
        faces: Vec<[usize; 3]>,
    ) -> Result<Self, &'static str> {
        let (rest, cells, skin) = embedding.reference_geometry();
        super::prescribed_surface::validate_faces(skin, &faces)?;
        let rest = rest.to_vec();
        let cells = cells.to_vec();
        let vertex_count = skin.len();
        Ok(Self {
            embedding,
            rest,
            cells,
            faces,
            vertex_count,
        })
    }
    /// All three pose components must keep their binding-time vertex order.
    /// # Errors
    /// Incompatible/nonfinite pose or degenerate deformed triangles.
    pub fn positions(&self, pose: RelativeSkinPose<'_>) -> Result<Vec<Vec3>, &'static str> {
        let mut skin = vec![[0.; 3]; self.vertex_count];
        self.embedding
            .deform_relative_into(pose.reference, pose.nodes, pose.base, &mut skin)?;
        super::prescribed_surface::validate_admitted_face_geometry(&skin, &self.faces)?;
        Ok(skin)
    }
    /// Contact potential and physical forces on tissue, rig and obstacle.
    /// # Errors
    /// Invalid geometry, closed barrier gap or nonfinite response.
    pub fn response(
        &self,
        pose: RelativeSkinPose<'_>,
        obstacle: &PrescribedTriangleSurface,
    ) -> Result<EmbeddedContactResponse, &'static str> {
        let skin = self.positions(pose)?;
        let response = obstacle.response_admitted_skin(&skin, &self.faces)?;
        let force: Vec<_> = response
            .body_gradient_n
            .into_iter()
            .map(|g| g.map(|x| -x))
            .collect();
        Ok(EmbeddedContactResponse {
            potential_j: response.potential_j,
            skin_loads: self.embedding.relative_loads(&force)?,
            obstacle_forces_n: response
                .obstacle_gradient_n
                .into_iter()
                .map(|g| g.map(|x| -x))
                .collect(),
        })
    }
    /// Apply W^T M W for the frozen-feature PSD normal contact metric.
    /// This is a nonlinear preconditioner, not the full contact Hessian.
    /// Prescribed reference/base/obstacle directions are held fixed at zero.
    /// # Errors
    /// Invalid pose/direction, closed gap or overflowing metric response.
    pub fn normal_metric_apply(
        &self,
        pose: RelativeSkinPose<'_>,
        obstacle: &PrescribedTriangleSurface,
        direction: &[Vec3],
    ) -> Result<Vec<Vec3>, &'static str> {
        if direction.len() != pose.nodes.len() {
            return Err("embedded metric direction size changed");
        }
        let skin = self.positions(pose)?;
        let surface_direction = self.embedding.deform_displacements(direction)?;
        let mut surface_action = vec![[0.; 3]; self.vertex_count];
        for stencil in obstacle.normal_stencils(&skin, &self.faces)? {
            let (action, _) = stencil.apply(
                stencil.body_face.map(|i| surface_direction[i]),
                [[0.; 3]; 3],
            );
            for (node, value) in stencil.body_face.into_iter().zip(action) {
                for axis in 0..3 {
                    surface_action[node][axis] += value[axis];
                }
            }
        }
        let mut result = vec![[0.; 3]; direction.len()];
        self.embedding
            .accumulate_forces_into(&surface_action, &mut result)?;
        Ok(result)
    }
    /// CCD for simultaneous linear node, reference, base and obstacle trajectories.
    /// Nonlinear skeletal motion must be resolved into appropriate linear segments.
    /// # Errors
    /// Invalid poses, topology, or changed obstacle owner/law.
    pub fn path_is_open(
        &self,
        start: RelativeSkinPose<'_>,
        end: RelativeSkinPose<'_>,
        obstacle: &PrescribedTriangleSurface,
        next: &PrescribedTriangleSurface,
    ) -> Result<bool, &'static str> {
        obstacle.path_is_open(
            next,
            &self.positions(start)?,
            &self.positions(end)?,
            &self.faces,
        )
    }
    /// Path-averaged loads with adaptive contact quadrature and a finite-work budget.
    /// Forces are integrated from the contact law, never replaced by endpoint energy.
    /// This response does not advance mechanical state or compute a full Hessian.
    /// # Errors
    /// Closed CCD path, invalid budget/geometry, or quadrature/work nonconvergence.
    pub fn path_response(
        &self,
        start: RelativeSkinPose<'_>,
        end: RelativeSkinPose<'_>,
        obstacle: &PrescribedTriangleSurface,
        next: &PrescribedTriangleSurface,
        tolerance_j: f64,
    ) -> Result<EmbeddedContactPathResponse, &'static str> {
        if !tolerance_j.is_finite() || tolerance_j <= 0. {
            return Err("invalid embedded contact work budget");
        }
        let x0 = self.positions(start)?;
        let x1 = self.positions(end)?;
        if !obstacle.path_is_open(next, &x0, &x1, &self.faces)? {
            return Err("embedded contact path crossing");
        }
        let potential_change_j = next.response(&x1, &self.faces)?.potential_j
            - obstacle
                .response_admitted_skin(&x0, &self.faces)?
                .potential_j;
        let node_delta: Vec<_> = end
            .nodes
            .iter()
            .zip(start.nodes)
            .map(|(&b, &a)| sub(b, a))
            .collect();
        let reference_delta: Vec<_> = end
            .reference
            .iter()
            .zip(start.reference)
            .map(|(&b, &a)| sub(b, a))
            .collect();
        let base_delta: Vec<_> = end
            .base
            .iter()
            .zip(start.base)
            .map(|(&b, &a)| sub(b, a))
            .collect();
        let mut knots = vec![0., 1.];
        for _ in 0..32 {
            let path = obstacle.prepare_path_partition(next, &knots)?;
            let average = path.response(&x0, &x1, &self.faces)?;
            let forces: Vec<_> = average
                .body_gradient_n
                .into_iter()
                .map(|g| g.map(|x| -x))
                .collect();
            let loads = self.embedding.relative_loads(&forces)?;
            let obstacle_forces: Vec<_> = average
                .obstacle_gradient_n
                .into_iter()
                .map(|g| g.map(|x| -x))
                .collect();
            let body_work: f64 = loads
                .nodal_forces_n()
                .iter()
                .zip(&node_delta)
                .map(|(&f, &d)| dot(f, d))
                .sum();
            let rig_work = loads.actuator_work_j(&reference_delta, &base_delta)?;
            let obstacle_work: f64 = obstacle_forces
                .iter()
                .zip(obstacle.positions().iter().zip(next.positions()))
                .map(|(&f, (&a, &b))| -dot(f, sub(b, a)))
                .sum();
            let defect = potential_change_j + body_work - rig_work - obstacle_work;
            if !defect.is_finite() {
                return Err("embedded contact work overflow");
            }
            if let Some(refined) = path.refine_by_work(&x0, &x1, &self.faces, tolerance_j)? {
                knots = refined;
                continue;
            }
            if defect.abs() > tolerance_j {
                return Err("embedded contact work budget exceeded");
            }
            return Ok(EmbeddedContactPathResponse {
                average_skin_loads: loads,
                average_obstacle_forces_n: obstacle_forces,
                midpoint_objective_j: average.midpoint_objective_j,
                potential_change_j,
                work_defect_j: defect,
                quadrature_panels: knots.len() - 1,
            });
        }
        Err("embedded contact quadrature nonconvergence")
    }
}

/// Immutable prescribed skin/obstacle geometry for ordinary mechanical stepping.
/// Updating it is a parameter change with separately booked energy, not motion integration.
#[derive(Clone, Debug)]
pub struct StationaryEmbeddedContact {
    contact: Arc<EmbeddedTriangleContact>,
    reference: Arc<[Vec3]>,
    base: Arc<[Vec3]>,
    obstacle: Arc<PrescribedTriangleSurface>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct EmbeddedSkinWork {
    pub rig_work_j: f64,
    pub obstacle_work_j: f64,
}
impl StationaryEmbeddedContact {
    pub(super) fn may_have_contact_pairs(&self) -> bool {
        self.obstacle.may_contact_faces(&self.contact.faces)
    }

    /// # Errors
    /// Invalid reference/base geometry. Contact admission uses actual body positions.
    pub fn new(
        contact: Arc<EmbeddedTriangleContact>,
        reference: Vec<Vec3>,
        base: Vec<Vec3>,
        obstacle: Arc<PrescribedTriangleSurface>,
    ) -> Result<Self, &'static str> {
        contact.positions(RelativeSkinPose {
            nodes: &contact.rest,
            reference: &reference,
            base: &base,
        })?;
        Ok(Self {
            contact,
            reference: reference.into(),
            base: base.into(),
            obstacle,
        })
    }
    /// Stage a prescribed pose retaining binding and obstacle-law identities.
    /// This does not commit mechanics or book finite-motion work.
    /// # Errors
    /// Changed obstacle owner/law or invalid reference/base geometry.
    pub fn with_pose(
        &self,
        reference: Vec<Vec3>,
        base: Vec<Vec3>,
        obstacle: Arc<PrescribedTriangleSurface>,
    ) -> Result<Self, &'static str> {
        self.obstacle.same_owner(&obstacle)?;
        Self::new(self.contact.clone(), reference, base, obstacle)
    }
    /// Evaluate a complete linear prescribed pose transition at trial body endpoints.
    /// CCD and integrated-work admission are evaluated against the actual nodes.
    /// # Errors
    /// Rebound skin owner, invalid trajectory, crossing or work-budget failure.
    pub fn path_response_to(
        &self,
        next: &Self,
        start_nodes: &[Vec3],
        end_nodes: &[Vec3],
        tolerance_j: f64,
    ) -> Result<EmbeddedContactPathResponse, &'static str> {
        if !Arc::ptr_eq(&self.contact, &next.contact) {
            return Err("embedded skin binding owner changed");
        }
        self.contact.path_response(
            self.pose(start_nodes),
            next.pose(end_nodes),
            &self.obstacle,
            &next.obstacle,
            tolerance_j,
        )
    }
    /// Sample one linear prescribed pose, keeping endpoint bits and all owners.
    /// # Errors
    /// Invalid phase, changed owner/law or degenerate intermediate geometry.
    pub fn sample_linear_pose(&self, next: &Self, phase: f64) -> Result<Self, &'static str> {
        self.same_owner(next)?;
        if !phase.is_finite() || !(0.0..=1.0).contains(&phase) {
            return Err("invalid embedded skin phase");
        }
        if phase == 0. {
            return Ok(self.clone());
        }
        if phase == 1. {
            return Ok(next.clone());
        }
        let interpolate = |a: &[Vec3], b: &[Vec3]| -> Vec<Vec3> {
            a.iter()
                .zip(b)
                .map(|(&a, &b)| super::surface_distance::trajectory_point(a, b, phase))
                .collect()
        };
        self.with_pose(
            interpolate(&self.reference, &next.reference),
            interpolate(&self.base, &next.base),
            Arc::new(self.obstacle.with_positions(interpolate(
                self.obstacle.positions(),
                next.obstacle.positions(),
            ))?),
        )
    }
    #[cfg(test)]
    pub(super) fn sampled_motion_work(
        &self,
        start: &Self,
        end: &Self,
        nodes: &[Vec3],
    ) -> Result<EmbeddedSkinWork, &'static str> {
        self.prepare_sampled_response(start, end, nodes)?
            .motion_work()
    }
    pub(super) fn prepare_sampled_response<'a>(
        &'a self,
        start: &'a Self,
        end: &'a Self,
        nodes: &'a [Vec3],
    ) -> Result<SampledEmbeddedResponse<'a>, &'static str> {
        self.same_owner(start)?;
        self.same_owner(end)?;
        Ok(SampledEmbeddedResponse {
            nodes,
            start,
            end,
            response: self.response(nodes)?,
        })
    }
    pub(super) fn same_owner(&self, next: &Self) -> Result<(), &'static str> {
        if !Arc::ptr_eq(&self.contact, &next.contact) {
            return Err("embedded skin binding owner changed");
        }
        self.obstacle.same_owner(&next.obstacle)
    }
    pub(super) fn certify_motion_to(
        &self,
        next: &Self,
        start: &[Vec3],
        end: &[Vec3],
    ) -> Result<(), &'static str> {
        self.same_owner(next)?;
        if !self.contact.path_is_open(
            self.pose(start),
            next.pose(end),
            &self.obstacle,
            &next.obstacle,
        )? {
            return Err("inertial embedded skin path crossing");
        }
        Ok(())
    }
    pub(super) fn endpoint_motion_work(
        &self,
        next: &Self,
        start: &[Vec3],
        end: &[Vec3],
    ) -> Result<EmbeddedSkinWork, &'static str> {
        self.same_owner(next)?;
        let a = self.response(start)?;
        let b = next.response(end)?;
        let reference_delta: Vec<_> = self
            .reference
            .iter()
            .zip(next.reference.iter())
            .map(|(&a, &b)| sub(b, a))
            .collect();
        let base_delta: Vec<_> = self
            .base
            .iter()
            .zip(next.base.iter())
            .map(|(&a, &b)| sub(b, a))
            .collect();
        let rig = 0.5
            * a.skin_loads
                .actuator_work_j(&reference_delta, &base_delta)?
            + 0.5
                * b.skin_loads
                    .actuator_work_j(&reference_delta, &base_delta)?;
        let obstacle: f64 = a
            .obstacle_forces_n
            .iter()
            .zip(&b.obstacle_forces_n)
            .zip(
                self.obstacle
                    .positions()
                    .iter()
                    .zip(next.obstacle.positions()),
            )
            .map(|((fa, fb), (&p, &q))| {
                -dot(
                    std::array::from_fn(|i| 0.5 * fa[i] + 0.5 * fb[i]),
                    sub(q, p),
                )
            })
            .sum();
        if !rig.is_finite() || !obstacle.is_finite() {
            return Err("embedded skin actuator work overflow");
        }
        Ok(EmbeddedSkinWork {
            rig_work_j: rig,
            obstacle_work_j: obstacle,
        })
    }
    #[must_use]
    pub fn obstacle(&self) -> &PrescribedTriangleSurface {
        &self.obstacle
    }
    fn pose<'a>(&'a self, nodes: &'a [Vec3]) -> RelativeSkinPose<'a> {
        RelativeSkinPose {
            nodes,
            reference: &self.reference,
            base: &self.base,
        }
    }
    /// Inspect the closest enabled skin/obstacle pair, including closed gaps.
    /// This read-only diagnostic uses the same embedding and pair domains as forces.
    /// # Errors
    /// Invalid body geometry, skin topology or closest-feature geometry.
    pub fn nearest_active_contact(
        &self,
        nodes: &[Vec3],
    ) -> Result<Option<super::PrescribedContactFeature>, &'static str> {
        let skin = self.contact.positions(self.pose(nodes))?;
        self.obstacle
            .nearest_active_contact(&skin, &self.contact.faces)
    }
    /// Sufficient geometric witness of a closed gap that FEM displacement cannot open.
    /// Only exact zero weights on every tissue-owned vertex qualify. The remaining
    /// barycentric point lies in the triangle for every possible nodal deformation,
    /// because its vertices and the obstacle are prescribed. No force threshold or
    /// pair exclusion is introduced. None does not certify general feasibility.
    /// # Errors
    /// Invalid body/skin geometry or closest-feature query.
    pub fn prescribed_contact_obstruction(
        &self,
        nodes: &[Vec3],
    ) -> Result<Option<super::PrescribedContactFeature>, &'static str> {
        let Some(feature) = self.nearest_active_contact(nodes)? else {
            return Ok(None);
        };
        let fixed_point =
            feature
                .body_face
                .iter()
                .zip(feature.body_weights)
                .all(|(&vertex, weight)| {
                    weight == 0. || !self.contact.embedding.tissue_owned_vertex(vertex)
                });
        Ok((feature.gap_m <= 0. && fixed_point).then_some(feature))
    }
    /// Physical tissue, rig and obstacle loads at the specified body geometry.
    /// # Errors
    /// Invalid geometry or closed contact gap.
    pub fn response(&self, nodes: &[Vec3]) -> Result<EmbeddedContactResponse, &'static str> {
        self.contact.response(self.pose(nodes), &self.obstacle)
    }
}
impl Body {
    /// Install/remove a stationary embedded skin contact. Returns its parameter-work
    /// contribution at fixed nodal positions. Failure preserves the entire body.
    /// # Errors
    /// Mismatched rest topology, closed contact or overflowing potential.
    pub fn set_stationary_embedded_contact(
        &mut self,
        contact: Option<StationaryEmbeddedContact>,
    ) -> Result<f64, &'static str> {
        if contact.as_ref().is_some_and(|c| {
            c.contact.rest != self.rest
                || c.contact.cells.iter().any(|cell| {
                    let mut key = *cell;
                    key.sort_unstable();
                    !self.elements.iter().any(|element| {
                        let mut nodes = element.nodes;
                        nodes.sort_unstable();
                        nodes == key
                    })
                })
        }) {
            return Err("embedded contact rest owner mismatch");
        }
        let before = self.evaluate(&self.positions)?.0;
        let mut candidate = self.clone();
        candidate.embedded_contact = contact;
        let work = candidate.evaluate(&candidate.positions)?.0 - before;
        if !work.is_finite() {
            return Err("embedded contact parameter work overflow");
        }
        *self = candidate;
        Ok(work)
    }
    #[must_use]
    pub fn stationary_embedded_contact(&self) -> Option<&StationaryEmbeddedContact> {
        self.embedded_contact.as_ref()
    }
    pub(super) fn embedded_contact_energy_gradient(
        contact: Option<&StationaryEmbeddedContact>,
        positions: &[Vec3],
        gradient: &mut [Vec3],
    ) -> Result<f64, &'static str> {
        let Some(contact) = contact else {
            return Ok(0.);
        };
        let response = contact.response(positions)?;
        for (g, f) in gradient
            .iter_mut()
            .zip(response.skin_loads.nodal_forces_n())
        {
            for a in 0..3 {
                g[a] -= f[a];
            }
        }
        Ok(response.potential_j)
    }
    pub(super) fn embedded_contact_path_is_open(&self, start: &[Vec3], end: &[Vec3]) -> bool {
        self.embedded_contact.as_ref().is_none_or(|c| {
            c.contact
                .path_is_open(c.pose(start), c.pose(end), &c.obstacle, &c.obstacle)
                .is_ok_and(|open| open)
        })
    }
}

/// A response bound to immutable trial coordinates and admitted motion owners.
/// No external response, coordinates or owner can be substituted after preparation.
pub(super) struct SampledEmbeddedResponse<'a> {
    nodes: &'a [Vec3],
    start: &'a StationaryEmbeddedContact,
    end: &'a StationaryEmbeddedContact,
    response: EmbeddedContactResponse,
}
impl SampledEmbeddedResponse<'_> {
    pub(super) fn positions(&self) -> &[Vec3] {
        self.nodes
    }
    pub(super) fn energy_gradient(&self, gradient: &mut [Vec3]) -> Result<f64, &'static str> {
        if gradient.len() != self.nodes.len()
            || self.response.skin_loads.nodal_forces_n().len() != gradient.len()
        {
            return Err("embedded sampled response gradient size changed");
        }
        for (g, f) in gradient
            .iter_mut()
            .zip(self.response.skin_loads.nodal_forces_n())
        {
            for axis in 0..3 {
                g[axis] -= f[axis];
            }
        }
        Ok(self.response.potential_j)
    }
    pub(super) fn motion_work(&self) -> Result<EmbeddedSkinWork, &'static str> {
        let start = self.start;
        let end = self.end;
        let response = &self.response;
        let reference_delta: Vec<_> = start
            .reference
            .iter()
            .zip(end.reference.iter())
            .map(|(&a, &b)| sub(b, a))
            .collect();
        let base_delta: Vec<_> = start
            .base
            .iter()
            .zip(end.base.iter())
            .map(|(&a, &b)| sub(b, a))
            .collect();
        let rig = response
            .skin_loads
            .actuator_work_j(&reference_delta, &base_delta)?;
        let obstacle: f64 = response
            .obstacle_forces_n
            .iter()
            .zip(
                start
                    .obstacle
                    .positions()
                    .iter()
                    .zip(end.obstacle.positions()),
            )
            .map(|(&f, (&a, &b))| -dot(f, sub(b, a)))
            .sum();
        if !obstacle.is_finite() {
            return Err("embedded obstacle sampled work overflow");
        }
        Ok(EmbeddedSkinWork {
            rig_work_j: rig,
            obstacle_work_j: obstacle,
        })
    }
}

#[cfg(test)]
mod response_profile {
    use super::*;
    #[test]
    #[ignore = "manual native embedded response timing; not a realtime engine claim"]
    fn inactive_embedded_response_profile() {
        let cells: Vec<_> = (0..8)
            .flat_map(|x| (0..8).flat_map(move |y| (0..8).map(move |z| [x, y, z])))
            .collect();
        let mesh = super::super::TetraMesh::from_lattice_cells([0.; 3], [0.01; 3], &cells).unwrap();
        let contact = EmbeddedTriangleContact::new(
            &mesh.points,
            &mesh.cells,
            &mesh.points,
            mesh.boundary.clone(),
        )
        .unwrap();
        let obstacle = PrescribedTriangleSurface::new(
            mesh.points.clone(),
            mesh.boundary.clone(),
            0.0001,
            0.003,
            100.,
        )
        .unwrap()
        .with_body_contact_domains(vec![(
            mesh.boundary.clone(),
            vec![false; mesh.boundary.len()],
        )])
        .unwrap();
        let pose = RelativeSkinPose {
            nodes: &mesh.points,
            reference: &mesh.points,
            base: &mesh.points,
        };
        let start = std::time::Instant::now();
        for _ in 0..200 {
            let response = std::hint::black_box(contact.response(pose, &obstacle).unwrap());
            assert_eq!(response.potential_j, 0.);
            assert!(
                response
                    .skin_loads
                    .nodal_forces_n()
                    .iter()
                    .flatten()
                    .all(|v| *v == 0.)
            );
            assert!(
                response
                    .obstacle_forces_n
                    .iter()
                    .flatten()
                    .all(|v| *v == 0.)
            );
        }
        eprintln!(
            "EMBEDDED_RESPONSE_PROFILE vertices={} faces={} iterations=200 elapsed_s={:.9}",
            mesh.points.len(),
            mesh.boundary.len(),
            start.elapsed().as_secs_f64()
        );
    }
}
