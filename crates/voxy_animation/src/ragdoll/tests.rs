use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use super::*;
use crate::{JointLimit, Skeleton, Transform};

/// root(0) → upper(1) → lower(2); link length 1.
fn chain_rig() -> Skeleton {
    let joint = |name: &str, parent, t: Vec3| Joint {
        name: Arc::from(name),
        parent,
        bind_local: Transform {
            translation: t,
            ..Transform::IDENTITY
        },
        inverse_bind: Mat4::IDENTITY,
    };
    Skeleton::new(vec![
        joint("root", None, Vec3::new(0.0, 3.0, 0.0)),
        joint("upper", Some(0), Vec3::new(0.0, -1.0, 0.0)),
        joint("lower", Some(1), Vec3::new(0.0, -1.0, 0.0)),
    ])
    .unwrap()
}

fn body(joint: u16) -> RagdollBody {
    RagdollBody {
        joint,
        half_extents: Vec3::new(0.1, 0.5, 0.1),
        mass: 1.0,
        // Body centre halfway down the link.
        offset: Transform {
            translation: Vec3::new(0.0, -0.5, 0.0),
            ..Transform::IDENTITY
        },
    }
}

fn chain_desc(limit: Option<JointLimit>) -> RagdollDesc {
    RagdollDesc {
        bodies: vec![body(0), body(1), body(2)],
        joints: vec![
            RagdollJoint {
                parent: 0,
                child: 1,
                anchor_parent: Vec3::new(0.0, -0.5, 0.0),
                anchor_child: Vec3::new(0.0, 0.5, 0.0),
                limit,
            },
            RagdollJoint {
                parent: 1,
                child: 2,
                anchor_parent: Vec3::new(0.0, -0.5, 0.0),
                anchor_child: Vec3::new(0.0, 0.5, 0.0),
                limit,
            },
        ],
    }
}

struct FreeSpace(DVec3);
impl RagdollEnvironment for FreeSpace {
    fn resolve(&self, _: DVec3, _: DVec3, _: DQuat) -> Option<ContactManifold> {
        None
    }
    fn gravity(&self) -> DVec3 {
        self.0
    }
}

fn anchor_gap(ragdoll: &Ragdoll, state: &RagdollState) -> f64 {
    ragdoll
        .desc
        .joints
        .iter()
        .map(|j| {
            let pa =
                state.positions[j.parent] + state.rotations[j.parent] * j.anchor_parent.as_dvec3();
            let pb =
                state.positions[j.child] + state.rotations[j.child] * j.anchor_child.as_dvec3();
            (pa - pb).length()
        })
        .fold(0.0, f64::max)
}

#[test]
fn single_body_free_fall_matches_symplectic_euler() {
    let rig = chain_rig();
    let ragdoll = Ragdoll::new(
        &rig,
        RagdollDesc {
            bodies: vec![body(0)],
            joints: vec![],
        },
    )
    .unwrap();
    let pose = rig.bind_pose();
    let mut state = ragdoll.state_from_pose(&pose, None, 0.0).unwrap();
    let start = state.positions[0].y;
    let env = FreeSpace(DVec3::new(0.0, -9.81, 0.0));
    let (dt, substeps) = (0.01, 4u8);
    let h = dt / f64::from(substeps);
    let steps = 100;
    for _ in 0..steps {
        ragdoll.step(&mut state, None, &env, dt, substeps).unwrap();
    }
    let n = f64::from(steps * i32::from(substeps));
    // Symplectic Euler: y = y0 - g h² n(n+1)/2.
    let expected = start - 9.81 * h * h * n * (n + 1.0) / 2.0;
    assert!(
        (state.positions[0].y - expected).abs() < 1e-9,
        "{} vs {expected}",
        state.positions[0].y
    );
    assert!((state.linear_velocities[0].y + 9.81 * h * n).abs() < 1e-9);
}

#[test]
fn chain_anchors_hold_and_round_trip_preserves_pose() {
    let rig = chain_rig();
    let ragdoll = Ragdoll::new(&rig, chain_desc(None)).unwrap();
    let mut pose = rig.bind_pose();
    pose.set_joint_rotation(1, Quat::from_rotation_z(0.4))
        .unwrap();
    pose.set_joint_rotation(2, Quat::from_rotation_x(-0.3))
        .unwrap();
    let state = ragdoll.state_from_pose(&pose, None, 0.0).unwrap();
    let back = ragdoll.pose_from_state(&state, &rig.bind_pose()).unwrap();
    for (a, b) in back.local().iter().zip(pose.local()) {
        assert!((a.translation - b.translation).length() < 1e-6);
        assert!(a.rotation.angle_between(b.rotation) < 1e-6);
    }
    let mut state = state;
    let env = FreeSpace(DVec3::new(0.0, -9.81, 0.0));
    let mut previous = state.clone();
    for _ in 0..120 {
        ragdoll.step(&mut state, None, &env, 1.0 / 60.0, 8).unwrap();
        assert!(anchor_gap(&ragdoll, &state) < 1e-6);
        previous = state.clone();
    }
    let _ = previous;
    // Determinism.
    let mut a = ragdoll.state_from_pose(&pose, None, 0.0).unwrap();
    let mut b = a.clone();
    for _ in 0..30 {
        ragdoll.step(&mut a, None, &env, 1.0 / 60.0, 4).unwrap();
        ragdoll.step(&mut b, None, &env, 1.0 / 60.0, 4).unwrap();
    }
    assert_eq!(a, b);
}

#[test]
fn limits_bound_relative_swing_under_gravity() {
    let rig = chain_rig();
    let limit = JointLimit::new(0.2, [-0.05, 0.05]).unwrap();
    let ragdoll = Ragdoll::new(&rig, chain_desc(Some(limit))).unwrap();
    let mut pose = rig.bind_pose();
    // Start tilted so gravity swings the chain sideways.
    pose.set_joint_rotation(0, Quat::from_rotation_z(0.8))
        .unwrap();
    let mut state = ragdoll.state_from_pose(&pose, None, 0.0).unwrap();
    let env = FreeSpace(DVec3::new(0.0, -9.81, 0.0));
    for _ in 0..120 {
        ragdoll.step(&mut state, None, &env, 1.0 / 60.0, 8).unwrap();
        for j in &ragdoll.desc.joints {
            let rel =
                (state.rotations[j.parent].conjugate() * state.rotations[j.child]).normalize();
            let (_, angle) = rel.to_axis_angle();
            let angle = if angle > std::f64::consts::PI {
                2.0 * std::f64::consts::PI - angle
            } else {
                angle
            };
            assert!(angle <= 0.2 + 0.05 + 2e-2, "relative angle {angle}");
        }
    }
}

#[test]
fn stiff_motors_reproduce_target_pose_and_mask_blends() {
    let rig = chain_rig();
    let ragdoll = Ragdoll::new(&rig, chain_desc(None)).unwrap();
    let mut target = rig.bind_pose();
    target
        .set_joint_rotation(1, Quat::from_rotation_x(0.6))
        .unwrap();
    target
        .set_joint_rotation(2, Quat::from_rotation_z(-0.4))
        .unwrap();
    let mut state = ragdoll
        .state_from_pose(&rig.bind_pose(), None, 0.0)
        .unwrap();
    let env = FreeSpace(DVec3::ZERO);
    let motor = MotorSettings {
        stiffness: 1.0,
        damping: 1.0,
    };
    let full = BoneMask::uniform(&rig, 1.0).unwrap();
    for _ in 0..30 {
        ragdoll
            .step(
                &mut state,
                Some((&target, &motor, &full)),
                &env,
                1.0 / 60.0,
                4,
            )
            .unwrap();
    }
    let solved = ragdoll.pose_from_state(&state, &rig.bind_pose()).unwrap();
    for joint in 1..3 {
        let err = solved.local()[joint]
            .rotation
            .angle_between(target.local()[joint].rotation);
        assert!(err < 1e-3, "joint {joint}: {err}");
    }
    assert!(
        anchor_gap(&ragdoll, &state) < 1e-6,
        "{}",
        anchor_gap(&ragdoll, &state)
    );
    // No external force: the chain's root stays put while motors act.
    assert!(
        state.positions[0].length() < 3.0 + 1e-3,
        "{:?}",
        state.positions[0]
    );
    let animated = rig.bind_pose();
    let zero = BoneMask::uniform(&rig, 0.0).unwrap();
    assert_eq!(
        powered_blend(&animated, &solved, &zero).unwrap().local(),
        animated.local()
    );
    let blended = powered_blend(&animated, &solved, &full).unwrap();
    for (a, b) in blended.local().iter().zip(solved.local()) {
        assert!((a.translation - b.translation).length() < 1e-6);
        assert!(a.rotation.angle_between(b.rotation) < 1e-6);
    }
}

#[test]
fn plane_contact_supports_a_resting_body_and_damps_it() {
    let rig = chain_rig();
    let ragdoll = Ragdoll::new(
        &rig,
        RagdollDesc {
            bodies: vec![body(0)],
            joints: vec![],
        },
    )
    .unwrap();
    let mut state = ragdoll
        .state_from_pose(&rig.bind_pose(), None, 0.0)
        .unwrap();
    let env = PlaneEnvironment {
        normal: DVec3::Y,
        offset: 0.0,
        gravity: DVec3::new(0.0, -9.81, 0.0),
    };
    for _ in 0..240 {
        ragdoll.step(&mut state, None, &env, 1.0 / 60.0, 4).unwrap();
        let lowest = env
            .resolve(
                state.positions[0],
                body(0).half_extents.as_dvec3(),
                state.rotations[0],
            )
            .map_or(0.0, |c| c.depth);
        assert!(lowest < 1e-3, "penetration {lowest}");
    }
    // Resting: centre at half height, negligible velocity.
    assert!(
        (state.positions[0].y - 0.5).abs() < 1e-3,
        "{}",
        state.positions[0].y
    );
    assert!(state.linear_velocities[0].length() < 1e-2);
}

#[test]
fn validation_rejects_bad_descriptions_and_steps() {
    let rig = chain_rig();
    assert!(Ragdoll::new(&rig, RagdollDesc::default()).is_err());
    assert!(
        Ragdoll::new(
            &rig,
            RagdollDesc {
                bodies: vec![body(0), body(0)],
                joints: vec![]
            }
        )
        .is_err()
    );
    assert!(
        Ragdoll::new(
            &rig,
            RagdollDesc {
                bodies: vec![body(0)],
                joints: vec![RagdollJoint {
                    parent: 0,
                    child: 3,
                    anchor_parent: Vec3::ZERO,
                    anchor_child: Vec3::ZERO,
                    limit: None
                }]
            }
        )
        .is_err()
    );
    let ragdoll = Ragdoll::new(&rig, chain_desc(None)).unwrap();
    let mut state = ragdoll
        .state_from_pose(&rig.bind_pose(), None, 0.0)
        .unwrap();
    let env = FreeSpace(DVec3::ZERO);
    assert!(ragdoll.step(&mut state, None, &env, -1.0, 4).is_err());
    assert!(ragdoll.step(&mut state, None, &env, 0.1, 0).is_err());
    state.positions.pop();
    assert!(ragdoll.step(&mut state, None, &env, 0.1, 1).is_err());
    assert!(
        ragdoll
            .state_from_pose(&rig.bind_pose(), Some(&rig.bind_pose()), 0.0)
            .is_err()
    );
}
