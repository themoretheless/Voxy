use crate::{CudaCompute, CudaError};
/// One continuous local f64 AABB sweep against a local obstacle box.
#[derive(Clone, Copy, Debug)]
pub struct CudaBoxSweep {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub displacement: [f64; 3],
    pub obstacle_min: [f64; 3],
    pub obstacle_max: [f64; 3],
}
/// Optional contact fraction and canonical axis normal.
pub type CudaBoxContact = Option<(f64, [i8; 3])>;
impl CudaCompute {
    /// Maximum box queries per batch within the combined input/output budget.
    #[must_use]
    pub fn box_sweep_capacity(&self) -> usize {
        (self.max_bytes / 160).min((u32::MAX / 15) as usize)
    }

    /// Runs independent exact local continuous box sweeps in parallel on CUDA.
    /// Coordinate anchors stay external. No CPU fallback is performed.
    /// # Errors
    /// Rejects invalid bounds/budgets and reports compile/transfer/output errors.
    #[allow(unsafe_code)]
    pub fn box_sweeps(&self, inputs: &[CudaBoxSweep]) -> Result<Vec<CudaBoxContact>, CudaError> {
        let (count, packed) = pack(inputs, self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{LaunchConfig, PushKernelArg};
            let allocation_bytes = inputs
                .len()
                .checked_mul(160)
                .ok_or(CudaError::BufferLimit)?;
            let reservation = self.allocation_budget.reserve(allocation_bytes)?;
            let function = {
                let mut cached = self
                    .box_sweep
                    .lock()
                    .map_err(|_| CudaError::KernelCachePoisoned)?;
                if cached.is_none() {
                    let ptx = cudarc::nvrtc::compile_ptx_with_opts(
                        include_str!("box_sweep.cu"),
                        self.compiler_options("box_sweep.cu")?,
                    )
                    .map_err(CudaError::Compile)?;
                    let module = self.context.load_module(ptx).map_err(CudaError::Driver)?;
                    *cached = Some(
                        module
                            .load_function("box_sweep")
                            .map_err(CudaError::Driver)?,
                    );
                }
                cached
                    .as_ref()
                    .ok_or(CudaError::KernelCachePoisoned)?
                    .clone()
            };
            let stream = self.context.default_stream();
            let input = stream.clone_htod(&packed).map_err(CudaError::Driver)?;
            let mut output = match stream.alloc_zeros::<f64>(inputs.len() * 5) {
                Ok(output) => output,
                Err(error) => {
                    reservation.release((input, function), &self.context)?;
                    return Err(CudaError::Driver(error));
                }
            };
            let result = (|| {
                let mut launch = stream.launch_builder(&function);
                launch.arg(&input).arg(&mut output).arg(&count);
                // SAFETY: The fixed kernel's fifteen-double records and count are
                // validated together; disjoint threads write initialized output.
                unsafe {
                    launch.launch(LaunchConfig {
                        grid_dim: (count.div_ceil(64), 1, 1),
                        block_dim: (64, 1, 1),
                        shared_mem_bytes: 0,
                    })
                }
                .map_err(CudaError::Driver)?;
                let words = stream.clone_dtoh(&output).map_err(CudaError::Driver)?;
                Ok::<_, CudaError>(words)
            })();
            reservation.release((input, output, function), &self.context)?;
            let words = result?;
            decode(&words)
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = (count, packed);
            Err(CudaError::Disabled)
        }
    }
}

fn pack(inputs: &[CudaBoxSweep], max_bytes: usize) -> Result<(u32, Vec<f64>), CudaError> {
    let count = u32::try_from(inputs.len()).map_err(|_| CudaError::BufferLimit)?;
    if count == 0
        || count > u32::MAX / 15
        || inputs.len().checked_mul(160).is_none_or(|n| n > max_bytes)
    {
        return Err(CudaError::BufferLimit);
    }
    let mut packed = Vec::with_capacity(inputs.len() * 15);
    for input in inputs {
        if input
            .min
            .iter()
            .chain(&input.max)
            .chain(&input.displacement)
            .chain(&input.obstacle_min)
            .chain(&input.obstacle_max)
            .any(|v| !v.is_finite() || v.abs() > 2_097_153.0)
            || (0..3).any(|a| {
                input.min[a] >= input.max[a] || input.obstacle_min[a] >= input.obstacle_max[a]
            })
        {
            return Err(CudaError::InvalidVoxelInput);
        }
        packed.extend(input.min);
        packed.extend(input.max);
        packed.extend(input.displacement);
        packed.extend(input.obstacle_min);
        packed.extend(input.obstacle_max);
    }
    Ok((count, packed))
}
#[cfg(any(feature = "cuda", test))]
// Hit flags and axis normals are discrete protocol values, so approximate equality is invalid.
#[allow(clippy::float_cmp)]
fn decode(words: &[f64]) -> Result<Vec<CudaBoxContact>, CudaError> {
    if !words.len().is_multiple_of(5) {
        return Err(CudaError::InvalidVoxelInput);
    }
    words
        .chunks_exact(5)
        .map(|row| {
            if row.iter().any(|v| !v.is_finite()) {
                return Err(CudaError::InvalidVoxelInput);
            }
            if row[0] == 0.0 {
                if row[1..].iter().any(|&v| v != 0.0) {
                    return Err(CudaError::InvalidVoxelInput);
                }
                return Ok(None);
            }
            if row[0] != 1.0
                || !(0.0..=1.0).contains(&row[1])
                || row[2..].iter().any(|&v| v != -1.0 && v != 0.0 && v != 1.0)
                || row[2..].iter().filter(|&&v| v != 0.0).count() > 1
            {
                return Err(CudaError::InvalidVoxelInput);
            }
            #[allow(clippy::cast_possible_truncation)]
            Ok(Some((row[1], [row[2] as i8, row[3] as i8, row[4] as i8])))
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combined_input_output_budget_boundary() {
        let input = CudaBoxSweep {
            min: [0.0; 3],
            max: [1.0; 3],
            displacement: [1.0, 0.0, 0.0],
            obstacle_min: [2.0; 3],
            obstacle_max: [3.0; 3],
        };
        assert!(matches!(pack(&[input], 159), Err(CudaError::BufferLimit)));
        assert!(pack(&[input], 160).is_ok());
        assert!(matches!(
            pack(&[input; 2], 319),
            Err(CudaError::BufferLimit)
        ));
        assert!(pack(&[input; 2], 320).is_ok());
    }
    #[test]
    fn corrupt_contacts_reject() {
        assert_eq!(
            decode(&[1.0, 0.25, -1.0, 0.0, 0.0]).unwrap(),
            [Some((0.25, [-1, 0, 0]))]
        );
        for row in [
            [1.0, 2.0, 0.0, 0.0, 0.0],
            [1.0, 0.5, 1.0, 1.0, 0.0],
            [0.0, 0.5, 0.0, 0.0, 0.0],
            [1.0, 0.5, 0.25, 0.0, 0.0],
        ] {
            assert!(decode(&row).is_err());
        }
    }
}
