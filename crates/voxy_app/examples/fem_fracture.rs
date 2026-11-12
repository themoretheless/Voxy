//! Native dynamic cohesive fracture. Space pauses, R resets, Esc exits.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|a| a == "--smoke");
    voxy_shell::SceneApp::new(smoke)?
        .with_demo(Box::new(voxy_demos::scenes::FemScene::new()?))
        .run(winit::event_loop::EventLoop::new()?)
}
