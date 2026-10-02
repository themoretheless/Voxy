//! Browser GPU readback compared against the same independent f64 solver as native probes.
use super::browser::{error, yield_browser};
use voxy_gpu::{GravityBody, GravityBudget, GravityJob, GravityParameters, GravityProgram};
use wasm_bindgen::JsValue;

pub(crate) async fn validate(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<f64, JsValue> {
    let program = GravityProgram::new(device, GravityBudget::default())
        .await
        .map_err(error)?;
    let bodies: Vec<_> = (0..257_u32)
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
        .collect();
    let parameters = GravityParameters {
        constant: 0.03,
        softening: 0.25,
        uniform_acceleration: [0.0, -0.01, 0.0],
        dt: 1.0 / 1024.0,
    };
    let solver = physics::gravity::Gravity {
        constant: f64::from(parameters.constant),
        softening: f64::from(parameters.softening),
        uniform_acceleration: parameters.uniform_acceleration.map(f64::from),
    };
    let mut cpu: Vec<_> = bodies
        .iter()
        .map(|body| physics::gravity::Body {
            mass: f64::from(body.mass),
            position: body.position.map(f64::from),
            velocity: body.velocity.map(f64::from),
        })
        .collect();
    let job = program
        .create_job(device, &bodies, parameters)
        .map_err(error)?;
    for _ in 0..4 {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        job.encode_steps(&mut encoder, 32).map_err(error)?;
        queue.submit([encoder.finish()]);
        for _ in 0..32 {
            solver
                .step(&mut cpu, f64::from(parameters.dt))
                .map_err(|failure| error(format!("CPU gravity: {failure:?}")))?;
        }
        yield_browser().await?;
    }
    let gpu = read(device, queue, &job).await?;
    if gpu.len() != cpu.len() {
        return Err(error("browser gravity body count mismatch"));
    }
    let mut maximum_error = 0.0_f64;
    for (actual, expected) in gpu.iter().zip(&cpu) {
        if f64::from(actual.mass).to_bits() != expected.mass.to_bits() {
            return Err(error("browser gravity mass changed"));
        }
        for (actual, expected) in actual
            .position
            .iter()
            .chain(&actual.velocity)
            .zip(expected.position.iter().chain(&expected.velocity))
        {
            let difference = (f64::from(*actual) - *expected).abs();
            if !difference.is_finite() || difference > 0.0002 {
                return Err(error(format!(
                    "browser GPU/f64 gravity mismatch: {difference}"
                )));
            }
            maximum_error = maximum_error.max(difference);
        }
    }
    Ok(maximum_error)
}

async fn read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    job: &GravityJob,
) -> Result<Vec<GravityBody>, JsValue> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let dispatch = job.encode_readback(device, &mut encoder).map_err(error)?;
    queue.submit([encoder.finish()]);
    let mut pending = dispatch.begin_read();
    let deadline = js_sys::Date::now() + 30000.0;
    loop {
        if let Some(bodies) = pending.try_read().map_err(error)? {
            return Ok(bodies);
        }
        if js_sys::Date::now() >= deadline {
            return Err(error("browser gravity readback timed out"));
        }
        yield_browser().await?;
    }
}
