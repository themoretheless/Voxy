use super::{Material, Response, State, Vec3, dot, kinematics};

/// Fixed-normal constrained slider; normal reaction performs no work.
#[derive(Clone, Copy, Debug)]
pub struct SliderStep {
    pub state: State,
    pub response: Response,
    pub gap_m: Vec3,
    pub velocity_m_s: Vec3,
    pub impulse_n_s: Vec3,
    pub physical_heat_j: f64,
    pub contact_numerical_loss_j: f64,
    pub integration_numerical_loss_j: f64,
    pub energy_defect_j: f64,
}
impl Material {
    /// Re-expresses accepted contact history near its elastic gap. Returned offset
    /// must be added back to both pose and slip reference; constitutive energy is unchanged.
    /// # Errors
    /// Invalid pose/normal or open contact.
    pub fn rebase_history(
        &self,
        old: &State,
        gap: Vec3,
        normal: Vec3,
    ) -> Result<(State, Vec3, Vec3), &'static str> {
        let (normal_gap, _) = kinematics(gap, normal)?;
        if !old.closed || normal_gap >= 0. {
            return Err("rebase requires closed contact");
        }
        let mut local = *old;
        local.slip = [0.; 3];
        let pose = std::array::from_fn(|i| normal_gap * normal[i] + old.elastic_gap[i]);
        Ok((local, pose, old.slip))
    }
    /// Restores a tangential reference shift without changing elastic/dissipative history.
    /// # Errors
    /// Nonfinite resulting slip/reference or shift outside tangent plane.
    pub fn shift_history(
        &self,
        state: &State,
        offset: Vec3,
        normal: Vec3,
    ) -> Result<State, &'static str> {
        kinematics(offset, normal)?;
        let length = offset.iter().fold(0_f64, |a, v| a.hypot(*v));
        if dot(offset, normal).abs() > 1e-12 * length.max(1e-300) {
            return Err("contact reference shift must be tangential");
        }
        let mut result = *state;
        result.slip = std::array::from_fn(|i| state.slip[i] + offset[i]);
        if !result.slip.iter().all(|v| v.is_finite()) {
            return Err("contact reference overflow");
        }
        Ok(result)
    }
    /// Implicit momentum/contact solve for one finite tangential translating mass.
    /// Normal gap is fixed, surface stationary, contact already closed. The radial
    /// Coulomb/elastic projection has a closed solution; no trial history commits.
    /// Backward-Euler losses remain separate from physical friction heat.
    /// # Errors
    /// Invalid mass/area/time, non-tangential velocity, inconsistent contact pose,
    /// open contact, overflow, or actual stored momentum/energy balance failure.
    #[allow(clippy::too_many_arguments)]
    pub fn advance_slider(
        &self,
        old: &State,
        gap_m: Vec3,
        normal: Vec3,
        velocity_m_s: Vec3,
        mass_kg: f64,
        area_m2: f64,
        dt_s: f64,
    ) -> Result<SliderStep, &'static str> {
        let (normal_gap, tangent) = kinematics(gap_m, normal)?;
        if !old.closed
            || normal_gap >= 0.
            || ![mass_kg, area_m2, dt_s]
                .iter()
                .all(|v| v.is_finite() && *v > 0.)
            || !velocity_m_s.iter().all(|v| v.is_finite())
        {
            return Err("invalid constrained slider state");
        }
        let speed = velocity_m_s.iter().fold(0_f64, |a, v| a.hypot(*v));
        if dot(velocity_m_s, normal).abs() > 1e-12 * speed.max(1e-300) {
            return Err("slider velocity must be tangential");
        }
        let elastic: Vec3 = std::array::from_fn(|i| tangent[i] - old.slip[i]);
        for i in 0..3 {
            if (elastic[i] - old.elastic_gap[i]).abs()
                > 1e-10
                    * (tangent[i].abs() + old.slip[i].abs() + old.elastic_gap[i].abs()).max(1e-300)
            {
                return Err("slider pose disagrees with accepted contact history");
            }
        }
        let pressure = -self.normal_stiffness * normal_gap;
        let limit = self.coefficient * pressure;
        let factor = area_m2 * dt_s / mass_kg;
        let denominator = 1. + factor * dt_s * self.tangential_stiffness;
        let q: Vec3 = std::array::from_fn(|i| old.elastic_gap[i] + dt_s * velocity_m_s[i]);
        let length = q.iter().fold(0_f64, |a, v| a.hypot(*v));
        if ![pressure, limit, factor, denominator, length]
            .iter()
            .all(|v| v.is_finite())
            || factor <= 0.
            || denominator <= 0.
        {
            return Err("slider projection overflow");
        }
        let magnitude = ((self.tangential_stiffness / denominator) * length).min(limit);
        let traction = if length > 0. {
            q.map(|v| magnitude * (v / length))
        } else {
            [0.; 3]
        };
        let velocity: Vec3 = std::array::from_fn(|i| velocity_m_s[i] - factor * traction[i]);
        let gap: Vec3 = std::array::from_fn(|i| gap_m[i] + dt_s * velocity[i]);
        let (local, local_gap, offset) = self.rebase_history(old, gap_m, normal)?;
        let local_new = std::array::from_fn(|i| local_gap[i] + dt_s * velocity[i]);
        let (state, response) = self.response(&local, local_new, normal)?;
        let state = self.shift_history(&state, offset, normal)?;
        let dv: Vec3 = std::array::from_fn(|i| velocity[i] - velocity_m_s[i]);
        let impulse = response.tangential_traction_pa.map(|t| -area_m2 * dt_s * t);
        for i in 0..3 {
            let actual = mass_kg * dv[i];
            if (actual - impulse[i]).abs() > 1e-9 * (actual.abs() + impulse[i].abs()).max(1e-300) {
                return Err("slider momentum balance failure");
            }
        }
        let kinetic =
            0.5 * mass_kg * dot(dv, std::array::from_fn(|i| velocity[i] + velocity_m_s[i]));
        let elastic_change = std::array::from_fn(|i| state.elastic_gap[i] - old.elastic_gap[i]);
        let stored = 0.5
            * area_m2
            * self.tangential_stiffness
            * dot(
                elastic_change,
                std::array::from_fn(|i| state.elastic_gap[i] + old.elastic_gap[i]),
            );
        let heat = area_m2 * limit * response.slip_increment_m;
        let contact_loss =
            0.5 * area_m2 * self.tangential_stiffness * dot(elastic_change, elastic_change);
        let integration_loss = 0.5 * mass_kg * dot(dv, dv);
        let defect = kinetic + stored + heat + contact_loss + integration_loss;
        let scale = kinetic.abs() + stored.abs() + heat + contact_loss + integration_loss;
        if ![heat, contact_loss, integration_loss]
            .iter()
            .all(|x| x.is_finite() && *x >= 0.)
            || !defect.is_finite()
            || defect.abs() > 1e-9 * scale.max(1e-300)
        {
            return Err("slider energy balance failure");
        }
        Ok(SliderStep {
            state,
            response,
            gap_m: gap,
            velocity_m_s: velocity,
            impulse_n_s: impulse,
            physical_heat_j: heat,
            contact_numerical_loss_j: contact_loss,
            integration_numerical_loss_j: integration_loss,
            energy_defect_j: defect,
        })
    }
}
