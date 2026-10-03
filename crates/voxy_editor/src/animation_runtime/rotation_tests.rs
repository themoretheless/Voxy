use super::*;
use glam::{DQuat, Quat, Vec3};
use voxy_gameplay::{BoxCollider, CharacterBody, CharacterPhysics, PhysicsError};
fn model() -> Arc<ModelAsset> {
    Arc::new(
        ModelAsset::parse(
            include_bytes!("../../../voxy_render/examples/assets/root-pivot-turn.glb"),
            &[],
            voxy_render::ModelLimits::default(),
        )
        .unwrap(),
    )
}
fn scene_fixture() -> (
    SceneGraph,
    NodeId,
    Arc<ModelAsset>,
    BTreeMap<AssetId, Arc<ModelAsset>>,
) {
    let model = model();
    let mut scene = SceneGraph::new(8);
    let owner = scene.spawn(None, Default::default()).unwrap();
    let asset = AssetId("turn".into());
    scene
        .insert_component(
            owner,
            ModelInstance {
                asset: asset.clone(),
            },
        )
        .unwrap();
    scene
        .insert_component(
            owner,
            ModelAnimation {
                root_motion_rotation: true,
                root_motion_bone: "root".into(),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            owner,
            CharacterBody {
                half_extents: [0.4, 0.1, 0.02],
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let wall = scene
        .spawn(
            None,
            voxy_scene::Transform {
                translation: Vec3::Z * 0.25,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [2., 2., 0.02],
            },
        )
        .unwrap();
    (
        scene,
        owner,
        model.clone(),
        BTreeMap::from([(asset, model)]),
    )
}
#[test]
fn prepared_rotation_and_translation_only_owners_publish_one_atomic_tick() {
    let (mut scene, owner, model, mut models) = scene_fixture();
    let walking = Arc::new(
        ModelAsset::parse(
            include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb"),
            &[],
            voxy_render::ModelLimits::default(),
        )
        .unwrap(),
    );
    let walk = scene
        .spawn(
            None,
            voxy_scene::Transform {
                translation: Vec3::X * 10.,
                ..Default::default()
            },
        )
        .unwrap();
    let id = AssetId("walk".into());
    scene
        .insert_component(walk, ModelInstance { asset: id.clone() })
        .unwrap();
    scene
        .insert_component(
            walk,
            ModelAnimation {
                root_motion_axes: [true, false, false],
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            walk,
            CharacterBody {
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    models.insert(id, walking);
    let runtime = AnimationRuntime::default();
    let candidate = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
    let paths = candidate.trajectories();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].pivot, Vec3::X * 0.6);
    assert_eq!(
        candidate.frame(owner, &model).unwrap().pose,
        model.skeleton.bind_pose()
    );
    let before = [scene.local(owner).unwrap(), scene.local(walk).unwrap()];
    let mut input = voxy_gameplay::player_input().unwrap();
    input.event(voxy_gameplay::JUMP, 1.).unwrap();
    let mut limited = CharacterPhysics::new(&scene, 2, 1)
        .with_angular_trajectory_query_budget(1)
        .unwrap();
    assert_eq!(
        limited
            .fixed_step_with_motion_and_trajectories(
                &mut scene,
                &mut input,
                1. / 60.,
                candidate.motions(),
                &paths
            )
            .unwrap_err(),
        PhysicsError::SweepBudget
    );
    assert_eq!(runtime.serial(), 0);
    assert!(runtime.frame(owner, &model).is_none());
    assert!(input.state("jump").unwrap().pressed);
    assert_eq!(scene.local(owner).unwrap(), before[0]);
    assert_eq!(scene.local(walk).unwrap(), before[1]);
    let mut physics = CharacterPhysics::new(&scene, 2, 1);
    let receipt = physics
        .fixed_step_with_motion_and_trajectories(
            &mut scene,
            &mut input,
            1. / 60.,
            candidate.motions(),
            &paths,
        )
        .unwrap()[0];
    assert!(!receipt.complete);
    assert!((scene.local(walk).unwrap().translation.x - (10. + 2. / 60.)).abs() < 1e-6);
    assert!(!input.state("jump").unwrap().pressed);
    let mut runtime = candidate;
    for _ in 0..11 {
        let next = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
        physics
            .fixed_step_with_motion_and_trajectories(
                &mut scene,
                &mut input,
                1. / 60.,
                next.motions(),
                &next.trajectories(),
            )
            .unwrap();
        runtime = next;
    }
    let angle = (0.23 / 1_f64.hypot(0.02)).asin() - 0.02_f64.atan2(1.);
    assert!(
        scene
            .local(owner)
            .unwrap()
            .rotation
            .abs_diff_eq(Quat::from_rotation_y(angle as f32), 1e-6)
    );
    assert_eq!(runtime.serial(), 12);
    assert_eq!(
        runtime.frame(owner, &model).unwrap().pose,
        model.skeleton.bind_pose()
    );
    let conflicts = [(owner, Vec3::X)];
    let pose = scene.local(owner).unwrap();
    assert_eq!(
        physics
            .fixed_step_with_motion_and_trajectories(
                &mut scene,
                &mut input,
                1. / 60.,
                &conflicts,
                &runtime.trajectories()
            )
            .unwrap_err(),
        PhysicsError::InvalidMotion
    );
    assert_eq!(scene.local(owner).unwrap(), pose);
}
#[test]
fn ordinary_play_rotates_from_rig_limits_at_wall_and_stop_restores_authoring() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/root-pivot-turn.glb");
    let mut app = crate::App::new(&path, false).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
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
            CharacterBody {
                half_extents: [0.4, 0.1, 0.02],
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
                root_motion_rotation: true,
                root_motion_bone: "root".into(),
                ..Default::default()
            },
        )
        .unwrap();
    app.scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [2., 2., 0.02],
            },
        )
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
                translation: app.scene.local(owner).unwrap().translation + Vec3::Z * 0.25,
                ..Default::default()
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    let authored = app.authoring_document().unwrap();
    let start = app.scene.local(owner).unwrap();
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
    for _ in 0..12 {
        app.advance_game(1. / 60.).unwrap();
    }
    let angle = (0.23 / 1_f64.hypot(0.02)).asin() - 0.02_f64.atan2(1.);
    let pose = app.scene.local(owner).unwrap();
    assert!(
        pose.rotation
            .abs_diff_eq(Quat::from_rotation_y(angle as f32), 1e-6)
    );
    let center = Vec3::new(
        (0.6 * (1. - angle.cos())) as f32,
        0.,
        (0.6 * angle.sin()) as f32,
    );
    assert!(
        pose.translation
            .abs_diff_eq(start.translation + center, 1e-6)
    );
    let frame = app.play.animations.frame(owner, &model).unwrap();
    assert_eq!(frame.pose, model.skeleton.bind_pose());
    assert_eq!(app.play.animations.serial(), 12);
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &authored);
    app.scene
        .component_mut::<BoxCollider>(wall)
        .unwrap()
        .unwrap()
        .half_extents[0] = f32::NAN;
    app.play
        .player_input
        .event(voxy_gameplay::JUMP, 1.)
        .unwrap();
    assert!(app.advance_game(1. / 60.).is_err());
    assert_eq!(app.scene.local(owner).unwrap(), pose);
    assert_eq!(app.play.animations.serial(), 12);
    assert!(Arc::ptr_eq(
        &frame,
        &app.play.animations.frame(owner, &model).unwrap()
    ));
    assert!(app.play.player_input.state("jump").unwrap().pressed);
    app.scene
        .component_mut::<BoxCollider>(wall)
        .unwrap()
        .unwrap()
        .half_extents[0] = 2.;
    app.advance_game(1. / 60.).unwrap();
    assert_eq!(app.play.animations.serial(), 13);
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), authored);
    assert_eq!(app.play.animations.serial(), 0);
    app.stop_workers().unwrap();
}
#[test]
fn old_authored_settings_default_rotation_off() {
    let settings: ModelAnimation =
        serde_json::from_str(r#"{"clip":0,"speed":1,"root_motion_axes":[true,false,false]}"#)
            .unwrap();
    assert!(!settings.root_motion_rotation);
    let mut value = serde_json::to_value(settings).unwrap();
    value["root_motion_rotation"] = serde_json::json!(true);
    assert!(
        serde_json::from_value::<ModelAnimation>(value)
            .unwrap()
            .root_motion_rotation
    );
}

#[test]
fn signed_uniform_parent_converts_axes_and_nonrigid_or_moving_pivots_reject() {
    use voxy_animation::{
        AnimationClip, Interpolation, Joint, JointTangents, JointTrack, Playback, QuatKey,
        Skeleton, TrackInterpolation, Transform, Vec3Key,
    };
    let base = model();
    let build = |scale: Vec3, moving: bool, moving_parent: bool| {
        let mut asset = (*base).clone();
        let parent = Transform {
            translation: Vec3::new(1., 2., 0.),
            rotation: Quat::from_rotation_z(0.3),
            scale,
        };
        asset.skeleton = Skeleton::new(vec![
            Joint {
                name: Arc::from("basis"),
                parent: None,
                bind_local: parent,
                inverse_bind: glam::Mat4::IDENTITY,
            },
            Joint {
                name: Arc::from("root"),
                parent: Some(0),
                bind_local: Transform {
                    translation: Vec3::X * 0.6,
                    ..Transform::IDENTITY
                },
                inverse_bind: glam::Mat4::IDENTITY,
            },
        ])
        .unwrap();
        let mut tracks = vec![JointTrack::default(); 2];
        tracks[1].rotations = vec![
            QuatKey {
                time: 0.,
                value: Quat::IDENTITY,
            },
            QuatKey {
                time: 1.,
                value: Quat::from_rotation_y(1.),
            },
        ];
        if moving_parent {
            tracks[0].translations = vec![
                Vec3Key {
                    time: 0.,
                    value: parent.translation,
                },
                Vec3Key {
                    time: 1.,
                    value: parent.translation + Vec3::Y,
                },
            ];
        }
        let mut modes = vec![TrackInterpolation::default(); 2];
        let mut tangents = vec![JointTangents::default(); 2];
        if moving {
            // Equal translation endpoints do not prove a fixed pivot.
            tracks[1].translations = vec![
                Vec3Key {
                    time: 0.,
                    value: Vec3::X * 0.6,
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::X * 0.6,
                },
            ];
            modes[1].translation = Interpolation::CubicSpline;
            tangents[1].translation = vec![[Vec3::ZERO, Vec3::Y], [-Vec3::Y, Vec3::ZERO]];
        }
        asset.animations = vec![Arc::new(
            AnimationClip::new_with_tangents(
                "turn",
                1.,
                Playback::Clamp,
                tracks,
                modes,
                tangents,
                &asset.skeleton,
            )
            .unwrap(),
        )];
        Arc::new(asset)
    };
    let model = build(Vec3::new(-2., 2., 2.), false, false);
    let mut scene = SceneGraph::new(2);
    let owner = scene.spawn(None, Default::default()).unwrap();
    let id = AssetId("turn".into());
    scene
        .insert_component(owner, ModelInstance { asset: id.clone() })
        .unwrap();
    scene
        .insert_component(
            owner,
            ModelAnimation {
                root_motion_rotation: true,
                root_motion_joint: 1,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            owner,
            CharacterBody {
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let runtime = AnimationRuntime::default();
    let models = BTreeMap::from([(id.clone(), model.clone())]);
    let prepared = runtime.prepare(&scene, &models, 0.1).unwrap();
    let paths = prepared.trajectories();
    let parent = model.skeleton.joints()[0].bind_local;
    assert!(
        paths[0]
            .pivot
            .abs_diff_eq(parent.matrix().transform_point3(Vec3::X * 0.6), 1e-6)
    );
    let expected_axis = DQuat::from_rotation_z(0.3) * -glam::DVec3::Y;
    let delta = paths[0].basis * paths[0].rotation.end_rotation() * paths[0].basis.conjugate();
    assert!(delta.abs_diff_eq(
        DQuat::from_axis_angle(expected_axis, f64::from(0.1_f32)),
        1e-7
    ));
    let frame = prepared.frame(owner, &model).unwrap();
    assert_eq!(frame.pose.local()[1].rotation, Quat::IDENTITY);
    for (scale, moving, moving_parent, message) in [
        (Vec3::new(-2., 3., 2.), false, false, "nonuniform scale"),
        (Vec3::new(-2., 2., 2.), true, false, "moving pivot"),
        (Vec3::new(-2., 2., 2.), false, true, "parent is moving"),
    ] {
        let unsupported = build(scale, moving, moving_parent);
        assert!(
            prepared
                .prepare(&scene, &BTreeMap::from([(id.clone(), unsupported)]), 0.1)
                .unwrap_err()
                .contains(message)
        );
        assert_eq!(prepared.serial(), 1);
        assert!(Arc::ptr_eq(&frame, &prepared.frame(owner, &model).unwrap()));
    }
}
