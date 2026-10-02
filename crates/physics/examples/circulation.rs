//! Synthetic four-chamber closed circulation experiment; no fitted human defaults.
use physics::circulation::{Circulation, Compartment, Vessel};
fn unit(volume_ml: f64, unstressed_ml: f64, elastance: f64) -> Compartment {
    Compartment {
        initial_volume_m3: volume_ml * 1e-6,
        unstressed_volume_m3: unstressed_ml * 1e-6,
        initial_elastance_pa_per_m3: elastance,
        initial_external_pressure_pa: 0.,
    }
}
fn edge(from: usize, to: usize, resistance: f64, valve: bool) -> Vessel {
    Vessel {
        from,
        to,
        resistance,
        quadratic_resistance: if valve { 5e10 } else { 0. },
        inertance: if valve { 1e4 } else { 0. },
        valve,
    }
}
fn activation(phase: f64, start: f64, duration: f64) -> f64 {
    let local = (phase - start).rem_euclid(1.);
    if local < duration {
        (std::f64::consts::PI * local / duration).sin().powi(2)
    } else {
        0.
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // LV, systemic artery, systemic vein, RA, RV, pulmonary artery, pulmonary vein, LA.
    let mut blood = Circulation::new(
        vec![
            unit(120., 30., 1e7),
            unit(700., 580., 1e8),
            unit(3200., 2200., 5e5),
            unit(180., 120., 1e7),
            unit(120., 30., 5e6),
            unit(180., 117.5, 4e7),
            unit(400., 350., 2e7),
            unit(100., 0., 1e7),
        ],
        vec![
            edge(0, 1, 1e6, true),
            edge(1, 2, 1.6e8, false),
            edge(2, 3, 5e5, false),
            edge(3, 4, 1e6, true),
            edge(4, 5, 1e6, true),
            edge(5, 6, 2e7, false),
            edge(6, 7, 5e5, false),
            edge(7, 0, 1e6, true),
        ],
    )?;
    println!(
        "time_s,lv_pa,rv_pa,systemic_pa,pulmonary_pa,lv_ml,rv_ml,aortic_flow_ml_per_s,pulmonary_flow_ml_per_s,total_blood_ml,residual_m3"
    );
    for step in 1..=6000 {
        let time = f64::from(step) * 0.002;
        let phase = (time / 0.8).fract();
        let ventricular = activation(phase, 0., 0.4);
        let atrial = activation(phase, 0.75, 0.25);
        let e = [
            1e7 + 2e8 * ventricular,
            1e8,
            5e5,
            1e7 + 2e7 * atrial,
            5e6 + 5e7 * ventricular,
            4e7,
            2e7,
            1e7 + 2e7 * atrial,
        ];
        let report = blood.step(0.002, &e, &[0.; 8], 32, 1e-13)?;
        println!(
            "{time},{},{},{},{},{},{},{},{},{},{}",
            blood.pressures()[0],
            blood.pressures()[4],
            blood.pressures()[1],
            blood.pressures()[5],
            blood.volumes()[0] * 1e6,
            blood.volumes()[4] * 1e6,
            blood.flows()[0] * 1e6,
            blood.flows()[4] * 1e6,
            blood.total_volume() * 1e6,
            report.residual_m3
        );
    }
    Ok(())
}
