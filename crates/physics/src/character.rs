use std::fmt;

use crate::{AnchoredAabb, CollisionWorld};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterState {
    pub body: AnchoredAabb,
    pub velocity: [f64; 3],
    pub grounded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterInput {
    /// Desired X/Z velocity for this simulation tick.
    pub planar_velocity: [f64; 2],
    pub jump_pressed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterConfig {
    pub gravity: f64,
    pub jump_speed: f64,
    pub terminal_fall_speed: f64,
    pub step_height: f64,
    pub ground_snap_distance: f64,
    pub max_slide_iterations: u8,
    pub max_candidates_per_sweep: usize,
}

impl Default for CharacterConfig {
    fn default() -> Self {
        Self {
            gravity: -24.0,
            jump_speed: 8.5,
            terminal_fall_speed: 48.0,
            step_height: 1.01,
            ground_snap_distance: 0.1,
            max_slide_iterations: 4,
            max_candidates_per_sweep: 16_384,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterContact<O> {
    pub normal: [i8; 3],
    pub obstacle: O,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterStep<O> {
    pub requested_displacement: [f64; 3],
    pub applied_displacement: [f64; 3],
    pub contacts: Vec<CharacterContact<O>>,
    pub grounded: bool,
    pub stepped_up: bool,
}

/// Advances a character using a bounded iterative sweep-and-slide solver.
///
/// Horizontal input is a desired velocity rather than an acceleration, making input sampling
/// independent from frame rate. The caller supplies the fixed simulation `dt`; render
/// interpolation remains outside this authoritative step.
///
/// # Errors
///
/// Rejects invalid configuration/input/timestep, coordinate rebasing overflow, or backend sweep
/// failures. State is only published after the full step succeeds.
pub fn step_character<W: CollisionWorld>(
    world: &W,
    state: &mut CharacterState,
    input: CharacterInput,
    dt: f64,
    config: CharacterConfig,
) -> Result<CharacterStep<W::Obstacle>, CharacterError<W::Error>> {
    validate(state, input, dt, config)?;
    let mut next = *state;
    let step_start = next.body;
    let may_step = next.grounded
        && squared_length([input.planar_velocity[0], 0.0, input.planar_velocity[1]]) > f64::EPSILON;
    next.velocity[0] = input.planar_velocity[0];
    next.velocity[2] = input.planar_velocity[1];
    if input.jump_pressed && next.grounded {
        next.velocity[1] = config.jump_speed;
        next.grounded = false;
    }
    next.velocity[1] = (next.velocity[1] + config.gravity * dt).max(-config.terminal_fall_speed);
    let requested = next.velocity.map(|velocity| velocity * dt);
    let mut remaining = requested;
    let mut applied = [0.0; 3];
    let mut contacts = Vec::new();
    let mut grounded = false;
    for _ in 0..config.max_slide_iterations {
        if squared_length(remaining) <= f64::EPSILON {
            break;
        }
        let hit = world
            .sweep_aabb(next.body, remaining, config.max_candidates_per_sweep)
            .map_err(CharacterError::Sweep)?;
        let movement = remaining.map(|value| value * hit.fraction);
        translate(&mut next.body, movement);
        add_assign(&mut applied, movement);
        let Some(obstacle) = hit.obstacle else {
            break;
        };
        contacts.push(CharacterContact {
            normal: hit.normal,
            obstacle,
        });
        if hit.normal[1] > 0 {
            grounded = true;
        }
        clip_against_plane(&mut next.velocity, hit.normal);
        remaining = remaining.map(|value| value * (1.0 - hit.fraction));
        clip_against_plane(&mut remaining, hit.normal);
    }
    let regular_horizontal = applied[0] * applied[0] + applied[2] * applied[2];
    let stepped = if may_step && contacts.iter().any(|contact| contact.normal[1] == 0) {
        try_step_up(world, step_start, requested, config)?
    } else {
        None
    };
    let mut stepped_up = false;
    if let Some(candidate) = stepped {
        let candidate_horizontal = candidate.applied[0] * candidate.applied[0]
            + candidate.applied[2] * candidate.applied[2];
        if candidate_horizontal > regular_horizontal {
            next.body = candidate.body;
            applied = candidate.applied;
            contacts = candidate.contacts;
            grounded = true;
            next.velocity[1] = 0.0;
            stepped_up = true;
        }
    }
    next.grounded = grounded;
    rebase(&mut next.body)?;
    *state = next;
    Ok(CharacterStep {
        requested_displacement: requested,
        applied_displacement: applied,
        contacts,
        grounded,
        stepped_up,
    })
}

#[derive(Debug)]
struct StepCandidate<O> {
    body: AnchoredAabb,
    applied: [f64; 3],
    contacts: Vec<CharacterContact<O>>,
}

fn try_step_up<W: CollisionWorld>(
    world: &W,
    start: AnchoredAabb,
    requested: [f64; 3],
    config: CharacterConfig,
) -> Result<Option<StepCandidate<W::Obstacle>>, CharacterError<W::Error>> {
    if config.step_height == 0.0 {
        return Ok(None);
    }
    let budget = config.max_candidates_per_sweep;
    let lift = [0.0, config.step_height, 0.0];
    let lift_result = world
        .sweep_aabb(start, lift, budget)
        .map_err(CharacterError::Sweep)?;
    if lift_result.obstacle.is_some() {
        return Ok(None);
    }
    let mut body = start;
    translate(&mut body, lift);
    let horizontal = [requested[0], 0.0, requested[2]];
    let horizontal_result = world
        .sweep_aabb(body, horizontal, budget)
        .map_err(CharacterError::Sweep)?;
    if horizontal_result.fraction < 1.0 {
        return Ok(None);
    }
    translate(&mut body, horizontal);
    let drop = [
        0.0,
        -(config.step_height + config.ground_snap_distance),
        0.0,
    ];
    let drop_result = world
        .sweep_aabb(body, drop, budget)
        .map_err(CharacterError::Sweep)?;
    let Some(obstacle) = drop_result.obstacle else {
        return Ok(None);
    };
    if drop_result.normal != [0, 1, 0] {
        return Ok(None);
    }
    let vertical = lift[1] + drop[1] * drop_result.fraction;
    translate(&mut body, [0.0, drop[1] * drop_result.fraction, 0.0]);
    Ok(Some(StepCandidate {
        body,
        applied: [horizontal[0], vertical, horizontal[2]],
        contacts: vec![CharacterContact {
            normal: drop_result.normal,
            obstacle,
        }],
    }))
}

fn translate(body: &mut AnchoredAabb, displacement: [f64; 3]) {
    for (axis, value) in displacement.into_iter().enumerate() {
        body.min[axis] += value;
        body.max[axis] += value;
    }
}

fn add_assign(target: &mut [f64; 3], value: [f64; 3]) {
    for (target, value) in target.iter_mut().zip(value) {
        *target += value;
    }
}

fn clip_against_plane(vector: &mut [f64; 3], normal: [i8; 3]) {
    let dot = vector
        .iter()
        .zip(normal)
        .map(|(value, normal)| value * f64::from(normal))
        .sum::<f64>();
    if dot < 0.0 {
        for (value, normal) in vector.iter_mut().zip(normal) {
            *value -= f64::from(normal) * dot;
        }
    }
}

fn rebase<E>(body: &mut AnchoredAabb) -> Result<(), CharacterError<E>> {
    for axis in 0..3 {
        #[allow(clippy::cast_possible_truncation)]
        let shift = body.min[axis].floor() as i64;
        let anchor = match axis {
            0 => &mut body.anchor.x,
            1 => &mut body.anchor.y,
            _ => &mut body.anchor.z,
        };
        *anchor = anchor
            .checked_add(shift)
            .ok_or(CharacterError::CoordinateOverflow)?;
        #[allow(clippy::cast_precision_loss)]
        let shift = shift as f64;
        body.min[axis] -= shift;
        body.max[axis] -= shift;
    }
    Ok(())
}

fn squared_length(vector: [f64; 3]) -> f64 {
    vector.iter().map(|value| value * value).sum()
}

fn validate<E>(
    state: &CharacterState,
    input: CharacterInput,
    dt: f64,
    config: CharacterConfig,
) -> Result<(), CharacterError<E>> {
    let finite_state = state.velocity.iter().all(|value| value.is_finite())
        && state.body.min.iter().all(|value| value.is_finite())
        && state.body.max.iter().all(|value| value.is_finite());
    let finite_input = input.planar_velocity.iter().all(|value| value.is_finite());
    if !finite_state || !finite_input {
        return Err(CharacterError::NonFiniteState);
    }
    if (0..3).any(|axis| {
        state.body.min[axis] >= state.body.max[axis]
            || state.body.min[axis].abs() > 1_048_576.0
            || state.body.max[axis].abs() > 1_048_576.0
    }) {
        return Err(CharacterError::InvalidBounds);
    }
    if !dt.is_finite() || !(0.0..=0.25).contains(&dt) || dt == 0.0 {
        return Err(CharacterError::InvalidTimeStep);
    }
    if !config.gravity.is_finite()
        || config.gravity > 0.0
        || !config.jump_speed.is_finite()
        || config.jump_speed < 0.0
        || !config.terminal_fall_speed.is_finite()
        || config.terminal_fall_speed <= 0.0
        || !config.step_height.is_finite()
        || !(0.0..=2.0).contains(&config.step_height)
        || !config.ground_snap_distance.is_finite()
        || !(0.0..=1.0).contains(&config.ground_snap_distance)
        || !(1..=16).contains(&config.max_slide_iterations)
        || config.max_candidates_per_sweep == 0
    {
        return Err(CharacterError::InvalidConfig);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterError<E> {
    NonFiniteState,
    InvalidBounds,
    InvalidTimeStep,
    InvalidConfig,
    CoordinateOverflow,
    Sweep(E),
}

impl<E: fmt::Debug> fmt::Display for CharacterError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "character controller error: {self:?}")
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CharacterError<E> {}
