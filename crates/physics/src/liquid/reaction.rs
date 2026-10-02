//! One irreversible mass-action reaction with conservative heat release.
use super::{Error, Liquid, Particle, transport::Transport};

/// Fuel + oxidizer -> product. Species indices refer to the configured global schema.
/// Kinetics: `dY_f/dt = -rate * Y_f * Y_o`, with rate in 1/s.
#[derive(Clone, Debug, PartialEq)]
pub struct Reaction {
    pub fuel: usize,
    pub oxidizer: usize,
    pub product: usize,
    /// kg oxidizer consumed per kg fuel.
    pub oxidizer_ratio: f64,
    pub rate: f64,
    /// Chemical reference energies in J/kg, one per species.
    pub chemical_energies: Vec<f64>,
}
/// Optional temperature dependence for the mass-fraction reaction coefficient.
/// `k(T) = Reaction::rate * exp(-activation_temperature / T)` above the threshold.
/// The threshold is an explicit model cutoff, not an experimentally derived ignition law.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactionKinetics {
    /// Activation energy divided by the universal gas constant, in kelvin.
    pub activation_temperature: f64,
    /// Reaction is suppressed below this temperature, in kelvin.
    pub ignition_temperature: f64,
}
/// Local step-doubling error control for temperature-dependent chemistry.
/// The tolerances estimate local truncation error, not a bound on total flow error.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactionAccuracy {
    pub relative_tolerance: f64,
    /// Absolute error scale for each dimensionless mass fraction.
    pub fraction_tolerance: f64,
    /// Absolute error scale for temperature, in kelvin.
    pub temperature_tolerance: f64,
    /// Maximum accepted plus rejected attempts per particle and chemical stage.
    pub max_attempts: usize,
}
impl Default for ReactionAccuracy {
    fn default() -> Self {
        Self {
            relative_tolerance: 1e-6,
            fraction_tolerance: 1e-10,
            temperature_tolerance: 1e-6,
            max_attempts: 4096,
        }
    }
}
impl ReactionKinetics {
    /// Evaluates the effective coefficient in 1/s.
    /// # Errors
    /// Invalid coefficients or negative/nonfinite temperature or prefactor.
    pub fn coefficient(self, prefactor: f64, temperature: f64) -> Result<f64, Error> {
        if [
            self.activation_temperature,
            self.ignition_temperature,
            prefactor,
            temperature,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err(Error::InvalidTransport);
        }
        if temperature < self.ignition_temperature
            || (temperature == 0.0 && self.activation_temperature > 0.0)
        {
            return Ok(0.0);
        }
        let factor = if self.activation_temperature == 0.0 {
            1.0
        } else {
            (-self.activation_temperature / temperature).exp()
        };
        Ok(prefactor * factor)
    }
}
impl Liquid {
    /// Enables or disables the single reaction. Configuration changes the energy reference;
    /// it does not inject heat. Disable before replacing the species schema.
    /// # Errors
    /// Missing composition, invalid indices, coefficients, or endothermic reaction.
    pub fn configure_reaction(&mut self, reaction: Option<Reaction>) -> Result<(), Error> {
        let species = self
            .transport
            .as_mut()
            .and_then(|t| t.species.as_mut())
            .ok_or(Error::InvalidTransport)?;
        if let Some(r) = &reaction {
            let n = species.names.len();
            if r.fuel >= n
                || r.oxidizer >= n
                || r.product >= n
                || r.fuel == r.oxidizer
                || r.product == r.fuel
                || r.product == r.oxidizer
                || !r.oxidizer_ratio.is_finite()
                || r.oxidizer_ratio <= 0.0
                || !r.rate.is_finite()
                || r.rate < 0.0
                || r.chemical_energies.len() != n
                || r.chemical_energies
                    .iter()
                    .any(|e| !e.is_finite() || *e < 0.0)
            {
                return Err(Error::InvalidTransport);
            }
            let heat = r.chemical_energies[r.fuel]
                + r.oxidizer_ratio * r.chemical_energies[r.oxidizer]
                - (1.0 + r.oxidizer_ratio) * r.chemical_energies[r.product];
            if !heat.is_finite() || heat < 0.0 {
                return Err(Error::InvalidTransport);
            }
        }
        species.reaction = reaction;
        species.kinetics = None;
        species.reaction_accuracy = None;
        Ok(())
    }
    /// Configures temperature-dependent kinetics for the active reaction.
    /// Enables default local error control using exponential midpoint prediction.
    /// This controls chemistry alone; coupled flow still needs timestep refinement.
    /// Configuring a replacement reaction resets these kinetics.
    /// # Errors
    /// Missing active reaction or invalid activation/threshold temperatures. Atomic.
    pub fn configure_reaction_kinetics(
        &mut self,
        kinetics: Option<ReactionKinetics>,
    ) -> Result<(), Error> {
        if let Some(k) = kinetics {
            k.coefficient(1.0, 300.0)?;
        }
        let species = self
            .transport
            .as_mut()
            .and_then(|t| t.species.as_mut())
            .ok_or(Error::InvalidTransport)?;
        if species.reaction.is_none() {
            return Err(Error::InvalidTransport);
        }
        species.kinetics = kinetics;
        species.reaction_accuracy = kinetics.map(|_| ReactionAccuracy::default());
        Ok(())
    }
    /// Sets local chemical error control. Thermal kinetics enable default control;
    /// `None` explicitly selects fixed midpoint stages for refinement experiments.
    /// # Errors
    /// Missing thermal kinetics, invalid tolerances or zero attempt limit. Atomic.
    pub fn configure_reaction_accuracy(
        &mut self,
        accuracy: Option<ReactionAccuracy>,
    ) -> Result<(), Error> {
        if accuracy.is_some_and(|a| {
            !a.relative_tolerance.is_finite()
                || a.relative_tolerance < 0.0
                || !a.fraction_tolerance.is_finite()
                || a.fraction_tolerance <= 0.0
                || !a.temperature_tolerance.is_finite()
                || a.temperature_tolerance <= 0.0
                || a.max_attempts == 0
        }) {
            return Err(Error::InvalidTransport);
        }
        let species = self
            .transport
            .as_mut()
            .and_then(|t| t.species.as_mut())
            .ok_or(Error::InvalidTransport)?;
        if species.kinetics.is_none() {
            return Err(Error::InvalidTransport);
        }
        species.reaction_accuracy = accuracy;
        Ok(())
    }
    /// Total chemical reference energy. Add thermal and kinetic energy for the full ledger.
    /// # Errors
    /// Overflow of the chemical energy sum.
    pub fn chemical_energy(&self) -> Result<Option<f64>, Error> {
        let Some(s) = self.transport.as_ref().and_then(|t| t.species.as_ref()) else {
            return Ok(None);
        };
        let Some(r) = &s.reaction else {
            return Ok(None);
        };
        let total = self
            .particles
            .iter()
            .zip(&s.fractions)
            .map(|(p, row)| p.mass * energy(row, &r.chemical_energies))
            .sum::<f64>();
        if !total.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(Some(total))
    }
    /// Advances chemistry alone atomically. Constant kinetics use an exact solution;
    /// temperature-dependent kinetics use an exponential midpoint approximation.
    /// # Errors
    /// Invalid time or unrepresentable thermal/composition state.
    pub fn react(&mut self, dt: f64) -> Result<(), Error> {
        if !dt.is_finite() || dt < 0.0 {
            return Err(Error::InvalidTransport);
        }
        let mut candidate = self.clone();
        candidate
            .transport
            .as_mut()
            .ok_or(Error::InvalidTransport)?
            .react(&candidate.particles, dt)?;
        candidate.effective_materials()?;
        *self = candidate;
        Ok(())
    }
}
fn energy(row: &[f64], energies: &[f64]) -> f64 {
    row.iter().zip(energies).map(|(y, e)| y * e).sum()
}
#[derive(Clone)]
struct ChemicalState {
    index: usize,
    fractions: Vec<f64>,
    thermal: f64,
}
impl Transport {
    pub(super) fn react(&mut self, particles: &[Particle], dt: f64) -> Result<(), Error> {
        let Some(r) = self.species.as_ref().and_then(|s| s.reaction.clone()) else {
            return Ok(());
        };
        if dt == 0.0 || r.rate == 0.0 {
            return Ok(());
        }
        let kinetics = self.species.as_ref().and_then(|s| s.kinetics);
        let accuracy = self.species.as_ref().and_then(|s| s.reaction_accuracy);
        for (i, p) in particles.iter().enumerate() {
            let state = ChemicalState {
                index: i,
                fractions: self
                    .species
                    .as_ref()
                    .ok_or(Error::InvalidTransport)?
                    .fractions[i]
                    .clone(),
                thermal: self.energy(p, &self.fields[i], i)?,
            };
            let updated = if let Some(k) = kinetics {
                if let Some(a) = accuracy {
                    self.adaptive_reaction(p, state, &r, k, a, dt)?
                } else {
                    self.reaction_trial(p, &state, &r, k, dt)?
                }
            } else {
                fixed_reaction(p, &state, &r, r.rate, dt)?
            };
            self.species
                .as_mut()
                .ok_or(Error::InvalidTransport)?
                .fractions[i] = updated.fractions;
            self.set_energy(i, p, updated.thermal)?;
        }
        Ok(())
    }
    fn reaction_trial(
        &self,
        p: &Particle,
        state: &ChemicalState,
        r: &Reaction,
        k: ReactionKinetics,
        dt: f64,
    ) -> Result<ChemicalState, Error> {
        let (temperature, _) = self.decode_capacity(
            p,
            state.thermal,
            state.index,
            self.heat_capacity_for_row(&state.fractions, p.material)?,
        )?;
        let initial = k.coefficient(r.rate, temperature)?;
        let predicted = fixed_reaction(p, state, r, initial, 0.5 * dt)?;
        let (midpoint, _) = self.decode_capacity(
            p,
            predicted.thermal,
            predicted.index,
            self.heat_capacity_for_row(&predicted.fractions, p.material)?,
        )?;
        fixed_reaction(p, state, r, k.coefficient(r.rate, midpoint)?, dt)
    }
    fn adaptive_reaction(
        &self,
        p: &Particle,
        mut state: ChemicalState,
        r: &Reaction,
        k: ReactionKinetics,
        accuracy: ReactionAccuracy,
        dt: f64,
    ) -> Result<ChemicalState, Error> {
        let mut elapsed = 0.0;
        let mut step = dt;
        for _ in 0..accuracy.max_attempts {
            let remaining = dt - elapsed;
            step = step.min(remaining);
            if step <= 0.0 || elapsed + step <= elapsed {
                return Err(Error::NumericalFailure);
            }
            let coarse = self.reaction_trial(p, &state, r, k, step)?;
            let half = self.reaction_trial(p, &state, r, k, 0.5 * step)?;
            let fine = self.reaction_trial(p, &half, r, k, 0.5 * step)?;
            let error = self.reaction_error(p, &state, &coarse, &fine, accuracy)?;
            if error <= 1.0 {
                state = fine;
                if step >= remaining {
                    return Ok(state);
                }
                elapsed += step;
            }
            let factor = if error == 0.0 {
                2.0
            } else {
                (0.9 * error.powf(-1.0 / 3.0)).clamp(0.2, 2.0)
            };
            step *= factor;
        }
        Err(Error::NumericalFailure)
    }
    fn reaction_error(
        &self,
        p: &Particle,
        start: &ChemicalState,
        coarse: &ChemicalState,
        fine: &ChemicalState,
        a: ReactionAccuracy,
    ) -> Result<f64, Error> {
        let mut error: f64 = 0.0;
        for ((before, full), refined) in start
            .fractions
            .iter()
            .zip(&coarse.fractions)
            .zip(&fine.fractions)
        {
            let scale =
                a.fraction_tolerance + a.relative_tolerance * before.abs().max(refined.abs());
            if !scale.is_finite() {
                return Err(Error::NumericalFailure);
            }
            error = error.max(((full - refined) / scale).abs() / 3.0);
        }
        let (before, _) = self.decode_capacity(
            p,
            start.thermal,
            start.index,
            self.heat_capacity_for_row(&start.fractions, p.material)?,
        )?;
        let (full, _) = self.decode_capacity(
            p,
            coarse.thermal,
            coarse.index,
            self.heat_capacity_for_row(&coarse.fractions, p.material)?,
        )?;
        let (refined, _) = self.decode_capacity(
            p,
            fine.thermal,
            fine.index,
            self.heat_capacity_for_row(&fine.fractions, p.material)?,
        )?;
        let scale =
            a.temperature_tolerance + a.relative_tolerance * before.abs().max(refined.abs());
        if !scale.is_finite() {
            return Err(Error::NumericalFailure);
        }
        error = error.max(((full - refined) / scale).abs() / 3.0);
        if !error.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(error)
    }
}
fn fixed_reaction(
    p: &Particle,
    state: &ChemicalState,
    r: &Reaction,
    rate: f64,
    dt: f64,
) -> Result<ChemicalState, Error> {
    let fractions = products(&state.fractions, r, rate, dt)?;
    let thermal = state.thermal
        + p.mass
            * (energy(&state.fractions, &r.chemical_energies)
                - energy(&fractions, &r.chemical_energies));
    if !thermal.is_finite() || thermal < 0.0 {
        return Err(Error::NumericalFailure);
    }
    Ok(ChemicalState {
        index: state.index,
        fractions,
        thermal,
    })
}
fn products(original: &[f64], r: &Reaction, rate: f64, dt: f64) -> Result<Vec<f64>, Error> {
    let mut row = original.to_vec();
    if rate == 0.0 || dt == 0.0 {
        return Ok(row);
    }
    let fuel = row[r.fuel];
    let oxidizer = row[r.oxidizer];
    let scaled = r.oxidizer_ratio * fuel;
    let low = scaled.min(oxidizer);
    let gap = (scaled - oxidizer).abs();
    let loss = if gap == 0.0 {
        let exposure = rate * low * dt;
        if exposure.is_infinite() {
            low
        } else {
            low * (exposure / (1.0 + exposure))
        }
    } else {
        let decay = -(-rate * gap * dt).exp_m1();
        low * decay * ((gap + low) / (gap + low * decay))
    };
    let consumed = (loss / r.oxidizer_ratio)
        .min(fuel)
        .min(oxidizer / r.oxidizer_ratio);
    row[r.fuel] = (fuel - consumed).max(0.0);
    row[r.oxidizer] = (oxidizer - r.oxidizer_ratio * consumed).max(0.0);
    row[r.product] += (fuel - row[r.fuel]) + (oxidizer - row[r.oxidizer]);
    super::species::validate_row(&row, r.chemical_energies.len())?;
    Ok(row)
}
