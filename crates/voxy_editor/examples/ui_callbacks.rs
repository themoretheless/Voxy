//! A saved UI action named app.hide is bound by application code on each Play.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let model = args
        .next()
        .ok_or("usage: ui_callbacks model.obj scene.json")?;
    let scene = args.next().ok_or("missing scene path")?;
    let setup = voxy_gameplay::UiActionSetup::new(|handlers| {
        handlers.register("app.hide".into(), |_, event, commands| {
            commands
                .push(voxy_scene::SceneCommand::SetActive(event.owner, false))
                .map_err(|error| error.to_string())
        })
    });
    voxy_editor::run_model_viewport_with_ui_actions(
        &voxy_editor::ModelSource::File(model.into()),
        voxy_editor::ViewportMode::Interactive,
        Some(std::path::Path::new(&scene)),
        setup,
    )
}
