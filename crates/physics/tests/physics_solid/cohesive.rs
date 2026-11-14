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

#[test]
fn terminal_fracture_survives_parameter_cycles_without_fabricating_separation() {
    let wet = Material::new(1e6, 1e7, 1000., 2.5).unwrap();
    let dry = Material::new(1e6, 1e7, 1000., 10.).unwrap();
    let n = [1., 0., 0.];
    let jump = [0.006, 0., 0.];
    let (mut state, accepted) = wet.response(&State::default(), jump, n).unwrap();
    assert_eq!(accepted.damage, 1.);
    let maximum = state.maximum_separation_m();
    for _ in 0..10 {
        let (dried, work) = dry.migrate_history(&wet, &state, jump, n).unwrap();
        assert_eq!(work, 0.);
        assert_eq!(dried.maximum_separation_m(), maximum);
        let (_, r) = dry.response(&dried, [0.001, 0., 0.], n).unwrap();
        assert_eq!(r.damage, 1.);
        assert_eq!(r.traction_pa, [0.; 3]);
        assert_eq!(r.tangent_pa_m, [[0.; 3]; 3]);
        assert_eq!(r.dissipated_j_m2, accepted.dissipated_j_m2);
        let (_, closed) = dry.response(&dried, [-0.001, 0., 0.], n).unwrap();
        assert_eq!(closed.traction_pa[0], -10000.);
        state = wet.migrate_history(&dry, &dried, jump, n).unwrap().0;
    }
    let partial = wet
        .response(&State::default(), [0.002, 0., 0.], n)
        .unwrap()
        .0;
    let (retained, _) = dry
        .migrate_history(&wet, &partial, [0.002, 0., 0.], n)
        .unwrap();
    assert!(dry.damage(&retained) >= wet.damage(&partial));
}

#[test]
fn partial_drying_retains_damage_and_closes_piecewise_loading_work() {
    let wet = Material::new(1e6, 1e7, 1000., 2.5).unwrap();
    let dry = Material::new(1e6, 1e7, 1000., 10.).unwrap();
    let n = [1., 0., 0.];
    let a = 0.002;
    let (old, before) = wet.response(&State::default(), [a, 0., 0.], n).unwrap();
    let floor = before.damage;
    let (retained, parameter_work) = dry.migrate_history(&wet, &old, [a, 0., 0.], n).unwrap();
    let (_, start) = dry.response(&retained, [a, 0., 0.], n).unwrap();
    close(start.damage, floor, 1e-14);
    close(start.dissipated_j_m2, before.dissipated_j_m2, 1e-14);
    close(
        start.stored_j_m2 - before.stored_j_m2,
        parameter_work,
        1e-14,
    );
    assert_eq!(retained.maximum_separation_m(), a);
    let crossover =
        dry.onset_m() * dry.failure_m() / ((1. - floor) * dry.failure_m() + floor * dry.onset_m());
    let mut cycling = retained;
    for _ in 0..10 {
        let (wetted, wet_work) = wet.migrate_history(&dry, &cycling, [a, 0., 0.], n).unwrap();
        let (dried, dry_work) = dry.migrate_history(&wet, &wetted, [a, 0., 0.], n).unwrap();
        assert_eq!(dried.maximum_separation_m(), a);
        let (_, response) = dry.response(&dried, [a, 0., 0.], n).unwrap();
        close(response.damage, floor, 1e-14);
        close(response.dissipated_j_m2, start.dissipated_j_m2, 1e-14);
        close(response.stored_j_m2, start.stored_j_m2, 1e-14);
        assert!((wet_work + dry_work).abs() < 1e-12);
        cycling = dried;
    }
    let b = 0.004;
    let (_, end) = dry.response(&retained, [b, 0., 0.], n).unwrap();
    let exact_work = 0.5 * 1e6 * (1. - floor) * (crossover * crossover - a * a)
        + 1e6 * dry.onset_m() / (dry.failure_m() - dry.onset_m())
            * (dry.failure_m() * (b - crossover) - 0.5 * (b * b - crossover * crossover));
    close(
        end.stored_j_m2 - start.stored_j_m2 + end.dissipated_j_m2 - start.dissipated_j_m2,
        exact_work,
        1e-12,
    );
    for gap in [0.0022, 0.003] {
        let (_, value) = dry.response(&retained, [gap, 0., 0.], n).unwrap();
        let eps = 1e-8;
        let (_, plus) = dry.response(&retained, [gap + eps, 0., 0.], n).unwrap();
        let (_, minus) = dry.response(&retained, [gap - eps, 0., 0.], n).unwrap();
        close(
            value.tangent_pa_m[0][0],
            (plus.traction_pa[0] - minus.traction_pa[0]) / (2. * eps),
            1e-9,
        );
    }
    let (_, unloaded) = dry.response(&retained, [0.0001, 0., 0.], n).unwrap();
    close(unloaded.damage, floor, 1e-14);
    close(unloaded.dissipated_j_m2, start.dissipated_j_m2, 1e-14);
}

#[test]
fn wet_dry_migration_preserves_accepted_stick_and_slide_with_identical_friction() {
    let wet = Material::new(1e6, 1048576., 1000., 1.5)
        .unwrap()
        .with_friction(0.25, 65536.)
        .unwrap();
    let dry = Material::new(1e6, 1048576., 1000., 10.)
        .unwrap()
        .with_friction(0.25, 65536.)
        .unwrap();
    let n = [1., 0., 0.];
    let fractured = wet
        .response(&State::default(), [0.004, 0., 0.], n)
        .unwrap()
        .0;
    for shear in [2_f64.powi(-12), 2_f64.powi(-8)] {
        let jump = [-2_f64.powi(-12), shear, 0.];
        let (accepted, before) = wet.response(&fractured, jump, n).unwrap();
        assert_eq!(
            before.friction_mode,
            Some(if shear == 2_f64.powi(-12) {
                physics::friction::Mode::Stick
            } else {
                physics::friction::Mode::Slip
            })
        );
        // Compare queries of the same accepted history. The original slipping
        // transition tangent need not equal the fixed-pose re-evaluation tangent.
        let (_, before) = wet.response(&accepted, jump, n).unwrap();
        let (dried, work) = dry.migrate_history(&wet, &accepted, jump, n).unwrap();
        assert_eq!(work, 0.);
        assert_eq!(
            dried.maximum_separation_m(),
            accepted.maximum_separation_m()
        );
        let (_, after) = dry.response(&dried, jump, n).unwrap();
        assert_eq!(after.damage, 1.);
        assert_eq!(after.traction_pa, before.traction_pa);
        assert_eq!(after.tangent_pa_m, before.tangent_pa_m);
        assert_eq!(after.friction_mode, before.friction_mode);
        assert_eq!(
            after.friction_dissipated_j_m2,
            before.friction_dissipated_j_m2
        );
        assert_eq!(
            after.friction_numerical_j_m2,
            before.friction_numerical_j_m2
        );
        assert_eq!(after.friction_released_j_m2, before.friction_released_j_m2);
        close(after.dissipated_j_m2, before.dissipated_j_m2, 1e-14);
        let changed = dry.with_friction(0.5, 65536.).unwrap();
        assert!(changed.migrate_history(&wet, &accepted, jump, n).is_err());
        assert!(
            dry.migrate_history(&wet, &accepted, [jump[0], shear * 2., 0.], n)
                .is_err()
        );
    }
}

#[test]
fn migrated_partial_damage_is_objective_and_closes_mixed_mode_energy_gradient() {
    let wet = Material::new(1e6, 1e7, 1000., 2.5).unwrap();
    let dry = Material::new(1e6, 1e7, 1000., 10.).unwrap();
    let normal = [1., 0., 0.];
    let opening = [0.002, 0., 0.];
    let old = wet.response(&State::default(), opening, normal).unwrap().0;
    let retained = dry.migrate_history(&wet, &old, opening, normal).unwrap().0;
    let rotation = [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0., 0.6]];
    let rotate = |v: [f64; 3]| std::array::from_fn(|i| (0..3).map(|j| rotation[i][j] * v[j]).sum());
    for jump in [
        [0.0021, 0.0003, -0.0004],
        [0.003, 0.0004, -0.0002],
        [-0.0002, 0.0022, 0.0003],
    ] {
        let (_, response) = dry.response(&retained, jump, normal).unwrap();
        for axis in 0..3 {
            let eps = 1e-8;
            let mut a = jump;
            let mut b = jump;
            a[axis] += eps;
            b[axis] -= eps;
            let (_, plus) = dry.response(&retained, a, normal).unwrap();
            let (_, minus) = dry.response(&retained, b, normal).unwrap();
            let gradient = ((plus.stored_j_m2 + plus.dissipated_j_m2)
                - (minus.stored_j_m2 + minus.dissipated_j_m2))
                / (2. * eps);
            assert!((gradient - response.traction_pa[axis]).abs() < 1e-5);
            for row in 0..3 {
                let tangent = (plus.traction_pa[row] - minus.traction_pa[row]) / (2. * eps);
                assert!((tangent - response.tangent_pa_m[row][axis]).abs() < 0.03);
            }
        }
        let (_, turned) = dry
            .response(&retained, rotate(jump), rotate(normal))
            .unwrap();
        close(turned.damage, response.damage, 1e-12);
        close(turned.stored_j_m2, response.stored_j_m2, 1e-12);
        close(turned.dissipated_j_m2, response.dissipated_j_m2, 1e-12);
        let force = rotate(response.traction_pa);
        for axis in 0..3 {
            assert!((turned.traction_pa[axis] - force[axis]).abs() < 1e-9);
        }
        for i in 0..3 {
            for j in 0..3 {
                let expected: f64 = (0..3)
                    .flat_map(|a| {
                        (0..3).map(move |b| {
                            rotation[i][a] * response.tangent_pa_m[a][b] * rotation[j][b]
                        })
                    })
                    .sum();
                assert!((turned.tangent_pa_m[i][j] - expected).abs() < 1e-7);
            }
        }
    }
}
