use physics::surface_film::{Material, SurfaceFilm};
fn patch(material: Material) -> SurfaceFilm {
    SurfaceFilm::new(
        &[
            [0., 0., 0.],
            [0.01, 0., 0.],
            [0.01, -0.01, 0.],
            [0., -0.01, 0.],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        material,
    )
    .unwrap()
}
#[test]
fn deposit_receipt_tracks_actual_growth_and_late_unresolved_addition_rolls_back() {
    let mut film = patch(Material::default());
    film.deposit(0, 0.1).unwrap();
    let before = film.total_volume();
    let receipt = film.deposit_batch(&[(0, 0.02)]).unwrap();
    let actual = film.total_volume() - before;
    assert_ne!(
        actual, 0.02,
        "fixture must distinguish requested and actual volume"
    );
    assert_eq!(receipt, actual);
    let before = format!("{film:?}");
    assert_eq!(
        film.deposit_batch(&[(0, 0.01), (0, 1e-30)]),
        Err("film deposit volume change cannot be represented")
    );
    assert_eq!(format!("{film:?}"), before);
    assert_eq!(
        film.deposit(0, 1e-30),
        Err("film deposit volume change cannot be represented")
    );
    assert_eq!(format!("{film:?}"), before);
    assert_eq!(
        film.add_sources(0.1, &[(0, 0.1), (0, 1e-29)]),
        Err("film deposit volume change cannot be represented")
    );
    assert_eq!(format!("{film:?}"), before);
    assert_eq!(film.deposit_batch(&[(0, -0.), (1, 0.)]).unwrap(), 0.);
    assert_eq!(format!("{film:?}"), before);
}
#[test]
fn gravity_moves_film_down_and_retains_volume() {
    let mut f = patch(Material {
        surface_tension: 0.,
        wetting: 0.,
        ..Default::default()
    });
    // First cell center is above the second.
    f.deposit(0, 5e-8).unwrap();
    let total = f.total_volume();
    for _ in 0..100 {
        f.step(0.01, [0., -9.81, 0.]).unwrap();
    }
    assert!(f.thickness()[1] > 0.);
    assert!(f.thickness().iter().all(|h| *h >= 0.));
    assert!((f.total_volume() - total).abs() < total * 1e-12);
}
#[test]
fn equilibrium_and_wetting_are_conservative() {
    let mut f = patch(Material::default());
    f.deposit(0, 1e-9).unwrap();
    f.deposit(1, 1e-9).unwrap();
    let initial = f.thickness();
    f.step(0.1, [0.; 3]).unwrap();
    assert_eq!(initial, f.thickness());
    let mut f = patch(Material::default());
    f.deposit(0, 1e-9).unwrap();
    let total = f.total_volume();
    for _ in 0..100 {
        f.step(0.01, [0.; 3]).unwrap();
    }
    assert!(f.thickness()[1] > 0.);
    assert!((f.total_volume() - total).abs() < 1e-20);
}
#[test]
fn geometry_motion_retains_mass_and_invalid_input_is_atomic() {
    let mut f = patch(Material::default());
    f.deposit(0, 1e-9).unwrap();
    let before = f.thickness();
    assert!(f.step(f64::NAN, [0.; 3]).is_err());
    assert_eq!(before, f.thickness());
    assert!(f.update_geometry(&[[0.; 3]; 4]).is_err());
    assert_eq!(before, f.thickness());
    f.update_geometry(&[
        [0., 0., 0.],
        [0.02, 0., 0.],
        [0.02, -0.02, 0.],
        [0., -0.02, 0.],
    ])
    .unwrap();
    assert!((f.thickness()[0] - before[0] / 4.).abs() < 1e-15);
    assert_eq!(f.total_volume(), 1e-9);
    assert_eq!(f.vertex_thickness(4).unwrap().len(), 4);
}

#[test]
fn contact_exchange_limits_multiple_receivers_and_preserves_total_volume() {
    let mut a = patch(Material::default());
    let mut b = patch(Material::default());
    a.deposit(0, 1e-9).unwrap();
    let total = a.total_volume() + b.total_volume();
    let moved = a
        .exchange_with(&mut b, 0.1, &[(0, 0, 1.), (0, 1, 1.)])
        .unwrap();
    assert!((moved - 1e-9).abs() < 1e-20);
    assert!((a.total_volume() + b.total_volume() - total).abs() < 1e-20);
    assert!(
        a.thickness()
            .iter()
            .chain(b.thickness().iter())
            .all(|v| *v >= 0.)
    );
    let before = (a.thickness(), b.thickness());
    assert!(
        a.exchange_with(&mut b, 0.1, &[(0, 0, 1.), (999, 0, 1.)])
            .is_err()
    );
    assert_eq!(before, (a.thickness(), b.thickness()));
}
#[test]
fn sources_are_explicit_and_atomic() {
    let mut f = patch(Material::default());
    assert!((f.add_sources(0.1, &[(0, 1e-8), (1, 2e-8)]).unwrap() - 3e-9).abs() < 1e-20);
    let before = f.thickness();
    assert!(f.add_sources(0.1, &[(0, 1e-8), (99, 2e-8)]).is_err());
    assert_eq!(f.thickness(), before);
}

#[test]
fn automatic_bridges_require_proximity_opposed_faces_and_matching_density() {
    use physics::surface_film::BridgeConfig;
    let mut a = patch(Material::default());
    let points = [
        [0., 0., 0.0005],
        [0.01, 0., 0.0005],
        [0.01, -0.01, 0.0005],
        [0., -0.01, 0.0005],
    ];
    let mut b = SurfaceFilm::new(&points, vec![[2, 1, 0], [3, 2, 0]], Material::default()).unwrap();
    a.deposit(0, 1e-9).unwrap();
    let config = BridgeConfig::default();
    assert_eq!(a.detect_bridges(&b, config).unwrap().len(), 2);
    let total = a.total_volume() + b.total_volume();
    assert!(a.exchange_contact(&mut b, 0.01, config).unwrap() > 0.);
    assert!((a.total_volume() + b.total_volume() - total).abs() < 1e-20);
    let translated: Vec<_> = points.iter().map(|p| [p[0], p[1], p[2] + 0.01]).collect();
    b.update_geometry(&translated).unwrap();
    assert!(a.detect_bridges(&b, config).unwrap().is_empty());
    let facing_same =
        SurfaceFilm::new(&points, vec![[0, 1, 2], [0, 2, 3]], Material::default()).unwrap();
    assert!(a.detect_bridges(&facing_same, config).unwrap().is_empty());
    let different = SurfaceFilm::new(
        &points,
        vec![[2, 1, 0], [3, 2, 0]],
        Material {
            density: 900.,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(a.detect_bridges(&different, config).is_err());
}
#[test]
fn precursor_wetting_counts_added_liquid_and_keeps_uniform_layer_in_equilibrium() {
    use physics::surface_film::Wetting;
    let mut f = patch(Material {
        wetting: 0.,
        ..Default::default()
    });
    let wetting = Wetting {
        contact_angle: 0.2,
        precursor_thickness: 1e-6,
    };
    f.set_wetting(Some(wetting)).unwrap();
    let added = f.seed_precursor().unwrap();
    assert!((added - 1e-10).abs() < 1e-20);
    assert_eq!(f.seed_precursor().unwrap(), 0.);
    let before = f.thickness();
    f.step(0.01, [0.; 3]).unwrap();
    assert_eq!(before, f.thickness());
    assert_eq!(wetting.pressure(1e-6, 0.04).unwrap(), 0.);
    assert!(wetting.pressure(2e-6, 0.04).unwrap() > 0.);
    assert!(wetting.pressure(0.5e-6, 0.04).unwrap() < 0.);
    assert!(
        f.set_wetting(Some(Wetting {
            contact_angle: 2.,
            ..wetting
        }))
        .is_err()
    );
}
#[test]
fn wetting_diffusion_converges_to_analytic_two_cell_solution() {
    // Triangle area=5e-5, shared length=sqrt(2)*.01, centroid distance=sqrt(2)*.01/3.
    let initial = 1e-9;
    let time = 0.1;
    let rate: f64 = 2. * 1e-5 * 3. / 5e-5;
    let exact = initial * 0.5 * (1. + (-rate * time).exp());
    let mut errors = Vec::new();
    for step in [0.001, 0.0005, 0.00025] {
        let mut f = patch(Material {
            surface_tension: 0.,
            wetting: 1e-5,
            ..Default::default()
        });
        f.deposit(0, initial).unwrap();
        f.step_with_max_substep(time, [0.; 3], step).unwrap();
        errors.push((f.thickness()[0] * 5e-5 - exact).abs());
        assert!((f.total_volume() - initial).abs() < 1e-20);
    }
    assert!(errors[1] < 0.51 * errors[0]);
    assert!(errors[2] < 0.51 * errors[1]);
}

#[test]
fn bridge_exchange_is_invariant_under_planar_mesh_refinement() {
    use physics::surface_film::BridgeConfig;
    let mut results = Vec::new();
    for n in [1, 2, 4] {
        let mut points = Vec::new();
        let mut cells = Vec::new();
        for y in 0..=n {
            for x in 0..=n {
                points.push([x as f64 * 0.01 / n as f64, y as f64 * 0.01 / n as f64, 0.]);
            }
        }
        for y in 0..n {
            for x in 0..n {
                let a = y * (n + 1) + x;
                let b = a + 1;
                let c = a + n + 1;
                let d = c + 1;
                cells.extend([[a, b, d], [a, d, c]]);
            }
        }
        let count = cells.len();
        let opposite: Vec<_> = cells.iter().map(|t| [t[2], t[1], t[0]]).collect();
        let mut a = SurfaceFilm::new(&points, cells, Material::default()).unwrap();
        let shifted: Vec<_> = points.iter().map(|p| [p[0], p[1], 0.0005]).collect();
        let mut b = SurfaceFilm::new(&shifted, opposite, Material::default()).unwrap();
        for i in 0..count {
            a.deposit(i, 1e-9 / count as f64).unwrap();
        }
        let config = BridgeConfig {
            max_candidates: 10000,
            ..Default::default()
        };
        let links = a.detect_bridges(&b, config).unwrap();
        for _ in 0..100 {
            a.exchange_with(&mut b, 0.001, &links).unwrap();
        }
        results.push(b.total_volume());
        assert!((a.total_volume() + b.total_volume() - 1e-9).abs() < 1e-20);
    }
    assert!(results.windows(2).all(|r| (r[0] - r[1]).abs() < 1e-20));
}

#[test]
fn material_replacement_conserves_mass_and_rejects_invalid_edits() {
    use physics::surface_film::{Material, SurfaceFilm};
    let mut film = SurfaceFilm::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        vec![[0, 1, 2]],
        Material::default(),
    )
    .unwrap();
    film.deposit(0, 1e-7).unwrap();
    let mass = film.total_mass();
    let mut material = film.material();
    material.density = 2000.;
    material.viscosity = 0.2;
    film.set_material(material).unwrap();
    assert!((film.total_mass() - mass).abs() < 1e-15);
    assert!((film.total_volume() - 5e-8).abs() < 1e-20);
    material.viscosity = 0.;
    assert!(film.set_material(material).is_err());
    assert_eq!(film.material().viscosity, 0.2);
    assert!((film.total_mass() - mass).abs() < 1e-15);
}

#[test]
fn material_edit_allows_roundoff_in_dry_cells_without_losing_bulk_mass() {
    use physics::surface_film::{Material, SurfaceFilm};
    let mut film = SurfaceFilm::new(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [2., 0., 0.],
            [3., 0., 0.],
            [2., 1., 0.],
        ],
        vec![[0, 1, 2], [3, 4, 5]],
        Material::default(),
    )
    .unwrap();
    film.deposit(0, 1e-7).unwrap();
    film.deposit(1, f64::from_bits(1)).unwrap();
    let mass = film.total_mass();
    let mut material = film.material();
    material.density = 2000.;
    film.set_material(material).unwrap();
    assert!((film.total_mass() - mass).abs() < 1e-15);
}

#[test]
fn driving_pressure_matches_hydrostatic_potential_and_is_read_only() {
    let mut f = patch(Material {
        surface_tension: 0.,
        wetting: 0.,
        ..Default::default()
    });
    f.deposit(0, 1e-9).unwrap();
    let before = f.total_mass();
    let pressure = f.driving_pressure([0., -9.81, 0.]).unwrap();
    assert!((pressure[0] + 1000. * 9.81 * 0.01 / 3.).abs() < 1e-12);
    assert!((pressure[1] + 1000. * 9.81 * 0.02 / 3.).abs() < 1e-12);
    assert_eq!(f.driving_pressure([0.; 3]).unwrap(), [0., 0.]);
    assert!(f.driving_pressure([f64::NAN, 0., 0.]).is_err());
    assert_eq!(f.total_mass(), before);
}

#[test]
#[ignore = "expensive anisotropic fold convergence study"]
fn concave_fold_accumulates_film_by_capillarity_without_mass_loss() {
    let mut report = Vec::new();
    let mut accumulation = Vec::new();
    for columns in [25usize, 49, 97] {
        let rows = 5;
        let points: Vec<[f64; 3]> = (0..columns * rows)
            .map(|i| {
                let x = -0.01 + 0.02 * (i % columns) as f64 / (columns - 1) as f64;
                [
                    x,
                    -0.001 * (-(x / 0.003).powi(2)).exp(),
                    -0.005 + 0.01 * (i / columns) as f64 / (rows - 1) as f64,
                ]
            })
            .collect();
        let mut cells = Vec::new();
        for row in 0..rows - 1 {
            for col in 0..columns - 1 {
                let a = row * columns + col;
                let b = a + 1;
                let c = a + columns;
                let d = c + 1;
                cells.extend([[a, c, b], [b, c, d]]);
            }
        }
        let geometry: Vec<_> = cells
            .iter()
            .map(|t| {
                let a = std::array::from_fn::<_, 3, _>(|k| points[t[1]][k] - points[t[0]][k]);
                let b = std::array::from_fn::<_, 3, _>(|k| points[t[2]][k] - points[t[0]][k]);
                let cross = [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ];
                let area = 0.5 * cross.iter().map(|v| v * v).sum::<f64>().sqrt();
                let fraction = triangle_strip_fraction(t.map(|i| points[i]), -0.002, 0.002);
                (area, fraction)
            })
            .collect();
        let mut fractions = Vec::new();
        for tension in [0., 0.04] {
            let mut film = SurfaceFilm::new(
                &points,
                cells.clone(),
                Material {
                    viscosity: 0.001,
                    surface_tension: tension,
                    wetting: 0.,
                    ..Default::default()
                },
            )
            .unwrap();
            for (i, (area, _)) in geometry.iter().enumerate() {
                film.deposit(i, area * 0.0001).unwrap();
            }
            let volume = film.total_volume();
            let dt = 0.0001 * (24_f64 / (columns - 1) as f64).powi(4);
            let steps = (0.05 / dt).round() as usize;
            for _ in 0..steps {
                film.step_with_max_substep(dt, [0.; 3], dt).unwrap();
            }
            let heights = film.thickness();
            assert!(heights.iter().all(|h| h.is_finite() && *h >= 0.));
            let error = (film.total_volume() - volume).abs() / volume;
            assert!(error < 1e-12);
            let fraction = heights
                .iter()
                .zip(&geometry)
                .map(|(h, (area, fraction))| h * area * fraction)
                .sum::<f64>()
                / volume;
            fractions.push(fraction);
            report.push(format!(
                "{columns},{},{tension},{dt:.9},{steps},{fraction:.12},{error:.3e}",
                cells.len()
            ));
        }
        assert!(
            fractions[1] > fractions[0] + 0.0001,
            "fold has no accumulation: {fractions:?}"
        );
        accumulation.push(fractions[1] - fractions[0]);
    }
    let coarse_difference = (accumulation[1] - accumulation[0]).abs();
    let fine_difference = (accumulation[2] - accumulation[1]).abs();
    assert!(fine_difference < coarse_difference, "{accumulation:?}");
    assert!(fine_difference / accumulation[2] < 0.02, "{accumulation:?}");
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/film-fold-retention.csv");
    std::fs::write(path,format!("columns,triangles,surface_tension_N_per_m,dt_seconds,steps,central_volume_fraction,relative_mass_error\n{}\n",report.join("\n"))).unwrap();
}

#[test]
fn substrate_curvature_converges_on_anisotropic_cylindrical_graph() {
    let mut errors = Vec::new();
    for columns in [25usize, 49, 97] {
        let rows = 7;
        let points: Vec<_> = (0..columns * rows)
            .map(|i| {
                let x = -0.01 + 0.02 * (i % columns) as f64 / (columns - 1) as f64;
                [x, 20. * x * x, (i / columns) as f64 * 0.0025]
            })
            .collect();
        let mut cells = Vec::new();
        for row in 0..rows - 1 {
            for col in 0..columns - 1 {
                let a = row * columns + col;
                cells.extend([
                    [a, a + columns, a + 1],
                    [a + 1, a + columns, a + columns + 1],
                ]);
            }
        }
        let film = SurfaceFilm::new(
            &points,
            cells.clone(),
            Material {
                surface_tension: 1.,
                wetting: 0.,
                ..Default::default()
            },
        )
        .unwrap();
        let pressure = film.driving_pressure([0.; 3]).unwrap();
        let mut error = 0f64;
        for (t, value) in cells.iter().zip(pressure) {
            if t.iter().any(|v| {
                v % columns < 2
                    || v % columns >= columns - 2
                    || v / columns < 2
                    || v / columns >= rows - 2
            }) {
                continue;
            }
            let x = t.iter().map(|&v| points[v][0] / 3.).sum::<f64>();
            let expected = -40. / (1. + (40. * x).powi(2)).powf(1.5);
            error = error.max((value - expected).abs() / expected.abs());
        }
        errors.push(error);
    }
    eprintln!("anisotropic curvature relative errors: {errors:?}");
    assert!(errors[2] < 0.001, "{errors:?}");
    assert!(errors[1] < errors[0] && errors[2] < errors[1], "{errors:?}");
}

// Exact clipping of a planar triangle to a fixed world-x measurement strip.
// Counting whole cells by their centroids changes the measured domain on refinement.
fn triangle_strip_fraction(triangle: [[f64; 3]; 3], low: f64, high: f64) -> f64 {
    fn area(polygon: &[[f64; 3]]) -> f64 {
        if polygon.len() < 3 {
            return 0.;
        }
        (1..polygon.len() - 1)
            .map(|i| {
                let a: [f64; 3] = std::array::from_fn(|k| polygon[i][k] - polygon[0][k]);
                let b: [f64; 3] = std::array::from_fn(|k| polygon[i + 1][k] - polygon[0][k]);
                let cross = [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ];
                0.5 * cross.iter().map(|v| v * v).sum::<f64>().sqrt()
            })
            .sum()
    }
    let original = area(&triangle);
    let mut polygon = triangle.to_vec();
    for (bound, keep_above) in [(low, true), (high, false)] {
        let mut clipped = Vec::new();
        for i in 0..polygon.len() {
            let a = polygon[i];
            let b = polygon[(i + 1) % polygon.len()];
            let inside_a = if keep_above {
                a[0] >= bound
            } else {
                a[0] <= bound
            };
            let inside_b = if keep_above {
                b[0] >= bound
            } else {
                b[0] <= bound
            };
            if inside_a {
                clipped.push(a);
            }
            if inside_a != inside_b {
                let t = (bound - a[0]) / (b[0] - a[0]);
                clipped.push(std::array::from_fn(|k| a[k] + t * (b[k] - a[k])));
            }
        }
        polygon = clipped;
    }
    (area(&polygon) / original).clamp(0., 1.)
}

#[test]
fn fixed_strip_measurement_clips_partial_triangles() {
    let triangle = [[0., 0., 0.], [1., 0., 0.], [0., 0., 1.]];
    assert!((triangle_strip_fraction(triangle, 0., 0.5) - 0.75).abs() < 1e-14);
    assert_eq!(triangle_strip_fraction(triangle, 2., 3.), 0.);
    assert_eq!(triangle_strip_fraction(triangle, -1., 2.), 1.);
    let inclined = triangle.map(|[x, y, z]| [x, y + 3. * x, z]);
    assert!((triangle_strip_fraction(inclined, 0., 0.5) - 0.75).abs() < 1e-14);
}

#[test]
fn gravity_flux_uses_cross_edge_distance_on_skew_cells() {
    let points = [
        [0., 0., 0.],
        [0., 0., 0.01],
        [-0.001, 0., 0.],
        [0.001, 0., 0.01],
    ];
    let material = Material {
        surface_tension: 0.,
        wetting: 0.,
        viscosity: 0.05,
        ..Default::default()
    };
    let mut film = SurfaceFilm::new(&points, vec![[0, 1, 2], [1, 0, 3]], material).unwrap();
    let h: f64 = 0.0001;
    let area = 0.000005;
    film.deposit(0, h * area).unwrap();
    film.deposit(1, h * area).unwrap();
    let dt = 0.000001;
    let expected = dt * 0.01 * h.powi(3) / (3. * material.viscosity) * material.density;
    film.step(dt, [1., 0., 0.]).unwrap();
    let moved = (film.thickness()[1] - h) * area;
    assert!(
        (moved - expected).abs() < expected * 1e-7,
        "{moved} != {expected}"
    );
    assert!((film.total_volume() - 2. * h * area).abs() < 1e-23);
}

#[test]
fn gravity_parallel_to_shared_edge_has_no_spurious_cross_edge_flow() {
    let points = [
        [0., 0., 0.],
        [0., 0., 0.01],
        [-0.001, 0., 0.],
        [0.001, 0., 0.01],
    ];
    let mut film = SurfaceFilm::new(
        &points,
        vec![[0, 1, 2], [1, 0, 3]],
        Material {
            surface_tension: 0.,
            wetting: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    film.deposit(0, 5e-10).unwrap();
    film.deposit(1, 5e-10).unwrap();
    let before = film.thickness();
    film.step(0.001, [0., 0., 9.81]).unwrap();
    for (a, b) in film.thickness().iter().zip(before) {
        assert!((a - b).abs() < 1e-16);
    }
}

#[test]
#[ignore = "paired release benchmark for internal gravity reuse"]
fn gravity_substep_reuse_benchmark() {
    let columns = 41;
    let points: Vec<_> = (0..columns * columns)
        .map(|i| [(i % columns) as f64 * 0.01, 0., (i / columns) as f64 * 0.01])
        .collect();
    let mut cells = Vec::new();
    for row in 0..columns - 1 {
        for col in 0..columns - 1 {
            let a = row * columns + col;
            cells.extend([
                [a, a + columns, a + 1],
                [a + 1, a + columns, a + columns + 1],
            ]);
        }
    }
    let mut report = String::from(
        "sample,batched_seconds,single_substeps_seconds,maximum_thickness_difference_m\n",
    );
    for sample in 0..7 {
        let make = || {
            let mut f = SurfaceFilm::new(
                &points,
                cells.clone(),
                Material {
                    surface_tension: 0.,
                    wetting: 0.,
                    ..Default::default()
                },
            )
            .unwrap();
            for i in 0..cells.len() {
                f.deposit(i, 5e-9).unwrap();
            }
            f
        };
        let mut batched = make();
        let mut singles = make();
        let start = std::time::Instant::now();
        batched.step(0.05, [2., -9.81, 0.5]).unwrap();
        let batch = start.elapsed().as_secs_f64();
        let start = std::time::Instant::now();
        for _ in 0..50 {
            singles.step(0.001, [2., -9.81, 0.5]).unwrap();
        }
        let single = start.elapsed().as_secs_f64();
        let difference = batched
            .thickness()
            .iter()
            .zip(singles.thickness())
            .map(|(a, b)| (a - b).abs())
            .fold(0., f64::max);
        assert!(difference < 1e-15, "{difference}");
        report.push_str(&format!(
            "{sample},{batch:.9},{single:.9},{difference:.3e}\n"
        ));
    }
    println!("{report}");
    if let Ok(path) = std::env::var("VOXY_GRAVITY_CACHE_REPORT") {
        std::fs::write(path, report).unwrap();
    }
}

fn self_contact_fixture() -> (SurfaceFilm, Vec<[f64; 3]>) {
    let mut points = Vec::new();
    for z in [0., 0.00005, 0.00008] {
        points.extend([[0., 0., z], [0.01, 0., z], [0., 0.01, z]]);
    }
    let film = SurfaceFilm::new(
        &points,
        vec![[0, 1, 2], [3, 5, 4], [6, 8, 7]],
        Material::default(),
    )
    .unwrap();
    (film, points)
}
#[test]
fn self_contacts_share_a_limited_donor_without_losing_mass() {
    use physics::surface_film::BridgeConfig;
    let (mut film, _) = self_contact_fixture();
    film.deposit(0, 1e-8).unwrap();
    let config = BridgeConfig {
        transfer_speed: 1e8,
        max_candidates: 2,
        ..Default::default()
    };
    let links = film.detect_self_bridges(config).unwrap();
    assert_eq!(
        links.iter().map(|&(a, b, _)| (a, b)).collect::<Vec<_>>(),
        vec![(0, 1), (0, 2)]
    );
    let mass = film.total_mass();
    let moved = film.exchange_self_contact(0.01, config).unwrap();
    assert!((moved - 1e-8).abs() < 1e-20);
    let h = film.thickness();
    assert!(h[1] > 0. && h[2] > 0. && h.iter().all(|v| *v >= 0.));
    assert!((film.total_mass() - mass).abs() < mass * 1e-12);
}
#[test]
fn self_bridges_require_film_to_span_gap_and_follow_geometry() {
    use physics::surface_film::BridgeConfig;
    let (mut film, mut points) = self_contact_fixture();
    assert!(
        film.detect_self_bridges(BridgeConfig::default())
            .unwrap()
            .is_empty()
    );
    film.deposit(0, 1e-10).unwrap(); // 2 um: cannot reach surfaces 50/80 um away.
    assert!(
        film.detect_self_bridges(BridgeConfig::default())
            .unwrap()
            .is_empty()
    );
    film.deposit(0, 1e-8).unwrap();
    assert_eq!(
        film.detect_self_bridges(BridgeConfig::default())
            .unwrap()
            .len(),
        2
    );
    let mass = film.total_mass();
    for p in &mut points[3..] {
        p[2] += 0.002;
    }
    film.update_geometry(&points).unwrap();
    assert!(
        film.detect_self_bridges(BridgeConfig::default())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        film.exchange_self_contact(0.001, BridgeConfig::default())
            .unwrap(),
        0.
    );
    assert_eq!(film.total_mass(), mass);
}
#[test]
fn self_contact_excludes_shared_vertices_and_rejects_invalid_links_atomically() {
    use physics::surface_film::BridgeConfig;
    let points = [
        [0., 0., 0.],
        [0.01, 0., 0.],
        [0., 0.01, 0.],
        [0.008, 0., 0.],
        [0., 0.008, 0.],
    ];
    let mut film =
        SurfaceFilm::new(&points, vec![[0, 1, 2], [0, 4, 3]], Material::default()).unwrap();
    film.deposit(0, 1e-8).unwrap();
    assert!(
        film.detect_self_bridges(BridgeConfig {
            max_candidates: 1,
            ..Default::default()
        })
        .unwrap()
        .is_empty()
    );
    let before = film.thickness();
    for links in [
        vec![(0, 1, 1.), (1, 0, 1.)],
        vec![(0, 0, 1.)],
        vec![(0, 1, f64::NAN)],
        vec![(0, 2, 1.)],
    ] {
        assert!(film.exchange_self_with(0.01, &links).is_err());
        assert_eq!(film.thickness(), before);
    }
    assert!(
        film.exchange_self_contact(f64::NAN, BridgeConfig::default())
            .is_err()
    );
    assert_eq!(film.thickness(), before);
}

fn opposed_uniform_film_grid(divisions: usize) -> (SurfaceFilm, usize, f64) {
    let columns = divisions + 1;
    let mut points = Vec::new();
    for z in [0., 0.00005] {
        for row in 0..columns {
            for col in 0..columns {
                points.push([
                    0.01 * col as f64 / divisions as f64,
                    0.01 * row as f64 / divisions as f64,
                    z,
                ]);
            }
        }
    }
    let mut triangles = Vec::new();
    for layer in 0..2 {
        for row in 0..divisions {
            for col in 0..divisions {
                let a = layer * columns * columns + row * columns + col;
                let pair = [
                    [a, a + 1, a + columns],
                    [a + 1, a + columns + 1, a + columns],
                ];
                for mut t in pair {
                    if layer == 1 {
                        t.swap(1, 2);
                    }
                    triangles.push(t);
                }
            }
        }
    }
    let plate_cells = 2 * divisions * divisions;
    let area = 0.0001 / plate_cells as f64;
    let mut film = SurfaceFilm::new(&points, triangles, Material::default()).unwrap();
    for cell in 0..plate_cells {
        film.deposit(cell, area * 0.0002).unwrap();
    }
    (film, plate_cells, area)
}

#[test]
fn self_contact_exchange_converges_to_analytic_uniform_plate_solution() {
    use physics::surface_film::BridgeConfig;
    let config = BridgeConfig {
        transfer_speed: 0.0005,
        max_candidates: 20000,
        ..Default::default()
    };
    let initial_height = 0.0002;
    let rate = 2. * config.transfer_speed / config.max_gap * (1. - 0.00005 / config.max_gap);
    let expected_receiver = 0.5 * initial_height * (1. - (-rate).exp());
    let mut errors = Vec::new();
    let mut report = String::from(
        "dt_seconds,receiver_thickness_m,analytic_receiver_thickness_m,error_over_initial_thickness\n",
    );
    for dt in [0.1, 0.05, 0.025] {
        let (mut film, count, area) = opposed_uniform_film_grid(4);
        let mass = film.total_mass();
        for _ in 0..(1_f64 / dt).round() as usize {
            film.exchange_self_contact(dt, config).unwrap();
        }
        let h = film.thickness();
        assert!(h.iter().all(|v| *v >= 0. && v.is_finite()));
        let receiver = h[count..].iter().sum::<f64>() / count as f64;
        let error = (receiver - expected_receiver).abs() / initial_height;
        errors.push(error);
        report.push_str(&format!(
            "{dt},{receiver:.15e},{expected_receiver:.15e},{error:.15e}\n"
        ));
        assert!((film.total_mass() - mass).abs() < mass * 1e-12);
        assert!(
            h[count..]
                .iter()
                .all(|v| (v - receiver).abs() < initial_height * 1e-12)
        );
        assert!((h.iter().sum::<f64>() * area - film.total_volume()).abs() < 1e-20);
    }
    assert!(errors[1] < errors[0] && errors[2] < errors[1], "{errors:?}");
    assert!(errors[2] < 0.003, "{errors:?}");
    std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/film-self-contact-time-convergence.csv"),
        report,
    )
    .unwrap();
    eprintln!("uniform contact temporal errors relative to initial thickness: {errors:?}");
}

#[test]
fn self_contact_uniform_plate_transfer_is_mesh_invariant() {
    use physics::surface_film::BridgeConfig;
    let config = BridgeConfig {
        transfer_speed: 0.0005,
        max_candidates: 20000,
        ..Default::default()
    };
    let mut receiver_volumes = Vec::new();
    let mut report = String::from("divisions,triangles,receiver_volume_m3,relative_mass_error\n");
    for divisions in [1, 2, 4, 8] {
        let (mut film, count, area) = opposed_uniform_film_grid(divisions);
        let mass = film.total_mass();
        for _ in 0..40 {
            film.exchange_self_contact(0.025, config).unwrap();
        }
        let volume = film.thickness()[count..].iter().sum::<f64>() * area;
        let mass_error = (film.total_mass() - mass).abs() / mass;
        assert!(mass_error < 1e-12);
        receiver_volumes.push(volume);
        report.push_str(&format!(
            "{divisions},{},{volume:.15e},{mass_error:.3e}\n",
            2 * count
        ));
    }
    for volume in &receiver_volumes {
        assert!(
            (volume - receiver_volumes[0]).abs() < receiver_volumes[0] * 1e-12,
            "{receiver_volumes:?}"
        );
    }
    std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/film-self-contact-convergence.csv"),
        report,
    )
    .unwrap();
}

#[test]
fn conservative_remap_preserves_mass_and_uniform_refinement_thickness() {
    let mut source = patch(Material::default());
    source.deposit(0, 5e-8).unwrap();
    source.deposit(1, 5e-8).unwrap();
    let points = [
        [0., 0., 0.],
        [0.01, 0., 0.],
        [0.01, -0.01, 0.],
        [0., -0.01, 0.],
        [0.005, -0.005, 0.],
    ];
    let refined = source
        .remapped(
            &points,
            vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]],
            &[vec![(0, 0.5), (1, 0.5)], vec![(2, 0.5), (3, 0.5)]],
        )
        .unwrap();
    assert!((refined.total_mass() - source.total_mass()).abs() < 1e-15);
    for thickness in refined.thickness() {
        assert!((thickness - source.thickness()[0]).abs() < 1e-12);
    }
    let merged = refined
        .remapped(
            &points,
            vec![[0, 1, 2], [0, 2, 3]],
            &[vec![(0, 1.)], vec![(0, 1.)], vec![(1, 1.)], vec![(1, 1.)]],
        )
        .unwrap();
    assert_eq!(merged.thickness(), source.thickness());
    assert_eq!(source.total_volume(), 1e-7);
}
#[test]
fn conservative_remap_rejects_incomplete_and_invalid_maps_without_mutation() {
    let mut source = patch(Material::default());
    source.deposit(0, 5e-8).unwrap();
    let points = [
        [0., 0., 0.],
        [0.01, 0., 0.],
        [0.01, -0.01, 0.],
        [0., -0.01, 0.],
    ];
    let triangles = vec![[0, 1, 2], [0, 2, 3]];
    for map in [
        vec![],
        vec![vec![], vec![]],
        vec![vec![(0, 0.9)], vec![]],
        vec![vec![(2, 1.)], vec![]],
        vec![vec![(0, f64::NAN)], vec![]],
        vec![vec![(0, -0.1), (1, 1.1)], vec![]],
    ] {
        assert!(source.remapped(&points, triangles.clone(), &map).is_err());
        assert_eq!(source.total_volume(), 5e-8);
    }
    let remapped = source
        .remapped(&points, triangles, &[vec![(1, 1.)], vec![]])
        .unwrap();
    assert_eq!(remapped.total_volume(), source.total_volume());
    assert_eq!(remapped.thickness()[0], 0.);
}

#[test]
fn atomic_frame_rejects_post_source_overflow_without_changing_geometry_or_mass() {
    let mut film = patch(Material::default());
    film.deposit(0, 1e-8).unwrap();
    let volume = film.total_volume();
    let thickness = film.thickness();
    let points = [
        [0., 0., 0.],
        [0.02, 0., 0.],
        [0.02, -0.02, 0.],
        [0., -0.02, 0.],
    ];
    assert!(
        film.advance_on_geometry(&points, 0.01, &[(0, 1e200)], [0., -9.81, 0.])
            .is_err()
    );
    assert_eq!(film.total_volume(), volume);
    assert_eq!(film.thickness(), thickness);
    let mut expected = patch(Material::default());
    expected.deposit(0, 1e-8).unwrap();
    expected.update_geometry(&points).unwrap();
    let before_source = expected.total_volume();
    let expected_added = expected.add_sources(0.01, &[(0, 1e-9)]).unwrap();
    assert_eq!(expected_added, expected.total_volume() - before_source);
    expected.step(0.01, [0., -9.81, 0.]).unwrap();
    let added = film
        .advance_on_geometry(&points, 0.01, &[(0, 1e-9)], [0., -9.81, 0.])
        .unwrap();
    assert_eq!(film.thickness(), expected.thickness());
    assert_eq!(added, expected_added);
    assert!((film.total_volume() - volume - added).abs() < 1e-20);
}
