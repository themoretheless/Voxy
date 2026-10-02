//! Atomic dynamic constitutive audit and accepted fragment partition.
use super::{QuadraticDynamics, Vec3};
use crate::plasticity::mesh::{CoupledQuadraticWork, QuadraticFragment};

#[derive(Clone, Debug)]
pub struct QuadraticFractureStep {
    /// Actual full mechanical-step defect, distinct from endpoint constitutive errors.
    pub energy_defect_j: f64,
    pub work: CoupledQuadraticWork,
    pub fragments: Vec<QuadraticFragment>,
    pub fragment_count_before: usize,
    pub newly_broken_interfaces: Vec<usize>,
    pub fragment_linear_roundoff_n_s: Vec3,
    pub fragment_angular_roundoff_kg_m2_s: Vec3,
}
impl QuadraticDynamics {
    /// Advance small-strain dynamics and publish constitutive work plus fragments
    /// from the same accepted geometry/history. Pieces remain deformable, with
    /// consistent mass; this does not replace their motion by a rigid-body fit.
    /// Explicit interface endpoint-error budget is separate from the actual
    /// integration energy guard. A failure in either audit rolls back all state.
    pub fn step_loaded_with_fracture_work(
        &mut self,
        dt_s: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        energy_tolerance_j: f64,
        max_interface_error_j: f64,
    ) -> Result<QuadraticFractureStep, &'static str> {
        if self.finite_materials.is_some() {
            return Err("J2 fracture work requires small-strain dynamics");
        }
        if !max_interface_error_j.is_finite() || max_interface_error_j < 0. {
            return Err("invalid dynamic interface work budget");
        }
        let fragment_count_before = self.fragments()?.len();
        let mut next = self.clone();
        let energy_defect = next.step_loaded(dt_s, loads, acceleration, energy_tolerance_j)?;
        let work = self.body.coupled_work_at(&next.body.positions)?;
        let error: f64 = work
            .interfaces
            .iter()
            .map(|w| w.endpoint_error_j.abs())
            .sum();
        if !error.is_finite() || error > max_interface_error_j {
            return Err("dynamic cohesive work budget exceeded");
        }
        for (cell, points) in next.body.cells.iter().zip(&work.bulk.quadrature) {
            if cell
                .states
                .iter()
                .zip(points)
                .any(|(state, p)| *state != p.state)
            {
                return Err("dynamic bulk work history mismatch");
            }
        }
        let mut newly_broken = Vec::new();
        for (i, (old, accepted)) in self
            .body
            .interfaces
            .iter()
            .zip(&next.body.interfaces)
            .enumerate()
        {
            if old.trial_at(&next.body.positions)?.candidate.states() != accepted.states() {
                return Err("dynamic interface work history mismatch");
            }
            if !old.is_fully_broken() && accepted.is_fully_broken() {
                newly_broken.push(i);
            }
        }
        let fragments = next.fragments()?;
        let energy = next.energy()?;
        let mass: f64 = fragments.iter().map(|f| f.mass_kg).sum();
        let kinetic: f64 = fragments.iter().map(|f| f.kinetic_j).sum();
        let close =
            |a: f64, b: f64, scale: f64| (a - b).abs() <= 1e-10 * scale.max(f64::MIN_POSITIVE);
        if !close(mass, energy.mass_kg, mass.abs() + energy.mass_kg.abs())
            || !close(
                kinetic,
                energy.kinetic_j,
                kinetic.abs() + energy.kinetic_j.abs(),
            )
        {
            return Err("dynamic fragment mass/energy partition failure");
        }
        let mut linear_roundoff = [0.; 3];
        let mut angular_roundoff = [0.; 3];
        for axis in 0..3 {
            let b = (axis + 1) % 3;
            let c = (axis + 2) % 3;
            let factor = 8. * (next.mass.len().pow(2) + next.mass.len()) as f64 * f64::EPSILON;
            for (i, row) in next.mass.iter().enumerate() {
                for (j, m) in row.iter().enumerate() {
                    linear_roundoff[axis] += factor * m.abs() * next.velocities[j][axis].abs();
                    angular_roundoff[axis] += factor
                        * m.abs()
                        * ((next.body.positions[i][b] * next.velocities[j][c]).abs()
                            + (next.body.positions[i][c] * next.velocities[j][b]).abs());
                }
            }
            if !linear_roundoff[axis].is_finite() || !angular_roundoff[axis].is_finite() {
                return Err("fragment summation bound overflow");
            }
            let p: f64 = fragments.iter().map(|f| f.momentum_kg_m_s[axis]).sum();
            let l: f64 = fragments
                .iter()
                .map(|f| f.angular_momentum_kg_m2_s[axis])
                .sum();
            let pscale: f64 = fragments
                .iter()
                .map(|f| f.momentum_kg_m_s[axis].abs())
                .sum();
            let lscale: f64 = fragments
                .iter()
                .map(|f| f.angular_momentum_kg_m2_s[axis].abs())
                .sum();
            if (p - energy.momentum_kg_m_s[axis]).abs()
                > 1e-10 * (pscale + energy.momentum_kg_m_s[axis].abs()).max(f64::MIN_POSITIVE)
                    + linear_roundoff[axis]
                || (l - energy.angular_momentum_kg_m2_s[axis]).abs()
                    > 1e-10
                        * (lscale + energy.angular_momentum_kg_m2_s[axis].abs())
                            .max(f64::MIN_POSITIVE)
                        + angular_roundoff[axis]
            {
                return Err("dynamic fragment momentum partition failure");
            }
        }
        let report = QuadraticFractureStep {
            energy_defect_j: energy_defect,
            work,
            fragments,
            fragment_count_before,
            newly_broken_interfaces: newly_broken,
            fragment_linear_roundoff_n_s: linear_roundoff,
            fragment_angular_roundoff_kg_m2_s: angular_roundoff,
        };
        *self = next;
        Ok(report)
    }
}

#[derive(Clone, Debug)]
pub struct QuadraticFractureSubstep {
    pub dt_s: f64,
    pub energy_defect_j: f64,
    pub interface_absolute_error_j: f64,
    pub fracture_work_j: f64,
    pub plastic_heat_j: f64,
    pub fragment_count_before: usize,
    pub fragment_count_after: usize,
    pub newly_broken_interfaces: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct QuadraticFractureAdvance {
    pub substeps: Vec<QuadraticFractureSubstep>,
    pub attempts: usize,
    pub absolute_energy_defect_j: f64,
    pub interface_absolute_error_j: f64,
    pub cell_plastic_heat_j: Vec<f64>,
    pub fragments: Vec<QuadraticFragment>,
}
impl QuadraticDynamics {
    /// Atomic adaptive interval with independent mechanical-energy and interface
    /// endpoint-error budgets. Rejected trials supply no heat or crack events.
    /// Retains bounded substep summaries, not every material-point trial report.
    pub fn advance_loaded_with_fracture_work(
        &mut self,
        interval_s: f64,
        loads: &[Vec3],
        acceleration: Vec3,
        limits: super::QuadraticAdvanceLimits,
        interface_error_budget_j: f64,
    ) -> Result<QuadraticFractureAdvance, &'static str> {
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
            || !interface_error_budget_j.is_finite()
            || interface_error_budget_j <= 0.
        {
            return Err("invalid adaptive fracture work limits");
        }
        let mut candidate = self.clone();
        let mut pending = vec![interval_s];
        let mut report = QuadraticFractureAdvance {
            substeps: Vec::new(),
            attempts: 0,
            absolute_energy_defect_j: 0.,
            interface_absolute_error_j: 0.,
            cell_plastic_heat_j: vec![0.; self.body.cells.len()],
            fragments: Vec::new(),
        };
        while let Some(dt) = pending.pop() {
            let split = |pending: &mut Vec<f64>| -> Result<(), &'static str> {
                let half = dt * 0.5;
                if half < limits.minimum_dt_s || half >= dt {
                    return Err("adaptive fracture minimum timestep reached");
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
                return Err("adaptive fracture attempt limit reached");
            }
            report.attempts += 1;
            let energy_budget = limits.energy_tolerance_j * (dt / interval_s);
            let interface_budget = interface_error_budget_j * (dt / interval_s);
            if energy_budget <= 0. || interface_budget <= 0. {
                return Err("adaptive fracture work budget underflow");
            }
            match candidate.step_loaded_with_fracture_work(
                dt,
                loads,
                acceleration,
                energy_budget,
                interface_budget,
            ) {
                Ok(step) => {
                    let absolute_error: f64 = step
                        .work
                        .interfaces
                        .iter()
                        .map(|w| w.endpoint_error_j.abs())
                        .sum();
                    report.absolute_energy_defect_j += step.energy_defect_j.abs();
                    report.interface_absolute_error_j += absolute_error;
                    if report.absolute_energy_defect_j > limits.energy_tolerance_j
                        || report.interface_absolute_error_j > interface_error_budget_j
                    {
                        return Err("adaptive fracture interval work budget exceeded");
                    }
                    for (heat, increment) in report
                        .cell_plastic_heat_j
                        .iter_mut()
                        .zip(&step.work.bulk.cell_plastic_heat_j)
                    {
                        let before = *heat;
                        *heat += increment;
                        if *increment > 0. && *heat == before {
                            return Err("unrepresentable adaptive fracture heat accumulation");
                        }
                        if !heat.is_finite() {
                            return Err("adaptive fracture heat overflow");
                        }
                    }
                    report.substeps.push(QuadraticFractureSubstep {
                        dt_s: dt,
                        energy_defect_j: step.energy_defect_j,
                        interface_absolute_error_j: absolute_error,
                        fracture_work_j: step
                            .work
                            .interfaces
                            .iter()
                            .map(|w| w.fracture_work_j)
                            .sum(),
                        plastic_heat_j: step.work.bulk.plastic_heat_j,
                        fragment_count_before: step.fragment_count_before,
                        fragment_count_after: step.fragments.len(),
                        newly_broken_interfaces: step.newly_broken_interfaces,
                    });
                }
                Err(
                    "quadratic dynamic energy defect"
                    | "dynamic cohesive work budget exceeded"
                    | "inverted quadratic integration point"
                    | "quadratic friction kick energy increase"
                    | "quadratic Coulomb iteration limit reached"
                    | "quadratic surface motion limit reached"
                    | "quadratic surface sweep clearance reached"
                    | "quadratic surface sweep unresolved",
                ) => split(&mut pending)?,
                Err(error) => return Err(error),
            }
        }
        report.fragments = candidate.fragments()?;
        *self = candidate;
        Ok(report)
    }
}
