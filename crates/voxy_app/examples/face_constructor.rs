//! Live native face preview driven by the detailed constructor's JSON preset.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/voxy-face-current.json".to_owned());
    voxy_app::SceneApp::new(false)?
        .with_female_animation()?
        .with_face_parameter_file(path)?
        .run(winit::event_loop::EventLoop::new()?)
}
