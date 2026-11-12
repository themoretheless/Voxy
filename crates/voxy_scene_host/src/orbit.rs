//! The orbit camera state shared by the shell rigs and the demo viewers.
//!
//! Input arrives as plain numbers: the caller owns the device, the sign
//! convention and the sensitivity, and passes radians and distance units.
//! Smoothing stays with the caller as well, because the rigs differ in kind
//! (a first-order lag in the walk camera, a critically damped spring in the
//! character viewer); a smoothed rig keeps one controller for the goal and
//! one for the presented view.
use glam::Vec3;

/// Clamp ranges of one orbit. Every viewer keeps its own values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitLimits {
    /// Lowest and highest pitch in radians.
    pub pitch: (f32, f32),
    /// Nearest and farthest distance from the target.
    pub distance: (f32, f32),
}

impl OrbitLimits {
    /// Limits with a pitch range symmetric about the horizon.
    #[must_use]
    pub const fn symmetric(pitch: f32, nearest: f32, farthest: f32) -> Self {
        Self {
            pitch: (-pitch, pitch),
            distance: (nearest, farthest),
        }
    }

    /// `pitch` moved into the allowed range.
    #[must_use]
    pub fn clamp_pitch(&self, pitch: f32) -> f32 {
        pitch.clamp(self.pitch.0, self.pitch.1)
    }

    /// `distance` moved into the allowed range.
    #[must_use]
    pub fn clamp_distance(&self, distance: f32) -> f32 {
        distance.clamp(self.distance.0, self.distance.1)
    }
}

/// Yaw, pitch and distance about a target point, Y up.
///
/// `direction` is the unit vector the angles describe. A viewer that looks at
/// the target from outside places the eye at `target + direction * distance`
/// ([`OrbitController::eye`]); a follow rig whose angles are the view
/// direction places it at `target - direction * distance`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitController {
    /// Rotation about the vertical axis in radians; unbounded.
    pub yaw: f32,
    /// Elevation above the horizon in radians.
    pub pitch: f32,
    /// Distance from the target.
    pub distance: f32,
    /// The point the orbit turns about.
    pub target: Vec3,
}

impl OrbitController {
    /// An orbit about the origin.
    #[must_use]
    pub const fn new(yaw: f32, pitch: f32, distance: f32) -> Self {
        Self {
            yaw,
            pitch,
            distance,
            target: Vec3::ZERO,
        }
    }

    /// Turns by the given angles; pitch stops at the limits, yaw is free.
    pub fn drag(&mut self, yaw_delta: f32, pitch_delta: f32, limits: &OrbitLimits) {
        self.yaw += yaw_delta;
        self.pitch = limits.clamp_pitch(self.pitch + pitch_delta);
    }

    /// Moves along the view axis; the distance stops at the limits.
    pub fn zoom(&mut self, distance_delta: f32, limits: &OrbitLimits) {
        self.distance = limits.clamp_distance(self.distance + distance_delta);
    }

    /// Unit vector of the current yaw and pitch.
    #[must_use]
    pub fn direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
    }

    /// Eye position of a viewer looking at the target from outside.
    #[must_use]
    pub fn eye(&self) -> Vec3 {
        self.target
            + Vec3::new(
                self.yaw.sin() * self.pitch.cos() * self.distance,
                self.pitch.sin() * self.distance,
                self.yaw.cos() * self.pitch.cos() * self.distance,
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: OrbitLimits = OrbitLimits::symmetric(1.2, 0.7, 5.0);

    #[test]
    fn drag_adds_the_angles_and_leaves_yaw_unbounded() {
        let mut orbit = OrbitController::new(0.15, 0.0, 2.4);
        orbit.drag(0.5, 0.25, &LIMITS);
        assert_eq!((orbit.yaw, orbit.pitch), (0.15 + 0.5, 0.25));
        orbit.drag(100.0, 0.0, &LIMITS);
        assert_eq!(orbit.yaw, 0.15 + 0.5 + 100.0);
        assert_eq!(orbit.distance, 2.4);
    }

    #[test]
    fn pitch_stops_at_both_limits() {
        let mut orbit = OrbitController::new(0.0, 0.0, 2.4);
        orbit.drag(0.0, 10.0, &LIMITS);
        assert_eq!(orbit.pitch, 1.2);
        orbit.drag(0.0, -10.0, &LIMITS);
        assert_eq!(orbit.pitch, -1.2);
        let below_horizon = OrbitLimits {
            pitch: (-75_f32.to_radians(), -5_f32.to_radians()),
            distance: (4.0, 36.0),
        };
        orbit.drag(0.0, 10.0, &below_horizon);
        assert_eq!(orbit.pitch, -5_f32.to_radians());
    }

    #[test]
    fn zoom_stops_at_both_limits() {
        let mut orbit = OrbitController::new(0.0, 0.0, 2.4);
        orbit.zoom(-0.4, &LIMITS);
        assert_eq!(orbit.distance, 2.4 - 0.4);
        orbit.zoom(-100.0, &LIMITS);
        assert_eq!(orbit.distance, 0.7);
        orbit.zoom(100.0, &LIMITS);
        assert_eq!(orbit.distance, 5.0);
    }

    #[test]
    fn eye_is_the_target_plus_the_scaled_direction() {
        let mut orbit = OrbitController::new(0.7, 0.3, 2.0);
        orbit.target = Vec3::new(1.0, 2.0, 3.0);
        let expected = orbit.target + orbit.direction() * 2.0;
        assert!(orbit.eye().distance(expected) < 1e-6);
        assert!((orbit.direction().length() - 1.0).abs() < 1e-6);
        let ahead = OrbitController::new(0.0, 0.0, 1.0);
        assert_eq!(ahead.direction(), Vec3::Z);
    }
}
