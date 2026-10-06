//! Midpoint kinematics and path-averaged forces with independently admitted work.
use super::super::lbfgs::secant_scale;
use super::super::lbfgs::{SecantPair, push_secant, secant_direction};
use super::{DrivenSupportStep, InertialBody, PrescribedTriangleSurface, SupportTarget, Vec3, dot};
use std::sync::Arc;
// Observe failures without replacing the original error or changing admission.
fn observe_contact_stage<T>(
    stage: &str,
    dt: f64,
    result: Result<T, &'static str>,
) -> Result<T, &'static str> {
    if let Err(error) = &result {
        if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
            eprintln!("IMPLICIT_STAGE_REJECTION stage={stage:?} dt={dt:.17e} error={error:?}");
        }
    }
    result
}
impl InertialBody {
    /// Integrate the material/internal-contact/fixed-plane force along the same
    /// frozen trajectory as external contact. The search potential has derivative
    /// sum(w * gradient), since a free midpoint displacement enters as 2*t*d.
    /// Endpoint energy remains an independent admission check, not force work.
    fn average_material_path(
        &self,
        points: &[Vec3],
        end: &[Vec3],
        nodes: &[(f64, f64)],
        initial_potential_j: f64,
    ) -> Result<super::PotentialEvaluation, &'static str> {
        self.average_material_path_with_skin(points, end, nodes, initial_potential_j, None)
    }
    // All material/contact gradients and prescribed skin work share one frozen quadrature.
    fn average_material_path_with_skin(
        &self,
        points: &[Vec3],
        end: &[Vec3],
        nodes: &[(f64, f64)],
        initial_potential_j: f64,
        skin_motion: Option<(
            &super::super::StationaryEmbeddedContact,
            &[super::super::StationaryEmbeddedContact],
        )>,
    ) -> Result<super::PotentialEvaluation, &'static str> {
        if points.len() != self.masses.len()
            || end.len() != self.masses.len()
            || nodes
                .iter()
                .any(|(t, w)| !t.is_finite() || *t <= 0. || *t > 1. || !w.is_finite() || *w <= 0.)
        {
            return Err("invalid material path quadrature");
        }
        if let Some((next, samples)) = skin_motion {
            let current = self
                .body
                .stationary_embedded_contact()
                .ok_or("skin path requires installed contact")?;
            current.same_owner(next)?;
            if samples.len() != nodes.len() {
                return Err("skin quadrature sample count changed");
            }
            for sample in samples {
                current.same_owner(sample)?;
            }
        }
        let mut average = super::PotentialEvaluation {
            potential_j: 0.,
            gradient: vec![[0.; 3]; self.masses.len()],
            contact_j: 0.,
            plane_offset_gradient: 0.,
            plane_rotation_gradient: [0.; 3],
            surface_gradient: Vec::new(),
            embedded_skin_work: super::super::EmbeddedSkinWork::default(),
        };
        for (sample_index, &(time, weight)) in nodes.iter().enumerate() {
            let world: Vec<Vec3> = self
                .body
                .positions
                .iter()
                .enumerate()
                .map(|(node, old)| {
                    if self.body.pinned[node] {
                        super::super::surface_distance::trajectory_point(*old, end[node], time)
                    } else {
                        std::array::from_fn(|axis| old[axis] + (2. * time) * points[node][axis])
                    }
                })
                .collect();
            let value = if let Some((next, samples)) = skin_motion {
                let sample = &samples[sample_index];
                let work = sample.sampled_motion_work(
                    self.body.stationary_embedded_contact().unwrap(),
                    next,
                    &world,
                )?;
                average.embedded_skin_work.rig_work_j += weight * work.rig_work_j;
                average.embedded_skin_work.obstacle_work_j += weight * work.obstacle_work_j;
                self.evaluate_at_contacts_and_skin(&world, self.plane, None, Some(sample))?
            } else {
                self.evaluate_at_contacts(&world, self.plane, None)?
            };
            average.potential_j += weight * (value.potential_j - initial_potential_j) / (2. * time);
            average.contact_j += weight * value.contact_j;
            average.plane_offset_gradient += weight * value.plane_offset_gradient;
            for axis in 0..3 {
                average.plane_rotation_gradient[axis] +=
                    weight * value.plane_rotation_gradient[axis];
            }
            for (gradient, sample) in average.gradient.iter_mut().zip(value.gradient) {
                for axis in 0..3 {
                    gradient[axis] += weight * sample[axis];
                }
            }
        }
        if !average.potential_j.is_finite()
            || !average.embedded_skin_work.rig_work_j.is_finite()
            || !average.embedded_skin_work.obstacle_work_j.is_finite()
            || average.gradient.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("averaged material evaluation overflow");
        }
        Ok(average)
    }
    /// Implicit midpoint kinematics with path-averaged material/contact forces,
    /// prescribed supports and a finite obstacle. Finite quadrature is admitted
    /// against independently evaluated endpoint energy and integrated work.
    /// Time-independent material only; Maxwell staging is not performed here.
    /// # Errors
    /// Invalid owners/targets, nonlinear nonconvergence, swept crossing or work defect.
    pub fn step_implicit_with_surface_motion(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next: Arc<PrescribedTriangleSurface>,
        dt: f64,
        tolerance_j: f64,
    ) -> Result<DrivenSupportStep, &'static str> {
        self.require_time_independent_material()?;
        self.advance_implicit_surface(targets, next, dt, tolerance_j)
    }
    /// Advance native and embedded contact together through the shared implicit solver.
    /// Embedded rig/obstacle work is included once in `surface_work_j`.
    /// Time-independent material; both contact owners must already be installed.
    /// # Errors
    /// Invalid owners, crossing, nonlinear or quadrature nonconvergence, work defect.
    pub fn step_implicit_with_surface_and_skin_motion(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next: Arc<PrescribedTriangleSurface>,
        next_skin: super::super::StationaryEmbeddedContact,
        dt: f64,
        tolerance_j: f64,
    ) -> Result<DrivenSupportStep, &'static str> {
        self.require_time_independent_material()?;
        self.advance_implicit_surface_and_skin(targets, next, Some(next_skin), dt, tolerance_j)
            .map(|(report, _)| report)
    }
    pub(super) fn advance_implicit_surface(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next: Arc<PrescribedTriangleSurface>,
        dt: f64,
        tolerance_j: f64,
    ) -> Result<DrivenSupportStep, &'static str> {
        self.advance_implicit_surface_and_skin(targets, next, None, dt, tolerance_j)
            .map(|(report, _)| report)
    }
    pub(super) fn advance_implicit_surface_and_skin(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next: Arc<PrescribedTriangleSurface>,
        next_skin: Option<super::super::StationaryEmbeddedContact>,
        dt: f64,
        tolerance_j: f64,
    ) -> Result<(DrivenSupportStep, super::super::EmbeddedSkinWork), &'static str> {
        let mut knots = vec![0., 1.];
        for attempt in 0..32 {
            match self.advance_implicit_surface_quadrature(
                targets,
                next.clone(),
                dt,
                tolerance_j,
                &mut knots,
                next_skin.as_ref(),
            ) {
                Err(
                    reason @ ("implicit contact work defect"
                    | "implicit contact path quadrature refinement"
                    | "implicit material path quadrature refinement"),
                ) => {
                    if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                        eprintln!(
                            "IMPLICIT_QUADRATURE_RETRY attempt={attempt} panels={} reason={reason:?}",
                            knots.len() - 1
                        );
                    }
                    continue;
                }
                Err(
                    reason @ ("implicit contact nonlinear nonconvergence"
                    | "implicit contact line search failed"),
                ) if next_skin.is_some() && knots.len() - 1 <= 64 => {
                    // Re-evaluate actual sampled forces; no force averaging across poses
                    // or relaxation of nonlinear/work admission. Retry remains atomic.
                    let mut refined = Vec::with_capacity(2 * knots.len() - 1);
                    for panel in knots.windows(2) {
                        refined.push(panel[0]);
                        refined.push(0.5 * (panel[0] + panel[1]));
                    }
                    refined.push(1.);
                    knots = refined;
                    if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                        eprintln!(
                            "IMPLICIT_NONLINEAR_QUADRATURE_RETRY attempt={attempt} panels={} reason={reason:?}",
                            knots.len() - 1
                        );
                    }
                }
                result => return result,
            }
        }
        Err("implicit contact quadrature nonconvergence")
    }
    fn advance_implicit_surface_quadrature(
        &mut self,
        targets: Option<&[SupportTarget]>,
        next: Arc<PrescribedTriangleSurface>,
        dt: f64,
        tolerance_j: f64,
        knots: &mut Vec<f64>,
        next_skin: Option<&super::super::StationaryEmbeddedContact>,
    ) -> Result<(DrivenSupportStep, super::super::EmbeddedSkinWork), &'static str> {
        if !dt.is_finite() || dt <= 0. || !tolerance_j.is_finite() || tolerance_j <= 0. {
            return Err("invalid implicit inertial step");
        }
        if targets.is_none() {
            self.require_stationary_supports()?;
        }
        let current = self
            .prescribed_surface
            .as_ref()
            .ok_or("surface motion requires installed contact")?;
        current.same_owner(&next)?;
        if let Some(next_skin) = next_skin {
            self.body
                .stationary_embedded_contact()
                .ok_or("skin motion requires installed contact")?
                .same_owner(next_skin)?;
        }
        let skin_path_open = |endpoint: &[Vec3]| {
            if let Some(next_skin) = next_skin {
                self.body
                    .gap_path_is_open_without_embedded(&self.body.positions, endpoint)
                    && self
                        .body
                        .stationary_embedded_contact()
                        .unwrap()
                        .certify_motion_to(next_skin, &self.body.positions, endpoint)
                        .is_ok()
            } else {
                self.body.gap_path_is_open(&self.body.positions, endpoint)
            }
        };
        let n = self.masses.len();
        let mut end = self.body.positions.clone();
        if let Some(targets) = targets {
            if targets.len() != self.body.pinned.iter().filter(|&&pin| pin).count() {
                return Err("incomplete prescribed support targets");
            }
            let mut seen = vec![false; n];
            for target in targets {
                if target.node >= n
                    || !self.body.pinned[target.node]
                    || seen[target.node]
                    || target.position_m.iter().any(|v| !v.is_finite())
                {
                    return Err("invalid prescribed support target");
                }
                seen[target.node] = true;
                end[target.node] = target.position_m;
            }
        }
        // Solve for displacement from the initial pose, so inertia does not
        // subtract nearly equal world coordinates on very small substeps.
        let mut mid: Vec<Vec3> = self
            .body
            .positions
            .iter()
            .zip(&end)
            .map(|(a, b)| std::array::from_fn(|i| 0.5 * (b[i] - a[i])))
            .collect();
        let weight: Vec<_> = self.masses.iter().map(|m| 4. * m / (dt * dt)).collect();
        if weight.iter().any(|v| !v.is_finite() || *v <= 0.) {
            return Err("implicit inertia overflow");
        }
        let faces = self.body.surface();
        let prepared_path = current.prepare_path_partition(&next, knots)?;
        // Contact is sensitive to sub-ULP world displacements near the barrier.
        // Preserve the solver displacement in a frame at the initial body pose.
        // Admission and committed energy still use the actual world endpoints.
        let contact_origin = self.body.positions[0];
        let local_start: Vec<Vec3> = self
            .body
            .positions
            .iter()
            .map(|&p| crate::biomechanics::sub(p, contact_origin))
            .collect();
        let local_current = current.with_positions(
            current
                .positions()
                .iter()
                .map(|&p| crate::biomechanics::sub(p, contact_origin))
                .collect(),
        )?;
        let local_next = next.with_positions(
            next.positions()
                .iter()
                .map(|&p| crate::biomechanics::sub(p, contact_origin))
                .collect(),
        )?;
        let local_path = local_current.prepare_path_partition(&local_next, knots)?;
        let material_nodes: Vec<_> = local_path.quadrature_nodes().collect();
        let skin_samples = next_skin
            .map(|next_skin| {
                let current_skin = self.body.stationary_embedded_contact().unwrap();
                material_nodes
                    .iter()
                    .map(|(t, _)| current_skin.sample_linear_pose(next_skin, *t))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        let skin_motion = next_skin.zip(skin_samples.as_deref());
        let initial_material_potential = self
            .evaluate_at_contacts(&self.body.positions, self.plane, None)?
            .potential_j;
        let local_endpoint = |points: &[Vec3]| -> Vec<Vec3> {
            relative_midpoint_endpoint(
                points,
                &local_start,
                &end,
                &self.body.pinned,
                contact_origin,
            )
        };
        let prepared_motion = current.prepare_motion(&next)?;
        let world_endpoint = |points: &[Vec3]| -> Vec<Vec3> {
            points
                .iter()
                .enumerate()
                .map(|(node, d)| {
                    if self.body.pinned[node] {
                        end[node]
                    } else {
                        std::array::from_fn(|axis| self.body.positions[node][axis] + 2. * d[axis])
                    }
                })
                .collect()
        };
        let feasible_guess = |points: &[Vec3]| {
            let endpoint = world_endpoint(points);
            endpoint.iter().flatten().all(|v| v.is_finite())
                && skin_path_open(&endpoint)
                && self.volume_path_is_open(&endpoint)
                && prepared_motion
                    .rejection_time(&self.body.positions, &endpoint, &faces)
                    .is_ok_and(|t| t.is_none())
        };

        let path_evaluation =
            |points: &[Vec3]| -> Result<super::PotentialEvaluation, &'static str> {
                let endpoint = local_endpoint(points);
                let path = observe_contact_stage(
                    "local contact quadrature",
                    dt,
                    local_path.response(&local_start, &endpoint, &faces),
                )?;
                let mut evaluation = observe_contact_stage(
                    "world averaged material path",
                    dt,
                    self.average_material_path_with_skin(
                        points,
                        &end,
                        &material_nodes,
                        initial_material_potential,
                        skin_motion,
                    ),
                )?;
                evaluation.potential_j += path.midpoint_objective_j;
                for (g, contact) in evaluation.gradient.iter_mut().zip(path.body_gradient_n) {
                    for axis in 0..3 {
                        g[axis] += contact[axis];
                    }
                }
                evaluation.surface_gradient = path.obstacle_gradient_n;
                Ok(evaluation)
            };
        let assemble_residual = |points: &[Vec3], gradient: &[Vec3]| {
            let mut residual = gradient.to_vec();
            let mut norm = 0.;
            for node in 0..n {
                if !self.body.pinned[node] {
                    for axis in 0..3 {
                        residual[node][axis] += weight[node]
                            * (points[node][axis] - 0.5 * dt * self.velocities[node][axis])
                            - self.masses[node] * self.acceleration[axis];
                        norm += residual[node][axis].powi(2) / weight[node];
                    }
                }
            }
            (residual, norm)
        };
        let objective_value = |points: &[Vec3], potential_j: f64| -> Result<f64, &'static str> {
            let mut value = potential_j;
            for node in 0..n {
                value -= self.masses[node]
                    * dot(
                        self.acceleration,
                        std::array::from_fn(|axis| {
                            self.body.positions[node][axis] + points[node][axis]
                                - self.body.rest[node][axis]
                        }),
                    );
                if !self.body.pinned[node] {
                    for axis in 0..3 {
                        let delta = points[node][axis] - 0.5 * dt * self.velocities[node][axis];
                        value += 0.5 * weight[node] * delta * delta;
                    }
                }
            }
            if value.is_finite() {
                Ok(value)
            } else {
                Err("implicit objective overflow")
            }
        };
        let objective = |points: &[Vec3]| -> Result<f64, &'static str> {
            let evaluation = path_evaluation(points)?;
            objective_value(points, evaluation.potential_j)
        };
        let mut converged = false;
        let mut history: Vec<SecantPair> = Vec::new();
        let mut previous: Option<(Vec<Vec3>, Vec<Vec3>)> = None;
        for iteration in 0..96 {
            let mut evaluation = path_evaluation(&mid);
            let mut initial_feasible = iteration != 0 || feasible_guess(&mid);
            if iteration == 0
                && (evaluation.is_ok() || matches!(evaluation, Err("closed surface contact gap")))
            {
                // Compare objectives only between feasible initial trajectories.
                // A stationary guess can be ill-conditioned or already crossed
                // by a moving obstacle, even when co-motion is admissible.
                let predictor: Vec<Vec3> = mid
                    .iter()
                    .enumerate()
                    .map(|(node, &d)| {
                        if self.body.pinned[node] {
                            d
                        } else {
                            self.velocities[node].map(|v| 0.5 * dt * v)
                        }
                    })
                    .collect();
                if predictor != mid {
                    let predicted_end: Vec<Vec3> = predictor
                        .iter()
                        .enumerate()
                        .map(|(node, d)| {
                            if self.body.pinned[node] {
                                end[node]
                            } else {
                                std::array::from_fn(|axis| {
                                    self.body.positions[node][axis] + 2. * d[axis]
                                })
                            }
                        })
                        .collect();
                    if predicted_end.iter().flatten().all(|v| v.is_finite())
                        && skin_path_open(&predicted_end)
                        && self.volume_path_is_open(&predicted_end)
                        && prepared_motion
                            .rejection_time(&self.body.positions, &predicted_end, &faces)
                            .is_ok_and(|time| time.is_none())
                    {
                        if let Ok(value) = path_evaluation(&predictor) {
                            let improves = objective_value(&predictor, value.potential_j)
                                .is_ok_and(|candidate| match &evaluation {
                                    Ok(old) => {
                                        !initial_feasible
                                            || objective_value(&mid, old.potential_j)
                                                .is_ok_and(|initial| candidate < initial)
                                    }
                                    Err(_) => true,
                                });
                            if improves {
                                if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                                    eprintln!("IMPLICIT_FEASIBLE_VELOCITY_PREDICTOR dt={dt:.17e}");
                                }
                                initial_feasible = true;
                                mid = predictor;
                                evaluation = Ok(value);
                            }
                        }
                    }
                    if evaluation.is_err()
                        && std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some()
                    {
                        // Read-only observations must not replace the original error.
                        let nearest = next.nearest_active_contact(&predicted_end, &faces);
                        let ccd = prepared_motion.rejection_time(
                            &self.body.positions,
                            &predicted_end,
                            &faces,
                        );
                        let gap_open = skin_path_open(&predicted_end);
                        let volume_open = self.volume_path_is_open(&predicted_end);
                        eprintln!(
                            "IMPLICIT_INFEASIBLE_PREDICTOR dt={dt:.17e} internal_gap_open={gap_open} volume_open={volume_open} ccd={ccd:?} nearest={nearest:?}"
                        );
                    }
                }
            }
            if iteration == 0
                && (!initial_feasible || matches!(evaluation, Err("closed surface contact gap")))
            {
                let predicted: Vec<Vec3> = self
                    .body
                    .positions
                    .iter()
                    .enumerate()
                    .map(|(node, p)| {
                        if self.body.pinned[node] {
                            end[node]
                        } else {
                            std::array::from_fn(|axis| {
                                self.velocities[node][axis].mul_add(dt, p[axis])
                            })
                        }
                    })
                    .collect();
                if let Ok(restored) = next.restore_separation_guess(
                    &predicted,
                    &faces,
                    &self.body.pinned,
                    &self.masses,
                ) {
                    if skin_path_open(&restored)
                        && self.volume_path_is_open(&restored)
                        && prepared_motion
                            .rejection_time(&self.body.positions, &restored, &faces)
                            .is_ok_and(|t| t.is_none())
                    {
                        let guess: Vec<Vec3> = restored
                            .iter()
                            .zip(&self.body.positions)
                            .map(|(p, old)| std::array::from_fn(|axis| 0.5 * (p[axis] - old[axis])))
                            .collect();
                        if let Ok(value) = path_evaluation(&guess) {
                            if !feasible_guess(&guess) {
                                return Err("implicit initial contact trajectory inadmissible");
                            }
                            initial_feasible = true;
                            if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                                eprintln!("IMPLICIT_RESTORED_CONTACT_GUESS dt={dt:.17e}");
                            }
                            mid = guess;
                            evaluation = Ok(value);
                        }
                    }
                }
            }
            if evaluation.is_err() && std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                if let Some((next_skin, _)) = skin_motion {
                    let endpoint = world_endpoint(&mid);
                    let current_skin = self.body.stationary_embedded_contact().unwrap();
                    eprintln!(
                        "IMPLICIT_SKIN_REJECTED_GUESS dt={dt:.17e} iteration={iteration} initial={:?} fixed_nodes_next_pose={:?} trial_endpoint={:?}",
                        current_skin.nearest_active_contact(&self.body.positions),
                        next_skin.nearest_active_contact(&self.body.positions),
                        next_skin.nearest_active_contact(&endpoint),
                    );
                }
            }
            let evaluation = evaluation?;
            if !initial_feasible {
                if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                    eprintln!("IMPLICIT_INITIAL_TRAJECTORY_REJECTION dt={dt:.17e}");
                }
                return Err("implicit initial contact trajectory inadmissible");
            }
            let (residual, norm) = assemble_residual(&mid, &evaluation.gradient);
            // The inertia-scaled force norm alone does not bound the energy
            // effect of the kinematic residual under large contact forces.
            let residual_impulse_work: f64 = (0..n)
                .filter(|&node| !self.body.pinned[node])
                .map(|node| {
                    (0..3)
                        .map(|axis| {
                            2. * residual[node][axis]
                                * (evaluation.gradient[node][axis]
                                    - self.masses[node] * self.acceleration[axis])
                                / weight[node]
                        })
                        .sum::<f64>()
                })
                .sum();
            if !norm.is_finite() {
                return Err("implicit residual overflow");
            }
            if !residual_impulse_work.is_finite() {
                return Err("implicit residual overflow");
            }
            // A diagnostic loose energy budget must never bypass equilibrium.
            if norm <= tolerance_j.min(1e-8) * 1e-4
                && residual_impulse_work.abs() <= tolerance_j.min(1e-8) * 0.125
            {
                converged = true;
                break;
            }
            let free_residual: Vec<Vec3> = residual
                .iter()
                .enumerate()
                .map(|(node, r)| if self.body.pinned[node] { [0.; 3] } else { *r })
                .collect();
            if let Some((old_mid, old_residual)) = previous.take() {
                let delta = mid
                    .iter()
                    .zip(old_mid)
                    .map(|(a, b)| std::array::from_fn(|axis| a[axis] - b[axis]))
                    .collect();
                let change = free_residual
                    .iter()
                    .zip(old_residual)
                    .map(|(a, b)| std::array::from_fn(|axis| a[axis] - b[axis]))
                    .collect();
                push_secant(&mut history, delta, change, 12);
            }
            let mut diagonal: Vec<Vec3> = weight.iter().map(|&v| [v; 3]).collect();
            let endpoint = local_endpoint(&mid);
            let blocks = observe_contact_stage(
                "local normal blocks",
                dt,
                local_path.normal_stencils(&local_start, &endpoint, &faces),
            )?;
            for block in &blocks {
                for corner in 0..3 {
                    for axis in 0..3 {
                        diagonal[block.body_face[corner]][axis] += block.normal_curvature_n_m
                            * block.body_weights[corner].powi(2)
                            * block.normal[axis].powi(2);
                    }
                }
            }
            if diagonal
                .iter()
                .flatten()
                .any(|value| !value.is_finite() || *value <= 0.)
            {
                return Err("implicit search metric overflow");
            }
            let base_direction = |r: &[Vec3]| {
                coupled_contact_direction(&weight, &diagonal, &blocks, &self.body.pinned, r)
            };
            let mut direction = secant_direction(&history, &free_residual, |q| {
                base_direction(q).iter().map(|d| d.map(|v| -v)).collect()
            });
            let mut slope: f64 = free_residual
                .iter()
                .zip(&direction)
                .map(|(r, d)| dot(*r, *d))
                .sum();
            if !slope.is_finite()
                || slope >= 0.
                || direction.iter().flatten().any(|v| !v.is_finite())
            {
                history.clear();
                direction = base_direction(&free_residual);
                slope = free_residual
                    .iter()
                    .zip(&direction)
                    .map(|(r, d)| dot(*r, *d))
                    .sum();
            }
            if iteration == 95 && std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                let maximum_residual = residual
                    .iter()
                    .enumerate()
                    .filter(|(node, _)| !self.body.pinned[*node])
                    .flat_map(|(_, r)| r)
                    .map(|v| v.abs())
                    .fold(0.0_f64, f64::max);
                let maximum_direction = direction
                    .iter()
                    .flatten()
                    .map(|v| v.abs())
                    .fold(0.0_f64, f64::max);
                let maximum_diagonal = diagonal
                    .iter()
                    .enumerate()
                    .filter(|(node, _)| !self.body.pinned[*node])
                    .flat_map(|(_, d)| d)
                    .copied()
                    .fold(0.0_f64, f64::max);
                let minimum_inertia = weight
                    .iter()
                    .enumerate()
                    .filter(|(node, _)| !self.body.pinned[*node])
                    .map(|(_, &w)| w)
                    .fold(f64::INFINITY, f64::min);
                // Compare the two independently evaluated contact frames only
                // when diagnosing exhaustion. This must never alter the solve,
                // its original error, or the publication decision.
                let physical_endpoint = world_endpoint(&mid);
                let relative_endpoint = local_endpoint(&mid);
                let endpoint_roundoff = physical_endpoint
                    .iter()
                    .zip(&relative_endpoint)
                    .flat_map(|(world, local)| {
                        (0..3).map(move |axis| {
                            (world[axis] - (local[axis] + contact_origin[axis])).abs()
                        })
                    })
                    .fold(0.0_f64, f64::max);
                match (
                    prepared_path.response(&self.body.positions, &physical_endpoint, &faces),
                    local_path.response(&local_start, &relative_endpoint, &faces),
                ) {
                    (Ok(world), Ok(local)) => {
                        let force_roundoff = world
                            .body_gradient_n
                            .iter()
                            .zip(&local.body_gradient_n)
                            .enumerate()
                            .filter(|(node, _)| !self.body.pinned[*node])
                            .flat_map(|(_, (world, local))| {
                                (0..3).map(move |axis| (world[axis] - local[axis]).abs())
                            })
                            .fold(0.0_f64, f64::max);
                        eprintln!(
                            "IMPLICIT_CONTACT_FRAME_DIFFERENCE dt={dt:.17e} maximum_endpoint_roundoff_m={endpoint_roundoff:.17e} maximum_force_difference_n={force_roundoff:.17e} objective_difference_j={:.17e}",
                            world.midpoint_objective_j - local.midpoint_objective_j
                        );
                    }
                    (world, local) => {
                        eprintln!(
                            "IMPLICIT_CONTACT_FRAME_REJECTION dt={dt:.17e} maximum_endpoint_roundoff_m={endpoint_roundoff:.17e} world_error={:?} local_error={:?}",
                            world.err(),
                            local.err()
                        );
                    }
                }
                eprintln!(
                    "IMPLICIT_NONLINEAR_LIMIT dt={dt:.17e} weighted_residual_j={norm:.17e} residual_budget_j={:.17e} residual_impulse_work_j={residual_impulse_work:.17e} impulse_work_budget_j={:.17e} maximum_residual_n={maximum_residual:.17e} maximum_direction_m={maximum_direction:.17e} maximum_diagonal_n_m={maximum_diagonal:.17e} minimum_inertia_n_m={minimum_inertia:.17e}",
                    tolerance_j.min(1e-8) * 1e-4,
                    tolerance_j.min(1e-8) * 0.125
                );
            }
            if !slope.is_finite() {
                return Err("implicit residual overflow");
            }
            let initial = objective(&mid)?;
            let noise_scale = evaluation
                .gradient
                .iter()
                .zip(&mid)
                .map(|(g, p)| {
                    (0..3)
                        .map(|axis| g[axis].abs() * (p[axis].abs() + 1.))
                        .sum::<f64>()
                })
                .sum::<f64>();
            let objective_noise = 64. * f64::EPSILON * (initial.abs() + noise_scale + 1.);
            let mut accepted = false;
            let mut safe_trials = 0;
            let mut gap_trials = 0;
            let mut volume_trials = 0;
            let mut objective_trials = 0;
            let mut smallest_objective_change = f64::INFINITY;
            let mut smallest_trial_norm = f64::INFINITY;
            let mut last_rejection_time = None;
            let mut alpha = 1.;
            for _ in 0..48 {
                let trial: Vec<Vec3> = mid
                    .iter()
                    .zip(&direction)
                    .map(|(p, d)| std::array::from_fn(|axis| p[axis] + alpha * d[axis]))
                    .collect();
                // Smaller steps cannot recover progress once every coordinate
                // rounds to its previous value. Never accept a zero iteration.
                if trial == mid {
                    break;
                }
                let endpoint = world_endpoint(&trial);
                let gap_open = skin_path_open(&endpoint);
                if gap_open {
                    gap_trials += 1;
                }
                let volume_open = gap_open && self.volume_path_is_open(&endpoint);
                if volume_open {
                    volume_trials += 1;
                }
                let rejection_time = if volume_open {
                    prepared_motion.rejection_time(&self.body.positions, &endpoint, &faces)?
                } else {
                    None
                };
                if rejection_time.is_some() {
                    last_rejection_time = rejection_time;
                }
                let safe = volume_open && rejection_time.is_none();
                if safe {
                    safe_trials += 1;
                }
                let trial_objective = if safe { objective(&trial).ok() } else { None };
                if let Some(value) = trial_objective {
                    objective_trials += 1;
                    smallest_objective_change = smallest_objective_change.min(value - initial);
                }
                let mut admissible =
                    trial_objective.is_some_and(|value| value <= initial + 1e-4 * alpha * slope);
                // When predicted decrease is below coordinate/geometry noise,
                // objective decrease can be obscured. Require force-residual decrease
                // within a gradient-scaled numerical band, never looser work.
                if !admissible
                    && -alpha * slope <= objective_noise
                    && objective_noise.is_finite()
                    && trial_objective
                        .is_some_and(|value| (value - initial).abs() <= objective_noise)
                {
                    if let Ok(trial_evaluation) = path_evaluation(&trial) {
                        let trial_norm = assemble_residual(&trial, &trial_evaluation.gradient).1;
                        smallest_trial_norm = smallest_trial_norm.min(trial_norm);
                        admissible =
                            trial_norm.is_finite() && trial_norm < norm * (1. - 1e-4 * alpha);
                    }
                }
                if admissible {
                    if iteration == 95 && std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some()
                    {
                        let actual_increment = trial
                            .iter()
                            .zip(&mid)
                            .flat_map(|(a, b)| (0..3).map(move |axis| (a[axis] - b[axis]).abs()))
                            .fold(0.0_f64, f64::max);
                        eprintln!(
                            "IMPLICIT_LAST_STEP alpha={alpha:.17e} maximum_applied_increment_m={actual_increment:.17e}"
                        );
                    }
                    previous = Some((mid.clone(), free_residual.clone()));
                    mid = trial;
                    accepted = true;
                    break;
                }
                alpha *= 0.5;
            }
            if !accepted {
                if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                    eprintln!(
                        "IMPLICIT_LINE_SEARCH_REJECTION iteration={iteration} dt={dt:.17e} weighted_residual_j={norm:.17e} residual_budget_j={:.17e} slope_j={slope:.17e} objective_noise_j={objective_noise:.17e} alpha={alpha:.17e} gap_trials={gap_trials} volume_trials={volume_trials} safe_trials={safe_trials} objective_trials={objective_trials} smallest_objective_change_j={smallest_objective_change:.17e} smallest_trial_norm_j={smallest_trial_norm:.17e}",
                        tolerance_j.min(1e-8) * 1e-4
                    );
                }
                if safe_trials == 0 && volume_trials > 0 {
                    if let Some(refined) =
                        last_rejection_time.and_then(|time| prepared_path.refine_near(time))
                    {
                        *knots = refined;
                        return Err("implicit contact path quadrature refinement");
                    }
                }
                return Err("implicit contact line search failed");
            }
        }
        if !converged {
            if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some()
                || std::env::var_os("VOXY_CONTACT_TERMINAL_TRACE").is_some()
            {
                let endpoint = world_endpoint(&mid);
                let nearest = next.nearest_active_contact(&endpoint, &faces);
                let obstacle_triangle = nearest.as_ref().ok().and_then(|feature| {
                    feature.as_ref().map(|feature| {
                        next.faces()[feature.obstacle_face_index].map(|i| next.positions()[i])
                    })
                });
                eprintln!(
                    "IMPLICIT_TERMINAL_GEOMETRY dt={dt:.17e} body_start={:?} body_end={endpoint:?} nearest_end={nearest:?} obstacle_triangle_end={obstacle_triangle:?}",
                    self.body.positions,
                );
                let local_body = local_endpoint(&mid);
                let local_obstacle = nearest.as_ref().ok().and_then(|f| {
                    f.as_ref().map(|f| {
                        local_next.faces()[f.obstacle_face_index].map(|i| local_next.positions()[i])
                    })
                });
                let local_nearest = local_next.nearest_active_contact(&local_body, &faces);
                eprintln!(
                    "IMPLICIT_FRAME_GEOMETRY dt={dt:.17e} world_body={endpoint:?} local_body={local_body:?} world_obstacle={obstacle_triangle:?} local_obstacle={local_obstacle:?} nearest_world={nearest:?} nearest_local={local_nearest:?}"
                );
            }
            return Err("implicit contact nonlinear nonconvergence");
        }
        if std::env::var_os("VOXY_CONTACT_FRAME_TRACE").is_some() {
            let endpoint = world_endpoint(&mid);
            let nearest = next.nearest_active_contact(&endpoint, &faces);
            if let Ok(Some(feature)) = &nearest {
                let obstacle_triangle =
                    Some(next.faces()[feature.obstacle_face_index].map(|i| next.positions()[i]));
                let local_body = local_endpoint(&mid);
                let local_obstacle = Some(
                    local_next.faces()[feature.obstacle_face_index]
                        .map(|i| local_next.positions()[i]),
                );
                let local_nearest = local_next.nearest_active_contact(&local_body, &faces);
                eprintln!(
                    "IMPLICIT_FRAME_GEOMETRY dt={dt:.17e} world_body={endpoint:?} local_body={local_body:?} world_obstacle={obstacle_triangle:?} local_obstacle={local_obstacle:?} nearest_world={nearest:?} nearest_local={local_nearest:?}"
                );
            }
        }
        let middle = path_evaluation(&mid)?;
        let before = observe_contact_stage("world initial diagnostics", dt, self.diagnostics())?;
        let mut velocity = self.velocities.clone();
        let mut reaction = 0.;
        let mut pin_work = 0.;
        for node in 0..n {
            for axis in 0..3 {
                if !self.body.pinned[node] {
                    // Equivalent midpoint impulse equation avoids dividing
                    // world-coordinate subtraction roundoff by a tiny dt.
                    velocity[node][axis] = self.velocities[node][axis]
                        - dt * (middle.gradient[node][axis] / self.masses[node]
                            - self.acceleration[axis]);
                    // Preserve the endpoint used by quadrature and nonlinear
                    // admission. Reconstructing it again from impulse can move
                    // a near-barrier endpoint and invalidate integrated work.
                    end[node][axis] = self.body.positions[node][axis] + 2. * mid[node][axis];
                } else if targets.is_some() {
                    velocity[node][axis] = (end[node][axis] - self.body.positions[node][axis]) / dt;
                }
                if self.body.pinned[node] {
                    reaction += (middle.gradient[node][axis]
                        - self.masses[node] * self.acceleration[axis])
                        * (end[node][axis] - self.body.positions[node][axis]);
                    pin_work += 0.5
                        * self.masses[node]
                        * (velocity[node][axis].powi(2) - self.velocities[node][axis].powi(2));
                }
            }
        }
        if !skin_path_open(&end)
            || !self.volume_path_is_open(&end)
            || prepared_motion
                .rejection_time(&self.body.positions, &end, &faces)?
                .is_some()
        {
            return Err("implicit contact path crossing");
        }
        let mut surface_work =
            middle.embedded_skin_work.rig_work_j + middle.embedded_skin_work.obstacle_work_j;
        for (node, g) in middle.surface_gradient.iter().enumerate() {
            surface_work += dot(
                *g,
                crate::biomechanics::sub(next.positions()[node], current.positions()[node]),
            );
        }
        let final_eval = observe_contact_stage(
            "world final contact",
            dt,
            self.evaluate_at_contacts_and_skin(
                &end,
                self.plane,
                Some(&next),
                next_skin.or(self.body.stationary_embedded_contact()),
            ),
        )?;
        let after = self.diagnostics_at(
            final_eval.potential_j,
            final_eval.contact_j,
            &end,
            &velocity,
        )?;
        let free_kinetic_change: f64 = (0..n)
            .filter(|&node| !self.body.pinned[node])
            .map(|node| {
                0.5 * self.masses[node]
                    * (dot(velocity[node], velocity[node])
                        - dot(self.velocities[node], self.velocities[node]))
            })
            .sum();
        let defect =
            free_kinetic_change + after.potential_j - before.potential_j - reaction - surface_work;
        if [defect, reaction, pin_work, surface_work]
            .iter()
            .any(|v| !v.is_finite())
            || defect.abs() > tolerance_j
        {
            if std::env::var_os("VOXY_CONTACT_REJECTION_TRACE").is_some() {
                // Diagnostic decomposition only; failures never replace the
                // original admission result. Potentials excluding the prescribed
                // surface still include material, internal contact and fixed plane.
                let components = (|| -> Result<[f64; 4], &'static str> {
                    let old = self.evaluate_at_contacts(&self.body.positions, self.plane, None)?;
                    let new = self.evaluate_at_contacts(&end, self.plane, None)?;
                    let material =
                        self.average_material_path(&mid, &end, &material_nodes, old.potential_j)?;
                    let potential_work: f64 = material
                        .gradient
                        .iter()
                        .enumerate()
                        .map(|(node, g)| {
                            dot(
                                *g,
                                crate::biomechanics::sub(end[node], self.body.positions[node]),
                            )
                        })
                        .sum();
                    let non_surface = new.potential_j - old.potential_j - potential_work;
                    let relative_end = relative_midpoint_endpoint(
                        &mid,
                        &local_start,
                        &end,
                        &self.body.pinned,
                        contact_origin,
                    );
                    let path = local_path.response(&local_start, &relative_end, &faces)?;
                    let body_surface_work: f64 = path
                        .body_gradient_n
                        .iter()
                        .enumerate()
                        .map(|(node, g)| {
                            dot(
                                *g,
                                crate::biomechanics::sub(end[node], self.body.positions[node]),
                            )
                        })
                        .sum();
                    let surface = next.response(&end, &faces)?.potential_j
                        - current.response(&self.body.positions, &faces)?.potential_j
                        - body_surface_work
                        - surface_work;
                    let kinematic: f64 = (0..n)
                        .filter(|&node| !self.body.pinned[node])
                        .map(|node| {
                            (0..3)
                                .map(|axis| {
                                    let old_v = self.velocities[node][axis];
                                    let new_v = velocity[node][axis];
                                    0.5 * self.masses[node] * (new_v - old_v) * (new_v + old_v)
                                        + (middle.gradient[node][axis]
                                            - self.masses[node] * self.acceleration[axis])
                                            * (end[node][axis] - self.body.positions[node][axis])
                                })
                                .sum::<f64>()
                        })
                        .sum();
                    Ok([
                        non_surface,
                        surface,
                        kinematic,
                        defect - non_surface - surface - kinematic,
                    ])
                })();
                match components {
                    Ok([non_surface, surface, kinematic, remainder]) => eprintln!(
                        "IMPLICIT_WORK_COMPONENTS origin={contact_origin:?} dt={dt:.17e} non_surface_potential_defect_j={non_surface:.17e} prescribed_surface_defect_j={surface:.17e} kinematic_defect_j={kinematic:.17e} arithmetic_remainder_j={remainder:.17e}"
                    ),
                    Err(error) => eprintln!(
                        "IMPLICIT_WORK_COMPONENT_REJECTION origin={contact_origin:?} dt={dt:.17e} error={error:?}"
                    ),
                }
                if dt <= 1e-6 {
                    let impulse_end: Vec<Vec3> = self
                        .body
                        .positions
                        .iter()
                        .enumerate()
                        .map(|(node, old)| {
                            if self.body.pinned[node] {
                                end[node]
                            } else {
                                std::array::from_fn(|axis| {
                                    old[axis]
                                        + 0.5
                                            * dt
                                            * (self.velocities[node][axis] + velocity[node][axis])
                                })
                            }
                        })
                        .collect();
                    let maximum_endpoint_change = impulse_end
                        .iter()
                        .zip(&end)
                        .flat_map(|(a, b)| (0..3).map(move |axis| (a[axis] - b[axis]).abs()))
                        .fold(0.0_f64, f64::max);
                    let energy_change = self
                        .evaluate_at_contacts(&impulse_end, self.plane, Some(&next))
                        .and_then(|evaluation| {
                            self.diagnostics_at(
                                evaluation.potential_j,
                                evaluation.contact_j,
                                &impulse_end,
                                &velocity,
                            )
                        })
                        .map(|diagnostics| diagnostics.potential_j - after.potential_j);
                    eprintln!(
                        "IMPLICIT_ENDPOINT_COMPARISON dt={dt:.17e} maximum_endpoint_change_m={maximum_endpoint_change:.17e} discarded_impulse_endpoint_energy_change_j={energy_change:?}"
                    );
                }
                let mut maximum_kinematic_residual = 0.0_f64;
                for node in 0..n {
                    if !self.body.pinned[node] {
                        for axis in 0..3 {
                            maximum_kinematic_residual = maximum_kinematic_residual.max(
                                (end[node][axis]
                                    - self.body.positions[node][axis]
                                    - 0.5
                                        * dt
                                        * (self.velocities[node][axis] + velocity[node][axis]))
                                    .abs(),
                            );
                        }
                    }
                }
                eprintln!(
                    "IMPLICIT_WORK_REJECTION dt={dt:.17e} defect_j={defect:.17e} budget_j={tolerance_j:.17e} maximum_kinematic_residual_m={maximum_kinematic_residual:.17e}"
                );
            }
            if let Some(refined) = observe_contact_stage(
                "world quadrature error estimator",
                dt,
                prepared_path.refine_by_work(
                    &self.body.positions,
                    &end,
                    &faces,
                    tolerance_j * 0.25,
                ),
            )? {
                *knots = refined;
                return Err("implicit contact work defect");
            }
            // Moving prescribed skin requires temporal refinement of its own samples.
            // Endpoint energy remains the admission oracle; never correct forces from it.
            if next_skin.is_some() {
                if knots.len() - 1 <= 64 {
                    let mut refined = Vec::with_capacity(2 * knots.len() - 1);
                    for panel in knots.windows(2) {
                        refined.push(panel[0]);
                        refined.push(0.5 * (panel[0] + panel[1]));
                    }
                    refined.push(1.);
                    *knots = refined;
                    return Err("implicit material path quadrature refinement");
                }
                return Err("implicit midpoint work defect");
            }
            if let Some(refined) = observe_contact_stage(
                "material quadrature error estimator",
                dt,
                prepared_path.refine_potential_by_work(
                    &self.body.positions,
                    &end,
                    |positions| {
                        let value = self.evaluate_at_contacts(positions, self.plane, None)?;
                        Ok((value.potential_j, value.gradient))
                    },
                    tolerance_j * 0.25,
                ),
            )? {
                *knots = refined;
                return Err("implicit material path quadrature refinement");
            }
            return Err("implicit midpoint work defect");
        }
        let support_work = reaction + pin_work;
        let lost_work = if reaction != 0. && support_work == pin_work {
            reaction.abs()
        } else if pin_work != 0. && support_work == reaction {
            pin_work.abs()
        } else {
            0.
        };
        if !support_work.is_finite() || lost_work > tolerance_j {
            return Err("unrepresentable implicit support work");
        }
        let committed_skin = next_skin.cloned();
        self.body.positions = end;
        if let Some(skin) = committed_skin {
            self.body.embedded_contact = Some(skin);
        }
        self.velocities = velocity;
        self.prescribed_surface = Some(next);
        Ok((
            DrivenSupportStep {
                support_work_j: support_work,
                reaction_work_j: reaction,
                pin_kinetic_work_j: pin_work,
                plane_work_j: 0.,
                plane_translation_work_j: 0.,
                plane_rotation_work_j: 0.,
                surface_work_j: surface_work,
                energy_defect_j: defect,
            },
            middle.embedded_skin_work,
        ))
    }
    /// Path-averaged implicit midpoint mechanics with prescribed supports.
    /// Uses the same material quadrature and secant algebra as contact motion.
    /// # Errors
    /// Rejects external surface ownership, invalid targets, inadmissible paths,
    /// nonlinear nonconvergence and independent endpoint work imbalance.
    pub fn step_implicit_with_supports(
        &mut self,
        targets: Option<&[SupportTarget]>,
        dt: f64,
        tolerance_j: f64,
    ) -> Result<DrivenSupportStep, &'static str> {
        self.step_implicit_supports_controlled(targets, dt, tolerance_j, None)
    }
    pub(super) fn step_implicit_supports_controlled(
        &mut self,
        targets: Option<&[SupportTarget]>,
        dt: f64,
        tolerance_j: f64,
        motion_tolerance: Option<(f64, f64)>,
    ) -> Result<DrivenSupportStep, &'static str> {
        if motion_tolerance
            .is_some_and(|(p, v)| !p.is_finite() || p <= 0. || !v.is_finite() || v <= 0.)
        {
            return Err("invalid implicit nonlinear motion controls");
        }
        if !dt.is_finite()
            || dt <= 0.
            || !tolerance_j.is_finite()
            || tolerance_j <= 0.
            || self.prescribed_surface.is_some()
        {
            return Err("invalid implicit support step");
        }
        if targets.is_none() {
            self.require_stationary_supports()?;
        }
        let n = self.masses.len();
        let mut end = self.body.positions.clone();
        if let Some(targets) = targets {
            if targets.len() != self.body.pinned.iter().filter(|&&p| p).count() {
                return Err("incomplete prescribed support targets");
            }
            let mut seen = vec![false; n];
            for target in targets {
                if target.node >= n
                    || !self.body.pinned[target.node]
                    || seen[target.node]
                    || target.position_m.iter().any(|v| !v.is_finite())
                {
                    return Err("invalid prescribed support target");
                }
                seen[target.node] = true;
                end[target.node] = target.position_m;
            }
        }
        let weight: Vec<_> = self.masses.iter().map(|m| 4. * m / dt / dt).collect();
        if weight.iter().any(|w| !w.is_finite() || *w <= 0.) {
            return Err("implicit inertia overflow");
        }
        let initial = self.evaluate_at_contacts(&self.body.positions, self.plane, None)?;
        let before = self.diagnostics()?;
        let mut mid: Vec<Vec3> = (0..n)
            .map(|i| {
                std::array::from_fn(|a| {
                    if self.body.pinned[i] {
                        0.5 * (end[i][a] - self.body.positions[i][a])
                    } else {
                        0.5 * dt * self.velocities[i][a] + 0.25 * dt * dt * self.acceleration[a]
                    }
                })
            })
            .collect();
        // Four-point Gauss rule; final energy/work is admitted independently.
        let nodes = [
            (0.06943184420297371, 0.17392742256872693),
            (0.33000947820757187, 0.32607257743127305),
            (0.6699905217924281, 0.32607257743127305),
            (0.9305681557970262, 0.17392742256872693),
        ];
        // Search potential subtracts the initial energy at every quadrature
        // sample. Its cancellation scale survives that subtraction, even when
        // the resulting objective is tiny. Endpoint energy admission is separate.
        let baseline_roundoff_j = (128. * f64::EPSILON * initial.potential_j.abs())
            * nodes
                .iter()
                .map(|(time, weight)| weight / (2. * time))
                .sum::<f64>();
        let evaluate = |x: &[Vec3]| -> Result<_, &'static str> {
            let endpoint: Vec<Vec3> = (0..n)
                .map(|i| {
                    std::array::from_fn(|a| {
                        if self.body.pinned[i] {
                            end[i][a]
                        } else {
                            self.body.positions[i][a] + 2. * x[i][a]
                        }
                    })
                })
                .collect();
            if !self.volume_path_is_open(&endpoint)
                || !self.body.gap_path_is_open(&self.body.positions, &endpoint)
            {
                return Err("implicit support path crossing");
            }
            let value = self.average_material_path(x, &end, &nodes, initial.potential_j)?;
            let mut residual = value.gradient.clone();
            let mut objective = value.potential_j;
            let mut norm = 0.;
            let mut impulse_work = 0.;
            for i in 0..n {
                objective -= self.masses[i] * dot(self.acceleration, x[i]);
                if self.body.pinned[i] {
                    residual[i] = [0.; 3];
                    continue;
                }
                for a in 0..3 {
                    let delta = x[i][a] - 0.5 * dt * self.velocities[i][a];
                    objective += 0.5 * weight[i] * delta * delta;
                    residual[i][a] += weight[i] * delta - self.masses[i] * self.acceleration[a];
                    norm += residual[i][a].powi(2) / weight[i];
                    impulse_work += 2.
                        * residual[i][a]
                        * (value.gradient[i][a] - self.masses[i] * self.acceleration[a])
                        / weight[i];
                }
            }
            if !objective.is_finite() || !norm.is_finite() || !impulse_work.is_finite() {
                return Err("implicit residual overflow");
            }
            Ok((value, residual, objective, norm, impulse_work))
        };
        // Rest-configuration material stiffness is a search metric only.
        // Residual, impulse work and endpoint energy admission still use inertia.
        let inverse_diagonal: Vec<_> = weight
            .iter()
            .zip(&self.body.diagonal)
            .map(|(&inertia, &stiffness)| {
                let scale = inertia.max(stiffness);
                (1. / scale) / (inertia / scale + stiffness / scale)
            })
            .collect();
        if inverse_diagonal.len() != n
            || inverse_diagonal.iter().any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("implicit support preconditioner overflow");
        }
        let inverse_material = |g: &[Vec3]| {
            preconditioned_direction(
                &self.body.pinned,
                g,
                |v| rest_material_action(&self.body, &weight, v),
                |r| {
                    r.iter()
                        .enumerate()
                        .map(|(i, r)| r.map(|x| x * inverse_diagonal[i]))
                        .collect()
                },
            )
            .into_iter()
            .map(|v| v.map(|x| -x))
            .collect::<Vec<_>>()
        };
        let mut history = Vec::new();
        let mut state = evaluate(&mid)?;
        let mut converged = false;
        for _ in 0..256 {
            let motion_resolved = motion_tolerance.is_none_or(|(position, velocity)| {
                (0..n).filter(|&i| !self.body.pinned[i]).all(|i| {
                    (0..3).all(|a| {
                        let correction = state.1[i][a].abs() / weight[i];
                        2. * correction <= position && 4. * correction / dt <= velocity
                    })
                })
            });
            if motion_resolved
                && state.3 <= tolerance_j.min(1e-8) * 1e-4
                && state.4.abs() <= tolerance_j.min(1e-8) * 0.125
            {
                converged = true;
                break;
            }
            let scale = secant_scale(&history, |g| {
                g.iter()
                    .zip(inverse_material(g))
                    .map(|(g, v)| dot(*g, v))
                    .sum()
            });
            let direction = secant_direction(&history, &state.1, |g| {
                inverse_material(g)
                    .into_iter()
                    .map(|v| v.map(|x| scale * x))
                    .collect()
            });
            let slope: f64 = direction
                .iter()
                .zip(&state.1)
                .map(|(d, g)| dot(*d, *g))
                .sum();
            if !slope.is_finite() || slope >= 0. {
                return Err("implicit support search direction failed");
            }
            let mut accepted = None;
            let mut alpha = 1.;
            for _ in 0..40 {
                let trial: Vec<Vec3> = mid
                    .iter()
                    .zip(&direction)
                    .map(|(x, d)| std::array::from_fn(|a| x[a] + alpha * d[a]))
                    .collect();
                if let Ok(value) = evaluate(&trial) {
                    let roundoff = 64. * f64::EPSILON * state.2.abs()
                        + 64. * f64::EPSILON * value.2.abs()
                        + baseline_roundoff_j;
                    let resolved_descent = motion_tolerance.is_some()
                        && value.2 <= state.2 + roundoff
                        && value.3 < state.3;
                    if value.2 <= state.2 + 1e-4 * alpha * slope || resolved_descent {
                        accepted = Some((trial, value));
                        break;
                    }
                }
                alpha *= 0.5;
            }
            let (trial, value) = accepted.ok_or("implicit support line search failed")?;
            let displacement = trial
                .iter()
                .zip(&mid)
                .map(|(a, b)| super::super::sub(*a, *b))
                .collect();
            let gradient_change = value
                .1
                .iter()
                .zip(&state.1)
                .map(|(a, b)| super::super::sub(*a, *b))
                .collect();
            push_secant(&mut history, displacement, gradient_change, 8);
            mid = trial;
            state = value;
        }
        if !converged {
            return Err("implicit support nonlinear nonconvergence");
        }
        let middle = state.0;
        let mut velocity = self.velocities.clone();
        let (mut reaction, mut pin_work) = (0., 0.);
        for i in 0..n {
            for a in 0..3 {
                if self.body.pinned[i] {
                    if targets.is_some() {
                        velocity[i][a] = (end[i][a] - self.body.positions[i][a]) / dt;
                    }
                    reaction += (middle.gradient[i][a] - self.masses[i] * self.acceleration[a])
                        * (end[i][a] - self.body.positions[i][a]);
                    pin_work += 0.5
                        * self.masses[i]
                        * (velocity[i][a].powi(2) - self.velocities[i][a].powi(2));
                } else {
                    end[i][a] = self.body.positions[i][a] + 2. * mid[i][a];
                    velocity[i][a] -=
                        dt * (middle.gradient[i][a] / self.masses[i] - self.acceleration[a]);
                }
            }
        }
        let final_eval = self.evaluate_at_contacts(&end, self.plane, None)?;
        let after = self.diagnostics_at(
            final_eval.potential_j,
            final_eval.contact_j,
            &end,
            &velocity,
        )?;
        let kinetic_change: f64 = (0..n)
            .filter(|&i| !self.body.pinned[i])
            .map(|i| {
                0.5 * self.masses[i]
                    * (dot(velocity[i], velocity[i]) - dot(self.velocities[i], self.velocities[i]))
            })
            .sum();
        let defect = kinetic_change + after.potential_j - before.potential_j - reaction;
        let support_work = reaction + pin_work;
        let lost_work = if reaction != 0. && support_work == pin_work {
            reaction.abs()
        } else if pin_work != 0. && support_work == reaction {
            pin_work.abs()
        } else {
            0.
        };
        if lost_work > tolerance_j {
            return Err("unrepresentable implicit support work");
        }
        if [defect, reaction, pin_work, support_work]
            .iter()
            .any(|v| !v.is_finite())
            || defect.abs() > tolerance_j
        {
            return Err("implicit support work defect");
        }
        self.body.positions = end;
        self.velocities = velocity;
        Ok(DrivenSupportStep {
            support_work_j: support_work,
            reaction_work_j: reaction,
            pin_kinetic_work_j: pin_work,
            plane_work_j: 0.,
            plane_translation_work_j: 0.,
            plane_rotation_work_j: 0.,
            surface_work_j: 0.,
            energy_defect_j: defect,
        })
    }
}

fn relative_midpoint_endpoint(
    points: &[Vec3],
    local_start: &[Vec3],
    world_end: &[Vec3],
    pinned: &[bool],
    origin: Vec3,
) -> Vec<Vec3> {
    points
        .iter()
        .zip(local_start)
        .enumerate()
        .map(|(node, (d, old))| {
            if pinned[node] {
                crate::biomechanics::sub(world_end[node], origin)
            } else {
                std::array::from_fn(|axis| 2_f64.mul_add(d[axis], old[axis]))
            }
        })
        .collect()
}

// Solve the positive inertia + frozen-feature normal operator, retaining
// coupling between vertices and axes. This is a search metric, not the full
// material/geometric Hessian; nonlinear admission still checks the real model.
fn coupled_contact_direction(
    weight: &[f64],
    diagonal: &[Vec3],
    blocks: &[super::super::PrescribedContactStencil],
    pinned: &[bool],
    residual: &[Vec3],
) -> Vec<Vec3> {
    let apply = |v: &[Vec3]| {
        let mut result: Vec<Vec3> = v
            .iter()
            .zip(weight)
            .enumerate()
            .map(|(node, (v, w))| {
                if pinned[node] {
                    [0.; 3]
                } else {
                    std::array::from_fn(|axis| w * v[axis])
                }
            })
            .collect();
        for block in blocks {
            let body = block
                .body_face
                .map(|node| if pinned[node] { [0.; 3] } else { v[node] });
            let action = block.apply(body, [[0.; 3]; 3]).0;
            for corner in 0..3 {
                let node = block.body_face[corner];
                if !pinned[node] {
                    for axis in 0..3 {
                        result[node][axis] += action[corner][axis];
                    }
                }
            }
        }
        result
    };
    preconditioned_direction(pinned, residual, apply, |r| {
        r.iter()
            .zip(diagonal)
            .map(|(r, d)| std::array::from_fn(|axis| r[axis] / d[axis]))
            .collect()
    })
}

// Rest isotropic elasticity plus inertia is only a search metric. Nonlinear
// forces, anisotropy, memory and endpoint work remain in the real evaluator.
fn rest_material_action(body: &super::super::Body, weight: &[f64], v: &[Vec3]) -> Vec<Vec3> {
    let mut result: Vec<_> = v
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if body.pinned[i] {
                [0.; 3]
            } else {
                v.map(|x| weight[i] * x)
            }
        })
        .collect();
    for e in &body.elements {
        let (mu, bulk) = e
            .viscoelastic
            .as_ref()
            .map_or((e.material.shear_pa, e.material.bulk_pa), |m| {
                m.rest_moduli()
            });
        let mut h = [[0.; 3]; 3];
        for corner in 0..4 {
            let node = e.nodes[corner];
            if !body.pinned[node] {
                for a in 0..3 {
                    for b in 0..3 {
                        h[a][b] += v[node][a] * e.gradients[corner][b];
                    }
                }
            }
        }
        let trace = h[0][0] + h[1][1] + h[2][2];
        let stress: [[f64; 3]; 3] = std::array::from_fn(|a| {
            std::array::from_fn(|b| {
                let sym = 0.5 * (h[a][b] + h[b][a]);
                let dev = if a == b { sym - trace / 3. } else { sym };
                2. * mu * dev + if a == b { bulk * trace } else { 0. }
            })
        });
        for corner in 0..4 {
            let node = e.nodes[corner];
            if !body.pinned[node] {
                for a in 0..3 {
                    result[node][a] += e.volume * dot(stress[a], e.gradients[corner]);
                }
            }
        }
    }
    result
}

// Shared SPD search solve; finite descent admission retains the diagonal fallback.
fn preconditioned_direction(
    pinned: &[bool],
    residual: &[Vec3],
    apply: impl Fn(&[Vec3]) -> Vec<Vec3>,
    precondition: impl Fn(&[Vec3]) -> Vec<Vec3>,
) -> Vec<Vec3> {
    let n = residual.len();
    let inner = |a: &[Vec3], b: &[Vec3]| -> f64 { a.iter().zip(b).map(|(a, b)| dot(*a, *b)).sum() };
    let mut r: Vec<Vec3> = residual
        .iter()
        .enumerate()
        .map(|(node, r)| if pinned[node] { [0.; 3] } else { r.map(|v| -v) })
        .collect();
    let mut z = precondition(&r);
    let fallback = z.clone();
    let mut search = z.clone();
    let mut direction = vec![[0.; 3]; n];
    let initial = inner(&r, &z);
    let mut rz = initial;
    for _ in 0..64 {
        if rz <= initial * 1e-12 || rz <= 0. {
            break;
        }
        let action = apply(&search);
        let curvature = inner(&search, &action);
        if !curvature.is_finite() || curvature <= 0. {
            return fallback;
        }
        let alpha = rz / curvature;
        for node in 0..n {
            for axis in 0..3 {
                direction[node][axis] += alpha * search[node][axis];
                r[node][axis] -= alpha * action[node][axis];
            }
        }
        z = precondition(&r);
        let next = inner(&r, &z);
        let beta = next / rz;
        for node in 0..n {
            for axis in 0..3 {
                search[node][axis] = z[node][axis] + beta * search[node][axis];
            }
        }
        rz = next;
    }
    if direction.iter().flatten().all(|v| v.is_finite()) && inner(residual, &direction) < 0. {
        direction
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Independent two-branch generalized-stationarity oracle. Kept test-only
    // until exact coincident contact branches are available to the production solver.
    fn project_branch_segment(
        first: &[Vec3],
        second: &[Vec3],
        inertia: &[f64],
        pinned: &[bool],
    ) -> Result<(f64, f64), &'static str> {
        if first.len() != second.len()
            || first.len() != inertia.len()
            || first.len() != pinned.len()
            || inertia.iter().any(|w| !w.is_finite() || *w <= 0.)
            || first.iter().chain(second).flatten().any(|v| !v.is_finite())
        {
            return Err("invalid branch projection");
        }
        let mut linear = 0.;
        let mut quadratic = 0.;
        for node in 0..first.len() {
            if pinned[node] {
                continue;
            }
            for axis in 0..3 {
                let a = first[node][axis] / inertia[node].sqrt();
                let delta = (second[node][axis] - first[node][axis]) / inertia[node].sqrt();
                linear += a * delta;
                quadratic += delta * delta;
            }
        }
        if !linear.is_finite() || !quadratic.is_finite() {
            return Err("branch projection overflow");
        }
        let alpha = if quadratic > 0. {
            (-linear / quadratic).clamp(0., 1.)
        } else {
            0.
        };
        let mut norm = 0.;
        for node in 0..first.len() {
            if pinned[node] {
                continue;
            }
            for axis in 0..3 {
                let residual = (1. - alpha) * first[node][axis] + alpha * second[node][axis];
                norm += residual * residual / inertia[node];
            }
        }
        if !norm.is_finite() {
            return Err("branch projection overflow");
        }
        Ok((alpha, norm))
    }
    #[test]
    fn generalized_branch_projection_preserves_convexity_and_inertia_metric() {
        let first = [[2., 1., 0.], [1e20; 3]];
        let second = [[-1., 1., 0.], [-1e20; 3]];
        let (alpha, norm) =
            project_branch_segment(&first, &second, &[4., 1.], &[false, true]).unwrap();
        assert!((alpha - 2. / 3.).abs() < 1e-15);
        assert!((norm - 0.25).abs() < 1e-15);
        // Independent dense search verifies the weighted minimum and ignores pin reaction.
        for i in 0..=1000 {
            let a = i as f64 / 1000.;
            let reference = ((1. - a) * 2. - a).powi(2) / 4. + 0.25;
            assert!(norm <= reference + 1e-15);
        }
        let (alpha, norm) =
            project_branch_segment(&[[2., 0., 0.]], &[[1., 0., 0.]], &[1.], &[false]).unwrap();
        assert_eq!((alpha, norm), (1., 1.)); // Same-sign branches cannot fabricate equilibrium.
        let (alpha, norm) =
            project_branch_segment(&[[2., 0., 0.]], &[[2., 0., 0.]], &[1.], &[false]).unwrap();
        assert_eq!((alpha, norm), (0., 4.));
        assert!(project_branch_segment(&first, &second, &[0., 1.], &[false, true]).is_err());
        assert!(project_branch_segment(&first, &second, &[4., 1.], &[false]).is_err());
    }
    #[test]
    fn rest_material_metric_matches_affine_energy_and_solves_coupled_system() {
        use super::super::super::{Body, Material};
        let points = vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let mut body = Body::new(
            points.clone(),
            vec![false; 4],
            vec![(
                [0, 1, 2, 3],
                Material {
                    shear_pa: 3.,
                    bulk_pa: 11.,
                    fibers: vec![],
                },
            )],
        )
        .unwrap();
        let weight = vec![2.; 4];
        let affine: Vec<Vec3> = points
            .iter()
            .map(|p| [0.2 * p[0] + 0.4 * p[1], -0.1 * p[1], 0.3 * p[2]])
            .collect();
        let action = rest_material_action(&body, &weight, &affine);
        let quadratic: f64 = affine.iter().zip(&action).map(|(v, a)| dot(*v, *a)).sum();
        // Independent affine strain: trace=.4, sym off-diagonal=.2.
        let dev_squared = (0.2_f64 - 0.4 / 3.).powi(2)
            + (-0.1_f64 - 0.4 / 3.).powi(2)
            + (0.3_f64 - 0.4 / 3.).powi(2)
            + 2. * 0.2_f64.powi(2);
        let inertia: f64 = affine.iter().map(|v| 2. * dot(*v, *v)).sum();
        assert!(
            (quadratic - inertia - (6. * dev_squared + 11. * 0.4_f64.powi(2)) / 6.).abs() < 1e-14
        );
        let translation = vec![[0.7, -0.2, 0.1]; 4];
        let translated = rest_material_action(&body, &weight, &translation);
        for a in translated {
            assert_eq!(a, [1.4, -0.4, 0.2]);
        }
        body.pinned[0] = true;
        let residual = vec![
            [1., 2., -1.],
            [-0.2, 0.3, 0.8],
            [0.4, -0.7, 0.1],
            [0.8, -0.2, 0.6],
        ];
        let solve = preconditioned_direction(
            &body.pinned,
            &residual,
            |v| rest_material_action(&body, &weight, v),
            |r| r.iter().map(|r| r.map(|x| x / 2.)).collect(),
        );
        assert_eq!(solve[0], [0.; 3]);
        let checked = rest_material_action(&body, &weight, &solve);
        for i in 1..4 {
            for a in 0..3 {
                assert!((checked[i][a] + residual[i][a]).abs() < 1e-9);
            }
        }
    }
    #[test]
    fn averaged_material_search_gradient_matches_energy_differences() {
        use super::super::super::{Body, Material};
        let reference = vec![[0., 0., 0.1], [0.1, 0., 0.1], [0., 0.1, 0.1], [0., 0., 0.2]];
        let mut body = Body::new(
            reference.clone(),
            vec![true, false, false, false],
            vec![(
                [0, 1, 2, 3],
                Material::from_young_poisson(1e6, 0.45).unwrap(),
            )],
        )
        .unwrap();
        // Begin with nonzero strain energy, so the constant subtraction in the
        // search objective is covered as well as prescribed support motion.
        body.positions[1][0] += 0.002;
        let dynamics =
            InertialBody::new_with_fixed_supports(body, &[1000.], vec![[0.; 3]; 4]).unwrap();
        let surface = PrescribedTriangleSurface::new(
            vec![[-1., -1., -1.], [1., -1., -1.], [0., 1., -1.]],
            vec![[0, 1, 2]],
            0.001,
            0.03,
            100.,
        )
        .unwrap();
        let path = surface
            .prepare_path_partition(&surface, &[0., 0.2, 0.7, 1.])
            .unwrap();
        let nodes: Vec<_> = path.quadrature_nodes().collect();
        let initial = dynamics
            .evaluate_at_contacts(&dynamics.body.positions, None, None)
            .unwrap()
            .potential_j;
        let mut end = dynamics.body.positions.clone();
        end[0][1] += 0.001;
        let points = [
            [0., 0.0005, 0.],
            [0.001, -0.0002, 0.0003],
            [-0.0004, 0.0008, -0.0001],
            [0.0002, -0.0003, 0.0007],
        ];
        let average = dynamics
            .average_material_path(&points, &end, &nodes, initial)
            .unwrap();
        for node in 1..4 {
            for axis in 0..3 {
                let h = 1e-7;
                let mut plus = points;
                let mut minus = points;
                plus[node][axis] += h;
                minus[node][axis] -= h;
                let fd = (dynamics
                    .average_material_path(&plus, &end, &nodes, initial)
                    .unwrap()
                    .potential_j
                    - dynamics
                        .average_material_path(&minus, &end, &nodes, initial)
                        .unwrap()
                        .potential_j)
                    / (2. * h);
                assert!(
                    (fd - average.gradient[node][axis]).abs()
                        < 1e-5 * average.gradient[node][axis].abs().max(1.),
                    "node={node} axis={axis} fd={fd} gradient={}",
                    average.gradient[node][axis]
                );
            }
        }
    }
    #[test]
    fn moving_skin_shared_quadrature_matches_derivative_and_independent_work() {
        use super::super::super::{
            Body, EmbeddedTriangleContact, Material, StationaryEmbeddedContact,
        };
        use std::sync::Arc;
        let rest = vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let base = vec![[0.1, 0.1, 0.015], [0.15, 0.1, 0.04], [0.1, 0.15, 0.04]];
        let obstacle = Arc::new(
            PrescribedTriangleSurface::new(
                vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
                vec![[0, 1, 2]],
                0.001,
                0.03,
                100.,
            )
            .unwrap(),
        );
        let current = StationaryEmbeddedContact::new(
            Arc::new(
                EmbeddedTriangleContact::new(&rest, &[[0, 1, 2, 3]], &base, vec![[0, 1, 2]])
                    .unwrap(),
            ),
            rest.clone(),
            base.clone(),
            obstacle.clone(),
        )
        .unwrap();
        let shift = |points: &[Vec3], z: f64| -> Vec<Vec3> {
            points.iter().map(|p| [p[0], p[1], p[2] + z]).collect()
        };
        let next = current
            .with_pose(
                shift(&rest, 4e-6),
                shift(&base, 1e-5),
                Arc::new(
                    obstacle
                        .with_positions(shift(obstacle.positions(), 2e-6))
                        .unwrap(),
                ),
            )
            .unwrap();
        assert_eq!(
            format!("{:?}", current.sample_linear_pose(&next, 0.).unwrap()),
            format!("{current:?}")
        );
        assert_eq!(
            format!("{:?}", current.sample_linear_pose(&next, 1.).unwrap()),
            format!("{next:?}")
        );
        for phase in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
            assert!(current.sample_linear_pose(&next, phase).is_err());
        }
        let body = Body::new(
            rest.clone(),
            vec![false; 4],
            vec![(
                [0, 1, 2, 3],
                Material::from_young_poisson(300., 0.4).unwrap(),
            )],
        )
        .unwrap();
        let mut dynamics = InertialBody::new(body, &[1000.], vec![[0.; 3]; 4]).unwrap();
        dynamics
            .set_stationary_embedded_contact(Some(current.clone()))
            .unwrap();
        let before = format!("{dynamics:?}");
        let path = obstacle
            .prepare_path_partition(next.obstacle(), &[0., 0.25, 0.5, 0.75, 1.])
            .unwrap();
        let nodes: Vec<_> = path.quadrature_nodes().collect();
        let samples: Vec<_> = nodes
            .iter()
            .map(|(t, _)| current.sample_linear_pose(&next, *t).unwrap())
            .collect();
        let initial = dynamics
            .evaluate_at_contacts(&rest, None, None)
            .unwrap()
            .potential_j;
        let points = [
            [1e-6, -2e-6, 3e-6],
            [-2e-6, 1e-6, 2e-6],
            [1e-6, 3e-6, -1e-6],
            [-1e-6, 2e-6, 4e-6],
        ];
        let end: Vec<_> = rest
            .iter()
            .zip(points)
            .map(|(p, d)| std::array::from_fn(|a| p[a] + 2. * d[a]))
            .collect();
        let evaluate = |points: &[Vec3]| {
            dynamics
                .average_material_path_with_skin(
                    points,
                    &end,
                    &nodes,
                    initial,
                    Some((&next, &samples)),
                )
                .unwrap()
        };
        let average = evaluate(&points);
        for node in 0..4 {
            for axis in 0..3 {
                let mut plus = points;
                let mut minus = points;
                let h = 1e-7;
                plus[node][axis] += h;
                minus[node][axis] -= h;
                let fd = (evaluate(&plus).potential_j - evaluate(&minus).potential_j) / (2. * h);
                assert!(
                    (fd - average.gradient[node][axis]).abs() < 1e-6 * (1. + fd.abs()),
                    "node={node} axis={axis} fd={fd} gradient={}",
                    average.gradient[node][axis]
                );
            }
        }
        let final_energy = dynamics
            .evaluate_at_contacts_and_skin(&end, None, None, Some(&next))
            .unwrap()
            .potential_j;
        let gradient_work: f64 = average
            .gradient
            .iter()
            .zip(&points)
            .flat_map(|(g, d)| (0..3).map(move |a| 2. * g[a] * d[a]))
            .sum();
        let work = average.embedded_skin_work;
        assert!(work.rig_work_j.abs() > 1e-10 && work.obstacle_work_j.abs() > 1e-10);
        let defect =
            final_energy - initial - gradient_work - work.rig_work_j - work.obstacle_work_j;
        assert!(defect.abs() < 1e-10, "independent defect={defect}");
        assert!(
            dynamics
                .average_material_path_with_skin(
                    &points,
                    &end,
                    &nodes,
                    initial,
                    Some((&next, &samples[..samples.len() - 1]))
                )
                .is_err()
        );
        assert_eq!(before, format!("{dynamics:?}"));
    }
    #[test]
    fn coupled_direction_matches_rank_one_inverse_and_holds_pins() {
        let normal = [0.6, 0.8, 0.];
        let block = super::super::super::PrescribedContactStencil {
            body_face: [0, 1, 2],
            obstacle_face_index: 0,
            obstacle_face: [0, 1, 2],
            body_weights: [0.2, 0.3, 0.5],
            obstacle_weights: [1., 0., 0.],
            normal,
            normal_curvature_n_m: 1000.,
        };
        let weight = [2., 3., 4.];
        let pinned = [false, true, false];
        let residual = [[3., -1., 2.], [99.; 3], [-2., 4., 1.]];
        let mut diagonal = weight.map(|w| [w; 3]);
        let u: [Vec3; 3] = std::array::from_fn(|node| {
            if pinned[node] {
                [0.; 3]
            } else {
                normal.map(|v| v * block.body_weights[node])
            }
        });
        for node in 0..3 {
            for axis in 0..3 {
                diagonal[node][axis] += 1000. * u[node][axis].powi(2);
            }
        }
        let actual = coupled_contact_direction(
            &weight,
            &diagonal,
            std::slice::from_ref(&block),
            &pinned,
            &residual,
        );
        let ur: f64 = (0..3)
            .map(|node| dot(u[node], residual[node]) / weight[node])
            .sum();
        let uu: f64 = (0..3)
            .map(|node| dot(u[node], u[node]) / weight[node])
            .sum();
        for node in [0, 2] {
            for axis in 0..3 {
                let expected = -residual[node][axis] / weight[node]
                    + 1000. * u[node][axis] / weight[node] * ur / (1. + 1000. * uu);
                assert!((actual[node][axis] - expected).abs() < 1e-10);
            }
        }
        assert_eq!(actual[1], [0.; 3]);
        // A = 5*B, where B is the coupled rank-one search operator. The
        // generalized Rayleigh scale must recover A^-1 = B^-1/5.
        let displacement: [Vec3; 3] = [[0.01, -0.02, 0.03], [0.; 3], [-0.03, 0.01, 0.02]];
        let us: f64 = (0..3).map(|i| dot(u[i], displacement[i])).sum();
        let change: Vec<Vec3> = (0..3)
            .map(|i| {
                if pinned[i] {
                    [0.; 3]
                } else {
                    std::array::from_fn(|axis| {
                        5. * (weight[i] * displacement[i][axis] + 1000. * u[i][axis] * us)
                    })
                }
            })
            .collect();
        let mut history = Vec::new();
        push_secant(&mut history, displacement.to_vec(), change, 12);
        let gamma = secant_scale(&history, |y| {
            y.iter()
                .zip(coupled_contact_direction(
                    &weight,
                    &diagonal,
                    std::slice::from_ref(&block),
                    &pinned,
                    y,
                ))
                .map(|(y, d)| -dot(*y, d))
                .sum()
        });
        // PCG targets a relative preconditioned residual of 1e-6; the
        // independent closed-form inverse is compared at that solver accuracy.
        assert!((gamma - 0.2).abs() < 0.2 * 1e-6);
        let scaled = secant_direction(&history, &residual, |q| {
            coupled_contact_direction(&weight, &diagonal, std::slice::from_ref(&block), &pinned, q)
                .iter()
                .map(|d| d.map(|v| -gamma * v))
                .collect()
        });
        for node in 0..3 {
            for axis in 0..3 {
                assert!((scaled[node][axis] - actual[node][axis] / 5.).abs() < 1e-6);
            }
        }
        assert_eq!(scaled[1], [0.; 3]);
    }
}
