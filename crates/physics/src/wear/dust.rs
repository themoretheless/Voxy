use super::{Layer, Material, Removal};
use crate::suspension::Particle;

/// Removed mass already belongs to `particles`; do not also emit `wear.mass_kg`.
#[must_use]
#[derive(Debug)]
pub struct DustRemoval {
    pub wear: Removal,
    pub particles: Vec<Particle>,
    pub translational_kinetic_j: f64,
}

/// Caller-owned mechanical energy available for creating fresh particle surfaces.
/// Specific energy is per single surface area, not a two-face fracture toughness.
#[derive(Clone, Debug)]
pub struct SurfaceEnergy {
    available_j: f64,
    specific_j_m2: f64,
}
impl SurfaceEnergy {
    /// # Errors
    /// Nonfinite/negative energy or nonpositive specific surface energy.
    pub fn new(available_j: f64, specific_j_m2: f64) -> Result<Self, &'static str> {
        if !available_j.is_finite()
            || available_j < 0.
            || !specific_j_m2.is_finite()
            || specific_j_m2 <= 0.
        {
            return Err("invalid dust surface energy");
        }
        Ok(Self {
            available_j,
            specific_j_m2,
        })
    }
    #[must_use]
    pub fn available_j(&self) -> f64 {
        self.available_j
    }
}
#[must_use]
#[derive(Debug)]
pub struct SurfaceDustRemoval {
    pub dust: DustRemoval,
    pub created_surface_m2: f64,
    pub surface_energy_j: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct DustFriction {
    /// Magnitude of accepted resisting tangential force, not the normal load.
    pub tangential_force_n: f64,
    /// Specific energy for fresh single surface area; measured material input.
    pub surface_energy_j_m2: f64,
}
#[must_use]
#[derive(Debug)]
pub struct FrictionDustRemoval {
    pub formation: SurfaceDustRemoval,
    pub friction_work_j: f64,
    /// Remainder after new-surface creation; caller must deposit this in thermal state.
    pub friction_heat_j: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct ContactDustSettings<'a> {
    pub wear_material: Material,
    pub surface_energy_j_m2: f64,
    pub emission_positions_m: &'a [[f64; 3]],
    pub inherited_velocity_m_s: [f64; 3],
}
#[must_use]
#[derive(Debug)]
pub struct ContactDustRemoval {
    pub contact: crate::friction::Response,
    pub formation: SurfaceDustRemoval,
    pub friction_work_j: f64,
    pub friction_heat_j: f64,
}

/// Explicit destination for all physical friction heat left after surface creation.
/// Weights are per liquid particle and must sum to one; no spatial mapping implied.
#[derive(Debug)]
pub struct LiquidHeatSink<'a> {
    pub liquid: &'a mut crate::liquid::Liquid,
    pub weights: &'a [f64],
}

impl Layer {
    /// Contact, wear, energetic dust and weighted liquid enthalpy deposit as one
    /// transaction. Latent heat uses the existing liquid phase model. All reported
    /// friction heat is already deposited, and must not be deposited a second time.
    /// # Errors
    /// Invalid heat weights, missing transport, rejected thermal properties,
    /// unrepresentable heat transfer, or any contact/dust validation failure.
    #[allow(clippy::too_many_arguments)]
    pub fn advance_contact_dust_heated(
        &mut self,
        contact_material: crate::friction::Material,
        contact_state: &mut crate::friction::State,
        gap_m: [f64; 3],
        normal: [f64; 3],
        settings: ContactDustSettings<'_>,
        sink: LiquidHeatSink<'_>,
    ) -> Result<ContactDustRemoval, &'static str> {
        let sum: f64 = sink.weights.iter().sum();
        if sink.weights.len() != sink.liquid.particles().len()
            || sink.weights.iter().any(|w| !w.is_finite() || *w < 0.)
            || !sum.is_finite()
            || (sum - 1.).abs() > 1e-12
        {
            return Err("invalid contact dust heat partition");
        }
        let before = sink
            .liquid
            .transport_totals()
            .map_err(|_| "invalid contact heat sink")?
            .ok_or("contact heat sink requires transport")?
            .0;
        let buffered_before: f64 = sink
            .liquid
            .suspension_heat_buffer()
            .iter()
            .chain(sink.liquid.suspension_heat_correction())
            .sum();
        let mut layer = self.clone();
        let mut state = *contact_state;
        let report =
            layer.advance_contact_dust(contact_material, &mut state, gap_m, normal, settings)?;
        let heat: Vec<_> = sink
            .weights
            .iter()
            .map(|w| report.friction_heat_j * w / sum)
            .collect();
        let mut liquid = sink.liquid.clone();
        liquid
            .deposit_dissipation_heat(&heat)
            .map_err(|_| "contact friction heat deposit failed")?;
        let after = liquid
            .transport_totals()
            .map_err(|_| "invalid heated contact sink")?
            .ok_or("missing heated contact transport")?
            .0;
        let buffered_after: f64 = liquid
            .suspension_heat_buffer()
            .iter()
            .chain(liquid.suspension_heat_correction())
            .sum();
        let accepted = after - before + buffered_after - buffered_before;
        if (report.friction_heat_j > 0. && accepted <= 0.)
            || (accepted - report.friction_heat_j).abs()
                > 1e-10 * report.friction_heat_j.max(f64::MIN_POSITIVE)
                    + 16. * f64::EPSILON * (before.abs() + after.abs())
        {
            return Err("unrepresentable contact friction heat transfer");
        }
        *self = layer;
        *contact_state = state;
        *sink.liquid = liquid;
        Ok(report)
    }

    /// Couples a fixed-normal Coulomb contact trial to wear and energetic dust.
    /// Accepted contact pressure sets normal load; plastic slip sets wear distance.
    /// Only physical friction dissipation funds surface creation, never penalty
    /// spring storage, numerical loss or released spring energy. Contact history
    /// and layer commit together. Caller applies returned traction in mechanics.
    /// # Errors
    /// Invalid contact/dust, exhausted layer, insufficient physical work, or layer
    /// exhaustion before the contact increment ends (caller must subdivide).
    pub fn advance_contact_dust(
        &mut self,
        contact_material: crate::friction::Material,
        contact_state: &mut crate::friction::State,
        gap_m: [f64; 3],
        normal: [f64; 3],
        settings: ContactDustSettings<'_>,
    ) -> Result<ContactDustRemoval, &'static str> {
        if self.thickness_m() == 0. {
            return Err("contact dust layer is exhausted");
        }
        let (next_state, contact) = contact_material.response(contact_state, gap_m, normal)?;
        let work = (next_state.dissipated_j_m2() - contact_state.dissipated_j_m2()) * self.area_m2;
        let load = contact.pressure_pa * self.area_m2;
        if !work.is_finite() || work < 0. || !load.is_finite() {
            return Err("invalid contact dust work or load");
        }
        let mut bank = SurfaceEnergy::new(work, settings.surface_energy_j_m2)?;
        let mut candidate = self.clone();
        let formation = candidate.advance_dust_with_surface_energy(
            settings.wear_material,
            load,
            contact.slip_increment_m,
            settings.emission_positions_m,
            settings.inherited_velocity_m_s,
            &mut bank,
        )?;
        if formation.dust.wear.remaining_sliding_distance_m > 0. {
            return Err("contact dust increment crosses layer exhaustion; subdivide");
        }
        *self = candidate;
        *contact_state = next_state;
        Ok(ContactDustRemoval {
            contact,
            formation,
            friction_work_j: work,
            friction_heat_j: bank.available_j(),
        })
    }

    /// Partitions accepted sliding work into fresh-grain surface energy and heat.
    /// Work uses only distance consumed by this layer; leftover sliding belongs
    /// to the next substrate. Actual force/work removal from the mechanical
    /// contact remains the caller's responsibility; it is not applied twice here.
    /// # Errors
    /// Invalid force, insufficient work for the selected grain size, any dust
    /// failure or unrepresentable energy balance. Layer remains unchanged.
    pub fn advance_dust_friction(
        &mut self,
        material: Material,
        normal_load_n: f64,
        sliding_distance_m: f64,
        emission_positions_m: &[[f64; 3]],
        inherited_velocity_m_s: [f64; 3],
        friction: DustFriction,
    ) -> Result<FrictionDustRemoval, &'static str> {
        if !friction.tangential_force_n.is_finite() || friction.tangential_force_n < 0. {
            return Err("invalid dust friction force");
        }
        let mut candidate = self.clone();
        let preview = candidate.advance(material, normal_load_n, sliding_distance_m)?;
        let work = friction.tangential_force_n * preview.consumed_sliding_distance_m;
        if !work.is_finite()
            || (friction.tangential_force_n > 0.
                && preview.consumed_sliding_distance_m > 0.
                && work == 0.)
        {
            return Err("unrepresentable dust friction work");
        }
        let mut bank = SurfaceEnergy::new(work, friction.surface_energy_j_m2)?;
        candidate = self.clone();
        let formation = candidate.advance_dust_with_surface_energy(
            material,
            normal_load_n,
            sliding_distance_m,
            emission_positions_m,
            inherited_velocity_m_s,
            &mut bank,
        )?;
        let heat = bank.available_j();
        if (heat + formation.surface_energy_j - work).abs() > 1e-10 * work.max(f64::MIN_POSITIVE) {
            return Err("dust friction energy balance failure");
        }
        *self = candidate;
        Ok(FrictionDustRemoval {
            formation,
            friction_work_j: work,
            friction_heat_j: heat,
        })
    }

    /// Atomic wear/dust formation with a finite surface-creation energy bank.
    /// Charges every spherical grain's complete surface as fresh area. Existing
    /// free surfaces are not credited; this is an explicit all-fresh model.
    /// Inherited translation is carried with removed mass, not bought from this
    /// bank. The caller must supply energy from actual mechanical work and retain
    /// the returned surface energy rather than also treating it as heat.
    /// # Errors
    /// Dust validation, insufficient energy, overflow or unrepresentable debit.
    /// Both layer and bank remain unchanged on failure.
    pub fn advance_dust_with_surface_energy(
        &mut self,
        material: Material,
        normal_load_n: f64,
        sliding_distance_m: f64,
        emission_positions_m: &[[f64; 3]],
        inherited_velocity_m_s: [f64; 3],
        bank: &mut SurfaceEnergy,
    ) -> Result<SurfaceDustRemoval, &'static str> {
        let mut candidate = self.clone();
        let dust = candidate.advance_dust(
            material,
            normal_load_n,
            sliding_distance_m,
            emission_positions_m,
            inherited_velocity_m_s,
        )?;
        let area: f64 = dust.particles.iter().map(Particle::surface_area_m2).sum();
        let energy = area * bank.specific_j_m2;
        let remaining = bank.available_j - energy;
        if !area.is_finite()
            || !energy.is_finite()
            || remaining < 0.
            || (area > 0. && energy <= 0.)
            || (energy > 0. && remaining >= bank.available_j)
            || (bank.available_j - remaining - energy).abs() > 1e-10 * energy.max(f64::MIN_POSITIVE)
        {
            return Err("insufficient or unrepresentable dust surface energy");
        }
        *self = candidate;
        bank.available_j = remaining;
        Ok(SurfaceDustRemoval {
            dust,
            created_surface_m2: area,
            surface_energy_j: energy,
        })
    }

    /// Removes material and partitions it into equal spherical dry grains.
    /// Caller supplies physical emission positions and common inherited velocity.
    /// Grain count sets the size distribution; no calibrated fragmentation law,
    /// ejection impulse, rotation, surface-creation work or pore water is included.
    /// No material is removed unless the entire emitted inventory is valid.
    /// # Errors
    /// Invalid wear, >4096 grains, missing positions for positive removal,
    /// unrepresentable geometry, mass/volume mismatch or nonfinite kinetic energy.
    pub fn advance_dust(
        &mut self,
        material: Material,
        normal_load_n: f64,
        sliding_distance_m: f64,
        emission_positions_m: &[[f64; 3]],
        inherited_velocity_m_s: [f64; 3],
    ) -> Result<DustRemoval, &'static str> {
        if emission_positions_m.len() > 4096
            || !emission_positions_m
                .iter()
                .flatten()
                .chain(inherited_velocity_m_s.iter())
                .all(|x| x.is_finite())
        {
            return Err("invalid wear dust emission settings");
        }
        let mut candidate = self.clone();
        let wear = candidate.advance(material, normal_load_n, sliding_distance_m)?;
        let mut particles = Vec::new();
        let kinetic =
            0.5 * wear.mass_kg * inherited_velocity_m_s.iter().map(|v| v * v).sum::<f64>();
        if !kinetic.is_finite() {
            return Err("wear dust kinetic energy overflow");
        }
        if wear.volume_m3 > 0. {
            if emission_positions_m.is_empty() {
                return Err("missing wear dust emission positions");
            }
            let density = wear.mass_kg / wear.volume_m3;
            let volume = wear.volume_m3 / emission_positions_m.len() as f64;
            let radius = (volume / ((4. / 3.) * std::f64::consts::PI)).cbrt();
            for position in emission_positions_m {
                particles.push(Particle::new(
                    radius,
                    density,
                    *position,
                    inherited_velocity_m_s,
                )?);
            }
            let mass: f64 = particles.iter().map(Particle::mass_kg).sum();
            let volume: f64 = particles.iter().map(Particle::volume_m3).sum();
            if (mass - wear.mass_kg).abs() > 1e-10 * wear.mass_kg
                || (volume - wear.volume_m3).abs() > 1e-10 * wear.volume_m3
            {
                return Err("wear dust mass or volume balance failure");
            }
        }
        *self = candidate;
        Ok(DustRemoval {
            wear,
            particles,
            translational_kinetic_j: kinetic,
        })
    }
}
