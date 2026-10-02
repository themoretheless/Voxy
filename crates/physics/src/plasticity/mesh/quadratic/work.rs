//! Transactional bulk work accounting for small-strain T10 equilibrium.
use super::{Equilibrium, QuadraticBody, Vec3, strain_at};
use crate::plasticity::WorkStep;

#[derive(Clone, Debug, Default)]
pub struct QuadraticWork {
    pub quadrature: Vec<[WorkStep; 4]>,
    pub endpoint_work_j: f64,
    pub elastic_energy_change_j: f64,
    pub hardening_energy_change_j: f64,
    pub plastic_heat_j: f64,
    pub numerical_loss_j: f64,
    pub energy_defect_j: f64,
    /// Reference-volume integrated heat per cell, for thermal coupling.
    pub cell_plastic_heat_j: Vec<f64>,
}
#[derive(Clone, Debug)]
pub struct QuadraticWorkEquilibrium {
    pub equilibrium: Equilibrium,
    /// Absent when equilibrium did not converge; no histories were committed.
    pub work: Option<QuadraticWork>,
}
impl QuadraticBody {
    /// Bulk work candidate at supplied geometry from this body's accepted state.
    /// This report excludes cohesive-interface work. No state is changed.
    pub fn bulk_work_at(&self, positions: &[Vec3]) -> Result<QuadraticWork, &'static str> {
        if positions.len() != self.rest.len() || positions.iter().flatten().any(|x| !x.is_finite())
        {
            return Err("invalid bulk work geometry");
        }
        let mut report = QuadraticWork::default();
        for cell in &self.cells {
            let weight = cell.volume / 4.;
            let mut points = Vec::with_capacity(4);
            let mut heat = 0.;
            for (index, g) in cell.gradients.iter().enumerate() {
                let old_strain = strain_at(cell, g, &self.positions, &self.rest)?;
                let strain = strain_at(cell, g, positions, &self.rest)?;
                let step =
                    cell.material
                        .response_with_work(&cell.states[index], old_strain, strain)?;
                report.endpoint_work_j += weight * step.endpoint_work_j_m3;
                report.elastic_energy_change_j += weight * step.elastic_energy_change_j_m3;
                report.hardening_energy_change_j += weight * step.hardening_energy_change_j_m3;
                heat += weight * step.plastic_heat_j_m3;
                report.numerical_loss_j += weight * step.numerical_loss_j_m3;
                points.push(step);
            }
            report.plastic_heat_j += heat;
            report.cell_plastic_heat_j.push(heat);
            report.quadrature.push(
                points
                    .try_into()
                    .map_err(|_| "invalid bulk quadrature count")?,
            );
        }
        report.energy_defect_j = report.endpoint_work_j
            - report.elastic_energy_change_j
            - report.hardening_energy_change_j
            - report.plastic_heat_j
            - report.numerical_loss_j;
        let scale = report.endpoint_work_j.abs()
            + report.elastic_energy_change_j.abs()
            + report.hardening_energy_change_j.abs()
            + report.plastic_heat_j
            + report.numerical_loss_j;
        if ![scale, report.energy_defect_j]
            .iter()
            .all(|x| x.is_finite())
            || report.energy_defect_j.abs() > 1e-10 * scale.max(f64::MIN_POSITIVE)
        {
            return Err("quadratic bulk work balance failure");
        }
        Ok(report)
    }
    /// Solve an uncracked body and accept geometry, plastic history and work together.
    /// Load plus constrained reactions supplies the endpoint work; numerical loss
    /// remains separate from physical heat. Existing cohesive solves retain their API.
    pub fn equilibrate_with_work(
        &mut self,
        loads: &[Vec3],
        prescribed: &[[Option<f64>; 3]],
        max_iterations: usize,
        tolerance_n: f64,
    ) -> Result<QuadraticWorkEquilibrium, &'static str> {
        if !self.interfaces.is_empty() {
            return Err("coupled bulk work acceptance requires a body without cohesive interfaces");
        }
        let mut next = self.clone();
        let equilibrium = next.equilibrate(loads, prescribed, max_iterations, tolerance_n)?;
        if !equilibrium.converged {
            return Ok(QuadraticWorkEquilibrium {
                equilibrium,
                work: None,
            });
        }
        let work = self.bulk_work_at(&next.positions)?;
        let nodal_work: f64 = (0..loads.len())
            .flat_map(|node| (0..3).map(move |axis| (node, axis)))
            .map(|(node, axis)| {
                (loads[node][axis] + equilibrium.reactions_n[node][axis])
                    * (next.positions[node][axis] - self.positions[node][axis])
            })
            .sum();
        let scale = nodal_work.abs() + work.endpoint_work_j.abs();
        if !nodal_work.is_finite()
            || (nodal_work - work.endpoint_work_j).abs() > 1e-10 * scale.max(f64::MIN_POSITIVE)
        {
            return Err("quadratic nodal/quadrature work mismatch");
        }
        for (cell, points) in next.cells.iter().zip(&work.quadrature) {
            if cell
                .states
                .iter()
                .zip(points)
                .any(|(state, point)| *state != point.state)
            {
                return Err("quadratic work history mismatch");
            }
        }
        *self = next;
        Ok(QuadraticWorkEquilibrium {
            equilibrium,
            work: Some(work),
        })
    }
}
