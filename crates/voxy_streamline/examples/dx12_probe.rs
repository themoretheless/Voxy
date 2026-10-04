use std::process::ExitCode;
#[cfg(all(windows, feature = "scene-dx12"))]
#[path = "support/fg_resources.rs"]
mod fg_resources;
#[cfg(any(test, all(windows, feature = "wgpu-dx12")))]
#[path = "support/sr_pixels.rs"]
mod sr_pixels;
#[cfg(all(windows, feature = "wgpu-dx12"))]
#[path = "support/sr_probe.rs"]
mod sr_probe;

#[cfg(all(windows, feature = "wgpu-dx12"))]
#[allow(unsafe_code)]
async fn probe(path: Option<&std::path::Path>) -> Result<(), String> {
    use voxy_streamline::{StreamlineFeature, StreamlineFeatures, StreamlineRuntime};
    let evaluate_sr = std::env::var("VOXY_DLSS_SR").as_deref() == Ok("1");
    if evaluate_sr && path.is_none() {
        return Err("DLSS SR requires a signed SDK DLL path".into());
    }
    let mut runtime = if let Some(path) = path {
        let mut runtime =
            StreamlineRuntime::load(path).map_err(|error| format!("load: {error:?}"))?;
        runtime
            .initialize_dx12(StreamlineFeatures {
                super_resolution: true,
                frame_generation: true,
                reflex: true,
                ray_reconstruction: true,
                neural_rendering: false,
            })
            .map_err(|error| format!("init: {error:?}"))?;
        Some(runtime)
    } else {
        None
    };
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::DX12;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = select_adapter(&instance, runtime.as_ref(), evaluate_sr).await?;
    println!("Selected adapter: {:?}", adapter.get_info());
    if let Some(runtime) = &runtime {
        for feature in [
            StreamlineFeature::SuperResolution,
            StreamlineFeature::FrameGeneration,
            StreamlineFeature::Reflex,
            StreamlineFeature::RayReconstruction,
        ] {
            println!(
                "SDK {feature:?}: {:?}",
                runtime.wgpu_dx12_support(feature, &adapter)
            );
        }
    }
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|error| format!("device: {error}"))?;
    if let Some(runtime) = &mut runtime {
        // SAFETY: Initialized before device creation; no work in flight,
        // retained COM ownership and shutdown precede device release.
        unsafe { runtime.register_wgpu_dx12(&device) }
            .map_err(|error| format!("register: {error:?}"))?;
    }
    native_queue_probe(&device, &queue)?;
    sr_resource_probe(&device, &queue)?;
    #[cfg(feature = "scene-dx12")]
    fg_resources::verify(&device, &queue)?;
    if evaluate_sr {
        let mut sr_runtime = runtime
            .take()
            .ok_or("DLSS SR requires a signed SDK DLL path")?;
        if let Err(error) = sr_probe::evaluate(&mut sr_runtime, &device, &queue) {
            // Evaluation may have submitted work or retained SDK inputs; preserve
            // module/device ownership when completion cannot be established.
            std::mem::forget(sr_runtime);
            return Err(error);
        }
        runtime = Some(sr_runtime);
    }
    if let Some(runtime) = &mut runtime {
        runtime
            .close()
            .map_err(|error| format!("shutdown: {error:?}"))?;
        println!("SDK startup/shutdown probe passed");
    }
    drop(queue);
    drop(device);
    println!("Native queue probe passed; no FG frames were presented");
    Ok(())
}

#[cfg(all(windows, feature = "wgpu-dx12"))]
async fn select_adapter(
    instance: &wgpu::Instance,
    runtime: Option<&voxy_streamline::StreamlineRuntime>,
    evaluate_sr: bool,
) -> Result<wgpu::Adapter, String> {
    if evaluate_sr {
        let runtime = runtime.ok_or("DLSS SR requires initialized SDK runtime")?;
        for adapter in instance.enumerate_adapters(wgpu::Backends::DX12).await {
            let support = runtime.wgpu_dx12_support(
                voxy_streamline::StreamlineFeature::SuperResolution,
                &adapter,
            );
            match support {
                Ok(()) => return Ok(adapter),
                Err(error) => eprintln!("Rejecting SR adapter {:?}: {error:?}", adapter.get_info()),
            }
        }
        return Err("no DX12 adapter accepted by the DLSS SR SDK support query".into());
    }
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            ..Default::default()
        })
        .await
        .map_err(|error| format!("adapter: {error}"))
}

#[cfg(all(windows, feature = "wgpu-dx12"))]
#[allow(unsafe_code)]
fn sr_resource_probe(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(), String> {
    use voxy_streamline::{
        StreamlineError,
        dx12::{SuperResolutionResources, TextureAccess, WgpuQueue},
    };
    use windows::Win32::Graphics::Direct3D12::D3D12_RESOURCE_STATE_COPY_SOURCE;
    // Synthetic RGBA inputs exercise ownership/barriers, not SDK format support.
    let textures = std::array::from_fn::<_, 4, _>(|_| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sr-resource-handoff-probe"),
            size: wgpu::Extent3d {
                width: 64,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        })
    });
    let pixels = [9, 27, 81, 255].repeat(64 * 4);
    for texture in &textures {
        queue.write_texture(
            texture.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
            texture.size(),
        );
    }
    // SAFETY: Same live device, serialized queue and no explicit resource destruction.
    let aliased = unsafe {
        SuperResolutionResources::import(&textures[0], &textures[1], &textures[2], &textures[0])
    };
    if !matches!(aliased, Err(StreamlineError::InvalidOptions)) {
        return Err("aliased SR output was accepted".into());
    }
    // SAFETY: Four distinct resources on the same live device.
    let resources = unsafe {
        SuperResolutionResources::import(&textures[0], &textures[1], &textures[2], &textures[3])
    }
    .map_err(|error| format!("SR resource import: {error:?}"))?;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    resources
        .prepare(&mut encoder)
        .map_err(|error| format!("SR prepare: {error:?}"))?;
    queue.submit([encoder.finish()]);
    // SAFETY: Device and queue remain live, submissions are serialized.
    let native = unsafe { WgpuQueue::from_wgpu(device) }
        .map_err(|error| format!("SR queue import: {error:?}"))?;
    let mut recorder = native
        .recorder()
        .map_err(|error| format!("SR recorder: {error:?}"))?;
    let borrowed = resources.textures();
    // SAFETY: Preparation established these states; all subresources transition
    // on the same queue, then restore before wgpu readback.
    unsafe {
        for (index, texture) in [
            borrowed.color,
            borrowed.depth,
            borrowed.motion,
            borrowed.output,
        ]
        .into_iter()
        .enumerate()
        {
            let before = if index == 3 {
                TextureAccess::StorageReadWrite
            } else {
                TextureAccess::ShaderRead
            };
            recorder.transition_texture(
                texture,
                before.native_state(),
                D3D12_RESOURCE_STATE_COPY_SOURCE,
            );
        }
        borrowed.restore_prepared_states(&mut recorder, [D3D12_RESOURCE_STATE_COPY_SOURCE; 4]);
    }
    let commands = recorder
        .finish()
        .map_err(|error| format!("SR close: {error:?}"))?;
    // SAFETY: Same device/queue, restored states and complete retained resource bundle.
    let submission = unsafe { resources.submit(&native, commands) }
        .map_err(|error| format!("SR submit: {error:?}"))?;
    wait_submission(&submission)?;
    drop(submission);
    for texture in &textures {
        verify_texture(device, queue, texture, &pixels)?;
    }
    println!("SR resource alias rejection and four-texture state restoration/readback passed");
    Ok(())
}

#[cfg(all(windows, feature = "wgpu-dx12"))]
fn wait_submission(submission: &voxy_streamline::dx12::Submission) -> Result<(), String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !submission
        .is_complete()
        .map_err(|error| format!("SR fence: {error:?}"))?
    {
        if std::time::Instant::now() >= deadline {
            return Err("SR resource fence timed out".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    Ok(())
}

#[cfg(all(windows, feature = "wgpu-dx12"))]
#[allow(unsafe_code)]
fn native_queue_probe(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(), String> {
    use std::time::{Duration, Instant};
    use voxy_streamline::{
        Dx12TextureLease,
        dx12::{TextureAccess, WgpuQueue, prepare_texture},
    };
    use windows::Win32::Graphics::Direct3D12::D3D12_RESOURCE_STATE_COPY_SOURCE;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("native-handoff-probe"),
        size: wgpu::Extent3d {
            width: 64,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let pixels = [17, 34, 51, 255].repeat(64 * 4);
    queue.write_texture(
        texture.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(256),
            rows_per_image: Some(4),
        },
        texture.size(),
    );
    // SAFETY: Texture/device stay live; no concurrent submissions or explicit destruction.
    let lease = unsafe { Dx12TextureLease::from_wgpu(&texture) }
        .map_err(|error| format!("texture import: {error:?}"))?;
    let mut handoff = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepare_texture(&mut handoff, &lease, TextureAccess::ShaderRead)
        .map_err(|error| format!("handoff: {error:?}"))?;
    queue.submit([handoff.finish()]);
    // SAFETY: Device remains live; all submissions are serialized on its direct queue.
    let native = unsafe { WgpuQueue::from_wgpu(device) }
        .map_err(|error| format!("native queue import: {error:?}"))?;
    let mut recorder = native
        .recorder()
        .map_err(|error| format!("recorder: {error:?}"))?;
    // SAFETY: wgpu handoff establishes shader-read state; both transitions execute
    // in order after it and restore the tracker state before further wgpu access.
    unsafe {
        recorder.transition_texture(
            &lease,
            TextureAccess::ShaderRead.native_state(),
            TextureAccess::StorageReadWrite.native_state(),
        );
        recorder.uav_barrier(&lease);
        recorder.transition_texture(
            &lease,
            TextureAccess::StorageReadWrite.native_state(),
            D3D12_RESOURCE_STATE_COPY_SOURCE,
        );
        recorder.transition_texture(
            &lease,
            D3D12_RESOURCE_STATE_COPY_SOURCE,
            TextureAccess::ShaderRead.native_state(),
        );
    }
    let commands = recorder
        .finish()
        .map_err(|error| format!("finish: {error:?}"))?;
    // SAFETY: Same queue/device, all resource leases retained, states restored.
    let submission = unsafe { native.submit(commands, vec![lease]) }
        .map_err(|error| format!("native submit: {error:?}"))?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if submission
            .is_complete()
            .map_err(|error| format!("native fence: {error:?}"))?
        {
            break;
        }
        if Instant::now() >= deadline {
            return Err("native fence did not complete within 5 seconds".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(submission);
    verify_texture(device, queue, &texture, &pixels)?;
    println!("Native DX12 texture handoff and 1024-byte readback passed");
    Ok(())
}

#[cfg(all(windows, feature = "wgpu-dx12"))]
fn verify_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    expected: &[u8],
) -> Result<(), String> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("handoff-readback"),
        size: 1024,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        texture.size(),
    );
    let index = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(index),
            timeout: Some(std::time::Duration::from_secs(5)),
        })
        .map_err(|error| format!("readback poll: {error}"))?;
    receiver
        .recv_timeout(std::time::Duration::from_secs(1))
        .map_err(|error| format!("readback callback: {error}"))?
        .map_err(|error| format!("readback map: {error}"))?;
    let mapped = buffer
        .slice(..)
        .get_mapped_range()
        .map_err(|error| format!("readback range: {error}"))?;
    let matches = mapped.as_ref() == expected;
    drop(mapped);
    buffer.unmap();
    if !matches {
        return Err("texture bytes changed during native handoff".into());
    }
    Ok(())
}

fn main() -> ExitCode {
    #[cfg(all(windows, feature = "wgpu-dx12"))]
    {
        let Some(path) = std::env::args_os().nth(1) else {
            eprintln!(
                "usage: dx12_probe --queue-only | <absolute path to signed sl.interposer.dll>"
            );
            return ExitCode::FAILURE;
        };
        let path = if path == "--queue-only" {
            None
        } else {
            Some(std::path::PathBuf::from(path))
        };
        match pollster::block_on(probe(path.as_deref())) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        }
    }
    #[cfg(not(all(windows, feature = "wgpu-dx12")))]
    {
        eprintln!("dx12_probe requires Windows and the wgpu-dx12 feature");
        ExitCode::FAILURE
    }
}
