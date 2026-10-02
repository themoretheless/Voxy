//! Explicit bounded temperature/moisture property calibration.
use super::{Calibration, Properties};
#[derive(Clone, Copy, Debug)]
pub struct ThermalCalibration {
    cold_k: f64,
    hot_k: f64,
    cold: Calibration,
    hot: Calibration,
}
impl ThermalCalibration {
    /// Linear temperature interpolation of dry/saturated property endpoints.
    /// This empirical choice needs measured data; there is no extrapolation,
    /// phase transition, thermal expansion or automatic material weakening law.
    pub fn new(
        cold_k: f64,
        hot_k: f64,
        cold: Calibration,
        hot: Calibration,
    ) -> Result<Self, &'static str> {
        if !cold_k.is_finite()
            || !hot_k.is_finite()
            || cold_k <= 0.
            || hot_k <= cold_k
            || !(hot_k - cold_k).is_finite()
        {
            return Err("invalid material temperature calibration domain");
        }
        Ok(Self {
            cold_k,
            hot_k,
            cold,
            hot,
        })
    }
    pub fn at_temperature(self, temperature_k: f64) -> Result<Calibration, &'static str> {
        if !temperature_k.is_finite() || temperature_k < self.cold_k || temperature_k > self.hot_k {
            return Err("material temperature outside calibration domain");
        }
        let fraction = (temperature_k - self.cold_k) / (self.hot_k - self.cold_k);
        let mix = |a: Properties, b: Properties| {
            let lerp = |x: f64, y: f64| (1. - fraction) * x + fraction * y;
            Properties {
                young_pa: lerp(a.young_pa, b.young_pa),
                poisson: lerp(a.poisson, b.poisson),
                yield_pa: lerp(a.yield_pa, b.yield_pa),
                hardening_pa: lerp(a.hardening_pa, b.hardening_pa),
                hardness_pa: lerp(a.hardness_pa, b.hardness_pa),
                wear_coefficient: lerp(a.wear_coefficient, b.wear_coefficient),
            }
        };
        Calibration::new(
            mix(self.cold.at(0.)?, self.hot.at(0.)?),
            mix(self.cold.at(1.)?, self.hot.at(1.)?),
        )
    }
}
