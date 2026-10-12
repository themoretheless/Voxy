//! Native voxel abrasion scene. Space pauses, R resets, Esc exits.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    voxy_shell::SceneApp::new(smoke)?
        .with_voxel_wear()?
        .run(winit::event_loop::EventLoop::new()?)
}
