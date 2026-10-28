use crate::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
pub(super) struct Temp(PathBuf);
impl Temp {
    pub(super) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "voxy-locomotion-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn project(&self) -> Result<(PathBuf, Vec<u8>), Box<dyn std::error::Error>> {
        let path = project::create(&self.0.join("source"), "Locomotion regression")?;
        let root = path.parent().unwrap();
        let glb = gltf::binary::Glb::from_slice(include_bytes!(
            "../../voxy_render/examples/assets/animated-triangle.glb"
        ))?;
        let mut json: serde_json::Value = serde_json::from_slice(&glb.json)?;
        let source = json["animations"][0].clone();
        json["animations"] = serde_json::Value::Array(
            ["Idle", "Walking", "Jump", "Fall"]
                .into_iter()
                .map(|name| {
                    let mut clip = source.clone();
                    clip["name"] = name.into();
                    clip
                })
                .collect(),
        );
        let bytes = gltf::binary::Glb {
            header: glb.header,
            json: serde_json::to_vec(&json)?.into(),
            bin: glb.bin,
        }
        .to_vec()?;
        std::fs::write(root.join("rig.glb"), &bytes)?;
        std::fs::write(
            root.join("assets.json"),
            br#"{"version":1,"assets":[{"asset":"rig","source":"rig.glb"}]}"#,
        )?;
        let body = voxy_gameplay::CharacterBody {
            half_extents: [0.1; 3],
            speed: 1.,
            gravity: -9.8,
            jump_speed: 3.,
        };
        fn object(
            id: &str,
            parent: Option<&str>,
            position: [f32; 3],
            components: serde_json::Value,
        ) -> serde_json::Value {
            serde_json::json!({"id":id,"parent":parent,"name":id,"active":true,"translation":position,"rotation":[0,0,0,1],"scale":[1,1,1],"components":components})
        }
        let scene = serde_json::json!({"version":1,"objects":[
            object("body",None,[0.,0.101,0.],serde_json::json!({"game.character.v1":body})),
            object("visual",Some("body"),[0.,0.,0.],serde_json::json!({"editor.model.v1":"rig","editor.model-animation.v1":ModelAnimation{clip_name:"Idle".into(),..ModelAnimation::default()},"editor.model-locomotion.v1":ModelLocomotion{idle:"Idle".into(),moving:"Walking".into(),rising:"Jump".into(),falling:"Fall".into(),speed_threshold:0.01,transition_seconds:0.05}})),
            object("ground",None,[0.,-0.1,0.],serde_json::json!({"game.box.v1":{"half_extents":[10.,0.1,10.]}}))]});
        std::fs::write(root.join("main.scene.json"), serde_json::to_vec(&scene)?)?;
        Ok((path, bytes))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn host() -> Result<GameHost, Box<dyn std::error::Error>> {
    Ok(GameHost::new(editor_component_registry()?))
}
fn replay(game: &mut HeadlessGame) -> Result<Vec<AnimationSnapshot>, Box<dyn std::error::Error>> {
    replay_owner(game, &ObjectId("visual".into()))
}
fn replay_owner(
    game: &mut HeadlessGame,
    owner: &ObjectId,
) -> Result<Vec<AnimationSnapshot>, Box<dyn std::error::Error>> {
    let mut trace = Vec::new();
    let initial = game.snapshot()?;
    let body = initial
        .objects
        .iter()
        .find(|object| &object.id == owner)
        .and_then(|object| object.parent.clone())
        .ok_or("missing character parent")?;
    let start = initial
        .objects
        .iter()
        .find(|object| object.id == body)
        .ok_or("missing character body")?
        .translation;
    let mut positions = Vec::new();
    for tick in 0..180 {
        if tick == 20 {
            game.key(KeyCode::ArrowRight, true)?;
        }
        if tick == 40 {
            game.key(KeyCode::Space, true)?;
            game.key(KeyCode::Space, false)?;
        }
        if tick == 120 {
            game.key(KeyCode::ArrowRight, false)?;
        }
        game.advance(1. / 60.)?;
        positions.push(
            game.snapshot()?
                .objects
                .iter()
                .find(|object| object.id == body)
                .ok_or("character body disappeared")?
                .translation,
        );
        trace.push(
            game.animation_snapshot(owner)?
                .ok_or("missing accepted animation")?,
        );
    }
    for name in ["Idle", "Walking", "Jump", "Fall"] {
        assert!(trace.iter().any(|frame| frame.clip_name == name));
    }
    assert!(
        trace
            .iter()
            .any(|frame| frame.transition_weight > 0. && frame.transition_weight < 1.)
    );
    assert!(
        trace
            .windows(2)
            .any(|pair| pair[0].skin_matrices != pair[1].skin_matrices)
    );
    assert_eq!(trace.last().unwrap().clip_name, "Idle");
    assert!(
        positions
            .iter()
            .any(|position| position[1] > start[1] + 0.1),
        "character never left ground"
    );
    let end = positions.last().unwrap();
    assert!(end[0] > start[0] + 0.5, "character did not walk");
    assert!((end[1] - start[1]).abs() < 0.01, "character did not land");
    Ok(trace)
}
#[test]
fn locomotion_drives_existing_player_and_source_free_package_with_quick_jump()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, _) = temp.project()?;
    let mut game = HeadlessGame::open_project(&path, host()?)?;
    let expected = replay(&mut game)?;
    let snapshot = game.snapshot()?;
    game.stop()?;
    let mut editor = App::from_project(&path)?;
    editor.start_standalone()?;
    let authored = editor.play.playing.clone().unwrap();
    for tick in 0..180 {
        if tick == 20 {
            editor.game_key(KeyCode::ArrowRight, ElementState::Pressed)?;
        }
        if tick == 40 {
            editor.game_key(KeyCode::Space, ElementState::Pressed)?;
            editor.game_key(KeyCode::Space, ElementState::Released)?;
        }
        if tick == 120 {
            editor.game_key(KeyCode::ArrowRight, ElementState::Released)?;
        }
        editor.advance_game(1. / 60.)?;
        let owner = editor.instances[1];
        assert_eq!(
            editor.play.animations.clip_name(owner),
            Some(expected[tick].clip_name.as_str())
        );
    }
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.stop_workers()?;
    let package = temp.0.join("game.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    assert_eq!(replay(&mut packaged)?, expected);
    assert_eq!(packaged.snapshot()?, snapshot);
    packaged.stop()
}
#[test]
fn missing_inactive_clip_rejects_reload_and_failed_tick_restores_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, valid) = temp.project()?;
    let mut app = App::from_project(&path)?;
    app.start_standalone()?;
    app.advance_game(1. / 60.)?;
    let visual = app.instances[1];
    let id = AssetId("rig".into());
    let good = app.catalog.snapshot(&id).unwrap();
    let glb = gltf::binary::Glb::from_slice(&valid)?;
    let mut json: serde_json::Value = serde_json::from_slice(&glb.json)?;
    json["animations"]
        .as_array_mut()
        .unwrap()
        .retain(|clip| clip["name"] != "Jump");
    let invalid = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&json)?.into(),
        bin: glb.bin,
    }
    .to_vec()?;
    let model_path = path.parent().unwrap().join("rig.glb");
    std::fs::write(&model_path, invalid)?;
    app.reload.insert(id.clone());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !matches!(
        app.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Failed(_))
    ) {
        app.tick()?;
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(std::sync::Arc::ptr_eq(
        &app.catalog.snapshot(&id).unwrap(),
        &good
    ));
    std::fs::write(&model_path, &valid)?;
    app.reload.insert(id.clone());
    while std::sync::Arc::ptr_eq(&app.catalog.snapshot(&id).unwrap(), &good) {
        app.tick()?;
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    app.scene
        .component_mut::<ModelAnimation>(visual)?
        .unwrap()
        .root_motion_bone = "missing-root".into();
    let before = app
        .scene
        .component::<ModelAnimation>(visual)?
        .unwrap()
        .clone();
    let serial = app.play.animations.serial();
    let body_before = app.scene.local(app.instances[0])?;
    app.game_key(KeyCode::Space, ElementState::Pressed)?;
    assert!(app.advance_game(1. / 60.).is_err());
    assert_eq!(
        app.scene.component::<ModelAnimation>(visual)?.unwrap(),
        &before
    );
    assert_eq!(app.play.animations.serial(), serial);
    assert_eq!(app.scene.local(app.instances[0])?, body_before);
    assert!(app.play.player_input.state("jump").unwrap().pressed);
    app.stop_workers()
}

#[test]
fn multiple_mesh_skin_import_uses_one_playback_and_survives_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, source) = temp.project()?;
    let glb = gltf::binary::Glb::from_slice(&source)?;
    let mut document: serde_json::Value = serde_json::from_slice(&glb.json)?;
    document["nodes"][0]["children"] = serde_json::json!([1, 2]);
    document["nodes"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"mesh":0,"skin":1}));
    let second_skin = document["skins"][0].clone();
    document["skins"].as_array_mut().unwrap().push(second_skin);
    let bytes = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&document)?.into(),
        bin: glb.bin,
    }
    .to_vec()?;
    std::fs::write(path.parent().unwrap().join("rig.glb"), bytes)?;
    let mut game = HeadlessGame::open_project(&path, host()?)?;
    let expected = replay(&mut game)?;
    assert!(expected.iter().all(|frame| frame.skin_matrices.len() == 5));
    game.stop()?;
    let package = temp.0.join("multi.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    assert_eq!(replay(&mut packaged)?, expected);
    packaged.stop()
}

pub(super) fn morph_project(temp: &Temp) -> Result<(PathBuf, Vec<u8>), Box<dyn std::error::Error>> {
    let (path, source) = temp.project()?;
    let glb = gltf::binary::Glb::from_slice(&source)?;
    let mut document: serde_json::Value = serde_json::from_slice(&glb.json)?;
    let mut bin = glb.bin.as_ref().ok_or("fixture bin missing")?.to_vec();
    fn accessor(
        document: &mut serde_json::Value,
        bin: &mut Vec<u8>,
        values: &[f32],
        count: usize,
        kind: &str,
    ) -> usize {
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let offset = bin.len();
        bin.extend(values.iter().flat_map(|v| v.to_le_bytes()));
        let view = document["bufferViews"].as_array().unwrap().len();
        document["bufferViews"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"buffer":0,"byteOffset":offset,"byteLength":values.len()*4}));
        let index = document["accessors"].as_array().unwrap().len();
        let mut value =
            serde_json::json!({"bufferView":view,"componentType":5126,"count":count,"type":kind});
        if kind == "VEC3" {
            value["min"] = serde_json::json!([-1, -1, -1]);
            value["max"] = serde_json::json!([1, 1, 1]);
        }
        document["accessors"].as_array_mut().unwrap().push(value);
        index
    }
    let mesh_node = document["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node.get("mesh").is_some())
        .ok_or("fixture mesh missing")?;
    let mesh = document["nodes"][mesh_node]["mesh"].as_u64().unwrap() as usize;
    let positions = document["meshes"][mesh]["primitives"][0]["attributes"]["POSITION"]
        .as_u64()
        .unwrap() as usize;
    let vertices = document["accessors"][positions]["count"].as_u64().unwrap() as usize;
    let x = accessor(
        &mut document,
        &mut bin,
        &[0.25, 0., 0.].repeat(vertices),
        vertices,
        "VEC3",
    );
    let y = accessor(
        &mut document,
        &mut bin,
        &[0., 0.25, 0.].repeat(vertices),
        vertices,
        "VEC3",
    );
    document["meshes"][mesh]["primitives"][0]["targets"] =
        serde_json::json!([{"POSITION":x},{"POSITION":y}]);
    document["meshes"][mesh]["weights"] = serde_json::json!([0.125, -0.25]);
    document["nodes"][mesh_node]["weights"] = serde_json::json!([0.25, -0.5]);
    let endpoints = [(0., 1.), (1., 0.2), (0.2, 1.5), (1.5, -0.5)];
    for (index, (a, b)) in endpoints.into_iter().enumerate() {
        let input = document["animations"][index]["samplers"][0]["input"]
            .as_u64()
            .unwrap() as usize;
        let keys = document["accessors"][input]["count"].as_u64().unwrap() as usize;
        let values = (0..keys)
            .flat_map(|key| {
                let weight = a + (b - a) * key as f32 / (keys - 1) as f32;
                [weight, -0.5 * weight]
            })
            .collect::<Vec<_>>();
        let output = accessor(&mut document, &mut bin, &values, keys * 2, "SCALAR");
        let sampler = document["animations"][index]["samplers"]
            .as_array()
            .unwrap()
            .len();
        document["animations"][index]["samplers"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"input":input,"output":output,"interpolation":"LINEAR"}));
        document["animations"][index]["channels"]
            .as_array_mut()
            .unwrap()
            .push(
                serde_json::json!({"sampler":sampler,"target":{"node":mesh_node,"path":"weights"}}),
            );
    }
    document["buffers"][0]["byteLength"] = bin.len().into();
    let result = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&document)?.into(),
        bin: Some(bin.into()),
    }
    .to_vec()?;
    std::fs::write(path.parent().unwrap().join("rig.glb"), &result)?;
    Ok((path, result))
}

#[test]
fn morph_locomotion_uses_shared_import_player_and_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, _) = morph_project(&temp)?;
    let mut game = HeadlessGame::open_project(&path, host()?)?;
    let trace = replay(&mut game)?;
    assert!(
        trace
            .iter()
            .all(|frame| frame.morph_weights.len() == 1 && frame.morph_weights[0].len() == 2)
    );
    assert!(
        trace
            .windows(2)
            .any(|pair| pair[0].morph_weights != pair[1].morph_weights)
    );
    assert!(
        trace
            .iter()
            .any(|frame| frame.morph_weights[0].iter().any(|w| *w < 0.))
    );
    let snapshot = game.snapshot()?;
    game.stop()?;
    let package = temp.0.join("morph.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    assert_eq!(replay(&mut packaged)?, trace);
    assert_eq!(packaged.snapshot()?, snapshot);
    packaged.stop()
}

#[test]
fn morph_only_reload_fades_old_revision_and_rejects_invalid_defaults()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, source) = morph_project(&temp)?;
    let root = path.parent().unwrap();
    let scene_path = root.join("main.scene.json");
    let mut scene: serde_json::Value = serde_json::from_slice(&std::fs::read(&scene_path)?)?;
    let visual = scene["objects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|object| object["id"] == "visual")
        .unwrap();
    visual["components"]
        .as_object_mut()
        .unwrap()
        .remove("editor.model-locomotion.v1");
    visual["components"]["editor.model-animation.v1"]["transition_seconds"] = 0.5.into();
    std::fs::write(scene_path, serde_json::to_vec(&scene)?)?;
    let mut app = App::from_project(&path)?;
    app.start_standalone()?;
    app.advance_game(0.25)?;
    let id = AssetId("rig".into());
    let node = app.instances[1];
    let good = app.catalog.snapshot(&id).unwrap();
    let model = good.value().animated.as_ref().unwrap().clone();
    let before = app.play.animations.frame(node, &model).unwrap();
    let phase = app.play.animations.clip_phase(node).unwrap();
    let glb = gltf::binary::Glb::from_slice(&source)?;
    let document: serde_json::Value = serde_json::from_slice(&glb.json)?;
    let mut bin = glb.bin.as_ref().unwrap().to_vec();
    let output = document["animations"][0]["samplers"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["output"]
        .as_u64()
        .unwrap() as usize;
    let accessor = &document["accessors"][output];
    let view = &document["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize];
    let offset = view["byteOffset"].as_u64().unwrap() as usize
        + accessor
            .get("byteOffset")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize;
    for key in 0..accessor["count"].as_u64().unwrap() as usize / 2 {
        let value = 6. - key as f32 * 2.;
        bin[offset + key * 8..offset + key * 8 + 4].copy_from_slice(&value.to_le_bytes());
    }
    let revised = gltf::binary::Glb {
        header: glb.header,
        json: glb.json.clone(),
        bin: Some(bin.into()),
    }
    .to_vec()?;
    let source_path = root.join("rig.glb");
    std::fs::write(&source_path, &revised)?;
    app.reload.insert(id.clone());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::sync::Arc::ptr_eq(&app.catalog.snapshot(&id).unwrap(), &good) {
        app.tick()?;
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let newer = app.catalog.snapshot(&id).unwrap();
    let newer_model = newer.value().animated.as_ref().unwrap().clone();
    assert!(!model.has_same_morph_animation(&newer_model, false));
    app.advance_game(1. / 60.)?;
    let accepted = app.play.animations.frame(node, &newer_model).unwrap();
    assert!(accepted.transition_weight > 0. && accepted.transition_weight < 1.);
    let idle = newer_model
        .animations
        .iter()
        .position(|clip| clip.name() == "Idle")
        .unwrap();
    let expected =
        (phase + (1. / 60.) / f64::from(newer_model.animations[idle].duration())).rem_euclid(1.);
    assert!((app.play.animations.clip_phase(node).unwrap() - expected).abs() < 1e-10);
    assert!(
        newer_model.frame_weights(&accepted)?[0][0] > model.frame_weights(&before)?[0][0] + 0.1
    );
    let mut invalid: serde_json::Value = serde_json::from_slice(&glb.json)?;
    let mesh_node = invalid["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node.get("mesh").is_some())
        .unwrap();
    invalid["nodes"][mesh_node]["weights"] = serde_json::json!([1]);
    let rejected = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&invalid)?.into(),
        bin: glb.bin.clone(),
    }
    .to_vec()?;
    std::fs::write(&source_path, rejected)?;
    app.reload.insert(id.clone());
    while !matches!(
        app.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Failed(_))
    ) {
        app.tick()?;
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(std::sync::Arc::ptr_eq(
        &app.catalog.snapshot(&id).unwrap(),
        &newer
    ));
    assert!(std::sync::Arc::ptr_eq(
        &app.play.animations.frame(node, &newer_model).unwrap(),
        &accepted
    ));
    std::fs::write(&source_path, revised)?;
    app.reload.insert(id.clone());
    while std::sync::Arc::ptr_eq(&app.catalog.snapshot(&id).unwrap(), &newer) {
        app.tick()?;
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    app.advance_game(1. / 60.)?;
    app.stop_workers()
}

#[test]
fn overflowing_morph_tick_preserves_accepted_pose_clock_and_body()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, source) = morph_project(&temp)?;
    let root = path.parent().unwrap();
    let glb = gltf::binary::Glb::from_slice(&source)?;
    let mut document: serde_json::Value = serde_json::from_slice(&glb.json)?;
    let mut bin = glb.bin.as_ref().unwrap().to_vec();
    let delta = document["meshes"][0]["primitives"][0]["targets"][0]["POSITION"]
        .as_u64()
        .unwrap() as usize;
    let offset_of = |document: &serde_json::Value, index: usize| {
        let accessor = &document["accessors"][index];
        document["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize]["byteOffset"]
            .as_u64()
            .unwrap() as usize
            + accessor
                .get("byteOffset")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize
    };
    let offset = offset_of(&document, delta);
    for vertex in 0..document["accessors"][delta]["count"].as_u64().unwrap() as usize {
        bin[offset + vertex * 12..offset + vertex * 12 + 4].copy_from_slice(&2f32.to_le_bytes());
    }
    document["accessors"][delta]["min"] = serde_json::json!([2, 0, 0]);
    document["accessors"][delta]["max"] = serde_json::json!([2, 0, 0]);
    let output = document["animations"][0]["samplers"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["output"]
        .as_u64()
        .unwrap() as usize;
    let offset = offset_of(&document, output);
    for key in 1..document["accessors"][output]["count"].as_u64().unwrap() as usize / 2 {
        bin[offset + key * 8..offset + key * 8 + 4].copy_from_slice(&f32::MAX.to_le_bytes());
    }
    let revised = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&document)?.into(),
        bin: Some(bin.into()),
    }
    .to_vec()?;
    std::fs::write(root.join("rig.glb"), revised)?;
    let scene_path = root.join("main.scene.json");
    let mut scene: serde_json::Value = serde_json::from_slice(&std::fs::read(&scene_path)?)?;
    scene["objects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["id"] == "visual")
        .unwrap()["components"]
        .as_object_mut()
        .unwrap()
        .remove("editor.model-locomotion.v1");
    std::fs::write(scene_path, serde_json::to_vec(&scene)?)?;
    let mut app = App::from_project(&path)?;
    app.start_standalone()?;
    app.advance_game(1. / 60.)?;
    let model = app
        .catalog
        .snapshot(&AssetId("rig".into()))
        .unwrap()
        .value()
        .animated
        .as_ref()
        .unwrap()
        .clone();
    let visual = app.instances[1];
    let mut rejected = false;
    for _ in 0..240 {
        let serial = app.play.animations.serial();
        let body = app.scene.local(app.instances[0])?;
        let phase = app.play.animations.clip_phase(visual);
        let before = app.play.animations.frame(visual, &model).unwrap();
        if app.advance_game(1. / 60.).is_err() {
            assert_eq!(app.play.animations.serial(), serial);
            assert_eq!(app.scene.local(app.instances[0])?, body);
            assert_eq!(app.play.animations.clip_phase(visual), phase);
            assert!(std::sync::Arc::ptr_eq(
                &app.play.animations.frame(visual, &model).unwrap(),
                &before
            ));
            rejected = true;
            break;
        }
    }
    assert!(
        rejected,
        "extreme morph output was not rejected before publication"
    );
    app.stop_workers()
}

pub(super) fn rigid_morph_project(
    temp: &Temp,
) -> Result<(PathBuf, Vec<u8>), Box<dyn std::error::Error>> {
    // Construct an explicit unskinned regression input; never alter upstream assets.
    let (path, source) = morph_project(temp)?;
    let glb = gltf::binary::Glb::from_slice(&source)?;
    let mut document: serde_json::Value = serde_json::from_slice(&glb.json)?;
    document.as_object_mut().unwrap().remove("skins");
    for node in document["nodes"].as_array_mut().unwrap() {
        node.as_object_mut().unwrap().remove("skin");
    }
    let mesh_node = document["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node.get("mesh").is_some())
        .unwrap();
    mesh_node["translation"] = serde_json::json!([0.2, -0.1, 0]);
    mesh_node["scale"] = serde_json::json!([-0.75, 1.25, 0.5]);
    let attributes = document["meshes"][0]["primitives"][0]["attributes"]
        .as_object_mut()
        .unwrap();
    attributes.remove("JOINTS_0");
    attributes.remove("WEIGHTS_0");
    let result = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&document)?.into(),
        bin: glb.bin,
    }
    .to_vec()?;
    assert_eq!(gltf::Gltf::from_slice(&result)?.skins().count(), 0);
    std::fs::write(path.parent().unwrap().join("rig.glb"), &result)?;
    Ok((path, result))
}

#[test]
fn unskinned_rigid_morph_project_and_source_free_package_share_full_frames()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, source) = rigid_morph_project(&temp)?;
    let nodes = gltf::Gltf::from_slice(&source)?.nodes().count();
    let mut game = HeadlessGame::open_project(&path, host()?)?;
    let trace = replay(&mut game)?;
    assert!(
        trace
            .iter()
            .all(|f| f.skin_matrices.len() == nodes && f.morph_weights[0].len() == 2)
    );
    assert!(
        trace
            .windows(2)
            .any(|pair| pair[0].morph_weights != pair[1].morph_weights)
    );
    game.stop()?;
    let package = temp.0.join("rigid-morph.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    assert_eq!(replay(&mut packaged)?, trace);
    packaged.stop()
}

#[test]
fn morph_without_skins_or_clips_keeps_defaults_through_project_and_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, source) = rigid_morph_project(&temp)?;
    let root = path.parent().unwrap();
    let glb = gltf::binary::Glb::from_slice(&source)?;
    let mut document: serde_json::Value = serde_json::from_slice(&glb.json)?;
    document.as_object_mut().unwrap().remove("animations");
    let source = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&document)?.into(),
        bin: glb.bin,
    }
    .to_vec()?;
    let parsed = gltf::Gltf::from_slice(&source)?;
    assert_eq!(parsed.skins().count(), 0);
    assert_eq!(parsed.animations().count(), 0);
    std::fs::write(root.join("rig.glb"), source)?;
    let scene_path = root.join("main.scene.json");
    let mut scene: serde_json::Value = serde_json::from_slice(&std::fs::read(&scene_path)?)?;
    let visual = scene["objects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|object| object["id"] == "visual")
        .unwrap();
    visual["components"]
        .as_object_mut()
        .unwrap()
        .remove("editor.model-locomotion.v1");
    visual["components"]["editor.model-animation.v1"]["clip"] = serde_json::Value::Null;
    visual["components"]["editor.model-animation.v1"]["clip_name"] = "".into();
    std::fs::write(scene_path, serde_json::to_vec(&scene)?)?;
    #[derive(Debug, PartialEq)]
    struct BindState {
        properties: voxy_animation::PropertySet,
        palette: Vec<[f32; 16]>,
        positions: Vec<[f32; 3]>,
    }
    fn read(path: &std::path::Path) -> Result<BindState, Box<dyn std::error::Error>> {
        let mut app = App::from_project(path)?;
        app.start_standalone()?;
        app.advance_game(1. / 60.)?;
        let snapshot = app.catalog.snapshot(&AssetId("rig".into())).unwrap();
        let model = snapshot.value().animated.as_ref().unwrap();
        assert!(model.animations.is_empty());
        assert!(model.morph_source(0)?.is_some());
        let frame = app.play.animations.frame(app.instances[1], model).unwrap();
        assert_eq!(model.frame_weights(&frame)?, vec![vec![0.25, -0.5]]);
        let positions = model.scene_meshes_frame(&frame)?[0]
            .vertices()
            .iter()
            .map(|v| v.position)
            .collect();
        let state = BindState {
            properties: frame.properties.clone(),
            palette: frame
                .skin_matrices
                .iter()
                .map(glam::Mat4::to_cols_array)
                .collect(),
            positions,
        };
        app.stop_workers()?;
        Ok(state)
    }
    let expected = read(&path)?;
    // With no clip or playback component the ordinary owner must still publish
    // the complete authored defaults, rather than require a dummy animation.
    scene["objects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|object| object["id"] == "visual")
        .unwrap()["components"]
        .as_object_mut()
        .unwrap()
        .remove("editor.model-animation.v1");
    std::fs::write(root.join("main.scene.json"), serde_json::to_vec(&scene)?)?;
    assert_eq!(read(&path)?, expected);
    let package = temp.0.join("no-clips-morph.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    std::fs::remove_dir_all(root)?;
    let storage = crate::packaged_game::MaterializedPackage::open(&package)?;
    assert_eq!(read(&storage.project_path()?)?, expected);
    Ok(())
}

#[test]
fn authored_morph_overrides_survive_locomotion_and_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, source) = morph_project(&temp)?;
    let glb = gltf::binary::Glb::from_slice(&source)?;
    let mut document: serde_json::Value = serde_json::from_slice(&glb.json)?;
    let node = document["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node.get("mesh").is_some())
        .ok_or("mesh missing")?;
    let name = "AuthoredMorphNode";
    document["nodes"][node]["name"] = name.into();
    let named = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&document)?.into(),
        bin: glb.bin,
    }
    .to_vec()?;
    std::fs::write(path.parent().unwrap().join("rig.glb"), named)?;
    let scene_path = path.parent().unwrap().join("main.scene.json");
    let mut scene: serde_json::Value = serde_json::from_slice(&std::fs::read(&scene_path)?)?;
    scene["objects"][1]["components"]["editor.model-animation.v1"]["morph_weights"] =
        serde_json::json!({name:[1.25,-0.75]});
    std::fs::write(&scene_path, serde_json::to_vec(&scene)?)?;
    let mut app = App::from_project(&path)?;
    app.selected = 1;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.catalog.snapshot(&AssetId("rig".into())).is_none() {
        app.tick()?;
        assert!(
            std::time::Instant::now() < deadline,
            "morph import timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let original = app.authoring_document()?;
    let token = crate::component_fields::morph_edit_token(&original.objects[1]);
    app.panel_action(panels::Action::MorphDefaults(token))?;
    assert_eq!(
        app.scene
            .component::<ModelAnimation>(app.instances[1])?
            .unwrap()
            .morph_weights[name],
        vec![0.25, -0.5]
    );
    assert!(
        app.panel_action(panels::Action::MorphResume(token))
            .is_err()
    );
    let current = app.authoring_document()?;
    let token = crate::component_fields::morph_edit_token(&current.objects[1]);
    app.panel_action(panels::Action::MorphResume(token))?;
    assert!(
        app.scene
            .component::<ModelAnimation>(app.instances[1])?
            .unwrap()
            .morph_weights
            .is_empty()
    );
    app.preview_animation_phase(0.5, false)?;
    let asset = app.catalog.snapshot(&AssetId("rig".into())).unwrap();
    let model = asset.value().animated.as_ref().unwrap();
    let settings = app
        .scene
        .component::<ModelAnimation>(app.instances[1])?
        .unwrap();
    let preview = app
        .preview_animation_frame(app.instances[1], model, settings)
        .unwrap();
    let sampled = model.sample_frame_phase(settings.resolve_clip(model)?, 0.5)?;
    assert_eq!(
        model.frame_weights(&preview)?,
        model.frame_weights(&sampled)?
    );
    app.edit_key(winit::keyboard::KeyCode::KeyZ)?;
    assert_eq!(app.authoring_document()?, current);
    app.edit_key(winit::keyboard::KeyCode::KeyZ)?;
    assert_eq!(app.authoring_document()?, original);
    let before = app.authoring_document()?;
    let index = crate::component_fields::fields(&before.objects[1])?
        .iter()
        .position(|field| field.path == "/morph_weights")
        .ok_or("override inspector missing")?;
    app.inspector = InspectorMode::Components(index / 6);
    app.panel_action(panels::Action::Field(index))?;
    app.field_key(
        winit::keyboard::KeyCode::KeyA,
        Some(r#"{"AuthoredMorphNode":[0.75,-0.25]}"#),
    )?;
    app.field_key(winit::keyboard::KeyCode::Enter, None)?;
    let edited = app.authoring_document()?;
    assert_ne!(edited, before);
    app.edit_key(winit::keyboard::KeyCode::KeyZ)?;
    assert_eq!(app.authoring_document()?, before);
    app.edit_key(winit::keyboard::KeyCode::KeyY)?;
    assert_eq!(app.authoring_document()?, edited);
    assert!(
        app.edit_component_field(index, r#"{"AuthoredMorphNode":[]}"#)
            .is_err()
    );
    assert_eq!(app.authoring_document()?, edited);
    app.save_authoring()?;
    app.stop_workers()?;
    let mut game = HeadlessGame::open_project(&path, host()?)?;
    let trace = replay(&mut game)?;
    assert!(
        trace
            .iter()
            .all(|frame| frame.morph_weights == vec![vec![0.75, -0.25]])
    );
    let snapshot = game.snapshot()?;
    game.stop()?;
    let package = temp.0.join("morph-overrides.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    assert_eq!(replay(&mut packaged)?, trace);
    assert_eq!(packaged.snapshot()?, snapshot);
    packaged.stop()
}

#[test]
fn model_picking_uses_morphed_triangles_outside_imported_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, source) = morph_project(&temp)?;
    let glb = gltf::binary::Glb::from_slice(&source)?;
    let mut json: serde_json::Value = serde_json::from_slice(&glb.json)?;
    let node = json["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node.get("mesh").is_some())
        .unwrap();
    json["nodes"][node]["name"] = "MorphPick".into();
    let source = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&json)?.into(),
        bin: glb.bin,
    }
    .to_vec()?;
    std::fs::write(path.parent().unwrap().join("rig.glb"), source)?;
    let mut app = App::from_project(&path)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.catalog.snapshot(&AssetId("rig".into())).is_none() {
        app.tick()?;
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let owner = app.instances[1];
    app.scene.insert_component(
        owner,
        ModelAnimation {
            clip: None,
            morph_weights: [("MorphPick".into(), vec![8., 0.])].into(),
            ..Default::default()
        },
    )?;
    let mut local = app.scene.local(owner)?;
    local.scale = Vec3::new(-1., 1.3, 0.7);
    app.scene.set_local(owner, local)?;
    let asset = app.catalog.snapshot(&AssetId("rig".into())).unwrap();
    let model = asset.value().animated.as_ref().unwrap();
    let bind = model.bind_frame()?;
    let initial = model.scene_meshes_frame(&bind)?;
    let mut frame = bind;
    model.set_frame_morph_weights(&mut frame, node, &[8., 0.])?;
    let deformed = model.scene_meshes_frame(&frame)?;
    let centroid = |mesh: &voxy_render::SceneMesh| {
        mesh.indices()[..3]
            .iter()
            .map(|&i| Vec3::from_array(mesh.vertices()[i as usize].position))
            .sum::<Vec3>()
            / 3.
    };
    let world = app.scene.world_matrix(owner)?;
    let moved = world.transform_point3(centroid(&deformed[0]));
    let old = world.transform_point3(centroid(&initial[0]));
    app.camera.legacy = false;
    app.camera.distance = 8.;
    app.camera.target = (moved + old) * 0.5;
    let size = Vec2::new(800., 600.);
    for perspective in [false, true] {
        app.camera.perspective = perspective;
        let matrix = app.camera.matrix(size)?;
        let pixel = |point| {
            let p = matrix.project_point3(point);
            Vec2::new(p.x + 1., 1. - p.y) * size * 0.5
        };
        let cursor = pixel(moved);
        assert!(
            crate::picking::pick_mesh_depth(
                &app.scene,
                &[owner],
                &initial[0],
                cursor,
                size,
                matrix
            )?
            .is_none()
        );
        app.selected = 0;
        assert!(app.pick_model(cursor, size)?);
        assert_eq!(app.selected, 1);
        assert!(!app.pick_model(pixel(old), size)?);
        let min = deformed[0]
            .vertices()
            .iter()
            .map(|v| Vec3::from_array(v.position))
            .reduce(Vec3::min)
            .unwrap();
        let max = deformed[0]
            .vertices()
            .iter()
            .map(|v| Vec3::from_array(v.position))
            .reduce(Vec3::max)
            .unwrap();
        let expected = world.transform_point3((min + max) * 0.5);
        app.edit_key(KeyCode::KeyF)?;
        assert!(app.camera.target.abs_diff_eq(expected, 1e-6));
        assert!(!app.camera.target.abs_diff_eq(world.w_axis.truncate(), 0.1));
        for vertex in deformed[0].vertices() {
            let p = app
                .camera
                .matrix(size)?
                .project_point3(world.transform_point3(Vec3::from_array(vertex.position)));
            assert!(p.x.abs() <= 0.9 && p.y.abs() <= 0.9 && (0. ..=1.).contains(&p.z));
        }
    }
    app.selected = 1;
    app.scene
        .insert_component(owner, ModelAnimation::default())?;
    app.preview_animation_phase(0.6, false)?;
    let preview = app
        .preview_animation_frame(
            owner,
            model,
            app.scene.component::<ModelAnimation>(owner)?.unwrap(),
        )
        .unwrap();
    let preview_mesh = model.scene_meshes_frame(&preview)?;
    let preview_point = app
        .scene
        .world_matrix(owner)?
        .transform_point3(centroid(&preview_mesh[0]));
    app.camera.target = preview_point;
    let p = app.camera.matrix(size)?.project_point3(preview_point);
    assert!(app.pick_model(Vec2::new(p.x + 1., 1. - p.y) * size * 0.5, size)?);
    app.start_standalone()?;
    app.advance_game(1. / 60.)?;
    let owner = app.instances[1];
    let accepted = app
        .play
        .animations
        .frame(owner, model)
        .ok_or("runtime frame missing")?;
    let accepted_mesh = model.scene_meshes_frame(&accepted)?;
    let runtime_point = app
        .scene
        .world_matrix(owner)?
        .transform_point3(centroid(&accepted_mesh[0]));
    app.camera.target = runtime_point;
    let p = app.camera.matrix(size)?.project_point3(runtime_point);
    assert!(app.pick_model(Vec2::new(p.x + 1., 1. - p.y) * size * 0.5, size)?);
    app.scene.set_active(owner, false)?;
    let p = app.camera.matrix(size)?.project_point3(runtime_point);
    assert!(!app.pick_model(Vec2::new(p.x + 1., 1. - p.y) * size * 0.5, size)?);
    app.stop_workers()
}

pub(super) fn nested_morph_project(temp: &Temp) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let (path, _) = morph_project(temp)?;
    let root = path.parent().unwrap();
    let scene_path = root.join("main.scene.json");
    let scene: serde_json::Value = serde_json::from_slice(&std::fs::read(&scene_path)?)?;
    let objects = scene["objects"].as_array().unwrap();
    std::fs::write(
        root.join("player.prefab"),
        serde_json::to_vec(&serde_json::json!({
            "version":1,"objects":[objects[0].clone(),objects[1].clone()]
        }))?,
    )?;
    std::fs::write(root.join("nested.prefab"), br#"{"version":1,"objects":[],"instances":[{"id":"inner","asset":"player","parent":null,"overrides":{}}]}"#)?;
    let manifest = serde_json::json!({"version":1,"assets":[
        {"asset":"rig","source":"rig.glb"},
        {"asset":"player","source":"player.prefab"},
        {"asset":"nested","source":"nested.prefab"}
    ]});
    std::fs::write(root.join("assets.json"), serde_json::to_vec(&manifest)?)?;
    std::fs::write(
        &scene_path,
        serde_json::to_vec(&serde_json::json!({
            "version":1,"objects":[objects[2].clone()],
            "instances":[{"id":"hero","asset":"nested","parent":null,"overrides":{}}]
        }))?,
    )?;
    Ok(path)
}

#[test]
fn nested_prefab_locomotion_four_cycles_match_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = nested_morph_project(&temp)?;
    let root = path.parent().unwrap();
    let registry = editor_component_registry()?;
    let opened = project::open(&path, &registry)?;
    let owner = opened
        .scene
        .objects
        .iter()
        .find(|object| object.components.contains_key("editor.model-locomotion.v1"))
        .unwrap()
        .id
        .clone();
    let mut game = HeadlessGame::open_project(&path, host()?)?;
    let mut expected = Vec::new();
    for _ in 0..4 {
        expected.extend(replay_owner(&mut game, &owner)?);
    }
    assert_eq!(game.fixed_ticks(), 720);
    assert!(expected.iter().any(|frame| {
        frame
            .morph_weights
            .iter()
            .flatten()
            .any(|weight| *weight < 0.)
    }));
    assert!(
        expected
            .windows(2)
            .any(|pair| pair[0].morph_weights != pair[1].morph_weights)
    );
    let snapshot = game.snapshot()?;
    game.stop()?;
    let output = temp.0.join("nested-cycle.vpak");
    export_project_package_with_host(&path, &output, host()?)?;
    std::fs::remove_dir_all(root)?;
    let mut packaged = HeadlessGame::open_package(&output, host()?)?;
    let mut actual = Vec::new();
    for _ in 0..4 {
        actual.extend(replay_owner(&mut packaged, &owner)?);
    }
    assert_eq!(packaged.fixed_ticks(), 720);
    assert_eq!(actual, expected);
    assert_eq!(packaged.snapshot()?, snapshot);
    packaged.stop()
}
