use physics::biomechanics::{
    Body, InertialBody, Material, MaxwellBranch, OgdenTerm, TetraMesh, ViscoelasticOgden,
};
fn configured(specific_heat: f64, kelvin: Vec<f64>) -> InertialBody {
    let mesh = TetraMesh::ellipsoid([0.; 3], [1.; 3], 0).unwrap();
    let mut body = Body::new(
        mesh.points.clone(),
        vec![true; mesh.points.len()],
        mesh.cells
            .iter()
            .map(|&cell| {
                (
                    cell,
                    Material {
                        shear_pa: 100.,
                        bulk_pa: 1000.,
                        fibers: vec![],
                    },
                )
            })
            .collect(),
    )
    .unwrap();
    let law = ViscoelasticOgden::new(
        vec![OgdenTerm {
            shear_pa: 100.,
            exponent: 2.,
        }],
        1000.,
        vec![MaxwellBranch {
            shear_pa: 100.,
            relaxation_seconds: 0.2,
        }],
    )
    .unwrap();
    body.set_viscoelastic_ogden_batch(
        &(0..mesh.cells.len())
            .map(|cell| (cell, law.clone()))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut body = InertialBody::new_viscoelastic_with_supports(
        body,
        &vec![1.; mesh.cells.len()],
        vec![[0.; 3]; mesh.points.len()],
    )
    .unwrap();
    body.enable_maxwell_thermal(&vec![specific_heat; mesh.cells.len()], &kelvin)
        .unwrap();
    body
}
fn specimen() -> InertialBody {
    let mut kelvin = vec![300.; 8];
    kelvin[0] = 400.;
    configured(6., kelvin)
}
#[test]
fn two_cells_follow_analytic_relaxation_preserve_energy_and_increase_entropy() {
    let mut body = specimen();
    body.conduct_maxwell_heat(&[(0, 1, 1.)], 1., 1e-10).unwrap();
    let t = body.maxwell_temperatures_kelvin().unwrap();
    let excess = 50. * (-2_f64).exp();
    assert!((t[0] - 350. - excess).abs() < 1e-12);
    assert!((t[1] - 350. + excess).abs() < 1e-12);
    assert!((t[0] / 400.).ln() + (t[1] / 300.).ln() > 0.);
    assert!(
        body.maxwell_sensible_energy_j()
            .unwrap()
            .iter()
            .sum::<f64>()
            .abs()
            < 1e-12
    );
    assert!(t[2..].iter().all(|v| *v == 300.));
}
#[test]
fn chain_is_bounded_conservative_and_converges_with_time_refinement() {
    let mut endpoints = Vec::new();
    for steps in [10, 20, 40] {
        let mut body = specimen();
        for _ in 0..steps {
            body.conduct_maxwell_heat(&[(0, 1, 1.), (1, 2, 0.7)], 1. / f64::from(steps), 1e-10)
                .unwrap();
        }
        let temperatures = body.maxwell_temperatures_kelvin().unwrap();
        assert!(
            temperatures
                .iter()
                .all(|v| (300. - 1e-12..=400. + 1e-12).contains(v))
        );
        assert!(
            body.maxwell_sensible_energy_j()
                .unwrap()
                .iter()
                .sum::<f64>()
                .abs()
                < 1e-10
        );
        endpoints.push(temperatures);
    }
    let difference = |a: &Vec<f64>, b: &Vec<f64>| {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    assert!(
        difference(&endpoints[1], &endpoints[2]) < 0.3 * difference(&endpoints[0], &endpoints[1])
    );
}
#[test]
fn invalid_links_and_late_energy_guard_preserve_full_owner() {
    let mut body = specimen();
    let before = format!("{body:?}");
    for links in [
        vec![(0, 8, 1.)],
        vec![(0, 0, 1.)],
        vec![(0, 1, 1.), (1, 0, 1.)],
        vec![(0, 1, f64::NAN)],
    ] {
        assert!(body.conduct_maxwell_heat(&links, 1., 1e-10).is_err());
        assert_eq!(format!("{body:?}"), before);
    }
    let mut temperatures = vec![300.; 8];
    temperatures[0] = 400.;
    temperatures[2] = 1e308;
    let mut overflow = configured(1e308, temperatures);
    let mut first_link_proof = overflow.clone();
    first_link_proof
        .conduct_maxwell_heat(&[(0, 1, 1.)], 1., 1e-10)
        .unwrap();
    assert_ne!(first_link_proof.maxwell_sensible_energy_j().unwrap()[0], 0.);
    let overflow_before = format!("{overflow:?}");
    assert!(
        overflow
            .conduct_maxwell_heat(&[(0, 1, 1.), (2, 3, 1e308)], 1., 1e-10)
            .is_err()
    );
    assert_eq!(format!("{overflow:?}"), overflow_before);
    body.conduct_maxwell_heat(&[(0, 1, 1.)], 0., 1e-10).unwrap();
    assert_eq!(format!("{body:?}"), before);
}

fn liquid(cp: f64) -> physics::surface_film::ThermalFilmMixture {
    use physics::surface_film::{FilmMixture, Material, SurfaceFilm, ThermalFilmMixture};
    let mut film = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [1., 0., 1.]],
        vec![[0, 1, 2], [1, 3, 2]],
        Material::default(),
    )
    .unwrap();
    film.deposit(0, 1e-6).unwrap();
    let mixture = FilmMixture::new(film, vec!["water".into()], vec![vec![1.], vec![1.]]).unwrap();
    ThermalFilmMixture::new(mixture, vec![cp], &[300., 300.]).unwrap()
}
#[test]
fn owned_solid_film_exchange_matches_analytic_solution_and_preserves_inventory() {
    let mut body = specimen();
    let mut film = liquid(4000.);
    let inventory = format!("{:?}", film.mixture());
    let initial_film = film.energies_j()[0];
    body.exchange_maxwell_film_heat(&mut film, &[(0, 0, 1.)], 1., 1e-10)
        .unwrap();
    let difference = 100. * (-1.25_f64).exp();
    assert!(
        (body.maxwell_temperatures_kelvin().unwrap()[0] - 320. - 0.8 * difference).abs() < 1e-10
    );
    assert!((film.temperatures().unwrap()[0].unwrap() - 320. + 0.2 * difference).abs() < 1e-10);
    assert!(
        (body
            .maxwell_sensible_energy_j()
            .unwrap()
            .iter()
            .sum::<f64>()
            + film.energies_j()[0]
            - initial_film)
            .abs()
            < 1e-10
    );
    assert_eq!(inventory, format!("{:?}", film.mixture()));
    let before = (format!("{body:?}"), format!("{film:?}"));
    body.exchange_maxwell_film_heat(&mut film, &[(0, 1, 1.)], 1., 1e-10)
        .unwrap();
    assert_eq!(before, (format!("{body:?}"), format!("{film:?}")));
}
#[test]
fn solid_film_numerical_loss_and_invalid_contacts_roll_back_both_owners() {
    let mut body = specimen();
    let mut film = liquid(1e300);
    let before = (format!("{body:?}"), format!("{film:?}"));
    assert_eq!(
        body.exchange_maxwell_film_heat(&mut film, &[(0, 0, 1.)], 1., 1e-10)
            .unwrap_err(),
        "solid film heat transfer defect"
    );
    assert_eq!(before, (format!("{body:?}"), format!("{film:?}")));
    for links in [
        vec![(8, 0, 1.)],
        vec![(0, 2, 1.)],
        vec![(0, 0, 1.), (0, 0, 1.)],
        vec![(0, 0, f64::NAN)],
    ] {
        assert!(
            body.exchange_maxwell_film_heat(&mut film, &links, 1., 1e-10)
                .is_err()
        );
        assert_eq!(before, (format!("{body:?}"), format!("{film:?}")));
    }
}
#[test]
fn shared_film_contacts_refine_symmetric_order_and_conserve_total_heat() {
    let mut endpoints = Vec::new();
    for steps in [10, 20, 40] {
        let mut body = specimen();
        let mut film = liquid(4000.);
        let initial = film.energies_j().iter().sum::<f64>();
        for _ in 0..steps {
            body.exchange_maxwell_film_heat(
                &mut film,
                &[(0, 0, 1.), (1, 0, 0.7)],
                1. / f64::from(steps),
                1e-10,
            )
            .unwrap();
        }
        let solid = body.maxwell_temperatures_kelvin().unwrap();
        let fluid = film.temperatures().unwrap()[0].unwrap();
        assert!((300. - 1e-12..=400. + 1e-12).contains(&fluid));
        assert!(
            solid
                .iter()
                .all(|t| (300. - 1e-12..=400. + 1e-12).contains(t))
        );
        assert!(
            (body
                .maxwell_sensible_energy_j()
                .unwrap()
                .iter()
                .sum::<f64>()
                + film.energies_j().iter().sum::<f64>()
                - initial)
                .abs()
                < 1e-9
        );
        endpoints.push(vec![solid[0], solid[1], fluid]);
    }
    let difference = |a: &Vec<f64>, b: &Vec<f64>| {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    assert!(
        difference(&endpoints[1], &endpoints[2]) < 0.3 * difference(&endpoints[0], &endpoints[1])
    );
}

#[test]
fn late_solid_film_transfer_failure_discards_an_earlier_successful_contact() {
    use physics::surface_film::{FilmMixture, Material, SurfaceFilm, ThermalFilmMixture};
    let mut base = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [1., 0., 1.]],
        vec![[0, 1, 2], [1, 3, 2]],
        Material::default(),
    )
    .unwrap();
    base.deposit(0, 1e-6).unwrap();
    base.deposit(1, 1e-6).unwrap();
    let mixture = FilmMixture::new(
        base,
        vec!["water".into(), "large-capacity-fixture".into()],
        vec![vec![1., 0.], vec![0., 1.]],
    )
    .unwrap();
    let mut film = ThermalFilmMixture::new(mixture, vec![4000., 1e300], &[300., 300.]).unwrap();
    let mut body = specimen();
    let mut first_body = body.clone();
    let mut first_film = film.clone();
    first_body
        .exchange_maxwell_film_heat(&mut first_film, &[(0, 0, 1.)], 1., 1e-10)
        .unwrap();
    assert!(first_body.maxwell_sensible_energy_j().unwrap()[0] < 0.);
    assert!(first_film.temperatures().unwrap()[0].unwrap() > 300.);
    let before = (format!("{body:?}"), format!("{film:?}"));
    assert!(
        body.exchange_maxwell_film_heat(&mut film, &[(0, 0, 1.), (0, 1, 1.)], 1., 1e-10)
            .is_err()
    );
    assert_eq!(before, (format!("{body:?}"), format!("{film:?}")));
}

#[test]
fn geometry_network_scales_with_length_and_material_conductivity() {
    use physics::biomechanics::SolidFilmBinding;
    let body = specimen();
    let binding = SolidFilmBinding::new(&body).unwrap();
    let links = binding.internal_heat_contacts(&body, &[1.; 8]).unwrap();
    assert_eq!(links.len(), 12);
    // Octahedron sectors: shared radial triangle area 1/2, each
    // centroid lies 1/4 from its plane, hence unit conductance.
    assert!(links.iter().all(|(_, _, g)| (*g - 1.).abs() < 1e-12));
    let doubled = binding.internal_heat_contacts(&body, &[2.; 8]).unwrap();
    assert!(doubled.iter().all(|(_, _, g)| (*g - 2.).abs() < 1e-12));
    assert!(binding.internal_heat_contacts(&body, &[0.; 8]).is_err());
    assert!(binding.internal_heat_contacts(&body, &[1.; 7]).is_err());
    let mut body = body;
    body.conduct_maxwell_heat(&links, 0.1, 1e-10).unwrap();
    let temperatures = body.maxwell_temperatures_kelvin().unwrap();
    assert!(temperatures.iter().all(|t| *t >= 300. && *t <= 400.));
    assert!(
        body.maxwell_sensible_energy_j()
            .unwrap()
            .iter()
            .sum::<f64>()
            .abs()
            < 1e-10
    );
}
