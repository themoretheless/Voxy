use physics::{
    moisture::{Body, Cell, FilmSupply, Link, VaporLink, VaporReservoir},
    surface_film::{FilmMixture, Material, SurfaceFilm},
};
fn film() -> FilmMixture {
    let f = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [1., 0., 1.]],
        vec![[0, 1, 2], [1, 3, 2]],
        Material::default(),
    )
    .unwrap();
    let mut f = FilmMixture::new(
        f,
        vec!["water".into(), "residue".into()],
        vec![vec![1., 0.]; 2],
    )
    .unwrap();
    f.deposit_component_masses_batch(&[(0, vec![0.1, 0.2]), (1, vec![10., 0.3])])
        .unwrap();
    f
}
fn body() -> Body {
    Body::new(
        vec![
            Cell {
                capacity_kg: 1.,
                water_kg: 0.,
            },
            Cell {
                capacity_kg: 2.,
                water_kg: 0.,
            },
        ],
        vec![Link {
            cells: [0, 1],
            conductance_kg_s: 1.,
        }],
    )
    .unwrap()
}
fn links() -> [FilmSupply; 2] {
    [
        FilmSupply {
            film_cell: 0,
            material_cell: 0,
            conductance_kg_s: 1.,
        },
        FilmSupply {
            film_cell: 1,
            material_cell: 1,
            conductance_kg_s: 1.,
        },
    ]
}
#[test]
fn film_uptake_matches_capped_network_and_drying_closes_water_and_latent_energy() {
    let mut f = film();
    let mut b = body();
    let initial = f.component_masses().unwrap();
    let r = b
        .advance_surface_film_supplies(1., &mut f, 0, &links())
        .unwrap();
    let s1 = 1.05 / 3.5;
    let s0 = (0.1 + s1) / 2.;
    assert!((b.cells()[0].water_kg - s0).abs() < 1e-12);
    assert!((b.cells()[1].water_kg - 2. * s1).abs() < 1e-12);
    assert_eq!(f.component_masses_kg()[0][0], 0.);
    assert_eq!(f.component_masses().unwrap()[1], initial[1]);
    assert!(r.mass_defect_kg.abs() < 1e-12);
    let combined =
        || f.component_masses().unwrap()[0] + b.cells().iter().map(|c| c.water_kg).sum::<f64>();
    assert!((combined() - initial[0]).abs() < 1e-12);
    let wet = b.cells().iter().map(|c| c.water_kg).sum::<f64>();
    let mut vapor = VaporReservoir::new(2., 0., 2.4e6, 5e6).unwrap();
    let energy = vapor.accounted_energy_j();
    let drying = b
        .advance_vapor(
            1.,
            &mut vapor,
            &[
                VaporLink {
                    material_cell: 0,
                    conductance_kg_s: 1.,
                },
                VaporLink {
                    material_cell: 1,
                    conductance_kg_s: 1.,
                },
            ],
        )
        .unwrap();
    assert!(vapor.water_kg() > 0.);
    assert!(b.cells().iter().map(|c| c.water_kg).sum::<f64>() < wet);
    assert!(
        (f.component_masses().unwrap()[0]
            + b.cells().iter().map(|c| c.water_kg).sum::<f64>()
            + vapor.water_kg()
            - initial[0])
            .abs()
            < 1e-12
    );
    assert!((vapor.accounted_energy_j() - energy).abs() < 1e-8);
    assert!((energy - vapor.thermal_j() - 2.4e6 * vapor.water_kg()).abs() < 1e-8);
    assert!(drying.mass_defect_kg.abs() < 1e-12);
}
#[test]
fn invalid_or_duplicate_ownership_preserves_both_inventories() {
    let mut f = film();
    let mut b = body();
    let before = format!("{b:?}{f:?}");
    for bad in [
        FilmSupply {
            film_cell: 9,
            ..links()[0]
        },
        FilmSupply {
            material_cell: 9,
            ..links()[1]
        },
        FilmSupply {
            conductance_kg_s: f64::NAN,
            ..links()[1]
        },
        links()[0],
    ] {
        assert!(
            b.advance_surface_film_supplies(1., &mut f, 0, &[links()[0], bad])
                .is_err()
        );
        assert_eq!(format!("{b:?}{f:?}"), before);
    }
    assert!(
        b.advance_surface_film_supplies(1., &mut f, 2, &links())
            .is_err()
    );
    for dt in [0., -1., f64::NAN] {
        assert!(
            b.advance_surface_film_supplies(dt, &mut f, 0, &links())
                .is_err()
        );
        assert_eq!(format!("{b:?}{f:?}"), before);
    }
}

#[test]
fn unrepresentable_film_drain_rejects_the_completed_material_candidate() {
    let mut f = film();
    f.deposit_component_masses_batch(&[(0, vec![1e20, 0.])])
        .unwrap();
    let mut b = body();
    let before = format!("{b:?}{f:?}");
    let link = FilmSupply {
        conductance_kg_s: 1e-4,
        ..links()[0]
    };
    assert!(
        b.advance_surface_film_supplies(1., &mut f, 0, &[link])
            .is_err()
    );
    assert_eq!(format!("{b:?}{f:?}"), before);
}
