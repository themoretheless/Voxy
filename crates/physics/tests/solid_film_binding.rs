use physics::biomechanics::{
    Body, InertialBody, Material, MaxwellBranch, OgdenTerm, SolidFilmBinding, SupportTarget,
    ViscoelasticOgden,
};
use physics::surface_film::{FilmMixture, SurfaceFilm, ThermalFilmMixture};
fn solid(cells: Vec<[usize; 4]>) -> InertialBody {
    let points = vec![
        [0.; 3],
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [0., 0., -1.],
    ];
    let n = cells.len();
    let mut body = Body::new(
        points.clone(),
        vec![true; points.len()],
        cells
            .into_iter()
            .map(|cell| {
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
    body.set_viscoelastic_ogden_batch(&(0..n).map(|cell| (cell, law.clone())).collect::<Vec<_>>())
        .unwrap();
    let mut dynamics = InertialBody::new_viscoelastic_with_supports(
        body,
        &vec![1.; n],
        vec![[0.; 3]; points.len()],
    )
    .unwrap();
    dynamics
        .enable_maxwell_thermal(&vec![6.; n], &vec![400.; n])
        .unwrap();
    dynamics
}
fn specimen() -> InertialBody {
    solid(vec![[0, 1, 2, 3], [0, 1, 2, 4]])
}
fn wet(mut film: SurfaceFilm) -> ThermalFilmMixture {
    let n = film.triangles().len();
    for cell in 0..n {
        film.deposit(cell, film.cell_area_m2(cell).unwrap() * 1e-4)
            .unwrap();
    }
    let mixture = FilmMixture::new(film, vec!["water".into()], vec![vec![1.]; n]).unwrap();
    ThermalFilmMixture::new(mixture, vec![4000.], &vec![300.; n]).unwrap()
}
#[test]
fn complete_boundary_maps_to_unique_cells_and_uses_physical_area() {
    let body = specimen();
    let binding = SolidFilmBinding::new(&body).unwrap();
    let film = binding.new_film(&body, Default::default()).unwrap();
    assert_eq!(film.triangles().len(), 6);
    let contacts = binding.heat_contacts(&body, &film, &[2.; 6]).unwrap();
    for &(owner, face, g) in &contacts {
        assert_eq!(
            owner,
            if film.triangles()[face].contains(&3) {
                0
            } else {
                1
            }
        );
        assert_eq!(g, film.cell_area_m2(face).unwrap() / 2.);
        assert!(!film.triangles()[face].iter().all(|node| *node < 3));
    }
    let total = contacts.iter().map(|c| c.2).sum::<f64>();
    assert!((total - (2. + 3_f64.sqrt()) / 2.).abs() < 1e-12);
}
#[test]
fn stale_geometry_rejects_until_owned_refresh_preserves_mass_and_heat() {
    let mut body = specimen();
    let binding = SolidFilmBinding::new(&body).unwrap();
    let mut film = wet(binding.new_film(&body, Default::default()).unwrap());
    let mass = film.mixture().component_masses_kg().to_vec();
    let energy = film.energies_j().to_vec();
    let controls: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: p.map(|v| v * 1.001),
        })
        .collect();
    body.step_viscoelastic(Some(&controls), 0.1, 1e-4).unwrap();
    assert_eq!(
        binding
            .heat_contacts(&body, film.mixture().film(), &[2.; 6])
            .unwrap_err(),
        "stale bound film geometry"
    );
    let old_areas: Vec<_> = (0..6)
        .map(|cell| film.mixture().film().cell_area_m2(cell).unwrap())
        .collect();
    film.update_geometry(body.body().positions()).unwrap();
    assert_eq!(mass, film.mixture().component_masses_kg());
    assert_eq!(energy, film.energies_j());
    for (cell, area) in old_areas.into_iter().enumerate() {
        assert!(
            (film.mixture().film().cell_area_m2(cell).unwrap() / area - 1.001_f64.powi(2)).abs()
                < 1e-12
        );
    }
    let initial = film.energies_j().iter().sum::<f64>();
    binding
        .exchange_heat(&mut body, &mut film, &[2.; 6], 0.1, 1e-8)
        .unwrap();
    let solid_heat = body
        .maxwell_sensible_energy_j()
        .unwrap()
        .iter()
        .sum::<f64>();
    assert!(solid_heat < 0.);
    assert!((solid_heat + film.energies_j().iter().sum::<f64>() - initial).abs() < 1e-8);
}
#[test]
fn wrong_reference_and_reordered_film_topology_do_not_rebind_by_proximity() {
    let body = specimen();
    let binding = SolidFilmBinding::new(&body).unwrap();
    let other = solid(vec![[0, 1, 2, 4], [0, 1, 2, 3]]);
    assert!(binding.new_film(&other, Default::default()).is_err());
    let mut triangles = body.body().surface();
    triangles.reverse();
    let film = SurfaceFilm::new(body.body().positions(), triangles, Default::default()).unwrap();
    assert!(binding.heat_contacts(&body, &film, &[2.; 6]).is_err());
    let film = binding.new_film(&body, Default::default()).unwrap();
    for resistance in [vec![2.; 5], vec![0.; 6], vec![f64::NAN; 6]] {
        assert!(binding.heat_contacts(&body, &film, &resistance).is_err());
    }
}
#[test]
fn overlapping_duplicate_tetrahedra_are_rejected_by_existing_topology_audit() {
    // Both apex nodes must participate so inertial nodal mass remains valid.
    let body = solid(vec![[0, 1, 2, 3], [0, 1, 2, 3], [0, 1, 2, 4]]);
    assert!(SolidFilmBinding::new(&body).is_err());
}

#[test]
fn heterogeneous_internal_conductance_tracks_current_deformation() {
    let mut body = specimen();
    let binding = SolidFilmBinding::new(&body).unwrap();
    let before = binding.internal_heat_contacts(&body, &[1., 3.]).unwrap();
    assert_eq!(before.len(), 1);
    // Shared area 1/2; normal half distances 1/4. Resistances
    // are 1/4 and 1/12 m² K/W, giving 1.5 W/K.
    assert!((before[0].2 - 1.5).abs() < 1e-12);
    let controls: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: p.map(|v| v * 1.001),
        })
        .collect();
    body.step_viscoelastic(Some(&controls), 0.1, 1e-4).unwrap();
    let after = binding.internal_heat_contacts(&body, &[1., 3.]).unwrap();
    assert!((after[0].2 / before[0].2 - 1.001).abs() < 1e-12);
    let reversed = binding.internal_heat_contacts(&body, &[3., 1.]).unwrap();
    assert!((reversed[0].2 - after[0].2).abs() < 1e-12);
}
