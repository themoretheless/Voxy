//! Uniform-temperature grey bodies in SI units, with constant heat capacity.
//! Backward Euler radiation is positive and stable even for stiff cooling.
use crate::astrophysics::Error;
/// Stefan–Boltzmann constant, W m^-2 K^-4 (rounded CODATA value).
pub const STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalBody {
    /// Total heat capacity in J/K; no phase transitions or structural evolution.
    pub heat_capacity: f64,
    pub temperature: f64,
    pub area: f64,
    pub emissivity: f64,
    /// Cumulative net radiated energy; negative when the bath heats the body.
    pub radiated_energy: f64,
}
impl ThermalBody {
    fn validate(self) -> Result<(), Error> {
        if ![
            self.heat_capacity,
            self.temperature,
            self.area,
            self.emissivity,
            self.radiated_energy,
        ]
        .into_iter()
        .all(f64::is_finite)
            || self.heat_capacity <= 0.0
            || self.temperature < 0.0
            || self.area < 0.0
            || !(0.0..=1.0).contains(&self.emissivity)
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
    /// Net luminosity to an isotropic thermal bath, in W.
    /// # Errors
    /// Rejects invalid body/bath parameters and floating-point overflow.
    pub fn luminosity(self, bath_temperature: f64) -> Result<f64, Error> {
        self.validate()?;
        if !bath_temperature.is_finite() || bath_temperature < 0.0 {
            return Err(Error::InvalidInput);
        }
        let value = self.area
            * self.emissivity
            * STEFAN_BOLTZMANN
            * (self.temperature.powi(4) - bath_temperature.powi(4));
        if value.is_finite() {
            Ok(value)
        } else {
            Err(Error::NumericalOverflow)
        }
    }
    /// Deposit energy (J), then cool for dt (s). All errors leave state unchanged.
    /// Net radiation plus internal energy equals initial energy plus deposited heat.
    /// # Errors
    /// Rejects invalid inputs or numerical overflow without changing the body.
    pub fn step(
        &mut self,
        deposited_heat: f64,
        bath_temperature: f64,
        dt: f64,
    ) -> Result<(), Error> {
        self.validate()?;
        if !deposited_heat.is_finite()
            || deposited_heat < 0.0
            || !dt.is_finite()
            || dt < 0.0
            || !bath_temperature.is_finite()
            || bath_temperature < 0.0
        {
            return Err(Error::InvalidInput);
        }
        let heated = self.temperature + deposited_heat / self.heat_capacity;
        let coefficient = dt * self.area * self.emissivity * STEFAN_BOLTZMANN / self.heat_capacity;
        if !heated.is_finite() || !coefficient.is_finite() || !bath_temperature.powi(4).is_finite()
        {
            return Err(Error::NumericalOverflow);
        }
        let mut low = heated.min(bath_temperature);
        let mut high = heated.max(bath_temperature);
        if coefficient == 0.0 {
            low = heated;
            high = heated;
        }
        for _ in 0..128 {
            let mid = low + (high - low) * 0.5;
            let residual = mid - heated + coefficient * (mid.powi(4) - bath_temperature.powi(4));
            if residual.is_nan() {
                return Err(Error::NumericalOverflow);
            }
            if residual > 0.0 {
                high = mid;
            } else {
                low = mid;
            }
        }
        let temperature = low + (high - low) * 0.5;
        let radiated_energy = self.radiated_energy + self.heat_capacity * (heated - temperature);
        if !radiated_energy.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        self.temperature = temperature;
        self.radiated_energy = radiated_energy;
        Ok(())
    }
}
