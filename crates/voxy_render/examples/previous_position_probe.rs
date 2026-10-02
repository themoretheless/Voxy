//! GPU correspondence raster and temporal motion verification without app/physics.
#[path = "../../voxy_ray_probe/src/primary_background.rs"]
mod primary_background;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await?;
        println!("Correspondence GPU: {:?}", adapter.get_info());
        primary_background::probe(&device, &adapter, &queue)
    })
}
