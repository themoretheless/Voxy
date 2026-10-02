//! Exact two-way single-drop quadratic drag with a finite well-mixed gas cell.
use super::{Error, Liquid, VaporCell, finite, norm, positive, sub};
pub(super) fn finite_pair(
    p: &mut super::Particle,
    cell: &mut VaporCell,
    dt: f64,
    radius: f64,
    cd: f64,
) -> Result<([f64; 3], f64), Error> {
    let relative = sub(p.velocity, cell.velocity);
    let speed = norm(relative);
    let inverse_mass = 1.0 / p.mass + 1.0 / cell.mass;
    let reduced_mass = 1.0 / inverse_mass;
    let coefficient = 0.5 * (cell.mass / cell.volume) * cd * std::f64::consts::PI * radius * radius;
    let decay = coefficient * inverse_mass * speed * dt;
    if !speed.is_finite() || !positive(reduced_mass) || !decay.is_finite() {
        return Err(Error::NumericalFailure);
    }
    let loss = decay / (1.0 + decay);
    let heat = 0.5 * reduced_mass * speed * speed * loss * (2.0 - loss);
    let impulse = relative.map(|w| reduced_mass * w * loss);
    for k in 0..3 {
        p.velocity[k] -= impulse[k] / p.mass;
        cell.velocity[k] += impulse[k] / cell.mass;
    }
    if !finite(p.velocity) || !finite(cell.velocity) || !heat.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok((impulse, heat))
}
impl Liquid {
    /// Exchanges drag momentum with a finite fixed-volume gas cell and deposits
    /// all dissipated relative kinetic energy into its heat. No mass transfer,
    /// advection, pressure work or particle heating. The gas's latent reference is
    /// unchanged because its mass is unchanged. Radius and Cd are prescribed.
    ///
    /// Relative velocity decays exactly at fixed density/Cd via reduced mass.
    /// Multiple-drop sequential calls are a pair splitting, not an exact shared
    /// cloud solve. Caller supplies spatial membership; no gas sampling is inferred.
    /// # Errors
    /// Invalid selected particle, time, radius, coefficient/cell, unrepresentable
    /// heat/velocity or constitutive overflow. Fluid and gas roll back together.
    pub fn exchange_droplet_drag(
        &mut self,
        index: usize,
        dt: f64,
        gas: &mut VaporCell,
        radius: f64,
        drag_coefficient: f64,
    ) -> Result<f64, Error> {
        if !positive(dt)
            || !positive(radius)
            || !drag_coefficient.is_finite()
            || drag_coefficient < 0.0
            || !positive(gas.mass)
            || !positive(gas.volume)
            || !positive(gas.temperature)
            || !positive(gas.specific_heat_cv)
            || !finite(gas.velocity)
        {
            return Err(Error::InvalidConfig);
        }
        let particle = *self.particles.get(index).ok_or(Error::InvalidParticle)?;
        // A fixed positive latent reference cancels because gas mass is unchanged.
        let initial_energy =
            gas.energy(1.0)? + 0.5 * particle.mass * norm(particle.velocity).powi(2);
        if !initial_energy.is_finite() {
            return Err(Error::NumericalFailure);
        }
        let mut candidate = self.clone();
        let mut cell = *gas;
        let (_, heat) = finite_pair(
            &mut candidate.particles[index],
            &mut cell,
            dt,
            radius,
            drag_coefficient,
        )?;
        let capacity = cell.mass * cell.specific_heat_cv;
        cell.temperature += heat / capacity;
        if !positive(capacity)
            || !heat.is_finite()
            || !positive(cell.temperature)
            || !finite(cell.velocity)
            || !finite(candidate.particles[index].velocity)
            || (heat > 0.0 && cell.temperature == gas.temperature)
        {
            return Err(Error::NumericalFailure);
        }
        let final_energy = cell.energy(1.0)?
            + 0.5 * particle.mass * norm(candidate.particles[index].velocity).powi(2);
        if !final_energy.is_finite()
            || (final_energy - initial_energy).abs()
                > 128.0 * f64::EPSILON * initial_energy.abs().max(1.0)
        {
            return Err(Error::NumericalFailure);
        }
        candidate.effective_materials()?;
        *self = candidate;
        *gas = cell;
        Ok(heat)
    }
}

/// Two-way finite-cell exchange ledger. Gas kinetic change can be negative.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FiniteDropletDragReport {
    pub pair_steps: usize,
    pub gas_impulse: [f64; 3],
    pub dissipated_heat: f64,
    pub gas_kinetic_energy_change: f64,
}
impl Liquid {
    /// Couples all marked drops to one caller-selected well-mixed finite gas cell.
    /// A forward half-step followed by a reverse half-step gives symmetric pair
    /// splitting; it is not an exact simultaneous cloud solution. Positions,
    /// component fields, liquid heat and gas mass/volume remain unchanged.
    /// Dissipation is accumulated before the gas temperature update. No spatial
    /// membership, gas advection, pressure or shielding is inferred.
    /// # Errors
    /// Invalid mask/controls/cell/radii, pair budget, unrepresentable heating or
    /// constitutive overflow. The entire fluid and cell commit together.
    pub fn exchange_marked_droplet_drag(
        &mut self,
        dt: f64,
        gas: &mut VaporCell,
        radii: &[f64],
        drag_coefficient: f64,
    ) -> Result<FiniteDropletDragReport, Error> {
        let flags = self
            .droplet_population
            .as_ref()
            .ok_or(Error::InvalidConfig)?;
        if !positive(dt)
            || radii.len() != self.particles.len()
            || radii.iter().any(|r| !positive(*r))
            || !drag_coefficient.is_finite()
            || drag_coefficient < 0.0
            || !positive(gas.mass)
            || !positive(gas.volume)
            || !positive(gas.temperature)
            || !positive(gas.specific_heat_cv)
            || !finite(gas.velocity)
        {
            return Err(Error::InvalidConfig);
        }
        gas.energy(1.0)?;
        let selected: Vec<_> = flags
            .iter()
            .enumerate()
            .filter_map(|(i, marked)| marked.then_some(i))
            .collect();
        let pair_steps = selected.len().checked_mul(2).ok_or(Error::PairBudget)?;
        if pair_steps > self.config.max_neighbor_checks {
            return Err(Error::PairBudget);
        }
        let mut candidate = self.clone();
        let mut cell = *gas;
        let mut report = FiniteDropletDragReport {
            pair_steps,
            ..FiniteDropletDragReport::default()
        };
        for i in selected.iter().chain(selected.iter().rev()).copied() {
            let (impulse, heat) = finite_pair(
                &mut candidate.particles[i],
                &mut cell,
                0.5 * dt,
                radii[i],
                drag_coefficient,
            )?;
            report.dissipated_heat += heat;
            for k in 0..3 {
                report.gas_impulse[k] += impulse[k];
            }
            if !report.dissipated_heat.is_finite() {
                return Err(Error::NumericalFailure);
            }
        }
        let capacity = cell.mass * cell.specific_heat_cv;
        if !positive(capacity) {
            return Err(Error::NumericalFailure);
        }
        cell.temperature += report.dissipated_heat / capacity;
        if !positive(cell.temperature)
            || (report.dissipated_heat > 0.0 && cell.temperature == gas.temperature)
        {
            return Err(Error::NumericalFailure);
        }
        report.gas_kinetic_energy_change = 0.5
            * cell.mass
            * (0..3)
                .map(|k| {
                    (cell.velocity[k] + gas.velocity[k]) * (cell.velocity[k] - gas.velocity[k])
                })
                .sum::<f64>();
        let liquid_change: f64 = selected
            .iter()
            .map(|&i| {
                let a = self.particles[i];
                let b = candidate.particles[i];
                0.5 * a.mass
                    * (0..3)
                        .map(|k| (a.velocity[k] + b.velocity[k]) * (b.velocity[k] - a.velocity[k]))
                        .sum::<f64>()
            })
            .sum();
        let scale = selected
            .iter()
            .map(|&i| {
                let p = self.particles[i];
                0.5 * p.mass * norm(p.velocity).powi(2)
            })
            .sum::<f64>()
            + 0.5 * gas.mass * norm(gas.velocity).powi(2);
        if !scale.is_finite()
            || !liquid_change.is_finite()
            || !finite(report.gas_impulse)
            || !report.gas_kinetic_energy_change.is_finite()
            || (liquid_change + report.gas_kinetic_energy_change + report.dissipated_heat).abs()
                > 512.0 * f64::EPSILON * scale.max(1.0)
        {
            return Err(Error::NumericalFailure);
        }
        cell.energy(1.0)?;
        candidate.effective_materials()?;
        *self = candidate;
        *gas = cell;
        Ok(report)
    }
}
