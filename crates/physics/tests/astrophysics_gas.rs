use physics::astrophysics_gas::{Boundary, Cell, Gas};
fn tube(n: usize, boundary: Boundary) -> Gas {
    Gas {
        cells: (0..n)
            .map(|i| {
                if i < n / 2 {
                    Cell::from_primitive(1.0, 0.0, 1.0, 1.4).unwrap()
                } else {
                    Cell::from_primitive(0.125, 0.0, 0.1, 1.4).unwrap()
                }
            })
            .collect(),
        spacing: 1.0 / n as f64,
        gamma: 1.4,
        boundary,
    }
}
#[test]
fn periodic_conservation_and_uniform_flow() {
    let mut gas = tube(100, Boundary::Periodic);
    let before = gas.totals().unwrap();
    gas.step(0.2, 1000).unwrap();
    for (a, b) in before.into_iter().zip(gas.totals().unwrap()) {
        assert!((a - b).abs() < 1e-12);
    }
    let cell = Cell::from_primitive(2.0, 3.0, 4.0, 1.4).unwrap();
    gas.cells.fill(cell);
    gas.step(0.2, 1000).unwrap();
    assert!(gas.cells.iter().all(|c| *c == cell));
}
#[test]
fn sod_shock_tube_matches_star_region() {
    let mut gas = tube(800, Boundary::Outflow);
    gas.step(0.2, 2000).unwrap();
    // Exact Sod star pressure 0.30313 and velocity 0.92745; sample left of contact.
    let cell = gas.cells[480];
    let p = cell.pressure(1.4).unwrap();
    let u = cell.momentum / cell.density;
    assert!((p - 0.30313).abs() < 0.015, "{p}");
    assert!((u - 0.92745).abs() < 0.025, "{u}");
    assert!((cell.density - 0.42632).abs() < 0.025, "{}", cell.density);
    assert_eq!(gas.cells[0].density, 1.0);
    assert_eq!(gas.cells[799].density, 0.125);
}
#[test]
fn reflecting_wall_conserves_mass_energy_and_budget_is_atomic() {
    let mut gas = tube(100, Boundary::Reflecting);
    let before = gas.clone();
    assert!(gas.step(0.2, 1).is_err());
    assert_eq!(gas, before);
    let totals = gas.totals().unwrap();
    gas.step(0.2, 1000).unwrap();
    let after = gas.totals().unwrap();
    assert!((totals[0] - after[0]).abs() < 1e-12);
    assert!((totals[2] - after[2]).abs() < 1e-12);
    let before = gas.clone();
    assert!(gas.step(f64::NAN, 1000).is_err());
    assert_eq!(gas, before);
}
#[test]
fn reported_fluxes_match_cell_and_boundary_changes() {
    let mut gas = tube(100, Boundary::Outflow);
    for c in &mut gas.cells {
        *c = Cell::from_primitive(c.density, 1.0, c.pressure(1.4).unwrap(), 1.4).unwrap();
    }
    let before = gas.clone();
    let initial = gas.totals().unwrap();
    let report = gas.advance(0.02, 1000).unwrap();
    for (i, (a, b)) in before.cells.iter().zip(&gas.cells).enumerate() {
        assert!(
            ((b.density - a.density) * gas.spacing - report.mass[i] + report.mass[i + 1]).abs()
                < 1e-13
        );
    }
    for ((a, b), escaped) in initial
        .into_iter()
        .zip(gas.totals().unwrap())
        .zip(report.boundary)
    {
        assert!((b + escaped - a).abs() < 1e-12);
    }
}

#[test]
fn contact_flux_preserves_stationary_density_jumps_and_supersonic_upwind() {
    use physics::astrophysics_gas::contact_flux;
    let gamma = 1.4;
    let state = |rho: f64, p: f64, u: f64| Cell {
        density: rho,
        momentum: rho * u,
        energy: p / (gamma - 1.0) + 0.5 * rho * u * u,
    };
    for rho in [0.001, 0.5, 2.0, 1000.0] {
        let flux = contact_flux(state(1.0, 1.0, 0.0), state(rho, 1.0, 0.0), gamma).unwrap();
        assert_eq!(flux, [0.0, 1.0, 0.0]);
    }
    let left = state(1.0, 1.0, 10.0);
    let flux = contact_flux(left, state(0.5, 0.5, 12.0), gamma).unwrap();
    assert_eq!(flux, [10.0, 101.0, (left.energy + 1.0) * 10.0]);
    let forward = contact_flux(state(1.0, 1.0, 0.0), state(0.125, 0.1, 0.0), gamma).unwrap();
    let reverse = contact_flux(state(0.125, 0.1, 0.0), state(1.0, 1.0, 0.0), gamma).unwrap();
    for i in [0, 2] {
        assert!((forward[i] + reverse[i]).abs() < 1e-12);
    }
    assert!((forward[1] - reverse[1]).abs() < 1e-12);
    assert!(forward[0] > 0.0);
}

#[test]
fn contact_evolution_keeps_static_layers_and_conserves_dynamic_transport() {
    let mut gas = tube(100, Boundary::Periodic);
    for (i, cell) in gas.cells.iter_mut().enumerate() {
        *cell =
            Cell::from_primitive(if i < 50 { 1.0 } else { 0.125 }, 0.0, 1.0, gas.gamma).unwrap();
    }
    let saved = gas.clone();
    let report = gas.advance_contact(0.2, 1000).unwrap();
    assert_eq!(gas, saved);
    assert!(report.mass.iter().all(|m| *m == 0.0));
    let mut gas = tube(100, Boundary::Reflecting);
    let saved = gas.clone();
    assert!(gas.advance_contact(0.2, 1).is_err());
    assert_eq!(gas, saved);
    let before = gas.totals().unwrap();
    gas.advance_contact(0.2, 1000).unwrap();
    let after = gas.totals().unwrap();
    for i in [0, 2] {
        assert!((before[i] - after[i]).abs() < 1e-12);
    }
    let mut gas = tube(800, Boundary::Outflow);
    gas.advance_contact(0.2, 2000).unwrap();
    let cell = gas.cells[480];
    assert!((cell.pressure(1.4).unwrap() - 0.30313).abs() < 0.015);
    assert!((cell.momentum / cell.density - 0.92745).abs() < 0.025);
    assert!((cell.density - 0.42632).abs() < 0.025);
}

#[test]
fn ionized_contact_flux_uses_local_composition_and_radiation_pressure() {
    use physics::{
        astrophysics_eos::{Mixture, Species},
        astrophysics_gas::{contact_flux, contact_flux_ionized},
    };
    let hydrogen = Mixture::new(&[Species {
        mass_fraction: 1.0,
        mass_number: 1,
        nuclear_charge: 1,
    }])
    .unwrap();
    let helium = Mixture::new(&[Species {
        mass_fraction: 1.0,
        mass_number: 4,
        nuclear_charge: 2,
    }])
    .unwrap();
    let state = hydrogen.at(1e5, 2e8).unwrap();
    let p = state.gas_pressure + state.radiation_pressure;
    let temperature = helium.temperature_from_pressure(2e5, p).unwrap();
    let other = helium.at(2e5, temperature).unwrap();
    let left = Cell {
        density: 1e5,
        momentum: 0.0,
        energy: state.internal_energy_density,
    };
    let right = Cell {
        density: 2e5,
        momentum: 0.0,
        energy: other.internal_energy_density,
    };
    let flux = contact_flux_ionized(left, right, hydrogen, helium).unwrap();
    let acoustic = state
        .sound_speed_squared
        .sqrt()
        .max(other.sound_speed_squared.sqrt());
    assert!(flux[0].abs() / (acoustic * 2e5) < 1e-12);
    assert!((flux[1] / p - 1.0).abs() < 1e-12);
    assert!(flux[2].abs() / (acoustic * right.energy) < 1e-12);
    let thermal = hydrogen.at(1.0, 1000.0).unwrap();
    let left = Cell {
        density: 1.0,
        momentum: 100.0,
        energy: thermal.internal_energy_density + 5000.0,
    };
    let right = Cell {
        density: 0.5,
        momentum: 0.0,
        energy: thermal.internal_energy_density * 0.5,
    };
    let a = contact_flux(left, right, 5.0 / 3.0).unwrap();
    let b = contact_flux_ionized(left, right, hydrogen, hydrogen).unwrap();
    for (a, b) in a.into_iter().zip(b) {
        assert!((a / b - 1.0).abs() < 1e-8);
    }
}
