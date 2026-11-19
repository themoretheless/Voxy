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
    use voxy_demos::scenes::GravityScene;
    let demo = if planet {
        GravityScene::planet()
    } else {
        GravityScene::new(collision)
    };
    let app = voxy_shell::SceneApp::new(smoke)?.with_demo(Box::new(demo));
    app.run(winit::event_loop::EventLoop::new()?)
}
