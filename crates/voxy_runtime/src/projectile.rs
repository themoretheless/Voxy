use std::fmt;

use voxy_world::{
    BlockRegistry, DestructionError, DestructionPlan, EditSource, Explosion, RayOrigin,
    RaycastConfig, RaycastError, RaycastResult, UnavailableReason, VoxelHit, VoxelView,
    plan_explosion, raycast,
};

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
    validate(state, dt, config)?;
    let next_age = state.age + dt;
    if next_age > config.max_lifetime {
        state.age = next_age;
        return Ok(ProjectileOutcome::Expired);
    }
    let next_velocity =
        std::array::from_fn(|axis| state.velocity[axis] + config.gravity[axis] * dt);
    let displacement = next_velocity.map(|velocity| velocity * dt);
    let distance = squared_length(displacement).sqrt();
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
        #[allow(clippy::cast_possible_truncation)]
        let shift = value.floor() as i64;
        voxel[axis] = voxel[axis]
            .checked_add(shift)
            .ok_or(ProjectileError::CoordinateOverflow)?;
        #[allow(clippy::cast_precision_loss)]
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
    }
}
