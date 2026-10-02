use voxy_scene::{AssociatedData, SceneGraph, Transform};
#[test]
fn reuse_failure_and_explicit_invalidation_never_expose_stale_data() {
    let mut scene = SceneGraph::new(2);
    let a = scene.spawn(None, Transform::default()).unwrap();
    let b = scene.spawn(None, Transform::default()).unwrap();
    for id in [a, b] {
        scene.insert_component(id, 1_u32).unwrap();
        scene.insert_component(id, true).unwrap();
    }
    let mut data = AssociatedData::<u32, bool, u32>::new(&scene);
    let mut calls = 0;
    data.synchronize::<()>(&scene, |_, n, _, _| {
        calls += 1;
        Ok(*n)
    })
    .unwrap();
    assert_eq!(calls, 2);
    let address = std::ptr::from_ref(data.get(&scene, a).unwrap().unwrap());
    data.synchronize::<()>(&scene, |_, _, _, _| panic!("unchanged row rebuilt"))
        .unwrap();
    assert_eq!(
        std::ptr::from_ref(data.get(&scene, a).unwrap().unwrap()),
        address
    );
    *scene.component_mut::<u32>(a).unwrap().unwrap() = 2;
    assert_eq!(data.get(&scene, a).unwrap(), None);
    assert!(
        data.synchronize(&scene, |_, _, _, _| Err("injected"))
            .is_err()
    );
    assert_eq!(data.get(&scene, a).unwrap(), None);
    assert_eq!(data.get(&scene, b).unwrap(), Some(&1));
    data.synchronize::<()>(&scene, |id, n, _, old| {
        assert_eq!(id, a);
        assert_eq!(old, Some(&1));
        Ok(*n)
    })
    .unwrap();
    assert_eq!(data.get(&scene, a).unwrap(), Some(&2));
    data.invalidate();
    assert_eq!(data.get(&scene, b).unwrap(), None);
    assert!(data.synchronize(&scene, |_, _, _, _| Err(())).is_err());
    assert_eq!(data.get(&scene, b).unwrap(), None);
    data.synchronize::<()>(&scene, |_, n, _, _| Ok(*n)).unwrap();
    scene.remove_component::<bool>(a).unwrap();
    assert_eq!(data.get(&scene, a).unwrap(), None);
    scene.set_active(b, false).unwrap();
    assert_eq!(data.get(&scene, b).unwrap(), None);
    data.synchronize::<()>(&scene, |_, _, _, _| panic!("ineligible owner"))
        .unwrap();
    scene.set_active(b, true).unwrap();
    data.synchronize::<()>(&scene, |id, n, _, old| {
        assert_eq!(id, b);
        assert!(old.is_none());
        Ok(*n)
    })
    .unwrap();
    scene.remove_subtree(b).unwrap();
    assert!(data.get(&scene, b).is_err());
    let reused = scene.spawn(None, Transform::default()).unwrap();
    scene.insert_component(reused, 9_u32).unwrap();
    scene.insert_component(reused, true).unwrap();
    data.synchronize::<()>(&scene, |_, n, _, old| {
        assert!(old.is_none());
        Ok(*n)
    })
    .unwrap();
    assert_eq!(data.get(&scene, reused).unwrap(), Some(&9));
}

#[test]
fn late_failure_releases_candidates_and_preserves_shared_published_resources() {
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
    let mut scene = SceneGraph::new(3);
    let mut ids = Vec::new();
    for _ in 0..3 {
        let id = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(id, 1_u32).unwrap();
        scene.insert_component(id, true).unwrap();
        ids.push(id);
    }
    let old_drops = Arc::new(AtomicUsize::new(0));
    let new_drops = Arc::new(AtomicUsize::new(0));
    let mut data = AssociatedData::<u32, bool, Resource>::new(&scene);
    data.synchronize::<()>(&scene, |_, _, _, _| Ok(Resource(old_drops.clone())))
        .unwrap();
    let unchanged = std::ptr::from_ref(data.get(&scene, ids[0]).unwrap().unwrap());
    scene.insert_component(ids[1], 2_u32).unwrap();
    scene.insert_component(ids[2], 2_u32).unwrap();
    let mut calls = 0;
    assert!(
        data.synchronize(&scene, |_, _, _, old| {
            assert!(old.is_some());
            calls += 1;
            if calls == 2 {
                Err(())
            } else {
                Ok(Resource(new_drops.clone()))
            }
        })
        .is_err()
    );
    assert_eq!(calls, 2);
    assert_eq!(new_drops.load(Ordering::SeqCst), 1);
    assert_eq!(old_drops.load(Ordering::SeqCst), 0);
    assert_eq!(
        std::ptr::from_ref(data.get(&scene, ids[0]).unwrap().unwrap()),
        unchanged
    );
    assert!(data.get(&scene, ids[1]).unwrap().is_none());
    let other = SceneGraph::new(3);
    assert!(
        data.synchronize::<()>(&other, |_, _, _, _| panic!("foreign factory"))
            .is_err()
    );
    assert_eq!(old_drops.load(Ordering::SeqCst), 0);
    data.synchronize::<()>(&scene, |_, _, _, _| Ok(Resource(new_drops.clone())))
        .unwrap();
    assert_eq!(old_drops.load(Ordering::SeqCst), 2);
    drop(data);
    assert_eq!(old_drops.load(Ordering::SeqCst), 3);
    assert_eq!(new_drops.load(Ordering::SeqCst), 3);
}

#[test]
fn published_query_filters_current_requirements_and_rejects_foreign_scene() {
    let mut scene = SceneGraph::new(4);
    let mut ids = Vec::new();
    for value in 0..4_u32 {
        let id = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(id, value).unwrap();
        scene.insert_component(id, true).unwrap();
        ids.push(id);
    }
    let mut data = AssociatedData::<u32, bool, u32>::new(&scene);
    data.synchronize::<()>(&scene, |_, n, _, _| Ok(*n)).unwrap();
    assert_eq!(data.query(&scene).unwrap().count(), 4);
    scene.component_mut::<u32>(ids[0]).unwrap();
    scene.remove_component::<bool>(ids[1]).unwrap();
    scene.set_active(ids[2], false).unwrap();
    assert_eq!(
        data.query(&scene)
            .unwrap()
            .map(|(id, n)| (id, *n))
            .collect::<Vec<_>>(),
        vec![(ids[3], 3)]
    );
    let other = SceneGraph::new(4);
    assert!(data.query(&other).is_err());
    scene.remove_subtree(ids[3]).unwrap();
    assert_eq!(data.query(&scene).unwrap().count(), 0);
    data.invalidate();
    assert_eq!(data.query(&scene).unwrap().count(), 0);
}

#[test]
fn external_asset_revision_requires_invalidation_and_failure_keeps_it_pending() {
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    scene.insert_component(owner, 2_u32).unwrap();
    scene.insert_component(owner, true).unwrap();
    let mut data = AssociatedData::<u32, bool, u32>::new(&scene);
    let first_asset_value = 10;
    data.synchronize::<()>(&scene, |_, n, _, _| Ok(*n * first_asset_value))
        .unwrap();
    assert_eq!(data.get(&scene, owner).unwrap(), Some(&20));
    // The asset owner publishes a changed version and invalidates its consumer.
    let next_asset_value = 30;
    data.invalidate();
    assert_eq!(data.query(&scene).unwrap().count(), 0);
    assert!(
        data.synchronize(&scene, |_, _, _, _| Err("asset preparation failed"))
            .is_err()
    );
    assert!(data.get(&scene, owner).unwrap().is_none());
    data.synchronize::<()>(&scene, |_, n, _, old| {
        assert_eq!(old, Some(&20));
        Ok(*n * next_asset_value)
    })
    .unwrap();
    assert_eq!(data.get(&scene, owner).unwrap(), Some(&60));
    data.synchronize::<()>(&scene, |_, _, _, _| panic!("stable dependency rebuilt"))
        .unwrap();
}

#[test]
fn explicit_retirement_releases_resources_once_and_rebuilds_without_old_rows() {
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
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    scene.insert_component(owner, 1_u32).unwrap();
    scene.insert_component(owner, true).unwrap();
    let drops = Arc::new(AtomicUsize::new(0));
    let mut data = AssociatedData::<u32, bool, Resource>::new(&scene);
    data.synchronize::<()>(&scene, |_, _, _, _| Ok(Resource(drops.clone())))
        .unwrap();
    data.clear();
    data.clear();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(data.get(&scene, owner).unwrap().is_none());
    assert_eq!(scene.component::<u32>(owner).unwrap(), Some(&1));
    data.synchronize::<()>(&scene, |_, _, _, old| {
        assert!(old.is_none());
        Ok(Resource(drops.clone()))
    })
    .unwrap();
    assert!(data.get(&scene, owner).unwrap().is_some());
    drop(data);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}
