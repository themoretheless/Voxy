//! Explicit small-strain quadratic FEM with consistent inertia.
use super::{ConsistentInertia, QuadraticBody};
#[path = "dynamic_contact.rs"]
mod dynamic_contact;
mod fracture_work;
mod fragment_impact;
mod rotation;
pub use fragment_impact::QuadraticFragmentImpactConstraint;
mod automatic_impact;
pub use automatic_impact::{
    QuadraticAutomaticImpact, QuadraticAutomaticMultiImpact, QuadraticDetectedFragmentContact,
};
mod clearance_step;
pub use clearance_step::{QuadraticClearanceAdvance, QuadraticClearanceStep};
pub use fracture_work::{
    QuadraticFractureAdvance, QuadraticFractureStep, QuadraticFractureSubstep,
};
#[path = "wet_update.rs"]
mod wet_update;
use crate::plasticity::mesh::{Vec3, dot, sub};
pub use wet_update::{QuadraticCohesiveWetUpdate, QuadraticWetAdvance, QuadraticWetUpdate};
#[derive(Clone, Debug)]
pub struct QuadraticDynamics {
    body: QuadraticBody,
    mass: Vec<Vec<f64>>,
    inertia: ConsistentInertia,
    velocities: Vec<Vec3>,
    free_nodes: Vec<usize>,
    finite_materials: Option<Vec<crate::biomechanics::Material>>,
    contact: Option<super::QuadraticPlaneContact>,
    surface_contact: Option<dynamic_contact::SurfaceContact>,
    friction: Option<super::QuadraticPlaneFriction>,
    coulomb: Option<super::QuadraticPlaneCoulomb>,
    densities: Vec<f64>,
    last_coulomb: Option<super::QuadraticCoulombImpulse>,
    friction_numerical_j: f64,
    friction_dissipated_j: f64,
    friction_impulse_n_s: Vec3,
    impact_dissipated_j: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct QuadraticEnergy {
    pub mass_kg: f64,
    pub momentum_kg_m_s: Vec3,
    pub angular_momentum_kg_m2_s: Vec3,
    pub kinetic_j: f64,
    /// Accepted restitution and impact-friction loss, not automatically deposited as heat.
    pub impact_dissipated_j: f64,
    pub elastic_j: f64,
    pub hardening_j: f64,
    pub dissipated_j: f64,
    pub cohesive_stored_j: f64,
    pub fracture_dissipated_j: f64,
    pub cohesive_friction_dissipated_j: f64,
    pub cohesive_friction_numerical_j: f64,
    pub cohesive_friction_released_j: f64,
    pub contact_j: f64,
    pub surface_contact_j: f64,
    pub surface_contact_search_error_bound_j: f64,
    pub contact_force_n: Vec3,
    pub friction_dissipated_j: f64,
    /// Coulomb projection loss diagnostic; included in actual impulse work,
    /// so do not add it again to the total physical energy ledger.
    pub friction_numerical_j: f64,
    pub friction_impulse_n_s: Vec3,
}
/// Adaptive explicit advancement with constant loads and stationary supports.
#[derive(Clone, Copy, Debug)]
pub struct QuadraticAdvanceLimits {
    pub minimum_dt_s: f64,
    /// Independent phase/contact sampling cap; energy control alone is insufficient.
    pub maximum_dt_s: f64,
    /// Counts both accepted and rejected actual integration attempts.
    pub max_attempts: usize,
    /// Budget for the sum of absolute substep energy defects over the interval.
    pub energy_tolerance_j: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct QuadraticSubstep {
    pub dt_s: f64,
    pub energy_defect_j: f64,
}
#[derive(Clone, Debug)]
pub struct QuadraticAdvance {
    pub substeps: Vec<QuadraticSubstep>,
    pub attempts: usize,
    pub absolute_energy_defect_j: f64,
}
impl QuadraticDynamics {
    /// Construct a free body; use `new_supported` for stationary whole-node supports.
    /// # Errors
    /// Invalid density, velocity, mass factor or initial diagnostics.
    pub fn new(
        body: QuadraticBody,
        densities: &[f64],
        velocities: Vec<Vec3>,
    ) -> Result<Self, &'static str> {
        let pinned = vec![false; body.rest.len()];
        Self::new_supported(body, densities, velocities, &pinned)
    }
    /// Fixed whole-node supports are held at their initial accepted positions.
    /// # Errors
    /// Invalid support count, nonzero pinned velocity, no free nodes or mass failure.
    pub fn new_supported(
        body: QuadraticBody,
        densities: &[f64],
        velocities: Vec<Vec3>,
        pinned: &[bool],
    ) -> Result<Self, &'static str> {
        Self::new_with_law(body, densities, velocities, pinned, None)
    }
    fn new_with_law(
        body: QuadraticBody,
        densities: &[f64],
        velocities: Vec<Vec3>,
        pinned: &[bool],
        finite_materials: Option<Vec<crate::biomechanics::Material>>,
    ) -> Result<Self, &'static str> {
        if velocities.len() != body.rest.len()
            || pinned.len() != body.rest.len()
            || velocities.iter().flatten().any(|v| !v.is_finite())
            || pinned
                .iter()
                .zip(&velocities)
                .any(|(&p, v)| p && v.iter().any(|x| x.abs() > 0.))
        {
            return Err("invalid quadratic dynamic velocity or support");
        }
        let free_nodes: Vec<_> = pinned
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| (!p).then_some(i))
            .collect();
        if free_nodes.is_empty() {
            return Err("quadratic dynamics requires a free node");
        }
        let mass = body.consistent_mass(densities)?;
        let reduced: Vec<Vec<f64>> = free_nodes
            .iter()
            .map(|&i| free_nodes.iter().map(|&j| mass[i][j]).collect())
            .collect();
        let inertia = ConsistentInertia::factor(&reduced)?;
        let result = Self {
            body,
            mass,
            inertia,
            velocities,
            free_nodes,
            finite_materials,
            contact: None,
            surface_contact: None,
            friction: None,
            coulomb: None,
            densities: densities.to_vec(),
            last_coulomb: None,
            friction_numerical_j: 0.,
            friction_dissipated_j: 0.,
            friction_impulse_n_s: [0.; 3],
            impact_dissipated_j: 0.,
        };
        result.energy()?;
        Ok(result)
    }
    #[must_use]
    pub fn body(&self) -> &QuadraticBody {
        &self.body
    }
    #[must_use]
    pub fn velocities(&self) -> &[Vec3] {
        &self.velocities
    }
    /// Connected deformable pieces with consistent-mass center/momenta/energy.
    /// # Errors
    /// Invalid mass partition or diagnostic overflow.
    pub fn fragments(&self) -> Result<Vec<super::QuadraticFragment>, &'static str> {
        self.body
            .fragment_nodes()
            .into_iter()
            .map(|nodes| {
                super::fragments::summarize(
                    nodes,
                    &self.mass,
                    &self.body.positions,
                    &self.velocities,
                )
            })
            .collect()
    }
    /// Consistent (not point-lumped) momentum and kinetic energy.
    /// # Errors
    /// Constitutive error or diagnostic overflow.
    pub fn energy(&self) -> Result<QuadraticEnergy, &'static str> {
        let mut result = QuadraticEnergy {
            mass_kg: self.mass.iter().flatten().sum(),
            momentum_kg_m_s: [0.; 3],
            angular_momentum_kg_m2_s: [0.; 3],
            kinetic_j: 0.,
            impact_dissipated_j: self.impact_dissipated_j,
            elastic_j: 0.,
            hardening_j: 0.,
            dissipated_j: 0.,
            cohesive_stored_j: 0.,
            fracture_dissipated_j: 0.,
            cohesive_friction_dissipated_j: 0.,
            cohesive_friction_numerical_j: 0.,
            cohesive_friction_released_j: 0.,
            contact_j: 0.,
            surface_contact_j: 0.,
            surface_contact_search_error_bound_j: 0.,
            contact_force_n: [0.; 3],
            friction_dissipated_j: self.friction_dissipated_j,
            friction_numerical_j: self.friction_numerical_j,
            friction_impulse_n_s: self.friction_impulse_n_s,
        };
        for (i, row) in self.mass.iter().enumerate() {
            for (j, &mass) in row.iter().enumerate() {
                result.kinetic_j += 0.5 * mass * dot(self.velocities[i], self.velocities[j]);
                let angular =
                    crate::plasticity::mesh::cross(self.body.positions[i], self.velocities[j]);
                for (axis, &component) in angular.iter().enumerate() {
                    result.angular_momentum_kg_m2_s[axis] += mass * component;
                    result.momentum_kg_m_s[axis] += mass * self.velocities[j][axis];
                }
            }
        }
        if let Some(materials) = &self.finite_materials {
            result.elastic_j = self
                .body
                .finite_elastic_at(&self.body.positions, materials)?
                .energy_j;
        } else {
            for (cell, responses) in self
                .body
                .cells
                .iter()
                .zip(self.body.responses_at(&self.body.positions)?)
            {
                for (point, response) in responses.iter().enumerate() {
                    let weight = cell.volume / 4.;
                    result.elastic_j += weight * response.elastic_energy_j_m3;
                    result.hardening_j += weight * response.hardening_energy_j_m3;
                    result.dissipated_j += weight * cell.states[point].dissipated_j_m3();
                }
            }
        }
        for trial in self.body.cohesive_trials_at(&self.body.positions)? {
            result.cohesive_stored_j += trial.stored_j;
            result.fracture_dissipated_j += trial.dissipated_j;
            result.cohesive_friction_dissipated_j += trial.friction_dissipated_j;
            result.cohesive_friction_numerical_j += trial.friction_numerical_j;
            result.cohesive_friction_released_j += trial.friction_released_j;
        }
        if let Some(plane) = &self.contact {
            let response = self.body.plane_contact_at(&self.body.positions, *plane)?;
            result.contact_j = response.energy_j;
            result.contact_force_n = response.force_n;
        }
        if let Some(response) = self.surface_contact_evaluation()? {
            result.surface_contact_j = response.energy_j;
            result.surface_contact_search_error_bound_j = response.search_energy_error_bound_j;
        }
        if [
            result.mass_kg,
            result.kinetic_j,
            result.elastic_j,
            result.hardening_j,
            result.dissipated_j,
            result.cohesive_stored_j,
            result.fracture_dissipated_j,
            result.cohesive_friction_dissipated_j,
            result.cohesive_friction_numerical_j,
            result.cohesive_friction_released_j,
            result.contact_j,
            result.surface_contact_j,
            result.surface_contact_search_error_bound_j,
            result.friction_dissipated_j,
            result.friction_numerical_j,
        ]
        .iter()
        .chain(&result.momentum_kg_m_s)
        .chain(&result.angular_momentum_kg_m2_s)
        .chain(&result.contact_force_n)
        .chain(&result.friction_impulse_n_s)
        .any(|v| !v.is_finite())
        {
            return Err("quadratic dynamic diagnostic overflow");
        }
        Ok(result)
    }
    fn evaluate_internal(&self) -> Result<super::Evaluation, &'static str> {
        let mut evaluation = if let Some(materials) = &self.finite_materials {
            let response = self
                .body
                .finite_elastic_at(&self.body.positions, materials)?;
            Ok(super::Evaluation {
                internal: response.internal_n,
                tangent: Vec::new(),
                states: self.body.states(),
                interfaces: Vec::new(),
            })
        } else {
            self.body.evaluate(
                &self.body.positions,
                &vec![[None; 3]; self.body.rest.len()],
                0,
                false,
            )
        }?;
        if self.finite_materials.is_some() {
            self.body.assemble_cohesive(
                &self.body.positions,
                &vec![[None; 3]; self.body.rest.len()],
                false,
                &mut evaluation,
            )?;
        }
        if let Some(plane) = &self.contact {
            let response = self.body.plane_contact_at(&self.body.positions, *plane)?;
            for (internal, gradient) in evaluation.internal.iter_mut().zip(response.gradient_n) {
                for axis in 0..3 {
                    internal[axis] += gradient[axis];
                }
            }
        }
        if let Some(response) = self.surface_contact_evaluation()? {
            for (internal, force) in evaluation.internal.iter_mut().zip(response.forces_n) {
                for axis in 0..3 {
                    internal[axis] -= force[axis];
                }
            }
        }
        if evaluation.internal.iter().flatten().any(|v| !v.is_finite()) {
            return Err("quadratic combined force overflow");
        }
        Ok(evaluation)
    }
    /// Install/remove a stationary plane, returning contact-potential parameter
    /// work at fixed geometry. Any failure leaves accepted state unchanged.
    /// # Errors
    /// Invalid boundary topology or contact evaluation overflow.
    pub fn set_plane_contact(
        &mut self,
        plane: Option<super::QuadraticPlaneContact>,
    ) -> Result<f64, &'static str> {
        let old = self.energy()?.contact_j;
        let mut candidate = self.clone();
        candidate.contact = plane;
        if candidate.contact.is_none() {
            candidate.friction = None;
            candidate.coulomb = None;
        }
        let change = candidate.energy()?.contact_j - old;
        if !change.is_finite() {
            return Err("quadratic contact parameter overflow");
        }
        *self = candidate;
        Ok(change)
    }
    /// Enable/disable kinetic friction on the installed stationary plane.
    /// Dissipation/impulse history is preserved when changing the law.
    /// # Errors
    /// A plane must be installed before enabling friction.
    pub fn set_plane_friction(
        &mut self,
        law: Option<super::QuadraticPlaneFriction>,
    ) -> Result<(), &'static str> {
        if law.is_some() && self.contact.is_none() {
            return Err("quadratic friction requires plane");
        }
        self.friction = law;
        self.coulomb = None;
        Ok(())
    }
    /// Select unregularized Coulomb impulses; replaces the regularized law.
    /// # Errors
    /// Enabling requires an installed stationary plane.
    pub fn set_plane_coulomb(
        &mut self,
        law: Option<super::QuadraticPlaneCoulomb>,
    ) -> Result<(), &'static str> {
        if law.is_some() && self.contact.is_none() {
            return Err("quadratic friction requires plane");
        }
        self.coulomb = law;
        self.friction = None;
        Ok(())
    }
    /// Last accepted Coulomb half-kick, including whole-node support impulses.
    #[must_use]
    pub fn last_coulomb_impulse(&self) -> Option<&super::QuadraticCoulombImpulse> {
        self.last_coulomb.as_ref()
    }
    fn coulomb_kick(&mut self, dt: f64, before_force: &[Vec3]) -> Result<(), &'static str> {
        if let (Some(plane), Some(law)) = (&self.contact, self.coulomb) {
            let mut pinned = vec![true; self.body.rest.len()];
            for &node in &self.free_nodes {
                pinned[node] = false;
            }
            let report = self.body.coulomb_plane_impulse_at(
                &self.body.positions,
                &self.velocities,
                &self.densities,
                &pinned,
                *plane,
                law,
                dt,
            )?;
            let work: f64 = report
                .nodal_impulse_n_s
                .iter()
                .zip(before_force.iter().zip(&report.velocities))
                .map(|(impulse, (before, after))| {
                    dot(
                        *impulse,
                        std::array::from_fn(|axis| 0.5 * before[axis] + 0.5 * after[axis]),
                    )
                })
                .sum();
            if !work.is_finite() {
                return Err("quadratic Coulomb diagnostic overflow");
            }
            self.velocities.clone_from(&report.velocities);
            self.friction_dissipated_j += (-work).max(0.);
            self.friction_numerical_j += report.numerical_dissipated_j;
            for impulse in &report.nodal_impulse_n_s {
                for (axis, &value) in impulse.iter().enumerate() {
                    self.friction_impulse_n_s[axis] += value;
                }
            }
            self.last_coulomb = Some(report);
        }
        Ok(())
    }
    fn friction_forces(&self) -> Result<Vec<Vec3>, &'static str> {
        if let (Some(plane), Some(law)) = (&self.contact, self.friction) {
            Ok(self
                .body
                .plane_friction_at(&self.body.positions, &self.velocities, *plane, law)?
                .forces_n)
        } else {
            Ok(vec![[0.; 3]; self.body.rest.len()])
        }
    }
    /// Explicit dissipative kick: exact work of its frozen nodal force on the
    /// average kick velocity. Reject a kick that injects energy; adaptive advance
    /// may reduce it. This is not an implicit sticking/friction projection.
    fn friction_kick(&mut self, dt: f64) -> Result<f64, &'static str> {
        if self.friction.is_none() {
            return Ok(0.);
        }
        let forces = self.friction_forces()?;
        let reduced: Vec<_> = self.free_nodes.iter().map(|&node| forces[node]).collect();
        let accelerations = self.inertia.accelerations(&reduced)?;
        let old = self.energy()?;
        let before = self.velocities.clone();
        for (&node, a) in self.free_nodes.iter().zip(accelerations) {
            for (axis, &value) in a.iter().enumerate() {
                self.velocities[node][axis] += dt * value;
            }
        }
        let work: f64 = forces
            .iter()
            .zip(before.iter().zip(&self.velocities))
            .map(|(force, (a, b))| {
                dt * dot(
                    *force,
                    std::array::from_fn(|axis| 0.5 * a[axis] + 0.5 * b[axis]),
                )
            })
            .sum();
        if !work.is_finite() || work > 0. {
            return Err("quadratic friction kick energy increase");
        }
        self.friction_dissipated_j -= work;
        for force in &forces {
            for (axis, &value) in force.iter().enumerate() {
                self.friction_impulse_n_s[axis] += dt * value;
            }
        }
        Ok(self.energy()?.kinetic_j - old.kinetic_j
            + (self.friction_dissipated_j - old.friction_dissipated_j))
    }
    /// Constraint forces on the body, in N. Free-node entries are equilibrium
    /// residuals near zero. Includes consistent-mass inertia coupling:
    /// `reaction = M*a + f_internal - f_external`, with zero pinned acceleration.
    /// # Errors
    /// Invalid loads, material response or reaction overflow.
    pub fn support_reactions(
        &self,
        loads: &[Vec3],
        acceleration: Vec3,
    ) -> Result<Vec<Vec3>, &'static str> {
        if self.coulomb.is_some() {
            return Err("Coulomb reactions require accepted impulse diagnostic");
        }
        if loads.len() != self.body.rest.len()
            || loads
                .iter()
                .flatten()
                .chain(&acceleration)
                .any(|v| !v.is_finite())
        {
            return Err("invalid quadratic reaction loads");
        }
        let evaluation = self.evaluate_internal()?;
        let friction = self.friction_forces()?;
        let external: Vec<Vec3> = self
            .mass
            .iter()
            .zip(loads)
            .zip(friction)
            .map(|((row, load), friction)| {
                let weight: f64 = row.iter().sum();
                std::array::from_fn(|axis| {
                    load[axis] + weight * acceleration[axis] + friction[axis]
                })
            })
            .collect();
        let free_forces: Vec<_> = self
            .free_nodes
            .iter()
            .map(|&node| sub(external[node], evaluation.internal[node]))
            .collect();
        let free_accelerations = self.inertia.accelerations(&free_forces)?;
        let mut nodal_accelerations = vec![[0.; 3]; self.body.rest.len()];
        for (&node, a) in self.free_nodes.iter().zip(free_accelerations) {
            nodal_accelerations[node] = a;
        }
        let reactions: Vec<Vec3> = self
            .mass
            .iter()
            .enumerate()
            .map(|(node, row)| {
                std::array::from_fn(|axis| {
                    row.iter()
                        .zip(&nodal_accelerations)
                        .map(|(m, a)| m * a[axis])
                        .sum::<f64>()
                        + evaluation.internal[node][axis]
                        - external[node][axis]
                })
            })
            .collect();
        if reactions.iter().flatten().any(|v| !v.is_finite()) {
            return Err("quadratic reaction overflow");
        }
        Ok(reactions)
    }
    /// Velocity Verlet under a uniform acceleration. Constant acceleration work
    /// uses consistent nodal weights; energy tolerance is per step in joules.
    /// Plastic endpoint histories commit with geometry/velocity only on acceptance.
    /// Explicit stability and plastic trajectories require timestep refinement.
    /// # Errors
    /// Invalid input, inversion, constitutive errors or energy-defect rejection.
    pub fn step(
        &mut self,
        dt: f64,
        acceleration: Vec3,
        energy_tolerance_j: f64,
    ) -> Result<f64, &'static str> {
        self.step_loaded(
            dt,
            &vec![[0.; 3]; self.body.rest.len()],
            acceleration,
            energy_tolerance_j,
        )
    }
    /// Advance a full interval transactionally under constant loads/acceleration.
    /// Bisects rejected energy/inverted-quadrature-point candidates. Other errors
    /// propagate directly. Budget allocation is proportional to substep duration;
    /// absolute defects cannot cancel. Maximum dt independently caps sampling.
    /// Dyadic subdivision can reach minimum dt before a requested cap is met.
    /// This is energy control, not CCD or an a priori explicit stability proof.
    /// # Errors
    /// Invalid limits/loads, initial failure, subdivision/attempt limit or overflow.
    pub fn advance_loaded(
        &mut self,
        interval_s: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        limits: QuadraticAdvanceLimits,
    ) -> Result<QuadraticAdvance, &'static str> {
        if !interval_s.is_finite()
            || interval_s <= 0.
            || !limits.minimum_dt_s.is_finite()
            || limits.minimum_dt_s <= 0.
            || limits.minimum_dt_s > interval_s
            || !limits.maximum_dt_s.is_finite()
            || limits.maximum_dt_s < limits.minimum_dt_s
            || limits.max_attempts == 0
            || !limits.energy_tolerance_j.is_finite()
            || limits.energy_tolerance_j <= 0.
            || loads.len() != self.body.rest.len()
            || loads
                .iter()
                .flatten()
                .chain(&acceleration)
                .any(|v| !v.is_finite())
        {
            return Err("invalid quadratic adaptive interval");
        }
        self.energy()?;
        self.evaluate_internal()?;
        let mut candidate = self.clone();
        let mut pending = vec![interval_s];
        let mut report = QuadraticAdvance {
            substeps: Vec::new(),
            attempts: 0,
            absolute_energy_defect_j: 0.,
        };
        while let Some(dt) = pending.pop() {
            let split = |pending: &mut Vec<f64>| -> Result<(), &'static str> {
                let half = dt * 0.5;
                if half < limits.minimum_dt_s || half >= dt {
                    return Err("quadratic adaptive minimum timestep reached");
                }
                pending.push(half);
                pending.push(half);
                Ok(())
            };
            if dt > limits.maximum_dt_s {
                split(&mut pending)?;
                continue;
            }
            if report.attempts == limits.max_attempts {
                return Err("quadratic adaptive attempt limit reached");
            }
            report.attempts += 1;
            let budget = limits.energy_tolerance_j * (dt / interval_s);
            if budget <= 0. {
                return Err("quadratic adaptive energy budget underflow");
            }
            match candidate.step_loaded(dt, loads, acceleration, budget) {
                Ok(defect) => {
                    report.absolute_energy_defect_j += defect.abs();
                    if report.absolute_energy_defect_j > limits.energy_tolerance_j {
                        return Err("quadratic adaptive interval energy budget exceeded");
                    }
                    report.substeps.push(QuadraticSubstep {
                        dt_s: dt,
                        energy_defect_j: defect,
                    });
                }
                Err(
                    "quadratic dynamic energy defect"
                    | "inverted quadratic integration point"
                    | "quadratic friction kick energy increase"
                    | "quadratic Coulomb iteration limit reached"
                    | "quadratic surface motion limit reached"
                    | "quadratic surface sweep clearance reached"
                    | "quadratic surface sweep unresolved",
                ) => {
                    split(&mut pending)?;
                }
                Err(error) => return Err(error),
            }
        }
        *self = candidate;
        Ok(report)
    }
    /// Constant-load step with dissipative half kicks around the conservative
    /// Verlet update. Friction work, impulse and all histories publish atomically.
    /// # Errors
    /// Invalid step/load, constitutive failure, positive friction work or energy guard.
    pub fn step_loaded(
        &mut self,
        dt: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        energy_tolerance_j: f64,
    ) -> Result<f64, &'static str> {
        if !dt.is_finite() || dt <= 0. {
            return Err("invalid quadratic dynamic step");
        }
        let mut candidate = self.clone();
        let first = candidate.friction_kick(0.5 * dt)?;
        let conservative =
            candidate.conservative_step_loaded(dt, loads, acceleration, energy_tolerance_j)?;
        let last = candidate.friction_kick(0.5 * dt)?;
        let defect = first + conservative + last;
        if !defect.is_finite() || defect.abs() > energy_tolerance_j {
            return Err("quadratic dynamic energy defect");
        }
        *self = candidate;
        Ok(defect)
    }
    /// Constant nodal forces plus uniform acceleration, sampled for this step.
    /// Fixed supports do no work. Full mass row weights generate gravity loads.
    /// # Errors
    /// Invalid loads, constitutive failure, inversion or energy-defect rejection.
    fn conservative_step_loaded(
        &mut self,
        dt: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        energy_tolerance_j: f64,
    ) -> Result<f64, &'static str> {
        if !dt.is_finite()
            || dt <= 0.
            || !energy_tolerance_j.is_finite()
            || energy_tolerance_j <= 0.
            || acceleration.iter().any(|v| !v.is_finite())
            || loads.len() != self.body.rest.len()
            || loads.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid quadratic dynamic step");
        }
        let old = self.energy()?;
        let evaluation = self.evaluate_internal()?;
        let forces: Vec<_> = self
            .free_nodes
            .iter()
            .map(|&node| {
                std::array::from_fn(|axis| {
                    loads[node][axis] - evaluation.internal[node][axis]
                        + self.mass[node].iter().sum::<f64>() * acceleration[axis]
                })
            })
            .collect();
        let a = self.inertia.accelerations(&forces)?;
        let mut candidate = self.clone();
        for (&node, &solved_acceleration) in self.free_nodes.iter().zip(&a) {
            for (axis, &component) in solved_acceleration.iter().enumerate() {
                candidate.velocities[node][axis] += 0.5 * dt * component;
            }
        }
        candidate.coulomb_kick(0.5 * dt, &self.velocities)?;
        for &node in &self.free_nodes {
            for axis in 0..3 {
                candidate.body.positions[node][axis] += dt * candidate.velocities[node][axis];
            }
        }
        self.check_surface_motion(&candidate)?;
        self.check_surface_sweep(&candidate)?;
        let evaluation = candidate.evaluate_internal()?;
        let forces: Vec<_> = self
            .free_nodes
            .iter()
            .map(|&node| {
                std::array::from_fn(|axis| {
                    loads[node][axis] - evaluation.internal[node][axis]
                        + self.mass[node].iter().sum::<f64>() * acceleration[axis]
                })
            })
            .collect();
        let a = self.inertia.accelerations(&forces)?;
        let before_second = candidate.velocities.clone();
        for (&node, &solved_acceleration) in self.free_nodes.iter().zip(&a) {
            for (axis, &component) in solved_acceleration.iter().enumerate() {
                candidate.velocities[node][axis] += 0.5 * dt * component;
            }
        }
        candidate.coulomb_kick(0.5 * dt, &before_second)?;
        for (cell, states) in candidate.body.cells.iter_mut().zip(evaluation.states) {
            cell.states = states;
        }
        candidate.body.interfaces = evaluation.interfaces;
        let new = candidate.energy()?;
        let work: f64 = self
            .mass
            .iter()
            .enumerate()
            .map(|(i, row)| {
                dot(
                    std::array::from_fn(|axis| {
                        row.iter().sum::<f64>() * acceleration[axis] + loads[i][axis]
                    }),
                    sub(candidate.body.positions[i], self.body.positions[i]),
                )
            })
            .sum();
        let defect = (new.kinetic_j - old.kinetic_j)
            + (new.elastic_j - old.elastic_j)
            + (new.hardening_j - old.hardening_j)
            + (new.dissipated_j - old.dissipated_j)
            + (new.cohesive_stored_j - old.cohesive_stored_j)
            + (new.fracture_dissipated_j - old.fracture_dissipated_j)
            + (new.cohesive_friction_dissipated_j - old.cohesive_friction_dissipated_j)
            + (new.cohesive_friction_released_j - old.cohesive_friction_released_j)
            + (new.contact_j - old.contact_j)
            + (new.surface_contact_j - old.surface_contact_j)
            + (new.friction_dissipated_j - old.friction_dissipated_j)
            - work;
        if !defect.is_finite() || defect.abs() > energy_tolerance_j {
            return Err("quadratic dynamic energy defect");
        }
        *self = candidate;
        Ok(defect)
    }
}

/// Objective finite-elastic quadratic dynamics, distinct from small-strain J2.
#[derive(Clone, Debug)]
pub struct FiniteQuadraticDynamics {
    inner: QuadraticDynamics,
}
impl FiniteQuadraticDynamics {
    /// Material per cell, reference density per cell, velocity per node and
    /// stationary whole-node supports. Active contraction is disabled.
    /// # Errors
    /// Invalid material/geometry/history, velocity/supports or mass factor.
    pub fn new(
        mesh: QuadraticBody,
        materials: Vec<crate::biomechanics::Material>,
        densities: &[f64],
        velocities: Vec<Vec3>,
        pinned: &[bool],
    ) -> Result<Self, &'static str> {
        mesh.finite_elastic_at(&mesh.positions, &materials)?;
        let inner =
            QuadraticDynamics::new_with_law(mesh, densities, velocities, pinned, Some(materials))?;
        Ok(Self { inner })
    }
    /// # Errors
    /// Invalid topology or contact evaluation overflow; state is unchanged.
    pub fn set_plane_contact(
        &mut self,
        plane: Option<super::QuadraticPlaneContact>,
    ) -> Result<f64, &'static str> {
        self.inner.set_plane_contact(plane)
    }
    /// # Errors
    /// Friction requires an installed stationary plane.
    pub fn set_plane_friction(
        &mut self,
        law: Option<super::QuadraticPlaneFriction>,
    ) -> Result<(), &'static str> {
        self.inner.set_plane_friction(law)
    }
    /// # Errors
    /// Enabling Coulomb impulses requires an installed plane.
    pub fn set_plane_coulomb(
        &mut self,
        law: Option<super::QuadraticPlaneCoulomb>,
    ) -> Result<(), &'static str> {
        self.inner.set_plane_coulomb(law)
    }
    #[must_use]
    pub fn last_coulomb_impulse(&self) -> Option<&super::QuadraticCoulombImpulse> {
        self.inner.last_coulomb_impulse()
    }
    /// Immutable current geometry/history for shared contact/render queries.
    #[must_use]
    pub fn body(&self) -> &QuadraticBody {
        &self.inner.body
    }
    #[must_use]
    pub fn positions(&self) -> &[Vec3] {
        &self.inner.body.positions
    }
    #[must_use]
    pub fn velocities(&self) -> &[Vec3] {
        &self.inner.velocities
    }
    /// Spatial stress at the four quadrature points of each cell under the
    /// actual finite-elastic law, rather than the mesh's J2 construction material.
    /// # Errors
    /// Constitutive/geometry errors.
    pub fn stresses(&self) -> Result<Vec<[crate::biomechanics::Stress; 4]>, &'static str> {
        let materials = self
            .inner
            .finite_materials
            .as_ref()
            .ok_or("missing finite-elastic law")?;
        Ok(self
            .inner
            .body
            .finite_elastic_at(&self.inner.body.positions, materials)?
            .stresses)
    }
    /// # Errors
    /// Consistent-mass fragment diagnostic failure.
    pub fn fragments(&self) -> Result<Vec<super::QuadraticFragment>, &'static str> {
        self.inner.fragments()
    }
    /// # Errors
    /// Constitutive/diagnostic overflow.
    pub fn energy(&self) -> Result<QuadraticEnergy, &'static str> {
        self.inner.energy()
    }
    /// Transactional adaptive interval using the actual finite-elastic force law.
    /// # Errors
    /// Invalid input, constitutive failure or adaptive energy/work/minimum limits.
    pub fn advance_loaded(
        &mut self,
        interval_s: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        limits: QuadraticAdvanceLimits,
    ) -> Result<QuadraticAdvance, &'static str> {
        self.inner
            .advance_loaded(interval_s, loads, acceleration, limits)
    }
    /// # Errors
    /// Invalid step/load, inversion or energy-defect rejection; state is atomic.
    pub fn step_loaded(
        &mut self,
        dt: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        energy_tolerance_j: f64,
    ) -> Result<f64, &'static str> {
        self.inner
            .step_loaded(dt, loads, acceleration, energy_tolerance_j)
    }
    /// # Errors
    /// Invalid step, inversion or energy-defect rejection; state is atomic.
    pub fn step(
        &mut self,
        dt: f64,
        acceleration: Vec3,
        energy_tolerance_j: f64,
    ) -> Result<f64, &'static str> {
        self.inner.step(dt, acceleration, energy_tolerance_j)
    }
    /// # Errors
    /// Invalid loads or constitutive/diagnostic overflow.
    pub fn support_reactions(
        &self,
        loads: &[Vec3],
        acceleration: Vec3,
    ) -> Result<Vec<Vec3>, &'static str> {
        self.inner.support_reactions(loads, acceleration)
    }
}
