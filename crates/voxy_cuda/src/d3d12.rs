//! Typed import for a shared D3D12 committed buffer resource.
#![allow(unsafe_code)]

use crate::{CudaCompute, CudaError, CudaExternalU32Buffer};

impl CudaCompute {
    /// Queries the Windows adapter LUID and node mask of this CUDA device.
    /// Use both values to match a D3D12 device before importing its memory.
    /// # Errors
    /// Reports driver query failures or a disabled CUDA build.
    pub fn windows_adapter_identity(&self) -> Result<([u8; 8], u32), CudaError> {
        #[cfg(feature = "cuda")]
        {
            let mut luid = [0_i8; 8];
            let mut mask = 0;
            // SAFETY: CUDA writes exactly eight bytes and one node-mask word.
            unsafe {
                sys::cuDeviceGetLuid(luid.as_mut_ptr(), &raw mut mask, self.context.cu_device())
            }
            .result()
            .map_err(CudaError::Driver)?;
            Ok((luid.map(|byte| byte.to_ne_bytes()[0]), mask))
        }
        #[cfg(not(feature = "cuda"))]
        {
            Err(CudaError::Disabled)
        }
    }

    /// Imports an NT handle exported from a shared D3D12 committed buffer.
    /// Consumes the handle, including on failure. Heap and opaque Win32 handles
    /// must not be passed here. CUDA's required dedicated flag is set internally.
    ///
    /// # Safety
    /// The handle must refer to a `D3D12_HEAP_FLAG_SHARED` committed buffer on the
    /// same physical GPU as this context, on a non-linked-node D3D12 device.
    /// `allocation_bytes` must equal its resource allocation size, and the mapped
    /// range must match the buffer and CUDA alignment requirements. The resource
    /// must remain alive and exclusively owned by CUDA until this mapping drops.
    /// Complete all prior D3D12 queue work before import. After CUDA completion and
    /// successful checked `release`, perform the required D3D12 state transition
    /// before reuse. A failed release does not authorize graphics reacquisition.
    ///
    /// # Errors
    /// Rejects invalid ranges before accessing the driver; reports CUDA failures
    /// or a disabled CUDA build.
    pub unsafe fn import_d3d12_resource_u32(
        &self,
        file: std::fs::File,
        allocation_bytes: usize,
        offset_bytes: usize,
        count: usize,
    ) -> Result<CudaExternalU32Buffer, CudaError> {
        let (range, count) = crate::external::validated_range(
            allocation_bytes,
            offset_bytes,
            count,
            self.max_bytes,
        )?;
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
            // SAFETY: The caller establishes the resource identity and ownership.
            let mapping = unsafe {
                ResourceMapping::new(
                    self.context.clone(),
                    file,
                    allocation_bytes,
                    range,
                    reservation,
                )
            }
            .map_err(CudaError::Driver)?;
            Ok(CudaExternalU32Buffer {
                mapping: crate::external::ExternalMapping::D3d12(mapping),
                stream: self.context.default_stream(),
                function,
                count,
                _reservation: None,
            })
        }
        #[cfg(not(feature = "cuda"))]
        {
            let _ = (file, range, count);
            Err(CudaError::Disabled)
        }
    }
}

#[cfg(feature = "cuda")]
use cudarc::driver::{
    CudaContext, CudaStream, DevicePtr, DeviceSlice, DriverError, SyncOnDrop, result, sys,
};
#[cfg(feature = "cuda")]
use std::{os::windows::io::AsRawHandle, sync::Arc};

/// Owns both CUDA objects and the NT handle. The mapped pointer is freed before
/// destroying imported memory; the retained NT handle closes last.
#[cfg(feature = "cuda")]
#[derive(Debug)]
pub(crate) struct ResourceMapping {
    context: Arc<CudaContext>,
    stream: Arc<CudaStream>,
    memory: sys::CUexternalMemory,
    pointer: Option<sys::CUdeviceptr>,
    len: usize,
    _file: std::fs::File,
    reservation: Option<crate::budget::Reservation>,
}

#[cfg(feature = "cuda")]
impl ResourceMapping {
    fn retain_reservation(&mut self) {
        // Cleanup failure deliberately retains CUDA's allocation reference;
        // preserve its budget charge for the same lifetime.
        if let Some(reservation) = self.reservation.take() {
            std::mem::forget(reservation);
        }
    }
    unsafe fn new(
        context: Arc<CudaContext>,
        file: std::fs::File,
        size: usize,
        range: std::ops::Range<usize>,
        reservation: crate::budget::Reservation,
    ) -> Result<Self, DriverError> {
        context.bind_to_thread()?;
        let descriptor = resource_descriptor(file.as_raw_handle(), size as u64);
        let mut memory = std::ptr::null_mut();
        // SAFETY: Valid NT resource handle and exact size are caller requirements.
        unsafe { sys::cuImportExternalMemory(&raw mut memory, &raw const descriptor) }.result()?;
        let mut mapping = Self {
            stream: context.default_stream(),
            context,
            memory,
            pointer: None,
            len: range.len(),
            _file: file,
            reservation: Some(reservation),
        };
        // SAFETY: The validated range belongs to this exclusively owned resource.
        mapping.pointer = Some(unsafe {
            result::external_memory::get_mapped_buffer(
                memory,
                range.start as u64,
                range.len() as u64,
            )
        }?);
        Ok(mapping)
    }
}

#[cfg(feature = "cuda")]
fn resource_descriptor(
    handle: std::os::windows::io::RawHandle,
    size: u64,
) -> sys::CUDA_EXTERNAL_MEMORY_HANDLE_DESC {
    sys::CUDA_EXTERNAL_MEMORY_HANDLE_DESC {
        type_: sys::CUexternalMemoryHandleType::CU_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE,
        handle: sys::CUDA_EXTERNAL_MEMORY_HANDLE_DESC_st__bindgen_ty_1 {
            win32: sys::CUDA_EXTERNAL_MEMORY_HANDLE_DESC_st__bindgen_ty_1__bindgen_ty_1 {
                handle,
                name: std::ptr::null(),
            },
        },
        size,
        flags: sys::CUDA_EXTERNAL_MEMORY_DEDICATED,
        reserved: [0; 16],
    }
}

#[cfg(feature = "cuda")]
impl Drop for ResourceMapping {
    fn drop(&mut self) {
        // A context-wide drain covers kernels even on failed launch/error paths.
        // If completion cannot be established, retain CUDA's resource reference
        // rather than free a pointer potentially still used by a device stream.
        if let Err(error) = self
            .context
            .bind_to_thread()
            .and_then(|()| self.context.synchronize())
        {
            self.context.record_err::<()>(Err(error));
            self.retain_reservation();
            return;
        }
        if let Some(pointer) = self.pointer.take() {
            // SAFETY: All CUDA work completed and this is the sole mapping owner.
            if let Err(error) = unsafe { result::memory_free(pointer) } {
                self.context.record_err::<()>(Err(error));
                self.retain_reservation();
                return;
            }
        }
        // SAFETY: The mapping was freed first; CUDA does not own the NT handle.
        if let Err(error) = unsafe { result::external_memory::destroy_external_memory(self.memory) }
        {
            self.context.record_err::<()>(Err(error));
            self.retain_reservation();
        }
    }
}

#[cfg(feature = "cuda")]
impl DeviceSlice<u8> for ResourceMapping {
    fn len(&self) -> usize {
        self.len
    }
    fn stream(&self) -> &Arc<CudaStream> {
        &self.stream
    }
}
#[cfg(feature = "cuda")]
impl DevicePtr<u8> for ResourceMapping {
    fn device_ptr<'a>(&'a self, stream: &'a CudaStream) -> (sys::CUdeviceptr, SyncOnDrop<'a>) {
        (
            self.pointer.expect("live mapping"),
            SyncOnDrop::sync_stream(stream),
        )
    }
}

#[cfg(all(test, feature = "cuda"))]
mod tests {
    use super::*;
    #[test]
    fn committed_resource_descriptor_is_dedicated_and_not_opaque() {
        let descriptor = resource_descriptor(std::ptr::null_mut(), 65536);
        assert_eq!(
            descriptor.type_,
            sys::CUexternalMemoryHandleType::CU_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE
        );
        assert_eq!(descriptor.flags, sys::CUDA_EXTERNAL_MEMORY_DEDICATED);
        assert_eq!(descriptor.size, 65536);
        assert_eq!(descriptor.reserved, [0; 16]);
    }
}
