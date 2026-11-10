use physics::surface_film::{Material, SurfaceFilm};
fn film() -> SurfaceFilm {
    SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
        vec![[0, 1, 2]],
        Material::default(),
    )
    .unwrap()
}
#[test]
fn finite_thickness_advances_contact_with_independent_falling_solution() {
    let mut f = film();
    f.deposit(0, 0.1).unwrap(); // area .5 -> thickness .2
    let state = f.state();
    let wet = f.free_surface(1.0).unwrap();
    assert!(wet.points().iter().all(|p| (p[1] - 0.2).abs() < 1e-14));
    let start = [0.2, 1.0, 0.2];
    let wet_hit = wet
        .first_closing_accelerated_sphere_hit(start, [0.0; 3], [0.0, -2.0, 0.0], 1.0, 0.1, 100)
        .unwrap()
        .unwrap();
    let dry_hit = f
        .first_closing_accelerated_sphere_hit(start, [0.0; 3], [0.0, -2.0, 0.0], 1.0, 0.1, 100)
        .unwrap()
        .unwrap();
    assert!((wet_hit.time - 0.7_f64.sqrt()).abs() < 1e-12);
    assert!((dry_hit.time - 0.9_f64.sqrt()).abs() < 1e-12);
    assert!(wet_hit.time < dry_hit.time);
    assert_eq!(wet_hit.cell, 0);
    assert!((wet_hit.point[1] - 0.2).abs() < 1e-12);
    assert_eq!(f.state().cell_volumes_m3, state.cell_volumes_m3);
    assert_eq!(f.state().points, state.points);
}
#[test]
fn winding_side_selects_wet_side_and_zero_height_matches_substrate() {
    let mut f = film();
    let start = [0.2, 1.0, 0.2];
    let end = [0.2, -1.0, 0.2];
    let dry = f.first_sphere_hit(start, end, 0.1).unwrap().unwrap();
    let reconstructed = f
        .free_surface(-1.0)
        .unwrap()
        .first_sphere_hit(start, end, 0.1)
        .unwrap()
        .unwrap();
    assert_eq!(dry, reconstructed);
    f.deposit(0, 0.1).unwrap();
    assert!(
        f.free_surface(-1.0)
            .unwrap()
            .points()
            .iter()
            .all(|p| (p[1] + 0.2).abs() < 1e-14)
    );
    assert!(f.free_surface(0.0).is_err());
    assert!(f.free_surface(f64::NAN).is_err());
}
#[test]
fn nonuniform_cell_heights_reconstruct_a_continuous_sloping_surface() {
    let mut f = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material::default(),
    )
    .unwrap();
    f.deposit(0, 0.1).unwrap();
    f.deposit(1, 0.2).unwrap();
    let wet = f.free_surface(1.0).unwrap();
    let expected = [0.3, 0.2, 0.3, 0.4];
    for (p, h) in wet.points().iter().zip(expected) {
        assert!((p[1] - h).abs() < 1e-14);
    }
    let hit = wet
        .first_sphere_hit([0.2, 1.0, 0.4], [0.2, -1.0, 0.4], 0.01)
        .unwrap()
        .unwrap();
    assert_eq!(hit.cell, 0);
    assert!(hit.normal[0].abs() > 0.05 || hit.normal[2].abs() > 0.05);
    assert!((f.total_volume() - 0.3).abs() < 1e-14);
}
#[test]
fn contact_snapshot_is_frozen_until_explicit_rebuild() {
    let mut f = film();
    let snapshot = f.free_surface(1.0).unwrap();
    f.deposit(0, 0.1).unwrap();
    assert_eq!(snapshot.points()[0][1], 0.0);
    assert!((f.free_surface(1.0).unwrap().points()[0][1] - 0.2).abs() < 1e-14);
    assert!(
        snapshot
            .first_accelerated_sphere_hit([0.2, 1.0, 0.2], [0.0, -2.0, 0.0], [0.0; 3], 1.0, 0.01, 1)
            .is_err()
    );
}

#[test]
fn immersed_centers_and_partial_top_overlap_exclude_dry_and_substrate_backside() {
    let mut f = film();
    assert!(
        f.free_surface(1.0)
            .unwrap()
            .immersed_cell([0.2, 0.0, 0.2], 100)
            .unwrap()
            .is_none()
    );
    f.deposit(0, 0.1).unwrap();
    let surface = f.free_surface(1.0).unwrap();
    assert_eq!(
        surface.immersed_cell([0.2, 0.1, 0.2], 100).unwrap(),
        Some(0)
    );
    for p in [[0.2, -0.01, 0.2], [0.2, 0.21, 0.2], [2.0, 0.1, 0.2]] {
        assert!(surface.immersed_cell(p, 100).unwrap().is_none());
    }
    assert_eq!(
        surface
            .overlapping_wet_cell([0.2, 0.21, 0.2], 0.02)
            .unwrap(),
        Some(0)
    );
    assert!(
        surface
            .overlapping_wet_cell([0.2, -0.01, 0.2], 0.3)
            .unwrap()
            .is_none()
    );
    assert!(surface.immersed_cell([0.2, 0.1, 0.2], 1).is_err());
}
