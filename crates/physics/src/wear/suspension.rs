use super::{ContactDustSettings, Layer, LiquidHeatSink, Material, Removal, SurfaceEnergy};
use crate::{friction, liquid, suspension};
mod slider;
pub use slider::{WearSliderInput, WearSliderStep};

/// Owned local contact/wear/fluid/cloud state. Contact pose is prescribed;
/// deformable-body dynamics and contact discovery are outside this owner.
#[derive(Clone, Debug)]
pub struct WearSuspension {
    layer: Layer,
    contact: friction::Material,
    contact_state: friction::State,
    wear: Material,
    surface_energy_j_m2: f64,
    liquid: liquid::Liquid,
    grains: Vec<suspension::Particle>,
    energy: WearSuspensionEnergy,
    energy_tail: [Vec<f64>; 6],
    parent_velocity_m_s: Option<[f64; 3]>,
    slider_pose: Option<([f64; 3], [f64; 3])>,
}
/// Physical stores, external inputs and numerical damping remain distinct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WearSuspensionEnergy {
    pub surface_energy_j: f64,
    pub drag_numerical_loss_j: f64,
    pub friction_input_j: f64,
    pub emission_kinetic_input_j: f64,
    /// Zero for an external source; finite remaining parent energy otherwise.
    pub parent_kinetic_j: f64,
    /// Signed accumulated work of separately applied external parent impulses.
    pub parent_impulse_work_j: f64,
    pub contact_spring_j: f64,
    pub contact_numerical_loss_j: f64,
    pub parent_integration_numerical_loss_j: f64,
    pub kinetic_j: f64,
    pub enthalpy_j: f64,
    pub suspension_heat_buffer_j: f64,
    /// Low components: surface, drag loss, friction input, emission input,
    /// contact loss, parent integration loss. Total store = high + correction + owner tail.
    pub accumulation_correction_j: [f64; 6],
}
#[derive(Clone, Copy, Debug)]
pub struct WearSuspensionInput<'a> {
    pub gap_m: [f64; 3],
    pub normal: [f64; 3],
    pub emission_positions_m: &'a [[f64; 3]],
    pub inherited_velocity_m_s: [f64; 3],
    pub heat_weights: &'a [f64],
    pub dt_s: f64,
    pub coupling_steps: usize,
}
/// Emitted particles belong solely to the system, not to this diagnostic report.
#[derive(Clone, Copy, Debug)]
pub struct WearSuspensionStep {
    pub contact: friction::Response,
    pub wear: Removal,
    pub emitted_grains: usize,
    pub emitted_kinetic_j: f64,
    pub surface_energy_j: f64,
    pub friction_work_j: f64,
    pub friction_heat_j: f64,
    pub fluid: liquid::StepStats,
    pub drag: suspension::CloudExchange,
    pub energy_defect_j: f64,
    pub emitted_mass_defect_kg: f64,
    pub momentum_defect_n_s: [f64; 3],
}
fn kinetic(liquid: &liquid::Liquid, grains: &[suspension::Particle]) -> f64 {
    liquid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum::<f64>()
        + grains
            .iter()
            .map(|p| 0.5 * p.mass_kg() * p.velocity_m_s().iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
}
// Grow an ordered floating-point expansion with error-free TwoSum.
// Retain every nonzero residual instead of rejecting increments below two-part precision.
fn add_inventory(
    store: &mut f64,
    correction: &mut f64,
    tail: &mut Vec<f64>,
    increment: f64,
) -> Result<(), &'static str> {
    if !increment.is_finite() || increment < 0. {
        return Err("invalid energy inventory increment");
    }
    let mut expansion = Vec::with_capacity(tail.len() + 3);
    let mut q = increment;
    for component in tail.iter().copied().chain([*correction, *store]) {
        let sum = q + component;
        let recovered = sum - q;
        let error = (q - (sum - recovered)) + (component - recovered);
        if !sum.is_finite() || !error.is_finite() {
            return Err("energy inventory overflow");
        }
        if error != 0. {
            expansion.push(error);
        }
        q = sum;
    }
    if q != 0. {
        expansion.push(q);
    }
    if expansion.len() > 2048 {
        return Err("energy inventory expansion budget exceeded");
    }
    let high = expansion.pop().unwrap_or(0.);
    let low = expansion.pop().unwrap_or(0.);
    *store = high;
    *correction = low;
    *tail = expansion;
    Ok(())
}

impl WearSuspension {
    /// Residual expansion below the two reported components. Same six-store order.
    /// Total inventory is high + correction + all tail components.
    pub fn energy_accumulation_tail(&self) -> &[Vec<f64>; 6] {
        &self.energy_tail
    }

    /// Starts with empty cloud and uninitialized contact history.
    /// # Errors
    /// Invalid fresh-surface energy, missing transport or nonfinite kinetic energy.
    pub fn new(
        layer: Layer,
        contact: friction::Material,
        wear: Material,
        liquid: liquid::Liquid,
        surface_energy_j_m2: f64,
    ) -> Result<Self, &'static str> {
        SurfaceEnergy::new(0., surface_energy_j_m2)?;
        let enthalpy_j = liquid
            .transport_totals()
            .map_err(|_| "invalid liquid energy")?
            .ok_or("wear suspension requires thermal transport")?
            .0;
        let kinetic_j = kinetic(&liquid, &[]);
        if !kinetic_j.is_finite() {
            return Err("invalid initial suspension kinetic energy");
        }
        let suspension_heat_buffer_j = liquid
            .suspension_heat_buffer()
            .iter()
            .chain(liquid.suspension_heat_correction())
            .sum();
        Ok(Self {
            layer,
            contact,
            contact_state: friction::State::default(),
            wear,
            surface_energy_j_m2,
            liquid,
            grains: Vec::new(),
            energy_tail: std::array::from_fn(|_| Vec::new()),
            energy: WearSuspensionEnergy {
                surface_energy_j: 0.,
                drag_numerical_loss_j: 0.,
                friction_input_j: 0.,
                emission_kinetic_input_j: 0.,
                parent_kinetic_j: 0.,
                parent_impulse_work_j: 0.,
                contact_spring_j: 0.,
                contact_numerical_loss_j: 0.,
                parent_integration_numerical_loss_j: 0.,
                kinetic_j,
                enthalpy_j,
                suspension_heat_buffer_j,
                accumulation_correction_j: [0.; 6],
            },
            parent_velocity_m_s: None,
            slider_pose: None,
        })
    }
    /// Finite parent with prescribed translation and no spin/ejection velocity.
    /// Detached grains inherit this velocity; mass, momentum and kinetic energy
    /// transfer from parent to grains. Contact-force acceleration is not solved.
    /// # Errors
    /// Invalid velocity/kinetic inventory or ordinary constructor failure.
    pub fn new_translating(
        layer: Layer,
        contact: friction::Material,
        wear: Material,
        liquid: liquid::Liquid,
        surface_energy_j_m2: f64,
        velocity: [f64; 3],
    ) -> Result<Self, &'static str> {
        let mut system = Self::new(layer, contact, wear, liquid, surface_energy_j_m2)?;
        let energy =
            0.5 * system.layer.remaining_mass_kg() * velocity.iter().map(|v| v * v).sum::<f64>();
        if !velocity.iter().all(|v| v.is_finite()) || !energy.is_finite() {
            return Err("invalid translating wear parent");
        }
        system.parent_velocity_m_s = Some(velocity);
        system.energy.parent_kinetic_j = energy;
        Ok(system)
    }
    #[must_use]
    pub fn parent_momentum_kg_m_s(&self) -> Option<[f64; 3]> {
        self.parent_velocity_m_s
            .map(|v| v.map(|x| self.layer.remaining_mass_kg() * x))
    }
    #[must_use]
    pub fn parent_velocity_m_s(&self) -> Option<[f64; 3]> {
        self.parent_velocity_m_s
    }

    /// Applies an externally supplied impulse to the remaining finite parent.
    /// Returns signed accepted kinetic work and retains it in the energy ledger.
    /// Existing detached grains are unaffected. This does not automatically
    /// integrate contact traction: the caller owns that force/impulse solve.
    /// # Errors
    /// No finite parent, exhausted mass, nonfinite or unrepresentable impulse/work.
    /// Parent velocity and all energy stores remain unchanged on failure.
    pub fn apply_parent_impulse(&mut self, impulse_n_s: [f64; 3]) -> Result<f64, &'static str> {
        let old = self
            .parent_velocity_m_s
            .ok_or("external-emission fixture has no finite parent")?;
        let mass = self.layer.remaining_mass_kg();
        if mass <= 0. || !impulse_n_s.iter().all(|x| x.is_finite()) {
            return Err("invalid finite parent impulse");
        }
        let velocity: [f64; 3] = std::array::from_fn(|i| old[i] + impulse_n_s[i] / mass);
        for axis in 0..3 {
            let actual = mass * (velocity[axis] - old[axis]);
            let average_twice = velocity[axis] + old[axis];
            let component_work = 0.5 * actual * average_twice;
            if !velocity[axis].is_finite()
                || (actual != 0. && average_twice != 0. && component_work == 0.)
                || (actual - impulse_n_s[axis]).abs()
                    > 1e-10 * impulse_n_s[axis].abs().max(f64::MIN_POSITIVE)
            {
                return Err("unrepresentable finite parent impulse");
            }
        }
        let energy = 0.5 * mass * velocity.iter().map(|v| v * v).sum::<f64>();
        let expected: f64 = (0..3)
            .map(|i| 0.5 * mass * (velocity[i] - old[i]) * (velocity[i] + old[i]))
            .sum();
        let work = energy - self.energy.parent_kinetic_j;
        let accumulated = self.energy.parent_impulse_work_j + work;
        if !energy.is_finite()
            || !expected.is_finite()
            || !work.is_finite()
            || !accumulated.is_finite()
            || (expected != 0. && work == 0.)
            || (work - expected).abs()
                > 16. * f64::EPSILON * (energy + self.energy.parent_kinetic_j)
            || (accumulated - self.energy.parent_impulse_work_j - work).abs()
                > 1e-10 * work.abs().max(f64::MIN_POSITIVE)
        {
            return Err("unrepresentable finite parent impulse work");
        }
        self.parent_velocity_m_s = Some(velocity);
        self.energy.parent_kinetic_j = energy;
        self.energy.parent_impulse_work_j = accumulated;
        Ok(work)
    }
    #[must_use]
    pub fn layer(&self) -> &Layer {
        &self.layer
    }
    #[must_use]
    pub fn liquid(&self) -> &liquid::Liquid {
        &self.liquid
    }
    #[must_use]
    pub fn grains(&self) -> &[suspension::Particle] {
        &self.grains
    }
    #[must_use]
    pub fn contact_state(&self) -> &friction::State {
        &self.contact_state
    }
    #[must_use]
    pub fn energy(&self) -> WearSuspensionEnergy {
        self.energy
    }

    /// External parent kick followed by contact/wear/heat/fluid advance, committed
    /// as one frame. Emission velocity is derived from the accepted kicked parent;
    /// the input's kinematic emission velocity is ignored in this finite mode.
    /// The kick is externally supplied, not computed from contact traction.
    /// # Errors
    /// No finite parent or any impulse/contact/thermal/motion validation failure.
    /// Even a late error restores parent momentum and impulse-work accounting.
    pub fn step_contact_with_parent_impulse(
        &mut self,
        impulse_n_s: [f64; 3],
        mut input: WearSuspensionInput<'_>,
    ) -> Result<(f64, WearSuspensionStep), &'static str> {
        let mut candidate = self.clone();
        let work = candidate.apply_parent_impulse(impulse_n_s)?;
        input.inherited_velocity_m_s = candidate
            .parent_velocity_m_s
            .ok_or("missing kicked parent velocity")?;
        let step = candidate.step_contact(input)?;
        *self = candidate;
        Ok((work, step))
    }

    /// Complete contact formation, heat deposit and cloud/fluid motion transaction.
    /// Prescribed contact updates happen before first-order fluid/cloud splitting.
    /// # Errors
    /// Invalid contact, unsupported fluid regime, cloud budget or any late motion/
    /// thermal failure. Every owned state rolls back, including emitted particles.
    pub fn step_contact(
        &mut self,
        input: WearSuspensionInput<'_>,
    ) -> Result<WearSuspensionStep, &'static str> {
        if self.slider_pose.is_some() {
            return Err("dynamic slider requires step_sliding");
        }
        if self
            .parent_velocity_m_s
            .is_some_and(|v| v != input.inherited_velocity_m_s)
        {
            return Err("dust must inherit translating parent's velocity");
        }
        if self
            .grains
            .len()
            .checked_add(input.emission_positions_m.len())
            .is_none_or(|n| n > 4096)
        {
            return Err("wear suspension cloud budget exceeded");
        }
        let mut next = self.clone();
        let formation = next.layer.advance_contact_dust_heated(
            next.contact,
            &mut next.contact_state,
            input.gap_m,
            input.normal,
            ContactDustSettings {
                wear_material: next.wear,
                surface_energy_j_m2: next.surface_energy_j_m2,
                emission_positions_m: input.emission_positions_m,
                inherited_velocity_m_s: input.inherited_velocity_m_s,
            },
            LiquidHeatSink {
                liquid: &mut next.liquid,
                weights: input.heat_weights,
            },
        )?;
        let emitted_grains = formation.formation.dust.particles.len();
        let wear = formation.formation.dust.wear;
        let emitted_kinetic_j = formation.formation.dust.translational_kinetic_j;
        next.grains.extend(formation.formation.dust.particles);
        let (fluid, drag) =
            next.liquid
                .step_suspension_free(&mut next.grains, input.dt_s, input.coupling_steps)?;
        let actual_emitted_mass: f64 = next.grains[self.grains.len()..]
            .iter()
            .map(suspension::Particle::mass_kg)
            .sum();
        let mass_defect = actual_emitted_mass - wear.mass_kg;
        if !mass_defect.is_finite()
            || mass_defect.abs() > 1e-10 * wear.mass_kg.max(f64::MIN_POSITIVE)
        {
            return Err("wear suspension emitted mass balance failure");
        }
        let mut momentum = [0.; 3];
        let mut scale = [0.; 3];
        // Actual velocity increments avoid subtracting two large total momenta.
        for (old, new) in self.liquid.particles().iter().zip(next.liquid.particles()) {
            for axis in 0..3 {
                let impulse = old.mass * (new.velocity[axis] - old.velocity[axis]);
                momentum[axis] += impulse;
                scale[axis] += impulse.abs();
            }
        }
        for (old, new) in self.grains.iter().zip(&next.grains) {
            for axis in 0..3 {
                let impulse = old.mass_kg() * (new.velocity_m_s()[axis] - old.velocity_m_s()[axis]);
                momentum[axis] += impulse;
                scale[axis] += impulse.abs();
            }
        }
        for grain in &next.grains[self.grains.len()..] {
            for axis in 0..3 {
                let impulse = grain.mass_kg() * grain.velocity_m_s()[axis];
                momentum[axis] += impulse;
                scale[axis] += impulse.abs();
            }
        }
        for axis in 0..3 {
            let mut roundoff = 0.;
            let supplied = if let Some(velocity) = self.parent_velocity_m_s {
                let parent_impulse = (next.layer.remaining_mass_kg()
                    - self.layer.remaining_mass_kg())
                    * velocity[axis];
                momentum[axis] += parent_impulse;
                scale[axis] += parent_impulse.abs();
                roundoff =
                    16. * f64::EPSILON * self.layer.remaining_mass_kg() * velocity[axis].abs();
                0.
            } else {
                wear.mass_kg * input.inherited_velocity_m_s[axis]
            };
            momentum[axis] -= supplied;
            scale[axis] += supplied.abs();
            if !momentum[axis].is_finite()
                || momentum[axis].abs() > 1e-9 * scale[axis].max(f64::MIN_POSITIVE) + roundoff
            {
                return Err("wear suspension whole-step momentum balance failure");
            }
        }
        add_inventory(
            &mut next.energy.surface_energy_j,
            &mut next.energy.accumulation_correction_j[0],
            &mut next.energy_tail[0],
            formation.formation.surface_energy_j,
        )?;
        add_inventory(
            &mut next.energy.drag_numerical_loss_j,
            &mut next.energy.accumulation_correction_j[1],
            &mut next.energy_tail[1],
            drag.numerical_loss_j,
        )?;
        add_inventory(
            &mut next.energy.friction_input_j,
            &mut next.energy.accumulation_correction_j[2],
            &mut next.energy_tail[2],
            formation.friction_work_j,
        )?;
        if let Some(velocity) = next.parent_velocity_m_s {
            next.energy.parent_kinetic_j =
                0.5 * next.layer.remaining_mass_kg() * velocity.iter().map(|v| v * v).sum::<f64>();
            if (wear.mass_kg > 0.
                && next.layer.remaining_mass_kg() >= self.layer.remaining_mass_kg())
                || (emitted_kinetic_j > 0.
                    && next.energy.parent_kinetic_j >= self.energy.parent_kinetic_j)
                || (self.energy.parent_kinetic_j - next.energy.parent_kinetic_j - emitted_kinetic_j)
                    .abs()
                    > 16. * f64::EPSILON * self.energy.parent_kinetic_j + 1e-10 * emitted_kinetic_j
            {
                return Err("unrepresentable finite parent transfer");
            }
        } else {
            add_inventory(
                &mut next.energy.emission_kinetic_input_j,
                &mut next.energy.accumulation_correction_j[3],
                &mut next.energy_tail[3],
                emitted_kinetic_j,
            )?;
        }
        next.energy.kinetic_j = kinetic(&next.liquid, &next.grains);
        next.energy.enthalpy_j = next
            .liquid
            .transport_totals()
            .map_err(|_| "invalid advanced thermal energy")?
            .ok_or("missing advanced thermal transport")?
            .0;
        next.energy.suspension_heat_buffer_j = next
            .liquid
            .suspension_heat_buffer()
            .iter()
            .chain(next.liquid.suspension_heat_correction())
            .sum();
        let dk = next.energy.kinetic_j - self.energy.kinetic_j;
        let dh = next.energy.enthalpy_j - self.energy.enthalpy_j
            + next.energy.suspension_heat_buffer_j
            - self.energy.suspension_heat_buffer_j;
        let parent_change = next.energy.parent_kinetic_j - self.energy.parent_kinetic_j;
        let external_emission = if self.parent_velocity_m_s.is_none() {
            emitted_kinetic_j
        } else {
            0.
        };
        let defect =
            dk + dh + parent_change + formation.formation.surface_energy_j + drag.numerical_loss_j
                - formation.friction_work_j
                - external_emission;
        let scale = dk.abs()
            + dh.abs()
            + formation.formation.surface_energy_j
            + drag.numerical_loss_j
            + formation.friction_work_j
            + emitted_kinetic_j;
        if !next.energy.kinetic_j.is_finite()
            || !defect.is_finite()
            || defect.abs()
                > 1e-9 * scale.max(f64::MIN_POSITIVE)
                    + 8. * f64::EPSILON
                        * (self.energy.kinetic_j
                            + next.energy.kinetic_j
                            + self.energy.parent_kinetic_j
                            + next.energy.parent_kinetic_j
                            + self.energy.suspension_heat_buffer_j
                            + next.energy.suspension_heat_buffer_j)
        {
            return Err("wear suspension whole-step energy balance failure");
        }
        let report = WearSuspensionStep {
            contact: formation.contact,
            wear,
            emitted_grains,
            emitted_kinetic_j,
            surface_energy_j: formation.formation.surface_energy_j,
            friction_work_j: formation.friction_work_j,
            friction_heat_j: formation.friction_heat_j,
            fluid,
            drag,
            energy_defect_j: defect,
            emitted_mass_defect_kg: mass_defect,
            momentum_defect_n_s: momentum,
        };
        *self = next;
        Ok(report)
    }
}

#[cfg(test)]
mod compensated_tests {
    use super::add_inventory;
    #[test]
    fn increments_below_two_part_precision_remain_owned() {
        let mut high = 1.;
        let mut low = 0.;
        let mut tail = Vec::new();
        for exponent in [60, 120, 180, 240] {
            add_inventory(&mut high, &mut low, &mut tail, 2_f64.powi(-exponent)).unwrap();
        }
        assert_eq!(high, 1.);
        assert_eq!(low, 2_f64.powi(-60));
        assert_eq!(
            tail,
            vec![2_f64.powi(-240), 2_f64.powi(-180), 2_f64.powi(-120)]
        );
        let before = (high, low, tail.clone());
        assert!(add_inventory(&mut high, &mut low, &mut tail, f64::INFINITY).is_err());
        assert_eq!((high, low, tail), before);
    }
    #[test]
    fn sub_ulp_increments_accumulate_and_overflow_is_atomic() {
        let mut high = 1.;
        let mut low = 0.;
        let mut tail = Vec::new();
        add_inventory(&mut high, &mut low, &mut tail, f64::EPSILON / 4.).unwrap();
        assert_eq!(high, 1.);
        assert_eq!(low, f64::EPSILON / 4.);
        for _ in 0..7 {
            add_inventory(&mut high, &mut low, &mut tail, f64::EPSILON / 4.).unwrap();
        }
        assert_eq!(high, 1. + 2. * f64::EPSILON);
        assert_eq!(low, 0.);
        high = f64::MAX;
        assert!(add_inventory(&mut high, &mut low, &mut tail, f64::MAX).is_err());
        assert_eq!(high, f64::MAX);
        assert_eq!(low, 0.);
    }
}
