//! Synthetic edema/drainage comparison. All values are exploratory, not fitted.
use physics::lymph::{Exchange, FluidSpace, LymphNetwork};
fn network(drainage: f64) -> LymphNetwork {
    let spaces = vec![
        FluidSpace {
            reference_volume_m3: 0.003,
            reference_pressure_pa: 1800.,
            compliance_m3_per_pa: 3e-8,
            initial_volume_m3: 0.003,
            initial_protein_kg: 0.18,
            oncotic_pa_per_kg_m3: 40.,
        },
        FluidSpace {
            reference_volume_m3: 0.001,
            reference_pressure_pa: 0.,
            compliance_m3_per_pa: 1e-8,
            initial_volume_m3: 0.001,
            initial_protein_kg: 0.01,
            oncotic_pa_per_kg_m3: 40.,
        },
        FluidSpace {
            reference_volume_m3: 0.0001,
            reference_pressure_pa: -50.,
            compliance_m3_per_pa: 1e-8,
            initial_volume_m3: 0.0001,
            initial_protein_kg: 0.001,
            oncotic_pa_per_kg_m3: 40.,
        },
    ];
    let edges = vec![
        Exchange {
            from: 0,
            to: 1,
            hydraulic_m3_per_pa_s: 1e-9,
            reflection: 0.8,
            protein_permeability_m3_per_s: 1e-10,
            pump_head_pa: 0.,
            valve: false,
        },
        Exchange {
            from: 1,
            to: 2,
            hydraulic_m3_per_pa_s: 1e-9 * drainage,
            reflection: 0.,
            protein_permeability_m3_per_s: 0.,
            pump_head_pa: 0.,
            valve: true,
        },
        Exchange {
            from: 2,
            to: 0,
            hydraulic_m3_per_pa_s: 1e-9,
            reflection: 0.,
            protein_permeability_m3_per_s: 0.,
            pump_head_pa: 1900.,
            valve: true,
        },
    ];
    LymphNetwork::new(spaces, edges).unwrap()
}
fn main() {
    let h: f64 = std::env::args()
        .nth(1)
        .map_or(0.1, |s| s.parse().expect("maximum step seconds"));
    let mut open = network(1.);
    let mut impaired = network(0.01);
    println!(
        "time_s,tissue_open_ml,tissue_impaired_ml,open_pressure_pa,impaired_pressure_pa,volume_drift_m3,protein_drift_kg"
    );
    let v = open.total_volume();
    let m = open.total_protein();
    for i in 1..=600 {
        open.step(1., h).unwrap();
        impaired.step(1., h).unwrap();
        println!(
            "{i},{:.9},{:.9},{:.9},{:.9},{:.12e},{:.12e}",
            open.volumes()[1] * 1e6,
            impaired.volumes()[1] * 1e6,
            open.pressures()[1],
            impaired.pressures()[1],
            (open.total_volume() - v)
                .abs()
                .max((impaired.total_volume() - v).abs()),
            (open.total_protein() - m)
                .abs()
                .max((impaired.total_protein() - m).abs())
        );
    }
}
