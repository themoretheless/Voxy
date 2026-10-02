fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut smoke = false;
    let mut collision = false;
    let mut planet = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--smoke" => smoke = true,
            "--collision" => collision = true,
            "--planet" => planet = true,
            _ => {
                return Err(format!(
                    "unknown argument {arg}; gravity [--smoke] [--collision|--planet]"
                )
                .into());
            }
        }
    }
    let app = voxy_app::SceneApp::new(smoke)?;
    let app = if planet {
        app.with_gravity_planet()
    } else {
        app.with_gravity(collision)
    };
    app.run(winit::event_loop::EventLoop::new()?)
}
