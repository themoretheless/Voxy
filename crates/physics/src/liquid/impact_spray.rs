//! Energy-gated, prescribed impact fragmentation; not a calibrated splash onset law.
use super::{DropletSplit, FilmRebound, Liquid, positive};
use crate::surface_film::SurfaceFilm;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactSpray {
    pub rebound: FilmRebound,
    pub children: usize,
    /// Minimum fragment ring radius; equivalent-volume sphere packing may expand it.
    pub position_radius: f64,
    pub surface_tension: f64,
    /// Fraction of each impact's lost kinetic energy available for fragmentation.
    /// Material-dependent impact hydrodynamics do not determine this coefficient.
    pub fragmentation_fraction: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImpactSprayReport {
    pub impacts: usize,
    pub fragmented_particles: usize,
    pub fragments_created: usize,
    pub substrate_impulse: [f64; 3],
    pub substrate_heat: f64,
    pub created_surface_energy: f64,
    pub added_fragment_kinetic_energy: f64,
}
impl Liquid {
    /// Reflects all crossing trajectories, then fragments impacts whose allocated
    /// kinetic loss covers the additional spherical surface energy. Other impacts
    /// retain the ordinary rebound response. No external energy is supplied.
    /// All particle fields survive via the existing fragmentation primitive.
    /// The film remains unchanged; heat and new surface energy are external ledgers.
    /// Any failure, including a later particle-budget error, rolls back all impacts.
    pub fn impact_spray_surface_film(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &SurfaceFilm,
        model: ImpactSpray,
    ) -> Result<ImpactSprayReport, &'static str> {
        self.impact_spray_geometry(previous_positions, film, model, None)
    }
    /// Finite-radius impact followed by tangential fragment placement using the
    /// actual contact normal. New equivalent-volume spheres must clear the mesh.
    /// All fluid state and ledgers roll back if any fragment penetrates a triangle.
    pub fn impact_spray_spheres_surface_film(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &SurfaceFilm,
        model: ImpactSpray,
        radii: &[f64],
    ) -> Result<ImpactSprayReport, &'static str> {
        if radii.len() != self.particles.len() || radii.iter().any(|r| !positive(*r)) {
            return Err("invalid film sphere radii");
        }
        self.impact_spray_geometry(previous_positions, film, model, Some(radii))
    }
    fn impact_spray_geometry(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &SurfaceFilm,
        model: ImpactSpray,
        radii: Option<&[f64]>,
    ) -> Result<ImpactSprayReport, &'static str> {
        if !(2..=64).contains(&model.children)
            || !positive(model.position_radius)
            || !model.surface_tension.is_finite()
            || model.surface_tension < 0.0
            || !model.fragmentation_fraction.is_finite()
            || !(0.0..=1.0).contains(&model.fragmentation_fraction)
            || previous_positions.len() != self.particles.len()
            || self.gas_active()
        {
            return Err("invalid impact spray controls");
        }
        let mut candidate = self.clone();
        let mut final_radii = radii.map(|r| r.to_vec());
        let rebound = if let Some(radii) = radii {
            candidate.rebound_spheres_surface_film(
                previous_positions,
                film,
                model.rebound,
                radii,
            )?
        } else {
            candidate.rebound_surface_film(previous_positions, film, model.rebound)?
        };
        let mut report = ImpactSprayReport {
            impacts: rebound.particles,
            substrate_impulse: rebound.substrate_impulse,
            ..ImpactSprayReport::default()
        };
        let mut splits = Vec::new();
        for (i, start) in previous_positions.iter().enumerate() {
            let normal = if let Some(radii) = radii {
                let Some(hit) = super::film_rebound::sphere_contact(
                    *start,
                    self.particles[i].position,
                    film,
                    radii[i],
                )?
                else {
                    continue;
                };
                hit.normal
            } else {
                let Some((cell, _)) = film.first_segment_hit(*start, self.particles[i].position)?
                else {
                    continue;
                };
                film.cell_normal(cell)?
            };
            let kinetic = |v: [f64; 3]| v.iter().map(|x| x * x).sum::<f64>();
            let lost = (0.5
                * self.particles[i].mass
                * (kinetic(self.particles[i].velocity) - kinetic(candidate.particles[i].velocity)))
            .max(0.0);
            let budget = model.fragmentation_fraction * lost;
            let surface = candidate
                .droplet_fragment_surface_energy(i, model.children, model.surface_tension)
                .map_err(|_| "invalid impact surface energy")?;
            if budget > 0.0 && budget >= surface {
                report.substrate_heat += lost - budget;
                splits.push((i, normal, budget));
            } else {
                report.substrate_heat += lost;
            }
        }
        // Removing higher original indices leaves every lower original index valid.
        for (i, axis, budget) in splits.into_iter().rev() {
            let split = candidate
                .split_droplet(
                    i,
                    DropletSplit {
                        children: model.children,
                        axis,
                        position_radius: model.position_radius,
                        surface_tension: model.surface_tension,
                        available_energy: budget,
                    },
                )
                .map_err(|_| "impact fragmentation failed")?;
            report.fragmented_particles += 1;
            report.fragments_created += split.children;
            report.created_surface_energy += split.created_surface_energy;
            report.added_fragment_kinetic_energy += split.added_kinetic_energy;
            if let Some(values) = &mut final_radii {
                values.remove(i);
                let child_radii = candidate
                    .equivalent_sphere_radii()
                    .map_err(|_| "invalid fragment radii")?;
                values.extend_from_slice(&child_radii[child_radii.len() - split.children..]);
            }
        }
        if let Some(values) = final_radii {
            for (particle, radius) in candidate.particles.iter().zip(values) {
                if film
                    .first_sphere_hit(particle.position, particle.position, radius)?
                    .is_some_and(|hit| hit.penetration > 64.0 * f64::EPSILON * radius)
                {
                    return Err("fragment sphere penetrates film mesh");
                }
            }
        }
        if [
            report.substrate_heat,
            report.created_surface_energy,
            report.added_fragment_kinetic_energy,
        ]
        .iter()
        .any(|v| !v.is_finite())
        {
            return Err("impact spray ledger overflow");
        }
        *self = candidate;
        Ok(report)
    }
}
