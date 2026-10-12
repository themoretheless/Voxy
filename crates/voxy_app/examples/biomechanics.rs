//! Isolated penile pressure chambers and layered anal sphincter FEM specimens.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|a| a == "--smoke");
    voxy_shell::SceneApp::new(smoke)?
        .with_biomechanics()?
        .run(winit::event_loop::EventLoop::new()?)
}
