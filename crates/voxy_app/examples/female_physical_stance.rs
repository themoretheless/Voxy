//! Actual character fixed stance through accepted physics and geometric foot IK.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    voxy_shell::SceneApp::new(false)?
        .with_demo(Box::new(
            voxy_demos::female_scene::FemaleScene::physical_stance()?,
        ))
        .run(winit::event_loop::EventLoop::new()?)
}
