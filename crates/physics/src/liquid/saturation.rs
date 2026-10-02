//! Prescribed local-pressure phase equilibrium with constant-latent-heat saturation.
use super::{Error, Liquid, positive};

/// Integrated Clausius-Clapeyron approximation for a pure substance.
/// Constant latent enthalpy and ideal vapor; use only within declared temperature bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SaturationCurve {
    pub reference_temperature: f64,
    pub reference_pressure: f64,
    /// Vaporization enthalpy in J/kg, matching the material phase model.
    pub latent_heat: f64,
    /// Specific vapor gas constant in J/(kg K).
    pub vapor_gas_constant: f64,
    pub min_temperature: f64,
    pub max_temperature: f64,
}
impl SaturationCurve {
    fn validate(self) -> Result<(), Error> {
        if [
            self.reference_temperature,
            self.reference_pressure,
            self.latent_heat,
            self.vapor_gas_constant,
            self.min_temperature,
            self.max_temperature,
        ]
        .iter()
        .any(|v| !positive(*v))
            || self.min_temperature >= self.max_temperature
            || self.reference_temperature < self.min_temperature
            || self.reference_temperature > self.max_temperature
        {
            return Err(Error::InvalidPhaseChange);
        }
        Ok(())
    }
    /// Equilibrium vapor pressure in absolute pascals.
    /// # Errors
    /// Invalid model, temperature outside its domain, or unrepresentable pressure.
    pub fn pressure(self, temperature: f64) -> Result<f64, Error> {
        self.validate()?;
        if !temperature.is_finite()
            || temperature < self.min_temperature
            || temperature > self.max_temperature
        {
            return Err(Error::InvalidPhaseChange);
        }
        let logarithm = self.reference_pressure.ln()
            + self.latent_heat / self.vapor_gas_constant
                * (1.0 / self.reference_temperature - 1.0 / temperature);
        let pressure = logarithm.exp();
        if !positive(pressure) {
            return Err(Error::NumericalFailure);
        }
        Ok(pressure)
    }
    /// Saturation temperature in kelvin for an absolute pressure.
    /// # Errors
    /// Invalid model/pressure, no positive temperature, or temperature outside the domain.
    pub fn temperature(self, pressure: f64) -> Result<f64, Error> {
        self.validate()?;
        if !positive(pressure) {
            return Err(Error::InvalidPhaseChange);
        }
        let inverse = 1.0 / self.reference_temperature
            - self.vapor_gas_constant / self.latent_heat
                * (pressure.ln() - self.reference_pressure.ln());
        if !positive(inverse) {
            return Err(Error::InvalidPhaseChange);
        }
        let temperature = 1.0 / inverse;
        let tolerance = 32.0 * f64::EPSILON * self.max_temperature;
        if !temperature.is_finite()
            || temperature < self.min_temperature - tolerance
            || temperature > self.max_temperature + tolerance
        {
            return Err(Error::InvalidPhaseChange);
        }
        Ok(temperature.clamp(self.min_temperature, self.max_temperature))
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SaturationState {
    pub curves: Vec<Option<SaturationCurve>>,
    pub pressures: Vec<f64>,
    pub temperatures: Vec<Option<f64>>,
}
impl Liquid {
    /// Configures prescribed absolute pressure per particle for optional saturation curves.
    /// Updates temperature and phase fraction while preserving each particle's stored
    /// thermal/latent energy. Pressure work is not included in this equilibrium remap.
    /// Curves are material-indexed. `None` entries retain their fixed phase temperature.
    /// # Errors
    /// Missing phase model, mismatched counts/latent heat, invalid pressure/domain, or
    /// invalid resulting fields/properties. Atomic. All-None curves disable overrides.
    pub fn configure_saturation(
        &mut self,
        curves: Vec<Option<SaturationCurve>>,
        pressures: Vec<f64>,
    ) -> Result<(), Error> {
        if curves.len() != self.materials.len()
            || pressures.len() != self.particles.len()
            || pressures.iter().any(|p| !positive(*p))
        {
            return Err(Error::InvalidPhaseChange);
        }
        let mut fields = self.transport.clone().ok_or(Error::InvalidPhaseChange)?;
        let phase = fields.phase.as_ref().ok_or(Error::InvalidPhaseChange)?;
        for (curve, model) in curves.iter().zip(&phase.models) {
            if let Some(curve) = curve {
                curve.validate()?;
                let model = model.ok_or(Error::InvalidPhaseChange)?;
                if (curve.latent_heat - model.latent_heat).abs()
                    > 64.0 * f64::EPSILON * model.latent_heat
                {
                    return Err(Error::InvalidPhaseChange);
                }
            }
        }
        let energies = self
            .particles
            .iter()
            .enumerate()
            .map(|(i, p)| fields.energy(p, &fields.fields[i], i))
            .collect::<Result<Vec<_>, _>>()?;
        let temperatures = self
            .particles
            .iter()
            .zip(&pressures)
            .map(|(p, pressure)| {
                curves[p.material]
                    .map(|curve| curve.temperature(*pressure))
                    .transpose()
            })
            .collect::<Result<Vec<_>, _>>()?;
        fields
            .phase
            .as_mut()
            .ok_or(Error::InvalidPhaseChange)?
            .saturation = curves
            .iter()
            .any(Option::is_some)
            .then_some(SaturationState {
                curves,
                pressures,
                temperatures,
            });
        for (i, (p, energy)) in self.particles.iter().zip(energies).enumerate() {
            fields.set_energy(i, p, energy)?;
        }
        self.evaluate_materials(&self.particles, Some(&fields))?;
        self.transport = Some(fields);
        Ok(())
    }
    /// Changes prescribed pressures using the existing material saturation curves.
    /// # Errors
    /// Missing saturation configuration or errors described by `configure_saturation`.
    pub fn set_saturation_pressures(&mut self, pressures: Vec<f64>) -> Result<(), Error> {
        let curves = self
            .transport
            .as_ref()
            .and_then(|t| t.phase.as_ref())
            .and_then(|p| p.saturation.as_ref())
            .ok_or(Error::InvalidPhaseChange)?
            .curves
            .clone();
        self.configure_saturation(curves, pressures)
    }
    #[must_use]
    pub fn saturation_pressures(&self) -> Option<&[f64]> {
        Some(
            &self
                .transport
                .as_ref()?
                .phase
                .as_ref()?
                .saturation
                .as_ref()?
                .pressures,
        )
    }
}
