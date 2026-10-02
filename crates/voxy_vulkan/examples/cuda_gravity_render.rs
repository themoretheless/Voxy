//! Real NVIDIA CUDA physics -> Vulkan or explicit DX12 storage -> shader pixels.
#![allow(unsafe_code)]
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod linux {
    use voxy_cuda::{CudaCompute, CudaGravityBody, CudaGravityBudget, CudaGravityParameters};
    use voxy_vulkan::CudaGravityGraphics;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    enum Gravity {
        Vulkan(Box<CudaGravityGraphics>),
        #[cfg(target_os = "windows")]
        Dx12(Box<voxy_vulkan::CudaGravityD3d12Graphics>),
    }
    impl Gravity {
        fn buffer(&self) -> Result<&wgpu::Buffer, Error> {
            match self {
                Self::Vulkan(job) => job.buffer(),
                #[cfg(target_os = "windows")]
                Self::Dx12(job) => job.buffer(),
            }
        }
        fn body_count(&self) -> u32 {
            match self {
                Self::Vulkan(job) => job.body_count(),
                #[cfg(target_os = "windows")]
                Self::Dx12(job) => job.body_count(),
            }
        }
        unsafe fn publish(&mut self, steps: u32) -> Result<(), Error> {
            match self {
                Self::Vulkan(job) => unsafe { job.publish(steps) },
                #[cfg(target_os = "windows")]
                Self::Dx12(job) => unsafe { job.publish(steps) },
            }
        }
    }
    pub fn run() -> Result<(), Error> {
        let (ordinal, dx12) = arguments()?;
        let compute = CudaCompute::new(ordinal, 1024 * 1024)?;
        let identity = compute.capabilities()?;
        println!("CUDA graphics gravity: {identity:?}");
        if identity.uuid == [0; 16] {
            return Err("CUDA device UUID unavailable".into());
        }
        let (device, queue) = if dx12 {
            #[cfg(target_os = "windows")]
            {
                gpu_dx12(compute.windows_adapter_identity()?)?
            }
            #[cfg(not(target_os = "windows"))]
            {
                return Err("DX12 mode requires Windows".into());
            }
        } else {
            gpu(identity.uuid)?
        };
        let mut gravity = create_gravity(&device, &queue, compute, dx12)?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let drawing = pollster::block_on(voxy_gpu::GravityView::from_buffer(
            &device,
            gravity.buffer()?,
            gravity.body_count(),
            wgpu::TextureFormat::Rgba8Unorm,
        ))?;
        let (texture, pixels) = targets(&device);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        for (steps, expected_x) in [(0, 16_usize), (32, 32), (32, 48)] {
            // SAFETY: Prior graphics work was submitted; no binding is used until publication returns.
            unsafe {
                gravity.publish(steps)?;
            }
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            drawing.encode(&mut encoder, &view);
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::default(),
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &pixels,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(64),
                    },
                },
                texture.size(),
            );
            queue.submit([encoder.finish()]);
            verify_pixels(&device, &pixels, expected_x, steps)?;
        }
        if let Some(error) = pollster::block_on(scope.pop()) {
            return Err(error.into());
        }
        println!(
            "PASS: device-matched CUDA f64 gravity -> {} export -> wgpu shader; initial and 64 evolved steps, exact pixels, no body readback or per-frame body upload",
            if dx12 { "DX12" } else { "Vulkan" }
        );
        Ok(())
    }
    fn create_gravity(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        compute: CudaCompute,
        dx12: bool,
    ) -> Result<Gravity, Error> {
        #[cfg(not(target_os = "windows"))]
        let _ = queue;
        let bodies = [CudaGravityBody {
            mass: 1.0,
            position: [-0.5, 0.0, 0.0],
            velocity: [1.0, 0.0, 0.0],
        }];
        let parameters = CudaGravityParameters {
            constant: 0.0,
            softening: 0.0,
            uniform_acceleration: [0.0; 3],
            dt: 1.0 / 64.0,
        };
        // SAFETY: Single-threaded graphics queue, no pending submissions.
        let gravity = if dx12 {
            #[cfg(target_os = "windows")]
            {
                Gravity::Dx12(Box::new(unsafe {
                    voxy_vulkan::CudaGravityD3d12Graphics::new(
                        device,
                        queue,
                        compute,
                        &bodies,
                        parameters,
                        CudaGravityBudget::default(),
                    )?
                }))
            }
            #[cfg(not(target_os = "windows"))]
            {
                return Err("DX12 mode requires Windows".into());
            }
        } else {
            Gravity::Vulkan(Box::new(unsafe {
                CudaGravityGraphics::new(
                    device,
                    compute,
                    &bodies,
                    parameters,
                    CudaGravityBudget::default(),
                )?
            }))
        };
        Ok(gravity)
    }
    fn arguments() -> Result<(usize, bool), Error> {
        let mut args = std::env::args().skip(1);
        let ordinal = args.next().map_or(Ok(0), |value| value.parse::<usize>())?;
        let dx12 = match args.next().as_deref() {
            None => false,
            Some("--dx12") => true,
            Some(_) => return Err("cuda_gravity_render [device-ordinal] [--dx12]".into()),
        };
        if args.next().is_some() || (dx12 && !cfg!(target_os = "windows")) {
            return Err("DX12 mode requires Windows and accepts no extra arguments".into());
        }
        Ok((ordinal, dx12))
    }
    fn gpu(uuid: [u8; 16]) -> Result<(wgpu::Device, wgpu::Queue), Error> {
        let instance = voxy_render::GraphicsOptions {
            backend: voxy_render::GraphicsBackend::Vulkan,
            ..voxy_render::GraphicsOptions::default()
        }
        .create_instance();
        let adapter = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN))
            .into_iter()
            .find(|adapter| {
                voxy_vulkan::adapter_uuid(adapter).is_ok_and(|candidate| candidate == uuid)
            })
            .ok_or("no Vulkan adapter matching the selected CUDA device UUID")?;
        println!(
            "CUDA render Vulkan: {:?}, UUID {uuid:?}",
            adapter.get_info()
        );
        Ok(pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: voxy_vulkan::EXTERNAL_MEMORY_FEATURE,
                ..wgpu::DeviceDescriptor::default()
            },
        ))?)
    }
    #[cfg(target_os = "windows")]
    fn gpu_dx12((luid, mask): ([u8; 8], u32)) -> Result<(wgpu::Device, wgpu::Queue), Error> {
        if luid == [0; 8] || mask != 1 {
            return Err("CUDA LUID/node identity unavailable or linked".into());
        }
        let instance = voxy_render::GraphicsOptions {
            backend: voxy_render::GraphicsBackend::DirectX12,
            ..voxy_render::GraphicsOptions::default()
        }
        .create_instance();
        let adapter = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::DX12))
            .into_iter()
            .find(|adapter| voxy_vulkan::adapter_luid(adapter).is_ok_and(|value| value == luid))
            .ok_or("no DX12 adapter matching selected CUDA LUID")?;
        println!("CUDA render DX12: {:?}, LUID {luid:?}", adapter.get_info());
        Ok(pollster::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor::default()),
        )?)
    }
    fn targets(device: &wgpu::Device) -> (wgpu::Texture, wgpu::Buffer) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let pixels = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 64 * 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        (texture, pixels)
    }

    fn verify_pixels(
        device: &wgpu::Device,
        pixels: &wgpu::Buffer,
        expected_x: usize,
        steps: u32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (sender, receiver) = std::sync::mpsc::channel();
        pixels
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver.recv()??;
        let data = pixels.slice(..).get_mapped_range()?;
        for x in [16_usize, 32, 48] {
            let offset = 32 * 256 + x * 4;
            let expected = if x == expected_x {
                [0, 255, 0, 255]
            } else {
                [0, 0, 0, 255]
            };
            assert_eq!(
                &data[offset..offset + 4],
                &expected,
                "pixel x={x}, steps={steps}"
            );
        }
        drop(data);
        pixels.unmap();
        Ok(())
    }
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    linux::run()
}
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Err("requires Linux/Windows Vulkan and NVIDIA CUDA".into())
}
