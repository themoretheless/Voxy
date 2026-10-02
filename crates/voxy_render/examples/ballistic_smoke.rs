//! Ordered resident GPU physics steps compared against an f64 CPU trajectory.
use voxy_render::{BALLISTIC_SHADER, ComputeProgram, GraphicsOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Ballistic GPU: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let program = pollster::block_on(ComputeProgram::new(&device, BALLISTIC_SHADER))?;
    let count = 257_u32;
    let dt = 1.0_f32 / 64.0;
    let mut input = vec![0.0_f32, -9.0, 0.0, dt];
    for i in 0..count {
        #[allow(clippy::cast_precision_loss)]
        let x = i as f32;
        input.extend_from_slice(&[x, 100.0, -x, 1.0, 2.0, 3.0, -1.0, 0.0]);
    }
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let job = program.create_job(&device, bytemuck::cast_slice(&input))?;
    // Multiple submissions prove that resident data survives frame boundaries.
    for _ in 0..3 {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for _ in 0..16 {
            job.encode_step(&mut encoder, [count.div_ceil(64), 1, 1])?;
        }
        queue.submit([encoder.finish()]);
    }
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    // Final encode also performs one step before copying for readback.
    let dispatch = job.encode(&mut encoder, [count.div_ceil(64), 1, 1])?;
    queue.submit([encoder.finish()]);
    let mut readback = dispatch.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let bytes = readback.try_read()?.ok_or("readback pending")?;
    let output: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
        .collect();
    assert_eq!(&output[..4], &input[..4]);
    let t = 49.0 * f64::from(dt);
    for (before, after) in input[4..].chunks_exact(8).zip(output[4..].chunks_exact(8)) {
        for axis in 0..3 {
            let a = f64::from(input[axis]);
            let position =
                f64::from(before[axis]) + f64::from(before[4 + axis]) * t + 0.5 * a * t * t;
            let velocity = f64::from(before[4 + axis]) + a * t;
            assert!((f64::from(after[axis]) - position).abs() < 0.001);
            assert!((f64::from(after[4 + axis]) - velocity).abs() < 0.001);
        }
        assert_eq!(after[3].to_bits(), before[3].to_bits());
        assert_eq!(after[7].to_bits(), before[7].to_bits());
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    println!("PASS: 257 bodies, 49 resident GPU steps, 4 submissions, f64 trajectory comparison");
    Ok(())
}
