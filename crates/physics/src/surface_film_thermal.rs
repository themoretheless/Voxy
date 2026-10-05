//! Owned sensible thermal inventory for equal-density film mixtures.
//! Sensible-energy transport and an adapter to the shared solution vapor solver.
use super::{FilmMixture, FilmWithdrawal};

#[derive(Clone, Debug)]
pub struct ThermalFilmMixture {
    mixture: FilmMixture,
    capacities: Vec<f64>,
    energy: Vec<f64>,
}
/// Extensive sensible energy accompanying an inventory transfer.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalFilmWithdrawal {
    pub inventory: FilmWithdrawal,
    pub sensible_energy_j: f64,
}
/// Borrowed finite contact reservoirs; their owning solid system retains storage.
#[derive(Debug)]
pub struct FilmSubstrateHeat<'a> {
    pub energies_j: &'a mut [f64],
    pub capacities_j_per_k: &'a [f64],
    pub conductances_w_per_k: &'a [f64],
}
// Closed two-capacity relaxation; shared by in-plane and substrate conduction.
fn exchange_heat_pair(
    ea: f64,
    ca: f64,
    eb: f64,
    cb: f64,
    g: f64,
    dt: f64,
) -> Result<(f64, f64), &'static str> {
    let transfer = crate::heat_exchange::finite_pair_transfer(ea / ca, ca, eb / cb, cb, g, dt)?;
    let a = ea - transfer;
    let b = eb + transfer;
    if !a.is_finite() || !b.is_finite() || a <= 0. || b <= 0. {
        return Err("invalid film conduction heat balance");
    }
    if (a == ea) != (b == eb) {
        return Err("film conduction heat change cannot be represented");
    }
    Ok((a, b))
}
impl ThermalFilmMixture {
    /// Heat exchange with one finite substrate reservoir per film cell.
    /// Capacities are J/K, conductances W/K and reservoir energies joules.
    /// Conductance includes contact area and resistance, supplied by the caller.
    /// Independent exact pair relaxations publish film and substrate together.
    /// Dry cells have no liquid heat capacity and do not exchange heat.
    /// This is lumped contact exchange, not spatial heat diffusion in the solid.
    pub fn exchange_substrate_heat(
        &mut self,
        substrate_energies: &mut [f64],
        substrate_capacities: &[f64],
        conductances: &[f64],
        dt: f64,
    ) -> Result<(), &'static str> {
        let n = self.energy.len();
        if substrate_energies.len() != n
            || substrate_capacities.len() != n
            || conductances.len() != n
            || !dt.is_finite()
            || dt < 0.
        {
            return Err("invalid film substrate heat configuration");
        }
        self.temperatures()?;
        let mut candidate = self.clone();
        let mut staged = substrate_energies.to_vec();
        for cell in 0..n {
            let eb = staged[cell];
            let cb = substrate_capacities[cell];
            let g = conductances[cell];
            if !eb.is_finite()
                || eb <= 0.
                || !cb.is_finite()
                || cb <= 0.
                || !(eb / cb).is_finite()
                || eb / cb <= 0.
                || !g.is_finite()
                || g < 0.
            {
                return Err("invalid film substrate heat reservoir");
            }
            let ca = self.capacity(&self.mixture.component_masses_kg()[cell])?;
            if ca == 0. || g == 0. || dt == 0. {
                continue;
            }
            let (ea, eb) = exchange_heat_pair(self.energy[cell], ca, eb, cb, g, dt)?;
            candidate.energy[cell] = ea;
            staged[cell] = eb;
        }
        candidate.temperatures()?;
        *self = candidate;
        substrate_energies.copy_from_slice(&staged);
        Ok(())
    }
    /// In-plane Fourier conduction for a uniform conductivity in W/(m K).
    /// Edge cross-section uses the harmonic mean of neighboring film heights.
    /// Dry cells insulate; there is no substrate or ambient heat exchange here.
    /// Exact two-cell exchanges compose in symmetric sweeps. Refine max_step
    /// for multi-cell splitting error; unconditional pair stability is not an
    /// accuracy guarantee. Inventory is fixed and the entire call is atomic.
    pub fn conduct_heat(
        &mut self,
        conductivity: f64,
        dt: f64,
        max_step: f64,
    ) -> Result<(), &'static str> {
        if !conductivity.is_finite()
            || conductivity < 0.
            || !dt.is_finite()
            || dt < 0.
            || !max_step.is_finite()
            || max_step <= 0.
        {
            return Err("invalid film conduction controls");
        }
        self.temperatures()?;
        if dt == 0. || conductivity == 0. {
            return Ok(());
        }
        let count = (dt / max_step).ceil().max(1.);
        if !count.is_finite() || count > 1_000_000. {
            return Err("film conduction substep budget exceeded");
        }
        let step = dt / count;
        let film = self.mixture.film();
        let capacities = self
            .mixture
            .component_masses_kg()
            .iter()
            .map(|row| self.capacity(row))
            .collect::<Result<Vec<_>, _>>()?;
        let mut links = Vec::new();
        for &(a, b, length, distance) in &film.edges {
            if capacities[a] == 0. || capacities[b] == 0. {
                continue;
            }
            let ha = film.volume[a] / film.area[a];
            let hb = film.volume[b] / film.area[b];
            let height = ha.min(hb) / (0.5 + 0.5 * ha.min(hb) / ha.max(hb));
            let conductance = conductivity * height * length / distance;
            if !conductance.is_finite() || conductance < 0. {
                return Err("film conduction conductance overflow");
            }
            links.push((a, b, conductance));
        }
        let mut candidate = self.clone();
        for _ in 0..count as usize {
            for &(a, b, conductance) in links.iter().chain(links.iter().rev()) {
                let (ea, eb) = exchange_heat_pair(
                    candidate.energy[a],
                    capacities[a],
                    candidate.energy[b],
                    capacities[b],
                    conductance,
                    0.5 * step,
                )?;
                candidate.energy[a] = ea;
                candidate.energy[b] = eb;
            }
        }
        candidate.temperatures()?;
        *self = candidate;
        Ok(())
    }
    /// Component capacities in J/(kg K), temperatures in kelvin per cell.
    /// Reference is zero sensible energy at absolute zero; dry cells hold no heat.
    pub fn new(
        mixture: FilmMixture,
        capacities: Vec<f64>,
        temperatures: &[f64],
    ) -> Result<Self, &'static str> {
        if capacities.len() != mixture.component_names().len()
            || capacities.iter().any(|c| !c.is_finite() || *c <= 0.)
            || temperatures.len() != mixture.component_masses_kg().len()
            || temperatures.iter().any(|t| !t.is_finite() || *t <= 0.)
        {
            return Err("invalid film thermal configuration");
        }
        let mut state = Self {
            mixture,
            capacities,
            energy: vec![],
        };
        for (row, temperature) in state.mixture.component_masses_kg().iter().zip(temperatures) {
            let capacity = state.capacity(row)?;
            let energy = capacity * temperature;
            if !energy.is_finite() || (capacity > 0. && energy <= 0.) {
                return Err("film thermal energy cannot be represented");
            }
            state.energy.push(energy);
        }
        state.temperatures()?;
        Ok(state)
    }
    fn capacity(&self, row: &[f64]) -> Result<f64, &'static str> {
        let value = row
            .iter()
            .zip(&self.capacities)
            .map(|(m, c)| m * c)
            .sum::<f64>();
        if !value.is_finite() || (row.iter().any(|v| *v > 0.) && value <= 0.) {
            return Err("film thermal capacity cannot be represented");
        }
        Ok(value)
    }
    /// Read-only access prevents volume/composition changes bypassing the energy ledger.
    /// Refresh geometry while preserving owned component mass and sensible heat.
    /// Geometry work must be accounted for by the mechanical coupling caller.
    pub fn update_geometry(&mut self, points: &[[f64; 3]]) -> Result<(), &'static str> {
        self.mixture.update_geometry(points)
    }
    pub fn mixture(&self) -> &FilmMixture {
        &self.mixture
    }
    pub fn energies_j(&self) -> &[f64] {
        &self.energy
    }
    /// Dry cells have no temperature. Wet cells must have positive finite temperature.
    pub fn temperatures(&self) -> Result<Vec<Option<f64>>, &'static str> {
        self.mixture
            .component_masses_kg()
            .iter()
            .zip(&self.energy)
            .map(|(row, energy)| {
                let capacity = self.capacity(row)?;
                if capacity == 0. {
                    if *energy != 0. {
                        return Err("dry film has thermal energy");
                    }
                    Ok(None)
                } else {
                    let temperature = energy / capacity;
                    if !temperature.is_finite() || temperature <= 0. {
                        return Err("invalid film temperature");
                    }
                    Ok(Some(temperature))
                }
            })
            .collect()
    }
    /// Derived total cell capacities, J/K, from canonical component masses.
    pub fn heat_capacities_j_per_k(&self) -> Result<Vec<f64>, &'static str> {
        self.mixture
            .component_masses_kg()
            .iter()
            .map(|row| self.capacity(row))
            .collect()
    }
    /// Constant component capacities in J/(kg K), in the mixture's species order.
    pub fn specific_heats(&self) -> &[f64] {
        &self.capacities
    }

    pub(crate) fn accept_captured_mixture(
        &mut self,
        mixture: FilmMixture,
        heat: &[(usize, Option<f64>)],
    ) -> Result<(), &'static str> {
        if mixture.component_names() != self.mixture.component_names()
            || mixture.component_masses_kg().len() != self.mixture.component_masses_kg().len()
            || mixture
                .component_masses_kg()
                .iter()
                .zip(self.mixture.component_masses_kg())
                .any(|(a, b)| a.iter().zip(b).any(|(new, old)| *new < *old))
        {
            return Err("invalid captured film mixture");
        }
        let heat = heat
            .iter()
            .map(|(cell, energy)| {
                Ok((*cell, energy.ok_or("missing captured film thermal energy")?))
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        let mut requested = vec![0.; self.energy.len()];
        for (cell, energy) in &heat {
            let old = self
                .mixture
                .component_masses_kg()
                .get(*cell)
                .ok_or("invalid captured film cell")?;
            let new = &mixture.component_masses_kg()[*cell];
            if old == new || mixture.film().volume[*cell] <= self.mixture.film().volume[*cell] {
                return Err("captured film inventory change cannot be represented");
            }
            if !energy.is_finite() || *energy < 0. {
                return Err("invalid captured film energy");
            }
            requested[*cell] += energy;
            if !requested[*cell].is_finite() {
                return Err("captured film heat overflow");
            }
        }
        let mut candidate = self.clone();
        candidate.mixture = mixture;
        candidate.add_heat_batch(&heat)?;
        for (cell, energy) in requested.iter().enumerate() {
            if *energy > 0. && candidate.energy[cell] <= self.energy[cell] {
                return Err("captured film heat change cannot be represented");
            }
        }
        *self = candidate;
        Ok(())
    }

    /// Apply signed heat in joules to cells, atomically across the batch.
    /// Returns the actual representable net energy increment. External reservoirs
    /// must book the opposite increment. This operation supplies no heat-transfer law.
    pub fn add_heat_batch(&mut self, heat: &[(usize, f64)]) -> Result<f64, &'static str> {
        let mut candidate = self.clone();
        let mut transferred = 0.;
        for &(cell, increment) in heat {
            transferred += candidate.add_heat_staged_cell(cell, increment)?;
        }
        if !transferred.is_finite() {
            return Err("film heat transfer overflow");
        }
        candidate.temperatures()?;
        *self = candidate;
        Ok(transferred)
    }

    /// Used only on an outer transaction's private candidate. Checks this cell
    /// before mutation; batch and cross-owner callers retain publication control.
    pub(crate) fn add_heat_staged_cell(
        &mut self,
        cell: usize,
        increment: f64,
    ) -> Result<f64, &'static str> {
        if !increment.is_finite() {
            return Err("invalid film heat increment");
        }
        let previous = *self.energy.get(cell).ok_or("invalid film heat cell")?;
        let next = previous + increment;
        if !next.is_finite() || next < 0. {
            return Err("invalid film heat balance");
        }
        let capacity = self.capacity(&self.mixture.component_masses_kg()[cell])?;
        if capacity == 0. {
            if next != 0. {
                return Err("dry film has thermal energy");
            }
        } else if !(next / capacity).is_finite() || next / capacity <= 0. {
            return Err("invalid film temperature");
        }
        self.energy[cell] = next;
        Ok(next - previous)
    }
    /// Deposit mixtures at prescribed incoming temperatures (kelvin).
    /// All deposits commit together, including sensible heat. Zero-volume requests
    /// still validate composition and temperature, but add no heat.
    pub fn deposit_batch(
        &mut self,
        deposits: &[(usize, f64, Vec<f64>, f64)],
    ) -> Result<f64, &'static str> {
        let mut candidate = self.clone();
        let mut added = 0.;
        for (cell, volume, fractions, temperature) in deposits {
            if !temperature.is_finite() || *temperature <= 0. {
                return Err("invalid film deposit temperature");
            }
            let before = candidate
                .mixture
                .component_masses_kg()
                .get(*cell)
                .ok_or("invalid film deposit cell")?
                .clone();
            let bulk_before = candidate.mixture.film().volume[*cell];
            candidate.mixture.deposit(*cell, *volume, fractions)?;
            let after = &candidate.mixture.component_masses_kg()[*cell];
            let delta: Vec<_> = after.iter().zip(&before).map(|(a, b)| a - b).collect();
            let bulk_added = candidate.mixture.film().volume[*cell] - bulk_before;
            if delta.iter().all(|v| *v == 0.) {
                if bulk_added != 0. {
                    return Err("film deposit species change cannot be represented");
                }
                continue;
            }
            if bulk_added <= 0. {
                return Err("film deposit bulk change cannot be represented");
            }
            let energy = candidate.capacity(&delta)? * temperature;
            let previous = candidate.energy[*cell];
            let next = previous + energy;
            let actual = next - previous;
            if !energy.is_finite()
                || energy <= 0.
                || !next.is_finite()
                || actual <= 0.
                || (actual - energy).abs() > 16. * f64::EPSILON * next
            {
                return Err("film deposit energy cannot be represented");
            }
            candidate.energy[*cell] = next;
            added += actual;
        }
        if !added.is_finite() {
            return Err("film deposit thermal overflow");
        }
        candidate.temperatures()?;
        *self = candidate;
        Ok(added)
    }

    /// Species diffusion carries sensible heat at each species donor's temperature.
    /// Does not add Fourier conduction between cells.
    pub fn diffuse(
        &mut self,
        dt: f64,
        diffusivity: f64,
        max_substep: f64,
    ) -> Result<(), &'static str> {
        let mut candidate = self.clone();
        candidate.mixture.diffuse_with_energy(
            dt,
            diffusivity,
            max_substep,
            Some((&mut candidate.energy, &candidate.capacities)),
        )?;
        candidate.temperatures()?;
        *self = candidate;
        Ok(())
    }

    /// Transport sensible energy with the existing shear-driven donor flux.
    /// Prescribed shear supplies no mechanical-work or viscous-heating ledger here.
    pub fn step_with_surface_shear(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        traction: &[[f64; 3]],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        let mut candidate = self.clone();
        candidate.mixture.step_with_shear_energy(
            dt,
            gravity,
            traction,
            max_substep,
            &mut candidate.energy,
        )?;
        candidate.temperatures()?;
        *self = candidate;
        Ok(())
    }

    /// Advect sensible energy with the same donor-limited volume/species flux.
    /// No viscous heating or substrate heat exchange is added by this operation.
    pub fn step_with_advection(
        &mut self,
        dt: f64,
        gravity: [f64; 3],
        velocity: &[[f64; 3]],
        max_substep: f64,
    ) -> Result<(), &'static str> {
        let mut candidate = self.clone();
        candidate.mixture.step_with_energy(
            dt,
            gravity,
            velocity,
            max_substep,
            &mut candidate.energy,
        )?;
        candidate.temperatures()?;
        *self = candidate;
        Ok(())
    }

    /// Remove species at each donor's current temperature, atomically across all cells.
    /// Destination coupling must credit both the inventory and sensible energy.
    pub fn withdraw_components_batch(
        &mut self,
        requests: &[(usize, Vec<f64>)],
    ) -> Result<ThermalFilmWithdrawal, &'static str> {
        let temperatures = self.temperatures()?;
        let mut candidate = self.clone();
        let inventory = candidate.mixture.withdraw_components_batch(requests)?;
        let mut transferred = 0.;
        for cell in 0..self.energy.len() {
            let before = &self.mixture.component_masses_kg()[cell];
            let after = &candidate.mixture.component_masses_kg()[cell];
            if before == after {
                continue;
            }
            let temperature = temperatures[cell].ok_or("dry film withdrawal")?;
            let removed: Vec<_> = before.iter().zip(after).map(|(a, b)| a - b).collect();
            let carried = self.capacity(&removed)? * temperature;
            let remaining_capacity = candidate.capacity(after)?;
            let remaining = if remaining_capacity == 0. {
                0.
            } else {
                remaining_capacity * temperature
            };
            let delta = self.energy[cell] - remaining;
            let tolerance = 32. * f64::EPSILON * self.energy[cell];
            if !carried.is_finite()
                || !remaining.is_finite()
                || delta <= 0.
                || (delta - carried).abs() > tolerance
            {
                return Err("film thermal withdrawal cannot be represented");
            }
            candidate.energy[cell] = remaining;
            transferred += delta;
        }
        if !transferred.is_finite() {
            return Err("film thermal transfer overflow");
        }
        candidate.temperatures()?;
        *self = candidate;
        Ok(ThermalFilmWithdrawal {
            inventory,
            sensible_energy_j: transferred,
        })
    }
}

#[path = "surface_film_thermal_vapor.rs"]
mod vapor;
