//! Headless public-API acceptance for simulation, lifecycle and render extraction.
use glam::Vec3;
use voxy_scene::{
    Behavior, NodeId, SceneCommand, SceneExtraction, SceneGraph, SceneSimulation, SimulationLimits,
    Transform,
};
#[derive(Clone, Debug)]
struct MeshReference(u32);
#[derive(Debug)]
struct Motion;
impl Behavior for Motion {
    #[allow(clippy::cast_possible_truncation)]
    fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, delta: f64) {
        let mut local = scene.local(owner).expect("live owner");
        local.translation.x += 0.5 * delta as f32;
        scene.set_local(owner, local).expect("finite motion");
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = SceneGraph::new(2);
    let root = scene.spawn(
        None,
        Transform {
            translation: Vec3::X,
            ..Transform::default()
        },
    )?;
    let actor = scene.spawn(Some(root), Transform::default())?;
    scene.insert_component(actor, MeshReference(7))?;
    let mut simulation = SceneSimulation::new(
        &scene,
        SimulationLimits {
            fixed_step: 1.0 / 60.0,
            max_steps: 8,
            max_behaviors: 2,
            max_commands: 4,
        },
    )?;
    simulation.attach(&mut scene, actor, Motion)?;
    let mut ticks = 0;
    for _ in 0..100 {
        let frame = simulation.advance(&mut scene, 0.01)?;
        for result in frame.commands {
            result?;
        }
        ticks += frame.time.steps;
    }
    let mut rendering = SceneExtraction::<MeshReference>::new(2);
    rendering.refresh(&scene)?;
    assert_eq!(ticks, 60);
    assert_eq!(rendering.instances()[0].component.0, 7);
    assert!((rendering.instances()[0].world.w_axis.x - 1.5).abs() < 1e-5);
    let before = scene.local(actor)?;
    simulation
        .commands()
        .push(SceneCommand::SetActive(root, false))?;
    simulation.advance(&mut scene, 0.1)?;
    rendering.refresh(&scene)?;
    assert!(rendering.instances().is_empty());
    assert_eq!(scene.local(actor)?, before);
    simulation
        .commands()
        .push(SceneCommand::RemoveSubtree(root))?;
    simulation.advance(&mut scene, 0.0)?;
    simulation.stop(&mut scene)?;
    rendering.refresh(&scene)?;
    assert!(rendering.instances().is_empty());
    assert!(scene.is_empty());
    println!(
        "FIXED SCENE PASS: 60 ticks, inherited motion/activity, command deletion, empty extraction, lifecycle stop"
    );
    Ok(())
}
