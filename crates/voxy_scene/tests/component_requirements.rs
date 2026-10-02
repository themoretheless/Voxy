use voxy_scene::{SceneGraph, Transform};

#[test]
fn requirements_follow_replacement_removal_activity_and_generational_reuse() {
    let mut scene = SceneGraph::new(8);
    let parent = scene.spawn(None, Transform::default()).unwrap();
    let owner = scene.spawn(Some(parent), Transform::default()).unwrap();
    scene.insert_component(owner, 10_u32).unwrap();
    assert_eq!(scene.active_components_with::<u32, String>().count(), 0);
    scene
        .insert_component(owner, String::from("first"))
        .unwrap();
    assert_eq!(
        scene
            .active_components_with::<u32, String>()
            .map(|(id, a, b)| (id, *a, b.clone()))
            .collect::<Vec<_>>(),
        [(owner, 10, String::from("first"))]
    );
    scene
        .insert_component(owner, String::from("replacement"))
        .unwrap();
    assert_eq!(
        scene
            .active_components_with::<u32, String>()
            .next()
            .unwrap()
            .2,
        "replacement"
    );
    scene.set_active(parent, false).unwrap();
    assert_eq!(scene.active_components_with::<u32, String>().count(), 0);
    scene.set_active(parent, true).unwrap();
    scene.remove_component::<String>(owner).unwrap();
    assert_eq!(scene.active_components_with::<u32, String>().count(), 0);
    scene
        .insert_component(owner, String::from("restored"))
        .unwrap();
    assert_eq!(scene.active_components_with::<u32, String>().count(), 1);
    scene.remove_subtree(owner).unwrap();
    let reused = scene.spawn(Some(parent), Transform::default()).unwrap();
    assert_ne!(owner, reused);
    scene.insert_component(reused, 20_u32).unwrap();
    scene
        .insert_component(reused, String::from("new owner"))
        .unwrap();
    let owners: Vec<_> = scene
        .active_components_with::<u32, String>()
        .map(|(id, _, _)| id)
        .collect();
    assert_eq!(owners, [reused]);
    assert!(scene.component::<u32>(owner).is_err());
}

#[test]
fn same_type_requirements_are_shared_borrows_in_slot_order() {
    let mut scene = SceneGraph::new(4);
    let a = scene.spawn(None, Transform::default()).unwrap();
    let b = scene.spawn(None, Transform::default()).unwrap();
    scene.insert_component(a, 1_u32).unwrap();
    scene.insert_component(b, 2_u32).unwrap();
    let values: Vec<_> = scene
        .active_components_with::<u32, u32>()
        .map(|(id, left, right)| {
            assert!(std::ptr::eq(left, right));
            (id, *left)
        })
        .collect();
    assert_eq!(values, [(a, 1), (b, 2)]);
}
