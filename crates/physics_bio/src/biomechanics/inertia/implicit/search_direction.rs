//! Preconditioned conjugate-gradient search direction and rest-material metric operators.
use super::{
    InertialBody, SupportTarget, Vec3, dot, push_secant, try_secant_direction, try_secant_scale,
};

impl InertialBody {
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
    ) -> Result<super::DrivenSupportStep, &'static str> {
        self.step_implicit_supports_controlled(targets, dt, tolerance_j, None)
    }
    pub(in crate::biomechanics::inertia) fn step_implicit_supports_controlled(
        &mut self,
        targets: Option<&[SupportTarget]>,
        dt: f64,
        tolerance_j: f64,
        motion_tolerance: Option<(f64, f64)>,
    ) -> Result<super::DrivenSupportStep, &'static str> {
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
        let search_operation = self.prepare_tissue_search(&weight)?;
        let rest_moduli = search_operation
            .is_none()
            .then(|| rest_material_moduli(&self.body));
        let inverse_material = |g: &[Vec3]| {
            try_preconditioned_direction(
                &self.body.pinned,
                g,
                |v| {
                    if let Some(operation) = search_operation.as_deref() {
                        crate::biomechanics::search_snapshot::checked_search_action(
                            operation,
                            v,
                            &self.body.pinned,
                        )
                    } else {
                        Ok(rest_material_action_prepared(
                            &self.body,
                            &weight,
                            v,
                            rest_moduli.as_deref(),
                        ))
                    }
                },
                |r| {
                    r.iter()
                        .enumerate()
                        .map(|(i, r)| r.map(|x| x * inverse_diagonal[i]))
                        .collect()
                },
            )
            .map(|vectors| {
                vectors
                    .into_iter()
                    .map(|v| v.map(|x| -x))
                    .collect::<Vec<_>>()
            })
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
            let scale = try_secant_scale(&history, |g| {
                inverse_material(g)
                    .map(|values| g.iter().zip(values).map(|(g, v)| dot(*g, v)).sum())
            })?;
            let direction = try_secant_direction(&history, &state.1, |g| {
                inverse_material(g)
                    .map(|values| values.into_iter().map(|v| v.map(|x| scale * x)).collect())
            })?;
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
                .map(|(a, b)| crate::biomechanics::sub(*a, *b))
                .collect();
            let gradient_change = value
                .1
                .iter()
                .zip(&state.1)
                .map(|(a, b)| crate::biomechanics::sub(*a, *b))
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
        let kinetic_change = self.free_kinetic_change(&velocity);
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
        Ok(super::DrivenSupportStep {
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

pub(super) fn relative_midpoint_endpoint(
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
#[cfg(test)]
pub(super) fn coupled_contact_direction(
    weight: &[f64],
    diagonal: &[Vec3],
    blocks: &[crate::biomechanics::PrescribedContactStencil],
    pinned: &[bool],
    residual: &[Vec3],
) -> Vec<Vec3> {
    coupled_contact_direction_with_material(weight, diagonal, blocks, pinned, residual, None)
}

#[cfg(test)]
pub(super) fn coupled_contact_direction_with_material(
    weight: &[f64],
    diagonal: &[Vec3],
    blocks: &[crate::biomechanics::PrescribedContactStencil],
    pinned: &[bool],
    residual: &[Vec3],
    body: Option<&crate::biomechanics::Body>,
) -> Vec<Vec3> {
    try_coupled_contact_direction_with_material(
        weight, diagonal, blocks, pinned, residual, body, None,
    )
    .unwrap()
}
#[cfg(test)]
pub(super) fn try_coupled_contact_direction_with_material(
    weight: &[f64],
    diagonal: &[Vec3],
    blocks: &[crate::biomechanics::PrescribedContactStencil],
    pinned: &[bool],
    residual: &[Vec3],
    body: Option<&crate::biomechanics::Body>,
    operation: Option<&dyn crate::biomechanics::TissueSearchOperation>,
) -> Result<Vec<Vec3>, &'static str> {
    try_coupled_contact_direction_with_material_prepared(
        weight, diagonal, blocks, pinned, residual, body, None, operation,
    )
}
pub(super) fn try_coupled_contact_direction_with_material_prepared(
    weight: &[f64],
    diagonal: &[Vec3],
    blocks: &[crate::biomechanics::PrescribedContactStencil],
    pinned: &[bool],
    residual: &[Vec3],
    body: Option<&crate::biomechanics::Body>,
    moduli: Option<&[(f64, f64)]>,
    operation: Option<&dyn crate::biomechanics::TissueSearchOperation>,
) -> Result<Vec<Vec3>, &'static str> {
    let apply = |v: &[Vec3]| {
        // The material operator already includes inertia. Construct only the
        // selected operator output instead of allocating and discarding an
        // inertia-only vector at every conjugate-gradient iteration.
        let mut result: Vec<Vec3> = if let Some(body) = body {
            if let Some(operation) = operation {
                crate::biomechanics::search_snapshot::checked_search_action(operation, v, pinned)?
            } else {
                rest_material_action_prepared(body, weight, v, moduli)
            }
        } else {
            v.iter()
                .zip(weight)
                .enumerate()
                .map(|(node, (v, w))| {
                    if pinned[node] {
                        [0.; 3]
                    } else {
                        std::array::from_fn(|axis| w * v[axis])
                    }
                })
                .collect()
        };
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
        Ok(result)
    };
    try_preconditioned_direction(pinned, residual, apply, |r| {
        r.iter()
            .zip(diagonal)
            .map(|(r, d)| std::array::from_fn(|axis| r[axis] / d[axis]))
            .collect()
    })
}

// Rest isotropic elasticity plus inertia is only a search metric. Nonlinear
// forces, anisotropy, memory and endpoint work remain in the real evaluator.
pub(super) fn rest_material_moduli(body: &crate::biomechanics::Body) -> Vec<(f64, f64)> {
    body.elements
        .iter()
        .map(|e| {
            e.viscoelastic
                .as_ref()
                .map_or((e.material.shear_pa, e.material.bulk_pa), |m| {
                    m.rest_moduli()
                })
        })
        .collect()
}

pub(in crate::biomechanics) fn rest_material_action(
    body: &crate::biomechanics::Body,
    weight: &[f64],
    v: &[Vec3],
) -> Vec<Vec3> {
    rest_material_action_prepared(body, weight, v, None)
}

// Moduli belong to one immutable search solve, never to a persistent body cache.
pub(super) fn rest_material_action_prepared(
    body: &crate::biomechanics::Body,
    weight: &[f64],
    v: &[Vec3],
    moduli: Option<&[(f64, f64)]>,
) -> Vec<Vec3> {
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
    for (index, e) in body.elements.iter().enumerate() {
        let (mu, bulk) = moduli.map_or_else(
            || {
                e.viscoelastic
                    .as_ref()
                    .map_or((e.material.shear_pa, e.material.bulk_pa), |m| {
                        m.rest_moduli()
                    })
            },
            |values| values[index],
        );
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
        let mut stress = [[0.; 3]; 3];
        for a in 0..3 {
            for b in a..3 {
                let sym = 0.5 * (h[a][b] + h[b][a]);
                let dev = if a == b { sym - trace / 3. } else { sym };
                let value = 2. * mu * dev + if a == b { bulk * trace } else { 0. };
                stress[a][b] = value;
                stress[b][a] = value;
            }
        }
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
#[cfg(test)]
pub(super) fn preconditioned_direction(
    pinned: &[bool],
    residual: &[Vec3],
    apply: impl Fn(&[Vec3]) -> Vec<Vec3>,
    precondition: impl Fn(&[Vec3]) -> Vec<Vec3>,
) -> Vec<Vec3> {
    if let Ok(value) = std::env::var("VOXY_TEST_SEARCH_ACTION_LIMIT") {
        let limit: usize = value.parse().unwrap();
        assert!((1..=4096).contains(&limit));
        let report = try_preconditioned_direction_report_with_limit::<std::convert::Infallible>(
            pinned,
            residual,
            |v| Ok(apply(v)),
            precondition,
            limit,
        )
        .unwrap();
        let ratio = true_search_residual_ratio(pinned, residual, &apply(&report.direction));
        eprintln!(
            "TEST_SEARCH_BUDGET limit={limit} actions={} stop={:?} squared_l2_residual_ratio={ratio:?}",
            report.actions, report.stop
        );
        return report.direction;
    }
    try_preconditioned_direction::<std::convert::Infallible>(
        pinned,
        residual,
        |v| Ok(apply(v)),
        precondition,
    )
    .unwrap_or_else(|never| match never {})
}

// Backend errors propagate immediately; only an accepted numerical metric retains
// the existing diagonal direction fallback. Physical owners decide transaction commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SearchDirectionStop {
    RecursiveTolerance,
    IterationLimit,
    NonpositiveRecursiveResidual,
    InvalidCurvature,
    InvalidDirection,
    ZeroResidual,
}
pub(super) struct SearchDirectionReport {
    pub(super) direction: Vec<Vec3>,
    pub(super) actions: usize,
    pub(super) stop: SearchDirectionStop,
    pub(super) initial_rz: f64,
    pub(super) final_rz: f64,
    pub(super) used_fallback: bool,
}
pub(super) fn try_preconditioned_direction<E>(
    pinned: &[bool],
    residual: &[Vec3],
    apply: impl Fn(&[Vec3]) -> Result<Vec<Vec3>, E>,
    precondition: impl Fn(&[Vec3]) -> Vec<Vec3>,
) -> Result<Vec<Vec3>, E> {
    let report = try_preconditioned_direction_report(pinned, residual, &apply, precondition)?;
    if std::env::var_os("VOXY_CONTACT_SEARCH_VERIFY").is_some() {
        // Optional diagnostic action; never changes the accepted direction or policy.
        let diagnostic_action = apply(&report.direction);
        let action_succeeded = diagnostic_action.is_ok();
        let ratio = diagnostic_action
            .ok()
            .and_then(|action| true_search_residual_ratio(pinned, residual, &action));
        eprintln!(
            "TISSUE_SEARCH_EQUATION nodes={} diagnostic_actions=1 operator_action_succeeded={action_succeeded} squared_l2_residual_ratio={ratio:?} diagnostic_only=true",
            residual.len()
        );
    }
    if std::env::var_os("VOXY_CONTACT_SEARCH_TRACE").is_some() {
        let ratio = (report.initial_rz > 0. && report.initial_rz.is_finite())
            .then(|| report.final_rz / report.initial_rz);
        eprintln!(
            "TISSUE_SEARCH nodes={} actions={} stop={:?} used_fallback={} recursive_squared_residual_ratio={ratio:?} true_equation_residual_verified=false",
            residual.len(),
            report.actions,
            report.stop,
            report.used_fallback
        );
    }
    Ok(report.direction)
}
pub(super) fn true_search_residual_ratio(
    pinned: &[bool],
    residual: &[Vec3],
    action: &[Vec3],
) -> Option<f64> {
    if pinned.len() != residual.len() || action.len() != residual.len() {
        return None;
    }
    let mut original = 0.;
    let mut remaining = 0.;
    for node in 0..residual.len() {
        if pinned[node] {
            continue;
        }
        for axis in 0..3 {
            let r = residual[node][axis];
            let a = action[node][axis];
            if !r.is_finite() || !a.is_finite() {
                return None;
            }
            original += r * r;
            remaining += (r + a).powi(2);
        }
    }
    if !original.is_finite() || !remaining.is_finite() {
        return None;
    }
    if original == 0. {
        return (remaining == 0.).then_some(0.);
    }
    let ratio = remaining / original;
    ratio.is_finite().then_some(ratio)
}
#[cfg(test)]
mod true_search_residual_tests {
    use super::*;
    #[test]
    fn equation_residual_uses_returned_direction_and_free_nodes() {
        let r = [[2., -4., 6.], [9.; 3]];
        assert_eq!(
            true_search_residual_ratio(&[false, true], &r, &[[-2., 4., -6.], [0.; 3]]),
            Some(0.)
        );
        assert_eq!(
            true_search_residual_ratio(&[false, true], &r, &[[0.; 3]; 2]),
            Some(1.)
        );
        assert_eq!(
            true_search_residual_ratio(&[false, true], &r, &[[-1., 2., -3.], [0.; 3]]),
            Some(0.25)
        );
        assert_eq!(true_search_residual_ratio(&[false], &r, &[]), None);
        assert_eq!(
            true_search_residual_ratio(&[false], &[[0.; 3]], &[[1.; 3]]),
            None
        );
        assert_eq!(
            true_search_residual_ratio(&[false], &[[f64::MAX; 3]], &[[0.; 3]]),
            None
        );
    }
}
pub(super) fn try_preconditioned_direction_report<E>(
    pinned: &[bool],
    residual: &[Vec3],
    apply: impl Fn(&[Vec3]) -> Result<Vec<Vec3>, E>,
    precondition: impl Fn(&[Vec3]) -> Vec<Vec3>,
) -> Result<SearchDirectionReport, E> {
    try_preconditioned_direction_report_with_limit(pinned, residual, apply, precondition, 64)
}
pub(super) fn try_preconditioned_direction_report_with_limit<E>(
    pinned: &[bool],
    residual: &[Vec3],
    apply: impl Fn(&[Vec3]) -> Result<Vec<Vec3>, E>,
    precondition: impl Fn(&[Vec3]) -> Vec<Vec3>,
    action_limit: usize,
) -> Result<SearchDirectionReport, E> {
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
    let mut actions = 0;
    let mut stop = SearchDirectionStop::IterationLimit;
    for _ in 0..action_limit {
        if rz <= initial * 1e-12 || rz <= 0. {
            stop = if rz < 0. {
                SearchDirectionStop::NonpositiveRecursiveResidual
            } else {
                SearchDirectionStop::RecursiveTolerance
            };
            break;
        }
        let action = apply(&search)?;
        actions += 1;
        let curvature = inner(&search, &action);
        if !curvature.is_finite() || curvature <= 0. {
            return Ok(SearchDirectionReport {
                direction: fallback,
                actions,
                stop: SearchDirectionStop::InvalidCurvature,
                initial_rz: initial,
                final_rz: rz,
                used_fallback: true,
            });
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
        if matches!(stop, SearchDirectionStop::IterationLimit) && rz <= initial * 1e-12 {
            stop = SearchDirectionStop::RecursiveTolerance;
        }
        Ok(SearchDirectionReport {
            direction,
            actions,
            stop,
            initial_rz: initial,
            final_rz: rz,
            used_fallback: false,
        })
    } else {
        Ok(SearchDirectionReport {
            direction: fallback,
            actions,
            stop: if initial == 0. && actions == 0 {
                SearchDirectionStop::ZeroResidual
            } else {
                SearchDirectionStop::InvalidDirection
            },
            initial_rz: initial,
            final_rz: rz,
            used_fallback: true,
        })
    }
}
