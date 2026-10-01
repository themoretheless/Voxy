use physics::{
    astrophysics_gas::{Boundary, Cell, Gas},
    astrophysics_radhydro::{CoupledBudget, RadiatingGas},
    astrophysics_radiation::blackbody,
};
fn main() -> Result<(), String> {
    let mut gravity = false;
    for arg in std::env::args().skip(1) {
        if arg == "--gravity" {
            gravity = true;
        } else {
            return Err(format!("unknown argument: {arg}"));
        }
    }
    let total = |system: &RadiatingGas| -> Result<f64, String> {
        if gravity {
            system
                .total_energy_with_gravity(1000.0)
                .map_err(|e| format!("{e:?}"))
        } else {
            Ok(system.gas.totals().map_err(|e| format!("{e:?}"))?[2] + system.escaped_energy)
        }
    };
    let mut system = RadiatingGas {
        gas: Gas {
            cells: vec![
                Cell::from_primitive(1.0, 0.0, 120000.0, 1.4)
                    .map_err(|e| format!("{e:?}"))?;
                40
            ],
            spacing: 0.1,
            gamma: 1.4,
            boundary: Boundary::Reflecting,
        },
        specific_heat: 1000.0,
        opacity: 0.5,
        escaped_energy: 0.0,
    };
    let incident = blackbody(600.0).map_err(|e| format!("{e:?}"))?;
    let initial = total(&system)?;
    for _ in 0..1000 {
        if gravity {
            system
                .step_with_gravity(
                    1000.0,
                    incident,
                    0.0,
                    0.001,
                    CoupledBudget {
                        max_step: 0.0001,
                        gravity_steps: 100,
                        hydro_steps: 1000,
                        thermal_steps: 1000,
                    },
                )
                .map_err(|e| format!("{e:?}"))?;
        } else {
            system
                .step(incident, 0.0, 0.001, 1000, 1000)
                .map_err(|e| format!("{e:?}"))?;
        }
    }
    let column = system.column().map_err(|e| format!("{e:?}"))?;
    println!("height_m,density_kg_m3,velocity_m_s,temperature_K");
    for (i, (cell, slab)) in system.gas.cells.iter().zip(column.slabs).enumerate() {
        println!(
            "{},{},{},{}",
            (i as f64 + 0.5) * system.gas.spacing,
            cell.density,
            cell.momentum / cell.density,
            slab.temperature
        );
    }
    eprintln!("energy error J/m2: {}", total(&system)? - initial);
    Ok(())
}
