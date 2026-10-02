//! Combined bulk/interface work audit with an explicit signed interface error budget.
use super::{Equilibrium, QuadraticBody, QuadraticWork, Vec3};
#[derive(Clone, Copy, Debug, Default)]
pub struct InterfaceWork {
    pub endpoint_work_j: f64,
    pub stored_energy_change_j: f64,
    pub fracture_work_j: f64,
    pub friction_heat_j: f64,
    pub friction_numerical_loss_j: f64,
    pub released_contact_energy_j: f64,
    /// Signed endpoint integration remainder. May be negative during softening;
    /// never deposit this as heat or hide it as positive damping.
    pub endpoint_error_j: f64,
}
#[derive(Clone, Debug)]
pub struct CoupledQuadraticWork {
    pub bulk: QuadraticWork,
    pub interfaces: Vec<InterfaceWork>,
    pub endpoint_work_j: f64,
    pub interface_error_j: f64,
}
#[derive(Clone, Debug)]
pub struct CoupledWorkEquilibrium {
    pub equilibrium: Equilibrium,
    pub work: Option<CoupledQuadraticWork>,
}
impl QuadraticBody {
    /// Nonmutating work audit from the accepted geometry/history to a candidate.
    pub fn coupled_work_at(
        &self,
        positions: &[Vec3],
    ) -> Result<CoupledQuadraticWork, &'static str> {
        let bulk = self.bulk_work_at(positions)?;
        let mut report = CoupledQuadraticWork {
            endpoint_work_j: bulk.endpoint_work_j,
            bulk,
            interfaces: Vec::new(),
            interface_error_j: 0.,
        };
        for face in &self.interfaces {
            let before = face.trial_at(&self.positions)?;
            let after = face.trial_at(positions)?;
            let mut work = InterfaceWork {
                endpoint_work_j: after
                    .internal_n
                    .iter()
                    .zip(positions.iter().zip(&self.positions))
                    .map(|(f, (p, old))| (0..3).map(|i| f[i] * (p[i] - old[i])).sum::<f64>())
                    .sum(),
                stored_energy_change_j: after.stored_j - before.stored_j,
                fracture_work_j: after.dissipated_j - before.dissipated_j,
                friction_heat_j: after.friction_dissipated_j - before.friction_dissipated_j,
                friction_numerical_loss_j: after.friction_numerical_j - before.friction_numerical_j,
                released_contact_energy_j: after.friction_released_j - before.friction_released_j,
                endpoint_error_j: 0.,
            };
            work.endpoint_error_j = work.endpoint_work_j
                - work.stored_energy_change_j
                - work.fracture_work_j
                - work.friction_heat_j
                - work.friction_numerical_loss_j
                - work.released_contact_energy_j;
            if ![
                work.endpoint_work_j,
                work.stored_energy_change_j,
                work.fracture_work_j,
                work.friction_heat_j,
                work.friction_numerical_loss_j,
                work.released_contact_energy_j,
                work.endpoint_error_j,
            ]
            .iter()
            .all(|v| v.is_finite())
                || [
                    work.fracture_work_j,
                    work.friction_heat_j,
                    work.friction_numerical_loss_j,
                    work.released_contact_energy_j,
                ]
                .iter()
                .any(|v| *v < 0.)
            {
                return Err("invalid cohesive work partition");
            }
            report.endpoint_work_j += work.endpoint_work_j;
            report.interface_error_j += work.endpoint_error_j;
            report.interfaces.push(work);
        }
        if !report.endpoint_work_j.is_finite() || !report.interface_error_j.is_finite() {
            return Err("coupled work overflow");
        }
        Ok(report)
    }
    /// Accept bulk and cohesive histories with an explicit per-step interface
    /// work-error budget in J. Uses sum of absolute per-interface errors, so
    /// opposing softening/loading errors cannot cancel to pass acceptance.
    /// Reduce the load increment if the budget is exceeded. This method does
    /// not supply the missing work or silently reinterpret it as thermal energy.
    pub fn equilibrate_with_interface_work(
        &mut self,
        loads: &[Vec3],
        prescribed: &[[Option<f64>; 3]],
        max_iterations: usize,
        tolerance_n: f64,
        max_interface_error_j: f64,
    ) -> Result<CoupledWorkEquilibrium, &'static str> {
        if !max_interface_error_j.is_finite() || max_interface_error_j < 0. {
            return Err("invalid cohesive work error budget");
        }
        let mut next = self.clone();
        let equilibrium = next.equilibrate(loads, prescribed, max_iterations, tolerance_n)?;
        if !equilibrium.converged {
            return Ok(CoupledWorkEquilibrium {
                equilibrium,
                work: None,
            });
        }
        let work = self.coupled_work_at(&next.positions)?;
        let absolute_error: f64 = work
            .interfaces
            .iter()
            .map(|w| w.endpoint_error_j.abs())
            .sum();
        if absolute_error > max_interface_error_j {
            return Err("cohesive work error budget exceeded");
        }
        let nodal: f64 = loads
            .iter()
            .zip(&equilibrium.reactions_n)
            .zip(next.positions.iter().zip(&self.positions))
            .map(|((load, reaction), (p, old))| {
                (0..3)
                    .map(|i| (load[i] + reaction[i]) * (p[i] - old[i]))
                    .sum::<f64>()
            })
            .sum();
        // Force assembly and strain interpolation use stored global positions.
        // Their coordinate-rounding floor matters after cohesive traction vanishes.
        let coordinate_roundoff: f64 = loads
            .iter()
            .zip(&equilibrium.reactions_n)
            .zip(next.positions.iter().zip(&self.positions))
            .map(|((load, reaction), (p, old))| {
                (0..3)
                    .map(|i| {
                        16. * f64::EPSILON
                            * (load[i].abs() + reaction[i].abs())
                            * (p[i].abs() + old[i].abs() + (p[i] - old[i]).abs())
                    })
                    .sum::<f64>()
            })
            .sum();
        if !nodal.is_finite()
            || !coordinate_roundoff.is_finite()
            || (nodal - work.endpoint_work_j).abs()
                > 1e-10 * (nodal.abs() + work.endpoint_work_j.abs()).max(f64::MIN_POSITIVE)
                    + coordinate_roundoff
        {
            return Err("coupled nodal/quadrature work mismatch");
        }
        for (cell, points) in next.cells.iter().zip(&work.bulk.quadrature) {
            if cell
                .states
                .iter()
                .zip(points)
                .any(|(state, point)| *state != point.state)
            {
                return Err("coupled bulk history mismatch");
            }
        }
        for (old, accepted) in self.interfaces.iter().zip(&next.interfaces) {
            if old.trial_at(&next.positions)?.candidate.states() != accepted.states() {
                return Err("coupled interface history mismatch");
            }
        }
        *self = next;
        Ok(CoupledWorkEquilibrium {
            equilibrium,
            work: Some(work),
        })
    }
}
