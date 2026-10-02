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
    }
}
