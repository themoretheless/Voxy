use super::{Error, FloatingBody, Liquid, Particle, StepStats, finite, norm, positive, sub};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyContactConfig {
    pub restitution: f64,
    pub max_bodies: usize,
    pub max_contacts: usize,
    /// Candidate checks across the entire outer fluid step.
    pub max_checks: usize,
}
impl Default for BodyContactConfig {
    fn default() -> Self {
        Self {
            restitution: 0.0,
            max_bodies: 1024,
            max_contacts: 100_000,
            max_checks: 4_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyContactReport {
    pub fluid: StepStats,
    pub contacts: usize,
    pub checks: usize,
    /// Kinetic energy removed by inelastic normal contacts; not yet deposited as heat.
    pub dissipated_energy: f64,
}
#[derive(Clone, Copy)]
struct Node {
    position: [f64; 3],
    velocity: [f64; 3],
    mass: f64,
    radius: f64,
}
#[derive(Default)]
struct Ledger {
    contacts: usize,
    checks: usize,
    loss: f64,
}
impl Liquid {
    /// Integrates local, swept two-way normal contacts with dynamic spherical bodies.
    /// Uses each adaptive fluid substep; bodies receive fluid gravity and equal opposite
    /// contact impulses. Includes body/body contacts. No prescribed fluid layers are used.
    /// No wall collisions, body rotation, tangential friction or SPH boundary pressure here.
    /// # Errors
    /// Invalid bodies/settings, overlap, numerical failure or exhausted contact/check limits.
    /// Both the entire fluid state and the caller's bodies remain unchanged on error.
    pub fn step_with_bodies(
        &mut self,
        dt: f64,
        bodies: &mut [FloatingBody],
        config: BodyContactConfig,
    ) -> Result<BodyContactReport, Error> {
        if !(0.0..=1.0).contains(&config.restitution)
            || config.max_bodies == 0
            || config.max_contacts == 0
            || config.max_checks == 0
            || bodies.len() > config.max_bodies
        {
            return Err(Error::InvalidCollision);
        }
        if bodies.iter().any(|body| {
            !finite(body.position)
                || !finite(body.velocity)
                || !positive(body.mass)
                || !positive(body.radius)
        }) {
            return Err(Error::InvalidBuoyancy);
        }
        let mut candidate = bodies.to_vec();
        let mut ledger = Ledger::default();
        let radius = self.config.particle_radius;
        let gravity = self.config.gravity;
        let empty = self.particles.is_empty();
        let stats = self.advance(dt, None, |particles, time| {
            move_coupled(
                particles,
                &mut candidate,
                radius,
                gravity,
                time,
                config,
                &mut ledger,
            )
        })?;
        if empty {
            move_coupled(
                &mut [],
                &mut candidate,
                radius,
                gravity,
                dt,
                config,
                &mut ledger,
            )?;
        }
        bodies.copy_from_slice(&candidate);
        Ok(BodyContactReport {
            fluid: stats,
            contacts: ledger.contacts,
            checks: ledger.checks,
            dissipated_energy: ledger.loss,
        })
    }
}
fn move_coupled(
    particles: &mut [Particle],
    bodies: &mut [FloatingBody],
    radius: f64,
    gravity: [f64; 3],
    dt: f64,
    config: BodyContactConfig,
    ledger: &mut Ledger,
) -> Result<(), Error> {
    let mut nodes: Vec<_> = particles
        .iter()
        .map(|particle| Node {
            position: particle.position,
            velocity: particle.velocity,
            mass: particle.mass,
            radius,
        })
        .collect();
    let count = particles.len();
    for body in bodies.iter() {
        let velocity = std::array::from_fn(|axis| body.velocity[axis] + gravity[axis] * dt);
        if !finite(velocity) {
            return Err(Error::NumericalFailure);
        }
        nodes.push(Node {
            position: body.position,
            velocity,
            mass: body.mass,
            radius: body.radius,
        });
    }
    let mut remaining = dt;
    loop {
        let mut earliest = None;
        for i in 0..nodes.len() {
            for j in (i + 1).max(count)..nodes.len() {
                if ledger.checks >= config.max_checks {
                    return Err(Error::CollisionBudget);
                }
                ledger.checks += 1;
                if let Some(time) = impact(nodes[i], nodes[j], remaining)?
                    && earliest.is_none_or(|(_, _, old)| time < old)
                {
                    earliest = Some((i, j, time));
                }
            }
        }
        let time = earliest.map_or(remaining, |(_, _, time)| time);
        for node in &mut nodes {
            for axis in 0..3 {
                node.position[axis] += node.velocity[axis] * time;
            }
            if !finite(node.position) {
                return Err(Error::NumericalFailure);
            }
        }
        remaining = (remaining - time).max(0.0);
        let Some((i, j, _)) = earliest else {
            break;
        };
        if ledger.contacts >= config.max_contacts {
            return Err(Error::CollisionBudget);
        }
        ledger.contacts += 1;
        let delta = sub(nodes[i].position, nodes[j].position);
        let distance = norm(delta);
        if !positive(distance) {
            return Err(Error::NumericalFailure);
        }
        let normal = delta.map(|value| value / distance);
        let speed = dot(sub(nodes[i].velocity, nodes[j].velocity), normal);
        let reduced = 1.0 / (1.0 / nodes[i].mass + 1.0 / nodes[j].mass);
        if !positive(reduced) {
            return Err(Error::NumericalFailure);
        }
        let impulse = -(1.0 + config.restitution) * reduced * speed;
        ledger.loss +=
            0.5 * reduced * speed * speed * (1.0 - config.restitution * config.restitution);
        for (axis, component) in normal.into_iter().enumerate() {
            nodes[i].velocity[axis] += impulse * component / nodes[i].mass;
            nodes[j].velocity[axis] -= impulse * component / nodes[j].mass;
        }
        if !finite(nodes[i].velocity) || !finite(nodes[j].velocity) || !ledger.loss.is_finite() {
            return Err(Error::NumericalFailure);
        }
        if remaining == 0.0 {
            break;
        }
    }
    for (particle, node) in particles.iter_mut().zip(&nodes) {
        particle.position = node.position;
        particle.velocity = node.velocity;
    }
    for (body, node) in bodies.iter_mut().zip(&nodes[count..]) {
        body.position = node.position;
        body.velocity = node.velocity;
    }
    Ok(())
}
fn impact(first: Node, second: Node, remaining: f64) -> Result<Option<f64>, Error> {
    let delta = sub(first.position, second.position);
    let velocity = sub(first.velocity, second.velocity);
    let radius = first.radius + second.radius;
    let distance = norm(delta);
    let tolerance = 128.0 * f64::EPSILON * radius.max(distance).max(1.0);
    if !distance.is_finite() || !radius.is_finite() {
        return Err(Error::NumericalFailure);
    }
    if distance < radius - tolerance {
        return Err(Error::InitialOverlap);
    }
    let approach = dot(delta, velocity);
    let speed_squared = dot(velocity, velocity);
    if !approach.is_finite() || !speed_squared.is_finite() {
        return Err(Error::NumericalFailure);
    }
    if approach >= -128.0 * f64::EPSILON * distance.max(1.0) * speed_squared.sqrt().max(1.0) {
        return Ok(None);
    }
    if distance <= radius + tolerance {
        return Ok(Some(0.0));
    }
    let gap = (distance - radius) * (distance + radius);
    let discriminant = approach * approach - speed_squared * gap;
    if !discriminant.is_finite() || !speed_squared.is_finite() {
        return Err(Error::NumericalFailure);
    }
    if discriminant < 0.0 {
        return Ok(None);
    }
    // Stable smaller quadratic root avoids cancellation for near-contact positions.
    let time = gap / (-approach + discriminant.sqrt());
    if !time.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok((time >= 0.0 && time <= remaining).then_some(time))
}
fn dot(first: [f64; 3], second: [f64; 3]) -> f64 {
    first.into_iter().zip(second).map(|(a, b)| a * b).sum()
}
