//! Neutral clothed mannequin: front/rear secondary tissue motion.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    voxy_shell::SceneApp::new(smoke)?
        .with_demo(Box::new(voxy_demos::scenes::TissueScene::body()))
        .run(winit::event_loop::EventLoop::new()?)
}
