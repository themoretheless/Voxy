//! Coupled frictionless perfectly inelastic impulse projection.
use super::{ConsistentInertia, Vec3, dot};
#[derive(Clone, Debug)]
pub struct QuadraticImpactConstraint {
    pub weights: Vec<f64>,
    pub normal: Vec3,
}
#[derive(Clone, Debug)]
pub struct QuadraticMultiImpact {
    pub velocities: Vec<Vec3>,
    pub nodal_impulse_n_s: Vec<Vec3>,
    pub impulses_n_s: Vec<f64>,
    pub relative_before_m_s: Vec<f64>,
    pub relative_after_m_s: Vec<f64>,
    pub dissipated_j: f64,
    /// Actual impulse work plus retained loss; positive roundoff is not heat.
    pub energy_defect_j: f64,
    pub energy_roundoff_bound_j: f64,
    pub sweeps: usize,
}
impl ConsistentInertia {
    /// Solve lambda>=0, g_after>=0, lambda*g_after=0 with the full coupled
    /// contact compliance J M^-1 J^T, including initially separating contacts.
    /// Perfectly inelastic normals only; no friction, pose correction or contact
    /// discovery. Absolute velocity tolerance is supplied in m/s. No mutation.
    pub fn inelastic_impacts(
        &self,
        velocities: &[Vec3],
        constraints: &[QuadraticImpactConstraint],
        velocity_tolerance_m_s: f64,
        max_sweeps: usize,
    ) -> Result<QuadraticMultiImpact, &'static str> {
        self.restitution_impacts(
            velocities,
            constraints,
            0.,
            velocity_tolerance_m_s,
            max_sweeps,
        )
    }
    /// Coupled Newton normal restitution targets for approaching constraints.
    /// Initially separating contacts retain zero target. Some contact networks
    /// make these targets energetically inadmissible; such a solve rejects.
    pub fn restitution_impacts(
        &self,
        velocities: &[Vec3],
        constraints: &[QuadraticImpactConstraint],
        restitution: f64,
        velocity_tolerance_m_s: f64,
        max_sweeps: usize,
    ) -> Result<QuadraticMultiImpact, &'static str> {
        if !restitution.is_finite() || !(0. ..=1.).contains(&restitution) {
            return Err("invalid coupled impact restitution");
        }
        let n = self.lower.len();
        let k = constraints.len();
        if velocities.len() != n
            || velocities.iter().flatten().any(|x| !x.is_finite())
            || k == 0
            || k > 64
            || !velocity_tolerance_m_s.is_finite()
            || velocity_tolerance_m_s <= 0.
            || max_sweeps == 0
            || max_sweeps > 65536
        {
            return Err("invalid coupled impact request");
        }
        let mut units = Vec::new();
        let mut responses = Vec::new();
        let mut before = Vec::new();
        for c in constraints {
            // Reuse unit-normal, dimension and positive contact compliance validation.
            self.normal_impact(velocities, &c.weights, c.normal, 0.)?;
            let unit: Vec<Vec3> = c.weights.iter().map(|w| c.normal.map(|x| w * x)).collect();
            let response = self.accelerations(&unit)?;
            before.push(
                unit.iter()
                    .zip(velocities)
                    .map(|(p, v)| dot(*p, *v))
                    .sum::<f64>(),
            );
            units.push(unit);
            responses.push(response);
        }
        let compliance: Vec<Vec<f64>> = units
            .iter()
            .map(|u| {
                responses
                    .iter()
                    .map(|r| u.iter().zip(r).map(|(a, b)| dot(*a, *b)).sum())
                    .collect()
            })
            .collect();
        if compliance.iter().flatten().any(|x| !x.is_finite()) {
            return Err("coupled impact compliance overflow");
        }
        let targets: Vec<f64> = before.iter().map(|g| -restitution * g.min(0.)).collect();
        let mut impulses = vec![0.; k];
        let mut used = 0;
        let mut converged = false;
        for sweep in 1..=max_sweeps {
            used = sweep;
            for i in 0..k {
                let g = before[i] - targets[i]
                    + compliance[i]
                        .iter()
                        .zip(&impulses)
                        .map(|(w, p)| w * p)
                        .sum::<f64>();
                impulses[i] = (impulses[i] - g / compliance[i][i]).max(0.);
                if !g.is_finite() || !impulses[i].is_finite() {
                    return Err("coupled impact iteration overflow");
                }
            }
            converged = (0..k).all(|i| {
                let g = before[i] - targets[i]
                    + compliance[i]
                        .iter()
                        .zip(&impulses)
                        .map(|(w, p)| w * p)
                        .sum::<f64>();
                g.is_finite()
                    && if impulses[i] > 0. {
                        g.abs() <= velocity_tolerance_m_s
                    } else {
                        g >= -velocity_tolerance_m_s
                    }
            });
            if converged {
                break;
            }
        }
        if !converged {
            return Err("coupled impact iteration limit");
        }
        let mut updated = velocities.to_vec();
        let mut nodal = vec![[0.; 3]; n];
        for i in 0..k {
            for node in 0..n {
                for a in 0..3 {
                    updated[node][a] += impulses[i] * responses[i][node][a];
                    nodal[node][a] += impulses[i] * units[i][node][a];
                }
            }
        }
        let after: Vec<f64> = units
            .iter()
            .map(|u| u.iter().zip(&updated).map(|(p, v)| dot(*p, *v)).sum())
            .collect();
        // Check the actual returned velocities, not only the compliance residual.
        if (0..k).any(|i| {
            !after[i].is_finite()
                || if impulses[i] > 0. {
                    (after[i] - targets[i]).abs() > velocity_tolerance_m_s
                } else {
                    after[i] - targets[i] < -velocity_tolerance_m_s
                }
        }) {
            return Err("coupled impact velocity residual");
        }
        let work: f64 = nodal
            .iter()
            .zip(velocities.iter().zip(&updated))
            .map(|(p, (a, b))| dot(*p, std::array::from_fn(|axis| a[axis].midpoint(b[axis]))))
            .sum();
        let work_scale: f64 = nodal
            .iter()
            .zip(velocities.iter().zip(&updated))
            .map(|(p, (a, b))| {
                (0..3)
                    .map(|k| 0.5 * p[k].abs() * (a[k].abs() + b[k].abs()))
                    .sum::<f64>()
            })
            .sum();
        let energy_roundoff_bound_j = 16. * (n + 1) as f64 * f64::EPSILON * work_scale;
        if !work.is_finite()
            || !energy_roundoff_bound_j.is_finite()
            || work > energy_roundoff_bound_j
            || updated
                .iter()
                .chain(&nodal)
                .flatten()
                .any(|x| !x.is_finite())
        {
            return Err("coupled impact energy increase or overflow");
        }
        Ok(QuadraticMultiImpact {
            velocities: updated,
            nodal_impulse_n_s: nodal,
            impulses_n_s: impulses,
            relative_before_m_s: before,
            relative_after_m_s: after,
            dissipated_j: (-work).max(0.),
            energy_defect_j: work + (-work).max(0.),
            energy_roundoff_bound_j,
            sweeps: used,
        })
    }
}
