//! Native SPH liquid showcase. Space pauses, R resets, Esc exits.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    let finite = std::env::args().any(|arg| arg == "--finite-source");
    let impacts = std::env::args().any(|arg| arg == "--impacts");
    use voxy_demos::scenes::LiquidScene;
    let app = voxy_shell::SceneApp::new(smoke)?;
    let demo = if finite && impacts {
        LiquidScene::finite_impacts()?
    } else if finite {
        LiquidScene::finite_sources()?
    } else if impacts {
        LiquidScene::impacts()?
    } else {
        LiquidScene::new()?
    };
    let demo = demo.with_optics(
        std::env::args().any(|arg| arg == "--impacts")
            && !std::env::args().any(|arg| arg == "--particles"),
    );
    app.with_demo(Box::new(demo))
        .run(winit::event_loop::EventLoop::new()?)
}
