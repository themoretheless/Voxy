//! Linux Vulkan opaque-FD export acceptance; CUDA handoff remains pending.
#![allow(unsafe_code)]
#[cfg(target_os = "linux")]
mod linux {
    use ash::{Entry, vk};
    use std::{ffi::CStr, fs::File, os::fd::FromRawFd};
    type Error = Box<dyn std::error::Error>;
    struct Export {
        _entry: Entry,
        instance: ash::Instance,
        device: ash::Device,
        physical: vk::PhysicalDevice,
        queue: vk::Queue,
        family: u32,
        buffers: Vec<vk::Buffer>,
        memory: Vec<vk::DeviceMemory>,
        pool: Option<vk::CommandPool>,
    }
    impl Drop for Export {
        fn drop(&mut self) {
            // SAFETY: Owns all Vulkan resources; complete work before destroying them.
            unsafe {
                let _ = self.device.device_wait_idle();
                if let Some(pool) = self.pool {
                    self.device.destroy_command_pool(pool, None);
                }
                for buffer in &self.buffers {
                    self.device.destroy_buffer(*buffer, None);
                }
                for memory in &self.memory {
                    self.device.free_memory(*memory, None);
                }
                self.device.destroy_device(None);
                self.instance.destroy_instance(None);
            }
        }
    }
    impl Export {
        unsafe fn new(required_uuid: Option<[u8; 16]>) -> Result<Self, Error> {
            let entry = unsafe { Entry::load()? };
            let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_1);
            let instance = unsafe {
                entry.create_instance(
                    &vk::InstanceCreateInfo::default().application_info(&app),
                    None,
                )?
            };
            let selected = (|| -> Result<_, Error> {
                for physical in unsafe { instance.enumerate_physical_devices()? } {
                    let properties = unsafe { instance.get_physical_device_properties(physical) };
                    if properties.api_version < vk::API_VERSION_1_1 {
                        continue;
                    }
                    if let Some(required_uuid) = required_uuid
                        && unsafe { device_uuid(&instance, physical) } != required_uuid
                    {
                        continue;
                    }

                    let extensions =
                        unsafe { instance.enumerate_device_extension_properties(physical)? };
                    if !extensions.iter().any(|extension| unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) } == ash::khr::external_memory_fd::NAME) { continue; }
                    let info = vk::PhysicalDeviceExternalBufferInfo::default()
                        .usage(
                            vk::BufferUsageFlags::STORAGE_BUFFER
                                | vk::BufferUsageFlags::TRANSFER_SRC
                                | vk::BufferUsageFlags::TRANSFER_DST,
                        )
                        .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
                    let mut properties = vk::ExternalBufferProperties::default();
                    unsafe {
                        instance.get_physical_device_external_buffer_properties(
                            physical,
                            &info,
                            &mut properties,
                        );
                    }
                    let flags = properties
                        .external_memory_properties
                        .external_memory_features;
                    if !flags.contains(vk::ExternalMemoryFeatureFlags::EXPORTABLE)
                        || flags.contains(vk::ExternalMemoryFeatureFlags::DEDICATED_ONLY)
                    {
                        continue;
                    }
                    let families =
                        unsafe { instance.get_physical_device_queue_family_properties(physical) };
                    if let Some(family) = families.iter().position(|queue| {
                        queue.queue_count > 0
                            && queue.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                    }) {
                        return Ok((physical, u32::try_from(family)?));
                    }
                }
                Err("no Vulkan adapter supports non-dedicated opaque FD buffer export".into())
            })();
            let (physical, family) = match selected {
                Ok(value) => value,
                Err(error) => {
                    unsafe {
                        instance.destroy_instance(None);
                    }
                    return Err(error);
                }
            };
            let queues = [vk::DeviceQueueCreateInfo::default()
                .queue_family_index(family)
                .queue_priorities(&[1.0])];
            let extensions = [ash::khr::external_memory_fd::NAME.as_ptr()];
            let result = unsafe {
                instance.create_device(
                    physical,
                    &vk::DeviceCreateInfo::default()
                        .queue_create_infos(&queues)
                        .enabled_extension_names(&extensions),
                    None,
                )
            };
            let device = match result {
                Ok(device) => device,
                Err(error) => {
                    unsafe {
                        instance.destroy_instance(None);
                    }
                    return Err(error.into());
                }
            };
            let queue = unsafe { device.get_device_queue(family, 0) };
            Ok(Self {
                _entry: entry,
                instance,
                device,
                physical,
                queue,
                family,
                buffers: Vec::new(),
                memory: Vec::new(),
                pool: None,
            })
        }
        unsafe fn allocate(
            &mut self,
            external: bool,
            host: bool,
        ) -> Result<(vk::Buffer, vk::DeviceMemory, u64), Error> {
            let mut export_info = vk::ExternalMemoryBufferCreateInfo::default()
                .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
            let mut buffer_info = vk::BufferCreateInfo::default().size(1024).usage(
                vk::BufferUsageFlags::STORAGE_BUFFER
                    | vk::BufferUsageFlags::TRANSFER_SRC
                    | vk::BufferUsageFlags::TRANSFER_DST,
            );
            if external {
                buffer_info = buffer_info.push_next(&mut export_info);
            }
            let buffer = unsafe { self.device.create_buffer(&buffer_info, None)? };
            self.buffers.push(buffer);
            let requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };
            let properties = unsafe {
                self.instance
                    .get_physical_device_memory_properties(self.physical)
            };
            let flags = if host {
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT
            } else {
                vk::MemoryPropertyFlags::DEVICE_LOCAL
            };
            let index = (0..properties.memory_type_count)
                .find(|index| {
                    requirements.memory_type_bits & (1 << index) != 0
                        && properties.memory_types[*index as usize]
                            .property_flags
                            .contains(flags)
                })
                .ok_or("compatible Vulkan memory type missing")?;
            let mut export = vk::ExportMemoryAllocateInfo::default()
                .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
            let mut info = vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(index);
            if external {
                info = info.push_next(&mut export);
            }
            let memory = unsafe { self.device.allocate_memory(&info, None)? };
            self.memory.push(memory);
            unsafe {
                self.device.bind_buffer_memory(buffer, memory, 0)?;
            }
            Ok((buffer, memory, requirements.size))
        }
        unsafe fn expect_host(&self, memory: vk::DeviceMemory, expected: u32) -> Result<(), Error> {
            unsafe { self.expect_host_words(memory, &vec![expected; 256]) }
        }
        unsafe fn expect_host_words(
            &self,
            memory: vk::DeviceMemory,
            expected: &[u32],
        ) -> Result<(), Error> {
            let pointer = unsafe {
                self.device
                    .map_memory(memory, 0, 1024, vk::MemoryMapFlags::empty())?
            };
            let values = unsafe { std::slice::from_raw_parts(pointer.cast::<u32>(), 256) };
            let matches = values == expected;
            unsafe {
                self.device.unmap_memory(memory);
            }
            if !matches {
                return Err("Vulkan host readback mismatch".into());
            }
            Ok(())
        }
        unsafe fn handoff(
            &self,
            buffer: vk::Buffer,
            staging: Option<vk::Buffer>,
        ) -> Result<(), Error> {
            let pool = self.pool.ok_or("command pool missing")?;
            let command = unsafe {
                self.device.allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::default()
                        .command_pool(pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1),
                )?
            }[0];
            let acquire = staging.is_some();
            let barrier = vk::BufferMemoryBarrier::default()
                .buffer(buffer)
                .offset(0)
                .size(1024)
                .src_queue_family_index(if acquire {
                    vk::QUEUE_FAMILY_EXTERNAL
                } else {
                    self.family
                })
                .dst_queue_family_index(if acquire {
                    self.family
                } else {
                    vk::QUEUE_FAMILY_EXTERNAL
                })
                .src_access_mask(if acquire {
                    vk::AccessFlags::empty()
                } else {
                    vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE
                })
                .dst_access_mask(if acquire {
                    vk::AccessFlags::TRANSFER_READ
                } else {
                    vk::AccessFlags::empty()
                });
            unsafe {
                self.device
                    .begin_command_buffer(command, &vk::CommandBufferBeginInfo::default())?;
                self.device.cmd_pipeline_barrier(
                    command,
                    if acquire {
                        vk::PipelineStageFlags::TOP_OF_PIPE
                    } else {
                        vk::PipelineStageFlags::ALL_COMMANDS
                    },
                    if acquire {
                        vk::PipelineStageFlags::TRANSFER
                    } else {
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE
                    },
                    vk::DependencyFlags::empty(),
                    &[],
                    &[barrier],
                    &[],
                );
                if let Some(staging) = staging {
                    self.device.cmd_copy_buffer(
                        command,
                        buffer,
                        staging,
                        &[vk::BufferCopy::default().size(1024)],
                    );
                    let host = vk::MemoryBarrier::default()
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(vk::AccessFlags::HOST_READ);
                    self.device.cmd_pipeline_barrier(
                        command,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::HOST,
                        vk::DependencyFlags::empty(),
                        &[host],
                        &[],
                        &[],
                    );
                }
                self.device.end_command_buffer(command)?;
                self.device.queue_submit(
                    self.queue,
                    &[vk::SubmitInfo::default().command_buffers(&[command])],
                    vk::Fence::null(),
                )?;
                self.device.queue_wait_idle(self.queue)?;
            }
            Ok(())
        }
        unsafe fn initialize(
            &mut self,
            source: vk::Buffer,
            destination: vk::Buffer,
            negative_validation: bool,
        ) -> Result<(), Error> {
            let pool = unsafe {
                self.device.create_command_pool(
                    &vk::CommandPoolCreateInfo::default().queue_family_index(self.family),
                    None,
                )?
            };
            self.pool = Some(pool);
            let command = unsafe {
                self.device.allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::default()
                        .command_pool(pool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1),
                )?
            }[0];
            unsafe {
                self.device
                    .begin_command_buffer(command, &vk::CommandBufferBeginInfo::default())?;
                if negative_validation {
                    // Diagnostic only: record a known invalid offset, never submit.
                    self.device.cmd_fill_buffer(command, source, 1, 4, 0);
                    self.device.end_command_buffer(command)?;
                    return Err(
                        "injected invalid fill offset; command buffer was not submitted".into(),
                    );
                }
                self.device
                    .cmd_fill_buffer(command, source, 0, 1024, 0x1234_5678);
                let barrier = vk::MemoryBarrier::default()
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::TRANSFER_READ);
                self.device.cmd_pipeline_barrier(
                    command,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[barrier],
                    &[],
                    &[],
                );
                self.device.cmd_copy_buffer(
                    command,
                    source,
                    destination,
                    &[vk::BufferCopy::default().size(1024)],
                );
                let host_barrier = vk::MemoryBarrier::default()
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::HOST_READ);
                self.device.cmd_pipeline_barrier(
                    command,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::HOST,
                    vk::DependencyFlags::empty(),
                    &[host_barrier],
                    &[],
                    &[],
                );
                self.device.end_command_buffer(command)?;
                self.device.queue_submit(
                    self.queue,
                    &[vk::SubmitInfo::default().command_buffers(&[command])],
                    vk::Fence::null(),
                )?;
                self.device.queue_wait_idle(self.queue)?;
            }
            Ok(())
        }
    }
    unsafe fn device_uuid(instance: &ash::Instance, physical: vk::PhysicalDevice) -> [u8; 16] {
        let mut identity = vk::PhysicalDeviceIDProperties::default();
        let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut identity);
        unsafe {
            instance.get_physical_device_properties2(physical, &mut properties);
        }
        identity.device_uuid
    }
    fn arguments() -> Result<(bool, usize, bool), Error> {
        let mut export_only = false;
        let mut ordinal = 0;
        let mut negative_validation = false;
        let mut selected = false;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--export-only" => export_only = true,
                "--validation-negative" => {
                    export_only = true;
                    negative_validation = true;
                }
                "--cuda-device" => {
                    ordinal = args
                        .next()
                        .ok_or("--cuda-device requires an ordinal")?
                        .parse()?;
                    selected = true;
                }
                _ => {
                    return Err(format!(
                        "unknown argument: {arg}; vulkan_external [--export-only|--cuda-device N]"
                    )
                    .into());
                }
            }
        }
        if export_only && selected {
            return Err("--export-only and --cuda-device cannot be combined".into());
        }
        Ok((export_only, ordinal, negative_validation))
    }
    fn verify_gravity_view(
        compute: &voxy_cuda::CudaCompute,
        imported: &mut voxy_cuda::CudaExternalU32Buffer,
        sentinel: u32,
    ) -> Result<Vec<u32>, Error> {
        use voxy_cuda::{CudaError, CudaGravityBody, CudaGravityBudget, CudaGravityParameters};
        let parameters = CudaGravityParameters {
            constant: 0.0,
            softening: 0.0,
            uniform_acceleration: [0.0; 3],
            dt: 0.25,
        };
        let bodies = [
            CudaGravityBody {
                mass: 2.0,
                position: [-0.5, -0.25, 0.0],
                velocity: [1.0, -1.0, 0.0],
            },
            CudaGravityBody {
                mass: 1.0,
                position: [0.5, 0.25, 0.0],
                velocity: [-1.0, 1.0, 0.0],
            },
        ];
        let mut oversized = compute.create_gravity_job(
            &[bodies[0]; 32],
            parameters,
            CudaGravityBudget::default(),
        )?;
        assert!(matches!(
            oversized.write_render_view(imported),
            Err(CudaError::BufferLimit)
        ));
        assert_eq!(imported.read()?, vec![sentinel; 256]);
        let mut job =
            compute.create_gravity_job(&bodies, parameters, CudaGravityBudget::default())?;
        job.step(1)?;
        job.write_render_view(imported)?;
        let mut expected = vec![sentinel; 256];
        expected[..8].fill(0);
        expected[3] = 2;
        for (index, values) in [
            [-0.25_f32, -0.5, 0.0, 2.0, 1.0, -1.0, 0.0, 0.0],
            [0.25, 0.5, 0.0, 1.0, -1.0, 1.0, 0.0, 0.0],
        ]
        .iter()
        .enumerate()
        {
            for (component, value) in values.iter().enumerate() {
                expected[8 + index * 8 + component] = value.to_bits();
            }
        }
        if imported.read()? != expected {
            return Err("CUDA gravity render view mismatch".into());
        }
        let mut large = bodies[0];
        large.position[0] = 1e300;
        let mut overflow =
            compute.create_gravity_job(&[large], parameters, CudaGravityBudget::default())?;
        assert!(matches!(
            overflow.write_render_view(imported),
            Err(CudaError::RenderViewOutOfRange)
        ));
        assert_eq!(imported.read()?, expected);
        large = bodies[0];
        large.mass = 1e-300;
        let mut underflow =
            compute.create_gravity_job(&[large], parameters, CudaGravityBudget::default())?;
        assert!(matches!(
            underflow.write_render_view(imported),
            Err(CudaError::RenderViewOutOfRange)
        ));
        assert_eq!(imported.read()?, expected);
        let mut singular = compute.create_gravity_job(
            &[bodies[0]; 2],
            CudaGravityParameters {
                constant: 1.0,
                ..parameters
            },
            CudaGravityBudget::default(),
        )?;
        singular.step(1)?;
        assert!(matches!(
            singular.write_render_view(imported),
            Err(CudaError::SingularPair)
        ));
        assert_eq!(imported.read()?, expected);
        println!(
            "PASS: CUDA f64 gravity -> external f32 render view, four-byte status only; overflow/singular publication rollback"
        );
        Ok(expected)
    }
    pub fn run() -> Result<(), Error> {
        let (export_only, ordinal, negative_validation) = arguments()?;
        let compute = if export_only {
            None
        } else {
            Some(voxy_cuda::CudaCompute::new(ordinal, 1024 * 1024)?)
        };
        let required_uuid = compute
            .as_ref()
            .map(|compute| compute.capabilities().map(|capabilities| capabilities.uuid))
            .transpose()?;
        if required_uuid == Some([0; 16]) {
            return Err("CUDA returned a zero device UUID; refusing import".into());
        }
        // SAFETY: This probe owns every Vulkan object, waits for queue completion
        // and performs no graphics accesses while the imported mapping exists.
        unsafe {
            let mut context = Export::new(required_uuid)?;
            let properties = context
                .instance
                .get_physical_device_properties(context.physical);
            println!(
                "Vulkan export: {:?}",
                CStr::from_ptr(properties.device_name.as_ptr())
            );
            let (buffer, memory, size) = context.allocate(true, false)?;
            println!("Exported allocation bytes: {size}");
            let (staging, host, _) = context.allocate(false, true)?;
            context.initialize(buffer, staging, negative_validation)?;
            context.expect_host(host, 0x1234_5678)?;
            let exporter =
                ash::khr::external_memory_fd::Device::new(&context.instance, &context.device);
            let fd = exporter.get_memory_fd(
                &vk::MemoryGetFdInfoKHR::default()
                    .memory(memory)
                    .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD),
            )?;
            let file = File::from_raw_fd(fd);
            if export_only {
                drop(file);
                context.handoff(buffer, None)?;
                context.handoff(buffer, Some(staging))?;
                context.expect_host(host, 0x1234_5678)?;
                println!(
                    "PASS: Vulkan opaque FD export, external-family release/acquire, 256-word fill/copy readback; no external writer or CUDA execution"
                );
                return Ok(());
            }
            let compute = compute.as_ref().ok_or("CUDA context missing")?;
            let cuda = compute.capabilities()?;
            let vulkan_uuid = device_uuid(&context.instance, context.physical);
            if vulkan_uuid == [0; 16] || vulkan_uuid != cuda.uuid {
                return Err(format!(
                    "CUDA/Vulkan UUID mismatch: CUDA {:?}, Vulkan {:?}; refusing import",
                    cuda.uuid, vulkan_uuid
                )
                .into());
            }
            println!("PASS: CUDA/Vulkan physical GPU UUID match {:?}", cuda.uuid);
            context.handoff(buffer, None)?;
            let mut imported = compute.import_external_u32(file, usize::try_from(size)?, 0, 256)?;
            if compute.reserved_device_bytes()? != usize::try_from(size)? {
                return Err("CUDA Vulkan import reservation size mismatch".into());
            }
            imported.affine(3, 7)?;
            let expected = 0x1234_5678_u32.wrapping_mul(3).wrapping_add(7);
            if imported.read()?.iter().any(|value| *value != expected) {
                return Err("CUDA imported memory write/readback mismatch".into());
            }
            let view = verify_gravity_view(compute, &mut imported, expected)?;
            imported.release()?;
            if compute.reserved_device_bytes()? != 0 {
                return Err("CUDA Vulkan import or gravity reservation leaked".into());
            }
            println!(
                "PASS: Vulkan CUDA import reserves full allocation and releases all gravity/import reservations"
            );
            context.handoff(buffer, Some(staging))?;
            context.expect_host_words(host, &view)?;
            println!(
                "PASS: Vulkan export -> CUDA mapped write -> Vulkan acquire/readback, gravity render view and untouched tail; synchronized same-GPU ownership"
            );
            Ok(())
        }
    }
}
#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux::run()
}
#[cfg(not(target_os = "linux"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err("vulkan_external requires Linux".into())
}
