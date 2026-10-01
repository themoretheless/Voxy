//! Grey atmosphere algebra demonstration with a fully ionized EOS.
use physics::{
    astrophysics_atmosphere::Atmosphere,
    astrophysics_eos::{Mixture, Species},
};
fn main() -> Result<(), String> {
    let atmosphere = Atmosphere {
        gravity: 1e5,
        opacity: 0.01,
        effective_temperature: 1e5,
        top_gas_pressure: 1.0,
        mixture: Mixture::new(&[Species {
            mass_fraction: 1.0,
            mass_number: 1,
            nuclear_charge: 1,
        }])
        .map_err(|e| format!("{e:?}"))?,
    };
    println!(
        "optical_depth,temperature_K,density_kg_m3,gas_pressure_Pa,radiation_pressure_Pa,internal_energy_J_m3"
    );
    for depth in [0.0, 0.01, 0.1, 2.0 / 3.0, 1.0, 10.0, 100.0] {
        let state = atmosphere.at(depth).map_err(|e| format!("{e:?}"))?;
        let cell = atmosphere
            .gas_cell(depth, 0.0)
            .map_err(|e| format!("{e:?}"))?;
        println!(
            "{depth},{},{},{},{},{}",
            state.temperature,
            state.density,
            state.gas_pressure,
            state.radiation_pressure,
            cell.energy
        );
    }
    Ok(())
}
