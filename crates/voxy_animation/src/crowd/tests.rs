use std::sync::Arc;

use glam::{Mat4, Vec3};

use super::*;
use crate::anim_graph::{FloatInput, GraphDesc, GraphNode};
use crate::{AnimationClip, Joint, JointTrack, Playback, Skeleton, Transform, Vec3Key};

fn rig() -> Skeleton {
    Skeleton::new(vec![
        Joint {
            name: Arc::from("root"),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        },
        Joint {
            name: Arc::from("child"),
            parent: Some(0),
            bind_local: Transform {
                translation: Vec3::Y,
                ..Transform::IDENTITY
            },
            inverse_bind: Mat4::from_translation(-Vec3::Y),
        },
    ])
    .unwrap()
}

fn graph(rig: &Skeleton) -> (Arc<AnimGraph>, ParamId) {
    let mut tracks = vec![JointTrack::default(); 2];
    tracks[0].translations = vec![
        Vec3Key {
            time: 0.0,
            value: Vec3::ZERO,
        },
        Vec3Key {
            time: 1.0,
            value: Vec3::X,
        },
    ];
    let clip = Arc::new(AnimationClip::new("move", 1.0, Playback::Loop, tracks, rig).unwrap());
    let mut desc = GraphDesc::default();
    let speed = desc.add_param("speed", ParamValue::Float(1.0)).unwrap();
    let node = desc
        .add_node(GraphNode::Clip {
            clip,
            speed: FloatInput::Param(speed),
        })
        .unwrap();
    desc.root = Some(node);
    (Arc::new(AnimGraph::compile(desc, rig).unwrap()), speed)
}

#[test]
fn crowd_matches_independent_players_and_holds_lod_palettes() {
    let rig = rig();
    let (graph, speed) = graph(&rig);
    let count = 40;
    let mut crowd = CrowdEvaluator::new(graph.clone(), count).unwrap();
    let mut players: Vec<GraphPlayer> = (0..count)
        .map(|_| GraphPlayer::new(graph.clone()).unwrap())
        .collect();
    for i in 0..count {
        let s = 0.5 + i as f32 * 0.1;
        crowd.set_param(i, speed, ParamValue::Float(s)).unwrap();
        players[i].set_param(speed, ParamValue::Float(s)).unwrap();
    }
    crowd.set_lod(1, CrowdLod::Half).unwrap();
    crowd.set_lod(2, CrowdLod::Frozen).unwrap();
    let dt = 0.05;
    for tick in 1..=4 {
        let outcomes = crowd.advance_all(dt).unwrap();
        for (i, player) in players.iter_mut().enumerate() {
            let expected = player.advance(dt).unwrap();
            let palette = crowd.palettes().instance(i).unwrap();
            match i {
                1 => {
                    if tick % 2 == 0 {
                        assert_eq!(outcomes[i], CrowdOutcome::Updated);
                        assert_eq!(palette, expected.skin_matrices.as_slice(), "tick {tick}");
                    } else {
                        assert_eq!(outcomes[i], CrowdOutcome::Held);
                    }
                }
                2 => {
                    assert_eq!(outcomes[i], CrowdOutcome::Held);
                    assert!(palette.iter().all(|m| *m == Mat4::IDENTITY));
                }
                _ => {
                    assert_eq!(outcomes[i], CrowdOutcome::Updated);
                    assert_eq!(
                        palette,
                        expected.skin_matrices.as_slice(),
                        "tick {tick} instance {i}"
                    );
                }
            }
        }
    }
    assert_eq!(crowd.len(), count);
    assert_eq!(crowd.joint_count(), 2);
    assert!(crowd.palettes().instance(count).is_none());
    assert!(crowd.set_lod(count, CrowdLod::Full).is_err());
    assert!(crowd.advance_all(f64::NAN).is_err());
}

#[test]
fn failures_are_isolated_per_instance() {
    let rig = rig();
    // Scale passes through zero at t = 0.5: sampling there is invalid.
    let mut tracks = vec![JointTrack::default(); 2];
    tracks[1].scales = vec![
        Vec3Key {
            time: 0.0,
            value: Vec3::ONE,
        },
        Vec3Key {
            time: 1.0,
            value: -Vec3::ONE,
        },
    ];
    let clip =
        Arc::new(AnimationClip::new("singular", 1.0, Playback::Clamp, tracks, &rig).unwrap());
    let mut desc = GraphDesc::default();
    let speed = desc.add_param("speed", ParamValue::Float(2.0)).unwrap();
    let node = desc
        .add_node(GraphNode::Clip {
            clip,
            speed: FloatInput::Param(speed),
        })
        .unwrap();
    desc.root = Some(node);
    let graph = Arc::new(AnimGraph::compile(desc, &rig).unwrap());
    let mut crowd = CrowdEvaluator::new(graph, 3).unwrap();
    crowd.set_param(1, speed, ParamValue::Float(1.0)).unwrap();
    crowd.set_param(2, speed, ParamValue::Float(0.0)).unwrap();
    let outcomes = crowd.advance_all(0.5).unwrap();
    assert_eq!(outcomes[0], CrowdOutcome::Updated);
    assert!(
        matches!(outcomes[1], CrowdOutcome::Failed(_)),
        "{outcomes:?}"
    );
    assert_eq!(outcomes[2], CrowdOutcome::Updated);
    assert!(
        crowd
            .palettes()
            .instance(1)
            .unwrap()
            .iter()
            .all(|m| *m == Mat4::IDENTITY)
    );
    // The failed instance's clock did not advance; a later valid step works.
    assert_eq!(crowd.player(1).unwrap().clock(node), Some(0.0));
}
