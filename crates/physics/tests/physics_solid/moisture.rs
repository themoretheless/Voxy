use physics::moisture::{Body, Cell, Link, Reservoir};
#[test]
fn closed_heterogeneous_cells_conserve_water_and_approach_common_saturation() {
    let mut body = Body::new(
        vec![
            Cell {
                capacity_kg: 1.,
                water_kg: 1.,
            },
            Cell {
                capacity_kg: 2.,
                water_kg: 0.,
            },
        ],
        vec![Link {
            cells: [0, 1],
            conductance_kg_s: 0.5,
        }],
    )
    .unwrap();
    let r = body.advance(0.1, &[]).unwrap();
    // Saturation difference decays by 1/(1+dt*G*(1/C0+1/C1)).
    let difference = 1. / 1.075;
    let expected = (1. + 2. * difference) / 3.;
    assert!((body.cells()[0].water_kg - expected).abs() < 1e-12);
    assert!(r.mass_defect_kg.abs() < 1e-12);
    for _ in 0..20 {
        body.advance(10., &[]).unwrap();
    }
    assert!((body.cells().iter().map(|c| c.water_kg).sum::<f64>() - 1.).abs() < 1e-10);
    for c in body.cells() {
        assert!((c.water_kg / c.capacity_kg - 1. / 3.).abs() < 1e-10);
    }
}
fn uptake(dt: f64, steps: usize) -> f64 {
    let mut body = Body::new(
        vec![Cell {
            capacity_kg: 2.,
            water_kg: 0.,
        }],
        vec![],
    )
    .unwrap();
    let bath = Reservoir {
        cell: 0,
        saturation: 1.,
        conductance_kg_s: 1.,
    };
    let mut supplied = 0.;
    for _ in 0..steps {
        supplied += body.advance(dt, &[bath]).unwrap().reservoir_water_kg[0];
    }
    assert!((body.cells()[0].water_kg - supplied).abs() < 1e-12);
    body.cells()[0].water_kg
}
#[test]
fn bath_uptake_and_drying_are_bounded_accounted_and_time_refine() {
    let exact = 2. * (1. - (-0.5_f64).exp());
    let coarse = (uptake(0.1, 10) - exact).abs();
    let fine = (uptake(0.05, 20) - exact).abs();
    assert!(fine < 0.6 * coarse);
    let mut body = Body::new(
        vec![Cell {
            capacity_kg: 2.,
            water_kg: 2.,
        }],
        vec![],
    )
    .unwrap();
    let r = body
        .advance(
            100.,
            &[Reservoir {
                cell: 0,
                saturation: 0.,
                conductance_kg_s: 1.,
            }],
        )
        .unwrap();
    assert!((body.cells()[0].water_kg - 2. / 51.).abs() < 1e-12);
    assert!((r.reservoir_water_kg[0] - (2. / 51. - 2.)).abs() < 1e-12);
    let r = body
        .advance(
            100.,
            &[Reservoir {
                cell: 0,
                saturation: 1.,
                conductance_kg_s: 1.,
            }],
        )
        .unwrap();
    assert!(r.reservoir_water_kg[0] > 0. && body.cells()[0].water_kg <= 2.);
}
#[test]
fn invalid_and_overflowing_updates_preserve_inventory() {
    let mut body = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 0.25,
        }],
        vec![],
    )
    .unwrap();
    assert!(body.advance(-1., &[]).is_err());
    assert!(
        body.advance(
            1.,
            &[Reservoir {
                cell: 0,
                saturation: 1.1,
                conductance_kg_s: 1.
            }]
        )
        .is_err()
    );
    assert!(
        body.advance(
            1e300,
            &[Reservoir {
                cell: 0,
                saturation: 1.,
                conductance_kg_s: 1e300
            }]
        )
        .is_err()
    );
    assert_eq!(body.cells()[0].water_kg, 0.25);
    assert!(
        Body::new(
            vec![Cell {
                capacity_kg: 1.,
                water_kg: 2.
            }],
            vec![]
        )
        .is_err()
    );
}
