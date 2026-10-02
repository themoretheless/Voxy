use voxy_scene::{SceneGraph, Transform};
#[test]
fn revisions_track_mutable_access_replacement_and_reinsertion() {
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    assert_eq!(scene.component_revision::<u32>(owner).unwrap(), None);
    scene.insert_component(owner, 1_u32).unwrap();
    let initial = scene.component_revision::<u32>(owner).unwrap();
    assert_eq!(scene.component::<u32>(owner).unwrap(), Some(&1));
    assert_eq!(scene.component_revision::<u32>(owner).unwrap(), initial);
    scene.component_mut::<u32>(owner).unwrap();
    let borrowed = scene.component_revision::<u32>(owner).unwrap();
    assert_ne!(initial, borrowed);
    scene.insert_component(owner, 1_u32).unwrap();
    let replaced = scene.component_revision::<u32>(owner).unwrap();
    assert_ne!(borrowed, replaced);
    scene.remove_component::<u32>(owner).unwrap();
    assert_eq!(scene.component_revision::<u32>(owner).unwrap(), None);
    scene.insert_component(owner, 1_u32).unwrap();
    assert_ne!(scene.component_revision::<u32>(owner).unwrap(), replaced);
    scene.remove_subtree(owner).unwrap();
    assert!(scene.component_revision::<u32>(owner).is_err());
}

#[test]
fn structural_rollback_and_invalid_access_preserve_revisions() {
    use voxy_scene::SceneMutation;
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    scene.insert_component(owner, 7_u32).unwrap();
    let original = scene.component_revision::<u32>(owner).unwrap();
    assert!(scene.component_mut::<bool>(owner).unwrap().is_none());
    let mut other = SceneGraph::new(1);
    assert!(other.component_mut::<u32>(owner).is_err());
    assert!(other.insert_component(owner, 9_u32).is_err());
    assert_eq!(scene.component_revision::<u32>(owner).unwrap(), original);
    assert!(
        scene
            .apply_atomic(&[
                SceneMutation::RemoveSubtree(owner),
                SceneMutation::SetLocal(owner, Transform::default()),
            ])
            .is_err()
    );
    assert_eq!(scene.component_revision::<u32>(owner).unwrap(), original);
    assert_eq!(scene.component::<u32>(owner).unwrap(), Some(&7));
    scene
        .apply_atomic(&[SceneMutation::SetLocal(owner, Transform::default())])
        .unwrap();
    assert_eq!(scene.component_revision::<u32>(owner).unwrap(), original);
}
