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
