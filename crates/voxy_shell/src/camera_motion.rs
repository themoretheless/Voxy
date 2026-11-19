//! Frame-rate independent decay of a released orbit gesture.

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
    fn released_orbit_velocity_decays_and_stops_below_the_threshold() {
        let v = inertial_orbit_velocity_decay([2.0, 1.0], 5.0, 0.01, 0.1);
        assert!(v[0] > 0.0 && v[0] < 2.0);
        assert_eq!(
            inertial_orbit_velocity_decay([0.005, 0.0], 5.0, 0.01, 0.1),
            [0.0, 0.0]
        );
    }
}
