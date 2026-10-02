//! Trapezoidal endpoint-force integration: exact energy balance for linear elasticity.
//! Nonlinear plastic/contact/fracture trajectories require timestep refinement.
use super::{Body, Vec3, dot, reduced, solve_dense, sub};
#[derive(Clone, Debug)]
pub struct DynamicBody {
    body: Body,
    masses: Vec<f64>,
    velocities: Vec<Vec3>,
}
#[derive(Clone, Debug)]
pub struct DynamicStep {
    pub converged: bool,
    pub iterations: usize,
    pub residual_n: f64,
    pub reactions_n: Vec<Vec3>,
    /// Midpoint external-force work, excluding constraint work.
    pub external_work_j: f64,
    pub constraint_work_j: f64,
    /// Signed change of the full energy ledger minus external/constraint work.
    /// Present once force balance converges; energy failure still rejects the step.
    pub energy_defect_j: Option<f64>,
}
/// Limits for adaptive free-body advancement with constant applied forces.
#[derive(Clone, Copy, Debug)]
pub struct AdvanceLimits {
    pub minimum_dt_s: f64,
    pub max_steps: usize,
    pub max_iterations: usize,
    pub force_tolerance_n: f64,
    /// Absolute energy-defect budget across the entire interval.
    pub energy_tolerance_j: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Diagnostics {
    pub mass_kg: f64,
    pub momentum_kg_m_s: Vec3,
    pub kinetic_j: f64,
    pub elastic_j: f64,
    pub hardening_j: f64,
    pub plastic_dissipated_j: f64,
    pub interface_stored_j: f64,
    pub fracture_dissipated_j: f64,
    pub friction_dissipated_j: f64,
    pub friction_numerical_j: f64,
    pub friction_released_j: f64,
}
#[derive(Clone, Debug)]
pub struct Fragment {
    pub nodes: Vec<usize>,
    pub mass_kg: f64,
    pub center_m: Vec3,
    pub momentum_kg_m_s: Vec3,
    pub velocity_m_s: Vec3,
}
impl DynamicBody {
    /// Advance an unconstrained body under constant loads/acceleration.
    /// Rejected force/energy candidates are bisected; absolute accepted defects
    /// consume the interval energy budget. Accepted substeps publish only when the
    /// whole interval succeeds. This controls the energy ledger, not phase accuracy.
    /// # Errors
    /// Invalid inputs, constitutive failures, minimum step or work limit reached.
    pub fn advance_free(
        &mut self,
        interval_s: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        limits: AdvanceLimits,
    ) -> Result<Vec<DynamicStep>, &'static str> {
        if !interval_s.is_finite()
            || interval_s <= 0.
            || !limits.minimum_dt_s.is_finite()
            || limits.minimum_dt_s <= 0.
            || limits.minimum_dt_s > interval_s
            || limits.max_steps == 0
        {
            return Err("invalid adaptive interval limits");
        }
        let mut candidate = self.clone();
        let prescribed = vec![[None; 3]; self.masses.len()];
        let mut pending = vec![interval_s];
        let mut reports = Vec::new();
        let mut attempts = 0;
        let mut remaining_energy = limits.energy_tolerance_j;
        while let Some(dt) = pending.pop() {
            if attempts == limits.max_steps {
                return Err("adaptive step work limit reached");
            }
            attempts += 1;
            let report = candidate.step(
                dt,
                loads,
                acceleration,
                &prescribed,
                limits.max_iterations,
                limits.force_tolerance_n,
                remaining_energy,
            )?;
            if report.converged {
                remaining_energy -= report
                    .energy_defect_j
                    .ok_or("missing converged energy diagnostic")?
                    .abs();
                reports.push(report);
            } else {
                let half = dt * 0.5;
                if half < limits.minimum_dt_s || half >= dt {
                    return Err("adaptive minimum timestep reached");
                }
                pending.push(half);
                pending.push(half);
            }
        }
        *self = candidate;
        Ok(reports)
    }
    /// Connected solid components, merging faces with any remaining cohesive bond.
    /// Fully fractured contact pairs do not merge fragments on reclosure.
    /// # Errors
    /// Rejects nonfinite interface responses or fragment diagnostics.
    pub fn fragments(&self) -> Result<Vec<Fragment>, &'static str> {
        fn root(parents: &mut [usize], mut node: usize) -> usize {
            while parents[node] != node {
                parents[node] = parents[parents[node]];
                node = parents[node];
            }
            node
        }
        let mut parents: Vec<_> = (0..self.masses.len()).collect();
        let mut join = |a, b| {
            let a = root(&mut parents, a);
            let b = root(&mut parents, b);
            parents[a.max(b)] = a.min(b);
        };
        for element in &self.body.elements {
            for &node in &element.nodes[1..] {
                join(element.nodes[0], node);
            }
        }
        for interface in self.body.interface_reports()? {
            if interface.quadrature.iter().any(|q| q.damage < 1.) {
                for node in interface.minus.into_iter().chain(interface.plus) {
                    join(interface.minus[0], node);
                }
            }
        }
        let mut groups = std::collections::BTreeMap::<usize, Vec<usize>>::new();
        for node in 0..self.masses.len() {
            groups
                .entry(root(&mut parents, node))
                .or_default()
                .push(node);
        }
        groups
            .into_values()
            .map(|nodes| {
                let mass: f64 = nodes.iter().map(|&node| self.masses[node]).sum();
                let center: Vec3 = std::array::from_fn(|axis| {
                    nodes
                        .iter()
                        .map(|&node| (self.masses[node] / mass) * self.body.positions[node][axis])
                        .sum::<f64>()
                });
                let momentum: Vec3 = std::array::from_fn(|axis| {
                    nodes
                        .iter()
                        .map(|&node| self.masses[node] * self.velocities[node][axis])
                        .sum()
                });
                let velocity = momentum.map(|v| v / mass);
                if !mass.is_finite()
                    || center
                        .iter()
                        .chain(&momentum)
                        .chain(&velocity)
                        .any(|v| !v.is_finite())
                {
                    return Err("fragment diagnostic overflow");
                }
                Ok(Fragment {
                    nodes,
                    mass_kg: mass,
                    center_m: center,
                    momentum_kg_m_s: momentum,
                    velocity_m_s: velocity,
                })
            })
            .collect()
    }
    /// Lump each tetrahedron's reference mass equally onto its four nodes.
    /// Density is per element in kg/m³; velocities are nodal m/s.
    /// # Errors
    /// Rejects invalid density/velocity counts, nonpositive mass and overflow.
    pub fn new(body: Body, densities: &[f64], velocities: Vec<Vec3>) -> Result<Self, &'static str> {
        if densities.len() != body.elements.len()
            || densities.iter().any(|v| !v.is_finite() || *v <= 0.)
            || velocities.len() != body.rest.len()
            || velocities.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid dynamic solid input");
        }
        let mut masses = vec![0.; body.rest.len()];
        for (element, &density) in body.elements.iter().zip(densities) {
            for &node in &element.nodes {
                masses[node] += element.volume * density / 4.;
            }
        }
        if masses.iter().any(|v| !v.is_finite() || *v <= 0.) {
            return Err("dynamic nodal mass overflow");
        }
        let result = Self {
            body,
            masses,
            velocities,
        };
        result.diagnostics()?;
        Ok(result)
    }
    #[must_use]
    pub fn body(&self) -> &Body {
        &self.body
    }
    #[must_use]
    pub fn velocities(&self) -> &[Vec3] {
        &self.velocities
    }
    #[must_use]
    pub fn masses(&self) -> &[f64] {
        &self.masses
    }
    /// # Errors
    /// Rejects nonfinite energy, momentum or material responses.
    pub fn diagnostics(&self) -> Result<Diagnostics, &'static str> {
        let mut result = Diagnostics {
            mass_kg: self.masses.iter().sum(),
            momentum_kg_m_s: [0.; 3],
            kinetic_j: 0.,
            elastic_j: 0.,
            hardening_j: 0.,
            plastic_dissipated_j: 0.,
            interface_stored_j: 0.,
            fracture_dissipated_j: 0.,
            friction_dissipated_j: 0.,
            friction_numerical_j: 0.,
            friction_released_j: 0.,
        };
        for (&mass, velocity) in self.masses.iter().zip(&self.velocities) {
            result.kinetic_j += 0.5 * mass * dot(*velocity, *velocity);
            for (i, value) in result.momentum_kg_m_s.iter_mut().enumerate() {
                *value += mass * velocity[i];
            }
        }
        for ((element, state), response) in self
            .body
            .elements
            .iter()
            .zip(self.body.states())
            .zip(self.body.responses()?)
        {
            result.elastic_j += element.volume * response.elastic_energy_j_m3;
            result.hardening_j += element.volume * response.hardening_energy_j_m3;
            result.plastic_dissipated_j += element.volume * state.dissipated_j_m3();
        }
        for interface in self.body.interface_reports()? {
            result.interface_stored_j += interface.stored_j;
            result.fracture_dissipated_j += interface.dissipated_j;
            result.friction_dissipated_j += interface.friction_dissipated_j;
            result.friction_numerical_j += interface.friction_numerical_j;
            result.friction_released_j += interface.friction_released_j;
        }
        if [
            result.mass_kg,
            result.kinetic_j,
            result.elastic_j,
            result.hardening_j,
            result.plastic_dissipated_j,
            result.interface_stored_j,
            result.fracture_dissipated_j,
            result.friction_dissipated_j,
            result.friction_numerical_j,
            result.friction_released_j,
        ]
        .iter()
        .chain(&result.momentum_kg_m_s)
        .any(|v| !v.is_finite())
        {
            return Err("dynamic diagnostic overflow");
        }
        Ok(result)
    }
    /// Nodal external forces and uniform acceleration are sampled at the midpoint.
    /// Prescribed displacements are endpoint values relative to reference positions.
    /// Solves `m*(v_new-v_old)/dt + (f_int_old+f_int_new)/2 = f_ext_mid`,
    /// with `x_new-x_old = dt*(v_old+v_new)/2`. Initial acceleration is not required.
    /// Material histories are evaluated at endpoint strain from the last accepted
    /// state and commit atomically with positions/velocities only after convergence.
    /// The supplied energy tolerance in joules must also be met. Force-converged
    /// but energy-inconsistent candidates return `converged=false` with a defect.
    /// Released contact spring energy is an unresolved reservoir in this ledger;
    /// numerical endpoint-work defect is not counted as physical dissipation.
    /// # Errors
    /// Rejects invalid inputs, singular tangents, inversion and overflow.
    #[allow(clippy::too_many_lines)] // Keep all step candidates and the final commit transactional.
    #[allow(clippy::too_many_arguments)] // Force and energy tolerances use distinct physical units.
    pub fn step(
        &mut self,
        dt: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        prescribed: &[[Option<f64>; 3]],
        max_iterations: usize,
        tolerance_n: f64,
        energy_tolerance_j: f64,
    ) -> Result<DynamicStep, &'static str> {
        if !dt.is_finite()
            || dt <= 0.
            || loads.len() != self.masses.len()
            || prescribed.len() != self.masses.len()
            || loads
                .iter()
                .flatten()
                .chain(&acceleration)
                .any(|v| !v.is_finite())
            || prescribed
                .iter()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
            || max_iterations == 0
            || !tolerance_n.is_finite()
            || tolerance_n <= 0.
            || !energy_tolerance_j.is_finite()
            || energy_tolerance_j <= 0.
        {
            return Err("invalid dynamic step");
        }
        let old_diagnostics = self.diagnostics()?;
        let inertia: Vec<_> = self.masses.iter().map(|m| 2. * (m / dt) / dt).collect();
        if inertia.iter().any(|v| !v.is_finite() || *v <= 0.) {
            return Err("dynamic timestep inertia overflow");
        }
        let mut count = 0;
        let dofs: Vec<[Option<usize>; 3]> = prescribed
            .iter()
            .map(|row| {
                row.map(|value| {
                    if value.is_none() {
                        let index = count;
                        count += 1;
                        Some(index)
                    } else {
                        None
                    }
                })
            })
            .collect();
        let old_positions = &self.body.positions;
        let prediction: Vec<Vec3> = old_positions
            .iter()
            .zip(&self.velocities)
            .map(|(x, v)| std::array::from_fn(|i| x[i] + dt * v[i]))
            .collect();
        let external: Vec<Vec3> = loads
            .iter()
            .zip(&self.masses)
            .map(|(force, mass)| std::array::from_fn(|i| force[i] + mass * acceleration[i]))
            .collect();
        if prediction
            .iter()
            .chain(&external)
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err("dynamic prediction overflow");
        }
        let old_internal = self
            .body
            .evaluate(old_positions, &dofs, count, false)?
            .internal;
        let residual_for = |x: &[Vec3], internal: &[Vec3]| -> Vec<Vec3> {
            x.iter()
                .enumerate()
                .map(|(node, row)| {
                    std::array::from_fn(|axis| {
                        inertia[node] * (row[axis] - prediction[node][axis])
                            + old_internal[node][axis].midpoint(internal[node][axis])
                            - external[node][axis]
                    })
                })
                .collect()
        };
        let mut x = prediction.clone();
        for (node, row) in prescribed.iter().enumerate() {
            for (axis, value) in row.iter().enumerate() {
                if let Some(value) = value {
                    x[node][axis] = self.body.rest[node][axis] + value;
                }
            }
        }
        for iteration in 0..=max_iterations {
            let mut evaluation = self.body.evaluate(&x, &dofs, count, true)?;
            let reactions = residual_for(&x, &evaluation.internal);
            let residual = reduced(&reactions, &dofs, count);
            let norm = residual.iter().fold(0_f64, |a, v| a.hypot(*v));
            if !norm.is_finite() || reactions.iter().flatten().any(|v| !v.is_finite()) {
                return Err("dynamic residual overflow");
            }
            let mut report = DynamicStep {
                converged: false,
                iterations: iteration,
                residual_n: norm,
                reactions_n: reactions,
                external_work_j: 0.,
                constraint_work_j: 0.,
                energy_defect_j: None,
            };
            if norm <= tolerance_n {
                let velocities: Vec<Vec3> = x
                    .iter()
                    .enumerate()
                    .map(|(node, row)| {
                        std::array::from_fn(|axis| {
                            2. * ((row[axis] - old_positions[node][axis]) / dt)
                                - self.velocities[node][axis]
                        })
                    })
                    .collect();
                report.external_work_j = x
                    .iter()
                    .enumerate()
                    .map(|(node, row)| dot(external[node], sub(*row, old_positions[node])))
                    .sum();
                report.constraint_work_j = x
                    .iter()
                    .enumerate()
                    .map(|(node, row)| {
                        (0..3)
                            .filter(|&axis| prescribed[node][axis].is_some())
                            .map(|axis| {
                                report.reactions_n[node][axis]
                                    * (row[axis] - old_positions[node][axis])
                            })
                            .sum::<f64>()
                    })
                    .sum();
                if velocities.iter().flatten().any(|v| !v.is_finite())
                    || !report.external_work_j.is_finite()
                    || !report.constraint_work_j.is_finite()
                {
                    return Err("dynamic accepted-step overflow");
                }
                // Validate every candidate diagnostic before publishing anything.
                let mut candidate = self.clone();
                candidate.body.positions = x;
                candidate.velocities = velocities;
                for (element, state) in candidate.body.elements.iter_mut().zip(evaluation.states) {
                    element.state = state;
                }
                for (interface, states) in candidate
                    .body
                    .interfaces
                    .iter_mut()
                    .zip(evaluation.interface_states)
                {
                    interface.states = states;
                }
                let diagnostics = candidate.diagnostics()?;
                let energy_change = [
                    diagnostics.kinetic_j - old_diagnostics.kinetic_j,
                    diagnostics.elastic_j - old_diagnostics.elastic_j,
                    diagnostics.hardening_j - old_diagnostics.hardening_j,
                    diagnostics.plastic_dissipated_j - old_diagnostics.plastic_dissipated_j,
                    diagnostics.interface_stored_j - old_diagnostics.interface_stored_j,
                    diagnostics.fracture_dissipated_j - old_diagnostics.fracture_dissipated_j,
                    diagnostics.friction_dissipated_j - old_diagnostics.friction_dissipated_j,
                    diagnostics.friction_released_j - old_diagnostics.friction_released_j,
                ]
                .iter()
                .sum::<f64>();
                let defect = energy_change - report.external_work_j - report.constraint_work_j;
                if !defect.is_finite() {
                    return Err("dynamic energy defect overflow");
                }
                report.energy_defect_j = Some(defect);
                if defect.abs() > energy_tolerance_j {
                    return Ok(report);
                }
                *self = candidate;
                report.converged = true;
                return Ok(report);
            }
            if iteration == max_iterations {
                return Ok(report);
            }
            for row in &mut evaluation.stiffness {
                for value in row {
                    *value *= 0.5;
                }
            }
            for (node, row) in dofs.iter().enumerate() {
                for &index in row.iter().flatten() {
                    evaluation.stiffness[index][index] += inertia[node];
                }
            }
            if evaluation
                .stiffness
                .iter()
                .flatten()
                .any(|v| !v.is_finite())
            {
                return Err("dynamic tangent overflow");
            }
            let direction =
                solve_dense(evaluation.stiffness, residual.iter().map(|v| -v).collect())?;
            let mut fraction = 1.;
            let mut accepted = None;
            for _ in 0..32 {
                let trial: Vec<Vec3> = x
                    .iter()
                    .enumerate()
                    .map(|(node, row)| {
                        std::array::from_fn(|axis| {
                            row[axis]
                                + dofs[node][axis].map_or(0., |index| fraction * direction[index])
                        })
                    })
                    .collect();
                if let Ok(candidate) = self.body.evaluate(&trial, &dofs, count, false) {
                    let forces = residual_for(&trial, &candidate.internal);
                    let trial_norm = reduced(&forces, &dofs, count)
                        .iter()
                        .fold(0_f64, |a, v| a.hypot(*v));
                    if forces.iter().flatten().all(|v| v.is_finite())
                        && trial_norm.is_finite()
                        && trial_norm < norm * (1. - 1e-4 * fraction)
                    {
                        accepted = Some(trial);
                        break;
                    }
                }
                fraction *= 0.5;
            }
            let Some(trial) = accepted else {
                return Ok(report);
            };
            x = trial;
        }
        unreachable!()
    }
}
