//! Neutral clothed mannequin: front/rear secondary tissue motion.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    voxy_app::SceneApp::new(smoke)?
        .with_body_motion()
        .run(winit::event_loop::EventLoop::new()?)
}
