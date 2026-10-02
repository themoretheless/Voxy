//! Platform-specific graphics allocation import. Platform exporters must provide exclusive
//! ownership and match the CUDA device; this module does not export wgpu memory.
use crate::{CudaCompute, CudaError};

/// CUDA mapping of an external allocation, with synchronous fixed writes.
/// Use `release` to check cleanup before returning the allocation to graphics.
#[derive(Debug)]
pub struct CudaExternalU32Buffer {
    #[cfg(feature = "cuda")]
    pub(crate) mapping: ExternalMapping,
    #[cfg(feature = "cuda")]
    pub(crate) stream: std::sync::Arc<cudarc::driver::CudaStream>,
    #[cfg(feature = "cuda")]
    pub(crate) function: cudarc::driver::CudaFunction,
    #[cfg(feature = "cuda")]
    pub(crate) count: u32,
    #[cfg(feature = "cuda")]
    pub(crate) _reservation: Option<crate::budget::Reservation>,
}
impl CudaCompute {
    /// Imports an opaque FD on Unix or an opaque Win32 memory handle on Windows.
    /// The File is consumed, including on failure; duplicate a borrowed export.
    /// This does not accept D3D12 resource handles or CUDA IPC handles.
    /// # Safety
    /// The handle must identify a CUDA-compatible, non-dedicated opaque export on
    /// the same physical GPU as this context, with the exact allocation size.
    /// Offset and range must satisfy the exporter's/CUDA's mapping constraints.
    /// The graphics allocation must stay alive, exclusively owned by CUDA, until
    /// this buffer is dropped. Prior graphics use must be complete before import;
    /// external API references must not read/write it while CUDA owns it. The
    /// caller must perform the graphics API's ownership/state transition before
    /// reusing it after successful `release`. Dropping the import does not perform
    /// that transition or report cleanup failure; use `release` before handoff.
    /// # Errors
    /// Rejects empty, overflowing, misaligned or over-budget ranges before driver
    /// access; reports import, mapping, module or driver failures.
    /// Mapping failure conservatively retains the allocation budget because the
    /// driver wrapper may have obtained a pointer without returning its owner.
    /// A failed import does not authorize graphics reuse or an ownership handoff.
    #[cfg(any(unix, windows))]
    #[allow(unsafe_code)]
    pub unsafe fn import_external_u32(
        &self,
        file: std::fs::File,
        allocation_bytes: usize,
        offset_bytes: usize,
        count: usize,
    ) -> Result<CudaExternalU32Buffer, CudaError> {
        let (range, count) =
            validated_range(allocation_bytes, offset_bytes, count, self.max_bytes)?;
        #[cfg(feature = "cuda")]
        {
            let reservation = self.allocation_budget.reserve(allocation_bytes)?;
            let module = self
                .context
                .load_module(cudarc::nvrtc::Ptx::from_src(include_str!("affine.ptx")))
                .map_err(CudaError::Driver)?;
            let function = module
                .load_function("affine_u32")
                .map_err(CudaError::Driver)?;
            // SAFETY: Caller guarantees export type, size, GPU identity and exclusive
            // graphics ownership. cudarc consumes the OS handle according to CUDA's ABI.
            let memory = unsafe {
                self.context
                    .import_external_memory(file, allocation_bytes as u64)
            }
            .map_err(CudaError::Driver)?;
            let mapping = match memory.map_range(range) {
                Ok(mapping) => mapping,
                Err(error) => {
                    // cudarc consumes the import during mapping. An event creation
                    // failure can occur after obtaining the device pointer, so no
                    // returned owner proves that mapped storage was freed.
                    std::mem::forget(reservation);
                    return Err(CudaError::Driver(error));
                }
            };
            Ok(CudaExternalU32Buffer {
                mapping: ExternalMapping::Opaque(mapping),
                stream: self.context.default_stream(),
                function,
                count,
                _reservation: Some(reservation),
            })
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = (file, range, count);
            Err(CudaError::Disabled)
        }
    }
}
/// Internal mapping preserves the CUDA handle type selected by the exporter.
#[cfg(feature = "cuda")]
#[derive(Debug)]
pub(crate) enum ExternalMapping {
    Opaque(cudarc::driver::MappedBuffer),
    #[cfg(windows)]
    D3d12(super::d3d12::ResourceMapping),
}
#[cfg(feature = "cuda")]
impl cudarc::driver::DeviceSlice<u8> for ExternalMapping {
    fn len(&self) -> usize {
        match self {
            Self::Opaque(mapping) => mapping.len(),
            #[cfg(windows)]
            Self::D3d12(mapping) => mapping.len(),
        }
    }
    fn stream(&self) -> &std::sync::Arc<cudarc::driver::CudaStream> {
        match self {
            Self::Opaque(mapping) => mapping.stream(),
            #[cfg(windows)]
            Self::D3d12(mapping) => mapping.stream(),
        }
    }
}
#[cfg(feature = "cuda")]
impl cudarc::driver::DevicePtr<u8> for ExternalMapping {
    fn device_ptr<'a>(
        &'a self,
        stream: &'a cudarc::driver::CudaStream,
    ) -> (
        cudarc::driver::sys::CUdeviceptr,
        cudarc::driver::SyncOnDrop<'a>,
    ) {
        match self {
            Self::Opaque(mapping) => mapping.device_ptr(stream),
            #[cfg(windows)]
            Self::D3d12(mapping) => mapping.device_ptr(stream),
        }
    }
}
impl CudaExternalU32Buffer {
    /// Completes CUDA work and checks imported-memory destruction before handoff.
    /// On failure, the caller must not return the allocation to graphics use.
    /// Failed completion retains the mapping and its budget reservation.
    /// # Errors
    /// Reports synchronization/destruction failures or disabled CUDA support.
    pub fn release(self) -> Result<(), CudaError> {
        #[cfg(feature = "cuda")]
        {
            let Self {
                mapping,
                stream,
                function,
                _reservation: reservation,
                ..
            } = self;
            let context = std::sync::Arc::clone(stream.context());
            if let Err(error) = context
                .bind_to_thread()
                .and_then(|()| context.synchronize())
            {
                std::mem::forget(mapping);
                std::mem::forget(function);
                if let Some(reservation) = reservation {
                    std::mem::forget(reservation);
                }
                return Err(CudaError::Driver(error));
            }
            drop((mapping, function));
            if let Err(error) = context.synchronize().and_then(|()| context.check_err()) {
                if let Some(reservation) = reservation {
                    std::mem::forget(reservation);
                }
                return Err(CudaError::Driver(error));
            }
            drop(reservation);
            Ok(())
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = self;
            Err(CudaError::Disabled)
        }
    }
    /// Copies this mapped range to host u32 values after CUDA completion.
    /// Intended for diagnostics/acceptance, not a zero-copy rendering handoff.
    /// # Errors
    /// Reports transfer/synchronization failures or a disabled CUDA build.
    pub fn read(&self) -> Result<Vec<u32>, CudaError> {
        #[cfg(feature = "cuda")]
        {
            let bytes = self
                .stream
                .clone_dtoh(&self.mapping)
                .map_err(CudaError::Driver)?;
            self.stream.synchronize().map_err(CudaError::Driver)?;
            self.stream
                .context()
                .check_err()
                .map_err(CudaError::Driver)?;
            decode_bytes(&bytes, self.count)
        }
        #[cfg(not(feature = "cuda"))]
        {
            Err(CudaError::Disabled)
        }
    }
    /// Applies wrapping u32 affine writes and waits for CUDA completion before return.
    /// No host data transfer is performed. The caller still owns graphics handoff.
    /// # Errors
    /// Reports launch/synchronization failures or a disabled CUDA build.
    #[allow(unsafe_code)]
    pub fn affine(&mut self, multiplier: u32, bias: u32) -> Result<(), CudaError> {
        #[cfg(feature = "cuda")]
        {
            use cudarc::driver::{DevicePtr, LaunchConfig, PushKernelArg};
            let (pointer, usage) = self.mapping.device_ptr(&self.stream);
            let mut launch = self.stream.launch_builder(&self.function);
            launch
                .arg(&pointer)
                .arg(&self.count)
                .arg(&multiplier)
                .arg(&bias);
            // SAFETY: Imported range is 4-byte aligned and count bounds every u32
            // access in the fixed PTX. Import contract gives exclusive ownership.
            // The usage guard retains mapping stream tracking even on launch failure;
            // successful writes complete before return or graphics handoff.
            let result = unsafe {
                launch.launch(LaunchConfig {
                    grid_dim: (self.count.div_ceil(256), 1, 1),
                    block_dim: (256, 1, 1),
                    shared_mem_bytes: 0,
                })
            }
            .map_err(CudaError::Driver);
            drop(usage);
            result?;
            self.stream
                .synchronize()
                .and_then(|()| self.stream.context().check_err())
                .map_err(CudaError::Driver)
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = (multiplier, bias);
            Err(CudaError::Disabled)
        }
    }
}
#[cfg(any(feature = "cuda", test))]
fn decode_bytes(bytes: &[u8], count: u32) -> Result<Vec<u32>, CudaError> {
    let expected = usize::try_from(count)
        .map_err(|_| CudaError::BufferLimit)?
        .checked_mul(4)
        .ok_or(CudaError::BufferLimit)?;
    if bytes.len() != expected || expected == 0 {
        return Err(CudaError::BufferLimit);
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}
#[cfg(any(unix, windows, test))]
pub(crate) fn validated_range(
    allocation: usize,
    offset: usize,
    count: usize,
    budget: usize,
) -> Result<(std::ops::Range<usize>, u32), CudaError> {
    let bytes = count.checked_mul(4).ok_or(CudaError::BufferLimit)?;
    let end = offset.checked_add(bytes).ok_or(CudaError::BufferLimit)?;
    let count = u32::try_from(count).map_err(|_| CudaError::BufferLimit)?;
    if allocation == 0
        || allocation > budget
        || bytes == 0
        || end > allocation
        || !offset.is_multiple_of(4)
    {
        return Err(CudaError::BufferLimit);
    }
    Ok((offset..end, count))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_readback_preserves_bits_and_rejects_wrong_lengths() {
        let values = [0_u32, 1, 0x1234_5678, u32::MAX];
        let bytes: Vec<_> = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        assert_eq!(decode_bytes(&bytes, 4).unwrap(), values);
        assert!(matches!(
            decode_bytes(&bytes, 3),
            Err(CudaError::BufferLimit)
        ));
        assert!(matches!(
            decode_bytes(&bytes[..15], 4),
            Err(CudaError::BufferLimit)
        ));
        assert!(matches!(decode_bytes(&[], 0), Err(CudaError::BufferLimit)));
    }
    #[test]
    fn export_range_bounds_precede_driver_access() {
        assert_eq!(validated_range(64, 16, 8, 64).unwrap(), (16..48, 8));
        for (allocation, offset, count, budget) in [
            (0, 0, 1, 64),
            (64, 1, 1, 64),
            (64, 60, 2, 64),
            (64, 0, 0, 64),
            (64, 0, 1, 63),
            (64, usize::MAX, 1, 64),
            (64, 0, usize::MAX, 64),
        ] {
            assert!(matches!(
                validated_range(allocation, offset, count, budget),
                Err(CudaError::BufferLimit)
            ));
        }
    }
}
