//! Analytic time-refinement control of three sensible fluid capacities and a finite body.
use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, ThermalTranslatingBody, TranslatingBody,
    TransportMaterial,
};
fn run(steps: u32, symmetric: bool) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let particles = (0..3)
        .map(|i| Particle {
            position: [0.1 * f64::from(i), 0.0, 0.0],
            velocity: [0.0; 3],
            mass: 1.0,
            material: 0,
        })
        .collect();
    let mut fluid = Liquid::new(particles, vec![Material::WATER], Config::default())?;
    fluid.configure_transport(
        vec![
            LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            },
            LiquidField {
                temperature: 310.0,
                concentration: 0.0,
            },
            LiquidField {
                temperature: 320.0,
                concentration: 0.0,
            },
        ],
        vec![TransportMaterial {
            specific_heat: 100.0,
            conductivity: 0.0,
            ..TransportMaterial::default()
        }],
    )?;
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 3.0,
        },
        specific_heat: 500.0,
        temperature: 290.0,
    };
    for _ in 0..steps {
        let dt = 1.0 / f64::from(steps);
        if symmetric {
            fluid.exchange_body_heat_symmetric(dt, &mut body, &[100.0; 3])?;
        } else {
            fluid.exchange_body_heat(dt, &mut body, &[100.0; 3])?;
        }
    }
    let equilibrium = 528000.0 / 1800.0;
    let difference = 20.0 * (-1.2_f64).exp();
    let average = equilibrium + 1500.0 / 1800.0 * difference;
    let expected_body = equilibrium - 300.0 / 1800.0 * difference;
    let mut error = (body.temperature - expected_body).abs();
    for (i, field) in fluid.fields().unwrap().iter().enumerate() {
        let initial_difference = [-10.0, 0.0, 10.0][i];
        let expected = average + initial_difference * (-1.0_f64).exp();
        error = error.max((field.temperature - expected).abs());
    }
    let energy_error =
        (fluid.transport_totals()?.unwrap().0 + body.thermal_energy()? - 528000.0).abs();
    Ok((error, energy_error))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("symmetric,steps,max_temperature_error,energy_error");
    for symmetric in [false, true] {
        for steps in [4, 8, 16, 32] {
            let (error, energy_error) = run(steps, symmetric)?;
            println!("{symmetric},{steps},{error},{energy_error}");
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sensible_star_symmetric_exchange_refines_at_second_order() {
        let coarse = run(4, true).unwrap();
        let fine = run(8, true).unwrap();
        assert!(fine.0 < 0.3 * coarse.0);
        assert!(fine.1 < 1e-8);
        let sequential = run(8, false).unwrap();
        assert!(fine.0 < 0.1 * sequential.0);
    }
}
