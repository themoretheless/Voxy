//! Single fixed-normal restitution impulse with consistent inertia.
use super::{ConsistentInertia, Vec3, dot};
#[derive(Clone, Debug)]
pub struct QuadraticNormalImpact {
    pub velocities: Vec<Vec3>,
    pub nodal_impulse_n_s: Vec<Vec3>,
    pub impulse_n_s: f64,
    pub effective_mass_kg: f64,
    pub relative_velocity_before_m_s: f64,
    pub relative_velocity_after_m_s: f64,
    pub dissipated_j: f64,
    /// Consistent kinetic impulse work plus total impact loss, ideally zero.
    pub energy_defect_j: f64,
}
impl ConsistentInertia {
    /// Resolve one fixed-normal impact constraint, without mutating input velocity.
    /// Dimensionless weights define g=sum_i w_i*n·v_i (positive separating).
    /// For point-to-face contact use +source shape weights and -target weights.
    /// Restitution e in [0,1] gives g_after=-e*g_before for approaching contact.
    /// The inertia may be restricted to free nodes; support impulses then require
    /// the caller's full constraint mapping. Multiple simultaneous contacts need
    /// a coupled solve rather than independent application of this kernel.
    /// # Errors
    /// Invalid dimensions, nonfinite data, nonunit normal, invalid restitution,
    /// singular effective contact mass or arithmetic overflow.
    pub fn normal_impact(
        &self,
        velocities: &[Vec3],
        weights: &[f64],
        normal: Vec3,
        restitution: f64,
    ) -> Result<QuadraticNormalImpact, &'static str> {
        if velocities.len() != self.lower.len()
            || weights.len() != velocities.len()
            || velocities
                .iter()
                .flatten()
                .chain(&normal)
                .any(|x| !x.is_finite())
            || weights.iter().any(|x| !x.is_finite())
            || (dot(normal, normal) - 1.).abs() > 1e-10
            || !restitution.is_finite()
            || !(0. ..=1.).contains(&restitution)
        {
            return Err("invalid quadratic normal impact");
        }
        let unit_impulse: Vec<Vec3> = weights.iter().map(|&w| normal.map(|x| w * x)).collect();
        let response = self.accelerations(&unit_impulse)?;
        let compliance: f64 = unit_impulse
            .iter()
            .zip(&response)
            .map(|(p, v)| dot(*p, *v))
            .sum();
        if !compliance.is_finite() || compliance <= 0. {
            return Err("invalid quadratic impact effective mass");
        }
        let before: f64 = unit_impulse
            .iter()
            .zip(velocities)
            .map(|(p, v)| dot(*p, *v))
            .sum();
        let impulse = if before < 0. {
            -(1. + restitution) * before / compliance
        } else {
            0.
        };
        let updated: Vec<Vec3> = velocities
            .iter()
            .zip(&response)
            .map(|(v, dv)| std::array::from_fn(|i| v[i] + impulse * dv[i]))
            .collect();
        let nodal: Vec<Vec3> = unit_impulse
            .iter()
            .map(|p| p.map(|x| impulse * x))
            .collect();
        let after: f64 = unit_impulse
            .iter()
            .zip(&updated)
            .map(|(p, v)| dot(*p, *v))
            .sum();
        let dissipated = if before < 0. {
            0.5 * (1. - restitution * restitution) * before * before / compliance
        } else {
            0.
        };
        let work: f64 = nodal
            .iter()
            .zip(velocities.iter().zip(&updated))
            .map(|(p, (a, b))| dot(*p, std::array::from_fn(|i| a[i].midpoint(b[i]))))
            .sum();
        let result = QuadraticNormalImpact {
            velocities: updated,
            nodal_impulse_n_s: nodal,
            impulse_n_s: impulse,
            effective_mass_kg: 1. / compliance,
            relative_velocity_before_m_s: before,
            relative_velocity_after_m_s: after,
            dissipated_j: dissipated,
            energy_defect_j: work + dissipated,
        };
        if [
            result.impulse_n_s,
            result.effective_mass_kg,
            before,
            after,
            dissipated,
            result.energy_defect_j,
        ]
        .iter()
        .chain(result.velocities.iter().flatten())
        .chain(result.nodal_impulse_n_s.iter().flatten())
        .any(|x| !x.is_finite())
        {
            return Err("quadratic normal impact overflow");
        }
        Ok(result)
    }
}

impl ConsistentInertia {
    /// Single isotropic-mass Coulomb impact. Tangential impulse opposes slip,
    /// capped by mu times the normal impulse; sticking never reverses slip.
    /// The returned dissipated_j includes restitution and tangential losses.
    pub fn frictional_impact(
        &self,
        velocities: &[Vec3],
        weights: &[f64],
        normal: Vec3,
        restitution: f64,
        friction: f64,
    ) -> Result<QuadraticNormalImpact, &'static str> {
        if !friction.is_finite() || friction < 0. {
            return Err("invalid impact friction");
        }
        let mut result = self.normal_impact(velocities, weights, normal, restitution)?;
        if result.impulse_n_s == 0. || friction == 0. {
            return Ok(result);
        }
        let relative: Vec3 =
            std::array::from_fn(|a| weights.iter().zip(velocities).map(|(w, v)| w * v[a]).sum());
        let tangent: Vec3 =
            std::array::from_fn(|a| relative[a] - result.relative_velocity_before_m_s * normal[a]);
        let speed = tangent[0].hypot(tangent[1]).hypot(tangent[2]);
        if !speed.is_finite() {
            return Err("impact slip overflow");
        }
        if speed == 0. {
            return Ok(result);
        }
        let sticking = result.effective_mass_kg * speed;
        let cap = friction * result.impulse_n_s;
        if !sticking.is_finite() || !cap.is_finite() {
            return Err("impact friction impulse overflow");
        }
        let magnitude = sticking.min(cap);
        let impulse: Vec3 = tangent.map(|x| -magnitude * (x / speed));
        let nodal: Vec<Vec3> = weights.iter().map(|w| impulse.map(|x| w * x)).collect();
        let response = self.accelerations(&nodal)?;
        for i in 0..velocities.len() {
            for a in 0..3 {
                result.velocities[i][a] += response[i][a];
                result.nodal_impulse_n_s[i][a] += nodal[i][a];
            }
        }
        let loss = magnitude * (speed - 0.5 * magnitude / result.effective_mass_kg);
        let combined_loss = result.dissipated_j + loss;
        if loss > 0. && combined_loss <= result.dissipated_j {
            return Err("unrepresentable impact friction loss");
        }
        result.dissipated_j = combined_loss;
        let work: f64 = result
            .nodal_impulse_n_s
            .iter()
            .zip(velocities.iter().zip(&result.velocities))
            .map(|(p, (a, b))| dot(*p, std::array::from_fn(|k| a[k].midpoint(b[k]))))
            .sum();
        result.energy_defect_j = work + result.dissipated_j;
        if !result.dissipated_j.is_finite()
            || !result.energy_defect_j.is_finite()
            || result
                .velocities
                .iter()
                .chain(&result.nodal_impulse_n_s)
                .flatten()
                .any(|x| !x.is_finite())
        {
            return Err("frictional impact overflow");
        }
        Ok(result)
    }
}
