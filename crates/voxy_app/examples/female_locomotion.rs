//! Authored Quaternius gait on the Voxy character; in-place animation preview.
use voxy_demos::humanoid_locomotion::LocomotionClip;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let clip = match std::env::args().nth(1).as_deref().unwrap_or("walk") {
        "idle" => LocomotionClip::Idle,
        "walk" => LocomotionClip::Walk,
        "jog" => LocomotionClip::Jog,
        "sprint" => LocomotionClip::Sprint,
        _ => return Err("choose idle, walk, jog or sprint".into()),
    };
    voxy_shell::SceneApp::new(false)?
        .with_female_locomotion(clip)?
        .run(winit::event_loop::EventLoop::new()?)
}
