//! Opt-in CUDA ownership boundary. No implicit wgpu buffer sharing.
#[derive(Debug)]
pub enum CudaError {
    Disabled,
    DriverUnavailable,
    InvalidDeviceOrdinal,
    BufferLimit,
    KernelCachePoisoned,
    CompilerUnavailable,
    UnsupportedCompilerArchitecture,
    #[cfg(feature = "cuda")]
    CompilerQuery(cudarc::nvrtc::result::NvrtcError),
    InvalidTerrainInput,
    InvalidGravityInput,
    InvalidProjectileInput,
    InvalidVoxelInput,
    InvalidWaterInput,
    InvalidTissueSearchInput,
    SingularPair,
    NumericalOverflow,
    RenderViewOutOfRange,
    ContextMismatch,
    #[cfg(feature = "cuda")]
    Compile(cudarc::nvrtc::CompileError),
    #[cfg(feature = "cuda")]
    Driver(cudarc::driver::DriverError),
}
impl std::fmt::Display for CudaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(feature = "cuda")]
        if let Self::Compile(cudarc::nvrtc::CompileError::CompileError {
            nvrtc,
            options,
            log,
        }) = self
        {
            return write!(
                f,
                "CUDA shader compilation failed: {nvrtc:?}\noptions: {}\n{}",
                options.join(" "),
                log.to_string_lossy()
            );
        }
        write!(f, "CUDA error: {self:?}")
    }
}
impl std::error::Error for CudaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "cuda")]
            Self::CompilerQuery(error) => Some(error),
            #[cfg(feature = "cuda")]
            Self::Compile(error) => Some(error),
            #[cfg(feature = "cuda")]
            Self::Driver(error) => Some(error),
            _ => None,
        }
    }
}

/// Live CUDA driver properties for the explicitly selected device.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CudaCapabilities {
    pub ordinal: usize,
    /// Physical device identity for graphics/CUDA matching; never compare ordinals.
    pub uuid: [u8; 16],
    pub name: String,
    pub compute_capability: (i32, i32),
    pub total_memory_bytes: usize,
    pub allocation_budget_bytes: usize,
}

#[derive(Debug)]
pub struct CudaCompute {
    #[cfg(feature = "cuda")]
    context: std::sync::Arc<cudarc::driver::CudaContext>,
    max_bytes: usize,
    #[cfg(feature = "cuda")]
    allocation_budget: std::sync::Arc<budget::AllocationBudget>,
    #[cfg(feature = "cuda")]
    affine: std::sync::Mutex<Option<cudarc::driver::CudaFunction>>,
    #[cfg(feature = "cuda")]
    terrain: std::sync::Mutex<Option<cudarc::driver::CudaFunction>>,
    #[cfg(feature = "cuda")]
    gravity: std::sync::Mutex<Option<[cudarc::driver::CudaFunction; 5]>>,
    #[cfg(feature = "cuda")]
    projectile: std::sync::Mutex<Option<cudarc::driver::CudaFunction>>,
    #[cfg(feature = "cuda")]
    voxel_regions: std::sync::Mutex<Option<cudarc::driver::CudaFunction>>,
    #[cfg(feature = "cuda")]
    box_sweep: std::sync::Mutex<Option<cudarc::driver::CudaFunction>>,
    #[cfg(feature = "cuda")]
    water: std::sync::Mutex<Option<cudarc::driver::CudaFunction>>,
    #[cfg(feature = "cuda")]
    tissue_search: std::sync::Mutex<Option<[cudarc::driver::CudaFunction; 2]>>,
}
impl CudaCompute {
    /// Current bytes reserved by private device buffers and external imports.
    /// This is an atomic observation; concurrent operations may change it.
    /// Host staging, compiler modules and driver overhead are excluded.
    /// # Errors
    /// Returns `Disabled` when built without CUDA support.
    pub fn reserved_device_bytes(&self) -> Result<usize, CudaError> {
        #[cfg(feature = "cuda")]
        {
            Ok(self.allocation_budget.used_bytes())
        }
        #[cfg(not(feature = "cuda"))]
        {
            Err(CudaError::Disabled)
        }
    }
    /// Completes all queued work in this CUDA context before a graphics handoff.
    /// Does not copy device data to CPU.
    /// # Errors
    /// Reports disabled CUDA, asynchronous failures and recorded cleanup errors.
    pub fn synchronize(&self) -> Result<(), CudaError> {
        #[cfg(feature = "cuda")]
        {
            self.context.synchronize().map_err(CudaError::Driver)?;
            self.context.check_err().map_err(CudaError::Driver)
        }
        #[cfg(not(feature = "cuda"))]
        {
            Err(CudaError::Disabled)
        }
    }
    /// Runs a fixed CUDA kernel: `value * multiplier + bias`, wrapping at 32 bits.
    /// # Errors
    /// Rejects invalid/over-budget buffers, then reports module/launch/readback errors.
    pub fn affine_u32(
        &self,
        values: &[u32],
        multiplier: u32,
        bias: u32,
    ) -> Result<Vec<u32>, CudaError> {
        let mut buffer = self.upload_u32(values)?;
        let result = buffer.affine(multiplier, bias).and_then(|()| buffer.read());
        buffer.release()?;
        result
    }
    /// Uploads values once into an owned, reusable device allocation. Operations
    /// on this buffer share one stream; no wgpu resources are imported.
    /// Live uploaded buffers share this compute owner's allocation budget;
    /// dropping a buffer releases its reservation after device storage is freed.
    /// # Errors
    /// Rejects invalid budgets before allocation, then reports driver errors.
    pub fn upload_u32(&self, values: &[u32]) -> Result<CudaU32Buffer, CudaError> {
        let count = checked_count(values.len(), self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            let reservation = self.allocation_budget.reserve(values.len() * 4)?;
            let function = {
                let mut cached = self
                    .affine
                    .lock()
                    .map_err(|_| CudaError::KernelCachePoisoned)?;
                if cached.is_none() {
                    let module = self
                        .context
                        .load_module(cudarc::nvrtc::Ptx::from_src(include_str!("affine.ptx")))
                        .map_err(CudaError::Driver)?;
                    // Publish only after both module loading and symbol lookup succeed.
                    *cached = Some(
                        module
                            .load_function("affine_u32")
                            .map_err(CudaError::Driver)?,
                    );
                }
                cached
                    .as_ref()
                    .ok_or(CudaError::KernelCachePoisoned)?
                    .clone()
            };
            let stream = self.context.default_stream();
            let data = stream.clone_htod(values).map_err(CudaError::Driver)?;
            Ok(CudaU32Buffer {
                data,
                stream,
                function,
                count,
                _reservation: reservation,
            })
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = count;
            Err(CudaError::Disabled)
        }
    }
    /// Runs the engine's fixed procedural-terrain v1 kernel. Packed ABI: 12
    /// header words, 1024 records of 26 words, then 32768 output block IDs.
    /// Header holds seed limbs, signed clamped origin y, padding, and five IDs.
    /// Each record holds four (cell x/z limbs, x/z remainders) tuples and output
    /// height/biome. No arbitrary CUDA code or pointer is accepted by this API.
    /// # Errors
    /// Rejects malformed records/budgets before driver work; returns missing
    /// NVRTC/compiler, module, launch or readback errors without CPU fallback.
    #[allow(unsafe_code)]
    pub fn procedural_terrain(&self, words: &[u32]) -> Result<Vec<u32>, CudaError> {
        validate_terrain(words, self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{LaunchConfig, PushKernelArg};
            let reservation = self.allocation_budget.reserve(words.len() * 4)?;
            let function = {
                let mut cached = self
                    .terrain
                    .lock()
                    .map_err(|_| CudaError::KernelCachePoisoned)?;
                if cached.is_none() {
                    let ptx = cudarc::nvrtc::compile_ptx_with_opts(
                        include_str!("terrain.cu"),
                        self.compiler_options("terrain.cu")?,
                    )
                    .map_err(CudaError::Compile)?;
                    let module = self.context.load_module(ptx).map_err(CudaError::Driver)?;
                    *cached = Some(
                        module
                            .load_function("procedural_terrain")
                            .map_err(CudaError::Driver)?,
                    );
                }
                cached
                    .as_ref()
                    .ok_or(CudaError::KernelCachePoisoned)?
                    .clone()
            };
            let stream = self.context.default_stream();
            let mut data = stream.clone_htod(words).map_err(CudaError::Driver)?;
            let result = (|| {
                let mut launch = stream.launch_builder(&function);
                launch.arg(&mut data);
                // SAFETY: The immutable kernel has one u32 pointer argument. Shape
                // validation guarantees every fixed header/record/output access fits
                // the allocation. Each invocation writes distinct column/voxel slots.
                unsafe {
                    launch.launch(LaunchConfig {
                        grid_dim: (16, 1, 1),
                        block_dim: (64, 1, 1),
                        shared_mem_bytes: 0,
                    })
                }
                .map_err(CudaError::Driver)?;
                let output = stream.clone_dtoh(&data).map_err(CudaError::Driver)?;
                Ok::<_, CudaError>(output)
            })();
            reservation.release((data, function), &self.context)?;
            let output = result?;
            Ok(output)
        }
        #[cfg(not(feature = "cuda"))]
        {
            Err(CudaError::Disabled)
        }
    }
    /// Selects a CUDA device by ordinal; does not silently use another compute API.
    /// # Errors
    /// Rejects ordinals outside the driver i32 range before loading CUDA, and
    /// rejects indices beyond the actual device count before context creation.
    /// Reports disabled/unavailable/driver errors; default builds link no toolkit.
    pub fn new(ordinal: usize, max_bytes: usize) -> Result<Self, CudaError> {
        if max_bytes == 0 {
            return Err(CudaError::BufferLimit);
        }
        let driver_ordinal = i32::try_from(ordinal).map_err(|_| CudaError::InvalidDeviceOrdinal)?;
        #[cfg(feature = "cuda")]
        {
            if !driver_present() {
                return Err(CudaError::DriverUnavailable);
            }
            let count = cudarc::driver::CudaContext::device_count().map_err(CudaError::Driver)?;
            if driver_ordinal >= count {
                return Err(CudaError::InvalidDeviceOrdinal);
            }
            let context = cudarc::driver::CudaContext::new(ordinal).map_err(CudaError::Driver)?;
            Ok(Self {
                context,
                max_bytes,
                allocation_budget: budget::AllocationBudget::new(max_bytes),
                affine: std::sync::Mutex::new(None),
                terrain: std::sync::Mutex::new(None),
                gravity: std::sync::Mutex::new(None),
                projectile: std::sync::Mutex::new(None),
                voxel_regions: std::sync::Mutex::new(None),
                box_sweep: std::sync::Mutex::new(None),
                water: std::sync::Mutex::new(None),
                tissue_search: std::sync::Mutex::new(None),
            })
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = driver_ordinal;
            Err(CudaError::Disabled)
        }
    }
    /// Queries the selected CUDA device rather than inferring NVIDIA support
    /// from the graphics adapter. The allocation budget is application policy.
    /// # Errors
    /// Returns disabled or driver query errors.
    pub fn capabilities(&self) -> Result<CudaCapabilities, CudaError> {
        #[cfg(feature = "cuda")]
        {
            Ok(CudaCapabilities {
                ordinal: self.context.ordinal(),
                uuid: self
                    .context
                    .uuid()
                    .map_err(CudaError::Driver)?
                    .bytes
                    .map(|byte| byte.to_ne_bytes()[0]),
                name: self.context.name().map_err(CudaError::Driver)?,
                compute_capability: self
                    .context
                    .compute_capability()
                    .map_err(CudaError::Driver)?,
                total_memory_bytes: self.context.total_mem().map_err(CudaError::Driver)?,
                allocation_budget_bytes: self.max_bytes,
            })
        }
        #[cfg(not(feature = "cuda"))]
        {
            Err(CudaError::Disabled)
        }
    }
    /// Transfers owned u32 values to CUDA memory and reads them back synchronously.
    /// Device memory is private to this call and freed after readback.
    /// # Errors
    /// Rejects empty/over-budget buffers before allocation, then reports driver failures.
    pub fn roundtrip_u32(&self, values: &[u32]) -> Result<Vec<u32>, CudaError> {
        let bytes = values.len().checked_mul(4).ok_or(CudaError::BufferLimit)?;
        if bytes == 0 || bytes > self.max_bytes {
            return Err(CudaError::BufferLimit);
        }
        #[cfg(feature = "cuda")]
        {
            let reservation = self.allocation_budget.reserve(bytes)?;
            let stream = self.context.default_stream();
            let buffer = stream.clone_htod(values).map_err(CudaError::Driver)?;
            let output = stream.clone_dtoh(&buffer).map_err(CudaError::Driver);
            reservation.release(buffer, &self.context)?;
            output
        }
        #[cfg(not(feature = "cuda"))]
        {
            Err(CudaError::Disabled)
        }
    }
}
/// An owned CUDA allocation with ordered in-place kernels and explicit readback.
/// Drop releases device storage through cudarc's stream-aware ownership.
#[derive(Debug)]
pub struct CudaU32Buffer {
    #[cfg(feature = "cuda")]
    data: cudarc::driver::CudaSlice<u32>,
    #[cfg(feature = "cuda")]
    stream: std::sync::Arc<cudarc::driver::CudaStream>,
    #[cfg(feature = "cuda")]
    function: cudarc::driver::CudaFunction,
    #[cfg(feature = "cuda")]
    count: u32,
    #[cfg(feature = "cuda")]
    _reservation: budget::Reservation,
}
impl CudaU32Buffer {
    /// Completes pending work and checks storage destruction before releasing budget.
    /// Failure retains the budget charge conservatively.
    /// # Errors
    /// Reports CUDA synchronization/cleanup failures or disabled CUDA support.
    pub fn release(self) -> Result<(), CudaError> {
        #[cfg(feature = "cuda")]
        {
            let Self {
                data,
                stream,
                function,
                _reservation: reservation,
                ..
            } = self;
            reservation.release((data, function), stream.context())
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = self;
            Err(CudaError::Disabled)
        }
    }
    /// Enqueues the fixed affine kernel without transferring data to the host.
    /// # Errors
    /// Reports driver launch errors or a disabled CUDA build.
    #[allow(unsafe_code)]
    pub fn affine(&mut self, multiplier: u32, bias: u32) -> Result<(), CudaError> {
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{LaunchConfig, PushKernelArg};
            let config = LaunchConfig {
                grid_dim: (self.count.div_ceil(256), 1, 1),
                block_dim: (256, 1, 1),
                shared_mem_bytes: 0,
            };
            let mut launch = self.stream.launch_builder(&self.function);
            launch
                .arg(&mut self.data)
                .arg(&self.count)
                .arg(&multiplier)
                .arg(&bias);
            // SAFETY: Embedded immutable PTX matches these typed arguments. The
            // count guard bounds every access to this exact count-sized buffer.
            // cudarc tracks stream usage and retains the module via CudaFunction.
            unsafe { launch.launch(config) }.map_err(CudaError::Driver)?;
            Ok(())
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = (multiplier, bias);
            Err(CudaError::Disabled)
        }
    }

    /// Synchronizes ordered work and copies device results to the host. The
    /// allocation remains usable for subsequent kernels and repeated reads.
    /// # Errors
    /// Reports driver transfer/synchronization errors or disabled CUDA builds.
    pub fn read(&self) -> Result<Vec<u32>, CudaError> {
        #[cfg(feature = "cuda")]
        {
            let output = self
                .stream
                .clone_dtoh(&self.data)
                .map_err(CudaError::Driver)?;
            self.stream.synchronize().map_err(CudaError::Driver)?;
            self.stream
                .context()
                .check_err()
                .map_err(CudaError::Driver)?;
            Ok(output)
        }
        #[cfg(not(feature = "cuda"))]
        {
            Err(CudaError::Disabled)
        }
    }
}

/// Fixed procedural-terrain storage size, in u32 words.
pub const TERRAIN_WORD_COUNT: usize = 12 + 1024 * 26 + 32768;

fn validate_terrain(words: &[u32], max_bytes: usize) -> Result<(), CudaError> {
    if words.len() != TERRAIN_WORD_COUNT {
        return Err(CudaError::InvalidTerrainInput);
    }
    checked_count(words.len(), max_bytes)?;
    let y = i32::from_ne_bytes(words[2].to_ne_bytes());
    if !(-64..=32).contains(&y) {
        return Err(CudaError::InvalidTerrainInput);
    }
    for record in words[12..26636].chunks_exact(26) {
        for (scale, period) in [128, 192, 32, 8].into_iter().enumerate() {
            if record[scale * 6 + 4] >= period || record[scale * 6 + 5] >= period {
                return Err(CudaError::InvalidTerrainInput);
            }
        }
    }
    Ok(())
}

fn checked_count(count: usize, max_bytes: usize) -> Result<u32, CudaError> {
    let bytes = count.checked_mul(4).ok_or(CudaError::BufferLimit)?;
    if count == 0 || bytes > max_bytes {
        return Err(CudaError::BufferLimit);
    }
    u32::try_from(count).map_err(|_| CudaError::BufferLimit)
}
#[cfg(feature = "cuda")]
#[allow(unsafe_code)]
fn driver_present() -> bool {
    // cudarc exposes this dynamic-library availability probe only as unsafe.
    // It does not pass pointers or execute kernels; check before APIs that panic
    // when no driver library can be loaded.
    unsafe { cudarc::driver::sys::is_culib_present() }
}
#[cfg(test)]
mod tests {
    #[cfg(feature = "cuda")]
    #[test]
    fn shader_error_preserves_readable_log_and_error_source() {
        use std::error::Error;
        let error = super::CudaError::Compile(cudarc::nvrtc::CompileError::CompileError {
            nvrtc: cudarc::nvrtc::result::NvrtcError(
                cudarc::nvrtc::sys::nvrtcResult::NVRTC_ERROR_COMPILATION,
            ),
            options: vec![
                "--gpu-architecture=compute_89".into(),
                "--fmad=false".into(),
            ],
            log: std::ffi::CString::new("kernel.cu(7): error: invalid token\nsecond diagnostic\n")
                .unwrap(),
        });
        let message = error.to_string();
        assert!(message.contains("NVRTC_ERROR_COMPILATION"));
        assert!(message.contains("options: --gpu-architecture=compute_89 --fmad=false"));
        assert!(message.contains("kernel.cu(7): error: invalid token\nsecond diagnostic\n"));
        assert!(!message.contains("\\n"));
        assert!(
            error
                .source()
                .unwrap()
                .downcast_ref::<cudarc::nvrtc::CompileError>()
                .is_some()
        );
    }
    use super::*;
    #[test]
    fn terrain_shape_validation_precedes_driver_work() {
        let mut input = vec![0; TERRAIN_WORD_COUNT];
        assert!(validate_terrain(&input, input.len() * 4).is_ok());
        assert!(matches!(
            validate_terrain(&input[..12], usize::MAX),
            Err(CudaError::InvalidTerrainInput)
        ));
        assert!(matches!(
            validate_terrain(&input, input.len() * 4 - 1),
            Err(CudaError::BufferLimit)
        ));
        input[2] = 33;
        assert!(matches!(
            validate_terrain(&input, usize::MAX),
            Err(CudaError::InvalidTerrainInput)
        ));
        input[2] = u32::from_ne_bytes((-64_i32).to_ne_bytes());
        assert!(validate_terrain(&input, usize::MAX).is_ok());
        input[12 + 4] = 128;
        assert!(matches!(
            validate_terrain(&input, usize::MAX),
            Err(CudaError::InvalidTerrainInput)
        ));
    }
    #[test]
    fn kernel_launch_bounds() {
        assert_eq!(checked_count(257, 1028).unwrap(), 257);
        for (count, budget) in [(0, 10), (257, 1027), (usize::MAX, usize::MAX)] {
            assert!(matches!(
                checked_count(count, budget),
                Err(CudaError::BufferLimit)
            ));
        }
    }
    #[test]
    fn large_ordinals_do_not_truncate_to_a_different_device() {
        for ordinal in [usize::MAX, usize::try_from(i32::MAX).unwrap() + 1] {
            assert!(matches!(
                CudaCompute::new(ordinal, 1024),
                Err(CudaError::InvalidDeviceOrdinal)
            ));
        }
    }
    #[test]
    fn zero_budget_rejects_before_driver_access() {
        assert!(matches!(
            CudaCompute::new(0, 0),
            Err(CudaError::BufferLimit)
        ));
    }
    #[test]
    #[cfg(not(feature = "cuda"))]
    fn buffer_budget_rejects_before_driver_access() {
        {
            let compute = CudaCompute { max_bytes: 4 };
            assert!(matches!(
                compute.roundtrip_u32(&[]),
                Err(CudaError::BufferLimit)
            ));
            assert!(matches!(
                compute.roundtrip_u32(&[1, 2]),
                Err(CudaError::BufferLimit)
            ));
            assert!(matches!(CudaCompute::new(0, 4), Err(CudaError::Disabled)));
        }
    }
}

mod gravity;
pub use gravity::{
    CudaGravityBody, CudaGravityBudget, CudaGravityFailure, CudaGravityJob, CudaGravityParameters,
    CudaGravitySnapshot,
};

#[cfg(windows)]
mod d3d12;
mod external;
pub use external::CudaExternalU32Buffer;

#[cfg(any(feature = "cuda", test))]
mod compiler;

mod projectile;
pub use projectile::{CudaProjectileInput, CudaProjectileMotion};

mod voxel_regions;

mod box_sweep;
pub use box_sweep::{CudaBoxContact, CudaBoxSweep};

mod water;

#[cfg(any(feature = "cuda", test))]
mod budget;

mod tissue_search;
