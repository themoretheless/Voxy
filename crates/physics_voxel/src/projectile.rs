use std::fmt;

use crate::{
    DestructionError, DestructionPlan, Explosion, RayOrigin, RaycastConfig, RaycastError,
    RaycastResult, VoxelHit, plan_explosion, raycast,
};
use voxy_world::{BlockRegistry, EditSource, UnavailableReason, VoxelView};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileConfig {
    pub gravity: [f64; 3],
    pub max_lifetime: f64,
    pub max_raycast_steps: u32,
}

impl Default for ProjectileConfig {
    fn default() -> Self {
        Self {
            gravity: [0.0, -24.0, 0.0],
            max_lifetime: 10.0,
            max_raycast_steps: 256,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileState {
    pub position: RayOrigin,
    pub velocity: [f64; 3],
    pub age: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProjectileOutcome {
    Flying,
    Impact(VoxelHit),
    Expired,
    Unloaded {
        at: voxy_world::VoxelPos,
        chunk: voxy_world::ChunkPos,
    },
    Unavailable {
        at: voxy_world::VoxelPos,
        chunk: voxy_world::ChunkPos,
        cause: UnavailableReason,
    },
    StepBudgetExhausted,
}

/// Creates a projectile from a normalized far-world origin and finite non-zero direction.
///
/// # Errors
///
/// Rejects invalid origins, directions or muzzle speeds.
pub fn spawn_projectile(
    position: RayOrigin,
    direction: [f64; 3],
    muzzle_speed: f64,
) -> Result<ProjectileState, ProjectileError> {
    validate_origin(position)?;
    if direction.iter().any(|value| !value.is_finite())
        || !muzzle_speed.is_finite()
        || muzzle_speed <= 0.0
    {
        return Err(ProjectileError::InvalidLaunch);
    }
    let length = squared_length(direction).sqrt();
    if length <= f64::EPSILON {
        return Err(ProjectileError::InvalidLaunch);
    }
    Ok(ProjectileState {
        position,
        velocity: direction.map(|component| component / length * muzzle_speed),
        age: 0.0,
    })
}

/// Advances one ballistic projectile with a bounded DDA sweep over its complete displacement.
///
/// # Errors
///
/// Rejects invalid state/config/timestep and coordinate or raycast failures. State advances only
/// after a miss, so an impact position remains reproducible for gameplay processing.
pub fn step_projectile(
    view: &impl VoxelView,
    state: &mut ProjectileState,
    dt: f64,
    config: ProjectileConfig,
) -> Result<ProjectileOutcome, ProjectileError> {
    let velocity = std::array::from_fn(|axis| state.velocity[axis] + config.gravity[axis] * dt);
    let displacement = velocity.map(|value| value * dt);
    step_projectile_with_motion(view, state, dt, config, velocity, displacement)
}

/// Sweeps externally integrated motion through the authoritative voxel view.
/// The caller supplies semi-implicit Euler motion in f64; anchored coordinates
/// remain on the CPU. Hits and unavailable/budget outcomes preserve state.
/// # Errors
/// Rejects invalid state or nonfinite motion before querying the world.
pub fn step_projectile_with_motion(
    view: &impl VoxelView,
    state: &mut ProjectileState,
    dt: f64,
    config: ProjectileConfig,
    next_velocity: [f64; 3],
    displacement: [f64; 3],
) -> Result<ProjectileOutcome, ProjectileError> {
    validate(state, dt, config)?;
    let next_age = state.age + dt;
    if next_age > config.max_lifetime {
        state.age = next_age;
        return Ok(ProjectileOutcome::Expired);
    }
    let distance = squared_length(displacement).sqrt();
    if next_velocity
        .iter()
        .chain(&displacement)
        .any(|value| !value.is_finite())
        || !distance.is_finite()
    {
        return Err(ProjectileError::InvalidState);
    }
    if distance <= f64::EPSILON {
        state.velocity = next_velocity;
        state.age = next_age;
        return Ok(ProjectileOutcome::Flying);
    }
    let result = raycast(
        view,
        state.position,
        displacement,
        RaycastConfig {
            max_distance: distance,
            max_steps: config.max_raycast_steps,
        },
    )?;
    match result {
        RaycastResult::Miss => {
            state.position = translated(state.position, displacement)?;
            state.velocity = next_velocity;
            state.age = next_age;
            Ok(ProjectileOutcome::Flying)
        }
        RaycastResult::Hit(hit) => Ok(ProjectileOutcome::Impact(hit)),
        RaycastResult::Unloaded { at, chunk } => Ok(ProjectileOutcome::Unloaded { at, chunk }),
        RaycastResult::Unavailable { at, chunk, cause } => {
            Ok(ProjectileOutcome::Unavailable { at, chunk, cause })
        }
        RaycastResult::StepBudgetExhausted => Ok(ProjectileOutcome::StepBudgetExhausted),
    }
}

/// Samples gravity at the projectile's anchored position, then performs its bounded sweep.
/// `config.gravity` is replaced by the sampled field acceleration.
/// # Errors
/// Field errors and the original projectile validation/raycast errors do not advance state.
pub fn step_projectile_in_field(
    view: &impl VoxelView,
    state: &mut ProjectileState,
    dt: f64,
    mut config: ProjectileConfig,
    field: &impl physics::gravity_field::GravityField,
) -> Result<ProjectileOutcome, ProjectileError> {
    config.gravity = field
        .acceleration(
            physics::Origin {
                x: state.position.voxel.x,
                y: state.position.voxel.y,
                z: state.position.voxel.z,
            },
            state.position.offset,
        )
        .map_err(ProjectileError::Gravity)?;
    step_projectile(view, state, dt, config)
}

/// Converts a projectile impact into the same revision-checked atomic transaction used by other
/// destruction sources.
///
/// # Errors
///
/// Returns the bounded explosion planner's validation or availability error.
pub fn plan_impact_explosion(
    view: &impl VoxelView,
    registry: &BlockRegistry,
    source: EditSource,
    hit: VoxelHit,
    mut explosion: Explosion,
) -> Result<DestructionPlan, DestructionError> {
    explosion.center = hit.pos;
    plan_explosion(view, registry, source, explosion)
}

fn translated(origin: RayOrigin, displacement: [f64; 3]) -> Result<RayOrigin, ProjectileError> {
    let mut voxel = [origin.voxel.x, origin.voxel.y, origin.voxel.z];
    let mut offset = origin.offset;
    for axis in 0..3 {
        let value = offset[axis] + displacement[axis];
        let shift = value.floor() as i64;
        voxel[axis] = voxel[axis]
            .checked_add(shift)
            .ok_or(ProjectileError::CoordinateOverflow)?;
        let shift = shift as f64;
        offset[axis] = value - shift;
    }
    Ok(RayOrigin {
        voxel: voxy_world::VoxelPos {
            x: voxel[0],
            y: voxel[1],
            z: voxel[2],
        },
        offset,
    })
}

fn validate(
    state: &ProjectileState,
    dt: f64,
    config: ProjectileConfig,
) -> Result<(), ProjectileError> {
    validate_origin(state.position)?;
    if state.velocity.iter().any(|value| !value.is_finite())
        || !state.age.is_finite()
        || state.age < 0.0
        || !dt.is_finite()
        || !(0.0..=1.0).contains(&dt)
        || config.gravity.iter().any(|value| !value.is_finite())
        || !config.max_lifetime.is_finite()
        || config.max_lifetime <= 0.0
        || config.max_raycast_steps == 0
    {
        return Err(ProjectileError::InvalidState);
    }
    Ok(())
}

fn validate_origin(origin: RayOrigin) -> Result<(), ProjectileError> {
    if origin
        .offset
        .iter()
        .any(|value| !value.is_finite() || !(0.0..1.0).contains(value))
    {
        Err(ProjectileError::InvalidOrigin)
    } else {
        Ok(())
    }
}

fn squared_length(vector: [f64; 3]) -> f64 {
    vector.iter().map(|value| value * value).sum()
}

#[derive(Debug)]
pub enum ProjectileError {
    Gravity(physics::gravity::Error),
    InvalidOrigin,
    InvalidLaunch,
    InvalidState,
    CoordinateOverflow,
    Raycast(RaycastError),
}

impl From<RaycastError> for ProjectileError {
    fn from(error: RaycastError) -> Self {
        Self::Raycast(error)
    }
}

impl fmt::Display for ProjectileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "projectile error: {self:?}")
    }
}

impl std::error::Error for ProjectileError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use voxy_world::{
        BlockDef, BlockStateId, ChunkPos, ChunkSnapshot, CollisionShape, MaterialId, Occlusion,
        RenderKind, ResourceKey, Sample, VoxelPos,
    };

    use super::*;

    #[derive(Default)]
    struct TestView(BTreeMap<VoxelPos, BlockStateId>);

    impl VoxelView for TestView {
        fn sample(&self, pos: VoxelPos) -> Sample<BlockStateId> {
            Sample::Loaded(self.0.get(&pos).copied().unwrap_or(BlockStateId::AIR))
        }

        fn chunk(&self, _pos: ChunkPos) -> Option<ChunkSnapshot> {
            None
        }
    }

    fn origin() -> RayOrigin {
        RayOrigin {
            voxel: VoxelPos { x: 0, y: 0, z: 0 },
            offset: [0.5; 3],
        }
    }

    #[test]
    fn sampled_uniform_field_matches_legacy_and_radial_source_curves_flight() {
        use physics::gravity_field::{NewtonianField, Source};
        let view = TestView::default();
        let mut legacy = spawn_projectile(origin(), [1.0, 0.0, 0.0], 1.0).unwrap();
        let mut sampled = legacy;
        let config = ProjectileConfig::default();
        for _ in 0..10 {
            assert_eq!(
                step_projectile(&view, &mut legacy, 0.01, config).unwrap(),
                step_projectile_in_field(&view, &mut sampled, 0.01, config, &config.gravity)
                    .unwrap()
            );
            assert_eq!(legacy, sampled);
        }
        let sources = [Source {
            anchor: physics::Origin { x: 0, y: 3, z: 0 },
            position: [0.5; 3],
            mass: 9.0,
            radius: 1.0,
        }];
        let field = NewtonianField {
            gravity: physics::gravity::Gravity {
                constant: 1.0,
                ..Default::default()
            },
            sources: &sources,
        };
        let mut radial = spawn_projectile(origin(), [1.0, 0.0, 0.0], 1.0).unwrap();
        step_projectile_in_field(&view, &mut radial, 0.1, config, &field).unwrap();
        assert!(radial.velocity[1] > 0.0);
        let before = radial;
        assert!(
            step_projectile_in_field(&view, &mut radial, 0.1, config, &[f64::NAN, 0.0, 0.0])
                .is_err()
        );
        assert_eq!(radial, before);
    }

    fn solid() -> BlockStateId {
        let definition = |name: &str, render, occlusion, collision| BlockDef {
            key: ResourceKey::parse(format!("voxy:{name}")).unwrap(),
            render,
            occlusion,
            collision,
            face_materials: [MaterialId(0); 6],
            translucent_interface_group: None,
            emission: 0,
            blast_resistance: u16::from(name != "air"),
        };
        BlockRegistry::new(vec![
            definition(
                "air",
                RenderKind::Invisible,
                Occlusion::None,
                CollisionShape::Empty,
            ),
            definition(
                "solid",
                RenderKind::Opaque,
                Occlusion::FullCube,
                CollisionShape::FullCube,
            ),
        ])
        .unwrap()
        .find(&ResourceKey::parse(Arc::<str>::from("voxy:solid")).unwrap())
        .unwrap()
    }

    #[test]
    fn ballistic_step_applies_gravity_and_preserves_fractional_position() {
        let mut projectile = spawn_projectile(origin(), [1.0, 0.0, 0.0], 10.0).unwrap();
        assert_eq!(
            step_projectile(
                &TestView::default(),
                &mut projectile,
                0.1,
                ProjectileConfig::default()
            )
            .unwrap(),
            ProjectileOutcome::Flying
        );
        assert!((projectile.position.offset[0] - 0.5).abs() < 1.0e-12);
        assert_eq!(projectile.position.voxel.x, 1);
        assert!((projectile.velocity[1] + 2.4).abs() < 1.0e-12);
    }

    #[test]
    fn external_motion_preserves_far_world_anchor_and_sweep_atomicity() {
        let anchor = VoxelPos {
            x: 9_007_199_254_740_993,
            y: 0,
            z: 0,
        };
        let mut state = spawn_projectile(
            RayOrigin {
                voxel: anchor,
                offset: [0.5; 3],
            },
            [1.0, 0.0, 0.0],
            40.0,
        )
        .unwrap();
        let before = state;
        let config = ProjectileConfig {
            gravity: [0.0; 3],
            ..ProjectileConfig::default()
        };
        let mut view = TestView::default();
        view.0.insert(
            VoxelPos {
                x: anchor.x + 3,
                ..anchor
            },
            solid(),
        );
        let outcome = step_projectile_with_motion(
            &view,
            &mut state,
            0.1,
            config,
            [40.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
        )
        .unwrap();
        assert!(matches!(outcome, ProjectileOutcome::Impact(hit) if hit.pos.x == anchor.x + 3));
        assert_eq!(state, before);
        assert!(
            step_projectile_with_motion(
                &view,
                &mut state,
                0.1,
                config,
                [f64::INFINITY; 3],
                [0.0; 3]
            )
            .is_err()
        );
        assert_eq!(state, before);
        view.0.clear();
        assert_eq!(
            step_projectile_with_motion(
                &view,
                &mut state,
                0.1,
                config,
                [40.0, 0.0, 0.0],
                [4.0, 0.0, 0.0]
            )
            .unwrap(),
            ProjectileOutcome::Flying
        );
        assert_eq!(state.position.voxel.x, anchor.x + 4);
        assert_eq!(state.position.offset, [0.5; 3]);
    }

    #[test]
    fn continuous_sweep_hits_a_voxel_between_ticks_without_advancing_state() {
        let mut view = TestView::default();
        view.0.insert(VoxelPos { x: 3, y: 0, z: 0 }, solid());
        let mut projectile = spawn_projectile(origin(), [1.0, 0.0, 0.0], 40.0).unwrap();
        let before = projectile;
        let outcome = step_projectile(
            &view,
            &mut projectile,
            0.1,
            ProjectileConfig {
                gravity: [0.0; 3],
                ..ProjectileConfig::default()
            },
        )
        .unwrap();
        assert!(matches!(outcome, ProjectileOutcome::Impact(hit) if hit.pos.x == 3));
        assert_eq!(projectile, before);
        let (low, high) = solve_ballistic_elevation_angles(30.0, 9.81, 50.0, 0.0).unwrap();
        assert!(low > 0.0 && low < high && high < std::f64::consts::FRAC_PI_2);
        assert!(solve_ballistic_elevation_angles(10.0, 9.81, 500.0, 0.0).is_none());
    }
}

/// Solves for the `(low_arc_rad, high_arc_rad)` launch elevation angles required for a projectile
/// fired with muzzle speed `speed_m_s` under downward gravity `gravity_m_s2` to hit a target at
/// horizontal distance `horizontal_dist_m` and altitude difference `vertical_delta_m`.
/// Returns `None` if the target is out of ballistic range or parameters are invalid.
#[must_use]
pub fn solve_ballistic_elevation_angles(
    speed_m_s: f64,
    gravity_m_s2: f64,
    horizontal_dist_m: f64,
    vertical_delta_m: f64,
) -> Option<(f64, f64)> {
    if !speed_m_s.is_finite()
        || !gravity_m_s2.is_finite()
        || !horizontal_dist_m.is_finite()
        || !vertical_delta_m.is_finite()
        || speed_m_s <= 0.0
        || gravity_m_s2 <= 0.0
        || horizontal_dist_m <= 0.0
    {
        return None;
    }
    let v2 = speed_m_s * speed_m_s;
    let v4 = v2 * v2;
    let x = horizontal_dist_m;
    let y = vertical_delta_m;
    let g = gravity_m_s2;
    let disc = v4 - g * (g * x * x + 2.0 * y * v2);
    if disc < 0.0 {
        return None;
    }
    let root = disc.sqrt();
    let gx = g * x;
    let low = ((v2 - root) / gx).atan();
    let high = ((v2 + root) / gx).atan();
    Some((low.min(high), low.max(high)))
}

/// Evaluates the aerodynamic Magnus lift acceleration
/// $\mathbf{a}_M = C_L (\boldsymbol{\omega} \times \mathbf{v}_{\text{rel}})$ on a spinning projectile.
#[must_use]
pub fn magnus_lift_acceleration(
    velocity_rel_air: [f64; 3],
    spin_rad_s: [f64; 3],
    spin_lift_coeff: f64,
) -> [f64; 3] {
    if !spin_lift_coeff.is_finite()
        || velocity_rel_air.iter().any(|x| !x.is_finite())
        || spin_rad_s.iter().any(|x| !x.is_finite())
    {
        return [0.0; 3];
    }
    let cross = [
        spin_rad_s[1] * velocity_rel_air[2] - spin_rad_s[2] * velocity_rel_air[1],
        spin_rad_s[2] * velocity_rel_air[0] - spin_rad_s[0] * velocity_rel_air[2],
        spin_rad_s[0] * velocity_rel_air[1] - spin_rad_s[1] * velocity_rel_air[0],
    ];
    cross.map(|c| c * spin_lift_coeff)
}

/// Computes the post-impact ricochet velocity vector when a projectile with `incident_vel`
/// strikes a surface with unit `normal` at a grazing angle whose normal cosine magnitude
/// $|\hat{\mathbf{v}} \cdot \mathbf{n}| \le \text{critical\_grazing\_cos}$.
#[must_use]
pub fn ricochet_reflection_velocity(
    incident_vel: [f64; 3],
    normal: [f64; 3],
    restitution_n: f64,
    tangential_retention: f64,
    critical_grazing_cos: f64,
) -> Option<[f64; 3]> {
    if incident_vel.iter().chain(&normal).any(|x| !x.is_finite())
        || !restitution_n.is_finite()
        || !tangential_retention.is_finite()
        || !critical_grazing_cos.is_finite()
        || !(0.0..=1.0).contains(&restitution_n)
        || !(0.0..=1.0).contains(&tangential_retention)
        || !(0.0..=1.0).contains(&critical_grazing_cos)
    {
        return None;
    }
    let speed =
        (incident_vel[0].powi(2) + incident_vel[1].powi(2) + incident_vel[2].powi(2)).sqrt();
    let n_len = (normal[0].powi(2) + normal[1].powi(2) + normal[2].powi(2)).sqrt();
    if speed <= 1e-9 || (n_len - 1.0).abs() > 1e-6 {
        return None;
    }
    let vn =
        incident_vel[0] * normal[0] + incident_vel[1] * normal[1] + incident_vel[2] * normal[2];
    if vn >= 0.0 || (-vn / speed) > critical_grazing_cos {
        return None;
    }
    let vt = [
        incident_vel[0] - vn * normal[0],
        incident_vel[1] - vn * normal[1],
        incident_vel[2] - vn * normal[2],
    ];
    Some([
        tangential_retention * vt[0] - restitution_n * vn * normal[0],
        tangential_retention * vt[1] - restitution_n * vn * normal[1],
        tangential_retention * vt[2] - restitution_n * vn * normal[2],
    ])
}

#[cfg(test)]
mod magnus_and_ricochet_tests {
    use super::*;

    #[test]
    fn magnus_backspin_lifts_upward_and_shallow_grazing_ricochets() {
        let lift = magnus_lift_acceleration([30.0, 0.0, 0.0], [0.0, 0.0, 100.0], 0.002);
        assert!(lift[1] > 0.0 && lift[0].abs() < 1e-12);
        let ric = ricochet_reflection_velocity([20.0, -2.0, 0.0], [0.0, 1.0, 0.0], 0.5, 0.9, 0.3)
            .unwrap();
        assert!(ric[1] > 0.0 && ric[0] > 15.0);
        assert!(
            ricochet_reflection_velocity([2.0, -20.0, 0.0], [0.0, 1.0, 0.0], 0.5, 0.9, 0.3)
                .is_none()
        );
    }
}
