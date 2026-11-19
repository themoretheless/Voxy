//! CC0 realistic adult female mannequin with a nonlinear full-body skin shell.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|a| a == "--smoke");
    voxy_shell::SceneApp::new(smoke)?
        .with_demo(Box::new(voxy_demos::female_scene::FemaleScene::new()?))
        .run(winit::event_loop::EventLoop::new()?)
}
