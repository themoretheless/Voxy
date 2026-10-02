//! Conservative prescribed coalescence; contact/onset scheduling is a separate operator.
use super::{Error, Liquid, LiquidField, Particle, ParticleInput, finite, positive};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropletMergeReport {
    pub removed: usize,
    pub particle_index: usize,
    pub released_surface_energy: f64,
    /// Free energy released by volume-weighted mixing of polymer conformations.
    /// External ledger like the surface release; not automatically deposited as heat.
    pub released_polymer_energy: f64,
    /// Relative translational energy removed from resolved motion; caller allocates
    /// it to internal rotation/deformation/heat along with the angular ledger.
    pub unresolved_kinetic_energy: f64,
    /// Orbital angular momentum no longer represented by the single point particle.
    /// Caller must carry this in an unresolved rotation ledger, not discard it.
    pub unresolved_angular_momentum: [f64; 3],
}
impl Liquid {
    /// Prescribed merging of 2..=64 distinct indices into one equivalent sphere.
    /// Preserves mass, center of mass, linear momentum, sensible heat, dissolved
    /// mass and each species mass. Surface release and unresolved motion are external
    /// energy ledgers; temperature is not heated a second time. No contact or
    /// coalescence criterion is inferred. Same material, no phase model or gas.
    /// Rejects constitutive changes that violate additive volume. Atomic.
    pub fn merge_droplets(
        &mut self,
        indices: &[usize],
        surface_tension: f64,
    ) -> Result<DropletMergeReport, Error> {
        if !(2..=64).contains(&indices.len())
            || !surface_tension.is_finite()
            || surface_tension < 0.0
            || self.gas_active()
            || self.phase_fractions().is_some()
        {
            return Err(Error::InvalidConfig);
        }
        let mut mask = vec![false; self.particles.len()];
        let first = *indices.first().ok_or(Error::InvalidParticle)?;
        let material = self
            .particles
            .get(first)
            .ok_or(Error::InvalidParticle)?
            .material;
        for &i in indices {
            let marked = mask.get_mut(i).ok_or(Error::InvalidParticle)?;
            if *marked || self.particles[i].material != material {
                return Err(Error::InvalidParticle);
            }
            *marked = true;
        }
        let mass: f64 = indices.iter().map(|i| self.particles[*i].mass).sum();
        if !positive(mass) {
            return Err(Error::NumericalFailure);
        }
        let center: [f64; 3] = std::array::from_fn(|k| {
            indices
                .iter()
                .map(|i| self.particles[*i].mass / mass * self.particles[*i].position[k])
                .sum()
        });
        let velocity: [f64; 3] = std::array::from_fn(|k| {
            indices
                .iter()
                .map(|i| self.particles[*i].mass / mass * self.particles[*i].velocity[k])
                .sum()
        });
        if !finite(center) || !finite(velocity) {
            return Err(Error::NumericalFailure);
        }
        let materials = self.effective_materials()?;
        let volume: f64 = indices
            .iter()
            .map(|i| self.particles[*i].mass / materials[*i].rest_density)
            .sum();
        let area = |volume: f64| {
            4.0 * std::f64::consts::PI
                * (3.0 * volume / (4.0 * std::f64::consts::PI)).cbrt().powi(2)
        };
        let old_area: f64 = indices
            .iter()
            .map(|i| area(self.particles[*i].mass / materials[*i].rest_density))
            .sum();
        let mut unresolved_kinetic_energy = 0.0;
        let mut angular = [0.0; 3];
        for &i in indices {
            let p = self.particles[i];
            let x: [f64; 3] = std::array::from_fn(|k| p.position[k] - center[k]);
            let v: [f64; 3] = std::array::from_fn(|k| p.velocity[k] - velocity[k]);
            unresolved_kinetic_energy += 0.5 * p.mass * v.iter().map(|v| v * v).sum::<f64>();
            for k in 0..3 {
                angular[k] +=
                    p.mass * (x[(k + 1) % 3] * v[(k + 2) % 3] - x[(k + 2) % 3] * v[(k + 1) % 3]);
            }
        }
        let row = self.species_fractions().map(|rows| {
            (0..rows[0].len())
                .map(|k| {
                    indices
                        .iter()
                        .map(|i| self.particles[*i].mass / mass * rows[*i][k])
                        .sum()
                })
                .collect::<Vec<f64>>()
        });
        let field = if let Some(fields) = &self.transport {
            let energy: f64 = indices
                .iter()
                .map(|i| fields.energy(&self.particles[*i], &fields.fields[*i], *i))
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .sum();
            let cp = if let Some(row) = &row {
                fields.heat_capacity_for_row(row, material)?
            } else {
                fields.materials[material].specific_heat
            };
            let concentration = indices
                .iter()
                .map(|i| self.particles[*i].mass / mass * fields.fields[*i].concentration)
                .sum();
            Some(LiquidField {
                temperature: energy / (mass * cp),
                concentration,
            })
        } else {
            None
        };
        let structure: f64 = indices
            .iter()
            .map(|i| self.particles[*i].mass / mass * self.structure[*i])
            .sum();
        let merge_buffer = |buffer: &[f64]| -> Result<Vec<f64>, Error> {
            if buffer.len() != self.particles.len() && buffer.iter().any(|v| *v != 0.0) {
                return Err(Error::NumericalFailure);
            }
            let mut result: Vec<_> = mask
                .iter()
                .enumerate()
                .filter_map(|(i, remove)| (!remove).then(|| buffer.get(i).copied().unwrap_or(0.0)))
                .collect();
            result.push(
                indices
                    .iter()
                    .map(|i| buffer.get(*i).copied().unwrap_or(0.0))
                    .sum(),
            );
            if result.iter().any(|v| !v.is_finite()) {
                return Err(Error::NumericalFailure);
            }
            Ok(result)
        };
        let heat = merge_buffer(&self.suspension_heat_buffer)?;
        let correction = merge_buffer(&self.suspension_heat_correction)?;
        let source = ParticleInput {
            particle: Particle {
                position: center,
                velocity,
                mass,
                material,
            },
            field,
            phase_fraction: None,
        };
        let mut candidate = self.clone();
        if let Some(row) = row {
            candidate.exchange_particles_with_species(indices, &[source], &[row])?;
        } else {
            candidate.exchange_particles(indices, &[source])?;
        }
        let index = candidate.particles.len() - 1;
        candidate.structure[index] = structure;
        if let Some(flags) = &mut candidate.droplet_population {
            flags[index] = true;
        }
        candidate.suspension_heat_buffer = heat;
        candidate.suspension_heat_correction = correction;
        let properties = self.effective_materials()?;
        let old_polymer_energy: f64 = indices
            .iter()
            .map(|i| self.polymer_particle_energy(*i, &self.particles[*i], properties[*i]))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .sum();
        if self.maxwell_fluids[material].is_some() {
            candidate.conformation[index] = std::array::from_fn(|a| {
                std::array::from_fn(|b| {
                    indices
                        .iter()
                        .map(|i| {
                            self.particles[*i].mass / properties[*i].rest_density / volume
                                * self.conformation[*i][a][b]
                        })
                        .sum()
                })
            });
        }
        let density = candidate.effective_materials()?[index].rest_density;
        let new_volume = mass / density;
        let new_polymer_energy = candidate.polymer_particle_energy(
            index,
            &candidate.particles[index],
            candidate.effective_materials()?[index],
        )?;
        let released_polymer_energy = old_polymer_energy - new_polymer_energy;
        if !released_polymer_energy.is_finite()
            || released_polymer_energy < -1e-12 * old_polymer_energy.max(1e-30)
        {
            return Err(Error::NumericalFailure);
        }
        let released_surface_energy = surface_tension * (old_area - area(new_volume));
        if !positive(volume)
            || !positive(new_volume)
            || (new_volume - volume).abs() > 1e-10 * volume
            || !unresolved_kinetic_energy.is_finite()
            || !finite(angular)
            || !released_surface_energy.is_finite()
            || released_surface_energy < 0.0
        {
            return Err(Error::NumericalFailure);
        }
        *self = candidate;
        Ok(DropletMergeReport {
            removed: indices.len(),
            particle_index: index,
            released_surface_energy,
            released_polymer_energy: released_polymer_energy.max(0.0),
            unresolved_kinetic_energy,
            unresolved_angular_momentum: angular,
        })
    }
}
