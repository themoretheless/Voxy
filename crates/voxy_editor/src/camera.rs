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
        _ => return 0.,
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
