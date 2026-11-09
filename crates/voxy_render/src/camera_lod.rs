use crate::{LodError, LodLevel, LodPolicy, SceneCamera};
use glam::{Mat4, Vec3};

/// Conservative transformed domain and object-to-view error amplification bound.
#[derive(Clone, Copy, Debug)]
pub struct SceneLodBounds {
    pub view_min: Vec3,
    pub view_max: Vec3,
    pub error_scale: f64,
}
impl LodPolicy {
    /// Applies the common projection/near-plane policy to any validated LOD
    /// metadata, including a pose-specific error envelope. Residency is separate.
    /// # Errors
    /// Rejects invalid bounds, projection, viewport, levels, policy or history.
    pub fn select_for_camera(
        self,
        levels: &[LodLevel],
        camera: SceneCamera,
        bounds: SceneLodBounds,
        viewport: [u32; 2],
        previous: Option<usize>,
    ) -> Result<usize, LodError> {
        let pixels = camera.lod_pixel_scale(bounds.view_min, bounds.view_max, viewport)?;
        let selected = self.select(levels, bounds.error_scale, pixels.unwrap_or(0.0), previous)?;
        Ok(if pixels.is_some() { selected } else { 0 })
    }
}
impl SceneCamera {
    /// Transforms all eight object AABB corners and bounds affine error amplification.
    /// Includes rotation, nonuniform scale and shear; bounds must enclose every
    /// source/approximation position. Calculations use f64 and outward-rounded f32
    /// bounds. Projective model transforms are unsupported.
    /// # Errors
    /// Rejects invalid cameras, unordered/nonfinite bounds, non-affine matrices,
    /// or transformed bounds outside finite f32 representation.
    pub fn lod_bounds(self, model: Mat4, min: Vec3, max: Vec3) -> Result<SceneLodBounds, LodError> {
        self.view_projection()
            .map_err(|_| LodError::InvalidProjection)?;
        if !model.is_finite()
            || !min.is_finite()
            || !max.is_finite()
            || min.cmpgt(max).any()
            || model.x_axis.w != 0.0
            || model.y_axis.w != 0.0
            || model.z_axis.w != 0.0
            || model.w_axis.w.to_bits() != 1.0_f32.to_bits()
        {
            return Err(LodError::InvalidProjection);
        }
        let view = glam::camera::rh::view::look_at_mat4(self.eye, self.target, self.up);
        let transform = view.as_dmat4() * model.as_dmat4();
        let mut low = glam::DVec3::splat(f64::INFINITY);
        let mut high = glam::DVec3::splat(f64::NEG_INFINITY);
        for corner in 0..8 {
            let point = Vec3::new(
                if corner & 1 == 0 { min.x } else { max.x },
                if corner & 2 == 0 { min.y } else { max.y },
                if corner & 4 == 0 { min.z } else { max.z },
            );
            let transformed = transform.transform_point3(point.as_dvec3());
            low = low.min(transformed);
            high = high.max(transformed);
        }
        let view_min = low.as_vec3().map(f32::next_down);
        let view_max = high.as_vec3().map(f32::next_up);
        if !view_min.is_finite() || !view_max.is_finite() {
            return Err(LodError::InvalidProjection);
        }
        let columns = transform.to_cols_array_2d();
        let one = columns[..3]
            .iter()
            .map(|column| column[..3].iter().map(|n| n.abs()).sum::<f64>())
            .fold(0.0_f64, f64::max);
        let infinity = (0..3)
            .map(|row| {
                columns[..3]
                    .iter()
                    .map(|column| column[row].abs())
                    .sum::<f64>()
            })
            .fold(0.0_f64, f64::max);
        let error_scale = (one * infinity).sqrt();
        if !error_scale.is_finite() {
            return Err(LodError::InvalidProjection);
        }
        Ok(SceneLodBounds {
            view_min,
            view_max,
            error_scale,
        })
    }
}

/// Projects a world-space geometric simplification error `world_error_m` at camera distance
/// `distance_m` into screen-space pixels:
/// `px = (world_error_m / distance_m) * (viewport_height_px / (2 * tan(vertical_fov_rad * 0.5)))`.
#[must_use]
pub fn screen_space_geometric_error_px(
    world_error_m: f32,
    distance_m: f32,
    vertical_fov_rad: f32,
    viewport_height_px: f32,
) -> f32 {
    if !world_error_m.is_finite()
        || !distance_m.is_finite()
        || !vertical_fov_rad.is_finite()
        || !viewport_height_px.is_finite()
        || world_error_m <= 0.0
        || distance_m <= 1e-5
        || !(1e-3..std::f32::consts::PI).contains(&vertical_fov_rad)
        || viewport_height_px <= 0.0
    {
        return 0.0;
    }
    let half_tan = (0.5 * vertical_fov_rad).tan().max(1e-6);
    ((world_error_m / distance_m) * (0.5 * viewport_height_px / half_tan)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lod_bounds_transform_covers_shear_and_rejects_projective_models() {
        let camera = SceneCamera {
            eye: Vec3::ZERO,
            target: Vec3::NEG_Z,
            up: Vec3::Y,
            projection: crate::SceneProjection::Perspective {
                vertical_fov: 1.0,
                aspect: 1.0,
                near: 0.1,
                far: 100.0,
            },
        };
        let model = Mat4::from_cols_array(&[
            1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -10.0, 1.0,
        ]);
        let bounds = camera.lod_bounds(model, -Vec3::ONE, Vec3::ONE).unwrap();
        assert!(bounds.view_min.x < -2.0 && bounds.view_max.x > 2.0);
        assert!(bounds.error_scale >= 2.0);
        assert!(
            camera
                .lod_pixel_scale(bounds.view_min, bounds.view_max, [800, 800])
                .unwrap()
                .is_some()
        );
        assert!(
            camera
                .lod_bounds(
                    glam::camera::rh::proj::directx::perspective(1.0, 1.0, 0.1, 100.0),
                    -Vec3::ONE,
                    Vec3::ONE
                )
                .is_err()
        );
        let px_near = screen_space_geometric_error_px(0.05, 5.0, std::f32::consts::FRAC_PI_2, 1080.0);
        let px_far = screen_space_geometric_error_px(0.05, 50.0, std::f32::consts::FRAC_PI_2, 1080.0);
        assert!((px_near - 5.4).abs() < 1e-4);
        assert!((px_near - 10.0 * px_far).abs() < 1e-4);
    }
}
