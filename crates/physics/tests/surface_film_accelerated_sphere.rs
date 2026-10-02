use physics::surface_film::{Material, SurfaceFilm};
fn film() -> SurfaceFilm {
    SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material::default(),
    )
    .unwrap()
}
#[test]
fn curved_face_impact_is_found_when_endpoint_chord_misses() {
    let f = film();
    let start = [0.2, 1.0, 0.2];
    assert!(f.first_sphere_hit(start, start, 0.1).unwrap().is_none());
    let hit = f
        .first_accelerated_sphere_hit(start, [0.0, -4.0, 0.0], [0.0, 8.0, 0.0], 1.0, 0.1, 100)
        .unwrap()
        .unwrap();
    assert!((hit.time - (1.0 - 0.1_f64.sqrt()) / 2.0).abs() < 1e-12);
    assert!((hit.point[1]).abs() < 1e-12);
    assert!((hit.normal[1] - 1.0).abs() < 1e-12);
}
#[test]
fn curved_edge_and_vertex_contacts_have_radial_normals() {
    let f = film();
    for (start, radius, time, normal) in [
        (
            [-0.2, 0.2, 0.5],
            0.25,
            (1.0 - 0.75_f64.sqrt()) / 2.0,
            [-0.8, 0.6, 0.0],
        ),
        (
            [-0.2, 0.2, -0.2],
            0.3,
            (1.0 - 0.5_f64.sqrt()) / 2.0,
            [-2.0 / 3.0, 1.0 / 3.0, -2.0 / 3.0],
        ),
    ] {
        let hit = f
            .first_accelerated_sphere_hit(
                start,
                [0.0, -0.8, 0.0],
                [0.0, 1.6, 0.0],
                1.0,
                radius,
                100,
            )
            .unwrap()
            .unwrap();
        assert!((hit.time - time).abs() < 1e-12);
        for k in 0..3 {
            assert!((hit.normal[k] - normal[k]).abs() < 1e-12);
        }
    }
    assert!(
        f.first_accelerated_sphere_hit(
            [-0.3, 0.2, 0.5],
            [0.0, -0.8, 0.0],
            [0.0, 1.6, 0.0],
            1.0,
            0.25,
            100
        )
        .unwrap()
        .is_none()
    );
}
#[test]
fn linear_limit_and_initial_overlap_match_existing_geometry_query() {
    let f = film();
    let start = [0.2, 0.4, 0.2];
    let old = f
        .first_sphere_hit(start, [0.2, -0.4, 0.2], 0.1)
        .unwrap()
        .unwrap();
    let new = f
        .first_accelerated_sphere_hit(start, [0.0, -0.8, 0.0], [0.0; 3], 1.0, 0.1, 100)
        .unwrap()
        .unwrap();
    assert!((new.time - old.time).abs() < 1e-13);
    assert_eq!(new.cell, old.cell);
    let hit = f
        .first_accelerated_sphere_hit([0.2, 0.05, 0.2], [0.0, 1.0, 0.0], [0.0; 3], 1.0, 0.1, 100)
        .unwrap()
        .unwrap();
    assert_eq!(hit.time, 0.0);
    assert!((hit.penetration - 0.05).abs() < 1e-14);
}
#[test]
fn tangent_contact_and_transformations_are_resolved_without_time_sampling() {
    for scale in [0.001, 1.0, 1000.0] {
        let transform = |p: [f64; 3]| [2.0 + scale * p[0], -3.0 + scale * p[1], 5.0 + scale * p[2]];
        let f = SurfaceFilm::new(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]].map(transform),
            vec![[0, 1, 2]],
            Material::default(),
        )
        .unwrap();
        let hit = f
            .first_accelerated_sphere_hit(
                transform([0.2, 1.1, 0.2]),
                [0.0, -4.0 * scale, 0.0],
                [0.0, 8.0 * scale, 0.0],
                1.0,
                0.1 * scale,
                100,
            )
            .unwrap()
            .unwrap();
        assert!((hit.time - 0.5).abs() < 1e-6);
    }
}
#[test]
fn invalid_controls_overflow_and_feature_budget_do_not_change_film() {
    let f = film();
    let mass = f.total_mass();
    assert_eq!(
        f.first_accelerated_sphere_hit(
            [0.2, 1.0, 0.2],
            [0.0, -4.0, 0.0],
            [0.0, 8.0, 0.0],
            1.0,
            0.1,
            1
        )
        .unwrap_err(),
        "accelerated sphere feature budget"
    );
    for dt in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            f.first_accelerated_sphere_hit(
                [0.2, 1.0, 0.2],
                [0.0, -4.0, 0.0],
                [0.0, 8.0, 0.0],
                dt,
                0.1,
                100
            )
            .is_err()
        );
    }
    assert!(
        f.first_accelerated_sphere_hit(
            [0.2, 1.0, 0.2],
            [0.0, f64::MAX, 0.0],
            [0.0; 3],
            2.0,
            0.1,
            100
        )
        .is_err()
    );
    assert_eq!(f.total_mass(), mass);
}

#[test]
fn downward_gravity_finds_an_overhead_contact_that_the_chord_misses() {
    let f = SurfaceFilm::new(
        &[[0.0, 2.0, 0.0], [1.0, 2.0, 0.0], [0.0, 2.0, 1.0]],
        vec![[0, 1, 2]],
        Material::default(),
    )
    .unwrap();
    let start = [0.2, 1.0, 0.2];
    assert!(f.first_sphere_hit(start, start, 0.1).unwrap().is_none());
    let hit = f
        .first_accelerated_sphere_hit(start, [0.0, 4.0, 0.0], [0.0, -8.0, 0.0], 1.0, 0.1, 100)
        .unwrap()
        .unwrap();
    assert!((hit.time - (1.0 - 0.1_f64.sqrt()) / 2.0).abs() < 1e-12);
    assert!((hit.normal[1] + 1.0).abs() < 1e-12);
}

#[test]
fn a_large_future_displacement_does_not_turn_a_local_near_miss_into_contact() {
    let f = film();
    // y=1-1e8*t+5e15*t² reaches a minimum of 0.5 at t=1e-8.
    // The enormous endpoint cannot set a metre-scale contact tolerance near y=1.
    assert!(
        f.first_accelerated_sphere_hit(
            [0.2, 1.0, 0.2],
            [0.0, -1e8, 0.0],
            [0.0, 1e16, 0.0],
            1.0,
            0.1,
            100
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn closing_sweep_skips_departure_and_finds_gravity_return() {
    let f = film();
    let start = [0.2, 0.1, 0.2];
    let hit = f
        .first_closing_accelerated_sphere_hit(
            start,
            [0.0, 1.0, 0.0],
            [0.0, -4.0, 0.0],
            1.0,
            0.1,
            100,
        )
        .unwrap()
        .unwrap();
    assert!((hit.time - 0.5).abs() < 1e-12);
    assert!(
        f.first_closing_accelerated_sphere_hit(start, [0.0, 1.0, 0.0], [0.0; 3], 1.0, 0.1, 100)
            .unwrap()
            .is_none()
    );
    assert!(
        f.first_closing_accelerated_sphere_hit(start, [0.0; 3], [0.0, 1.0, 0.0], 1.0, 0.1, 100)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        f.first_closing_accelerated_sphere_hit(start, [0.0; 3], [0.0, -1.0, 0.0], 1.0, 0.1, 100)
            .unwrap()
            .unwrap()
            .time,
        0.0
    );
}
