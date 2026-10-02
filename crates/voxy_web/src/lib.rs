//! Browser platform shell using the shared general 2D/3D renderer.
#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(target_arch = "wasm32")]
mod gravity_validation;

#[cfg(target_arch = "wasm32")]
mod collision_validation;
#[cfg(target_arch = "wasm32")]
mod water_validation;

#[cfg(target_arch = "wasm32")]
mod voxel_scene;

#[cfg(target_arch = "wasm32")]
mod voxel_game;

#[cfg(target_arch = "wasm32")]
mod voxel_vehicle;

#[cfg(target_arch = "wasm32")]
mod voxel_terrain;

#[cfg(target_arch = "wasm32")]
mod voxel_water;

#[cfg(target_arch = "wasm32")]
mod animated_scene;

#[cfg(target_arch = "wasm32")]
mod temporal_validation;

#[cfg(target_arch = "wasm32")]
mod temporal_guides_probe;

#[cfg(target_arch = "wasm32")]
mod temporal_color_probe;
