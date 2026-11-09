//! Frame-rate independent camera response; input changes a target, never the view directly.

use voxy_demos::orbit::{ORBIT_MAX_DISTANCE, ORBIT_PITCH_LIMIT, zoom_range};
#[derive(Debug, Default)]
pub(crate) struct CameraMotion {
    state: Option<Motion>,
    pub(crate) held: [bool; 4],
}
#[derive(Debug)]
struct Motion {
    target: [f32; 3],
    velocity: [f32; 3],
    last: [f32; 3],
}
impl CameraMotion {
    fn state(&mut self, current: [f32; 3]) -> &mut Motion {
        if self.state.as_ref().is_none_or(|s| s.last != current) {
            self.state = Some(Motion {
                target: current,
                velocity: [0.; 3],
                last: current,
            });
        }
        self.state.as_mut().unwrap()
    }
    pub(crate) fn orbit(&mut self, current: [f32; 3], x: f32, y: f32) {
        let s = self.state(current);
        s.target[0] += x;
        s.target[1] = (s.target[1] + y).clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);
    }
    pub(crate) fn zoom(&mut self, current: [f32; 3], amount: f32, hand: bool) {
        let s = self.state(current);
        let (minimum, scale) = zoom_range(hand);
        s.target[2] = (s.target[2] + amount * scale).clamp(minimum, ORBIT_MAX_DISTANCE);
    }
    pub(crate) fn advance(&mut self, current: [f32; 3], dt: f32, hand: bool) -> [f32; 3] {
        // A suspension must not turn held input into a large camera jump.
        let dt = dt.clamp(0., 0.05);
        let turn = i32::from(self.held[1]) - i32::from(self.held[0]);
        let zoom = i32::from(self.held[3]) - i32::from(self.held[2]);
        if turn != 0 {
            self.orbit(current, turn as f32 * 1.5 * dt, 0.);
        }
        if zoom != 0 {
            self.zoom(current, zoom as f32 * 1.2 * dt, hand);
        }
        let s = self.state(current);
        // Exact critically damped spring for a constant target, with no Euler instability.
        let omega = 22.;
        let decay = (-omega * dt).exp();
        let mut out = current;
        for k in 0..3 {
            let error = current[k] - s.target[k];
            let c = s.velocity[k] + omega * error;
            out[k] = s.target[k] + (error + c * dt) * decay;
            s.velocity[k] = (s.velocity[k] - omega * c * dt) * decay;
        }
        s.last = out;
        out
    }
}

/// Applies frame-rate independent exponential drag and a stop threshold to a 2D orbital
/// angular velocity `[yaw_vel, pitch_vel]` after pointer/touch release.
#[must_use]
pub fn inertial_orbit_velocity_decay(
    velocity_rad_s: [f32; 2],
    damping_rate_inv_s: f32,
    stop_epsilon_rad_s: f32,
    dt_s: f32,
) -> [f32; 2] {
    if !velocity_rad_s[0].is_finite()
        || !velocity_rad_s[1].is_finite()
        || !damping_rate_inv_s.is_finite()
        || damping_rate_inv_s < 0.0
        || !stop_epsilon_rad_s.is_finite()
        || stop_epsilon_rad_s < 0.0
        || !dt_s.is_finite()
        || dt_s <= 0.0
    {
        return [0.0, 0.0];
    }
    let decay = (-damping_rate_inv_s * dt_s.clamp(0.0, 1.0)).exp();
    let vx = velocity_rad_s[0] * decay;
    let vy = velocity_rad_s[1] * decay;
    if vx.hypot(vy) <= stop_epsilon_rad_s {
        [0.0, 0.0]
    } else {
        [vx, vy]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn camera_response_is_continuous_monotone_and_frame_rate_independent() {
        let run = |fps: usize| {
            let mut c = CameraMotion::default();
            let mut p = [0., 0., 2.4];
            c.orbit(p, 1., 0.5);
            c.zoom(p, -0.8, false);
            assert_eq!(p, [0., 0., 2.4]);
            for _ in 0..fps {
                let next = c.advance(p, 1. / fps as f32, false);
                assert!(next[0] >= p[0] && next[0] <= 1.000001);
                assert!(next[2] <= p[2] && next[2] >= 1.599999);
                p = next;
            }
            p
        };
        let a = run(30);
        for fps in [60, 120, 240] {
            let b = run(fps);
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() < 2e-6);
            }
        }
    }
    #[test]
    fn held_input_stops_on_release_and_external_view_change_resynchronizes() {
        let mut c = CameraMotion::default();
        c.held[1] = true;
        let mut p = [0., 0., 2.4];
        for _ in 0..120 {
            p = c.advance(p, 1. / 120., false);
        }
        assert!(p[0] > 1.2);
        c.held = [false; 4];
        for _ in 0..120 {
            p = c.advance(p, 1. / 120., false);
        }
        assert!((p[0] - 1.5).abs() < 1e-4);
        let changed = [0.7, 0., 0.32];
        assert_eq!(c.advance(changed, 1. / 120., true), changed);
        c.zoom(changed, -100., true);
        let p = c.advance(changed, 0.02, true);
        assert!(p[2] > 0.18 && p[2] < 0.32);
        let v = inertial_orbit_velocity_decay([2.0, 1.0], 5.0, 0.01, 0.1);
        assert!(v[0] > 0.0 && v[0] < 2.0);
        assert_eq!(inertial_orbit_velocity_decay([0.005, 0.0], 5.0, 0.01, 0.1), [0.0, 0.0]);
    }
}
