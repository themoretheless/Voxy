//! Native Vulkan storage exported for an exclusively owned external compute phase.
#![allow(unsafe_code)]
use ash::vk;
use std::fs::File;
#[cfg(target_os = "linux")]
use std::os::fd::FromRawFd;
#[cfg(target_os = "windows")]
use std::os::windows::io::FromRawHandle;
#[cfg(target_os = "linux")]
type Exporter = ash::khr::external_memory_fd::Device;
#[cfg(target_os = "windows")]
type Exporter = ash::khr::external_memory_win32::Device;
#[cfg(target_os = "linux")]
const HANDLE_KIND: vk::ExternalMemoryHandleTypeFlags = vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;
#[cfg(target_os = "windows")]
const HANDLE_KIND: vk::ExternalMemoryHandleTypeFlags =
    vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32;
/// Required wgpu feature for this platform's opaque external-memory export.
#[cfg(target_os = "linux")]
pub const EXTERNAL_MEMORY_FEATURE: wgpu::Features = wgpu::Features::VULKAN_EXTERNAL_MEMORY_FD;
#[cfg(target_os = "windows")]
pub const EXTERNAL_MEMORY_FEATURE: wgpu::Features = wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32;
type Error = Box<dyn std::error::Error + Send + Sync>;

/// Initialized storage on the wgpu device. wgpu owns buffer/memory destruction.
pub struct VulkanExportBuffer {
    buffer: wgpu::Buffer,
    _device: wgpu::Device,
    raw: ash::Device,
    handle: vk::Buffer,
    memory: vk::DeviceMemory,
    allocation_bytes: u64,
    family: u32,
    queue: vk::Queue,
    exporter: Exporter,
    external: bool,
}
impl std::fmt::Debug for VulkanExportBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VulkanExportBuffer")
            .field("buffer", &self.buffer)
            .field("allocation_bytes", &self.allocation_bytes)
            .field("external", &self.external)
            .finish_non_exhaustive()
    }
}
struct Allocation {
    raw: ash::Device,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
}
impl Drop for Allocation {
    fn drop(&mut self) {
        unsafe {
            if self.buffer != vk::Buffer::null() {
                self.raw.destroy_buffer(self.buffer, None);
            }
            if self.memory != vk::DeviceMemory::null() {
                self.raw.free_memory(self.memory, None);
            }
        }
    }
}
struct Commands {
    raw: ash::Device,
    pool: vk::CommandPool,
    queue: vk::Queue,
}
impl Drop for Commands {
    fn drop(&mut self) {
        unsafe {
            let _ = self.raw.queue_wait_idle(self.queue);
            self.raw.destroy_command_pool(self.pool, None);
        }
    }
}
impl VulkanExportBuffer {
    /// # Safety
    /// Caller must exclude concurrent host access/submission to this device's
    /// Vulkan queue during this operation. Requires native Vulkan and explicit
    /// the platform's `EXTERNAL_MEMORY_FEATURE`. No CUDA or CPU fallback is selected.
    /// # Errors
    /// Reports unsupported export/device/memory, invalid size and Vulkan failures.
    pub unsafe fn new(device: &wgpu::Device, bytes: u64) -> Result<Self, Error> {
        if bytes == 0 || !bytes.is_multiple_of(4) || bytes > device.limits().max_buffer_size {
            return Err("invalid external storage size".into());
        }
        if !device.features().contains(EXTERNAL_MEMORY_FEATURE) {
            return Err("platform external-memory feature must be enabled".into());
        }
        let hal =
            unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }.ok_or("not a Vulkan device")?;
        let raw = hal.raw_device().clone();
        let instance = hal.shared_instance().raw_instance();
        let (mut allocation, allocation_bytes) = unsafe { allocate(&hal, bytes)? };
        let buffer = allocation.buffer;
        let queue = hal.raw_queue();
        let family = hal.queue_family_index();
        unsafe {
            commands(&raw, queue, family, |command| {
                raw.cmd_fill_buffer(command, buffer, 0, bytes, 0);
                let barrier = vk::BufferMemoryBarrier::default()
                    .buffer(buffer)
                    .offset(0)
                    .size(bytes)
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED);
                raw.cmd_pipeline_barrier(
                    command,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::ALL_COMMANDS,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[barrier],
                    &[],
                );
            })?;
        }
        let memory = allocation.memory;
        let handle = buffer;
        let exporter = Exporter::new(instance, &raw);
        // SAFETY: Same device, initialized storage, exact descriptor and owned allocation.
        let imported = unsafe {
            wgpu::hal::vulkan::Buffer::from_raw_managed(buffer, memory, 0, allocation_bytes)
        };
        allocation.buffer = vk::Buffer::null();
        allocation.memory = vk::DeviceMemory::null();
        drop(hal);
        let buffer = unsafe {
            device.create_buffer_from_hal::<wgpu::hal::api::Vulkan>(
                imported,
                &wgpu::BufferDescriptor {
                    label: Some("exportable external-compute storage"),
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
            raw,
            handle,
            memory,
            allocation_bytes,
            family,
            queue,
            exporter,
            external: false,
        })
    }
    /// Buffer may be used by wgpu only in the graphics-owned phase.
    /// # Errors
    /// Rejects access while external compute owns the allocation.
    pub fn buffer(&self) -> Result<&wgpu::Buffer, Error> {
        if self.external {
            return Err("storage is externally owned".into());
        }
        Ok(&self.buffer)
    }
    #[must_use]
    pub fn allocation_bytes(&self) -> u64 {
        self.allocation_bytes
    }
    /// # Safety
    /// Flush all pending wgpu writes (submit), exclude concurrent queue host access,
    /// and do not use any retained buffer/bind-group clones until acquire returns.
    /// Exported allocation must remain alive through import/mapping destruction.
    /// # Errors
    /// Reports wrong ownership, Vulkan synchronization or export errors.
    pub unsafe fn release(&mut self) -> Result<File, Error> {
        if self.external {
            return Err("already externally owned".into());
        }
        unsafe {
            self.transition(true)?;
        }
        self.external = true;
        #[cfg(target_os = "linux")]
        {
            let fd = unsafe {
                self.exporter.get_memory_fd(
                    &vk::MemoryGetFdInfoKHR::default()
                        .memory(self.memory)
                        .handle_type(HANDLE_KIND),
                )?
            };
            Ok(unsafe { File::from_raw_fd(fd) })
        }
        #[cfg(target_os = "windows")]
        {
            let handle = unsafe {
                self.exporter.get_memory_win32_handle(
                    &vk::MemoryGetWin32HandleInfoKHR::default()
                        .memory(self.memory)
                        .handle_type(HANDLE_KIND),
                )?
            };
            if handle == 0 || handle == -1 {
                return Err("invalid Vulkan Win32 export handle".into());
            }
            // SAFETY: Vulkan returned an owned NT handle. File closes it exactly
            // once; the CUDA importer retains it for the full Win32 import lifetime.
            Ok(unsafe { File::from_raw_handle(handle as std::os::windows::io::RawHandle) })
        }
    }
    /// # Safety
    /// External writes must have completed and all CUDA imports/mappings must
    /// have been dropped. Exclude concurrent host access to the Vulkan queue.
    /// # Errors
    /// Reports wrong ownership or Vulkan synchronization errors.
    pub unsafe fn acquire(&mut self) -> Result<(), Error> {
        if !self.external {
            return Err("already graphics owned".into());
        }
        unsafe {
            self.transition(false)?;
        }
        self.external = false;
        Ok(())
    }
    unsafe fn transition(&self, release: bool) -> Result<(), Error> {
        let access = vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE;
        let (src, dst, src_access, dst_access, src_stage, dst_stage) = if release {
            (
                self.family,
                vk::QUEUE_FAMILY_EXTERNAL,
                access,
                vk::AccessFlags::empty(),
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            )
        } else {
            (
                vk::QUEUE_FAMILY_EXTERNAL,
                self.family,
                vk::AccessFlags::empty(),
                access,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::ALL_COMMANDS,
            )
        };
        unsafe {
            commands(&self.raw, self.queue, self.family, |command| {
                let barrier = vk::BufferMemoryBarrier::default()
                    .buffer(self.handle)
                    .offset(0)
                    .size(self.buffer.size())
                    .src_access_mask(src_access)
                    .dst_access_mask(dst_access)
                    .src_queue_family_index(src)
                    .dst_queue_family_index(dst);
                self.raw.cmd_pipeline_barrier(
                    command,
                    src_stage,
                    dst_stage,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[barrier],
                    &[],
                );
            })
        }
    }
}
unsafe fn commands(
    raw: &ash::Device,
    queue: vk::Queue,
    family: u32,
    record: impl FnOnce(vk::CommandBuffer),
) -> Result<(), Error> {
    unsafe {
        raw.queue_wait_idle(queue)?;
    }
    let pool = unsafe {
        raw.create_command_pool(
            &vk::CommandPoolCreateInfo::default().queue_family_index(family),
            None,
        )?
    };
    let guard = Commands {
        raw: raw.clone(),
        pool,
        queue,
    };
    let command = unsafe {
        raw.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )?[0]
    };
    unsafe {
        raw.begin_command_buffer(
            command,
            &vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )?;
    }
    record(command);
    unsafe {
        raw.end_command_buffer(command)?;
        raw.queue_submit(
            queue,
            &[vk::SubmitInfo::default().command_buffers(&[command])],
            vk::Fence::null(),
        )?;
        raw.queue_wait_idle(queue)?;
    }
    drop(guard);
    Ok(())
}

unsafe fn allocate(
    hal: &wgpu::hal::vulkan::Device,
    bytes: u64,
) -> Result<(Allocation, u64), Error> {
    let raw = hal.raw_device();
    let instance = hal.shared_instance().raw_instance();
    let usage = vk::BufferUsageFlags::STORAGE_BUFFER
        | vk::BufferUsageFlags::TRANSFER_SRC
        | vk::BufferUsageFlags::TRANSFER_DST;
    let info = vk::PhysicalDeviceExternalBufferInfo::default()
        .usage(usage)
        .handle_type(HANDLE_KIND);
    let mut properties = vk::ExternalBufferProperties::default();
    unsafe {
        instance.get_physical_device_external_buffer_properties(
            hal.raw_physical_device(),
            &info,
            &mut properties,
        );
    }
    let features = properties
        .external_memory_properties
        .external_memory_features;
    if !features.contains(vk::ExternalMemoryFeatureFlags::EXPORTABLE)
        || features.contains(vk::ExternalMemoryFeatureFlags::DEDICATED_ONLY)
    {
        return Err("non-dedicated opaque platform storage export unavailable".into());
    }
    let mut external = vk::ExternalMemoryBufferCreateInfo::default().handle_types(HANDLE_KIND);
    let descriptor = vk::BufferCreateInfo::default()
        .size(bytes)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .push_next(&mut external);
    let buffer = unsafe { raw.create_buffer(&descriptor, None)? };
    let mut allocation = Allocation {
        raw: raw.clone(),
        buffer,
        memory: vk::DeviceMemory::null(),
    };
    let mut dedicated = vk::MemoryDedicatedRequirements::default();
    let mut queried = vk::MemoryRequirements2::default().push_next(&mut dedicated);
    unsafe {
        raw.get_buffer_memory_requirements2(
            &vk::BufferMemoryRequirementsInfo2::default().buffer(buffer),
            &mut queried,
        );
    }
    let requirements = queried.memory_requirements;
    if dedicated.requires_dedicated_allocation != 0 {
        return Err("dedicated Vulkan allocation is not supported by this export".into());
    }
    let allocation_bytes = requirements.size;
    let memory =
        unsafe { instance.get_physical_device_memory_properties(hal.raw_physical_device()) };
    let kind = (0..memory.memory_type_count)
        .find(|index| {
            requirements.memory_type_bits & (1 << index) != 0
                && memory.memory_types[*index as usize]
                    .property_flags
                    .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
        })
        .ok_or("no device-local export memory")?;
    let mut export = vk::ExportMemoryAllocateInfo::default().handle_types(HANDLE_KIND);
    allocation.memory = unsafe {
        raw.allocate_memory(
            &vk::MemoryAllocateInfo::default()
                .allocation_size(allocation_bytes)
                .memory_type_index(kind)
                .push_next(&mut export),
            None,
        )?
    };
    unsafe {
        raw.bind_buffer_memory(buffer, allocation.memory, 0)?;
    }
    Ok((allocation, allocation_bytes))
}

/// Physical Vulkan device identity for matching a CUDA device before allocation.
/// # Errors
/// Rejects a non-Vulkan adapter or an absent/all-zero device UUID.
pub fn adapter_uuid(adapter: &wgpu::Adapter) -> Result<[u8; 16], Error> {
    // SAFETY: Read-only physical-device query; the HAL guard retains its instance.
    let hal =
        unsafe { adapter.as_hal::<wgpu::hal::api::Vulkan>() }.ok_or("not a Vulkan adapter")?;
    let mut identity = vk::PhysicalDeviceIDProperties::default();
    let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut identity);
    unsafe {
        hal.shared_instance()
            .raw_instance()
            .get_physical_device_properties2(hal.raw_physical_device(), &mut properties);
    }
    if identity.device_uuid == [0; 16] {
        return Err("Vulkan device UUID unavailable".into());
    }
    Ok(identity.device_uuid)
}

/// Identity of the actual logical device's physical Vulkan GPU.
/// # Errors
/// Rejects a non-Vulkan device or an unavailable UUID.
pub fn device_uuid(device: &wgpu::Device) -> Result<[u8; 16], Error> {
    // SAFETY: Read-only physical-device query while the HAL guard retains device/instance.
    let hal = unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }.ok_or("not a Vulkan device")?;
    let mut identity = vk::PhysicalDeviceIDProperties::default();
    let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut identity);
    unsafe {
        hal.shared_instance()
            .raw_instance()
            .get_physical_device_properties2(hal.raw_physical_device(), &mut properties);
    }
    if identity.device_uuid == [0; 16] {
        return Err("Vulkan device UUID unavailable".into());
    }
    Ok(identity.device_uuid)
}
