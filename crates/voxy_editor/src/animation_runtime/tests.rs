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
