//! Prescribed stationary-surface impact response; not a splash or contact-angle model.
use super::{Liquid, finite, sub};
use crate::surface_film::SurfaceFilm;
pub(super) fn sphere_contact(
    start: [f64; 3],
    end: [f64; 3],
    film: &SurfaceFilm,
    radius: f64,
) -> Result<Option<crate::surface_film::SphereFilmHit>, &'static str> {
    let Some(hit) = film.first_closing_sphere_hit(start, end, radius)? else {
        return Ok(None);
    };
    if hit.penetration > 64.0 * f64::EPSILON * radius {
        return Err("initial film sphere penetration");
    }
    Ok(Some(hit))
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
/// Prescribed impact coefficients; neither is inferred from material or wetting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilmRebound {
    /// Normal speed retained, in [0,1].
    pub restitution: f64,
    /// Tangential speed removed, in [0,1].
    pub friction: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FilmReboundReport {
    pub particles: usize,
    /// Equal and opposite of the fluid momentum change.
    pub substrate_impulse: [f64; 3],
    /// Lost kinetic energy transferred to the caller's substrate/heat ledger.
    pub dissipated_energy: f64,
}
impl Liquid {
    /// Reflects crossing point trajectories at their first surface triangle.
    /// The remaining segment is reflected with the same prescribed coefficients as
    /// velocity. All per-particle composition, phase, structure and thermal fields
    /// stay with their particle. Film volume is unchanged.
    ///
    /// This discrete-time operation assumes a stationary sheet and approximately
    /// linear path; it does not recompute forces after impact or sweep particle radii.
    /// A roundoff-scale normal separation keeps zero-restitution contacts detectable.
    /// Use it instead of capture/bounce for the same crossing, and refine dt.
    /// # Errors
    /// Invalid coefficients/positions, overflow, or endpoint velocity pointing away
    /// from the incident side despite a crossing. Entire fluid rolls back on error.
    pub fn rebound_surface_film(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &SurfaceFilm,
        model: FilmRebound,
    ) -> Result<FilmReboundReport, &'static str> {
        self.rebound_film_geometry(previous_positions, film, model, None)
    }
    /// Reflects finite spherical particles at first face/edge/vertex contact.
    /// Radii are explicit per-particle SI values, not inferred from SPH kernels.
    /// Initial penetration is rejected; separating initial touches are skipped
    /// so later closing contacts can still be found. Only the first contact is resolved.
    /// All particle fields and stationary-substrate momentum/energy ledgers are retained.
    pub fn rebound_spheres_surface_film(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &SurfaceFilm,
        model: FilmRebound,
        radii: &[f64],
    ) -> Result<FilmReboundReport, &'static str> {
        if radii.len() != self.particles.len() || radii.iter().any(|r| !r.is_finite() || *r <= 0.0)
        {
            return Err("invalid film sphere radii");
        }
        self.rebound_film_geometry(previous_positions, film, model, Some(radii))
    }
    fn rebound_film_geometry(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &SurfaceFilm,
        model: FilmRebound,
        radii: Option<&[f64]>,
    ) -> Result<FilmReboundReport, &'static str> {
        if previous_positions.len() != self.particles.len()
            || [model.restitution, model.friction]
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err("invalid film rebound controls");
        }
        let mut candidate = self.clone();
        let mut report = FilmReboundReport::default();
        for (i, start) in previous_positions.iter().enumerate() {
            let particle = &mut candidate.particles[i];
            let direction = sub(particle.position, *start);
            let (t, normal) = if let Some(radii) = radii {
                let Some(hit) = sphere_contact(*start, particle.position, film, radii[i])? else {
                    continue;
                };
                (hit.time, hit.normal)
            } else {
                let Some((cell, t)) = film.first_segment_hit(*start, particle.position)? else {
                    continue;
                };
                let mut normal = film.cell_normal(cell)?;
                if dot(direction, normal) > 0.0 {
                    normal = normal.map(|n| -n);
                }
                (t, normal)
            };
            let speed = dot(particle.velocity, normal);
            if !speed.is_finite() || speed > 0.0 {
                return Err("inconsistent film impact velocity");
            }
            let reflect = |vector: [f64; 3]| {
                let perpendicular = dot(vector, normal);
                std::array::from_fn(|axis| {
                    (1.0 - model.friction) * (vector[axis] - perpendicular * normal[axis])
                        - model.restitution * perpendicular * normal[axis]
                })
            };
            let before = particle.velocity;
            let after = reflect(before);
            let remainder = reflect(direction.map(|v| (1.0 - t) * v));
            let mut position =
                std::array::from_fn(|axis| start[axis] + t * direction[axis] + remainder[axis]);
            // Keep resting impacts on their incident side at roundoff scale so
            // the next interval can detect a new inward crossing (including e=0).
            let scale = position
                .iter()
                .chain(&direction)
                .fold(0.0_f64, |a, v| a.max(v.abs()));
            let separation = 128.0 * f64::EPSILON * scale;
            for axis in 0..3 {
                position[axis] += separation * normal[axis];
            }
            let lost = 0.5 * particle.mass * (dot(before, before) - dot(after, after));
            if !finite(after) || !finite(position) || !lost.is_finite() || lost < -1e-12 {
                return Err("film rebound overflow");
            }
            for axis in 0..3 {
                report.substrate_impulse[axis] += particle.mass * (before[axis] - after[axis]);
            }
            report.dissipated_energy += lost.max(0.0);
            report.particles += 1;
            particle.velocity = after;
            particle.position = position;
        }
        if !finite(report.substrate_impulse) || !report.dissipated_energy.is_finite() {
            return Err("film rebound ledger overflow");
        }
        candidate
            .effective_materials()
            .map_err(|_| "invalid reflected fluid state")?;
        *self = candidate;
        Ok(report)
    }
}
