fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graphics = match std::env::args().nth(1).as_deref() {
        None | Some("vulkan") => voxy_xr::XrGraphics::Vulkan,
        Some("dx12") => voxy_xr::XrGraphics::DirectX12,
        Some("gles") => voxy_xr::XrGraphics::OpenGlEs,
        Some("gl") => voxy_xr::XrGraphics::OpenGl,
        _ => return Err("expected vulkan|dx12|gl|gles".into()),
    };
    let runtime = voxy_xr::XrRuntime::discover(graphics)?;
    let actions = voxy_xr::XrActions::new(runtime.instance(), voxy_xr::ControllerProfile::Simple)?;
    println!("Actions created: {actions:?}");
    println!(
        "Graphics requirements: {:?}",
        runtime.graphics_requirements()
    );
    println!("OpenXR runtime: {:?}", runtime.instance().properties()?);
    println!("Stereo views: {:?}", runtime.stereo_view_configuration());
    println!("Blend modes: {:?}", runtime.environment_blend_modes());
    println!(
        "VR opaque mode: {:?}",
        runtime.select_environment_blend_mode(&[openxr::EnvironmentBlendMode::OPAQUE])
    );
    println!(
        "HMD: {:?}",
        runtime.instance().system_properties(runtime.system())?
    );
    Ok(())
}
