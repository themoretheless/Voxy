use std::fmt;

use voxy_world::{AnchoredAabb, BlockRegistry, VoxelView};

use crate::{
    CharacterConfig, CharacterError, CharacterInput, CharacterState, CharacterStep, step_character,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VehicleState {
    pub chassis: CharacterState,
    pub heading: f64,
    pub longitudinal_speed: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VehicleInput {
    pub throttle: f64,
    pub steering: f64,
    pub brake: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VehicleConfig {
    pub mass: f64,
    pub engine_force: f64,
    pub reverse_force: f64,
    pub brake_force: f64,
    pub aerodynamic_drag: f64,
    pub rolling_resistance: f64,
    pub lateral_grip: f64,
    pub wheelbase: f64,
    pub max_steering_angle: f64,
    pub max_forward_speed: f64,
    pub max_reverse_speed: f64,
    pub collision: CharacterConfig,
}

impl Default for VehicleConfig {
    fn default() -> Self {
        Self {
            mass: 1_250.0,
            engine_force: 9_000.0,
            reverse_force: 4_000.0,
            brake_force: 14_000.0,
            aerodynamic_drag: 0.42,
            rolling_resistance: 18.0,
            lateral_grip: 8.0,
            wheelbase: 2.6,
            max_steering_angle: 32_f64.to_radians(),
            max_forward_speed: 55.0,
            max_reverse_speed: 16.0,
            collision: CharacterConfig {
                step_height: 0.45,
                jump_speed: 0.0,
                ..CharacterConfig::default()
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VehicleStep {
    pub previous_center: [f64; 3],
    pub current_center: [f64; 3],
    pub collision: CharacterStep,
}

/// Advances an arcade-readable bicycle model through the authoritative voxel sweep solver.
///
/// Throttle/brake forces use mass, speed-dependent drag and rolling resistance. Steering follows
/// wheelbase curvature, lateral velocity is damped by tire grip, and the resulting chassis motion
/// is resolved by the same swept-AABB collision/gravity path as the character.
///
/// # Errors
///
/// Rejects non-finite/out-of-range input, invalid vehicle configuration/state/timestep, or
/// underlying voxel collision failures. State is published only after the complete step succeeds.
pub fn step_vehicle(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    state: &mut VehicleState,
    input: VehicleInput,
    dt: f64,
    config: VehicleConfig,
) -> Result<VehicleStep, VehicleError> {
    validate(state, input, dt, config)?;
    let mut next = *state;
    let previous_center = center(next.chassis.body);
    let forward = [next.heading.sin(), next.heading.cos()];
    let right = [forward[1], -forward[0]];
    let planar = [next.chassis.velocity[0], next.chassis.velocity[2]];
    let mut longitudinal = dot(planar, forward);
    let lateral = dot(planar, right);
    let drive_force = if input.throttle >= 0.0 {
        input.throttle * config.engine_force
    } else {
        input.throttle * config.reverse_force
    };
    let drag = config.aerodynamic_drag * longitudinal * longitudinal.abs();
    let rolling = if longitudinal.abs() > 1.0e-6 {
        config.rolling_resistance * longitudinal.signum()
    } else {
        0.0
    };
    let brake = if input.brake && longitudinal.abs() > 1.0e-6 {
        config.brake_force * longitudinal.signum()
    } else {
        0.0
    };
    longitudinal += (drive_force - drag - rolling - brake) / config.mass * dt;
    if input.brake && longitudinal.signum() != state.longitudinal_speed.signum() {
        longitudinal = 0.0;
    }
    longitudinal = longitudinal.clamp(-config.max_reverse_speed, config.max_forward_speed);
    let lateral = lateral * (-config.lateral_grip * dt).exp();
    let steering_angle = input.steering * config.max_steering_angle;
    next.heading += longitudinal / config.wheelbase * steering_angle.tan() * dt;
    next.heading = wrap_angle(next.heading);
    let forward = [next.heading.sin(), next.heading.cos()];
    let right = [forward[1], -forward[0]];
    let desired = [
        forward[0] * longitudinal + right[0] * lateral,
        forward[1] * longitudinal + right[1] * lateral,
    ];
    let collision = step_character(
        view,
        registry,
        &mut next.chassis,
        CharacterInput {
            planar_velocity: desired,
            jump_pressed: false,
        },
        dt,
        config.collision,
    )?;
    let requested_horizontal =
        collision.requested_displacement[0].hypot(collision.requested_displacement[2]);
    let applied_horizontal =
        collision.applied_displacement[0].hypot(collision.applied_displacement[2]);
    if requested_horizontal > 1.0e-9 && applied_horizontal < requested_horizontal * 0.25 {
        longitudinal = 0.0;
    }
    next.longitudinal_speed = longitudinal;
    let current_center = center(next.chassis.body);
    *state = next;
    Ok(VehicleStep {
        previous_center,
        current_center,
        collision,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaceCheckpoint {
    pub center: [f64; 3],
    pub radius: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RaceTrack {
    checkpoints: Box<[RaceCheckpoint]>,
    laps: u16,
}

impl RaceTrack {
    /// Creates an ordered checkpoint circuit.
    ///
    /// # Errors
    ///
    /// Rejects fewer than two checkpoints, zero laps, non-finite data or invalid radii.
    pub fn new(checkpoints: Vec<RaceCheckpoint>, laps: u16) -> Result<Self, RaceError> {
        if checkpoints.len() < 2
            || laps == 0
            || checkpoints.iter().any(|checkpoint| {
                checkpoint.center.iter().any(|value| !value.is_finite())
                    || !checkpoint.radius.is_finite()
                    || checkpoint.radius <= 0.0
            })
        {
            return Err(RaceError::InvalidTrack);
        }
        Ok(Self {
            checkpoints: checkpoints.into_boxed_slice(),
            laps,
        })
    }

    #[must_use]
    pub fn checkpoints(&self) -> &[RaceCheckpoint] {
        &self.checkpoints
    }

    #[must_use]
    pub const fn laps(&self) -> u16 {
        self.laps
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RaceProgress {
    pub next_checkpoint: usize,
    pub completed_laps: u16,
    pub elapsed: f64,
    pub finished: bool,
}

/// Advances race progress when the swept vehicle center intersects the next ordered gate.
///
/// # Errors
///
/// Rejects invalid progress, segment coordinates or timestep.
pub fn update_race(
    track: &RaceTrack,
    progress: &mut RaceProgress,
    previous: [f64; 3],
    current: [f64; 3],
    dt: f64,
) -> Result<bool, RaceError> {
    if progress.next_checkpoint >= track.checkpoints.len()
        || progress.completed_laps > track.laps
        || !progress.elapsed.is_finite()
        || progress.elapsed < 0.0
        || previous
            .iter()
            .chain(&current)
            .any(|value| !value.is_finite())
        || !dt.is_finite()
        || dt < 0.0
    {
        return Err(RaceError::InvalidProgress);
    }
    if progress.finished {
        return Ok(false);
    }
    progress.elapsed += dt;
    let checkpoint = track.checkpoints[progress.next_checkpoint];
    if segment_distance_squared(previous, current, checkpoint.center)
        > checkpoint.radius * checkpoint.radius
    {
        return Ok(false);
    }
    progress.next_checkpoint += 1;
    if progress.next_checkpoint == track.checkpoints.len() {
        progress.next_checkpoint = 0;
        progress.completed_laps += 1;
        progress.finished = progress.completed_laps == track.laps;
    }
    Ok(true)
}

// Race circuits are local gameplay zones. World streaming rebases their checkpoint frame before
// this conversion; keeping the integrator local avoids carrying global floats in canonical state.
#[allow(clippy::cast_precision_loss)]
fn center(body: AnchoredAabb) -> [f64; 3] {
    [
        body.anchor.x as f64 + (body.min[0] + body.max[0]) * 0.5,
        body.anchor.y as f64 + (body.min[1] + body.max[1]) * 0.5,
        body.anchor.z as f64 + (body.min[2] + body.max[2]) * 0.5,
    ]
}

fn dot(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[0] + left[1] * right[1]
}

fn wrap_angle(angle: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    (angle + std::f64::consts::PI).rem_euclid(tau) - std::f64::consts::PI
}

fn segment_distance_squared(start: [f64; 3], end: [f64; 3], point: [f64; 3]) -> f64 {
    let segment = std::array::from_fn::<_, 3, _>(|axis| end[axis] - start[axis]);
    let relative = std::array::from_fn::<_, 3, _>(|axis| point[axis] - start[axis]);
    let length_squared = segment.iter().map(|value| value * value).sum::<f64>();
    let fraction = if length_squared <= f64::EPSILON {
        0.0
    } else {
        relative
            .iter()
            .zip(segment)
            .map(|(left, right)| left * right)
            .sum::<f64>()
            / length_squared
    }
    .clamp(0.0, 1.0);
    (0..3)
        .map(|axis| start[axis] + segment[axis] * fraction - point[axis])
        .map(|value| value * value)
        .sum()
}

fn validate(
    state: &VehicleState,
    input: VehicleInput,
    dt: f64,
    config: VehicleConfig,
) -> Result<(), VehicleError> {
    if !state.heading.is_finite()
        || !state.longitudinal_speed.is_finite()
        || !input.throttle.is_finite()
        || !input.steering.is_finite()
        || !(-1.0..=1.0).contains(&input.throttle)
        || !(-1.0..=1.0).contains(&input.steering)
        || !dt.is_finite()
        || !(0.0..=1.0).contains(&dt)
        || [
            config.mass,
            config.engine_force,
            config.reverse_force,
            config.brake_force,
            config.aerodynamic_drag,
            config.rolling_resistance,
            config.lateral_grip,
            config.wheelbase,
            config.max_steering_angle,
            config.max_forward_speed,
            config.max_reverse_speed,
        ]
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(VehicleError::InvalidState);
    }
    Ok(())
}

#[derive(Debug)]
pub enum VehicleError {
    InvalidState,
    Collision(CharacterError),
}

impl From<CharacterError> for VehicleError {
    fn from(error: CharacterError) -> Self {
        Self::Collision(error)
    }
}

impl fmt::Display for VehicleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "vehicle physics error: {self:?}")
    }
}

impl std::error::Error for VehicleError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RaceError {
    InvalidTrack,
    InvalidProgress,
}

impl fmt::Display for RaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "race error: {self:?}")
    }
}

impl std::error::Error for RaceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_swept_checkpoints_complete_laps_without_tunneling() {
        let track = RaceTrack::new(
            vec![
                RaceCheckpoint {
                    center: [5.0, 0.0, 0.0],
                    radius: 0.5,
                },
                RaceCheckpoint {
                    center: [10.0, 0.0, 0.0],
                    radius: 0.5,
                },
            ],
            1,
        )
        .unwrap();
        let mut progress = RaceProgress::default();
        assert!(update_race(&track, &mut progress, [0.0; 3], [8.0, 0.0, 0.0], 0.1).unwrap());
        assert!(!progress.finished);
        assert!(
            update_race(
                &track,
                &mut progress,
                [8.0, 0.0, 0.0],
                [12.0, 0.0, 0.0],
                0.1
            )
            .unwrap()
        );
        assert!(progress.finished);
        assert_eq!(progress.completed_laps, 1);
    }

    #[test]
    fn checkpoints_cannot_be_completed_out_of_order() {
        let track = RaceTrack::new(
            vec![
                RaceCheckpoint {
                    center: [0.0; 3],
                    radius: 1.0,
                },
                RaceCheckpoint {
                    center: [10.0, 0.0, 0.0],
                    radius: 1.0,
                },
            ],
            2,
        )
        .unwrap();
        let mut progress = RaceProgress::default();
        assert!(
            !update_race(
                &track,
                &mut progress,
                [9.0, 0.0, 0.0],
                [11.0, 0.0, 0.0],
                0.1
            )
            .unwrap()
        );
        assert_eq!(progress.next_checkpoint, 0);
    }
}
