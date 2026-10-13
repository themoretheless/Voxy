use super::*;
use crate::{Animator, BlendSample, Joint, JointTrack, Playback, QuatKey, Transform, Vec3Key};
use glam::Quat;

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
        joint("head", Some(1), 0.4),
        joint("thigh", Some(0), -0.1),
        joint("shin", Some(3), -0.5),
    ])
    .unwrap()
}

fn clip(
    rig: &Skeleton,
    name: &str,
    duration: f32,
    amplitude: f32,
    playback: Playback,
) -> Arc<AnimationClip> {
    let mut tracks = vec![JointTrack::default(); 5];
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
            value: Quat::from_rotation_z(amplitude * 0.3),
        },
        QuatKey {
            time: duration,
            value: Quat::IDENTITY,
        },
    ];
    tracks[4].translations = vec![
        Vec3Key {
            time: 0.0,
            value: Vec3::new(0.0, -0.5, 0.0),
        },
        Vec3Key {
            time: duration,
            value: Vec3::new(0.0, -0.5, amplitude),
        },
    ];
    Arc::new(AnimationClip::new(name, duration, playback, tracks, rig).unwrap())
}

fn single_clip_graph(
    rig: &Skeleton,
    clip: Arc<AnimationClip>,
    speed: FloatInput,
) -> Arc<AnimGraph> {
    let mut desc = GraphDesc::default();
    let node = desc.add_node(GraphNode::Clip { clip, speed }).unwrap();
    desc.root = Some(node);
    Arc::new(AnimGraph::compile(desc, rig).unwrap())
}

#[test]
fn single_clip_node_matches_animator_bit_for_bit() {
    let rig = rig();
    for playback in [Playback::Loop, Playback::Clamp] {
        let clip = clip(&rig, "walk", 0.7, 1.3, playback);
        let mut animator = Animator::new(clip.clone());
        animator.set_speed(1.25).unwrap();
        let graph = single_clip_graph(&rig, clip, FloatInput::Const(1.25));
        let mut player = GraphPlayer::new(graph).unwrap();
        for step in 0..200 {
            let dt = 0.013 + f64::from(step % 7) * 0.004;
            let expected = animator.advance_wall(&rig, dt).unwrap();
            let actual = player.advance(dt).unwrap();
            assert_eq!(actual.pose.local(), expected.pose.local(), "step {step}");
            assert_eq!(actual.skin_matrices, expected.skin_matrices, "step {step}");
            assert_eq!(actual.root_motion, expected.root_motion, "step {step}");
        }
    }
}

#[test]
fn speed_param_changes_take_effect_next_tick() {
    let rig = rig();
    let clip = clip(&rig, "walk", 1.0, 1.0, Playback::Loop);
    let mut desc = GraphDesc::default();
    let speed = desc.add_param("speed", ParamValue::Float(1.0)).unwrap();
    let node = desc
        .add_node(GraphNode::Clip {
            clip,
            speed: FloatInput::Param(speed),
        })
        .unwrap();
    desc.root = Some(node);
    let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
    assert_eq!(graph.param("speed"), Some(speed));
    let mut player = GraphPlayer::new(graph).unwrap();
    player.advance(0.25).unwrap();
    assert_eq!(player.clock(node), Some(0.25));
    player.set_param(speed, ParamValue::Float(2.0)).unwrap();
    player.advance(0.25).unwrap();
    assert_eq!(player.clock(node), Some(0.75));
    assert!(player.set_param(speed, ParamValue::Bool(true)).is_err());
    assert!(
        player
            .set_param(speed, ParamValue::Float(f32::NAN))
            .is_err()
    );
    assert!(
        player
            .set_param(ParamId(9), ParamValue::Float(1.0))
            .is_err()
    );
}

#[test]
fn blend_node_equals_pose_blend_and_mixes_root_motion() {
    let rig = rig();
    let a = clip(&rig, "a", 1.0, 1.0, Playback::Loop);
    let b = clip(&rig, "b", 1.0, 3.0, Playback::Loop);
    let mut desc = GraphDesc::default();
    let weight = desc.add_param("w", ParamValue::Float(0.25)).unwrap();
    let na = desc
        .add_node(GraphNode::Clip {
            clip: a.clone(),
            speed: FloatInput::Const(1.0),
        })
        .unwrap();
    let nb = desc
        .add_node(GraphNode::Clip {
            clip: b.clone(),
            speed: FloatInput::Const(1.0),
        })
        .unwrap();
    let root = desc
        .add_node(GraphNode::Blend {
            a: na,
            b: nb,
            weight: FloatInput::Param(weight),
        })
        .unwrap();
    desc.root = Some(root);
    let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
    assert_eq!(graph.evaluation_order().len(), 3);
    let mut player = GraphPlayer::new(graph).unwrap();
    let frame = player.advance(0.3).unwrap();
    let pa = a.try_sample_clock(&rig, 0.3).unwrap();
    let pb = b.try_sample_clock(&rig, 0.3).unwrap();
    assert_eq!(
        frame.pose.local(),
        Pose::blend(&pa, &pb, 0.25).unwrap().local()
    );
    let ma = a.root_curve.integral(0.0, 0.3, 1., 1.).as_vec3();
    let mb = b.root_curve.integral(0.0, 0.3, 1., 1.).as_vec3();
    assert_eq!(frame.root_motion, ma + (mb - ma) * 0.25);
}

#[test]
fn layer_and_additive_nodes_match_pose_operations() {
    let rig = rig();
    let base = clip(&rig, "base", 1.0, 1.0, Playback::Loop);
    let over = clip(&rig, "over", 1.0, 4.0, Playback::Loop);
    let mask = Arc::new(BoneMask::subtree(&rig, 1, 1.0).unwrap());
    let reference = Arc::new(rig.bind_pose());
    let mut desc = GraphDesc::default();
    let nb = desc
        .add_node(GraphNode::Clip {
            clip: base.clone(),
            speed: FloatInput::Const(1.0),
        })
        .unwrap();
    let no = desc
        .add_node(GraphNode::Clip {
            clip: over.clone(),
            speed: FloatInput::Const(1.0),
        })
        .unwrap();
    let layer = desc
        .add_node(GraphNode::Layer {
            base: nb,
            overlay: no,
            mask: mask.clone(),
            weight: FloatInput::Const(0.5),
        })
        .unwrap();
    let na = desc
        .add_node(GraphNode::Clip {
            clip: over.clone(),
            speed: FloatInput::Const(1.0),
        })
        .unwrap();
    let root = desc
        .add_node(GraphNode::Additive {
            base: layer,
            additive: na,
            reference: reference.clone(),
            mask: mask.clone(),
            weight: FloatInput::Const(1.0),
        })
        .unwrap();
    desc.root = Some(root);
    let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
    let mut player = GraphPlayer::new(graph).unwrap();
    let frame = player.advance(0.2).unwrap();
    let pb = base.try_sample_clock(&rig, 0.2).unwrap();
    let po = over.try_sample_clock(&rig, 0.2).unwrap();
    let layered = Pose::blend_masked(&pb, &po, &mask.scaled(0.5).unwrap()).unwrap();
    let expected = Pose::apply_additive(&layered, &po, &reference, &mask).unwrap();
    assert_eq!(frame.pose.local(), expected.local());
    // Layer and additive keep the base root motion.
    assert_eq!(
        frame.root_motion,
        base.root_curve.integral(0.0, 0.2, 1., 1.).as_vec3()
    );
}

#[test]
fn blend_space_node_keeps_phase_and_follows_parameter() {
    let rig = rig();
    let walk = clip(&rig, "walk", 1.0, 1.0, Playback::Loop);
    let run = clip(&rig, "run", 0.5, 2.0, Playback::Loop);
    let space = Arc::new(
        BlendSpace1D::new(
            vec![
                BlendSample {
                    parameter: 1.0,
                    clip: walk,
                },
                BlendSample {
                    parameter: 3.0,
                    clip: run,
                },
            ],
            &rig,
        )
        .unwrap(),
    );
    let mut desc = GraphDesc::default();
    let speed = desc.add_param("speed", ParamValue::Float(1.0)).unwrap();
    let node = desc
        .add_node(GraphNode::BlendSpace1D {
            space: space.clone(),
            parameter: FloatInput::Param(speed),
        })
        .unwrap();
    desc.root = Some(node);
    let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
    let mut player = GraphPlayer::new(graph).unwrap();
    player.advance(0.25).unwrap();
    assert!((player.clock(node).unwrap() - 0.25).abs() < 1e-12);
    player.set_param(speed, ParamValue::Float(3.0)).unwrap();
    let frame = player.advance(0.25).unwrap();
    assert!((player.clock(node).unwrap() - 0.75).abs() < 1e-12);
    let expected = space.evaluate(&rig, 3.0, 0.75).unwrap();
    assert_eq!(frame.pose.local(), expected.local());
}

#[test]
fn failed_tick_preserves_clocks() {
    let rig = rig();
    let clip = clip(&rig, "walk", 1.0, 1.0, Playback::Loop);
    let graph = single_clip_graph(&rig, clip, FloatInput::Const(1.0));
    let mut player = GraphPlayer::new(graph).unwrap();
    player.advance(0.4).unwrap();
    let before = player.clone();
    assert_eq!(
        player.advance(2.0).err(),
        Some(AnimationError::InvalidAnimationTimeStep)
    );
    assert_eq!(
        player.advance(f64::NAN).err(),
        Some(AnimationError::InvalidAnimationTimeStep)
    );
    assert_eq!(player.clock(NodeId(0)), before.clock(NodeId(0)));
}

#[test]
fn compile_rejects_cycles_dangling_ids_and_wrong_rigs() {
    let rig = rig();
    let c = clip(&rig, "walk", 1.0, 1.0, Playback::Loop);
    // Cycle: node 0 blends itself with node 1.
    let mut desc = GraphDesc::default();
    desc.nodes.push(GraphNode::Blend {
        a: NodeId(0),
        b: NodeId(1),
        weight: FloatInput::Const(0.5),
    });
    desc.nodes.push(GraphNode::Clip {
        clip: c.clone(),
        speed: FloatInput::Const(1.0),
    });
    desc.root = Some(NodeId(0));
    assert_eq!(
        AnimGraph::compile(desc, &rig).err(),
        Some(AnimationError::GraphCycle)
    );

    let mut desc = GraphDesc::default();
    desc.nodes.push(GraphNode::Blend {
        a: NodeId(1),
        b: NodeId(7),
        weight: FloatInput::Const(0.5),
    });
    desc.root = Some(NodeId(0));
    assert_eq!(
        AnimGraph::compile(desc, &rig).err(),
        Some(AnimationError::InvalidGraphNode(0))
    );

    let mut desc = GraphDesc::default();
    let bool_param = desc.add_param("flag", ParamValue::Bool(true)).unwrap();
    let node = desc
        .add_node(GraphNode::Clip {
            clip: c.clone(),
            speed: FloatInput::Param(bool_param),
        })
        .unwrap();
    desc.root = Some(node);
    assert_eq!(
        AnimGraph::compile(desc, &rig).err(),
        Some(AnimationError::InvalidGraphParam)
    );

    let other = Skeleton::new(vec![Joint {
        name: Arc::from("solo"),
        parent: None,
        bind_local: Transform::IDENTITY,
        inverse_bind: Mat4::IDENTITY,
    }])
    .unwrap();
    let mut desc = GraphDesc::default();
    let node = desc
        .add_node(GraphNode::Clip {
            clip: c,
            speed: FloatInput::Const(1.0),
        })
        .unwrap();
    desc.root = Some(node);
    assert_eq!(
        AnimGraph::compile(desc, &other).err(),
        Some(AnimationError::SkeletonMismatch)
    );

    assert!(
        GraphDesc::default()
            .add_param("", ParamValue::Int(1))
            .is_err()
    );
    let mut desc = GraphDesc::default();
    desc.add_param("x", ParamValue::Int(1)).unwrap();
    assert!(desc.add_param("x", ParamValue::Int(2)).is_err());
}

#[test]
fn unreachable_nodes_are_not_evaluated() {
    let rig = rig();
    let c = clip(&rig, "walk", 1.0, 1.0, Playback::Loop);
    let mut desc = GraphDesc::default();
    let root = desc
        .add_node(GraphNode::Clip {
            clip: c.clone(),
            speed: FloatInput::Const(1.0),
        })
        .unwrap();
    let orphan = desc
        .add_node(GraphNode::Clip {
            clip: c,
            speed: FloatInput::Const(1.0),
        })
        .unwrap();
    desc.root = Some(root);
    let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
    assert_eq!(graph.evaluation_order(), &[root]);
    let mut player = GraphPlayer::new(graph).unwrap();
    player.advance(0.5).unwrap();
    assert_eq!(player.clock(orphan), Some(0.0));
}

#[test]
fn blend_space_2d_node_tracks_two_parameters() {
    let rig = rig();
    let make = |x: f32, y: f32, d: f32| crate::BlendSample2D {
        parameter: glam::Vec2::new(x, y),
        clip: clip(&rig, "s", d, x + y, Playback::Loop),
    };
    let space = Arc::new(
        crate::BlendSpace2D::new(
            vec![
                make(0.0, 0.0, 1.0),
                make(2.0, 0.0, 0.5),
                make(0.0, 2.0, 0.5),
            ],
            &rig,
        )
        .unwrap(),
    );
    let mut desc = GraphDesc::default();
    let x = desc.add_param("x", ParamValue::Float(0.0)).unwrap();
    let y = desc.add_param("y", ParamValue::Float(0.0)).unwrap();
    let node = desc
        .add_node(GraphNode::BlendSpace2D {
            space: space.clone(),
            x: FloatInput::Param(x),
            y: FloatInput::Param(y),
        })
        .unwrap();
    desc.root = Some(node);
    let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
    let mut player = GraphPlayer::new(graph).unwrap();
    player.advance(0.25).unwrap();
    player.set_param(x, ParamValue::Float(1.0)).unwrap();
    player.set_param(y, ParamValue::Float(1.0)).unwrap();
    let frame = player.advance(0.25).unwrap();
    let phase = player.clock(node).unwrap();
    assert!((phase - 0.75).abs() < 1e-9, "{phase}");
    let expected = space
        .evaluate(&rig, glam::Vec2::new(1.0, 1.0), phase)
        .unwrap();
    assert_eq!(frame.pose.local(), expected.local());
}

mod events {
    use super::*;
    use crate::{ClipEvent, ClipEvents, EventWindow};

    fn track(clip: &Arc<AnimationClip>) -> Arc<ClipEvents> {
        Arc::new(
            ClipEvents::new(
                clip.clone(),
                vec![
                    ClipEvent {
                        name: "start".into(),
                        phase: 0.0,
                    },
                    ClipEvent {
                        name: "step".into(),
                        phase: 0.5,
                    },
                    ClipEvent {
                        name: "end".into(),
                        phase: 1.0,
                    },
                ],
            )
            .unwrap()
            .with_windows(vec![EventWindow {
                name: "planted".into(),
                start: 0.8,
                end: 0.2,
            }])
            .unwrap(),
        )
    }

    #[test]
    fn single_clip_events_match_animator_preview() {
        let rig = rig();
        for playback in [Playback::Loop, Playback::Clamp] {
            let clip = clip(&rig, "walk", 0.7, 1.0, playback);
            let track = track(&clip);
            let mut animator = Animator::new(clip.clone());
            animator.set_speed(1.5).unwrap();
            let mut desc = GraphDesc::default();
            let node = desc
                .add_node(GraphNode::Clip {
                    clip,
                    speed: FloatInput::Const(1.5),
                })
                .unwrap();
            desc.attach_events(node, track.clone()).unwrap();
            desc.root = Some(node);
            let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
            let mut player = GraphPlayer::new(graph).unwrap();
            let mut total = 0;
            for step in 0..200 {
                let dt = 0.011 + f64::from(step % 5) * 0.03;
                let (_, expected) = animator
                    .advance_wall_with_events(&rig, dt, &track, 16)
                    .unwrap();
                let frame = player.advance(dt).unwrap();
                let actual: Vec<_> = frame
                    .events
                    .iter()
                    .map(|e| crate::ClipEventOccurrence {
                        name: e.name.clone(),
                        phase: e.phase,
                    })
                    .collect();
                assert_eq!(actual, expected, "step {step}");
                assert!(
                    frame
                        .events
                        .iter()
                        .all(|e| e.weight == 1.0 && e.node == node)
                );
                total += actual.len();
            }
            assert!(total > 0);
        }
    }

    #[test]
    fn blend_weights_propagate_and_windows_wrap_the_seam() {
        let rig = rig();
        let a = clip(&rig, "a", 1.0, 1.0, Playback::Loop);
        let b = clip(&rig, "b", 1.0, 2.0, Playback::Loop);
        let mut desc = GraphDesc::default();
        let w = desc.add_param("w", ParamValue::Float(0.3)).unwrap();
        let na = desc
            .add_node(GraphNode::Clip {
                clip: a.clone(),
                speed: FloatInput::Const(1.0),
            })
            .unwrap();
        let nb = desc
            .add_node(GraphNode::Clip {
                clip: b.clone(),
                speed: FloatInput::Const(1.0),
            })
            .unwrap();
        desc.attach_events(na, track(&a)).unwrap();
        desc.attach_events(nb, track(&b)).unwrap();
        let root = desc
            .add_node(GraphNode::Blend {
                a: na,
                b: nb,
                weight: FloatInput::Param(w),
            })
            .unwrap();
        desc.root = Some(root);
        let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
        let mut player = GraphPlayer::new(graph).unwrap();
        let frame = player.advance(0.6).unwrap();
        let steps: Vec<(NodeId, f32)> = frame
            .events
            .iter()
            .filter(|e| e.name.as_ref() == "step")
            .map(|e| (e.node, e.weight))
            .collect();
        assert_eq!(steps.len(), 2);
        assert!(steps.contains(&(na, 0.7)) && steps.contains(&(nb, 0.3)));
        assert!(frame.active_windows.is_empty());
        // Phase 0.9 lies in the wrapped window [0.8, 1] ∪ [0, 0.2].
        let frame = player.advance(0.3).unwrap();
        let windows: Vec<_> = frame
            .active_windows
            .iter()
            .map(|w| (w.node, w.weight))
            .collect();
        assert_eq!(windows, vec![(na, 0.7), (nb, 0.3)]);
        // Phase 0.1 after the seam still lies in the window.
        let frame = player.advance(0.2).unwrap();
        assert_eq!(frame.active_windows.len(), 2);
        assert!(frame.events.iter().any(|e| e.name.as_ref() == "end"));
        // Fully faded-out clips emit nothing by default.
        player.set_param(w, ParamValue::Float(1.0)).unwrap();
        let frame = player.advance(0.5).unwrap();
        assert!(frame.events.iter().all(|e| e.node == nb));
        player.set_min_event_weight(0.5).unwrap();
        player.set_param(w, ParamValue::Float(0.4)).unwrap();
        let frame = player.advance(0.5).unwrap();
        assert!(frame.events.iter().all(|e| e.node == na));
        assert!(player.set_min_event_weight(f32::NAN).is_err());
    }

    #[test]
    fn budget_failure_preserves_clock_and_foreign_track_is_rejected() {
        let rig = rig();
        let clip = clip(&rig, "walk", 0.1, 1.0, Playback::Loop);
        let mut desc = GraphDesc::default();
        let node = desc
            .add_node(GraphNode::Clip {
                clip: clip.clone(),
                speed: FloatInput::Const(1.0),
            })
            .unwrap();
        desc.attach_events(node, track(&clip)).unwrap();
        assert!(desc.attach_events(node, track(&clip)).is_err());
        desc.root = Some(node);
        let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
        let mut player = GraphPlayer::new(graph).unwrap();
        player.set_event_budget(2);
        player.advance(0.05).unwrap();
        assert_eq!(player.advance(1.0).err(), Some(AnimationError::EventBudget));
        assert_eq!(player.clock(node), Some(0.05));

        let other = super::clip(&rig, "other", 0.1, 1.0, Playback::Loop);
        let mut desc = GraphDesc::default();
        let node = desc
            .add_node(GraphNode::Clip {
                clip,
                speed: FloatInput::Const(1.0),
            })
            .unwrap();
        desc.attach_events(node, track(&other)).unwrap();
        desc.root = Some(node);
        assert_eq!(
            AnimGraph::compile(desc, &rig).err(),
            Some(AnimationError::InvalidGraphNode(0))
        );
    }
}
