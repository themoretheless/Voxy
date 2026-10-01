use physics::{
    astrophysics_column::{Column, Slab},
    astrophysics_radiation::blackbody,
};
fn main() -> Result<(), String> {
    let mut column = Column {
        slabs: vec![
            Slab {
                thickness: 1.0,
                absorption: 0.5,
                temperature: 300.0,
                heat_capacity: 1000.0
            };
            4
        ],
        escaped_energy: 0.0,
    };
    let initial = column.internal_energy().map_err(|e| format!("{e:?}"))?;
    let boundary = blackbody(600.0).map_err(|e| format!("{e:?}"))?;
    println!("time,bottom_K,inner_K,upper_inner_K,top_K,net_escaped_J_m2,energy_error_J_m2");
    for i in 0..=100 {
        println!(
            "{},{},{},{},{},{},{}",
            i,
            column.slabs[0].temperature,
            column.slabs[1].temperature,
            column.slabs[2].temperature,
            column.slabs[3].temperature,
            column.escaped_energy,
            column.internal_energy().map_err(|e| format!("{e:?}"))? + column.escaped_energy
                - initial
        );
        if i < 100 {
            column
                .step(boundary, 0.0, 1.0, 10000)
                .map_err(|e| format!("{e:?}"))?;
        }
    }
    Ok(())
}
