//! Previous presented transforms for temporal rendering and motion-vector passes.
use glam::Mat4;

/// Renders backward motion in normalized top-left UV coordinates to float RG/RGBA.
/// Reload into a dedicated scene renderer and encode only opaque world geometry.
pub const MOTION_SCENE_SHADER: &str = include_str!("motion.wgsl");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionMatrices {
    pub current: Mat4,
    pub previous: Mat4,
    /// False after reset/first frame: use zero motion and reset temporal accumulation.
    pub history_valid: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MotionHistory {
    presented: Option<Mat4>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidMotionMatrix;
impl std::fmt::Display for InvalidMotionMatrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("motion matrix must contain only finite values")
    }
}
impl std::error::Error for InvalidMotionMatrix {}

impl MotionHistory {
    /// Prepares an object's current and previous unjittered model-view-projection.
    /// Calling this does not advance history. Both camera and object motion belong
    /// in the supplied matrix. Maintain a separate history per object and VR eye.
    /// # Errors
    /// Rejects non-finite matrices without changing the previous presented frame.
    pub fn prepare(&self, current: Mat4) -> Result<MotionMatrices, InvalidMotionMatrix> {
        if !current.is_finite() {
            return Err(InvalidMotionMatrix);
        }
        Ok(MotionMatrices {
            current,
            previous: self.presented.unwrap_or(current),
            history_valid: self.presented.is_some(),
        })
    }

    /// Advance only after this object's frame has actually been presented.
    /// Failed/skipped acquisition or rendering must not advance temporal history.
    /// # Errors
    /// Rejects non-finite matrices while retaining the previous presented frame.
    pub fn presented(&mut self, current: Mat4) -> Result<(), InvalidMotionMatrix> {
        if !current.is_finite() {
            return Err(InvalidMotionMatrix);
        }
        self.presented = Some(current);
        Ok(())
    }

    /// Reset after a camera cut, teleport, resize or graphics-resource recreation.
    pub fn reset(&mut self) {
        self.presented = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn skipped_frames_do_not_advance_and_cuts_reset_motion() {
        let mut history = MotionHistory::default();
        let first = Mat4::IDENTITY;
        let second = Mat4::from_translation(Vec3::X);
        let third = Mat4::from_translation(Vec3::Y);
        let initial = history.prepare(first).unwrap();
        assert!(!initial.history_valid);
        assert_eq!(initial.previous, first);
        history.presented(first).unwrap();
        // A prepared second frame was skipped; third still references first.
        history.prepare(second).unwrap();
        assert_eq!(history.prepare(third).unwrap().previous, first);
        history.presented(third).unwrap();
        assert_eq!(history.prepare(second).unwrap().previous, third);
        history.reset();
        let cut = history.prepare(second).unwrap();
        assert!(!cut.history_valid);
        assert_eq!(cut.previous, second);
    }

    #[test]
    fn invalid_updates_preserve_history() {
        let mut history = MotionHistory::default();
        history.presented(Mat4::IDENTITY).unwrap();
        let invalid = Mat4::from_cols_array(&[f32::NAN; 16]);
        assert!(history.presented(invalid).is_err());
        assert!(history.prepare(invalid).is_err());
        assert_eq!(
            history.prepare(Mat4::IDENTITY).unwrap().previous,
            Mat4::IDENTITY
        );
    }
}
