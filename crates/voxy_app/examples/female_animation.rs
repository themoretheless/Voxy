//! Smooth rig preview without the full-body nonlinear solver.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    voxy_shell::SceneApp::new(false)?
        .with_demo(Box::new(voxy_demos::female_scene::FemaleScene::animation()?))
        .run(winit::event_loop::EventLoop::new()?)
}
