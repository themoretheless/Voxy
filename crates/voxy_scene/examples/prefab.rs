//! Headless gameplay demonstration of independent enemy prefab instances.
use glam::Vec3;
use voxy_scene::{Behavior, BehaviorRunner, NodeId, Prefab, PrefabNode, SceneGraph, Transform};

#[derive(Clone, Debug)]
struct Health(u32);

#[derive(Debug)]
struct Regeneration;
impl Behavior for Regeneration {
    fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, _delta: f64) {
        if let Some(health) = scene.component_mut::<Health>(owner).unwrap() {
            health.0 = health.0.saturating_add(1).min(100);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let enemy = Prefab {
        nodes: vec![
            PrefabNode::new(None, Transform::default()).with_component(Health(100)),
            PrefabNode::new(
                Some(0),
                Transform {
                    translation: Vec3::Y,
                    ..Transform::default()
                },
            )
            .with_component(String::from("enemy label")),
        ],
    };
    let mut scene = SceneGraph::new(4);
    let first = enemy.instantiate(&mut scene, None)?;
    let second = enemy.instantiate(&mut scene, None)?;
    scene.set_local(
        second[0],
        Transform {
            translation: Vec3::X * 10.0,
            ..Transform::default()
        },
    )?;
    scene.component_mut::<Health>(first[0])?.unwrap().0 -= 25;
    assert_eq!(scene.component::<Health>(second[0])?.unwrap().0, 100);
    assert_eq!(
        scene.world_matrix(second[1])?.transform_point3(Vec3::ZERO),
        Vec3::new(10.0, 1.0, 0.0)
    );
    scene.set_name(first[0], "enemy")?;
    scene.set_name(second[0], "enemy")?;
    assert_eq!(scene.find_named("enemy").count(), 2);
    let mut behaviors = BehaviorRunner::default();
    behaviors.attach(&mut scene, first[0], Regeneration)?;
    behaviors.fixed_update(&mut scene, 1.0 / 60.0);
    assert_eq!(scene.component::<Health>(first[0])?.unwrap().0, 76);
    scene.set_active(first[0], false)?;
    behaviors.fixed_update(&mut scene, 1.0 / 60.0);
    assert_eq!(scene.component::<Health>(first[0])?.unwrap().0, 76);
    assert_eq!(scene.active_components::<Health>().count(), 1);
    assert!(!scene.active_in_hierarchy(first[1])?);
    assert!(scene.active_self(first[1])?);
    scene.set_active(first[0], true)?;
    assert_eq!(scene.active_components::<Health>().count(), 2);
    behaviors.fixed_update(&mut scene, 1.0 / 60.0);
    assert_eq!(scene.component::<Health>(first[0])?.unwrap().0, 77);
    scene.remove_subtree(first[0])?;
    behaviors.sync(&mut scene);
    assert!(behaviors.is_empty());
    assert_eq!(scene.components::<Health>().count(), 1);
    println!(
        "Prefab gameplay passed: independent health, parent motion, hierarchy activity, behavior lifecycle, subtree cleanup"
    );
    Ok(())
}
