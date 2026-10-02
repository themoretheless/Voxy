//! Windows shared storage acceptance; optional explicit CUDA writer.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let ordinal = match args.as_slice() {
        [] => None,
        [flag, ordinal] if flag == "--cuda-device" => {
            let ordinal = ordinal.parse::<usize>()?;
            i32::try_from(ordinal)?;
            Some(ordinal)
        }
        _ => return Err("d3d12_export [--cuda-device N]".into()),
    };
    if ordinal.is_some() && !cfg!(feature = "cuda") {
        return Err("--cuda-device requires the cuda feature".into());
    }
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = if let Some(ordinal) = ordinal {
            cuda_adapter(&instance, ordinal).await?
        } else {
            instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await?
        };
        let info = adapter.get_info();
        if info.backend != wgpu::Backend::Dx12 {
            return Err("D3D12 acceptance requires DX12".into());
        }
        println!("D3D12 export: {info:?}");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;
        let storage = unsafe { voxy_vulkan::D3d12ExportBuffer::new(&device, &queue, 96) }?;
        if storage.allocation_bytes() < 96 {
            return Err("allocation smaller than logical buffer".into());
        }
        // Exercise owned NT handle lifetime, without importing CUDA or accessing
        // the resource externally. No graphics/CUDA handoff is claimed here.
        drop(unsafe { storage.export_handle() }?);
        let expected = if let Some(ordinal) = ordinal {
            write_cuda(&storage, ordinal)?;
            7_u32
        } else {
            0_u32
        };
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shared committed diagnostic readback"),
            size: 96,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(storage.buffer(), 0, &readback, 0, 96);
        queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        readback.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = sender.send(result);
        });
        device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        receiver.recv()??;
        let data = readback.get_mapped_range(..)?;
        if data
            .chunks_exact(4)
            .any(|word| u32::from_le_bytes([word[0], word[1], word[2], word[3]]) != expected)
        {
            return Err("shared committed buffer does not match expected GPU words".into());
        }
        drop(data);
        readback.unmap();
        if ordinal.is_some() {
            println!(
                "PASS: CUDA writes in shared D3D12 committed storage visible to wgpu; 24 exact words"
            );
        } else {
            println!(
                "PASS: DX12 shared committed storage, owned NT export, zero initialization and wgpu readback; no CUDA writer"
            );
        }
        Ok(())
    })
}

#[cfg(all(target_os = "windows", feature = "cuda"))]
async fn cuda_adapter(
    instance: &wgpu::Instance,
    ordinal: usize,
) -> Result<wgpu::Adapter, Box<dyn std::error::Error + Send + Sync>> {
    let compute = voxy_cuda::CudaCompute::new(ordinal, 8 * 1024 * 1024)?;
    let (luid, mask) = compute.windows_adapter_identity()?;
    if luid == [0; 8] || mask != 1 {
        return Err("CUDA LUID/node identity unavailable or linked".into());
    }
    let adapter = instance
        .enumerate_adapters(wgpu::Backends::DX12)
        .await
        .into_iter()
        .find(|adapter| voxy_vulkan::adapter_luid(adapter).is_ok_and(|value| value == luid))
        .ok_or("no DX12 adapter matching selected CUDA GPU")?;
    println!("CUDA/DX12 selected device {ordinal}, LUID {luid:?}");
    Ok(adapter)
}

#[cfg(all(target_os = "windows", not(feature = "cuda")))]
fn cuda_adapter(
    _: &wgpu::Instance,
    _: usize,
) -> std::future::Ready<Result<wgpu::Adapter, Box<dyn std::error::Error + Send + Sync>>> {
    std::future::ready(Err(
        "CUDA adapter selection requires the cuda feature".into()
    ))
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("D3D12 export acceptance requires Windows");
    std::process::exit(1);
}

#[cfg(all(target_os = "windows", feature = "cuda"))]
#[allow(unsafe_code)]
fn write_cuda(
    storage: &voxy_vulkan::D3d12ExportBuffer,
    ordinal: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let compute = voxy_cuda::CudaCompute::new(ordinal, 8 * 1024 * 1024)?;
    // SAFETY: Constructor drained graphics work; no retained bindings or concurrent
    // access exist in this probe, and storage outlives the imported mapping.
    let mut mapping = unsafe { storage.import_cuda(&compute, 0, 24) }?;
    if compute.reserved_device_bytes()? != usize::try_from(storage.allocation_bytes())? {
        return Err("CUDA DX12 import reservation size mismatch".into());
    }
    mapping.affine(3, 7)?;
    mapping.release()?;
    compute.synchronize()?;
    if compute.reserved_device_bytes()? != 0 {
        return Err("CUDA DX12 import reservation leaked".into());
    }
    println!("PASS: DX12 CUDA import reserves full allocation and releases reservation");
    Ok(())
}
#[cfg(all(target_os = "windows", not(feature = "cuda")))]
fn write_cuda(
    _: &voxy_vulkan::D3d12ExportBuffer,
    _: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Err("CUDA writer requires the cuda feature".into())
}
