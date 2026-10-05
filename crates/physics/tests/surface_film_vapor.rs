use physics::liquid::{
    SaturationCurve, SolutionVaporInterface, VaporCell, VaporExchangeAccuracy, VaporInterface,
};
use physics::surface_film::{
    FilmMixture, FilmSubstrateHeat, Material, SurfaceFilm, ThermalFilmMixture,
};
#[test]
fn solvent_free_residue_stays_in_vacuum_and_accepts_condensation() {
    for vapor_mass in [0., 1.] {
        let mut f = film();
        f.withdraw_components_batch(&[(0, vec![0.7, 0.])]).unwrap();
        let residue = f.mixture().component_masses().unwrap()[1];
        assert_eq!(f.mixture().component_masses().unwrap()[0], 0.);
        let mut v = vapor(vapor_mass);
        let mut velocity = [0.; 3];
        let before = totals(&f, velocity, v);
        let transferred = f
            .exchange_solution_vapor(
                0,
                &mut velocity,
                &mut v,
                &model(),
                0.1,
                VaporExchangeAccuracy::default(),
            )
            .unwrap();
        if vapor_mass == 0. {
            assert_eq!(transferred, 0.);
            assert_eq!(f.mixture().component_masses().unwrap()[0], 0.);
        } else {
            assert!(transferred < 0.);
            assert!(f.mixture().component_masses().unwrap()[0] > 0.);
        }
        assert_eq!(f.mixture().component_masses().unwrap()[1], residue);
        let after = totals(&f, velocity, v);
        assert!((before.0 - after.0).abs() < 1e-12);
        assert!((before.2 - after.2).abs() < 1e-10);
        for axis in 0..3 {
            assert!((before.1[axis] - after.1[axis]).abs() < 1e-12);
        }
    }
}
#[test]
fn substrate_vapor_coupling_refines_when_elapsed_time_is_subdivided() {
    let run = |steps: usize| {
        let mut f = film();
        let mut substrate = [60.];
        let mut velocities = [[0.; 3]];
        let mut v = vapor(0.);
        v.velocity = [0.; 3];
        let m = model();
        let accuracy = VaporExchangeAccuracy {
            relative_tolerance: 1e-10,
            mass_tolerance: 1e-16,
            temperature_tolerance: 1e-10,
            ..VaporExchangeAccuracy::default()
        };
        for _ in 0..steps {
            f.exchange_solution_vapor_with_substrate(
                FilmSubstrateHeat {
                    energies_j: &mut substrate,
                    capacities_j_per_k: &[4.],
                    conductances_w_per_k: &[10.],
                },
                &mut velocities,
                &mut v,
                &[(0, &m)],
                0.2 / steps as f64,
                accuracy,
            )
            .unwrap();
        }
        (
            f.temperatures().unwrap()[0].unwrap(),
            substrate[0] / 4.,
            v.mass,
        )
    };
    let reference = run(256);
    let mut last_error = f64::INFINITY;
    for steps in [1, 2, 4, 8] {
        let state = run(steps);
        let error = (state.0 - reference.0).abs()
            + (state.1 - reference.1).abs()
            + (state.2 - reference.2).abs();
        assert!(
            error < last_error / 3.,
            "steps={steps}, error={error}, previous={last_error}"
        );
        last_error = error;
    }
}
#[test]
fn hot_finite_substrate_feeds_evaporation_with_closed_mass_energy_and_momentum() {
    let mut f = film();
    let mut velocities = [[0.; 3]];
    let mut v = vapor(0.);
    v.velocity = [0.; 3];
    let mut substrate = [60.]; // capacity 4 J/K, temperature 15 K.
    let before = totals(&f, velocities[0], v);
    let before_energy = before.2 + substrate[0];
    let m = model();
    let mut unheated = f.clone();
    let mut unheated_v = v;
    let mut unheated_velocities = velocities;
    unheated
        .exchange_solution_vapor_batch(
            &mut unheated_velocities,
            &mut unheated_v,
            &[(0, &m)],
            0.1,
            VaporExchangeAccuracy::default(),
        )
        .unwrap();
    let transfer = f
        .exchange_solution_vapor_with_substrate(
            FilmSubstrateHeat {
                energies_j: &mut substrate,
                capacities_j_per_k: &[4.],
                conductances_w_per_k: &[10.],
            },
            &mut velocities,
            &mut v,
            &[(0, &m)],
            0.1,
            VaporExchangeAccuracy::default(),
        )
        .unwrap();
    assert!(transfer > unheated_v.mass);
    assert!(substrate[0] < 60.);
    let after = totals(&f, velocities[0], v);
    assert!((after.0 - before.0).abs() < 1e-12);
    assert!((after.2 + substrate[0] - before_energy).abs() < 1e-10);
    assert!((f.mixture().component_masses().unwrap()[1] - 0.3).abs() < 1e-14);
    for axis in 0..3 {
        assert!((after.1[axis] - before.1[axis]).abs() < 1e-12);
    }
}

#[test]
fn coupled_vapor_failure_rolls_back_prior_substrate_heating_and_all_owners() {
    let mut f = film();
    let before = format!("{f:?}");
    let mut velocities = [[0.; 3]];
    let mut v = vapor(0.);
    let old_v = v;
    let mut substrate = [60.];
    let mut m = model();
    m.interface.area = f64::NAN;
    assert!(
        f.exchange_solution_vapor_with_substrate(
            FilmSubstrateHeat {
                energies_j: &mut substrate,
                capacities_j_per_k: &[4.],
                conductances_w_per_k: &[10.]
            },
            &mut velocities,
            &mut v,
            &[(0, &m)],
            0.1,
            VaporExchangeAccuracy::default()
        )
        .is_err()
    );
    assert_eq!(format!("{f:?}"), before);
    assert_eq!(substrate, [60.]);
    assert_eq!(velocities, [[0.; 3]]);
    assert_eq!(v, old_v);
}
fn model() -> SolutionVaporInterface {
    SolutionVaporInterface {
        interface: VaporInterface {
            curve: SaturationCurve {
                reference_temperature: 10.,
                reference_pressure: 10.,
                latent_heat: 100.,
                vapor_gas_constant: 1.,
                min_temperature: 5.,
                max_temperature: 20.,
            },
            area: 0.1,
            accommodation: 0.01,
        },
        solvent: 0,
        molar_masses: vec![1., 1.],
    }
}
fn film() -> ThermalFilmMixture {
    let mut f = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.]],
        vec![[0, 1, 2]],
        Material {
            density: 1.,
            ..Material::default()
        },
    )
    .unwrap();
    f.deposit(0, 1.).unwrap();
    ThermalFilmMixture::new(
        FilmMixture::new(
            f,
            vec!["solvent".into(), "residue".into()],
            vec![vec![0.7, 0.3]],
        )
        .unwrap(),
        vec![2., 3.],
        &[10.],
    )
    .unwrap()
}
fn vapor(mass: f64) -> VaporCell {
    VaporCell {
        mass,
        volume: 1.,
        temperature: 10.,
        velocity: [-1., 1., 0.],
        specific_heat_cv: 1.,
    }
}
fn totals(f: &ThermalFilmMixture, velocity: [f64; 3], v: VaporCell) -> (f64, [f64; 3], f64) {
    let mass = f.mixture().component_masses().unwrap().iter().sum::<f64>();
    let kinetic = 0.5 * mass * velocity.iter().map(|x| x * x).sum::<f64>();
    (
        mass + v.mass,
        std::array::from_fn(|i| mass * velocity[i] + v.mass * v.velocity[i]),
        f.energies_j()[0] + kinetic + v.energy(100.).unwrap(),
    )
}
#[test]
fn film_evaporation_and_condensation_preserve_mass_energy_momentum_and_residue() {
    for mass in [0., 0.05, 2.] {
        let mut f = film();
        let mut velocity = [2., 0., 0.];
        let mut v = vapor(mass);
        let before = totals(&f, velocity, v);
        let transfer = f
            .exchange_solution_vapor(
                0,
                &mut velocity,
                &mut v,
                &model(),
                0.1,
                VaporExchangeAccuracy::default(),
            )
            .unwrap();
        assert!(if mass == 2. {
            transfer < 0.
        } else {
            transfer > 0.
        });
        assert!((f.mixture().component_masses().unwrap()[1] - 0.3).abs() < 1e-14);
        let after = totals(&f, velocity, v);
        assert!((after.0 - before.0).abs() < 1e-12);
        assert!((after.2 - before.2).abs() < 1e-10);
        for i in 0..3 {
            assert!((after.1[i] - before.1[i]).abs() < 1e-12);
        }
        if mass == 0. {
            assert!(f.temperatures().unwrap()[0].unwrap() < 10.);
        }
    }
}
#[test]
fn invalid_vapor_exchange_preserves_all_three_owners() {
    let mut f = film();
    let mut velocity = [2., 0., 0.];
    let mut v = vapor(0.05);
    let before = format!("{f:?}");
    let old_velocity = velocity;
    let old_vapor = v;
    for dt in [f64::NAN, -1., 0.] {
        assert!(
            f.exchange_solution_vapor(
                0,
                &mut velocity,
                &mut v,
                &model(),
                dt,
                VaporExchangeAccuracy::default()
            )
            .is_err()
        );
        assert_eq!(format!("{f:?}"), before);
        assert_eq!(velocity, old_velocity);
        assert_eq!(v, old_vapor);
    }
    let mut bad = model();
    bad.molar_masses[0] = 0.;
    assert!(
        f.exchange_solution_vapor(
            0,
            &mut velocity,
            &mut v,
            &bad,
            0.1,
            VaporExchangeAccuracy::default()
        )
        .is_err()
    );
    assert_eq!(format!("{f:?}"), before);
    assert_eq!(velocity, old_velocity);
    assert_eq!(v, old_vapor);
}

#[test]
fn sub_inventory_resolution_flux_does_not_create_vapor_mass() {
    let mut f = film();
    let mut velocity = [2., 0., 0.];
    let mut v = vapor(0.);
    let before = format!("{f:?}");
    let old_velocity = velocity;
    let old_vapor = v;
    let mut tiny = model();
    tiny.interface.area = 1e-20;
    assert!(
        f.exchange_solution_vapor(
            0,
            &mut velocity,
            &mut v,
            &tiny,
            0.1,
            VaporExchangeAccuracy::default()
        )
        .is_err()
    );
    assert_eq!(format!("{f:?}"), before);
    assert_eq!(velocity, old_velocity);
    assert_eq!(v, old_vapor);
}

fn two_cells() -> ThermalFilmMixture {
    let mut base = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [1., 0., 1.]],
        vec![[0, 1, 2], [1, 3, 2]],
        Material {
            density: 1.,
            ..Material::default()
        },
    )
    .unwrap();
    base.deposit(0, 1.).unwrap();
    base.deposit(1, 2.).unwrap();
    ThermalFilmMixture::new(
        FilmMixture::new(
            base,
            vec!["solvent".into(), "residue".into()],
            vec![vec![0.7, 0.3], vec![0.4, 0.6]],
        )
        .unwrap(),
        vec![2., 3.],
        &[10., 11.],
    )
    .unwrap()
}
fn shared_totals(
    f: &ThermalFilmMixture,
    velocities: &[[f64; 3]],
    v: VaporCell,
) -> (f64, [f64; 3], f64) {
    let density = f.mixture().film().material().density;
    let masses: Vec<_> = f
        .mixture()
        .component_volumes_m3()
        .iter()
        .map(|row| row.iter().sum::<f64>() * density)
        .collect();
    let momentum = std::array::from_fn(|i| {
        masses
            .iter()
            .zip(velocities)
            .map(|(m, u)| m * u[i])
            .sum::<f64>()
            + v.mass * v.velocity[i]
    });
    let kinetic = masses
        .iter()
        .zip(velocities)
        .map(|(m, u)| 0.5 * m * u.iter().map(|x| x * x).sum::<f64>())
        .sum::<f64>();
    (
        f.mixture().film().total_mass() + v.mass,
        momentum,
        f.energies_j().iter().sum::<f64>() + kinetic + v.energy(100.).unwrap(),
    )
}
#[test]
fn shared_vapor_batch_matches_ordered_exchange_and_rolls_back_late_failure() {
    let mut f = two_cells();
    let mut velocities = [[2., 0., 0.], [-1., 0.5, 0.]];
    let mut v = vapor(0.05);
    let before = format!("{f:?}");
    let old_velocities = velocities;
    let old_vapor = v;
    assert!(
        f.exchange_solution_vapor_batch(
            &mut velocities,
            &mut v,
            &[(0, &model()), (99, &model())],
            0.1,
            VaporExchangeAccuracy::default()
        )
        .is_err()
    );
    assert_eq!(format!("{f:?}"), before);
    assert_eq!(velocities, old_velocities);
    assert_eq!(v, old_vapor);
    assert!(
        f.exchange_solution_vapor_batch(
            &mut velocities,
            &mut v,
            &[(0, &model()), (0, &model())],
            0.1,
            VaporExchangeAccuracy::default()
        )
        .is_err()
    );
    let mut incompatible = model();
    incompatible.interface.curve.latent_heat = 101.;
    assert!(
        f.exchange_solution_vapor_batch(
            &mut velocities,
            &mut v,
            &[(0, &model()), (1, &incompatible)],
            0.1,
            VaporExchangeAccuracy::default()
        )
        .is_err()
    );
    assert_eq!(format!("{f:?}"), before);
    assert_eq!(velocities, old_velocities);
    assert_eq!(v, old_vapor);
    let initial_totals = shared_totals(&f, &velocities, v);
    let mut control = f.clone();
    let mut control_velocity = velocities;
    let mut control_vapor = v;
    for cell in [0, 1, 1, 0] {
        control
            .exchange_solution_vapor(
                cell,
                &mut control_velocity[cell],
                &mut control_vapor,
                &model(),
                0.05,
                VaporExchangeAccuracy::default(),
            )
            .unwrap();
    }
    let transferred = f
        .exchange_solution_vapor_batch(
            &mut velocities,
            &mut v,
            &[(0, &model()), (1, &model())],
            0.1,
            VaporExchangeAccuracy::default(),
        )
        .unwrap();
    assert_eq!(format!("{f:?}"), format!("{control:?}"));
    assert_eq!(velocities, control_velocity);
    assert_eq!(v, control_vapor);
    assert_eq!(transferred, v.mass - old_vapor.mass);
    let final_totals = shared_totals(&f, &velocities, v);
    assert!((initial_totals.0 - final_totals.0).abs() < 1e-12);
    assert!((initial_totals.2 - final_totals.2).abs() < 1e-10);
    for i in 0..3 {
        assert!((initial_totals.1[i] - final_totals.1[i]).abs() < 1e-12);
    }
    assert!((f.mixture().component_masses().unwrap()[1] - 1.5).abs() < 1e-14);
}

#[test]
fn symmetric_shared_vapor_sweeps_refine_the_interface_order_error() {
    let accuracy = VaporExchangeAccuracy {
        relative_tolerance: 1e-10,
        mass_tolerance: 1e-16,
        temperature_tolerance: 1e-10,
        ..VaporExchangeAccuracy::default()
    };
    let mut profile = model();
    profile.interface.area = 5.;
    profile.interface.accommodation = 0.02;
    let run = |steps: usize, reverse: bool| {
        let mut f = two_cells();
        let mut velocities = [[2., 0., 0.], [-1., 0.5, 0.]];
        let mut v = vapor(0.05);
        let requests = if reverse {
            [(1, &profile), (0, &profile)]
        } else {
            [(0, &profile), (1, &profile)]
        };
        for _ in 0..steps {
            f.exchange_solution_vapor_batch(
                &mut velocities,
                &mut v,
                &requests,
                0.1 / steps as f64,
                accuracy,
            )
            .unwrap();
        }
        (
            f.temperatures()
                .unwrap()
                .into_iter()
                .map(Option::unwrap)
                .collect::<Vec<_>>(),
            v.temperature,
        )
    };
    let mut gaps = Vec::new();
    for steps in [1, 2, 4, 8] {
        let (forward, t_forward) = run(steps, false);
        let (reverse, t_reverse) = run(steps, true);
        gaps.push(
            (t_forward - t_reverse).abs()
                + forward
                    .iter()
                    .zip(reverse)
                    .map(|(a, b)| (a - b).abs())
                    .sum::<f64>(),
        );
    }
    for pair in gaps.windows(2) {
        assert!(pair[0] / pair[1] > 3., "order gaps: {gaps:?}");
    }
}
