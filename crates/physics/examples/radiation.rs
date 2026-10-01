//! Grey hot gas observed through a cooler foreground slab, SI units.
use physics::astrophysics_radiation::{Layer, blackbody, trace};
fn main() -> Result<(), String> {
    let incident = blackbody(6000.0).map_err(|e| format!("{e:?}"))?;
    println!("optical_depth,outgoing_intensity,net_deposited_intensity");
    for i in 0..=100 {
        let depth = f64::from(i) / 10.0;
        let result = trace(
            incident,
            &[Layer {
                length: 1.0,
                absorption: depth,
                temperature: 3000.0,
            }],
            1,
        )
        .map_err(|e| format!("{e:?}"))?;
        println!("{depth},{},{}", result.intensity, result.deposited[0]);
    }
    Ok(())
}
