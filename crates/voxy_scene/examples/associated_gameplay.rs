//! Typed derived gameplay data, updated after queued scene changes.
use voxy_scene::{AssociatedData, NodeId, SceneCommand, SceneCommands, SceneGraph, Transform};

#[derive(Debug)]
struct Radius(f32);
#[derive(Debug)]
struct Density(f32);
#[derive(Debug)]
struct Mass(f32);
fn build(
    _: NodeId,
    radius: &Radius,
    density: &Density,
    _: Option<&Mass>,
) -> Result<Mass, &'static str> {
    if !radius.0.is_finite() || radius.0 <= 0.0 || !density.0.is_finite() || density.0 <= 0.0 {
        return Err("radius and density must be finite and positive");
    }
    let mass = std::f32::consts::PI * radius.0 * radius.0 * density.0;
    if !mass.is_finite() {
        return Err("derived mass overflow");
    }
    Ok(Mass(mass))
}
fn apply(commands: &mut SceneCommands, scene: &mut SceneGraph) {
    // SceneCommands is ordered, not an atomic batch: check every command result.
    for result in commands.apply(scene).unwrap() {
        result.unwrap();
    }
}
fn main() {
    let mut scene = SceneGraph::new(2);
    let a = scene.spawn(None, Transform::default()).unwrap();
    let b = scene.spawn(None, Transform::default()).unwrap();
    let mut commands = SceneCommands::new(&scene, 8);
    for owner in [a, b] {
        commands.insert_component(owner, Radius(1.0)).unwrap();
        commands.insert_component(owner, Density(2.0)).unwrap();
    }
    apply(&mut commands, &mut scene);
    let mut data = AssociatedData::<Radius, Density, Mass>::new(&scene);
    data.synchronize(&scene, build).unwrap();
    assert_eq!(data.query(&scene).unwrap().count(), 2);
    let old_mass = data.get(&scene, a).unwrap().unwrap().0;
    data.synchronize::<()>(&scene, |_, _, _, _| panic!("unchanged data rebuilt"))
        .unwrap();

    commands.insert_component(a, Radius(-1.0)).unwrap();
    assert_eq!(data.query(&scene).unwrap().count(), 2);
    apply(&mut commands, &mut scene);
    assert!(data.synchronize(&scene, build).is_err());
    assert!(data.get(&scene, a).unwrap().is_none());
    assert_eq!(data.query(&scene).unwrap().count(), 1);

    commands.insert_component(a, Radius(2.0)).unwrap();
    apply(&mut commands, &mut scene);
    data.synchronize(&scene, build).unwrap();
    let expected = old_mass * 4.0;
    let actual = data.get(&scene, a).unwrap().unwrap().0;
    assert!((actual - expected).abs() <= expected.abs() * f32::EPSILON);
    commands.push(SceneCommand::SetActive(b, false)).unwrap();
    apply(&mut commands, &mut scene);
    data.synchronize(&scene, build).unwrap();
    assert_eq!(data.query(&scene).unwrap().count(), 1);
    println!("associated gameplay: reuse, failed build, repair and activity checks passed");
}
