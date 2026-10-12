//! Renderer-independent voxel physics.
mod character;
mod chunk_cursor;
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
    CharacterStep, VoxelCollisionWorld, step_character, step_character_in_field,
    step_character_with_motion,
};
pub use collision::{
    AnchoredAabb, SweepConfig, SweepError, SweepObstacle, SweepResult, sweep_aabb,
    sweep_candidate_bounds,
};
pub use destruction::{DestructionError, DestructionPlan, Explosion, plan_explosion};
pub use projectile::{
    ProjectileConfig, ProjectileError, ProjectileOutcome, ProjectileState, plan_impact_explosion,
    spawn_projectile, step_projectile, step_projectile_in_field, step_projectile_with_motion,
};
pub use raycast::{RayOrigin, RaycastConfig, RaycastError, RaycastResult, VoxelHit, raycast};
pub use vehicle::{
    RaceCheckpoint, RaceError, RaceProgress, RaceTrack, VehicleConfig, VehicleError, VehicleInput,
    VehicleState, VehicleStep, step_vehicle, step_vehicle_with_chassis_step, update_race,
};
pub use water::{WaterBudget, WaterError, WaterPlan, WaterStates, step_water};

pub use water::{LiquidBudget, LiquidError, LiquidFlow, LiquidPlan, LiquidStates, step_liquid};

mod liquid_layers;
pub use liquid_layers::{LiquidMaterial, step_liquid_layers};

#[cfg(test)]
mod liquid_collision_tests;

mod wear;
pub use character::step_character_with_wear;
pub use wear::{VoxelWear, VoxelWearStep, WearSurface, WornVoxelCollisionWorld, sweep_worn_voxels};

pub use wear::{
    VoxelWearBatch, VoxelWearSuspension, VoxelWearSuspensionStep, advance_voxel_wear_batch,
};

mod moisture;
pub use moisture::{VoxelMoistureContact, VoxelMoistureStep, advance_voxel_moisture};

pub use moisture::{LiquidSurplusReturn, return_liquid_surplus};

pub use moisture::VoxelMoistureBank;
