//! Detailed continuous face morphology in bind coordinates; reference defaults are identity.
use crate::DemoError;
use glam::{Mat4, Vec3};
use std::collections::BTreeMap;
use voxy_render::SceneVertex;

mod controls_lower;
mod controls_upper;

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

fn all_controls() -> impl Iterator<Item = &'static Control> {
    controls_upper::CONTROLS_UPPER
        .iter()
        .chain(controls_lower::CONTROLS_LOWER.iter())
}

#[cfg(test)]
fn control_count() -> usize {
    controls_upper::CONTROLS_UPPER.len() + controls_lower::CONTROLS_LOWER.len()
}

pub fn wrinkle_profile(key: &str, p: Vec3) -> f32 {
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
pub fn wrinkle_microprofile(key: &str, p: Vec3) -> f32 {
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
            values: all_controls()
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
    pub fn patched(&self, patch: &serde_json::Value) -> Result<Self, DemoError> {
        let object = patch.as_object().ok_or("expected face parameter object")?;
        let mut result = self.clone();
        for (key, value) in object {
            let control = all_controls()
                .find(|c| c.key == key)
                .ok_or_else(|| format!("unknown face control: {key}"))?;
            let number = value
                .as_f64()
                .ok_or_else(|| format!("{key}: expected number"))? as f32;
            if !number.is_finite() || !(control.min..=control.max).contains(&number) {
                return Err(format!("{key}: allowed {}..{}", control.min, control.max).into());
            }
            result.values.insert(key.clone(), number);
        }
        Ok(result)
    }
    /// # Errors
    /// Rejects malformed JSON and invalid controls.
    pub fn from_json(text: &str) -> Result<Self, DemoError> {
        Self::default().patched(&serde_json::from_str(text)?)
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
    #[allow(dead_code)]
    pub fn apply(&self, vertices: &mut [SceneVertex], head: Mat4) {
        self.apply_filtered(vertices, head, None);
    }
    pub fn apply_wrinkles(&self, vertices: &mut [SceneVertex]) {
        self.apply_filtered(vertices, Mat4::IDENTITY, Some(true));
    }
    pub fn apply_shape(&self, vertices: &mut [SceneVertex], head: Mat4) {
        self.apply_filtered(vertices, head, Some(false));
    }
    pub fn material_creases(&self) -> Vec<(&'static str, Vec3, Vec3, f32)> {
        all_controls()
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
    pub fn material_signature(&self) -> Vec<f32> {
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
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn apply_filtered(&self, vertices: &mut [SceneVertex], head: Mat4, wrinkles: Option<bool>) {
        struct ActiveControl<'a> {
            control: &'a Control,
            amount: f32,
            is_left: bool,
            is_right: bool,
            is_nose: bool,
            center: Vec3,
            inv_radius: Vec3,
        }

        let active: Vec<_> = all_controls()
            .filter_map(|control| {
                let amount = self.values[control.key] - control.default;
                (control.mode != 2
                    && amount != 0.
                    && wrinkles.is_none_or(|only| (control.mode == 3) == only))
                .then(|| {
                    let rad = Vec3::from_array(control.radius);
                    ActiveControl {
                        control,
                        amount,
                        is_left: control.key.ends_with("_left"),
                        is_right: control.key.ends_with("_right"),
                        is_nose: control.key.starts_with("nose_"),
                        center: Vec3::from_array(control.center),
                        inv_radius: Vec3::new(1.0 / rad.x, 1.0 / rad.y, 1.0 / rad.z),
                    }
                })
            })
            .collect();
        if active.is_empty() {
            return;
        }
        let mut min_bound = Vec3::splat(f32::INFINITY);
        let mut max_bound = Vec3::splat(f32::NEG_INFINITY);
        for active_ctrl in &active {
            let r = Vec3::from_array(active_ctrl.control.radius);
            let c = active_ctrl.center;
            let cx = c.x.abs();
            min_bound = min_bound.min(Vec3::new(-cx - r.x, c.y - r.y, c.z - r.z));
            max_bound = max_bound.max(Vec3::new(cx + r.x, c.y + r.y, c.z + r.z));
        }

        let inverse = head.inverse();
        for vertex in vertices {
            let point = inverse.transform_point3(Vec3::from_array(vertex.position));
            if point.y < 0.575
                || point.x < min_bound.x
                || point.x > max_bound.x
                || point.y < min_bound.y
                || point.y > max_bound.y
                || point.z < min_bound.z
                || point.z > max_bound.z
            {
                continue;
            }
            let mut delta = Vec3::ZERO;
            for active_ctrl in &active {
                // Anatomical left is positive X in this bind-space model.
                if (active_ctrl.is_left && point.x <= 0.) || (active_ctrl.is_right && point.x >= 0.)
                {
                    continue;
                }
                let mut center = active_ctrl.center;
                if center.x != 0. {
                    center.x *= if point.x < 0. { -1. } else { 1. };
                }
                let local = point - center;
                let norm_local = local * active_ctrl.inv_radius;
                let distance = norm_local.length_squared();
                if distance >= 1.0 {
                    continue;
                }
                // Compact C1 support: no movement leaks into the opposite feature or neck.
                let weight = (1. - distance).powi(2);
                let control = active_ctrl.control;
                let amount = active_ctrl.amount;
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
                let side_weight = if active_ctrl.is_left || active_ctrl.is_right {
                    let t = (point.x.abs() / 0.005).min(1.);
                    t * t * (3. - 2. * t)
                } else {
                    1.
                };
                // Per-component feature mask, analogous to Simplex targetWeights:
                // the nasal ellipsoid overlaps upper-lip vertices in this asset.
                // Taper its contribution to zero below the nasal base.
                let feature_weight = if active_ctrl.is_nose {
                    let t = ((point.y - 0.653) / 0.010).clamp(0., 1.);
                    t * t * (3. - 2. * t)
                } else {
                    1.
                };
                delta[control.axis] += weight * side_weight * feature_weight * change;
            }
            if delta != Vec3::ZERO {
                vertex.position = head.transform_point3(point + delta).to_array();
            }
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
        assert_eq!(schema.as_array().unwrap().len(), control_count());
        let asset = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        for control in all_controls() {
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
        let (w_a, w_b) = clamp_antagonist_facs_weights(0.8, 0.6);
        assert!((w_a - 0.8 / 1.4).abs() < 1e-6);
        assert!((w_b - 0.6 / 1.4).abs() < 1e-6);
        assert_eq!(clamp_antagonist_facs_weights(0.3, 0.4), (0.3, 0.4));
    }
}

/// Normalizes a pair of mutually antagonistic Facial Action Coding System (FACS) weights
/// `(primary_au, antagonist_au)` (such as brow raiser AU1/2 vs brow lowerer AU4, or lip corner
/// puller AU12 vs depressor AU15) into `[0.0, 1.0]` with `primary + antagonist <= 1.0`.
#[must_use]
pub fn clamp_antagonist_facs_weights(primary_au: f32, antagonist_au: f32) -> (f32, f32) {
    let a = if primary_au.is_finite() {
        primary_au.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let b = if antagonist_au.is_finite() {
        antagonist_au.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let sum = a + b;
    if sum > 1.0 {
        (a / sum, b / sum)
    } else {
        (a, b)
    }
}
