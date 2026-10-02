//! Finite-capacity lumped body/fluid conduction with caller-supplied interface conductances.
use super::{Error, Liquid, ThermalTranslatingBody, positive};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalBodyStep {
    pub impacts: super::DynamicImpactReport,
    /// Net heat entering liquid by conduction, excluding impact deposition.
    pub conductive_heat: f64,
}
impl Liquid {
    /// Forward half exchange followed by reverse half exchange. Sensible constant-
    /// capacity stars admit second-order time accuracy; latent transitions and constitutive feedback need separate convergence checks.
    /// # Errors
    /// Same validation as `exchange_body_heat`; both whole states roll back on failure.
    pub fn exchange_body_heat_symmetric(
        &mut self,
        dt: f64,
        body: &mut ThermalTranslatingBody,
        conductances: &[f64],
    ) -> Result<f64, Error> {
        let mut candidate = self.clone();
        let mut body_candidate = *body;
        let first = candidate.exchange_body_heat_ordered(
            0.5 * dt,
            &mut body_candidate,
            conductances,
            false,
        )?;
        let last = candidate.exchange_body_heat_ordered(
            0.5 * dt,
            &mut body_candidate,
            conductances,
            true,
        )?;
        let heat = first + last;
        if !heat.is_finite() {
            return Err(Error::NumericalFailure);
        }
        *self = candidate;
        *body = body_candidate;
        Ok(heat)
    }
    /// Half conduction, symmetric mechanics/impact heating, then half conduction.
    /// Caller-supplied conductances are fixed over this outer interval. This does
    /// not imply second-order convergence of latent or multi-particle thermal exchange.
    /// # Errors
    /// Any conduction/impact/constitutive error rolls back both complete states.
    pub fn step_symmetric_with_thermal_body_conduction(
        &mut self,
        dt: f64,
        body: &mut ThermalTranslatingBody,
        world: &impl crate::CollisionWorld,
        config: super::DynamicWorldConfig,
        fluid_heat_fraction: f64,
        conductances: &[f64],
    ) -> Result<ThermalBodyStep, Error> {
        let mut fluid_candidate = self.clone();
        let mut body_candidate = *body;
        let first = fluid_candidate.exchange_body_heat_symmetric(
            0.5 * dt,
            &mut body_candidate,
            conductances,
        )?;
        let impacts = fluid_candidate.step_symmetric_with_thermal_body(
            dt,
            &mut body_candidate,
            world,
            config,
            fluid_heat_fraction,
        )?;
        let last = fluid_candidate.exchange_body_heat_symmetric(
            0.5 * dt,
            &mut body_candidate,
            conductances,
        )?;
        let conductive_heat = first + last;
        if !conductive_heat.is_finite() {
            return Err(Error::NumericalFailure);
        }
        *self = fluid_candidate;
        *body = body_candidate;
        Ok(ThermalBodyStep {
            impacts,
            conductive_heat,
        })
    }
    /// Conducts heat between a uniform body and each fluid particle in index order.
    /// Conductances are W/K in SI; geometry and interface area are supplied by caller.
    /// Pairs use exact exponential exchange with event splitting at latent boundaries.
    /// This star composition is first-order and does not move fluid or body.
    /// Returns signed heat entering liquid, with the opposite change in body energy.
    /// # Errors
    /// Invalid time/conductance/capacity, missing fields, constitutive failure or overflow.
    /// Both complete states are preserved on error.
    pub fn exchange_body_heat(
        &mut self,
        dt: f64,
        body: &mut ThermalTranslatingBody,
        conductances: &[f64],
    ) -> Result<f64, Error> {
        self.exchange_body_heat_ordered(dt, body, conductances, false)
    }
    fn exchange_body_heat_ordered(
        &mut self,
        dt: f64,
        body: &mut ThermalTranslatingBody,
        conductances: &[f64],
        reverse: bool,
    ) -> Result<f64, Error> {
        if !positive(dt)
            || conductances.len() != self.particles.len()
            || conductances.iter().any(|g| !g.is_finite() || *g < 0.0)
        {
            return Err(Error::InvalidTransport);
        }
        let mut fields = self.transport.clone().ok_or(Error::InvalidTransport)?;
        let mut candidate = *body;
        let initial_body_energy = body.thermal_energy()?;
        let capacity = body.mechanics.mass * body.specific_heat;
        let mut body_energy = initial_body_energy;
        let total = self.transport_totals()?.ok_or(Error::InvalidTransport)?.0 + body_energy;
        if !total.is_finite() {
            return Err(Error::NumericalFailure);
        }
        for index in 0..conductances.len() {
            let i = if reverse {
                conductances.len() - 1 - index
            } else {
                index
            };
            let conductance = &conductances[i];
            if *conductance <= 0.0 {
                continue;
            }
            let particle = &self.particles[i];
            let energy = fields.energy(particle, &fields.fields[i], i)?;
            let temperature = fields.fields[i].temperature;
            let body_temperature = body_energy / capacity;
            let phase = fields
                .phase
                .as_ref()
                .and_then(|p| p.model(i, particle.material));
            let transfer = if phase.is_none() {
                let fluid_capacity = particle.mass * fields.specific_heat(i, particle.material)?;
                let small = fluid_capacity.min(capacity);
                let big = fluid_capacity.max(capacity);
                let reduced = small / (1.0 + small / big);
                if !positive(reduced) {
                    return Err(Error::NumericalFailure);
                }
                let fraction = -(-(*conductance / reduced) * dt).exp_m1();
                reduced * (body_temperature - temperature) * fraction
            } else {
                let model = phase.ok_or(Error::InvalidPhaseChange)?;
                let fluid_capacity = particle.mass * fields.specific_heat(i, particle.material)?;
                let low = fluid_capacity * model.temperature;
                let high = low + particle.mass * model.latent_heat;
                phase_pair_transfer(
                    [energy, body_energy],
                    [fluid_capacity, capacity],
                    [low, high],
                    *conductance,
                    dt,
                )?
            };
            if !transfer.is_finite() {
                return Err(Error::NumericalFailure);
            }
            fields.set_energy(i, particle, energy + transfer)?;
            body_energy -= transfer;
        }
        candidate.temperature = body_energy / capacity;
        candidate.thermal_energy()?;
        self.evaluate_materials(&self.particles, Some(&fields))?;
        if !body_energy.is_finite() {
            return Err(Error::NumericalFailure);
        }
        let mut final_total = body_energy;
        for (i, particle) in self.particles.iter().enumerate() {
            final_total += fields.energy(particle, &fields.fields[i], i)?;
            if !final_total.is_finite() {
                return Err(Error::NumericalFailure);
            }
        }
        let heat = initial_body_energy - body_energy;
        if !heat.is_finite() {
            return Err(Error::NumericalFailure);
        }
        self.transport = Some(fields);
        *body = candidate;
        Ok(heat)
    }
}

// Each sensible/latent region has linear temperature in enthalpy, hence an exact
// exponential exchange. At most two phase boundaries can be crossed monotonically.
#[allow(clippy::float_cmp)] // Exactly equal temperatures drive exactly zero heat.
fn phase_pair_transfer(
    energies: [f64; 2],
    capacities: [f64; 2],
    plateau: [f64; 2],
    conductance: f64,
    dt: f64,
) -> Result<f64, Error> {
    let [initial, mut body_energy] = energies;
    let [fluid_capacity, body_capacity] = capacities;
    let [low, high] = plateau;
    if !low.is_finite() || !high.is_finite() || high <= low {
        return Err(Error::NumericalFailure);
    }
    let mut energy = initial;
    let mut remaining = dt;
    for _ in 0..3 {
        let temperature = if energy < low {
            energy / fluid_capacity
        } else if energy > high {
            (energy - high + low) / fluid_capacity
        } else {
            low / fluid_capacity
        };
        let difference = body_energy / body_capacity - temperature;
        if difference == 0.0 {
            return Ok(energy - initial);
        }
        let (latent, boundary) = if energy < low || (energy <= low && difference < 0.0) {
            (false, if difference > 0.0 { Some(low) } else { None })
        } else if energy > high || (energy >= high && difference > 0.0) {
            (false, if difference < 0.0 { Some(high) } else { None })
        } else {
            (true, Some(if difference > 0.0 { high } else { low }))
        };
        let effective_capacity = if latent {
            body_capacity
        } else {
            let small = fluid_capacity.min(body_capacity);
            small / (1.0 + small / fluid_capacity.max(body_capacity))
        };
        if !positive(effective_capacity) {
            return Err(Error::NumericalFailure);
        }
        let equilibrium_transfer = effective_capacity * difference;
        let rate = conductance / effective_capacity;
        let transfer = equilibrium_transfer * (-(-rate * remaining).exp_m1());
        if !transfer.is_finite() {
            return Err(Error::NumericalFailure);
        }
        let Some(boundary) = boundary.filter(|bound| {
            if difference > 0.0 {
                energy + transfer >= *bound
            } else {
                energy + transfer <= *bound
            }
        }) else {
            return Ok(energy - initial + transfer);
        };
        let to_boundary = boundary - energy;
        let ratio = to_boundary / equilibrium_transfer;
        if ratio >= 1.0 {
            return Ok(energy - initial + transfer);
        }
        let crossing_time = -(-ratio).ln_1p() / rate;
        if !crossing_time.is_finite() || crossing_time < 0.0 || crossing_time > remaining {
            return Err(Error::NumericalFailure);
        }
        energy = boundary;
        body_energy -= to_boundary;
        remaining -= crossing_time;
        if remaining <= 0.0 {
            return Ok(energy - initial);
        }
    }
    Err(Error::NumericalFailure)
}
