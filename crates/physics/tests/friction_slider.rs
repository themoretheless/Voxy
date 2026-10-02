use physics::friction::{Material, Mode, State};
#[test]
fn implicit_stick_matches_analytic_spring_mass_and_slip_matches_coulomb_impulse() {
    let material = Material::new(1e6, 100., 0.5).unwrap();
    let gap = [-1e-4, 0., 0.];
    let state = material
        .response(&State::default(), gap, [1., 0., 0.])
        .unwrap()
        .0;
    let step = material
        .advance_slider(&state, gap, [1., 0., 0.], [0., 0.01, 0.], 2., 0.1, 0.1)
        .unwrap();
    let expected = 0.01 / (1. + 0.1 * 100. * 0.1_f64.powi(2) / 2.);
    assert_eq!(step.response.mode, Mode::Stick);
    assert!((step.velocity_m_s[1] - expected).abs() < 1e-15);
    assert_eq!(step.physical_heat_j, 0.);
    assert!(step.contact_numerical_loss_j > 0. && step.integration_numerical_loss_j > 0.);
    let slip = material
        .advance_slider(&state, gap, [1., 0., 0.], [0., 10., 0.], 2., 0.1, 0.1)
        .unwrap();
    assert_eq!(slip.response.mode, Mode::Slip);
    assert!((slip.velocity_m_s[1] - 9.75).abs() < 1e-14);
    assert!((slip.impulse_n_s[1] + 0.5).abs() < 1e-14);
    assert!(slip.physical_heat_j > 0.);
    assert!(slip.energy_defect_j.abs() < 1e-12);
}
#[test]
fn repeated_solver_steps_dissipate_energy_and_preserve_normal_gap() {
    let material = Material::new(1e6, 1000., 0.5).unwrap();
    let normal = [1., 0., 0.];
    let mut gap = [-1e-4, 0., 0.];
    let mut state = material.response(&State::default(), gap, normal).unwrap().0;
    let mut velocity = [0., 1., 0.5];
    let initial = velocity.iter().map(|v| v * v).sum::<f64>();
    let mut integration_loss = 0.;
    for _ in 0..100 {
        let result = material
            .advance_slider(&state, gap, normal, velocity, 2., 0.1, 0.001)
            .unwrap();
        integration_loss += result.integration_numerical_loss_j;
        state = result.state;
        gap = result.gap_m;
        velocity = result.velocity_m_s;
        assert_eq!(gap[0], -1e-4);
        let kinetic = velocity.iter().map(|v| v * v).sum::<f64>();
        let stored = result.response.tangential_stored_j_m2 * 0.1;
        let physical = state.dissipated_j_m2() * 0.1;
        let numerical = state.numerical_dissipated_j_m2() * 0.1 + integration_loss;
        assert!((kinetic + stored + physical + numerical - initial).abs() < 1e-12);
    }
    assert!(
        material
            .advance_slider(&state, gap, normal, [0.1, 0., 0.], 2., 0.1, 0.001)
            .is_err()
    );
}
#[test]
fn time_refinement_converges_to_undamped_sticking_oscillator() {
    let material = Material::new(1e6, 100., 0.5).unwrap();
    let normal = [1., 0., 0.];
    let duration = 0.5;
    let omega = (0.1_f64 * 100. / 2.).sqrt();
    let exact_v = 0.01 * (omega * duration).cos();
    let exact_gap = 0.01 / omega * (omega * duration).sin();
    let mut previous_error = f64::INFINITY;
    let mut previous_loss = f64::INFINITY;
    for count in [4, 16, 64] {
        let mut gap = [-1e-4, 0., 0.];
        let mut state = material.response(&State::default(), gap, normal).unwrap().0;
        let mut v = [0., 0.01, 0.];
        let mut loss = 0.;
        for _ in 0..count {
            let step = material
                .advance_slider(&state, gap, normal, v, 2., 0.1, duration / f64::from(count))
                .unwrap();
            assert_eq!(step.response.mode, Mode::Stick);
            assert_eq!(step.physical_heat_j, 0.);
            loss += step.integration_numerical_loss_j + step.contact_numerical_loss_j;
            state = step.state;
            gap = step.gap_m;
            v = step.velocity_m_s;
        }
        let error = (v[1] - exact_v).abs() + omega * (gap[1] - exact_gap).abs();
        assert!(error < previous_error * 0.4);
        assert!(loss < previous_loss * 0.4);
        previous_error = error;
        previous_loss = loss;
    }
}
#[test]
fn large_tangential_reference_does_not_change_contact_motion_or_energy() {
    let material = Material::new(1e6, 100., 0.5).unwrap();
    let normal = [1., 0., 0.];
    let mut local_gap = [-1e-4, 0., 0.];
    let mut shifted_gap = [-1e-4, 1e6, 0.];
    let mut local = material
        .response(&State::default(), local_gap, normal)
        .unwrap()
        .0;
    let reference = material
        .inactive_reference(&State::default(), shifted_gap, normal)
        .unwrap();
    let mut shifted = material
        .response(&reference, shifted_gap, normal)
        .unwrap()
        .0;
    let mut v = [0., 0.01, 0.];
    for _ in 0..20 {
        let first = material
            .advance_slider(&local, local_gap, normal, v, 2., 0.1, 0.01)
            .unwrap();
        let second = material
            .advance_slider(&shifted, shifted_gap, normal, v, 2., 0.1, 0.01)
            .unwrap();
        assert_eq!(first.velocity_m_s, second.velocity_m_s);
        assert_eq!(first.impulse_n_s, second.impulse_n_s);
        assert_eq!(
            first.response.tangential_stored_j_m2,
            second.response.tangential_stored_j_m2
        );
        assert_eq!(
            first.contact_numerical_loss_j,
            second.contact_numerical_loss_j
        );
        local = first.state;
        shifted = second.state;
        local_gap = first.gap_m;
        shifted_gap = second.gap_m;
        v = first.velocity_m_s;
    }
}
