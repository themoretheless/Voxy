use physics::liquid::*;
fn fluid() -> Liquid {
    Liquid::new(
        vec![
            Particle {
                position: [-0.0008, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 4e-6,
                material: 0,
            },
            Particle {
                position: [0.0008, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 1e-6,
                material: 0,
            },
        ],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap()
}
fn config() -> SurfaceConfig {
    SurfaceConfig {
        min: [-0.006; 3],
        max: [0.006; 3],
        cell_size: 0.0004,
        isovalue: 0.25,
        material: None,
        max_samples: 50000,
        max_checks: 1_000_000,
        max_triangles: 15000,
    }
}
fn control() -> SurfaceVolumeControl {
    SurfaceVolumeControl {
        relative_tolerance: 0.01,
        max_iterations: 24,
        max_checks: 10_000_000,
    }
}
#[test]
fn connected_unequal_particle_surface_preserves_physical_volume_and_closed_edges() {
    let fluid = fluid();
    let before = fluid.clone();
    let supports: Vec<_> = fluid
        .equivalent_sphere_radii()
        .unwrap()
        .iter()
        .map(|r| 2.5 * r)
        .collect();
    let result = fluid
        .surface_volume_matched(config(), &supports, control())
        .unwrap();
    assert!((result.measured_volume - 5e-9).abs() / 5e-9 < 0.01);
    assert!(result.iterations > 1 && result.surface.triangles.len() > 100);
    let mut edges = std::collections::BTreeMap::new();
    for triangle in result.surface.triangles {
        let keys = triangle.map(|p| p.map(|v| (v * 1e12).round() as i64));
        for mut e in [[keys[0], keys[1]], [keys[1], keys[2]], [keys[2], keys[0]]] {
            e.sort_unstable();
            *edges.entry(e).or_insert(0usize) += 1;
        }
    }
    assert!(edges.values().all(|n| *n == 2));
    assert_eq!(fluid, before);
}
#[test]
fn isolated_small_drop_retains_volume_despite_large_carrier_support() {
    let fluid = Liquid::new(
        vec![Particle {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 1e-6,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            smoothing_radius: 0.1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(fluid.surface(config()).unwrap().triangles.is_empty());
    let r = fluid.equivalent_sphere_radii().unwrap()[0];
    let result = fluid
        .surface_volume_matched(config(), &[2.5 * r], control())
        .unwrap();
    assert!((result.measured_volume - 1e-9).abs() / 1e-9 < 0.01);
}
#[test]
fn clipping_invalid_support_and_late_volume_budget_are_errors_without_state_changes() {
    let fluid = fluid();
    let before = fluid.clone();
    assert!(
        fluid
            .surface_volume_matched(config(), &[0.1, 0.1], control())
            .is_err()
    );
    assert!(
        fluid
            .surface_with_particle_support(config(), &[f64::NAN, 0.001])
            .is_err()
    );
    assert!(
        fluid
            .surface_volume_matched(
                config(),
                &[0.002, 0.002],
                SurfaceVolumeControl {
                    max_iterations: 1,
                    ..control()
                }
            )
            .is_err()
    );
    assert!(
        fluid
            .surface_volume_matched(
                config(),
                &[0.002, 0.002],
                SurfaceVolumeControl {
                    max_checks: 1,
                    ..control()
                }
            )
            .is_err()
    );
    assert_eq!(fluid, before);
}
