//! Skeletal hand grasp. Argument: cylinder, sphere, or a closed outward-wound OBJ.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let object = std::env::args().nth(1).unwrap_or_else(|| "cylinder".into());
    voxy_app::SceneApp::new(false)?
        .with_female_grasp(&object)?
        .run(winit::event_loop::EventLoop::new()?)
}
