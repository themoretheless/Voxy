use physics::{
    astrophysics_gas::{Boundary, Cell},
    astrophysics_spherical::Sphere,
};
fn main() -> Result<(), String> {
    let mut sphere = Sphere {
        cells: vec![Cell::from_primitive(1.0, 0.0, 0.1, 1.4).map_err(|e| format!("{e:?}"))?; 100],
        spacing: 0.01,
        gamma: 1.4,
        g: 0.1,
        outer: Boundary::Reflecting,
    };
    let initial = sphere.energy().map_err(|e| format!("{e:?}"))?;
    sphere
        .step(0.1, 0.0005, 10000)
        .map_err(|e| format!("{e:?}"))?;
    println!("radius,density,radial_velocity,pressure");
    for (i, c) in sphere.cells.iter().enumerate() {
        println!(
            "{},{},{},{}",
            (i as f64 + 0.5) * sphere.spacing,
            c.density,
            c.momentum / c.density,
            c.pressure(sphere.gamma).map_err(|e| format!("{e:?}"))?
        );
    }
    eprintln!(
        "energy error: {}",
        sphere.energy().map_err(|e| format!("{e:?}"))? - initial
    );
    Ok(())
}
