//! Real NVIDIA f64 gravity acceptance; requires CUDA driver and NVRTC.
#[path = "support/device_argument.rs"]
mod device_argument;
use voxy_cuda::{CudaCompute, CudaGravityBody, CudaGravityBudget, CudaGravityParameters};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ordinal = device_argument::parse(std::env::args().skip(1))?;
    let compute = CudaCompute::new(ordinal, 1024 * 1024)?;
    println!("CUDA gravity: {:?}", compute.capabilities()?);
    verify_failures(&compute)?;
    verify_shared_budget(ordinal)?;
    let input: Vec<_> = (0..257_u32)
        .map(|i| CudaGravityBody {
            mass: 1.0 + f64::from(i % 7) * 0.25,
            position: [
                f64::from(i % 17) * 0.5,
                f64::from(i / 17) * 0.5,
                f64::from(i % 3) * 0.25,
            ],
            velocity: [0.01, -0.02, 0.005],
        })
        .collect();
    let parameters = CudaGravityParameters {
        constant: 0.03,
        softening: 0.25,
        uniform_acceleration: [0.0; 3],
        dt: 1.0 / 1024.0,
    };
    let mut job = compute.create_gravity_job(&input, parameters, CudaGravityBudget::default())?;
    let cpu_gravity = physics::gravity::Gravity {
        constant: parameters.constant,
        softening: parameters.softening,
        uniform_acceleration: parameters.uniform_acceleration,
    };
    let mut cpu: Vec<_> = input
        .iter()
        .map(|body| physics::gravity::Body {
            mass: body.mass,
            position: body.position,
            velocity: body.velocity,
        })
        .collect();
    // Retained stream/context/module ownership must permit use after owner drop.
    #[cfg(feature = "cuda")]
    drop(compute);
    for _ in 0..4 {
        job.step(32)?;
        for _ in 0..32 {
            cpu_gravity
                .step(&mut cpu, parameters.dt)
                .map_err(|error| format!("CPU gravity: {error:?}"))?;
        }
    }
    let actual = job.read()?;
    assert_eq!(actual, job.read()?);
    assert_eq!(actual.len(), cpu.len());
    let mut max_error = 0.0_f64;
    for (a, b) in actual.iter().zip(&cpu) {
        assert_eq!(a.mass.to_bits(), b.mass.to_bits());
        for (x, y) in a
            .position
            .iter()
            .chain(&a.velocity)
            .zip(b.position.iter().chain(&b.velocity))
        {
            let error = (x - y).abs();
            max_error = max_error.max(error);
            if error > 1e-10 {
                return Err(format!("CUDA f64 trajectory difference {error}").into());
            }
        }
    }
    job.release()?;
    println!(
        "CUDA PASS: 257 f64 bodies x128 resident Verlet steps, CPU tolerance 1e-10, max error {max_error}; repeated readback and owner-drop lifetime"
    );
    Ok(())
}

fn verify_failures(compute: &CudaCompute) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_cuda::CudaError;
    let parameters = CudaGravityParameters {
        constant: 1.0,
        softening: 0.0,
        uniform_acceleration: [0.0; 3],
        dt: 1.0,
    };
    let body = CudaGravityBody {
        mass: 1.0,
        position: [0.0; 3],
        velocity: [0.0; 3],
    };
    let mut singular =
        compute.create_gravity_job(&[body, body], parameters, CudaGravityBudget::default())?;
    let before = singular.snapshot()?.bodies;
    singular.step(1)?;
    same_state(&singular.snapshot()?.bodies, &before);
    assert!(matches!(singular.read(), Err(CudaError::SingularPair)));
    singular.step(2)?;
    same_state(&singular.snapshot()?.bodies, &before);
    assert!(matches!(singular.read(), Err(CudaError::SingularPair)));
    let mut overflowing = body;
    overflowing.position = [f64::MAX; 3];
    overflowing.velocity = [f64::MAX; 3];
    let mut overflow = compute.create_gravity_job(
        &[overflowing],
        CudaGravityParameters {
            constant: 0.0,
            ..parameters
        },
        CudaGravityBudget::default(),
    )?;
    let before = overflow.snapshot()?.bodies;
    overflow.step(1)?;
    same_state(&overflow.snapshot()?.bodies, &before);
    assert!(matches!(overflow.read(), Err(CudaError::NumericalOverflow)));
    overflow.step(2)?;
    same_state(&overflow.snapshot()?.bodies, &before);
    assert!(matches!(overflow.read(), Err(CudaError::NumericalOverflow)));
    assert!(matches!(singular.step(0), Err(CudaError::BufferLimit)));
    assert!(matches!(singular.step(257), Err(CudaError::BufferLimit)));
    assert!(matches!(
        compute.create_gravity_job(
            &[body, body],
            parameters,
            CudaGravityBudget {
                max_bodies: 1,
                max_steps_per_call: 256,
            }
        ),
        Err(CudaError::BufferLimit)
    ));
    println!("CUDA PASS: singular/overflow sticky errors and step/body budgets");
    Ok(())
}

fn same_state(actual: &[CudaGravityBody], expected: &[CudaGravityBody]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.mass.to_bits(), expected.mass.to_bits());
        for (actual, expected) in actual
            .position
            .iter()
            .chain(&actual.velocity)
            .zip(expected.position.iter().chain(&expected.velocity))
        {
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
    }
}

fn verify_shared_budget(ordinal: usize) -> Result<(), Box<dyn std::error::Error>> {
    let compute = CudaCompute::new(ordinal, 256)?;
    let body = CudaGravityBody {
        mass: 1.0,
        position: [0.0; 3],
        velocity: [0.0; 3],
    };
    let parameters = CudaGravityParameters {
        constant: 1.0,
        softening: 0.1,
        uniform_acceleration: [0.0; 3],
        dt: 0.01,
    };
    let job = compute.create_gravity_job(&[body], parameters, CudaGravityBudget::default())?;
    if compute.reserved_device_bytes()? != 256 {
        return Err("CUDA gravity reservation size mismatch".into());
    }
    if !matches!(
        compute.upload_u32(&[0]),
        Err(voxy_cuda::CudaError::BufferLimit)
    ) || !matches!(
        compute.create_gravity_job(&[body], parameters, CudaGravityBudget::default()),
        Err(voxy_cuda::CudaError::BufferLimit)
    ) {
        return Err("CUDA gravity shared budget not enforced".into());
    }
    if job.read()? != [body] {
        return Err("CUDA gravity budget rejection changed resident state".into());
    }
    job.release()?;
    if compute.reserved_device_bytes()? != 0 {
        return Err("CUDA gravity reservation leaked".into());
    }
    let buffer = compute.upload_u32(&[3; 64])?;
    if buffer.read()? != [3; 64] {
        return Err("CUDA gravity budget was not released".into());
    }
    if compute.reserved_device_bytes()? != 256 {
        return Err("CUDA gravity replacement reservation mismatch".into());
    }
    buffer.release()?;
    if compute.reserved_device_bytes()? != 0 {
        return Err("CUDA gravity replacement reservation leaked".into());
    }
    println!(
        "CUDA PASS: gravity shared budget rejects competing allocations and releases for fresh work"
    );
    Ok(())
}
