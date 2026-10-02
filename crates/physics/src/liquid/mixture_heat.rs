//! Additive species sensible enthalpy and conservative enthalpy carried by diffusion.
use super::{Error, Liquid, Particle, positive, transport::Transport};
impl Liquid {
    /// Configures constant component heat capacities in species order, J/(kg K).
    /// Each particle's stored thermal/latent energy is preserved; temperature and
    /// phase fraction are decoded with its new mass-weighted capacity. None restores
    /// material capacities with the same energy-preserving remap.
    /// # Errors
    /// Missing species, invalid lengths/capacities, active gas EOS, or invalid resulting
    /// thermal/mechanical state. Atomic. Full composition-dependent gas EOS remains open.
    pub fn configure_species_heat_capacities(
        &mut self,
        capacities: Option<Vec<f64>>,
    ) -> Result<(), Error> {
        let fields = self.transport.as_ref().ok_or(Error::InvalidTransport)?;
        let species = fields.species.as_ref().ok_or(Error::InvalidTransport)?;
        if let Some(values) = &capacities {
            if values.len() != species.names.len()
                || values.iter().any(|c| !positive(*c))
                || self.gas_active()
            {
                return Err(Error::InvalidTransport);
            }
        }
        let energies = self
            .particles
            .iter()
            .enumerate()
            .map(|(i, p)| fields.energy(p, &fields.fields[i], i))
            .collect::<Result<Vec<_>, _>>()?;
        let mut candidate = self.clone();
        let fields = candidate
            .transport
            .as_mut()
            .ok_or(Error::InvalidTransport)?;
        fields
            .species
            .as_mut()
            .ok_or(Error::InvalidTransport)?
            .heat_capacities = capacities;
        for (i, (p, energy)) in candidate.particles.iter().zip(energies).enumerate() {
            fields.set_energy(i, p, energy)?;
        }
        candidate.effective_materials()?;
        candidate.transport_totals()?;
        *self = candidate;
        Ok(())
    }
    /// Current specific heat of each particle, including optional species capacities.
    /// # Errors
    /// Invalid composition or unrepresentable capacity. None means transport is disabled.
    pub fn particle_specific_heats(&self) -> Result<Option<Vec<f64>>, Error> {
        let Some(fields) = &self.transport else {
            return Ok(None);
        };
        self.particles
            .iter()
            .enumerate()
            .map(|(i, p)| fields.specific_heat(i, p.material))
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }
}
impl Transport {
    pub(super) fn specific_heat(&self, index: usize, material: usize) -> Result<f64, Error> {
        if let Some(species) = &self.species {
            self.heat_capacity_for_row(&species.fractions[index], material)
        } else {
            Ok(self.materials[material].specific_heat)
        }
    }
    pub(super) fn heat_capacity_for_row(&self, row: &[f64], material: usize) -> Result<f64, Error> {
        let Some(capacities) = self
            .species
            .as_ref()
            .and_then(|s| s.heat_capacities.as_ref())
        else {
            return Ok(self.materials[material].specific_heat);
        };
        super::species::validate_row(row, capacities.len())?;
        let capacity = row.iter().zip(capacities).map(|(y, c)| y * c).sum::<f64>();
        if !positive(capacity) {
            return Err(Error::NumericalFailure);
        }
        Ok(capacity)
    }
    pub(super) fn exchange_species(
        &mut self,
        indices: [usize; 2],
        particles: &[Particle],
        conductance: f64,
        dt: f64,
    ) -> Result<(), Error> {
        let [i, j] = indices;
        let a = particles[i];
        let b = particles[j];
        let Some(species) = self.species.as_ref() else {
            return Ok(());
        };
        let Some(capacities) = species.heat_capacities.clone() else {
            return self
                .species
                .as_mut()
                .ok_or(Error::InvalidTransport)?
                .exchange(i, j, a.mass, b.mass, conductance, dt);
        };
        let before = species.fractions[i].clone();
        let first_energy = self.energy(&a, &self.fields[i], i)?;
        let second_energy = self.energy(&b, &self.fields[j], j)?;
        let first_temperature = self.fields[i].temperature;
        let second_temperature = self.fields[j].temperature;
        let first_latent =
            first_energy / a.mass - self.specific_heat(i, a.material)? * first_temperature;
        let second_latent =
            second_energy / b.mass - self.specific_heat(j, b.material)? * second_temperature;
        self.species
            .as_mut()
            .ok_or(Error::InvalidTransport)?
            .exchange(i, j, a.mass, b.mass, conductance, dt)?;
        let after = &self
            .species
            .as_ref()
            .ok_or(Error::InvalidTransport)?
            .fractions[i];
        let carried = after
            .iter()
            .zip(&before)
            .zip(&capacities)
            .map(|((new, old), c)| {
                let mass = a.mass * (new - old);
                mass * if mass >= 0.0 {
                    c * second_temperature + second_latent
                } else {
                    c * first_temperature + first_latent
                }
            })
            .sum::<f64>();
        self.set_energy(i, &a, first_energy + carried)?;
        self.set_energy(j, &b, second_energy - carried)?;
        Ok(())
    }
}
