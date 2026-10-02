//! Shared D3D12 committed buffers imported into wgpu without a CPU copy.
#![allow(unsafe_code)]

use std::{fs::File, os::windows::io::FromRawHandle};
use windows::Win32::Graphics::{Direct3D12 as dx, Dxgi::Common as dxgi};
use windows::core::Interface;

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Reads the Windows adapter identity without creating a D3D12 device.
/// # Errors
/// Rejects non-DX12 adapters and native descriptor query failures.
pub fn adapter_luid(adapter: &wgpu::Adapter) -> Result<[u8; 8], Error> {
    // SAFETY: Read-only descriptor query under wgpu's retained native guard.
    let hal = unsafe { adapter.as_hal::<wgpu::hal::api::Dx12>() }.ok_or("adapter is not DX12")?;
    let description = unsafe { hal.as_raw().GetDesc1() }?;
    let mut bytes = [0; 8];
    bytes[..4].copy_from_slice(&description.AdapterLuid.LowPart.to_le_bytes());
    bytes[4..].copy_from_slice(&description.AdapterLuid.HighPart.to_le_bytes());
    Ok(bytes)
}

/// Owns the shared committed resource and its wgpu wrapper. Exported handles
/// retain the resource independently, but must obey the documented GPU lifetime.
#[derive(Debug)]
pub struct D3d12ExportBuffer {
    buffer: wgpu::Buffer,
    _device: wgpu::Device,
    raw_device: dx::ID3D12Device,
    resource: dx::ID3D12Resource,
    allocation_bytes: u64,
}

impl D3d12ExportBuffer {
    /// Returns the physical Windows adapter identifier for CUDA matching.
    #[must_use]
    pub fn adapter_luid(&self) -> [u8; 8] {
        // SAFETY: Read-only query on the retained native device.
        let luid = unsafe { self.raw_device.GetAdapterLuid() };
        let mut bytes = [0; 8];
        bytes[..4].copy_from_slice(&luid.LowPart.to_le_bytes());
        bytes[4..].copy_from_slice(&luid.HighPart.to_le_bytes());
        bytes
    }

    /// Rejects a different CUDA GPU or a linked-node identity before import.
    /// # Errors
    /// Reports query failure, missing identity or incompatible physical device.
    #[cfg(feature = "cuda")]
    pub fn validate_cuda_device(&self, compute: &voxy_cuda::CudaCompute) -> Result<(), Error> {
        let (cuda_luid, mask) = compute.windows_adapter_identity()?;
        let graphics_luid = self.adapter_luid();
        if !matching_identity(graphics_luid, cuda_luid, mask) {
            return Err("CUDA and D3D12 adapter identity or node mask mismatch".into());
        }
        Ok(())
    }

    /// Imports a range into CUDA after checking physical adapter and node identity.
    /// Uses this resource's exact allocation size and dedicated D3D12 handle type.
    /// # Safety
    /// Complete prior graphics queue work and exclude all graphics access,
    /// including retained bind groups, until CUDA completion and mapping drop.
    /// Keep this owner alive and satisfy CUDA's offset/range alignment rules.
    /// Restore the required D3D12 state before graphics reuse; this method does
    /// not perform a queue or resource-state handoff.
    /// # Errors
    /// Reports device mismatch, invalid range/budget, handle or CUDA import failure.
    #[cfg(feature = "cuda")]
    pub unsafe fn import_cuda(
        &self,
        compute: &voxy_cuda::CudaCompute,
        offset_bytes: usize,
        count: usize,
    ) -> Result<voxy_cuda::CudaExternalU32Buffer, Error> {
        self.validate_cuda_device(compute)?;
        let allocation = usize::try_from(self.allocation_bytes)?;
        let logical = usize::try_from(self.buffer.size())?;
        let end = count
            .checked_mul(4)
            .and_then(|bytes| offset_bytes.checked_add(bytes));
        if end.is_none_or(|end| end > logical) {
            return Err("CUDA range exceeds logical D3D12 buffer".into());
        }
        let file = unsafe { self.export_handle() }?;
        // SAFETY: Resource type/size/identity are checked here; ownership,
        // completion and alignment beyond u32 requirements are caller contracts.
        Ok(unsafe { compute.import_d3d12_resource_u32(file, allocation, offset_bytes, count) }?)
    }

    /// Creates and zero-initializes a shared committed storage buffer on DX12.
    /// Does not choose another backend if the device is not DX12.
    ///
    /// # Safety
    /// `queue` must belong to `device`. Exclude concurrent queue submissions and
    /// native device operations during construction. This function waits for the
    /// initialization submission before exposing the buffer to an external writer.
    ///
    /// # Errors
    /// Rejects invalid sizes, non-DX12 or linked-node devices and reports native
    /// resource creation, allocation or queue completion failures.
    pub unsafe fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: u64,
    ) -> Result<Self, Error> {
        validate_size(bytes, device.limits().max_buffer_size)?;
        // SAFETY: Read-only native device access under the caller's exclusion.
        let hal = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }
            .ok_or("D3D12 export requires a DX12 device")?;
        let raw_device = hal.raw_device().clone();
        if unsafe { raw_device.GetNodeCount() } != 1 {
            return Err("CUDA sharing requires a non-linked-node D3D12 device".into());
        }
        let description = buffer_description(bytes);
        let allocation = unsafe { raw_device.GetResourceAllocationInfo(0, &[description]) };
        if allocation.SizeInBytes == u64::MAX || allocation.SizeInBytes < bytes {
            return Err("invalid D3D12 resource allocation size".into());
        }
        let heap = dx::D3D12_HEAP_PROPERTIES {
            Type: dx::D3D12_HEAP_TYPE_DEFAULT,
            CreationNodeMask: 1,
            VisibleNodeMask: 1,
            ..Default::default()
        };
        let mut resource: Option<dx::ID3D12Resource> = None;
        // SAFETY: Valid buffer descriptor and single-node shared default heap.
        unsafe {
            raw_device.CreateCommittedResource(
                &raw const heap,
                dx::D3D12_HEAP_FLAG_SHARED,
                &raw const description,
                dx::D3D12_RESOURCE_STATE_COMMON,
                None,
                &raw mut resource,
            )
        }?;
        let resource = resource.ok_or("D3D12 returned no committed resource")?;
        drop(hal);
        // Zero a temporary source through wgpu before importing the raw resource.
        // The HAL import requires initialized memory, not just a future clear.
        let zero = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("D3D12 shared initialization source"),
            size: bytes,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.clear_buffer(&zero, 0, None);
        let submission = queue.submit([encoder.finish()]);
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        let source = {
            let source_hal = unsafe { zero.as_hal::<wgpu::hal::api::Dx12>() }
                .ok_or("initialization source is not DX12")?;
            unsafe { source_hal.raw_resource() }.clone()
        };
        let raw_queue = unsafe { queue.as_hal::<wgpu::hal::api::Dx12>() }
            .ok_or("initialization queue is not DX12")?
            .as_raw()
            .clone();
        // Completed buffer submissions decay to COMMON. CopyBufferRegion implicitly
        // promotes both buffers; completion restores COMMON before wgpu adoption.
        let allocator: dx::ID3D12CommandAllocator =
            unsafe { raw_device.CreateCommandAllocator(dx::D3D12_COMMAND_LIST_TYPE_DIRECT) }?;
        let commands: dx::ID3D12GraphicsCommandList = unsafe {
            raw_device.CreateCommandList(0, dx::D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None)
        }?;
        unsafe {
            commands.CopyBufferRegion(&resource, 0, &source, 0, bytes);
            commands.Close()?;
        }
        let list: dx::ID3D12CommandList = commands.cast()?;
        unsafe { raw_queue.ExecuteCommandLists(&[Some(list)]) };
        // An ordered empty wgpu submission places its completion fence after the
        // native copy. Keep allocator, list and source alive until that fence.
        let submission = queue.submit([]);
        if let Err(error) = device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        }) {
            // wgpu cannot retain our native allocator/list or shared resource.
            // An unsuccessful wait is not proof that native GPU work stopped.
            // Conservatively retain every native copy dependency rather than
            // release memory/commands potentially still referenced by the GPU.
            // No graphics owner is returned from this failed constructor.
            std::mem::forget((
                allocator, commands, source, zero, resource, raw_queue, raw_device,
            ));
            return Err(error.into());
        }
        // SAFETY: Same device, descriptor and initialized memory. COM clones
        // retain the resource; HAL owns no suballocation for this raw resource.
        let hal_buffer =
            unsafe { wgpu::hal::dx12::Device::buffer_from_raw(resource.clone(), bytes) };
        let buffer = unsafe {
            device.create_buffer_from_hal::<wgpu::hal::api::Dx12>(
                hal_buffer,
                &wgpu::BufferDescriptor {
                    label: Some("D3D12 shared committed storage"),
                    size: bytes,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                },
            )
        };
        Ok(Self {
            buffer,
            _device: device.clone(),
            raw_device,
            resource,
            allocation_bytes: allocation.SizeInBytes,
        })
    }

    /// The resource allocation size required by CUDA's dedicated import.
    #[must_use]
    pub fn allocation_bytes(&self) -> u64 {
        self.allocation_bytes
    }

    /// Graphics binding; the caller must exclude all external writes/imports
    /// before issuing commands that refer to it, including retained bind groups.
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Creates an owned NT handle for `import_d3d12_resource_u32`.
    /// The handle is a D3D12 resource export, never an opaque Win32 export.
    ///
    /// # Safety
    /// Complete all graphics work before external use. Keep this owner alive and
    /// exclude wgpu/native access while CUDA owns the resource. The caller must
    /// establish CUDA completion, drop its mappings, and restore required D3D12
    /// resource state before graphics reuse. This export does not implement a
    /// semaphore, resource-state or queue ownership handoff.
    ///
    /// # Errors
    /// Reports native handle creation failure or an invalid handle.
    pub unsafe fn export_handle(&self) -> Result<File, Error> {
        // GENERIC_ALL is the documented access value for CreateSharedHandle.
        let handle = unsafe {
            self.raw_device.CreateSharedHandle(
                &self.resource,
                None,
                windows::Win32::Foundation::GENERIC_ALL.0,
                windows::core::PCWSTR::null(),
            )
        }?;
        if handle.is_invalid() {
            return Err("invalid D3D12 shared resource handle".into());
        }
        // SAFETY: CreateSharedHandle returned one owned NT handle; File closes it.
        Ok(unsafe { File::from_raw_handle(handle.0) })
    }
}

fn validate_size(bytes: u64, limit: u64) -> Result<(), Error> {
    if bytes == 0 || !bytes.is_multiple_of(4) || bytes > limit {
        return Err("invalid D3D12 shared buffer size".into());
    }
    Ok(())
}

#[cfg(any(feature = "cuda", test))]
fn matching_identity(graphics: [u8; 8], cuda: [u8; 8], mask: u32) -> bool {
    graphics != [0; 8] && graphics == cuda && mask == 1
}

fn buffer_description(bytes: u64) -> dx::D3D12_RESOURCE_DESC {
    dx::D3D12_RESOURCE_DESC {
        Dimension: dx::D3D12_RESOURCE_DIMENSION_BUFFER,
        Width: bytes,
        Height: 1,
        DepthOrArraySize: 1,
        MipLevels: 1,
        Format: dxgi::DXGI_FORMAT_UNKNOWN,
        SampleDesc: dxgi::DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Layout: dx::D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        Flags: dx::D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_identity_requires_same_nonzero_single_node_adapter() {
        let identity = [1, 2, 3, 4, 5, 6, 7, 8];
        assert!(matching_identity(identity, identity, 1));
        assert!(!matching_identity(identity, [2; 8], 1));
        assert!(!matching_identity([0; 8], [0; 8], 1));
        for mask in [0, 2, 3, u32::MAX] {
            assert!(!matching_identity(identity, identity, mask));
        }
    }
    #[test]
    fn shared_storage_size_and_descriptor() {
        assert!(validate_size(96, 96).is_ok());
        for size in [0, 1, 95, 100, u64::MAX] {
            assert!(validate_size(size, 96).is_err());
        }
        let description = buffer_description(96);
        assert_eq!(description.Width, 96);
        assert_eq!(description.Dimension, dx::D3D12_RESOURCE_DIMENSION_BUFFER);
        assert_eq!(description.Layout, dx::D3D12_TEXTURE_LAYOUT_ROW_MAJOR);
        assert_eq!(
            description.Flags,
            dx::D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS
        );
    }
}
