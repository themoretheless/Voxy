use super::*;
use crate::humanoid_locomotion::{LocomotionClip, VoxyHumanoidLocomotion};

#[test]
fn actual_body_accepts_quaternius_locomotion_profile_and_keeps_leg_lengths() {
    let body = voxy_render::ObjAsset::parse(
        include_str!(
            "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
        ),
        voxy_render::ObjLimits::default(),
    )
    .unwrap();
    let rig = FemaleRig::new(body.mesh.vertices()).unwrap();
    let source = std::sync::Arc::new(voxy_model::ModelAsset::parse(
        include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.gltf"),
        &[include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.bin")],voxy_model::ModelLimits::default()).unwrap());
    let bind = source
        .skeleton
        .bind_pose64()
        .global_matrices(&source.skeleton)
        .unwrap();
    let positions: Vec<_> = ["DEF-upper_arm.L", "DEF-forearm.L", "DEF-hand.L"]
        .map(|name| {
            bind[usize::from(source.resolve_joint_name(name).unwrap())]
                .w_axis
                .truncate()
        })
        .into_iter()
        .collect();
    eprintln!("LOCOMOTION source_left_arm_bind={positions:?}");
    let library = VoxyHumanoidLocomotion::new(source.clone(), &rig.skeleton).unwrap();
    assert!((0.1..10.).contains(&library.leg_ratio()));
    let rest = rig
        .skeleton
        .bind_pose64()
        .global_matrices(&rig.skeleton)
        .unwrap();
    for clip in [
        LocomotionClip::Idle,
        LocomotionClip::Walk,
        LocomotionClip::Jog,
        LocomotionClip::Sprint,
        LocomotionClip::JumpStart,
        LocomotionClip::JumpLoop,
        LocomotionClip::JumpLand,
    ] {
        assert!(library.duration(clip) > 0.);
        let mut maximum_motion = 0f32;
        for sample in 0..=120 {
            let pose = library.pose_phase(clip, sample as f64 / 120.).unwrap();
            let (in_place, pelvis_offset) = library
                .pose_phase_in_place(clip, sample as f64 / 120., &rig.skeleton)
                .unwrap();
            assert_eq!(
                in_place.local()[0].translation,
                rig.skeleton.joints()[0].bind_local.translation
            );
            assert_eq!(&in_place.local()[1..], &pose.local()[1..]);
            assert_eq!(in_place.local()[0].rotation, pose.local()[0].rotation);
            assert_eq!(in_place.local()[0].scale, pose.local()[0].scale);
            let in_place_palette = in_place.skin_matrices(&rig.skeleton).unwrap();
            for (joint, local) in pose.local().iter().enumerate() {
                assert_eq!(local.scale, rig.skeleton.joints()[joint].bind_local.scale);
            }
            let palette = pose.skin_matrices(&rig.skeleton).unwrap();
            assert!(palette.iter().all(|matrix| matrix.is_finite()));
            for (index, local) in pose.local().iter().enumerate().skip(1) {
                assert_eq!(
                    local.translation,
                    rig.skeleton.joints()[index].bind_local.translation
                );
            }
            for pair in [[7, 8], [8, 9], [13, 14], [14, 15]] {
                let [a, b] = pair.map(|joint| {
                    palette[joint].transform_point3(rest[joint].w_axis.truncate().as_vec3())
                });
                let expected = rest[pair[0]]
                    .w_axis
                    .truncate()
                    .distance(rest[pair[1]].w_axis.truncate())
                    as f32;
                assert!(
                    (a.distance(b) - expected).abs() < 1e-5,
                    "{clip:?} phase {sample} changed leg length"
                );
            }
            if sample % 30 == 0 {
                let in_place_transforms: Vec<_> = in_place_palette
                    .into_iter()
                    .map(crate::rig_skinning::RigidSkinTransform::from_matrix)
                    .collect();
                let transforms: Vec<_> = palette
                    .into_iter()
                    .map(crate::rig_skinning::RigidSkinTransform::from_matrix)
                    .collect();
                for (vertex, weights) in body.mesh.vertices().iter().zip(&rig.weights) {
                    let before = Vec3::from_array(vertex.position);
                    let after = crate::rig_skinning::deform_point(before, weights, &transforms);
                    assert!(after.is_finite());
                    let reconstructed =
                        crate::rig_skinning::deform_point(before, weights, &in_place_transforms)
                            + pelvis_offset;
                    assert!(
                        after.distance(reconstructed) < 2e-6,
                        "{clip:?} phase {sample}: translation extraction changed skinning"
                    );
                    maximum_motion = maximum_motion.max(after.distance(before));
                }
            }
        }
        assert!(
            maximum_motion > 0.01,
            "{clip:?} did not move actual body vertices"
        );
        eprintln!(
            "LOCOMOTION actual_body={clip:?} samples=121 vertices={} maximum_bind_displacement_m={maximum_motion}",
            body.mesh.vertices().len()
        );
    }
    // Inspect the actual retargeted mesh at authored jump boundaries before
    // wiring this library into the shared physical jump clock. Imported clips
    // need not have matching endpoint poses, even when named as one sequence.
    for (from, to) in [
        (LocomotionClip::Idle, LocomotionClip::JumpStart),
        (LocomotionClip::JumpStart, LocomotionClip::JumpLoop),
        (LocomotionClip::JumpLoop, LocomotionClip::JumpLand),
        (LocomotionClip::JumpLand, LocomotionClip::Idle),
    ] {
        let a = library.pose_phase(from, 1.).unwrap();
        let b = library.pose_phase(to, 0.).unwrap();
        let transforms = |pose: &Pose| {
            pose.skin_matrices(&rig.skeleton)
                .unwrap()
                .into_iter()
                .map(crate::rig_skinning::RigidSkinTransform::from_matrix)
                .collect::<Vec<_>>()
        };
        let ta = transforms(&a);
        let tb = transforms(&b);
        let mut max_gap = 0f32;
        for (vertex, weights) in body.mesh.vertices().iter().zip(&rig.weights) {
            let point = Vec3::from_array(vertex.position);
            max_gap = max_gap.max(
                crate::rig_skinning::deform_point(point, weights, &ta)
                    .distance(crate::rig_skinning::deform_point(point, weights, &tb)),
            );
        }
        let root_gap = a.local()[0].translation.distance(b.local()[0].translation);
        assert!(max_gap.is_finite() && root_gap.is_finite());
        eprintln!(
            "AUTHORED_JUMP_SEAM from={from:?} to={to:?} mesh_gap_m={max_gap} pelvis_gap_m={root_gap} from_duration_s={} to_duration_s={}",
            library.duration(from),
            library.duration(to)
        );
    }
    // Check every clock boundary from both sides, including cycle wrapping.
    // This checks pose continuity before contact IK, not physical contact quality.
    for event in [0., 0.18, 0.56, 0.96, 1.06, 1.50, 1.65, 2., 4., 6.] {
        let a = library.jump_pose(event - 1e-7, &rig.skeleton).unwrap();
        let b = library.jump_pose(event + 1e-7, &rig.skeleton).unwrap();
        for (a, b) in a.local().iter().zip(b.local()) {
            assert!(
                a.translation.distance(b.translation) < 2e-6,
                "translation discontinuity at {event}"
            );
            assert!(
                a.rotation.dot(b.rotation).abs() > 1. - 2e-6,
                "rotation discontinuity at {event}"
            );
        }
    }
    for frame in 0..=720 {
        let time = frame as f64 / 120.;
        let pose = library.jump_pose(time, &rig.skeleton).unwrap();
        assert_eq!(
            pose.local()[0].translation,
            rig.skeleton.joints()[0].bind_local.translation
        );
        assert!(
            pose.skin_matrices(&rig.skeleton)
                .unwrap()
                .iter()
                .all(|m| m.is_finite())
        );
    }
    let sole_vertices: Vec<_> = body
        .mesh
        .vertices()
        .iter()
        .zip(&rig.weights)
        .filter(|(v, weights)| {
            v.position[1] < -0.755
                && weights
                    .iter()
                    .any(|(joint, weight)| (*joint == 9 || *joint == 15) && *weight > 0.9999)
        })
        .collect();
    assert!(sole_vertices.len() > 100);
    let floor = sole_vertices
        .iter()
        .map(|(v, _)| v.position[1])
        .fold(f32::INFINITY, f32::min);
    for event in [0., 0.56, 1.06, 2., 6.] {
        for dt in [1e-3, 1e-4, 1e-5] {
            for resolved in [false, true] {
                let sample = |time| {
                    if resolved {
                        rig.authored_jump_pose(&library, time)
                    } else {
                        library.jump_pose(time, &rig.skeleton)
                    }
                };
                let a = sample(event - dt).unwrap();
                let b = sample(event).unwrap();
                let c = sample(event + dt).unwrap();
                let omega = |from: Quat, to: Quat| {
                    let mut delta = (to.as_dquat() * from.as_dquat().conjugate()).normalize();
                    if delta.w < 0. {
                        delta = -delta;
                    }
                    delta.to_scaled_axis() / dt
                };
                for joint in [0, 1, 7, 8, 13, 14] {
                    let left = omega(a.local()[joint].rotation, b.local()[joint].rotation);
                    let right = omega(b.local()[joint].rotation, c.local()[joint].rotation);
                    let mismatch = left.distance(right);
                    let root_left = (b.local()[0].translation.as_dvec3()
                        - a.local()[0].translation.as_dvec3())
                        / dt;
                    let root_right = (c.local()[0].translation.as_dvec3()
                        - b.local()[0].translation.as_dvec3())
                        / dt;
                    assert!(mismatch.is_finite());
                    if event == 0.56 || event == 1.06 {
                        assert!(
                            mismatch < 0.2,
                            "angular velocity seam: event={event} dt={dt} resolved={resolved} joint={joint} mismatch={mismatch}"
                        );
                        assert!(
                            root_left.distance(root_right) < 0.02,
                            "pelvis velocity seam: event={event} dt={dt} resolved={resolved}"
                        );
                    }

                    eprintln!(
                        "AUTHORED_JUMP_DERIVATIVE event_s={event} step_s={dt} resolved={resolved} joint={joint} left_rad_s={} right_rad_s={} jump_rad_s={mismatch} pelvis_velocity_jump_m_s={}",
                        left.length(),
                        right.length(),
                        root_left.distance(root_right)
                    );
                }
            }
        }
    }
    for frame in 0..=720 {
        let time = frame as f64 / 120.;
        let pose = rig.authored_jump_pose(&library, time).unwrap();
        let palette = pose.skin_matrices(&rig.skeleton).unwrap();
        let motion = crate::jump_motion::sample(time);
        let transforms: Vec<_> = palette
            .iter()
            .copied()
            .map(crate::rig_skinning::RigidSkinTransform::from_matrix)
            .collect();
        for &(vertex, weights) in &sole_vertices {
            let rest = Vec3::from_array(vertex.position);
            let world = crate::rig_skinning::deform_point(rest, weights, &transforms)
                + Vec3::Y * motion.height as f32;
            assert!(
                world.y >= floor - 3e-6,
                "sole penetrated support at {time}: {world:?}, floor={floor}"
            );
            if motion.flight.is_none() {
                assert!(
                    world.distance(rest) < 4e-6,
                    "sole slid at {time}: {world:?}"
                );
            }
        }
        for foot in [9, 15] {
            let rest = rig.skeleton.joints()[foot]
                .inverse_bind
                .inverse()
                .transform_point3(Vec3::ZERO);
            let world = palette[foot].transform_point3(rest) + Vec3::Y * motion.height as f32;
            if motion.flight.is_none() {
                assert!(
                    world.distance(rest) < 3e-6,
                    "authored foot slid at {time}: {world:?}"
                );
            } else {
                assert!(
                    world.y >= rest.y - 3e-6,
                    "authored airborne foot under floor at {time}: {world:?}"
                );
            }
        }
    }
    for event in [0., 0.56, 1.06, 1.65, 2., 4., 6.] {
        let a = rig.authored_jump_pose(&library, event - 1e-7).unwrap();
        let b = rig.authored_jump_pose(&library, event + 1e-7).unwrap();
        for (a, b) in a.local().iter().zip(b.local()) {
            assert!(
                a.translation.distance(b.translation) < 2e-5,
                "contact translation jump at {event}"
            );
            assert!(
                a.rotation.dot(b.rotation).abs() > 1. - 2e-6,
                "contact rotation jump at {event}"
            );
        }
    }
    eprintln!(
        "AUTHORED_JUMP_SOLES vertices={} samples=721 floor_y_m={floor} grounded_tolerance_m=0.000004 floor_tolerance_m=0.000003",
        sole_vertices.len()
    );
    assert!(library.jump_pose(f64::NAN, &rig.skeleton).is_err());
    assert!(library.pose_phase(LocomotionClip::Walk, f64::NAN).is_err());
    let mut invalid = rig.skeleton.joints().to_vec();
    invalid[7].parent = Some(2); // Same names, wrong pelvis/leg ownership.
    let invalid = Skeleton::new(invalid).unwrap();
    assert!(
        library
            .pose_phase_in_place(LocomotionClip::JumpStart, 0.5, &invalid)
            .is_err()
    );
    assert!(VoxyHumanoidLocomotion::new(source, &invalid).is_err());
}

#[test]
fn grounded_preview_drives_actual_body_and_head_from_the_same_authored_pose() {
    let body = voxy_render::ObjAsset::parse(
        include_str!(
            "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
        ),
        voxy_render::ObjLimits::default(),
    )
    .unwrap();
    let mut rig = FemaleRig::new(body.mesh.vertices()).unwrap();
    let source=std::sync::Arc::new(voxy_model::ModelAsset::parse(
        include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.gltf"),
        &[include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.bin")],voxy_model::ModelLimits::default()).unwrap());
    let library =
        std::sync::Arc::new(VoxyHumanoidLocomotion::new(source.clone(), &rig.skeleton).unwrap());
    for clip in [
        LocomotionClip::Walk,
        LocomotionClip::Jog,
        LocomotionClip::Sprint,
    ] {
        rig.install_authored_gait(library.clone(), clip).unwrap();
        assert!(rig.authored_jump_source().is_none());
        assert!(std::sync::Arc::ptr_eq(
            &rig.authored_gait_source().unwrap().0,
            &source
        ));
        let time = library.duration(clip) * 0.25;
        let expected = library
            .pose_phase_in_place(
                clip,
                f64::from(time) / f64::from(library.duration(clip)),
                &rig.skeleton,
            )
            .unwrap()
            .0;
        rig.publish_grounded_pose(expected.clone(), clip);
        assert_eq!(rig.pose(time).local(), expected.local());
        let palette = expected.skin_matrices(&rig.skeleton).unwrap();
        assert_eq!(rig.head_matrix(time), palette[3]);
        let mut vertices = body.mesh.vertices().to_vec();
        rig.deform(&mut vertices, time);
        assert!(
            vertices
                .iter()
                .all(|v| v.position.iter().all(|x| x.is_finite()))
        );
        assert!(vertices.iter().zip(body.mesh.vertices()).any(|(a, b)| {
            Vec3::from_array(a.position).distance(Vec3::from_array(b.position)) > 0.01
        }));
        let before = rig.pose(time);
        assert!(
            rig.install_authored_gait(library.clone(), LocomotionClip::JumpLoop)
                .is_err()
        );
        assert_eq!(
            rig.pose(time).local(),
            before.local(),
            "invalid selection must preserve the admitted gait"
        );
    }
}

#[test]
fn shared_grounded_playback_preserves_pose_across_interrupted_fades_and_errors() {
    use crate::humanoid_locomotion::GroundedPlayback;
    let body = voxy_render::ObjAsset::parse(
        include_str!(
            "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
        ),
        voxy_render::ObjLimits::default(),
    )
    .unwrap();
    let rig = FemaleRig::new(body.mesh.vertices()).unwrap();
    let source=std::sync::Arc::new(voxy_model::ModelAsset::parse(
        include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.gltf"),
        &[include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.bin")],voxy_model::ModelLimits::default()).unwrap());
    let library = VoxyHumanoidLocomotion::new(source.clone(), &rig.skeleton).unwrap();
    let mut scaled = body.mesh.vertices().to_vec();
    for vertex in &mut scaled {
        vertex.position = vertex.position.map(|x| x * 1.1);
    }
    let second = FemaleRig::new(&scaled).unwrap();
    let shell = VoxyHumanoidLocomotion::new(source, &second.skeleton).unwrap();
    let mut player = GroundedPlayback::new(&library, LocomotionClip::Walk).unwrap();
    player.advance(0.173).unwrap();
    let close = |a: &Pose, b: &Pose| {
        let a = a.skin_matrices(&rig.skeleton).unwrap();
        let b = b.skin_matrices(&rig.skeleton).unwrap();
        assert!(
            a.iter().zip(b).all(|(a, b)| a.abs_diff_eq(b, 1e-5)),
            "displayed pose jumped at a transition boundary"
        );
    };
    let before = player.pose_in_place(&library, &rig.skeleton).unwrap().0;
    let phase = player.normalized_phase();
    player.transition_to(LocomotionClip::Jog, 0.3).unwrap();
    assert!((player.normalized_phase() - phase).abs() < 1e-12);
    close(
        &before,
        &player.pose_in_place(&library, &rig.skeleton).unwrap().0,
    );
    player.advance(0.1).unwrap();
    let before = player.pose_in_place(&library, &rig.skeleton).unwrap().0;
    player.transition_to(LocomotionClip::Sprint, 0.2).unwrap();
    assert!(matches!(
        player.pose_blend_phases().source,
        Some(voxy_animation::PoseBlendSource::FrozenPose(_))
    ));
    close(
        &before,
        &player.pose_in_place(&library, &rig.skeleton).unwrap().0,
    );
    let phase = player.normalized_phase();
    player.transition_to(LocomotionClip::Sprint, 0.2).unwrap();
    assert!(
        player
            .transition_to(LocomotionClip::Sprint, f32::NAN)
            .is_err()
    );
    close(
        &before,
        &player.pose_in_place(&library, &rig.skeleton).unwrap().0,
    );
    player.pose_in_place(&shell, &second.skeleton).unwrap();
    assert_eq!(
        player.normalized_phase(),
        phase,
        "another consumer advanced the shared clock"
    );
    for (clip, duration) in [(LocomotionClip::Walk, -1.), (LocomotionClip::JumpLoop, 0.2)] {
        assert!(player.transition_to(clip, duration).is_err());
        close(
            &before,
            &player.pose_in_place(&library, &rig.skeleton).unwrap().0,
        );
        assert_eq!(player.selected(), LocomotionClip::Sprint);
        assert_eq!(player.normalized_phase(), phase);
    }
    assert!(player.advance(f64::NAN).is_err());
    assert_eq!(player.normalized_phase(), phase);
    close(
        &before,
        &player.pose_in_place(&library, &rig.skeleton).unwrap().0,
    );
    player.advance(0.5).unwrap();
    let (pose, _) = player.pose_in_place(&library, &rig.skeleton).unwrap();
    let expected = library
        .pose_phase_in_place(
            LocomotionClip::Sprint,
            player.normalized_phase(),
            &rig.skeleton,
        )
        .unwrap()
        .0;
    close(&pose, &expected);
}

/// Real authored poses must admit exact, explicitly requested double stance.
/// This is IK qualification, not automatic stance classification for swing legs.
#[test]
fn actual_authored_gaits_admit_support_relative_plants_without_changing_bone_metric() {
    use glam::{DQuat, DVec3};
    use voxy_animation::{FootPlant, RootRigidTransform, TwoBoneChain};
    let body = voxy_render::ObjAsset::parse(
        include_str!(
            "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
        ),
        voxy_render::ObjLimits::default(),
    )
    .unwrap();
    let rig = FemaleRig::new(body.mesh.vertices()).unwrap();
    let source = std::sync::Arc::new(voxy_model::ModelAsset::parse(
        include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.gltf"),
        &[include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.bin")],
        voxy_model::ModelLimits::default(),
    ).unwrap());
    let library = VoxyHumanoidLocomotion::new(source, &rig.skeleton).unwrap();
    let index = |name: &str| {
        rig.skeleton
            .joints()
            .iter()
            .position(|j| j.name.as_ref() == name)
            .unwrap()
    };
    let indices = ["left", "right"]
        .map(|side| ["thigh", "shin", "foot"].map(|part| index(&format!("{side}_{part}"))));
    let chains = indices.map(|j| TwoBoneChain::new(&rig.skeleton, j.map(|i| i as u16)).unwrap());
    let bind = rig
        .skeleton
        .bind_pose64()
        .global_matrices(&rig.skeleton)
        .unwrap();
    let support = RootRigidTransform {
        translation: DVec3::new(1e8, 0., -1e8),
        rotation: DQuat::IDENTITY,
    };
    let actor_at_capture = support;
    let pivot = DVec3::new(0., -0.02, 0.04);
    let ankles = indices.map(|j| bind[j[2]].w_axis.truncate() + DVec3::Y * 0.04);
    let plants = ankles.map(|p| {
        FootPlant::capture_pivot(
            RootRigidTransform {
                translation: actor_at_capture.translation + p,
                rotation: DQuat::IDENTITY,
            },
            pivot,
            support,
        )
        .unwrap()
    });
    let mut maximum_error = 0f64;
    for clip in [
        LocomotionClip::Idle,
        LocomotionClip::Walk,
        LocomotionClip::Jog,
        LocomotionClip::Sprint,
    ] {
        for sample in 0..=120 {
            let phase = f64::from(sample) / 120.;
            let pose = library
                .pose_phase_in_place(clip, phase, &rig.skeleton)
                .unwrap()
                .0;
            // Move the actor relative to the captured support, using the same
            // support-local authority instead of recapturing every frame.
            let actor = RootRigidTransform {
                translation: support.translation + DVec3::X * (0.03 * phase),
                rotation: DQuat::from_rotation_y(0.04 * phase),
            };
            let targets = std::array::from_fn(|side| {
                plants[side]
                    .target(
                        support,
                        actor,
                        Vec3::new(if side == 0 { -0.2 } else { 0.2 }, 0.5, 1.),
                        1.,
                    )
                    .unwrap()
            });
            let (solved, report) = pose
                .solve_planted_legs_with_extension_margin(&chains, targets, Vec3::Y, 0.35, 0.005)
                .unwrap_or_else(|e| panic!("{clip:?} phase {phase}: {e}"));
            let world = solved.skin_matrices(&rig.skeleton).unwrap();
            for side in 0..2 {
                let ankle = world[indices[side][2]]
                    .transform_point3(bind[indices[side][2]].w_axis.truncate().as_vec3());
                let expected = (actor.rotation.conjugate()
                    * ((support.translation - actor.translation) + ankles[side]))
                    .as_vec3();
                let error = f64::from(ankle.distance(expected));
                maximum_error = maximum_error.max(error);
                assert!(error < 3e-6, "{clip:?} phase {phase} side {side}: {error}m");
                assert!(!report.legs[side].reach_clamped);
                let sole_axis = actor.rotation
                    * world[indices[side][2]]
                        .transform_vector3(Vec3::Y)
                        .as_dvec3();
                assert!(sole_axis.normalize().distance(DVec3::Y) < 3e-6);
                let contact_in_actor = world[indices[side][2]].transform_point3(
                    bind[indices[side][2]].w_axis.truncate().as_vec3() + pivot.as_vec3(),
                );
                let contact_in_support = (actor.translation - support.translation)
                    + actor.rotation * contact_in_actor.as_dvec3();
                assert!(contact_in_support.distance(ankles[side] + pivot) < 3e-6);
                for pair in [
                    [indices[side][0], indices[side][1]],
                    [indices[side][1], indices[side][2]],
                ] {
                    let points = pair
                        .map(|i| world[i].transform_point3(bind[i].w_axis.truncate().as_vec3()));
                    let length = bind[pair[0]]
                        .w_axis
                        .truncate()
                        .distance(bind[pair[1]].w_axis.truncate());
                    assert!((f64::from(points[0].distance(points[1])) - length).abs() < 3e-6);
                }
            }
            for joint in 1..pose.local().len() {
                assert_eq!(
                    solved.local()[joint].translation,
                    pose.local()[joint].translation
                );
                assert_eq!(solved.local()[joint].scale, pose.local()[joint].scale);
            }
        }
    }
    eprintln!("REAL_GAIT_PLANTS poses=484 max_ankle_error_m={maximum_error}");
}

#[test]
fn prepared_quaternius_jog_closes_the_cycle_and_preserves_original_motion_and_other_clips() {
    use crate::humanoid_locomotion::prepare_quaternius_gait_source;
    let original = voxy_model::ModelAsset::parse(
        include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.gltf"),
        &[include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.bin")],
        voxy_model::ModelLimits::default(),
    ).unwrap();
    let prepared = prepare_quaternius_gait_source(original.clone()).unwrap();
    let index = original
        .animations
        .iter()
        .position(|c| c.name() == "Jog_Fwd_Loop")
        .unwrap();
    let joint = usize::from(original.resolve_joint_name("DEF-foot.L").unwrap());
    let globals = |asset: &voxy_model::ModelAsset, phase| {
        asset
            .sample_pose_phase64(Some(index), phase)
            .unwrap()
            .global_matrices(&asset.skeleton)
            .unwrap()
    };
    let raw_gap = globals(&original, 0.)[joint]
        .w_axis
        .distance(globals(&original, 1.)[joint].w_axis);
    let closed_gap = globals(&prepared, 0.)[joint]
        .w_axis
        .distance(globals(&prepared, 1.)[joint].w_axis);
    assert!(raw_gap > 0.02);
    assert!(closed_gap < 1e-12);
    let old_duration = f64::from(original.animations[index].duration());
    let new_duration = f64::from(prepared.animations[index].duration());
    assert!((new_duration - old_duration - 1. / 24.).abs() < 1e-7);
    for sample in 0..=240 {
        let phase = f64::from(sample) / 240.;
        let after = prepared.animations[index]
            .try_sample_time64(&prepared.skeleton, phase * old_duration)
            .unwrap()
            .global_matrices(&prepared.skeleton)
            .unwrap();
        for (a, b) in globals(&original, phase).iter().zip(after) {
            assert!(a.abs_diff_eq(b, 1e-11), "source motion changed at {phase}");
        }
    }
    for (i, (a, b)) in original
        .animations
        .iter()
        .zip(&prepared.animations)
        .enumerate()
    {
        assert_eq!(std::sync::Arc::ptr_eq(a, b), i != index);
    }
    let again = prepare_quaternius_gait_source(prepared.clone()).unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &again.animations[index],
        &prepared.animations[index]
    ));
    let body = voxy_render::ObjAsset::parse(
        include_str!(
            "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
        ),
        voxy_render::ObjLimits::default(),
    )
    .unwrap();
    let rig = FemaleRig::new(body.mesh.vertices()).unwrap();
    let library =
        VoxyHumanoidLocomotion::new(std::sync::Arc::new(prepared), &rig.skeleton).unwrap();
    let start = library
        .pose_phase_in_place(LocomotionClip::Jog, 0., &rig.skeleton)
        .unwrap()
        .0;
    let end = library
        .pose_phase_in_place(LocomotionClip::Jog, 1., &rig.skeleton)
        .unwrap()
        .0;
    assert_eq!(start.local(), end.local());
    eprintln!(
        "PREPARED_JOG raw_ankle_gap_m={raw_gap} prepared_ankle_gap_m={closed_gap} duration_s={new_duration} original_samples=241"
    );
}

#[test]
fn graph_locomotion_matches_grounded_playback_at_rest_and_blends_gaits_by_speed() {
    use crate::humanoid_locomotion::{
        GraphLocomotion, GroundedPlayback, JOG_SPEED_M_S, WALK_SPEED_M_S,
    };
    let body = voxy_render::ObjAsset::parse(
        include_str!(
            "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
        ),
        voxy_render::ObjLimits::default(),
    )
    .unwrap();
    let rig = FemaleRig::new(body.mesh.vertices()).unwrap();
    let source = std::sync::Arc::new(voxy_model::ModelAsset::parse(
        include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.gltf"),
        &[include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.bin")],voxy_model::ModelLimits::default()).unwrap());
    let library = VoxyHumanoidLocomotion::new(source.clone(), &rig.skeleton).unwrap();
    // Idle through the graph is the single-clip Animator path, bit for bit.
    let mut graph = GraphLocomotion::new(&library, 0.25).unwrap();
    let mut grounded = GroundedPlayback::new(&library, LocomotionClip::Idle).unwrap();
    for _ in 0..30 {
        graph.advance(1. / 60.).unwrap();
        grounded.advance(1. / 60.).unwrap();
        let (a, _) = graph.pose_in_place(&library, &rig.skeleton).unwrap();
        let (b, _) = grounded.pose_in_place(&library, &rig.skeleton).unwrap();
        assert_eq!(a.local(), b.local());
    }
    // Walking: after the crossfade the moving state plays the walk clip at
    // the shared gait phase.
    graph.set_speed(WALK_SPEED_M_S).unwrap();
    for _ in 0..40 {
        graph.advance(1. / 60.).unwrap();
    }
    let status = graph.status().unwrap();
    assert_eq!(status.fading_from, None);
    let phase = graph.gait_phase().unwrap();
    let (pose, _) = graph.pose_in_place(&library, &rig.skeleton).unwrap();
    let (walk, _) = library
        .pose_phase_in_place(LocomotionClip::Walk, phase, &rig.skeleton)
        .unwrap();
    for (a, b) in pose.local().iter().zip(walk.local()) {
        assert!((a.translation - b.translation).length() < 1e-5);
        assert!(a.rotation.angle_between(b.rotation) < 1e-4);
    }
    // Speeding up to a jog keeps the phase continuous and changes the pose.
    let before = graph.gait_phase().unwrap();
    graph.set_speed(JOG_SPEED_M_S).unwrap();
    graph.advance(1. / 60.).unwrap();
    let after = graph.gait_phase().unwrap();
    assert!((after - before).rem_euclid(1.) < 0.1, "{before} -> {after}");
    let (jog_pose, _) = graph.pose_in_place(&library, &rig.skeleton).unwrap();
    assert_ne!(jog_pose.local(), pose.local());
    // Stopping fades back to idle.
    graph.set_speed(0.).unwrap();
    graph.advance(1. / 60.).unwrap();
    assert!(graph.status().unwrap().fading_from.is_some());
    for _ in 0..40 {
        graph.advance(1. / 60.).unwrap();
    }
    assert_eq!(graph.status().unwrap().current, voxy_animation::StateId(0));
    assert!(graph.set_speed(f32::NAN).is_err());
}

#[test]
fn humanoid_profiles_reproduce_the_hand_authored_quaternius_mapping() {
    use voxy_animation::{BoneRole, HumanoidProfile, RetargetBinding, Side};
    let body = voxy_render::ObjAsset::parse(
        include_str!(
            "../../../assets/characters/blender-female/prepared/body-forehead-refined.obj"
        ),
        voxy_render::ObjLimits::default(),
    )
    .unwrap();
    let rig = FemaleRig::new(body.mesh.vertices()).unwrap();
    let source = std::sync::Arc::new(voxy_model::ModelAsset::parse(
        include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.gltf"),
        &[include_bytes!("../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.bin")],voxy_model::ModelLimits::default()).unwrap());
    let library = VoxyHumanoidLocomotion::new(source.clone(), &rig.skeleton).unwrap();
    let mut pairs: Vec<(BoneRole, String, String)> = vec![
        (BoneRole::Hips, "DEF-hips".into(), "pelvis".into()),
        (BoneRole::Spine, "DEF-spine.001".into(), "spine".into()),
        (BoneRole::Chest, "DEF-spine.003".into(), "chest".into()),
        (BoneRole::Head, "DEF-head".into(), "head".into()),
    ];
    for (side, suffix, prefix) in [(Side::Left, "L", "left"), (Side::Right, "R", "right")] {
        for (role, part) in [
            (BoneRole::UpperArm(side), "upper_arm"),
            (BoneRole::Forearm(side), "forearm"),
            (BoneRole::Hand(side), "hand"),
            (BoneRole::Thigh(side), "thigh"),
            (BoneRole::Shin(side), "shin"),
            (BoneRole::Foot(side), "foot"),
        ] {
            pairs.push((
                role,
                format!("DEF-{part}.{suffix}"),
                format!("{prefix}_{part}"),
            ));
        }
        for (finger, part) in ["thumb", "f_index", "f_middle", "f_ring", "f_pinky"]
            .into_iter()
            .enumerate()
        {
            for segment in 0..3 {
                pairs.push((
                    BoneRole::Custom(format!("{prefix}_finger_{finger}_{segment}").into()),
                    format!("DEF-{part}.{:02}.{suffix}", segment + 1),
                    format!("{prefix}_finger_{finger}_{segment}"),
                ));
            }
        }
    }
    // Authored glTF names resolve to the imported skeleton's joint names.
    let source_names: Vec<std::sync::Arc<str>> = pairs
        .iter()
        .map(|(_, s, _)| {
            source.skeleton.joints()[usize::from(source.resolve_joint_name(s).unwrap())]
                .name
                .clone()
        })
        .collect();
    let source_roles: Vec<(BoneRole, &str)> = pairs
        .iter()
        .zip(&source_names)
        .map(|((r, _, _), name)| (r.clone(), name.as_ref()))
        .collect();
    let target_roles: Vec<(BoneRole, &str)> = pairs
        .iter()
        .map(|(r, _, t)| (r.clone(), t.as_str()))
        .collect();
    let source_profile = HumanoidProfile::new(&source.skeleton, &source_roles).unwrap();
    let target_profile = HumanoidProfile::new(&rig.skeleton, &target_roles).unwrap();
    let binding = RetargetBinding::from_profiles(
        &source_profile,
        &target_profile,
        &source.skeleton,
        &rig.skeleton,
    )
    .unwrap();
    // The same authored reference pose and model-rotation compile as the preset.
    let index = |name: &str| {
        rig.skeleton
            .joints()
            .iter()
            .position(|j| j.name.as_ref() == name)
            .unwrap()
    };
    let mut reference = rig.skeleton.bind_pose();
    for (side, sign) in [("left", 1.), ("right", -1.)] {
        let upper = index(&format!("{side}_upper_arm"));
        let forearm = index(&format!("{side}_forearm"));
        let hand = index(&format!("{side}_hand"));
        let a = rig.skeleton.joints()[forearm].bind_local.translation;
        let b = rig.skeleton.joints()[hand].bind_local.translation;
        let theta_upper = (-a.y).atan2(sign * a.x);
        let theta_forearm = (-b.y).atan2(sign * b.x);
        let theta_hand = 0.12f32.atan2(0.02);
        for (joint, angle) in [
            (upper, theta_upper),
            (forearm, theta_forearm - theta_upper),
            (hand, theta_hand - theta_forearm),
        ] {
            reference
                .set_joint_rotation(joint, glam::Quat::from_rotation_z(sign * angle))
                .unwrap();
        }
    }
    let compiled = binding
        .with_target_reference_pose(&reference)
        .unwrap()
        .compile_model_rotations(glam::Quat::IDENTITY)
        .unwrap();
    for clip in [
        LocomotionClip::Idle,
        LocomotionClip::Walk,
        LocomotionClip::Sprint,
    ] {
        for sample in 0..=24 {
            let phase = f64::from(sample) / 24.;
            let expected = library.pose_phase(clip, phase).unwrap();
            let source_pose = source
                .sample_pose_phase(Some(library_clip_index(&source, clip)), phase)
                .unwrap();
            let mut actual = compiled.apply_pose(&source_pose).unwrap();
            for (joint, j) in rig.skeleton.joints().iter().enumerate() {
                actual.set_joint_scale(joint, j.bind_local.scale).unwrap();
            }
            assert_eq!(actual.local(), expected.local(), "{clip:?} phase {phase}");
        }
    }
}

fn library_clip_index(source: &voxy_model::ModelAsset, clip: LocomotionClip) -> usize {
    let name = match clip {
        LocomotionClip::Idle => "Idle_Loop",
        LocomotionClip::Walk => "Walk_Loop",
        LocomotionClip::Jog => "Jog_Fwd_Loop",
        LocomotionClip::Sprint => "Sprint_Loop",
        LocomotionClip::JumpStart => "Jump_Start",
        LocomotionClip::JumpLoop => "Jump_Loop",
        LocomotionClip::JumpLand => "Jump_Land",
    };
    source
        .animations
        .iter()
        .position(|c| c.name() == name)
        .unwrap()
}
