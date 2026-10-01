use physics::{
    astrophysics_eos::{Mixture, Species},
    astrophysics_gas::{Boundary, Cell},
    astrophysics_spherical::Sphere,
};
fn main() -> Result<(), String> {
    let mixture = Mixture::new(&[Species {
        mass_fraction: 1.0,
        mass_number: 1,
        nuclear_charge: 1,
    }])
    .map_err(|e| format!("{e:?}"))?;
    let mut sphere = Sphere {
        cells: (0..20)
            .map(|i| {
                Ok(Cell {
                    density: 0.001,
                    momentum: 0.0,
                    energy: mixture
                        .at(0.001, 1e7 * (1.0 - 0.01 * f64::from(i)))
                        .map_err(|e| format!("{e:?}"))?
                        .internal_energy_density,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        spacing: 0.05,
        gamma: 5.0 / 3.0,
        g: 1e15,
        outer: Boundary::Reflecting,
    };
    let initial = sphere.energy().map_err(|e| format!("{e:?}"))?;
    sphere
        .step_ionized(1e-9, 1e-10, 100, mixture)
        .map_err(|e| format!("{e:?}"))?;
    println!("radius,density,radial_velocity,temperature,gas_pressure,radiation_pressure");
    for (i, c) in sphere.cells.iter().enumerate() {
        let internal = c.energy - 0.5 * c.momentum * c.momentum / c.density;
        let t = mixture
            .temperature(c.density, internal)
            .map_err(|e| format!("{e:?}"))?;
        let eos = mixture.at(c.density, t).map_err(|e| format!("{e:?}"))?;
        println!(
            "{},{},{},{},{},{}",
            (i as f64 + 0.5) * sphere.spacing,
            c.density,
            c.momentum / c.density,
            t,
            eos.gas_pressure,
            eos.radiation_pressure
        );
    }
    eprintln!(
        "relative energy error: {}",
        (sphere.energy().map_err(|e| format!("{e:?}"))? - initial) / initial
    );
    Ok(())
}
