//! Skin, buttock, breast, lip sphincter and penis: abstract soft-tissue samples.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    voxy_app::SceneApp::new(smoke)?
        .with_tissues()
        .run(winit::event_loop::EventLoop::new()?)
}
