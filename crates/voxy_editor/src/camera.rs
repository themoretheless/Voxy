//! Editor view state is separate from authoring transforms and simulation poses.
use glam::{Mat4, Vec2, Vec3};
use voxy_render::{InvalidSceneCamera, SceneCamera, SceneProjection};

// One wheel line is an editor sensitivity unit of 40 logical pixels.
// PixelDelta arrives in physical pixels; normalize DPI before applying zoom.
pub(crate) fn wheel_steps(delta: winit::event::MouseScrollDelta, scale: f64) -> f32 {
    let value = match delta {
        winit::event::MouseScrollDelta::LineDelta(_, y) => f64::from(y),
        winit::event::MouseScrollDelta::PixelDelta(point) if scale.is_finite() && scale > 0. => {
            point.y / scale / 40.
        }
        winit::event::MouseScrollDelta::PixelDelta(_) => return 0.,
    };
    if value.is_finite() {
        value.clamp(-20., 20.) as f32
    } else {
        0.
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewportCamera {
    pub legacy: bool,
    pub perspective: bool,
    pub target: Vec3,
    pub distance: f32,
    yaw: f32,
    pitch: f32,
}
impl Default for ViewportCamera {
    fn default() -> Self {
        Self {
            legacy: true,
            perspective: false,
            target: Vec3::new(0., 0., 0.5),
            distance: 2.,
            yaw: 0.,
            pitch: 0.,
        }
    }
}
impl ViewportCamera {
    pub(crate) fn valid(&self) -> bool {
        self.target.is_finite()
            && self.target.abs().max_element() <= 1e6
            && self.distance.is_finite()
            && (0.01..=10_000.).contains(&self.distance)
            && self.yaw.is_finite()
            && self.pitch.is_finite()
            && self.pitch.abs() <= 1.5
    }
    pub(crate) fn matrix(&self, size: Vec2) -> Result<Mat4, InvalidSceneCamera> {
        if !self.valid() || !size.is_finite() || size.min_element() <= 0. {
            return Err(InvalidSceneCamera);
        }
        if self.legacy {
            return Ok(Mat4::IDENTITY);
        }
        self.scene_camera(size)?
            .ok_or(InvalidSceneCamera)?
            .view_projection()
    }
    pub(crate) fn scene_camera(
        &self,
        size: Vec2,
    ) -> Result<Option<SceneCamera>, InvalidSceneCamera> {
        if !self.valid() || !size.is_finite() || size.min_element() <= 0. {
            return Err(InvalidSceneCamera);
        }
        if self.legacy {
            return Ok(None);
        }
        let direction = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        let aspect = size.x / size.y;
        let half_height = self.distance * 0.5;
        Ok(Some(SceneCamera {
            eye: self.target + direction * self.distance,
            target: self.target,
            up: Vec3::Y,
            projection: if self.perspective {
                SceneProjection::Perspective {
                    vertical_fov: std::f32::consts::FRAC_PI_3,
                    aspect,
                    near: self.distance * 0.001,
                    far: self.distance * 100.,
                }
            } else {
                SceneProjection::Orthographic {
                    left: -half_height * aspect,
                    right: half_height * aspect,
                    bottom: -half_height,
                    top: half_height,
                    near: 0.,
                    far: self.distance * 100.,
                }
            },
        }))
    }
    /// Fit every corner of finite world bounds while retaining orbit orientation.
    /// Publishes no partial camera change when the bounds exceed camera limits.
    pub(crate) fn fit_bounds(
        &mut self,
        min: glam::DVec3,
        max: glam::DVec3,
        size: Vec2,
    ) -> Result<(), InvalidSceneCamera> {
        self.fit_bounds_in_rect(min, max, size, [0., 0., size.x, size.y])
    }

    /// Fit into a physical-pixel subrectangle, without changing projection or orbit.
    pub(crate) fn fit_bounds_in_rect(
        &mut self,
        min: glam::DVec3,
        max: glam::DVec3,
        size: Vec2,
        rect: [f32; 4],
    ) -> Result<(), InvalidSceneCamera> {
        if !min.is_finite()
            || !max.is_finite()
            || min.cmpgt(max).any()
            || !size.is_finite()
            || size.min_element() <= 0.
        {
            return Err(InvalidSceneCamera);
        }
        if rect.iter().any(|v| !v.is_finite())
            || rect[0] < 0.
            || rect[1] < 0.
            || rect[2] <= 0.
            || rect[3] <= 0.
            || rect[0] + rect[2] > size.x
            || rect[1] + rect[3] > size.y
        {
            return Err(InvalidSceneCamera);
        }
        let cx = 2. * (f64::from(rect[0]) + f64::from(rect[2]) * 0.5) / f64::from(size.x) - 1.;
        let cy = 1. - 2. * (f64::from(rect[1]) + f64::from(rect[3]) * 0.5) / f64::from(size.y);
        let hx = f64::from(rect[2]) / f64::from(size.x) * 0.85;
        let hy = f64::from(rect[3]) / f64::from(size.y) * 0.85;
        let (left, right_edge, bottom, top) = (cx - hx, cx + hx, cy - hy, cy + hy);
        let center = min * 0.5 + max * 0.5;
        let mut candidate = self.clone();
        candidate.target = center.as_vec3();
        candidate.legacy = false;
        let outward = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
        .as_dvec3()
        .normalize();
        let right = (-outward).cross(glam::DVec3::Y).normalize();
        let up = right.cross(-outward);
        let aspect = f64::from(size.x) / f64::from(size.y);
        let slope = f64::from(std::f32::consts::FRAC_PI_3 * 0.5).tan();
        let corners: [glam::DVec3; 8] = std::array::from_fn(|i| {
            glam::DVec3::new(
                if i & 1 == 0 { min.x } else { max.x },
                if i & 2 == 0 { min.y } else { max.y },
                if i & 4 == 0 { min.z } else { max.z },
            )
        });
        let mut distance = 0.01f64.max(center.abs().max_element() * f64::from(f32::EPSILON) * 8.);
        for corner in corners {
            let offset = corner - candidate.target.as_dvec3();
            let z = offset.dot(outward);
            let x = offset.dot(right);
            let y = offset.dot(up);
            distance = distance.max(-z / 99.);
            if self.perspective {
                distance = distance
                    .max((x / (slope * aspect) + right_edge * z) / hx)
                    .max((-x / (slope * aspect) - left * z) / hx)
                    .max((y / slope + top * z) / hy)
                    .max((-y / slope - bottom * z) / hy)
                    .max(z / 0.999);
            } else {
                distance = distance
                    .max(2. * x.abs() / (aspect * hx))
                    .max(2. * y.abs() / hy)
                    .max(z * 1.01);
            }
        }
        candidate.distance = (distance * 1.05) as f32;
        for _ in 0..8 {
            let half_height =
                f64::from(candidate.distance) * if self.perspective { slope } else { 0.5 };
            candidate.target =
                (center - right * (cx * half_height * aspect) - up * (cy * half_height)).as_vec3();
            if !candidate.valid() {
                return Err(InvalidSceneCamera);
            }
            let matrix = candidate.matrix(size)?.as_dmat4();
            if corners.iter().all(|corner| {
                let p = matrix.project_point3(*corner);
                p.is_finite()
                    && (cx - hx / 0.85 * 0.9..=cx + hx / 0.85 * 0.9).contains(&p.x)
                    && (cy - hy / 0.85 * 0.9..=cy + hy / 0.85 * 0.9).contains(&p.y)
                    && (0. ..=1.).contains(&p.z)
            }) {
                *self = candidate;
                return Ok(());
            }
            candidate.distance *= 1.1;
        }
        Err(InvalidSceneCamera)
    }

    pub(crate) fn orbit(&mut self, delta: Vec2) {
        if !delta.is_finite() {
            return;
        }
        self.legacy = false;
        self.yaw -= delta.x * 0.006;
        self.pitch = (self.pitch + delta.y * 0.006).clamp(-1.5, 1.5);
    }
    pub(crate) fn zoom(&mut self, delta: f32) {
        if !delta.is_finite() {
            return;
        }
        self.legacy = false;
        self.distance =
            (self.distance * (-delta.clamp(-20., 20.) * 0.12).exp()).clamp(0.01, 10_000.);
    }
    pub(crate) fn pan(&mut self, delta: Vec2, size: Vec2) -> Result<(), InvalidSceneCamera> {
        let vp = self.matrix(size)?;
        let depth = vp.project_point3(self.target).z;
        let center = size * 0.5;
        self.target +=
            unproject(vp, center, size, depth)? - unproject(vp, center + delta, size, depth)?;
        self.legacy = false;
        Ok(())
    }

    /// Focuses the camera on an axis-aligned bounding box `[min, max]`, positioning
    /// `target` at the AABB center and setting `distance` so the bounding sphere fits
    /// comfortably inside the vertical field of view.
    #[must_use]
    pub fn focus_aabb(&mut self, min: Vec3, max: Vec3, fov_y_radians: f32) -> bool {
        if !min.is_finite()
            || !max.is_finite()
            || min.x > max.x
            || min.y > max.y
            || min.z > max.z
            || !fov_y_radians.is_finite()
            || fov_y_radians <= 0.05
            || fov_y_radians >= std::f32::consts::PI - 0.05
        {
            return false;
        }
        let center = 0.5 * (min + max);
        let radius = (0.5 * (max - min)).length().max(0.1);
        let sin_half = (0.5 * fov_y_radians).sin().max(0.1);
        self.target = center;
        self.distance = (radius / sin_half * 1.15).clamp(0.01, 10_000.0);
        self.legacy = false;
        true
    }
}

pub(crate) fn unproject(
    vp: Mat4,
    cursor: Vec2,
    size: Vec2,
    depth: f32,
) -> Result<Vec3, InvalidSceneCamera> {
    if !cursor.is_finite() || !size.is_finite() || size.min_element() <= 0. || !depth.is_finite() {
        return Err(InvalidSceneCamera);
    }
    let inverse = vp.inverse();
    let ndc = cursor / size * 2. - Vec2::ONE;
    let result = inverse.project_point3(Vec3::new(ndc.x, -ndc.y, depth));
    if inverse.is_finite() && result.is_finite() {
        Ok(result)
    } else {
        Err(InvalidSceneCamera)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn wheel_pixel_units_match_lines_across_dpi_and_event_splitting() {
        use winit::{dpi::PhysicalPosition, event::MouseScrollDelta};
        let line = super::wheel_steps(MouseScrollDelta::LineDelta(0., 1.), 2.);
        for scale in [1., 1.5, 2.] {
            assert_eq!(
                super::wheel_steps(
                    MouseScrollDelta::PixelDelta(PhysicalPosition::new(0., 40. * scale)),
                    scale
                ),
                line
            );
        }
        let mut one = super::ViewportCamera::default();
        let mut split = one.clone();
        one.zoom(line);
        for _ in 0..40 {
            split.zoom(super::wheel_steps(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(0., 2.)),
                2.,
            ));
        }
        assert!((one.distance - split.distance).abs() < 1e-5);
        assert_eq!(
            super::wheel_steps(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(0., f64::NAN)),
                2.
            ),
            0.
        );
        assert_eq!(
            super::wheel_steps(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(0., 40.)),
                0.
            ),
            0.
        );
    }

    use super::*;
    #[test]
    fn both_projections_round_trip_after_orbit_pan_zoom() {
        let size = Vec2::new(800., 600.);
        for perspective in [false, true] {
            let mut camera = ViewportCamera {
                perspective,
                ..Default::default()
            };
            camera.orbit(Vec2::new(75., 30.));
            camera.zoom(2.);
            camera.pan(Vec2::new(20., -15.), size).unwrap();
            let matrix = camera.matrix(size).unwrap();
            let point = camera.target + Vec3::new(0.15, 0.2, -0.1);
            let projected = matrix.project_point3(point);
            let cursor = Vec2::new(projected.x + 1., 1. - projected.y) * size * 0.5;
            assert!(
                unproject(matrix, cursor, size, projected.z)
                    .unwrap()
                    .distance(point)
                    < 0.002
            );
        }
    }
}

#[cfg(test)]
mod fit_tests {
    use super::*;
    #[test]
    fn fitted_bounds_project_inside_both_camera_types_and_aspects() {
        for perspective in [false, true] {
            for size in [
                Vec2::new(800., 600.),
                Vec2::new(300., 1000.),
                Vec2::new(1600., 300.),
            ] {
                for (min, max) in [
                    (
                        glam::DVec3::new(-5., -2., -1.),
                        glam::DVec3::new(5., 2., 1.),
                    ),
                    (glam::DVec3::splat(3.), glam::DVec3::splat(3.)),
                ] {
                    let mut camera = ViewportCamera {
                        perspective,
                        ..Default::default()
                    };
                    camera.orbit(Vec2::new(71., -50.));
                    camera.fit_bounds(min, max, size).unwrap();
                    let matrix = camera.matrix(size).unwrap().as_dmat4();
                    for i in 0..8 {
                        let p = matrix.project_point3(glam::DVec3::new(
                            if i & 1 == 0 { min.x } else { max.x },
                            if i & 2 == 0 { min.y } else { max.y },
                            if i & 4 == 0 { min.z } else { max.z },
                        ));
                        assert!(
                            p.is_finite()
                                && p.x.abs() <= 0.9
                                && p.y.abs() <= 0.9
                                && (0. ..=1.).contains(&p.z)
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn invalid_or_unframeable_bounds_preserve_camera() {
        let mut camera = ViewportCamera::default();
        let before = serde_json::to_value(&camera).unwrap();
        for (min, max, size) in [
            (glam::DVec3::splat(f64::NAN), glam::DVec3::ONE, Vec2::ONE),
            (glam::DVec3::ONE, glam::DVec3::ZERO, Vec2::ONE),
            (glam::DVec3::ZERO, glam::DVec3::ONE, Vec2::ZERO),
            (glam::DVec3::splat(-1e9), glam::DVec3::splat(1e9), Vec2::ONE),
        ] {
            assert!(camera.fit_bounds(min, max, size).is_err());
            assert_eq!(serde_json::to_value(&camera).unwrap(), before);
        }
    }
}

#[cfg(test)]
mod safe_fit_tests {
    use super::*;
    #[test]
    fn fitted_bounds_stay_inside_off_center_safe_rectangles() {
        for perspective in [false, true] {
            for rect in [
                [150., 0., 470., 600.],
                [300., 0., 212., 600.],
                [0., 80., 200., 400.],
            ] {
                let size = Vec2::new(800., 600.);
                let min = glam::DVec3::new(-3., -2., -1.);
                let max = glam::DVec3::new(3., 2., 1.);
                let mut camera = ViewportCamera {
                    perspective,
                    ..Default::default()
                };
                camera.orbit(Vec2::new(75., -45.));
                camera.fit_bounds_in_rect(min, max, size, rect).unwrap();
                let matrix = camera.matrix(size).unwrap().as_dmat4();
                for i in 0..8 {
                    let p = matrix.project_point3(glam::DVec3::new(
                        if i & 1 == 0 { min.x } else { max.x },
                        if i & 2 == 0 { min.y } else { max.y },
                        if i & 4 == 0 { min.z } else { max.z },
                    ));
                    let x = (p.x + 1.) * f64::from(size.x) * 0.5;
                    let y = (1. - p.y) * f64::from(size.y) * 0.5;
                    assert!(x >= f64::from(rect[0]) && x <= f64::from(rect[0] + rect[2]));
                    assert!(y >= f64::from(rect[1]) && y <= f64::from(rect[1] + rect[3]));
                }
            }
        }
    }
}
