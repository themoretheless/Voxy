use super::*;
use glam::{Mat4, Vec3};
use std::{collections::BTreeMap, sync::Arc};
use voxy_assets::AssetId;
use voxy_gameplay::{BoxCollider, CharacterBody, CharacterPhysics, player_input};
use voxy_scene::{SceneGraph, Transform};
fn model_bytes() -> Vec<u8> {
    let original = gltf::binary::Glb::from_slice(include_bytes!(
        "../../../voxy_render/examples/assets/animated-triangle.glb"
    ))
    .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&original.json).unwrap();
    json["nodes"] = serde_json::json!([
        {"name":"hip","translation":[0.,0.5,0.],"children":[1,3]},
        {"name":"knee","translation":[0.25,-0.7,0.],"children":[2]},
        {"name":"foot","translation":[-0.25,-0.7,0.]},
        {"mesh":0,"skin":0}
    ]);
    json["skins"][0]["joints"] = serde_json::json!([0, 1, 2]);
    json["animations"] = serde_json::json!([]);
    let bytes = gltf::binary::Glb {
        header: original.header,
        json: serde_json::to_vec(&json).unwrap().into(),
        bin: original.bin,
    }
    .to_vec()
    .unwrap();
    bytes
}
fn model() -> Arc<ModelAsset> {
    Arc::new(ModelAsset::parse(&model_bytes(), &[], voxy_render::ModelLimits::default()).unwrap())
}
fn settings() -> ModelFootPlacement {
    ModelFootPlacement {
        feet: vec![FootBinding {
            bones: ["hip".into(), "knee".into(), "foot".into()],
            sole_offset: [0., -0.1, 0.],
            sole_up: [0., 1., 0.],
            pole: [1., 0., 0.],
            plant: true,
            weight: 1.,
            contact: FootContactSettings::default(),
            contact_curve: vec![], clip_contact_curves: Default::default(),
        }],
    }
}
fn sole(model: &ModelAsset, frame: &AnimatorFrame, actor: Mat4) -> Vec3 {
    let mut globals: Vec<Mat4> = Vec::new();
    for (local, joint) in frame.pose.local().iter().zip(model.skeleton.joints()) {
        globals.push(
            joint
                .parent
                .map_or(local.matrix(), |p| globals[usize::from(p)] * local.matrix()),
        );
    }
    actor.transform_point3(globals[2].transform_point3(Vec3::new(0., -0.1, 0.)))
}
#[test]
fn fixed_play_candidates_keep_sole_on_anchor_and_failures_preserve_frame_and_clock() {
    let model = model();
    let mut scene = SceneGraph::new(8);
    let owner = scene
        .spawn(
            None,
            Transform {
                translation: Vec3::Y,
                ..Default::default()
            },
        )
        .unwrap();
    let asset = AssetId("feet".into());
    scene
        .insert_component(
            owner,
            crate::ModelInstance {
                asset: asset.clone(),
            },
        )
        .unwrap();
    scene
        .insert_component(
            owner,
            crate::ModelAnimation {
                clip: None,
                ..Default::default()
            },
        )
        .unwrap();
    scene.insert_component(owner, settings()).unwrap();
    scene
        .insert_component(
            owner,
            CharacterBody {
                half_extents: [0.1, 1., 0.1],
                ..Default::default()
            },
        )
        .unwrap();
    let floor = scene
        .spawn(
            None,
            Transform {
                translation: -Vec3::Y * 0.1,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [4., 0.1, 4.],
            },
        )
        .unwrap();
    let models = BTreeMap::from([(asset, model.clone())]);
    let mut runtime = crate::animation_runtime::AnimationRuntime::default();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    for _ in 0..4 {
        let candidate = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
        let (_, corrected) = physics
            .fixed_step_with_preparation(
                &mut scene,
                &mut input,
                1. / 60.,
                &[(owner, Vec3::X * 0.03)],
                &[],
                |preview, budget| candidate.clone().correct_feet(preview, budget),
            )
            .unwrap();
        runtime = corrected;
        let point = sole(
            &model,
            &runtime.frame(owner, &model).unwrap(),
            scene.world_matrix(owner).unwrap(),
        );
        assert!(
            point.abs_diff_eq(Vec3::new(0.03, 0., 0.), 3e-6),
            "{point:?}"
        );
    }
    let frame = runtime.frame(owner, &model).unwrap();
    let serial = runtime.serial();
    let pose = scene.local(owner).unwrap();
    physics = physics.with_angular_trajectory_query_budget(1).unwrap();
    let second = scene
        .spawn(
            None,
            Transform {
                translation: Vec3::new(10., -0.1, 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            second,
            BoxCollider {
                half_extents: [1., 0.1, 1.],
            },
        )
        .unwrap();
    // Force fresh acquisition to scan two surfaces with one remaining query.
    scene
        .component_mut::<ModelFootPlacement>(owner)
        .unwrap()
        .unwrap()
        .feet[0]
        .contact
        .probe_lift += 0.01;
    let candidate = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
    assert!(
        physics
            .fixed_step_with_preparation(
                &mut scene,
                &mut input,
                1. / 60.,
                &[],
                &[],
                |preview, budget| candidate.clone().correct_feet(preview, budget)
            )
            .is_err()
    );
    assert_eq!(scene.local(owner).unwrap(), pose);
    assert_eq!(runtime.serial(), serial);
    assert!(Arc::ptr_eq(&frame, &runtime.frame(owner, &model).unwrap()));
    runtime.clear();
    assert!(runtime.frame(owner, &model).is_none());
    let mut registry = crate::model_registry().unwrap();
    // Durable authored settings contain names/settings, never live support IDs.
    let encoded = serde_json::to_string(&settings()).unwrap();
    assert_eq!(
        serde_json::from_str::<ModelFootPlacement>(&encoded).unwrap(),
        settings()
    );
    let _ = &mut registry;
}
#[test]
fn binding_rejects_missing_and_dependent_chains() {
    let model = model();
    let mut config = settings();
    config.feet.push(config.feet[0].clone());
    assert!(FootRuntime::new(&model, config).is_err());
    let mut config = settings();
    config.feet[0].bones[2] = "unknown".into();
    assert!(FootRuntime::new(&model, config).is_err());
}

#[test]
fn slope_normal_and_nonuniform_reflected_tip_keep_the_authored_sole_on_surface() {
    let mut model = (*model()).clone();
    let mut joints = model.skeleton.joints().to_vec();
    joints[0].bind_local.scale = Vec3::new(-1., 1., 1.);
    joints[2].bind_local.scale = Vec3::new(-1., 2., 1.);
    model.skeleton = voxy_animation::Skeleton::new(joints).unwrap();
    let mut scene = SceneGraph::new(2);
    let floor = scene
        .spawn(
            None,
            Transform {
                translation: -Vec3::Y * 0.1,
                rotation: glam::Quat::from_rotation_z(0.2),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [4., 0.1, 4.],
            },
        )
        .unwrap();
    let preview = CharacterTickPreview {
        characters: vec![],
        support: voxy_gameplay::SupportWorld::from_scene(&scene, 1).unwrap(),
    };
    let pose = model.skeleton.bind_pose();
    let frame = AnimatorFrame {
        skin_matrices: pose.skin_matrices(&model.skeleton).unwrap(),
        pose,
        root_motion: Vec3::new(0.3, 0., 0.),
        root_motion_joint: 0,
        transition_weight: 0.4,
    };
    let actor = Mat4::from_translation(Vec3::Y * 1.1);
    let mut runtime = FootRuntime::new(&model, settings()).unwrap();
    let solved = runtime
        .correct(
            &model,
            frame.clone(),
            actor.as_dmat4(),
            true,
            &preview,
            &mut SupportQueryBudget::new(100).unwrap(),
        )
        .unwrap();
    let contact = preview
        .support
        .resolve(
            runtime.feet[0].state.anchor().unwrap(),
            &mut SupportQueryBudget::new(100).unwrap(),
        )
        .unwrap()
        .unwrap();
    assert!(
        sole(&model, &solved, actor)
            .as_dvec3()
            .abs_diff_eq(contact.position, 4e-6)
    );
    let tip = solved.skin_matrices[2];
    let normal = tip
        .inverse()
        .transpose()
        .transform_vector3(Vec3::Y)
        .normalize();
    assert!(
        normal.as_dvec3().abs_diff_eq(contact.normal, 4e-6),
        "{normal:?} {:?}",
        contact.normal
    );
    assert_eq!(solved.root_motion, frame.root_motion);
    assert_eq!(solved.transition_weight, frame.transition_weight);
    for (a, b) in frame.pose.local().iter().zip(solved.pose.local()) {
        assert_eq!(a.translation, b.translation);
        assert_eq!(a.scale, b.scale);
    }
}

#[test]
fn ordinary_app_play_applies_foot_ik_and_stop_restores_authored_settings() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("voxy-foot-play-{}-{unique}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("foot.glb");
    std::fs::write(&path, model_bytes()).unwrap();
    let mut app = crate::App::new(&path, false).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.catalog.snapshot(&app.id).is_none() {
        app.tick().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    app.edit_key(winit::keyboard::KeyCode::KeyD).unwrap();
    let owner = app.instances[0];
    let floor = app.instances[1];
    app.scene
        .set_local(
            owner,
            Transform {
                translation: Vec3::Y,
                ..Default::default()
            },
        )
        .unwrap();
    app.scene
        .set_local(
            floor,
            Transform {
                translation: -Vec3::Y * 0.1,
                ..Default::default()
            },
        )
        .unwrap();
    app.scene
        .insert_component(
            owner,
            CharacterBody {
                half_extents: [0.1, 1., 0.1],
                speed: 1.8,
                ..Default::default()
            },
        )
        .unwrap();
    app.scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [4., 0.1, 4.],
            },
        )
        .unwrap();
    app.scene
        .insert_component(
            owner,
            crate::ModelAnimation {
                clip: None,
                ..Default::default()
            },
        )
        .unwrap();
    app.scene.insert_component(owner, settings()).unwrap();
    app.commit_authoring().unwrap();
    let authored = app.authoring_document().unwrap();
    let members = crate::component_fields::fields(&authored.objects[0]).unwrap();
    for (path, value) in [
        ("/feet/0/weight", "2"),
        ("/feet/0/sole_up/1", "0"),
        ("/feet/0/contact/release_distance", "0.01"),
        ("/feet/0/contact/probe_lift", "-1"),
        ("/feet/0/bones/2", "missing-foot"),
    ] {
        let field = members.iter().position(|field|
            field.schema == "editor.foot-placement.v1" && field.path == path).unwrap();
        assert!(app.edit_component_field(field, value).is_err(), "{path}");
        assert_eq!(app.authoring_document().unwrap(), authored);
        assert_eq!(app.authoring.history.as_ref().unwrap().current(), &authored);
    }
    app.toggle_play().unwrap();
    let owner = app.instances[0];
    let model = app
        .catalog
        .snapshot(&app.id)
        .unwrap()
        .value()
        .animated
        .as_ref()
        .unwrap()
        .clone();
    app.play
        .player_input
        .event(voxy_gameplay::RIGHT, 1.)
        .unwrap();
    for _ in 0..4 {
        app.advance_game(1. / 60.).unwrap();
        let frame = app.play.animations.frame(owner, &model).unwrap();
        let point = sole(&model, &frame, app.scene.world_matrix(owner).unwrap());
        assert!(
            point.abs_diff_eq(Vec3::new(0.03, 0., 0.), 4e-6),
            "{point:?}"
        );
    }
    assert!(app.scene.local(owner).unwrap().translation.x > 0.1);
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), authored);
    assert_eq!(app.play.animations.serial(), 0);
    app.stop_workers().unwrap();
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unreachable_retained_contact_releases_without_publishing_a_clamped_pose() {
    let model = model();
    let mut scene = SceneGraph::new(2);
    let floor = scene
        .spawn(
            None,
            Transform {
                translation: -Vec3::Y * 0.1,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [4., 0.1, 4.],
            },
        )
        .unwrap();
    let preview = CharacterTickPreview {
        characters: vec![],
        support: voxy_gameplay::SupportWorld::from_scene(&scene, 1).unwrap(),
    };
    let pose = model.skeleton.bind_pose();
    let frame = AnimatorFrame {
        skin_matrices: pose.skin_matrices(&model.skeleton).unwrap(),
        pose,
        root_motion: Vec3::ZERO,
        root_motion_joint: 0,
        transition_weight: 0.,
    };
    let mut config = settings();
    config.feet[0].contact.release_distance = 10.;
    let mut runtime = FootRuntime::new(&model, config).unwrap();
    runtime
        .correct(
            &model,
            frame.clone(),
            DMat4::from_translation(DVec3::Y),
            true,
            &preview,
            &mut SupportQueryBudget::new(100).unwrap(),
        )
        .unwrap();
    assert!(runtime.feet[0].state.anchor().is_some());
    let released = runtime
        .correct(
            &model,
            frame.clone(),
            DMat4::from_translation(DVec3::new(1., 1., 0.)),
            true,
            &preview,
            &mut SupportQueryBudget::new(100).unwrap(),
        )
        .unwrap();
    assert!(runtime.feet[0].state.anchor().is_none());
    assert_eq!(released.pose, frame.pose);
    let blocked = runtime
        .correct(
            &model,
            frame.clone(),
            DMat4::from_translation(DVec3::Y),
            true,
            &preview,
            &mut SupportQueryBudget::new(100).unwrap(),
        )
        .unwrap();
    assert!(runtime.feet[0].state.anchor().is_none());
    assert_eq!(blocked.pose, frame.pose);
}

#[test]
fn contact_curves_are_smooth_bounded_and_preserve_crossed_swing_events() {
    use voxy_animation::AnimationPhaseInterval;
    let keys = vec![
        FootContactKey {
            phase: 0.,
            weight: 1.,
        },
        FootContactKey {
            phase: 0.5,
            weight: 0.,
        },
        FootContactKey {
            phase: 1.,
            weight: 1.,
        },
    ];
    assert_eq!(contact_weight(&keys, Some(0.)).unwrap(), 1.);
    assert_eq!(contact_weight(&keys, Some(0.5)).unwrap(), 0.);
    assert_eq!(contact_weight(&keys, Some(0.25)).unwrap(), 0.5);
    assert!((contact_weight(&keys, Some(0.1)).unwrap() - 0.896).abs() < 1e-6);
    assert!(contact_weight(&keys, None).is_err());
    assert!(crossed_swing(
        &keys,
        Some(AnimationPhaseInterval {
            start: 0.1,
            end: 0.9,
            looping: true
        })
    ));
    assert!(crossed_swing(
        &keys,
        Some(AnimationPhaseInterval {
            start: 0.9,
            end: 4.1,
            looping: true
        })
    ));
    assert!(!crossed_swing(
        &keys,
        Some(AnimationPhaseInterval {
            start: 0.1,
            end: 0.1,
            looping: true
        })
    ));
    let mut invalid = settings();
    invalid.feet[0].contact_curve = keys.clone();
    invalid.feet[0].contact_curve[1].phase = 0.;
    assert!(FootRuntime::new(&model(), invalid).is_err());
}

#[test]
fn clip_phase_uses_playback_clock_speed_pause_loops_and_failed_publication() {
    use voxy_animation::{AnimationClip, Animator, JointTrack, Playback};
    let model = model();
    for playback in [Playback::Loop, Playback::Clamp] {
        let clip = Arc::new(
            AnimationClip::new(
                "phase",
                1.,
                playback,
                vec![JointTrack::default(); model.skeleton.joints().len()],
                &model.skeleton,
            )
            .unwrap(),
        );
        let mut animator = Animator::new(clip);
        animator.set_speed(2.).unwrap();
        let interval = animator.phase_interval(0.4).unwrap();
        assert!((interval.end - 0.8).abs() < 1e-7);
        animator.advance(&model.skeleton, 0.4).unwrap();
        assert!((animator.normalized_phase() - 0.8).abs() < 1e-7);
        animator.set_speed(0.).unwrap();
        animator.advance(&model.skeleton, 0.2).unwrap();
        assert!((animator.normalized_phase() - 0.8).abs() < 1e-7);
        animator.set_speed(2.).unwrap();
        animator.advance(&model.skeleton, 0.2).unwrap();
        let expected = if playback == Playback::Loop { 0.2 } else { 1. };
        assert!((animator.normalized_phase() - expected).abs() < 1e-7);
        let before = animator.normalized_phase();
        assert!(animator.advance(&model.skeleton, f32::NAN).is_err());
        assert_eq!(animator.normalized_phase(), before);
    }
    let mut model = (*model).clone();
    model.animations.push(Arc::new(
        AnimationClip::new(
            "phase",
            1.,
            Playback::Loop,
            vec![JointTrack::default(); model.skeleton.joints().len()],
            &model.skeleton,
        )
        .unwrap(),
    ));
    let mut playback = crate::model_playback::ModelPlayback::new(
        Arc::new(model),
        crate::ModelAnimation::default(),
    )
    .unwrap();
    assert!(
        playback
            .advance_with(0.3, |_, _| Err::<(), _>("palette rejected".into()))
            .is_err()
    );
    assert_eq!(playback.contact_phase(), Some(0.));
    assert!(playback.contact_interval().is_none());
    playback.advance_with(0.3, |_, _| Ok(())).unwrap();
    assert!((playback.contact_phase().unwrap() - 0.3).abs() < 1e-7);
}

#[test]
fn skipped_swing_rearms_at_new_sole_and_failed_tick_preserves_clip_phase_and_contacts() {
    use voxy_animation::{AnimationClip, JointTrack, Playback};
    let mut model = (*model()).clone();
    model.animations.push(Arc::new(
        AnimationClip::new(
            "stance",
            1.,
            Playback::Loop,
            vec![JointTrack::default(); model.skeleton.joints().len()],
            &model.skeleton,
        )
        .unwrap(),
    ));
    let model = Arc::new(model);
    let mut scene = SceneGraph::new(4);
    let owner = scene
        .spawn(
            None,
            Transform {
                translation: Vec3::Y,
                ..Default::default()
            },
        )
        .unwrap();
    let asset = AssetId("stance".into());
    scene
        .insert_component(
            owner,
            crate::ModelInstance {
                asset: asset.clone(),
            },
        )
        .unwrap();
    scene
        .insert_component(owner, crate::ModelAnimation::default())
        .unwrap();
    scene
        .insert_component(
            owner,
            CharacterBody {
                half_extents: [0.1, 1., 0.1],
                ..Default::default()
            },
        )
        .unwrap();
    let mut config = settings();
    config.feet[0].contact_curve = vec![
        FootContactKey {
            phase: 0.,
            weight: 1.,
        },
        FootContactKey {
            phase: 0.25,
            weight: 0.,
        },
        FootContactKey {
            phase: 0.75,
            weight: 1.,
        },
        FootContactKey {
            phase: 1.,
            weight: 1.,
        },
    ];
    let curve = std::mem::take(&mut config.feet[0].contact_curve);
    config.feet[0].clip_contact_curves.insert("stance".into(), curve);
    scene.insert_component(owner, config).unwrap();
    let floor = scene
        .spawn(
            None,
            Transform {
                translation: -Vec3::Y * 0.1,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [4., 0.1, 4.],
            },
        )
        .unwrap();
    let models = BTreeMap::from([(asset, model.clone())]);
    let mut runtime = crate::animation_runtime::AnimationRuntime::default();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    for _ in 0..2 {
        let next = runtime.prepare(&scene, &models, 0.1).unwrap();
        runtime = physics
            .fixed_step_with_preparation(
                &mut scene,
                &mut input,
                0.1,
                &[(owner, Vec3::X * 0.03)],
                &[],
                |preview, budget| next.clone().correct_feet(preview, budget),
            )
            .unwrap()
            .1;
    }
    let frame = runtime.frame(owner, &model).unwrap();
    let position = sole(&model, &frame, scene.world_matrix(owner).unwrap());
    assert!(position.x > 0.03 && position.x < 0.06);
    let extra = scene
        .spawn(
            None,
            Transform {
                translation: Vec3::new(10., -0.1, 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            extra,
            BoxCollider {
                half_extents: [1., 0.1, 1.],
            },
        )
        .unwrap();
    physics = physics.with_angular_trajectory_query_budget(1).unwrap();
    let next = runtime.prepare(&scene, &models, 0.1).unwrap();
    let before = scene.local(owner).unwrap();
    assert!(
        physics
            .fixed_step_with_preparation(
                &mut scene,
                &mut input,
                0.1,
                &[(owner, Vec3::X * 0.03)],
                &[],
                |preview, budget| next.clone().correct_feet(preview, budget)
            )
            .is_err()
    );
    assert_eq!(scene.local(owner).unwrap(), before);
    assert_eq!(runtime.serial(), 2);
    assert!(Arc::ptr_eq(&frame, &runtime.frame(owner, &model).unwrap()));
    physics = physics.with_angular_trajectory_query_budget(4096).unwrap();
    let next = runtime.prepare(&scene, &models, 0.1).unwrap();
    runtime = physics
        .fixed_step_with_preparation(
            &mut scene,
            &mut input,
            0.1,
            &[(owner, Vec3::X * 0.03)],
            &[],
            |preview, budget| next.clone().correct_feet(preview, budget),
        )
        .unwrap()
        .1;
    let point = sole(
        &model,
        &runtime.frame(owner, &model).unwrap(),
        scene.world_matrix(owner).unwrap(),
    );
    assert!(
        point.abs_diff_eq(Vec3::new(0.09, 0., 0.), 4e-6),
        "{point:?}"
    );
}

#[test]
fn named_clip_contacts_select_distinct_curves_and_reject_unmapped_clips() {
    let mut authored = settings();
    let foot = &mut authored.feet[0];
    foot.clip_contact_curves.insert("walk".into(), vec![
        FootContactKey { phase: 0., weight: 1. },
        FootContactKey { phase: 1., weight: 1. },
    ]);
    foot.clip_contact_curves.insert("jump".into(), vec![
        FootContactKey { phase: 0., weight: 0. },
        FootContactKey { phase: 1., weight: 0. },
    ]);
    assert_eq!(contact_weight(foot.contact_keys(Some("walk")).unwrap(), Some(0.5)).unwrap(), 1.);
    assert_eq!(contact_weight(foot.contact_keys(Some("jump")).unwrap(), Some(0.5)).unwrap(), 0.);
    assert!(foot.contact_keys(None).is_err());
    assert!(foot.contact_keys(Some("run")).is_err());
    authored.validate().unwrap();
    let encoded = serde_json::to_vec(&authored).unwrap();
    assert_eq!(serde_json::from_slice::<ModelFootPlacement>(&encoded).unwrap(), authored);
    assert!(FootRuntime::new(&model(), authored.clone()).is_err());
    let mut rig = (*model()).clone();
    for name in ["walk", "jump"] {
        rig.animations.push(Arc::new(voxy_animation::AnimationClip::new(
            name, 1., voxy_animation::Playback::Loop,
            vec![voxy_animation::JointTrack::default(); rig.skeleton.joints().len()],
            &rig.skeleton).unwrap()));
    }
    assert!(FootRuntime::new(&rig, authored.clone()).is_ok());
    rig.animations.push(rig.animations[0].clone());
    assert!(FootRuntime::new(&rig, authored.clone()).is_err());
    authored.feet[0].clip_contact_curves.get_mut("walk").unwrap()[1].phase = 0.;
    assert!(authored.validate().is_err());
}

#[test]
fn authoring_clip_switch_requires_contact_mapping_and_preserves_history_on_rejection() {
    let original = gltf::binary::Glb::from_slice(include_bytes!(
        "../../../voxy_render/examples/assets/foot-contact.glb")).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&original.json).unwrap();
    let mut clip = json["animations"][0].clone();
    json["animations"][0]["name"] = serde_json::json!("walk");
    clip["name"] = serde_json::json!("jump");
    json["animations"].as_array_mut().unwrap().push(clip);
    let bytes = gltf::binary::Glb { header: original.header,
        json: serde_json::to_vec(&json).unwrap().into(), bin: original.bin }.to_vec().unwrap();
    let root = std::env::temp_dir().join(format!("voxy-foot-clip-admission-{}-{}",
        std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("foot.glb");
    std::fs::write(&path, bytes).unwrap();
    let mut app = crate::App::new(&path, false).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.catalog.snapshot(&app.id).is_none() {
        app.tick().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let owner = app.instances[0];
    app.scene.insert_component(owner, CharacterBody::default()).unwrap();
    app.scene.insert_component(owner, crate::ModelAnimation::default()).unwrap();
    let mut contact = settings();
    contact.feet[0].clip_contact_curves.insert("walk".into(), vec![
        FootContactKey { phase: 0., weight: 1. }, FootContactKey { phase: 1., weight: 1. }]);
    app.scene.insert_component(owner, contact.clone()).unwrap();
    app.commit_authoring().unwrap();
    let before = app.authoring_document().unwrap();
    app.validate_authoring_document(&before).unwrap();
    let fields = crate::component_fields::fields(&before.objects[0]).unwrap();
    let index = fields.iter().position(|f| f.schema == "editor.model-animation.v1" && f.path == "/clip").unwrap();
    assert!(app.edit_component_field(index, "1").is_err());
    assert_eq!(app.authoring_document().unwrap(), before);
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &before);
    contact.feet[0].clip_contact_curves.insert("jump".into(), vec![
        FootContactKey { phase: 0., weight: 0. }, FootContactKey { phase: 1., weight: 0. }]);
    app.scene.insert_component(owner, contact).unwrap();
    app.commit_authoring().unwrap();
    let after_mapping = app.authoring_document().unwrap();
    let fields = crate::component_fields::fields(&after_mapping.objects[0]).unwrap();
    let index = fields.iter().position(|f| f.schema == "editor.model-animation.v1" && f.path == "/clip").unwrap();
    app.edit_component_field(index, "1").unwrap();
    assert_eq!(app.scene.component::<crate::ModelAnimation>(app.instances[0]).unwrap().unwrap().clip, Some(1));
    app.stop_workers().unwrap();
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn contact_key_budget_is_shared_between_named_clips_and_legacy_curve() {
    fn keys(count: usize) -> Vec<FootContactKey> {
        (0..count).map(|index| FootContactKey {
            phase: index as f32 / (count - 1) as f32,
            weight: 1.,
        }).collect()
    }
    let mut authored = settings();
    authored.feet[0].clip_contact_curves.insert("walk".into(), keys(2048));
    authored.feet[0].clip_contact_curves.insert("jump".into(), keys(2048));
    authored.validate().unwrap();
    authored.feet[0].clip_contact_curves.insert("jump".into(), keys(2049));
    assert_eq!(authored.validate().unwrap_err(), "aggregate foot contact key budget exceeded");
    authored.feet[0].clip_contact_curves.insert("jump".into(), keys(2048));
    authored.feet[0].contact_curve = keys(2);
    assert_eq!(authored.validate().unwrap_err(), "aggregate foot contact key budget exceeded");
    authored.feet[0].clip_contact_curves.insert("walk".into(), keys(2046));
    authored.validate().unwrap();
}

#[test]
fn contact_blending_keeps_displayed_weight_across_repeated_interruptions_and_staging() {
    let rig = model();
    let clip = |name| Arc::new(voxy_animation::AnimationClip::new(name, 1., voxy_animation::Playback::Loop,
        vec![voxy_animation::JointTrack::default(); rig.skeleton.joints().len()], &rig.skeleton).unwrap());
    let mut config = settings();
    let binding = &mut config.feet[0];
    for (name, weight) in [("walk", 1.), ("jump", 0.), ("land", 1.)] {
        binding.clip_contact_curves.insert(name.into(), vec![
            FootContactKey { phase: 0., weight }, FootContactKey { phase: 1., weight }]);
    }
    let mut animator = voxy_animation::Animator::new(clip("walk"));
    let mut state = ContactBlendState::default();
    let sample = |state: &mut ContactBlendState, animator: &voxy_animation::Animator| {
        let phases = animator.pose_blend_phases();
        state.sample(binding, Some(phases.target.clip.name()), Some(phases.target.normalized_phase), Some(phases)).unwrap()
    };
    assert_eq!(sample(&mut state, &animator), 1.);
    animator.transition_to(clip("jump"), 0.5).unwrap();
    animator.advance(&rig.skeleton, 0.1).unwrap();
    assert!((sample(&mut state, &animator) - 0.8).abs() < 1e-6);
    animator.advance(&rig.skeleton, 0.1).unwrap();
    assert!((sample(&mut state, &animator) - 0.6).abs() < 1e-6);
    animator.transition_to(clip("land"), 0.5).unwrap();
    assert!((sample(&mut state, &animator) - 0.6).abs() < 1e-6);
    animator.advance(&rig.skeleton, 0.1).unwrap();
    assert!((sample(&mut state, &animator) - 0.68).abs() < 1e-6);
    animator.advance(&rig.skeleton, 0.1).unwrap();
    assert!((sample(&mut state, &animator) - 0.76).abs() < 1e-6);
    animator.transition_to(clip("jump"), 0.5).unwrap();
    animator.advance(&rig.skeleton, 0.1).unwrap();
    let accepted = state.last_weight;
    let mut staged = state.clone();
    assert!((sample(&mut staged, &animator) - 0.608).abs() < 1e-6);
    assert_eq!(state.last_weight, accepted);
    assert!((sample(&mut state, &animator) - 0.608).abs() < 1e-6);
    assert!(ContactBlendState::default().sample(binding, Some("jump"), Some(0.1), Some(animator.pose_blend_phases())).is_err());
}

#[test]
fn physical_clip_fades_retain_the_planted_anchor_and_retry_frozen_completion_atomically() {
    let mut asset = (*model()).clone();
    for name in ["walk", "run"] {
        asset.animations.push(Arc::new(voxy_animation::AnimationClip::new(name, 1.,
            voxy_animation::Playback::Loop, vec![voxy_animation::JointTrack::default(); asset.skeleton.joints().len()],
            &asset.skeleton).unwrap()));
    }
    let model = Arc::new(asset);
    let mut scene = SceneGraph::new(4);
    let owner = scene.spawn(None, Transform { translation: Vec3::Y, ..Default::default() }).unwrap();
    let id = AssetId("fading-feet".into());
    scene.insert_component(owner, crate::ModelInstance { asset: id.clone() }).unwrap();
    scene.insert_component(owner, crate::ModelAnimation::default()).unwrap();
    let mut feet = settings();
    // Explicit curves incur event work even while the existing anchor remains valid.
    // Two distant colliders alone do not force a probe of an already planted foot.
    feet.feet[0].contact_curve = vec![
        FootContactKey { phase: 0., weight: 1. },
        FootContactKey { phase: 1., weight: 1. },
    ];
    scene.insert_component(owner, feet).unwrap();
    scene.insert_component(owner, CharacterBody { half_extents: [0.1, 1., 0.1], ..Default::default() }).unwrap();
    for x in [0., 10.] {
        let floor = scene.spawn(None, Transform { translation: Vec3::new(x, -0.1, 0.), ..Default::default() }).unwrap();
        scene.insert_component(floor, BoxCollider { half_extents: [4., 0.1, 4.] }).unwrap();
    }
    let models = BTreeMap::from([(id, model.clone())]);
    let mut runtime = crate::animation_runtime::AnimationRuntime::default();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    let tick = |runtime: &crate::animation_runtime::AnimationRuntime, physics: &mut CharacterPhysics,
        scene: &mut SceneGraph, input: &mut voxy_input::InputMap| {
        let candidate = runtime.prepare(scene, &models, 1./60.).unwrap();
        physics.fixed_step_with_preparation(scene, input, 1./60., &[(owner, Vec3::X*0.02)], &[],
            |preview, budget| candidate.clone().correct_feet(preview, budget)).map(|(_, frame)| frame)
    };
    runtime = tick(&runtime, &mut physics, &mut scene, &mut input).unwrap();
    scene.insert_component(owner, crate::ModelAnimation { clip: Some(1), transition_seconds: 0.05, ..Default::default() }).unwrap();
    for _ in 0..2 {
        runtime = tick(&runtime, &mut physics, &mut scene, &mut input).unwrap();
        assert!(sole(&model, &runtime.frame(owner, &model).unwrap(), scene.world_matrix(owner).unwrap())
            .abs_diff_eq(Vec3::new(0.02,0.,0.), 3e-6));
    }
    scene.insert_component(owner, crate::ModelAnimation { clip: Some(0), transition_seconds: 0.01, ..Default::default() }).unwrap();
    let frame = runtime.frame(owner, &model).unwrap();
    let serial = runtime.serial();
    let position = scene.local(owner).unwrap();
    physics = physics.with_angular_trajectory_query_budget(1).unwrap();
    let error = tick(&runtime, &mut physics, &mut scene, &mut input).unwrap_err();
    assert!(format!("{error:?}").contains("contact event budget exceeded"));
    assert!(Arc::ptr_eq(&frame, &runtime.frame(owner, &model).unwrap()));
    assert_eq!(runtime.serial(), serial);
    assert_eq!(scene.local(owner).unwrap(), position);
    physics = physics.with_angular_trajectory_query_budget(65536).unwrap();
    runtime = tick(&runtime, &mut physics, &mut scene, &mut input).unwrap();
    assert_eq!(runtime.frame(owner, &model).unwrap().transition_weight, 1.);
    assert!(sole(&model, &runtime.frame(owner, &model).unwrap(), scene.world_matrix(owner).unwrap())
        .abs_diff_eq(Vec3::new(0.02,0.,0.), 3e-6));
}

#[test]
fn physical_mixed_contact_windows_release_only_when_both_clips_are_in_swing() {
    for shared_swing in [false, true] {
        let mut asset = (*model()).clone();
        for name in ["source", "target"] {
            asset.animations.push(Arc::new(voxy_animation::AnimationClip::new(name, 1.,
                voxy_animation::Playback::Loop, vec![voxy_animation::JointTrack::default(); asset.skeleton.joints().len()],
                &asset.skeleton).unwrap()));
        }
        let model = Arc::new(asset);
        let mut scene = SceneGraph::new(3);
        let owner = scene.spawn(None, Transform { translation: Vec3::Y, ..Default::default() }).unwrap();
        let id = AssetId("mixed-contact".into());
        scene.insert_component(owner, crate::ModelInstance { asset: id.clone() }).unwrap();
        scene.insert_component(owner, crate::ModelAnimation::default()).unwrap();
        scene.insert_component(owner, CharacterBody { half_extents: [0.1, 1., 0.1], ..Default::default() }).unwrap();
        let mut feet = settings();
        let keys = |pairs: &[(f32, f32)]| pairs.iter().map(|&(phase, weight)| FootContactKey { phase, weight }).collect();
        feet.feet[0].clip_contact_curves.insert("source".into(), if shared_swing {
            keys(&[(0.,1.),(0.25,0.),(0.35,0.),(0.4,1.),(1.,1.)])
        } else { keys(&[(0.,1.),(1.,1.)]) });
        feet.feet[0].clip_contact_curves.insert("target".into(),
            keys(&[(0.,1.),(0.15,0.),(0.25,0.),(0.3,1.),(1.,1.)]));
        scene.insert_component(owner, feet).unwrap();
        let floor = scene.spawn(None, Transform { translation: -Vec3::Y*0.1, ..Default::default() }).unwrap();
        scene.insert_component(floor, BoxCollider { half_extents: [4.,0.1,4.] }).unwrap();
        let models = BTreeMap::from([(id, model.clone())]);
        let mut runtime = crate::animation_runtime::AnimationRuntime::default();
        let mut physics = CharacterPhysics::new(&scene,3,3);
        let mut input = player_input().unwrap();
        for step in 0..7 {
            if step == 1 {
                scene.insert_component(owner, crate::ModelAnimation { clip: Some(1), transition_seconds: 0.5, ..Default::default() }).unwrap();
            }
            let candidate = runtime.prepare(&scene,&models,0.1).unwrap();
            runtime = physics.fixed_step_with_preparation(&mut scene,&mut input,0.1,
                &[(owner,Vec3::X*0.02)],&[],|preview,budget| candidate.clone().correct_feet(preview,budget)).unwrap().1;
        }
        let final_sole = sole(&model,&runtime.frame(owner,&model).unwrap(),scene.world_matrix(owner).unwrap());
        // The target-only zero window must retain the original anchor. Coincident
        // source/target swing releases it and the following stance acquires x=.08.
        let expected_x = if shared_swing { 0.08 } else { 0.02 };
        assert!(final_sole.abs_diff_eq(Vec3::new(expected_x,0.,0.),3e-6),
            "shared_swing={shared_swing}, sole={final_sole:?}");
    }
}

#[test]
fn interrupted_variable_contact_fade_retains_anchor_and_accepted_snapshot_on_retry() {
    let mut asset = (*model()).clone();
    for name in ["stance", "air", "step", "void"] {
        asset.animations.push(Arc::new(voxy_animation::AnimationClip::new(name,1.,voxy_animation::Playback::Loop,
            vec![voxy_animation::JointTrack::default();asset.skeleton.joints().len()],&asset.skeleton).unwrap()));
    }
    let model = Arc::new(asset);
    let mut scene = SceneGraph::new(3);
    let owner = scene.spawn(None,Transform { translation: Vec3::Y,..Default::default() }).unwrap();
    let id = AssetId("interrupted-contacts".into());
    scene.insert_component(owner,crate::ModelInstance { asset:id.clone() }).unwrap();
    scene.insert_component(owner,crate::ModelAnimation::default()).unwrap();
    scene.insert_component(owner,CharacterBody { half_extents:[0.1,1.,0.1],..Default::default() }).unwrap();
    let keys = |pairs: &[(f32,f32)]| pairs.iter().map(|&(phase,weight)| FootContactKey {phase,weight}).collect();
    let mut feet = settings();
    feet.feet[0].clip_contact_curves.insert("stance".into(),keys(&[(0.,1.),(1.,1.)]));
    feet.feet[0].clip_contact_curves.insert("air".into(),keys(&[(0.,0.),(1.,0.)]));
    feet.feet[0].clip_contact_curves.insert("step".into(),keys(&[(0.,1.),(0.15,0.),(0.25,0.),(0.3,1.),(1.,1.)]));
    feet.feet[0].clip_contact_curves.insert("void".into(),keys(&[(0.,0.),(1.,0.)]));
    scene.insert_component(owner,feet).unwrap();
    let floor = scene.spawn(None,Transform {translation:-Vec3::Y*0.1,..Default::default()}).unwrap();
    scene.insert_component(floor,BoxCollider {half_extents:[4.,0.1,4.]}).unwrap();
    let models = BTreeMap::from([(id,model.clone())]);
    let mut runtime = crate::animation_runtime::AnimationRuntime::default();
    let mut physics = CharacterPhysics::new(&scene,3,3);
    let mut input = player_input().unwrap();
    let tick = |runtime: &crate::animation_runtime::AnimationRuntime,physics: &mut CharacterPhysics,
        scene: &mut SceneGraph,input: &mut voxy_input::InputMap| {
        let candidate = runtime.prepare(scene,&models,0.1).unwrap();
        physics.fixed_step_with_preparation(scene,input,0.1,&[(owner,Vec3::X*0.02)],&[],
            |preview,budget|candidate.clone().correct_feet(preview,budget)).map(|(_,candidate)|candidate)
    };
    runtime = tick(&runtime,&mut physics,&mut scene,&mut input).unwrap();
    scene.insert_component(owner,crate::ModelAnimation {clip:Some(1),transition_seconds:0.5,..Default::default()}).unwrap();
    runtime = tick(&runtime,&mut physics,&mut scene,&mut input).unwrap();
    scene.insert_component(owner,crate::ModelAnimation {clip:Some(2),transition_seconds:0.5,..Default::default()}).unwrap();
    let accepted = runtime.frame(owner,&model).unwrap();
    let position = scene.local(owner).unwrap();
    let serial = runtime.serial();
    physics = physics.with_angular_trajectory_query_budget(1).unwrap();
    let error = tick(&runtime,&mut physics,&mut scene,&mut input).unwrap_err();
    assert!(format!("{error:?}").contains("contact event budget exceeded"));
    assert!(Arc::ptr_eq(&accepted,&runtime.frame(owner,&model).unwrap()));
    assert_eq!(scene.local(owner).unwrap(),position);
    assert_eq!(runtime.serial(),serial);
    physics = physics.with_angular_trajectory_query_budget(65536).unwrap();
    // Interrupt again while the displayed contact is partial, then return to step.
    for _ in 0..2 { runtime = tick(&runtime,&mut physics,&mut scene,&mut input).unwrap(); }
    scene.insert_component(owner,crate::ModelAnimation {clip:Some(1),transition_seconds:0.5,..Default::default()}).unwrap();
    runtime = tick(&runtime,&mut physics,&mut scene,&mut input).unwrap();
    scene.insert_component(owner,crate::ModelAnimation {clip:Some(2),transition_seconds:0.5,..Default::default()}).unwrap();
    for _ in 0..6 { runtime = tick(&runtime,&mut physics,&mut scene,&mut input).unwrap(); }
    assert_eq!(runtime.frame(owner,&model).unwrap().transition_weight,1.);
    assert!(sole(&model,&runtime.frame(owner,&model).unwrap(),scene.world_matrix(owner).unwrap())
        .abs_diff_eq(Vec3::new(0.02,0.,0.),3e-6));
    // Fully release, start a live zero-to-zero fade, then interrupt its zero
    // snapshot with a stepping clip. The new swing must rearm at the new sole.
    scene.insert_component(owner,crate::ModelAnimation {clip:Some(1),transition_seconds:0.01,..Default::default()}).unwrap();
    runtime = tick(&runtime,&mut physics,&mut scene,&mut input).unwrap();
    scene.insert_component(owner,crate::ModelAnimation {clip:Some(3),transition_seconds:0.5,..Default::default()}).unwrap();
    runtime = tick(&runtime,&mut physics,&mut scene,&mut input).unwrap();
    scene.insert_component(owner,crate::ModelAnimation {clip:Some(2),transition_seconds:0.5,..Default::default()}).unwrap();
    let mut expected_anchor = 0.;
    for step in 0..6 {
        runtime = tick(&runtime,&mut physics,&mut scene,&mut input).unwrap();
        if step == 2 { expected_anchor = scene.world_matrix(owner).unwrap().w_axis.x; }
    }
    assert_eq!(runtime.frame(owner,&model).unwrap().transition_weight,1.);
    assert!(sole(&model,&runtime.frame(owner,&model).unwrap(),scene.world_matrix(owner).unwrap())
        .abs_diff_eq(Vec3::new(expected_anchor,0.,0.),3e-6));
    assert!(expected_anchor > 0.1);

}

#[test]
fn equivalent_rig_reload_preserves_fade_and_anchor_but_changed_clip_resets_them() {
    let mut asset = (*model()).clone();
    for name in ["walk", "run"] {
        asset.animations.push(Arc::new(voxy_animation::AnimationClip::new(name, 1.,
            voxy_animation::Playback::Loop, vec![voxy_animation::JointTrack::default(); asset.skeleton.joints().len()],
            &asset.skeleton).unwrap()));
    }
    let mut model = Arc::new(asset);
    let mut scene = SceneGraph::new(4);
    let owner = scene.spawn(None, Transform { translation: Vec3::Y, ..Default::default() }).unwrap();
    let id = AssetId("fading-feet".into());
    scene.insert_component(owner, crate::ModelInstance { asset: id.clone() }).unwrap();
    scene.insert_component(owner, crate::ModelAnimation::default()).unwrap();
    let mut feet = settings();
    // Explicit curves incur event work even while the existing anchor remains valid.
    // Two distant colliders alone do not force a probe of an already planted foot.
    feet.feet[0].contact_curve = vec![
        FootContactKey { phase: 0., weight: 1. },
        FootContactKey { phase: 1., weight: 1. },
    ];
    scene.insert_component(owner, feet).unwrap();
    scene.insert_component(owner, CharacterBody { half_extents: [0.1, 1., 0.1], ..Default::default() }).unwrap();
    for x in [0., 10.] {
        let floor = scene.spawn(None, Transform { translation: Vec3::new(x, -0.1, 0.), ..Default::default() }).unwrap();
        scene.insert_component(floor, BoxCollider { half_extents: [4., 0.1, 4.] }).unwrap();
    }
    let mut models = BTreeMap::from([(id.clone(), model.clone())]);
    let mut runtime = crate::animation_runtime::AnimationRuntime::default();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    let tick = |runtime: &crate::animation_runtime::AnimationRuntime, physics: &mut CharacterPhysics,
        scene: &mut SceneGraph, input: &mut voxy_input::InputMap, models: &BTreeMap<AssetId,Arc<ModelAsset>>| {
        let candidate = runtime.prepare(scene, models, 1./60.).unwrap();
        physics.fixed_step_with_preparation(scene, input, 1./60., &[(owner, Vec3::X*0.02)], &[],
            |preview, budget| candidate.clone().correct_feet(preview, budget)).map(|(_, frame)| frame)
    };
    runtime = tick(&runtime, &mut physics, &mut scene, &mut input, &models).unwrap();
    scene.insert_component(owner, crate::ModelAnimation { clip: Some(1), transition_seconds: 0.05, ..Default::default() }).unwrap();
    for step in 0..2 {
        let mut rebuilt = (*model).clone();
        rebuilt.skeleton = voxy_animation::Skeleton::new(model.skeleton.joints().to_vec()).unwrap();
        rebuilt.animations = ["walk", "run"].into_iter().map(|name| Arc::new(
            voxy_animation::AnimationClip::new(name,1.,voxy_animation::Playback::Loop,
                vec![voxy_animation::JointTrack::default();rebuilt.skeleton.joints().len()],&rebuilt.skeleton).unwrap())).collect();
        assert!(!Arc::ptr_eq(&rebuilt.animations[0], &model.animations[0]));
        model = Arc::new(rebuilt);
        models.insert(id.clone(),model.clone());
        runtime = tick(&runtime, &mut physics, &mut scene, &mut input, &models).unwrap();
        assert!((runtime.frame(owner,&model).unwrap().transition_weight - (step + 1) as f32 / 3.).abs() < 1e-6);
        assert!(sole(&model, &runtime.frame(owner, &model).unwrap(), scene.world_matrix(owner).unwrap())
            .abs_diff_eq(Vec3::new(0.02,0.,0.), 3e-6));
    }
    scene.insert_component(owner, crate::ModelAnimation { clip: Some(0), transition_seconds: 0.01, ..Default::default() }).unwrap();
    let frame = runtime.frame(owner, &model).unwrap();
    let serial = runtime.serial();
    let position = scene.local(owner).unwrap();
    physics = physics.with_angular_trajectory_query_budget(1).unwrap();
    let error = tick(&runtime, &mut physics, &mut scene, &mut input, &models).unwrap_err();
    assert!(format!("{error:?}").contains("contact event budget exceeded"));
    assert!(Arc::ptr_eq(&frame, &runtime.frame(owner, &model).unwrap()));
    assert_eq!(runtime.serial(), serial);
    assert_eq!(scene.local(owner).unwrap(), position);
    physics = physics.with_angular_trajectory_query_budget(65536).unwrap();
    runtime = tick(&runtime, &mut physics, &mut scene, &mut input, &models).unwrap();
    assert_eq!(runtime.frame(owner, &model).unwrap().transition_weight, 1.);
    assert!(sole(&model, &runtime.frame(owner, &model).unwrap(), scene.world_matrix(owner).unwrap())
        .abs_diff_eq(Vec3::new(0.02,0.,0.), 3e-6));
    scene.insert_component(owner,crate::ModelAnimation { clip:Some(0),transition_seconds:0.,..Default::default() }).unwrap();
    let mut changed = (*model).clone();
    changed.animations[0] = Arc::new(voxy_animation::AnimationClip::new("walk",2.,voxy_animation::Playback::Loop,
        vec![voxy_animation::JointTrack::default();changed.skeleton.joints().len()],&changed.skeleton).unwrap());
    model = Arc::new(changed);
    models.insert(id.clone(),model.clone());
    runtime = tick(&runtime,&mut physics,&mut scene,&mut input,&models).unwrap();
    let actor_x = scene.world_matrix(owner).unwrap().w_axis.x;
    assert!(sole(&model,&runtime.frame(owner,&model).unwrap(),scene.world_matrix(owner).unwrap())
        .abs_diff_eq(Vec3::new(actor_x,0.,0.),3e-6));
    assert!(actor_x > 0.02);

    // An explicitly authored fade admits a revised duration at the current phase.
    scene.insert_component(owner,crate::ModelAnimation { clip:Some(0),transition_seconds:0.5,..Default::default() }).unwrap();
    let mut revised = (*model).clone();
    revised.animations[0] = Arc::new(voxy_animation::AnimationClip::new("walk",3.,voxy_animation::Playback::Loop,
        vec![voxy_animation::JointTrack::default();revised.skeleton.joints().len()],&revised.skeleton).unwrap());
    model = Arc::new(revised);
    models.insert(id.clone(),model.clone());
    runtime = tick(&runtime,&mut physics,&mut scene,&mut input,&models).unwrap();
    assert!((runtime.frame(owner,&model).unwrap().transition_weight - 1./30.).abs() < 1e-6);
    assert!(sole(&model,&runtime.frame(owner,&model).unwrap(),scene.world_matrix(owner).unwrap())
        .abs_diff_eq(Vec3::new(actor_x,0.,0.),3e-6));

    let named = crate::ModelAnimation {clip:Some(usize::MAX),clip_name:"walk".into(),transition_seconds:0.5,..Default::default()};
    let encoded = serde_json::to_value(&named).unwrap();
    assert_eq!(serde_json::from_value::<crate::ModelAnimation>(encoded.clone()).unwrap(),named);
    let mut legacy = encoded;
    legacy.as_object_mut().unwrap().remove("clip_name");
    assert!(serde_json::from_value::<crate::ModelAnimation>(legacy).unwrap().clip_name.is_empty());
    scene.insert_component(owner,named.clone()).unwrap();
    runtime = tick(&runtime,&mut physics,&mut scene,&mut input,&models).unwrap();
    let old_phase = runtime.clip_phase(owner).unwrap();
    let old_weight = runtime.frame(owner,&model).unwrap().transition_weight;
    let mut reordered = (*model).clone();
    reordered.animations.swap(0,1);
    model = Arc::new(reordered);
    models.insert(id.clone(),model.clone());
    runtime = tick(&runtime,&mut physics,&mut scene,&mut input,&models).unwrap();
    assert!((runtime.clip_phase(owner).unwrap() - old_phase - (1./60.)/3.).abs()<1e-7);
    assert!((runtime.frame(owner,&model).unwrap().transition_weight-old_weight-1./30.).abs()<1e-6);
    assert!(sole(&model,&runtime.frame(owner,&model).unwrap(),scene.world_matrix(owner).unwrap())
        .abs_diff_eq(Vec3::new(actor_x,0.,0.),3e-6));
    assert_eq!(named.resolve_clip(&model).unwrap(),Some(1));
    let mut duplicate = (*model).clone();
    duplicate.animations.push(duplicate.animations[1].clone());
    assert!(named.resolve_clip(&duplicate).unwrap_err().contains("ambiguous"));
    let accepted = runtime.frame(owner,&model).unwrap();
    let serial = runtime.serial();
    models.insert(id,Arc::new(duplicate));
    assert!(runtime.prepare(&scene,&models,1./60.).unwrap_err().contains("ambiguous"));
    assert_eq!(runtime.serial(),serial);
    assert!(Arc::ptr_eq(&accepted,&runtime.frame(owner,&model).unwrap()));
    let absent = crate::ModelAnimation {clip_name:"missing".into(),..Default::default()};
    assert!(absent.resolve_clip(&model).unwrap_err().contains("missing"));

}

#[test]
fn retargeted_source_clips_keep_target_sole_locked_through_fade_and_budget_retry() {
    let target = model();
    let original = model_bytes();
    let glb = gltf::binary::Glb::from_slice(&original).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&glb.json).unwrap();
    for (index,name) in ["sourceHip","sourceKnee","sourceFoot"].iter().enumerate() {
        json["nodes"][index]["name"] = serde_json::json!(name);
    }
    json["nodes"][0]["translation"] = serde_json::json!([0.,0.8,0.]);
    let bytes = gltf::binary::Glb { header:glb.header, json:serde_json::to_vec(&json).unwrap().into(), bin:glb.bin }.to_vec().unwrap();
    let mut source = ModelAsset::parse(&bytes,&[],voxy_render::ModelLimits::default()).unwrap();
    for (name,speed) in [("walk",0.2),("run",0.4)] {
        let mut tracks = vec![voxy_animation::JointTrack::default();source.skeleton.joints().len()];
        tracks[0].translations=vec![voxy_animation::Vec3Key {time:0.,value:Vec3::new(0.,0.8,0.)},voxy_animation::Vec3Key {time:1.,value:Vec3::new(speed,0.8,0.)}];
        source.animations.push(Arc::new(voxy_animation::AnimationClip::new(name,1.,voxy_animation::Playback::Loop,tracks,&source.skeleton).unwrap()));
    }
    let source=Arc::new(source);
    let mut scene=SceneGraph::new(4);
    let owner=scene.spawn(None,Transform {translation:Vec3::Y,..Default::default()}).unwrap();
    scene.insert_component(owner,crate::ModelInstance {asset:AssetId("target".into())}).unwrap();
    scene.insert_component(owner,crate::ModelAnimation {clip_name:"walk".into(),root_motion_bone:"sourceHip".into(),root_motion_axes:[true,false,false],..Default::default()}).unwrap();
    scene.insert_component(owner,crate::ModelRetarget {source:"source".into(),joints:[("sourceHip","hip"),("sourceKnee","knee"),("sourceFoot","foot")].into_iter().map(|(a,b)|crate::RetargetJointProfile {source:a.into(),target:b.into(),rotation_basis:glam::Quat::IDENTITY.to_array(),translation_basis:glam::Quat::IDENTITY.to_array(),translation_scale:1.}).collect()}).unwrap();
    let mut feet=settings();
    for name in ["walk","run"] {feet.feet[0].clip_contact_curves.insert(name.into(),vec![FootContactKey {phase:0.,weight:1.},FootContactKey {phase:1.,weight:1.}]);}
    assert!(FootRuntime::new(&target,feet.clone()).is_err());
    scene.insert_component(owner,feet).unwrap();
    scene.insert_component(owner,CharacterBody {half_extents:[0.1,1.,0.1],..Default::default()}).unwrap();
    let floor=scene.spawn(None,Transform {translation:-Vec3::Y*0.1,..Default::default()}).unwrap();
    scene.insert_component(floor,BoxCollider {half_extents:[4.,0.1,4.]}).unwrap();
    let models=BTreeMap::from([(AssetId("target".into()),target.clone()),(AssetId("source".into()),source)]);
    let mut runtime=crate::animation_runtime::AnimationRuntime::default();
    let mut physics=CharacterPhysics::new(&scene,4,4);
    let mut input=player_input().unwrap();
    let tick=|runtime:&crate::animation_runtime::AnimationRuntime,physics:&mut CharacterPhysics,scene:&mut SceneGraph,input:&mut voxy_input::InputMap| {
        let candidate=runtime.prepare(scene,&models,1./60.).unwrap();
        physics.fixed_step_with_preparation(scene,input,1./60.,candidate.motions(),&[],|preview,budget|candidate.clone().correct_feet(preview,budget)).map(|(_,frame)|frame)
    };
    runtime=tick(&runtime,&mut physics,&mut scene,&mut input).unwrap();
    let anchor=sole(&target,&runtime.frame(owner,&target).unwrap(),scene.world_matrix(owner).unwrap());
    assert!(anchor.abs_diff_eq(Vec3::new(0.2/60.,0.,0.),3e-6));
    scene.component_mut::<crate::ModelAnimation>(owner).unwrap().unwrap().clip_name="run".into();
    scene.component_mut::<crate::ModelAnimation>(owner).unwrap().unwrap().transition_seconds=0.1;
    for _ in 0..3 {
        runtime=tick(&runtime,&mut physics,&mut scene,&mut input).unwrap();
        assert!(sole(&target,&runtime.frame(owner,&target).unwrap(),scene.world_matrix(owner).unwrap()).abs_diff_eq(anchor,3e-6));
    }
    assert!(runtime.frame(owner,&target).unwrap().transition_weight<1.);
    let frame=runtime.frame(owner,&target).unwrap();let serial=runtime.serial();let position=scene.local(owner).unwrap();let phase=runtime.clip_phase(owner);
    physics=physics.with_angular_trajectory_query_budget(1).unwrap();
    assert!(format!("{:?}",tick(&runtime,&mut physics,&mut scene,&mut input).unwrap_err()).contains("contact event budget exceeded"));
    assert!(Arc::ptr_eq(&frame,&runtime.frame(owner,&target).unwrap()));assert_eq!(runtime.serial(),serial);assert_eq!(runtime.clip_phase(owner),phase);assert_eq!(scene.local(owner).unwrap(),position);
    physics=physics.with_angular_trajectory_query_budget(65536).unwrap();
    for _ in 0..5 {runtime=tick(&runtime,&mut physics,&mut scene,&mut input).unwrap();assert!(sole(&target,&runtime.frame(owner,&target).unwrap(),scene.world_matrix(owner).unwrap()).abs_diff_eq(anchor,3e-6));}
    assert_eq!(runtime.frame(owner,&target).unwrap().transition_weight,1.);
    assert!(scene.local(owner).unwrap().translation.x>anchor.x+0.03);
}
