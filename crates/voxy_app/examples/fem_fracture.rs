//! Native dynamic cohesive fracture. Space pauses, R resets, Esc exits.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|a| a == "--smoke");
    voxy_app::SceneApp::new(smoke)?
        .with_fem_fracture()?
        .run(winit::event_loop::EventLoop::new()?)
}
