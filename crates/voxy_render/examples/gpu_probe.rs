use voxy_render::{GraphicsBackend, GraphicsCapabilities, GraphicsOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = match std::env::args().nth(1).as_deref() {
        None | Some("auto") => GraphicsBackend::Auto,
        Some("dx12") => GraphicsBackend::DirectX12,
        Some("metal") => GraphicsBackend::Metal,
        Some("vulkan") => GraphicsBackend::Vulkan,
        Some("gl") => GraphicsBackend::OpenGl,
        Some("webgl") => GraphicsBackend::WebGl,
        Some("webgpu") => GraphicsBackend::WebGpu,
        Some(value) => return Err(format!("Unknown backend: {value}").into()),
    };
    let options = GraphicsOptions {
        backend,
        ..Default::default()
    };
    let instance = options.create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: options.power_preference,
        ..Default::default()
    }))?;
    println!("{:#?}", GraphicsCapabilities::discover(&adapter));
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    queue.submit([device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
        .finish()]);
    println!("GPU device and command submission succeeded");
    Ok(())
}
