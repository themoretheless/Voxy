//! Fully ionized, nondegenerate monatomic gas plus trapped LTE radiation in SI.
//! Nuclear masses are approximated by `A*m_u`; no ionization, degeneracy or pairs.
use crate::astrophysics_thermal::STEFAN_BOLTZMANN;
pub const BOLTZMANN: f64 = 1.380_649e-23;
pub const ATOMIC_MASS: f64 = 1.660_539_068_92e-27;
pub const LIGHT_SPEED: f64 = 299_792_458.0;
pub const RADIATION_CONSTANT: f64 = 4.0 * STEFAN_BOLTZMANN / LIGHT_SPEED;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Species {
    pub mass_fraction: f64,
    pub mass_number: u16,
    pub nuclear_charge: u16,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mixture {
    gas_constant: f64,
    inverse_mu: f64,
    inverse_electron_mu: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub gas_pressure: f64,
    pub radiation_pressure: f64,
    pub internal_energy_density: f64,
    /// J/kg/K at fixed density.
    pub specific_heat_cv: f64,
    /// Newtonian adiabatic derivative dP/drho|entropy, including trapped radiation.
    /// Radiation inertia is neglected; inappropriate near relativistic sound speeds.
    pub sound_speed_squared: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    NumericalOverflow,
}
impl Mixture {
    /// Fully ionized species, inverse mu=sum X*(1+Z)/A.
    /// Fractions must sum to one within 1e-12 and are then normalized.
    /// # Errors
    /// Empty, invalid species, negative fractions or nonunit composition sum.
    pub fn new(species: &[Species]) -> Result<Self, Error> {
        if species.is_empty() {
            return Err(Error::InvalidInput);
        }
        let mut total = 0.0;
        let mut inverse_mu = 0.0;
        let mut inverse_electron_mu = 0.0;
        for s in species {
            if !s.mass_fraction.is_finite()
                || s.mass_fraction < 0.0
                || s.mass_number == 0
                || s.nuclear_charge == 0
                || s.nuclear_charge > s.mass_number
            {
                return Err(Error::InvalidInput);
            }
            total += s.mass_fraction;
            inverse_mu +=
                s.mass_fraction * (1.0 + f64::from(s.nuclear_charge)) / f64::from(s.mass_number);
            inverse_electron_mu +=
                s.mass_fraction * f64::from(s.nuclear_charge) / f64::from(s.mass_number);
        }
        if !total.is_finite() || (total - 1.0).abs() > 1e-12 {
            return Err(Error::InvalidInput);
        }
        inverse_mu /= total;
        inverse_electron_mu /= total;
        Ok(Self {
            gas_constant: BOLTZMANN / ATOMIC_MASS * inverse_mu,
            inverse_mu,
            inverse_electron_mu,
        })
    }
    #[must_use]
    pub fn mean_molecular_weight(self) -> f64 {
        1.0 / self.inverse_mu
    }
    #[must_use]
    pub fn electron_molecular_weight(self) -> f64 {
        1.0 / self.inverse_electron_mu
    }
    #[must_use]
    pub fn specific_gas_constant(self) -> f64 {
        self.gas_constant
    }
    /// Evaluate gas+radiation thermodynamics at density kg/m³ and temperature K.
    /// # Errors
    /// Nonpositive density, negative temperature, nonfinite input or overflow.
    pub fn at(self, density: f64, temperature: f64) -> Result<State, Error> {
        if !density.is_finite() || density <= 0.0 || !temperature.is_finite() || temperature < 0.0 {
            return Err(Error::InvalidInput);
        }
        let gas_pressure = density * self.gas_constant * temperature;
        let radiation_energy = RADIATION_CONSTANT * temperature.powi(4);
        let radiation_pressure = radiation_energy / 3.0;
        let internal_energy_density = 1.5 * gas_pressure + radiation_energy;
        let specific_heat_cv =
            1.5 * self.gas_constant + 4.0 * RADIATION_CONSTANT * temperature.powi(3) / density;
        let pressure_temperature =
            density * self.gas_constant + 4.0 * RADIATION_CONSTANT * temperature.powi(3) / 3.0;
        let sound_speed_squared = self.gas_constant * temperature
            + temperature * (pressure_temperature / density).powi(2) / specific_heat_cv;
        if ![
            gas_pressure,
            radiation_pressure,
            internal_energy_density,
            specific_heat_cv,
            sound_speed_squared,
        ]
        .into_iter()
        .all(f64::is_finite)
        {
            return Err(Error::NumericalOverflow);
        }
        Ok(State {
            gas_pressure,
            radiation_pressure,
            internal_energy_density,
            specific_heat_cv,
            sound_speed_squared,
        })
    }
    /// Invert total internal energy density, including trapped radiation.
    /// Monotonic bisection brackets the root using gas and radiation limits.
    /// # Errors
    /// Invalid density/energy or overflowing thermodynamics.
    pub fn temperature(self, density: f64, internal_energy_density: f64) -> Result<f64, Error> {
        if !density.is_finite()
            || density <= 0.0
            || !internal_energy_density.is_finite()
            || internal_energy_density < 0.0
        {
            return Err(Error::InvalidInput);
        }
        if internal_energy_density == 0.0 {
            return Ok(0.0);
        }
        let mut low: f64 = 0.0;
        let mut high = (internal_energy_density / (1.5 * density * self.gas_constant))
            .min((internal_energy_density / RADIATION_CONSTANT).powf(0.25));
        if !high.is_finite() || high <= 0.0 {
            return Err(Error::NumericalOverflow);
        }
        for _ in 0..128 {
            let mid = low.midpoint(high);
            if self.at(density, mid)?.internal_energy_density > internal_energy_density {
                high = mid;
            } else {
                low = mid;
            }
        }
        Ok(low.midpoint(high))
    }
}

impl Mixture {
    /// Invert total gas plus trapped-radiation pressure in pascals at fixed
    /// density kg/m³. Uses positive monotonic bisection between gas/radiation
    /// limiting temperatures; no gas-only pressure approximation.
    /// # Errors
    /// Invalid density/pressure or overflowing thermodynamics.
    pub fn temperature_from_pressure(self, density: f64, pressure: f64) -> Result<f64, Error> {
        if !density.is_finite() || density <= 0.0 || !pressure.is_finite() || pressure < 0.0 {
            return Err(Error::InvalidInput);
        }
        if pressure == 0.0 {
            return Ok(0.0);
        }
        // Logs avoid overflowing rho*R and 3*pressure in the bracket.
        let gas_limit = (pressure.ln() - density.ln() - self.gas_constant.ln()).exp();
        let radiation_limit = ((pressure.ln() + 3_f64.ln() - RADIATION_CONSTANT.ln()) / 4.0).exp();
        let mut high = gas_limit.min(radiation_limit);
        if !high.is_finite() || high <= 0.0 {
            return Err(Error::NumericalOverflow);
        }
        let mut low: f64 = 0.0;
        for _ in 0..128 {
            let mid = low.midpoint(high);
            let state = self.at(density, mid)?;
            if state.gas_pressure + state.radiation_pressure > pressure {
                high = mid;
            } else {
                low = mid;
            }
        }
        Ok(low.midpoint(high))
    }
}
