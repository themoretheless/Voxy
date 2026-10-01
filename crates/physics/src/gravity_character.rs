//! Spherical kinematic character with gravity-relative locomotion and arbitrary
//! surface normals. Coordinate anchors and query budgets are independent of voxels.
use crate::{Origin, gravity, gravity_field::GravityField};

type Vector = [f64; 3];
fn dot(a: Vector, b: Vector) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn norm(v: Vector) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn add(a: &mut Vector, b: Vector) {
    for k in 0..3 {
        a[k] += b[k];
    }
}
fn clip(v: &mut Vector, normal: Vector) {
    let inward = dot(*v, normal).min(0.0);
    for k in 0..3 {
        v[k] -= normal[k] * inward;
    }
}
fn finite(v: Vector) -> bool {
    v.iter().all(|x| x.is_finite())
}
fn up_from(acceleration: Vector, fallback: Vector) -> Vector {
    let scale = acceleration.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
    if scale > 0.0 {
        let scaled = acceleration.map(|v| v / scale);
        let length = norm(scaled);
        scaled.map(|v| -v / length)
    } else {
        fallback
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub anchor: Origin,
    pub center: Vector,
    pub radius: f64,
    pub velocity: Vector,
    /// Unit fallback direction at zero gravity; updated from the field each step.
    pub up: Vector,
    pub grounded: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    /// World-space desired velocity, projected onto the tangent plane.
    /// None preserves momentum, including while airborne.
    pub tangent_velocity: Option<Vector>,
    pub jump_pressed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub jump_speed: f64,
    pub ground_snap: f64,
    /// Minimum dot(contact normal, up) accepted as ground, in (0, 1].
    pub ground_cosine: f64,
    pub max_iterations: usize,
    pub max_candidates: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            jump_speed: 4.0,
            ground_snap: 0.05,
            ground_cosine: 0.7,
            max_iterations: 8,
            max_candidates: 16_384,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub fraction: f64,
    pub normal: Vector,
    pub obstacle: Option<usize>,
}
pub trait CollisionWorld {
    /// # Errors
    /// Missing geometry, initial overlap, invalid inputs, or exhausted query work.
    fn sweep(
        &self,
        state: &State,
        displacement: Vector,
        max_candidates: usize,
    ) -> Result<Hit, Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    InvalidContact,
    InitialOverlap,
    BudgetExceeded,
    CoordinateOverflow,
    Gravity(gravity::Error),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub applied_displacement: Vector,
    pub contacts: Vec<usize>,
    pub jumped: bool,
}

fn checked_hit(hit: Hit) -> Result<Hit, Error> {
    if !hit.fraction.is_finite()
        || !(0.0..=1.0).contains(&hit.fraction)
        || !finite(hit.normal)
        || (hit.obstacle.is_some() && (norm(hit.normal) - 1.0).abs() > 1e-9)
        || (hit.obstacle.is_none() && hit.fraction < 1.0)
    {
        return Err(Error::InvalidContact);
    }
    Ok(hit)
}
fn rebase(state: &mut State) -> Result<(), Error> {
    for k in 0..3 {
        let shift = state.center[k].floor();
        // The upper boundary is exclusive: i64::MAX rounds to 2^63 in f64.
        if !shift.is_finite()
            || !(-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&shift)
        {
            return Err(Error::CoordinateOverflow);
        }
        #[allow(clippy::cast_possible_truncation)]
        let integer = shift as i64;
        let anchor = match k {
            0 => &mut state.anchor.x,
            1 => &mut state.anchor.y,
            _ => &mut state.anchor.z,
        };
        *anchor = anchor
            .checked_add(integer)
            .ok_or(Error::CoordinateOverflow)?;
        state.center[k] -= shift;
    }
    Ok(())
}

fn validate(state: &State, input: Input, dt: f64, config: Config) -> Result<(), Error> {
    if !finite(state.center)
        || !finite(state.velocity)
        || !finite(state.up)
        || (norm(state.up) - 1.0).abs() > 1e-9
        || !state.radius.is_finite()
        || state.radius <= 0.0
        || !dt.is_finite()
        || !(0.0..=1.0).contains(&dt)
        || dt == 0.0
        || input.tangent_velocity.is_some_and(|v| !finite(v))
        || !config.jump_speed.is_finite()
        || config.jump_speed < 0.0
        || !config.ground_snap.is_finite()
        || config.ground_snap < 0.0
        || !config.ground_cosine.is_finite()
        || config.ground_cosine <= 0.0
        || config.ground_cosine > 1.0
        || config.max_iterations == 0
        || config.max_candidates == 0
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}

/// Advances a gravity-relative sphere with sweep-and-slide and ground snapping.
/// Input velocity is a kinematic motor; gravity/free motion uses None to retain
/// momentum. Jumping disables snapping for the entire step.
/// # Errors
/// Invalid state/config, field/query failures, invalid backend contacts and work
/// exhaustion leave the original state unchanged.
pub fn step(
    world: &impl CollisionWorld,
    field: &impl GravityField,
    state: &mut State,
    input: Input,
    dt: f64,
    config: Config,
) -> Result<Step, Error> {
    validate(state, input, dt, config)?;
    let mut next = *state;
    checked_hit(world.sweep(&next, [0.0; 3], config.max_candidates)?)?;
    let acceleration = field
        .acceleration(next.anchor, next.center)
        .map_err(Error::Gravity)?;
    if !finite(acceleration) {
        return Err(Error::InvalidInput);
    }
    next.up = up_from(acceleration, next.up);
    if let Some(desired) = input.tangent_velocity {
        let vertical = dot(next.velocity, next.up);
        let unwanted = dot(desired, next.up);
        next.velocity = std::array::from_fn(|k| desired[k] + next.up[k] * (vertical - unwanted));
    }
    let jumped = input.jump_pressed && next.grounded;
    if jumped {
        let vertical = dot(next.velocity, next.up);
        add(
            &mut next.velocity,
            next.up.map(|v| v * (config.jump_speed - vertical)),
        );
    }
    add(&mut next.velocity, acceleration.map(|v| v * dt));
    if !finite(next.velocity) {
        return Err(Error::InvalidInput);
    }
    let mut remaining = next.velocity.map(|v| v * dt);
    let mut applied = [0.0; 3];
    let mut contacts = Vec::new();
    next.grounded = false;
    for iteration in 0..config.max_iterations {
        if norm(remaining) <= 1e-12 {
            break;
        }
        let hit = checked_hit(world.sweep(&next, remaining, config.max_candidates)?)?;
        let movement = remaining.map(|v| v * hit.fraction);
        add(&mut next.center, movement);
        add(&mut applied, movement);
        let Some(obstacle) = hit.obstacle else {
            break;
        };
        contacts.push(obstacle);
        if dot(hit.normal, next.up) >= config.ground_cosine {
            next.grounded = true;
        }
        clip(&mut next.velocity, hit.normal);
        remaining = remaining.map(|v| v * (1.0 - hit.fraction));
        clip(&mut remaining, hit.normal);
        if iteration + 1 == config.max_iterations && norm(remaining) > 1e-12 {
            return Err(Error::BudgetExceeded);
        }
    }
    let end_acceleration = field
        .acceleration(next.anchor, next.center)
        .map_err(Error::Gravity)?;
    if !finite(end_acceleration) {
        return Err(Error::InvalidInput);
    }
    next.up = up_from(end_acceleration, next.up);
    if !jumped && state.grounded && config.ground_snap > 0.0 {
        let downward = next.up.map(|v| -v * config.ground_snap);
        let hit = checked_hit(world.sweep(&next, downward, config.max_candidates)?)?;
        if let Some(obstacle) = hit.obstacle
            && dot(hit.normal, next.up) >= config.ground_cosine
        {
            let movement = downward.map(|v| v * hit.fraction);
            add(&mut next.center, movement);
            add(&mut applied, movement);
            clip(&mut next.velocity, hit.normal);
            contacts.push(obstacle);
            next.grounded = true;
        }
    }
    if !finite(next.center) || !finite(next.velocity) || !finite(applied) {
        return Err(Error::InvalidInput);
    }
    rebase(&mut next)?;
    *state = next;
    Ok(Step {
        applied_displacement: applied,
        contacts,
        jumped,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceSphere {
    pub anchor: Origin,
    pub center: Vector,
    pub radius: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct SphereWorld<'a> {
    pub surfaces: &'a [SurfaceSphere],
}
impl CollisionWorld for SphereWorld<'_> {
    #[allow(clippy::cast_precision_loss)]
    fn sweep(
        &self,
        state: &State,
        displacement: Vector,
        max_candidates: usize,
    ) -> Result<Hit, Error> {
        if self.surfaces.len() > max_candidates {
            return Err(Error::BudgetExceeded);
        }
        if !finite(state.center)
            || !finite(displacement)
            || !state.radius.is_finite()
            || state.radius <= 0.0
        {
            return Err(Error::InvalidInput);
        }
        let mut nearest = Hit {
            fraction: 1.0,
            normal: [0.0; 3],
            obstacle: None,
        };
        for (index, surface) in self.surfaces.iter().enumerate() {
            if !finite(surface.center) || !surface.radius.is_finite() || surface.radius <= 0.0 {
                return Err(Error::InvalidInput);
            }
            let origins = [
                i128::from(state.anchor.x) - i128::from(surface.anchor.x),
                i128::from(state.anchor.y) - i128::from(surface.anchor.y),
                i128::from(state.anchor.z) - i128::from(surface.anchor.z),
            ];
            let relative: Vector =
                std::array::from_fn(|k| origins[k] as f64 + state.center[k] - surface.center[k]);
            let radius = state.radius + surface.radius;
            let distance = norm(relative);
            if !distance.is_finite() || !radius.is_finite() {
                return Err(Error::InvalidInput);
            }
            if distance < radius * (1.0 - 1e-10) {
                return Err(Error::InitialOverlap);
            }
            let speed_squared = dot(displacement, displacement);
            let approach = dot(relative, displacement);
            let gap = dot(relative, relative) - radius * radius;
            let discriminant = approach * approach - speed_squared * gap;
            if !speed_squared.is_finite()
                || !approach.is_finite()
                || !gap.is_finite()
                || !discriminant.is_finite()
            {
                return Err(Error::InvalidInput);
            }
            // Ignore roundoff-sized closing motion after projecting a tangent
            // contact; otherwise touching pairs can consume the slide budget.
            if speed_squared == 0.0
                || approach >= -1e-12 * distance * speed_squared.sqrt()
                || discriminant < 0.0
            {
                continue;
            }
            let fraction = if gap <= 0.0 {
                0.0
            } else {
                gap / (-approach + discriminant.sqrt())
            };
            if fraction > nearest.fraction {
                continue;
            }
            let contact: Vector = std::array::from_fn(|k| relative[k] + displacement[k] * fraction);
            let length = norm(contact);
            if length == 0.0 || !length.is_finite() {
                return Err(Error::InvalidContact);
            }
            nearest = Hit {
                fraction,
                normal: contact.map(|v| v / length),
                obstacle: Some(index),
            };
        }
        Ok(nearest)
    }
}
