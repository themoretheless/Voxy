//! Conservative interfacial mass transfer to a finite, well-mixed pure-vapor cell.
use super::{Error, Liquid, SaturationCurve, finite, positive};

/// A fixed-volume, well-mixed pure-vapor control volume, colocated with the interface.
/// Thermal reference: `mass * (specific_heat_cv * temperature + latent_heat)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaporCell {
    pub mass: f64,
    pub volume: f64,
    pub temperature: f64,
    pub velocity: [f64; 3],
    pub specific_heat_cv: f64,
}
impl VaporCell {
    /// Absolute partial pressure of the pure vapor (zero for an empty cell).
    /// # Errors
    /// Invalid state, gas constant, or numerical overflow.
    pub fn pressure(self, gas_constant: f64) -> Result<f64, Error> {
        if !self.mass.is_finite()
            || self.mass < 0.0
            || !positive(self.volume)
            || !positive(self.temperature)
            || !positive(self.specific_heat_cv)
            || !finite(self.velocity)
            || !positive(gas_constant)
        {
            return Err(Error::InvalidTransport);
        }
        let pressure = self.mass / self.volume * gas_constant * self.temperature;
        if !pressure.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(pressure)
    }
    /// Thermal plus bulk kinetic energy, including the supplied vapor reference offset.
    /// # Errors
    /// Invalid state/offset or overflow.
    pub fn energy(self, latent_heat: f64) -> Result<f64, Error> {
        self.pressure(1.0)?;
        if !positive(latent_heat) {
            return Err(Error::InvalidPhaseChange);
        }
        let energy = self.mass
            * (self.specific_heat_cv * self.temperature + latent_heat + kinetic(self.velocity));
        if !energy.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(energy)
    }
}
/// Prescribed exposed area and kinetic accommodation for a pure-substance interface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaporInterface {
    pub curve: SaturationCurve,
    /// Exposed area in m²; does not include hidden/immersed surfaces automatically.
    pub area: f64,
    /// Common evaporation/condensation accommodation coefficient in [0,1].
    pub accommodation: f64,
}
impl VaporInterface {
    /// Hertz-Knudsen net mass flux in kg/(m² s), positive into vapor.
    /// # Errors
    /// Invalid coefficients, temperatures outside the curve domain or pressure overflow.
    pub fn mass_flux(self, liquid_temperature: f64, vapor: VaporCell) -> Result<f64, Error> {
        self.activity_flux(liquid_temperature, vapor, 1.0)
    }
    fn activity_flux(
        self,
        liquid_temperature: f64,
        vapor: VaporCell,
        activity: f64,
    ) -> Result<f64, Error> {
        if !self.area.is_finite()
            || self.area < 0.0
            || !self.accommodation.is_finite()
            || !(0.0..=1.0).contains(&self.accommodation)
        {
            return Err(Error::InvalidTransport);
        }
        let saturation = self.curve.pressure(liquid_temperature)?;
        self.curve.pressure(vapor.temperature)?;
        let pressure = vapor.pressure(self.curve.vapor_gas_constant)?;
        let denominator = (2.0 * std::f64::consts::PI * self.curve.vapor_gas_constant).sqrt();
        let flux = self.accommodation
            * (activity * saturation / liquid_temperature.sqrt()
                - pressure / vapor.temperature.sqrt())
            / denominator;
        if !flux.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(flux)
    }
}
/// Ideal solution with one volatile solvent and otherwise nonvolatile components.
/// Molar masses are in kg/mol, in configured species order. No activity coefficients,
/// dissociation, precipitation or heat of mixing are modeled.
#[derive(Clone, Debug, PartialEq)]
pub struct SolutionVaporInterface {
    pub interface: VaporInterface,
    pub solvent: usize,
    pub molar_masses: Vec<f64>,
}
impl SolutionVaporInterface {
    pub(crate) fn weights(&self, fractions: &[f64]) -> Result<(f64, f64), Error> {
        if self.solvent >= self.molar_masses.len()
            || self.molar_masses.len() != fractions.len()
            || self.molar_masses.iter().any(|m| !positive(*m))
        {
            return Err(Error::InvalidTransport);
        }
        super::species::validate_row(fractions, self.molar_masses.len())?;
        let minimum = self
            .molar_masses
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let solvent_weight = minimum / self.molar_masses[self.solvent];
        let solute_weight: f64 = fractions
            .iter()
            .zip(&self.molar_masses)
            .enumerate()
            .filter(|(i, _)| *i != self.solvent)
            .map(|(_, (y, m))| y * (minimum / m))
            .sum();
        Ok((solvent_weight, solute_weight))
    }
    /// Solvent mole fraction, using stable common-scaled mole weights.
    /// # Errors
    /// Invalid molar masses/composition or unrepresentable mole weights.
    pub fn activity(&self, fractions: &[f64]) -> Result<f64, Error> {
        let (weight, solute) = self.weights(fractions)?;
        mole_fraction(fractions[self.solvent] * weight, solute)
    }
    /// Raoult-law solvent equilibrium pressure in absolute Pa.
    /// # Errors
    /// Invalid model/composition or saturation temperature outside its declared domain.
    pub fn equilibrium_pressure(&self, temperature: f64, fractions: &[f64]) -> Result<f64, Error> {
        Ok(self.activity(fractions)? * self.interface.curve.pressure(temperature)?)
    }
}
/// Local midpoint step-doubling controls. These are local estimates, not global bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaporExchangeAccuracy {
    pub relative_tolerance: f64,
    /// Absolute mass scale in kg.
    pub mass_tolerance: f64,
    /// Absolute temperature scale in K.
    pub temperature_tolerance: f64,
    pub max_attempts: usize,
}
impl Default for VaporExchangeAccuracy {
    fn default() -> Self {
        Self {
            relative_tolerance: 1e-6,
            mass_tolerance: 1e-12,
            temperature_tolerance: 1e-6,
            max_attempts: 4096,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct State {
    pub(crate) mass: f64,
    pub(crate) solvent_mass: f64,
    pub(crate) solvent_weight: f64,
    pub(crate) solvent_specific_heat: f64,
    pub(crate) solute_capacity: f64,
    pub(crate) solute_weight: f64,
    pub(crate) temperature: f64,
    pub(crate) velocity: [f64; 3],
    pub(crate) vapor: VaporCell,
}
impl State {
    pub(crate) fn capacity(self) -> f64 {
        self.solute_capacity + self.solvent_mass * self.solvent_specific_heat
    }
}
impl Liquid {
    /// Exchanges a pure liquid particle with a finite pure-vapor cell at a fixed area.
    /// Returns net mass entering vapor. Includes evaporative cooling, condensation heat,
    /// donor momentum, and conversion of receiver mixing kinetic energy into heat.
    /// Vapor cp must equal liquid cp (`cv + R`), consistent with constant latent enthalpy.
    /// No motion/pressure-volume work is advanced and no exposed area is inferred.
    /// # Errors
    /// Invalid index/fields/control, non-pure liquid, latent/gas EOS on this material,
    /// incompatible capacities, model-domain/resource limits, or attempt exhaustion.
    /// Fluid and vapor cell roll back together. Liquid must retain positive mass.
    pub fn exchange_vapor(
        &mut self,
        index: usize,
        vapor: &mut VaporCell,
        interface: VaporInterface,
        dt: f64,
        accuracy: VaporExchangeAccuracy,
    ) -> Result<f64, Error> {
        self.exchange_vapor_impl(index, vapor, interface, dt, accuracy, None)
    }
    /// Selectively transfers the configured solvent; nonvolatile species masses stay
    /// in the liquid. Solvent activity is updated during every midpoint trial.
    /// # Errors
    /// Missing species, active reaction, invalid molar masses or ordinary exchange errors.
    /// Composition, fluid mechanics/heat and vapor commit or roll back together.
    pub fn exchange_solution_vapor(
        &mut self,
        index: usize,
        vapor: &mut VaporCell,
        model: &SolutionVaporInterface,
        dt: f64,
        accuracy: VaporExchangeAccuracy,
    ) -> Result<f64, Error> {
        self.exchange_vapor_impl(index, vapor, model.interface, dt, accuracy, Some(model))
    }
    fn exchange_vapor_impl(
        &mut self,
        index: usize,
        vapor: &mut VaporCell,
        interface: VaporInterface,
        dt: f64,
        accuracy: VaporExchangeAccuracy,
        solution: Option<&SolutionVaporInterface>,
    ) -> Result<f64, Error> {
        if !positive(dt)
            || !accuracy.relative_tolerance.is_finite()
            || accuracy.relative_tolerance < 0.0
            || !positive(accuracy.mass_tolerance)
            || !positive(accuracy.temperature_tolerance)
            || accuracy.max_attempts == 0
        {
            return Err(Error::InvalidTransport);
        }
        let particle = self.particles.get(index).ok_or(Error::InvalidParticle)?;
        let fields = self.transport.as_ref().ok_or(Error::InvalidTransport)?;
        if (solution.is_none() && fields.species.is_some())
            || fields.fields[index].concentration != 0.0
            || fields
                .phase
                .as_ref()
                .is_some_and(|p| p.models[particle.material].is_some())
            || self.gas_equations[particle.material].is_some()
        {
            return Err(Error::InvalidPhaseChange);
        }
        let cp = fields.materials[particle.material].specific_heat;
        let (solvent_mass, solvent_weight, solute_weight) = if let Some(model) = solution {
            let species = fields.species.as_ref().ok_or(Error::InvalidTransport)?;
            if species.reaction.is_some() {
                return Err(Error::InvalidTransport);
            }
            let row = &species.fractions[index];
            let (weight, solute) = model.weights(row)?;
            (
                particle.mass * row[model.solvent],
                weight,
                particle.mass * solute,
            )
        } else {
            (particle.mass, 1.0, 0.0)
        };
        let (solvent_specific_heat, solute_capacity) = if let Some(model) = solution {
            let species = fields.species.as_ref().ok_or(Error::InvalidTransport)?;
            if let Some(capacities) = &species.heat_capacities {
                let solute = species.fractions[index]
                    .iter()
                    .zip(capacities)
                    .enumerate()
                    .filter(|(component, _)| *component != model.solvent)
                    .map(|(_, (fraction, capacity))| particle.mass * fraction * capacity)
                    .sum::<f64>();
                (capacities[model.solvent], solute)
            } else {
                (cp, (particle.mass - solvent_mass) * cp)
            }
        } else {
            (cp, 0.0)
        };
        let vapor_cp = vapor.specific_heat_cv + interface.curve.vapor_gas_constant;
        if !positive(vapor_cp)
            || (solvent_specific_heat - vapor_cp).abs()
                > 64.0 * f64::EPSILON * solvent_specific_heat
            || !solute_capacity.is_finite()
        {
            return Err(Error::InvalidTransport);
        }
        let initial = State {
            mass: particle.mass,
            solvent_mass,
            solvent_weight,
            solvent_specific_heat,
            solute_capacity,
            solute_weight,
            temperature: fields.fields[index].temperature,
            velocity: particle.velocity,
            vapor: *vapor,
        };
        state_flux(initial, interface)?;
        let total = fields.energy(particle, &fields.fields[index], index)?
            + particle.mass * kinetic(particle.velocity)
            + vapor.energy(interface.curve.latent_heat)?;
        if !total.is_finite() {
            return Err(Error::NumericalFailure);
        }
        let updated = advance(initial, interface, dt, accuracy)?;
        let mut candidate = self.clone();
        candidate.particles[index].mass = updated.mass;
        candidate.particles[index].velocity = updated.velocity;
        candidate
            .transport
            .as_mut()
            .ok_or(Error::InvalidTransport)?
            .fields[index]
            .temperature = updated.temperature;
        if let Some(model) = solution {
            let species = candidate
                .transport
                .as_mut()
                .and_then(|t| t.species.as_mut())
                .ok_or(Error::InvalidTransport)?;
            let row = &mut species.fractions[index];
            for (component, value) in row.iter_mut().enumerate() {
                *value = if component == model.solvent {
                    updated.solvent_mass / updated.mass
                } else {
                    particle.mass * *value / updated.mass
                };
            }
            super::species::validate_row(row, model.molar_masses.len())?;
            candidate.species_totals()?;
        }
        candidate.effective_materials()?;
        let mass = updated.vapor.mass - vapor.mass;
        *self = candidate;
        *vapor = updated.vapor;
        Ok(mass)
    }
}
fn kinetic(velocity: [f64; 3]) -> f64 {
    0.5 * velocity.iter().map(|v| v * v).sum::<f64>()
}
pub(crate) fn advance(
    state: State,
    interface: VaporInterface,
    dt: f64,
    accuracy: VaporExchangeAccuracy,
) -> Result<State, Error> {
    if !positive(dt)
        || !accuracy.relative_tolerance.is_finite()
        || accuracy.relative_tolerance < 0.
        || !positive(accuracy.mass_tolerance)
        || !positive(accuracy.temperature_tolerance)
        || accuracy.max_attempts == 0
        || !positive(state.mass)
        || !state.solvent_mass.is_finite()
        || state.solvent_mass < 0.
        || state.solvent_mass > state.mass
        || !positive(state.solvent_weight)
        || !positive(state.solvent_specific_heat)
        || !state.solute_capacity.is_finite()
        || state.solute_capacity < 0.
        || !state.solute_weight.is_finite()
        || state.solute_weight < 0.
        || !positive(state.temperature)
        || !finite(state.velocity)
        || !positive(state.capacity())
    {
        return Err(Error::InvalidTransport);
    }
    let vapor_cp = state.vapor.specific_heat_cv + interface.curve.vapor_gas_constant;
    if !positive(vapor_cp)
        || (state.solvent_specific_heat - vapor_cp).abs()
            > 64. * f64::EPSILON * state.solvent_specific_heat
    {
        return Err(Error::InvalidTransport);
    }
    state_flux(state, interface)?;
    adaptive_midpoint(
        state,
        dt,
        accuracy.max_attempts,
        |state, step| trial(state, interface, step),
        |start, coarse, fine| error(start, coarse, fine, accuracy),
    )
}

/// Shared local step-doubling controller for interface constitutive models.
/// Trial states are values; only the returned accepted state is published by
/// the particle/film owner. The model supplies a midpoint trial and normalized
/// local error (acceptance <= 1), including its caloric domain admission.
pub(crate) fn adaptive_midpoint<S: Copy>(
    mut state: S,
    dt: f64,
    max_attempts: usize,
    trial: impl Fn(S, f64) -> Result<S, Error>,
    error: impl Fn(S, S, S) -> Result<f64, Error>,
) -> Result<S, Error> {
    if !positive(dt) || max_attempts == 0 {
        return Err(Error::InvalidTransport);
    }
    let mut elapsed = 0.0;
    let mut step = dt;
    for _ in 0..max_attempts {
        let remaining = dt - elapsed;
        step = step.min(remaining);
        if step <= 0.0 || elapsed + step <= elapsed {
            return Err(Error::NumericalFailure);
        }
        let attempts = trial(state, step).and_then(|coarse| {
            let half = trial(state, 0.5 * step)?;
            let fine = trial(half, 0.5 * step)?;
            let estimate = error(state, coarse, fine)?;
            if !estimate.is_finite() || estimate < 0. {
                return Err(Error::NumericalFailure);
            }
            Ok((fine, estimate))
        });
        match attempts {
            Ok((fine, estimate)) => {
                if estimate <= 1.0 {
                    state = fine;
                    if step >= remaining {
                        return Ok(state);
                    }
                    elapsed += step;
                }
                let factor = if estimate == 0.0 {
                    2.0
                } else {
                    (0.9 * estimate.powf(-1.0 / 3.0)).clamp(0.2, 2.0)
                };
                step *= factor;
            }
            Err(_) => step *= 0.5,
        }
    }
    Err(Error::NumericalFailure)
}
fn trial(state: State, interface: VaporInterface, dt: f64) -> Result<State, Error> {
    let flux = state_flux(state, interface)?;
    let half = transfer(state, 0.5 * dt * interface.area * flux, state, interface)?;
    let midpoint_flux = state_flux(half, interface)?;
    let result = transfer(state, dt * interface.area * midpoint_flux, half, interface)?;
    state_flux(result, interface)?;
    Ok(result)
}
fn transfer(
    state: State,
    mass: f64,
    midpoint: State,
    interface: VaporInterface,
) -> Result<State, Error> {
    if !mass.is_finite() || (mass > 0.0 && mass >= state.solvent_mass) || -mass > state.vapor.mass {
        return Err(Error::NumericalFailure);
    }
    if mass == 0.0 {
        return Ok(state);
    }
    let mut result = state;
    result.mass -= mass;
    result.solvent_mass -= mass;
    result.vapor.mass += mass;
    let liquid_energy = state.capacity() * state.temperature;
    let vapor_energy = state.vapor.mass
        * (state.vapor.specific_heat_cv * state.vapor.temperature + interface.curve.latent_heat);
    let donor_temperature = if mass > 0.0 {
        midpoint.temperature
    } else {
        midpoint.vapor.temperature
    };
    let carried =
        mass * (state.vapor.specific_heat_cv * donor_temperature + interface.curve.latent_heat);
    if mass > 0.0 {
        for axis in 0..3 {
            result.vapor.velocity[axis] = state.vapor.velocity[axis]
                + mass / result.vapor.mass * (state.velocity[axis] - state.vapor.velocity[axis]);
        }
    } else {
        for axis in 0..3 {
            result.velocity[axis] = state.velocity[axis]
                + (-mass) / result.mass * (state.vapor.velocity[axis] - state.velocity[axis]);
        }
    }
    let before_kinetic =
        state.mass * kinetic(state.velocity) + state.vapor.mass * kinetic(state.vapor.velocity);
    let after_kinetic =
        result.mass * kinetic(result.velocity) + result.vapor.mass * kinetic(result.vapor.velocity);
    let mixing_heat = before_kinetic - after_kinetic;
    let liquid_after = liquid_energy - carried + if mass < 0.0 { mixing_heat } else { 0.0 };
    let vapor_after = vapor_energy + carried + if mass > 0.0 { mixing_heat } else { 0.0 };
    result.temperature = liquid_after / result.capacity();
    if result.vapor.mass > 0.0 {
        result.vapor.temperature = (vapor_after / result.vapor.mass - interface.curve.latent_heat)
            / result.vapor.specific_heat_cv;
    }
    if !positive(result.temperature)
        || !positive(result.vapor.temperature)
        || !finite(result.velocity)
        || !finite(result.vapor.velocity)
    {
        return Err(Error::NumericalFailure);
    }
    Ok(result)
}
fn error(start: State, coarse: State, fine: State, a: VaporExchangeAccuracy) -> Result<f64, Error> {
    let values = [
        (start.mass, coarse.mass, fine.mass, a.mass_tolerance),
        (
            start.solvent_mass,
            coarse.solvent_mass,
            fine.solvent_mass,
            a.mass_tolerance,
        ),
        (
            start.vapor.mass,
            coarse.vapor.mass,
            fine.vapor.mass,
            a.mass_tolerance,
        ),
        (
            start.temperature,
            coarse.temperature,
            fine.temperature,
            a.temperature_tolerance,
        ),
        (
            start.vapor.temperature,
            coarse.vapor.temperature,
            fine.vapor.temperature,
            a.temperature_tolerance,
        ),
    ];
    let mut maximum: f64 = 0.0;
    for (initial, full, refined, absolute) in values {
        let scale = absolute + a.relative_tolerance * initial.abs().max(refined.abs());
        if !positive(scale) {
            return Err(Error::NumericalFailure);
        }
        maximum = maximum.max(((full - refined) / scale).abs() / 3.0);
    }
    if !maximum.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(maximum)
}

fn mole_fraction(solvent: f64, solute: f64) -> Result<f64, Error> {
    let total = solvent + solute;
    if !positive(total)
        || !solvent.is_finite()
        || solvent < 0.0
        || !solute.is_finite()
        || solute < 0.0
    {
        return Err(Error::NumericalFailure);
    }
    Ok(solvent / total)
}
fn state_flux(state: State, interface: VaporInterface) -> Result<f64, Error> {
    let activity = mole_fraction(
        state.solvent_mass * state.solvent_weight,
        state.solute_weight,
    )?;
    interface.activity_flux(state.temperature, state.vapor, activity)
}
