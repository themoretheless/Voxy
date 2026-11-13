//! Grass and hair: fixed-step CPU strands in the native scene renderer.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    voxy_shell::SceneApp::new(smoke)?
        .with_demo(Box::new(voxy_demos::scenes::StrandsScene::new()))
        .run(winit::event_loop::EventLoop::new()?)
}
