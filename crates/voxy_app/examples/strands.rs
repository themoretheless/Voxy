//! Grass and hair: fixed-step CPU strands in the native scene renderer.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    voxy_app::SceneApp::new(smoke)?
        .with_strands()
        .run(winit::event_loop::EventLoop::new()?)
}
