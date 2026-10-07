//! Shared native platform shell for general 2D/3D scenes.
mod camera_motion;
mod full_model_worker;
mod gravity_demo;
mod scene_app;
mod touch;
pub use scene_app::SceneApp;

mod strands_demo;

mod tissue_demo;

mod liquid_demo;
mod wear_demo;

mod xray_demo;

mod biomechanics_demo;

mod female_demo;

mod female_rig;

mod female_hair;

mod rig_skinning;

mod female_face;
mod female_lids;

mod female_complexion;

mod volume_regions;

mod female_features;

mod female_eyes;
mod female_transmission;

mod face_preview;
pub use face_preview::FacePreview;

pub mod body_parameters;

pub mod face_parameters;

mod scene_input;
pub use scene_input::{SceneInput, SceneInputActions};

mod surface_film_preview;

pub mod model_presentation;

pub mod cuda_motion;

pub mod film_settings;

pub mod tissue_diagnostics;

pub mod view_settings;

mod diagnostic_legend;

mod scene_rush;

mod rush_physics;

mod rush_diagnostics;

pub mod fem_surface;

mod fem_demo;
