use physics::surface_film::{Material, SqueezePressureControl, SurfaceFilm};
fn triangular_film(n: usize, gap: f64, mu: f64) -> (SurfaceFilm, Vec<[f64; 3]>, Vec<f64>) {
    let mut points = Vec::new();
    let mut index = Vec::new();
    for j in 0..=n {
        let row: Vec<_> = (0..=n - j)
            .map(|i| {
                let id = points.len();
                points.push([
                    (i as f64 + 0.5 * j as f64) / n as f64,
                    0.0,
                    j as f64 * (3.0_f64.sqrt() / 2.0) / n as f64,
                ]);
                id
            })
            .collect();
        index.push(row);
    }
    let mut triangles = Vec::new();
    for j in 0..n {
        for i in 0..n - j {
            triangles.push([index[j][i], index[j][i + 1], index[j + 1][i]]);
            if i + 1 < n - j {
                triangles.push([index[j][i + 1], index[j + 1][i + 1], index[j + 1][i]]);
            }
        }
    }
    let centers: Vec<_> = triangles
        .iter()
        .map(|t| std::array::from_fn(|k| t.iter().map(|&v| points[v][k] / 3.0).sum()))
        .collect();
    let areas: Vec<_> = triangles
        .iter()
        .map(|t| {
            let a = points[t[0]];
            let b = points[t[1]];
            let c = points[t[2]];
            0.5 * ((b[0] - a[0]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[0] - a[0])).abs()
        })
        .collect();
    let mut film = SurfaceFilm::new(
        &points,
        triangles,
        Material {
            viscosity: mu,
            ..Material::default()
        },
    )
    .unwrap();
    for (cell, area) in areas.iter().enumerate() {
        film.deposit(cell, area * gap).unwrap();
    }
    (film, centers, areas)
}
#[test]
fn equilateral_squeeze_pressure_refines_to_independent_cubic_solution() {
    let gap = 0.001_f64;
    let mu = 0.05;
    let speed = 1e-5;
    let mobility = gap.powi(3) / (12.0 * mu);
    let mut errors = Vec::new();
    for n in [4, 8, 16, 32] {
        let (film, centers, areas) = triangular_film(n, gap, mu);
        let report = film
            .solve_squeeze_pressure(
                &vec![gap; areas.len()],
                &vec![speed; areas.len()],
                None,
                SqueezePressureControl::default(),
            )
            .unwrap();
        // For a unit equilateral triangle, Laplacian(lambda1*lambda2*lambda3)=-4/3.
        let exact: Vec<_> = centers
            .iter()
            .map(|c| {
                let l3 = 2.0 * c[2] / 3.0_f64.sqrt();
                let l2 = c[0] - c[2] / 3.0_f64.sqrt();
                let l1 = 1.0 - l2 - l3;
                3.0 * speed / (4.0 * mobility) * l1 * l2 * l3
            })
            .collect();
        let squared: f64 = report
            .pressure
            .iter()
            .zip(&exact)
            .zip(&areas)
            .map(|((p, e), a)| a * (p - e).powi(2))
            .sum();
        let scale: f64 = exact.iter().zip(&areas).map(|(p, a)| a * p * p).sum();
        errors.push((squared / scale).sqrt());
        let area: f64 = areas.iter().sum();
        assert!((report.vented_volume_rate - area * speed).abs() < 1e-8 * area * speed);
        assert!(
            (report.dissipated_power - report.normal_load * speed).abs()
                < 1e-8 * report.dissipated_power
        );
        assert!(report.pressure.iter().all(|p| *p >= 0.0));
        assert!(report.relative_residual <= 1e-10);
        if n == 32 {
            // Integral of the barycentric product is area/60.
            let load = speed * area / (80.0 * mobility);
            assert!((report.normal_load - load).abs() / load < 0.01);
        }
    }
    for e in errors.windows(2) {
        assert!(
            e[0] / e[1] > 3.0 && e[0] / e[1] < 5.0,
            "spatial errors: {errors:?}"
        );
    }
}
#[test]
fn pressure_scales_with_viscosity_speed_and_inverse_gap_cubed() {
    let (film, _, areas) = triangular_film(8, 0.002, 0.05);
    let n = areas.len();
    let solve = |gap, speed, mu| {
        film.solve_squeeze_pressure(
            &vec![gap; n],
            &vec![speed; n],
            Some(&vec![mu; n]),
            SqueezePressureControl::default(),
        )
        .unwrap()
    };
    let base = solve(0.001, 1e-5, 0.05);
    for (gap, speed, mu, factor) in [
        (0.001, 2e-5, 0.05, 2.0),
        (0.001, 1e-5, 0.1, 2.0),
        (0.002, 1e-5, 0.05, 0.125),
    ] {
        let report = solve(gap, speed, mu);
        for (a, b) in report.pressure.iter().zip(&base.pressure) {
            assert!((a - factor * b).abs() < 1e-8 * b.abs().max(1.0));
        }
    }
    let zero = solve(0.001, 0.0, 0.05);
    assert_eq!(zero.pressure, vec![0.0; n]);
    assert_eq!(zero.iterations, 0);
}
#[test]
fn invalid_unfilled_sealed_and_nonconverged_queries_preserve_film() {
    let (film, _, areas) = triangular_film(8, 0.001, 0.05);
    let n = areas.len();
    let before = film.thickness();
    assert!(
        film.solve_squeeze_pressure(
            &vec![0.002; n],
            &vec![1e-5; n],
            None,
            SqueezePressureControl::default()
        )
        .is_err()
    );
    assert!(
        film.solve_squeeze_pressure(
            &vec![0.001; n],
            &vec![-1e-5; n],
            None,
            SqueezePressureControl::default()
        )
        .is_err()
    );
    assert!(
        film.solve_squeeze_pressure(
            &vec![0.001; n],
            &vec![1e-5; n],
            None,
            SqueezePressureControl {
                max_iterations: 1,
                ..SqueezePressureControl::default()
            }
        )
        .is_err()
    );
    assert_eq!(film.thickness(), before);
    let mut closed = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
        Material::default(),
    )
    .unwrap();
    for i in 0..4 {
        closed.deposit(i, 0.1).unwrap();
    }
    assert!(
        closed
            .solve_squeeze_pressure(
                &[0.001; 4],
                &[1e-5; 4],
                None,
                SqueezePressureControl::default()
            )
            .is_err()
    );
}

#[test]
fn heterogeneous_viscosity_matches_independent_two_cell_resistance_network() {
    let gap = 0.001_f64;
    let speed = 1e-5;
    let mut film = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material::default(),
    )
    .unwrap();
    for i in 0..2 {
        film.deposit(i, 0.5 * gap).unwrap();
    }
    let ma = gap.powi(3) / (12.0 * 0.05);
    let mb = gap.powi(3) / (12.0 * 0.1);
    let edge = 6.0 * ma * mb / (ma + mb);
    let va = 6.0 * ma;
    let vb = 6.0 * mb;
    let determinant = (edge + va) * (edge + vb) - edge * edge;
    let exact = [
        0.5 * speed * (2.0 * edge + vb) / determinant,
        0.5 * speed * (2.0 * edge + va) / determinant,
    ];
    let report = film
        .solve_squeeze_pressure(
            &[gap; 2],
            &[speed; 2],
            Some(&[0.05, 0.1]),
            SqueezePressureControl::default(),
        )
        .unwrap();
    for (actual, exact) in report.pressure.iter().zip(exact) {
        assert!((actual - exact).abs() < 1e-10 * exact);
    }
    assert!((report.vented_volume_rate - speed).abs() < 1e-12 * speed);
}

fn single_cell_film() -> SurfaceFilm {
    let mut film = SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material {
            viscosity: 0.05,
            ..Material::default()
        },
    )
    .unwrap();
    film.deposit(0, 0.0005).unwrap();
    film
}
#[test]
fn squeeze_transport_closes_volume_and_refines_to_continuous_pressure_work() {
    let h0 = 0.001_f64;
    let w = 0.0005;
    let dt = 0.1;
    let hf = h0 - w * dt;
    // One right triangle's vents give load=mu*w/(4*h³).
    let exact_impulse = 0.05 / 8.0 * (hf.powi(-2) - h0.powi(-2));
    let mut errors = Vec::new();
    for step in [0.001, 0.0005, 0.00025] {
        let mut film = single_cell_film();
        let report = film
            .step_squeeze(dt, &[w], SqueezePressureControl::default(), step)
            .unwrap();
        assert!((film.thickness()[0] - hf).abs() < 1e-15);
        assert!((film.total_volume() + report.vented_volume - 0.0005).abs() < 1e-17);
        assert!((report.vented_volume - 0.5 * w * dt).abs() < 1e-17);
        assert!((report.dissipated_energy - w * report.normal_impulse).abs() < 1e-12);
        errors.push((report.normal_impulse - exact_impulse).abs());
    }
    for pair in errors.windows(2) {
        assert!(
            pair[0] / pair[1] > 1.9 && pair[0] / pair[1] < 2.1,
            "impulse errors: {errors:?}"
        );
    }
}
#[test]
fn composed_squeeze_tracks_internal_mixing_and_vented_component_inventory() {
    let mut film = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material::default(),
    )
    .unwrap();
    for i in 0..2 {
        film.deposit(i, 0.0005).unwrap();
    }
    let mut mixture = physics::surface_film::FilmMixture::new(
        film,
        vec!["a".into(), "b".into()],
        vec![vec![1.0, 0.0], vec![0.0, 1.0]],
    )
    .unwrap();
    mixture
        .configure_viscosities(Some(vec![0.01, 0.1]))
        .unwrap();
    let masses = mixture.component_masses().unwrap();
    let report = mixture
        .step_squeeze(0.1, &[1e-5; 2], SqueezePressureControl::default(), 0.001)
        .unwrap();
    for h in mixture.film().thickness() {
        assert!((h - 0.000999).abs() < 1e-14);
    }
    assert!(mixture.fractions()[0][1] > 0.0);
    assert!((mixture.film().total_mass() + report.vented_mass - 1.0).abs() < 1e-13);
    for (k, remaining) in mixture.component_masses().unwrap().iter().enumerate() {
        assert!(
            (remaining + report.vented_component_masses.as_ref().unwrap()[k] - masses[k]).abs()
                < 1e-13
        );
    }
}
#[test]
fn late_gap_closure_restores_squeeze_volume_and_every_component() {
    let mut mixture = physics::surface_film::FilmMixture::new(
        single_cell_film(),
        vec!["a".into(), "b".into()],
        vec![vec![0.3, 0.7]],
    )
    .unwrap();
    let heights = mixture.film().thickness();
    let fractions = mixture.fractions();
    let masses = mixture.component_masses().unwrap();
    assert!(
        mixture
            .step_squeeze(0.1, &[0.02], SqueezePressureControl::default(), 0.001)
            .is_err()
    );
    assert_eq!(mixture.film().thickness(), heights);
    assert_eq!(mixture.fractions(), fractions);
    assert_eq!(mixture.component_masses().unwrap(), masses);
}
