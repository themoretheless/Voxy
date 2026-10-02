//! Requires a real NVIDIA device; compares the actual CUDA kernel to f64 Euler.
#[path = "support/device_argument.rs"]
mod device_argument;
use voxy_cuda::{CudaCompute, CudaError, CudaProjectileInput};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ordinal = device_argument::parse(std::env::args().skip(1))?;
    let compute = CudaCompute::new(ordinal, 1024 * 1024)?;
    println!("CUDA projectile: {:?}", compute.capabilities()?);
    let mut inputs: Vec<_> = (0..257_u32)
        .map(|i| CudaProjectileInput {
            velocity: [f64::from(i) * 0.125, -0.25, -0.0],
            acceleration: [0.125, -24.0, 0.0],
        })
        .collect();
    let dt = 1.0 / 64.0;
    for _ in 0..128 {
        let motions = compute.projectile_motion(&inputs, dt)?;
        assert_eq!(motions.len(), inputs.len());
        for (input, motion) in inputs.iter_mut().zip(motions) {
            for axis in 0..3 {
                let velocity = input.velocity[axis] + input.acceleration[axis] * dt;
                assert_eq!(motion.velocity[axis].to_bits(), velocity.to_bits());
                assert_eq!(
                    motion.displacement[axis].to_bits(),
                    (velocity * dt).to_bits()
                );
            }
            input.velocity = motion.velocity;
        }
    }
    let overflow = CudaProjectileInput {
        velocity: [f64::MAX; 3],
        acceleration: [f64::MAX; 3],
    };
    assert!(matches!(
        compute.projectile_motion(&[overflow], 1.0),
        Err(CudaError::NumericalOverflow)
    ));
    // A failed batch must not poison a subsequent independent integration.
    assert_eq!(compute.projectile_motion(&inputs, 0.0)?.len(), 257);
    println!(
        "CUDA PASS: 257 f64 projectiles x128 batches, exact Euler motion, overflow rejection and recovery"
    );
    Ok(())
}
