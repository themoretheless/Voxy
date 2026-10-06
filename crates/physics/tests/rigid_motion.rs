use physics::{
    astrophysics_spin::Spin,
    contact::{ContactBody, resolve_normal_impact},
    gravity::Body,
    rigid_motion::Error,
    spin_path::Config,
};
fn config() -> Config {
    Config {
        max_angular_error_rad: 1e-4,
        min_step_s: 1e-9,
        max_arcs: 10000,
        max_trials: 30000,
    }
}
fn body() -> ContactBody {
    ContactBody {
        motion: Body {
            mass: 2.,
            position: [1., 2., 3.],
            velocity: [2., -1., 0.],
        },
        spin: Some(Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0., 2.],
            inertia: [1.; 3],
        }),
    }
}

#[test]
fn constant_wrench_closes_momentum_work_and_analytic_spherical_rotation() {
    let initial = body();
    let path = initial
        .prepare_motion([4., 0., 0.], [0., 0., 1.], 0.2, config())
        .unwrap();
    assert_eq!(path.sample(0.).unwrap(), initial);
    assert_eq!(path.sample(path.duration()).unwrap(), path.end());
    for i in 0..=32 {
        let t = 0.2 * i as f64 / 32.;
        let state = path.sample(t).unwrap();
        assert!((state.motion.position[0] - (1. + 2. * t + t * t)).abs() < 1e-14);
        assert!((state.motion.position[1] - (2. - t)).abs() < 1e-14);
        assert!((state.motion.velocity[0] - (2. + 2. * t)).abs() < 1e-14);
        let spin = state.spin.unwrap();
        let angle = 2. * t + 0.5 * t * t;
        let model_bound = path
            .rotation()
            .unwrap()
            .segments()
            .iter()
            .find(|segment| t <= segment.end_s)
            .unwrap()
            .model_angular_error_rad;
        assert!((spin.orientation[2] - (angle / 2.).sin()).abs() <= model_bound + 1e-12);
        let momentum_roundoff =
            8. * f64::EPSILON * (path.rotation().unwrap().segments().len() + 1) as f64 * (2. + t);
        assert!((spin.angular_momentum[2] - (2. + t)).abs() <= momentum_roundoff);
        let evaluated = path.work(t).unwrap();
        assert_eq!(evaluated.force_work, 4. * (state.motion.position[0] - 1.));
        assert!((evaluated.torque_work - angle).abs() <= model_bound + 1e-12);
        assert!(evaluated.energy_residual.abs() <= model_bound + 4. * momentum_roundoff + 1e-12);
        let work = 4. * (state.motion.position[0] - 1.) + angle;
        assert!(
            (state.energy().unwrap() - initial.energy().unwrap() - work).abs()
                <= 4. * momentum_roundoff + 1e-12
        );
    }
}

#[test]
fn off_center_impact_then_free_motion_preserves_total_angular_momentum() {
    let mut a = body();
    a.motion.position = [-1., 1., 0.];
    a.motion.velocity = [3., 0., 0.];
    a.spin = None;
    let mut b = body();
    b.motion.position = [0.; 3];
    b.motion.velocity = [0.; 3];
    b.spin.as_mut().unwrap().angular_momentum = [0.; 3];
    resolve_normal_impact(&mut a, Some(&mut b), [-1., 1., 0.], [-1., 0., 0.], 1.).unwrap();
    let angular = |s: ContactBody| {
        s.motion.mass
            * (s.motion.position[0] * s.motion.velocity[1]
                - s.motion.position[1] * s.motion.velocity[0])
            + s.spin.map_or(0., |r| r.angular_momentum[2])
    };
    let before = angular(a) + angular(b);
    let energy = a.energy().unwrap() + b.energy().unwrap();
    let pa = a.prepare_motion([0.; 3], [0.; 3], 0.1, config()).unwrap();
    let pb = b.prepare_motion([0.; 3], [0.; 3], 0.1, config()).unwrap();
    for i in 0..=16 {
        let t = 0.1 * i as f64 / 16.;
        let a = pa.sample(t).unwrap();
        let b = pb.sample(t).unwrap();
        assert!((angular(a) + angular(b) - before).abs() < 1e-12);
        assert!((a.energy().unwrap() + b.energy().unwrap() - energy).abs() < 1e-12);
    }
}

#[test]
fn preparation_rejects_budget_invalid_torque_and_interior_overflow_without_mutation() {
    let initial = body();
    let mut limited = config();
    limited.max_arcs = 1;
    assert!(
        initial
            .prepare_motion([0.; 3], [0., 0., 1.], 1., limited)
            .is_err()
    );
    assert_eq!(initial, body());
    let mut particle = initial;
    particle.spin = None;
    assert_eq!(
        particle.prepare_motion([0.; 3], [1., 0., 0.], 1., config()),
        Err(Error::InvalidInput)
    );
    // Both endpoints fit; the position parabola exceeds f64 range halfway.
    particle.motion = Body {
        mass: 1e-300,
        position: [1.6e308, 0., 0.],
        velocity: [1e154, 0., 0.],
    };
    assert_eq!(
        particle.prepare_motion([-1e-300, 0., 0.], [0.; 3], 2e154, config()),
        Err(Error::NumericalFailure)
    );
    let path = initial
        .prepare_motion([0.; 3], [0.; 3], 0.1, config())
        .unwrap();
    assert!(path.sample(-1.).is_err());
    assert!(path.sample(f64::NAN).is_err());
    assert!(path.sample(0.1001).is_err());
}

#[test]
fn subnormal_acceleration_survives_long_duration_without_time_squared_overflow() {
    let initial = ContactBody {
        motion: Body {
            mass: 1.,
            position: [0.; 3],
            velocity: [0.; 3],
        },
        spin: None,
    };
    let force = f64::from_bits(1);
    let duration = 1e160;
    let path = initial
        .prepare_motion([force, 0., 0.], [0.; 3], duration, config())
        .unwrap();
    let expected = (force * (0.5 * duration)) * duration;
    assert!(expected > 0.);
    assert_eq!(path.end().motion.position[0], expected);
    assert_eq!(path.end().motion.velocity[0], force * duration);
}

#[test]
fn impact_restarts_both_paths_and_continues_original_wrenches() {
    let mut a = body();
    a.spin = None;
    a.motion.position = [-1., 1., 0.];
    a.motion.velocity = [3., 0., 0.];
    let mut b = body();
    b.motion.position = [0.; 3];
    b.motion.velocity = [0.; 3];
    b.spin.as_mut().unwrap().angular_momentum = [0.; 3];
    let pa = a
        .prepare_motion([2., 0., 0.], [0.; 3], 0.1, config())
        .unwrap();
    let pb = b
        .prepare_motion([-1., 0., 0.], [0., 0., 0.05], 0.1, config())
        .unwrap();
    let time = 0.04;
    let before_a = pa.sample(time).unwrap();
    let before_b = pb.sample(time).unwrap();
    let point = before_a.motion.position;
    let event =
        physics::rigid_motion::prepare_impact(&pa, &pb, time, point, [-1., 0., 0.], 0.7, config())
            .unwrap();
    assert_eq!(
        event.first_remainder.as_ref().unwrap().initial(),
        event.first
    );
    assert_eq!(
        event.second_remainder.as_ref().unwrap().initial(),
        event.second
    );
    let before = before_a.energy().unwrap() + before_b.energy().unwrap();
    assert!(
        (event.first.energy().unwrap()
            + event.second.energy().unwrap()
            + event.impulse.dissipated_energy
            - before)
            .abs()
            < 1e-12
    );
    let end = event.endpoints();
    let remaining = 0.1 - time;
    assert!((end[0].motion.velocity[0] - event.first.motion.velocity[0] - remaining).abs() < 1e-12);
    assert!(
        (end[1].motion.velocity[0] - event.second.motion.velocity[0] + 0.5 * remaining).abs()
            < 1e-12
    );
    assert!(
        (end[1].spin.unwrap().angular_momentum[2]
            - event.second.spin.unwrap().angular_momentum[2]
            - 0.05 * remaining)
            .abs()
            < 1e-12
    );
    for k in 0..3 {
        assert!(
            (end[0].motion.position[k]
                - event.first.motion.position[k]
                - event.first.motion.velocity[k] * remaining
                - if k == 0 {
                    0.5 * remaining * remaining
                } else {
                    0.
                })
            .abs()
                < 1e-12
        );
    }
    assert_eq!(pa.sample(time).unwrap(), before_a);
    assert_eq!(pb.sample(time).unwrap(), before_b);
}

#[test]
fn late_second_remainder_failure_preserves_paths_and_endpoint_impact_needs_no_zero_step() {
    let mut a = body();
    a.spin = None;
    a.motion.velocity = [3., 0., 0.];
    let mut b = body();
    b.motion.velocity = [0.; 3];
    b.spin.as_mut().unwrap().angular_momentum = [0.; 3];
    let pa = a.prepare_motion([0.; 3], [0.; 3], 0.1, config()).unwrap();
    let pb = b.prepare_motion([0.; 3], [0.; 3], 0.1, config()).unwrap();
    let originals = (pa.clone(), pb.clone());
    let mut invalid = config();
    invalid.max_arcs = 0;
    assert!(
        physics::rigid_motion::prepare_impact(
            &pa,
            &pb,
            0.04,
            [1., 3., 3.],
            [-1., 0., 0.],
            0.5,
            invalid
        )
        .is_err()
    );
    assert_eq!((pa.clone(), pb.clone()), originals);
    let event = physics::rigid_motion::prepare_impact(
        &pa,
        &pb,
        0.1,
        [1., 3., 3.],
        [-1., 0., 0.],
        0.5,
        invalid,
    )
    .unwrap();
    assert!(event.first_remainder.is_none() && event.second_remainder.is_none());
    assert_eq!(event.endpoints(), [event.first, event.second]);
    assert!(
        physics::rigid_motion::prepare_impact(
            &pa,
            &pb,
            0.04,
            [1., 3., 3.],
            [1., 0., 0.],
            0.5,
            config()
        )
        .is_err()
    );
}

#[test]
fn endpoint_event_remainder_below_subdivision_floor_is_admitted_with_same_error_gate() {
    let mut a = body();
    a.motion.velocity = [3., 0., 0.];
    let mut b = body();
    b.motion.velocity = [0.; 3];
    let pa = a.prepare_motion([0.; 3], [0.; 3], 0.1, config()).unwrap();
    let pb = b.prepare_motion([0.; 3], [0.; 3], 0.1, config()).unwrap();
    let time = 0.1 - 1e-14;
    let point = pa.sample(time).unwrap().motion.position;
    let event =
        physics::rigid_motion::prepare_impact(&pa, &pb, time, point, [-1., 0., 0.], 0., config())
            .unwrap();
    let remainder = event.second_remainder.unwrap();
    assert!(remainder.duration() < config().min_step_s);
    assert!(
        remainder
            .rotation()
            .unwrap()
            .segments()
            .last()
            .unwrap()
            .model_angular_error_rad
            <= config().max_angular_error_rad
    );
    assert!(remainder.end().energy().unwrap().is_finite());
    let mut invalid = config();
    invalid.min_step_s = f64::NAN;
    assert!(a.prepare_motion([0.; 3], [0.; 3], 1e-14, invalid).is_err());
    // A clipped interval still cannot bypass an impossible error allowance.
    let mut strict = config();
    strict.max_angular_error_rad = f64::MIN_POSITIVE;
    assert!(a.prepare_motion([0.; 3], [0.; 3], 1e-14, strict).is_err());
}
