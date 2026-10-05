//! IAPWS SR1-86(1992) saturation correlations, equations (1)-(4), (6), (7).
//! https://iapws.org/technical-guidance/release/Supp-sat
//! Saturation properties only: not an off-saturation equation of state.
use super::Error;
const TRIPLE_TEMPERATURE: f64 = 273.16;
const CRITICAL_TEMPERATURE: f64 = 647.096;
const CRITICAL_PRESSURE: f64 = 22.064e6;
const CRITICAL_DENSITY: f64 = 322.;

/// Invert the IAPWS saturation pressure on the liquid/vapor coexistence domain.
/// Absolute pressure in pascals; returns kelvin on ITS-90. Uses a bracketed
/// solve with representable-temperature termination; never extrapolates.
pub fn water_saturation_temperature(pressure: f64) -> Result<f64, Error> {
    let lower_pressure = water_saturation(TRIPLE_TEMPERATURE)?.pressure_pa;
    if !pressure.is_finite() || pressure < lower_pressure || pressure > CRITICAL_PRESSURE {
        return Err(Error::InvalidPhaseChange);
    }
    if pressure == lower_pressure {
        return Ok(TRIPLE_TEMPERATURE);
    }
    if pressure == CRITICAL_PRESSURE {
        return Ok(CRITICAL_TEMPERATURE);
    }
    let mut low = TRIPLE_TEMPERATURE;
    let mut high = CRITICAL_TEMPERATURE;
    for _ in 0..64 {
        let middle = low + 0.5 * (high - low);
        if middle == low || middle == high {
            return Ok(middle);
        }
        let evaluated = water_saturation(middle)?.pressure_pa;
        if evaluated == pressure {
            return Ok(middle);
        }
        if evaluated < pressure {
            low = middle;
        } else {
            high = middle;
        }
    }
    Err(Error::NumericalFailure)
}

/// Ordinary-water coexistence properties in SI units, ITS-90 temperature scale.
/// Enthalpy reference is the IAPWS liquid triple-point internal-energy reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterSaturation {
    pub pressure_pa: f64,
    pub pressure_derivative_pa_per_k: f64,
    pub liquid_density_kg_per_m3: f64,
    pub vapor_density_kg_per_m3: f64,
    pub liquid_enthalpy_j_per_kg: f64,
    pub vapor_enthalpy_j_per_kg: f64,
}
impl WaterSaturation {
    pub fn vaporization_enthalpy_j_per_kg(self) -> f64 {
        self.vapor_enthalpy_j_per_kg - self.liquid_enthalpy_j_per_kg
    }
    pub fn liquid_internal_energy_j_per_kg(self) -> f64 {
        self.liquid_enthalpy_j_per_kg - self.pressure_pa / self.liquid_density_kg_per_m3
    }
    pub fn vapor_internal_energy_j_per_kg(self) -> f64 {
        self.vapor_enthalpy_j_per_kg - self.pressure_pa / self.vapor_density_kg_per_m3
    }
}
/// Pure-water saturation properties between triple and critical points inclusive.
/// No extrapolation to ice, supercritical fluid or unsaturated vapor.
/// These correlations are not interchangeable with the constant-latent energy
/// reference used by the current finite-cell vapor transport integrator.
pub fn water_saturation(temperature: f64) -> Result<WaterSaturation, Error> {
    let tc = CRITICAL_TEMPERATURE;
    if !temperature.is_finite() || !(TRIPLE_TEMPERATURE..=tc).contains(&temperature) {
        return Err(Error::InvalidPhaseChange);
    }
    let theta = temperature / tc;
    let tau = (1. - theta).max(0.);
    let pressure_terms = [
        (-7.85951783, 1.),
        (1.84408259, 1.5),
        (-11.7866497, 3.),
        (22.6807411, 3.5),
        (-15.9618719, 4.),
        (1.80122502, 7.5),
    ];
    let sum = pressure_terms
        .iter()
        .map(|(a, n)| a * tau.powf(*n))
        .sum::<f64>();
    let derivative = pressure_terms
        .iter()
        .map(|(a, n)| a * n * tau.powf(n - 1.))
        .sum::<f64>();
    let pressure = CRITICAL_PRESSURE * (sum / theta).exp();
    let dp_dt = pressure * (-tc * sum / temperature.powi(2) - derivative / temperature);
    let liquid_terms = [
        (1.99274064, 1. / 3.),
        (1.09965342, 2. / 3.),
        (-0.510839303, 5. / 3.),
        (-1.75493479, 16. / 3.),
        (-45.5170352, 43. / 3.),
        (-6.74694450e5, 110. / 3.),
    ];
    let vapor_terms = [
        (-2.03150240, 2. / 6.),
        (-2.68302940, 4. / 6.),
        (-5.38626492, 8. / 6.),
        (-17.2991605, 18. / 6.),
        (-44.7586581, 37. / 6.),
        (-63.9201063, 71. / 6.),
    ];
    let liquid_density = CRITICAL_DENSITY
        * (1.
            + liquid_terms
                .iter()
                .map(|(b, n)| b * tau.powf(*n))
                .sum::<f64>());
    let vapor_density = CRITICAL_DENSITY
        * vapor_terms
            .iter()
            .map(|(c, n)| c * tau.powf(*n))
            .sum::<f64>()
            .exp();
    let alpha = 1000.
        * (-1135.905627715 - 5.65134998e-8 * theta.powf(-19.)
            + 2690.66631 * theta
            + 127.287297 * theta.powf(4.5)
            - 135.003439 * theta.powi(5)
            + 0.981825814 * theta.powf(54.5));
    let result = WaterSaturation {
        pressure_pa: pressure,
        pressure_derivative_pa_per_k: dp_dt,
        liquid_density_kg_per_m3: liquid_density,
        vapor_density_kg_per_m3: vapor_density,
        liquid_enthalpy_j_per_kg: alpha + temperature * dp_dt / liquid_density,
        vapor_enthalpy_j_per_kg: alpha + temperature * dp_dt / vapor_density,
    };
    Ok(result)
}
