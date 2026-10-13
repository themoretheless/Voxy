//! Character, tissue, liquid and material demos shared by the native shell, the
//! examples and the ray probe. Pure library: no winit, no composition.

/// Orbit-camera limits shared with the shell's smoothed camera.
pub mod orbit {
    /// Pitch is clamped symmetrically so the orbit never flips over the poles.
    pub const ORBIT_PITCH_LIMIT: f32 = 1.2;
    /// Farthest orbit distance any viewer allows.
    pub const ORBIT_MAX_DISTANCE: f32 = 5.0;
    /// `(minimum distance, zoom step scale)` for the hand close-up versus the whole body.
    #[must_use]
    pub const fn zoom_range(hand: bool) -> (f32, f32) {
        if hand { (0.18, 0.2) } else { (0.7, 1.0) }
    }
}

mod error;
pub use error::DemoError;
pub mod biomechanics_demo;
pub mod body_parameters;
pub mod cuda_motion;
pub mod diagnostic_legend;
pub mod face_parameters;
pub mod face_preview;
pub mod fem_demo;
pub mod fem_surface;
pub mod female_complexion;
pub mod female_demo;
pub mod female_eyes;
pub mod female_face;
pub mod female_features;
pub mod female_hair;
pub mod female_lids;
pub mod female_rig;
pub mod female_transmission;
pub mod film_settings;
pub mod full_model_worker;
pub mod gpu_hair_solver;
pub mod gravity_demo;
pub mod jump_motion;
pub mod liquid_demo;
pub mod model_presentation;
pub mod rig_skinning;
pub mod strands_demo;
pub mod surface_film_preview;
pub mod tissue_demo;
pub mod tissue_diagnostics;
pub mod view_settings;
pub mod volume_regions;
pub mod wear_demo;
pub mod wet_fem_preview;
pub mod xray_demo;
pub use face_preview::FacePreview;

/// WGSL for the hair material drawn by the shell and the hair examples.
pub const FEMALE_HAIR_MATERIAL_SHADER: &str = include_str!("female_hair_material.wgsl");
/// WGSL for the tissue surface material used by the tissue snapshots.
pub const TISSUE_SURFACE_MATERIAL_SHADER: &str = include_str!("tissue_surface_material.wgsl");
