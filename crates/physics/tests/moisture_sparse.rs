use physics::moisture::{Body, Cell, Link, Reservoir};
#[test]
fn large_closed_chain_matches_exact_discrete_diffusion_mode() {
    let n = 1024;
    let capacity = 0.001;
    let conductance = 0.01;
    let initial: Vec<_> = (0..n)
        .map(|i| 0.5 + 0.25 * (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).cos())
        .collect();
    let mut body = Body::new(
        initial
            .iter()
            .map(|s| Cell {
                capacity_kg: capacity,
                water_kg: capacity * s,
            })
            .collect(),
        (0..n - 1)
            .map(|i| Link {
                cells: [i, i + 1],
                conductance_kg_s: conductance,
            })
            .collect(),
    )
    .unwrap();
    let before = body.cells().iter().map(|c| c.water_kg).sum::<f64>();
    let start = std::time::Instant::now();
    let report = body.advance(1., &[]).unwrap();
    let attenuation =
        1. / (1. + 2. * conductance / capacity * (1. - (std::f64::consts::PI / n as f64).cos()));
    let error = body
        .cells()
        .iter()
        .zip(initial)
        .map(|(c, s)| (c.water_kg / capacity - (0.5 + (s - 0.5) * attenuation)).abs())
        .fold(0., f64::max);
    assert!(error < 2e-10, "analytic saturation error {error}");
    assert!((body.cells().iter().map(|c| c.water_kg).sum::<f64>() - before).abs() < 1e-12);
    assert!(report.mass_defect_kg.abs() < 1e-12);
    eprintln!(
        "SPARSE_MOISTURE cells={n} max_saturation_error={error:e} elapsed_us={}",
        start.elapsed().as_micros()
    );
}
#[test]
fn sparse_heterogeneous_bath_matches_direct_solution_and_overflow_is_atomic() {
    let cells = vec![
        Cell {
            capacity_kg: 0.02,
            water_kg: 0.01,
        },
        Cell {
            capacity_kg: 0.03,
            water_kg: 0.005,
        },
    ];
    let links = vec![Link {
        cells: [0, 1],
        conductance_kg_s: 0.001,
    }];
    let baths = [Reservoir {
        cell: 0,
        saturation: 1.,
        conductance_kg_s: 0.003,
    }];
    let mut direct = Body::new(cells.clone(), links.clone()).unwrap();
    let mut large = cells;
    large.extend(vec![
        Cell {
            capacity_kg: 0.001,
            water_kg: 0.
        };
        127
    ]);
    let mut sparse = Body::new(large, links).unwrap();
    let a = direct.advance(0.2, &baths).unwrap();
    let b = sparse.advance(0.2, &baths).unwrap();
    for (a, b) in direct.cells().iter().zip(sparse.cells()) {
        assert!((a.water_kg - b.water_kg).abs() < 1e-13);
    }
    assert!((a.reservoir_water_kg[0] - b.reservoir_water_kg[0]).abs() < 1e-13);
    assert!(sparse.cells()[2..].iter().all(|c| c.water_kg == 0.));
    let mut body = Body::new(
        vec![
            Cell {
                capacity_kg: 1.,
                water_kg: 0.
            };
            129
        ],
        vec![Link {
            cells: [0, 1],
            conductance_kg_s: f64::MAX,
        }],
    )
    .unwrap();
    let before = format!("{body:?}");
    assert!(body.advance(f64::MAX, &[]).is_err());
    assert_eq!(format!("{body:?}"), before);
    assert!(
        Body::new(
            vec![
                Cell {
                    capacity_kg: 1.,
                    water_kg: 0.
                };
                16385
            ],
            vec![]
        )
        .is_err()
    );
}

#[test]
fn sparse_adaptive_thermal_exchange_matches_small_network_with_isolated_dry_cells() {
    use physics::{
        liquid::SaturationCurve,
        moisture::{ThermalVapor, ThermalVaporAccuracy, VaporLink},
    };
    let curve = SaturationCurve {
        reference_temperature: 300.,
        reference_pressure: 3500.,
        latent_heat: 2.4e6,
        vapor_gas_constant: 461.,
        min_temperature: 280.,
        max_temperature: 320.,
    };
    let mut small = Body::new(
        vec![Cell {
            capacity_kg: 0.001,
            water_kg: 0.0005,
        }],
        vec![],
    )
    .unwrap();
    let mut cells = small.cells().to_vec();
    cells.extend(vec![
        Cell {
            capacity_kg: 0.001,
            water_kg: 0.
        };
        128
    ]);
    let mut large = Body::new(cells, vec![]).unwrap();
    let mut a = ThermalVapor::new(300., 1000., 10., 0.0001, curve).unwrap();
    let mut b = a.clone();
    let accuracy = ThermalVaporAccuracy::default();
    let links = [VaporLink {
        material_cell: 0,
        conductance_kg_s: 0.0001,
    }];
    small
        .advance_thermal_vapor_adaptive(0.01, &mut a, &links, accuracy)
        .unwrap();
    let report = large
        .advance_thermal_vapor_adaptive_with_receipt(0.01, &mut b, &links, accuracy)
        .unwrap();
    assert!((small.cells()[0].water_kg - large.cells()[0].water_kg).abs() < 1e-12);
    assert!((a.water_kg() - b.water_kg()).abs() < 1e-12);
    assert!((a.temperature_k() - b.temperature_k()).abs() < 1e-8);
    assert!(large.cells()[1..].iter().all(|c| c.water_kg == 0.));
    assert_eq!(report.elapsed_s, 0.01);
    assert!(report.transfer.mass_defect_kg.abs() < 1e-13);
    assert!(report.transfer.energy_defect_j.abs() < 1e-8);
}

#[test]
fn disconnected_stiff_pair_and_bath_use_independent_exact_blocks() {
    let mut cells = vec![
        Cell {
            capacity_kg: 1.,
            water_kg: 0.
        };
        129
    ];
    cells[0].water_kg = 1.;
    let mut body = Body::new(
        cells,
        vec![
            Link {
                cells: [0, 1],
                conductance_kg_s: 10000.,
            },
            Link {
                cells: [1, 2],
                conductance_kg_s: 0.,
            },
        ],
    )
    .unwrap();
    let report = body
        .advance(
            1.,
            &[Reservoir {
                cell: 128,
                saturation: 1.,
                conductance_kg_s: 1.,
            }],
        )
        .unwrap();
    assert!((body.cells()[0].water_kg - (0.5 + 0.5 / 20001.)).abs() < 1e-9);
    assert!((body.cells()[1].water_kg - (0.5 - 0.5 / 20001.)).abs() < 1e-9);
    assert_eq!(body.cells()[128].water_kg, 0.5);
    assert!(body.cells()[2..128].iter().all(|c| c.water_kg == 0.));
    assert_eq!(report.reservoir_water_kg, vec![0.5]);
    let before = format!("{body:?}");
    assert!(
        body.advance(
            f64::MAX,
            &[Reservoir {
                cell: 128,
                saturation: 1.,
                conductance_kg_s: f64::MAX
            }]
        )
        .is_err()
    );
    assert_eq!(before, format!("{body:?}"));
}
