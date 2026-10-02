//! Smooth rig preview without the full-body nonlinear solver.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    voxy_app::SceneApp::new(false)?
        .with_female_animation()?
        .run(winit::event_loop::EventLoop::new()?)
}
