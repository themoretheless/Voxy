use physics::{
    astrophysics_gas::{Boundary, Cell},
    astrophysics_spherical::Sphere,
    astrophysics_spherical_radiation::Heating,
};
fn main() -> Result<(), String> {
    let mut sphere = Sphere {
        cells: vec![
            Cell::from_primitive(1.0, 0.0, 200000.0, 1.4).map_err(|e| format!("{e:?}"))?;
            20
        ],
        spacing: 0.05,
        gamma: 1.4,
        g: 0.0,
        outer: Boundary::Reflecting,
    };
    let initial = sphere.energy().map_err(|e| format!("{e:?}"))?;
    let settings = Heating {
        specific_heat: 1000.0,
        opacity: 0.1,
        ambient: 0.0,
        rays_per_annulus: 8,
        max_segments: 1000000,
        max_step: 0.1,
        max_steps: 1000,
    };
    let exchange = sphere
        .radiate(1.0, settings)
        .map_err(|e| format!("{e:?}"))?;
    println!("radius_m,temperature_K");
    for (i, c) in sphere.cells.iter().enumerate() {
        println!(
            "{},{}",
            (i as f64 + 0.5) * sphere.spacing,
            c.pressure(sphere.gamma).map_err(|e| format!("{e:?}"))?
                / (sphere.gamma - 1.0)
                / c.density
                / settings.specific_heat
        );
    }
    eprintln!(
        "escaped J: {}; energy error J: {}",
        exchange.escaped_energy,
        sphere.energy().map_err(|e| format!("{e:?}"))? + exchange.escaped_energy - initial
    );
    Ok(())
}
