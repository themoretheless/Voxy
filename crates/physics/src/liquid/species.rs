//! Complete mass-fraction vectors transported with particles and conservative diffusion.
use super::{Error, Liquid, positive};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SpeciesState {
    pub names: Vec<String>,
    pub fractions: Vec<Vec<f64>>,
    pub reaction: Option<super::reaction::Reaction>,
    pub kinetics: Option<super::reaction::ReactionKinetics>,
    pub reaction_accuracy: Option<super::reaction::ReactionAccuracy>,
    pub properties: Option<super::mixture_properties::SpeciesProperties>,
    pub heat_capacities: Option<Vec<f64>>,
}
pub(super) fn validate_row(row: &[f64], count: usize) -> Result<(), Error> {
    if row.len() != count
        || row
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        return Err(Error::InvalidTransport);
    }
    let sum: f64 = row.iter().sum();
    if (sum - 1.0).abs()
        > 64.0
            * f64::EPSILON
            * f64::from(u32::try_from(count).map_err(|_| Error::InvalidTransport)?)
    {
        return Err(Error::InvalidTransport);
    }
    Ok(())
}
impl SpeciesState {
    pub fn validate(&self, particles: usize) -> Result<(), Error> {
        if self.names.is_empty() || self.names.len() > 64 || self.fractions.len() != particles {
            return Err(Error::InvalidTransport);
        }
        for row in &self.fractions {
            validate_row(row, self.names.len())?;
        }
        Ok(())
    }
    pub fn exchange(
        &mut self,
        i: usize,
        j: usize,
        first_mass: f64,
        second_mass: f64,
        conductance: f64,
        dt: f64,
    ) -> Result<(), Error> {
        for component in 0..self.names.len() {
            let (first, second) = super::transport::exchange(
                self.fractions[i][component],
                self.fractions[j][component],
                first_mass,
                second_mass,
                conductance,
                dt,
            )?;
            self.fractions[i][component] = first;
            self.fractions[j][component] = second;
        }
        Ok(())
    }
}
impl Liquid {
    /// Configures complete species vectors (including carrier), in fixed named order.
    /// At most 64 components. All species share the material diffusivity/mixing group;
    /// unequal individual diffusion needs a separate coupled transport closure.
    /// Neither the EOS nor temperature is changed by this passive composition setup.
    /// # Errors
    /// Missing transport, invalid names/counts, fractions outside [0,1], sum not one
    /// or unrepresentable total component masses. Atomic.
    pub fn configure_species(
        &mut self,
        names: Vec<String>,
        fractions: Vec<Vec<f64>>,
    ) -> Result<(), Error> {
        if names.iter().any(|name| name.trim().is_empty())
            || names.iter().collect::<BTreeSet<_>>().len() != names.len()
        {
            return Err(Error::InvalidTransport);
        }
        if self
            .transport
            .as_ref()
            .and_then(|t| t.species.as_ref())
            .is_some_and(|s| {
                s.reaction.is_some() || s.properties.is_some() || s.heat_capacities.is_some()
            })
        {
            return Err(Error::InvalidTransport);
        }
        let species = SpeciesState {
            names,
            fractions,
            reaction: None,
            kinetics: None,
            reaction_accuracy: None,
            properties: None,
            heat_capacities: None,
        };
        species.validate(self.particles.len())?;
        let mut candidate = self.clone();
        candidate
            .transport
            .as_mut()
            .ok_or(Error::InvalidTransport)?
            .species = Some(species);
        candidate.species_totals()?;
        *self = candidate;
        Ok(())
    }
    #[must_use]
    pub fn species_names(&self) -> Option<&[String]> {
        self.transport
            .as_ref()?
            .species
            .as_ref()
            .map(|s| s.names.as_slice())
    }
    #[must_use]
    pub fn species_fractions(&self) -> Option<&[Vec<f64>]> {
        self.transport
            .as_ref()?
            .species
            .as_ref()
            .map(|s| s.fractions.as_slice())
    }
    /// Total component masses, in species order. No chemical mass conversion occurs.
    /// # Errors
    /// Invalid composition state or overflow of a total mass.
    pub fn species_totals(&self) -> Result<Option<Vec<f64>>, Error> {
        let Some(species) = self.transport.as_ref().and_then(|t| t.species.as_ref()) else {
            return Ok(None);
        };
        species.validate(self.particles.len())?;
        let mut totals = vec![0.0; species.names.len()];
        for (particle, row) in self.particles.iter().zip(&species.fractions) {
            if !positive(particle.mass) {
                return Err(Error::NumericalFailure);
            }
            for (total, fraction) in totals.iter_mut().zip(row) {
                *total += particle.mass * fraction;
                if !total.is_finite() {
                    return Err(Error::NumericalFailure);
                }
            }
        }
        Ok(Some(totals))
    }
}
