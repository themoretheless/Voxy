//! Plan step D4: the CUDA gravity backend against the CPU f64 solver, through the shared
//! `GravitySolver` contract and the same reference check the wgpu backend passes.
#![cfg(feature = "cuda")]
use voxy_cuda::{CudaCompute, CudaGravityBody, CudaGravityBudget, CudaGravityParameters};
use voxy_gpu_core::{GravitySolver, compare_with_oracle};

#[test]
fn cuda_gravity_matches_the_cpu_oracle_through_the_shared_contract() {
    let compute = CudaCompute::new(0, 16 * 1024 * 1024).expect("CUDA device required");
    let bodies: Vec<CudaGravityBody> = (0..257_u32)
        .map(|index| CudaGravityBody {
            mass: 1.0 + f64::from(index % 7) * 0.25,
            position: [
                f64::from(index % 17) * 0.5,
                f64::from(index / 17) * 0.5,
                f64::from(index % 3) * 0.25,
            ],
            velocity: [0.01, -0.02, 0.005],
        })
        .collect();
    let parameters = CudaGravityParameters {
        constant: 0.03,
        softening: 0.25,
        uniform_acceleration: [0.0, -0.01, 0.0],
        dt: 1.0 / 1024.0,
    };
    let mut job = compute
        .create_gravity_job(&bodies, parameters, CudaGravityBudget::default())
        .unwrap();
    assert_eq!(job.body_count(), 257);
    let cpu = physics::gravity::Gravity {
        constant: parameters.constant,
        softening: parameters.softening,
        uniform_acceleration: parameters.uniform_acceleration,
    };
    let mut reference = bodies.clone();
    let report = compare_with_oracle(&mut job, &mut reference, 4, 32, 1e-9, |bodies, steps| {
        let mut cpu_bodies: Vec<_> = bodies
            .iter()
            .map(|body| physics::gravity::Body {
                mass: body.mass,
                position: body.position,
                velocity: body.velocity,
            })
            .collect();
        for _ in 0..steps {
            cpu.step(&mut cpu_bodies, parameters.dt)
                .map_err(|failure| format!("{failure:?}"))?;
        }
        for (body, cpu_body) in bodies.iter_mut().zip(&cpu_bodies) {
            body.position = cpu_body.position;
            body.velocity = cpu_body.velocity;
        }
        Ok(())
    })
    .unwrap();
    assert!(report.max_difference <= 1e-9, "{report:?}");
    job.release().unwrap();
}
