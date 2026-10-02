use crate::{CudaCompute, CudaError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CudaProjectileInput {
    pub velocity: [f64; 3],
    pub acceleration: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CudaProjectileMotion {
    pub velocity: [f64; 3],
    pub displacement: [f64; 3],
}

impl CudaCompute {
    /// Computes uniform-acceleration Euler motion for a batch of local velocities.
    /// # Errors
    /// Uses the same precise kernel, budgets and failure contract as `projectile_motion`.
    pub fn euler_motion(
        &self,
        inputs: &[CudaProjectileInput],
        dt: f64,
    ) -> Result<Vec<CudaProjectileMotion>, CudaError> {
        self.projectile_motion(inputs, dt)
    }

    /// Integrates one bounded batch with the voxel projectile solver's
    /// semi-implicit Euler rule. No absolute coordinates or collision data
    /// are converted to device floating point. No CPU fallback is performed.
    /// # Errors
    /// Rejects invalid input and aggregate allocation budgets before driver
    /// work. Reports compile/transfer failures and nonfinite device output.
    #[allow(unsafe_code)]
    pub fn projectile_motion(
        &self,
        inputs: &[CudaProjectileInput],
        dt: f64,
    ) -> Result<Vec<CudaProjectileMotion>, CudaError> {
        let (count, packed) = pack(inputs, dt, self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{LaunchConfig, PushKernelArg};
            let allocation_bytes = packed.len().checked_mul(16).ok_or(CudaError::BufferLimit)?;
            let reservation = self.allocation_budget.reserve(allocation_bytes)?;
            let function = {
                let mut cached = self
                    .projectile
                    .lock()
                    .map_err(|_| CudaError::KernelCachePoisoned)?;
                if cached.is_none() {
                    let ptx = cudarc::nvrtc::compile_ptx_with_opts(
                        include_str!("projectile.cu"),
                        self.compiler_options("projectile.cu")?,
                    )
                    .map_err(CudaError::Compile)?;
                    let module = self.context.load_module(ptx).map_err(CudaError::Driver)?;
                    *cached = Some(
                        module
                            .load_function("projectile_motion")
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
            let mut output = match stream.alloc_zeros::<f64>(packed.len()) {
                Ok(output) => output,
                Err(error) => {
                    reservation.release((input, function), &self.context)?;
                    return Err(CudaError::Driver(error));
                }
            };
            let result = (|| {
                let mut launch = stream.launch_builder(&function);
                launch.arg(&input).arg(&mut output).arg(&count).arg(&dt);
                // SAFETY: The fixed kernel's six-double records and count are
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

fn pack(
    inputs: &[CudaProjectileInput],
    dt: f64,
    max_bytes: usize,
) -> Result<(u32, Vec<f64>), CudaError> {
    let count = u32::try_from(inputs.len()).map_err(|_| CudaError::BufferLimit)?;
    if count == 0
        || count > u32::MAX / 6
        || inputs
            .len()
            .checked_mul(96)
            .is_none_or(|bytes| bytes > max_bytes)
    {
        return Err(CudaError::BufferLimit);
    }
    if !dt.is_finite()
        || !(0.0..=1.0).contains(&dt)
        || inputs.iter().any(|input| {
            input
                .velocity
                .iter()
                .chain(&input.acceleration)
                .any(|v| !v.is_finite())
        })
    {
        return Err(CudaError::InvalidProjectileInput);
    }
    let mut packed = Vec::with_capacity(inputs.len() * 6);
    for input in inputs {
        packed.extend(input.velocity);
        packed.extend(input.acceleration);
    }
    Ok((count, packed))
}

#[cfg(any(feature = "cuda", test))]
fn decode(words: &[f64]) -> Result<Vec<CudaProjectileMotion>, CudaError> {
    if words.is_empty() || !words.len().is_multiple_of(6) {
        return Err(CudaError::InvalidProjectileInput);
    }
    if words.iter().any(|v| !v.is_finite()) {
        return Err(CudaError::NumericalOverflow);
    }
    Ok(words
        .chunks_exact(6)
        .map(|record| CudaProjectileMotion {
            velocity: [record[0], record[1], record[2]],
            displacement: [record[3], record[4], record[5]],
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregate_budget_and_inputs_are_validated_before_allocation() {
        let input = CudaProjectileInput {
            velocity: [0.0; 3],
            acceleration: [0.0, -24.0, 0.0],
        };
        assert!(matches!(
            pack(&[input], 0.1, 95),
            Err(CudaError::BufferLimit)
        ));
        assert!(pack(&[input], 0.1, 96).is_ok());
        assert!(matches!(pack(&[], 0.1, 96), Err(CudaError::BufferLimit)));
        assert!(matches!(
            pack(&[input], f64::NAN, 96),
            Err(CudaError::InvalidProjectileInput)
        ));
        assert!(matches!(
            decode(&[f64::INFINITY; 6]),
            Err(CudaError::NumericalOverflow)
        ));
        assert!(matches!(
            decode(&[0.0; 5]),
            Err(CudaError::InvalidProjectileInput)
        ));
    }
}
