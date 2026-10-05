//! Equilibrium phase selection from the shared IAPWS-95 branch/coexistence models.
use super::{
    Error, WaterCoexistence, WaterHomogeneousState, water_coexistence, water_homogeneous_response,
    water_homogeneous_state,
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterEquilibriumState {
    pub temperature_k: f64,
    pub pressure_pa: f64,
    /// Vapor mass fraction; two-phase states use the specific-volume lever rule.
    pub vapor_mass_fraction: f64,
    pub internal_energy_j_per_kg: f64,
    pub enthalpy_j_per_kg: f64,
    pub entropy_j_per_kg_k: f64,
}

/// Subcritical equilibrium caloric/acoustic response. Inside the two-phase
/// region this assumes instantaneous phase, thermal and mechanical equilibrium;
/// it is not a frozen-composition or finite-relaxation acoustic model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterEquilibriumResponse {
    pub state: WaterEquilibriumState,
    pub cv_j_per_kg_k: f64,
    pub sound_speed_m_per_s: f64,
    pub pressure_density_pa_m3_per_kg: f64,
    pub pressure_temperature_pa_per_k: f64,
}

pub fn water_equilibrium_response(t: f64, density: f64) -> Result<WaterEquilibriumResponse, Error> {
    if !density.is_finite() || density <= 0. {
        return Err(Error::InvalidPhaseChange);
    }
    let (phases, branches) = super::water_helmholtz::solve_coexistence(t)?;
    let rl = phases.liquid_density_kg_per_m3;
    let rv = phases.vapor_density_kg_per_m3;
    if density >= rl || density <= rv {
        let response = water_homogeneous_response(t, density)?;
        return Ok(WaterEquilibriumResponse {
            state: homogeneous_equilibrium(t, if density <= rv { 1. } else { 0. }, response.state),
            cv_j_per_kg_k: response.state.cv_j_per_kg_k,
            sound_speed_m_per_s: response.state.sound_speed_m_per_s,
            pressure_density_pa_m3_per_kg: response.pressure_density_pa_m3_per_kg,
            pressure_temperature_pa_per_k: response.pressure_temperature_pa_per_k,
        });
    }
    let state = mixture_equilibrium(t, density, phases)?;
    let volumes = [1. / rl, 1. / rv];
    let volume_gap = volumes[1] - volumes[0];
    let entropy_gap = phases.vapor.entropy_j_per_kg_k - phases.liquid.entropy_j_per_kg_k;
    // Clapeyron slope; along each phase: rho'=(p_sat'-p_T)/p_rho.
    let pressure_temperature = entropy_gap / volume_gap;
    let densities = [rl, rv];
    let density_slopes = [0, 1].map(|i| {
        (pressure_temperature - branches[i].pressure_temperature_pa_per_k)
            / branches[i].pressure_density_pa_m3_per_kg
    });
    let volume_slopes = [0, 1].map(|i| -density_slopes[i] / densities[i].powi(2));
    let entropy_slopes = [0, 1].map(|i| {
        branches[i].state.cv_j_per_kg_k / t
            - branches[i].pressure_temperature_pa_per_k * density_slopes[i] / densities[i].powi(2)
    });
    let x = state.vapor_mass_fraction;
    let fraction_slope =
        -(volume_slopes[0] + x * (volume_slopes[1] - volume_slopes[0])) / volume_gap;
    let entropy_temperature =
        (1. - x) * entropy_slopes[0] + x * entropy_slopes[1] + fraction_slope * entropy_gap;
    let cv = t * entropy_temperature;
    // Isothermal mixture pressure is independent of density. Along constant
    // entropy, c² = T*p_T²/(rho²*cv), with changing equilibrium phase fraction.
    let sound2 = t * pressure_temperature.powi(2) / (density.powi(2) * cv);
    if [cv, sound2, pressure_temperature]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err(Error::NumericalFailure);
    }
    Ok(WaterEquilibriumResponse {
        state,
        cv_j_per_kg_k: cv,
        sound_speed_m_per_s: sound2.sqrt(),
        pressure_density_pa_m3_per_kg: 0.,
        pressure_temperature_pa_per_k: pressure_temperature,
    })
}
/// Fixed-volume water and a lumped contact reservoir with constant heat capacity.
/// Conductance includes contact area/resistance; it must be supplied/calibrated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterHeatContact {
    pub mass_kg: f64,
    pub volume_m3: f64,
    pub reservoir_capacity_j_per_k: f64,
    pub conductance_w_per_k: f64,
    pub temperature_interval: [f64; 2],
}

/// Reversible adiabatic volume change of a closed, equilibrium water mass.
/// The caller prescribes the new volume; this does not integrate a piston or
/// fluid motion. Work delivered by expansion is credited to the nonnegative
/// external work-energy store; compression draws from it. Returns delivered
/// work in J (negative for compression). Volume and both energy owners publish
/// together only after EOS/entropy and representable energy balance admission.
pub fn change_water_volume_adiabatically(
    mass_kg: f64,
    volume_m3: &mut f64,
    water_energy_j: &mut f64,
    work_energy_j: &mut f64,
    next_volume_m3: f64,
    temperature_interval: [f64; 2],
) -> Result<(WaterEquilibriumState, f64), Error> {
    if !next_volume_m3.is_finite() || next_volume_m3 <= 0. {
        return Err(Error::InvalidPhaseChange);
    }
    let (initial, _) = transfer_water_heat(
        mass_kg,
        *volume_m3,
        water_energy_j,
        work_energy_j,
        0.,
        temperature_interval,
    )?;
    if next_volume_m3 == *volume_m3 {
        return Ok((initial, 0.));
    }
    let density = mass_kg / next_volume_m3;
    let next =
        water_equilibrium_from_entropy(density, initial.entropy_j_per_kg_k, temperature_interval)?;
    let requested = mass_kg * next.internal_energy_j_per_kg - *water_energy_j;
    let (water_after, work_after, actual) =
        balanced_energy_transfer(*water_energy_j, *work_energy_j, requested)?;
    // Positive pressure means expansion releases work and compression consumes
    // it. Reject changes whose energy effect cannot be resolved by the owners.
    if (next_volume_m3 > *volume_m3 && actual >= 0.)
        || (next_volume_m3 < *volume_m3 && actual <= 0.)
    {
        return Err(Error::NumericalFailure);
    }
    let state =
        water_equilibrium_from_energy(density, water_after / mass_kg, temperature_interval)?;
    if (state.entropy_j_per_kg_k - initial.entropy_j_per_kg_k).abs()
        > 1e-7 + 1e-10 * initial.entropy_j_per_kg_k.abs()
    {
        return Err(Error::NumericalFailure);
    }
    *volume_m3 = next_volume_m3;
    *water_energy_j = water_after;
    *work_energy_j = work_after;
    Ok((state, -actual))
}
/// Backward-Euler contact exchange q=G dt (T_reservoir_after-T_water_after).
/// Uses the shared implicit heat kernel and the IAPWS equilibrium energy decoder.
/// Both external energy stores publish together through transfer_water_heat.
/// Refine dt for accuracy; unconditional implicit stability is not exact dynamics.
pub fn exchange_water_contact_heat(
    contact: WaterHeatContact,
    water_energy_j: &mut f64,
    reservoir_energy_j: &mut f64,
    dt: f64,
) -> Result<(WaterEquilibriumState, f64), Error> {
    let WaterHeatContact {
        mass_kg,
        volume_m3,
        reservoir_capacity_j_per_k: capacity,
        conductance_w_per_k: conductance,
        temperature_interval: interval,
    } = contact;
    if !capacity.is_finite()
        || capacity <= 0.
        || !conductance.is_finite()
        || conductance < 0.
        || !dt.is_finite()
        || dt < 0.
    {
        return Err(Error::InvalidPhaseChange);
    }
    let (initial, _) = transfer_water_heat(
        mass_kg,
        volume_m3,
        water_energy_j,
        reservoir_energy_j,
        0.,
        interval,
    )?;
    if conductance == 0. || dt == 0. {
        return Ok((initial, 0.));
    }
    let density = mass_kg / volume_m3;
    let reservoir_temperature = *reservoir_energy_j / capacity;
    if !reservoir_temperature.is_finite() {
        return Err(Error::NumericalFailure);
    }
    let inverse_rate = (1. / conductance) / dt;
    if inverse_rate.is_infinite() || reservoir_temperature == initial.temperature_k {
        return Ok((initial, 0.));
    }
    let low = interval[0].max(initial.temperature_k.min(reservoir_temperature));
    let high = interval[1].min(initial.temperature_k.max(reservoir_temperature));
    // Solve directly in T: q(T)=m*u(T,rho)-U_old. Every scalar trial needs
    // one equilibrium evaluation, rather than a nested energy inversion.
    let temperature = super::transport::monotone_root(low, high, |t| {
        let water = water_equilibrium_at_temperature(t, density)?;
        let q = mass_kg * water.internal_energy_j_per_kg - *water_energy_j;
        let reservoir_after_temperature = (*reservoir_energy_j - q) / capacity;
        if !q.is_finite() || !reservoir_after_temperature.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(q * inverse_rate - (reservoir_after_temperature - t))
    })?;
    let water = water_equilibrium_at_temperature(temperature, density)?;
    let heat = mass_kg * water.internal_energy_j_per_kg - *water_energy_j;
    transfer_water_heat(
        mass_kg,
        volume_m3,
        water_energy_j,
        reservoir_energy_j,
        heat,
        interval,
    )
}
/// Apply prescribed signed heat (J) from a finite reservoir to fixed-volume
/// pure water. Positive heat heats water; negative heat heats the reservoir.
/// Both external energy owners publish only after equilibrium admission.
/// Returns the new water state and actual representable water energy increment.
/// Reservoir energy must stay nonnegative. This supplies no contact transfer
/// rate, momentum exchange or finite-rate evaporation law.
pub fn transfer_water_heat(
    mass_kg: f64,
    volume_m3: f64,
    water_energy_j: &mut f64,
    reservoir_energy_j: &mut f64,
    heat_j: f64,
    temperature_interval: [f64; 2],
) -> Result<(WaterEquilibriumState, f64), Error> {
    if !mass_kg.is_finite()
        || mass_kg <= 0.
        || !volume_m3.is_finite()
        || volume_m3 <= 0.
        || !water_energy_j.is_finite()
        || !reservoir_energy_j.is_finite()
        || *reservoir_energy_j < 0.
        || !heat_j.is_finite()
    {
        return Err(Error::InvalidPhaseChange);
    }
    let density = mass_kg / volume_m3;
    let initial =
        water_equilibrium_from_energy(density, *water_energy_j / mass_kg, temperature_interval)?;
    if heat_j == 0. {
        return Ok((initial, 0.));
    }
    let (next, reservoir_next, actual) =
        balanced_energy_transfer(*water_energy_j, *reservoir_energy_j, heat_j)?;
    let state = water_equilibrium_from_energy(density, next / mass_kg, temperature_interval)?;
    *water_energy_j = next;
    *reservoir_energy_j = reservoir_next;
    Ok((state, actual))
}

/// Shared staged receipt for contact heat and reversible volume work.
fn balanced_energy_transfer(
    water_energy_j: f64,
    reservoir_energy_j: f64,
    heat_j: f64,
) -> Result<(f64, f64, f64), Error> {
    let next = water_energy_j + heat_j;
    let reservoir_next = reservoir_energy_j - heat_j;
    if !next.is_finite() || !reservoir_next.is_finite() || reservoir_next < 0. {
        return Err(Error::NumericalFailure);
    }
    let actual = next - water_energy_j;
    let removed = reservoir_energy_j - reservoir_next;
    let scale = water_energy_j.abs() + reservoir_energy_j.abs() + next.abs() + reservoir_next.abs();
    if !scale.is_finite()
        || !actual.is_finite()
        || !removed.is_finite()
        || (actual == 0.) != (removed == 0.)
        || (actual - removed).abs() > 128. * f64::EPSILON * scale
    {
        return Err(Error::NumericalFailure);
    }
    Ok((next, reservoir_next, actual))
}
/// Evaluate subcritical pure-water equilibrium at T (K), bulk density (kg/m3).
/// Between coexistence densities, specific volume, u and s are mass-weighted.
/// Outside them, evaluate the corresponding homogeneous branch. No ice,
/// supercritical state, surface energy or finite-rate interface kinetics here.
pub fn water_equilibrium_at_temperature(
    t: f64,
    density: f64,
) -> Result<WaterEquilibriumState, Error> {
    if !density.is_finite() || density <= 0. {
        return Err(Error::InvalidPhaseChange);
    }
    let phases = water_coexistence(t)?;
    let rl = phases.liquid_density_kg_per_m3;
    let rv = phases.vapor_density_kg_per_m3;
    if density >= rl || density <= rv {
        let state = water_homogeneous_state(t, density)?;
        return Ok(homogeneous_equilibrium(
            t,
            if density <= rv { 1. } else { 0. },
            state,
        ));
    }
    mixture_equilibrium(t, density, phases)
}

fn homogeneous_equilibrium(
    t: f64,
    fraction: f64,
    state: WaterHomogeneousState,
) -> WaterEquilibriumState {
    WaterEquilibriumState {
        temperature_k: t,
        pressure_pa: state.pressure_pa,
        vapor_mass_fraction: fraction,
        internal_energy_j_per_kg: state.internal_energy_j_per_kg,
        enthalpy_j_per_kg: state.enthalpy_j_per_kg,
        entropy_j_per_kg_k: state.entropy_j_per_kg_k,
    }
}

fn mixture_equilibrium(
    t: f64,
    density: f64,
    phases: WaterCoexistence,
) -> Result<WaterEquilibriumState, Error> {
    let rl = phases.liquid_density_kg_per_m3;
    let rv = phases.vapor_density_kg_per_m3;
    let fraction = (1. / density - 1. / rl) / (1. / rv - 1. / rl);
    if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
        return Err(Error::NumericalFailure);
    }
    let blend = |a: f64, b: f64| (1. - fraction) * a + fraction * b;
    let energy = blend(
        phases.liquid.internal_energy_j_per_kg,
        phases.vapor.internal_energy_j_per_kg,
    );
    let entropy = blend(
        phases.liquid.entropy_j_per_kg_k,
        phases.vapor.entropy_j_per_kg_k,
    );
    let enthalpy = energy + phases.pressure_pa / density;
    if [energy, entropy, enthalpy].iter().any(|v| !v.is_finite()) {
        return Err(Error::NumericalFailure);
    }
    Ok(WaterEquilibriumState {
        temperature_k: t,
        pressure_pa: phases.pressure_pa,
        vapor_mass_fraction: fraction,
        internal_energy_j_per_kg: energy,
        enthalpy_j_per_kg: enthalpy,
        entropy_j_per_kg_k: entropy,
    })
}
/// Recover subcritical equilibrium at fixed bulk density and specific energy.
/// The caller supplies a valid temperature bracket; no inventory is modified.
/// This includes liquid/vapor phase selection at every trial temperature rather
/// than crossing an unstable homogeneous branch. It is an equilibrium flash,
/// not a finite-rate evaporation step. Rejects unsupported or unbracketed states.
pub fn water_equilibrium_from_energy(
    density: f64,
    energy: f64,
    interval: [f64; 2],
) -> Result<WaterEquilibriumState, Error> {
    invert_equilibrium(
        density,
        energy,
        interval,
        |s| s.internal_energy_j_per_kg,
        1e-6,
    )
}

/// Recover subcritical equilibrium at fixed density and specific entropy
/// (J/(kg K)), on a caller-supplied temperature bracket. This is the state
/// query needed for reversible adiabatic volume changes. It does not apply
/// work, advance mechanics or describe entropy-producing flow/phase kinetics.
pub fn water_equilibrium_from_entropy(
    density: f64,
    entropy: f64,
    interval: [f64; 2],
) -> Result<WaterEquilibriumState, Error> {
    invert_equilibrium(density, entropy, interval, |s| s.entropy_j_per_kg_k, 1e-9)
}

/// A single bracket/inversion owner for the monotone caloric coordinates.
fn invert_equilibrium(
    density: f64,
    target: f64,
    interval: [f64; 2],
    coordinate: impl Fn(WaterEquilibriumState) -> f64,
    absolute_tolerance: f64,
) -> Result<WaterEquilibriumState, Error> {
    let [mut low, mut high] = interval;
    if !target.is_finite() || !low.is_finite() || !high.is_finite() || low >= high {
        return Err(Error::InvalidPhaseChange);
    }
    let lower = water_equilibrium_at_temperature(low, density)?;
    let upper = water_equilibrium_at_temperature(high, density)?;
    if coordinate(lower) > target || coordinate(upper) < target {
        return Err(Error::InvalidPhaseChange);
    }
    if target == coordinate(lower) {
        return Ok(lower);
    }
    if target == coordinate(upper) {
        return Ok(upper);
    }
    for _ in 0..80 {
        let middle = low + 0.5 * (high - low);
        if middle == low || middle == high {
            return Err(Error::NumericalFailure);
        }
        let state = water_equilibrium_at_temperature(middle, density)?;
        let difference = coordinate(state) - target;
        if difference.abs() <= absolute_tolerance + 1e-11 * target.abs() {
            return Ok(state);
        }
        if difference < 0. {
            low = middle;
        } else {
            high = middle;
        }
    }
    Err(Error::NumericalFailure)
}
