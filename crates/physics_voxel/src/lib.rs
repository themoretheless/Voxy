//! Renderer-independent voxel physics.
mod character;
mod collision;
mod destruction;
mod projectile;
mod raycast;
#[cfg(test)]
mod test_support;
mod vehicle;
mod water;
pub use character::{
    CharacterConfig, CharacterContact, CharacterError, CharacterInput, CharacterState,
    CharacterStep, VoxelCollisionWorld, step_character,
};
pub use collision::{
    AnchoredAabb, SweepConfig, SweepError, SweepObstacle, SweepResult, sweep_aabb,
};
pub use destruction::{DestructionError, DestructionPlan, Explosion, plan_explosion};
pub use projectile::{
    ProjectileConfig, ProjectileError, ProjectileOutcome, ProjectileState, plan_impact_explosion,
    spawn_projectile, step_projectile,
};
pub use raycast::{RayOrigin, RaycastConfig, RaycastError, RaycastResult, VoxelHit, raycast};
pub use vehicle::{
    RaceCheckpoint, RaceError, RaceProgress, RaceTrack, VehicleConfig, VehicleError, VehicleInput,
    VehicleState, VehicleStep, step_vehicle, update_race,
};
pub use water::{WaterBudget, WaterError, WaterPlan, WaterStates, step_water};
