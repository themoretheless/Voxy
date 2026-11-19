//! X-ray on the imported body; pass --abstract for the procedural shell.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let app = voxy_shell::SceneApp::new(args.iter().any(|a| a == "--smoke"))?;
    use voxy_demos::scenes::XrayScene;
    let demo = if args.iter().any(|a| a == "--abstract") {
        XrayScene::new()
    } else {
        XrayScene::with_model()?
    };
    app.with_demo(Box::new(demo))
        .run(winit::event_loop::EventLoop::new()?)
}
