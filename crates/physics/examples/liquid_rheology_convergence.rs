//! Time refinement of nonlinear viscosity; optional full mode also advects particles.
use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, ShearThinning, TransportMaterial,
    WhippedCreamProfile,
};
fn initial(cream: bool) -> Result<Liquid, Box<dyn std::error::Error>> {
    let positions = [
        [0.1, 0.2, 0.3],
        [-0.3, 0.2, 0.1],
        [0.2, -0.1, 0.1],
        [0.1, 0.1, -0.2],
        [-0.1, -0.2, -0.1],
        [0.3, -0.2, 0.2],
        [-0.2, 0.1, -0.3],
        [0.2, 0.3, -0.1],
    ];
    let particles = positions
        .into_iter()
        .map(|p| Particle {
            position: p,
            mass: 1.3,
            material: 0,
            velocity: [
                0.3 * p[0] + 0.4 * p[1] + 0.2 * p[2],
                0.4 * p[0] + 0.1 * p[1] - 0.1 * p[2],
                0.2 * p[0] - 0.1 * p[1] - 0.2 * p[2],
            ],
        })
        .collect();
    let profile = WhippedCreamProfile::DEMO;
    let mut liquid = Liquid::new(
        particles,
        vec![if cream {
            profile.material
        } else {
            Material::CONDENSED_MILK_DEMO
        }],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )?;
    liquid.configure_transport(
        vec![
            LiquidField {
                temperature: 300.0,
                concentration: 0.0
            };
            8
        ],
        vec![TransportMaterial {
            specific_heat: 1.0,
            conductivity: 0.0,
            ..TransportMaterial::default()
        }],
    )?;
    if cream {
        liquid.configure_herschel_bulkley(&[Some(profile.rheology)])?;
    } else {
        liquid.configure_shear_thinning(vec![Some(ShearThinning::CONDENSED_MILK_DEMO)])?;
    }
    Ok(liquid)
}
fn initial_pressure() -> Result<Liquid, Box<dyn std::error::Error>> {
    let base = initial(false)?;
    let mut liquid = Liquid::new(
        base.particles().to_vec(),
        vec![Material {
            rest_density: 5.0,
            sound_speed: 2.0,
            viscosity: 0.1,
        }],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )?;
    liquid.configure_transport(
        base.fields().unwrap().to_vec(),
        vec![TransportMaterial {
            specific_heat: 1.0,
            conductivity: 0.0,
            ..TransportMaterial::default()
        }],
    )?;
    liquid.set_pressure_work(true)?;
    Ok(liquid)
}
fn evolve(
    mut liquid: Liquid,
    steps: u32,
    midpoint: bool,
) -> Result<Liquid, Box<dyn std::error::Error>> {
    for _ in 0..steps {
        if midpoint {
            liquid.relax_viscosity_midpoint(0.01 / f64::from(steps))?;
        } else {
            liquid.relax_viscosity_symmetric(0.01 / f64::from(steps))?;
        }
    }
    Ok(liquid)
}
fn evolve_full(mut liquid: Liquid, steps: u32) -> Result<Liquid, Box<dyn std::error::Error>> {
    liquid.set_viscous_heating(true)?;
    liquid.set_viscous_integrator(physics::liquid::ViscousIntegrator::Midpoint)?;
    for _ in 0..steps {
        let stats = liquid.step(0.01 / f64::from(steps), None)?;
        if stats.substeps != 1 {
            return Err("control requires one internal step per interval".into());
        }
    }
    Ok(liquid)
}
fn evolve_split(mut liquid: Liquid, steps: u32) -> Result<Liquid, Box<dyn std::error::Error>> {
    liquid.set_viscous_heating(true)?;
    for _ in 0..steps {
        if liquid
            .step_symmetric_free(0.01 / f64::from(steps))?
            .substeps
            != 1
        {
            return Err("control requires one internal step per interval".into());
        }
    }
    Ok(liquid)
}
fn position_error(first: &Liquid, second: &Liquid) -> f64 {
    first
        .particles()
        .iter()
        .zip(second.particles())
        .map(|(a, b)| {
            a.mass
                * a.position
                    .iter()
                    .zip(b.position)
                    .map(|(v, w)| (v - w).powi(2))
                    .sum::<f64>()
        })
        .sum::<f64>()
        .sqrt()
}
fn error(first: &Liquid, second: &Liquid) -> f64 {
    first
        .particles()
        .iter()
        .zip(second.particles())
        .map(|(a, b)| {
            a.mass
                * a.velocity
                    .iter()
                    .zip(b.velocity)
                    .map(|(v, w)| (v - w).powi(2))
                    .sum::<f64>()
        })
        .sum::<f64>()
        .sqrt()
}
fn energy(liquid: &Liquid) -> f64 {
    liquid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum::<f64>()
        + liquid.transport_totals().unwrap().unwrap().0
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pressure = std::env::args().nth(1).as_deref() == Some("pressure");
    let (midpoint, full, split) = match std::env::args().nth(1).as_deref() {
        None => (false, false, false),
        Some("midpoint") => (true, false, false),
        Some("full") => (true, true, false),
        Some("split" | "pressure") => (true, true, true),
        Some(_) => return Err("expected midpoint, full, split or pressure".into()),
    };
    let run = |liquid, steps| {
        if split {
            evolve_split(liquid, steps)
        } else if full {
            evolve_full(liquid, steps)
        } else {
            evolve(liquid, steps, midpoint)
        }
    };
    println!("model,steps,velocity_error,energy_error,position_error");
    for cream in [false, true] {
        if pressure && cream {
            continue;
        }
        let liquid = if pressure {
            initial_pressure()?
        } else {
            initial(cream)?
        };
        let reference = run(liquid.clone(), 4096)?;
        let coarse_reference = run(liquid.clone(), 2048)?;
        let name = if pressure {
            "pressure"
        } else if cream {
            "cream"
        } else {
            "milk"
        };
        println!(
            "{name},2048,{},{},{}",
            error(&coarse_reference, &reference),
            (energy(&coarse_reference) - energy(&liquid)).abs(),
            position_error(&coarse_reference, &reference)
        );
        for steps in [8, 16, 32, 64] {
            let result = run(liquid.clone(), steps)?;
            println!(
                "{name},{steps},{},{},{}",
                error(&result, &reference),
                (energy(&result) - energy(&liquid)).abs(),
                position_error(&result, &reference)
            );
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn symmetric_pressure_viscosity_control_refines_and_balances_work() {
        let liquid = initial_pressure().unwrap();
        assert!(
            liquid
                .diagnostics()
                .unwrap()
                .densities
                .iter()
                .any(|rho| *rho > 5.0)
        );
        let reference = evolve_split(liquid.clone(), 512).unwrap();
        let coarse = evolve_split(liquid.clone(), 8).unwrap();
        let fine = evolve_split(liquid.clone(), 16).unwrap();
        assert!(position_error(&fine, &reference) < 0.3 * position_error(&coarse, &reference));
        assert!(error(&fine, &reference) < 0.3 * error(&coarse, &reference));
        assert!((energy(&fine) - energy(&liquid)).abs() < 1e-9);
    }
    #[test]
    fn symmetric_free_flow_refines_positions_and_preserves_angular_momentum() {
        for cream in [false, true] {
            let liquid = initial(cream).unwrap();
            let reference = evolve_split(liquid.clone(), 512).unwrap();
            let coarse = evolve_split(liquid.clone(), 8).unwrap();
            let fine = evolve_split(liquid.clone(), 16).unwrap();
            assert!(position_error(&fine, &reference) < 0.3 * position_error(&coarse, &reference));
            assert!(error(&fine, &reference) < 0.3 * error(&coarse, &reference));
            assert!((energy(&fine) - energy(&liquid)).abs() < 1e-9);
            let angular = |fluid: &Liquid| -> [f64; 3] {
                std::array::from_fn(|axis| {
                    let b = (axis + 1) % 3;
                    let c = (axis + 2) % 3;
                    fluid
                        .particles()
                        .iter()
                        .map(|p| {
                            p.mass * (p.position[b] * p.velocity[c] - p.position[c] * p.velocity[b])
                        })
                        .sum()
                })
            };
            for (a, b) in angular(&fine).iter().zip(angular(&liquid)) {
                assert!((a - b).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn symmetric_free_ballistics_and_midstep_budget_rollback() {
        let particle = Particle {
            position: [0.1, 0.2, 0.3],
            velocity: [1.0, -2.0, 3.0],
            mass: 1.0,
            material: 0,
        };
        let mut liquid = Liquid::new(
            vec![particle],
            vec![Material {
                viscosity: 0.0,
                ..Material::WATER
            }],
            Config {
                smoothing_radius: 1.0,
                max_substeps: 1,
                ..Config::default()
            },
        )
        .unwrap();
        liquid
            .configure_transport(
                vec![LiquidField {
                    temperature: 300.0,
                    concentration: 0.0,
                }],
                vec![TransportMaterial {
                    conductivity: 0.0,
                    ..TransportMaterial::default()
                }],
            )
            .unwrap();
        liquid.set_viscous_heating(true).unwrap();
        let before = liquid.clone();
        assert!(liquid.step_symmetric_free(0.1).is_err());
        assert_eq!(liquid, before);
        let dt = 0.001;
        assert_eq!(liquid.step_symmetric_free(dt).unwrap().substeps, 1);
        for (axis, gravity) in [0.0, -9.81, 0.0].iter().enumerate() {
            let actual = liquid.particles()[0];
            assert!((actual.velocity[axis] - particle.velocity[axis] - gravity * dt).abs() < 1e-12);
            assert!(
                (actual.position[axis]
                    - particle.position[axis]
                    - particle.velocity[axis] * dt
                    - 0.5 * gravity * dt * dt)
                    .abs()
                    < 1e-12
            );
        }
    }
    #[test]
    fn full_advected_nonlinear_control_converges_and_balances_energy() {
        for cream in [false, true] {
            let liquid = initial(cream).unwrap();
            let reference = evolve_full(liquid.clone(), 512).unwrap();
            let coarse = evolve_full(liquid.clone(), 8).unwrap();
            let fine = evolve_full(liquid.clone(), 16).unwrap();
            assert!(position_error(&fine, &reference) < 0.75 * position_error(&coarse, &reference));
            assert!(error(&fine, &reference) < error(&coarse, &reference));
            assert!(position_error(&fine, &liquid) > 1e-7);
            assert!((energy(&fine) - energy(&liquid)).abs() < 1e-9);
            for axis in 0..3 {
                let before: f64 = liquid
                    .particles()
                    .iter()
                    .map(|p| p.mass * p.velocity[axis])
                    .sum();
                let after: f64 = fine
                    .particles()
                    .iter()
                    .map(|p| p.mass * p.velocity[axis])
                    .sum();
                assert!((before - after).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn ordinary_midpoint_step_matches_isolated_viscosity_then_advection() {
        use physics::liquid::{BoundarySample, Formulation, ReflectingBox, ViscousIntegrator};
        for (cream, reflecting) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut full = initial(cream).unwrap();
            if reflecting {
                let mut shifted = full.particles().to_vec();
                for p in &mut shifted {
                    p.position[0] += 0.5;
                }
                let transport = full.fields().unwrap().to_vec();
                let profile = WhippedCreamProfile::DEMO;
                full = Liquid::new(
                    shifted,
                    vec![if cream {
                        profile.material
                    } else {
                        Material::CONDENSED_MILK_DEMO
                    }],
                    Config {
                        smoothing_radius: 1.0,
                        gravity: [0.0; 3],
                        ..Config::default()
                    },
                )
                .unwrap();
                full.configure_transport(
                    transport,
                    vec![TransportMaterial {
                        specific_heat: 1.0,
                        conductivity: 0.0,
                        ..TransportMaterial::default()
                    }],
                )
                .unwrap();
                if cream {
                    full.configure_herschel_bulkley(&[Some(profile.rheology)])
                        .unwrap();
                } else {
                    full.configure_shear_thinning(vec![Some(ShearThinning::CONDENSED_MILK_DEMO)])
                        .unwrap();
                }
                full.set_formulation(Formulation::RestVolumeWendland);
                full.set_reflecting_box(Some(ReflectingBox {
                    min: [-1.0; 3],
                    max: [1.0; 3],
                }))
                .unwrap();
                full.set_reflecting_no_slip(true).unwrap();
            }
            full.set_viscous_heating(true).unwrap();
            full.set_viscous_integrator(ViscousIntegrator::Midpoint)
                .unwrap();
            let mut frozen = full.clone();
            let dt = 1e-4;
            frozen.relax_viscosity_midpoint(dt).unwrap();
            assert_eq!(full.step(dt, None).unwrap().substeps, 1);
            for (a, b) in full.particles().iter().zip(frozen.particles()) {
                for axis in 0..3 {
                    assert!((a.velocity[axis] - b.velocity[axis]).abs() < 1e-12);
                    assert!(
                        (a.position[axis] - b.position[axis] - dt * b.velocity[axis]).abs() < 1e-12
                    );
                }
            }
            for (a, b) in full.fields().unwrap().iter().zip(frozen.fields().unwrap()) {
                assert!((a.temperature - b.temperature).abs() < 1e-12);
            }
            let before = full.clone();
            assert!(
                full.configure_boundaries(vec![BoundarySample {
                    position: [0.0; 3],
                    volume: 0.001,
                }])
                .is_err()
            );
            assert_eq!(full, before);
        }
    }
    #[test]
    fn nonlinear_midpoint_refinement_is_second_order_in_this_control() {
        for cream in [false, true] {
            let liquid = initial(cream).unwrap();
            let reference = evolve(liquid.clone(), 512, true).unwrap();
            let coarse = evolve(liquid.clone(), 8, true).unwrap();
            let fine = evolve(liquid.clone(), 16, true).unwrap();
            assert!(error(&fine, &reference) < 0.3 * error(&coarse, &reference));
            assert!((energy(&fine) - energy(&liquid)).abs() < 1e-9);
            let mut invalid = liquid;
            let before = invalid.clone();
            assert!(invalid.relax_viscosity_midpoint(f64::NAN).is_err());
            assert_eq!(invalid, before);
        }
    }
    #[test]
    fn nonlinear_viscous_refinement_reduces_error_and_preserves_energy() {
        for cream in [false, true] {
            let liquid = initial(cream).unwrap();
            let reference = evolve(liquid.clone(), 512, false).unwrap();
            let coarse = evolve(liquid.clone(), 8, false).unwrap();
            let fine = evolve(liquid.clone(), 16, false).unwrap();
            assert!(error(&fine, &reference) < 0.75 * error(&coarse, &reference));
            assert!((energy(&fine) - energy(&liquid)).abs() < 1e-9);
        }
    }
}
