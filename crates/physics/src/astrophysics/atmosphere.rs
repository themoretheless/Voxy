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

/// Evaluates the Jeans dimensionless thermal escape parameter $\lambda_J = \frac{G M m}{k_B T r}$
/// at exobase radius `r_exobase` for a molecular species of mass `particle_mass_kg`.
pub fn jeans_escape_parameter_lambda(
    planet_mass_kg: f64,
    exobase_radius_m: f64,
    particle_mass_kg: f64,
    exosphere_temp_k: f64,
    g: f64,
    k_b: f64,
) -> Option<f64> {
    if ![
        planet_mass_kg,
        exobase_radius_m,
        particle_mass_kg,
        exosphere_temp_k,
        g,
        k_b,
    ]
    .into_iter()
    .all(f64::is_finite)
        || planet_mass_kg <= 0.0
        || exobase_radius_m <= 0.0
        || particle_mass_kg <= 0.0
        || exosphere_temp_k <= 0.0
        || g <= 0.0
        || k_b <= 0.0
    {
        return None;
    }
    let lambda = (g * planet_mass_kg * particle_mass_kg) / (k_b * exosphere_temp_k * exobase_radius_m);
    lambda.is_finite().then_some(lambda)
}

#[cfg(test)]
mod jeans_tests {
    use super::*;

    #[test]
    fn jeans_escape_parameter_is_lower_for_hydrogen_than_nitrogen() {
        let h2 = jeans_escape_parameter_lambda(5.972e24, 6.871e6, 3.34e-27, 1000.0, 6.6743e-11, 1.380_649e-23).unwrap();
        let n2 = jeans_escape_parameter_lambda(5.972e24, 6.871e6, 4.65e-26, 1000.0, 6.6743e-11, 1.380_649e-23).unwrap();
        assert!(n2 > h2 && h2 > 0.0);
    }
}
