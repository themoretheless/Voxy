//! One clock for jump root motion, grounded leg IK and secondary excitation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct JumpSample {
    pub height: f64,
    pub velocity: f64,
    pub acceleration: f64,
    pub compression: f32,
    pub flight: Option<f32>,
    pub limb_weight: f32,
    pub arm_swing: f32,
}

const GRAVITY: f64 = 9.81;
const TAKEOFF: f64 = 0.56;
const FLIGHT: f64 = 0.50;
const LANDING: f64 = TAKEOFF + FLIGHT;

fn ease(t: f64) -> f64 {
    let t = t.clamp(0., 1.);
    t * t * t * (10. + t * (-15. + 6. * t))
}

// Quintic Hermite interpolation with position, velocity and acceleration at
// both boundaries. Derivatives are analytic and use physical seconds.
fn segment(t: f64, duration: f64, from: [f64; 3], to: [f64; 3]) -> [f64; 3] {
    let u = t / duration;
    let c0 = from[0];
    let c1 = from[1] * duration;
    let c2 = from[2] * duration * duration * 0.5;
    let a = to[0] - c0 - c1 - c2;
    let b = to[1] * duration - c1 - 2. * c2;
    let c = to[2] * duration * duration - 2. * c2;
    let c3 = 10. * a - 4. * b + 0.5 * c;
    let c4 = -15. * a + 7. * b - c;
    let c5 = 6. * a - 3. * b + 0.5 * c;
    [
        c0 + u * (c1 + u * (c2 + u * (c3 + u * (c4 + u * c5)))),
        (c1 + u * (2. * c2 + u * (3. * c3 + u * (4. * c4 + u * 5. * c5)))) / duration,
        (2. * c2 + u * (6. * c3 + u * (12. * c4 + u * 20. * c5))) / duration.powi(2),
    ]
}

pub(crate) fn sample(time: f64) -> JumpSample {
    let cycle = time.rem_euclid(6.);
    let t = cycle.rem_euclid(2.);
    let speed = GRAVITY * FLIGHT * 0.5;
    let motion = if cycle >= 4. || t >= 1.65 {
        [0.; 3]
    } else if t < 0.38 {
        segment(t, 0.38, [0.; 3], [-0.12, 0., 0.])
    } else if t < TAKEOFF {
        segment(
            t - 0.38,
            TAKEOFF - 0.38,
            [-0.12, 0., 0.],
            [0., speed, -GRAVITY],
        )
    } else if t < LANDING {
        let flight = t - TAKEOFF;
        [
            speed * flight - 0.5 * GRAVITY * flight * flight,
            speed - GRAVITY * flight,
            -GRAVITY,
        ]
    } else if t < 1.26 {
        segment(
            t - LANDING,
            1.26 - LANDING,
            [0., -speed, -GRAVITY],
            [-0.12, 0., 0.],
        )
    } else {
        segment(t - 1.26, 1.65 - 1.26, [-0.12, 0., 0.], [0.; 3])
    };
    let active = cycle < 4. && t < 1.65;
    let flight =
        (active && (TAKEOFF..LANDING).contains(&t)).then_some(((t - TAKEOFF) / FLIGHT) as f32);
    let limb_weight = if active {
        ease(t / 0.18) * (1. - ease((t - 1.26) / 0.39))
    } else {
        0.
    };
    let arm_swing = if !active {
        0.
    } else if t < 0.38 {
        0.25 * ease(t / 0.38)
    } else if t < TAKEOFF {
        0.25 - 0.85 * ease((t - 0.38) / (TAKEOFF - 0.38))
    } else if let Some(flight) = flight {
        -0.6 + 0.2 * (std::f32::consts::PI * flight).sin().powi(2) as f64
    } else if t < 1.26 {
        -0.6 + 0.75 * ease((t - LANDING) / (1.26 - LANDING))
    } else {
        0.15 * (1. - ease((t - 1.26) / 0.39))
    };
    JumpSample {
        height: motion[0],
        velocity: motion[1],
        acceleration: motion[2],
        compression: ease(-motion[0] / 0.12) as f32,
        flight,
        limb_weight: limb_weight as f32,
        arm_swing: arm_swing as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_flight_landing_are_continuous_and_ballistic() {
        for boundary in [0.38, TAKEOFF, LANDING, 1.26, 1.65, 2., 4., 6.] {
            let a = sample(boundary - 1e-8);
            let b = sample(boundary + 1e-8);
            assert!((a.height - b.height).abs() < 1e-6);
            assert!((a.velocity - b.velocity).abs() < 1e-5);
            assert!((a.acceleration - b.acceleration).abs() < 1e-3);
        }
        for time in [0.12, 0.31, 0.43, 0.68, 0.92, 1.12, 1.4] {
            let dt = 1e-5;
            let p = sample(time);
            let a = sample(time - dt);
            let b = sample(time + dt);
            assert!(((b.height - a.height) / (2. * dt) - p.velocity).abs() < 1e-6);
            assert!(((b.velocity - a.velocity) / (2. * dt) - p.acceleration).abs() < 1e-5);
        }
        assert!(sample(0.38).height < -0.11);
        assert!(sample(TAKEOFF + FLIGHT * 0.5).height > 0.30);
        assert_eq!(sample(0.8).acceleration, -GRAVITY);
        assert!(sample(1.26).height < -0.11);
        for time in [1.8, 4., 5.5] {
            assert_eq!(sample(time).height, 0.);
        }
    }
}
