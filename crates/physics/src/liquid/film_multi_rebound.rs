//! Bounded sequential finite-radius impacts with a stationary triangle substrate.
use super::{FilmRebound, FilmReboundReport, Liquid, finite, sub};
use crate::surface_film::SurfaceFilm;
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FilmMultiReboundReport {
    /// Counts particles that received at least one impact, with summed ledgers.
    pub rebound: FilmReboundReport,
    pub contacts: usize,
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
impl Liquid {
    /// Resweeps every reflected path remainder until no closing contact remains.
    /// Endpoint forces remain frozen; there is no curved trajectory or coupled
    /// simultaneous-contact solve. All transported particle fields are retained.
    /// A contact budget error rolls back every particle and its complete state.
    pub fn rebound_spheres_surface_film_multi(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &SurfaceFilm,
        model: FilmRebound,
        radii: &[f64],
        max_contacts_per_particle: usize,
    ) -> Result<FilmMultiReboundReport, &'static str> {
        if previous_positions.len() != self.particles.len()
            || radii.len() != self.particles.len()
            || radii.iter().any(|r| !r.is_finite() || *r <= 0.0)
            || !(1..=64).contains(&max_contacts_per_particle)
            || [model.restitution, model.friction]
                .iter()
                .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
        {
            return Err("invalid multi-impact controls");
        }
        let mut candidate = self.clone();
        let mut report = FilmMultiReboundReport::default();
        for (i, &previous) in previous_positions.iter().enumerate() {
            let particle = &mut candidate.particles[i];
            let mut start = previous;
            let mut end = particle.position;
            let mut velocity = particle.velocity;
            let mut count = 0;
            while let Some(hit) = super::film_rebound::sphere_contact(start, end, film, radii[i])? {
                if count == max_contacts_per_particle {
                    return Err("film multi-impact contact budget");
                }
                let direction = sub(end, start);
                let speed = dot(velocity, hit.normal);
                if !speed.is_finite() || speed > 0.0 {
                    return Err("inconsistent film impact velocity");
                }
                let reflect = |v: [f64; 3]| {
                    let normal = dot(v, hit.normal);
                    std::array::from_fn(|k| {
                        (1.0 - model.friction) * (v[k] - normal * hit.normal[k])
                            - model.restitution * normal * hit.normal[k]
                    })
                };
                let after = reflect(velocity);
                let remainder = reflect(direction.map(|x| (1.0 - hit.time) * x));
                let contact: [f64; 3] = std::array::from_fn(|k| start[k] + hit.time * direction[k]);
                let scale = contact
                    .iter()
                    .chain(&direction)
                    .fold(radii[i], |s, x| s.max(x.abs()));
                let skin = 128.0 * f64::EPSILON * scale;
                start = std::array::from_fn(|k| contact[k] + skin * hit.normal[k]);
                end = std::array::from_fn(|k| start[k] + remainder[k]);
                let lost = 0.5 * particle.mass * (dot(velocity, velocity) - dot(after, after));
                if !finite(start)
                    || !finite(end)
                    || !finite(after)
                    || !lost.is_finite()
                    || lost < -1e-12
                {
                    return Err("film multi-impact overflow");
                }
                for k in 0..3 {
                    report.rebound.substrate_impulse[k] += particle.mass * (velocity[k] - after[k]);
                }
                report.rebound.dissipated_energy += lost.max(0.0);
                report.contacts += 1;
                count += 1;
                velocity = after;
            }
            particle.position = end;
            particle.velocity = velocity;
            if count > 0 {
                report.rebound.particles += 1;
            }
        }
        if !finite(report.rebound.substrate_impulse)
            || !report.rebound.dissipated_energy.is_finite()
        {
            return Err("film multi-impact ledger overflow");
        }
        candidate
            .effective_materials()
            .map_err(|_| "invalid reflected fluid state")?;
        *self = candidate;
        Ok(report)
    }
}
