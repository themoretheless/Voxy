//! Chronological impact, fragmentation and deposition within one interval.
use super::{
    DepositingImpact, DepositingImpactReport, DropletSplit, ExchangeTotals, FilmCapture,
    FilmMixtureCapture, ImpactSprayReport, Liquid, finite, norm, positive, sub,
};
use crate::surface_film::FilmMixture;
#[derive(Clone, Copy, Debug)]
pub struct FilmImpactControl {
    pub dt: f64,
    /// Inherited by fragments; splitting does not reset their contact budget.
    pub max_contacts_per_lineage: usize,
    pub max_events: usize,
}
impl Default for FilmImpactControl {
    fn default() -> Self {
        Self {
            dt: 0.001,
            max_contacts_per_lineage: 16,
            max_events: 4096,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct FilmImpactEventReport {
    pub impact: DepositingImpactReport,
    pub events: usize,
    pub coalescence: super::DropletCoalescenceReport,
    pub flight: DropletFlightReport,
}
/// Prescribed constant acceleration of marked droplets, starting before the flow step.
/// All marked droplets share the acceleration, so their relative paths stay linear.
#[derive(Clone, Debug)]
pub struct DropletFlight {
    pub initial_velocities: Vec<[f64; 3]>,
    pub acceleration: [f64; 3],
    pub max_feature_checks: usize,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DropletFlightReport {
    /// External impulse added to droplets while alive in this interval.
    pub impulse: [f64; 3],
    /// External work, including only flight after each fragment's birth.
    pub work: f64,
}
#[derive(Clone, Debug, Default)]
pub struct DropletLifecycle {
    pub mass_fractions: Option<Vec<f64>>,
    pub splash_onset: Option<super::DryWallSplashOnset>,
    pub coalescence: Option<super::DropletCoalescenceControl>,
    /// Prescribed sticking on sphere growth into substrate, under capture_speed.
    pub capture_growth_contact: bool,
    pub flight: Option<DropletFlight>,
    /// Reconstructed film-height geometry, rebuilt after every prepared deposit.
    pub free_surface_side: Option<f64>,
    /// Prescribed whole-drop sticking for slow immersed centers or partial top overlap.
    pub capture_film_immersion: bool,
}
#[derive(Clone, Copy)]
struct Path {
    end: [f64; 3],
    remaining: f64,
    radius: f64,
    contacts: usize,
    accelerated: bool,
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn ballistic_end(p: super::Particle, a: [f64; 3], dt: f64) -> [f64; 3] {
    std::array::from_fn(|k| p.position[k] + p.velocity[k] * dt + 0.5 * a[k] * dt * dt)
}
fn advance_flight(p: &mut super::Particle, a: [f64; 3], dt: f64, ledger: &mut DropletFlightReport) {
    for k in 0..3 {
        let displacement = p.velocity[k] * dt + 0.5 * a[k] * dt * dt;
        p.position[k] += displacement;
        p.velocity[k] += a[k] * dt;
        ledger.impulse[k] += p.mass * a[k] * dt;
        ledger.work += p.mass * a[k] * displacement;
    }
}
fn add_totals(a: &mut ExchangeTotals, b: ExchangeTotals) -> Result<(), &'static str> {
    a.mass += b.mass;
    a.kinetic_energy += b.kinetic_energy;
    a.polymer_energy += b.polymer_energy;
    for k in 0..3 {
        a.momentum[k] += b.momentum[k];
    }
    for (a, b) in [
        (&mut a.thermal_energy, b.thermal_energy),
        (&mut a.dissolved_mass, b.dissolved_mass),
    ] {
        match (a, b) {
            (Some(a), Some(b)) => *a += b,
            (None, None) => {}
            _ => return Err("impact ledger schema mismatch"),
        }
    }
    if !finite(a.momentum)
        || [
            a.mass,
            a.kinetic_energy,
            a.polymer_energy,
            a.thermal_energy.unwrap_or(0.0),
            a.dissolved_mass.unwrap_or(0.0),
        ]
        .iter()
        .any(|v| !v.is_finite())
    {
        return Err("impact capture ledger overflow");
    }
    Ok(())
}
fn prepare_capture(
    candidate: &mut Liquid,
    film: &FilmMixture,
    i: usize,
    cell: usize,
    report: &mut FilmImpactEventReport,
    deposits: &mut Vec<(usize, f64, Vec<f64>)>,
) -> Result<(), &'static str> {
    let parent = candidate.particles[i];
    let density = film.film().material().density;
    if (candidate
        .effective_materials()
        .map_err(|_| "invalid event material")?[i]
        .rest_density
        - density)
        .abs()
        > 1e-10 * density
    {
        return Err("film and incident liquid densities differ");
    }
    let volume = parent.mass / density;
    if !positive(volume) {
        return Err("invalid capture volume");
    }
    let row = candidate
        .species_fractions()
        .ok_or("missing capture composition")?[i]
        .clone();
    for (total, y) in report
        .impact
        .deposition
        .component_masses
        .iter_mut()
        .zip(&row)
    {
        *total += parent.mass * y;
    }
    deposits.push((cell, volume, row));
    let removed = candidate
        .exchange_particles(&[i], &[])
        .map_err(|_| "film event capture failed")?
        .removed;
    add_totals(&mut report.impact.deposition.capture.absorbed, removed)?;
    report.impact.deposition.capture.particles += 1;
    Ok(())
}
impl Liquid {
    /// Resolves contacts in chronological order, advancing reflected parents and
    /// spawned fragments through the remaining interval. Paths and endpoint
    /// velocities are supplied by the caller's flow step; forces remain frozen.
    /// Relative fragment velocity acts only after birth, for the remaining time.
    /// Whole-particle sticking uses the prescribed incident-speed threshold.
    /// Film and complete liquid state commit together after every event succeeds.
    pub fn depositing_impact_spheres_surface_mixture_events(
        &mut self,
        previous: &[[f64; 3]],
        film: &mut FilmMixture,
        model: DepositingImpact,
        radii: &[f64],
        control: FilmImpactControl,
    ) -> Result<FilmImpactEventReport, &'static str> {
        self.impact_events_impl(
            previous, film, model, radii, control, None, None, None, false, None, None, false,
        )
    }
    /// Optional empirical dry-wall onset gate. Below onset particles still rebound
    /// or capture under the supplied model; the gate only suppresses fragmentation.
    /// Diameter comes from particle mass and current effective density, not the
    /// collision radius. Existing energy and clearance checks remain required.
    pub fn depositing_impact_spheres_surface_mixture_events_with_onset(
        &mut self,
        previous: &[[f64; 3]],
        film: &mut FilmMixture,
        model: DepositingImpact,
        radii: &[f64],
        control: FilmImpactControl,
        onset: super::DryWallSplashOnset,
    ) -> Result<FilmImpactEventReport, &'static str> {
        if !positive(onset.critical_parameter) || !positive(model.spray.surface_tension) {
            return Err("invalid splash onset controls");
        }
        self.impact_events_impl(
            previous,
            film,
            model,
            radii,
            control,
            Some(onset),
            None,
            None,
            false,
            None,
            None,
            false,
        )
    }
    /// Chronological impacts with caller-supplied fragment mass fractions.
    /// Every generation inherits the same distribution; each child retains its
    /// parent's local composition. An optional dry-wall onset gate may be supplied.
    pub fn depositing_impact_spheres_surface_mixture_events_with_mass_fractions(
        &mut self,
        previous: &[[f64; 3]],
        film: &mut FilmMixture,
        model: DepositingImpact,
        radii: &[f64],
        control: FilmImpactControl,
        fractions: &[f64],
        onset: Option<super::DryWallSplashOnset>,
    ) -> Result<FilmImpactEventReport, &'static str> {
        if fractions.len() != model.spray.children {
            return Err("invalid impact fragment distribution");
        }
        super::droplet_split::weighted_masses(1.0, fractions)
            .map_err(|_| "invalid impact fragment distribution")?;
        if onset.is_some_and(|o| {
            !positive(o.critical_parameter) || !positive(model.spray.surface_tension)
        }) {
            return Err("invalid splash onset controls");
        }
        self.impact_events_impl(
            previous,
            film,
            model,
            radii,
            control,
            onset,
            Some(fractions),
            None,
            false,
            None,
            None,
            false,
        )
    }
    /// Unified chronological wall impact and droplet coalescence. All paths advance
    /// to each event together; merges and fragments continue within the same interval.
    /// Coalescence controls must share dt and surface tension with this impact model.
    /// Point-particle unresolved motion and surface release remain explicit ledgers.
    /// Optional flight recomputes marked-drop parabolas and contact-time velocities;
    /// supplied marked endpoints must agree with the declared initial velocity and acceleration.
    pub fn depositing_impact_spheres_surface_mixture_lifecycle(
        &mut self,
        previous: &[[f64; 3]],
        film: &mut FilmMixture,
        model: DepositingImpact,
        radii: &[f64],
        control: FilmImpactControl,
        lifecycle: &DropletLifecycle,
    ) -> Result<FilmImpactEventReport, &'static str> {
        if let Some(fractions) = &lifecycle.mass_fractions {
            if fractions.len() != model.spray.children {
                return Err("invalid impact fragment distribution");
            }
            super::droplet_split::weighted_masses(1.0, fractions)
                .map_err(|_| "invalid impact fragment distribution")?;
        }
        if lifecycle.splash_onset.is_some_and(|o| {
            !positive(o.critical_parameter) || !positive(model.spray.surface_tension)
        }) {
            return Err("invalid splash onset controls");
        }
        if lifecycle.coalescence.is_some_and(|c| {
            c.dt != control.dt
                || c.surface_tension != model.spray.surface_tension
                || !c.maximum_normal_speed.is_finite()
                || c.maximum_normal_speed < 0.0
                || !(1..=100000).contains(&c.max_events)
        }) {
            return Err("invalid lifecycle coalescence controls");
        }
        self.impact_events_impl(
            previous,
            film,
            model,
            radii,
            control,
            lifecycle.splash_onset,
            lifecycle.mass_fractions.as_deref(),
            lifecycle.coalescence,
            lifecycle.capture_growth_contact,
            lifecycle.flight.as_ref(),
            lifecycle.free_surface_side,
            lifecycle.capture_film_immersion,
        )
    }
    fn impact_events_impl(
        &mut self,
        previous: &[[f64; 3]],
        film: &mut FilmMixture,
        model: DepositingImpact,
        radii: &[f64],
        control: FilmImpactControl,
        onset: Option<super::DryWallSplashOnset>,
        fractions: Option<&[f64]>,
        coalescence: Option<super::DropletCoalescenceControl>,
        capture_growth_contact: bool,
        flight: Option<&DropletFlight>,
        free_surface_side: Option<f64>,
        capture_film_immersion: bool,
    ) -> Result<FilmImpactEventReport, &'static str> {
        if previous.len() != self.particles.len()
            || radii.len() != self.particles.len()
            || radii.iter().any(|r| !positive(*r))
            || !positive(control.dt)
            || control.dt > 0.1
            || !(1..=64).contains(&control.max_contacts_per_lineage)
            || !(1..=100000).contains(&control.max_events)
            || !model.capture_speed.is_finite()
            || model.capture_speed < 0.0
            || !(2..=64).contains(&model.spray.children)
            || !positive(model.spray.position_radius)
            || !model.spray.surface_tension.is_finite()
            || model.spray.surface_tension < 0.0
            || [
                model.spray.fragmentation_fraction,
                model.spray.rebound.restitution,
                model.spray.rebound.friction,
            ]
            .iter()
            .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
        {
            return Err("invalid film impact event controls");
        }
        if self.species_names() != Some(film.component_names())
            || self.phase_fractions().is_some()
            || self.gas_active()
            || self
                .fields()
                .is_some_and(|rows| rows.iter().any(|r| r.concentration != 0.0))
        {
            return Err("incompatible mixture film capture state");
        }
        if let Some(f) = flight {
            if self.droplet_population.is_none()
                || f.initial_velocities.len() != self.particles.len()
                || f.initial_velocities.iter().any(|v| !finite(*v))
                || !finite(f.acceleration)
                || f.max_feature_checks == 0
            {
                return Err("invalid lifecycle flight controls");
            }
            for (i, p) in self.particles.iter().enumerate() {
                if !self.droplet_population.as_ref().unwrap()[i] {
                    continue;
                }
                let initial = super::Particle {
                    position: previous[i],
                    velocity: f.initial_velocities[i],
                    ..*p
                };
                let end = ballistic_end(initial, f.acceleration, control.dt);
                let v: [f64; 3] =
                    std::array::from_fn(|k| initial.velocity[k] + f.acceleration[k] * control.dt);
                if !finite(end)
                    || !finite(v)
                    || !finite(previous[i])
                    || (0..3).any(|k| {
                        (end[k] - p.position[k]).abs()
                            > 1e-10 * end[k].abs().max(p.position[k].abs()).max(1.0)
                            || (v[k] - p.velocity[k]).abs()
                                > 1e-10 * v[k].abs().max(p.velocity[k].abs()).max(1.0)
                    })
                {
                    return Err("inconsistent lifecycle ballistic endpoint");
                }
            }
        }
        if capture_film_immersion && free_surface_side.is_none() {
            return Err("film immersion requires free surface");
        }
        let mut staged_geometry_film = free_surface_side
            .map(|_| crate::surface_film::SurfaceFilm::from_state(&film.film().state()))
            .transpose()?;
        let mut free_surface = free_surface_side
            .map(|side| film.film().free_surface(side))
            .transpose()?;
        let acceleration = flight.map_or([0.0; 3], |f| f.acceleration);
        let mut candidate = self.clone();
        let mut paths: Vec<_> = candidate
            .particles
            .iter()
            .zip(radii)
            .enumerate()
            .map(|(i, (p, &radius))| Path {
                end: p.position,
                remaining: control.dt,
                radius,
                contacts: 0,
                accelerated: flight.is_some()
                    && candidate.droplet_population.as_ref().is_some_and(|f| f[i]),
            })
            .collect();
        for (i, (p, &start)) in candidate.particles.iter_mut().zip(previous).enumerate() {
            p.position = start;
            if paths[i].accelerated {
                p.velocity = flight.unwrap().initial_velocities[i];
            }
        }
        let mut report = FilmImpactEventReport {
            impact: DepositingImpactReport {
                deposition: FilmMixtureCapture {
                    capture: FilmCapture {
                        particles: 0,
                        deposited_volume: 0.0,
                        absorbed: ExchangeTotals {
                            thermal_energy: self.fields().map(|_| 0.0),
                            dissolved_mass: self.fields().map(|_| 0.0),
                            ..ExchangeTotals::default()
                        },
                    },
                    component_masses: vec![0.0; film.component_names().len()],
                },
                spray: ImpactSprayReport::default(),
            },
            events: 0,
            coalescence: super::DropletCoalescenceReport::default(),
            flight: DropletFlightReport::default(),
        };
        let mut deposits = Vec::new();
        let mut pair_checks = 0usize;
        let mut applied_deposits = 0usize;
        loop {
            if let Some(staged) = &mut staged_geometry_film {
                if applied_deposits != deposits.len() {
                    let additions: Vec<_> = deposits[applied_deposits..]
                        .iter()
                        .map(|(cell, volume, _)| (*cell, *volume))
                        .collect();
                    staged.deposit_batch(&additions)?;
                    applied_deposits = deposits.len();
                    free_surface = Some(staged.free_surface(free_surface_side.unwrap())?);
                }
            }
            let collision_geometry = free_surface
                .as_ref()
                .map_or(film.film(), |surface| surface.collision_geometry());
            if capture_film_immersion {
                let surface = free_surface
                    .as_ref()
                    .ok_or("missing film immersion geometry")?;
                let mut immersed = None;
                for (i, (p, path)) in candidate.particles.iter().zip(&paths).enumerate() {
                    let cell = surface.immersed_cell(
                        p.position,
                        flight.map_or(candidate.config.max_neighbor_checks, |f| {
                            f.max_feature_checks
                        }),
                    )?;
                    let wet_overlap = surface.overlapping_wet_cell(p.position, path.radius)?;
                    if let Some(cell) = cell.or(wet_overlap) {
                        if norm(p.velocity) > model.capture_speed {
                            return Err("fast immersed droplet requires wet impact response");
                        }
                        immersed = Some((i, cell));
                        break;
                    }
                }
                if let Some((i, cell)) = immersed {
                    if report.events == control.max_events {
                        return Err("film impact event budget");
                    }
                    if paths[i].contacts == control.max_contacts_per_lineage {
                        return Err("film impact lineage contact budget");
                    }
                    report.events += 1;
                    prepare_capture(&mut candidate, film, i, cell, &mut report, &mut deposits)?;
                    paths.remove(i);
                    continue;
                }
            }
            let mut next = None;
            let mut wall_hits = Vec::with_capacity(paths.len());
            for (i, (particle, path)) in candidate.particles.iter().zip(&paths).enumerate() {
                let wall = if path.accelerated && path.remaining > 0.0 {
                    let hit = collision_geometry.first_closing_accelerated_sphere_hit(
                        particle.position,
                        particle.velocity,
                        acceleration,
                        path.remaining,
                        path.radius,
                        flight.unwrap().max_feature_checks,
                    )?;
                    if hit.is_some_and(|h| h.penetration > 64.0 * f64::EPSILON * path.radius) {
                        return Err("initial film sphere penetration");
                    }
                    hit
                } else {
                    super::film_rebound::sphere_contact(
                        particle.position,
                        path.end,
                        collision_geometry,
                        path.radius,
                    )?
                };
                wall_hits.push(wall);
                if let Some(hit) = wall {
                    if next.as_ref().is_none_or(|(_, old, _)| hit.time < *old) {
                        next = Some((i, hit.time, hit));
                    }
                }
            }
            let mut pair = None;
            if let Some(c) = coalescence {
                for i in 0..candidate.particles.len() {
                    for j in 0..i {
                        if candidate
                            .droplet_population
                            .as_ref()
                            .is_some_and(|flags| !flags[i] || !flags[j])
                        {
                            continue;
                        }

                        pair_checks += 1;
                        if pair_checks > candidate.config.max_neighbor_checks {
                            return Err("lifecycle pair search budget");
                        }
                        if let Some(t) = super::droplet_coalescence::pair_time(
                            candidate.particles[i],
                            candidate.particles[j],
                            paths[i].end,
                            paths[j].end,
                            paths[i].radius + paths[j].radius,
                            c.maximum_normal_speed,
                        )
                        .map_err(|_| "invalid lifecycle pair contact")?
                        {
                            if pair.is_none_or(|(_, _, old)| t < old) {
                                pair = Some((i, j, t));
                            }
                        }
                    }
                }
            }
            // Wall priority for contacts at the same time within roundoff.
            let merge = pair.filter(|(_, _, t)| {
                next.as_ref()
                    .is_none_or(|(_, wall, _)| *t + 64.0 * f64::EPSILON < *wall)
            });
            let fraction = if let Some((_, _, t)) = merge {
                t
            } else if let Some((_, t, _)) = next {
                t
            } else {
                break;
            };
            if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
                return Err("impact event time overflow");
            }
            if report.events == control.max_events {
                return Err("film impact event budget");
            }
            report.events += 1;
            for ((particle, path), wall) in candidate
                .particles
                .iter_mut()
                .zip(&mut paths)
                .zip(wall_hits)
            {
                // A peer can reach the wall at the same representable time. Give
                // those contact positions the same roundoff skin as the selected
                // wall event, before the next sweep classifies initial penetration.
                let scale = particle
                    .position
                    .iter()
                    .chain(&path.end)
                    .fold(path.radius, |s, x| s.max(x.abs()));
                if path.accelerated {
                    advance_flight(
                        particle,
                        acceleration,
                        path.remaining * fraction,
                        &mut report.flight,
                    );
                } else {
                    for k in 0..3 {
                        particle.position[k] += fraction * (path.end[k] - particle.position[k]);
                    }
                }
                if let Some(hit) = wall.filter(|h| (h.time - fraction).abs() <= 64.0 * f64::EPSILON)
                {
                    for k in 0..3 {
                        particle.position[k] += 128.0 * f64::EPSILON * scale * hit.normal[k];
                    }
                }
                path.remaining *= 1.0 - fraction;
                if path.accelerated {
                    path.end = ballistic_end(*particle, acceleration, path.remaining);
                }
                if !finite(particle.position) || !finite(particle.velocity) || !finite(path.end) {
                    return Err("lifecycle path overflow");
                }
            }
            if let Some((i, j, _)) = merge {
                if report.coalescence.events.len()
                    == coalescence
                        .ok_or("missing coalescence controls")?
                        .max_events
                {
                    return Err("lifecycle merge budget");
                }
                let a = candidate.particles[i];
                let b = candidate.particles[j];
                let mass = a.mass + b.mass;
                let end = std::array::from_fn(|k| {
                    a.mass / mass * paths[i].end[k] + b.mass / mass * paths[j].end[k]
                });
                let contacts = paths[i].contacts.max(paths[j].contacts);
                let accelerated = paths[i].accelerated;
                if accelerated != paths[j].accelerated {
                    return Err("incompatible lifecycle pair acceleration");
                }
                let remaining = paths[i].remaining;
                let event = candidate
                    .merge_droplets(&[i, j], model.spray.surface_tension)
                    .map_err(|_| "lifecycle merge failed")?;
                let radius = candidate
                    .equivalent_sphere_radii()
                    .map_err(|_| "invalid merged radius")?[event.particle_index];
                let center = candidate.particles[event.particle_index].position;
                let growth_contact = collision_geometry
                    .first_sphere_hit(center, center, radius)?
                    .filter(|h| h.penetration > 64.0 * f64::EPSILON * radius);
                if growth_contact.is_some()
                    && (!capture_growth_contact
                        || norm(candidate.particles[event.particle_index].velocity)
                            > model.capture_speed)
                {
                    return Err("merged sphere penetrates film mesh");
                }
                paths.remove(i);
                paths.remove(j);
                paths.push(Path {
                    end,
                    remaining,
                    radius,
                    contacts,
                    accelerated,
                });
                report.coalescence.released_surface_energy += event.released_surface_energy;
                report.coalescence.released_polymer_energy += event.released_polymer_energy;
                report.coalescence.unresolved_kinetic_energy += event.unresolved_kinetic_energy;
                for k in 0..3 {
                    report.coalescence.unresolved_angular_momentum[k] +=
                        event.unresolved_angular_momentum[k];
                }
                if let Some(hit) = growth_contact {
                    if report.events == control.max_events {
                        return Err("film impact event budget");
                    }
                    if contacts == control.max_contacts_per_lineage {
                        return Err("film impact lineage contact budget");
                    }
                    report.events += 1;
                    prepare_capture(
                        &mut candidate,
                        film,
                        event.particle_index,
                        hit.cell,
                        &mut report,
                        &mut deposits,
                    )?;
                    paths.pop();
                }
                report.coalescence.events.push(event);
                continue;
            }
            let (i, _, mut hit) = next.ok_or("missing wall event")?;
            hit.time = 0.0;
            if paths[i].contacts == control.max_contacts_per_lineage {
                return Err("film impact lineage contact budget");
            }
            let parent = candidate.particles[i];
            let speed = norm(parent.velocity);
            if !speed.is_finite() || dot(parent.velocity, hit.normal) > 0.0 {
                return Err("inconsistent deposition impact velocity");
            }
            if speed <= model.capture_speed {
                prepare_capture(
                    &mut candidate,
                    film,
                    i,
                    hit.cell,
                    &mut report,
                    &mut deposits,
                )?;
                paths.remove(i);
                continue;
            }
            let direction = sub(paths[i].end, parent.position);
            let reflect = |v: [f64; 3]| {
                let normal = dot(v, hit.normal);
                std::array::from_fn(|k| {
                    (1.0 - model.spray.rebound.friction) * (v[k] - normal * hit.normal[k])
                        - model.spray.rebound.restitution * normal * hit.normal[k]
                })
            };
            let after = reflect(parent.velocity);
            let remainder = if paths[i].accelerated {
                let dt = paths[i].remaining;
                std::array::from_fn(|k| after[k] * dt + 0.5 * acceleration[k] * dt * dt)
            } else {
                reflect(direction.map(|x| (1.0 - hit.time) * x))
            };
            let contact: [f64; 3] =
                std::array::from_fn(|k| parent.position[k] + hit.time * direction[k]);
            let scale = contact
                .iter()
                .chain(&direction)
                .fold(paths[i].radius, |s, x| s.max(x.abs()));
            let position =
                std::array::from_fn(|k| contact[k] + 128.0 * f64::EPSILON * scale * hit.normal[k]);
            let remaining = paths[i].remaining * (1.0 - hit.time);
            let contacts = paths[i].contacts + 1;
            let lost =
                0.5 * parent.mass * (dot(parent.velocity, parent.velocity) - dot(after, after));
            if !finite(position)
                || !finite(after)
                || !finite(remainder)
                || !lost.is_finite()
                || lost < -1e-12
            {
                return Err("film event rebound overflow");
            }
            for k in 0..3 {
                report.impact.spray.substrate_impulse[k] +=
                    parent.mass * (parent.velocity[k] - after[k]);
            }
            report.impact.spray.impacts += 1;
            let permits_splash = if let Some(onset) = onset {
                let material = candidate
                    .effective_materials()
                    .map_err(|_| "invalid onset material")?[i];
                let diameter = 2.0
                    * (3.0 * parent.mass / (4.0 * std::f64::consts::PI * material.rest_density))
                        .cbrt();
                onset.permits(super::ImpactNumbers::new(
                    material.rest_density,
                    material.viscosity,
                    model.spray.surface_tension,
                    diameter,
                    (-dot(parent.velocity, hit.normal)).max(0.0),
                )?)?
            } else {
                true
            };
            candidate.particles[i].position = position;
            candidate.particles[i].velocity = after;
            let budget = model.spray.fragmentation_fraction * lost.max(0.0);
            let surface = if let Some(fractions) = fractions {
                candidate.droplet_fragment_surface_energy_with_mass_fractions(
                    i,
                    fractions,
                    model.spray.surface_tension,
                )
            } else {
                candidate.droplet_fragment_surface_energy(
                    i,
                    model.spray.children,
                    model.spray.surface_tension,
                )
            }
            .map_err(|_| "invalid event surface energy")?;
            if permits_splash && budget > 0.0 && budget >= surface {
                let controls = DropletSplit {
                    children: model.spray.children,
                    axis: hit.normal,
                    position_radius: model.spray.position_radius,
                    surface_tension: model.spray.surface_tension,
                    available_energy: budget,
                };
                let split = if let Some(fractions) = fractions {
                    candidate.split_droplet_with_mass_fractions(i, controls, fractions)
                } else {
                    candidate.split_droplet(i, controls)
                }
                .map_err(|_| "film event fragmentation failed")?;
                report.impact.spray.substrate_heat += lost.max(0.0) - budget;
                report.impact.spray.fragmented_particles += 1;
                report.impact.spray.fragments_created += split.children;
                report.impact.spray.created_surface_energy += split.created_surface_energy;
                report.impact.spray.added_fragment_kinetic_energy += split.added_kinetic_energy;
                paths.remove(i);
                let first = candidate.particles.len() - split.children;
                let child_radii = candidate
                    .equivalent_sphere_radii()
                    .map_err(|_| "invalid fragment radii")?;
                for j in first..candidate.particles.len() {
                    let child = candidate.particles[j];
                    let end = if flight.is_some() {
                        ballistic_end(child, acceleration, remaining)
                    } else {
                        std::array::from_fn(|k| {
                            child.position[k]
                                + remainder[k]
                                + (child.velocity[k] - after[k]) * remaining
                        })
                    };
                    if !finite(end) {
                        return Err("fragment remaining path overflow");
                    }
                    paths.push(Path {
                        end,
                        remaining,
                        radius: child_radii[j],
                        contacts,
                        accelerated: flight.is_some(),
                    });
                }
            } else {
                report.impact.spray.substrate_heat += lost.max(0.0);
                paths[i] = Path {
                    end: std::array::from_fn(|k| position[k] + remainder[k]),
                    remaining,
                    radius: paths[i].radius,
                    contacts,
                    accelerated: paths[i].accelerated,
                };
            }
        }
        let collision_geometry = free_surface
            .as_ref()
            .map_or(film.film(), |surface| surface.collision_geometry());
        for (particle, path) in candidate.particles.iter_mut().zip(&paths) {
            if path.accelerated {
                advance_flight(particle, acceleration, path.remaining, &mut report.flight);
            }
            particle.position = path.end;
            if !finite(particle.velocity) {
                return Err("lifecycle flight velocity overflow");
            }
            if collision_geometry
                .first_sphere_hit(path.end, path.end, path.radius)?
                .is_some_and(|h| h.penetration > 64.0 * f64::EPSILON * path.radius)
            {
                return Err("outgoing event sphere penetrates film mesh");
            }
        }
        if !finite(report.flight.impulse) || !report.flight.work.is_finite() {
            return Err("lifecycle flight ledger overflow");
        }
        let spray = &report.impact.spray;
        if !finite(spray.substrate_impulse)
            || [
                spray.substrate_heat,
                spray.created_surface_energy,
                spray.added_fragment_kinetic_energy,
            ]
            .iter()
            .chain(&report.impact.deposition.component_masses)
            .any(|x| !x.is_finite())
        {
            return Err("film impact event ledger overflow");
        }
        if !report.coalescence.released_surface_energy.is_finite()
            || !report.coalescence.unresolved_kinetic_energy.is_finite()
            || !finite(report.coalescence.unresolved_angular_momentum)
        {
            return Err("lifecycle merge ledger overflow");
        }
        candidate
            .effective_materials()
            .map_err(|_| "invalid event fluid state")?;
        report.impact.deposition.capture.deposited_volume = film.deposit_batch(&deposits)?;
        *self = candidate;
        Ok(report)
    }
}
