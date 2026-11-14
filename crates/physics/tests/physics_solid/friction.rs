use physics::friction::{Material, Mode, State};
fn close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() < tol, "{a} != {b}");
}
fn material() -> Material {
    Material::new(1e9, 1e9, 0.5).unwrap()
}
#[test]
fn constant_pressure_stick_slip_work_and_release_have_separate_ledgers() {
    let m = material();
    let mut state = m
        .response(&State::default(), [-1e-4, 0., 0.], [1., 0., 0.])
        .unwrap()
        .0;
    let mut work = 0.;
    let mut endpoint_work = 0.;
    let mut previous = 0.;
    for step in 1..=40 {
        let gap = f64::from(step) * 1e-4 / 40.;
        let (next, r) = m.response(&state, [-1e-4, gap, 0.], [1., 0., 0.]).unwrap();
        let force = r.tangential_traction_pa[1];
        let increment = 1e-4 / 40.;
        work += 0.5 * (previous + force) * increment;
        endpoint_work += force * increment;
        close(work, r.tangential_stored_j_m2 + r.dissipated_j_m2, 1e-12);
        close(
            endpoint_work,
            r.tangential_stored_j_m2 + r.dissipated_j_m2 + r.numerical_dissipated_j_m2,
            1e-12,
        );
        assert!(next.dissipated_j_m2() >= state.dissipated_j_m2());
        assert!(force <= r.pressure_pa * 0.5 + 1e-8);
        previous = force;
        state = next;
    }
    close(work, 3.75, 1e-12);
    close(state.dissipated_j_m2(), 2.5, 1e-12);
    let (held, r) = m.response(&state, [-1e-4, 1e-4, 0.], [1., 0., 0.]).unwrap();
    close(held.dissipated_j_m2(), state.dissipated_j_m2(), 1e-12);
    close(r.slip_increment_m, 0., 1e-15);
    let (opened, r) = m.response(&state, [1e-4, 0.1, 0.], [1., 0., 0.]).unwrap();
    assert_eq!(r.mode, Mode::Open);
    assert_eq!(r.tangential_traction_pa, [0.; 3]);
    close(opened.released_j_m2(), 1.25, 1e-12);
    let (_, reclosed) = m.response(&opened, [-1e-4, 0.1, 0.], [1., 0., 0.]).unwrap();
    assert_eq!(reclosed.mode, Mode::Stick);
    assert_eq!(reclosed.tangential_traction_pa, [0.; 3]);
}
#[test]
fn pressure_unloading_and_reversal_respect_coulomb_cone_and_nonnegative_work() {
    let m = material();
    let state = m
        .response(&State::default(), [-1e-4, 1e-4, 0.], [1., 0., 0.])
        .unwrap()
        .0;
    let old_energy = 1.25;
    let (reduced, r) = m.response(&state, [-5e-5, 1e-4, 0.], [1., 0., 0.]).unwrap();
    close(r.tangential_traction_pa[1], 25000., 1e-8);
    close(
        old_energy - r.tangential_stored_j_m2,
        (reduced.dissipated_j_m2() - state.dissipated_j_m2())
            + (reduced.numerical_dissipated_j_m2() - state.numerical_dissipated_j_m2()),
        1e-12,
    );
    let (reverse, r) = m
        .response(&reduced, [-5e-5, -1e-4, 0.], [1., 0., 0.])
        .unwrap();
    close(r.tangential_traction_pa[1], -25000., 1e-8);
    assert!(reverse.dissipated_j_m2() > reduced.dissipated_j_m2());
}
#[test]
fn coupled_nonsymmetric_tangent_matches_stick_and_slip_derivatives() {
    let m = material();
    let old = State::default();
    for jump in [[-1e-4, 1e-5, 2e-5], [-1e-4, 1e-4, 2e-4], [1e-4, 1e-4, 0.]] {
        let r = m.response(&old, jump, [1., 0., 0.]).unwrap().1;
        for j in 0..3 {
            let mut plus = jump;
            let mut minus = jump;
            plus[j] += 1e-10;
            minus[j] -= 1e-10;
            let upper = m.response(&old, plus, [1., 0., 0.]).unwrap().1;
            let lower = m.response(&old, minus, [1., 0., 0.]).unwrap().1;
            for i in 0..3 {
                let derivative =
                    (upper.tangential_traction_pa[i] - lower.tangential_traction_pa[i]) / 2e-10;
                close(derivative, r.tangential_tangent_pa_m[i][j], 0.2);
            }
        }
    }
    let r = m.response(&old, [-1e-4, 1e-4, 0.], [1., 0., 0.]).unwrap().1;
    close(r.tangential_tangent_pa_m[1][0], -5e8, 1e-5);
    close(r.tangential_tangent_pa_m[0][1], 0., 1e-12);
}
#[test]
fn numerical_defect_decreases_with_step_refinement_and_zero_mu_is_frictionless() {
    let run = |count: i32| {
        let m = material();
        let mut state = State::default();
        for step in 0..=count {
            state = m
                .response(
                    &state,
                    [-1e-4, f64::from(step) * 1e-4 / f64::from(count), 0.],
                    [1., 0., 0.],
                )
                .unwrap()
                .0;
        }
        state
    };
    let coarse = run(40);
    let fine = run(80);
    close(coarse.dissipated_j_m2(), fine.dissipated_j_m2(), 1e-12);
    close(
        coarse.numerical_dissipated_j_m2(),
        2. * fine.numerical_dissipated_j_m2(),
        1e-12,
    );
    let r = Material::new(1e9, 1e9, 0.)
        .unwrap()
        .response(&State::default(), [-1e-4, 0., 0.], [1., 0., 0.])
        .unwrap()
        .1;
    assert_eq!(r.tangential_traction_pa, [0.; 3]);
    assert_eq!(r.tangential_tangent_pa_m, [[0.; 3]; 3]);
}
#[test]
fn invalid_kinematics_or_trials_leave_history_unchanged() {
    let m = material();
    let old = m
        .response(&State::default(), [-1e-4, 1e-4, 0.], [1., 0., 0.])
        .unwrap()
        .0;
    let copy = old;
    for (jump, normal) in [
        ([f64::NAN; 3], [1., 0., 0.]),
        ([-f64::MAX; 3], [1., 0., 0.]),
        ([0.; 3], [2., 0., 0.]),
    ] {
        assert!(m.response(&old, jump, normal).is_err());
    }
    assert_eq!(old, copy);
    assert!(m.inactive_reference(&old, [0.; 3], [1., 0., 0.]).is_err());
    assert!(Material::new(1e9, 1e9, -0.1).is_err());
}

#[test]
fn fixed_frame_covariance_preserves_friction_force_work_and_coupled_tangent() {
    let rotation = [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0., 0.6]];
    let transform = |point: [f64; 3]| {
        std::array::from_fn(|row| {
            (0..3)
                .map(|column| rotation[row][column] * point[column])
                .sum()
        })
    };
    let material = material();
    let mut original = State::default();
    let mut rotated = State::default();
    for jump in [
        [-1e-4, 1e-4, 2e-5],
        [-5e-5, -7e-5, 1e-4],
        [1e-4, 0.001, -0.0002],
    ] {
        let (left, left_response) = material.response(&original, jump, [1., 0., 0.]).unwrap();
        let (right, right_response) = material
            .response(&rotated, transform(jump), transform([1., 0., 0.]))
            .unwrap();
        for (actual, expected) in right_response
            .tangential_traction_pa
            .into_iter()
            .zip(transform(left_response.tangential_traction_pa))
        {
            close(actual, expected, 1e-8);
        }
        close(left.dissipated_j_m2(), right.dissipated_j_m2(), 1e-10);
        close(left.released_j_m2(), right.released_j_m2(), 1e-10);
        for row in 0..3 {
            for column in 0..3 {
                let expected: f64 = (0..3)
                    .flat_map(|first| {
                        (0..3).map(move |second| {
                            rotation[row][first]
                                * left_response.tangential_tangent_pa_m[first][second]
                                * rotation[column][second]
                        })
                    })
                    .sum();
                close(
                    right_response.tangential_tangent_pa_m[row][column],
                    expected,
                    1e-6,
                );
            }
        }
        original = left;
        rotated = right;
    }
}
