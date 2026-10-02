//! Detailed continuous face morphology in bind coordinates; reference defaults are identity.
use glam::{Mat4, Vec3};
use std::collections::BTreeMap;
use voxy_render::SceneVertex;

#[derive(Clone, Debug, PartialEq)]
pub struct FaceParameters {
    values: BTreeMap<String, f32>,
}
#[derive(Clone, Copy)]
struct Control {
    key: &'static str,
    default: f32,
    min: f32,
    max: f32,
    center: [f32; 3],
    radius: [f32; 3],
    axis: usize,
    mode: u8,
}
const CONTROLS: &[Control] = &[
    Control {
        key: "face_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.705, 0.12],
        radius: [0.09, 0.1, 0.09],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "face_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.705, 0.12],
        radius: [0.09, 0.1, 0.09],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "face_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.705, 0.12],
        radius: [0.09, 0.1, 0.09],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "face_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.705, 0.12],
        radius: [0.09, 0.1, 0.09],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "face_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.705, 0.12],
        radius: [0.09, 0.1, 0.09],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "forehead_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.766, 0.125],
        radius: [0.065, 0.045, 0.06],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "forehead_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.766, 0.125],
        radius: [0.065, 0.045, 0.06],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "forehead_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.766, 0.125],
        radius: [0.065, 0.045, 0.06],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "forehead_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.766, 0.125],
        radius: [0.065, 0.045, 0.06],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "forehead_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.766, 0.125],
        radius: [0.065, 0.045, 0.06],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "cheeks_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "cheeks_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "cheeks_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "cheeks_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "cheeks_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "cheeks_spacing",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "jaw_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "jaw_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "jaw_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "jaw_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "jaw_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "jaw_spacing",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "chin_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.606, 0.123],
        radius: [0.037, 0.027, 0.045],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "chin_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.606, 0.123],
        radius: [0.037, 0.027, 0.045],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "chin_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.606, 0.123],
        radius: [0.037, 0.027, 0.045],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "chin_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.606, 0.123],
        radius: [0.037, 0.027, 0.045],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "chin_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.606, 0.123],
        radius: [0.037, 0.027, 0.045],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "nose_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.686, 0.145],
        radius: [0.026, 0.038, 0.04],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "nose_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.686, 0.145],
        radius: [0.026, 0.038, 0.04],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "nose_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.686, 0.145],
        radius: [0.026, 0.038, 0.04],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "nose_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.686, 0.145],
        radius: [0.026, 0.038, 0.04],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "nose_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.686, 0.145],
        radius: [0.026, 0.038, 0.04],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "nose_bridge_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.707, 0.144],
        radius: [0.014, 0.026, 0.027],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "nose_bridge_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.707, 0.144],
        radius: [0.014, 0.026, 0.027],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "nose_bridge_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.707, 0.144],
        radius: [0.014, 0.026, 0.027],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "nose_bridge_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.707, 0.144],
        radius: [0.014, 0.026, 0.027],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "nose_bridge_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.707, 0.144],
        radius: [0.014, 0.026, 0.027],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "nose_tip_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.675, 0.16],
        radius: [0.018, 0.017, 0.023],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "nose_tip_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.675, 0.16],
        radius: [0.018, 0.017, 0.023],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "nose_tip_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.675, 0.16],
        radius: [0.018, 0.017, 0.023],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "nose_tip_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.675, 0.16],
        radius: [0.018, 0.017, 0.023],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "nose_tip_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.675, 0.16],
        radius: [0.018, 0.017, 0.023],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "nostrils_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "nostrils_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "nostrils_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "nostrils_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "nostrils_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "nostrils_spacing",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "ears_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "ears_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "ears_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "ears_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "ears_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "ears_spacing",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "earlobes_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "earlobes_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "earlobes_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "earlobes_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "earlobes_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "earlobes_spacing",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "eyes_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "eyes_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "eyes_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "eyes_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "eyes_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "eyes_spacing",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "upper_lids_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "upper_lids_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "upper_lids_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "upper_lids_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "upper_lids_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "upper_lids_spacing",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "lower_lids_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "lower_lids_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "lower_lids_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "lower_lids_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "lower_lids_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "lower_lids_spacing",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "lips_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.6465, 0.142],
        radius: [0.038, 0.018, 0.032],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "lips_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.6465, 0.142],
        radius: [0.038, 0.018, 0.032],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "lips_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.6465, 0.142],
        radius: [0.038, 0.018, 0.032],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "lips_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.6465, 0.142],
        radius: [0.038, 0.018, 0.032],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "lips_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.6465, 0.142],
        radius: [0.038, 0.018, 0.032],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "upper_lip_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.651, 0.144],
        radius: [0.033, 0.009, 0.018],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "upper_lip_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.651, 0.144],
        radius: [0.033, 0.009, 0.018],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "upper_lip_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.651, 0.144],
        radius: [0.033, 0.009, 0.018],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "upper_lip_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.651, 0.144],
        radius: [0.033, 0.009, 0.018],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "upper_lip_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.651, 0.144],
        radius: [0.033, 0.009, 0.018],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "lower_lip_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.639, 0.144],
        radius: [0.033, 0.01, 0.018],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "lower_lip_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.639, 0.144],
        radius: [0.033, 0.01, 0.018],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "lower_lip_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.639, 0.144],
        radius: [0.033, 0.01, 0.018],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "lower_lip_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.639, 0.144],
        radius: [0.033, 0.01, 0.018],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "lower_lip_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.639, 0.144],
        radius: [0.033, 0.01, 0.018],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "mouth_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.642, 0.124],
        radius: [0.04, 0.027, 0.054],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "mouth_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.642, 0.124],
        radius: [0.04, 0.027, 0.054],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "mouth_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.0, 0.642, 0.124],
        radius: [0.04, 0.027, 0.054],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "mouth_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.642, 0.124],
        radius: [0.04, 0.027, 0.054],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "mouth_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.0, 0.642, 0.124],
        radius: [0.04, 0.027, 0.054],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "brows_width",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "brows_height",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "brows_depth",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "brows_vertical",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "brows_projection",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "brows_spacing",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "lashes_length",
        default: 1.0,
        min: 0.3,
        max: 2.0,
        center: [0.0, 0.0, 0.0],
        radius: [1.0, 1.0, 1.0],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "lashes_thickness",
        default: 1.0,
        min: 0.3,
        max: 2.0,
        center: [0.0, 0.0, 0.0],
        radius: [1.0, 1.0, 1.0],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "lashes_density",
        default: 1.0,
        min: 0.0,
        max: 1.0,
        center: [0.0, 0.0, 0.0],
        radius: [1.0, 1.0, 1.0],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "brow_hair_length",
        default: 1.0,
        min: 0.3,
        max: 2.0,
        center: [0.0, 0.0, 0.0],
        radius: [1.0, 1.0, 1.0],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "brow_hair_thickness",
        default: 1.0,
        min: 0.3,
        max: 2.0,
        center: [0.0, 0.0, 0.0],
        radius: [1.0, 1.0, 1.0],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "brow_hair_density",
        default: 1.0,
        min: 0.0,
        max: 1.0,
        center: [0.0, 0.0, 0.0],
        radius: [1.0, 1.0, 1.0],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "cheeks_width_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "cheeks_width_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "cheeks_height_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "cheeks_height_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "cheeks_depth_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "cheeks_depth_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "cheeks_vertical_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "cheeks_vertical_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "cheeks_projection_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "cheeks_projection_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "cheeks_spacing_left",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "cheeks_spacing_right",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.047, 0.684, 0.122],
        radius: [0.032, 0.04, 0.045],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "jaw_width_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "jaw_width_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "jaw_height_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "jaw_height_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "jaw_depth_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "jaw_depth_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "jaw_vertical_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "jaw_vertical_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "jaw_projection_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "jaw_projection_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "jaw_spacing_left",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "jaw_spacing_right",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.039, 0.627, 0.102],
        radius: [0.047, 0.037, 0.065],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "nostrils_width_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "nostrils_width_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "nostrils_height_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "nostrils_height_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "nostrils_depth_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "nostrils_depth_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "nostrils_vertical_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "nostrils_vertical_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "nostrils_projection_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "nostrils_projection_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "nostrils_spacing_left",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "nostrils_spacing_right",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.013, 0.67, 0.15],
        radius: [0.012, 0.012, 0.018],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "ears_width_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "ears_width_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "ears_height_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "ears_height_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "ears_depth_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "ears_depth_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "ears_vertical_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "ears_vertical_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "ears_projection_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "ears_projection_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "ears_spacing_left",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "ears_spacing_right",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.076, 0.691, 0.071],
        radius: [0.026, 0.045, 0.05],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "earlobes_width_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "earlobes_width_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "earlobes_height_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "earlobes_height_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "earlobes_depth_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "earlobes_depth_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "earlobes_vertical_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "earlobes_vertical_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "earlobes_projection_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "earlobes_projection_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "earlobes_spacing_left",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "earlobes_spacing_right",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.076, 0.663, 0.073],
        radius: [0.02, 0.019, 0.028],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "eyes_width_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "eyes_width_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "eyes_height_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "eyes_height_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "eyes_depth_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "eyes_depth_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "eyes_vertical_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "eyes_vertical_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "eyes_projection_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "eyes_projection_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "eyes_spacing_left",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "eyes_spacing_right",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.033, 0.7124, 0.1215],
        radius: [0.027, 0.024, 0.025],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "upper_lids_width_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "upper_lids_width_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "upper_lids_height_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "upper_lids_height_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "upper_lids_depth_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "upper_lids_depth_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "upper_lids_vertical_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "upper_lids_vertical_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "upper_lids_projection_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "upper_lids_projection_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "upper_lids_spacing_left",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "upper_lids_spacing_right",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.033, 0.72, 0.129],
        radius: [0.025, 0.012, 0.014],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "lower_lids_width_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "lower_lids_width_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "lower_lids_height_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "lower_lids_height_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "lower_lids_depth_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "lower_lids_depth_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "lower_lids_vertical_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "lower_lids_vertical_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "lower_lids_projection_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "lower_lids_projection_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "lower_lids_spacing_left",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "lower_lids_spacing_right",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.033, 0.705, 0.129],
        radius: [0.025, 0.011, 0.014],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "brows_width_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "brows_width_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 0,
        mode: 0,
    },
    Control {
        key: "brows_height_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "brows_height_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 1,
        mode: 0,
    },
    Control {
        key: "brows_depth_left",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "brows_depth_right",
        default: 1.0,
        min: 0.7,
        max: 1.3,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 2,
        mode: 0,
    },
    Control {
        key: "brows_vertical_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "brows_vertical_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 1,
        mode: 1,
    },
    Control {
        key: "brows_projection_left",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "brows_projection_right",
        default: 0.0,
        min: -5.0,
        max: 5.0,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "brows_spacing_left",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "brows_spacing_right",
        default: 0.0,
        min: -4.0,
        max: 4.0,
        center: [0.035, 0.741, 0.135],
        radius: [0.034, 0.018, 0.03],
        axis: 0,
        mode: 1,
    },
    Control {
        key: "wrinkles_forehead",
        default: 0.,
        min: 0.,
        max: 0.8,
        center: [0.0, 0.766, 0.13],
        radius: [0.065, 0.026, 0.04],
        axis: 2,
        mode: 3,
    },
    Control {
        key: "wrinkles_glabella",
        default: 0.,
        min: 0.,
        max: 0.8,
        center: [0.0, 0.741, 0.13],
        radius: [0.02, 0.024, 0.04],
        axis: 2,
        mode: 3,
    },
    Control {
        key: "wrinkles_crow_feet",
        default: 0.,
        min: 0.,
        max: 0.6,
        center: [0.055, 0.713, 0.13],
        radius: [0.02, 0.014, 0.04],
        axis: 2,
        mode: 3,
    },
    Control {
        key: "wrinkles_under_eyes",
        default: 0.,
        min: 0.,
        max: 0.6,
        center: [0.033, 0.701, 0.13],
        radius: [0.027, 0.014, 0.04],
        axis: 2,
        mode: 3,
    },
    Control {
        key: "folds_nasolabial",
        default: 0.,
        min: 0.,
        max: 1.5,
        center: [0.028, 0.673, 0.13],
        radius: [0.02, 0.035, 0.045],
        axis: 2,
        mode: 3,
    },
    Control {
        key: "folds_marionette",
        default: 0.,
        min: 0.,
        max: 1.2,
        center: [0.024, 0.635, 0.13],
        radius: [0.018, 0.025, 0.045],
        axis: 2,
        mode: 3,
    },
    Control {
        key: "wrinkles_upper_lip",
        default: 0.,
        min: 0.,
        max: 0.6,
        center: [0.0, 0.655, 0.14],
        radius: [0.025, 0.014, 0.04],
        axis: 2,
        mode: 3,
    },
    Control {
        key: "age_cheek_volume",
        default: 0.,
        min: -3.,
        max: 3.0,
        center: [0.041, 0.684, 0.13],
        radius: [0.034, 0.03, 0.045],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "age_cheek_descent",
        default: 0.,
        min: 0.,
        max: 4.,
        center: [0.039, 0.682, 0.13],
        radius: [0.035, 0.032, 0.045],
        axis: 1,
        mode: 4,
    },
    Control {
        key: "age_jowl_descent",
        default: 0.,
        min: 0.,
        max: 4.,
        center: [0.043, 0.644, 0.115],
        radius: [0.028, 0.027, 0.04],
        axis: 1,
        mode: 4,
    },
    Control {
        key: "under_eye_bags",
        default: 0.,
        min: 0.,
        max: 2.0,
        center: [0.033, 0.701, 0.13],
        radius: [0.023, 0.009, 0.035],
        axis: 2,
        mode: 1,
    },
    Control {
        key: "skin_laxity",
        default: 0.,
        min: 0.,
        max: 4.0,
        center: [0.044, 0.661, 0.12],
        radius: [0.045, 0.045, 0.055],
        axis: 1,
        mode: 4,
    },
    Control {
        key: "temple_hollowing",
        default: 0.,
        min: 0.,
        max: 3.0,
        center: [0.069, 0.731, 0.12],
        radius: [0.025, 0.029, 0.045],
        axis: 2,
        mode: 4,
    },
    Control {
        key: "skin_roughness_scale",
        default: 1.,
        min: 0.25,
        max: 2.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "skin_oil_scale",
        default: 1.,
        min: 0.,
        max: 2.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "pupil_diameter_mm",
        default: 4.05,
        min: 2.,
        max: 8.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "skin_surface_light",
        default: 0.35,
        min: 0.,
        max: 1.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "skin_pores",
        default: 1.,
        min: 0.,
        max: 1.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "skin_makeup",
        default: 1.,
        min: 0.,
        max: 1.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "makeup_lipstick",
        default: 1.,
        min: 0.,
        max: 1.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "makeup_blush",
        default: 1.,
        min: 0.,
        max: 1.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "makeup_eyeshadow",
        default: 1.,
        min: 0.,
        max: 1.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "lipstick_red",
        default: 0.92,
        min: 0.,
        max: 1.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "lipstick_green",
        default: 0.43,
        min: 0.,
        max: 1.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
    Control {
        key: "lipstick_blue",
        default: 0.53,
        min: 0.,
        max: 1.,
        center: [0.; 3],
        radius: [0.; 3],
        axis: 0,
        mode: 2,
    },
];

pub(crate) fn wrinkle_profile(key: &str, p: Vec3) -> f32 {
    let line = |distance: f32, width: f32| (-(distance / width).powi(2)).exp();
    let x = p.x.abs();
    match key {
        "wrinkles_forehead" => [0.761, 0.769, 0.776]
            .into_iter()
            .map(|y| line(p.y - y + 0.9 * p.x * p.x, 0.0009))
            .fold(0., f32::max),
        "wrinkles_glabella" => line(x - 0.004, 0.0012),
        "wrinkles_crow_feet" => [-0.35, 0., 0.35]
            .into_iter()
            .map(|slope| line(p.y - 0.712 - slope * (x - 0.05), 0.001))
            .fold(0., f32::max),
        "wrinkles_under_eyes" => [0.701, 0.697]
            .into_iter()
            .enumerate()
            .map(|(i, y)| {
                let phase = i as f32 * 1.7;
                let center = y + 6. * (x - 0.033).powi(2) + 0.00025 * (190. * x + phase).sin();
                let width = 0.0007 * (0.8 + 0.2 * (130. * x + phase).cos());
                line(p.y - center, width) * (0.55 + 0.45 * (95. * x + phase).sin().powi(2))
            })
            .fold(0., f32::max),
        "folds_nasolabial" => {
            let t = ((p.y - 0.650) / 0.036).clamp(0., 1.);
            line(x - (0.018 + 0.018 * t * t * (3. - 2. * t)), 0.002)
        }
        "folds_marionette" => line(x - 0.024 - 0.25 * (0.649 - p.y), 0.0018),
        "wrinkles_upper_lip" => [0.003, 0.006, 0.009, 0.012]
            .into_iter()
            .enumerate()
            .map(|(i, a)| {
                let phase = i as f32 * 1.7;
                let path = a + 0.10 * (p.y - 0.655) + 0.0002 * (350. * p.y + phase).sin();
                line(x - path, 0.00065) * (0.45 + 0.55 * (260. * p.y + phase).sin().powi(2))
            })
            .fold(0., f32::max),
        _ => 0.,
    }
}
pub(crate) fn wrinkle_microprofile(key: &str, p: Vec3) -> f32 {
    let base = wrinkle_profile(key, p).powi(4);
    let x = p.x.abs();
    let line = |d: f32| (-(d / 0.00022).powi(2)).exp();
    let fine = match key {
        "wrinkles_crow_feet" => [-0.5, -0.18, 0.18, 0.5]
            .into_iter()
            .enumerate()
            .map(|(i, slope)| {
                let phase = i as f32 * 1.7;
                let d = p.y - 0.712 - slope * (x - 0.05) + 0.00012 * (900. * x + phase).sin();
                line(d) * (0.35 + 0.65 * (410. * x + phase).sin().powi(2))
            })
            .fold(0., f32::max),
        "wrinkles_under_eyes" => [0.699, 0.6955, 0.6935]
            .into_iter()
            .enumerate()
            .map(|(i, y)| {
                line(p.y - y + 0.7 * (x - 0.033).powi(2) + 0.00012 * (700. * x + i as f32).sin())
                    * (0.35 + 0.65 * (370. * x + i as f32).sin().powi(2))
            })
            .fold(0., f32::max),
        _ => 0.,
    };
    base.max(fine * 0.65)
}
impl Default for FaceParameters {
    fn default() -> Self {
        Self {
            values: CONTROLS
                .iter()
                .map(|c| (c.key.to_owned(), c.default))
                .collect(),
        }
    }
}
impl FaceParameters {
    pub fn value(&self, key: &str) -> Option<f32> {
        self.values.get(key).copied()
    }
    /// Atomically validates a partial object; omitted values preserve the current preset.
    /// # Errors
    /// Rejects unknown controls, nonnumeric values, nonfinite values and out-of-range values.
    pub fn patched(&self, patch: &serde_json::Value) -> Result<Self, String> {
        let object = patch.as_object().ok_or("expected face parameter object")?;
        let mut result = self.clone();
        for (key, value) in object {
            let control = CONTROLS
                .iter()
                .find(|c| c.key == key)
                .ok_or_else(|| format!("unknown face control: {key}"))?;
            let number = value
                .as_f64()
                .ok_or_else(|| format!("{key}: expected number"))? as f32;
            if !number.is_finite() || !(control.min..=control.max).contains(&number) {
                return Err(format!("{key}: allowed {}..{}", control.min, control.max));
            }
            result.values.insert(key.clone(), number);
        }
        Ok(result)
    }
    /// # Errors
    /// Rejects malformed JSON and invalid controls.
    pub fn from_json(text: &str) -> Result<Self, String> {
        Self::default().patched(&serde_json::from_str(text).map_err(|e| e.to_string())?)
    }
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!(self.values)
    }
    pub fn schema() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../assets/characters/face-controls.json"
        ))
        .expect("compiled control schema")
    }
    /// Applies all deltas from the same reference point, preventing ordering drift.
    pub(crate) fn apply(&self, vertices: &mut [SceneVertex], head: Mat4) {
        self.apply_filtered(vertices, head, None);
    }
    pub(crate) fn apply_wrinkles(&self, vertices: &mut [SceneVertex]) {
        self.apply_filtered(vertices, Mat4::IDENTITY, Some(true));
    }
    pub(crate) fn apply_shape(&self, vertices: &mut [SceneVertex], head: Mat4) {
        self.apply_filtered(vertices, head, Some(false));
    }
    pub(crate) fn material_creases(&self) -> Vec<(&'static str, Vec3, Vec3, f32)> {
        CONTROLS
            .iter()
            .filter(|c| c.mode == 3)
            .map(|c| {
                (
                    c.key,
                    Vec3::from_array(c.center),
                    Vec3::from_array(c.radius),
                    self.values[c.key],
                )
            })
            .collect()
    }
    pub(crate) fn material_signature(&self) -> Vec<f32> {
        let mut signature: Vec<_> = self.material_creases().iter().map(|c| c.3).collect();
        signature.push(self.values["skin_roughness_scale"]);
        signature.push(self.values["skin_oil_scale"]);
        signature.push(self.values["skin_pores"]);
        signature.push(self.values["skin_makeup"]);
        signature.push(self.values["makeup_lipstick"]);
        signature.push(self.values["makeup_blush"]);
        signature.push(self.values["makeup_eyeshadow"]);
        signature.push(self.values["lipstick_red"]);
        signature.push(self.values["lipstick_green"]);
        signature.push(self.values["lipstick_blue"]);

        signature
    }
    fn apply_filtered(&self, vertices: &mut [SceneVertex], head: Mat4, wrinkles: Option<bool>) {
        let active: Vec<_> = CONTROLS
            .iter()
            .filter_map(|control| {
                let amount = self.values[control.key] - control.default;
                (control.mode != 2
                    && amount != 0.
                    && wrinkles.is_none_or(|only| (control.mode == 3) == only))
                .then_some((control, amount))
            })
            .collect();
        if active.is_empty() {
            return;
        }
        let inverse = head.inverse();
        for vertex in vertices {
            let point = inverse.transform_point3(Vec3::from_array(vertex.position));
            if point.y < 0.575 {
                continue;
            }
            let mut delta = Vec3::ZERO;
            for &(control, amount) in &active {
                // Anatomical left is positive X in this bind-space model.
                if (control.key.ends_with("_left") && point.x <= 0.)
                    || (control.key.ends_with("_right") && point.x >= 0.)
                {
                    continue;
                }
                let mut center = Vec3::from_array(control.center);
                if center.x != 0. {
                    center.x *= if point.x < 0. { -1. } else { 1. };
                }
                let local = point - center;
                let distance = (local / Vec3::from_array(control.radius)).length_squared();
                // Compact C1 support: no movement leaks into the opposite feature or neck.
                let weight = (1. - distance).max(0.).powi(2);
                let change = if control.mode == 3 {
                    -0.001 * amount * wrinkle_profile(control.key, point)
                } else if control.mode == 0 {
                    local[control.axis] * amount
                } else {
                    amount
                        * if control.mode == 4 { -0.001 } else { 0.001 }
                        * if control.axis == 0 && point.x < 0. {
                            -1.
                        } else {
                            1.
                        }
                };
                let side_weight =
                    if control.key.ends_with("_left") || control.key.ends_with("_right") {
                        let t = (point.x.abs() / 0.005).min(1.);
                        t * t * (3. - 2. * t)
                    } else {
                        1.
                    };
                // Per-component feature mask, analogous to Simplex targetWeights:
                // the nasal ellipsoid overlaps upper-lip vertices in this asset.
                // Taper its contribution to zero below the nasal base.
                let feature_weight = if control.key.starts_with("nose_") {
                    let t = ((point.y - 0.653) / 0.010).clamp(0., 1.);
                    t * t * (3. - 2. * t)
                } else {
                    1.
                };
                delta[control.axis] += weight * side_weight * feature_weight * change;
            }
            vertex.position = head.transform_point3(point + delta).to_array();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_feature_masks_preserve_remote_anatomy_on_the_actual_mesh() {
        let asset = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        for key in ["lips_height", "nose_width"] {
            let parameters = FaceParameters::default()
                .patched(&serde_json::json!({key:1.3}))
                .unwrap();
            let mut vertices = asset.mesh.vertices().to_vec();
            parameters.apply(&mut vertices, Mat4::IDENTITY);
            let mut remote = 0;
            let mut changed = 0;
            for (a, b) in vertices.iter().zip(asset.mesh.vertices()) {
                let p = Vec3::from_array(b.position);
                let outside = if key == "lips_height" {
                    p.y > 0.7 || p.x.abs() > 0.06
                } else {
                    p.y < 0.65 || p.x.abs() > 0.065
                };
                if outside {
                    assert_eq!(a.position, b.position, "{key} moved remote anatomy");
                    remote += 1;
                }
                if a.position != b.position {
                    changed += 1;
                }
            }
            assert!(remote > 100 && changed > 10);
        }
    }
    #[test]
    fn catalog_matches_ui_and_every_shape_control_changes_real_surface() {
        let schema = FaceParameters::schema();
        assert_eq!(schema.as_array().unwrap().len(), CONTROLS.len());
        let asset = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        for control in CONTROLS {
            let item = schema
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["key"] == control.key)
                .unwrap();
            assert!((item["default"].as_f64().unwrap() as f32 - control.default).abs() < 1e-6);
            if control.mode == 2 {
                continue;
            }
            let mut patch = serde_json::Map::new();
            patch.insert(control.key.into(), serde_json::json!(control.max));
            let parameters = FaceParameters::default().patched(&patch.into()).unwrap();
            let mut vertices = asset.mesh.vertices().to_vec();
            parameters.apply(&mut vertices, Mat4::IDENTITY);
            let moved = vertices
                .iter()
                .zip(asset.mesh.vertices())
                .filter(|(a, b)| {
                    Vec3::from_array(a.position).distance(Vec3::from_array(b.position)) > 0.00001
                })
                .count();
            assert!(
                moved > 5,
                "{} affects only {} source vertices",
                control.key,
                moved
            );
            assert!(
                vertices
                    .iter()
                    .all(|v| Vec3::from_array(v.position).is_finite())
            );
        }
    }
    #[test]
    fn side_controls_preserve_opposite_side_and_midline() {
        let original: Vec<_> = [0.038, -0.038, 0.]
            .into_iter()
            .map(|x| SceneVertex {
                position: [x, 0.712, 0.13],
                uv: [0.; 2],
                color: [1.; 4],
            })
            .collect();
        for (side, changed, unchanged) in [("left", 0, 1), ("right", 1, 0)] {
            let mut vertices = original.clone();
            let patch = serde_json::json!({format!("eyes_width_{side}"): 1.3});
            FaceParameters::default()
                .patched(&patch)
                .unwrap()
                .apply(&mut vertices, Mat4::IDENTITY);
            assert_ne!(vertices[changed].position, original[changed].position);
            assert_eq!(vertices[unchanged], original[unchanged]);
            assert_eq!(vertices[2], original[2]);
        }
    }
    #[test]
    fn rejects_invalid_patch_atomically() {
        let p = FaceParameters::default();
        assert!(
            p.patched(&serde_json::json!({"nose_width":1.2,"eyes_width":9}))
                .is_err()
        );
        assert!(p.patched(&serde_json::json!({"nose_wdth":1})).is_err());
        assert_eq!(p.value("nose_width"), Some(1.));
    }
    #[test]
    fn reference_identity_locality_and_independent_controls() {
        let original = vec![
            SceneVertex {
                position: [0.01, 0.68, 0.15],
                uv: [0.; 2],
                color: [1.; 4],
            },
            SceneVertex {
                position: [0., 0.3, 0.1],
                uv: [0.; 2],
                color: [1.; 4],
            },
        ];
        let mut v = original.clone();
        FaceParameters::default().apply(&mut v, Mat4::IDENTITY);
        assert_eq!(v, original);
        let p = FaceParameters::default()
            .patched(&serde_json::json!({"nose_width":1.3}))
            .unwrap();
        p.apply(&mut v, Mat4::IDENTITY);
        assert!(v[0].position[0] > original[0].position[0]);
        assert_eq!(v[0].position[1], original[0].position[1]);
        assert_eq!(v[1], original[1]);
    }
}
