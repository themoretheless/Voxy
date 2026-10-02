use voxy_scene::{ComponentTable, SceneGraph, TableBuildError, Transform};

#[test]
fn failed_rebuild_preserves_all_rows_and_retry_publishes_current_membership() {
    let mut scene = SceneGraph::new(4);
    let parent = scene.spawn(None, Transform::default()).unwrap();
    let a = scene.spawn(Some(parent), Transform::default()).unwrap();
    let b = scene.spawn(None, Transform::default()).unwrap();
    for (id, value) in [(a, 1_u32), (b, 2)] {
        scene.insert_component(id, value).unwrap();
        scene.insert_component(id, true).unwrap();
    }
    let mut table = ComponentTable::new(&scene);
    table
        .rebuild_active_with::<u32, bool, ()>(&scene, |_, n, _, _| Ok(*n))
        .unwrap();
    scene.insert_component(a, 10_u32).unwrap();
    let result = table.rebuild_active_with::<u32, bool, _>(&scene, |id, n, _, old| {
        assert_eq!(old, Some(if id == a { &1 } else { &2 }));
        if id == b { Err("injected") } else { Ok(*n) }
    });
    assert_eq!(
        result,
        Err(TableBuildError::Build {
            owner: b,
            error: "injected"
        })
    );
    assert_eq!(table.get(&scene, a).unwrap(), Some(&1));
    assert_eq!(table.get(&scene, b).unwrap(), Some(&2));
    scene.set_active(parent, false).unwrap();
    scene.remove_subtree(b).unwrap();
    let reused = scene.spawn(None, Transform::default()).unwrap();
    scene.insert_component(reused, 30_u32).unwrap();
    scene.insert_component(reused, true).unwrap();
    table
        .rebuild_active_with::<u32, bool, ()>(&scene, |_, n, _, old| {
            assert!(old.is_none());
            Ok(*n)
        })
        .unwrap();
    assert_eq!(table.stored_len(), 1);
    assert_eq!(table.get(&scene, a).unwrap(), None);
    assert_eq!(table.get(&scene, reused).unwrap(), Some(&30));
    scene.remove_component::<bool>(reused).unwrap();
    table
        .rebuild_active_with::<u32, bool, ()>(&scene, |_, _, _, _| panic!("missing requirement"))
        .unwrap();
    assert_eq!(table.stored_len(), 0);
}

#[test]
fn foreign_scene_does_not_invoke_factory_or_change_table() {
    let scene = SceneGraph::new(1);
    let other = SceneGraph::new(1);
    let mut table = ComponentTable::<u32>::new(&scene);
    assert_eq!(
        table.rebuild_active_with::<u32, bool, ()>(&other, |_, _, _, _| panic!("foreign factory")),
        Err(TableBuildError::ForeignScene)
    );
    assert_eq!(table.stored_len(), 0);
}

#[test]
fn failed_build_drops_staged_resources_without_dropping_published_resources() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Resource(Arc<AtomicUsize>);
    impl Drop for Resource {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let mut scene = SceneGraph::new(2);
    for _ in 0..2 {
        let id = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(id, 1_u32).unwrap();
        scene.insert_component(id, true).unwrap();
    }
    let published = Arc::new(AtomicUsize::new(0));
    let staged = Arc::new(AtomicUsize::new(0));
    let mut table = ComponentTable::new(&scene);
    table
        .rebuild_active_with::<u32, bool, ()>(&scene, |_, _, _, _| Ok(Resource(published.clone())))
        .unwrap();
    let mut calls = 0;
    let result = table.rebuild_active_with::<u32, bool, _>(&scene, |_, _, _, _| {
        calls += 1;
        if calls == 2 {
            Err(())
        } else {
            Ok(Resource(staged.clone()))
        }
    });
    assert!(result.is_err());
    assert_eq!(staged.load(Ordering::SeqCst), 1);
    assert_eq!(published.load(Ordering::SeqCst), 0);
    assert_eq!(table.stored_len(), 2);
    drop(table);
    assert_eq!(published.load(Ordering::SeqCst), 2);
}
