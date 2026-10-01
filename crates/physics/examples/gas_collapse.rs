use physics::{
    astrophysics_gas::{Boundary, Cell, Gas},
    astrophysics_gas_gravity::SelfGravitatingGas,
};
fn main() -> Result<(), String> {
    let mut system = SelfGravitatingGas {
        gas: Gas {
            cells: vec![
                Cell::from_primitive(1.0, 0.0, 0.1, 1.4).map_err(|e| format!("{e:?}"))?;
                200
            ],
            spacing: 0.005,
            gamma: 1.4,
            boundary: Boundary::Reflecting,
        },
        g: 0.1,
    };
    let initial = system.energy().map_err(|e| format!("{e:?}"))?;
    system
        .step(0.1, 0.0005, 10000, 100000)
        .map_err(|e| format!("{e:?}"))?;
    println!("position,density,velocity,pressure");
    for (i, c) in system.gas.cells.iter().enumerate() {
        println!(
            "{},{},{},{}",
            (i as f64 + 0.5) * system.gas.spacing,
            c.density,
            c.momentum / c.density,
            c.pressure(system.gas.gamma).map_err(|e| format!("{e:?}"))?
        );
    }
    eprintln!(
        "total energy error: {}",
        system.energy().map_err(|e| format!("{e:?}"))? - initial
    );
    Ok(())
}
