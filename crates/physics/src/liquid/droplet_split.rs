//! Energy-budgeted spherical-droplet fragmentation primitive; not a breakup criterion.
use super::{Error, Liquid, ParticleInput, finite, norm, positive};
/// Explicit external impact/breakup energy in joules. Numerical SPH particles are
/// treated as equivalent-volume spherical droplets only for this requested operation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropletSplit {
    pub children: usize,
    /// Normal to the ring of fragments; ordinarily the substrate impact normal.
    pub axis: [f64; 3],
    /// Requested minimum resampling radius in metres. Expanded when necessary
    /// to keep the equivalent-volume fragment spheres disjoint on the ring.
    /// This is not an inferred aerodynamic breakup size.
    pub position_radius: f64,
    /// Surface tension in N/m; new spherical surface consumes the available budget.
    pub surface_tension: f64,
    /// Supplied external energy: surface creation first, remaining energy becomes
    /// zero-mean fragment motion. Caller must debit its impact/energy reservoir.
    pub available_energy: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropletSplitReport {
    pub children: usize,
    /// Actual ring radius after the non-overlap bound, metres.
    pub position_radius: f64,
    pub created_surface_energy: f64,
    pub added_kinetic_energy: f64,
}
pub(super) fn weighted_masses(total: f64, fractions: &[f64]) -> Result<Vec<f64>, Error> {
    if !(2..=64).contains(&fractions.len()) || fractions.iter().any(|v| !positive(*v)) {
        return Err(Error::InvalidConfig);
    }
    let sum: f64 = fractions.iter().sum();
    if !sum.is_finite() || (sum - 1.0).abs() > 1e-12 {
        return Err(Error::InvalidConfig);
    }
    let mut masses: Vec<_> = fractions[..fractions.len() - 1]
        .iter()
        .map(|y| total * (y / sum))
        .collect();
    masses.push(total - masses.iter().sum::<f64>());
    if masses.iter().any(|v| !positive(*v)) {
        return Err(Error::NumericalFailure);
    }
    Ok(masses)
}
impl Liquid {
    /// Equivalent-volume spherical radii at each particle's current density.
    /// This geometric interpretation does not turn a WCSPH sample into a resolved droplet.
    pub fn equivalent_sphere_radii(&self) -> Result<Vec<f64>, Error> {
        if self.gas_active() {
            return Err(Error::InvalidConfig);
        }
        self.particles
            .iter()
            .zip(self.effective_materials()?)
            .map(|(p, m)| {
                let radius =
                    (3.0 * (p.mass / m.rest_density) / (4.0 * std::f64::consts::PI)).cbrt();
                if positive(radius) {
                    Ok(radius)
                } else {
                    Err(Error::NumericalFailure)
                }
            })
            .collect()
    }
    /// Smallest ring radius allowing equal-mass equivalent-volume spheres to
    /// touch without overlapping. The rounding-remainder child is included.
    /// This is geometric packing only; no hydrodynamic size distribution is inferred.
    pub fn droplet_fragment_ring_radius(
        &self,
        index: usize,
        children: usize,
    ) -> Result<f64, Error> {
        if !(2..=64).contains(&children) || self.gas_active() {
            return Err(Error::InvalidConfig);
        }
        let parent = self.particles.get(index).ok_or(Error::InvalidParticle)?;
        let density = self.effective_materials()?[index].rest_density;
        let part = parent.mass / children as f64;
        let remainder = parent.mass - part * (children - 1) as f64;
        let radius = (3.0 * (part.max(remainder) / density) / (4.0 * std::f64::consts::PI)).cbrt();
        let ring = radius / (std::f64::consts::PI / children as f64).sin() * (1.0 + 1e-12);
        if !positive(part) || !positive(remainder) || !positive(ring) {
            return Err(Error::NumericalFailure);
        }
        Ok(ring)
    }
    /// Additional equivalent-spherical surface energy needed for equal-mass
    /// fragmentation. This query supplies an energetic bound, not a measured
    /// impact breakup criterion. No fluid state is changed.
    pub fn droplet_fragment_surface_energy(
        &self,
        index: usize,
        children: usize,
        surface_tension: f64,
    ) -> Result<f64, Error> {
        if !(2..=64).contains(&children)
            || !surface_tension.is_finite()
            || surface_tension < 0.0
            || self.gas_active()
        {
            return Err(Error::InvalidConfig);
        }
        let parent = self.particles.get(index).ok_or(Error::InvalidParticle)?;
        let density = self.effective_materials()?[index].rest_density;
        let radius = (3.0 * (parent.mass / density) / (4.0 * std::f64::consts::PI)).cbrt();
        let area = 4.0 * std::f64::consts::PI * radius * radius;
        let surface = surface_tension * area * ((children as f64).cbrt() - 1.0);
        if !surface.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(surface)
    }
    /// Surface creation cost for explicitly supplied normalized positive mass fractions.
    /// The distribution is caller supplied, not inferred from a splash correlation.
    pub fn droplet_fragment_surface_energy_with_mass_fractions(
        &self,
        index: usize,
        fractions: &[f64],
        surface_tension: f64,
    ) -> Result<f64, Error> {
        if self.gas_active() || !surface_tension.is_finite() || surface_tension < 0.0 {
            return Err(Error::InvalidConfig);
        }
        let parent = self.particles.get(index).ok_or(Error::InvalidParticle)?;
        let masses = weighted_masses(parent.mass, fractions)?;
        let density = self.effective_materials()?[index].rest_density;
        let area = |mass: f64| {
            4.0 * std::f64::consts::PI
                * (3.0 * mass / (4.0 * std::f64::consts::PI * density))
                    .cbrt()
                    .powi(2)
        };
        let surface =
            surface_tension * (masses.iter().map(|m| area(*m)).sum::<f64>() - area(parent.mass));
        if !surface.is_finite() || surface < 0.0 {
            return Err(Error::NumericalFailure);
        }
        Ok(surface)
    }
    /// Pairwise non-overlap bound for a regular angular ring with unequal radii.
    /// Subtracting its mass-weighted center translates the ring without changing distances.
    pub fn droplet_fragment_ring_radius_with_mass_fractions(
        &self,
        index: usize,
        fractions: &[f64],
    ) -> Result<f64, Error> {
        if self.gas_active() {
            return Err(Error::InvalidConfig);
        }
        let parent = self.particles.get(index).ok_or(Error::InvalidParticle)?;
        let masses = weighted_masses(parent.mass, fractions)?;
        let density = self.effective_materials()?[index].rest_density;
        let radii: Vec<_> = masses
            .iter()
            .map(|m| (3.0 * m / (4.0 * std::f64::consts::PI * density)).cbrt())
            .collect();
        let mut ring: f64 = 0.0;
        for i in 0..radii.len() {
            for j in 0..i {
                let chord =
                    2.0 * (std::f64::consts::PI * (i - j) as f64 / radii.len() as f64).sin();
                ring = ring.max((radii[i] + radii[j]) / chord);
            }
        }
        ring *= 1.0 + 1e-12;
        if !positive(ring) {
            return Err(Error::NumericalFailure);
        }
        Ok(ring)
    }
    /// Replaces one particle with equal-mass fragments on a centered circular ring.
    /// Carries temperature, species, phase/pressure and structural state unchanged;
    /// conserves mass, center of mass, momentum and the stored thermal/chemical
    /// inventory. Increased surface plus kinetic energy equals the supplied budget.
    /// Children append after surviving particles, in deterministic angular order.
    ///
    /// No breakup threshold is inferred, and surface energy is reported rather than
    /// stored in the WCSPH constitutive model. Subsequent flow is still numerical
    /// particle flow, not a calibrated droplet drag/coalescence solver.
    /// # Errors
    /// Invalid controls/index, insufficient surface-creation energy, active gas,
    /// source/constitutive failure, overflow or particle budget. Entire fluid rolls back.
    pub fn split_droplet(
        &mut self,
        index: usize,
        model: DropletSplit,
    ) -> Result<DropletSplitReport, Error> {
        self.split_droplet_impl(index, model, None)
    }
    /// Unequal fragments with explicit positive mass fractions summing to one.
    /// Mass-weighted ring centering preserves center of mass and momentum;
    /// thermal, species and structural state follow each fragment's mass.
    pub fn split_droplet_with_mass_fractions(
        &mut self,
        index: usize,
        model: DropletSplit,
        fractions: &[f64],
    ) -> Result<DropletSplitReport, Error> {
        if fractions.len() != model.children {
            return Err(Error::InvalidConfig);
        }
        self.split_droplet_impl(index, model, Some(fractions))
    }
    fn split_droplet_impl(
        &mut self,
        index: usize,
        model: DropletSplit,
        fractions: Option<&[f64]>,
    ) -> Result<DropletSplitReport, Error> {
        if !(2..=64).contains(&model.children)
            || !finite(model.axis)
            || !positive(norm(model.axis))
            || !positive(model.position_radius)
            || !model.surface_tension.is_finite()
            || model.surface_tension < 0.0
            || !model.available_energy.is_finite()
            || model.available_energy < 0.0
            || self.gas_active()
        {
            return Err(Error::InvalidConfig);
        }
        let parent = *self.particles.get(index).ok_or(Error::InvalidParticle)?;
        if model.children - 1 > self.config.max_particles - self.particles.len() {
            return Err(Error::ParticleBudget);
        }
        let surface = if let Some(fractions) = fractions {
            self.droplet_fragment_surface_energy_with_mass_fractions(
                index,
                fractions,
                model.surface_tension,
            )?
        } else {
            self.droplet_fragment_surface_energy(index, model.children, model.surface_tension)?
        };
        let ring = if let Some(fractions) = fractions {
            self.droplet_fragment_ring_radius_with_mass_fractions(index, fractions)?
        } else {
            self.droplet_fragment_ring_radius(index, model.children)?
        };
        let position_radius = model.position_radius.max(ring);
        let kinetic = model.available_energy - surface;
        if !surface.is_finite() || !kinetic.is_finite() || kinetic < 0.0 {
            return Err(Error::NumericalFailure);
        }
        let axis = model.axis.map(|v| v / norm(model.axis));
        let least = (0..3)
            .min_by(|a, b| axis[*a].abs().total_cmp(&axis[*b].abs()))
            .unwrap_or(0);
        let mut reference = [0.0; 3];
        reference[least] = 1.0;
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let tangent = cross(axis, reference);
        let tangent = tangent.map(|v| v / norm(tangent));
        let bitangent = cross(axis, tangent);
        let mut directions: Vec<[f64; 3]> = (0..model.children)
            .map(|i| {
                let angle = std::f64::consts::TAU * i as f64 / model.children as f64;
                std::array::from_fn(|k| tangent[k] * angle.cos() + bitangent[k] * angle.sin())
            })
            .collect();
        let mass = parent.mass / model.children as f64;
        if !positive(mass) {
            return Err(Error::NumericalFailure);
        }
        let masses: Vec<_> = if let Some(fractions) = fractions {
            weighted_masses(parent.mass, fractions)?
        } else {
            (0..model.children)
                .map(|i| {
                    if i + 1 == model.children {
                        parent.mass - mass * (model.children - 1) as f64
                    } else {
                        mass
                    }
                })
                .collect()
        };
        let mut mean = [0.0; 3];
        for (direction, mass) in directions.iter().zip(&masses) {
            for k in 0..3 {
                mean[k] += mass / parent.mass * direction[k];
            }
        }
        for direction in &mut directions {
            for k in 0..3 {
                direction[k] -= mean[k];
            }
        }
        let quadratic: f64 = directions
            .iter()
            .zip(&masses)
            .map(|(d, m)| m * d.iter().map(|v| v * v).sum::<f64>())
            .sum();
        let speed = (2.0 * kinetic / quadratic).sqrt();
        if !speed.is_finite() {
            return Err(Error::NumericalFailure);
        }
        let field = self.fields().map(|fields| fields[index]);
        let phase_fraction = self.phase_fractions().map(|fractions| fractions[index]);
        let add: Vec<_> = directions
            .iter()
            .zip(masses.iter().copied())
            .map(|(d, mass)| {
                let mut particle = parent;
                particle.mass = mass;
                particle.position =
                    std::array::from_fn(|k| parent.position[k] + position_radius * d[k]);
                particle.velocity = std::array::from_fn(|k| parent.velocity[k] + speed * d[k]);
                ParticleInput {
                    particle,
                    field,
                    phase_fraction,
                }
            })
            .collect();
        let density = self.effective_materials()?[index].rest_density;
        let radii: Vec<_> = add
            .iter()
            .map(|source| {
                (3.0 * (source.particle.mass / density) / (4.0 * std::f64::consts::PI)).cbrt()
            })
            .collect();
        for i in 0..add.len() {
            for j in 0..i {
                let separation = norm(std::array::from_fn(|k| {
                    add[i].particle.position[k] - add[j].particle.position[k]
                }));
                if !separation.is_finite() || separation < (radii[i] + radii[j]) * (1.0 - 1e-12) {
                    return Err(Error::NumericalFailure);
                }
            }
        }
        let realized_kinetic: f64 = add
            .iter()
            .map(|source| {
                let relative = std::array::from_fn::<_, 3, _>(|k| {
                    source.particle.velocity[k] - parent.velocity[k]
                });
                0.5 * source.particle.mass * relative.iter().map(|v| v * v).sum::<f64>()
            })
            .sum();
        if !realized_kinetic.is_finite()
            || (realized_kinetic - kinetic).abs() > 1e-10 * kinetic + f64::MIN_POSITIVE
        {
            return Err(Error::NumericalFailure);
        }
        let rows = self
            .species_fractions()
            .map(|rows| vec![rows[index].clone(); model.children]);
        let pressure = self
            .transport
            .as_ref()
            .and_then(|t| t.phase.as_ref())
            .and_then(|p| p.saturation.as_ref())
            .map(|p| p.pressures[index]);
        let split_heat = |buffer: &[f64]| -> Result<Vec<f64>, Error> {
            if buffer.len() != self.particles.len() && buffer.iter().any(|v| *v != 0.0) {
                return Err(Error::NumericalFailure);
            }
            let mut result: Vec<_> = (0..self.particles.len())
                .filter(|i| *i != index)
                .map(|i| buffer.get(i).copied().unwrap_or(0.0))
                .collect();
            let parent = buffer.get(index).copied().unwrap_or(0.0);
            let part = parent / model.children as f64;
            let mut assigned = 0.0;
            for (child, mass) in masses.iter().enumerate() {
                let value = if child + 1 == model.children {
                    parent - assigned
                } else if fractions.is_some() {
                    parent * (mass / self.particles[index].mass)
                } else {
                    part
                };
                result.push(value);
                assigned += value;
            }
            if result.iter().any(|v| !v.is_finite()) {
                return Err(Error::NumericalFailure);
            }
            Ok(result)
        };
        let heat = split_heat(&self.suspension_heat_buffer)?;
        let correction = split_heat(&self.suspension_heat_correction)?;
        let structure = self.structure[index];
        let mut candidate = self.clone();
        if let Some(pressure) = pressure {
            candidate.exchange_particles_at_pressure(
                &[index],
                &add,
                &vec![pressure; model.children],
                rows.as_deref(),
            )?;
        } else if let Some(rows) = rows {
            candidate.exchange_particles_with_species(&[index], &add, &rows)?;
        } else {
            candidate.exchange_particles(&[index], &add)?;
        }
        let first = candidate.particles.len() - model.children;
        candidate.structure[first..].fill(structure);
        candidate.conformation[first..].fill(self.conformation[index]);
        if let Some(flags) = &mut candidate.droplet_population {
            flags[first..].fill(true);
        }
        candidate.suspension_heat_buffer = heat;
        candidate.suspension_heat_correction = correction;
        candidate.effective_materials()?;
        *self = candidate;
        Ok(DropletSplitReport {
            children: model.children,
            position_radius,
            created_surface_energy: surface,
            added_kinetic_energy: kinetic,
        })
    }
}
