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
fn decomposed_wrenches_use_one_actual_path_and_preserve_signed_work() {
    let path = body()
        .prepare_motion([4., 0., 0.], [0., 0., 1.], 0.2, config())
        .unwrap();
    for i in 0..=32 {
        let t = path.duration() * i as f64 / 32.;
        let a = path.wrench_work(t, [8., -3., 0.], [0., 0., 3.]).unwrap();
        let b = path.wrench_work(t, [-4., 3., 0.], [0., 0., -2.]).unwrap();
        let total = path.work(t).unwrap();
        assert!((a.0 + b.0 - total.force_work).abs() < 1e-13);
        assert!((a.1 + b.1 - total.torque_work).abs() < 1e-13);
        let dx = path.sample(t).unwrap().motion.position[0] - body().motion.position[0];
        assert!((a.0 - (8. * dx + 3. * t)).abs() < 1e-13);
        if t > 0. {
            assert!(b.0 < 0. && b.1 < 0.);
        }
    }
    let mut point = body();
    point.spin = None;
    let point = point
        .prepare_motion([0.; 3], [0.; 3], 0.2, config())
        .unwrap();
    assert_eq!(
        point.wrench_work(0.1, [0.; 3], [1., 0., 0.]),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        path.wrench_work(0.1, [f64::NAN, 0., 0.], [0.; 3]),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        path.wrench_work(0.3, [0.; 3], [0.; 3]),
        Err(Error::InvalidInput)
    );
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

fn vector_add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] + b[k])
}
fn vector_scale(a: [f64; 3], s: f64) -> [f64; 3] {
    a.map(|x| x * s)
}
fn vector_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3])
}
fn assert_vector_close(a: [f64; 3], b: [f64; 3], tolerance: f64) {
    for k in 0..3 {
        assert!((a[k] - b[k]).abs() < tolerance, "{a:?} != {b:?}");
    }
}
#[test]
fn angular_impulse_integrates_orbital_motion_and_preserves_frame_and_wrench_balance() {
    let mut initial = body();
    initial.motion.position = [2., -3., 5.];
    initial.motion.velocity = [1., 4., -2.];
    let force = [6., -2., 4.];
    let torque = [1., 2., -3.];
    let path = initial
        .prepare_motion(force, torque, 0.2, config())
        .unwrap();
    let momentum = |b: ContactBody| {
        vector_add(
            vector_cross(
                b.motion.position,
                vector_scale(b.motion.velocity, b.motion.mass),
            ),
            b.spin.unwrap().angular_momentum,
        )
    };
    let probe_force = [-3., 5., 2.];
    let probe_torque = [4., -2., 1.];
    let shift = [7., -11., 13.];
    let mut moved = initial;
    moved.motion.position = vector_add(initial.motion.position, shift);
    let moved_path = moved.prepare_motion(force, torque, 0.2, config()).unwrap();
    for i in 0..=32 {
        let time = 0.2 * i as f64 / 32.;
        let actual = path.wrench_angular_impulse(time, force, torque).unwrap();
        assert_vector_close(
            actual,
            vector_add(
                momentum(path.sample(time).unwrap()),
                vector_scale(momentum(initial), -1.),
            ),
            2e-13,
        );
        // Simpson independently integrates this quadratic orbital integrand.
        let rate = |t| {
            vector_add(
                vector_cross(path.sample(t).unwrap().motion.position, probe_force),
                probe_torque,
            )
        };
        let expected = vector_scale(
            vector_add(
                vector_add(rate(0.), vector_scale(rate(time * 0.5), 4.)),
                rate(time),
            ),
            time / 6.,
        );
        let probe = path
            .wrench_angular_impulse(time, probe_force, probe_torque)
            .unwrap();
        assert_vector_close(probe, expected, 2e-14);
        let rest = path
            .wrench_angular_impulse(
                time,
                vector_add(force, vector_scale(probe_force, -1.)),
                vector_add(torque, vector_scale(probe_torque, -1.)),
            )
            .unwrap();
        assert_vector_close(vector_add(probe, rest), actual, 2e-14);
        let translated = moved_path
            .wrench_angular_impulse(time, probe_force, probe_torque)
            .unwrap();
        assert_vector_close(
            translated,
            vector_add(probe, vector_scale(vector_cross(shift, probe_force), time)),
            5e-14,
        );
    }
    assert!(path.wrench_angular_impulse(-1., force, torque).is_err());
    assert!(path.wrench_angular_impulse(0.3, force, torque).is_err());
    assert!(
        path.wrench_angular_impulse(0.1, [f64::NAN; 3], torque)
            .is_err()
    );
}
#[test]
fn frozen_reciprocal_arms_expose_sliding_couple_instead_of_hiding_it() {
    let mut first = body();
    first.spin = None;
    first.motion.position = [0., 1., 0.];
    first.motion.velocity = [2., 0., 0.];
    let mut second = first;
    second.motion.position = [0.; 3];
    second.motion.velocity = [0.; 3];
    let a = first
        .prepare_motion([0.; 3], [0.; 3], 0.2, config())
        .unwrap();
    let b = second
        .prepare_motion([0.; 3], [0.; 3], 0.2, config())
        .unwrap();
    let residual = vector_add(
        a.wrench_angular_impulse(0.2, [0., 10., 0.], [0.; 3])
            .unwrap(),
        b.wrench_angular_impulse(0.2, [0., -10., 0.], [0.; 3])
            .unwrap(),
    );
    // Integral of relative travel 2t crossed with 10 N is 0.4 N m s.
    assert_vector_close(residual, [0., 0., 0.4], 1e-14);
    // A common moving world point needs an evolving second COM torque -20t.
    assert!((residual[2] - 20. * 0.2_f64.powi(2) / 2.).abs() < 1e-14);
}

#[test]
fn moving_reciprocal_application_preserves_angular_balance_and_uses_same_path_work() {
    use physics::astrophysics_spin::TorquePolynomial;
    let mut first = body();
    first.motion.position = [0., 1., 0.];
    first.motion.velocity = [2., 0., 0.];
    first.spin.as_mut().unwrap().angular_momentum = [0.; 3];
    let mut second = first;
    second.motion.position = [0.; 3];
    second.motion.velocity = [0.; 3];
    let up = [0., 10., 0.];
    let down = [0., -10., 0.];
    let first_law = TorquePolynomial::moving_arm([0., -0.5, 0.], [0.; 3], [0.; 3], up).unwrap();
    let second_law =
        TorquePolynomial::moving_arm([0., 0.5, 0.], [2., 0., 0.], [0.; 3], down).unwrap();
    // External COM loads cancel the reciprocal forces; keep their torque and
    // moving application point. This fixture qualifies wrench evolution only.
    let a = first
        .prepare_motion_with_torque([0.; 3], first_law, 0.2, config())
        .unwrap();
    let b = second
        .prepare_motion_with_torque([0.; 3], second_law, 0.2, config())
        .unwrap();
    for i in 0..=32 {
        let t = 0.2 * i as f64 / 32.;
        let impulse_a = a
            .polynomial_wrench_angular_impulse(t, up, first_law)
            .unwrap();
        let impulse_b = b
            .polynomial_wrench_angular_impulse(t, down, second_law)
            .unwrap();
        assert_vector_close(vector_add(impulse_a, impulse_b), [0.; 3], 2e-14);
        assert!(
            (b.sample(t).unwrap().spin.unwrap().angular_momentum[2] + 10. * t * t).abs() < 1e-13
        );
        let actual = b.work(t).unwrap();
        let probe = b.polynomial_wrench_work(t, down, second_law).unwrap();
        assert!((actual.force_work + actual.torque_work - probe.0 - probe.1).abs() < 1e-14);
        assert!(actual.energy_residual.abs() < 1e-5);
    }
    // Polynomial acceleration is essential for a quadratically moving arm.
    let accelerated =
        TorquePolynomial::moving_arm([0.; 3], [2., 0., 0.], [6., 0., 0.], down).unwrap();
    assert_vector_close(accelerated.impulse(0.2).unwrap(), [0., 0., -0.48], 1e-14);
}

#[test]
fn impact_remainder_rebases_polynomial_torque_at_actual_event_time() {
    use physics::astrophysics_spin::TorquePolynomial;
    let mut a = body();
    a.motion.velocity = [3., 0., 0.];
    let mut b = body();
    b.motion.velocity = [0.; 3];
    let law = TorquePolynomial {
        value: [0., 0., 1.],
        rate: [0., 0., 3.],
        acceleration: [0., 0., 4.],
        ..TorquePolynomial::constant([0.; 3])
    };
    let pa = a
        .prepare_motion_with_torque([0.; 3], law, 0.1, config())
        .unwrap();
    let pb = b
        .prepare_motion_with_torque([0.; 3], law, 0.1, config())
        .unwrap();
    let originals = (pa.clone(), pb.clone());
    let event = physics::rigid_motion::prepare_impact(
        &pa,
        &pb,
        0.04,
        [1., 2., 3.],
        [-1., 0., 0.],
        0.5,
        config(),
    )
    .unwrap();
    let end_impulse = law.impulse(0.1).unwrap();
    let event_impulse = law.impulse(0.04).unwrap();
    for (state, path) in [
        (event.first, event.first_remainder.unwrap()),
        (event.second, event.second_remainder.unwrap()),
    ] {
        let expected = vector_add(
            state.spin.unwrap().angular_momentum,
            vector_add(end_impulse, vector_scale(event_impulse, -1.)),
        );
        assert_vector_close(path.end().spin.unwrap().angular_momentum, expected, 1e-13);
    }
    assert_eq!((pa, pb), originals);
}

#[test]
fn affine_force_closes_cubic_com_work_momentum_and_independent_prefix_integrals() {
    use physics::astrophysics_spin::TorquePolynomial;
    let initial = body();
    let force = [4., -2., 6.];
    let rate = [-3., 5., 2.];
    let zero = TorquePolynomial::constant([0.; 3]);
    let path = initial
        .prepare_affine_motion(force, rate, zero, 0.2, config())
        .unwrap();
    assert!(!path.has_constant_acceleration());
    let momentum = |b: ContactBody| {
        vector_add(
            vector_cross(
                b.motion.position,
                vector_scale(b.motion.velocity, b.motion.mass),
            ),
            b.spin.unwrap().angular_momentum,
        )
    };
    for i in 0..=32 {
        let t = 0.2 * i as f64 / 32.;
        let state = path.sample(t).unwrap();
        for k in 0..3 {
            let expected = initial.motion.position[k]
                + initial.motion.velocity[k] * t
                + force[k] / initial.motion.mass * t * t / 2.
                + rate[k] / initial.motion.mass * t * t * t / 6.;
            assert!((state.motion.position[k] - expected).abs() < 2e-14);
            let impulse = force[k] * t + rate[k] * t * t / 2.;
            assert!(
                (initial.motion.mass * (state.motion.velocity[k] - initial.motion.velocity[k])
                    - impulse)
                    .abs()
                    < 2e-14
            );
        }
        let work = path.work(t).unwrap();
        assert!(work.energy_residual.abs() < 2e-13);
        let angular = path
            .affine_wrench_angular_impulse(t, force, rate, zero)
            .unwrap();
        assert_vector_close(
            angular,
            vector_add(momentum(state), vector_scale(momentum(initial), -1.)),
            3e-13,
        );
        // Three-node Gaussian quadrature independently integrates the quartic
        // orbital rate and cubic power from sampled states.
        let node = (3_f64 / 5.).sqrt();
        let mut oracle = [0.; 3];
        let mut power = 0.;
        for (x, w) in [(-node, 5. / 9.), (0., 8. / 9.), (node, 5. / 9.)] {
            let time = t * (x + 1.) / 2.;
            let sample = path.sample(time).unwrap();
            let f = vector_add(force, vector_scale(rate, time));
            oracle = vector_add(
                oracle,
                vector_scale(vector_cross(sample.motion.position, f), w * t / 2.),
            );
            power += (0..3)
                .map(|k| f[k] * sample.motion.velocity[k])
                .sum::<f64>()
                * w
                * t
                / 2.;
        }
        assert_vector_close(angular, oracle, 5e-14);
        assert!((work.force_work - power).abs() < 1e-13);
    }
    assert_eq!(path.initial(), initial);
}

#[test]
fn affine_force_impact_remainder_rebases_load_and_preserves_paths_on_failure() {
    use physics::astrophysics_spin::TorquePolynomial;
    let mut a = body();
    a.motion.velocity = [3., 0., 0.];
    let mut b = body();
    b.motion.velocity = [0.; 3];
    let zero = TorquePolynomial::constant([0.; 3]);
    let rate = [2., 0., 0.];
    let pa = a
        .prepare_affine_motion([1., 0., 0.], rate, zero, 0.1, config())
        .unwrap();
    let pb = b
        .prepare_affine_motion([1., 0., 0.], rate, zero, 0.1, config())
        .unwrap();
    let originals = (pa.clone(), pb.clone());
    let event = physics::rigid_motion::prepare_impact(
        &pa,
        &pb,
        0.04,
        [1., 2., 3.],
        [-1., 0., 0.],
        0.5,
        config(),
    )
    .unwrap();
    for (state, remainder) in [
        (event.first, event.first_remainder.unwrap()),
        (event.second, event.second_remainder.unwrap()),
    ] {
        let force_at_event = 1. + 2. * 0.04;
        let impulse = force_at_event * 0.06 + 2. * 0.06 * 0.06 / 2.;
        assert!(
            (remainder.end().motion.velocity[0]
                - state.motion.velocity[0]
                - impulse / state.motion.mass)
                .abs()
                < 1e-13
        );
        assert_eq!(remainder.jerk(), pa.jerk());
    }
    let mut invalid = config();
    invalid.max_arcs = 0;
    assert!(
        physics::rigid_motion::prepare_impact(
            &pa,
            &pb,
            0.04,
            [1., 2., 3.],
            [-1., 0., 0.],
            0.5,
            invalid
        )
        .is_err()
    );
    assert_eq!((pa, pb), originals);
}

#[test]
fn affine_force_admission_rejects_nonfinite_laws_and_interior_energy_overflow() {
    use physics::astrophysics_spin::TorquePolynomial;
    let mut initial = body();
    initial.spin = None;
    let zero = TorquePolynomial::constant([0.; 3]);
    let before = initial;
    assert_eq!(
        initial.prepare_affine_motion([0.; 3], [f64::NAN; 3], zero, 0.1, config()),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        initial.prepare_affine_motion([0.; 3], [1e308, 0., 0.], zero, 2., config()),
        Err(Error::NumericalFailure)
    );
    // Endpoints have low speed; the quadratic velocity has an unrepresentable
    // kinetic-energy peak. The complete control hull rejects before publication.
    initial.motion.mass = 1.;
    initial.motion.position = [0.; 3];
    initial.motion.velocity = [0.; 3];
    assert_eq!(
        initial.prepare_affine_motion([1e155, 0., 0.], [-2e155, 0., 0.], zero, 1., config()),
        Err(Error::NumericalFailure)
    );
    assert_eq!(before, body_without_spin());
}
fn body_without_spin() -> ContactBody {
    let mut b = body();
    b.spin = None;
    b
}

#[test]
fn cubic_velocity_hull_covers_hidden_speed_peak_and_clipped_intervals() {
    use physics::astrophysics_spin::TorquePolynomial;
    let mut initial = body();
    initial.spin = None;
    initial.motion.velocity = [0.; 3];
    let path = initial
        .prepare_affine_motion(
            [36., 0., 0.],
            [-72., 0., 0.],
            TorquePolynomial::constant([0.; 3]),
            1.,
            config(),
        )
        .unwrap();
    assert_eq!(path.initial().motion.velocity[0], 0.);
    assert_eq!(path.end().motion.velocity[0], 0.);
    assert_eq!(path.sample(0.5).unwrap().motion.velocity[0], 4.5);
    for (a, b) in [(0., 1.), (0.1, 0.7), (0.6, 1.)] {
        let controls = path.velocity_controls(a, b).unwrap();
        for i in 0..=64 {
            let t = a + (b - a) * i as f64 / 64.;
            let v = path.sample(t).unwrap().motion.velocity;
            for k in 0..3 {
                let lo = controls.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
                let hi = controls
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max);
                assert!(v[k] >= lo - 1e-13 && v[k] <= hi + 1e-13);
            }
        }
    }
    assert!(path.velocity_controls(0.7, 0.1).is_err());
    assert!(path.velocity_controls(0., 1.1).is_err());
    assert!(path.acceleration_at(f64::NAN).is_err());
}

#[test]
fn material_point_kinematics_follow_cubic_com_and_analytic_spin_without_mutation() {
    use physics::astrophysics_spin::TorquePolynomial;
    let initial = body();
    let force = [4., -2., 6.];
    let rate = [-3., 5., 2.];
    let local = [0.2, -0.1, 0.3];
    let path = initial
        .prepare_affine_motion(
            force,
            rate,
            TorquePolynomial::constant([0.; 3]),
            0.2,
            config(),
        )
        .unwrap();
    let saved = path.clone();
    for i in 0..=64 {
        let t = 0.2 * i as f64 / 64.;
        let (sin, cos) = (2. * t).sin_cos();
        let arm = [
            cos * local[0] - sin * local[1],
            sin * local[0] + cos * local[1],
            local[2],
        ];
        let sample = path.sample_material_point(t, local).unwrap();
        let position = std::array::from_fn(|k| {
            initial.motion.position[k]
                + initial.motion.velocity[k] * t
                + force[k] / 2. * t * t / 2.
                + rate[k] / 2. * t * t * t / 6.
                + arm[k]
        });
        let tangent = [-2. * arm[1], 2. * arm[0], 0.];
        let centrifugal = [-4. * arm[0], -4. * arm[1], 0.];
        let jerk = [8. * arm[1], -8. * arm[0], 0.];
        assert_vector_close(sample.position, position, 1e-12);
        assert_vector_close(
            sample.velocity,
            std::array::from_fn(|k| {
                initial.motion.velocity[k]
                    + force[k] / 2. * t
                    + rate[k] / 2. * t * t / 2.
                    + tangent[k]
            }),
            1e-12,
        );
        assert_vector_close(
            sample.acceleration,
            std::array::from_fn(|k| force[k] / 2. + rate[k] / 2. * t + centrifugal[k]),
            1e-12,
        );
        assert_vector_close(
            sample.jerk,
            std::array::from_fn(|k| rate[k] / 2. + jerk[k]),
            1e-12,
        );
        assert!(sample.arc_interval_s.is_some());
    }
    assert!(path.sample_material_point(-1., local).is_err());
    assert!(path.sample_material_point(0., [f64::NAN; 3]).is_err());
    assert_eq!(path, saved);
    let mut point = initial;
    point.spin = None;
    let linear = point
        .prepare_motion([0.; 3], [0.; 3], 0.2, config())
        .unwrap();
    let sample = linear.sample_material_point(0.1, local).unwrap();
    assert_eq!(sample.arc_interval_s, None);
    assert_eq!(sample.velocity, initial.motion.velocity);
    assert_eq!(sample.acceleration, [0.; 3]);
}

#[test]
fn material_point_derivatives_use_nominal_arc_velocity_and_report_knot_sides() {
    let path = body()
        .prepare_motion([0.; 3], [0., 0., 4.], 0.2, config())
        .unwrap();
    let local = [0.3, 0.1, 0.];
    let segments = path.rotation().unwrap().segments();
    assert!(segments.len() > 1);
    let start = path.sample_material_point(0., local).unwrap();
    let physical = path.initial().point_velocity(start.position).unwrap();
    assert!((start.velocity[1] - physical[1]).abs() > 1e-8);
    for segment in segments {
        let t = (segment.start_s + segment.end_s) / 2.;
        let h = (segment.end_s - segment.start_s) * 1e-4;
        let actual = path.sample_material_point(t, local).unwrap();
        let minus = path.sample_material_point(t - h, local).unwrap();
        let plus = path.sample_material_point(t + h, local).unwrap();
        assert_vector_close(
            actual.velocity,
            std::array::from_fn(|k| (plus.position[k] - minus.position[k]) / (2. * h)),
            1e-7,
        );
        assert_vector_close(
            actual.acceleration,
            std::array::from_fn(|k| (plus.velocity[k] - minus.velocity[k]) / (2. * h)),
            1e-7,
        );
        assert_vector_close(
            actual.jerk,
            std::array::from_fn(|k| (plus.acceleration[k] - minus.acceleration[k]) / (2. * h)),
            1e-7,
        );
    }
    for pair in segments.windows(2) {
        let sample = path.sample_material_point(pair[0].end_s, local).unwrap();
        assert_eq!(
            sample.arc_interval_s,
            Some([pair[1].start_s, pair[1].end_s])
        );
    }
    let last = segments.last().unwrap();
    assert_eq!(
        path.sample_material_point(path.duration(), local)
            .unwrap()
            .arc_interval_s,
        Some([last.start_s, last.end_s])
    );
}

#[test]
fn material_point_work_and_moment_match_independent_power_quadrature() {
    use physics::astrophysics_spin::TorquePolynomial;
    let initial = body();
    let path = initial
        .prepare_affine_motion(
            [4., -2., 6.],
            [-3., 5., 2.],
            TorquePolynomial::constant([0.; 3]),
            0.2,
            config(),
        )
        .unwrap();
    let saved = path.clone();
    let local = [0.2, -0.1, 0.3];
    let force = [1., 3., -2.];
    let rate = [-4., 2., 1.];
    for t in [0., 1e-9, 0.05, 0.2] {
        let actual = path
            .material_point_force_work(t, local, force, rate)
            .unwrap();
        let mut work = 0.;
        let mut moment = [0.; 3];
        // Composite Simpson integrates independently sampled point power/moment.
        let count = 1024;
        let h = t / count as f64;
        for i in 0..=count {
            let time = h * i as f64;
            let sample = path.sample_material_point(time, local).unwrap();
            let f = std::array::from_fn(|k| force[k] + rate[k] * time);
            let weight = if i == 0 || i == count {
                1.
            } else if i % 2 == 0 {
                2.
            } else {
                4.
            };
            work += weight * (0..3).map(|k| f[k] * sample.velocity[k]).sum::<f64>() * h / 3.;
            let m = vector_cross(sample.position, f);
            for k in 0..3 {
                moment[k] += weight * m[k] * h / 3.;
            }
        }
        assert!((actual.total_work - work).abs() < 1e-11);
        assert_vector_close(actual.angular_impulse, moment, 1e-11);
        assert!((actual.total_work - actual.com_force_work - actual.torque_work).abs() < 1e-14);
    }
    assert_eq!(path, saved);
    assert!(
        path.material_point_force_work(0.1, [f64::NAN; 3], force, rate)
            .is_err()
    );
    let center = path
        .material_point_force_work(0.2, [0.; 3], force, rate)
        .unwrap();
    assert_eq!(center.torque_work, 0.);
    assert_eq!(
        center.com_force_work,
        path.affine_wrench_work(0.2, force, rate, TorquePolynomial::constant([0.; 3]))
            .unwrap()
            .0
    );
}

#[test]
fn material_point_work_splits_changing_spin_arcs_and_keeps_stationary_arm_moment() {
    let force = [1., 3., -2.];
    let rate = [-4., 2., 1.];
    let local = [0.2, -0.1, 0.3];
    for torque in [[0., 0., 4.], [0.; 3]] {
        let mut initial = body();
        if torque == [0.; 3] {
            initial.spin.as_mut().unwrap().angular_momentum = [0.; 3];
        }
        let path = initial
            .prepare_motion([0.; 3], torque, 0.2, config())
            .unwrap();
        let mut work = 0.;
        let mut moment = [0.; 3];
        for arc in path.rotation().unwrap().segments() {
            let dt = (arc.end_s - arc.start_s) / 16.;
            for i in 0..16 {
                for (node, weight) in [
                    (-(3_f64 / 5.).sqrt(), 5. / 9.),
                    (0., 8. / 9.),
                    ((3_f64 / 5.).sqrt(), 5. / 9.),
                ] {
                    let t = arc.start_s + (i as f64 + (node + 1.) / 2.) * dt;
                    let sample = path.sample_material_point(t, local).unwrap();
                    let f = std::array::from_fn(|k| force[k] + rate[k] * t);
                    work +=
                        (0..3).map(|k| f[k] * sample.velocity[k]).sum::<f64>() * weight * dt / 2.;
                    let m = vector_cross(sample.position, f);
                    for k in 0..3 {
                        moment[k] += m[k] * weight * dt / 2.;
                    }
                }
            }
        }
        let actual = path
            .material_point_force_work(0.2, local, force, rate)
            .unwrap();
        assert!((actual.total_work - work).abs() < 1e-11);
        assert_vector_close(actual.angular_impulse, moment, 1e-11);
        if torque == [0.; 3] {
            assert_eq!(actual.torque_work, 0.);
        }
    }
    let mut point = body();
    point.spin = None;
    let path = point
        .prepare_motion([0.; 3], [0.; 3], 0.2, config())
        .unwrap();
    assert!(
        path.material_point_force_work(0.1, local, force, rate)
            .is_err()
    );
    assert!(
        path.material_point_force_work(0.1, [0.; 3], force, rate)
            .is_ok()
    );
}

#[test]
fn reciprocal_material_point_forces_close_world_moments_on_different_spin_paths() {
    use physics::astrophysics_spin::TorquePolynomial;
    let a = body()
        .prepare_affine_motion(
            [4., -2., 6.],
            [-3., 5., 2.],
            TorquePolynomial {
                value: [0., 0., 4.],
                rate: [0., 0., 3.],
                ..TorquePolynomial::constant([0.; 3])
            },
            0.2,
            config(),
        )
        .unwrap();
    let mut second = body();
    second.motion.position = [-2., 1., 0.5];
    second.motion.velocity = [-1., 2., 0.3];
    second.spin.as_mut().unwrap().angular_momentum = [0., 1., 0.];
    let b = second
        .prepare_motion([2., 3., -1.], [0., -2., 0.], 0.2, config())
        .unwrap();
    let local = [0.2, -0.1, 0.3];
    let force = [1., 3., -2.];
    let rate = [-4., 2., 1.];
    let originals = (a.clone(), b.clone());
    for time in [1e-9, 0.03, 0.2] {
        let first = a
            .material_point_force_work(time, local, force, rate)
            .unwrap();
        let second = b
            .moving_material_point_force_work(time, &a, local, force.map(|x| -x), rate.map(|x| -x))
            .unwrap();
        assert_vector_close(
            vector_add(first.angular_impulse, second.angular_impulse),
            [0.; 3],
            1e-11,
        );
        let mut knots = vec![0., time];
        for path in [&a, &b] {
            knots.extend(
                path.rotation()
                    .unwrap()
                    .segments()
                    .iter()
                    .filter(|s| s.end_s < time)
                    .map(|s| s.end_s),
            );
        }
        knots.sort_by(f64::total_cmp);
        knots.dedup();
        let mut work = 0.;
        for interval in knots.windows(2) {
            let dt = (interval[1] - interval[0]) / 16.;
            for i in 0..16 {
                for (node, weight) in [
                    (-(3_f64 / 5.).sqrt(), 5. / 9.),
                    (0., 8. / 9.),
                    ((3_f64 / 5.).sqrt(), 5. / 9.),
                ] {
                    let t = interval[0] + (i as f64 + (node + 1.) / 2.) * dt;
                    let point = a.sample_material_point(t, local).unwrap();
                    let state = b.sample(t).unwrap();
                    // Express the shared point in receiver coordinates only at this instant.
                    let q = state.spin.unwrap().orientation;
                    let r = vector_add(point.position, vector_scale(state.motion.position, -1.));
                    let rotate = |v: [f64; 3]| {
                        let axis = [-q[0], -q[1], -q[2]];
                        let one = vector_cross(axis, v);
                        let two = vector_cross(axis, one);
                        std::array::from_fn(|k| v[k] + 2. * (q[3] * one[k] + two[k]))
                    };
                    let velocity = b.sample_material_point(t, rotate(r)).unwrap().velocity;
                    let power: f64 = (0..3)
                        .map(|k| -(force[k] + rate[k] * t) * velocity[k])
                        .sum();
                    work += power * weight * dt / 2.;
                }
            }
        }
        assert!((second.total_work - work).abs() < 1e-10);
    }
    assert_eq!((a, b), originals);
}
