use physics::surface_film::{FilmRheology, Material, SurfaceFilm};
fn model() -> FilmRheology {
    FilmRheology {
        consistency: 2.0,
        flow_index: 0.5,
        yield_stress: 1.0,
        profile_samples: 64,
    }
}
#[test]
fn yield_threshold_and_power_law_shear_match_no_slip_profile() {
    let m = model();
    assert_eq!(m.flow_rate(0.1, 0.0, 0.5).unwrap(), 0.0);
    assert_eq!(m.flow_rate(0.1, 0.0, 1.0).unwrap(), 0.0);
    assert!((m.flow_rate(0.1, 0.0, 5.0).unwrap() - 0.02).abs() < 1e-15);
    assert!((m.flow_rate(0.1, 0.0, -5.0).unwrap() + 0.02).abs() < 1e-15);
    let mut linear = m;
    linear.flow_index = 1.0;
    linear.yield_stress = 0.0;
    assert!(
        (linear.flow_rate(0.1, 3.0, 4.0).unwrap() - (3.0 * 0.001 / 3.0 + 4.0 * 0.01 / 2.0) / 2.0)
            .abs()
            < 1e-15
    );
}
#[test]
fn pressure_driven_yielded_profile_matches_independent_integral_and_refines_mixed_flow() {
    let mut m = model();
    m.flow_index = 1.0;
    assert_eq!(m.flow_rate(0.1, 2.0, 0.0).unwrap(), 0.0);
    // shear rate=max((2*s-1)/2,0), integrate s*rate from 0.5 to 3.
    let integral = |s: f64| s.powi(3) / 3.0 - s * s / 4.0;
    assert!((m.flow_rate(3.0, 2.0, 0.0).unwrap() - (integral(3.0) - integral(0.5))).abs() < 1e-13);
    m = model();
    let mut reference = m;
    reference.profile_samples = 4096;
    let exact = reference.flow_rate(0.2, 20.0, 2.0).unwrap();
    let coarse = m.flow_rate(0.2, 20.0, 2.0).unwrap();
    m.profile_samples = 128;
    let fine = m.flow_rate(0.2, 20.0, 2.0).unwrap();
    assert!((coarse - exact).abs() / (fine - exact).abs() > 3.9);
}
fn film() -> SurfaceFilm {
    SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material {
            surface_tension: 0.0,
            wetting: 0.0,
            ..Material::default()
        },
    )
    .unwrap()
}
#[test]
fn thick_film_stays_below_yield_then_smears_conservatively() {
    let mut f = film();
    f.deposit(0, 0.0005).unwrap();
    let before = f.thickness();
    f.step_with_rheology(0.01, [0.0; 3], &[[-0.5, 0.0, 0.0]; 2], 0.001, model())
        .unwrap();
    assert_eq!(f.thickness(), before);
    f.step_with_rheology(0.01, [0.0; 3], &[[-5.0, 0.0, 0.0]; 2], 0.001, model())
        .unwrap();
    assert!(f.thickness()[1] > 0.0);
    assert!((f.total_volume() - 0.0005).abs() < 1e-18);
    let before = f.thickness();
    let mut bad = model();
    bad.flow_index = 0.0;
    assert!(
        f.step_with_rheology(0.01, [0.0; 3], &[[-5.0, 0.0, 0.0]; 2], 0.001, bad)
            .is_err()
    );
    assert_eq!(f.thickness(), before);
}
#[test]
fn oblique_shear_uses_total_stress_and_rotates_without_changing_yield() {
    let m = model();
    let height = 0.1;
    let aligned = m
        .flow_rate_vector(height, 0.0, [1.0, 0.0, 0.0], [2.0, 0.0, 0.0])
        .unwrap();
    let diagonal = 1.0 / 2.0_f64.sqrt();
    let oblique = m
        .flow_rate_vector(height, 0.0, [diagonal, 0.0, diagonal], [2.0, 0.0, 0.0])
        .unwrap();
    assert!((oblique - aligned * diagonal).abs() < 1e-15);
    let rotated = m
        .flow_rate_vector(height, 0.0, [0.0, 0.0, 1.0], [0.0, 0.0, 2.0])
        .unwrap();
    assert!((rotated - aligned).abs() < 1e-15);
    assert!(
        m.flow_rate_vector(height, 0.0, [0.0; 3], [2.0, 0.0, 0.0])
            .is_err()
    );
}
#[test]
fn opposed_pressure_and_shear_select_interior_extremum_at_interface() {
    let m = FilmRheology {
        consistency: 1.0,
        flow_index: 1.0,
        yield_stress: 0.0,
        profile_samples: 64,
    };
    // q(h)=-20*h³/3+h²/2 has a maximum at h=0.05, between these states.
    let normal = [1.0, 0.0, 0.0];
    let stress = [1.0, 0.0, 0.0];
    let expected = -20.0 * 0.05_f64.powi(3) / 3.0 + 0.05_f64.powi(2) / 2.0;
    let flux = m.interface_flow(0.1, 0.01, -20.0, normal, stress).unwrap();
    assert!((flux - expected).abs() < 1e-15);
    let rare = m.interface_flow(0.01, 0.1, -20.0, normal, stress).unwrap();
    let low = m.flow_rate(0.01, -20.0, 1.0).unwrap();
    let high = m.flow_rate(0.1, -20.0, 1.0).unwrap();
    assert!((rare - low.min(high)).abs() < 1e-15);
    assert!(m.interface_flow(0.0, 0.0, -20.0, normal, stress).unwrap() == 0.0);
}
#[test]
fn zero_yield_newtonian_limit_matches_existing_mesh_transport_for_pure_loads() {
    let m = FilmRheology {
        consistency: 0.05,
        flow_index: 1.0,
        yield_stress: 0.0,
        profile_samples: 64,
    };
    for (gravity, traction) in [
        ([0.0; 3], [[-1.0, 0.0, 0.0]; 2]),
        ([-1.0, 0.0, 0.0], [[0.0; 3]; 2]),
    ] {
        let mut base = film();
        let mut rheological = film();
        base.deposit(0, 0.0005).unwrap();
        rheological.deposit(0, 0.0005).unwrap();
        base.step_with_surface_shear(0.01, gravity, &traction, 0.001)
            .unwrap();
        rheological
            .step_with_rheology(0.01, gravity, &traction, 0.001, m)
            .unwrap();
        for (a, b) in base.thickness().iter().zip(rheological.thickness()) {
            assert!((a - b).abs() < 1e-15);
        }
    }
}
#[test]
fn moving_wall_profiles_match_couette_poiseuille_and_yielded_couette() {
    let linear = FilmRheology {
        consistency: 2.0,
        flow_index: 1.0,
        yield_stress: 0.0,
        profile_samples: 64,
    };
    let (flow, stress) = linear.sliding_profile(0.1, 3.0, 0.4).unwrap();
    assert!((flow - (0.1 * 0.4 / 2.0 + 3.0 * 0.001 / 24.0)).abs() < 1e-15);
    assert!((stress - (2.0 * 0.4 / 0.1 - 3.0 * 0.1 / 2.0)).abs() < 1e-14);
    let (flow, stress) = model().sliding_profile(0.1, 0.0, 0.4).unwrap();
    assert!((flow - 0.02).abs() < 1e-15);
    assert!((stress - 5.0).abs() < 1e-14);
    assert!(model().sliding_profile(0.0, 0.0, 0.4).is_err());
}
#[test]
fn mixed_nonlinear_wall_speed_is_recovered_from_inverted_traction() {
    let mut m = model();
    let gap = 0.1;
    let gradient = 20.0;
    let speed = 0.4;
    let mut errors = Vec::new();
    for samples in [64, 128] {
        m.profile_samples = samples;
        let (_, traction) = m.sliding_profile(gap, gradient, speed).unwrap();
        assert!(traction > m.yield_stress);
        // Independent continuous integral of ((tau+g*s-yield)/K)^2.
        let velocity = ((traction + gradient * gap - 1.0).powi(3) - (traction - 1.0).powi(3))
            / (3.0 * gradient * 4.0);
        errors.push((velocity - speed).abs());
    }
    assert!(errors[0] < 3e-6);
    assert!(errors[0] / errors[1] > 3.9);
}
#[test]
fn sliding_wall_work_and_pressure_work_match_independent_viscous_dissipation() {
    let m = FilmRheology {
        consistency: 2.0,
        flow_index: 1.0,
        yield_stress: 0.0,
        profile_samples: 64,
    };
    for (gradient, speed) in [(3.0, 0.4), (100.0, 0.1), (-100.0, 0.1)] {
        let gap = 0.1;
        let b = m.sliding_balance(gap, gradient, speed).unwrap();
        // Integral of mu*(U/h + g*(h/2-z)/mu)^2 over the confined gap.
        let independent =
            2.0 * speed * speed / gap + gradient * gradient * gap.powi(3) / (12.0 * 2.0);
        assert!((b.dissipated_power - independent).abs() < 1e-13);
        assert!((b.wall_power + b.pressure_power - b.dissipated_power).abs() < 1e-13);
    }
    let b = m.sliding_balance(0.1, 100.0, 0.1).unwrap();
    assert!(b.wall_power < 0.0);
    assert!(b.pressure_power > 0.0);
    assert!(b.dissipated_power > 0.0);
}
