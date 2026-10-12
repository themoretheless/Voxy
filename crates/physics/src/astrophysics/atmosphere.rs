//! Plane-parallel static Eddington-grey atmosphere; constant g and opacity.
//! Total pressure includes LTE radiation. No time-dependent boundary coupling.
use crate::astrophysics_eos::{Mixture, RADIATION_CONSTANT};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Atmosphere {
    pub gravity: f64,
    /// Constant grey mass opacity m²/kg.
    pub opacity: f64,
    pub effective_temperature: f64,
    /// Gas pressure at optical depth zero, Pa; radiation is added separately.
    pub top_gas_pressure: f64,
    pub mixture: Mixture,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub temperature: f64,
    pub gas_pressure: f64,
    pub radiation_pressure: f64,
    pub density: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    NoStaticAtmosphere,
    NumericalOverflow,
}
impl Atmosphere {
    /// `T⁴=(3/4)Teff⁴(tau+2/3)`; `dP_total/dtau=g/kappa`.
    /// Zero top gas pressure allows a vacuum density at tau=0.
    /// # Errors
    /// Invalid constants/depth, nonpositive gas-pressure gradient, or overflow.
    pub fn at(self, depth: f64) -> Result<State, Error> {
        if ![
            self.gravity,
            self.opacity,
            self.effective_temperature,
            self.top_gas_pressure,
            depth,
        ]
        .into_iter()
        .all(f64::is_finite)
            || self.gravity <= 0.0
            || self.opacity <= 0.0
            || self.effective_temperature <= 0.0
            || self.top_gas_pressure < 0.0
            || depth < 0.0
        {
            return Err(Error::InvalidInput);
        }
        let radiation_gradient = RADIATION_CONSTANT * self.effective_temperature.powi(4) / 4.0;
        let total_gradient = self.gravity / self.opacity;
        if !radiation_gradient.is_finite() || !total_gradient.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        if total_gradient <= radiation_gradient {
            return Err(Error::NoStaticAtmosphere);
        }
        let temperature = self.effective_temperature * (0.75 * (depth + 2.0 / 3.0)).powf(0.25);
        let gas_pressure = self.top_gas_pressure + (total_gradient - radiation_gradient) * depth;
        let radiation_pressure = radiation_gradient * (depth + 2.0 / 3.0);
        let density = gas_pressure / self.mixture.specific_gas_constant() / temperature;
        if ![temperature, gas_pressure, radiation_pressure, density]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(Error::NumericalOverflow);
        }
        Ok(State {
            temperature,
            gas_pressure,
            radiation_pressure,
            density,
        })
    }
}

impl Atmosphere {
    /// Convert a nonvacuum atmosphere depth into the Newtonian Euler gas state.
    /// Velocity is radial m/s; total energy includes gas, trapped LTE radiation
    /// and kinetic energy once. Does not install a hydrodynamic boundary.
    /// # Errors
    /// Invalid atmosphere, vacuum state, nonfinite velocity or overflow.
    pub fn gas_cell(
        self,
        depth: f64,
        velocity: f64,
    ) -> Result<crate::astrophysics_gas::Cell, Error> {
        if !velocity.is_finite() {
            return Err(Error::InvalidInput);
        }
        let state = self.at(depth)?;
        if state.density <= 0.0 {
            return Err(Error::InvalidInput);
        }
        let thermal = self
            .mixture
            .at(state.density, state.temperature)
            .map_err(|_| Error::NumericalOverflow)?;
        if thermal.sound_speed_squared >= crate::astrophysics_eos::LIGHT_SPEED.powi(2) {
            return Err(Error::InvalidInput);
        }
        let cell = crate::astrophysics_gas::Cell {
            density: state.density,
            momentum: state.density * velocity,
            energy: thermal.internal_energy_density + 0.5 * state.density * velocity * velocity,
        };
        if ![cell.density, cell.momentum, cell.energy]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(Error::NumericalOverflow);
        }
        Ok(cell)
    }
}
