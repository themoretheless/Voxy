//! REACLIB strong-reaction network in cgs rate conventions, SI energy output.
//! Baryon mass fractions neglect mass defect in hydrodynamic mass. Binding energy
//! is an explicit reservoir; no built-in fitted nuclear dataset or weak reactions.
pub const AVOGADRO: f64 = 6.022_140_76e23;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nucleus {
    pub mass_number: u16,
    pub charge: u16,
    pub binding_energy: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Reaclib {
    /// Sum of exp(a0+a1/T9+a2/T9^(1/3)+a3*T9^(1/3)+a4*T9+a5*T9^(5/3)+a6*ln T9).
    pub sets: Vec<[f64; 7]>,
    pub min_temperature: f64,
    pub max_temperature: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Reaction {
    pub reactants: Vec<u8>,
    pub products: Vec<u8>,
    pub rate: Reaclib,
    /// Fraction of positive binding-energy release escaping as neutrinos.
    pub neutrino_fraction: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Network {
    pub nuclei: Vec<Nucleus>,
    pub reactions: Vec<Reaction>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    TemperatureOutsideFit,
    NumericalOverflow,
    BudgetExceeded,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Budget {
    pub max_step: f64,
    pub steps: usize,
    pub fit_evaluations: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Burn {
    pub deposited_energy: f64,
    pub escaped_neutrinos: f64,
    pub steps: usize,
    pub fit_evaluations: usize,
}
/// Instantaneous composition derivative and specific nuclear powers.
#[derive(Clone, Debug, PartialEq)]
pub struct Rates {
    /// dX/dt, in inverse seconds.
    pub mass_fraction_rates: Vec<f64>,
    /// Thermal deposition, W/kg; may be negative for endothermic reactions.
    pub deposited_power: f64,
    /// Escaping neutrino power, W/kg.
    pub escaped_neutrino_power: f64,
    pub fit_evaluations: usize,
}
impl Reaclib {
    fn validate(&self) -> Result<(), Error> {
        if self.sets.is_empty()
            || !self.sets.iter().flatten().all(|x| x.is_finite())
            || !self.min_temperature.is_finite()
            || !self.max_temperature.is_finite()
            || self.min_temperature <= 0.0
            || self.max_temperature < self.min_temperature
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
    /// Evaluate rate coefficient in standard REACLIB cgs units (order dependent).
    /// # Errors
    /// Invalid fit/temperature, outside fitted range or overflowing rate.
    pub fn evaluate(&self, temperature: f64) -> Result<f64, Error> {
        self.validate()?;
        if !temperature.is_finite() || temperature <= 0.0 {
            return Err(Error::InvalidInput);
        }
        if temperature < self.min_temperature || temperature > self.max_temperature {
            return Err(Error::TemperatureOutsideFit);
        }
        let t = temperature * 1e-9;
        let third = t.cbrt();
        let basis = [
            1.0,
            1.0 / t,
            1.0 / third,
            third,
            t,
            t * third * third,
            t.ln(),
        ];
        let mut rate = 0.0;
        for set in &self.sets {
            let exponent: f64 = set
                .iter()
                .zip(basis)
                .map(|(a, b)| if *a == 0.0 { 0.0 } else { a * b })
                .sum();
            rate += exponent.exp();
        }
        if rate.is_finite() {
            Ok(rate)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
}
impl Network {
    fn validate(&self, fractions: &[f64]) -> Result<(), Error> {
        if self.nuclei.is_empty()
            || fractions.len() != self.nuclei.len()
            || !fractions.iter().all(|x| x.is_finite() && *x >= 0.0)
            || (fractions.iter().sum::<f64>() - 1.0).abs() > 1e-10
        {
            return Err(Error::InvalidInput);
        }
        for nucleus in &self.nuclei {
            if nucleus.mass_number == 0
                || nucleus.charge > nucleus.mass_number
                || !nucleus.binding_energy.is_finite()
                || nucleus.binding_energy < 0.0
            {
                return Err(Error::InvalidInput);
            }
        }
        for reaction in &self.reactions {
            reaction.rate.validate()?;
            if reaction.reactants.len() != self.nuclei.len()
                || reaction.products.len() != self.nuclei.len()
                || !reaction.neutrino_fraction.is_finite()
                || !(0.0..=1.0).contains(&reaction.neutrino_fraction)
            {
                return Err(Error::InvalidInput);
            }
            let count: u32 = reaction.reactants.iter().map(|n| u32::from(*n)).sum();
            let product_count: u32 = reaction.products.iter().map(|n| u32::from(*n)).sum();
            if !(1..=3).contains(&count) || !(1..=3).contains(&product_count) {
                return Err(Error::InvalidInput);
            }
            let mut baryons = 0_i64;
            let mut charge = 0_i64;
            for ((a, b), n) in reaction
                .reactants
                .iter()
                .zip(&reaction.products)
                .zip(&self.nuclei)
            {
                let delta = i64::from(*b) - i64::from(*a);
                baryons += delta * i64::from(n.mass_number);
                charge += delta * i64::from(n.charge);
            }
            if baryons != 0 || charge != 0 {
                return Err(Error::InvalidInput);
            }
        }
        Ok(())
    }
    /// Negative nuclear binding reservoir in J/kg (baryon-mass convention).
    /// # Errors
    /// Invalid network/composition or energy overflow.
    pub fn reservoir(&self, fractions: &[f64]) -> Result<f64, Error> {
        self.validate(fractions)?;
        let energy = -1000.0
            * AVOGADRO
            * self
                .nuclei
                .iter()
                .zip(fractions)
                .map(|(n, x)| x / f64::from(n.mass_number) * n.binding_energy)
                .sum::<f64>();
        if energy.is_finite() {
            Ok(energy)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
    /// Explicit adaptive burn at held density kg/m³ and temperature K.
    /// Simultaneous reactions, factorial symmetry factors and density^(order-1).
    /// Caps gross fuel consumption to 5% per substep. All failures leave fractions
    /// unchanged; returned specific energies are J/kg. Thermal feedback belongs
    /// to the caller's coupled EOS integration; no fitted range extrapolation.
    /// # Errors
    /// Invalid data, temperature outside fit, budget failure or overflow.
    pub fn burn(
        &self,
        fractions: &mut [f64],
        density: f64,
        temperature: f64,
        dt: f64,
        budget: Budget,
    ) -> Result<Burn, Error> {
        self.validate(fractions)?;
        if ![density, temperature, dt, budget.max_step]
            .into_iter()
            .all(f64::is_finite)
            || density <= 0.0
            || temperature <= 0.0
            || dt < 0.0
            || budget.max_step <= 0.0
        {
            return Err(Error::InvalidInput);
        }
        let evaluations = self
            .reactions
            .iter()
            .try_fold(0_usize, |sum, r| sum.checked_add(r.rate.sets.len()))
            .ok_or(Error::BudgetExceeded)?;
        let mut next = fractions.to_vec();
        let mut remaining = dt;
        let mut result = Burn {
            deposited_energy: 0.0,
            escaped_neutrinos: 0.0,
            steps: 0,
            fit_evaluations: 0,
        };
        while remaining > 0.0 {
            if result.steps >= budget.steps
                || evaluations > budget.fit_evaluations - result.fit_evaluations
            {
                return Err(Error::BudgetExceeded);
            }
            let (rates, consumption) = self.rates_impl(
                &next,
                density,
                temperature,
                budget.fit_evaluations - result.fit_evaluations,
            )?;
            let mut h = remaining.min(budget.max_step);
            for (x, rate) in next.iter().zip(&consumption) {
                if *rate > 0.0 {
                    h = h.min(0.05 * x / rate);
                }
            }
            if !h.is_finite() || h <= 0.0 || remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            for (x, derivative) in next.iter_mut().zip(&rates.mass_fraction_rates) {
                *x += h * derivative;
            }
            result.deposited_energy += h * rates.deposited_power;
            result.escaped_neutrinos += h * rates.escaped_neutrino_power;
            result.steps += 1;
            result.fit_evaluations += evaluations;
            remaining -= h;
        }
        self.validate(&next)?;
        if ![result.deposited_energy, result.escaped_neutrinos]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(Error::NumericalOverflow);
        }
        fractions.copy_from_slice(&next);
        Ok(result)
    }
}

impl Network {
    /// Fully ionized EOS for the current network composition.
    /// # Errors
    /// Invalid composition, or neutral nuclei unsupported by this EOS.
    pub fn mixture(&self, fractions: &[f64]) -> Result<crate::astrophysics_eos::Mixture, Error> {
        self.validate(fractions)?;
        let species: Vec<_> = self
            .nuclei
            .iter()
            .zip(fractions)
            .map(|(n, x)| crate::astrophysics_eos::Species {
                mass_fraction: *x,
                mass_number: n.mass_number,
                nuclear_charge: n.charge,
            })
            .collect();
        crate::astrophysics_eos::Mixture::new(&species).map_err(|_| Error::InvalidInput)
    }
    /// Isochoric one-zone burn with composition-dependent gas+radiation EOS.
    /// Specific internal energy is J/kg. Rates and temperature are refreshed
    /// every `max_step`; callers must demonstrate timestep convergence for stiff
    /// burning. Both composition and energy commit only after full success.
    /// # Errors
    /// Invalid EOS, fit-domain exit, exhausted shared budget or overflow.
    pub fn burn_isochoric(
        &self,
        fractions: &mut [f64],
        density: f64,
        specific_energy: &mut f64,
        dt: f64,
        budget: Budget,
    ) -> Result<Burn, Error> {
        if !density.is_finite()
            || density <= 0.0
            || !specific_energy.is_finite()
            || *specific_energy <= 0.0
            || !dt.is_finite()
            || dt < 0.0
            || !budget.max_step.is_finite()
            || budget.max_step <= 0.0
        {
            return Err(Error::InvalidInput);
        }
        let mut next = fractions.to_vec();
        let mut energy = *specific_energy;
        self.mixture(&next)?;
        let mut remaining = dt;
        let mut total = Burn {
            deposited_energy: 0.0,
            escaped_neutrinos: 0.0,
            steps: 0,
            fit_evaluations: 0,
        };
        while remaining > 0.0 {
            let temperature = self
                .mixture(&next)?
                .temperature(density, density * energy)
                .map_err(|_| Error::NumericalOverflow)?;
            let h = remaining.min(budget.max_step);
            if remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            let result = self.burn(
                &mut next,
                density,
                temperature,
                h,
                Budget {
                    max_step: h,
                    steps: budget.steps - total.steps,
                    fit_evaluations: budget.fit_evaluations - total.fit_evaluations,
                },
            )?;
            energy += result.deposited_energy;
            if !energy.is_finite() || energy <= 0.0 {
                return Err(Error::NumericalOverflow);
            }
            // Validate the updated thermodynamic state before committing it.
            self.mixture(&next)?
                .temperature(density, density * energy)
                .map_err(|_| Error::NumericalOverflow)?;
            total.deposited_energy += result.deposited_energy;
            total.escaped_neutrinos += result.escaped_neutrinos;
            total.steps += result.steps;
            total.fit_evaluations += result.fit_evaluations;
            remaining -= h;
        }
        fractions.copy_from_slice(&next);
        *specific_energy = energy;
        Ok(total)
    }
}

impl Network {
    /// Evaluate held-state nuclear rates without changing composition or energy.
    /// Density is SI kg/m³; temperature is K. Every coefficient set consumes
    /// one unit of the caller's fit-evaluation budget.
    /// # Errors
    /// Invalid input/network, fit-domain violation, budget or overflow.
    pub fn rates(
        &self,
        fractions: &[f64],
        density: f64,
        temperature: f64,
        fit_budget: usize,
    ) -> Result<Rates, Error> {
        self.rates_impl(fractions, density, temperature, fit_budget)
            .map(|r| r.0)
    }
    fn rates_impl(
        &self,
        fractions: &[f64],
        density: f64,
        temperature: f64,
        fit_budget: usize,
    ) -> Result<(Rates, Vec<f64>), Error> {
        self.validate(fractions)?;
        if !density.is_finite() || density <= 0.0 || !temperature.is_finite() || temperature <= 0.0
        {
            return Err(Error::InvalidInput);
        }
        let evaluations = self
            .reactions
            .iter()
            .try_fold(0_usize, |sum, r| sum.checked_add(r.rate.sets.len()))
            .ok_or(Error::BudgetExceeded)?;
        if evaluations > fit_budget {
            return Err(Error::BudgetExceeded);
        }
        let mut production = vec![0.0; fractions.len()];
        let mut consumption = vec![0.0; fractions.len()];
        let mut heat = 0.0;
        let mut neutrinos = 0.0;
        for reaction in &self.reactions {
            let count: u32 = reaction.reactants.iter().map(|n| u32::from(*n)).sum();
            let mut rate = reaction.rate.evaluate(temperature)?
                * (density * 1e-3).powi(i32::try_from(count - 1).map_err(|_| Error::InvalidInput)?);
            for ((amount, x), n) in reaction.reactants.iter().zip(fractions).zip(&self.nuclei) {
                rate *= (x / f64::from(n.mass_number)).powi(i32::from(*amount));
                for factor in 2..=*amount {
                    rate /= f64::from(factor);
                }
            }
            let mut release = 0.0;
            for (i, ((a, b), n)) in reaction
                .reactants
                .iter()
                .zip(&reaction.products)
                .zip(&self.nuclei)
                .enumerate()
            {
                consumption[i] += rate * f64::from(*a) * f64::from(n.mass_number);
                production[i] += rate * f64::from(*b) * f64::from(n.mass_number);
                release += (f64::from(*b) - f64::from(*a)) * n.binding_energy;
            }
            let energy = rate * release * 1000.0 * AVOGADRO;
            let escape = energy.max(0.0) * reaction.neutrino_fraction;
            heat += energy - escape;
            neutrinos += escape;
        }
        if ![heat, neutrinos].into_iter().all(f64::is_finite)
            || !consumption.iter().chain(&production).all(|r| r.is_finite())
        {
            return Err(Error::NumericalOverflow);
        }
        let mass_fraction_rates = production
            .into_iter()
            .zip(&consumption)
            .map(|(p, c)| p - c)
            .collect();
        Ok((
            Rates {
                mass_fraction_rates,
                deposited_power: heat,
                escaped_neutrino_power: neutrinos,
                fit_evaluations: evaluations,
            },
            consumption,
        ))
    }
}
