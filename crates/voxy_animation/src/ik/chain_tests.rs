use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use crate::{
    FabrikSettings, IkChain, Joint, JointLimit, LookAtChain, Pose, Skeleton, Transform,
    TwoBoneChain, TwoBoneTarget, look_at_error, swing_twist_angles,
};

/// Vertical chain root(0) → a(1) → b(2) → c(3) → tip(4), links of 1, 1, 0.5, 0.5.
fn chain_rig() -> Skeleton {
    let joint = |name: &str, parent, y: f32| Joint {
        name: Arc::from(name),
        parent,
        bind_local: Transform {
            translation: Vec3::new(0.0, y, 0.0),
            ..Transform::IDENTITY
        },
        inverse_bind: Mat4::IDENTITY,
    };
    Skeleton::new(vec![
        joint("root", None, 0.0),
        joint("a", Some(0), 1.0),
        joint("b", Some(1), 1.0),
        joint("c", Some(2), 0.5),
        joint("tip", Some(3), 0.5),
    ])
    .unwrap()
}

fn tip_position(pose: &Pose, rig: &Skeleton, joint: usize) -> Vec3 {
    let mut global = Vec::new();
    let mut skin = Vec::new();
    pose.skin_matrices_into(rig, &mut global, &mut skin)
        .unwrap();
    global[joint].w_axis.truncate()
}

fn link_lengths(pose: &Pose, rig: &Skeleton, joints: &[usize]) -> Vec<f32> {
    joints
        .windows(2)
        .map(|w| (tip_position(pose, rig, w[1]) - tip_position(pose, rig, w[0])).length())
        .collect()
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((self.0 >> 40) as f32) / ((1u64 << 24) as f32)
    }
}

#[test]
fn fabrik_three_joint_tip_matches_two_bone_when_reachable() {
    let rig = chain_rig();
    let two = TwoBoneChain::new(&rig, [0, 1, 2]).unwrap();
    let chain = IkChain::new(&rig, 0, 2).unwrap();
    let pose = rig.bind_pose();
    let mut rng = Lcg(3);
    for _ in 0..50 {
        let target = Vec3::new(
            rng.next() * 1.6 - 0.8,
            rng.next() * 1.6 + 0.2,
            rng.next() * 1.6 - 0.8,
        );
        if target.length() > 1.95 || target.length() < 0.5 {
            continue;
        }
        let (_, two_result) = pose
            .solve_two_bone(
                &two,
                TwoBoneTarget {
                    position: target,
                    pole: Vec3::new(0.0, 0.0, 1.0),
                    rotation: None,
                    weight: 1.0,
                },
            )
            .unwrap();
        let (solved, result) = pose
            .solve_fabrik(
                &chain,
                target,
                None,
                FabrikSettings {
                    iterations: 32,
                    tolerance: 1e-6,
                    weight: 1.0,
                },
            )
            .unwrap();
        assert!(!result.reach_clamped);
        assert!(
            (result.tip_position - two_result.tip_position).length() < 1e-4,
            "{target:?}"
        );
        assert!(result.residual < 1e-4, "{target:?}: {result:?}");
        let lengths = link_lengths(&solved, &rig, chain.joints());
        assert!(
            lengths.iter().all(|l| (l - 1.0).abs() < 1e-5),
            "{lengths:?}"
        );
        assert_eq!(solved.local()[0].translation, pose.local()[0].translation);
    }
}

#[test]
fn fabrik_preserves_links_and_straightens_when_unreachable() {
    let rig = chain_rig();
    let chain = IkChain::new(&rig, 0, 4).unwrap();
    let pose = rig.bind_pose();
    let target = Vec3::new(10.0, 2.0, -3.0);
    let (solved, result) = pose
        .solve_fabrik(&chain, target, None, FabrikSettings::default())
        .unwrap();
    assert!(result.reach_clamped);
    let lengths = link_lengths(&solved, &rig, chain.joints());
    for (l, expected) in lengths.iter().zip([1.0, 1.0, 0.5, 0.5]) {
        assert!((l - expected).abs() < 1e-5, "{lengths:?}");
    }
    let dir = target.normalize();
    let tip = tip_position(&solved, &rig, 4);
    assert!((tip - dir * 3.0).length() < 1e-4, "{tip:?}");
    assert!((result.residual - (target.length() - 3.0)).abs() < 1e-4);
    // Weight zero leaves the pose untouched.
    let (same, _) = pose
        .solve_fabrik(
            &chain,
            target,
            None,
            FabrikSettings {
                weight: 0.0,
                ..FabrikSettings::default()
            },
        )
        .unwrap();
    assert_eq!(same.local(), pose.local());
}

#[test]
fn fabrik_tip_rotation_and_limits_are_honoured() {
    let rig = chain_rig();
    let limit = JointLimit::new(0.3, [-0.1, 0.1]).unwrap();
    let chain = IkChain::new(&rig, 0, 4)
        .unwrap()
        .with_limit(0, limit)
        .unwrap()
        .with_limit(1, limit)
        .unwrap()
        .with_limit(2, limit)
        .unwrap();
    let pose = rig.bind_pose();
    let mut rng = Lcg(11);
    for _ in 0..40 {
        let target = Vec3::new(
            rng.next() * 4.0 - 2.0,
            rng.next() * 3.0,
            rng.next() * 4.0 - 2.0,
        );
        let (solved, _) = pose
            .solve_fabrik(&chain, target, None, FabrikSettings::default())
            .unwrap();
        for joint in 0..3 {
            let (swing, twist) = swing_twist_angles(&solved, joint);
            assert!(swing <= 0.3 + 1e-6, "joint {joint}: swing {swing}");
            assert!(twist.abs() <= 0.1 + 1e-6, "joint {joint}: twist {twist}");
        }
    }
    let rotation = Quat::from_rotation_x(0.7);
    let (solved, _) = pose
        .solve_fabrik(
            &IkChain::new(&rig, 0, 4).unwrap(),
            Vec3::new(1.0, 1.5, 0.5),
            Some(rotation),
            FabrikSettings::default(),
        )
        .unwrap();
    let mut global = Vec::new();
    let mut skin = Vec::new();
    solved
        .skin_matrices_into(&rig, &mut global, &mut skin)
        .unwrap();
    let (_, actual, _) = global[4].to_scale_rotation_translation();
    assert!(actual.angle_between(rotation) < 1e-4);
    assert!(JointLimit::new(-1.0, [0.0, 0.0]).is_err());
    assert!(JointLimit::new(0.5, [0.2, -0.2]).is_err());
    assert!(IkChain::new(&rig, 2, 1).is_err());
    assert!(IkChain::new(&rig, 0, 9).is_err());
}

#[test]
fn look_at_aligns_head_and_respects_weight_and_limits() {
    let rig = chain_rig();
    // Forward is +Y along the chain; look from the spine (joints 1..=3).
    let chain = LookAtChain::new(&rig, &[1, 2, 3], &[0.3, 0.3, 0.4], Vec3::Y, Vec3::Z).unwrap();
    let pose = rig.bind_pose();
    let target = Vec3::new(3.0, 1.0, 2.0);
    let before = look_at_error(&pose, &chain, target).unwrap();
    let solved = pose.solve_look_at(&chain, target, 1.0).unwrap();
    let after = look_at_error(&solved, &chain, target).unwrap();
    assert!(before > 0.5);
    assert!(after < 1e-6, "{after}");
    let half = pose.solve_look_at(&chain, target, 0.5).unwrap();
    let mid = look_at_error(&half, &chain, target).unwrap();
    assert!(mid > after && mid < before);
    assert_eq!(
        pose.solve_look_at(&chain, target, 0.0).unwrap().local(),
        pose.local()
    );
    let limited = LookAtChain::new(&rig, &[1, 2, 3], &[0.3, 0.3, 0.4], Vec3::Y, Vec3::Z)
        .unwrap()
        .with_limit(2, JointLimit::new(0.1, [-0.05, 0.05]).unwrap())
        .unwrap();
    let solved = pose.solve_look_at(&limited, target, 1.0).unwrap();
    let (swing, twist) = swing_twist_angles(&solved, 3);
    assert!(swing <= 0.1 + 1e-6 && twist.abs() <= 0.05 + 1e-6);
    assert!(LookAtChain::new(&rig, &[2, 1], &[0.5, 0.5], Vec3::Y, Vec3::Z).is_err());
    assert!(LookAtChain::new(&rig, &[1, 2], &[0.8, 0.8], Vec3::Y, Vec3::Z).is_err());
    assert!(pose.solve_look_at(&chain, Vec3::NAN, 1.0).is_err());
}
