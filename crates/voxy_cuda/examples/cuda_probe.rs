#[path = "support/device_argument.rs"]
mod device_argument;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ordinal = device_argument::parse(std::env::args().skip(1))?;
    let compute = voxy_cuda::CudaCompute::new(ordinal, 1024 * 1024)?;
    println!("{:#?}", compute.capabilities()?);
    verify_shared_budget(ordinal)?;
    let input: Vec<u32> = (0..4097_u32).map(|n| n.wrapping_mul(1_048_573)).collect();
    let output = compute.roundtrip_u32(&input)?;
    if output != input {
        return Err("CUDA roundtrip mismatch".into());
    }
    let output = compute.affine_u32(&input, 17, u32::MAX)?;
    let expected: Vec<_> = input
        .iter()
        .map(|value| value.wrapping_mul(17).wrapping_add(u32::MAX))
        .collect();
    if output != expected {
        return Err("CUDA kernel result mismatch".into());
    }
    // Reuse the loaded module with different parameters and a partial block.
    let repeated = compute.affine_u32(&input[..257], 0, 42)?;
    if repeated != vec![42; 257] {
        return Err("cached CUDA kernel result mismatch".into());
    }
    let mut resident = compute.upload_u32(&input)?;
    resident.affine(17, u32::MAX)?;
    resident.affine(3, 7)?;
    let chained: Vec<_> = expected
        .iter()
        .map(|v| v.wrapping_mul(3).wrapping_add(7))
        .collect();
    if resident.read()? != chained || resident.read()? != chained {
        return Err("resident CUDA chained kernel/readback mismatch".into());
    }
    // Buffer ownership retains its context, stream and module after owner drop.
    #[cfg(feature = "cuda")]
    drop(compute);
    resident.affine(0, 5)?;
    if resident.read()? != vec![5; input.len()] {
        return Err("resident CUDA ownership mismatch".into());
    }
    resident.release()?;
    println!(
        "CUDA PASS: device {ordinal}, {} u32 values transferred, transformed by kernel and read back",
        input.len()
    );
    Ok(())
}

fn verify_shared_budget(ordinal: usize) -> Result<(), Box<dyn std::error::Error>> {
    use voxy_cuda::{CudaCompute, CudaError, CudaProjectileInput};
    let compute = CudaCompute::new(ordinal, 128)?;
    let resident = compute.upload_u32(&[7; 16])?;
    if compute.reserved_device_bytes()? != 64 {
        return Err("CUDA resident reservation telemetry mismatch".into());
    }
    if !matches!(compute.upload_u32(&[0; 17]), Err(CudaError::BufferLimit))
        || !matches!(compute.roundtrip_u32(&[0; 17]), Err(CudaError::BufferLimit))
    {
        return Err("CUDA shared uploaded/transient budget not enforced".into());
    }
    let projectile = CudaProjectileInput {
        velocity: [1.0, 2.0, 3.0],
        acceleration: [0.0; 3],
    };
    if !matches!(
        compute.projectile_motion(&[projectile], 0.5),
        Err(CudaError::BufferLimit)
    ) {
        return Err("CUDA shared multi-buffer budget not enforced".into());
    }
    if resident.read()? != [7; 16] {
        return Err("CUDA budget rejection altered resident data".into());
    }
    resident.release()?;
    let motion = compute.projectile_motion(&[projectile], 0.5)?;
    if motion.len() != 1
        || motion[0].velocity.map(f64::to_bits) != [1.0_f64, 2.0, 3.0].map(f64::to_bits)
        || motion[0].displacement.map(f64::to_bits) != [0.5_f64, 1.0, 1.5].map(f64::to_bits)
    {
        return Err("CUDA budget release or fresh motion mismatch".into());
    }
    if compute.reserved_device_bytes()? != 0 {
        return Err("CUDA transient reservation leaked".into());
    }
    let full = compute.upload_u32(&[9; 32])?;
    if full.read()? != [9; 32] {
        return Err("CUDA transient budget was not released".into());
    }
    if compute.reserved_device_bytes()? != 128 {
        return Err("CUDA full reservation telemetry mismatch".into());
    }
    full.release()?;
    if compute.reserved_device_bytes()? != 0 {
        return Err("CUDA full reservation leaked".into());
    }
    println!(
        "CUDA PASS: shared resident/transient budgets reject, preserve data and release for fresh work"
    );
    Ok(())
}
