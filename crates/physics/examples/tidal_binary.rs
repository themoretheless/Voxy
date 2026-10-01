//! Run with `cargo run -p physics --example tidal_binary`.
use physics::{
    astrophysics::OrbitalState,
    astrophysics_binary::{Binary, Body, Tide},
    astrophysics_spin::Spin,
};
fn body(mass: f64, radius: f64) -> Body {
    Body {
        mass,
        radius,
        spin: Spin {
            orientation: [0.0, 0.0, 0.0, 1.0],
            angular_momentum: [0.0; 3],
            inertia: [0.4 * mass * radius * radius; 3],
        },
        tide: Tide::default(),
        heat: 0.0,
    }
}
fn main() -> Result<(), String> {
    let mut system = Binary {
        relative: OrbitalState {
            position: [2.0, 0.0, 0.0],
            velocity: [0.0, (11.0_f64 / 2.0).sqrt(), 0.0],
        },
        primary: body(10.0, 0.3),
        secondary: body(1.0, 0.5),
    };
    system.secondary.spin.angular_momentum = [0.0, 0.0, 0.4];
    system.secondary.tide = Tide {
        love_number: 0.5,
        time_lag: 0.01,
    };
    let initial = system.diagnostics(1.0).map_err(|e| format!("{e:?}"))?;
    println!("time,spin_z,heat,total_energy_error,angular_momentum_error");
    for step in 0..=20_000 {
        if step % 2000 == 0 {
            let d = system.diagnostics(1.0).map_err(|e| format!("{e:?}"))?;
            let omega = system
                .secondary
                .spin
                .angular_velocity()
                .map_err(|e| format!("{e:?}"))?;
            let momentum_error = d
                .angular_momentum
                .iter()
                .zip(initial.angular_momentum)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f64, f64::max);
            println!(
                "{:.3},{:.9},{:.9},{:.3e},{:.3e}",
                f64::from(step) * 0.001,
                omega[2],
                d.heat,
                d.mechanical_energy + d.heat - initial.mechanical_energy - initial.heat,
                momentum_error
            );
        }
        if step < 20_000 {
            system
                .step(1.0, 0.001, 0.001, 1)
                .map_err(|e| format!("{e:?}"))?;
        }
    }
    Ok(())
}
