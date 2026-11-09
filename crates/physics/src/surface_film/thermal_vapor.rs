//! Film adapter to the existing finite-cell solution vapor integrator.
use super::{FilmSubstrateHeat, ThermalFilmMixture};
use crate::liquid::evaporation::{State, advance};
use crate::liquid::{SolutionVaporInterface, VaporCell, VaporExchangeAccuracy};
impl ThermalFilmMixture {
    /// Symmetric contact/vapor/contact coupling with finite substrate heat.
    /// Film, substrate, velocities and shared vapor publish as one transaction.
    /// Refine dt for coupling/splitting error in addition to vapor tolerances.
    /// This inherits wet-cell/constant-latent restrictions of the vapor solver;
    /// it does not implement complete drying or dry-surface nucleation.
    pub fn exchange_solution_vapor_with_substrate(
        &mut self,
        substrate: FilmSubstrateHeat<'_>,
        velocities: &mut [[f64; 3]],
        vapor: &mut VaporCell,
        requests: &[(usize, &SolutionVaporInterface)],
        dt: f64,
        accuracy: VaporExchangeAccuracy,
    ) -> Result<f64, &'static str> {
        if !dt.is_finite() || dt <= 0. || 0.5 * dt == 0. {
            return Err("invalid coupled film vapor timestep");
        }
        let mut candidate = self.clone();
        let mut staged_substrate = substrate.energies_j.to_vec();
        let mut staged_velocities = velocities.to_vec();
        let mut staged_vapor = *vapor;
        candidate.exchange_substrate_heat(
            &mut staged_substrate,
            substrate.capacities_j_per_k,
            substrate.conductances_w_per_k,
            0.5 * dt,
        )?;
        let transferred = candidate.exchange_solution_vapor_batch(
            &mut staged_velocities,
            &mut staged_vapor,
            requests,
            dt,
            accuracy,
        )?;
        candidate.exchange_substrate_heat(
            &mut staged_substrate,
            substrate.capacities_j_per_k,
            substrate.conductances_w_per_k,
            0.5 * dt,
        )?;
        *self = candidate;
        substrate.energies_j.copy_from_slice(&staged_substrate);
        velocities.copy_from_slice(&staged_velocities);
        *vapor = staged_vapor;
        Ok(transferred)
    }
    /// Exchange several distinct wet cells with one shared pure-vapor reservoir.
    /// Symmetric forward/reverse half-step sweeps give each interface total dt.
    /// This is spatial operator splitting, not a simultaneous interface solve.
    /// Local solver tolerances do not bound splitting error; refine dt as well.
    /// All cells, velocities and vapor publish together or roll back together.
    /// Models must describe the same volatile species/thermodynamic reference;
    /// exposed areas and accommodation coefficients may differ by cell.
    pub fn exchange_solution_vapor_batch(
        &mut self,
        velocities: &mut [[f64; 3]],
        vapor: &mut VaporCell,
        requests: &[(usize, &SolutionVaporInterface)],
        dt: f64,
        accuracy: VaporExchangeAccuracy,
    ) -> Result<f64, &'static str> {
        if velocities.len() != self.energy.len()
            || velocities.iter().flatten().any(|v| !v.is_finite())
            || !dt.is_finite()
            || dt <= 0.
        {
            return Err("invalid film vapor batch controls");
        }
        let mut cells = std::collections::BTreeSet::new();
        if let Some((_, reference)) = requests.first() {
            for (cell, model) in requests {
                if !cells.insert(*cell) {
                    return Err("duplicate film vapor cell");
                }
                if model.solvent != reference.solvent
                    || model.molar_masses != reference.molar_masses
                    || model.interface.curve != reference.interface.curve
                {
                    return Err("incompatible shared film vapor species");
                }
            }
        }
        let mut candidate = self.clone();
        let mut staged_velocities = velocities.to_vec();
        let mut staged_vapor = *vapor;
        for (cell, model) in requests.iter().chain(requests.iter().rev()) {
            let velocity = staged_velocities
                .get_mut(*cell)
                .ok_or("invalid film vapor cell")?;
            candidate.exchange_solution_vapor(
                *cell,
                velocity,
                &mut staged_vapor,
                model,
                0.5 * dt,
                accuracy,
            )?;
        }
        let transferred = staged_vapor.mass - vapor.mass;
        if !transferred.is_finite() {
            return Err("film vapor batch overflow");
        }
        *self = candidate;
        velocities.copy_from_slice(&staged_velocities);
        *vapor = staged_vapor;
        Ok(transferred)
    }

    /// Exchange a wet cell's solvent with a finite pure-vapor reservoir.
    /// Film, cell velocity and vapor commit together. Velocity is the externally
    /// owned lumped cell velocity; this does not advance film mechanical motion.
    /// Uses the shared adaptive solution solver, including latent heat and donor
    /// momentum. Nonvolatile species remain. Complete solvent exhaustion and
    /// nucleation on a dry cell are not supported by this integrator yet.
    pub fn exchange_solution_vapor(
        &mut self,
        cell: usize,
        velocity: &mut [f64; 3],
        vapor: &mut VaporCell,
        model: &SolutionVaporInterface,
        dt: f64,
        accuracy: VaporExchangeAccuracy,
    ) -> Result<f64, &'static str> {
        let row = self
            .mixture
            .component_masses_kg()
            .get(cell)
            .ok_or("invalid film vapor cell")?;
        let temperature = self
            .temperatures()?
            .get(cell)
            .copied()
            .flatten()
            .ok_or("dry film vapor exchange")?;
        let fractions = self.mixture.fractions();
        let (solvent_weight, solute_weight) = model
            .weights(&fractions[cell])
            .map_err(|_| "invalid film vapor solution")?;
        let density = self.mixture.film().material.density;
        let mass = row.iter().sum::<f64>();
        let solvent_mass = row[model.solvent];
        let solute_capacity = row
            .iter()
            .zip(&self.capacities)
            .enumerate()
            .filter(|(k, _)| *k != model.solvent)
            .map(|(_, (m, c))| m * c)
            .sum::<f64>();
        let initial = State {
            mass,
            solvent_mass,
            solvent_weight,
            solvent_specific_heat: self.capacities[model.solvent],
            solute_capacity,
            solute_weight: mass * solute_weight,
            temperature,
            velocity: *velocity,
            vapor: *vapor,
        };
        let updated = advance(initial, model.interface, dt, accuracy)
            .map_err(|_| "film vapor exchange failed")?;
        let mut candidate = self.clone();
        let requested = initial.solvent_mass - updated.solvent_mass;
        if requested > 0. {
            let mut amounts = vec![0.; row.len()];
            amounts[model.solvent] = requested;
            candidate
                .mixture
                .withdraw_component_masses_batch(&[(cell, amounts)])?;
        } else if requested < 0. {
            let mut amounts = vec![0.; row.len()];
            amounts[model.solvent] = -requested;
            candidate
                .mixture
                .deposit_component_masses_batch(&[(cell, amounts)])?;
        }
        let new_row = &candidate.mixture.component_masses_kg()[cell];
        let new_mass = new_row.iter().sum::<f64>();
        let bulk_before = self.mixture.film().volume[cell] * density;
        let bulk_after = candidate.mixture.film().volume[cell] * density;
        let actual = row[model.solvent] - new_row[model.solvent];
        if (actual == 0. && updated.vapor.mass != vapor.mass)
            || (actual != 0. && updated.vapor.mass == vapor.mass)
            || (actual != 0. && bulk_before == bulk_after)
        {
            return Err("film vapor mass change cannot be represented");
        }
        candidate.energy[cell] = candidate.capacity(new_row)? * updated.temperature;
        candidate.temperatures()?;
        let kinetic = |v: [f64; 3]| 0.5 * v.iter().map(|x| x * x).sum::<f64>();
        let before = self.energy[cell]
            + mass * kinetic(*velocity)
            + vapor
                .energy(model.interface.curve.latent_heat)
                .map_err(|_| "invalid film vapor energy")?;
        let after = candidate.energy[cell]
            + new_mass * kinetic(updated.velocity)
            + updated
                .vapor
                .energy(model.interface.curve.latent_heat)
                .map_err(|_| "invalid film vapor energy")?;
        let close = |a: f64, b: f64| {
            a.is_finite()
                && b.is_finite()
                && (a - b).abs() <= 256. * f64::EPSILON * a.abs().max(b.abs())
        };
        if !close(mass + vapor.mass, new_mass + updated.vapor.mass)
            || !close(bulk_before + vapor.mass, bulk_after + updated.vapor.mass)
            || !close(before, after)
        {
            return Err("film vapor balance cannot be represented");
        }
        for axis in 0..3 {
            let p0 = mass * velocity[axis] + vapor.mass * vapor.velocity[axis];
            let p1 = new_mass * updated.velocity[axis]
                + updated.vapor.mass * updated.vapor.velocity[axis];
            let scale = (mass * velocity[axis]).abs()
                + (vapor.mass * vapor.velocity[axis]).abs()
                + (new_mass * updated.velocity[axis]).abs()
                + (updated.vapor.mass * updated.vapor.velocity[axis]).abs();
            if !p0.is_finite()
                || !p1.is_finite()
                || !scale.is_finite()
                || (p0 - p1).abs() > 256. * f64::EPSILON * scale
            {
                return Err("film vapor momentum cannot be represented");
            }
        }
        let transferred = updated.vapor.mass - vapor.mass;
        *self = candidate;
        *velocity = updated.velocity;
        *vapor = updated.vapor;
        Ok(transferred)
    }
}
