//! IAPWS R6-95(2018), attributed to International Association for the Properties
//! of Water and Steam. https://iapws.org/technical-guidance/release/IAPWS-95
//! Homogeneous-state evaluation; coexistence/Maxwell phase selection is separate.
use super::Error;
#[path = "water_helmholtz_coefficients.rs"]
mod coefficients;
#[path = "water_coexistence.rs"]
mod coexistence;
pub(super) use coexistence::solve_coexistence;
pub use coexistence::{WaterCoexistence, water_coexistence};

// Two-variable second-order forward differentiation avoids independent,
// potentially inconsistent formulas for each thermodynamic derivative.
#[derive(Clone, Copy)]
struct Jet {
    v: f64,
    x: f64,
    y: f64,
    xx: f64,
    xy: f64,
    yy: f64,
}
impl Jet {
    fn constant(v: f64) -> Self {
        Self {
            v,
            x: 0.,
            y: 0.,
            xx: 0.,
            xy: 0.,
            yy: 0.,
        }
    }
    fn variable(v: f64, x: bool) -> Self {
        let mut j = Self::constant(v);
        if x {
            j.x = 1.;
        } else {
            j.y = 1.;
        }
        j
    }
    fn add(self, b: Self) -> Self {
        Self {
            v: self.v + b.v,
            x: self.x + b.x,
            y: self.y + b.y,
            xx: self.xx + b.xx,
            xy: self.xy + b.xy,
            yy: self.yy + b.yy,
        }
    }
    fn scale(self, b: f64) -> Self {
        Self {
            v: self.v * b,
            x: self.x * b,
            y: self.y * b,
            xx: self.xx * b,
            xy: self.xy * b,
            yy: self.yy * b,
        }
    }
    fn mul(self, b: Self) -> Self {
        Self {
            v: self.v * b.v,
            x: self.x * b.v + self.v * b.x,
            y: self.y * b.v + self.v * b.y,
            xx: self.xx * b.v + 2. * self.x * b.x + self.v * b.xx,
            xy: self.xy * b.v + self.x * b.y + self.y * b.x + self.v * b.xy,
            yy: self.yy * b.v + 2. * self.y * b.y + self.v * b.yy,
        }
    }
    fn compose(self, v: f64, d: f64, dd: f64) -> Self {
        // At delta=1, powers of (delta-1)^2 may have singular intermediate
        // second derivatives but finite composed derivatives. Zero factors
        // must not generate 0*infinity NaNs.
        let term = |a: f64, b: f64| if a == 0. || b == 0. { 0. } else { dd * a * b };
        Self {
            v,
            x: d * self.x,
            y: d * self.y,
            xx: d * self.xx + term(self.x, self.x),
            xy: d * self.xy + term(self.x, self.y),
            yy: d * self.yy + term(self.y, self.y),
        }
    }
    fn pow(self, p: f64) -> Self {
        if p == 0. {
            return Self::constant(1.);
        }
        if p == 1. {
            return self;
        }
        self.compose(
            self.v.powf(p),
            p * self.v.powf(p - 1.),
            p * (p - 1.) * self.v.powf(p - 2.),
        )
    }
    fn exp(self) -> Self {
        let v = self.v.exp();
        self.compose(v, v, v)
    }
    fn ln(self) -> Self {
        self.compose(self.v.ln(), 1. / self.v, -1. / self.v.powi(2))
    }
}
/// Compensate cancellation independently in the potential and all derivatives.
/// Cold liquid pressure is a small difference of much larger residual terms.
struct JetSum {
    sum: Jet,
    correction: Jet,
}
impl JetSum {
    fn new() -> Self {
        Self {
            sum: Jet::constant(0.),
            correction: Jet::constant(0.),
        }
    }
    fn add(&mut self, value: Jet) {
        fn accumulate(sum: &mut f64, correction: &mut f64, value: f64) {
            let next = *sum + value;
            *correction += if sum.abs() >= value.abs() {
                (*sum - next) + value
            } else {
                (value - next) + *sum
            };
            *sum = next;
        }
        accumulate(&mut self.sum.v, &mut self.correction.v, value.v);
        accumulate(&mut self.sum.x, &mut self.correction.x, value.x);
        accumulate(&mut self.sum.y, &mut self.correction.y, value.y);
        accumulate(&mut self.sum.xx, &mut self.correction.xx, value.xx);
        accumulate(&mut self.sum.xy, &mut self.correction.xy, value.xy);
        accumulate(&mut self.sum.yy, &mut self.correction.yy, value.yy);
    }
    fn finish(self) -> Jet {
        self.sum.add(self.correction)
    }
}
fn potential(delta: f64, tau: f64) -> (Jet, Jet) {
    let d = Jet::variable(delta, true);
    let t = Jet::variable(tau, false);
    let mut ideal = d
        .ln()
        .add(Jet::constant(-8.3204464837497))
        .add(t.scale(6.6832105275932))
        .add(t.ln().scale(3.00632));
    for (n, g) in [
        (0.012436, 1.28728967),
        (0.97315, 3.53734222),
        (1.27950, 7.74073708),
        (0.96956, 9.24437796),
        (0.24873, 27.5075105),
    ] {
        ideal = ideal.add(
            Jet::constant(1.)
                .add(t.scale(-g).exp().scale(-1.))
                .ln()
                .scale(n),
        );
    }
    let mut residual = JetSum::new();
    for &(n, di, ti, ci) in &coefficients::TERMS {
        let mut term = d.pow(di).mul(t.pow(ti)).scale(n);
        if ci > 0. {
            term = term.mul(d.pow(ci).scale(-1.).exp());
        }
        residual.add(term);
    }
    for (n, ti, beta, gamma) in [
        (-31.306260323435, 0., 150., 1.21),
        (31.546140237781, 1., 150., 1.21),
        (-2521.3154341695, 4., 250., 1.25),
    ] {
        let exponent = d
            .add(Jet::constant(-1.))
            .pow(2.)
            .scale(-20.)
            .add(t.add(Jet::constant(-gamma)).pow(2.).scale(-beta));
        residual.add(d.pow(3.).mul(t.pow(ti)).mul(exponent.exp()).scale(n));
    }
    let distance = d.add(Jet::constant(-1.)).pow(2.);
    for (n, b, c, capd) in [
        (-0.14874640856724, 0.85, 28., 700.),
        (0.31806110878444, 0.95, 32., 800.),
    ] {
        let theta = Jet::constant(1.)
            .add(t.scale(-1.))
            .add(distance.pow(1. / 0.6).scale(0.32));
        let big_delta = theta.pow(2.).add(distance.pow(3.5).scale(0.2));
        let psi = distance
            .scale(-c)
            .add(t.add(Jet::constant(-1.)).pow(2.).scale(-capd))
            .exp();
        residual.add(big_delta.pow(b).mul(d).mul(psi).scale(n));
    }
    (ideal, residual.finish())
}

/// Homogeneous ordinary-water state in SI units. Positive local mechanical and
/// thermal stability does not perform global phase/coexistence selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterHomogeneousState {
    pub pressure_pa: f64,
    pub internal_energy_j_per_kg: f64,
    pub enthalpy_j_per_kg: f64,
    pub entropy_j_per_kg_k: f64,
    pub cv_j_per_kg_k: f64,
    pub cp_j_per_kg_k: f64,
    pub sound_speed_m_per_s: f64,
}
/// Recover a homogeneous water temperature from specific internal energy (J/kg)
/// and density (kg/m3) inside a caller-supplied temperature bracket (K).
/// The caller selects the phase branch. Invalid/unstable sampled states or an
/// unbracketed energy are rejected; this does not perform a multiphase flash.
/// Uses the same EOS and cv as the forward evaluator, with safeguarded Newton
/// steps. Returned state comes from the final temperature, not a separate fit.
pub fn water_homogeneous_from_energy(
    density: f64,
    energy: f64,
    temperature_interval: [f64; 2],
) -> Result<(f64, WaterHomogeneousState), Error> {
    let [mut low, mut high] = temperature_interval;
    if !energy.is_finite() || !low.is_finite() || !high.is_finite() || low >= high {
        return Err(Error::InvalidPhaseChange);
    }
    let lower = water_homogeneous_state(low, density)?;
    let upper = water_homogeneous_state(high, density)?;
    if lower.internal_energy_j_per_kg > energy || upper.internal_energy_j_per_kg < energy {
        return Err(Error::InvalidPhaseChange);
    }
    if lower.internal_energy_j_per_kg == energy {
        return Ok((low, lower));
    }
    if upper.internal_energy_j_per_kg == energy {
        return Ok((high, upper));
    }
    let mut temperature = low + 0.5 * (high - low);
    for _ in 0..80 {
        let state = water_homogeneous_state(temperature, density)?;
        let residual = state.internal_energy_j_per_kg - energy;
        let tolerance = 1e-7 + 1e-12 * energy.abs();
        if residual.abs() <= tolerance {
            return Ok((temperature, state));
        }
        if residual < 0. {
            low = temperature;
        } else {
            high = temperature;
        }
        let newton = temperature - residual / state.cv_j_per_kg_k;
        let midpoint = low + 0.5 * (high - low);
        if midpoint == low || midpoint == high {
            return Err(Error::NumericalFailure);
        }
        temperature = if newton > low && newton < high {
            newton
        } else {
            midpoint
        };
    }
    Err(Error::NumericalFailure)
}
/// Evaluate IAPWS-95 for a homogeneous fluid at supplied T (K) and density
/// (kg/m3). Rejects nonpositive pressure/compressibility/cv, pressures over
/// 1 GPa, and T outside 273.16–1273 K. No melting-curve or Maxwell selection
/// is performed; callers must qualify the phase. Exact critical singularity
/// is unsupported. This is not yet connected to particle/film transport.
pub fn water_homogeneous_state(
    temperature: f64,
    density: f64,
) -> Result<WaterHomogeneousState, Error> {
    evaluate_homogeneous(temperature, density, false)
}

/// Homogeneous EOS state and pressure response from the same potential pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterHomogeneousResponse {
    pub state: WaterHomogeneousState,
    /// (dp/d rho)_T, in Pa m³/kg; positive for locally stable admission.
    pub pressure_density_pa_m3_per_kg: f64,
    /// (dp/d T)_rho, in Pa/K. May be negative in anomalous-density water.
    pub pressure_temperature_pa_per_k: f64,
}

/// Uses the homogeneous-state domain and stability admission. This does not
/// supply equilibrium two-phase acoustics or select a globally stable phase.
pub fn water_homogeneous_response(
    temperature: f64,
    density: f64,
) -> Result<WaterHomogeneousResponse, Error> {
    evaluate_response(temperature, density, false)
}
// Newton trial states can have slightly negative liquid pressures because the
// initial SR1 density is approximate. Public state admission still rejects them.
fn evaluate_homogeneous(
    temperature: f64,
    density: f64,
    allow_negative_pressure: bool,
) -> Result<WaterHomogeneousState, Error> {
    evaluate_response(temperature, density, allow_negative_pressure).map(|r| r.state)
}

fn evaluate_response(
    temperature: f64,
    density: f64,
    allow_negative_pressure: bool,
) -> Result<WaterHomogeneousResponse, Error> {
    const R: f64 = 461.51805;
    if !temperature.is_finite()
        || !(273.16..=1273.).contains(&temperature)
        || !density.is_finite()
        || density <= 0.
        || (temperature == 647.096 && density == 322.)
    {
        return Err(Error::InvalidPhaseChange);
    }
    let delta = density / 322.;
    let tau = 647.096 / temperature;
    let (o, r) = potential(delta, tau);
    let total = o.add(r);
    let stiffness = 1. + 2. * delta * r.x + delta * delta * r.xx;
    let thermal = 1. + delta * r.x - delta * tau * r.xy;
    let cv = -R * tau * tau * total.yy;
    let cp = cv + R * thermal * thermal / stiffness;
    let sound2 = R * temperature * (stiffness - thermal * thermal / (tau * tau * total.yy));
    let pressure = density * R * temperature * (1. + delta * r.x);
    let result = WaterHomogeneousState {
        pressure_pa: pressure,
        internal_energy_j_per_kg: R * temperature * tau * total.y,
        enthalpy_j_per_kg: R * temperature * (1. + tau * total.y + delta * r.x),
        entropy_j_per_kg_k: R * (tau * total.y - total.v),
        cv_j_per_kg_k: cv,
        cp_j_per_kg_k: cp,
        sound_speed_m_per_s: sound2.sqrt(),
    };
    if [cv, cp, sound2, stiffness]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
        || !pressure.is_finite()
        || (!allow_negative_pressure && pressure <= 0.)
        || pressure.abs() > 1e9
        || !result.internal_energy_j_per_kg.is_finite()
        || !result.enthalpy_j_per_kg.is_finite()
        || !result.entropy_j_per_kg_k.is_finite()
    {
        return Err(Error::NumericalFailure);
    }
    let pressure_density = R * temperature * stiffness;
    let pressure_temperature = density * R * thermal;
    if !pressure_density.is_finite() || !pressure_temperature.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(WaterHomogeneousResponse {
        state: result,
        pressure_density_pa_m3_per_kg: pressure_density,
        pressure_temperature_pa_per_k: pressure_temperature,
    })
}

#[cfg(test)]
mod tests {
    use super::potential;
    #[test]
    fn water_helmholtz_matches_official_table_six_derivatives() {
        // IAPWS R6-95(2018), Table 6, T=500 K and rho=838.025 kg/m3.
        let (ideal, residual) = potential(838.025 / 322., 647.096 / 500.);
        for (name, actual, expected) in [
            ("ideal", ideal.v, 2.04797733),
            ("ideal_delta", ideal.x, 0.384236747),
            ("ideal_delta_delta", ideal.xx, -0.147637878),
            ("ideal_tau", ideal.y, 9.04611106),
            ("ideal_tau_tau", ideal.yy, -1.93249185),
            ("ideal_delta_tau", ideal.xy, 0.),
            ("residual", residual.v, -3.42693206),
            ("residual_delta", residual.x, -0.364366650),
            ("residual_delta_delta", residual.xx, 0.856063701),
            ("residual_tau", residual.y, -5.81403435),
            ("residual_tau_tau", residual.yy, -2.23440737),
            ("residual_delta_tau", residual.xy, -1.12176915),
        ] {
            assert!(
                (actual - expected).abs() < 1e-8,
                "{name}: actual={actual}, expected={expected}"
            );
        }
    }
    #[test]
    fn water_helmholtz_critical_density_axis_has_finite_derivatives_away_from_critical_temperature()
    {
        for tau in [0.8, 1.1, 1.3] {
            let (ideal, residual) = potential(1., tau);
            for p in [ideal, residual] {
                assert!(
                    [p.v, p.x, p.y, p.xx, p.xy, p.yy]
                        .iter()
                        .all(|v| v.is_finite())
                );
            }
            let difference = |eps: f64| {
                let (o1, r1) = potential(1. + eps, tau);
                let (o0, r0) = potential(1. - eps, tau);
                (o1.x + r1.x - o0.x - r0.x) / (2. * eps)
            };
            // Remove leading O(h^2) truncation error: curvature reaches 4.5e5
            // at tau=1.3, so a single h=1e-4 difference is not an accurate oracle.
            let numerical = (4. * difference(5e-5) - difference(1e-4)) / 3.;
            let analytic = ideal.xx + residual.xx;
            assert!(
                (numerical - analytic).abs() < 1e-4,
                "tau={tau}, numerical={numerical}, analytic={analytic}, error={}",
                (numerical - analytic).abs()
            );
        }
    }
}
