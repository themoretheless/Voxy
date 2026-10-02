use physics::cohesive::{Material, State};
fn material() -> Material {
    Material::new(1e9, 2e9, 1e5, 10.).unwrap()
}
fn close(a: f64, b: f64, relative: f64) {
    assert!((a - b).abs() <= relative * b.abs().max(1e-12), "{a} != {b}");
}
#[test]
fn monotonic_work_is_exact_fracture_energy_and_unloading_cannot_heal() {
    let m = material();
    let mut state = State::default();
    let mut work = 0.;
    let mut previous = 0.;
    for i in 1..=40 {
        let gap = f64::from(i) * m.failure_m() / 40.;
        let (next, r) = m.response(&state, [gap, 0., 0.], [1., 0., 0.]).unwrap();
        work += (previous + r.traction_pa[0]) * 0.5 * m.failure_m() / 40.;
        close(work, r.stored_j_m2 + r.dissipated_j_m2, 1e-12);
        assert!(next.maximum_separation_m() >= state.maximum_separation_m());
        previous = r.traction_pa[0];
        state = next;
    }
    close(work, 10., 1e-12);
    let (held, closed) = m.response(&state, [0.; 3], [1., 0., 0.]).unwrap();
    assert_eq!(held, state);
    close(closed.damage, 1., 1e-15);
    close(closed.dissipated_j_m2, 10., 1e-15);
    let (_, reopened) = m.response(&state, [0.00001, 0., 0.], [1., 0., 0.]).unwrap();
    assert_eq!(reopened.traction_pa, [0.; 3]);
}
#[test]
fn broken_faces_retain_frictionless_compression_contact() {
    let m = material();
    let state = m
        .response(&State::default(), [m.failure_m(), 0., 0.], [1., 0., 0.])
        .unwrap()
        .0;
    let (next, r) = m.response(&state, [-1e-5, 1e-4, 0.], [1., 0., 0.]).unwrap();
    assert_eq!(next, state);
    close(r.traction_pa[0], -2e4, 1e-12);
    close(r.traction_pa[1], 0., 1e-12);
    close(r.stored_j_m2, 0.1, 1e-12);
    close(r.tangent_pa_m[0][0], 2e9, 1e-12);
    close(r.damage, 1., 1e-12);
}
#[test]
fn mixed_mode_tangent_matches_derivative_and_compression_does_not_damage() {
    let m = material();
    let old = m
        .response(&State::default(), [0.00013, 0., 0.], [1., 0., 0.])
        .unwrap()
        .0;
    for (history, jump) in [
        (State::default(), [0.00002, 0.00001, 0.]),
        (State::default(), [0.00012, 0.00004, 0.00003]),
        (old, [0.00006, 0.00002, 0.]),
        (State::default(), [-0.00003, 0.00012, 0.00004]),
    ] {
        let r = m.response(&history, jump, [1., 0., 0.]).unwrap().1;
        for j in 0..3 {
            let mut plus = jump;
            let mut minus = jump;
            plus[j] += 1e-10;
            minus[j] -= 1e-10;
            let upper = m.response(&history, plus, [1., 0., 0.]).unwrap().1;
            let lower = m.response(&history, minus, [1., 0., 0.]).unwrap().1;
            for i in 0..3 {
                let expected = (upper.traction_pa[i] - lower.traction_pa[i]) / 2e-10;
                assert!(
                    (r.tangent_pa_m[i][j] - expected).abs() < 3.,
                    "{i},{j}: {} != {expected}",
                    r.tangent_pa_m[i][j]
                );
            }
        }
    }
    let (state, r) = m
        .response(&State::default(), [-0.001, 0., 0.], [1., 0., 0.])
        .unwrap();
    assert_eq!(state, State::default());
    close(r.damage, 0., 1e-12);
}
#[test]
fn rejected_trials_and_material_parameters_are_explicit() {
    let m = material();
    let state = State::default();
    for (jump, normal) in [
        ([f64::NAN; 3], [1., 0., 0.]),
        ([0.; 3], [2., 0., 0.]),
        ([f64::MAX; 3], [1., 0., 0.]),
    ] {
        assert!(m.response(&state, jump, normal).is_err());
    }
    assert_eq!(state, State::default());
    assert!(Material::new(1e9, 1e9, 1e5, 1.).is_err()); // Too little energy for the elastic peak.
    assert!(Material::new(0., 1e9, 1e5, 10.).is_err());
}

#[test]
fn arbitrary_interface_orientation_rotates_traction_and_tangent() {
    let rotation = [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0., 0.6]];
    let rotate = |v: [f64; 3]| std::array::from_fn(|i| (0..3).map(|j| rotation[i][j] * v[j]).sum());
    let m = material();
    let mut a = State::default();
    let mut b = State::default();
    for jump in [
        [0.00012, 0.00003, 0.00002],
        [0.00005, 0.00002, 0.],
        [-0.00003, 0.00012, 0.00004],
    ] {
        let (next_a, original) = m.response(&a, jump, [1., 0., 0.]).unwrap();
        let (next_b, rotated) = m.response(&b, rotate(jump), rotate([1., 0., 0.])).unwrap();
        close(
            next_a.maximum_separation_m(),
            next_b.maximum_separation_m(),
            1e-12,
        );
        close(original.damage, rotated.damage, 1e-12);
        close(original.stored_j_m2, rotated.stored_j_m2, 1e-12);
        for (x, y) in rotated
            .traction_pa
            .into_iter()
            .zip(rotate(original.traction_pa))
        {
            close(x, y, 1e-11);
        }
        for i in 0..3 {
            for j in 0..3 {
                let expected: f64 = (0..3)
                    .flat_map(|k| {
                        (0..3).map(move |l| {
                            rotation[i][k] * original.tangent_pa_m[k][l] * rotation[j][l]
                        })
                    })
                    .sum();
                assert!((rotated.tangent_pa_m[i][j] - expected).abs() < 1e-6);
            }
        }
        a = next_a;
        b = next_b;
    }
}
