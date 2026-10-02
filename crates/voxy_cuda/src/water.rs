use crate::{CudaCompute, CudaError};
impl CudaCompute {
    /// Executes the ordered packed water graph ABI. Header status and sampled
    /// provenance are returned even on solver failure; never publish them without
    /// decoding the status and revision-checking the captured world transaction.
    /// # Errors
    /// Rejects malformed graphs and aggregate input/output budgets before driver
    /// work. Reports CUDA compilation/transfer errors; no CPU fallback.
    #[allow(unsafe_code)]
    pub fn water_graph(&self, words: &[u32]) -> Result<Vec<u32>, CudaError> {
        validate(words, self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{LaunchConfig, PushKernelArg};
            let allocation_bytes = words.len().checked_mul(4).ok_or(CudaError::BufferLimit)?;
            let reservation = self.allocation_budget.reserve(allocation_bytes)?;
            let function = {
                let mut cache = self
                    .water
                    .lock()
                    .map_err(|_| CudaError::KernelCachePoisoned)?;
                if cache.is_none() {
                    let ptx = cudarc::nvrtc::compile_ptx_with_opts(
                        include_str!("water.cu"),
                        self.compiler_options("water.cu")?,
                    )
                    .map_err(CudaError::Compile)?;
                    let module = self.context.load_module(ptx).map_err(CudaError::Driver)?;
                    *cache = Some(
                        module
                            .load_function("water_transfer")
                            .map_err(CudaError::Driver)?,
                    );
                }
                cache
                    .as_ref()
                    .ok_or(CudaError::KernelCachePoisoned)?
                    .clone()
            };
            let stream = self.context.default_stream();
            let mut buffer = stream.clone_htod(words).map_err(CudaError::Driver)?;
            let result = (|| {
                let mut launch = stream.launch_builder(&function);
                launch.arg(&mut buffer);
                // SAFETY: validated graph indices and lengths fit this allocation;
                // only one thread runs the ordered solver.
                unsafe {
                    launch.launch(LaunchConfig {
                        grid_dim: (1, 1, 1),
                        block_dim: (1, 1, 1),
                        shared_mem_bytes: 0,
                    })
                }
                .map_err(CudaError::Driver)?;
                let output = stream.clone_dtoh(&buffer).map_err(CudaError::Driver)?;
                Ok::<_, CudaError>(output)
            })();
            reservation.release((buffer, function), &self.context)?;
            let output = result?;
            Ok(output)
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = words;
            Err(CudaError::Disabled)
        }
    }
}
fn validate(words: &[u32], max_bytes: usize) -> Result<(), CudaError> {
    if words.len() < 8 {
        return Err(CudaError::InvalidWaterInput);
    }
    let n = words[0] as usize;
    let active = words[1] as usize;
    if n == 0
        || n > 131_072
        || active > 16_384
        || words.len() != 8 + n * 8 + active
        || !(1..=8).contains(&words[2])
        || !(1..=8).contains(&words[3])
        || words[4] == 0
        || words[5] != 0
        || words[6] != 0
        || words[7] == 0
    {
        return Err(CudaError::InvalidWaterInput);
    }
    for node in words[8..8 + n * 8].chunks_exact(8) {
        if node[0] > 11
            || node[1] != node[0]
            || node[2] != 0
            || node[3..].iter().any(|&i| i >= words[0] && i < u32::MAX - 1)
        {
            return Err(CudaError::InvalidWaterInput);
        }
    }
    let indices = &words[8 + n * 8..];
    if indices.iter().any(|&i| i >= words[0]) || indices.windows(2).any(|p| p[0] >= p[1]) {
        return Err(CudaError::InvalidWaterInput);
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
    fn graph_bounds_precede_driver_access() {
        let words = [1, 1, 8, 4, 8, 0, 0, 1, 8, 8, 0, 0, 0, 0, 0, 0, 0];
        assert!(validate(&words, 136).is_ok());
        assert!(matches!(validate(&words, 135), Err(CudaError::BufferLimit)));
        for index in [0, 2, 4, 7] {
            let mut invalid = words;
            invalid[index] = 0;
            assert!(validate(&invalid, usize::MAX).is_err());
        }
        for (index, value) in [(8, 12), (9, 7), (10, 1), (11, 1), (16, 1), (5, 1)] {
            let mut invalid = words;
            invalid[index] = value;
            assert!(validate(&invalid, usize::MAX).is_err());
        }
    }

    #[test]
    fn graph_shape_and_active_order_are_validated() {
        let node = [0, 0, 0, u32::MAX, u32::MAX - 1, 0, 1, 0];
        let mut words = vec![2, 2, 8, 4, 8, 0, 0, 2];
        words.extend(node);
        words.extend(node);
        words.extend([0, 1]);
        assert!(validate(&words, usize::MAX).is_ok());
        for length in [0, 7, 8, words.len() - 1] {
            assert!(matches!(
                validate(&words[..length], usize::MAX),
                Err(CudaError::InvalidWaterInput)
            ));
        }
        let mut trailing = words.clone();
        trailing.push(0);
        assert!(matches!(
            validate(&trailing, usize::MAX),
            Err(CudaError::InvalidWaterInput)
        ));
        for indices in [[1, 0], [0, 0], [0, 2]] {
            let mut invalid = words.clone();
            invalid[24..].copy_from_slice(&indices);
            assert!(matches!(
                validate(&invalid, usize::MAX),
                Err(CudaError::InvalidWaterInput)
            ));
        }
        for (index, value) in [
            (0, 131_073),
            (0, u32::MAX),
            (1, 16_385),
            (1, u32::MAX),
            (2, 9),
            (3, 0),
            (3, 9),
            (6, 1),
            (11, 2),
            (11, u32::MAX - 2),
        ] {
            let mut invalid = words.clone();
            invalid[index] = value;
            assert!(matches!(
                validate(&invalid, usize::MAX),
                Err(CudaError::InvalidWaterInput)
            ));
        }
    }
}
