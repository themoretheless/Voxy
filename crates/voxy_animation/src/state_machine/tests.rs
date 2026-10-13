use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use super::*;
use crate::anim_graph::{AnimGraph, FloatInput, GraphDesc, GraphNode, GraphPlayer};
use crate::{
    AnimationClip, Animator, Joint, JointTrack, Playback, QuatKey, Skeleton, Transform, Vec3Key,
};

fn rig() -> Skeleton {
    let joint = |name: &str, parent, y: f32| Joint {
        name: Arc::from(name),
        parent,
        bind_local: Transform {
            translation: Vec3::new(0.0, y, 0.0),
            ..Transform::IDENTITY
        },
        inverse_bind: Mat4::from_translation(Vec3::new(0.0, -y, 0.0)),
    };
    Skeleton::new(vec![
        joint("hips", None, 1.0),
        joint("spine", Some(0), 0.5),
        joint("shin", Some(0), -0.5),
    ])
    .unwrap()
}

fn clip(rig: &Skeleton, name: &str, duration: f32, amplitude: f32) -> Arc<AnimationClip> {
    let mut tracks = vec![JointTrack::default(); 3];
    tracks[0].translations = vec![
        Vec3Key {
            time: 0.0,
            value: Vec3::new(0.0, 1.0, 0.0),
        },
        Vec3Key {
            time: duration,
            value: Vec3::new(amplitude, 1.0, 0.0),
        },
    ];
    tracks[1].rotations = vec![
        QuatKey {
            time: 0.0,
            value: Quat::IDENTITY,
        },
        QuatKey {
            time: duration * 0.5,
            value: Quat::from_rotation_z(amplitude * 0.2),
        },
        QuatKey {
            time: duration,
            value: Quat::IDENTITY,
        },
    ];
    tracks[2].translations = vec![
        Vec3Key {
            time: 0.0,
            value: Vec3::new(0.0, -0.5, 0.0),
        },
        Vec3Key {
            time: duration,
            value: Vec3::new(0.0, -0.5, amplitude),
        },
    ];
    Arc::new(AnimationClip::new(name, duration, playback_for(name), tracks, rig).unwrap())
}

fn playback_for(name: &str) -> Playback {
    if name.starts_with("clamp") {
        Playback::Clamp
    } else {
        Playback::Loop
    }
}

struct Built {
    graph: Arc<AnimGraph>,
    machine: NodeId,
    go: ParamId,
    speed: ParamId,
    states: Vec<NodeId>,
}

/// idle -(go)-> walk -(speed>3)-> run; run -(speed<2)-> walk; any -(stop)-> idle.
fn build(
    rig: &Skeleton,
    clips: &[Arc<AnimationClip>],
    duration: f32,
    curve: FadeCurve,
    sync: PhaseSync,
) -> Built {
    let mut desc = GraphDesc::default();
    let go = desc.add_param("go", ParamValue::Bool(false)).unwrap();
    let stop = desc.add_param("stop", ParamValue::Bool(false)).unwrap();
    let speed = desc.add_param("speed", ParamValue::Float(0.0)).unwrap();
    let states: Vec<NodeId> = clips
        .iter()
        .map(|c| {
            desc.add_node(GraphNode::Clip {
                clip: c.clone(),
                speed: FloatInput::Const(1.0),
            })
            .unwrap()
        })
        .collect();
    let mut sm = StateMachineDesc::default();
    let idle = sm.add_state("idle", states[0]).unwrap();
    let walk = sm.add_state("walk", states[1]).unwrap();
    let run = sm.add_state("run", states[2]).unwrap();
    sm.entry = Some(idle);
    let edge = |from, to, condition, priority| StateTransition {
        from,
        to,
        condition,
        duration,
        curve,
        sync,
        interruptible: true,
        priority,
    };
    sm.transitions = vec![
        edge(Some(idle), walk, Condition::Trigger(go), 0),
        edge(Some(walk), run, Condition::FloatGt(speed, 3.0), 0),
        edge(Some(run), walk, Condition::FloatLt(speed, 2.0), 0),
        edge(None, idle, Condition::Trigger(stop), 5),
    ];
    let machine = desc.add_node(GraphNode::StateMachine(sm)).unwrap();
    desc.root = Some(machine);
    let graph = Arc::new(AnimGraph::compile(desc, rig).unwrap());
    Built {
        graph,
        machine,
        go,
        speed,
        states,
    }
}

#[test]
fn two_state_linear_fade_matches_animator_transition() {
    let rig = rig();
    let idle = clip(&rig, "idle", 1.0, 0.5);
    let walk = clip(&rig, "walk", 0.8, 1.5);
    let run = clip(&rig, "run", 0.5, 3.0);
    let built = build(
        &rig,
        &[idle.clone(), walk.clone(), run],
        0.3,
        FadeCurve::Linear,
        PhaseSync::Restart,
    );
    let mut player = GraphPlayer::new(built.graph.clone()).unwrap();
    let mut animator = Animator::new(idle);
    let dt = 0.05;
    for _ in 0..7 {
        let a = animator.advance_wall(&rig, dt).unwrap();
        let g = player.advance(dt).unwrap();
        assert_eq!(a.pose.local(), g.pose.local());
        assert_eq!(a.root_motion, g.root_motion);
    }
    player.set_param(built.go, ParamValue::Bool(true)).unwrap();
    animator.transition_to(walk, 0.3).unwrap();
    let mut max_motion_error = 0.0f32;
    for step in 0..30 {
        let a = animator.advance_wall(&rig, dt).unwrap();
        let g = player.advance(dt).unwrap();
        assert_eq!(a.pose.local(), g.pose.local(), "step {step}");
        max_motion_error = max_motion_error.max((a.root_motion - g.root_motion).length());
        let status = player.machine_status(built.machine).unwrap();
        assert!(
            (status.weight - a.transition_weight).abs() < 1e-6,
            "step {step}"
        );
    }
    // Trapezoidal subtree weighting vs the animator's exact weighted integral.
    assert!(max_motion_error < 2e-3, "{max_motion_error}");
    assert_eq!(player.param(built.go), Some(ParamValue::Bool(false)));
    let status = player.machine_status(built.machine).unwrap();
    assert_eq!(status.current, StateId(1));
    assert_eq!(status.fading_from, None);
}

#[test]
fn interrupted_fade_starts_from_displayed_pose() {
    let rig = rig();
    let clips = [
        clip(&rig, "idle", 1.0, 0.5),
        clip(&rig, "walk", 0.8, 1.5),
        clip(&rig, "run", 0.5, 3.0),
    ];
    let built = build(&rig, &clips, 0.4, FadeCurve::Linear, PhaseSync::Restart);
    let mut player = GraphPlayer::new(built.graph.clone()).unwrap();
    player.advance(0.1).unwrap();
    player.set_param(built.go, ParamValue::Bool(true)).unwrap();
    player.advance(0.1).unwrap();
    let displayed = player.advance(0.1).unwrap();
    player
        .set_param(built.speed, ParamValue::Float(5.0))
        .unwrap();
    // Interrupt at weight 0.5: the frozen source is the pose displayed above.
    let frame = player.advance(0.1).unwrap();
    let status = player.machine_status(built.machine).unwrap();
    assert_eq!(status.current, StateId(2));
    assert_eq!(status.fading_from, Some(StateId(1)));
    assert!((status.weight - 0.25).abs() < 1e-6);
    let run_pose = clips[2].try_sample_phase(&rig, 0.1 / 0.5).unwrap();
    let expected = crate::Pose::blend(&displayed.pose, &run_pose, 0.25).unwrap();
    assert_eq!(frame.pose.local(), expected.local());
    // The frozen source no longer advances: walk's clock is untouched.
    let walk_clock = player.clock(built.states[1]).unwrap();
    player.advance(0.1).unwrap();
    assert_eq!(player.clock(built.states[1]), Some(walk_clock));
}

#[test]
fn priority_wins_and_triggers_consume_once() {
    let rig = rig();
    let clips = [
        clip(&rig, "idle", 1.0, 0.5),
        clip(&rig, "walk", 0.8, 1.5),
        clip(&rig, "run", 0.5, 3.0),
    ];
    let built = build(&rig, &clips, 0.0, FadeCurve::Linear, PhaseSync::Restart);
    let mut player = GraphPlayer::new(built.graph.clone()).unwrap();
    player.set_param(built.go, ParamValue::Bool(true)).unwrap();
    player.advance(0.1).unwrap();
    assert_eq!(
        player.machine_status(built.machine).unwrap().current,
        StateId(1)
    );
    player
        .set_param(built.speed, ParamValue::Float(9.0))
        .unwrap();
    let stop = built.graph.param("stop").unwrap();
    player.set_param(stop, ParamValue::Bool(true)).unwrap();
    // walk -> run (speed) and any -> idle (stop, priority 5) both hold.
    player.advance(0.1).unwrap();
    assert_eq!(
        player.machine_status(built.machine).unwrap().current,
        StateId(0)
    );
    assert_eq!(player.param(stop), Some(ParamValue::Bool(false)));
    // Entering a state restarts its subtree.
    assert_eq!(player.clock(built.states[0]), Some(0.1));
    // `go` was consumed earlier; idle stays.
    player.advance(0.1).unwrap();
    assert_eq!(
        player.machine_status(built.machine).unwrap().current,
        StateId(0)
    );
}

#[test]
fn phase_sync_preserve_and_at_initialize_target_clock() {
    let rig = rig();
    let clips = [
        clip(&rig, "idle", 1.0, 0.5),
        clip(&rig, "walk", 0.8, 1.5),
        clip(&rig, "run", 0.5, 3.0),
    ];
    for (sync, expected_phase) in [(PhaseSync::Preserve, 0.6), (PhaseSync::At(0.25), 0.25)] {
        let built = build(&rig, &clips, 0.0, FadeCurve::Linear, sync);
        let mut player = GraphPlayer::new(built.graph.clone()).unwrap();
        for _ in 0..6 {
            player.advance(0.1).unwrap();
        }
        player.set_param(built.go, ParamValue::Bool(true)).unwrap();
        player.advance(0.0).unwrap();
        let walk_time = player.clock(built.states[1]).unwrap();
        assert!(
            (walk_time / 0.8 - expected_phase).abs() < 1e-6,
            "{sync:?}: {walk_time}"
        );
    }
}

#[test]
fn smoothstep_curve_is_c1_and_arrives_exactly() {
    assert_eq!(FadeCurve::SmoothStep.weight(0.0), 0.0);
    assert_eq!(FadeCurve::SmoothStep.weight(1.0), 1.0);
    assert_eq!(FadeCurve::SmoothStep.weight(0.5), 0.5);
    let h = 1e-6;
    let slope0 = (FadeCurve::SmoothStep.weight(h) - FadeCurve::SmoothStep.weight(0.0)) / h;
    let slope1 = (FadeCurve::SmoothStep.weight(1.0) - FadeCurve::SmoothStep.weight(1.0 - h)) / h;
    assert!(slope0.abs() < 1e-5 && slope1.abs() < 1e-5);
    assert_eq!(FadeCurve::Linear.weight(1.7), 1.0);
}

#[test]
fn failed_tick_keeps_machine_and_triggers() {
    let rig = rig();
    let clips = [
        clip(&rig, "idle", 1.0, 0.5),
        clip(&rig, "walk", 0.8, 1.5),
        clip(&rig, "run", 0.5, 3.0),
    ];
    let built = build(&rig, &clips, 0.2, FadeCurve::Linear, PhaseSync::Restart);
    let mut player = GraphPlayer::new(built.graph.clone()).unwrap();
    player.set_param(built.go, ParamValue::Bool(true)).unwrap();
    assert!(player.advance(5.0).is_err());
    assert_eq!(player.param(built.go), Some(ParamValue::Bool(true)));
    assert_eq!(
        player.machine_status(built.machine).unwrap().current,
        StateId(0)
    );
}

#[test]
fn compile_rejects_bad_machines() {
    let rig = rig();
    let c = clip(&rig, "idle", 1.0, 0.5);
    let make = |edit: &dyn Fn(&mut GraphDesc, &mut StateMachineDesc, NodeId)| {
        let mut desc = GraphDesc::default();
        let node = desc
            .add_node(GraphNode::Clip {
                clip: c.clone(),
                speed: FloatInput::Const(1.0),
            })
            .unwrap();
        let mut sm = StateMachineDesc::default();
        edit(&mut desc, &mut sm, node);
        let machine = desc.add_node(GraphNode::StateMachine(sm)).unwrap();
        desc.root = Some(machine);
        AnimGraph::compile(desc, &rig).err()
    };
    assert_eq!(
        make(&|_, _, _| {}),
        Some(crate::AnimationError::InvalidStateMachine)
    );
    assert_eq!(
        make(&|_, sm, node| {
            sm.add_state("a", node).unwrap();
        }),
        Some(crate::AnimationError::InvalidStateMachine)
    );
    assert_eq!(
        make(&|_, sm, node| {
            let a = sm.add_state("a", node).unwrap();
            sm.entry = Some(a);
            sm.transitions.push(StateTransition {
                from: Some(a),
                to: StateId(4),
                condition: Condition::Always,
                duration: 0.1,
                curve: FadeCurve::Linear,
                sync: PhaseSync::Restart,
                interruptible: false,
                priority: 0,
            });
        }),
        Some(crate::AnimationError::InvalidStateMachine)
    );
    assert_eq!(
        make(&|desc, sm, node| {
            let a = sm.add_state("a", node).unwrap();
            sm.entry = Some(a);
            let flag = desc.add_param("flag", ParamValue::Int(0)).unwrap();
            sm.transitions.push(StateTransition {
                from: None,
                to: a,
                condition: Condition::Trigger(flag),
                duration: 0.1,
                curve: FadeCurve::Linear,
                sync: PhaseSync::Restart,
                interruptible: false,
                priority: 0,
            });
        }),
        Some(crate::AnimationError::InvalidGraphParam)
    );
    assert_eq!(
        make(&|_, sm, node| {
            let a = sm.add_state("a", node).unwrap();
            sm.entry = Some(a);
            sm.transitions.push(StateTransition {
                from: None,
                to: a,
                condition: Condition::Always,
                duration: 99.0,
                curve: FadeCurve::Linear,
                sync: PhaseSync::Restart,
                interruptible: false,
                priority: 0,
            });
        }),
        Some(crate::AnimationError::InvalidTransitionDuration)
    );
    // A state subtree containing the machine is a cycle.
    let mut desc = GraphDesc::default();
    let mut sm = StateMachineDesc::default();
    let a = sm.add_state("a", NodeId(0)).unwrap();
    sm.entry = Some(a);
    desc.nodes.push(GraphNode::StateMachine(sm));
    desc.root = Some(NodeId(0));
    assert_eq!(
        AnimGraph::compile(desc, &rig).err(),
        Some(crate::AnimationError::GraphCycle)
    );
}

#[test]
fn phase_condition_reads_current_leaf_phase() {
    let rig = rig();
    let a = clip(&rig, "clamp_a", 1.0, 0.5);
    let b = clip(&rig, "b", 0.5, 1.0);
    let mut desc = GraphDesc::default();
    let na = desc
        .add_node(GraphNode::Clip {
            clip: a,
            speed: FloatInput::Const(1.0),
        })
        .unwrap();
    let nb = desc
        .add_node(GraphNode::Clip {
            clip: b,
            speed: FloatInput::Const(1.0),
        })
        .unwrap();
    let mut sm = StateMachineDesc::default();
    let sa = sm.add_state("a", na).unwrap();
    let sb = sm.add_state("b", nb).unwrap();
    sm.entry = Some(sa);
    sm.transitions.push(StateTransition {
        from: Some(sa),
        to: sb,
        condition: Condition::All(vec![Condition::PhaseAtLeast(0.85), Condition::Always]),
        duration: 0.0,
        curve: FadeCurve::Linear,
        sync: PhaseSync::Restart,
        interruptible: false,
        priority: 0,
    });
    let machine = desc.add_node(GraphNode::StateMachine(sm)).unwrap();
    desc.root = Some(machine);
    let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
    let mut player = GraphPlayer::new(graph).unwrap();
    let mut switched_at = None;
    for step in 1..=12 {
        player.advance(0.1).unwrap();
        if player.machine_status(machine).unwrap().current == sb {
            switched_at = Some(step);
            break;
        }
    }
    // Phase reaches 0.85 after step 9; the edge fires on the next tick.
    assert_eq!(switched_at, Some(10));
}
