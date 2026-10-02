//! Synthetic active FEM chamber with a closed valve/reservoir circuit, not anatomical heart geometry.
use physics::biomechanics::{CouplingConfig, FemChamber, Fiber, Material, tube};
use physics::circulation::{Circulation, Compartment, Vessel};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let material = Material {
        shear_pa: 2000.,
        bulk_pa: 20000.,
        fibers: vec![Fiber {
            direction: [1., 0., 0.],
            stiffness_pa: 1000.,
            exponent: 3.,
            active_pa: 10000.,
        }],
    };
    let mut chamber = FemChamber::new(
        tube(&[0.01, 0.014], 0.025, 8, 1, &[material], true)?,
        0,
        CouplingConfig {
            force_tolerance_n: 1e-8,
            ..CouplingConfig::default()
        },
    )?;
    let volume = chamber.body().cavity_volume(0)?;
    let unit = |volume, unstressed, e| Compartment {
        initial_volume_m3: volume,
        unstressed_volume_m3: unstressed,
        initial_elastance_pa_per_m3: e,
        initial_external_pressure_pa: 0.,
    };
    let edge = |from, to, resistance, valve| Vessel {
        from,
        to,
        resistance,
        quadratic_resistance: 0.,
        inertance: 0.,
        valve,
    };
    let mut blood = Circulation::new(
        vec![
            unit(volume, volume, 1e8),
            unit(0.001, 0.00082, 1e6),
            unit(0.001, 0.00094, 1e6),
        ],
        vec![
            edge(0, 1, 2e6, true),
            edge(2, 0, 2e6, true),
            edge(1, 2, 1e8, false),
        ],
    )?;
    let total = blood.total_volume();
    println!(
        "time_s,activation,wall_pressure_pa,fem_volume_ml,blood_volume_ml,outflow_ml_per_s,inflow_ml_per_s,total_drift_m3,residual_m3"
    );
    for step in 1..=12 {
        let time = f64::from(step) * 0.02;
        let activation = 0.15 * (std::f64::consts::PI * time / 0.24).sin().powi(2);
        for i in 0..chamber.body().elements().len() {
            chamber.body_mut().set_activation(i, activation)?;
        }
        let report = chamber.step(&mut blood, 0.02, &[1e8, 1e6, 1e6], &[0.; 3], 32)?;
        println!(
            "{time},{activation},{},{},{},{},{},{},{}",
            blood.pressures()[0],
            chamber.body().cavity_volume(0)? * 1e6,
            blood.volumes()[0] * 1e6,
            blood.flows()[0] * 1e6,
            blood.flows()[1] * 1e6,
            blood.total_volume() - total,
            report.residual_m3
        );
    }
    Ok(())
}
