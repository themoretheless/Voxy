use physics::surface_film::{Material, SurfaceFilm, Wetting};

fn film() -> SurfaceFilm {
    let mut film = SurfaceFilm::new(
        &[
            [0., 0., 0.],
            [0.02, 0., 0.],
            [0.02, 0., 0.02],
            [0., 0., 0.02],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material::default(),
    )
    .unwrap();
    film.set_wetting(Some(Wetting {
        contact_angle: 0.1,
        precursor_thickness: 1e-6,
    }))
    .unwrap();
    film.deposit_batch(&[(0, 4e-9), (1, 1e-9)]).unwrap();
    film.update_geometry(&[
        [0., 0., 0.],
        [0.02, 0.001, 0.],
        [0.02, 0.002, 0.02],
        [0., 0.001, 0.02],
    ])
    .unwrap();
    film
}

#[test]
fn checkpoint_preserves_deformed_geometry_mass_distribution_and_continuation() {
    let mut original = film();
    original.step(0.001, [0., -9.81, 0.]).unwrap();
    let state = original.state();
    assert_eq!(state.points[2], [0.02, 0.002, 0.02]);
    let mut restored = SurfaceFilm::from_state(&state).unwrap();
    assert_eq!(restored.state().points, state.points);
    assert_eq!(restored.state().triangles, state.triangles);
    assert_eq!(restored.state().cell_volumes_m3, state.cell_volumes_m3);
    assert_eq!(restored.total_mass(), original.total_mass());
    assert_eq!(restored.thickness(), original.thickness());
    assert_eq!(
        restored.driving_pressure([0., -9.81, 0.]).unwrap(),
        original.driving_pressure([0., -9.81, 0.]).unwrap()
    );
    for _ in 0..5 {
        original.step(0.001, [0., -9.81, 0.]).unwrap();
        restored.step(0.001, [0., -9.81, 0.]).unwrap();
    }
    assert_eq!(
        restored.state().cell_volumes_m3,
        original.state().cell_volumes_m3
    );
}

#[test]
fn invalid_checkpoints_are_rejected_without_modifying_the_source() {
    let original = film();
    let state = original.state();
    for value in [-1., f64::NAN, f64::INFINITY, f64::MAX] {
        let mut invalid = state.clone();
        invalid.cell_volumes_m3[0] = value;
        assert!(SurfaceFilm::from_state(&invalid).is_err());
    }
    let mut invalid = state.clone();
    invalid.cell_volumes_m3.pop();
    assert!(SurfaceFilm::from_state(&invalid).is_err());
    let mut invalid = state.clone();
    invalid.points[0][0] = f64::NAN;
    assert!(SurfaceFilm::from_state(&invalid).is_err());
    let mut invalid = state.clone();
    invalid.triangles[0][0] = invalid.points.len();
    assert!(SurfaceFilm::from_state(&invalid).is_err());
    let mut invalid = state.clone();
    invalid.wetting.as_mut().unwrap().contact_angle = 1.;
    assert!(SurfaceFilm::from_state(&invalid).is_err());
    assert_eq!(original.state().cell_volumes_m3, state.cell_volumes_m3);
}
