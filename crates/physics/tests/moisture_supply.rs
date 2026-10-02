use physics::moisture::{Body, Cell, Link, WaterSupply};
#[test]
fn finite_pool_exhaustion_limits_uptake_and_preserves_combined_water() {
    let mut body = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 0.,
        }],
        vec![],
    )
    .unwrap();
    let mut supplies = [WaterSupply {
        cell: 0,
        water_kg: 0.1,
        conductance_kg_s: 1.,
    }];
    let report = body.advance_with_supplies(10., &mut supplies).unwrap();
    assert_eq!(supplies[0].water_kg, 0.);
    assert!((body.cells()[0].water_kg - 0.1).abs() < 1e-12);
    assert!((report.supplied_water_kg[0] - 0.1).abs() < 1e-12);
    assert!(report.solves >= 2);
    assert!(report.mass_defect_kg.abs() < 1e-12);
    body.advance_with_supplies(10., &mut supplies).unwrap();
    assert!((body.cells()[0].water_kg - 0.1).abs() < 1e-12);
}
#[test]
fn capped_source_and_uncapped_source_couple_through_the_same_network() {
    let mut body = Body::new(
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
    .unwrap();
    let mut supplies = [
        WaterSupply {
            cell: 0,
            water_kg: 0.1,
            conductance_kg_s: 1.,
        },
        WaterSupply {
            cell: 1,
            water_kg: 10.,
            conductance_kg_s: 1.,
        },
    ];
    let report = body.advance_with_supplies(1., &mut supplies).unwrap();
    // Capped node: 2*s0-s1=0.1. Uncapped node: -s0+4*s1=1.
    let s1 = 1.05 / 3.5;
    let s0 = (0.1 + s1) / 2.;
    assert!((body.cells()[0].water_kg - s0).abs() < 1e-12);
    assert!((body.cells()[1].water_kg - 2. * s1).abs() < 1e-12);
    assert_eq!(supplies[0].water_kg, 0.);
    let total = body.cells().iter().map(|c| c.water_kg).sum::<f64>()
        + supplies.iter().map(|r| r.water_kg).sum::<f64>();
    assert!((total - 10.1).abs() < 1e-12);
    assert!(report.mass_defect_kg.abs() < 1e-12);
}
#[test]
fn parallel_pools_and_invalid_updates_are_atomic() {
    let mut body = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 0.,
        }],
        vec![],
    )
    .unwrap();
    let mut supplies = [
        WaterSupply {
            cell: 0,
            water_kg: 0.1,
            conductance_kg_s: 1.,
        },
        WaterSupply {
            cell: 0,
            water_kg: 0.2,
            conductance_kg_s: 2.,
        },
    ];
    body.advance_with_supplies(100., &mut supplies).unwrap();
    assert!((body.cells()[0].water_kg - 0.3).abs() < 1e-12);
    assert!(supplies.iter().all(|r| r.water_kg == 0.));
    let before = body.cells()[0].water_kg;
    supplies[0].water_kg = 1.;
    supplies[0].cell = 1;
    assert!(body.advance_with_supplies(1., &mut supplies).is_err());
    assert_eq!(supplies[0].water_kg, 1.);
    assert_eq!(body.cells()[0].water_kg, before);
    supplies[0].cell = 0;
    assert!(body.advance_with_supplies(f64::NAN, &mut supplies).is_err());
    assert_eq!(supplies[0].water_kg, 1.);
    assert_eq!(body.cells()[0].water_kg, before);
}
