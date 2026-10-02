//! Resident GPU velocity-Verlet compared with the existing f64 CPU solver.
#[path = "support/native_backend.rs"]
mod native_backend;
use physics::gravity::{Body, Gravity};
use voxy_gpu::{
    GravityBody, GravityBudget, GravityComputeError, GravityParameters, GravityProgram,
};
use voxy_render::{ComputeDispatch, ComputeError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = native_backend::parse(std::env::args().skip(1))?;
    let instance = options.create_instance();
    let adapter = pollster::block_on(options.adapter(&instance))?;
    println!("Gravity GPU: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let program = pollster::block_on(GravityProgram::new(&device, GravityBudget::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let bodies: Vec<_> = (0..257_u32)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let position = [
                (i % 17) as f32 * 0.5,
                (i / 17) as f32 * 0.5,
                (i % 3) as f32 * 0.25,
            ];
            #[allow(clippy::cast_precision_loss)]
            let mass = 1.0 + (i % 7) as f32 * 0.25;
            GravityBody {
                mass,
                position,
                velocity: [0.01, -0.02, 0.005],
            }
        })
        .collect();
    let parameters = GravityParameters {
        constant: 0.03,
        softening: 0.25,
        uniform_acceleration: [0.0, -0.01, 0.0],
        dt: 1.0 / 1024.0,
    };
    let cpu_gravity = Gravity {
        constant: f64::from(parameters.constant),
        softening: f64::from(parameters.softening),
        uniform_acceleration: parameters.uniform_acceleration.map(f64::from),
    };
    let mut cpu: Vec<_> = bodies
        .iter()
        .map(|b| Body {
            mass: f64::from(b.mass),
            position: b.position.map(f64::from),
            velocity: b.velocity.map(f64::from),
        })
        .collect();
    let job = program.create_job(&device, &bodies, parameters)?;
    for _ in 0..4 {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        job.encode_steps(&mut encoder, 32)?;
        queue.submit([encoder.finish()]);
        for _ in 0..32 {
            cpu_gravity
                .step(&mut cpu, f64::from(parameters.dt))
                .map_err(|error| format!("CPU gravity: {error:?}"))?;
        }
    }
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let read = job.encode_readback(&device, &mut encoder)?;
    queue.submit([encoder.finish()]);
    let mut read = read.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let gpu = read.try_read()?.ok_or("gravity mapping pending")?;
    let mut maximum_error = 0.0_f64;
    for (actual, expected) in gpu.iter().zip(&cpu) {
        assert_eq!(f64::from(actual.mass).to_bits(), expected.mass.to_bits());
        for (a, b) in actual
            .position
            .iter()
            .chain(&actual.velocity)
            .zip(expected.position.iter().chain(&expected.velocity))
        {
            let error = (f64::from(*a) - *b).abs();
            maximum_error = maximum_error.max(error);
            assert!(error < 0.0002, "GPU/CPU trajectory difference {error}");
        }
    }
    assert!(matches!(
        read.try_read(),
        Err(GravityComputeError::Compute(ComputeError::Consumed))
    ));
    verify_errors(&device, &queue, &program)?;
    verify_orbit(&device, &queue, &program)?;
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    println!(
        "PASS: 257 bodies, 128 resident Verlet steps, f64 CPU parity tolerance 0.0002, measured max error {maximum_error}; singular/overflow atomicity and budgets"
    );
    Ok(())
}

fn verify_errors(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    program: &GravityProgram,
) -> Result<(), Box<dyn std::error::Error>> {
    let parameters = GravityParameters {
        constant: 1.0,
        softening: 0.0,
        uniform_acceleration: [0.0; 3],
        dt: 1.0,
    };
    let before = [
        GravityBody {
            mass: 1.0,
            position: [-1.0, 0.0, 0.0],
            velocity: [0.875, 0.0, 0.0],
        },
        GravityBody {
            mass: 1.0,
            position: [1.0, 0.0, 0.0],
            velocity: [-0.875, 0.0, 0.0],
        },
    ];
    assert!(matches!(
        program.create_job(device, &[], parameters),
        Err(GravityComputeError::Budget)
    ));
    let invalid = GravityParameters {
        dt: f32::NAN,
        ..parameters
    };
    assert!(matches!(
        program.create_job(device, &before, invalid),
        Err(GravityComputeError::InvalidInput)
    ));
    for overflow in [false, true] {
        let mut input = before;
        if overflow {
            input[0].position[0] = -1e30;
            input[1].position[0] = 1e30;
        }
        let job = program.create_job(device, &input, parameters)?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        assert!(matches!(
            job.encode_steps(&mut encoder, 0),
            Err(GravityComputeError::Budget)
        ));
        assert!(matches!(
            job.encode_steps(&mut encoder, 257),
            Err(GravityComputeError::Budget)
        ));
        for (offset, size) in [(0, 3), (1, 4), (job.buffer().size(), 4)] {
            assert!(matches!(
                ComputeDispatch::copy_buffer(device, &mut encoder, job.buffer(), offset, size),
                Err(ComputeError::InvalidBuffer)
            ));
        }
        job.encode_steps(&mut encoder, 2)?;
        let read = job.encode_readback(device, &mut encoder)?;
        let raw = ComputeDispatch::copy_buffer(device, &mut encoder, job.buffer(), 32, 64)?;
        queue.submit([encoder.finish()]);
        let mut read = read.begin_read();
        let mut raw = raw.begin_read();
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let result = read.try_read();
        if overflow {
            assert!(matches!(
                result,
                Err(GravityComputeError::NumericalOverflow)
            ));
        } else {
            assert!(matches!(result, Err(GravityComputeError::SingularPair)));
        }
        let bytes = raw.try_read()?.ok_or("raw mapping pending")?;
        let expected: Vec<u32> = input
            .iter()
            .flat_map(|body| {
                body.position
                    .into_iter()
                    .chain([body.mass])
                    .chain(body.velocity)
                    .chain([0.0])
                    .map(f32::to_bits)
            })
            .collect();
        assert_eq!(
            bytes,
            bytemuck::cast_slice::<u32, u8>(&expected),
            "failed passes changed committed input"
        );
    }
    Ok(())
}

fn verify_orbit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    program: &GravityProgram,
) -> Result<(), Box<dyn std::error::Error>> {
    let speed = (1.0_f32 / 6.0).sqrt();
    let bodies = [
        GravityBody {
            mass: 1.0,
            position: [-1.5, 0.0, 0.0],
            velocity: [0.0, -speed, 0.0],
        },
        GravityBody {
            mass: 1.0,
            position: [1.5, 0.0, 0.0],
            velocity: [0.0, speed, 0.0],
        },
    ];
    let mut cpu: Vec<_> = bodies
        .iter()
        .map(|body| Body {
            mass: f64::from(body.mass),
            position: body.position.map(f64::from),
            velocity: body.velocity.map(f64::from),
        })
        .collect();
    let gravity = Gravity {
        constant: 1.0,
        softening: 0.0,
        uniform_acceleration: [0.0; 3],
    };
    let initial = gravity
        .diagnostics(&cpu)
        .map_err(|e| format!("diagnostics {e:?}"))?;
    let parameters = GravityParameters {
        constant: 1.0,
        softening: 0.0,
        uniform_acceleration: [0.0; 3],
        dt: 1.0 / 256.0,
    };
    let job = program.create_job(device, &bodies, parameters)?;
    for _ in 0..10 {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        job.encode_steps(&mut encoder, 256)?;
        queue.submit([encoder.finish()]);
        for _ in 0..256 {
            gravity
                .step(&mut cpu, f64::from(parameters.dt))
                .map_err(|e| format!("orbit {e:?}"))?;
        }
    }
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let read = job.encode_readback(device, &mut encoder)?;
    queue.submit([encoder.finish()]);
    let mut read = read.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let gpu = read.try_read()?.ok_or("orbit pending")?;
    let actual: Vec<_> = gpu
        .iter()
        .map(|body| Body {
            mass: f64::from(body.mass),
            position: body.position.map(f64::from),
            velocity: body.velocity.map(f64::from),
        })
        .collect();
    let final_state = gravity
        .diagnostics(&actual)
        .map_err(|e| format!("diagnostics {e:?}"))?;
    let initial_energy = initial.kinetic_energy + initial.potential_energy;
    let final_energy = final_state.kinetic_energy + final_state.potential_energy;
    let drift = ((final_energy - initial_energy) / initial_energy).abs();
    assert!(drift < 0.001, "orbit relative energy drift {drift}");
    for (a, b) in actual.iter().zip(&cpu) {
        for (x, y) in a
            .position
            .iter()
            .chain(&a.velocity)
            .zip(b.position.iter().chain(&b.velocity))
        {
            assert!((x - y).abs() < 0.001, "orbit f64 comparison {x} {y}");
        }
    }
    assert!(final_state.linear_momentum.iter().all(|v| v.abs() < 1e-6));
    println!(
        "PASS: 2560 resident orbit steps, relative energy drift {drift}, f64 trajectory tolerance 0.001 and momentum tolerance 1e-6"
    );
    Ok(())
}
