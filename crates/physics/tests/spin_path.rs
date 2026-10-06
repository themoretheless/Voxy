use physics::{
    astrophysics_spin::Spin,
    spin_path::{Config, PathError},
};
fn config(error: f64) -> Config {
    Config {
        max_angular_error_rad: error,
        min_step_s: 1e-9,
        max_arcs: 10_000,
        max_trials: 100_000,
    }
}
fn spin(inertia: [f64; 3], momentum: [f64; 3]) -> Spin {
    Spin {
        orientation: [0., 0., 0., 1.],
        inertia,
        angular_momentum: momentum,
    }
}
fn distance(a: [f64; 4], b: [f64; 4]) -> f64 {
    let direct = (0..4).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt();
    let opposite = (0..4).map(|k| (a[k] + b[k]).powi(2)).sum::<f64>().sqrt();
    4. * (0.5 * direct.min(opposite)).clamp(0., 1.).asin()
}
fn derivative(state: Spin, q: [f64; 4]) -> [f64; 4] {
    let n = q.iter().map(|v| v * v).sum::<f64>().sqrt();
    let q = q.map(|v| v / n);
    let w = Spin {
        orientation: q,
        ..state
    }
    .angular_velocity()
    .unwrap();
    [
        0.5 * (w[0] * q[3] + w[1] * q[2] - w[2] * q[1]),
        0.5 * (-w[0] * q[2] + w[1] * q[3] + w[2] * q[0]),
        0.5 * (w[0] * q[1] - w[1] * q[0] + w[2] * q[3]),
        -0.5 * (w[0] * q[0] + w[1] * q[1] + w[2] * q[2]),
    ]
}
fn rk4(state: &mut Spin, h: f64) {
    let q = state.orientation;
    let k1 = derivative(*state, q);
    let k2 = derivative(*state, std::array::from_fn(|k| q[k] + 0.5 * h * k1[k]));
    let k3 = derivative(*state, std::array::from_fn(|k| q[k] + 0.5 * h * k2[k]));
    let k4 = derivative(*state, std::array::from_fn(|k| q[k] + h * k3[k]));
    let next: [f64; 4] =
        std::array::from_fn(|k| q[k] + h / 6. * (k1[k] + 2. * k2[k] + 2. * k3[k] + k4[k]));
    let norm = next.iter().map(|v| v * v).sum::<f64>().sqrt();
    state.orientation = next.map(|v| v / norm);
}
#[test]
fn isotropic_path_matches_analytic_rotation_and_retains_exact_step_endpoint() {
    let s = spin([2.; 3], [0., 0., 1.]);
    let path = s.prepare_path([0.; 3], 0.5, config(1e-7)).unwrap();
    assert_eq!(path.segments().len(), 1);
    let mut legacy = s;
    legacy.step([0.; 3], 0.5).unwrap();
    assert_eq!(path.end(), legacy);
    for time in [0., 0.125, 0.25, 0.5] {
        let sample = path.sample(time).unwrap();
        let angle = 0.5 * time;
        assert!(
            distance(
                sample.orientation,
                [0., 0., (angle / 2.).sin(), (angle / 2.).cos()]
            ) < 1e-14
        );
        assert_eq!(sample.angular_momentum, s.angular_momentum);
    }
    assert_eq!(path.sample(0.).unwrap(), s);
    assert_eq!(path.sample(0.5).unwrap(), path.end());
}
#[test]
fn anisotropic_free_spin_refines_and_model_bound_covers_independent_rk4_samples() {
    let s = spin([1., 2., 3.], [0.3, 0.5, 0.7]);
    let path = s.prepare_path([0.; 3], 0.1, config(1e-4)).unwrap();
    assert!(path.segments().len() > 1);
    assert!(path.model_angular_error_rad() <= 1e-4);
    let fine = s.prepare_path([0.; 3], 0.1, config(2e-5)).unwrap();
    assert!(fine.segments().len() > path.segments().len());
    let mut oracle = s;
    let mut maximum = 0_f64;
    for step in 0..=4096 {
        if step > 0 {
            rk4(&mut oracle, 0.1 / 4096.);
        }
        if step % 32 == 0 {
            let time = 0.1 * f64::from(step) / 4096.;
            let sample = path.sample(time).unwrap();
            maximum = maximum.max(distance(sample.orientation, oracle.orientation));
            assert!(
                distance(sample.orientation, oracle.orientation)
                    <= path.model_angular_error_rad() + 1e-12
            );
            assert_eq!(sample.angular_momentum, s.angular_momentum);
        }
    }
    assert!(
        distance(fine.end().orientation, oracle.orientation)
            < distance(path.end().orientation, oracle.orientation)
    );
    for segment in path.segments() {
        assert_eq!(path.sample(segment.end_s).unwrap(), segment.arc.end());
    }
    println!(
        "SPIN_PATH arcs={} trials={} measured_max_rad={} model_bound_rad={}",
        path.segments().len(),
        path.trials(),
        maximum,
        path.model_angular_error_rad()
    );
}
#[test]
fn constant_torque_arc_prefix_error_is_bounded_against_analytic_sphere() {
    let s = spin([1.; 3], [0.; 3]);
    let path = s.prepare_path([0., 0., 1.], 0.1, config(1e-3)).unwrap();
    for k in 0..=100 {
        let time = f64::from(k) * 0.001;
        let actual = path.sample(time).unwrap();
        let angle = 0.5 * time * time;
        let expected = [0., 0., (angle / 2.).sin(), (angle / 2.).cos()];
        assert!(distance(actual.orientation, expected) <= path.model_angular_error_rad() + 1e-12);
        assert!((actual.angular_momentum[2] - time).abs() < 1e-14);
    }
}
#[test]
fn late_path_budget_failure_preserves_caller_and_sampling_admission_is_explicit() {
    let s = spin([1., 2., 3.], [0.3, 0.5, 0.7]);
    let before = s;
    let mut limit = config(1e-4);
    limit.max_arcs = 1;
    assert_eq!(
        s.prepare_path([0.; 3], 0.1, limit).unwrap_err(),
        PathError::Budget
    );
    assert_eq!(s, before);
    let path = s.prepare_path([0.; 3], 0.1, config(1e-4)).unwrap();
    assert!(path.sample(-0.01).is_err());
    assert!(path.sample(f64::NAN).is_err());
    assert!(path.sample(0.2).is_err());
}

#[test]
fn long_precession_reserves_future_error_amplification_and_matches_rk4() {
    let s = spin([1., 1.1, 1.2], [0.1, 0.2, 1.5]);
    let path = s.prepare_path([0.; 3], 2.6, config(0.002)).unwrap();
    assert!(path.model_angular_error_rad() <= 0.002);
    let mut oracle = s;
    let mut maximum = 0_f64;
    for k in 0..=8192 {
        if k > 0 {
            rk4(&mut oracle, 2.6 / 8192.);
        }
        if k % 64 == 0 {
            let sample = path.sample(2.6 * f64::from(k) / 8192.).unwrap();
            maximum = maximum.max(distance(sample.orientation, oracle.orientation));
            assert!(
                distance(sample.orientation, oracle.orientation)
                    <= path.model_angular_error_rad() + 1e-11
            );
        }
    }
    println!(
        "LONG_SPIN_PATH arcs={} measured_max_rad={} model_bound_rad={}",
        path.segments().len(),
        maximum,
        path.model_angular_error_rad()
    );
}

#[test]
fn polynomial_torque_spherical_prefixes_match_integrated_momentum_and_analytic_attitude() {
    use physics::astrophysics_spin::TorquePolynomial;
    let law = TorquePolynomial {
        value: [0., 0., 2.],
        rate: [0., 0., -5.],
        acceleration: [0., 0., 8.],
        ..TorquePolynomial::constant([0.; 3])
    };
    let initial = spin([2.; 3], [0., 0., 1.]);
    let path = initial
        .prepare_polynomial_path(law, 0.1, config(1e-4))
        .unwrap();
    assert!(path.segments().len() > 1);
    for i in 0..=64 {
        let t = 0.1 * i as f64 / 64.;
        let state = path.sample(t).unwrap();
        let impulse = 2. * t - 2.5 * t * t + (8. / 6.) * t * t * t;
        assert!((state.angular_momentum[2] - 1. - impulse).abs() < 5e-14);
        let angle = 0.5 * (t + t * t - (5. / 6.) * t * t * t + (8. / 24.) * t * t * t * t);
        let oracle = [0., 0., (angle * 0.5).sin(), (angle * 0.5).cos()];
        assert!(distance(state.orientation, oracle) <= path.model_angular_error_rad() + 1e-12);
    }
    let mut invalid = law;
    invalid.rate[1] = f64::NAN;
    assert!(
        initial
            .prepare_polynomial_path(invalid, 0.1, config(1e-4))
            .is_err()
    );
    let mut budget = config(1e-4);
    budget.max_arcs = 1;
    assert_eq!(
        initial.prepare_polynomial_path(law, 0.1, budget),
        Err(PathError::Budget)
    );
    assert_eq!(initial, spin([2.; 3], [0., 0., 1.]));
}

#[test]
fn polynomial_torque_anisotropic_model_bound_covers_independent_time_dependent_rk4() {
    use physics::astrophysics_spin::TorquePolynomial;
    let initial = spin([1., 2., 3.], [0.3, 0.5, 0.7]);
    let law = TorquePolynomial {
        value: [0.4, -0.2, 0.3],
        rate: [-2., 3., 1.],
        acceleration: [5., -4., 2.],
        ..TorquePolynomial::constant([0.; 3])
    };
    let path = initial
        .prepare_polynomial_path(law, 0.1, config(1e-4))
        .unwrap();
    let finer = initial
        .prepare_polynomial_path(law, 0.1, config(2e-5))
        .unwrap();
    let mut oracle = initial;
    let h = 0.1 / 4096.;
    let momentum = |t: f64| {
        std::array::from_fn(|k| {
            initial.angular_momentum[k]
                + law.value[k] * t
                + law.rate[k] * t * t / 2.
                + law.acceleration[k] * t * t * t / 6.
        })
    };
    for step in 0..=4096 {
        let t = step as f64 * h;
        if step % 32 == 0 {
            let sample = path.sample(t).unwrap();
            assert!(
                distance(sample.orientation, oracle.orientation)
                    <= path.model_angular_error_rad() + 1e-12
            );
            for k in 0..3 {
                assert!((sample.angular_momentum[k] - momentum(t)[k]).abs() < 1e-13);
            }
        }
        if step == 4096 {
            break;
        }
        let q = oracle.orientation;
        let evaluate = |at, q| {
            derivative(
                Spin {
                    angular_momentum: momentum(at),
                    ..oracle
                },
                q,
            )
        };
        let k1 = evaluate(t, q);
        let k2 = evaluate(t + h / 2., std::array::from_fn(|k| q[k] + h * k1[k] / 2.));
        let k3 = evaluate(t + h / 2., std::array::from_fn(|k| q[k] + h * k2[k] / 2.));
        let k4 = evaluate(t + h, std::array::from_fn(|k| q[k] + h * k3[k]));
        let next: [f64; 4] =
            std::array::from_fn(|k| q[k] + h * (k1[k] + 2. * k2[k] + 2. * k3[k] + k4[k]) / 6.);
        let length = next.iter().map(|x| x * x).sum::<f64>().sqrt();
        oracle.orientation = next.map(|x| x / length);
        oracle.angular_momentum = momentum(t + h);
    }
    assert!(
        distance(finer.end().orientation, oracle.orientation)
            < distance(path.end().orientation, oracle.orientation)
    );
}

#[test]
fn cubic_arm_affine_force_quartic_torque_matches_direct_product_and_gaussian_impulse() {
    use physics::astrophysics_spin::TorquePolynomial;
    let arm = [0.3, -0.7, 0.2];
    let v = [0.8, 0.1, -0.4];
    let a = [-0.2, 0.5, 0.6];
    let j = [0.9, -0.3, 0.7];
    let f = [1.2, -2.1, 0.4];
    let rate = [-0.6, 0.9, 1.3];
    let law = TorquePolynomial::moving_affine_arm(arm, v, a, j, f, rate).unwrap();
    assert!(law.snap.iter().any(|x| x.abs() > 0.1));
    let direct = |t: f64| {
        let r: [f64; 3] =
            std::array::from_fn(|k| arm[k] + v[k] * t + a[k] * t * t / 2. + j[k] * t * t * t / 6.);
        let force: [f64; 3] = std::array::from_fn(|k| f[k] + rate[k] * t);
        std::array::from_fn::<_, 3, _>(|k| {
            let i = (k + 1) % 3;
            let h = (k + 2) % 3;
            r[i] * force[h] - r[h] * force[i]
        })
    };
    let initial = spin([2.; 3], [0.3, 0.5, 0.7]);
    let path = initial
        .prepare_polynomial_path(law, 0.1, config(1e-4))
        .unwrap();
    let shifted = law.shifted(0.04).unwrap();
    for i in 0..=32 {
        let t = 0.1 * i as f64 / 32.;
        let expected = direct(t);
        let actual = law.value_at(t).unwrap();
        let rebased = shifted.value_at(t).unwrap();
        let root = (3_f64 / 5.).sqrt();
        let mut integral = [0.; 3];
        for (node, weight) in [(-root, 5. / 9.), (0., 8. / 9.), (root, 5. / 9.)] {
            let sample = direct(t * (node + 1.) / 2.);
            for k in 0..3 {
                integral[k] += sample[k] * weight * t / 2.;
            }
        }
        let impulse = law.impulse(t).unwrap();
        let state = path.sample(t).unwrap();
        for k in 0..3 {
            assert!((actual[k] - expected[k]).abs() < 1e-13);
            assert!((rebased[k] - direct(t + 0.04)[k]).abs() < 1e-13);
            assert!((impulse[k] - integral[k]).abs() < 1e-13);
            assert!(
                (state.angular_momentum[k] - initial.angular_momentum[k] - integral[k]).abs()
                    < 1e-12
            );
        }
    }
    let legacy = TorquePolynomial::moving_arm(arm, v, a, f).unwrap();
    assert_eq!(
        legacy,
        TorquePolynomial::moving_affine_arm(arm, v, a, [0.; 3], f, [0.; 3]).unwrap()
    );
    let mut invalid = law;
    invalid.snap[1] = f64::NAN;
    assert!(
        initial
            .prepare_polynomial_path(invalid, 0.1, config(1e-4))
            .is_err()
    );
    assert!(TorquePolynomial::moving_affine_arm(arm, v, a, j, f, [f64::INFINITY; 3]).is_err());
}
