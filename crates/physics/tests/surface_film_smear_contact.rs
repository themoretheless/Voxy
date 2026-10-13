//! Plan step C1: smearing through a real sliding contact, with the Coulomb
//! and Couette limits selected per cell and the plain shear step as oracle.
use physics::friction;
use physics::surface_film::{Material, SmearContact, SmearRegime, SurfaceFilm};

fn film(viscosity: f64) -> SurfaceFilm {
    SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [0.01, 0.0, 0.0],
            [0.01, 0.0, 0.01],
            [0.0, 0.0, 0.01],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material {
            density: 1000.0,
            viscosity,
            surface_tension: 0.0,
            wetting: 0.0,
        },
    )
    .unwrap()
}

fn contact(speed: f64) -> SmearContact {
    SmearContact {
        normal: [0., 1., 0.],
        body_velocity_m_s: [speed, 0.3, 0.],
    }
}

#[test]
fn no_slip_leaves_the_film_exactly_as_a_plain_step_would() {
    let mut smeared = film(0.05);
    let mut plain = film(0.05);
    smeared.deposit(0, 5e-8).unwrap();
    plain.deposit(0, 5e-8).unwrap();
    let material = friction::Material::new(1e9, 1e8, 0.4).unwrap();
    let mut states = vec![friction::State::default(); 2];
    let report = smeared
        .advance_smear(
            0.1,
            contact(0.),
            &[1000., 1000.],
            &material,
            &mut states,
            [0.; 3],
            0.001,
        )
        .unwrap();
    plain
        .step_with_surface_shear(0.1, [0.; 3], &[[0.; 3]; 2], 0.001)
        .unwrap();
    assert_eq!(smeared.thickness(), plain.thickness());
    assert!(report.regimes.iter().all(|r| *r == SmearRegime::Idle));
    assert_eq!(report.slip_work_j, 0.);
}

#[test]
fn thick_film_is_lubricated_with_couette_traction_and_thin_film_is_coulomb() {
    let viscosity = 0.05;
    let speed = 0.2;
    let material = friction::Material::new(1e9, 1e8, 0.4).unwrap();
    // Thick film: Couette traction η v / h is far below μ p.
    let mut thick = film(viscosity);
    thick.deposit(0, 5e-8).unwrap();
    thick.deposit(1, 5e-8).unwrap();
    let h = thick.thickness()[0];
    let mut states = vec![friction::State::default(); 2];
    let report = thick
        .advance_smear(
            0.01,
            contact(speed),
            &[1e5, 1e5],
            &material,
            &mut states,
            [0.; 3],
            0.001,
        )
        .unwrap();
    let couette = viscosity * speed / h;
    assert!(couette < 0.4 * 1e5);
    for (t, regime) in report.traction_pa.iter().zip(&report.regimes) {
        assert_eq!(*regime, SmearRegime::Lubricated);
        assert!(
            (t[0] - couette).abs() < 1e-9 * couette,
            "{t:?} vs {couette}"
        );
        assert_eq!(t[1], 0.);
    }
    assert!(report.slip_work_j > 0.);
    assert_eq!(report.coulomb_dissipated_j, 0.);
    // Thin film: a tiny thickness makes Couette exceed the Coulomb limit.
    let mut thin = film(viscosity);
    thin.deposit(0, 1e-13).unwrap();
    thin.deposit(1, 1e-13).unwrap();
    let mut states = vec![friction::State::default(); 2];
    let mass = thin.total_mass();
    let report = thin
        .advance_smear(
            0.01,
            contact(speed),
            &[1e5, 1e5],
            &material,
            &mut states,
            [0.; 3],
            0.001,
        )
        .unwrap();
    for (t, regime) in report.traction_pa.iter().zip(&report.regimes) {
        assert_eq!(*regime, SmearRegime::Coulomb);
        assert!((t[0] - 0.4 * 1e5).abs() < 1e-6 * 1e5, "{t:?}");
    }
    assert!(
        report.coulomb_dissipated_j > 0.,
        "{}",
        report.coulomb_dissipated_j
    );
    assert!(states.iter().all(|s| s.dissipated_j_m2() > 0.));
    assert!((thin.total_mass() - mass).abs() <= 1e-15 * mass.max(1e-300));
}

#[test]
fn smearing_moves_mass_downstream_and_rejects_bad_input() {
    let mut film = film(0.05);
    film.deposit(0, 5e-8).unwrap();
    let material = friction::Material::new(1e9, 1e8, 0.4).unwrap();
    let mut states = vec![friction::State::default(); 2];
    let before = film.thickness();
    let mass = film.total_mass();
    for _ in 0..10 {
        film.advance_smear(
            0.1,
            contact(-0.5),
            &[1e4, 1e4],
            &material,
            &mut states,
            [0.; 3],
            0.001,
        )
        .unwrap();
    }
    let after = film.thickness();
    assert!(
        after[1] > before[1] && after[0] < before[0],
        "{before:?} -> {after:?}"
    );
    assert!((film.total_mass() - mass).abs() <= 1e-15);
    assert!(
        film.advance_smear(
            0.1,
            contact(-0.5),
            &[1e4],
            &material,
            &mut states,
            [0.; 3],
            0.001
        )
        .is_err()
    );
    let bad = SmearContact {
        normal: [0., 2., 0.],
        body_velocity_m_s: [0.; 3],
    };
    assert!(
        film.advance_smear(
            0.1,
            bad,
            &[1e4, 1e4],
            &material,
            &mut states,
            [0.; 3],
            0.001
        )
        .is_err()
    );
    assert!(
        film.advance_smear(
            0.1,
            contact(-0.5),
            &[-1., 1e4],
            &material,
            &mut states,
            [0.; 3],
            0.001
        )
        .is_err()
    );
}
