//! Native SPH liquid showcase. Space pauses, R resets, Esc exits.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    let app = voxy_app::SceneApp::new(smoke)?;
    let app = if std::env::args().any(|arg| arg == "--impacts") {
        app.with_liquid_impacts()?
    } else {
        app.with_liquids()?
    };
    let app = app.with_liquid_optics(
        std::env::args().any(|arg| arg == "--impacts")
            && !std::env::args().any(|arg| arg == "--particles"),
    );
    app.run(winit::event_loop::EventLoop::new()?)
}
