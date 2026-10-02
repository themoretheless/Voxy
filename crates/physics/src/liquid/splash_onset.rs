//! Configurable empirical dry-wall splash onset; not a wet-film closure.
use super::positive;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactNumbers {
    pub reynolds: f64,
    pub weber: f64,
    pub ohnesorge: f64,
    /// sqrt(We) * Re^(1/4), using incident normal speed.
    pub splash_parameter: f64,
}
impl ImpactNumbers {
    /// SI inputs: kg/m³, Pa s, N/m, m, m/s. Zero normal speed is valid.
    pub fn new(
        density: f64,
        viscosity: f64,
        surface_tension: f64,
        diameter: f64,
        normal_speed: f64,
    ) -> Result<Self, &'static str> {
        if [density, viscosity, surface_tension, diameter]
            .iter()
            .any(|v| !positive(*v))
            || !normal_speed.is_finite()
            || normal_speed < 0.0
        {
            return Err("invalid splash material or incident speed");
        }
        let reynolds = density * normal_speed * diameter / viscosity;
        let weber = density * normal_speed * normal_speed * diameter / surface_tension;
        let ohnesorge = viscosity / (density * surface_tension * diameter).sqrt();
        let splash_parameter = weber.sqrt() * reynolds.sqrt().sqrt();
        if [reynolds, weber, ohnesorge, splash_parameter]
            .iter()
            .any(|v| !v.is_finite())
            || ohnesorge <= 0.0
        {
            return Err("splash dimensionless number overflow");
        }
        Ok(Self {
            reynolds,
            weber,
            ohnesorge,
            splash_parameter,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DryWallSplashOnset {
    /// Must be calibrated for the target wall, environment and fluid regime.
    /// No universal default is supplied.
    pub critical_parameter: f64,
}
impl DryWallSplashOnset {
    pub fn permits(self, numbers: ImpactNumbers) -> Result<bool, &'static str> {
        if !positive(self.critical_parameter)
            || [
                numbers.reynolds,
                numbers.weber,
                numbers.ohnesorge,
                numbers.splash_parameter,
            ]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err("invalid splash onset controls");
        }
        Ok(numbers.splash_parameter > self.critical_parameter)
    }
}
