//! Native general 2D/3D scene demo with strict backend selection.
use voxy_render::{GraphicsBackend, GraphicsOptions};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut smoke = false;
    let mut motion = false;
    let mut options = GraphicsOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--motion" => motion = true,
            "--smoke" => smoke = true,
            "--fallback" => options.force_fallback_adapter = true,
            "--backend" => {
                options.backend = match args.next().as_deref() {
                    Some("auto") => GraphicsBackend::Auto,
                    Some("metal") => GraphicsBackend::Metal,
                    Some("dx12") => GraphicsBackend::DirectX12,
                    Some("vulkan") => GraphicsBackend::Vulkan,
                    Some("gl") => GraphicsBackend::OpenGl,
                    _ => return Err("--backend expects auto|metal|dx12|vulkan|gl".into()),
                };
            }
            "--help" => {
                println!(
                    "scene_demo [--smoke] [--motion] [--backend auto|metal|dx12|vulkan|gl] [--fallback]"
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    voxy_app::SceneApp::new(smoke)?
        .with_graphics(options)
        .with_motion_vectors(motion)
        .run(winit::event_loop::EventLoop::new()?)
}
