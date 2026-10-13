//! Plan step D4: the wgpu gravity backend against the CPU f64 solver, through the shared
//! `GravitySolver` contract and the shared reference check. Needs a real adapter (lavapipe
//! suffices); the noop backend executes no shaders.
use voxy_gpu::{GravityBody, GravityBudget, GravityParameters, GravityProgram};
use voxy_gpu_core::{GravitySolver, compare_with_oracle};

fn bodies() -> Vec<GravityBody> {
    (0..257_u32)
        .map(|index| {
            #[allow(clippy::cast_precision_loss)]
            let position = [
                (index % 17) as f32 * 0.5,
                (index / 17) as f32 * 0.5,
                (index % 3) as f32 * 0.25,
            ];
            #[allow(clippy::cast_precision_loss)]
            let mass = 1.0 + (index % 7) as f32 * 0.25;
            GravityBody {
                mass,
                position,
                velocity: [0.01, -0.02, 0.005],
            }
        })
        .collect()
}

#[test]
fn wgpu_gravity_matches_the_cpu_oracle_through_the_shared_contract() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("GPU required for the gravity reference check");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program =
        pollster::block_on(GravityProgram::new(&device, GravityBudget::default())).unwrap();
    let parameters = GravityParameters {
        constant: 0.03,
        softening: 0.25,
        uniform_acceleration: [0.0, -0.01, 0.0],
        dt: 1.0 / 1024.0,
    };
    let bodies = bodies();
    let mut job = program
        .create_job(&device, &bodies, parameters)
        .unwrap()
        .blocking(device.clone(), queue);
    assert_eq!(job.body_count(), 257);

    let cpu = physics::gravity::Gravity {
        constant: f64::from(parameters.constant),
        softening: f64::from(parameters.softening),
        uniform_acceleration: parameters.uniform_acceleration.map(f64::from),
    };
    let dt = f64::from(parameters.dt);
    let mut reference: Vec<_> = bodies.iter().map(|body| body.widened()).collect();
    let report = compare_with_oracle(&mut job, &mut reference, 4, 32, 2e-4, |bodies, steps| {
        let mut cpu_bodies: Vec<_> = bodies
            .iter()
            .map(|body| physics::gravity::Body {
                mass: body.mass,
                position: body.position,
                velocity: body.velocity,
            })
            .collect();
        for _ in 0..steps {
            cpu.step(&mut cpu_bodies, dt)
                .map_err(|failure| format!("{failure:?}"))?;
        }
        for (body, cpu_body) in bodies.iter_mut().zip(&cpu_bodies) {
            body.position = cpu_body.position;
            body.velocity = cpu_body.velocity;
        }
        Ok(())
    })
    .unwrap();
    assert_eq!((report.rounds, report.steps_per_round), (4, 32));
    assert!(report.max_difference <= 2e-4, "{report:?}");
}
