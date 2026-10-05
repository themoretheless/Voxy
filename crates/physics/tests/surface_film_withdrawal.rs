use physics::surface_film::{FilmMixture, Material, SurfaceFilm};
fn mixture() -> FilmMixture {
    let mut film = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [1., 0., 1.]],
        vec![[0, 1, 2], [1, 3, 2]],
        Material::default(),
    )
    .unwrap();
    film.deposit(0, 0.004).unwrap();
    film.deposit(1, 0.006).unwrap();
    FilmMixture::new(
        film,
        vec!["solvent".into(), "residue".into()],
        vec![vec![0.25, 0.75], vec![0.8, 0.2]],
    )
    .unwrap()
}
#[test]
fn kilogram_withdrawals_deplete_exactly_and_reject_late_overdraw_atomically() {
    let mut film = mixture();
    let density = film.film().material().density;
    let before = format!("{film:?}");
    assert!(
        film.withdraw_component_masses_batch(&[
            (0, vec![0.0005 * density, 0.]),
            (0, vec![0.0006 * density, 0.]),
        ])
        .is_err()
    );
    assert_eq!(format!("{film:?}"), before);
    for invalid in [f64::NAN, f64::INFINITY, -1.] {
        assert!(
            film.withdraw_component_masses_batch(&[
                (0, vec![0.0005 * density, 0.]),
                (1, vec![invalid, 0.]),
            ])
            .is_err()
        );
        assert_eq!(format!("{film:?}"), before);
    }
    let amounts: Vec<_> = film
        .component_volumes_m3()
        .iter()
        .enumerate()
        .map(|(cell, row)| (cell, row.iter().map(|v| v * density).collect()))
        .collect();
    let masses = film.component_masses().unwrap();
    let receipt = film.withdraw_component_masses_batch(&amounts).unwrap();
    assert_eq!(film.film().total_mass(), 0.);
    assert_eq!(film.component_masses().unwrap(), vec![0., 0.]);
    for (actual, expected) in receipt.component_masses_kg.iter().zip(masses) {
        assert!((actual - expected).abs() < 1e-14);
    }
}
#[test]
fn canonical_mass_survives_inexact_volume_projection_and_complete_removal() {
    let source = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.]],
        vec![[0, 1, 2]],
        Material {
            density: 0.1,
            ..Material::default()
        },
    )
    .unwrap();
    let mut film = FilmMixture::new(source, vec!["solvent".into()], vec![vec![1.]]).unwrap();
    let requested = 123.456789;
    let receipt = film
        .deposit_component_masses_batch(&[(0, vec![requested])])
        .unwrap();
    assert_eq!(receipt.mass_kg, requested);
    assert_eq!(film.component_masses_kg()[0][0], requested);
    assert_ne!(
        film.component_volumes_m3()[0][0] * film.film().material().density,
        requested,
        "fixture must distinguish canonical mass from rounded volume projection"
    );
    let receipt = film
        .withdraw_component_masses_batch(&[(0, vec![requested])])
        .unwrap();
    assert_eq!(receipt.mass_kg, requested);
    assert_eq!(film.component_masses_kg()[0][0], 0.);
    assert_eq!(film.film().total_mass(), 0.);
}
#[test]
fn kilogram_deposits_balance_species_and_bulk_across_repeated_cells() {
    let mut film = mixture();
    let before = film.component_masses().unwrap();
    let bulk = film.film().total_mass();
    let receipt = film
        .deposit_component_masses_batch(&[
            (0, vec![0.2, 0.3]),
            (1, vec![0.4, 0.]),
            (0, vec![0., 0.1]),
        ])
        .unwrap();
    let after = film.component_masses().unwrap();
    assert!((receipt.mass_kg - 1.).abs() < 1e-14);
    assert!((film.film().total_mass() - bulk - receipt.mass_kg).abs() < 1e-14);
    for k in 0..2 {
        assert!((after[k] - before[k] - receipt.component_masses_kg[k]).abs() < 1e-14);
    }
    let all: Vec<_> = film
        .component_volumes_m3()
        .iter()
        .enumerate()
        .map(|(i, row)| {
            (
                i,
                row.iter()
                    .map(|v| v * film.film().material().density)
                    .collect(),
            )
        })
        .collect();
    film.withdraw_component_masses_batch(&all).unwrap();
    let receipt = film
        .deposit_component_masses_batch(&[(0, vec![0.25, 0.75])])
        .unwrap();
    assert_eq!(film.fractions()[0], vec![0.25, 0.75]);
    assert!((receipt.mass_kg - film.film().total_mass()).abs() < 1e-14);
}
#[test]
fn kilogram_deposit_late_failure_and_unresolved_flux_preserve_inventory() {
    let mut film = mixture();
    let before = format!("{film:?}");
    for bad in [
        vec![f64::NAN, 0.],
        vec![f64::INFINITY, 0.],
        vec![-1., 0.],
        vec![0.],
    ] {
        assert!(
            film.deposit_component_masses_batch(&[(0, vec![0.5, 0.]), (1, bad)])
                .is_err()
        );
        assert_eq!(format!("{film:?}"), before);
    }
    assert!(
        film.deposit_component_masses_batch(&[(0, vec![0.5, 0.]), (99, vec![0., 0.])])
            .is_err()
    );
    assert_eq!(format!("{film:?}"), before);
    for requests in [
        vec![],
        vec![(0, vec![-0., 0.])],
        vec![(0, vec![1e-100, 0.])],
    ] {
        let receipt = film.deposit_component_masses_batch(&requests).unwrap();
        assert_eq!(receipt.mass_kg, 0.);
        assert_eq!(receipt.component_masses_kg, vec![0., 0.]);
        assert_eq!(format!("{film:?}"), before);
    }
}
#[test]
fn kilogram_deposit_rejects_species_growth_below_bulk_resolution() {
    let source = mixture();
    let mut film = FilmMixture::new(
        source.film().clone(),
        source.component_names().to_vec(),
        vec![vec![1e-20, 1.], vec![1e-20, 1.]],
    )
    .unwrap();
    let before = format!("{film:?}");
    assert_eq!(
        film.deposit_component_masses_batch(&[(0, vec![1e-22, 0.])]),
        Err("film deposit balance cannot be represented")
    );
    assert_eq!(format!("{film:?}"), before);
}
#[test]
fn mass_depletion_does_not_overdraw_when_volume_roundtrip_rounds_up() {
    let mut source = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.]],
        vec![[0, 1, 2]],
        Material {
            density: 0.1,
            ..Material::default()
        },
    )
    .unwrap();
    source.deposit(0, 0.1).unwrap();
    let mut film = FilmMixture::new(source, vec!["solvent".into()], vec![vec![1.]]).unwrap();
    let volume = film.component_volumes_m3()[0][0];
    let density = film.film().material().density;
    let mass = film.component_masses_kg()[0][0];
    assert!(
        mass / density > 0.1,
        "fixture must reproduce roundtrip growth from supplied volume"
    );
    assert_eq!(volume, mass / density);
    let receipt = film
        .withdraw_component_masses_batch(&[(0, vec![mass])])
        .unwrap();
    assert_eq!(receipt.mass_kg, mass);
    assert_eq!(receipt.volume_m3, volume);
    assert_eq!(film.film().total_mass(), 0.);
}
#[test]
fn selective_removal_preserves_residue_and_bulk_species_ledger() {
    let mut film = mixture();
    let before = film.component_masses().unwrap();
    let bulk = film.film().total_mass();
    let transfer = film
        .withdraw_components_batch(&[(0, vec![0.001, 0.]), (1, vec![0.002, 0.])])
        .unwrap();
    assert_eq!(film.fractions()[0], vec![0., 1.]);
    assert_eq!(transfer.component_masses_kg[1], 0.);
    assert!((film.film().total_mass() + transfer.mass_kg - bulk).abs() < 1e-14);
    for (i, after) in film.component_masses().unwrap().iter().enumerate() {
        assert!((after + transfer.component_masses_kg[i] - before[i]).abs() < 1e-14);
    }
    let all: Vec<_> = film
        .component_volumes_m3()
        .iter()
        .cloned()
        .enumerate()
        .collect();
    film.withdraw_components_batch(&all).unwrap();
    assert_eq!(film.film().total_mass(), 0.);
    assert!(film.fractions().iter().flatten().all(|v| *v == 0.));
    film.deposit(0, 0.002, &[0.1, 0.9]).unwrap();
    assert!((film.fractions()[0][0] - 0.1).abs() < 1e-15);
}
#[test]
fn late_invalid_requests_and_repeated_overdraw_preserve_complete_state() {
    let mut film = mixture();
    let before = format!("{film:?}");
    for bad in [
        vec![f64::NAN, 0.],
        vec![-1., 0.],
        vec![f64::INFINITY, 0.],
        vec![0.],
        vec![1., 0.],
    ] {
        assert!(
            film.withdraw_components_batch(&[(0, vec![0.0005, 0.]), (1, bad)])
                .is_err()
        );
        assert_eq!(format!("{film:?}"), before);
    }
    assert!(
        film.withdraw_components_batch(&[(0, vec![0.0006, 0.]), (0, vec![0.0006, 0.])])
            .is_err()
    );
    assert_eq!(format!("{film:?}"), before);
    assert!(
        film.withdraw_components_batch(&[(99, vec![0., 0.])])
            .is_err()
    );
    assert_eq!(format!("{film:?}"), before);
    let transfer = film
        .withdraw_components_batch(&[(0, vec![0.0002, 0.]), (0, vec![0.0003, 0.])])
        .unwrap();
    assert!((transfer.component_volumes_m3[0] - 0.0005).abs() < 1e-18);
}
#[test]
fn zero_and_sub_ulp_flux_do_not_credit_phantom_mass() {
    let mut film = mixture();
    let before = format!("{film:?}");
    for requests in [
        vec![],
        vec![(0, vec![0., 0.])],
        vec![(0, vec![f64::MIN_POSITIVE, 0.])],
    ] {
        let transfer = film.withdraw_components_batch(&requests).unwrap();
        assert_eq!(transfer.mass_kg, 0.);
        assert_eq!(transfer.component_masses_kg, vec![0., 0.]);
        assert_eq!(format!("{film:?}"), before);
    }
}

#[test]
fn signed_zero_withdrawal_preserves_zero_inventory_representation() {
    let source = mixture();
    let mut film = FilmMixture::new(
        source.film().clone(),
        source.component_names().to_vec(),
        vec![vec![-0., 1.], vec![-0., 1.]],
    )
    .unwrap();
    assert!(film.component_volumes_m3()[0][0].is_sign_negative());
    let before = format!("{film:?}");
    let transfer = film
        .withdraw_components_batch(&[(0, vec![-0., 0.])])
        .unwrap();
    assert_eq!(transfer.mass_kg, 0.);
    assert_eq!(format!("{film:?}"), before);
}

#[test]
fn unrepresentable_incoming_mass_does_not_create_volume() {
    let mut source = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.]],
        vec![[0, 1, 2]],
        Material {
            density: f64::MIN_POSITIVE,
            ..Material::default()
        },
    )
    .unwrap();
    let before = format!("{source:?}");
    assert_eq!(
        source.deposit(0, 1e-100),
        Err("film deposit mass change cannot be represented")
    );
    assert_eq!(format!("{source:?}"), before);
}

#[test]
fn species_change_below_bulk_resolution_is_rejected_without_ghost_parcel() {
    let source = mixture();
    let mut film = FilmMixture::new(
        source.film().clone(),
        source.component_names().to_vec(),
        vec![vec![1e-20, 1.], vec![1e-20, 1.]],
    )
    .unwrap();
    let tiny = film.component_volumes_m3()[0][0];
    assert!(tiny > 0.);
    let before = format!("{film:?}");
    assert_eq!(
        film.withdraw_components_batch(&[(0, vec![tiny, 0.])]),
        Err("film withdrawal balance cannot be represented")
    );
    assert_eq!(format!("{film:?}"), before);
}
