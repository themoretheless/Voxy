use super::*;
#[test]
fn token_detects_nonstructural_edits_reverts_and_component_access() {
    let mut scene = SceneGraph::new(8);
    assert_ne!(scene.change_token(), SceneGraph::new(8).change_token());
    let initial = scene.change_token();
    let node = scene.spawn(None, Transform::default()).unwrap();
    assert_ne!(scene.change_token(), initial);
    let mut previous = scene.change_token();
    let structure = scene.structure_revision();
    scene.set_name(node, "actor").unwrap();
    assert_ne!(previous, scene.change_token());
    assert_eq!(structure, scene.structure_revision());
    previous = scene.change_token();
    let moved = Transform {
        translation: Vec3::X,
        ..Default::default()
    };
    scene.set_local(node, moved).unwrap();
    assert_ne!(previous, scene.change_token());
    scene.set_local(node, Transform::default()).unwrap();
    assert_ne!(previous, scene.change_token());
    assert_eq!(structure, scene.structure_revision());
    previous = scene.change_token();
    scene.insert_component(node, 1_u32).unwrap();
    assert_ne!(previous, scene.change_token());
    previous = scene.change_token();
    let _ = scene.component_mut::<u32>(node).unwrap().unwrap();
    assert_ne!(previous, scene.change_token());
    previous = scene.change_token();
    scene.remove_component::<u32>(node).unwrap();
    assert_ne!(previous, scene.change_token());
    previous = scene.change_token();
    let _ = scene.local(node).unwrap();
    let _ = scene.name(node).unwrap();
    let _ = scene.component::<u32>(node).unwrap();
    assert_eq!(previous, scene.change_token());
}
#[test]
fn batches_track_changes_and_atomic_rejection_preserves_token() {
    let mut scene = SceneGraph::new(16);
    let a = scene.spawn(None, Transform::default()).unwrap();
    let b = scene.spawn(None, Transform::default()).unwrap();
    let moved = Transform {
        translation: Vec3::Y,
        ..Default::default()
    };
    let mut previous = scene.change_token();
    scene.set_locals(&[(a, moved), (b, moved)]).unwrap();
    assert_ne!(previous, scene.change_token());
    previous = scene.change_token();
    let invalid = Transform {
        translation: Vec3::NAN,
        ..Default::default()
    };
    assert!(
        scene
            .set_locals(&[(a, Transform::default()), (b, invalid)])
            .is_err()
    );
    assert_eq!(previous, scene.change_token());
    assert_eq!(scene.local(a).unwrap(), moved);
    assert!(
        scene
            .apply_atomic(&[
                SceneMutation::SetLocal(a, Transform::default()),
                SceneMutation::SetLocal(b, invalid),
            ])
            .is_err()
    );
    assert_eq!(previous, scene.change_token());
    scene.reparent(b, Some(a)).unwrap();
    assert_ne!(previous, scene.change_token());
    previous = scene.change_token();
    scene.set_active(a, false).unwrap();
    assert_ne!(previous, scene.change_token());
    previous = scene.change_token();
    scene.remove_subtree(a).unwrap();
    assert_ne!(previous, scene.change_token());
}
