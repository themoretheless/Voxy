//! Shared native platform shell for general 2D/3D scenes: the winit
//! `ApplicationHandler`, scene input, touch and the rush scene. Used by the
//! desktop binary (`voxy_app`), the mobile entry point and the examples.
mod camera_motion;
mod scene_app;
mod touch;
pub use camera_motion::inertial_orbit_velocity_decay;
pub use scene_app::SceneApp;
pub use touch::{classify_two_finger_pinch_rotate, pinch_zoom_ratio};

mod scene_input;
pub use scene_input::{SceneInput, SceneInputActions};

mod rush_diagnostics;
mod rush_physics;
mod scene_rush;
