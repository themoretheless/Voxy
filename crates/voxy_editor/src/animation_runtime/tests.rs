use super::*;
use glam::Vec3;
fn fixture() -> (
    SceneGraph,
    NodeId,
    Arc<ModelAsset>,
    BTreeMap<AssetId, Arc<ModelAsset>>,
) {
    let model = Arc::new(
        ModelAsset::parse(
            include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb"),
            &[],
            voxy_render::ModelLimits::default(),
        )
        .unwrap(),
    );
    let mut scene = SceneGraph::new(8);
    let owner = scene.spawn(None, Default::default()).unwrap();
    let id = AssetId("rig".into());
    scene
        .insert_component(owner, ModelInstance { asset: id.clone() })
        .unwrap();
    scene
        .insert_component(owner, ModelAnimation::default())
        .unwrap();
    (scene, owner, model.clone(), BTreeMap::from([(id, model)]))
}
#[test]
fn fixed_frames_wait_for_assets_pause_resume_and_never_advance_on_read() {
    let (mut scene, owner, model, models) = fixture();
    let runtime = AnimationRuntime::default();
    let mut runtime = runtime.prepare(&scene, &BTreeMap::new(), 1. / 60.).unwrap();
    assert!(runtime.frame(owner, &model).is_none());
    runtime = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
    let frame = runtime.frame(owner, &model).unwrap();
    assert!((frame.pose.local()[0].translation.x - 2. / 60.).abs() < 1e-6);
    for _ in 0..100 {
        assert!(Arc::ptr_eq(&frame, &runtime.frame(owner, &model).unwrap()));
    }
    scene.set_active(owner, false).unwrap();
    runtime = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
    assert!(Arc::ptr_eq(&frame, &runtime.frame(owner, &model).unwrap()));
    scene.set_active(owner, true).unwrap();
    runtime = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
    assert!(
        (runtime.frame(owner, &model).unwrap().pose.local()[0]
            .translation
            .x
            - 4. / 60.)
            .abs()
            < 1e-6
    );
    scene
        .component_mut::<ModelAnimation>(owner)
        .unwrap()
        .unwrap()
        .speed = 0.;
    runtime = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
    assert!(
        (runtime.frame(owner, &model).unwrap().pose.local()[0]
            .translation
            .x
            - 4. / 60.)
            .abs()
            < 1e-6
    );
    let serial = runtime.serial();
    scene
        .component_mut::<ModelAnimation>(owner)
        .unwrap()
        .unwrap()
        .speed = f32::NAN;
    assert!(runtime.prepare(&scene, &models, 1. / 60.).is_err());
    assert_eq!(runtime.serial(), serial);
    assert!(runtime.prepare(&scene, &models, 0.).is_err());
    let foreign = SceneGraph::new(8);
    assert!(runtime.prepare(&foreign, &models, 1. / 60.).is_err());
    scene.remove_subtree(owner).unwrap();
    runtime.synchronize(&scene).unwrap();
    assert!(runtime.frame(owner, &model).is_none());
    runtime.clear();
    assert_eq!(runtime.serial(), 0);
}
#[test]
fn motion_conversion_uses_fixed_parent_basis_and_rejects_animated_ancestors() {
    let (mut scene, owner, _, _) = fixture();
    let glb = gltf::binary::Glb::from_slice(include_bytes!(
        "../../../voxy_render/examples/assets/animated-triangle.glb"
    ))
    .unwrap();
    let mut doc: serde_json::Value = serde_json::from_slice(&glb.json).unwrap();
    let mut bytes = glb.bin.as_deref().unwrap().to_vec();
    let constant_offset = bytes.len();
    for value in [3.0_f32, 5., 7., 3., 5., 7.] {
        bytes.extend(value.to_le_bytes());
    }
    let rotation = glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2).to_array();
    doc["nodes"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "name":"basis", "children":[0], "scale":[2,3,4], "rotation": rotation
        }));
    doc["buffers"][0]["uri"] = serde_json::json!("fixture.bin");
    doc["buffers"][0]["byteLength"] = serde_json::json!(bytes.len());
    doc["scenes"] = serde_json::json!([{"nodes":[2]}]);
    doc["scene"] = serde_json::json!(0);
    let parse = |doc: &serde_json::Value| {
        Arc::new(
            ModelAsset::parse(
                &serde_json::to_vec(doc).unwrap(),
                &[&bytes],
                voxy_render::ModelLimits::default(),
            )
            .unwrap(),
        )
    };
    let model = parse(&doc);
    let models = BTreeMap::from([(AssetId("rig".into()), model.clone())]);
    scene
        .insert_component(
            owner,
            voxy_gameplay::CharacterBody {
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            owner,
            ModelAnimation {
                root_motion_bone: "root".into(),
                root_motion_axes: [true, false, false],
                ..Default::default()
            },
        )
        .unwrap();
    let runtime = AnimationRuntime::default()
        .prepare(&scene, &models, 1. / 60.)
        .unwrap();
    assert!(
        runtime.motions()[0]
            .1
            .abs_diff_eq(-Vec3::Z * (4. / 60.), 1e-6)
    );
    let joint = model.resolve_joint_name("root").unwrap() as usize;
    assert_eq!(
        runtime.frame(owner, &model).unwrap().pose.local()[joint].translation,
        Vec3::ZERO
    );
    let view = doc["bufferViews"].as_array().unwrap().len();
    let accessor = doc["accessors"].as_array().unwrap().len();
    doc["bufferViews"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"buffer":0,"byteOffset":constant_offset,"byteLength":24}));
    doc["accessors"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"bufferView":view,"componentType":5126,"count":2,"type":"VEC3"}));
    let sampler = doc["animations"][0]["samplers"].as_array().unwrap().len();
    doc["animations"][0]["samplers"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"input":4,"output":accessor,"interpolation":"LINEAR"}));
    doc["animations"][0]["channels"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"sampler":sampler,"target":{"node":2,"path":"scale"}}));
    let authored_model = parse(&doc);
    let authored_models = BTreeMap::from([(AssetId("rig".into()), authored_model.clone())]);
    let authored = runtime.prepare(&scene, &authored_models, 1. / 60.).unwrap();
    assert!(authored.motions()[0].1.abs_diff_eq(-Vec3::Z * 0.1, 1e-6));
    let parent = authored_model.resolve_joint_name("basis").unwrap() as usize;
    assert!(
        authored.frame(owner, &authored_model).unwrap().pose.local()[parent]
            .scale
            .abs_diff_eq(Vec3::new(3., 5., 7.), 1e-6)
    );
    doc["animations"][0]["channels"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "sampler":0, "target":{"node":2,"path":"translation"}
        }));
    let model = parse(&doc);
    let models = BTreeMap::from([(AssetId("rig".into()), model)]);
    assert!(
        runtime
            .prepare(&scene, &models, 1. / 60.)
            .unwrap_err()
            .contains("parent")
    );
}
#[test]
fn editor_play_drives_root_motion_through_wall_and_preserves_failed_tick() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/animated-triangle.glb");
    let mut app = crate::App::new(&source, false).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.catalog.snapshot(&app.id).is_none() {
        app.tick().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    app.edit_key(winit::keyboard::KeyCode::KeyD).unwrap();
    let owner = app.instances[0];
    let wall = app.instances[1];
    app.scene
        .insert_component(
            owner,
            voxy_gameplay::CharacterBody {
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    app.scene
        .insert_component(
            owner,
            ModelAnimation {
                root_motion_axes: [true, false, false],
                ..Default::default()
            },
        )
        .unwrap();
    app.scene
        .insert_component(wall, voxy_gameplay::BoxCollider::default())
        .unwrap();
    app.scene
        .insert_component(
            wall,
            ModelAnimation {
                speed: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    app.scene
        .set_local(
            wall,
            voxy_scene::Transform {
                translation: app.scene.local(owner).unwrap().translation + Vec3::X * 0.15,
                ..Default::default()
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    let authored = app.authoring_document().unwrap();
    app.toggle_play().unwrap();
    let owner = app.instances[0];
    let wall = app.instances[1];
    let model = app
        .catalog
        .snapshot(&app.id)
        .unwrap()
        .value()
        .animated
        .as_ref()
        .unwrap()
        .clone();
    app.advance_game(0.).unwrap();
    assert_eq!(app.play.animations.serial(), 0);
    for _ in 0..12 {
        app.advance_game(1. / 60.).unwrap();
    }
    assert!(
        (app.scene.local(owner).unwrap().translation.x - 0.05).abs() < 1e-5,
        "position={:?} serial={}",
        app.scene.local(owner).unwrap().translation,
        app.play.animations.serial()
    );
    let frame = app.play.animations.frame(owner, &model).unwrap();
    assert_eq!(frame.pose.local()[0].translation, Vec3::ZERO);
    for _ in 0..20 {
        app.advance_game(0.).unwrap();
    }
    assert_eq!(app.play.animations.serial(), 12);
    assert!(Arc::ptr_eq(
        &frame,
        &app.play.animations.frame(owner, &model).unwrap()
    ));
    app.scene
        .component_mut::<voxy_gameplay::BoxCollider>(wall)
        .unwrap()
        .unwrap()
        .half_extents[0] = f32::NAN;
    app.play
        .player_input
        .event(voxy_gameplay::JUMP, 1.)
        .unwrap();
    assert!(app.advance_game(1. / 60.).is_err());
    assert_eq!(app.play.animations.serial(), 12);
    assert!(Arc::ptr_eq(
        &frame,
        &app.play.animations.frame(owner, &model).unwrap()
    ));
    assert!(app.play.player_input.state("jump").unwrap().pressed);
    app.scene
        .component_mut::<voxy_gameplay::BoxCollider>(wall)
        .unwrap()
        .unwrap()
        .half_extents[0] = 0.05;
    app.advance_game(1. / 60.).unwrap();
    assert_eq!(app.play.animations.serial(), 13);
    #[derive(Debug)]
    struct FailSecondTick(u8);
    impl voxy_scene::Behavior for FailSecondTick {
        fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, _: f64) {
            self.0 += 1;
            if self.0 == 2 {
                scene
                    .component_mut::<voxy_gameplay::BoxCollider>(owner)
                    .unwrap()
                    .unwrap()
                    .half_extents[0] = f32::NAN;
            }
        }
    }
    app.play
        .simulation
        .as_mut()
        .unwrap()
        .attach(&mut app.scene, wall, FailSecondTick(0))
        .unwrap();
    assert!(app.advance_game(0.05).is_err());
    assert_eq!(app.play.animations.serial(), 14);
    assert_eq!(app.play.simulation_ticks, 14);
    app.toggle_play().unwrap();
    assert_eq!(app.play.animations.serial(), 0);
    assert!(app.play.animations.frame(owner, &model).is_none());
    assert_eq!(app.authoring_document().unwrap(), authored);
    app.stop_workers().unwrap();
}

#[test]
fn hierarchy_parts_do_not_consume_owner_capacity_and_overflow_is_atomic() {
    let (_, _, model, models) = fixture();
    let mut scene = SceneGraph::new(512);
    let mut owners = Vec::new();
    let mut parts = Vec::new();
    for index in 0..128 {
        let owner = scene.spawn(None, Default::default()).unwrap();
        scene
            .insert_component(
                owner,
                ModelInstance {
                    asset: AssetId("rig".into()),
                },
            )
            .unwrap();
        if index % 2 == 0 {
            scene
                .insert_component(owner, ModelPart { node: u32::MAX })
                .unwrap();
        }
        owners.push(owner);
        for node in 0..2 {
            let part = scene.spawn(Some(owner), Default::default()).unwrap();
            scene
                .insert_component(
                    part,
                    ModelInstance {
                        asset: AssetId("rig".into()),
                    },
                )
                .unwrap();
            scene.insert_component(part, ModelPart { node }).unwrap();
            parts.push(part);
        }
    }
    assert_eq!(scene.components::<ModelInstance>().count(), 384);
    let mut runtime = AnimationRuntime::default()
        .prepare(&scene, &models, 1. / 60.)
        .unwrap();
    assert_eq!(runtime.owners.len(), 128);
    let frames: Vec<_> = owners
        .iter()
        .map(|&owner| runtime.frame(owner, &model).unwrap())
        .collect();
    assert!(
        parts
            .iter()
            .all(|&part| runtime.frame(part, &model).is_none())
    );
    for frame in &frames {
        assert!((frame.pose.local()[0].translation.x - 2. / 60.).abs() < 1e-6);
    }
    let overflow = scene.spawn(None, Default::default()).unwrap();
    scene
        .insert_component(
            overflow,
            ModelInstance {
                asset: AssetId("rig".into()),
            },
        )
        .unwrap();
    for active in [true, false] {
        scene.set_active(overflow, active).unwrap();
        assert!(
            runtime
                .prepare(&scene, &models, 1. / 60.)
                .unwrap_err()
                .contains("capacity")
        );
        assert_eq!(runtime.serial(), 1);
        assert_eq!(runtime.owners.len(), 128);
        for (&owner, frame) in owners.iter().zip(&frames) {
            assert!(Arc::ptr_eq(frame, &runtime.frame(owner, &model).unwrap()));
        }
    }
    scene.remove_subtree(overflow).unwrap();
    runtime = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
    assert_eq!(runtime.serial(), 2);
    assert!(
        (runtime.frame(owners[0], &model).unwrap().pose.local()[0]
            .translation
            .x
            - 4. / 60.)
            .abs()
            < 1e-6
    );
    scene.remove_subtree(owners[0]).unwrap();
    runtime.synchronize(&scene).unwrap();
    assert_eq!(runtime.owners.len(), 127);
    assert!(runtime.frame(owners[0], &model).is_none());
    runtime.clear();
    assert!(runtime.owners.is_empty());
}

#[test]
fn editor_expanded_rig_scene_survives_history_save_load_play_and_stop() {
    let directory = std::env::temp_dir().join(format!(
        "voxy-rig-owner-scene-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let source = directory.join("model.gltf");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../artifacts/rig-constant-parents-2026-10-03");
    std::fs::copy(fixture.join("rig.gltf"), &source).unwrap();
    std::fs::copy(fixture.join("rig.bin"), directory.join("rig.bin")).unwrap();
    let scene_path = directory.join("scene.json");
    let manifest = directory.join("assets.json");
    std::fs::write(&manifest, r#"{"version":1,"assets":[{"asset":"model.gltf","source":"model.gltf"},{"asset":"static.gltf","source":"static.gltf"}]}"#).unwrap();
    let mut app =
        crate::App::from_manifest(&manifest, AssetId("model.gltf".into()), false).unwrap();
    app.configure_scene(&scene_path).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.catalog.snapshot(&app.id).is_none() {
        app.tick().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    for _ in 1..50 {
        app.edit_key(winit::keyboard::KeyCode::KeyD).unwrap();
    }
    // Skeletal resources retain their internal skeleton; expand a static hierarchy
    // beside the independent rigs through the actual editor command.
    let mut static_doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&source).unwrap()).unwrap();
    static_doc.as_object_mut().unwrap().remove("animations");
    static_doc.as_object_mut().unwrap().remove("skins");
    for node in static_doc["nodes"].as_array_mut().unwrap() {
        node.as_object_mut().unwrap().remove("skin");
    }
    for mesh in static_doc["meshes"].as_array_mut().unwrap() {
        for primitive in mesh["primitives"].as_array_mut().unwrap() {
            let attributes = primitive["attributes"].as_object_mut().unwrap();
            attributes.remove("JOINTS_0");
            attributes.remove("WEIGHTS_0");
        }
    }
    for index in 3..100 {
        static_doc["nodes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"name":format!("part-{index}")}));
        static_doc["nodes"][2]["children"]
            .as_array_mut()
            .unwrap()
            .push(index.into());
    }
    std::fs::write(
        directory.join("static.gltf"),
        serde_json::to_vec(&static_doc).unwrap(),
    )
    .unwrap();
    app.edit_key(winit::keyboard::KeyCode::KeyD).unwrap();
    let owner = app.instances[app.selected];
    let static_id = AssetId("static.gltf".into());
    app.scene
        .insert_component(
            owner,
            ModelInstance {
                asset: static_id.clone(),
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    app.reload.insert(static_id.clone());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.catalog.snapshot(&static_id).is_none() {
        app.tick().unwrap();
        assert!(std::time::Instant::now() < deadline, "{:?}", app.error);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    app.expand_model().unwrap();
    let authored = app.authoring_document().unwrap();
    assert_eq!(authored.objects.len(), 151);
    app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap().objects.len(), 51);
    app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), authored);
    app.save_authoring().unwrap();
    app.load_authoring().unwrap();
    assert_eq!(app.authoring_document().unwrap(), authored);
    app.tick().unwrap(); // Exercise ordinary scene extraction above the former 128 cap.
    app.toggle_play().unwrap();
    for _ in 0..3 {
        app.advance_game(1. / 60.).unwrap();
    }
    assert_eq!(app.play.animations.owners.len(), 50);
    assert_eq!(app.play.animations.serial(), 3);
    let model = app
        .catalog
        .snapshot(&app.id)
        .unwrap()
        .value()
        .animated
        .as_ref()
        .unwrap()
        .clone();
    let animated_owners: Vec<_> = app
        .scene
        .components::<ModelInstance>()
        .filter(|(_, instance)| instance.asset == app.id)
        .map(|(owner, _)| owner)
        .collect();
    assert_eq!(animated_owners.len(), 50);
    for owner in animated_owners {
        assert!(
            (app.play
                .animations
                .frame(owner, &model)
                .unwrap()
                .pose
                .local()[usize::from(model.resolve_joint_name("root").unwrap())]
            .translation
            .x - 0.1)
                .abs()
                < 1e-6
        );
    }
    app.toggle_play().unwrap();
    assert!(app.play.animations.owners.is_empty());
    assert_eq!(app.authoring_document().unwrap(), authored);
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn clip_switch_fades_from_the_running_source_and_completes_in_fixed_ticks() {
    let (mut scene, owner, source, _) = fixture();
    let mut asset = (*source).clone();
    let mut tracks = vec![voxy_animation::JointTrack::default(); asset.skeleton.joints().len()];
    tracks[0].translations = vec![voxy_animation::Vec3Key { time: 0., value: Vec3::X * 10. }];
    asset.animations.push(Arc::new(voxy_animation::AnimationClip::new(
        "target", 1., voxy_animation::Playback::Loop, tracks, &asset.skeleton).unwrap()));
    let mut tracks = vec![voxy_animation::JointTrack::default(); asset.skeleton.joints().len()];
    tracks[0].translations = vec![voxy_animation::Vec3Key { time: 0., value: Vec3::X * 20. }];
    asset.animations.push(Arc::new(voxy_animation::AnimationClip::new(
        "interrupted-target", 1., voxy_animation::Playback::Loop, tracks, &asset.skeleton).unwrap()));
    let asset = Arc::new(asset);
    let id = scene.component::<ModelInstance>(owner).unwrap().unwrap().asset.clone();
    let models = BTreeMap::from([(id, asset.clone())]);
    let mut runtime = AnimationRuntime::default().prepare(&scene, &models, 0.1).unwrap();
    let before = runtime.frame(owner, &asset).unwrap();
    scene.insert_component(owner, ModelAnimation {
        clip: Some(1), transition_seconds: 0.5, ..Default::default() }).unwrap();
    runtime = runtime.prepare(&scene, &models, 0.1).unwrap();
    let frame = runtime.frame(owner, &asset).unwrap();
    assert!((frame.transition_weight - 0.2).abs() < 1e-6);
    assert!((frame.pose.local()[0].translation.x - 2.32).abs() < 1e-5);
    assert!((before.pose.local()[0].translation.x - 0.2).abs() < 1e-6);
    let held = runtime.frame(owner, &asset).unwrap();
    scene.insert_component(owner, crate::ModelFootPlacement { feet: vec![crate::FootBinding {
        bones: ["root".into(), "root".into(), "root".into()], sole_offset: [0.; 3],
        sole_up: [0., 1., 0.], pole: [1., 0., 0.], plant: true, weight: 1.,
        contact: Default::default(), contact_curve: vec![], clip_contact_curves: Default::default(),
    }] }).unwrap();
    assert_eq!(runtime.prepare(&scene, &models, 0.1).err().unwrap(),
        "foot placement requires a CharacterBody on the model owner");
    assert!(Arc::ptr_eq(&held, &runtime.frame(owner, &asset).unwrap()));
    scene.insert_component(owner, ModelAnimation {
        clip: Some(2), transition_seconds: 0.5, ..Default::default() }).unwrap();
    assert_eq!(runtime.prepare(&scene, &models, 0.1).err().unwrap(),
        "foot placement requires a CharacterBody on the model owner");
    assert!(Arc::ptr_eq(&held, &runtime.frame(owner, &asset).unwrap()));
    scene.remove_component::<crate::ModelFootPlacement>(owner).unwrap();
    runtime = runtime.prepare(&scene, &models, 0.1).unwrap();
    assert!((runtime.frame(owner, &asset).unwrap().pose.local()[0].translation.x - 5.856).abs() < 1e-5);
    for _ in 0..4 { runtime = runtime.prepare(&scene, &models, 0.1).unwrap(); }
    assert_eq!(runtime.frame(owner, &asset).unwrap().transition_weight, 1.);
    assert_eq!(runtime.frame(owner, &asset).unwrap().pose.local()[0].translation.x, 20.);
}

#[test]
fn retarget_source_clock_produces_target_palette_and_failure_keeps_accepted_frame() {
    let (mut scene, owner, source, _) = fixture();
    let glb = gltf::binary::Glb::from_slice(include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb")).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&glb.json).unwrap();
    json["nodes"][0]["name"] = serde_json::json!("pelvis");
    json["nodes"][0]["translation"] = serde_json::json!([3.,4.,0.]);
    json["animations"] = serde_json::json!([]);
    let bytes = gltf::binary::Glb { header:glb.header, json:serde_json::to_vec(&json).unwrap().into(), bin:glb.bin }.to_vec().unwrap();
    let target = Arc::new(ModelAsset::parse(&bytes, &[], voxy_render::ModelLimits::default()).unwrap());
    let source_id = AssetId("source".into());
    let models = BTreeMap::from([(AssetId("rig".into()), target.clone()), (source_id, source.clone())]);
    let profile = crate::ModelRetarget { source:"source".into(), joints:vec![crate::RetargetJointProfile {
        source:"root".into(),target:"pelvis".into(),rotation_basis:glam::Quat::IDENTITY.to_array(),
        translation_basis:glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2).to_array(),translation_scale:2.
    }]};
    scene.insert_component(owner,profile.clone()).unwrap();
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().clip_name = "move".into();
    let runtime = AnimationRuntime::default().prepare(&scene,&models,0.05).unwrap();
    let frame = runtime.frame(owner,&target).unwrap();
    assert!(runtime.frame(owner,&source).is_none());
    assert!((frame.pose.local()[0].translation - Vec3::new(3.,4.2,0.)).length() < 1e-6);
    assert!((runtime.clip_phase(owner).unwrap()-0.05).abs()<1e-6);
    let independent = glam::Mat4::from_translation(Vec3::new(3.,4.2,0.)) * target.skeleton.joints()[0].inverse_bind;
    for (a,b) in frame.skin_matrices[0].to_cols_array().iter().zip(independent.to_cols_array()) { assert!((*a-b).abs()<1e-6); }
    scene.component_mut::<crate::ModelRetarget>(owner).unwrap().unwrap().joints[0].target = "missing".into();
    assert!(runtime.prepare(&scene,&models,0.05).is_err());
    assert_eq!(runtime.serial(),1);
    assert!(Arc::ptr_eq(&frame,&runtime.frame(owner,&target).unwrap()));
    scene.insert_component(owner,profile).unwrap();
    let resumed = runtime.prepare(&scene,&models,0.05).unwrap();
    assert!((resumed.frame(owner,&target).unwrap().pose.local()[0].translation.y-4.4).abs()<1e-6);
    scene.insert_component(owner,voxy_gameplay::CharacterBody::default()).unwrap();
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_axes=[false,true,false];
    let moving = resumed.prepare(&scene,&models,0.05).unwrap();
    assert!((moving.motions()[0].1 - Vec3::Y*0.2).length()<1e-6);
    assert!((moving.frame(owner,&target).unwrap().pose.local()[0].translation-Vec3::new(3.,4.,0.)).length()<1e-6);
    let mut missing = models.clone(); missing.remove(&AssetId("source".into()));
    assert!(resumed.prepare(&scene,&missing,0.05).is_err());
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_rotation=true;
    assert!(resumed.prepare(&scene,&models,0.05).is_err());
    assert!((resumed.clip_phase(owner).unwrap()-0.1).abs()<1e-6);
}

#[test]
fn partial_retarget_child_animation_keeps_unmapped_root_and_rejects_motion_atomically() {
    let glb = gltf::binary::Glb::from_slice(include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb")).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&glb.json).unwrap();
    json["nodes"] = serde_json::json!([
        {"name":"root","translation":[3.,0.,0.],"children":[1,2]},
        {"name":"arm","translation":[0.,1.,0.]},
        {"mesh":0,"skin":0}
    ]);
    json["skins"][0]["joints"] = serde_json::json!([0,1]);
    json["animations"] = serde_json::json!([]);
    let bytes = gltf::binary::Glb { header:glb.header, json:serde_json::to_vec(&json).unwrap().into(), bin:glb.bin }.to_vec().unwrap();
    let mut source = ModelAsset::parse(&bytes,&[],voxy_render::ModelLimits::default()).unwrap();
    let target = Arc::new(source.clone());
    let mut tracks = vec![voxy_animation::JointTrack::default(); source.skeleton.joints().len()];
    tracks[1].translations = vec![
        voxy_animation::Vec3Key { time:0., value:Vec3::Y },
        voxy_animation::Vec3Key { time:1., value:Vec3::Y+Vec3::X*2. }
    ];
    source.animations.push(Arc::new(voxy_animation::AnimationClip::new("arm",1.,voxy_animation::Playback::Clamp,tracks,&source.skeleton).unwrap()));
    let source = Arc::new(source);
    let mut scene = SceneGraph::new(8);
    let owner = scene.spawn(None,Default::default()).unwrap();
    scene.insert_component(owner,ModelInstance { asset:AssetId("target".into()) }).unwrap();
    scene.insert_component(owner,ModelAnimation { clip_name:"arm".into(), ..Default::default() }).unwrap();
    scene.insert_component(owner,crate::ModelRetarget { source:"source".into(), joints:vec![crate::RetargetJointProfile {
        source:"arm".into(), target:"arm".into(), rotation_basis:glam::Quat::IDENTITY.to_array(),
        translation_basis:glam::Quat::IDENTITY.to_array(), translation_scale:2.
    }] }).unwrap();
    let models = BTreeMap::from([(AssetId("target".into()),target.clone()),(AssetId("source".into()),source)]);
    let runtime = AnimationRuntime::default().prepare(&scene,&models,0.05).unwrap();
    let accepted = runtime.frame(owner,&target).unwrap();
    assert!((accepted.pose.local()[1].translation - (Vec3::Y+Vec3::X*0.2)).length()<1e-6);
    assert_eq!(accepted.pose.local()[0],target.skeleton.joints()[0].bind_local);
    let independent = glam::Mat4::from_translation(Vec3::new(3.2,1.,0.)) * target.skeleton.joints()[1].inverse_bind;
    for (actual,expected) in accepted.skin_matrices[1].to_cols_array().iter().zip(independent.to_cols_array()) {
        assert!((*actual-expected).abs()<1e-6);
    }
    assert_eq!(accepted.root_motion,Vec3::ZERO);
    assert!(runtime.motions().is_empty());
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_axes=[true,false,false];
    assert!(runtime.prepare(&scene,&models,0.05).is_err());
    assert!(Arc::ptr_eq(&accepted,&runtime.frame(owner,&target).unwrap()));
    assert!((runtime.clip_phase(owner).unwrap()-0.05).abs()<1e-6);
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_axes=[false;3];
    let resumed = runtime.prepare(&scene,&models,0.05).unwrap();
    assert!((resumed.frame(owner,&target).unwrap().pose.local()[1].translation-(Vec3::Y+Vec3::X*0.4)).length()<1e-6);
}

#[test]
fn accepted_physical_fade_uses_runtime_extraction_before_frame_publication() {
    use voxy_gameplay::{CharacterBody,CharacterPhysics,player_input};
    for blocked in [false,true] {
    let (mut scene,owner,model,models) = fixture();
    scene.insert_component(owner,CharacterBody {half_extents:[0.00390625;3],speed:0.,gravity:0.,..Default::default()}).unwrap();
    scene.insert_component(owner,ModelAnimation {root_motion_rotation:true,
        root_motion_axes:[true,false,false],..Default::default()}).unwrap();
    let mut runtime = AnimationRuntime::default().prepare(&scene,&models,1./120.).unwrap();
    runtime.owners.get_mut(&owner).unwrap().playback.transition_to_clip(0,0.125).unwrap();
    runtime.initialize_root_reference(owner,&model,voxy_animation::RootRigidTransform::IDENTITY,
        voxy_animation::RootRigidEnclosure::IDENTITY).unwrap();
    let before = runtime.frame(owner,&model).unwrap();
    let dt = 1. / 60.;
    let staged = runtime.prepare_owner_fade(&scene,&models,owner,&model,dt,0.01,0.01,4096)
        .unwrap().unwrap();
    let authored_settings = scene.component::<ModelAnimation>(owner).unwrap().unwrap().clone();
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_axes = [false;3];
    assert!(staged.admit_scene(&scene,&models).is_err());
    scene.insert_component(owner,authored_settings).unwrap();
    scene.component_mut::<ModelInstance>(owner).unwrap().unwrap().asset = AssetId("other".into());
    assert!(staged.admit_scene(&scene,&models).is_err());
    scene.component_mut::<ModelInstance>(owner).unwrap().unwrap().asset = AssetId("rig".into());
    let replacement = Arc::new(ModelAsset::parse(
        include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb"),
        &[],voxy_render::ModelLimits::default()).unwrap());
    let replaced_models = BTreeMap::from([(AssetId("rig".into()),replacement)]);
    assert!(staged.admit_scene(&scene,&replaced_models).is_err());
    let admitted = staged.admit_scene(&scene,&models).unwrap();
    if blocked {
        let wall = scene.spawn(None,voxy_scene::Transform {translation:Vec3::X*0.0234375,
            ..Default::default()}).unwrap();
        scene.insert_component(wall,voxy_gameplay::BoxCollider {half_extents:[0.00390625,2.,2.]}).unwrap();
    }
    let mut physics = CharacterPhysics::new(&scene,1,usize::from(blocked));
    let mut input = player_input().unwrap();
    let initial_pose = scene.local(owner).unwrap();
    for mismatch in 0..4 {
        let mut stale = runtime.clone();
        let state = stale.owners.get_mut(&owner).unwrap();
        match mismatch {
            0 => state.settings.root_motion_axes = [false;3],
            1 => state.root_reference = None,
            2 => state.root_reference.as_mut().unwrap().authored_to_body =
                voxy_animation::RootRigidEnclosure::from_transform(voxy_animation::RootRigidTransform {
                    translation: glam::DVec3::X * 0.25,
                    ..voxy_animation::RootRigidTransform::IDENTITY
                }).unwrap(),
            _ => state.root_reference.as_mut().unwrap().scale = voxy_animation::RootUniformScaleEnclosure::from_scale(2.).unwrap(),
        }
        let rejected = physics.fixed_step_with_certified_fade_preparation(&mut scene,&mut input,
            dt,&[admitted.request(1,0.)],|preview,_budget| {
                stale.clone().accept_fade(&model,&admitted,&preview.motions[0],&preview.characters[0])
            });
        assert!(rejected.is_err());
        assert_eq!(scene.local(owner).unwrap(),initial_pose);
        assert!(physics.state(&scene,owner).unwrap().is_none());
        assert!(Arc::ptr_eq(&before,&runtime.frame(owner,&model).unwrap()));
    }
    let frame_request = OwnerFadeAdmission {coordinate_axis:1,evaluation_radius:0.,evaluation_axes:None};
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_axes = [false;3];
    assert!(runtime.fixed_step_owner_fades(&mut scene,&models,&mut physics,&mut input,
        dt,&[(&staged,frame_request)]).is_err());
    assert_eq!(scene.local(owner).unwrap(),initial_pose);
    assert!(physics.state(&scene,owner).unwrap().is_none());
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_axes = [true,false,false];
    let (receipts,accepted) = runtime.fixed_step_owner_fades(&mut scene,&models,
        &mut physics,&mut input,dt,&[(&staged,frame_request)]).unwrap();
    assert!(Arc::ptr_eq(&before,&runtime.frame(owner,&model).unwrap()));
    let reference = accepted.owners[&owner].root_reference.unwrap();
    for bound in reference.body_to_world.compose(&reference.authored_to_body).unwrap().translation_bounds() {
        assert!(bound[0]<=0. && bound[1]>=0.);
    }
    assert_eq!(runtime.owners[&owner].root_reference.unwrap().authored_to_body.translation_bounds(),[[0.,0.];3]);
    let frame = accepted.frame(owner,&model).unwrap();
    assert_eq!(frame.pose.local()[0].translation.x,model.skeleton.joints()[0].bind_local.translation.x);
    assert_eq!(frame.root_motion.x,0.);
    assert_eq!(receipts[0].complete,!blocked);
    let contact = accepted.owners[&owner].playback.contact_interval().unwrap();
    assert_eq!(contact.end,accepted.clip_phase(owner).unwrap());
    if blocked {
        assert!(receipts[0].path_fraction>0. && receipts[0].path_fraction<1.);
        assert!(contact.end>0. && contact.end<dt);
        assert!(frame.transition_weight < (dt/0.125) as f32);
        assert!(scene.local(owner).unwrap().translation.x<=0.015625);
    } else {
        assert!(accepted.clip_phase(owner).unwrap()>runtime.clip_phase(owner).unwrap());
    }
    assert!(scene.local(owner).unwrap().translation.x>0.);
    }
}

#[test]
fn ordinary_preparation_transports_root_reference_without_foot_settings() {
    let (mut scene,owner,model,models) = fixture();
    scene.insert_component(owner,voxy_gameplay::CharacterBody {speed:0.,gravity:0.,..Default::default()}).unwrap();
    let mut runtime = AnimationRuntime::default().prepare(&scene,&models,1./60.).unwrap();
    runtime.initialize_root_reference(owner,&model,voxy_animation::RootRigidTransform::IDENTITY,
        voxy_animation::RootRigidEnclosure::IDENTITY).unwrap();
    assert!(!runtime.has_foot_placement());
    assert!(runtime.requires_pose_preparation());
    let candidate = runtime.prepare(&scene,&models,1./60.).unwrap();
    assert!(candidate.requires_pose_preparation());
    let mut physics = voxy_gameplay::CharacterPhysics::new(&scene,1,0);
    let mut input = voxy_gameplay::player_input().unwrap();
    let (_,accepted) = physics.fixed_step_with_preparation(&mut scene,&mut input,1./60.,
        &[(owner,Vec3::X*0.25)],&[],|preview,budget| candidate.clone().prepare_accepted_pose(preview,budget)).unwrap();
    let reference = accepted.owners[&owner].root_reference.unwrap();
    for bounds in reference.body_to_world.compose(&reference.authored_to_body).unwrap().translation_bounds() {
        assert!(bounds[0]<=0. && bounds[1]>=0.);
    }
    assert_eq!(runtime.owners[&owner].root_reference.unwrap().body_to_world.translation_bounds(),[[0.,0.];3]);
    assert_eq!(scene.local(owner).unwrap().translation.x,0.25);
}

#[test]
fn root_reference_initialization_rejects_stale_model_invalid_pose_and_reset() {
    let (scene,owner,model,models) = fixture();
    let mut runtime = AnimationRuntime::default().prepare(&scene,&models,1./60.).unwrap();
    let foreign = Arc::new(ModelAsset::parse(
        include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb"),
        &[],voxy_render::ModelLimits::default()).unwrap());
    let identity = voxy_animation::RootRigidTransform::IDENTITY;
    let reference = voxy_animation::RootRigidEnclosure::IDENTITY;
    assert!(runtime.prepare_owner_fade(&scene,&models,owner,&model,1./60.,0.01,0.01,4096).is_err());
    assert!(runtime.prepare_owner_fade(&scene,&models,owner,&foreign,1./60.,0.01,0.01,4096).is_err());
    assert!(runtime.initialize_root_reference_at_phase(owner,&model,identity,reference).is_err());
    assert!(runtime.initialize_root_reference(owner,&foreign,identity,reference).is_err());
    let invalid = voxy_animation::RootRigidTransform {translation:glam::DVec3::splat(f64::NAN),..identity};
    assert!(runtime.initialize_root_reference(owner,&model,invalid,reference).is_err());
    assert!(runtime.owners[&owner].root_reference.is_none());
    runtime.initialize_root_reference(owner,&model,identity,reference).unwrap();
    assert!(runtime.initialize_root_reference(owner,&model,identity,reference).is_err());
    assert_eq!(runtime.owners[&owner].root_reference.unwrap().authored_to_body.translation_bounds(),[[0.,0.];3]);
}

#[test]
fn root_reference_lifetime_tracks_asset_and_extraction_identity() {
    let (mut scene, owner, model, models) = fixture();
    scene.insert_component(owner, voxy_gameplay::CharacterBody {
        speed: 0., gravity: 0., ..Default::default()
    }).unwrap();
    let mut runtime = AnimationRuntime::default().prepare(&scene, &models, 1. / 60.).unwrap();
    runtime.initialize_root_reference(owner, &model,
        voxy_animation::RootRigidTransform::IDENTITY,
        voxy_animation::RootRigidEnclosure::IDENTITY).unwrap();
    let advanced = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
    assert!(advanced.owners[&owner].root_reference.is_some());
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_axes = [true, false, false];
    let changed_axes = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
    assert!(changed_axes.owners[&owner].root_reference.is_none());
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_axes = [false; 3];
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_rotation = true;
    let changed_rotation = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
    assert!(changed_rotation.owners[&owner].root_reference.is_none());
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_rotation = false;
    let replacement = Arc::new(ModelAsset::parse(
        include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb"),
        &[], voxy_render::ModelLimits::default()).unwrap());
    let replacement_models = BTreeMap::from([(AssetId("rig".into()), replacement)]);
    let reloaded = runtime.prepare(&scene, &replacement_models, 1. / 60.).unwrap();
    assert!(reloaded.owners[&owner].root_reference.is_none());
    assert!(runtime.owners[&owner].root_reference.is_some());
    assert!(runtime.prepare(&scene, &models, 1. / 60.).unwrap().owners[&owner].root_reference.is_some());
}

#[test]
fn owner_fade_operation_admits_all_owners_before_any_publication() {
    let (mut scene, first, model, models) = fixture();
    let second = scene.spawn(None,voxy_scene::Transform {
        translation: Vec3::X * 2., ..Default::default()
    }).unwrap();
    scene.insert_component(second,ModelInstance {asset:AssetId("rig".into())}).unwrap();
    for owner in [first,second] {
        scene.insert_component(owner,voxy_gameplay::CharacterBody {
            half_extents:[0.00390625;3],speed:0.,gravity:0.,..Default::default()
        }).unwrap();
        scene.insert_component(owner,ModelAnimation {
            root_motion_axes:[true,false,false],root_motion_rotation:true,..Default::default()
        }).unwrap();
    }
    let ordinary_owner = scene.spawn(None,voxy_scene::Transform {
        translation:Vec3::X*4.,..Default::default()
    }).unwrap();
    scene.insert_component(ordinary_owner,ModelInstance {asset:AssetId("rig".into())}).unwrap();
    scene.insert_component(ordinary_owner,voxy_gameplay::CharacterBody {
        half_extents:[0.00390625;3],speed:0.,gravity:0.,..Default::default()
    }).unwrap();
    scene.insert_component(ordinary_owner,ModelAnimation {
        root_motion_axes:[true,false,false],..Default::default()
    }).unwrap();
    let mut runtime = AnimationRuntime::default().prepare(&scene,&models,1./120.).unwrap();
    let old_ordinary_frame = runtime.frame(ordinary_owner,&model).unwrap();
    let old_motion = runtime.motions().iter().find(|(owner,_)| *owner==ordinary_owner).unwrap().1.x;

    for owner in [first,second] {
        runtime.owners.get_mut(&owner).unwrap().playback.transition_to_clip(0,0.125).unwrap();
        runtime.initialize_root_reference(owner,&model,voxy_animation::RootRigidTransform {
            translation:scene.local(owner).unwrap().translation.as_dvec3(),
            ..voxy_animation::RootRigidTransform::IDENTITY
        },voxy_animation::RootRigidEnclosure::IDENTITY).unwrap();
    }
    let first_frame = runtime.frame(first,&model).unwrap();
    let second_frame = runtime.frame(second,&model).unwrap();
    let dt = 1./60.;
    let a = runtime.prepare_owner_fade(&scene,&models,first,&model,dt,0.01,0.01,4096).unwrap().unwrap();
    let b = runtime.prepare_owner_fade(&scene,&models,second,&model,dt,0.01,0.01,4096).unwrap().unwrap();
    let frame = OwnerFadeAdmission {coordinate_axis:1,evaluation_radius:0.,evaluation_axes:None};
    let mut physics = voxy_gameplay::CharacterPhysics::new(&scene,3,0);
    let mut input = voxy_gameplay::player_input().unwrap();
    scene.component_mut::<ModelAnimation>(second).unwrap().unwrap().root_motion_axes=[false;3];
    assert!(runtime.fixed_step_owner_fades(&mut scene,&models,&mut physics,&mut input,
        dt,&[(&a,frame),(&b,frame)]).is_err());
    assert_eq!(scene.local(first).unwrap().translation,Vec3::ZERO);
    assert_eq!(scene.local(second).unwrap().translation,Vec3::X*2.);
    assert_eq!(scene.local(ordinary_owner).unwrap().translation,Vec3::X*4.);
    for owner in [first,second,ordinary_owner] {assert!(physics.state(&scene,owner).unwrap().is_none());}
    scene.component_mut::<ModelAnimation>(second).unwrap().unwrap().root_motion_axes=[true,false,false];
    let (receipts,accepted) = runtime.fixed_step_owner_fades(&mut scene,&models,&mut physics,
        &mut input,dt,&[(&a,frame),(&b,frame)]).unwrap();
    assert_eq!(receipts.len(),2);
    assert!(receipts.iter().all(|receipt|receipt.complete));
    assert!(scene.local(first).unwrap().translation.x>0.);
    assert!(scene.local(second).unwrap().translation.x>2.);
    let ordinary_motion = scene.local(ordinary_owner).unwrap().translation.x-4.;
    assert!((ordinary_motion-2.*old_motion).abs()<1e-6);
    assert!(accepted.clip_phase(ordinary_owner).unwrap()>runtime.clip_phase(ordinary_owner).unwrap());
    assert!(Arc::ptr_eq(&old_ordinary_frame,&runtime.frame(ordinary_owner,&model).unwrap()));
    assert!(!Arc::ptr_eq(&old_ordinary_frame,&accepted.frame(ordinary_owner,&model).unwrap()));
    assert!(Arc::ptr_eq(&first_frame,&runtime.frame(first,&model).unwrap()));
    assert!(Arc::ptr_eq(&second_frame,&runtime.frame(second,&model).unwrap()));
    for owner in [first,second] {
        assert!(accepted.clip_phase(owner).unwrap()>runtime.clip_phase(owner).unwrap());
    }
}

#[test]
fn certified_moving_fade_keeps_planted_sole_at_accepted_world_anchor() {
    use voxy_animation::{AnimationClip,JointTrack,Vec3Key,QuatKey,Playback};
    use crate::foot_placement::{ModelFootPlacement,FootBinding};
    use voxy_gameplay::FootContactSettings;
    for (blocked,interrupted) in [(false,false),(true,false),(false,true),(true,true)] {
    let original = gltf::binary::Glb::from_slice(include_bytes!(
        "../../../voxy_render/examples/assets/animated-triangle.glb")).unwrap();
    let mut json:serde_json::Value = serde_json::from_slice(&original.json).unwrap();
    json["nodes"] = serde_json::json!([
        {"name":"hip","translation":[0.,0.5,0.],"children":[1,3]},
        {"name":"knee","translation":[0.25,-0.7,0.],"children":[2]},
        {"name":"foot","translation":[-0.25,-0.7,0.]},{"mesh":0,"skin":0}]);
    json["skins"][0]["joints"] = serde_json::json!([0,1,2]);
    json["animations"] = serde_json::json!([]);
    let bytes = gltf::binary::Glb {header:original.header,
        json:serde_json::to_vec(&json).unwrap().into(),bin:original.bin}.to_vec().unwrap();
    let mut asset = ModelAsset::parse(&bytes,&[],voxy_render::ModelLimits::default()).unwrap();
    let mut tracks = vec![JointTrack::default();asset.skeleton.joints().len()];
    tracks[0].translations = vec![Vec3Key {time:0.,value:Vec3::Y*0.5},
        Vec3Key {time:1.,value:Vec3::Y*0.5+Vec3::X}];
    tracks[0].rotations = vec![QuatKey {time:0.,value:glam::Quat::IDENTITY},
        QuatKey {time:1.,value:glam::Quat::from_rotation_y(0.25)}];
    asset.animations.push(Arc::new(AnimationClip::new("walk",1.,Playback::Loop,
        tracks.clone(),&asset.skeleton).unwrap()));
    let mut faster = tracks.clone();
    faster[0].translations[1].value = Vec3::Y*0.5+Vec3::X*2.;
    faster[0].rotations[1].value = glam::Quat::from_rotation_y(0.5);
    asset.animations.push(Arc::new(AnimationClip::new("run",1.,Playback::Loop,
        faster,&asset.skeleton).unwrap()));
    let model = Arc::new(asset);
    let mut scene = SceneGraph::new(3);
    let owner = scene.spawn(None,voxy_scene::Transform {translation:Vec3::Y,..Default::default()}).unwrap();
    scene.insert_component(owner,ModelInstance {asset:AssetId("feet".into())}).unwrap();
    scene.insert_component(owner,ModelAnimation {root_motion_axes:[true,false,false],
        root_motion_rotation:true,..Default::default()}).unwrap();
    scene.insert_component(owner,voxy_gameplay::CharacterBody {half_extents:[0.125,1.,0.125],
        speed:0.,gravity:-9.8,..Default::default()}).unwrap();
    scene.insert_component(owner,ModelFootPlacement {feet:vec![FootBinding {
        bones:["hip".into(),"knee".into(),"foot".into()],sole_offset:[0.,-0.1,0.],
        sole_up:[0.,1.,0.],pole:[1.,0.,0.],plant:true,weight:1.,contact:FootContactSettings::default(),
        contact_curve:vec![],clip_contact_curves:Default::default(),
    }]}).unwrap();
    let floor = scene.spawn(None,voxy_scene::Transform {translation:-Vec3::Y*0.5,..Default::default()}).unwrap();
    scene.insert_component(floor,voxy_gameplay::BoxCollider {half_extents:[4.,0.5,4.]}).unwrap();
    let models = BTreeMap::from([(AssetId("feet".into()),model.clone())]);
    let mut runtime = AnimationRuntime::default().prepare(&scene,&models,1./120.).unwrap();
    let mut physics = voxy_gameplay::CharacterPhysics::new(&scene,1,2);
    let mut input = voxy_gameplay::player_input().unwrap();
    assert!(runtime.capture_owner_reference_from_parent(&scene,&models,&physics,owner,&model).is_err());
    assert!(runtime.owners[&owner].root_reference.is_none());
    runtime = physics.fixed_step_with_preparation(&mut scene,&mut input,1./120.,&[],&[],
        |preview,budget|runtime.clone().prepare_accepted_pose(preview,budget)).unwrap().1;
    assert!(physics.state(&scene,owner).unwrap().unwrap().grounded);
    assert!(runtime.owners[&owner].root_reference.is_some());
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().clip = Some(1);
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().transition_seconds = 0.125;
    let before_selection = runtime.frame(owner,&model).unwrap();
    let before_source_phase = runtime.clip_phase(owner).unwrap();
    let before_selection_serial = runtime.serial();
    scene.component_mut::<ModelFootPlacement>(owner).unwrap().unwrap().feet[0].plant = false;
    assert!(runtime.stage_owner_selection(&scene,&models,owner,&model).is_err());
    assert!(Arc::ptr_eq(&before_selection,&runtime.frame(owner,&model).unwrap()));
    scene.component_mut::<ModelFootPlacement>(owner).unwrap().unwrap().feet[0].plant = true;
    runtime = runtime.stage_owner_selection(&scene,&models,owner,&model).unwrap();
    assert!(Arc::ptr_eq(&before_selection,&runtime.frame(owner,&model).unwrap()));
    assert_eq!(runtime.serial(),before_selection_serial);
    let blend = runtime.owners[&owner].playback.pose_blend_phases().unwrap();
    match blend.source.unwrap() {
        voxy_animation::PoseBlendSource::Clip(source) => assert_eq!(source.normalized_phase,before_source_phase),
        _ => panic!("first selection must retain the running clip"),
    }
    let sole = |frame:&AnimatorFrame,world:glam::Mat4| {
        let mut globals:Vec<glam::Mat4> = Vec::new();
        for (local,joint) in frame.pose.local().iter().zip(model.skeleton.joints()) {
            globals.push(joint.parent.map_or(local.matrix(),|p|globals[usize::from(p)]*local.matrix()));
        }
        world.transform_point3(globals[2].transform_point3(Vec3::new(0.,-0.1,0.)))
    };
    let original_frame = runtime.frame(owner,&model).unwrap();
    let anchor = sole(&original_frame,scene.world_matrix(owner).unwrap());
    let staged = runtime.prepare_owner_fade(&scene,&models,owner,&model,1./60.,0.001,0.001,4096).unwrap().unwrap();
    scene.component_mut::<ModelFootPlacement>(owner).unwrap().unwrap().feet[0].plant = false;
    assert!(staged.admit_scene(&scene,&models).is_err());
    scene.component_mut::<ModelFootPlacement>(owner).unwrap().unwrap().feet[0].plant = true;
    let (receipts,accepted) = runtime.fixed_step_owner_fades(&mut scene,&models,&mut physics,
        &mut input,1./60.,&[(&staged,OwnerFadeAdmission {coordinate_axis:1,evaluation_radius:0.,evaluation_axes:None})]).unwrap();
    assert!(receipts[0].complete);
    assert!(scene.local(owner).unwrap().translation.x>0.);
    assert!(scene.local(owner).unwrap().rotation != glam::Quat::IDENTITY);
    let frame = accepted.frame(owner,&model).unwrap();
    assert!(sole(&frame,scene.world_matrix(owner).unwrap()).abs_diff_eq(anchor,3e-6), "anchor={anchor:?}, actual={:?}, grounded={:?}",sole(&frame,scene.world_matrix(owner).unwrap()),physics.state(&scene,owner).unwrap());
    assert_eq!(accepted.owners[&owner].playback.contact_interval().unwrap().end,accepted.clip_phase(owner).unwrap());
    assert!(Arc::ptr_eq(&original_frame,&runtime.frame(owner,&model).unwrap()));
    if blocked {
        let wall = scene.spawn(None,voxy_scene::Transform {
            translation:Vec3::new(0.1640625,1.,0.),..Default::default()
        }).unwrap();
        scene.insert_component(wall,voxy_gameplay::BoxCollider {
            half_extents:[0.015625,2.,2.]
        }).unwrap();
    }
    let mut clipped = false;
    let mut partial_advance = false;
    let mut accepted = accepted;
    for step in 0..7 {
        if interrupted && step == 2 {
            scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().clip = Some(0);
            scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().transition_seconds = 0.03;
            accepted = accepted.stage_owner_selection(&scene,&models,owner,&model).unwrap();
            assert!(matches!(accepted.owners[&owner].playback.pose_blend_phases().unwrap().source,
                Some(voxy_animation::PoseBlendSource::FrozenPose(_))));
        }
        let Some(staged) = accepted.prepare_owner_fade(&scene,&models,owner,&model,1./60.,0.001,0.001,4096)
            .unwrap() else {assert!(interrupted && !blocked);break;};
        if interrupted && step == 2 {
            let before_pose = scene.local(owner).unwrap();
            let before_frame = accepted.frame(owner,&model).unwrap();
            let before_phase = accepted.clip_phase(owner).unwrap();
            let before_body = physics.state(&scene,owner).unwrap().unwrap();
            physics = physics.with_angular_trajectory_query_budget(1).unwrap();
            assert!(accepted.fixed_step_owner_fades(&mut scene,&models,&mut physics,&mut input,
                1./60.,&[(&staged,OwnerFadeAdmission {coordinate_axis:1,evaluation_radius:0.,evaluation_axes:None})]).is_err());
            assert_eq!(scene.local(owner).unwrap(),before_pose);
            assert!(Arc::ptr_eq(&before_frame,&accepted.frame(owner,&model).unwrap()));
            assert_eq!(accepted.clip_phase(owner).unwrap(),before_phase);
            assert_eq!(physics.state(&scene,owner).unwrap().unwrap().body,before_body.body);
            physics = physics.with_angular_trajectory_query_budget(65536).unwrap();
        }
        let prior_phase = accepted.clip_phase(owner).unwrap();
        let (receipts,next) = accepted.fixed_step_owner_fades(&mut scene,&models,&mut physics,&mut input,
            1./60.,&[(&staged,OwnerFadeAdmission {coordinate_axis:1,evaluation_radius:0.,evaluation_axes:None})]).unwrap();
        if !receipts[0].complete {
            clipped = true;
            partial_advance |= next.clip_phase(owner).unwrap() > prior_phase;
            assert!(next.clip_phase(owner).unwrap() < prior_phase+1./60.);
        }
        if interrupted && step == 2 {
            assert!(next.owners[&owner].playback.frozen_source_tick().is_some());
        }
        accepted = next;
        assert!(sole(&accepted.frame(owner,&model).unwrap(),scene.world_matrix(owner).unwrap())
            .abs_diff_eq(anchor,3e-6));
        assert_eq!(accepted.owners[&owner].playback.contact_interval().unwrap().end,
            accepted.clip_phase(owner).unwrap());
    }
    assert_eq!(clipped,blocked);
    assert_eq!(partial_advance,blocked);
    if blocked {
        assert!(accepted.frame(owner,&model).unwrap().transition_weight<1.);
        let matrix = scene.world_matrix(owner).unwrap().as_dmat4();
        let extent = matrix.x_axis.x.abs()*0.125+matrix.y_axis.x.abs()+matrix.z_axis.x.abs()*0.125;
        assert!(matrix.w_axis.x+extent <= 0.1484375);
    } else {
        assert_eq!(accepted.frame(owner,&model).unwrap().transition_weight,1.);
    }
    }
}

#[test]
fn signed_reference_scale_survives_body_transport_and_next_fade() {
    for scale in [0.5,-2.] {
        let (mut scene,owner,original,_) = fixture();
        let mut asset = (*original).clone();
        asset.animations.push(asset.animations[0].clone());
        let model = Arc::new(asset);
        let models = BTreeMap::from([(AssetId("rig".into()),model.clone())]);
        scene.insert_component(owner,ModelAnimation {root_motion_rotation:true,
            root_motion_axes:[true,false,false],..Default::default()}).unwrap();
        scene.insert_component(owner,voxy_gameplay::CharacterBody {half_extents:[0.00390625;3],
            speed:0.,gravity:0.,..Default::default()}).unwrap();
        let mut runtime = AnimationRuntime::default().prepare(&scene,&models,0.0625).unwrap();
        let mut physics = voxy_gameplay::CharacterPhysics::new(&scene,1,0);
        let mut input = voxy_gameplay::player_input().unwrap();
        physics.fixed_step_with_preparation(&mut scene,&mut input,0.0625,&[],&[],
            |_,_|Ok::<_,String>(())).unwrap();
        for invalid in [0.,f64::NAN,f64::INFINITY] {
            assert!(runtime.capture_owner_reference_with_scale(&scene,&models,&physics,owner,
                &model,voxy_animation::RootRigidEnclosure::IDENTITY,invalid).is_err());
            assert!(runtime.owners[&owner].root_reference.is_none());
        }
        runtime = runtime.capture_owner_reference_with_scale(&scene,&models,&physics,owner,
            &model,voxy_animation::RootRigidEnclosure::IDENTITY,scale).unwrap();
        let anchor = runtime.owners[&owner].root_reference.unwrap().authored_to_body;
        scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().clip=Some(1);
        scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().transition_seconds=0.125;
        runtime = runtime.stage_owner_selection(&scene,&models,owner,&model).unwrap();
        for step in 1..=2 {
            let staged = runtime.prepare_owner_fade(&scene,&models,owner,&model,0.0625,0.001,0.001,4096)
                .unwrap().unwrap();
            let admitted=staged.admit_scene(&scene,&models).unwrap();
            let request=admitted.request(1,0.);
            assert_eq!(request.basis,glam::DQuat::IDENTITY);
            assert_eq!(request.origin,Vec3::ZERO);
            assert_eq!(request.scale,1.);
            let (receipts,accepted) = runtime.fixed_step_owner_fades(&mut scene,&models,&mut physics,
                &mut input,0.0625,&[(&staged,OwnerFadeAdmission {coordinate_axis:1,evaluation_radius:0.,evaluation_axes:None})]).unwrap();
            assert!(receipts[0].complete);
            assert!((f64::from(scene.local(owner).unwrap().translation.x)-scale*2.*0.0625*f64::from(step)).abs()<1e-6);
            let reference = accepted.owners[&owner].root_reference.unwrap();
            assert_eq!(reference.scale.bounds(),[scale,scale]);
            let world = reference.body_to_world.compose(&reference.authored_to_body).unwrap();
            for (bounds,expected) in world.translation_bounds().into_iter().zip(anchor.translation_bounds()) {
                assert!(bounds[0]<=expected[0] && bounds[1]>=expected[1]);
            }
            runtime = accepted;
        }
    }
}

#[test]
fn root_parent_capture_encloses_nested_reflections_and_rejects_moving_frames() {
    let original = gltf::binary::Glb::from_slice(include_bytes!(
        "../../../voxy_render/examples/assets/animated-triangle.glb")).unwrap();
    let mut json:serde_json::Value = serde_json::from_slice(&original.json).unwrap();
    json["nodes"].as_array_mut().unwrap().extend([
        serde_json::json!({"name":"inner","children":[0],"translation":[0.25,0.5,-0.25],
            "rotation":[0.,0.,1.,0.],"scale":[-0.5,0.5,0.5]}),
        serde_json::json!({"name":"outer","children":[2],"translation":[2.,-1.,0.5],
            "rotation":[0.,1.,0.,0.],"scale":[2.,2.,2.]})]);
    let parse = |json:&serde_json::Value| {
        let bytes = gltf::binary::Glb {header:original.header,
            json:serde_json::to_vec(json).unwrap().into(),bin:original.bin.clone()}.to_vec().unwrap();
        ModelAsset::parse(&bytes,&[],voxy_render::ModelLimits::default()).unwrap()
    };
    let model = parse(&json);
    let root = model.resolve_joint_name("root").unwrap();
    let (frame,scale) = constant_parent_similarity_enclosure(&model,root).unwrap();
    assert!(scale.bounds()[0]<=-1. && scale.bounds()[1]>=-1.);
    // The two exact half-turns and dyadic scales produce p=(1.5,0,1),
    // signed scale -1 and proper identity rotation. No floating matrix oracle.
    for point in [glam::DVec3::ZERO,glam::DVec3::X,glam::DVec3::new(0.25,0.5,-0.125)] {
        let point_frame = voxy_animation::RootRigidEnclosure::from_transform(
            voxy_animation::RootRigidTransform {translation:point,..voxy_animation::RootRigidTransform::IDENTITY})
            .unwrap().with_translation_scale_enclosed(scale).unwrap();
        let actual = frame.compose(&point_frame).unwrap();
        let expected = glam::DVec3::new(1.5,0.,1.)-point;
        for (bounds,value) in actual.translation_bounds().into_iter().zip(expected.to_array()) {
            assert!(bounds[0]<=value && value<=bounds[1]);
        }
    }
    json["nodes"][2]["scale"] = serde_json::json!([-0.5,0.25,0.5]);
    let nonuniform = parse(&json);
    assert!(constant_parent_similarity_enclosure(&nonuniform,nonuniform.resolve_joint_name("root").unwrap()).is_err());
    json["nodes"][2]["scale"] = serde_json::json!([-0.5,0.5,0.5]);
    json["animations"][0]["channels"][0]["target"]["node"] = serde_json::json!(2);
    let moving = parse(&json);
    assert!(constant_parent_similarity_enclosure(&moving,moving.resolve_joint_name("root").unwrap()).is_err());
}

#[test]
fn first_accepted_pose_bootstraps_root_reference_atomically_without_feet() {
    let (mut scene,owner,model,models) = fixture();
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().root_motion_rotation = true;
    scene.insert_component(owner,voxy_gameplay::CharacterBody {speed:0.,gravity:0.,..Default::default()}).unwrap();
    let runtime = AnimationRuntime::default().prepare(&scene,&models,1./60.).unwrap();
    assert!(!runtime.has_foot_placement());
    assert!(runtime.requires_pose_preparation());
    assert!(runtime.owners[&owner].root_reference.is_none());
    let before = scene.local(owner).unwrap();
    let frame = runtime.frame(owner,&model).unwrap();
    let mut physics = voxy_gameplay::CharacterPhysics::new(&scene,1,0);
    let mut input = voxy_gameplay::player_input().unwrap();
    let failed = physics.fixed_step_with_preparation(&mut scene,&mut input,1./60.,
        &[(owner,Vec3::X*0.25)],&[],|preview,budget| {
            let candidate = runtime.clone().prepare_accepted_pose(preview,budget)?;
            assert!(candidate.owners[&owner].root_reference.is_some());
            Err::<AnimationRuntime,String>("reject after root initialization".into())
        });
    assert!(failed.is_err());
    assert_eq!(scene.local(owner).unwrap(),before);
    assert!(physics.accepted_pose(&scene,owner).unwrap().is_none());
    assert!(runtime.owners[&owner].root_reference.is_none());
    assert!(Arc::ptr_eq(&frame,&runtime.frame(owner,&model).unwrap()));
    let (_,accepted) = physics.fixed_step_with_preparation(&mut scene,&mut input,1./60.,
        &[(owner,Vec3::X*0.25)],&[],|preview,budget|
            runtime.clone().prepare_accepted_pose(preview,budget)).unwrap();
    let reference = accepted.owners[&owner].root_reference.unwrap();
    let pose = physics.accepted_pose(&scene,owner).unwrap().unwrap();
    for (bound,value) in reference.body_to_world.translation_bounds().into_iter()
        .zip(pose.physical_center.to_array()) {
        assert!(bound[0] <= value && value <= bound[1]);
    }
    assert!(runtime.owners[&owner].root_reference.is_none());
}

#[test]
fn scene_fade_batch_stages_selection_without_advancing_and_rejects_late_owner() {
    let (mut scene,first,original,_) = fixture();
    let mut asset = (*original).clone();
    asset.animations.push(Arc::new(voxy_animation::AnimationClip::new("second",1.,
        voxy_animation::Playback::Loop,vec![voxy_animation::JointTrack::default();
            asset.skeleton.joints().len()],&asset.skeleton).unwrap()));
    let model = Arc::new(asset);
    let models = BTreeMap::from([(AssetId("rig".into()),model.clone())]);
    let second = scene.spawn(None,voxy_scene::Transform {translation:Vec3::X*2.,..Default::default()}).unwrap();
    scene.insert_component(second,ModelInstance {asset:AssetId("rig".into())}).unwrap();
    for owner in [first,second] {
        scene.insert_component(owner,voxy_gameplay::CharacterBody {speed:0.,gravity:0.,..Default::default()}).unwrap();
        scene.insert_component(owner,ModelAnimation {root_motion_rotation:true,
            root_motion_axes:[true,false,false],..Default::default()}).unwrap();
    }
    let mut runtime = AnimationRuntime::default().prepare(&scene,&models,1./60.).unwrap();
    for owner in [first,second] {
        runtime.initialize_root_reference(owner,&model,voxy_animation::RootRigidTransform {
            translation:scene.local(owner).unwrap().translation.as_dvec3(),
            ..voxy_animation::RootRigidTransform::IDENTITY},
            voxy_animation::RootRigidEnclosure::IDENTITY).unwrap();
        let settings = scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap();
        settings.clip = Some(1);
        settings.transition_seconds = 0.125;
    }
    let phase = runtime.clip_phase(first).unwrap();
    let frame = runtime.frame(first,&model).unwrap();
    let (candidate,plans) = runtime.prepare_scene_fades(&scene,&models,1./60.,0.01,0.01,4096).unwrap();
    assert_eq!(plans.len(),2);
    assert_eq!(candidate.clip_phase(first).unwrap(),0.);
    assert_eq!(candidate.serial(),runtime.serial());
    assert!(Arc::ptr_eq(&frame,&candidate.frame(first,&model).unwrap()));
    assert_eq!(runtime.owners[&first].settings.clip,Some(0));
    assert_eq!(candidate.owners[&first].settings.clip,Some(1));
    scene.component_mut::<ModelAnimation>(second).unwrap().unwrap().clip = Some(99);
    assert!(runtime.prepare_scene_fades(&scene,&models,1./60.,0.01,0.01,4096).is_err());
    assert_eq!(runtime.owners[&first].settings.clip,Some(0));
    assert_eq!(runtime.clip_phase(first).unwrap(),phase);
    assert!(Arc::ptr_eq(&frame,&runtime.frame(first,&model).unwrap()));
    scene.component_mut::<ModelAnimation>(second).unwrap().unwrap().clip = Some(1);
    let request = OwnerFadeAdmission {coordinate_axis:1,evaluation_radius:0.,evaluation_axes:None};
    let mut physics = voxy_gameplay::CharacterPhysics::new(&scene,2,0);
    let mut input = voxy_gameplay::player_input().unwrap();
    let before = scene.local(first).unwrap();
    assert!(runtime.fixed_step_scene_fades(&mut scene,&models,&mut physics,&mut input,
        1./60.,0.01,0.01,4096,&BTreeMap::from([(first,request)])).is_err());
    assert_eq!(scene.local(first).unwrap(),before);
    assert!(physics.accepted_pose(&scene,first).unwrap().is_none());
    let (receipts,accepted) = runtime.fixed_step_scene_fades(&mut scene,&models,
        &mut physics,&mut input,1./60.,0.01,0.01,4096,
        &BTreeMap::from([(first,request),(second,request)])).unwrap();
    assert_eq!(receipts.len(),2);
    for owner in [first,second] {
        assert!(physics.accepted_pose(&scene,owner).unwrap().is_some());
        assert_eq!(accepted.owners[&owner].settings.clip,Some(1));
        assert!(accepted.clip_phase(owner).unwrap()>0.);
        assert_eq!(runtime.owners[&owner].settings.clip,Some(0));
    }
    assert!(!Arc::ptr_eq(&frame,&accepted.frame(first,&model).unwrap()));
}

#[test]
fn rigid_bind_pose_waits_for_clip_before_reference_bootstrap() {
    let (mut scene,owner,model,models) = fixture();
    scene.insert_component(owner,voxy_gameplay::CharacterBody {speed:0.,gravity:0.,..Default::default()}).unwrap();
    scene.insert_component(owner,ModelAnimation {clip:None,root_motion_rotation:true,
        root_motion_axes:[true,false,false],..Default::default()}).unwrap();
    let runtime = AnimationRuntime::default().prepare(&scene,&models,1./60.).unwrap();
    assert!(!runtime.requires_pose_preparation());
    let mut physics = voxy_gameplay::CharacterPhysics::new(&scene,1,0);
    let mut input = voxy_gameplay::player_input().unwrap();
    let (_,runtime) = physics.fixed_step_with_preparation(&mut scene,&mut input,1./60.,
        &[],&[],|preview,budget|runtime.clone().prepare_accepted_pose(preview,budget)).unwrap();
    assert!(runtime.owners[&owner].root_reference.is_none());
    assert_eq!(runtime.frame(owner,&model).unwrap().pose.local(),model.skeleton.bind_pose().local());
    scene.component_mut::<ModelAnimation>(owner).unwrap().unwrap().clip = Some(0);
    let candidate = runtime.prepare(&scene,&models,1./60.).unwrap();
    assert!(candidate.requires_pose_preparation());
    let (_,accepted) = physics.fixed_step_with_preparation(&mut scene,&mut input,1./60.,
        candidate.motions(),&candidate.trajectories(),|preview,budget|
            candidate.clone().prepare_accepted_pose(preview,budget)).unwrap();
    assert!(accepted.owners[&owner].root_reference.is_some());
    assert!(runtime.owners[&owner].root_reference.is_none());
}

#[test]
fn imported_model_retains_translation_and_rotation_compilation_proofs() {
    let (_,_,model,_) = fixture();
    let clip = &model.animations[0];
    let curve = clip.root_rigid_curve(0).unwrap();
    assert!(curve.translation_compilation_error_bounds().unwrap().iter().all(|error|error.is_finite()));
    assert!(curve.rotation_key_normalization_error_bounds().iter().all(|error|error.is_finite()));
    for step in 0..=32 {
        let phase = f64::from(clip.duration())*f64::from(step)/32.;
        let error = curve.translation_phase_evaluation_error_bounds(phase).unwrap();
        assert!(error.iter().all(|error|error.is_finite() && *error<1e-12));
        assert_eq!(error[1],0.);
        assert_eq!(error[2],0.);
    }
    assert!(curve.rotation_cubic_phase_evaluation_error_bounds(0.25).unwrap().is_none());
}
