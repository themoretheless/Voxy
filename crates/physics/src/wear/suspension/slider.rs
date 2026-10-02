use super::{WearSuspension, WearSuspensionInput, WearSuspensionStep, add_inventory};
use crate::friction;

#[derive(Clone, Copy, Debug)]
pub struct WearSliderInput<'a> {
    pub emission_positions_m: &'a [[f64; 3]],
    pub heat_weights: &'a [f64],
    pub dt_s: f64,
    pub coupling_steps: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct WearSliderStep {
    pub slider: friction::SliderStep,
    pub formation: WearSuspensionStep,
}
impl WearSuspension {
    /// Initializes closed contact against a stationary plane with fixed normal gap.
    /// Tangential reference is stress-free; normal constraint energy is constant.
    /// # Errors
    /// Missing finite parent, previously used history/cloud, invalid closed pose.
    pub fn initialize_slider(
        &mut self,
        gap: [f64; 3],
        normal: [f64; 3],
    ) -> Result<(), &'static str> {
        if self.parent_velocity_m_s.is_none()
            || !self.grains.is_empty()
            || self.contact_state != friction::State::default()
            || self.slider_pose.is_some()
        {
            return Err("slider initialization requires a fresh finite parent");
        }
        let reference = self
            .contact
            .inactive_reference(&self.contact_state, gap, normal)?;
        let (state, response) = self.contact.response(&reference, gap, normal)?;
        if response.mode == friction::Mode::Open || response.tangential_stored_j_m2 != 0. {
            return Err("slider requires closed stress-free tangential reference");
        }
        self.contact_state = state;
        self.slider_pose = Some((gap, normal));
        Ok(())
    }
    #[must_use]
    pub fn slider_gap_m(&self) -> Option<[f64; 3]> {
        self.slider_pose.map(|p| p.0)
    }

    /// Finite-parent implicit friction solve, then mass detachment and cloud/fluid
    /// advance. First-order splitting uses pre-removal mass for the contact solve.
    /// Physical friction funds surface/enthalpy internally, not external driver work.
    /// Stationary plane receives opposite contact impulse; normal reaction does no work.
    /// # Errors
    /// Unsupported slider/formation/fluid state, exhaustion crossing, budgets or
    /// whole-frame energy failure. Every owned state and ledger rolls back together.
    pub fn step_sliding(
        &mut self,
        input: WearSliderInput<'_>,
    ) -> Result<WearSliderStep, &'static str> {
        let (gap, normal) = self.slider_pose.ok_or("slider is not initialized")?;
        let velocity = self
            .parent_velocity_m_s
            .ok_or("slider has no finite parent")?;
        let area = self.layer.area_m2;
        let (local_state, local_gap, reference) =
            self.contact
                .rebase_history(&self.contact_state, gap, normal)?;
        let slider = self.contact.advance_slider(
            &local_state,
            local_gap,
            normal,
            velocity,
            self.layer.remaining_mass_kg(),
            area,
            input.dt_s,
        )?;
        let mut next = self.clone();
        next.contact_state = local_state;
        next.slider_pose = None;
        next.parent_velocity_m_s = Some(slider.velocity_m_s);
        next.energy.parent_kinetic_j = 0.5
            * self.layer.remaining_mass_kg()
            * slider.velocity_m_s.iter().map(|v| v * v).sum::<f64>();
        let mut formation = next.step_contact(WearSuspensionInput {
            gap_m: slider.gap_m,
            normal,
            emission_positions_m: input.emission_positions_m,
            inherited_velocity_m_s: slider.velocity_m_s,
            heat_weights: input.heat_weights,
            dt_s: input.dt_s,
            coupling_steps: input.coupling_steps,
        })?;
        next.contact_state = self
            .contact
            .shift_history(&next.contact_state, reference, normal)?;
        let global_gap = std::array::from_fn(|i| slider.gap_m[i] + reference[i]);
        next.slider_pose = Some((global_gap, normal));
        next.energy.friction_input_j = self.energy.friction_input_j;
        next.energy.accumulation_correction_j[2] = self.energy.accumulation_correction_j[2];
        next.energy_tail[2] = self.energy_tail[2].clone();
        next.energy.contact_spring_j = area * slider.response.tangential_stored_j_m2;
        add_inventory(
            &mut next.energy.contact_numerical_loss_j,
            &mut next.energy.accumulation_correction_j[4],
            &mut next.energy_tail[4],
            slider.contact_numerical_loss_j,
        )?;
        add_inventory(
            &mut next.energy.parent_integration_numerical_loss_j,
            &mut next.energy.accumulation_correction_j[5],
            &mut next.energy_tail[5],
            slider.integration_numerical_loss_j,
        )?;
        let dk = next.energy.parent_kinetic_j - self.energy.parent_kinetic_j
            + next.energy.kinetic_j
            - self.energy.kinetic_j;
        let dh = next.energy.enthalpy_j - self.energy.enthalpy_j
            + next.energy.suspension_heat_buffer_j
            - self.energy.suspension_heat_buffer_j;
        let spring = next.energy.contact_spring_j - self.energy.contact_spring_j;
        let losses = formation.surface_energy_j
            + formation.drag.numerical_loss_j
            + slider.contact_numerical_loss_j
            + slider.integration_numerical_loss_j;
        let defect = dk + dh + spring + losses;
        // Whole-owner differences include the stored cloud kinetic inventory;
        // its laboratory-frame rounding remains finite after the slider stops.
        let roundoff = 8.
            * f64::EPSILON
            * (self.energy.kinetic_j.abs()
                + next.energy.kinetic_j.abs()
                + self.energy.parent_kinetic_j.abs()
                + next.energy.parent_kinetic_j.abs()
                + self.energy.contact_spring_j.abs()
                + next.energy.contact_spring_j.abs()
                + self.energy.suspension_heat_buffer_j.abs()
                + next.energy.suspension_heat_buffer_j.abs());
        if !defect.is_finite()
            || defect.abs()
                > 1e-9 * (dk.abs() + dh.abs() + spring.abs() + losses).max(1e-300) + roundoff
        {
            return Err("dynamic wear slider energy balance failure");
        }
        formation.energy_defect_j = defect;
        let slider = friction::SliderStep {
            gap_m: global_gap,
            state: next.contact_state,
            ..slider
        };
        *self = next;
        Ok(WearSliderStep { slider, formation })
    }
}
