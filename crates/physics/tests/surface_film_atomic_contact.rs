use physics::surface_film::{BridgeConfig, Material, SurfaceFilm};
fn fixture() -> SurfaceFilm {
    let mut film = SurfaceFilm::new(
        &[
            [0., 0., 0.],
            [0.01, 0., 0.],
            [0., 0., 0.01],
            [0., 0.0001, 0.],
            [0., 0.0001, 0.01],
            [0.01, 0.0001, 0.],
        ],
        vec![[0, 1, 2], [3, 4, 5]],
        Material {
            surface_tension: 0.,
            wetting: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    film.deposit_batch(&[(0, 2e-8), (1, 1e-8)]).unwrap();
    assert!(
        !film
            .detect_self_bridges(BridgeConfig::default())
            .unwrap()
            .is_empty()
    );
    film
}
#[test]
fn failed_contact_rolls_back_geometry_source_and_transport() {
    let mut film = fixture();
    let old = film.state();
    let mut points = old.points.clone();
    for p in &mut points {
        p[0] += 0.001;
    }
    let config = BridgeConfig {
        max_candidates: 0,
        ..Default::default()
    };
    assert!(
        film.advance_on_geometry_with_contact(&points, 0.01, &[(0, 1e-9)], [0.; 3], Some(config))
            .is_err()
    );
    let after = film.state();
    assert_eq!(after.points, old.points);
    assert_eq!(after.cell_volumes_m3, old.cell_volumes_m3);
    assert_eq!(after.triangles, old.triangles);
}
#[test]
fn successful_atomic_contact_matches_sequential_physics_and_source_mass() {
    let mut atomic = fixture();
    let mut sequential = fixture();
    let mut points = atomic.state().points;
    for p in &mut points {
        p[0] += 0.001;
    }
    let before = atomic.total_mass();
    let before_cell = atomic.state().cell_volumes_m3[0];
    let expected_added = (before_cell + 0.01 * 1e-9) - before_cell;
    let density = atomic.material().density;
    let config = BridgeConfig::default();
    let (added, transferred) = atomic
        .advance_on_geometry_with_contact(&points, 0.01, &[(0, 1e-9)], [0.; 3], Some(config))
        .unwrap();
    sequential
        .advance_on_geometry(&points, 0.01, &[(0, 1e-9)], [0.; 3])
        .unwrap();
    let expected_transfer = sequential.exchange_self_contact(0.01, config).unwrap();
    assert!(transferred > 0.);
    assert_eq!(transferred, expected_transfer);
    assert_eq!(
        atomic.state().cell_volumes_m3,
        sequential.state().cell_volumes_m3
    );
    assert_eq!(added, expected_added);
    assert!((atomic.total_mass() - before - added * density).abs() < before * 1e-12);
}

#[test]
fn moving_surfaces_exchange_only_during_contact_and_preserve_source_mass() {
    let mut film = fixture();
    let rest = film.state().points;
    let initial = film.total_mass();
    let density = film.material().density;
    let config = BridgeConfig::default();
    let mut maximum_error = 0_f64;
    let mut gross = 0.;
    let mut active = 0;
    let mut separated = 0;
    for frame in 1..=200 {
        let time = frame as f64 * 0.01;
        let gap = 0.0001 + 0.0029 * 0.5 * (1. + (time * std::f64::consts::TAU).cos());
        let stretch = 1. + 0.1 * (time * std::f64::consts::TAU).sin();
        let points: Vec<_> = rest
            .iter()
            .enumerate()
            .map(|(i, p)| {
                [
                    p[0] * stretch + 0.002 * time,
                    if i < 3 { 0. } else { gap },
                    p[2],
                ]
            })
            .collect();
        let before_cell = film.state().cell_volumes_m3[0];
        let expected_added = (before_cell + 0.01 * 1e-9) - before_cell;
        let (added, transferred) = film
            .advance_on_geometry_with_contact(&points, 0.01, &[(0, 1e-9)], [0.; 3], Some(config))
            .unwrap();
        assert_eq!(added, expected_added);
        if gap > config.max_gap {
            assert_eq!(transferred, 0.);
            separated += 1;
        }
        if transferred > 0. {
            active += 1;
            gross += transferred;
        }
        let expected = initial + time * 1e-9 * density;
        maximum_error = maximum_error.max((film.total_mass() - expected).abs() / expected);
        assert!(maximum_error < 1e-12);
        assert!(
            film.state()
                .cell_volumes_m3
                .iter()
                .all(|v| v.is_finite() && *v >= 0.)
        );
    }
    assert!(active > 0 && separated > 0 && gross > 0.);
    println!(
        "MOVING CONTACT: active={active}, separated={separated}, gross={gross:.9e} m3, max mass relative error={maximum_error:.9e}"
    );
}
