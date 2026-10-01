use physics::{
    astrophysics_eos::{Mixture, Species},
    astrophysics_evolution::{Budget, RadiatingSphere},
    astrophysics_gas::{Boundary, Cell},
    astrophysics_spherical::Sphere,
};
fn main() -> Result<(), String> {
    let mut ionized = false;
    for arg in std::env::args().skip(1) {
        if arg == "--ionized" {
            ionized = true;
        } else {
            return Err(format!("unknown argument: {arg}"));
        }
    }
    let mixture = Mixture::new(&[Species {
        mass_fraction: 1.0,
        mass_number: 1,
        nuclear_charge: 1,
    }])
    .map_err(|e| format!("{e:?}"))?;
    let mut model = RadiatingSphere {
        sphere: Sphere {
            cells: vec![
                Cell::from_primitive(1.0, 0.0, 200000.0, 1.4)
                    .map_err(|e| format!("{e:?}"))?;
                16
            ],
            spacing: 0.0625,
            gamma: 1.4,
            g: 1000.0,
            outer: Boundary::Reflecting,
        },
        specific_heat: 1000.0,
        opacity: 0.1,
        ambient: 0.0,
        escaped_radiation: 0.0,
        escaped_gas_energy: 0.0,
        escaped_mass: 0.0,
    };
    if ionized {
        let energy = mixture
            .at(1.0, 1e5)
            .map_err(|e| format!("{e:?}"))?
            .internal_energy_density;
        for cell in &mut model.sphere.cells {
            cell.energy = energy;
        }
        model.opacity = 0.001;
    }
    let initial = model.energy().map_err(|e| format!("{e:?}"))?;
    let budget = Budget {
        max_step: if ionized { 1e-7 } else { 1e-4 },
        outer_steps: 1000,
        hydro_steps: 10000,
        thermal_steps: 1000,
        ray_segments: 1000000,
        rays_per_annulus: 4,
    };
    let work = if ionized {
        model.step_ionized(1e-6, budget, mixture)
    } else {
        model.step(0.01, budget)
    }
    .map_err(|e| format!("{e:?}"))?;
    println!("radius_m,density_kg_m3,radial_velocity_m_s,temperature_K");
    for (i, c) in model.sphere.cells.iter().enumerate() {
        println!(
            "{},{},{},{}",
            (i as f64 + 0.5) * model.sphere.spacing,
            c.density,
            c.momentum / c.density,
            if ionized {
                mixture
                    .temperature(
                        c.density,
                        c.energy - 0.5 * c.momentum * c.momentum / c.density,
                    )
                    .map_err(|e| format!("{e:?}"))?
            } else {
                c.pressure(model.sphere.gamma)
                    .map_err(|e| format!("{e:?}"))?
                    / (model.sphere.gamma - 1.0)
                    / c.density
                    / model.specific_heat
            }
        );
    }
    eprintln!(
        "escaped radiation J: {}; energy error J: {}; work: {work:?}",
        model.escaped_radiation,
        model.energy().map_err(|e| format!("{e:?}"))? - initial
    );
    Ok(())
}
