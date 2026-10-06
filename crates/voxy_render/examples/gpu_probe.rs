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
    println!("Enabled device features: {:?}", device.features());
    println!("Enabled device limits: {:#?}", device.limits());
    let submission = queue.submit([device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
        .finish()]);
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: Some(std::time::Duration::from_secs(10)),
    })?;
    println!("GPU device and empty command submission completed; no shader workload executed");
    Ok(())
}
