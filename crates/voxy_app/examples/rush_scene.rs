fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let smoke = args.iter().any(|arg| arg == "--smoke");
    let custom = args.iter().find(|arg| arg.ends_with(".r"));
    let path = custom.cloned().unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../voxy_rush/examples/player.r"
        )
        .into()
    });
    let selected = if custom.is_none() {
        vec!["distance".into(), "events".into(), "contacts".into()]
    } else {
        vec![]
    };
    let app = voxy_app::SceneApp::new(smoke)?
        .with_script(path, selected)?
        .with_script_character()?;
    let app = if smoke && custom.is_none() {
        app.with_script_smoke()
    } else {
        app
    };
    app.run(winit::event_loop::EventLoop::new()?)
}
