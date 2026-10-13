use super::*;
#[test]
fn effective_clock_commits_only_with_the_physical_step() {
    let rod = HairRod::new(
        vec![[0., 0., 0.], [0., 0.01, 0.], [0., 0.02, 0.]],
        HairMaterial::default(),
    )
    .unwrap();
    let mut system = HairSystem::new(vec![rod]).unwrap();
    system.substeps = 1;
    system.iterations = 1;
    system.self_collision = false;
    let roots = vec![RootPose {
        position: [0.0001, 0., 0.],
        rotation: [0., 0., 0., 1.],
    }];
    assert_eq!(system.last_step_substeps(), 0);
    system
        .step(1. / 120., &roots, [0.; 3], [0.; 3], &[])
        .unwrap();
    assert_eq!(system.substeps, 1);
    assert_eq!(system.last_step_substeps(), 2);
    let original = format!("{system:?}");
    let calls = std::cell::Cell::new(0);
    assert_eq!(
        system.step_validated(1. / 60., &roots, [0.; 3], [0.; 3], &[], None, |staged| {
            calls.set(calls.get() + 1);
            assert_eq!(staged.last_step_substeps(), 4);
            Err("coupled body admission rejected")
        }),
        Err("coupled body admission rejected")
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(format!("{system:?}"), original);
}
