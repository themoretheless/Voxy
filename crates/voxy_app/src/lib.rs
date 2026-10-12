//! Shared native platform shell for general 2D/3D scenes.
mod camera_motion;
pub mod full_model_worker;
mod gravity_demo;
mod scene_app;
mod touch;
pub use scene_app::SceneApp;

mod strands_demo;

pub mod tissue_demo;

pub mod liquid_demo;
pub mod wear_demo;

mod xray_demo;

pub mod biomechanics_demo;

pub mod female_demo;

pub mod female_rig;
pub mod jump_motion;

pub mod female_hair;
pub mod gpu_hair_solver;

pub mod rig_skinning;

pub mod female_face;
mod female_lids;

pub mod female_complexion;

pub mod volume_regions;

pub mod female_features;

pub mod female_eyes;
pub mod female_transmission;

mod face_preview;
pub use face_preview::FacePreview;

pub mod body_parameters;

pub mod face_parameters;

mod scene_input;
pub use scene_input::{SceneInput, SceneInputActions};

pub mod surface_film_preview;

pub mod model_presentation;

pub mod cuda_motion;

pub mod film_settings;

pub mod tissue_diagnostics;

pub mod view_settings;

pub mod diagnostic_legend;

mod scene_rush;

mod rush_physics;

mod rush_diagnostics;

pub mod fem_surface;
pub mod wet_fem_preview;

pub mod fem_demo;
