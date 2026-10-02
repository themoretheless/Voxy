//! Per-eye camera math for XR runtime-provided poses and asymmetric frusta.
//! This module does not create an XR session or own headset presentation images.
use glam::{Mat4, Quat, Vec3, Vec4};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XrFov {
    pub left: f32,
    pub right: f32,
    pub down: f32,
    pub up: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XrView {
    /// World-space eye position in meters, with right-handed -Z forward.
    pub position: Vec3,
    pub orientation: Quat,
    pub fov: XrFov,
    pub position_valid: bool,
    pub orientation_valid: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XrViewError {
    TrackingUnavailable,
    InvalidPose,
    InvalidFrustum,
}
impl std::fmt::Display for XrViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "XR view error: {self:?}")
    }
}
impl std::error::Error for XrViewError {}

impl XrFov {
    /// Asymmetric right-handed projection with 0..1 depth, for per-eye rendering.
    /// # Errors
    /// Rejects non-finite angles, degenerate frusta and invalid clipping planes.
    pub fn projection(self, near: f32, far: f32) -> Result<Mat4, XrViewError> {
        let half_pi = std::f32::consts::FRAC_PI_2;
        if ![self.left, self.right, self.down, self.up]
            .into_iter()
            .all(|a| a.is_finite() && a > -half_pi && a < half_pi)
            || (self.right - self.left).abs() <= f32::EPSILON
            || (self.up - self.down).abs() <= f32::EPSILON
            || !near.is_finite()
            || !far.is_finite()
            || near <= 0.0
            || far <= near
        {
            return Err(XrViewError::InvalidFrustum);
        }
        let (left, right, down, up) = (
            self.left.tan(),
            self.right.tan(),
            self.down.tan(),
            self.up.tan(),
        );
        let width = right - left;
        let height = up - down;
        let projection = Mat4::from_cols(
            Vec4::new(2.0 / width, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 2.0 / height, 0.0, 0.0),
            Vec4::new(
                (right + left) / width,
                (up + down) / height,
                -far / (far - near),
                -1.0,
            ),
            Vec4::new(0.0, 0.0, -far * near / (far - near), 0.0),
        );
        if projection.is_finite() {
            Ok(projection)
        } else {
            Err(XrViewError::InvalidFrustum)
        }
    }
}
impl XrView {
    /// Copies a runtime eye without changing coordinate conventions, normalizing
    /// its quaternion or inventing tracking validity. `VALID` flags allow
    /// runtime-estimated poses even when the corresponding `TRACKED` flag is off.
    /// Use flags returned by the same `locate_views` call as this view.
    /// The reference space must match the world space used for rendering.
    #[cfg(feature = "openxr")]
    #[must_use]
    pub fn from_openxr(view: openxr::View, flags: openxr::ViewStateFlags) -> Self {
        let position = view.pose.position;
        let orientation = view.pose.orientation;
        Self {
            position: Vec3::new(position.x, position.y, position.z),
            orientation: Quat::from_xyzw(
                orientation.x,
                orientation.y,
                orientation.z,
                orientation.w,
            ),
            fov: XrFov {
                left: view.fov.angle_left,
                right: view.fov.angle_right,
                down: view.fov.angle_down,
                up: view.fov.angle_up,
            },
            position_valid: flags.contains(openxr::ViewStateFlags::POSITION_VALID),
            orientation_valid: flags.contains(openxr::ViewStateFlags::ORIENTATION_VALID),
        }
    }
    /// Uses the runtime eye pose directly, without deriving it from a desktop camera.
    /// # Errors
    /// Rejects invalid tracking, pose or projection before GPU upload.
    pub fn view_projection(self, near: f32, far: f32) -> Result<Mat4, XrViewError> {
        if !self.position_valid || !self.orientation_valid {
            return Err(XrViewError::TrackingUnavailable);
        }
        if !self.position.is_finite()
            || !self.orientation.is_finite()
            || (self.orientation.length_squared() - 1.0).abs() > 1e-4
        {
            return Err(XrViewError::InvalidPose);
        }
        let view = Mat4::from_rotation_translation(self.orientation, self.position).inverse();
        let vp = self.fov.projection(near, far)? * view;
        if vp.is_finite() {
            Ok(vp)
        } else {
            Err(XrViewError::InvalidPose)
        }
    }
}

/// Independent left/right camera history, committed as one stereo frame.
/// Matrices are unjittered. Call `presented` only after successful XR projection
/// layer submission; skipped frames must not advance history. Reset on session,
/// reference-space or swapchain changes and teleports. Clipping-plane changes
/// invalidate history automatically.
#[derive(Clone, Copy, Debug, Default)]
pub struct XrMotionHistory {
    presented: Option<([Mat4; 2], [f32; 2])>,
}

impl XrMotionHistory {
    /// Prepares left/right motion without advancing the submitted frame.
    /// Invalid tracking or camera data resets both eyes to prevent stale motion
    /// when tracking resumes.
    /// # Errors
    /// Returns the first invalid eye pose or frustum.
    pub fn prepare(
        &mut self,
        views: [XrView; 2],
        near: f32,
        far: f32,
    ) -> Result<[crate::MotionMatrices; 2], XrViewError> {
        let current = match stereo_matrices(views, near, far) {
            Ok(current) => current,
            Err(error) => {
                self.reset();
                return Err(error);
            }
        };
        if self.presented.is_some_and(|(_, clipping)| {
            clipping.map(f32::to_bits) != [near, far].map(f32::to_bits)
        }) {
            self.reset();
        }
        let previous = self.presented.map_or(current, |(matrices, _)| matrices);
        Ok(std::array::from_fn(|eye| crate::MotionMatrices {
            current: current[eye],
            previous: previous[eye],
            history_valid: self.presented.is_some(),
        }))
    }

    /// Commits the exact views used for a successfully submitted stereo layer.
    /// Validation completes for both eyes before either history changes.
    /// # Errors
    /// Rejects invalid eye data while preserving the last submitted stereo frame.
    pub fn presented(
        &mut self,
        views: [XrView; 2],
        near: f32,
        far: f32,
    ) -> Result<(), XrViewError> {
        self.presented = Some((stereo_matrices(views, near, far)?, [near, far]));
        Ok(())
    }

    /// Invalidates temporal accumulation for both eyes.
    pub fn reset(&mut self) {
        self.presented = None;
    }
}

fn stereo_matrices(views: [XrView; 2], near: f32, far: f32) -> Result<[Mat4; 2], XrViewError> {
    Ok([
        views[0].view_projection(near, far)?,
        views[1].view_projection(near, far)?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fov() -> XrFov {
        XrFov {
            left: -0.7,
            right: 0.9,
            down: -0.6,
            up: 0.8,
        }
    }
    #[cfg(feature = "openxr")]
    #[test]
    fn runtime_conversion_preserves_pose_fov_and_validity() {
        let runtime = openxr::View {
            pose: openxr::Posef {
                position: openxr::Vector3f {
                    x: 1.0,
                    y: 2.0,
                    z: -3.0,
                },
                orientation: openxr::Quaternionf {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 1.0,
                },
            },
            fov: openxr::Fovf {
                angle_left: -0.7,
                angle_right: 0.9,
                angle_down: -0.6,
                angle_up: 0.8,
            },
        };
        let valid =
            openxr::ViewStateFlags::POSITION_VALID | openxr::ViewStateFlags::ORIENTATION_VALID;
        let view = XrView::from_openxr(runtime, valid);
        assert_eq!(view.position, Vec3::new(1.0, 2.0, -3.0));
        assert_eq!(view.orientation, Quat::IDENTITY);
        assert_eq!(view.fov, fov());
        assert!(view.view_projection(0.1, 100.0).is_ok());
        // TRACKED alone does not establish a usable pose.
        let tracked =
            openxr::ViewStateFlags::POSITION_TRACKED | openxr::ViewStateFlags::ORIENTATION_TRACKED;
        assert_eq!(
            XrView::from_openxr(runtime, tracked).view_projection(0.1, 100.0),
            Err(XrViewError::TrackingUnavailable)
        );
        let mut bad = runtime;
        bad.pose.orientation.w = 0.0;
        assert_eq!(
            XrView::from_openxr(bad, valid).view_projection(0.1, 100.0),
            Err(XrViewError::InvalidPose)
        );
    }

    #[test]
    fn stereo_history_is_independent_atomic_and_recovers_after_tracking_loss() {
        let eye = XrView {
            position: Vec3::new(-0.032, 0.0, 0.0),
            orientation: Quat::IDENTITY,
            fov: fov(),
            position_valid: true,
            orientation_valid: true,
        };
        let first = [
            eye,
            XrView {
                position: -eye.position,
                ..eye
            },
        ];
        let mut history = XrMotionHistory::default();
        assert!(
            history
                .prepare(first, 0.1, 100.0)
                .unwrap()
                .iter()
                .all(|m| !m.history_valid)
        );
        history.presented(first, 0.1, 100.0).unwrap();
        let moved = first.map(|view| XrView {
            position: view.position + Vec3::X,
            ..view
        });
        // Preparation of an unsubmitted frame must not change either eye.
        history.prepare(moved, 0.1, 100.0).unwrap();
        let next = history.prepare(first, 0.1, 100.0).unwrap();
        assert_ne!(next[0].previous, next[1].previous);
        for eye in 0..2 {
            assert_eq!(
                next[eye].previous,
                first[eye].view_projection(0.1, 100.0).unwrap()
            );
        }
        let invalid = [
            moved[0],
            XrView {
                position_valid: false,
                ..moved[1]
            },
        ];
        assert_eq!(
            history.presented(invalid, 0.1, 100.0),
            Err(XrViewError::TrackingUnavailable)
        );
        assert_eq!(history.prepare(first, 0.1, 100.0).unwrap(), next);
        assert_eq!(
            history.prepare(invalid, 0.1, 100.0),
            Err(XrViewError::TrackingUnavailable)
        );
        let recovered = history.prepare(moved, 0.1, 100.0).unwrap();
        assert!(
            recovered
                .iter()
                .all(|m| !m.history_valid && m.previous == m.current)
        );
        history.presented(moved, 0.1, 100.0).unwrap();
        assert!(
            history
                .prepare(first, 0.1, 100.0)
                .unwrap()
                .iter()
                .all(|m| m.history_valid)
        );
        let changed_clip = history.prepare(first, 0.2, 100.0).unwrap();
        assert!(
            changed_clip
                .iter()
                .all(|m| !m.history_valid && m.previous == m.current)
        );
        history.presented(first, 0.2, 100.0).unwrap();
        history.reset();
        assert!(
            history
                .prepare(first, 0.1, 100.0)
                .unwrap()
                .iter()
                .all(|m| !m.history_valid)
        );
    }
    #[test]
    fn asymmetric_frustum_maps_all_edges_and_depth() {
        let fov = fov();
        let projection = fov.projection(0.1, 100.0).unwrap();
        for (point, axis, expected) in [
            (Vec3::new(fov.left.tan(), 0.0, -1.0), 0, -1.0),
            (Vec3::new(fov.right.tan(), 0.0, -1.0), 0, 1.0),
            (Vec3::new(0.0, fov.down.tan(), -1.0), 1, -1.0),
            (Vec3::new(0.0, fov.up.tan(), -1.0), 1, 1.0),
            (Vec3::new(0.0, 0.0, -0.1), 2, 0.0),
            (Vec3::new(0.0, 0.0, -100.0), 2, 1.0),
        ] {
            assert!((projection.project_point3(point)[axis] - expected).abs() < 1e-5);
        }
    }
    #[test]
    fn per_eye_positions_produce_distinct_views_and_tracking_is_required() {
        let eye = XrView {
            position: Vec3::new(-0.032, 0.0, 0.0),
            orientation: Quat::IDENTITY,
            fov: fov(),
            position_valid: true,
            orientation_valid: true,
        };
        let left = eye.view_projection(0.1, 100.0).unwrap();
        let right = XrView {
            position: Vec3::new(0.032, 0.0, 0.0),
            ..eye
        }
        .view_projection(0.1, 100.0)
        .unwrap();
        let point = Vec3::new(0.0, 0.0, -2.0);
        assert!(left.project_point3(point).x > right.project_point3(point).x);
        assert_eq!(
            XrView {
                position_valid: false,
                ..eye
            }
            .view_projection(0.1, 100.0),
            Err(XrViewError::TrackingUnavailable)
        );
        assert_eq!(
            XrView {
                orientation: Quat::from_xyzw(0.0, 0.0, 0.0, 0.0),
                ..eye
            }
            .view_projection(0.1, 100.0),
            Err(XrViewError::InvalidPose)
        );
    }
    #[test]
    fn mirrored_runtime_frusta_preserve_edge_mapping() {
        let normal = fov();
        let flipped = XrFov {
            left: normal.right,
            right: normal.left,
            down: normal.up,
            up: normal.down,
        };
        let p = flipped.projection(0.1, 100.0).unwrap();
        assert!((p.project_point3(Vec3::new(flipped.left.tan(), 0.0, -1.0)).x + 1.0).abs() < 1e-5);
        assert!((p.project_point3(Vec3::new(0.0, flipped.up.tan(), -1.0)).y - 1.0).abs() < 1e-5);
        assert_eq!(
            XrFov {
                right: normal.left,
                ..normal
            }
            .projection(0.1, 100.0),
            Err(XrViewError::InvalidFrustum)
        );
    }
}
