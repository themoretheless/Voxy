use voxy_scene::{
    SceneCommand, SceneGraph, SceneSimulation, SchedulePlan, SimulationLimits, SimulationStepError,
    SystemSpec, Transform,
};

#[test]
fn scheduled_ticks_observe_barrier_and_preserve_typed_failure() {
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    let mut simulation = SceneSimulation::new(
        &scene,
        SimulationLimits {
            fixed_step: 0.01,
            max_steps: 8,
            max_behaviors: 0,
            max_commands: 4,
        },
    )
    .unwrap();
    let spec = |name: &str, phase, after: Vec<String>| SystemSpec {
        name: name.into(),
        phase,
        after,
        access: vec![],
    };
    let plan = SchedulePlan::build(
        &[
            spec("sync", 0, vec![]),
            spec("step", 1, vec!["sync".into()]),
            spec("downstream", 2, vec!["step".into()]),
        ],
        3,
    )
    .unwrap();
    simulation
        .commands()
        .push(SceneCommand::SetActive(owner, false))
        .unwrap();
    let mut calls = vec![];
    let result = simulation.advance_scheduled(&mut scene, 0.03, &plan, |name, scene, _| {
        assert!(!scene.active_in_hierarchy(owner).unwrap());
        calls.push(name.to_owned());
        if name == "step" { Err(42_u32) } else { Ok(()) }
    });
    assert_eq!(calls, ["sync", "step"]);
    match result.unwrap_err() {
        SimulationStepError::System {
            completed_steps,
            error,
        } => {
            assert_eq!(completed_steps, 0);
            assert_eq!(error.system, "step");
            assert_eq!(error.error, 42);
        }
        other => panic!("unexpected failure: {other:?}"),
    }
    let frame = simulation
        .advance_scheduled(&mut scene, 0.0, &plan, |_, _, _| -> Result<(), u32> {
            panic!("zero-tick frame must not dispatch systems")
        })
        .unwrap();
    assert_eq!(frame.time.steps, 0);
    assert!(frame.commands.is_empty());
}

#[test]
fn scoped_scene_grants_reject_undeclared_reads_and_read_only_writes() {
    use voxy_scene::{SceneAccessDenied, SystemAccess};
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    for access in [
        vec![],
        vec![SystemAccess {
            resource: "scene".into(),
            write: false,
        }],
    ] {
        let plan = SchedulePlan::build(
            &[SystemSpec {
                name: "restricted".into(),
                phase: 0,
                after: vec![],
                access: access.clone(),
            }],
            1,
        )
        .unwrap();
        let failure = plan
            .run_scene(&mut scene, |_, mut grant| {
                assert_eq!(grant.read().is_ok(), !access.is_empty());
                grant.write()?.set_active(owner, false).unwrap();
                Ok::<(), SceneAccessDenied>(())
            })
            .unwrap_err();
        assert_eq!(failure.system, "restricted");
        assert!(scene.active_in_hierarchy(owner).unwrap());
    }
    let plan = SchedulePlan::build(
        &[SystemSpec {
            name: "writer".into(),
            phase: 0,
            after: vec![],
            access: vec![SystemAccess {
                resource: "scene".into(),
                write: true,
            }],
        }],
        1,
    )
    .unwrap();
    plan.run_scene(&mut scene, |_, mut grant| {
        grant.write()?.set_active(owner, false).unwrap();
        Ok::<(), SceneAccessDenied>(())
    })
    .unwrap();
    assert!(!scene.active_in_hierarchy(owner).unwrap());
}

#[test]
fn composed_plans_keep_barriers_grants_and_reject_duplicate_dispatch() {
    use voxy_scene::SystemAccess;
    let plan = |name: &str, write| {
        SchedulePlan::build(
            &[SystemSpec {
                name: name.into(),
                phase: 0,
                after: vec![],
                access: vec![SystemAccess {
                    resource: "scene".into(),
                    write,
                }],
            }],
            1,
        )
        .unwrap()
    };
    let read = plan("read", false);
    let write = plan("write", true);
    assert!(SchedulePlan::compose(&[&read, &read], 2).is_err());
    assert!(SchedulePlan::compose(&[&read, &write], 1).is_err());
    let combined = SchedulePlan::compose(&[&read, &write], 2).unwrap();
    assert_eq!(
        combined.batches(),
        &[vec!["read".to_owned()], vec!["write".to_owned()]]
    );
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    combined
        .run_scene(&mut scene, |name, mut grant| {
            if name == "read" {
                assert!(grant.read().unwrap().active_in_hierarchy(owner).unwrap());
                assert!(grant.write().is_err());
            } else {
                grant.write().unwrap().set_active(owner, false).unwrap();
            }
            Ok::<(), ()>(())
        })
        .unwrap();
    assert!(!scene.active_in_hierarchy(owner).unwrap());
}
