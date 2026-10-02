//! Whole-particle point-trajectory absorption with component inventories.
use super::{FilmCapture, ImpactSpray, ImpactSprayReport, Liquid};
use crate::surface_film::FilmMixture;
#[derive(Clone, Debug, PartialEq)]
pub struct FilmMixtureCapture {
    pub capture: FilmCapture,
    /// Removed component masses in the common liquid/film schema order, kg.
    pub component_masses: Vec<f64>,
}
/// Prescribed deposition/fragmentation selection, without wetting calibration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepositingImpact {
    /// Maximum total incident speed in m/s for sticking. Zero captures only rest.
    pub capture_speed: f64,
    pub spray: ImpactSpray,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DepositingImpactReport {
    pub deposition: FilmMixtureCapture,
    pub spray: ImpactSprayReport,
}
struct CapturePlan {
    remove: Vec<usize>,
    deposits: Vec<(usize, f64, Vec<f64>)>,
    component_masses: Vec<f64>,
}
impl Liquid {
    /// Absorbs complete particles into an equal-density multicomponent film.
    /// Species names and order must match exactly. Each impacting particle's
    /// effective density must match the film's shared density. Phase/gas states
    /// and a separate scalar concentration are unsupported. Momentum and all
    /// kinetic/thermal energy go to the external substrate ledger, as with pure
    /// film capture. This does not solve finite-radius contact or splash formation.
    /// Fluid and film commit together only after all candidate work succeeds.
    pub fn capture_surface_mixture(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &mut FilmMixture,
    ) -> Result<FilmMixtureCapture, &'static str> {
        let plan = self.plan_surface_mixture(previous_positions, film, None, None)?;
        let mut candidate = self.clone();
        let ledger = candidate
            .exchange_particles(&plan.remove, &[])
            .map_err(|_| "film capture drain failed")?;
        let deposited_volume = film.deposit_batch(&plan.deposits)?;
        // Both destinations commit without another fallible operation.
        *self = candidate;
        Ok(FilmMixtureCapture {
            capture: FilmCapture {
                particles: plan.remove.len(),
                deposited_volume,
                absorbed: ledger.removed,
            },
            component_masses: plan.component_masses,
        })
    }
    /// Deposits slow crossing particles and reflects/fragments faster crossings.
    /// All candidate particle work precedes the final atomic film batch, so a
    /// later spray or deposition failure leaves both complete states untouched.
    /// No automatic wetting, adhesion or deposition threshold is inferred.
    pub fn depositing_impact_surface_mixture(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &mut FilmMixture,
        model: DepositingImpact,
    ) -> Result<DepositingImpactReport, &'static str> {
        self.depositing_impact_geometry(previous_positions, film, model, None)
    }
    /// Finite spherical contact selects whole-particle deposition or energy-gated
    /// spray. Contact radii are explicit; new fragments use their equivalent volume.
    /// Film composition and complete liquid state commit together after all checks.
    pub fn depositing_impact_spheres_surface_mixture(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &mut FilmMixture,
        model: DepositingImpact,
        radii: &[f64],
    ) -> Result<DepositingImpactReport, &'static str> {
        if radii.len() != self.particles.len() || radii.iter().any(|r| !super::positive(*r)) {
            return Err("invalid film sphere radii");
        }
        self.depositing_impact_geometry(previous_positions, film, model, Some(radii))
    }
    fn depositing_impact_geometry(
        &mut self,
        previous_positions: &[[f64; 3]],
        film: &mut FilmMixture,
        model: DepositingImpact,
        radii: Option<&[f64]>,
    ) -> Result<DepositingImpactReport, &'static str> {
        if !model.capture_speed.is_finite() || model.capture_speed < 0.0 {
            return Err("invalid deposition speed threshold");
        }
        let plan =
            self.plan_surface_mixture(previous_positions, film, Some(model.capture_speed), radii)?;
        let mut candidate = self.clone();
        let ledger = candidate
            .exchange_particles(&plan.remove, &[])
            .map_err(|_| "film capture drain failed")?;
        let mut surviving_paths = Vec::with_capacity(previous_positions.len() - plan.remove.len());
        let mut surviving_radii = Vec::new();
        let mut removed = plan.remove.iter().peekable();
        for (i, start) in previous_positions.iter().enumerate() {
            if removed.peek().is_some_and(|index| **index == i) {
                removed.next();
            } else {
                surviving_paths.push(*start);
                if let Some(radii) = radii {
                    surviving_radii.push(radii[i]);
                }
            }
        }
        let spray = if radii.is_some() {
            candidate.impact_spray_spheres_surface_film(
                &surviving_paths,
                film.film(),
                model.spray,
                &surviving_radii,
            )?
        } else {
            candidate.impact_spray_surface_film(&surviving_paths, film.film(), model.spray)?
        };
        let deposited_volume = film.deposit_batch(&plan.deposits)?;
        *self = candidate;
        Ok(DepositingImpactReport {
            deposition: FilmMixtureCapture {
                capture: FilmCapture {
                    particles: plan.remove.len(),
                    deposited_volume,
                    absorbed: ledger.removed,
                },
                component_masses: plan.component_masses,
            },
            spray,
        })
    }
    fn plan_surface_mixture(
        &self,
        previous_positions: &[[f64; 3]],
        film: &FilmMixture,
        maximum_speed: Option<f64>,
        radii: Option<&[f64]>,
    ) -> Result<CapturePlan, &'static str> {
        if previous_positions.len() != self.particles.len()
            || self.species_names() != Some(film.component_names())
            || self.phase_fractions().is_some()
            || self.gas_active()
            || self
                .fields()
                .is_some_and(|rows| rows.iter().any(|row| row.concentration != 0.0))
        {
            return Err("incompatible mixture film capture state");
        }
        let density = film.film().material().density;
        let materials = self
            .effective_materials()
            .map_err(|_| "invalid capture material")?;
        let fractions = self
            .species_fractions()
            .ok_or("missing capture composition")?;
        let mut component_masses = vec![0.0; film.component_names().len()];
        let mut remove = Vec::new();
        let mut deposits = Vec::new();
        for (i, (start, particle)) in previous_positions.iter().zip(&self.particles).enumerate() {
            let spherical = if let Some(radii) = radii {
                super::film_rebound::sphere_contact(
                    *start,
                    particle.position,
                    film.film(),
                    radii[i],
                )?
            } else {
                None
            };
            let contact = if radii.is_some() {
                spherical.map(|hit| (hit.cell, hit.time))
            } else {
                film.film().first_segment_hit(*start, particle.position)?
            };
            if let Some((cell, _)) = contact {
                if let Some(limit) = maximum_speed {
                    let direction = super::sub(particle.position, *start);
                    let normal = if let Some(hit) = spherical {
                        hit.normal
                    } else {
                        film.film().cell_normal(cell)?
                    };
                    let dot = |v: [f64; 3]| v.iter().zip(normal).map(|(a, b)| a * b).sum::<f64>();
                    let inward = if dot(direction) > 0.0 {
                        dot(particle.velocity) >= 0.0
                    } else {
                        dot(particle.velocity) <= 0.0
                    };
                    if !inward {
                        return Err("inconsistent deposition impact velocity");
                    }
                    let speed = super::norm(particle.velocity);
                    if !speed.is_finite() {
                        return Err("deposition speed overflow");
                    }
                    if speed > limit {
                        continue;
                    }
                }
                if (materials[i].rest_density - density).abs() > 1e-10 * density {
                    return Err("film and incident liquid densities differ");
                }
                let volume = particle.mass / density;
                if !volume.is_finite() || volume <= 0.0 {
                    return Err("invalid capture volume");
                }
                for (mass, fraction) in component_masses.iter_mut().zip(&fractions[i]) {
                    *mass += particle.mass * fraction;
                    if !mass.is_finite() {
                        return Err("capture component mass overflow");
                    }
                }
                remove.push(i);
                deposits.push((cell, volume, fractions[i].clone()));
            }
        }
        Ok(CapturePlan {
            remove,
            deposits,
            component_masses,
        })
    }
}
