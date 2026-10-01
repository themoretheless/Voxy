//! Held-temperature triple-alpha example, using JINA fy05 coefficients distributed
//! in pynucastro/data/reaclib_default2_20250330. No thermal feedback in this example.
use physics::astrophysics_nuclear::{Budget, Network, Nucleus};
fn main() -> Result<(), String> {
    let rate = physics::astrophysics_reaclib::parse(
        include_str!("../tests/data/triple_alpha_fy05.reaclib"),
        2e8,
        2e8,
        3,
    )
    .map_err(|e| format!("{e:?}"))?;
    // Binding reference: helium zero; carbon differs from 3 helium by Q=7.275 MeV.
    let q = rate.q_mev * 1.602_176_634e-13;
    let mut network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 12,
                charge: 6,
                binding_energy: q,
            },
        ],
        reactions: vec![],
    };
    network.reactions.push(
        rate.reaction(&network, &["he4", "c12"], 0.0, 1e-10)
            .map_err(|e| format!("{e:?}"))?,
    );
    let mut fractions = [1.0, 0.0];
    let mut heat = 0.0;
    println!("time_s,helium_fraction,carbon_fraction,deposited_energy_J_kg");
    for i in 0..=100 {
        println!(
            "{},{},{},{}",
            f64::from(i) * 1e6,
            fractions[0],
            fractions[1],
            heat
        );
        if i < 100 {
            heat += network
                .burn(
                    &mut fractions,
                    1e8,
                    2e8,
                    1e6,
                    Budget {
                        max_step: 1e5,
                        steps: 1000,
                        fit_evaluations: 3000,
                    },
                )
                .map_err(|e| format!("{e:?}"))?
                .deposited_energy;
        }
    }
    eprintln!(
        "energy reservoir balance J/kg: {}",
        heat + network
            .reservoir(&fractions)
            .map_err(|e| format!("{e:?}"))?
    );
    Ok(())
}
