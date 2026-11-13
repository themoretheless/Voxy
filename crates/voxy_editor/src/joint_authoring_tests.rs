use crate::joint_authoring::SCHEMA;
use crate::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use voxy_gameplay::{PhysicsJoint, PhysicsJointKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "voxy-joint-authoring-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// New project whose objects sit at the given world positions, in order.
fn project_with(
    temp: &Temp,
    positions: &[[f32; 3]],
) -> Result<(PathBuf, App), Box<dyn std::error::Error>> {
    let path = project::create(&temp.0.join("source"), "Joint authoring")?;
    let mut app = App::from_project(&path)?;
    for _ in positions {
        app.edit_key(KeyCode::KeyD)?;
    }
    for (node, position) in app.instances.clone().into_iter().zip(positions) {
        app.scene.set_local(
            node,
            Transform {
                translation: Vec3::from_array(*position),
                ..Transform::default()
            },
        )?;
    }
    app.commit_authoring()?;
    app.inspector = InspectorMode::Components(0);
    Ok((path, app))
}
fn token(app: &App) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    Ok(joint_authoring::request_token(
        &app.authoring_document()?.objects[app.selected],
    )?)
}
fn choice(app: &App, target: usize) -> [u8; 32] {
    camera_authoring::choice_token(
        app.follow_pick.as_ref().unwrap().key,
        &app.object_ids[target],
    )
}
/// Add a joint to `owner` through the inspector button and the hierarchy pick.
fn add_joint(app: &mut App, owner: usize, target: usize) -> TestResult {
    app.selected = owner;
    app.panel_action(panels::Action::Joint(token(app)?))?;
    app.panel_action(panels::Action::FollowTargetChoice(choice(app, target)))
}
fn joint(app: &App, owner: usize) -> Result<PhysicsJoint, Box<dyn std::error::Error>> {
    Ok(serde_json::from_value(
        app.authoring_document()?.objects[owner].components[SCHEMA].clone(),
    )?)
}
/// Type `text` into the selected object's joint member at `path` and press Enter.
fn edit(app: &mut App, path: &str, text: &str) -> TestResult {
    let document = app.authoring_document()?;
    let index = component_fields::fields(&document.objects[app.selected])?
        .iter()
        .position(|field| field.schema == SCHEMA && field.path == path)
        .ok_or("missing joint field")?;
    app.inspector = InspectorMode::Components(index / 6);
    app.panel_action(panels::Action::Field(index))?;
    app.field_key(KeyCode::Digit0, Some(text))?;
    app.field_key(KeyCode::Enter, None)
}
fn change_kind(app: &mut App) -> TestResult {
    app.panel_action(panels::Action::JointKind(token(app)?))
}

#[test]
fn add_joint_through_target_pick_welds_in_place_and_undoes() -> TestResult {
    let temp = Temp::new();
    let (_, mut app) = project_with(&temp, &[[0., 0., 0.], [0.5, 0.25, 0.]])?;
    let before = app.authoring_document()?;
    app.selected = 1;
    app.panel_action(panels::Action::Joint(token(&app)?))?;
    assert_eq!(
        app.follow_pick.as_ref().unwrap().purpose,
        PickPurpose::PhysicsJoint
    );
    assert_eq!(app.authoring_document()?, before);
    app.panel_action(panels::Action::FollowTargetChoice(choice(&app, 0)))?;
    assert!(app.follow_pick.is_none());
    let added = app.authoring_document()?;
    assert_eq!(
        joint(&app, 1)?,
        PhysicsJoint {
            target: app.object_ids[0].clone(),
            target_anchor: [0.5, 0.25, 0.],
            local_anchor: [0.; 3],
            kind: PhysicsJointKind::Fixed {
                relative_rotation: [0., 0., 0., 1.],
            },
        }
    );
    // The inspector lands on the page that lists the new joint members.
    let InspectorMode::Components(page) = app.inspector else {
        panic!("joint fields are not shown");
    };
    let fields = component_fields::fields(&added.objects[1])?;
    assert_eq!(fields[page * 6].schema, SCHEMA);
    app.edit_key(KeyCode::KeyZ)?;
    assert_eq!(app.authoring_document()?, before);
    app.edit_key(KeyCode::KeyY)?;
    assert_eq!(app.authoring_document()?, added);
    app.panel_action(panels::Action::Joint(token(&app)?))?;
    assert_eq!(app.authoring_document()?, before);
    app.edit_key(KeyCode::KeyZ)?;
    assert_eq!(app.authoring_document()?, added);
    app.stop_workers()
}

#[test]
fn joint_parameters_edit_through_component_fields_and_reject_invalid_values() -> TestResult {
    let temp = Temp::new();
    let (_, mut app) = project_with(&temp, &[[0., 0., 0.], [0., -2., 0.]])?;
    add_joint(&mut app, 1, 0)?;
    edit(&mut app, "/target_anchor/1", "0")?;
    edit(&mut app, "/local_anchor/0", "0.25")?;
    let anchored = joint(&app, 1)?;
    assert_eq!(anchored.target_anchor, [0.; 3]);
    assert_eq!(anchored.local_anchor, [0.25, 0., 0.]);
    change_kind(&mut app)?;
    edit(&mut app, "/kind/max_angle_rad", "1")?;
    edit(&mut app, "/kind/min_angle_rad", "-0.5")?;
    let hinge = app.authoring_document()?;
    assert_eq!(
        joint(&app, 1)?.kind,
        PhysicsJointKind::Hinge {
            axis: [0., 1., 0.],
            min_angle_rad: -0.5,
            max_angle_rad: 1.,
        }
    );
    // Limits out of order, non-numbers and overflow leave the scene untouched.
    for (path, text) in [
        ("/kind/min_angle_rad", "2"),
        ("/kind/max_angle_rad", "true"),
        ("/kind/max_angle_rad", "1e999"),
        ("/target_anchor/0", "nan"),
        ("/kind/type", "fixed"),
    ] {
        assert!(edit(&mut app, path, text).is_err(), "{path} = {text}");
        assert_eq!(app.authoring_document()?, hinge);
    }
    change_kind(&mut app)?;
    edit(&mut app, "/kind/rest_length", "0.5")?;
    edit(&mut app, "/kind/stiffness", "20")?;
    let spring = app.authoring_document()?;
    for (path, text) in [
        ("/kind/stiffness", "0"),
        ("/kind/damping", "-1"),
        ("/kind/rest_length", "-0.5"),
    ] {
        assert!(edit(&mut app, path, text).is_err(), "{path} = {text}");
        assert_eq!(app.authoring_document()?, spring);
    }
    app.edit_key(KeyCode::Escape)?;
    app.edit_key(KeyCode::KeyZ)?;
    assert_eq!(
        joint(&app, 1)?.kind,
        PhysicsJointKind::DistanceSpring {
            rest_length: 0.5,
            stiffness: 50.,
            damping: 2.,
        }
    );
    app.edit_key(KeyCode::KeyY)?;
    assert_eq!(app.authoring_document()?, spring);
    app.stop_workers()
}

#[test]
fn joint_kind_cycles_from_the_authored_pose_and_unit_vectors_renormalize() -> TestResult {
    let temp = Temp::new();
    let (_, mut app) = project_with(&temp, &[[0., 0., 0.], [0., -2., 0.]])?;
    add_joint(&mut app, 1, 0)?;
    edit(&mut app, "/target_anchor/1", "0")?;
    let fixed = app.authoring_document()?;
    change_kind(&mut app)?;
    assert!(matches!(
        joint(&app, 1)?.kind,
        PhysicsJointKind::Hinge { .. }
    ));
    // One edited component rescales the axis; a zero axis is rejected.
    edit(&mut app, "/kind/axis/0", "1")?;
    let PhysicsJointKind::Hinge { axis, .. } = joint(&app, 1)?.kind else {
        panic!("hinge expected");
    };
    assert!((Vec3::from_array(axis) - Vec3::new(1., 1., 0.).normalize()).length() < 1e-6);
    edit(&mut app, "/kind/axis/1", "0")?;
    let x_axis = app.authoring_document()?;
    assert!(matches!(
        joint(&app, 1)?.kind,
        PhysicsJointKind::Hinge { axis, .. } if axis == [1., 0., 0.]
    ));
    assert!(edit(&mut app, "/kind/axis/0", "0").is_err());
    assert_eq!(app.authoring_document()?, x_axis);
    // The spring starts at the authored anchor distance.
    change_kind(&mut app)?;
    assert_eq!(
        joint(&app, 1)?.kind,
        PhysicsJointKind::DistanceSpring {
            rest_length: 2.,
            stiffness: 50.,
            damping: 2.,
        }
    );
    change_kind(&mut app)?;
    assert_eq!(app.authoring_document()?, fixed);
    edit(&mut app, "/kind/relative_rotation/0", "1")?;
    let PhysicsJointKind::Fixed { relative_rotation } = joint(&app, 1)?.kind else {
        panic!("weld expected");
    };
    let half = std::f32::consts::FRAC_1_SQRT_2;
    assert!(
        (glam::Quat::from_array(relative_rotation) - glam::Quat::from_xyzw(half, 0., 0., half))
            .length()
            < 1e-6
    );
    // A kind button presented for an older descriptor cannot overwrite a newer one.
    let stale = token(&app)?;
    edit(&mut app, "/local_anchor/2", "0.5")?;
    let edited = app.authoring_document()?;
    assert!(app.panel_action(panels::Action::JointKind(stale)).is_err());
    assert_eq!(app.authoring_document()?, edited);
    app.stop_workers()
}

#[test]
fn joint_rejects_self_and_missing_targets_and_keeps_its_target_alive() -> TestResult {
    let temp = Temp::new();
    let (_, mut app) = project_with(&temp, &[[0., 0., 0.], [0.5, 0., 0.]])?;
    let before = app.authoring_document()?;
    app.selected = 1;
    app.panel_action(panels::Action::Joint(token(&app)?))?;
    let own = choice(&app, 1);
    let error = app
        .panel_action(panels::Action::FollowTargetChoice(own))
        .unwrap_err();
    assert!(error.to_string().contains("its own object"), "{error}");
    assert_eq!(app.authoring_document()?, before);
    add_joint(&mut app, 1, 0)?;
    let jointed = app.authoring_document()?;
    let own_id = app.object_ids[1].0.clone();
    for target in [own_id.as_str(), "missing-object", ""] {
        assert!(edit(&mut app, "/target", target).is_err(), "{target}");
        assert_eq!(app.authoring_document()?, jointed);
    }
    app.edit_key(KeyCode::Escape)?;
    // Deleting the target is refused as a whole, exactly like a follow-camera target.
    app.selected = 0;
    let target_node = app.instances[0];
    assert!(app.edit_key(KeyCode::Delete).is_err());
    assert!(app.scene.local(target_node).is_ok());
    assert_eq!(app.authoring_document()?, jointed);
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &jointed);
    app.selected = 1;
    app.panel_action(panels::Action::Joint(token(&app)?))?;
    app.selected = 0;
    app.edit_key(KeyCode::Delete)?;
    assert_eq!(app.authoring_document()?.objects.len(), 1);
    app.stop_workers()
}

#[test]
fn joint_retarget_keeps_parameters_and_stale_or_canceled_picks_never_publish() -> TestResult {
    let temp = Temp::new();
    let (_, mut app) = project_with(&temp, &[[0., 0., 0.], [0., -2., 0.], [3., 0., 0.]])?;
    add_joint(&mut app, 1, 0)?;
    change_kind(&mut app)?;
    change_kind(&mut app)?;
    let spring = app.authoring_document()?;
    let request = token(&app)?;
    app.panel_action(panels::Action::JointTarget(request))?;
    let canceled = choice(&app, 2);
    app.edit_key(KeyCode::Escape)?;
    assert!(app.follow_pick.is_none());
    assert!(
        app.panel_action(panels::Action::FollowTargetChoice(canceled))
            .is_err()
    );
    assert_eq!(app.authoring_document()?, spring);
    app.panel_action(panels::Action::JointTarget(request))?;
    assert!(
        app.panel_action(panels::Action::FollowTargetChoice(canceled))
            .is_err()
    );
    app.panel_action(panels::Action::FollowTargetChoice(choice(&app, 2)))?;
    let retargeted = joint(&app, 1)?;
    let original: PhysicsJoint =
        serde_json::from_value(spring.objects[1].components[SCHEMA].clone())?;
    assert_eq!(
        retargeted,
        PhysicsJoint {
            target: app.object_ids[2].clone(),
            ..original
        }
    );
    let edited = app.authoring_document()?;
    // The first request was drawn for the old descriptor.
    assert!(
        app.panel_action(panels::Action::JointTarget(request))
            .is_err()
    );
    assert!(app.panel_action(panels::Action::Joint(request)).is_err());
    assert_eq!(app.authoring_document()?, edited);
    app.edit_key(KeyCode::KeyZ)?;
    assert_eq!(app.authoring_document()?, spring);
    app.stop_workers()
}

#[test]
fn joint_survives_scene_save_reopen_and_play_stop() -> TestResult {
    let temp = Temp::new();
    let (path, mut app) = project_with(&temp, &[[0., 0., 0.], [0., -2., 0.]])?;
    add_joint(&mut app, 1, 0)?;
    change_kind(&mut app)?;
    edit(&mut app, "/kind/max_angle_rad", "0.75")?;
    app.save_authoring()?;
    let saved = app.authoring_document()?;
    let stored: serde_json::Value = serde_json::from_slice(&std::fs::read(
        path.parent().unwrap().join("main.scene.json"),
    )?)?;
    assert_eq!(
        stored["objects"][1]["components"][SCHEMA],
        serde_json::json!({
            "target": app.object_ids[0].0,
            "target_anchor": [0.0, -2.0, 0.0],
            "local_anchor": [0.0, 0.0, 0.0],
            "kind": {
                "type": "hinge",
                "axis": [0.0, 1.0, 0.0],
                "min_angle_rad": -std::f32::consts::FRAC_PI_2,
                "max_angle_rad": 0.75,
            },
        })
    );
    app.toggle_play()?;
    // Joint actions are inert while the game runs.
    app.panel_action(panels::Action::Joint(token(&app)?))?;
    assert!(change_kind(&mut app).is_ok());
    app.advance_game(0.1)?;
    app.toggle_play()?;
    assert_eq!(app.authoring_document()?, saved);
    app.stop_workers()?;
    let mut reopened = App::from_project(&path)?;
    assert_eq!(reopened.authoring_document()?, saved);
    reopened.selected = 1;
    edit(&mut reopened, "/kind/max_angle_rad", "1")?;
    reopened.load_authoring()?;
    assert_eq!(reopened.authoring_document()?, saved);
    reopened.stop_workers()
}

#[test]
fn joint_rows_buttons_and_pick_title_are_presented_by_the_panels() -> TestResult {
    let temp = Temp::new();
    let (_, mut app) = project_with(&temp, &[[0., 0., 0.], [0., -2., 0.]])?;
    let mut panel = panels::Panels::new()?;
    let build = |panel: &mut panels::Panels, app: &App| {
        let document = app.authoring_document()?;
        panel.build(
            &document,
            app.selected,
            Vec2::new(640., 680.),
            false,
            None,
            0,
            false,
            app.inspector,
            "",
            "",
            None,
        )?;
        Ok::<_, Box<dyn std::error::Error>>(
            panel
                .regions
                .iter()
                .map(|(_, action)| *action)
                .collect::<Vec<_>>(),
        )
    };
    app.selected = 1;
    let add = panels::Action::Joint(token(&app)?);
    assert!(build(&mut panel, &app)?.contains(&add));
    app.panel_action(add)?;
    panel.follow_pick = app.follow_pick.as_ref().map(|pick| pick.key);
    panel.pick_title = app.follow_pick.as_ref().unwrap().purpose.title();
    assert_eq!(panel.pick_title, "PICK JOINT TARGET");
    let candidate = panels::Action::FollowTargetChoice(choice(&app, 0));
    assert!(build(&mut panel, &app)?.contains(&candidate));
    app.panel_action(candidate)?;
    panel.follow_pick = None;
    let request = token(&app)?;
    let mut presented = Vec::new();
    let pages = component_fields::fields(&app.authoring_document()?.objects[1])?
        .len()
        .div_ceil(6);
    for page in 0..pages {
        app.inspector = InspectorMode::Components(page);
        presented.extend(build(&mut panel, &app)?);
    }
    for action in [
        panels::Action::Joint(request),
        panels::Action::JointTarget(request),
        panels::Action::JointKind(request),
    ] {
        assert!(presented.contains(&action), "{action:?}");
    }
    for (path, label) in [
        ("/target", "Joint target"),
        ("/kind/type", "joint"),
        ("/kind/relative_rotation/3", "Weld rotation W"),
        ("/target_anchor/1", "Target anchor Y"),
        ("/local_anchor/2", "Own anchor Z"),
        ("/kind/axis/0", "Hinge axis X"),
        ("/kind/rest_length", "Rest length (m)"),
    ] {
        assert_eq!(joint_authoring::field_label(path).as_deref(), Some(label));
    }
    assert_eq!(joint_authoring::field_label("/kind/unknown"), None);
    app.stop_workers()
}

#[test]
fn play_preflight_rejects_broken_joints_before_the_first_tick() -> TestResult {
    let temp = Temp::new();
    let (_, mut app) = project_with(&temp, &[[0., 0., 0.], [0., -2., 0.]])?;
    let authored = app.authoring_document()?;
    let spring = |target: &str, stiffness: f32| PhysicsJoint {
        target: ObjectId(target.into()),
        target_anchor: [0.; 3],
        local_anchor: [0.; 3],
        kind: PhysicsJointKind::DistanceSpring {
            rest_length: 1.,
            stiffness,
            damping: 0.,
        },
    };
    let own = app.object_ids[1].0.clone();
    let target = app.object_ids[0].0.clone();
    // These descriptors bypass the inspector, as a hand-edited scene or a host component would.
    for (broken, message) in [
        (spring("missing-object", 10.), "is not in the scene"),
        (spring(&own, 10.), "targets its own object"),
        (
            spring(&target, 0.),
            "invalid game.physics-joint.v1 descriptor",
        ),
    ] {
        app.scene.insert_component(app.instances[1], broken)?;
        let error = app.toggle_play().unwrap_err().to_string();
        assert!(error.contains(message), "{error}");
        assert!(app.play.playing.is_none());
        assert_eq!(app.play.simulation_ticks, 0);
        assert!(app.commit_authoring().is_err());
    }
    app.scene
        .remove_component::<PhysicsJoint>(app.instances[1])?;
    assert_eq!(app.authoring_document()?, authored);
    app.scene
        .insert_component(app.instances[1], spring(&target, 10.))?;
    app.commit_authoring()?;
    app.toggle_play()?;
    app.advance_game(1. / 60.)?;
    assert_eq!(app.play.simulation_ticks, 1);
    app.toggle_play()?;
    app.stop_workers()
}

/// Mover (kinematic), welded follower, static post and a bob hanging on a spring.
fn author_joint_scene(app: &mut App) -> TestResult {
    app.scene
        .insert_component(app.instances[0], BoxCollider::default())?;
    app.scene.insert_component(
        app.instances[0],
        voxy_gameplay::KinematicBody {
            velocity_m_s: [0.6, 0., 0.],
        },
    )?;
    app.commit_authoring()?;
    add_joint(app, 1, 0)?;
    add_joint(app, 3, 2)?;
    edit(app, "/target_anchor/1", "0")?;
    change_kind(app)?;
    change_kind(app)?;
    edit(app, "/kind/rest_length", "0.5")?;
    app.save_authoring()
}

#[test]
fn authored_joints_hold_in_play_stop_standalone_and_source_free_package() -> TestResult {
    let temp = Temp::new();
    let positions = [[0., 0., 0.], [0.25, 0.5, 0.], [0., 0., 2.], [0., -2., 2.]];
    let (path, mut editor) = project_with(&temp, &positions)?;
    author_joint_scene(&mut editor)?;
    let authored = editor.authoring_document()?;
    let position = |document: &SceneDocument, index: usize| {
        Vec3::from_array(document.objects[index].translation)
    };
    let mut trace = Vec::new();
    let mut hang = 2.;
    editor.toggle_play()?;
    for tick in 1..=120 {
        editor.advance_game(1. / 60.)?;
        let frame = editor.authoring_document()?;
        let mover = position(&frame, 0);
        assert!((mover.x - 0.01 * tick as f32).abs() < 1e-4, "{mover}");
        // The weld keeps the authored offset from the moving target.
        let weld = position(&frame, 1) - mover;
        assert!((weld - Vec3::new(0.25, 0.5, 0.)).length() < 1e-5, "{weld}");
        assert_eq!(frame.objects[1].rotation, authored.objects[1].rotation);
        // The spring shortens monotonically toward its rest length below the static post.
        assert_eq!(frame.objects[2], authored.objects[2]);
        let rope = position(&frame, 3) - position(&frame, 2);
        assert!(rope.x.abs() < 1e-6 && rope.z.abs() < 1e-6, "{rope}");
        assert!(-rope.y <= hang && -rope.y >= 0.5, "{rope} after {hang}");
        assert!(tick > 1 || -rope.y < 1.5, "{rope}");
        hang = -rope.y;
        trace.push(frame);
    }
    assert!(hang - 0.5 < 1e-3, "spring did not converge: {hang}");
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.stop_workers()?;

    let host = || Ok::<_, Box<dyn std::error::Error>>(GameHost::new(editor_component_registry()?));
    let package = temp.0.join("joints.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    let mut standalone = HeadlessGame::open_project(&path, host()?)?;
    for expected in &trace {
        standalone.advance(1. / 60.)?;
        assert_eq!(&standalone.snapshot()?, expected);
    }
    standalone.stop()?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    for expected in &trace {
        packaged.advance(1. / 60.)?;
        assert_eq!(&packaged.snapshot()?, expected);
    }
    packaged.stop()
}
