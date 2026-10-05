//! Inertial preview. Optional JSON file is watched for live MCP updates.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = run();
    if let Err(error) = &result
        && let Ok(path) = std::env::var("VOXY_BODY_PARAMETER_FILE")
    {
        let _ = std::fs::write(
            std::path::Path::new(&path).with_extension("runtime-error.txt"),
            error.to_string(),
        );
    }
    result
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = voxy_app::SceneApp::new(false)?.with_female_secondary()?;
    let mut arguments: Vec<_> = std::env::args().skip(1).collect();
    // LaunchServices app bundles do not supply command-line arguments.
    if arguments.is_empty()
        && let Ok(path) = std::env::var("VOXY_BODY_PARAMETER_FILE")
    {
        arguments.push(path);
    }
    let mut next = 0;
    if let Some(path) = arguments.first().filter(|path| !path.starts_with("--")) {
        app = app.with_body_parameter_file(path)?;
        next = 1;
    }
    if arguments.len() > next {
        if arguments[next] != "--cold-response" || arguments.len() != next + 4 {
            return Err("usage: female_motion [BODY.json] [--cold-response INITIAL ONSET_SECONDS RECOVERY_SECONDS]".into());
        }
        app = app.with_body_cold_response(
            arguments[next + 1].parse()?,
            arguments[next + 2].parse()?,
            arguments[next + 3].parse()?,
        )?;
    }
    app.run(winit::event_loop::EventLoop::new()?)
}
