use std::process::ExitCode;

#[cfg(any(test, windows))]
#[path = "support/fg_policy_args.rs"]
mod fg_policy_args;

#[cfg(all(windows, feature = "wgpu-dx12"))]
#[allow(unsafe_code)]
fn probe(
    path: &std::path::Path,
    policy: Option<voxy_streamline::FrameGeneration>,
) -> Result<(), String> {
    use voxy_streamline::{Dx12Interposition, StreamlineFeatures, StreamlineRuntime};
    use windows::{
        Win32::Graphics::Dxgi::{CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, IDXGIFactory2},
        core::{IUnknown, Interface},
    };
    let mut runtime = StreamlineRuntime::load(path).map_err(|error| format!("load: {error:?}"))?;
    runtime
        .initialize_dx12_with_interposition(
            StreamlineFeatures {
                super_resolution: false,
                frame_generation: true,
                reflex: true,
                ray_reconstruction: false,
                neural_rendering: false,
            },
            Dx12Interposition::ManualFactoryProxy,
        )
        .map_err(|error| format!("initialize: {error:?}"))?;
    // SAFETY: DXGI factory creation has no borrowed inputs; SDK initialized first.
    let factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)) }
        .map_err(|error| format!("create factory: {error}"))?;
    let mut slot = factory.into_raw();
    let original = slot;
    // SAFETY: Transfer the owned COM reference through SDK's in-place replacement
    // immediately after creation; no other owner or graphics operation exists.
    if let Err(error) = unsafe { runtime.upgrade_interface(&mut slot) } {
        // SDK may mutate the slot on failure. Preserve its module and ambiguous
        // reference ownership for process lifetime instead of releasing stale COM.
        std::mem::forget(runtime);
        return Err(format!("upgrade factory: {error:?}"));
    }
    if slot.is_null() {
        std::mem::forget(runtime);
        return Err("SDK returned a null factory".into());
    }
    // SAFETY: Successful upgrade preserves the requested COM interface and transfers
    // the reference in the slot. Release the proxy before SDK shutdown.
    let factory = unsafe { IDXGIFactory2::from_raw(slot) };
    // SAFETY: Live upgraded DXGI factory; this method has no borrowed arguments.
    let current = unsafe { factory.IsCurrent() }.as_bool();
    println!("Upgraded DXGI factory IsCurrent: {current}");
    // SAFETY: The live proxy keeps its underlying factory alive. Borrow the SDK
    // result, then QueryInterface adds an owned reference for this local check.
    let native = unsafe { runtime.native_interface(factory.as_raw()) }
        .map_err(|error| format!("native factory: {error:?}"))?;
    let native_borrow =
        unsafe { IUnknown::from_raw_borrowed(&native) }.ok_or("null native factory")?;
    let native_factory: IDXGIFactory2 = native_borrow
        .cast()
        .map_err(|error| format!("native factory interface: {error}"))?;
    // SAFETY: Original reference was transferred into the proxy, whose native
    // factory remains live until proxy release. QueryInterface owns its result.
    let original_borrow =
        unsafe { IUnknown::from_raw_borrowed(&original) }.ok_or("null original factory")?;
    let original_identity: IUnknown = original_borrow
        .cast()
        .map_err(|error| format!("original identity: {error}"))?;
    let native_identity: IUnknown = native_factory
        .cast()
        .map_err(|error| format!("native identity: {error}"))?;
    if original_identity.as_raw() != native_identity.as_raw() {
        return Err("native factory COM identity differs from original".into());
    }
    println!("Native factory roundtrip preserves COM identity");
    drop(native_identity);
    drop(original_identity);
    drop(native_factory);
    if let Err(error) = composition_swapchain_probe(&factory, &mut runtime, policy) {
        // A failed device upgrade may have mutated an owned raw COM slot.
        // Keep SDK code loaded for any ambiguous reference until process exit.
        std::mem::forget(runtime);
        return Err(error);
    }
    drop(factory);
    runtime
        .close()
        .map_err(|error| format!("shutdown: {error:?}"))?;
    println!("Factory/device proxy lifecycle passed; no FG frames rendered");
    Ok(())
}

#[cfg(all(windows, feature = "wgpu-dx12"))]
#[allow(unsafe_code)]
fn composition_swapchain_probe(
    factory: &windows::Win32::Graphics::Dxgi::IDXGIFactory2,
    runtime: &mut voxy_streamline::StreamlineRuntime,
    policy: Option<voxy_streamline::FrameGeneration>,
) -> Result<(), String> {
    use windows::Win32::Graphics::{
        Direct3D::D3D_FEATURE_LEVEL_12_0,
        Direct3D12::{
            D3D12_COMMAND_QUEUE_DESC, D3D12CreateDevice, ID3D12CommandQueue, ID3D12Device,
            ID3D12Resource,
        },
        Dxgi::{
            Common::{DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
            DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_DISCARD,
            DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIOutput,
        },
    };
    use windows::core::Interface;
    let adapter = select_fg_adapter(factory, runtime)?;
    let mut device: Option<ID3D12Device> = None;
    // SAFETY: Adapter is live and the output slot is valid.
    unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_12_0, &raw mut device) }
        .map_err(|error| format!("DX12 device: {error}"))?;
    let device = device.ok_or("DX12 returned no device")?;
    let mut slot = device.into_raw();
    // SAFETY: Transfer the freshly created device reference immediately, before
    // registration/queue creation. Caller preserves SDK module on any failure.
    unsafe { runtime.upgrade_interface(&mut slot) }
        .map_err(|error| format!("upgrade device: {error:?}"))?;
    if slot.is_null() {
        return Err("SDK returned a null device".into());
    }
    // SAFETY: Successful upgrade returns the same requested COM interface with
    // transferred ownership. Runtime retains another owned reference below.
    let device = unsafe { ID3D12Device::from_raw(slot) };
    // SAFETY: This probe uses the native device only for queue and swapchain creation.
    // The SDK registration borrows it; device remains alive until resources release.
    unsafe { runtime.register_owned_dx12(&device) }
        .map_err(|error| format!("register device: {error:?}"))?;
    // SAFETY: Live native device, valid direct-queue descriptor.
    let queue: ID3D12CommandQueue =
        unsafe { device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC::default()) }
            .map_err(|error| format!("queue: {error}"))?;
    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: 64,
        Height: 64,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        Scaling: DXGI_SCALING_STRETCH,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        AlphaMode: DXGI_ALPHA_MODE_IGNORE,
        ..Default::default()
    };
    // SAFETY: Same live device queue; proxy factory performs swapchain interception.
    let swapchain = unsafe {
        factory.CreateSwapChainForComposition(&queue, &raw const desc, None::<&IDXGIOutput>)
    }
    .map_err(|error| format!("composition swapchain: {error}"))?;
    // SAFETY: The swapchain was created through the live SDK proxy factory.
    let native_swapchain = unsafe { runtime.native_interface(swapchain.as_raw()) }
        .map_err(|error| format!("native swapchain: {error:?}"))?;
    println!(
        "Swapchain proxy pointer differs from native: {}",
        native_swapchain != swapchain.as_raw()
    );
    let fg_state = runtime
        .frame_generation_state(0)
        .map_err(|error| format!("FG state: {error:?}"))?;
    println!("FG state: {fg_state:?}");
    if let Some(mode) = policy {
        mode.validate(&fg_state)
            .map_err(|error| format!("FG preflight: {error:?}"))?;
        let reflex_active = mode != voxy_streamline::FrameGeneration::Off;
        if reflex_active {
            runtime
                .configure_reflex(voxy_streamline::ReflexMode::LowLatency, 0)
                .map_err(|error| format!("enable Reflex: {error:?}"))?;
        }
        // Adapter selection above already confirmed SDK FG and Reflex support.
        #[cfg(feature = "render-policy")]
        {
            let renderer_mode = match mode {
                voxy_streamline::FrameGeneration::Off => voxy_render::FrameGenerationMode::Off,
                voxy_streamline::FrameGeneration::Fixed { generated_frames } => {
                    voxy_render::FrameGenerationMode::Fixed { generated_frames }
                }
                voxy_streamline::FrameGeneration::Dynamic { target_fps } => {
                    voxy_render::FrameGenerationMode::Dynamic { target_fps }
                }
            };
            runtime
                .configure_renderer_frame_generation(
                    0,
                    renderer_mode,
                    &fg_state,
                    true,
                    reflex_active,
                )
                .map_err(|error| format!("configure requested FG policy {mode:?}: {error:?}"))?;
        }
        #[cfg(not(feature = "render-policy"))]
        {
            mode.validate(&fg_state)
                .map_err(|error| format!("FG preflight: {error:?}"))?;
            runtime
                .configure_frame_generation(0, mode)
                .map_err(|error| format!("configure requested FG policy {mode:?}: {error:?}"))?;
        }
        println!("SDK accepted FG policy: {mode:?}; no generated frames presented");
        // This probe creates no frame inputs; disable generation before teardown.
        runtime
            .configure_frame_generation(0, voxy_streamline::FrameGeneration::Off)
            .map_err(|error| format!("disable FG: {error:?}"))?;
        if reflex_active {
            runtime
                .configure_reflex(voxy_streamline::ReflexMode::Off, 0)
                .map_err(|error| format!("disable Reflex: {error:?}"))?;
        }
    }
    // SAFETY: Query owned buffer from the live swapchain; no GPU work is submitted.
    let buffer: ID3D12Resource =
        unsafe { swapchain.GetBuffer(0) }.map_err(|error| format!("swapchain buffer: {error}"))?;
    // SAFETY: Resource query has no borrowed arguments.
    let actual = unsafe { buffer.GetDesc() };
    if actual.Width != 64 || actual.Height != 64 {
        return Err("unexpected swapchain buffer dimensions".into());
    }
    drop(buffer);
    drop(swapchain);
    drop(queue);
    // Runtime retains its own device reference until shutdown.
    drop(device);
    println!("Composition swapchain created through proxy factory; no frames presented");
    Ok(())
}

#[cfg(all(windows, feature = "wgpu-dx12"))]
#[allow(unsafe_code)]
fn select_fg_adapter(
    factory: &windows::Win32::Graphics::Dxgi::IDXGIFactory2,
    runtime: &voxy_streamline::StreamlineRuntime,
) -> Result<windows::Win32::Graphics::Dxgi::IDXGIAdapter1, String> {
    use voxy_streamline::StreamlineFeature;
    use windows::Win32::Graphics::Dxgi::DXGI_ERROR_NOT_FOUND;
    for index in 0..u32::MAX {
        // SAFETY: Factory remains live; DXGI returns an owned adapter reference.
        let adapter = match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => adapter,
            Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(error) => return Err(format!("enumerate adapter {index}: {error}")),
        };
        let fg = runtime.dxgi_support(StreamlineFeature::FrameGeneration, &adapter);
        let reflex = runtime.dxgi_support(StreamlineFeature::Reflex, &adapter);
        println!("Adapter {index}: FG={fg:?}, Reflex={reflex:?}");
        if fg.is_ok() && reflex.is_ok() {
            return Ok(adapter);
        }
    }
    Err("no adapter supports SDK Frame Generation and Reflex; see per-adapter results".into())
}

fn main() -> ExitCode {
    #[cfg(all(windows, feature = "wgpu-dx12"))]
    {
        let Some(path) = std::env::args_os().nth(1) else {
            eprintln!(
                "usage: dxgi_proxy_probe <absolute path to signed sl.interposer.dll> [off|fixed:N|dynamic|dynamic:FPS]"
            );
            return ExitCode::FAILURE;
        };
        let mut args = std::env::args().skip(2);
        let policy = match args
            .next()
            .as_deref()
            .map(fg_policy_args::parse)
            .transpose()
        {
            Ok(policy) => policy,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        };
        if args.next().is_some() {
            eprintln!("unexpected extra arguments");
            return ExitCode::FAILURE;
        }
        match probe(std::path::Path::new(&path), policy) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        }
    }
    #[cfg(not(all(windows, feature = "wgpu-dx12")))]
    {
        eprintln!("dxgi_proxy_probe requires Windows and the wgpu-dx12 feature");
        ExitCode::FAILURE
    }
}
