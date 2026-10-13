//! Participating-medium species transport (plan step B4), sharing the packed
//! word ABI with the wgpu kernel. The host launches one kernel per substep and
//! flips the ping-pong parity word in between; validation precedes any driver
//! work and the CPU reference in `physics::medium` remains the admission authority.
use crate::{CudaCompute, CudaError};

/// Header words before the velocity, mass and settled arrays.
pub const MEDIUM_HEADER_WORDS: usize = 16;
/// Upper bound on cells per job.
pub const MEDIUM_MAX_CELLS: usize = 1_000_000;

/// Packs one species transport job into the shared word ABI.
/// # Errors
/// Rejects empty or oversized grids, mismatched arrays, nonfinite or
/// nonpositive parameters.
#[allow(clippy::too_many_arguments)]
pub fn pack_medium_job(
    shape: [usize; 3],
    spacing: [f32; 3],
    up: [f32; 3],
    substeps: usize,
    dt: f32,
    settling: f32,
    diffusivity: f32,
    gas_velocity: &[[f32; 3]],
    mass: &[f32],
) -> Result<Vec<u32>, CudaError> {
    let cells = shape.iter().product::<usize>();
    if cells == 0
        || cells > MEDIUM_MAX_CELLS
        || shape.iter().any(|&n| n == 0 || n > u32::MAX as usize)
        || gas_velocity.len() != cells
        || mass.len() != cells
        || substeps == 0
        || substeps > u32::MAX as usize
        || !dt.is_finite()
        || dt < 0.
        || spacing.iter().any(|s| !s.is_finite() || *s <= 0.)
        || up.iter().any(|u| !u.is_finite())
        || !settling.is_finite()
        || settling < 0.
        || !diffusivity.is_finite()
        || diffusivity < 0.
        || gas_velocity.iter().flatten().any(|v| !v.is_finite())
        || mass.iter().any(|m| !m.is_finite() || *m < 0.)
    {
        return Err(CudaError::InvalidMediumInput);
    }
    let mut words = Vec::with_capacity(MEDIUM_HEADER_WORDS + cells * 6);
    words.push(cells as u32);
    words.extend(shape.map(|n| n as u32));
    words.push(substeps as u32);
    words.push((dt / substeps as f32).to_bits());
    words.extend(spacing.map(f32::to_bits));
    words.extend(up.map(f32::to_bits));
    words.push(settling.to_bits());
    words.push(diffusivity.to_bits());
    words.push(spacing.iter().product::<f32>().to_bits());
    words.push(0);
    words.extend(gas_velocity.iter().flatten().map(|v| v.to_bits()));
    words.extend(mass.iter().map(|m| m.to_bits()));
    words.extend(mass.iter().map(|m| m.to_bits()));
    words.extend(std::iter::repeat_n(0f32.to_bits(), cells));
    Ok(words)
}

/// Mass per cell and total settled mass from a finished job's words.
/// # Errors
/// Rejects a malformed word buffer.
pub fn unpack_medium_result(words: &[u32]) -> Result<(Vec<f32>, f32), CudaError> {
    validate(words, usize::MAX)?;
    let cells = words[0] as usize;
    let parity = words[15] as usize;
    let base = MEDIUM_HEADER_WORDS + cells * 3;
    let mass = words[base + parity * cells..base + (parity + 1) * cells]
        .iter()
        .map(|w| f32::from_bits(*w))
        .collect();
    let settled = words[base + 2 * cells..base + 3 * cells]
        .iter()
        .map(|w| f32::from_bits(*w))
        .sum();
    Ok((mass, settled))
}

impl CudaCompute {
    /// Runs every substep of a packed medium job and returns the final words.
    /// # Errors
    /// Rejects malformed jobs and budgets before driver work; reports CUDA
    /// compilation/transfer errors; no CPU fallback.
    #[allow(unsafe_code)]
    pub fn medium_transport(&self, words: &[u32]) -> Result<Vec<u32>, CudaError> {
        validate(words, self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{LaunchConfig, PushKernelArg};
            let allocation_bytes = words.len().checked_mul(4).ok_or(CudaError::BufferLimit)?;
            let reservation = self.allocation_budget.reserve(allocation_bytes)?;
            let function = {
                let mut cache = self
                    .medium
                    .lock()
                    .map_err(|_| CudaError::KernelCachePoisoned)?;
                if cache.is_none() {
                    let ptx = cudarc::nvrtc::compile_ptx_with_opts(
                        include_str!("medium.cu"),
                        self.compiler_options("medium.cu")?,
                    )
                    .map_err(CudaError::Compile)?;
                    let module = self.context.load_module(ptx).map_err(CudaError::Driver)?;
                    *cache = Some(
                        module
                            .load_function("medium_substep")
                            .map_err(CudaError::Driver)?,
                    );
                }
                cache
                    .as_ref()
                    .ok_or(CudaError::KernelCachePoisoned)?
                    .clone()
            };
            let stream = self.context.default_stream();
            let cells = words[0];
            let substeps = words[4];
            let mut buffer = stream.clone_htod(words).map_err(CudaError::Driver)?;
            let result = (|| {
                for step in 0..substeps {
                    // The parity word selects the source half for this substep.
                    let parity = [step % 2];
                    let mut parity_view = buffer.slice_mut(15..16);
                    stream
                        .memcpy_htod(&parity, &mut parity_view)
                        .map_err(CudaError::Driver)?;
                    let mut launch = stream.launch_builder(&function);
                    launch.arg(&mut buffer);
                    // SAFETY: validated cell count bounds every index the kernel touches.
                    unsafe {
                        launch.launch(LaunchConfig {
                            grid_dim: (cells.div_ceil(256), 1, 1),
                            block_dim: (256, 1, 1),
                            shared_mem_bytes: 0,
                        })
                    }
                    .map_err(CudaError::Driver)?;
                }
                let mut output = stream.clone_dtoh(&buffer).map_err(CudaError::Driver)?;
                // The result lives in the half the last substep wrote.
                output[15] = substeps % 2;
                Ok::<_, CudaError>(output)
            })();
            reservation.release((buffer, function), &self.context)?;
            result
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = words;
            Err(CudaError::Disabled)
        }
    }
}

fn validate(words: &[u32], max_bytes: usize) -> Result<(), CudaError> {
    if words.len() < MEDIUM_HEADER_WORDS {
        return Err(CudaError::InvalidMediumInput);
    }
    let cells = words[0] as usize;
    let shape = [words[1] as usize, words[2] as usize, words[3] as usize];
    let product = shape.iter().try_fold(1usize, |a, &b| a.checked_mul(b));
    let finite = |w: u32| f32::from_bits(w).is_finite();
    if cells == 0
        || cells > MEDIUM_MAX_CELLS
        || product != Some(cells)
        || words[4] == 0
        || words.len() != MEDIUM_HEADER_WORDS + cells * 6
        || words[15] > 1
        || !(5..15).all(|i| finite(words[i]))
        || (6..9).any(|i| f32::from_bits(words[i]) <= 0.)
        || f32::from_bits(words[5]) < 0.
        || f32::from_bits(words[12]) < 0.
        || f32::from_bits(words[13]) < 0.
        || words[MEDIUM_HEADER_WORDS..].iter().any(|w| !finite(*w))
    {
        return Err(CudaError::InvalidMediumInput);
    }
    if words
        .len()
        .checked_mul(8)
        .is_none_or(|bytes| bytes > max_bytes)
    {
        return Err(CudaError::BufferLimit);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_validate_and_unpack_round_trip() {
        let shape = [2usize, 3, 1];
        let velocity = vec![[0.1f32, -0.2, 0.0]; 6];
        let mass: Vec<f32> = (0..6).map(|i| i as f32 * 0.5).collect();
        let words = pack_medium_job(
            shape,
            [0.5; 3],
            [0., 1., 0.],
            4,
            0.2,
            0.3,
            1e-3,
            &velocity,
            &mass,
        )
        .unwrap();
        assert_eq!(words.len(), MEDIUM_HEADER_WORDS + 6 * 6);
        assert!(validate(&words, usize::MAX).is_ok());
        assert_eq!(f32::from_bits(words[5]), 0.05);
        assert_eq!(f32::from_bits(words[14]), 0.125);
        let (unpacked, settled) = unpack_medium_result(&words).unwrap();
        assert_eq!(unpacked, mass);
        assert_eq!(settled, 0.);
        assert!(matches!(validate(&words, 8), Err(CudaError::BufferLimit)));
        let mut bad = words.clone();
        bad[1] = 5;
        assert!(matches!(
            validate(&bad, usize::MAX),
            Err(CudaError::InvalidMediumInput)
        ));
        let mut bad = words.clone();
        bad[4] = 0;
        assert!(validate(&bad, usize::MAX).is_err());
        let mut bad = words.clone();
        bad[15] = 2;
        assert!(validate(&bad, usize::MAX).is_err());
        let mut bad = words.clone();
        bad[20] = f32::NAN.to_bits();
        assert!(validate(&bad, usize::MAX).is_err());
        assert!(validate(&words[..words.len() - 1], usize::MAX).is_err());
        assert!(
            pack_medium_job(
                shape,
                [0.5; 3],
                [0., 1., 0.],
                4,
                0.2,
                -1.,
                0.,
                &velocity,
                &mass
            )
            .is_err()
        );
        assert!(
            pack_medium_job([0, 1, 1], [0.5; 3], [0., 1., 0.], 4, 0.2, 0., 0., &[], &[]).is_err()
        );
    }
}
