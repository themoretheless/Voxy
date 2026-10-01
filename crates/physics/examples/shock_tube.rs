//! cargo run -p physics --example shock_tube > shock.csv
use physics::astrophysics_gas::{Boundary, Cell, Gas};
fn main() -> Result<(), String> {
    let mut gas = Gas {
        cells: (0..800)
            .map(|i| {
                let (rho, p) = if i < 400 { (1.0, 1.0) } else { (0.125, 0.1) };
                Cell::from_primitive(rho, 0.0, p, 1.4).map_err(|e| format!("{e:?}"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        spacing: 1.0 / 800.0,
        gamma: 1.4,
        boundary: Boundary::Outflow,
    };
    gas.step(0.2, 2000).map_err(|e| format!("{e:?}"))?;
    println!("x,density,velocity,pressure,energy");
    for (i, cell) in gas.cells.iter().enumerate() {
        println!(
            "{},{},{},{},{}",
            (i as f64 + 0.5) * gas.spacing,
            cell.density,
            cell.momentum / cell.density,
            cell.pressure(gas.gamma).map_err(|e| format!("{e:?}"))?,
            cell.energy
        );
    }
    Ok(())
}
