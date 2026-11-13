//! Character, tissue, liquid and material demos shared by the examples and the
//! ray probe, with their `SceneDemo` adapters for the native scene shell. Pure
//! library: no winit, no composition.

/// Orbit-camera limits shared by the viewers and the character scene's smoothed camera.
pub mod orbit {
    pub use voxy_scene_host::{OrbitController, OrbitLimits};

    /// Pitch is clamped symmetrically so the orbit never flips over the poles.
    pub const ORBIT_PITCH_LIMIT: f32 = 1.2;
    /// Farthest orbit distance any viewer allows.
    pub const ORBIT_MAX_DISTANCE: f32 = 5.0;
    /// Orbit limits and the zoom step scale for the hand close-up versus the whole body.
    #[must_use]
    pub const fn viewer(hand: bool) -> (OrbitLimits, f32) {
        let (nearest, step) = if hand { (0.18, 0.2) } else { (0.7, 1.0) };
        (
            OrbitLimits::symmetric(ORBIT_PITCH_LIMIT, nearest, ORBIT_MAX_DISTANCE),
            step,
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The viewer formulas as they were written before the shared controller.
        fn previous(view: [f32; 3], drag: [f32; 2], amount: f32, hand: bool) -> [f32; 3] {
            let (minimum, step) = if hand { (0.18, 0.2) } else { (0.7, 1.0) };
            [
                view[0] + drag[0],
                (view[1] + drag[1]).clamp(-1.2, 1.2),
                (view[2] + amount * step).clamp(minimum, 5.0),
            ]
        }

        #[test]
        fn viewer_limits_reproduce_the_previous_numbers() {
            let samples = [
                ([0.15, 0.0, 2.4], [0.413, 0.2171], -0.37, false),
                ([-0.85, 0.08, 0.32], [-1.9, 3.0], -0.15, true),
                ([1.1, -1.1, 4.9], [7.0, -0.4], 0.8, false),
                ([0.0, 1.19, 0.19], [0.007, 0.007], -100.0, true),
            ];
            for (view, drag, amount, hand) in samples {
                let (limits, step) = viewer(hand);
                let mut orbit = OrbitController::new(view[0], view[1], view[2]);
                orbit.drag(drag[0], drag[1], &limits);
                orbit.zoom(amount * step, &limits);
                assert_eq!(
                    [orbit.yaw, orbit.pitch, orbit.distance],
                    previous(view, drag, amount, hand)
                );
            }
        }
    }
}

mod error;
pub use error::DemoError;
pub mod biomechanics_demo;
pub mod body_parameters;
pub mod cuda_motion;
pub mod destruction_demo;
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
pub mod female_pelvic;
pub mod female_rig;
pub mod female_scene;
pub mod female_transmission;
pub mod film_settings;
pub mod full_model_worker;
pub mod gpu_hair_solver;
pub mod gravity_demo;
pub mod jump_motion;
pub mod liquid_demo;
pub mod medium_optics;
pub mod model_presentation;
pub mod rig_skinning;
pub mod scenes;
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

pub mod female_physical_stance;

pub mod humanoid_locomotion;

pub mod gpu_liquid;
