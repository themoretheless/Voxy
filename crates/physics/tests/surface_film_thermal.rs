use physics::surface_film::{FilmMixture, Material, SurfaceFilm, ThermalFilmMixture};
#[test]
fn finite_substrate_exchange_matches_analytic_decay_and_conserves_total_heat() {
    let mut film = thermal();
    let inventory = format!("{:?}", film.mixture());
    let initial = film.energies_j().to_vec();
    let capacities = [1000., 2000.];
    let mut substrate = [1000. * 400., 2000. * 280.];
    let previous = substrate;
    let conductances = [200., 500.];
    film.exchange_substrate_heat(&mut substrate, &capacities, &conductances, 3.)
        .unwrap();
    let t = film.temperatures().unwrap();
    for cell in 0..2 {
        let tf = [300., 350.][cell];
        let ca = initial[cell] / tf;
        let cb = capacities[cell];
        let ts = previous[cell] / cb;
        let equilibrium = (initial[cell] + previous[cell]) / (ca + cb);
        let difference = (tf - ts) * (-conductances[cell] * (1. / ca + 1. / cb) * 3.).exp();
        assert!((t[cell].unwrap() - (equilibrium + cb / (ca + cb) * difference)).abs() < 1e-10);
        assert!((substrate[cell] / cb - (equilibrium - ca / (ca + cb) * difference)).abs() < 1e-10);
        assert!(
            (film.energies_j()[cell] + substrate[cell] - initial[cell] - previous[cell]).abs()
                < 1e-8
        );
    }
    assert_eq!(format!("{:?}", film.mixture()), inventory);
    let mut whole = thermal();
    let mut whole_substrate = previous;
    whole
        .exchange_substrate_heat(&mut whole_substrate, &capacities, &conductances, 3.)
        .unwrap();
    let mut split = thermal();
    let mut split_substrate = previous;
    for _ in 0..4 {
        split
            .exchange_substrate_heat(&mut split_substrate, &capacities, &conductances, 0.75)
            .unwrap();
    }
    for cell in 0..2 {
        assert!((whole.energies_j()[cell] - split.energies_j()[cell]).abs() < 1e-8);
        assert!((whole_substrate[cell] - split_substrate[cell]).abs() < 1e-8);
    }
}

#[test]
fn late_invalid_substrate_rolls_back_both_owners_and_dry_cells_insulate() {
    let mut film = thermal();
    let before = format!("{film:?}");
    let mut substrate = [400000., 500000.];
    let saved = substrate;
    assert!(
        film.exchange_substrate_heat(&mut substrate, &[1000., 2000.], &[200., f64::NAN], 3.)
            .is_err()
    );
    assert_eq!(format!("{film:?}"), before);
    assert_eq!(substrate, saved);
    let row = film.mixture().component_volumes_m3()[0].clone();
    film.withdraw_components_batch(&[(0, row)]).unwrap();
    film.exchange_substrate_heat(&mut substrate, &[1000., 2000.], &[200., 500.], 3.)
        .unwrap();
    assert_eq!(substrate[0], saved[0]);
    assert_eq!(film.temperatures().unwrap()[0], None);
    assert!(substrate[1] != saved[1]);
}
#[test]
fn conduction_on_four_cells_refines_and_preserves_global_heat_and_bounds() {
    let mut film = SurfaceFilm::new(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 0., 1.],
            [0., 0., 1.],
            [0.5, 0., 0.5],
        ],
        vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]],
        Material::default(),
    )
    .unwrap();
    for cell in 0..4 {
        film.deposit(cell, 0.001 * (cell + 1) as f64).unwrap();
    }
    let mixture = FilmMixture::new(film, vec!["fluid".into()], vec![vec![1.]; 4]).unwrap();
    let initial = ThermalFilmMixture::new(mixture, vec![4000.], &[250., 300., 400., 330.]).unwrap();
    let heat = initial.energies_j().iter().sum::<f64>();
    let inventory = format!("{:?}", initial.mixture());
    let mut reference = initial.clone();
    reference.conduct_heat(10000., 5., 5. / 1024.).unwrap();
    let expected = reference.temperatures().unwrap();
    let mut previous_error = f64::INFINITY;
    for count in [2, 4, 8, 16] {
        let mut state = initial.clone();
        state.conduct_heat(10000., 5., 5. / count as f64).unwrap();
        assert!((state.energies_j().iter().sum::<f64>() - heat).abs() < 1e-7);
        assert_eq!(format!("{:?}", state.mixture()), inventory);
        let temperatures = state.temperatures().unwrap();
        let error: f64 = temperatures
            .iter()
            .zip(&expected)
            .map(|(t, r)| {
                let t = t.unwrap();
                assert!((250. ..=400.).contains(&t));
                (t - r.unwrap()).abs()
            })
            .sum();
        assert!(
            error < previous_error / 3.,
            "count={count}, error={error}, previous={previous_error}"
        );
        previous_error = error;
    }
}
fn thermal() -> ThermalFilmMixture {
    let mut film = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [1., 0., 1.]],
        vec![[0, 1, 2], [1, 3, 2]],
        Material::default(),
    )
    .unwrap();
    film.deposit(0, 0.004).unwrap();
    film.deposit(1, 0.006).unwrap();
    let mixture = FilmMixture::new(
        film,
        vec!["solvent".into(), "residue".into()],
        vec![vec![0.25, 0.75], vec![0.8, 0.2]],
    )
    .unwrap();
    ThermalFilmMixture::new(mixture, vec![4000., 1000.], &[300., 350.]).unwrap()
}
#[test]
fn conduction_matches_two_cell_fourier_solution_without_inventory_motion() {
    let mut state = thermal();
    let inventory = format!("{:?}", state.mixture());
    let before = state.energies_j().iter().sum::<f64>();
    let ca = state.energies_j()[0] / 300.;
    let cb = state.energies_j()[1] / 350.;
    let equilibrium = before / (ca + cb);
    // Two half-square cells: edge sqrt(2), center distance 2/(3 sqrt(2)).
    // Heights .008/.012 m, harmonic height .0096 m; k=100 W/(m K).
    let conductance = 100. * 0.0096 * 3.;
    let difference = -50. * (-conductance * (1. / ca + 1. / cb) * 10.).exp();
    state.conduct_heat(100., 10., 0.7).unwrap();
    let t = state.temperatures().unwrap();
    assert!((t[0].unwrap() - (equilibrium + cb / (ca + cb) * difference)).abs() < 1e-10);
    assert!((t[1].unwrap() - (equilibrium - ca / (ca + cb) * difference)).abs() < 1e-10);
    assert!((state.energies_j().iter().sum::<f64>() - before).abs() < 1e-8);
    assert_eq!(format!("{:?}", state.mixture()), inventory);
    state.conduct_heat(1e12, 100., 100.).unwrap();
    for t in state.temperatures().unwrap() {
        assert!((t.unwrap() - equilibrium).abs() < 1e-10);
    }
    let saved = format!("{state:?}");
    for controls in [
        (f64::NAN, 1., 1.),
        (1., -1., 1.),
        (1., 1., 0.),
        (1., 1e10, 1.),
    ] {
        assert!(
            state
                .conduct_heat(controls.0, controls.1, controls.2)
                .is_err()
        );
        assert_eq!(format!("{state:?}"), saved);
    }
}

#[test]
fn conduction_dry_cells_insulate_and_isothermal_cells_do_not_change() {
    let mut state = thermal();
    let row = state.mixture().component_volumes_m3()[1].clone();
    state.withdraw_components_batch(&[(1, row)]).unwrap();
    let saved = format!("{state:?}");
    state.conduct_heat(100., 10., 1.).unwrap();
    assert_eq!(format!("{state:?}"), saved);
    let base = thermal();
    let mut state =
        ThermalFilmMixture::new(base.mixture().clone(), vec![4000., 1000.], &[300., 300.]).unwrap();
    let saved = format!("{state:?}");
    state.conduct_heat(100., 10., 1.).unwrap();
    assert_eq!(format!("{state:?}"), saved);
}
#[test]
fn selective_transfer_carries_donor_heat_and_preserves_residue_temperature() {
    let mut film = thermal();
    let before = film.energies_j().iter().sum::<f64>();
    let transfer = film
        .withdraw_components_batch(&[(0, vec![0.001, 0.]), (1, vec![0.002, 0.])])
        .unwrap();
    let analytic = 1. * 4000. * 300. + 2. * 4000. * 350.;
    assert!((transfer.sensible_energy_j - analytic).abs() < 1e-8);
    assert!(
        (film.energies_j().iter().sum::<f64>() + transfer.sensible_energy_j - before).abs() < 1e-8
    );
    let temperatures = film.temperatures().unwrap();
    assert!((temperatures[0].unwrap() - 300.).abs() < 1e-12);
    assert!((temperatures[1].unwrap() - 350.).abs() < 1e-12);
    let all: Vec<_> = film
        .mixture()
        .component_volumes_m3()
        .iter()
        .cloned()
        .enumerate()
        .collect();
    let rest = film.withdraw_components_batch(&all).unwrap();
    assert_eq!(film.temperatures().unwrap(), vec![None, None]);
    assert_eq!(film.energies_j(), &[0., 0.]);
    assert!((rest.sensible_energy_j + transfer.sensible_energy_j - before).abs() < 1e-8);
}
#[test]
fn late_invalid_withdrawal_preserves_heat_and_inventory() {
    let mut film = thermal();
    let before = format!("{film:?}");
    assert!(
        film.withdraw_components_batch(&[(0, vec![0.0005, 0.]), (1, vec![f64::NAN, 0.])])
            .is_err()
    );
    assert_eq!(format!("{film:?}"), before);
    assert_eq!(
        film.withdraw_components_batch(&[])
            .unwrap()
            .sensible_energy_j,
        0.
    );
    assert_eq!(format!("{film:?}"), before);
}

#[test]
fn advection_carries_energy_to_dry_cells_and_refines_to_analytic_drainage() {
    let mut errors = Vec::new();
    for step in [0.001, 0.0005, 0.00025] {
        let mut base = SurfaceFilm::new(
            &[[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
            vec![[0, 1, 2], [0, 2, 3]],
            Material {
                surface_tension: 0.,
                wetting: 0.,
                ..Material::default()
            },
        )
        .unwrap();
        base.deposit(0, 0.0005).unwrap();
        let mixture =
            FilmMixture::new(base, vec!["water".into()], vec![vec![1.], vec![1.]]).unwrap();
        let mut film = ThermalFilmMixture::new(mixture, vec![4000.], &[300., 300.]).unwrap();
        let energy = film.energies_j().iter().sum::<f64>();
        film.step_with_advection(0.1, [0.; 3], &[[-1., 0., 0.]; 2], step)
            .unwrap();
        assert!(film.energies_j()[1] > 0.);
        assert!((film.energies_j().iter().sum::<f64>() - energy).abs() < 1e-8);
        for t in film.temperatures().unwrap() {
            assert!((t.unwrap() - 300.).abs() < 1e-10);
        }
        errors.push((film.energies_j()[0] - energy * (-0.2_f64).exp()).abs());
        let before = format!("{film:?}");
        assert!(
            film.step_with_advection(0.1, [0.; 3], &[[f64::NAN, 0., 0.]; 2], step)
                .is_err()
        );
        assert_eq!(format!("{film:?}"), before);
    }
    assert!(errors[0] / errors[1] > 1.9 && errors[1] / errors[2] > 1.9);
}

#[test]
fn species_diffusion_carries_heat_and_preserves_isothermal_state() {
    let original = thermal();
    let mut isothermal = ThermalFilmMixture::new(
        original.mixture().clone(),
        vec![4000., 1000.],
        &[300., 300.],
    )
    .unwrap();
    let before = isothermal.energies_j().iter().sum::<f64>();
    let fractions = isothermal.mixture().fractions();
    isothermal.diffuse(0.1, 0.5, 0.001).unwrap();
    assert_ne!(isothermal.mixture().fractions(), fractions);
    assert!((isothermal.energies_j().iter().sum::<f64>() - before).abs() < 1e-7);
    for t in isothermal.temperatures().unwrap() {
        assert!((t.unwrap() - 300.).abs() < 1e-9);
    }
    let mut nonuniform = thermal();
    let before = nonuniform.energies_j().iter().sum::<f64>();
    nonuniform.diffuse(0.1, 0.5, 0.001).unwrap();
    let temperatures = nonuniform.temperatures().unwrap();
    assert!(temperatures[0].unwrap() > 300. && temperatures[0].unwrap() < 350.);
    assert!(temperatures[1].unwrap() > 300. && temperatures[1].unwrap() < 350.);
    assert!((nonuniform.energies_j().iter().sum::<f64>() - before).abs() < 1e-7);
    let before = format!("{nonuniform:?}");
    assert!(nonuniform.diffuse(0.1, f64::INFINITY, 0.001).is_err());
    assert_eq!(format!("{nonuniform:?}"), before);
}

#[test]
fn thermal_deposit_mixes_by_heat_capacity_and_rolls_back_late_error() {
    let mut film = thermal();
    let unchanged = format!("{film:?}");
    assert!(
        film.deposit_batch(&[(0, 0.001, vec![1., 0.], 1e-300)])
            .is_err()
    );
    assert_eq!(format!("{film:?}"), unchanged);
    let before = film.energies_j()[0];
    let added = film
        .deposit_batch(&[(0, 0.001, vec![1., 0.], 400.)])
        .unwrap();
    assert!((added - 1. * 4000. * 400.).abs() < 1e-8);
    let expected = (before + added) / (2. * 4000. + 3. * 1000.);
    assert!((film.temperatures().unwrap()[0].unwrap() - expected).abs() < 1e-10);
    let before = format!("{film:?}");
    assert!(
        film.deposit_batch(&[
            (0, 0.001, vec![1., 0.], 350.),
            (1, 0.001, vec![0., 1.], f64::NAN)
        ])
        .is_err()
    );
    assert_eq!(format!("{film:?}"), before);
    let all: Vec<_> = film
        .mixture()
        .component_volumes_m3()
        .iter()
        .cloned()
        .enumerate()
        .collect();
    film.withdraw_components_batch(&all).unwrap();
    film.deposit_batch(&[(0, 0.001, vec![0., 1.], 325.)])
        .unwrap();
    assert_eq!(film.temperatures().unwrap(), vec![Some(325.), None]);
    assert_eq!(film.energies_j(), &[325000., 0.]);
}

#[test]
fn applied_heat_changes_temperature_and_rejects_late_overdraw() {
    let mut film = thermal();
    let before = film.energies_j().iter().sum::<f64>();
    let actual = film.add_heat_batch(&[(0, 7000.), (1, -1000.)]).unwrap();
    assert_eq!(actual, 6000.);
    assert_eq!(film.energies_j().iter().sum::<f64>() - before, actual);
    assert!((film.temperatures().unwrap()[0].unwrap() - 301.).abs() < 1e-12);
    let before = format!("{film:?}");
    assert!(film.add_heat_batch(&[(0, 500.), (1, -f64::MAX)]).is_err());
    assert_eq!(format!("{film:?}"), before);
    assert_eq!(film.add_heat_batch(&[(0, f64::MIN_POSITIVE)]).unwrap(), 0.);
    assert_eq!(format!("{film:?}"), before);
    let all: Vec<_> = film
        .mixture()
        .component_volumes_m3()
        .iter()
        .cloned()
        .enumerate()
        .collect();
    film.withdraw_components_batch(&all).unwrap();
    let before = format!("{film:?}");
    assert!(film.add_heat_batch(&[(0, 1.)]).is_err());
    assert_eq!(format!("{film:?}"), before);
}
