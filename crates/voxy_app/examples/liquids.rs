//! Native SPH liquid showcase. Space pauses, R resets, Esc exits.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    let finite = std::env::args().any(|arg| arg == "--finite-source");
    let impacts = std::env::args().any(|arg| arg == "--impacts");
    if finite && impacts {
        return Err("choose finite-source or impacts mode".into());
    }
    let app = voxy_app::SceneApp::new(smoke)?;
    let app = if finite {
        app.with_finite_liquid_sources()?
    } else if impacts {
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
