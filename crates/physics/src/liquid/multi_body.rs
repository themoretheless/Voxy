//! Shared earliest-event translation solver for fluid, finite bodies and fixed walls.
use super::dynamic_world::ContactLedger;
use super::{
    DynamicEnvironmentReport, DynamicWorldConfig, DynamicWorldReport, Error, GeometryHit, Liquid,
    Particle, TranslatingBody, finite, positive,
};

/// Geometry ownership stays with the backend; all fractions refer to this interval.
pub trait LiquidBodyWorld {
    fn sweep_particle_body(
        &self,
        particle: &Particle,
        radius: f64,
        body_index: usize,
        body: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<GeometryHit, Error>;
    fn sweep_body_pair(
        &self,
        first_index: usize,
        first: &TranslatingBody,
        second_index: usize,
        second: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<GeometryHit, Error>;
    fn sweep_particle_environment(
        &self,
        particle: &Particle,
        radius: f64,
        dt: f64,
        max_candidates: usize,
    ) -> Result<GeometryHit, Error>;
    fn sweep_body_environment(
        &self,
        body_index: usize,
        body: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<GeometryHit, Error>;
    fn has_environment(&self) -> bool {
        true
    }
}
#[derive(Clone, Copy)]
struct Node {
    position: [f64; 3],
    velocity: [f64; 3],
    mass: f64,
}
impl Node {
    fn body(self) -> TranslatingBody {
        TranslatingBody {
            position: self.position,
            velocity: self.velocity,
            mass: self.mass,
        }
    }
}

pub(super) fn solve(
    particles: &mut [Particle],
    bodies: &mut [TranslatingBody],
    radius: f64,
    dt: f64,
    config: DynamicWorldConfig,
    ledger: &mut ContactLedger,
    world: &impl LiquidBodyWorld,
) -> Result<(), Error> {
    let count = particles.len();
    let mut nodes: Vec<_> = particles
        .iter()
        .map(|p| Node {
            position: p.position,
            velocity: p.velocity,
            mass: p.mass,
        })
        .chain(bodies.iter().map(|b| Node {
            position: b.position,
            velocity: b.velocity,
            mass: b.mass,
        }))
        .collect();
    let mut remaining = dt;
    while remaining > 0. {
        let mut earliest: Option<(usize, Option<usize>, f64, [f64; 3])> = None;
        let mut admit =
            |first: usize, second: Option<usize>, hit: GeometryHit| -> Result<(), Error> {
                let GeometryHit::Contact { fraction, normal } = hit else {
                    return if matches!(hit, GeometryHit::Overlap) {
                        Err(Error::InitialOverlap)
                    } else {
                        Ok(())
                    };
                };
                let relative: [f64; 3] = std::array::from_fn(|a| {
                    nodes[first].velocity[a] - second.map_or(0., |s| nodes[s].velocity[a])
                });
                let norm = normal[0].hypot(normal[1]).hypot(normal[2]);
                if !fraction.is_finite()
                    || !(0. ..=1.).contains(&fraction)
                    || !norm.is_finite()
                    || (norm - 1.).abs() > 5e-11
                {
                    return Err(Error::InvalidCollision);
                }
                let normal = normal.map(|n| n / norm);
                let speed: f64 = (0..3).map(|a| relative[a] * normal[a]).sum();
                if !speed.is_finite() || speed >= 0. {
                    return Err(Error::InvalidCollision);
                }
                if earliest.is_none_or(|(_, _, old, _)| fraction < old) {
                    earliest = Some((first, second, fraction, normal));
                }
                Ok(())
            };
        for (index, original) in particles.iter().enumerate() {
            let p = Particle {
                position: nodes[index].position,
                velocity: nodes[index].velocity,
                ..*original
            };
            for body_index in 0..bodies.len() {
                charge(ledger, config)?;
                admit(
                    index,
                    Some(count + body_index),
                    world.sweep_particle_body(
                        &p,
                        radius,
                        body_index,
                        &nodes[count + body_index].body(),
                        remaining,
                        config.contact.max_candidates,
                    )?,
                )?;
            }
            if world.has_environment() {
                charge(ledger, config)?;
                admit(
                    index,
                    None,
                    world.sweep_particle_environment(
                        &p,
                        radius,
                        remaining,
                        config.contact.max_candidates,
                    )?,
                )?;
            }
        }
        for first in 0..bodies.len() {
            for second in first + 1..bodies.len() {
                charge(ledger, config)?;
                admit(
                    count + first,
                    Some(count + second),
                    world.sweep_body_pair(
                        first,
                        &nodes[count + first].body(),
                        second,
                        &nodes[count + second].body(),
                        remaining,
                        config.contact.max_candidates,
                    )?,
                )?;
            }
            if world.has_environment() {
                charge(ledger, config)?;
                admit(
                    count + first,
                    None,
                    world.sweep_body_environment(
                        first,
                        &nodes[count + first].body(),
                        remaining,
                        config.contact.max_candidates,
                    )?,
                )?;
            }
        }
        let fraction = earliest.map_or(1., |(_, _, f, _)| f);
        for n in &mut nodes {
            for a in 0..3 {
                n.position[a] += n.velocity[a] * remaining * fraction;
            }
            if !finite(n.position) {
                return Err(Error::NumericalFailure);
            }
        }
        let Some((first, second, _, normal)) = earliest else {
            break;
        };
        if ledger.contacts >= config.max_contacts {
            return Err(Error::CollisionBudget);
        }
        ledger.contacts += 1;
        let relative: [f64; 3] = std::array::from_fn(|a| {
            nodes[first].velocity[a] - second.map_or(0., |s| nodes[s].velocity[a])
        });
        let reduced = second.map_or(nodes[first].mass, |s| {
            1. / (1. / nodes[first].mass + 1. / nodes[s].mass)
        });
        if !positive(reduced) {
            return Err(Error::NumericalFailure);
        }
        let speed: f64 = (0..3).map(|a| relative[a] * normal[a]).sum();
        let tangent: [f64; 3] = std::array::from_fn(|a| relative[a] - speed * normal[a]);
        let loss = 0.5
            * reduced
            * (speed * speed * (1. - config.contact.restitution.powi(2))
                + tangent.iter().map(|v| v * v).sum::<f64>()
                    * config.contact.friction
                    * (2. - config.contact.friction));
        ledger.loss += loss;
        if let Some(total) = ledger.particle_loss.get_mut(first) {
            *total += loss;
        }
        let reference = second.map_or(nodes[first].position, |s| nodes[s].position);
        let epsilon = 64.
            * f64::EPSILON
            * (0..3)
                .map(|a| normal[a].abs() * nodes[first].position[a].abs().max(reference[a].abs()))
                .fold(1., f64::max);
        for a in 0..3 {
            let impulse = -reduced
                * ((1. + config.contact.restitution) * speed * normal[a]
                    + config.contact.friction * tangent[a]);
            nodes[first].velocity[a] += impulse / nodes[first].mass;
            if let Some(second) = second {
                nodes[second].velocity[a] -= impulse / nodes[second].mass;
            } else {
                ledger.environment_impulse[a] -= impulse;
            }
            nodes[first].position[a] += normal[a] * epsilon;
        }
        if !nodes
            .iter()
            .all(|n| finite(n.velocity) && finite(n.position))
            || !finite(ledger.environment_impulse)
            || !ledger.loss.is_finite()
        {
            return Err(Error::NumericalFailure);
        }
        remaining *= 1. - fraction;
    }
    for (p, n) in particles.iter_mut().zip(&nodes) {
        p.position = n.position;
        p.velocity = n.velocity;
    }
    for (b, n) in bodies.iter_mut().zip(&nodes[count..]) {
        b.position = n.position;
        b.velocity = n.velocity;
    }
    Ok(())
}
fn charge(ledger: &mut ContactLedger, config: DynamicWorldConfig) -> Result<(), Error> {
    if ledger.queries >= config.max_queries {
        return Err(Error::CollisionBudget);
    }
    ledger.queries += 1;
    Ok(())
}

impl Liquid {
    /// Coupled finite-body/body, fluid/body and fixed-world events in one transaction.
    /// Rotation is constrained; backend templates may be affine or compound geometry.
    /// # Errors
    /// Invalid bodies/budgets/backend contacts or numerical failure restore every owner.
    pub fn step_with_body_world(
        &mut self,
        dt: f64,
        bodies: &mut [TranslatingBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
    ) -> Result<DynamicEnvironmentReport, Error> {
        config.contact.validate()?;
        if bodies.len() > max_bodies
            || max_bodies == 0
            || config.max_contacts == 0
            || config.max_queries == 0
            || bodies
                .iter()
                .any(|b| !finite(b.position) || !finite(b.velocity) || !positive(b.mass))
        {
            return Err(Error::InvalidCollision);
        }
        let mut candidate = self.clone();
        let mut bodies_candidate = bodies.to_vec();
        let mut ledger = ContactLedger::default();
        let radius = self.config.particle_radius;
        let gravity = self.config.gravity;
        let empty = self.particles.is_empty();
        let fluid = candidate.advance(dt, None, |particles, time| {
            for body in &mut bodies_candidate {
                for a in 0..3 {
                    body.velocity[a] += gravity[a] * time;
                }
            }
            solve(
                particles,
                &mut bodies_candidate,
                radius,
                time,
                config,
                &mut ledger,
                world,
            )
        })?;
        if empty {
            for body in &mut bodies_candidate {
                for a in 0..3 {
                    body.velocity[a] += gravity[a] * dt;
                }
            }
            solve(
                &mut [],
                &mut bodies_candidate,
                radius,
                dt,
                config,
                &mut ledger,
                world,
            )?;
        }
        *self = candidate;
        bodies.copy_from_slice(&bodies_candidate);
        Ok(DynamicEnvironmentReport {
            dynamics: DynamicWorldReport {
                fluid,
                contacts: ledger.contacts,
                queries: ledger.queries,
                dissipated_energy: ledger.loss,
            },
            environment_impulse: ledger.environment_impulse,
        })
    }
}
