use physics::astrophysics_eos::{Mixture, Species};
fn main() -> Result<(), String> {
    let mixture = Mixture::new(&[
        Species {
            mass_fraction: 0.7,
            mass_number: 1,
            nuclear_charge: 1,
        },
        Species {
            mass_fraction: 0.28,
            mass_number: 4,
            nuclear_charge: 2,
        },
        Species {
            mass_fraction: 0.02,
            mass_number: 16,
            nuclear_charge: 8,
        },
    ])
    .map_err(|e| format!("{e:?}"))?;
    println!("temperature_K,gas_pressure_Pa,radiation_pressure_Pa,internal_energy_J_m3,cv_J_kg_K");
    for exponent in 3..=8 {
        let temperature = 10_f64.powi(exponent);
        let state = mixture.at(1.0, temperature).map_err(|e| format!("{e:?}"))?;
        println!(
            "{temperature},{},{},{},{}",
            state.gas_pressure,
            state.radiation_pressure,
            state.internal_energy_density,
            state.specific_heat_cv
        );
    }
    Ok(())
}
