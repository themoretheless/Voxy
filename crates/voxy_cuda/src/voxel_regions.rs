use crate::{CudaCompute, CudaError};
impl CudaCompute {
    /// Classifies inclusive local integer regions on CUDA. Classes: empty 0, solid 1,
    /// unavailable 2, unknown 3. Returns count, first solid and first fault; missing
    /// candidates use `u32::MAX`. Limits match the graphics compute broadphase.
    /// # Errors
    /// Rejects invalid grids, queries, aggregate work and allocation budgets before
    /// driver work. Reports compilation/transfer errors. No CPU fallback.
    #[allow(unsafe_code)]
    pub fn voxel_regions(
        &self,
        dimensions: [u32; 3],
        cells: &[u32],
        regions: &[[u32; 6]],
    ) -> Result<Vec<[u32; 3]>, CudaError> {
        let words = pack(dimensions, cells, regions, self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{LaunchConfig, PushKernelArg};
            let allocation_bytes = words.len().checked_mul(4).ok_or(CudaError::BufferLimit)?;
            let reservation = self.allocation_budget.reserve(allocation_bytes)?;
            let function = {
                let mut cache = self
                    .voxel_regions
                    .lock()
                    .map_err(|_| CudaError::KernelCachePoisoned)?;
                if cache.is_none() {
                    let ptx = cudarc::nvrtc::compile_ptx_with_opts(
                        include_str!("voxel_regions.cu"),
                        self.compiler_options("voxel_regions.cu")?,
                    )
                    .map_err(CudaError::Compile)?;
                    let module = self.context.load_module(ptx).map_err(CudaError::Driver)?;
                    *cache = Some(
                        module
                            .load_function("voxel_regions")
                            .map_err(CudaError::Driver)?,
                    );
                }
                cache
                    .as_ref()
                    .ok_or(CudaError::KernelCachePoisoned)?
                    .clone()
            };
            let stream = self.context.default_stream();
            let mut buffer = stream.clone_htod(&words).map_err(CudaError::Driver)?;
            let result = (|| {
                let mut launch = stream.launch_builder(&function);
                launch.arg(&mut buffer);
                // SAFETY: validated header/grid/query bounds fit one initialized allocation;
                // each thread writes only its query's three disjoint output words.
                unsafe {
                    launch.launch(LaunchConfig {
                        grid_dim: (words[3].div_ceil(64), 1, 1),
                        block_dim: (64, 1, 1),
                        shared_mem_bytes: 0,
                    })
                }
                .map_err(CudaError::Driver)?;
                let output = stream.clone_dtoh(&buffer).map_err(CudaError::Driver)?;
                Ok::<_, CudaError>(output)
            })();
            reservation.release((buffer, function), &self.context)?;
            let output = result?;
            decode(&words, &output)
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = words;
            Err(CudaError::Disabled)
        }
    }
}
fn pack(
    dimensions: [u32; 3],
    cells: &[u32],
    regions: &[[u32; 6]],
    max_bytes: usize,
) -> Result<Vec<u32>, CudaError> {
    let count = dimensions
        .iter()
        .try_fold(1_u32, |n, &d| {
            if d == 0 || d > 512 {
                None
            } else {
                n.checked_mul(d)
            }
        })
        .filter(|&n| n <= 131_072)
        .ok_or(CudaError::InvalidVoxelInput)?;
    if usize::try_from(count).ok() != Some(cells.len())
        || cells.iter().any(|&c| c > 3)
        || regions.is_empty()
        || regions.len() > 16_384
        || regions
            .iter()
            .any(|r| (0..3).any(|a| r[a] > r[a + 3] || r[a + 3] >= dimensions[a]))
    {
        return Err(CudaError::InvalidVoxelInput);
    }
    let visits: u64 = regions
        .iter()
        .map(|r| {
            (0..3)
                .map(|a| u64::from(r[a + 3] - r[a] + 1))
                .product::<u64>()
        })
        .sum();
    if visits > 4_194_304 {
        return Err(CudaError::BufferLimit);
    }
    let len = 5 + cells.len() + regions.len() * 9;
    if len.checked_mul(4).is_none_or(|bytes| bytes > max_bytes) {
        return Err(CudaError::BufferLimit);
    }
    let mut words = vec![
        dimensions[0],
        dimensions[1],
        dimensions[2],
        u32::try_from(regions.len()).map_err(|_| CudaError::BufferLimit)?,
        count,
    ];
    words.extend(cells);
    for region in regions {
        words.extend(region);
        words.extend([0, u32::MAX, u32::MAX]);
    }
    Ok(words)
}
#[cfg(any(feature = "cuda", test))]
fn decode(input: &[u32], output: &[u32]) -> Result<Vec<[u32; 3]>, CudaError> {
    let base = 5 + usize::try_from(input[4]).map_err(|_| CudaError::InvalidVoxelInput)?;
    if output.len() != input.len() || input[..base] != output[..base] {
        return Err(CudaError::InvalidVoxelInput);
    }
    input[base..]
        .chunks_exact(9)
        .zip(output[base..].chunks_exact(9))
        .map(|(original, row)| {
            let volume: u32 = (0..3).map(|a| original[a + 3] - original[a] + 1).product();
            let candidate = |index: u32, fault: bool| {
                if index == u32::MAX {
                    return true;
                }
                if index >= input[4] {
                    return false;
                }
                let pos = [
                    index / (input[1] * input[2]),
                    index / input[2] % input[1],
                    index % input[2],
                ];
                let class = input[5 + index as usize];
                (0..3).all(|a| pos[a] >= original[a] && pos[a] <= original[a + 3])
                    && if fault { class >= 2 } else { class == 1 }
            };
            if row[..6] != original[..6]
                || row[6] > volume
                || (row[6] == 0) != (row[7] == u32::MAX)
                || !candidate(row[7], false)
                || !candidate(row[8], true)
            {
                return Err(CudaError::InvalidVoxelInput);
            }
            Ok([row[6], row[7], row[8]])
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limits_and_corrupt_candidates_reject() {
        let words = pack([1, 1, 3], &[1, 1, 2], &[[0, 0, 1, 0, 0, 1]], 1024).unwrap();
        let mut output = words.clone();
        let base = 8;
        output[base + 6] = 1;
        output[base + 7] = 1;
        assert_eq!(decode(&words, &output).unwrap(), [[1, 1, u32::MAX]]);
        output[base + 7] = 0;
        assert!(decode(&words, &output).is_err());
        assert!(pack([0, 1, 3], &[1, 1, 2], &[[0; 6]], 1024).is_err());
        assert!(pack([1, 1, 3], &[1, 1, 2], &[[0; 6]], 1).is_err());
    }
}
