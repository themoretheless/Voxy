use std::sync::Arc;

use glam::{Mat4, Quat, Vec2, Vec3};

use super::*;
use crate::{
    AnimationClip, BlendSample2D, Joint, JointTrack, Playback, QuatKey, Transform, Vec3Key,
};

/// hips(0) → [l_thigh(1) → l_shin(2) → l_foot(3)], [r_thigh(4) → r_shin(5) → r_foot(6)], spine(7).
fn biped() -> Skeleton {
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
        joint("hips", None, Vec3::new(0.0, 1.0, 0.0)),
        joint("l_thigh", Some(0), Vec3::new(-0.1, 0.0, 0.0)),
        joint("l_shin", Some(1), Vec3::new(0.0, -0.5, 0.0)),
        joint("l_foot", Some(2), Vec3::new(0.0, -0.5, 0.0)),
        joint("r_thigh", Some(0), Vec3::new(0.1, 0.0, 0.0)),
        joint("r_shin", Some(4), Vec3::new(0.0, -0.5, 0.0)),
        joint("r_foot", Some(5), Vec3::new(0.0, -0.5, 0.0)),
        joint("spine", Some(0), Vec3::new(0.0, 0.3, 0.0)),
    ])
    .unwrap()
}

/// A cycle where each thigh swings and the hips bob so feet alternately dip
/// to ground level (y = 0) and lift.
fn cycle(rig: &Skeleton, duration: f32, swing: f32) -> Arc<AnimationClip> {
    let mut tracks = vec![JointTrack::default(); 8];
    let samples = 16;
    let key = |i: usize| duration * i as f32 / samples as f32;
    for i in 0..=samples {
        let phase = i as f32 / samples as f32 * std::f32::consts::TAU;
        // Thighs rotate about x out of phase; hips bob slightly.
        tracks[1].rotations.push(QuatKey {
            time: key(i),
            value: Quat::from_rotation_x(swing * phase.sin()),
        });
        tracks[4].rotations.push(QuatKey {
            time: key(i),
            value: Quat::from_rotation_x(-swing * phase.sin()),
        });
        // Shins bend so the lifted leg clears the ground.
        tracks[2].rotations.push(QuatKey {
            time: key(i),
            value: Quat::from_rotation_x((swing * 1.5 * phase.cos()).max(0.0)),
        });
        tracks[5].rotations.push(QuatKey {
            time: key(i),
            value: Quat::from_rotation_x((-swing * 1.5 * phase.cos()).max(0.0)),
        });
        tracks[0].translations.push(Vec3Key {
            time: key(i),
            value: Vec3::new(0.0, 1.0 - 0.02 * (2.0 * phase).cos().abs(), 0.0),
        });
    }
    Arc::new(AnimationClip::new("cycle", duration, Playback::Loop, tracks, rig).unwrap())
}

fn controller(rig: &Skeleton, settings: GaitSettings) -> GaitController {
    let space = Arc::new(
        BlendSpace2D::new(
            vec![
                BlendSample2D {
                    parameter: Vec2::new(0.0, 1.0),
                    clip: cycle(rig, 1.0, 0.5),
                },
                BlendSample2D {
                    parameter: Vec2::new(0.0, 3.0),
                    clip: cycle(rig, 0.6, 0.8),
                },
                BlendSample2D {
                    parameter: Vec2::new(1.0, 2.0),
                    clip: cycle(rig, 0.8, 0.6),
                },
            ],
            rig,
        )
        .unwrap(),
    );
    let feet = GaitFeet {
        chains: [
            TwoBoneChain::new(rig, [1, 2, 3]).unwrap(),
            TwoBoneChain::new(rig, [4, 5, 6]).unwrap(),
        ],
        poles: [Vec3::new(0.0, 0.0, 1.0); 2],
        up: Vec3::Y,
    };
    GaitController::new(GaitSpace::planar(space), feet, settings).unwrap()
}

#[test]
fn planted_feet_stay_fixed_in_world_while_actor_moves() {
    let rig = biped();
    let mut gait = controller(&rig, GaitSettings::default());
    let ground = FlatGround {
        up: Vec3::Y,
        height: 0.0,
    };
    let velocity = Vec2::new(0.0, 1.5);
    let dt = 1.0 / 60.0;
    let mut actor = Vec3::ZERO;
    let mut world_locked: [Option<Vec3>; 2] = [None, None];
    let mut stance_ticks = 0;
    let mut any_swing = false;
    for _ in 0..240 {
        let delta = Vec3::new(0.0, 0.0, -velocity.y * dt);
        actor += delta;
        let frame = gait.advance(&rig, velocity, delta, dt, &ground).unwrap();
        let ankles = ankle_positions(&frame.pose, &rig, gait_feet(&gait)).unwrap();
        for foot in 0..2 {
            if frame.contacts[foot] {
                let world = ankles[foot] + actor;
                match world_locked[foot] {
                    Some(previous) => {
                        assert!(
                            (world - previous).length() < 1e-3,
                            "foot {foot} slid {:?} -> {:?}",
                            previous,
                            world
                        );
                        stance_ticks += 1;
                    }
                    None => world_locked[foot] = Some(world),
                }
                // Planted ankles sit on the ground within the contact height.
                assert!(ankles[foot].y <= GaitSettings::default().release_height + 1e-4);
            } else {
                world_locked[foot] = None;
                any_swing = true;
            }
        }
    }
    assert!(stance_ticks > 20, "{stance_ticks}");
    assert!(any_swing);
}

fn gait_feet(gait: &GaitController) -> &GaitFeet {
    &gait.feet
}

#[test]
fn lean_follows_acceleration_and_is_bounded() {
    let rig = biped();
    let mut gait = controller(
        &rig,
        GaitSettings {
            lean_joint: Some(7),
            lean_per_accel: 0.05,
            max_lean: 0.2,
            lean_smoothing: 0.0,
            ..GaitSettings::default()
        },
    );
    let ground = FlatGround {
        up: Vec3::Y,
        height: 0.0,
    };
    let dt = 0.05;
    let still = gait
        .advance(&rig, Vec2::new(0.0, 1.0), Vec3::ZERO, dt, &ground)
        .unwrap();
    assert_eq!(still.lean, 0.0);
    let burst = gait
        .advance(&rig, Vec2::new(0.0, 3.0), Vec3::ZERO, dt, &ground)
        .unwrap();
    // Acceleration 40 m/s² × 0.05 rad = 2 rad, clamped to 0.2.
    assert!((burst.lean - 0.2).abs() < 1e-6, "{}", burst.lean);
    let spine = burst.pose.local()[7].rotation;
    assert!(spine.angle_between(Quat::IDENTITY) > 0.19);
    let steady = gait
        .advance(&rig, Vec2::new(0.0, 3.0), Vec3::ZERO, dt, &ground)
        .unwrap();
    assert_eq!(steady.lean, 0.0);
}

#[test]
fn invalid_input_leaves_state_unchanged_and_settings_are_validated() {
    let rig = biped();
    let mut gait = controller(&rig, GaitSettings::default());
    let ground = FlatGround {
        up: Vec3::Y,
        height: 0.0,
    };
    gait.advance(&rig, Vec2::new(0.0, 2.0), Vec3::ZERO, 0.1, &ground)
        .unwrap();
    let phase = gait.phase();
    let locked = gait.locked();
    assert!(
        gait.advance(&rig, Vec2::NAN, Vec3::ZERO, 0.1, &ground)
            .is_err()
    );
    assert!(
        gait.advance(&rig, Vec2::ZERO, Vec3::ZERO, -0.1, &ground)
            .is_err()
    );
    assert_eq!(gait.phase(), phase);
    assert_eq!(gait.locked(), locked);
    let bad = GaitSettings {
        release_height: 0.0,
        contact_height: 0.1,
        ..GaitSettings::default()
    };
    let space = gait.player.clone();
    let _ = space;
    let feet = GaitFeet {
        chains: [
            TwoBoneChain::new(&rig, [1, 2, 3]).unwrap(),
            TwoBoneChain::new(&rig, [4, 5, 6]).unwrap(),
        ],
        poles: [Vec3::Z; 2],
        up: Vec3::Y,
    };
    let any_space = Arc::new(
        BlendSpace2D::new(
            vec![
                BlendSample2D {
                    parameter: Vec2::new(0.0, 1.0),
                    clip: cycle(&rig, 1.0, 0.5),
                },
                BlendSample2D {
                    parameter: Vec2::new(0.0, 3.0),
                    clip: cycle(&rig, 0.6, 0.8),
                },
                BlendSample2D {
                    parameter: Vec2::new(1.0, 2.0),
                    clip: cycle(&rig, 0.8, 0.6),
                },
            ],
            &rig,
        )
        .unwrap(),
    );
    assert!(GaitController::new(GaitSpace::planar(any_space), feet, bad).is_err());
}

#[test]
fn stride_scaling_retimes_clip_and_foot_excursion_to_requested_walk_and_run_speeds() {
    let rig = biped();
    let mut gait = controller(&rig, GaitSettings::default());
    let base_walk = cycle(&rig, 1.0, 0.45);
    let forward = Vec3::new(0.0, 0.0, -1.0);
    let authored_speed =
        measure_authored_stride_speed(&base_walk, &rig, gait_feet(&gait), forward, 32).unwrap();
    assert!(authored_speed > 0.2, "authored_speed={authored_speed}");

    let target_run_speed = authored_speed * 2.4;
    let (scaled_clip, report) = scale_clip_stride_to_speed(
        &base_walk,
        &rig,
        gait_feet(&gait),
        forward,
        target_run_speed,
        1.6,
        32,
    )
    .unwrap();
    assert!((report.target_speed_m_s - target_run_speed).abs() < 1e-5);
    assert!(report.stride_scale > 1.0);
    assert!(report.cadence_scale > 1.0);
    assert!(
        (report.stride_scale * report.cadence_scale - 2.4).abs() < 1e-4,
        "report={report:?}"
    );
    let achieved_speed =
        measure_authored_stride_speed(&scaled_clip, &rig, gait_feet(&gait), forward, 32).unwrap();
    assert!(
        (achieved_speed - target_run_speed).abs() / target_run_speed < 0.15,
        "achieved_speed={achieved_speed}, target={target_run_speed}"
    );

    assert_eq!(GaitRegime::from_speed(0.01), GaitRegime::Idle);
    assert_eq!(GaitRegime::from_speed(1.2), GaitRegime::Walk);
    assert_eq!(GaitRegime::from_speed(2.8), GaitRegime::Jog);
    assert_eq!(GaitRegime::from_speed(5.2), GaitRegime::Run);

    gait.set_stride_and_cadence_scale(report.stride_scale, report.cadence_scale)
        .unwrap();
    assert_eq!(
        gait.stride_and_cadence_scale(),
        (report.stride_scale, report.cadence_scale)
    );

    let flat = FlatGround {
        up: Vec3::Y,
        height: 0.0,
    };
    let frame0 = gait
        .advance(
            &rig,
            Vec2::new(0.0, 3.2),
            Vec3::new(0.0, 0.0, -0.05),
            1.0 / 60.0,
            &flat,
        )
        .unwrap();
    let frame1 = gait
        .advance(
            &rig,
            Vec2::new(0.0, 3.2),
            Vec3::new(0.0, 0.0, -0.05),
            1.0 / 60.0,
            &flat,
        )
        .unwrap();
    let coupled = crate::extract_pelvis_thigh_frame(
        &rig,
        gait_feet(&gait),
        &frame0.pose,
        &frame1.pose,
        0,
        [0.0; 3],
        0.065,
        1.0 / 60.0,
        gait.phase(),
        gait.regime(),
    )
    .unwrap();
    assert_eq!(coupled.regime, GaitRegime::Jog);
    assert!(coupled.pelvis_position_m.iter().all(|v| v.is_finite()));
    assert!(
        coupled.thighs[0]
            .knee_velocity_m_s
            .iter()
            .any(|v| v.abs() > 0.0)
    );
}
