use physics::liquid::{ThermalTranslatingBody, TranslatingBody};
use physics::surface_film::{Material, SqueezePressureControl, SurfaceFilm};

#[test]
fn heterogeneous_body_squeeze_preserves_each_vented_component() {
    use physics::surface_film::FilmMixture;
    let mut film = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [1, 3, 2]],
        Material::default(),
    )
    .unwrap();
    film.deposit(0, 0.025).unwrap();
    film.deposit(1, 0.025).unwrap();
    let mut mixture = FilmMixture::new(
        film,
        vec!["aqueous".into(), "viscous".into()],
        vec![vec![1.0, 0.0], vec![0.0, 1.0]],
    )
    .unwrap();
    mixture
        .configure_viscosities(Some(vec![0.01, 0.1]))
        .unwrap();
    let before = mixture.component_masses().unwrap();
    let (_, mut body) = state(0.05);
    let report = mixture
        .advance_squeeze_body(
            0.01,
            [0.0, 1.0, 0.0],
            &mut body,
            SqueezePressureControl::default(),
            0.001,
        )
        .unwrap();
    for (k, mass) in mixture.component_masses().unwrap().iter().enumerate() {
        assert!(
            (mass + report.transport.vented_component_masses.as_ref().unwrap()[k] - before[k])
                .abs()
                < 1e-12
        );
    }
    for h in mixture.film().thickness() {
        assert!((h - body.mechanics.position[1]).abs() < 1e-12);
    }
    assert!(mixture.fractions()[0][1] > 0.0);
}

fn state(mu: f64) -> (SurfaceFilm, ThermalTranslatingBody) {
    let mut film = SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material {
            viscosity: mu,
            ..Material::default()
        },
    )
    .unwrap();
    film.deposit(0, 0.025).unwrap();
    (
        film,
        ThermalTranslatingBody {
            mechanics: TranslatingBody {
                position: [0.25, 0.05, 0.25],
                velocity: [0.0, -0.03, 0.0],
                mass: 2.0,
            },
            specific_heat: 4.0,
            temperature: 300.0,
        },
    )
}

#[test]
fn normal_pressure_feedback_matches_exponential_drag_and_conserves_ledgers() {
    let (mut film, mut body) = state(0.05);
    let initial = body;
    let report = film
        .advance_squeeze_body(
            0.001,
            [0.0, 1.0, 0.0],
            &mut body,
            SqueezePressureControl::default(),
            0.001,
        )
        .unwrap();
    // Independent one-cell boundary stencil: K = mu / (4 h^3) = 100 kg/s.
    let loss = -(-0.05_f64).exp_m1();
    let displacement = 0.03 * loss / 50.0;
    assert!((body.mechanics.velocity[1] + 0.03 * (-0.05_f64).exp()).abs() < 1e-15);
    assert!((body.mechanics.position[1] - (0.05 - displacement)).abs() < 1e-15);
    assert!((film.thickness()[0] - body.mechanics.position[1]).abs() < 1e-15);
    assert!((report.transport.vented_volume - 0.5 * displacement).abs() < 1e-15);
    assert!(
        (2.0 * (body.mechanics.velocity[1] - initial.mechanics.velocity[1])
            + report.substrate_impulse[1])
            .abs()
            < 1e-15
    );
    let heat = initial.mechanics.velocity[1].powi(2) - body.mechanics.velocity[1].powi(2);
    assert!((report.transport.dissipated_energy - heat).abs() < 1e-15);
    assert!(
        (body.thermal_energy().unwrap() - initial.thermal_energy().unwrap() - heat).abs() < 1e-12
    );
}

#[test]
fn variable_gap_drag_converges_to_independent_continuous_solution() {
    // dw/dh = mu/(4 M h^3), so w(h)=w0+mu/(8M)*(h0^-2-h^-2).
    let velocity = |h: f64| 0.03 + 0.05 / 16.0 * (400.0 - h.powi(-2));
    let mut exact_h = 0.05;
    let dt = 0.1 / 10000.0;
    for _ in 0..10000 {
        let a = -velocity(exact_h);
        let b = -velocity(exact_h + 0.5 * dt * a);
        let c = -velocity(exact_h + 0.5 * dt * b);
        let d = -velocity(exact_h + dt * c);
        exact_h += dt * (a + 2.0 * b + 2.0 * c + d) / 6.0;
    }
    let mut errors = Vec::new();
    for step in [0.001, 0.0005, 0.00025] {
        let (mut film, mut body) = state(0.05);
        let report = film
            .advance_squeeze_body(
                0.1,
                [0.0, 1.0, 0.0],
                &mut body,
                SqueezePressureControl::default(),
                step,
            )
            .unwrap();
        errors.push((body.mechanics.position[1] - exact_h).abs());
        assert!((film.thickness()[0] - body.mechanics.position[1]).abs() < 1e-14);
        assert!((0.025 - film.total_volume() - report.transport.vented_volume).abs() < 1e-14);
    }
    assert!(
        errors[0] / errors[1] > 1.9 && errors[1] / errors[2] > 1.9,
        "{errors:?}"
    );
}

#[test]
fn late_unrepresentable_heat_rolls_back_entire_body_and_film() {
    let (mut film, mut body) = state(10.0);
    let initial = body;
    let volume = film.total_volume();
    assert_eq!(
        film.advance_squeeze_body(
            0.01,
            [0.0, 1.0, 0.0],
            &mut body,
            SqueezePressureControl::default(),
            0.001
        )
        .unwrap_err(),
        "unrepresentable squeeze body heat"
    );
    assert_eq!(film.total_volume(), volume);
    assert_eq!(body.mechanics.position, initial.mechanics.position);
    assert_eq!(body.mechanics.velocity, initial.mechanics.velocity);
    assert_eq!(body.temperature, initial.temperature);
}
