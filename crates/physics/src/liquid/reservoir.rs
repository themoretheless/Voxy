//! Exact prescribed-temperature reservoir exchange at fixed thermal connections.
use super::{Error, Liquid, Particle, positive, transport::Transport};
impl Liquid {
    /// Exchanges heat with a prescribed-temperature reservoir at fixed positions.
    /// Conductance per particle is energy/(time*temperature), supplied by the caller
    /// from the wall/contact geometry. The returned signed energy enters the fluid;
    /// the reservoir receives its opposite. This operation does not advance motion.
    /// Constant heat capacities and isothermal latent plateaux are solved piecewise
    /// exactly. Connections are frozen for this interval, not inferred from a box.
    /// # Errors
    /// Requires finite nonnegative temperature/conductances, positive finite dt,
    /// matching particle count and thermal fields. Any failure preserves all state.
    pub fn exchange_reservoir_heat(
        &mut self,
        dt: f64,
        temperature: f64,
        conductances: &[f64],
    ) -> Result<f64, Error> {
        if !positive(dt) {
            return Err(Error::InvalidTimeStep);
        }
        if !temperature.is_finite()
            || temperature < 0.0
            || conductances.len() != self.particles.len()
            || conductances.iter().any(|g| !g.is_finite() || *g < 0.0)
        {
            return Err(Error::InvalidTransport);
        }
        let mut next = self.transport.clone().ok_or(Error::InvalidTransport)?;
        let mut transferred = 0.0;
        let mut final_energy = 0.0;
        for (index, (particle, conductance)) in self.particles.iter().zip(conductances).enumerate()
        {
            transferred +=
                next.reservoir_exchange(index, particle, temperature, *conductance, dt)?;
            final_energy += next.energy(particle, &next.fields[index], index)?;
            if !transferred.is_finite() || !final_energy.is_finite() {
                return Err(Error::NumericalFailure);
            }
        }
        self.evaluate_materials(&self.particles, Some(&next))?;
        self.transport = Some(next);
        Ok(transferred)
    }
}
impl Transport {
    #[allow(clippy::float_cmp)] // Exact equality means zero reservoir flux, including the latent plateau.
    fn reservoir_exchange(
        &mut self,
        index: usize,
        particle: &Particle,
        reservoir: f64,
        conductance: f64,
        dt: f64,
    ) -> Result<f64, Error> {
        if conductance == 0.0 {
            return Ok(0.0);
        }
        let capacity = particle.mass * self.specific_heat(index, particle.material)?;
        if !positive(capacity) {
            return Err(Error::NumericalFailure);
        }
        let model = self
            .phase
            .as_ref()
            .and_then(|p| p.model(index, particle.material));
        let initial = self.energy(particle, &self.fields[index], index)?;
        let mut energy = initial;
        let mut remaining = dt;
        // At most sensible heat -> latent plateau -> sensible heat.
        for _ in 0..3 {
            self.set_energy(index, particle, energy)?;
            let temperature = self.fields[index].temperature;
            if remaining == 0.0 || temperature == reservoir {
                break;
            }
            if let Some(phase) = model {
                let start = capacity * phase.temperature;
                let end = start + particle.mass * phase.latent_heat;
                if !end.is_finite() {
                    return Err(Error::NumericalFailure);
                }
                let heating = reservoir > phase.temperature;
                let plateau = (heating && energy >= start && energy < end)
                    || (!heating && energy > start && energy <= end);
                if plateau {
                    let flux = conductance * (reservoir - phase.temperature);
                    if !flux.is_finite() {
                        return Err(Error::NumericalFailure);
                    }
                    let target = if heating { end } else { start };
                    let crossing = (target - energy) / flux;
                    if remaining >= crossing {
                        energy = target;
                        remaining -= crossing;
                        continue;
                    }
                    energy += flux * remaining;
                    break;
                }
                if (heating && energy < start) || (!heating && energy > end) {
                    let crossing = capacity / conductance
                        * ((reservoir - temperature).abs() / (reservoir - phase.temperature).abs())
                            .ln();
                    if crossing.is_finite() && crossing >= 0.0 && remaining >= crossing {
                        energy = if heating { start } else { end };
                        remaining -= crossing;
                        continue;
                    }
                }
            }
            let fraction = -(-conductance / capacity * remaining).exp_m1();
            energy += capacity * (reservoir - temperature) * fraction;
            break;
        }
        self.set_energy(index, particle, energy)?;
        Ok(energy - initial)
    }
}
