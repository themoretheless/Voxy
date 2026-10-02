use physics::liquid::{Config, Error, Liquid, Material, Particle, SurfaceConfig};
use std::collections::BTreeMap;
fn liquid() -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 1.0,
            material: 0,
        }],
        vec![Material {
            rest_density: 1.0,
            sound_speed: 1.0,
            viscosity: 0.0,
        }],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap()
}
fn config() -> SurfaceConfig {
    SurfaceConfig {
        min: [-1.2; 3],
        max: [1.2; 3],
        cell_size: 0.1,
        isovalue: 0.5,
        material: None,
        max_samples: 20000,
        max_checks: 20000,
        max_triangles: 10000,
    }
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[test]
#[allow(clippy::cast_possible_truncation)]
fn isolated_particle_surface_is_closed_outward_and_matches_analytic_volume() {
    let surface = liquid().surface(config()).unwrap();
    assert!(surface.triangles.len() > 100);
    let mut edges = BTreeMap::new();
    let mut volume = 0.0;
    for triangle in surface.triangles {
        let a = std::array::from_fn(|i| triangle[1][i] - triangle[0][i]);
        let b = std::array::from_fn(|i| triangle[2][i] - triangle[0][i]);
        let normal = cross(a, b);
        let center: [f64; 3] =
            std::array::from_fn(|i| (triangle[0][i] + triangle[1][i] + triangle[2][i]) / 3.0);
        assert!(normal.iter().zip(center).map(|(a, b)| a * b).sum::<f64>() > 0.0);
        volume += triangle[0]
            .iter()
            .zip(cross(triangle[1], triangle[2]))
            .map(|(a, b)| a * b)
            .sum::<f64>()
            / 6.0;
        let keys = triangle.map(|point| point.map(|v| (v * 1e8).round() as i64));
        for pair in [[keys[0], keys[1]], [keys[1], keys[2]], [keys[2], keys[0]]] {
            let mut pair = pair;
            pair.sort_unstable();
            *edges.entry(pair).or_insert(0_usize) += 1;
        }
    }
    assert!(edges.values().all(|&count| count == 2));
    let peak = 315.0 / (64.0 * std::f64::consts::PI);
    let radius = (1.0 - (0.5 / peak).cbrt()).sqrt();
    let analytic = 4.0 * std::f64::consts::PI / 3.0 * radius.powi(3);
    assert!(
        (volume - analytic).abs() / analytic < 0.04,
        "mesh volume={volume}, expected={analytic}"
    );
}
#[test]
fn budgets_and_empty_material_selection_do_not_mutate_simulation() {
    let liquid = liquid();
    let before = liquid.clone();
    assert_eq!(
        liquid.surface(SurfaceConfig {
            max_triangles: 1,
            ..config()
        }),
        Err(Error::SurfaceBudget)
    );
    assert_eq!(
        liquid.surface(SurfaceConfig {
            max_checks: 1,
            ..config()
        }),
        Err(Error::SurfaceBudget)
    );
    assert_eq!(
        liquid.surface(SurfaceConfig {
            material: Some(5),
            ..config()
        }),
        Err(Error::InvalidSurface)
    );
    assert_eq!(liquid, before);
    let empty = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
    assert!(empty.surface(config()).unwrap().triangles.is_empty());
}
