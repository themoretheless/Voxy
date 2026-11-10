use physics::{
    astrophysics_evolution::{Budget, RadiatingSphere},
    astrophysics_gas::{Boundary, Cell},
    astrophysics_spherical::Sphere,
};
fn model() -> RadiatingSphere {
    RadiatingSphere {
        sphere: Sphere {
            cells: vec![Cell::from_primitive(1.0, 0.0, 200000.0, 1.4).unwrap(); 16],
            spacing: 0.0625,
            gamma: 1.4,
            g: 1000.0,
            outer: Boundary::Reflecting,
        },
        specific_heat: 1000.0,
        opacity: 0.1,
        ambient: 0.0,
        escaped_radiation: 0.0,
        escaped_gas_energy: 0.0,
        escaped_mass: 0.0,
    }
}
fn budget() -> Budget {
    Budget {
        max_step: 0.0001,
        outer_steps: 1000,
        hydro_steps: 10000,
        thermal_steps: 1000,
        ray_segments: 1000000,
        rays_per_annulus: 4,
    }
}
#[test]
fn dynamic_radiative_sphere_conserves_energy_and_mass() {
    for outer in [Boundary::Reflecting, Boundary::Outflow] {
        let mut s = model();
        s.sphere.outer = outer;
        if outer == Boundary::Outflow {
            for (i, c) in s.sphere.cells.iter_mut().enumerate() {
                let u = i as f64 * 20.0;
                *c = Cell::from_primitive(1.0, u, 200000.0, 1.4).unwrap();
            }
        }
        let initial = s.energy().unwrap();
        let mass = s.sphere.totals().unwrap()[0];
        let work = s.step(0.001, budget()).unwrap();
        assert!((s.energy().unwrap() - initial).abs() < 1e-7);
        assert!((s.sphere.totals().unwrap()[0] + s.escaped_mass - mass).abs() < 1e-12);
        assert!(s.escaped_radiation > 0.0);
        assert!(work.hydro_steps > 0 && work.ray_segments > 0);
        assert!(s.sphere.cells.iter().any(|c| c.momentum.abs() > 1e-4));
    }
}
#[test]
fn late_ray_budget_failure_restores_all_state_and_ledgers() {
    let mut s = model();
    let before = s.clone();
    assert!(
        s.step(
            0.001,
            Budget {
                ray_segments: 1500,
                ..budget()
            }
        )
        .is_err()
    );
    assert_eq!(s, before);
    assert!(
        s.step(
            0.001,
            Budget {
                hydro_steps: 1,
                ..budget()
            }
        )
        .is_err()
    );
    assert_eq!(s, before);
}
#[test]
fn segment_budget_counts_exactly_the_solved_rays() {
    let mut s = model();
    let required = 16 * 17 * 4;
    let w = s
        .step(
            0.0001,
            Budget {
                ray_segments: required,
                ..budget()
            },
        )
        .unwrap();
    assert_eq!(w.ray_segments, required);
    assert_eq!(w.thermal_steps, 1);
}
