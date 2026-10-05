//! Fixed-temperature liquid/vapor coexistence seeded by the SR1 correlation.
use super::{WaterHomogeneousResponse, WaterHomogeneousState, evaluate_response};
use crate::liquid::{Error, water_saturation};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterCoexistence {
    pub pressure_pa: f64,
    pub liquid_density_kg_per_m3: f64,
    pub vapor_density_kg_per_m3: f64,
    pub liquid: WaterHomogeneousState,
    pub vapor: WaterHomogeneousState,
}
fn evaluate(t: f64, rho: [f64; 2]) -> Result<([WaterHomogeneousResponse; 2], [f64; 2]), Error> {
    if rho.iter().any(|r| !r.is_finite() || *r <= 0.) || rho[0] <= rho[1] {
        return Err(Error::NumericalFailure);
    }
    let responses = [
        evaluate_response(t, rho[0], true)?,
        evaluate_response(t, rho[1], true)?,
    ];
    let states = responses.map(|r| r.state);
    let residual = [
        states[0].pressure_pa - states[1].pressure_pa,
        states[0].enthalpy_j_per_kg
            - t * states[0].entropy_j_per_kg_k
            - states[1].enthalpy_j_per_kg
            + t * states[1].entropy_j_per_kg_k,
    ];
    Ok((responses, residual))
}
/// Solve equality of pressure and chemical potential for liquid/vapor water.
/// Uses damped Newton with relative density increments and the same IAPWS-95 potential
/// for both branches. SR1 provides only initial guesses. Pressure residual bound
/// is 1e-6 Pa + 1e-12 relative; chemical-potential bound is 1e-12 R T J/kg.
/// Tight phase equality is needed for caloric/acoustic derivatives of the
/// published equilibrium state, especially at low saturation pressures.
/// Rejects failed convergence and collapsed equal-density roots. Exact critical
/// point and ice coexistence are unsupported; near-critical conditioning may
/// reject otherwise valid inputs. This is not an arbitrary multiphase flash.
pub fn water_coexistence(t: f64) -> Result<WaterCoexistence, Error> {
    solve_coexistence(t).map(|(state, _)| state)
}

pub(in crate::liquid) fn solve_coexistence(
    t: f64,
) -> Result<(WaterCoexistence, [WaterHomogeneousResponse; 2]), Error> {
    if !t.is_finite() || !(273.16..647.096).contains(&t) {
        return Err(Error::InvalidPhaseChange);
    }
    let seed = water_saturation(t)?;
    let mut rho = [seed.liquid_density_kg_per_m3, seed.vapor_density_kg_per_m3];
    let minimum_separation = (rho[0].ln() - rho[1].ln()) * 1e-3;
    let pressure_scale = seed.pressure_pa;
    let chemical_scale = 461.51805 * t;
    for _ in 0..64 {
        let (responses, f) = evaluate(t, rho)?;
        let states = responses.map(|r| r.state);
        let dp = responses.map(|r| r.pressure_density_pa_m3_per_kg);
        let pressure_bound = 1e-6 + 1e-12 * states[0].pressure_pa.max(states[1].pressure_pa);
        if f[0].abs() <= pressure_bound
            && f[1].abs() <= 1e-12 * chemical_scale
            && rho[0].ln() - rho[1].ln() >= minimum_separation
            && states.iter().all(|s| s.pressure_pa > 0.)
        {
            return Ok((
                WaterCoexistence {
                    pressure_pa: 0.5 * (states[0].pressure_pa + states[1].pressure_pa),
                    liquid_density_kg_per_m3: rho[0],
                    vapor_density_kg_per_m3: rho[1],
                    liquid: states[0],
                    vapor: states[1],
                },
                responses,
            ));
        }
        let a = rho[0] * dp[0] / pressure_scale;
        let b = -rho[1] * dp[1] / pressure_scale;
        let c = dp[0] / chemical_scale;
        let d = -dp[1] / chemical_scale;
        let determinant = a * d - b * c;
        let f0 = f[0] / pressure_scale;
        let f1 = f[1] / chemical_scale;
        let change = [
            (-f0 * d + b * f1) / determinant,
            (-a * f1 + c * f0) / determinant,
        ];
        if change.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        // Measure progress against the publication tolerances. Scaling by seed
        // pressure alone can let an already-admissible pressure residual hide
        // chemical-potential progress at low saturation pressures.
        let norm = (f[0] / pressure_bound)
            .abs()
            .max((f[1] / (1e-12 * chemical_scale)).abs());
        let mut scale = 1.;
        let mut admitted = None;
        for _ in 0..24 {
            // Keep relative conditioning without quantizing rho through log/exp.
            // Damped trials admit positivity. A cold-liquid log ULP can span
            // several density ULPs and prevent strict pressure convergence.
            let next = [
                rho[0] + scale * rho[0] * change[0],
                rho[1] + scale * rho[1] * change[1],
            ];
            if next.iter().all(|r| r.is_finite() && *r > 0.)
                && next[0].ln() - next[1].ln() >= minimum_separation
            {
                if let Ok((trial, residual)) = evaluate(t, next) {
                    let trial_pressure_bound =
                        1e-6 + 1e-12 * trial[0].state.pressure_pa.max(trial[1].state.pressure_pa);
                    let next_norm = (residual[0] / trial_pressure_bound)
                        .abs()
                        .max((residual[1] / (1e-12 * chemical_scale)).abs());
                    if next_norm < norm {
                        admitted = Some(next);
                        break;
                    }
                }
            }
            scale *= 0.5;
        }
        if admitted.is_none() && change.iter().all(|v| v.abs() < 1e-10) {
            // Near a root, cancellation in liquid pressure can make the last
            // representable density updates nonmonotone. Search neighbouring
            // floating-point densities instead of weakening phase equality.
            let mut lower = rho[0];
            let mut upper = rho[0];
            let mut best = norm;
            for _ in 0..64 {
                lower = lower.next_down();
                upper = upper.next_up();
                for liquid_density in [lower, upper] {
                    let next = [liquid_density, rho[1]];
                    if next[0].ln() - next[1].ln() < minimum_separation {
                        continue;
                    }
                    if let Ok((trial, residual)) = evaluate(t, next) {
                        let bound = 1e-6
                            + 1e-12 * trial[0].state.pressure_pa.max(trial[1].state.pressure_pa);
                        let merit = (residual[0] / bound)
                            .abs()
                            .max((residual[1] / (1e-12 * chemical_scale)).abs());
                        if merit < best {
                            best = merit;
                            admitted = Some(next);
                        }
                    }
                }
                if best <= 1. {
                    break;
                }
            }
        }
        rho = admitted.ok_or(Error::NumericalFailure)?;
    }
    Err(Error::NumericalFailure)
}
