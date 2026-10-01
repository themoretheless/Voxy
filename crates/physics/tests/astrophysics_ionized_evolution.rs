use physics::{
    astrophysics_eos::{Mixture, Species},
    astrophysics_evolution::{Budget, RadiatingSphere},
    astrophysics_gas::{Boundary, Cell},
    astrophysics_radiation::blackbody,
    astrophysics_spherical::Sphere,
};
fn mixture() -> Mixture {
    Mixture::new(&[Species {
        mass_fraction: 1.0,
        mass_number: 1,
        nuclear_charge: 1,
    }])
    .unwrap()
}
fn model() -> RadiatingSphere {
    RadiatingSphere {
        sphere: Sphere {
            cells: vec![
                Cell {
                    density: 1.0,
                    momentum: 0.0,
                    energy: mixture().at(1.0, 1e5).unwrap().internal_energy_density
                };
                8
            ],
            spacing: 0.125,
            gamma: 5.0 / 3.0,
            g: 1000.0,
            outer: Boundary::Reflecting,
        },
        specific_heat: 1.0,
        opacity: 0.001,
        ambient: 0.0,
        escaped_radiation: 0.0,
        escaped_gas_energy: 0.0,
        escaped_mass: 0.0,
    }
}
fn budget() -> Budget {
    Budget {
        max_step: 1e-7,
        outer_steps: 100,
        hydro_steps: 1000,
        thermal_steps: 1000,
        ray_segments: 100000,
        rays_per_annulus: 4,
    }
}
#[test]
fn same_eos_drives_radiation_and_dynamics_without_double_energy() {
    let mut s = model();
    let initial = s.energy().unwrap();
    let mass = s.sphere.totals().unwrap()[0];
    s.step_ionized(1e-6, budget(), mixture()).unwrap();
    assert!(s.escaped_radiation > 0.0);
    assert!((s.energy().unwrap() - initial).abs() / initial < 1e-12);
    assert!((s.sphere.totals().unwrap()[0] - mass).abs() < 1e-12);
    let c = s.sphere.cells[4];
    let t = mixture()
        .temperature(
            c.density,
            c.energy - 0.5 * c.momentum * c.momentum / c.density,
        )
        .unwrap();
    assert!(t < 1e5);
    let before = s.clone();
    assert!(
        s.step_ionized(
            1e-6,
            Budget {
                ray_segments: 300,
                ..budget()
            },
            mixture()
        )
        .is_err()
    );
    assert_eq!(s, before);
}
#[test]
fn ionized_lte_equilibrium_and_cv_field_independence() {
    let mut s = model();
    s.sphere.g = 0.0;
    s.ambient = blackbody(1e5).unwrap();
    let mut other = s.clone();
    other.specific_heat = 1e20;
    s.step_ionized(1e-6, budget(), mixture()).unwrap();
    other.step_ionized(1e-6, budget(), mixture()).unwrap();
    assert_eq!(s.sphere, other.sphere);
    for c in s.sphere.cells {
        assert!(c.momentum.abs() < 1e-6);
    }
    assert!(s.escaped_radiation.abs() < 1e-6);
}
