//! Original material pressure law and current-density admission.
use super::{Error, positive};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidPressureLaw {
    pub rest_density: f64,
    pub sound_speed: f64,
    pub heat_capacity_ratio: Option<f64>,
    /// Largest admitted tension (negative pressure) magnitude, Pa. `None`
    /// keeps the historic `max(ρ − ρ₀, 0)` clamp.
    pub tensile_floor_pa: Option<f64>,
}
impl LiquidPressureLaw {
    pub fn evaluate_native(self, density: f64) -> Result<f64, Error> {
        if !positive(density) {
            return Err(Error::NumericalFailure);
        }
        if !positive(self.rest_density)
            || !positive(self.sound_speed)
            || self
                .heat_capacity_ratio
                .is_some_and(|x| !x.is_finite() || x <= 1.)
            || self
                .tensile_floor_pa
                .is_some_and(|x| !x.is_finite() || x < 0.)
        {
            return Err(Error::InvalidMaterial);
        }
        let pressure = if let Some(gamma) = self.heat_capacity_ratio {
            density * (self.sound_speed.powi(2) / gamma)
        } else if let Some(floor) = self.tensile_floor_pa {
            (self.sound_speed.powi(2) * (density - self.rest_density)).max(-floor)
        } else {
            self.sound_speed.powi(2) * (density - self.rest_density).max(0.)
        };
        if !pressure.is_finite() || pressure < -self.tensile_floor_pa.unwrap_or(0.) {
            return Err(Error::NumericalFailure);
        }
        Ok(pressure)
    }
}
#[derive(Debug)]
pub struct LiquidPressureOperator<'a> {
    pub densities: &'a [f64],
    pub laws: &'a [LiquidPressureLaw],
}
impl LiquidPressureOperator<'_> {
    pub fn evaluate_native(&self) -> Result<Vec<f64>, Error> {
        if self.densities.len() != self.laws.len() {
            return Err(Error::InvalidConfig);
        }
        self.densities
            .iter()
            .zip(self.laws)
            .map(|(&density, &law)| law.evaluate_native(density))
            .collect()
    }
    pub fn admit(&self, candidate: &[f64], absolute: f64, relative: f64) -> Result<(), Error> {
        if candidate.len() != self.densities.len()
            || !absolute.is_finite()
            || absolute < 0.
            || !relative.is_finite()
            || relative < 0.
        {
            return Err(Error::InvalidConfig);
        }
        for ((&a, b), law) in candidate.iter().zip(self.evaluate_native()?).zip(self.laws) {
            let error = (a - b).abs();
            if !a.is_finite()
                || a < -law.tensile_floor_pa.unwrap_or(0.)
                || !error.is_finite()
                || (error > absolute && (b == 0. || error / b.abs() > relative))
            {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(())
    }
}
