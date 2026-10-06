//! Shared earliest-event solver for fluid, translating/rotating bodies and fixed walls.
use super::dynamic_world::ContactLedger;
use super::{
    DynamicEnvironmentReport, DynamicWorldConfig, DynamicWorldReport, Error, GeometryHit, Liquid,
    Particle, TranslatingBody, finite, positive,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactWitness {
    pub point: [f64; 3],
    pub tolerance_m: f64,
}
/// Optional geometry-owned world point at the reported contact time. Legacy
/// translating backends can omit it; angular response must require a witness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyGeometryHit {
    pub geometry: GeometryHit,
    pub witness: Option<ContactWitness>,
}
impl From<GeometryHit> for BodyGeometryHit {
    fn from(geometry: GeometryHit) -> Self {
        Self {
            geometry,
            witness: None,
        }
    }
}

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
    fn sweep_particle_body_contact(
        &self,
        particle: &Particle,
        radius: f64,
        body_index: usize,
        body: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<BodyGeometryHit, Error> {
        self.sweep_particle_body(particle, radius, body_index, body, dt, max_candidates)
            .map(Into::into)
    }
    fn sweep_body_pair_contact(
        &self,
        first_index: usize,
        first: &TranslatingBody,
        second_index: usize,
        second: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<BodyGeometryHit, Error> {
        self.sweep_body_pair(first_index, first, second_index, second, dt, max_candidates)
            .map(Into::into)
    }
    fn sweep_particle_environment_contact(
        &self,
        particle: &Particle,
        radius: f64,
        dt: f64,
        max_candidates: usize,
    ) -> Result<BodyGeometryHit, Error> {
        self.sweep_particle_environment(particle, radius, dt, max_candidates)
            .map(Into::into)
    }
    fn sweep_body_environment_contact(
        &self,
        body_index: usize,
        body: &TranslatingBody,
        dt: f64,
        max_candidates: usize,
    ) -> Result<BodyGeometryHit, Error> {
        self.sweep_body_environment(body_index, body, dt, max_candidates)
            .map(Into::into)
    }
    /// Prepared angular trajectory; legacy backends reject intrinsic spin.
    fn sweep_particle_rigid_contact(
        &self,
        p: &Particle,
        radius: f64,
        index: usize,
        path: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, Error> {
        let body = path.initial();
        if body.spin.is_some() {
            return Err(Error::CollisionBackend);
        }
        self.sweep_particle_body_contact(
            p,
            radius,
            index,
            &TranslatingBody {
                position: body.motion.position,
                velocity: body.motion.velocity,
                mass: body.motion.mass,
            },
            path.duration(),
            budget,
        )
    }
    fn sweep_rigid_pair_contact(
        &self,
        i: usize,
        first: &crate::rigid_motion::RigidMotion,
        j: usize,
        second: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, Error> {
        let a = first.initial();
        let b = second.initial();
        if a.spin.is_some() || b.spin.is_some() {
            return Err(Error::CollisionBackend);
        }
        self.sweep_body_pair_contact(
            i,
            &TranslatingBody {
                position: a.motion.position,
                velocity: a.motion.velocity,
                mass: a.motion.mass,
            },
            j,
            &TranslatingBody {
                position: b.motion.position,
                velocity: b.motion.velocity,
                mass: b.motion.mass,
            },
            first.duration(),
            budget,
        )
    }
    fn sweep_rigid_environment_contact(
        &self,
        index: usize,
        path: &crate::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<BodyGeometryHit, Error> {
        let body = path.initial();
        if body.spin.is_some() {
            return Err(Error::CollisionBackend);
        }
        self.sweep_body_environment_contact(
            index,
            &TranslatingBody {
                position: body.motion.position,
                velocity: body.motion.velocity,
                mass: body.motion.mass,
            },
            path.duration(),
            budget,
        )
    }
    fn has_environment(&self) -> bool {
        true
    }
}
#[derive(Clone, Copy)]
struct Node {
    spin: Option<crate::astrophysics_spin::Spin>,
    position: [f64; 3],
    velocity: [f64; 3],
    mass: f64,
}
impl Node {
    fn contact(self) -> crate::contact::ContactBody {
        crate::contact::ContactBody {
            motion: crate::gravity::Body {
                mass: self.mass,
                position: self.position,
                velocity: self.velocity,
            },
            spin: self.spin,
        }
    }
    fn update(&mut self, body: crate::contact::ContactBody) {
        self.position = body.motion.position;
        self.velocity = body.motion.velocity;
        self.spin = body.spin;
    }
}

fn default_rotation() -> crate::spin_path::Config {
    crate::spin_path::Config {
        max_angular_error_rad: 1e-5,
        min_step_s: 1e-9,
        max_arcs: 1024,
        max_trials: 4096,
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
    let mut rigid: Vec<_> = bodies
        .iter()
        .map(|b| crate::contact::ContactBody {
            motion: crate::gravity::Body {
                position: b.position,
                velocity: b.velocity,
                mass: b.mass,
            },
            spin: None,
        })
        .collect();
    solve_contact(
        particles,
        &mut rigid,
        radius,
        dt,
        config,
        ledger,
        world,
        default_rotation(),
    )?;
    for (body, state) in bodies.iter_mut().zip(rigid) {
        body.position = state.motion.position;
        body.velocity = state.motion.velocity;
    }
    Ok(())
}

fn solve_contact(
    particles: &mut [Particle],
    bodies: &mut [crate::contact::ContactBody],
    radius: f64,
    dt: f64,
    config: DynamicWorldConfig,
    ledger: &mut ContactLedger,
    world: &impl LiquidBodyWorld,
    rotation: crate::spin_path::Config,
) -> Result<(), Error> {
    let count = particles.len();
    let mut nodes: Vec<_> = particles
        .iter()
        .map(|p| Node {
            spin: None,
            position: p.position,
            velocity: p.velocity,
            mass: p.mass,
        })
        .chain(bodies.iter().map(|b| Node {
            position: b.motion.position,
            velocity: b.motion.velocity,
            mass: b.motion.mass,
            spin: b.spin,
        }))
        .collect();
    let mut remaining = dt;
    let require_witness = bodies.iter().any(|body| body.spin.is_some());
    while remaining > 0. {
        let paths: Vec<_> = nodes
            .iter()
            .map(|n| {
                n.contact()
                    .prepare_motion([0.; 3], [0.; 3], remaining, rotation)
                    .map_err(|_| Error::NumericalFailure)
            })
            .collect::<Result<_, _>>()?;
        let mut earliest: Option<(usize, Option<usize>, f64, [f64; 3], Option<ContactWitness>)> =
            None;
        let mut admit =
            |first: usize, second: Option<usize>, hit: BodyGeometryHit| -> Result<(), Error> {
                if hit.witness.is_some() && !matches!(hit.geometry, GeometryHit::Contact { .. }) {
                    return Err(Error::InvalidCollision);
                }
                let GeometryHit::Contact { fraction, normal } = hit.geometry else {
                    return if matches!(hit.geometry, GeometryHit::Overlap) {
                        Err(Error::InitialOverlap)
                    } else {
                        Ok(())
                    };
                };
                if hit.witness.is_some_and(|w| {
                    !finite(w.point) || !w.tolerance_m.is_finite() || w.tolerance_m < 0.
                }) {
                    return Err(Error::InvalidCollision);
                }
                if require_witness && hit.witness.is_none() {
                    return Err(Error::InvalidCollision);
                }
                let norm = normal[0].hypot(normal[1]).hypot(normal[2]);
                if !fraction.is_finite()
                    || !(0. ..=1.).contains(&fraction)
                    || !norm.is_finite()
                    || (norm - 1.).abs() > 5e-11
                {
                    return Err(Error::InvalidCollision);
                }
                let normal = normal.map(|n| n / norm);
                let time = remaining * fraction;
                let point = hit.witness.map_or(nodes[first].position, |w| w.point);
                let a = paths[first]
                    .sample(time)
                    .map_err(|_| Error::NumericalFailure)?;
                let va = a
                    .point_velocity(point)
                    .map_err(|_| Error::NumericalFailure)?;
                let vb = if let Some(index) = second {
                    paths[index]
                        .sample(time)
                        .map_err(|_| Error::NumericalFailure)?
                        .point_velocity(point)
                        .map_err(|_| Error::NumericalFailure)?
                } else {
                    [0.; 3]
                };
                let relative: [f64; 3] = std::array::from_fn(|k| va[k] - vb[k]);
                let speed: f64 = (0..3).map(|a| relative[a] * normal[a]).sum();
                if !speed.is_finite() || speed >= 0. {
                    return Err(Error::InvalidCollision);
                }
                if earliest.is_none_or(|(_, _, old, _, _)| fraction < old) {
                    earliest = Some((first, second, fraction, normal, hit.witness));
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
                    world.sweep_particle_rigid_contact(
                        &p,
                        radius,
                        body_index,
                        &paths[count + body_index],
                        config.contact.max_candidates,
                    )?,
                )?;
            }
            if world.has_environment() {
                charge(ledger, config)?;
                admit(
                    index,
                    None,
                    world.sweep_particle_environment_contact(
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
                    world.sweep_rigid_pair_contact(
                        first,
                        &paths[count + first],
                        second,
                        &paths[count + second],
                        config.contact.max_candidates,
                    )?,
                )?;
            }
            if world.has_environment() {
                charge(ledger, config)?;
                admit(
                    count + first,
                    None,
                    world.sweep_rigid_environment_contact(
                        first,
                        &paths[count + first],
                        config.contact.max_candidates,
                    )?,
                )?;
            }
        }
        let fraction = earliest.map_or(1., |(_, _, f, _, _)| f);
        for (index, n) in nodes.iter_mut().enumerate() {
            if n.spin.is_some() {
                n.spin = paths[index]
                    .sample(remaining * fraction)
                    .map_err(|_| Error::NumericalFailure)?
                    .spin;
            }
            for a in 0..3 {
                n.position[a] += n.velocity[a] * remaining * fraction;
            }
            if !finite(n.position) {
                return Err(Error::NumericalFailure);
            }
        }
        let Some((first, second, _, normal, witness)) = earliest else {
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
        let first_contact = nodes[first].contact();
        let second_contact = second.map(|s| nodes[s].contact());
        let normal_response = crate::contact::normal_impulse(
            &first_contact,
            second_contact.as_ref(),
            witness.map_or(nodes[first].position, |w| w.point),
            normal,
            config.contact.restitution,
        )
        .map_err(|_| Error::NumericalFailure)?;
        let loss = normal_response.dissipated_energy
            + if config.contact.friction == 0. {
                0.
            } else {
                0.5 * reduced
                    * tangent.iter().map(|v| v * v).sum::<f64>()
                    * config.contact.friction
                    * (2. - config.contact.friction)
            };
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
        let impulse: [f64; 3] = std::array::from_fn(|k| {
            normal_response.impulse[k] - reduced * config.contact.friction * tangent[k]
        });
        let point = witness.map_or(nodes[first].position, |w| w.point);
        let mut a = nodes[first].contact();
        a.apply_point_impulse(point, impulse)
            .map_err(|_| Error::NumericalFailure)?;
        nodes[first].update(a);
        if let Some(index) = second {
            let mut b = nodes[index].contact();
            b.apply_point_impulse(point, impulse.map(|j| -j))
                .map_err(|_| Error::NumericalFailure)?;
            nodes[index].update(b);
        } else {
            for k in 0..3 {
                ledger.environment_impulse[k] -= impulse[k];
            }
        }
        for k in 0..3 {
            nodes[first].position[k] += normal[k] * epsilon;
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
        *b = n.contact();
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
        if bodies.len() > max_bodies || max_bodies == 0 {
            return Err(Error::InvalidCollision);
        }
        let mut rigid: Vec<_> = bodies
            .iter()
            .map(|b| crate::contact::ContactBody {
                motion: crate::gravity::Body {
                    position: b.position,
                    velocity: b.velocity,
                    mass: b.mass,
                },
                spin: None,
            })
            .collect();
        let report = self.step_with_rigid_body_world(
            dt,
            &mut rigid,
            world,
            config,
            max_bodies,
            default_rotation(),
        )?;
        for (body, state) in bodies.iter_mut().zip(rigid) {
            body.position = state.motion.position;
            body.velocity = state.motion.velocity;
        }
        Ok(report)
    }
    /// Same event loop with intrinsic spin. Geometry must admit angular paths and
    /// world witnesses. Legacy tangential damping is unsupported for rigid spin.
    pub fn step_with_rigid_body_world(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
    ) -> Result<DynamicEnvironmentReport, Error> {
        config.contact.validate()?;
        if bodies.len() > max_bodies
            || max_bodies == 0
            || config.max_contacts == 0
            || config.max_queries == 0
            || bodies
                .iter()
                .any(|b| b.energy().is_err() || (b.spin.is_some() && config.contact.friction != 0.))
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
                    body.motion.velocity[a] += gravity[a] * time;
                }
            }
            solve_contact(
                particles,
                &mut bodies_candidate,
                radius,
                time,
                config,
                &mut ledger,
                world,
                rotation,
            )
        })?;
        if empty {
            for body in &mut bodies_candidate {
                for a in 0..3 {
                    body.motion.velocity[a] += gravity[a] * dt;
                }
            }
            solve_contact(
                &mut [],
                &mut bodies_candidate,
                radius,
                dt,
                config,
                &mut ledger,
                world,
                rotation,
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
