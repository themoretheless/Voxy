//! Neutral liquid patch on the torso; 0.05 ml over a 4 cm radius.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut smoke = false;
    let mut motion = false;
    let mut parameters = None;
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--smoke" => smoke = true,
            "--motion" => motion = true,
            _ if argument.starts_with("--") => {
                return Err(format!("unknown argument: {argument}").into());
            }
            _ if parameters.is_none() => parameters = Some(argument),
            _ => return Err("usage: surface_film [BODY.json] [--smoke] [--motion]".into()),
        }
    }
    let mut app = voxy_app::SceneApp::new(smoke)?
        .with_motion_vectors(motion)
        .with_female_secondary()?
        .with_surface_film([0., 0.20, 0.13], 0.04, 5e-8)?;
    if let Some(path) = parameters {
        app = app.with_body_parameter_file(path)?;
    }
    app.run(winit::event_loop::EventLoop::new()?)
}
