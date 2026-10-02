//! CC0 realistic adult female mannequin with a nonlinear full-body skin shell.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|a| a == "--smoke");
    voxy_app::SceneApp::new(smoke)?
        .with_female()?
        .run(winit::event_loop::EventLoop::new()?)
}
