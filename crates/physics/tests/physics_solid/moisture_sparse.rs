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

#[test]
fn connected_stiff_chain_matches_analytic_mode_without_iteration_budget() {
    let n = 257;
    let g = 10000.;
    let initial: Vec<_> = (0..n)
        .map(|i| 0.5 + 0.25 * (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).cos())
        .collect();
    let mut body = Body::new(
        initial
            .iter()
            .map(|&s| Cell {
                capacity_kg: 1.,
                water_kg: s,
            })
            .collect(),
        (0..n - 1)
            .map(|i| Link {
                cells: [i, i + 1],
                conductance_kg_s: g,
            })
            .collect(),
    )
    .unwrap();
    let attenuation = 1. / (1. + 2. * g * (1. - (std::f64::consts::PI / n as f64).cos()));
    let report = body.advance(1., &[]).unwrap();
    let error = body
        .cells()
        .iter()
        .zip(initial)
        .map(|(c, s)| (c.water_kg - (0.5 + (s - 0.5) * attenuation)).abs())
        .fold(0., f64::max);
    assert!(error < 1e-10, "analytic error {error}");
    assert!(report.mass_defect_kg.abs() < 1e-9);
}

#[test]
fn heterogeneous_branched_tree_matches_independent_dense_elimination() {
    let n = 129;
    let cells: Vec<_> = (0..n)
        .map(|i| Cell {
            capacity_kg: 0.1 + (i % 7) as f64 * 0.03,
            water_kg: (0.1 + (i % 7) as f64 * 0.03) * (0.1 + (i % 11) as f64 * 0.07),
        })
        .collect();
    let links: Vec<_> = (1..n)
        .map(|i| Link {
            cells: [(i - 1) / 2, i],
            conductance_kg_s: 0.01 + (i % 5) as f64 * 0.02,
        })
        .collect();
    let baths = [
        Reservoir {
            cell: 33,
            saturation: 0.9,
            conductance_kg_s: 0.3,
        },
        Reservoir {
            cell: 105,
            saturation: 0.05,
            conductance_kg_s: 0.2,
        },
    ];
    let dt = 0.7;
    let mut matrix = vec![vec![0.; n]; n];
    let mut rhs: Vec<_> = cells.iter().map(|c| c.water_kg).collect();
    for (i, c) in cells.iter().enumerate() {
        matrix[i][i] = c.capacity_kg;
    }
    for l in &links {
        let [a, b] = l.cells;
        let w = dt * l.conductance_kg_s;
        matrix[a][a] += w;
        matrix[b][b] += w;
        matrix[a][b] -= w;
        matrix[b][a] -= w;
    }
    for r in &baths {
        let w = dt * r.conductance_kg_s;
        matrix[r.cell][r.cell] += w;
        rhs[r.cell] += w * r.saturation;
    }
    // Independent dense Gaussian elimination, without the production tree ordering.
    for k in 0..n {
        for i in k + 1..n {
            let ratio = matrix[i][k] / matrix[k][k];
            for j in k..n {
                matrix[i][j] -= ratio * matrix[k][j];
            }
            rhs[i] -= ratio * rhs[k];
        }
    }
    let mut expected = vec![0.; n];
    for i in (0..n).rev() {
        expected[i] =
            (rhs[i] - (i + 1..n).map(|j| matrix[i][j] * expected[j]).sum::<f64>()) / matrix[i][i];
    }
    let mut body = Body::new(cells, links).unwrap();
    let report = body.advance(dt, &baths).unwrap();
    for (c, s) in body.cells().iter().zip(expected) {
        assert!((c.water_kg / c.capacity_kg - s).abs() < 1e-12);
    }
    assert!(report.mass_defect_kg.abs() < 1e-12);
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
    assert_eq!(format!("{body:?}"), before);
}

#[test]
fn cyclic_network_retains_sparse_solution_for_periodic_diffusion_mode() {
    let n = 129;
    let angle = std::f64::consts::TAU / n as f64;
    let mut body = Body::new(
        (0..n)
            .map(|i| Cell {
                capacity_kg: 1.,
                water_kg: 0.5 + 0.2 * (angle * i as f64).cos(),
            })
            .collect(),
        (0..n)
            .map(|i| Link {
                cells: [i, (i + 1) % n],
                conductance_kg_s: 2.,
            })
            .collect(),
    )
    .unwrap();
    body.advance(1., &[]).unwrap();
    let attenuation = 1. / (1. + 4. * (1. - angle.cos()));
    for (i, c) in body.cells().iter().enumerate() {
        assert!((c.water_kg - (0.5 + 0.2 * attenuation * (angle * i as f64).cos())).abs() < 1e-11);
    }
}

#[test]
fn stiff_cyclic_network_matches_exact_periodic_diffusion_mode() {
    let n = 257;
    let angle = std::f64::consts::TAU / n as f64;
    let g = 10000.;
    let mut body = Body::new(
        (0..n)
            .map(|i| Cell {
                capacity_kg: 1.,
                water_kg: 0.5 + 0.2 * (angle * i as f64).cos(),
            })
            .collect(),
        (0..n)
            .map(|i| Link {
                cells: [i, (i + 1) % n],
                conductance_kg_s: g,
            })
            .collect(),
    )
    .unwrap();
    let report = body.advance(1., &[]).unwrap();
    let attenuation = 1. / (1. + 2. * g * (1. - angle.cos()));
    let error = body
        .cells()
        .iter()
        .enumerate()
        .map(|(i, c)| (c.water_kg - (0.5 + 0.2 * attenuation * (angle * i as f64).cos())).abs())
        .fold(0., f64::max);
    assert!(error < 1e-10, "analytic error {error}");
    assert!(report.mass_defect_kg.abs() < 1e-9);
}

#[test]
fn heterogeneous_stiff_cycles_match_independent_dense_elimination() {
    let n = 129;
    let cells: Vec<_> = (0..n)
        .map(|i| Cell {
            capacity_kg: 0.1 + (i % 7) as f64 * 0.03,
            water_kg: (0.1 + (i % 7) as f64 * 0.03) * (0.1 + (i % 11) as f64 * 0.07),
        })
        .collect();
    let mut links: Vec<_> = (1..n)
        .map(|i| Link {
            cells: [(i - 1) / 2, i],
            conductance_kg_s: 0.01 + (i % 5) as f64 * 0.02,
        })
        .collect();
    for i in 65..128 {
        links.push(Link {
            cells: [i, i + 1],
            conductance_kg_s: 10000.,
        });
    }
    let baths = [
        Reservoir {
            cell: 33,
            saturation: 0.9,
            conductance_kg_s: 0.3,
        },
        Reservoir {
            cell: 105,
            saturation: 0.05,
            conductance_kg_s: 0.2,
        },
    ];
    let dt = 0.7;
    let mut matrix = vec![vec![0.; n]; n];
    let mut rhs: Vec<_> = cells.iter().map(|c| c.water_kg).collect();
    for (i, c) in cells.iter().enumerate() {
        matrix[i][i] = c.capacity_kg;
    }
    for l in &links {
        let [a, b] = l.cells;
        let w = dt * l.conductance_kg_s;
        matrix[a][a] += w;
        matrix[b][b] += w;
        matrix[a][b] -= w;
        matrix[b][a] -= w;
    }
    for r in &baths {
        let w = dt * r.conductance_kg_s;
        matrix[r.cell][r.cell] += w;
        rhs[r.cell] += w * r.saturation;
    }
    // Independent dense Gaussian elimination, without the production tree ordering.
    for k in 0..n {
        for i in k + 1..n {
            let ratio = matrix[i][k] / matrix[k][k];
            for j in k..n {
                matrix[i][j] -= ratio * matrix[k][j];
            }
            rhs[i] -= ratio * rhs[k];
        }
    }
    let mut expected = vec![0.; n];
    for i in (0..n).rev() {
        expected[i] =
            (rhs[i] - (i + 1..n).map(|j| matrix[i][j] * expected[j]).sum::<f64>()) / matrix[i][i];
    }
    let mut body = Body::new(cells, links).unwrap();
    let report = body.advance(dt, &baths).unwrap();
    for (c, s) in body.cells().iter().zip(expected) {
        assert!((c.water_kg / c.capacity_kg - s).abs() < 1e-9);
    }
    assert!(report.mass_defect_kg.abs() < 1e-9);
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
    assert_eq!(format!("{body:?}"), before);
}

#[test]
#[ignore = "controlled transport timing comparison"]
fn stiff_cyclic_transport_cost() {
    let n = 257;
    let angle = std::f64::consts::TAU / n as f64;
    let template = Body::new(
        (0..n)
            .map(|i| Cell {
                capacity_kg: 1.,
                water_kg: 0.5 + 0.2 * (angle * i as f64).cos(),
            })
            .collect(),
        (0..n)
            .map(|i| Link {
                cells: [i, (i + 1) % n],
                conductance_kg_s: 10000.,
            })
            .collect(),
    )
    .unwrap();
    let attenuation = 1. / (1. + 20000. * (1. - angle.cos()));
    let mut durations = Vec::new();
    let mut max_error = 0_f64;
    let mut max_defect = 0_f64;
    let mut checksum = 0.;
    for trial in 0..9 {
        let mut body = template.clone();
        let start = std::time::Instant::now();
        let report = body.advance(1., &[]).unwrap();
        let elapsed = start.elapsed().as_nanos();
        if trial > 0 {
            durations.push(elapsed);
        }
        for (i, c) in body.cells().iter().enumerate() {
            max_error = max_error
                .max((c.water_kg - (0.5 + 0.2 * attenuation * (angle * i as f64).cos())).abs());
            checksum += (i + 1) as f64 * c.water_kg;
        }
        max_defect = max_defect.max(report.mass_defect_kg.abs());
    }
    assert!(max_error < 1e-10);
    assert!(max_defect < 1e-9);
    durations.sort_unstable();
    eprintln!(
        "CYCLIC_COST cells={n} samples=8 median_ns={} min_ns={} max_ns={} max_error={max_error:e} max_defect={max_defect:e} checksum={checksum:.17e}",
        durations[4], durations[0], durations[7]
    );
}

#[test]
fn localized_wetting_in_stiff_cycle_matches_all_fourier_modes() {
    let n = 257;
    let g = 10000.;
    let angle = std::f64::consts::TAU / n as f64;
    let mut cells = vec![
        Cell {
            capacity_kg: 1.,
            water_kg: 0.
        };
        n
    ];
    cells[0].water_kg = 1.;
    let mut body = Body::new(
        cells,
        (0..n)
            .map(|i| Link {
                cells: [i, (i + 1) % n],
                conductance_kg_s: g,
            })
            .collect(),
    )
    .unwrap();
    let report = body.advance(1., &[]).unwrap();
    let mut max_error = 0_f64;
    for (i, c) in body.cells().iter().enumerate() {
        let expected = (1.
            + 2. * (1..=(n - 1) / 2)
                .map(|k| {
                    (angle * (k * i) as f64).cos() / (1. + 2. * g * (1. - (angle * k as f64).cos()))
                })
                .sum::<f64>())
            / n as f64;
        max_error = max_error.max((c.water_kg - expected).abs());
        assert!(c.water_kg > 0. && c.water_kg < 1.);
    }
    assert!(max_error < 1e-11, "all-mode error {max_error:e}");
    assert!(report.mass_defect_kg.abs() < 1e-11);
}
