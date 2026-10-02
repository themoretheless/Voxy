//! Synthetic two-region ventilation specimen; no patient calibration or geometric breathing animation.
use physics::respiration::{Airway, LungUnit, Recoil, RespiratoryNetwork};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let unit = LungUnit {
        reference_volume_m3: 0.001,
        reference_transpulmonary_pa: 500.,
        recoil: Recoil::Logarithmic { scale_pa: 1200. },
    };
    let mut lung = RespiratoryNetwork::new(
        vec![None, None, Some(unit), Some(unit)],
        vec![
            Airway {
                from: 0,
                to: 1,
                resistance_pa_s_per_m3: 1e5,
            },
            Airway {
                from: 1,
                to: 2,
                resistance_pa_s_per_m3: 1e5,
            },
            Airway {
                from: 1,
                to: 3,
                resistance_pa_s_per_m3: 8e5,
            },
        ],
        vec![0.; 4],
        vec![0., 0., -500., -500.],
    )?;
    println!(
        "time_s,pleural_pa,region_a_ml,region_b_ml,mouth_flow_ml_per_s,balance_error_m3,airway_loss_j"
    );
    for step in 1..=1000 {
        let t = f64::from(step) * 0.01;
        let pleural = -500. - 150. * (1. - (2. * std::f64::consts::PI * t / 4.).cos());
        let report = lung.step(0.01, 0., &[0., 0., pleural, pleural], 32, 1e-13)?;
        println!(
            "{t},{pleural},{},{},{},{},{}",
            lung.volumes()[2] * 1e6,
            lung.volumes()[3] * 1e6,
            lung.flows()[0] * 1e6,
            report.volume_change_m3 - report.mouth_volume_m3,
            report.airway_loss_j
        );
    }
    Ok(())
}
