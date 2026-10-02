//! X-ray on the imported body; pass --abstract for the procedural shell.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let app = voxy_app::SceneApp::new(args.iter().any(|a| a == "--smoke"))?;
    let app = if args.iter().any(|a| a == "--abstract") {
        app.with_xray()
    } else {
        app.with_xray_model()?
    };
    app.run(winit::event_loop::EventLoop::new()?)
}
