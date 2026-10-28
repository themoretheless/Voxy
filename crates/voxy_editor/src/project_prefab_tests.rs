use crate::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
pub(crate) struct Temp(pub(crate) PathBuf);
impl Temp {
    pub(crate) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "voxy-project-prefab-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    pub(crate) fn setup(&self) -> Result<(PathBuf, serde_json::Value), Box<dyn std::error::Error>> {
        let path = project::create(&self.0.join("game"), "Prefab project")?;
        let root = path.parent().unwrap();
        std::fs::write(
            root.join("mesh.obj"),
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )?;
        let leaf = serde_json::json!({"version":1,"objects":[{"id":"root","parent":null,"name":"Model","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"editor.model.v1":"mesh","game.angular-motion.v1":{"axis":[0,0,1],"radians_per_second":1}}}]});
        std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
        std::fs::write(root.join("nested.prefab"),br#"{"version":1,"objects":[],"instances":[{"id":"child","asset":"leaf","parent":null,"overrides":{}}]}"#)?;
        std::fs::write(root.join("assets.json"),br#"{"version":1,"assets":[{"asset":"mesh","source":"mesh.obj"},{"asset":"leaf","source":"leaf.prefab"},{"asset":"nested","source":"nested.prefab"}]}"#)?;
        std::fs::write(root.join("main.scene.json"),br#"{"version":1,"objects":[],"instances":[{"id":"first","asset":"nested","parent":null,"overrides":{}},{"id":"second","asset":"nested","parent":null,"overrides":{}}]}"#)?;
        Ok((path, leaf))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn project_nested_prefabs_preserve_overrides_recover_and_package_after_relocation()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, mut leaf) = temp.setup()?;
    let registry = editor_component_registry()?;
    let opened = project::open(&path, &registry)?;
    assert_eq!(opened.scene.objects.len(), 2);
    let root = path.parent().unwrap();
    let scene_path = root.join("main.scene.json");
    let mut app = App::from_project(&path)?;
    assert!(app.id.is_none());
    let original = app.authoring_document()?;
    assert_eq!(original, opened.scene);
    let identities = app.object_ids.clone();
    app.edit_key(KeyCode::ArrowRight)?;
    let owner = app.instances[0];
    app.scene
        .component_mut::<voxy_gameplay::AngularMotion>(owner)?
        .ok_or("motion missing")?
        .radians_per_second = 2.;
    app.commit_authoring()?;
    let edited = app.authoring_document()?;
    app.save_authoring()?;
    let saved: voxy_scene::PrefabSceneDocument =
        serde_json::from_slice(&std::fs::read(&scene_path)?)?;
    assert!(saved.objects.is_empty());
    assert_eq!(saved.instances.len(), 2);
    assert_eq!(saved.instances[0].asset, "nested");
    assert!(saved.instances[1].overrides.is_empty());
    let changes = saved.instances[0]
        .overrides
        .values()
        .next()
        .ok_or("missing overrides")?;
    assert!(changes.components.is_empty());
    assert_eq!(
        changes.component_members["game.angular-motion.v1"]["/radians_per_second"],
        serde_json::json!(2.)
    );
    app.history_key(KeyCode::KeyZ)?;
    assert_eq!(
        app.authoring_document()?.objects[0].components["game.angular-motion.v1"]["radians_per_second"],
        serde_json::json!(1.)
    );
    app.history_key(KeyCode::KeyY)?;
    assert_eq!(app.authoring_document()?, edited);
    leaf["objects"][0]["components"]["game.angular-motion.v1"]["axis"] =
        serde_json::json!([0, 1, 0]);
    leaf["objects"][0]["components"]["game.angular-motion.v1"]["radians_per_second"] =
        serde_json::json!(3.);
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    app.load_authoring()?;
    let inherited = app.authoring_document()?;
    assert_eq!(app.object_ids, identities);
    for object in &inherited.objects {
        assert_eq!(
            object.components["game.angular-motion.v1"]["axis"],
            serde_json::json!([0., 1., 0.])
        );
    }
    assert_eq!(
        inherited.objects[0].components["game.angular-motion.v1"]["radians_per_second"],
        serde_json::json!(2.)
    );
    assert_eq!(
        inherited.objects[1].components["game.angular-motion.v1"]["radians_per_second"],
        serde_json::json!(3.)
    );
    assert_eq!(project::open(&path, &registry)?.scene, inherited);
    let metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
    let disk = std::fs::read(&scene_path)?;
    std::fs::write(root.join("leaf.prefab"), b"broken")?;
    assert!(project::open(&path, &registry).is_err());
    assert!(app.load_authoring().is_err());
    assert!(app.save_authoring().is_err());
    assert_eq!(app.authoring_document()?, inherited);
    assert_eq!(
        app.authoring.history.as_ref().unwrap().metadata(),
        &metadata
    );
    assert_eq!(std::fs::read(&scene_path)?, disk);
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    app.load_authoring()?;
    app.save_authoring()?;
    app.stop_workers()?;
    std::fs::rename(root, temp.0.join("moved"))?;
    let moved = temp.0.join("moved/project.voxy");
    let mut reopened = App::from_project(&moved)?;
    assert_eq!(reopened.authoring_document()?, inherited);
    reopened.toggle_play()?;
    reopened.advance_game(1.0 / 60.0)?;
    reopened.toggle_play()?;
    assert_eq!(reopened.authoring_document()?, inherited);
    reopened.stop_workers()?;
    let output = temp.0.join("prefabs.vpak");
    export_project_package(&moved, &output)?;
    let mut direct =
        HeadlessGame::open_project(&moved, GameHost::new(editor_component_registry()?))?;
    for _ in 0..120 {
        direct.advance(1.0 / 60.0)?;
    }
    let expected = direct.snapshot()?;
    direct.stop()?;
    std::fs::remove_dir_all(moved.parent().unwrap())?;
    let mut packaged =
        HeadlessGame::open_package(&output, GameHost::new(editor_component_registry()?))?;
    for _ in 0..120 {
        packaged.advance(1.0 / 60.0)?;
    }
    assert_eq!(packaged.snapshot()?, expected);
    packaged.stop()
}
#[test]
fn project_prefab_validation_rejects_cycles_unknown_assets_stale_overrides_and_depth()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, leaf) = temp.setup()?;
    let root = path.parent().unwrap();
    let registry = editor_component_registry()?;
    let nested = root.join("nested.prefab");
    let valid = std::fs::read(&nested)?;
    for invalid in [
        serde_json::json!({"version":1,"objects":[],"instances":[{"id":"loop","asset":"nested","parent":null,"overrides":{}}]}),
        serde_json::json!({"version":1,"objects":[],"instances":[{"id":"missing","asset":"absent","parent":null,"overrides":{}}]}),
        serde_json::json!({"version":1,"objects":[],"instances":[{"id":"child","asset":"leaf","parent":null,"overrides":{"absent":{"name":"stale"}}}]}),
        serde_json::json!({"version":2,"objects":[]}),
    ] {
        std::fs::write(&nested, serde_json::to_vec(&invalid)?)?;
        assert!(project::open(&path, &registry).is_err());
        assert!(App::from_project(&path).is_err());
    }
    std::fs::write(&nested, valid)?;
    // A finite acyclic chain must still honor the shared editor depth budget.
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("assets.json"))?)?;
    for i in 0..18 {
        let asset = format!("depth{i}");
        let file = format!("{asset}.prefab");
        let next = if i == 17 {
            "leaf".into()
        } else {
            format!("depth{}", i + 1)
        };
        std::fs::write(
            root.join(&file),
            serde_json::to_vec(
                &serde_json::json!({"version":1,"objects":[],"instances":[{"id":"next","asset":next,"parent":null,"overrides":{}}]}),
            )?,
        )?;
        manifest["assets"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"asset":asset,"source":file}));
    }
    std::fs::write(root.join("assets.json"), serde_json::to_vec(&manifest)?)?;
    std::fs::write(&nested,br#"{"version":1,"objects":[],"instances":[{"id":"deep","asset":"depth0","parent":null,"overrides":{}}]}"#)?;
    assert!(project::open(&path, &registry).is_err());
    std::fs::write(&nested, serde_json::to_vec(&leaf)?)?;
    assert!(project::open(&path, &registry).is_ok());
    Ok(())
}

#[test]
fn deferred_prefabs_are_validated_before_package_publication_and_restore_without_sources()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, leaf) = temp.setup()?;
    let root = path.parent().unwrap();
    std::fs::write(
        root.join("main.scene.json"),
        br#"{"version":1,"objects":[]}"#,
    )?;
    let output = temp.0.join("deferred.vpak");
    std::fs::write(root.join("leaf.prefab"), b"broken")?;
    assert!(
        export_project_package(&path, &output).is_err(),
        "invalid deferred prefab was packaged"
    );
    assert!(!output.exists());
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    std::fs::write(root.join("nested.prefab"), br#"{"version":1,"objects":[],"instances":[{"id":"self","asset":"nested","parent":null,"overrides":{}}]}"#)?;
    assert!(export_project_package(&path, &output).is_err());
    assert!(!output.exists());
    std::fs::write(root.join("nested.prefab"), br#"{"version":1,"objects":[],"instances":[{"id":"child","asset":"leaf","parent":null,"overrides":{}}]}"#)?;
    export_project_package(&path, &output)?;
    std::fs::remove_dir_all(root)?;
    let materialized = packaged_game::MaterializedPackage::open(&output)?;
    let mut restored = App::from_project(&materialized.project_path()?)?;
    assert!(restored.authoring_document()?.objects.is_empty());
    let imported = restored
        .authoring
        .authoring_project
        .load_prefab(&AssetId("nested".into()))?;
    assert_eq!(imported.value().expanded.objects.len(), 1);
    assert_eq!(
        imported.value().expanded.objects[0].components["editor.model.v1"],
        serde_json::json!("mesh")
    );
    for name in ["assets.json", "nested.prefab", "leaf.prefab"] {
        assert!(
            imported
                .inputs()
                .observations()
                .contains_key(&AssetId(name.into()))
        );
    }
    restored.stop_workers()?;
    run_packaged_game(&output, ViewportMode::GameCheck)?;
    Ok(())
}

#[test]
fn deferred_prefab_model_bindings_validate_expanded_overrides_before_build()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, leaf) = temp.setup()?;
    let root = path.parent().unwrap();
    std::fs::write(
        root.join("main.scene.json"),
        br#"{"version":1,"objects":[]}"#,
    )?;
    let output = temp.0.join("bindings.vpak");
    for model in ["missing-model", "leaf", ""] {
        let mut invalid = leaf.clone();
        invalid["objects"][0]["components"]["editor.model.v1"] = model.into();
        std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&invalid)?)?;
        assert!(
            export_project_package(&path, &output).is_err(),
            "deferred prefab accepted unavailable model {model:?}"
        );
        assert!(!output.exists());
    }
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    let original_nested = std::fs::read(root.join("nested.prefab"))?;
    let mut nested: serde_json::Value = serde_json::from_slice(&original_nested)?;
    nested["instances"][0]["overrides"] =
        serde_json::json!({"root":{"components":{"editor.model.v1":"missing-override-model"}}});
    std::fs::write(root.join("nested.prefab"), serde_json::to_vec(&nested)?)?;
    let mut app = App::from_project(&path)?;
    let expanded = app
        .authoring
        .authoring_project
        .load_prefab(&AssetId("nested".into()))?;
    assert_eq!(
        expanded.value().expanded.objects[0].components["editor.model.v1"],
        "missing-override-model"
    );
    app.stop_workers()?;
    assert!(
        export_project_package(&path, &output).is_err(),
        "expanded override hid an unavailable model binding"
    );
    assert!(!output.exists());
    nested["instances"][0]["overrides"]["root"]["components"]["editor.model.v1"] = "mesh".into();
    std::fs::write(root.join("nested.prefab"), serde_json::to_vec(&nested)?)?;
    export_project_package(&path, &output)?;
    std::fs::remove_dir_all(root)?;
    let materialized = packaged_game::MaterializedPackage::open(&output)?;
    let mut restored = App::from_project(&materialized.project_path()?)?;
    let imported = restored
        .authoring
        .authoring_project
        .load_prefab(&AssetId("nested".into()))?;
    assert_eq!(
        imported.value().expanded.objects[0].components["editor.model.v1"],
        "mesh"
    );
    restored.authoring.prefab_assets = restored.authoring.authoring_project.prefabs()?;
    restored.authoring.prefab_choice = restored
        .authoring
        .prefab_assets
        .iter()
        .position(|asset| asset.0 == "nested")
        .unwrap();
    restored.place_prefab()?;
    assert_eq!(restored.authoring_document()?.objects.len(), 1);
    restored.start_standalone()?;
    assert!(restored.catalog.snapshot(&AssetId("mesh".into())).is_some());
    let moving = restored
        .scene
        .components::<voxy_gameplay::AngularMotion>()
        .next()
        .unwrap()
        .0;
    let before = restored.scene.world_matrix(moving)?;
    restored.advance_game(1. / 60.)?;
    assert_ne!(restored.scene.world_matrix(moving)?, before);
    assert_eq!(restored.play.simulation_ticks, 1);
    restored.stop_workers()?;
    run_packaged_game(&output, ViewportMode::GameCheck)?;
    Ok(())
}

#[test]
fn deferred_prefab_retarget_requires_resolvable_animated_rigs_and_valid_mapping()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, mut leaf) = temp.setup()?;
    let root = path.parent().unwrap();
    std::fs::write(
        root.join("main.scene.json"),
        br#"{"version":1,"objects":[]}"#,
    )?;
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/foot-contact.glb"),
        root.join("rig.glb"),
    )?;
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("assets.json"))?)?;
    manifest["assets"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"asset":"rig","source":"rig.glb"}));
    std::fs::write(root.join("assets.json"), serde_json::to_vec(&manifest)?)?;
    leaf["objects"][0]["components"]["editor.model.v1"] = "rig".into();
    let valid = serde_json::json!({"source":"rig","joints":[{"source":"hip","target":"hip"}]});
    let output = temp.0.join("retarget.vpak");
    for profile in [
        serde_json::json!({"source":"missing-rig","joints":[{"source":"hip","target":"hip"}]}),
        serde_json::json!({"source":"leaf","joints":[{"source":"hip","target":"hip"}]}),
        serde_json::json!({"source":"mesh","joints":[{"source":"hip","target":"hip"}]}),
        serde_json::json!({"source":"rig","joints":[{"source":"absent-bone","target":"hip"}]}),
        serde_json::json!({"source":"rig","joints":[{"source":"hip","target":"absent-bone"}]}),
        serde_json::json!({"source":"rig","joints":[{"source":"hip","target":"hip","rotation_basis":[0,0,0,0]}]}),
    ] {
        leaf["objects"][0]["components"]["editor.model-retarget.v1"] = profile;
        std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
        assert!(
            export_project_package(&path, &output).is_err(),
            "deferred retarget profile was not validated against native rigs"
        );
        assert!(!output.exists());
    }
    leaf["objects"][0]["components"]["editor.model-retarget.v1"] = valid.clone();
    leaf["objects"][0]["components"]["editor.model.v1"] = "mesh".into();
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    assert!(
        export_project_package(&path, &output).is_err(),
        "static target accepted retarget profile"
    );
    assert!(!output.exists());
    leaf["objects"][0]["components"]["editor.model.v1"] = "rig".into();
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    let nested_path = root.join("nested.prefab");
    let original_nested = std::fs::read(&nested_path)?;
    let mut nested: serde_json::Value = serde_json::from_slice(&original_nested)?;
    let mut invalid = valid;
    invalid["source"] = "missing-override-rig".into();
    nested["instances"][0]["overrides"] =
        serde_json::json!({"root":{"components":{"editor.model-retarget.v1":invalid}}});
    std::fs::write(&nested_path, serde_json::to_vec(&nested)?)?;
    assert!(export_project_package(&path, &output).is_err());
    assert!(!output.exists());
    std::fs::write(&nested_path, original_nested)?;
    export_project_package(&path, &output)?;
    std::fs::remove_dir_all(root)?;
    let materialized = packaged_game::MaterializedPackage::open(&output)?;
    let mut restored = App::from_project(&materialized.project_path()?)?;
    restored.authoring.prefab_assets = restored.authoring.authoring_project.prefabs()?;
    restored.authoring.prefab_choice = restored
        .authoring
        .prefab_assets
        .iter()
        .position(|asset| asset.0 == "nested")
        .unwrap();
    restored.place_prefab()?;
    restored.start_standalone()?;
    restored.advance_game(1. / 60.)?;
    assert_eq!(restored.play.simulation_ticks, 1);
    assert_eq!(restored.scene.components::<ModelRetarget>().count(), 1);
    assert!(
        restored
            .catalog
            .snapshot(&AssetId("rig".into()))
            .unwrap()
            .value()
            .animated
            .is_some()
    );
    restored.stop_workers()?;
    Ok(())
}

#[test]
fn deferred_prefab_model_parts_check_imported_nodes_after_nested_overrides()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, mut leaf) = temp.setup()?;
    let root = path.parent().unwrap();
    std::fs::write(
        root.join("main.scene.json"),
        br#"{"version":1,"objects":[]}"#,
    )?;
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/scene3d/assembly.glb"),
        root.join("assembly.glb"),
    )?;
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("assets.json"))?)?;
    manifest["assets"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"asset":"assembly","source":"assembly.glb"}));
    std::fs::write(root.join("assets.json"), serde_json::to_vec(&manifest)?)?;
    let mut probe =
        App::from_manifest(&root.join("assets.json"), AssetId("assembly".into()), false)?;
    let until = Instant::now() + Duration::from_secs(10);
    while probe
        .catalog
        .snapshot(&AssetId("assembly".into()))
        .is_none()
    {
        probe.tick()?;
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let count = u32::try_from(
        probe
            .catalog
            .snapshot(&AssetId("assembly".into()))
            .unwrap()
            .value()
            .nodes
            .len(),
    )?;
    probe.stop_workers()?;
    let output = temp.0.join("parts.vpak");
    for (model, part) in [("mesh", 0), ("assembly", count), ("assembly", u32::MAX - 1)] {
        leaf["objects"][0]["components"]["editor.model.v1"] = model.into();
        leaf["objects"][0]["components"]["editor.model-part.v1"] = serde_json::json!({"node":part});
        std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
        assert!(
            export_project_package(&path, &output).is_err(),
            "deferred prefab accepted nonexistent imported part {model}:{part}"
        );
        assert!(!output.exists());
    }
    leaf["objects"][0]["components"]["editor.model.v1"] = "assembly".into();
    leaf["objects"][0]["components"]["editor.model-part.v1"] = serde_json::json!({"node":0});
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    export_project_package(&path, &temp.0.join("group.vpak"))?;
    leaf["objects"][0]["components"]["editor.model-part.v1"] = serde_json::json!({"node":u32::MAX});
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    export_project_package(&path, &temp.0.join("container.vpak"))?;
    leaf["objects"][0]["components"]["editor.model-part.v1"] = serde_json::json!({"node":1});
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    let nested_path = root.join("nested.prefab");
    let original = std::fs::read(&nested_path)?;
    let mut nested: serde_json::Value = serde_json::from_slice(&original)?;
    nested["instances"][0]["overrides"] =
        serde_json::json!({"root":{"components":{"editor.model-part.v1":{"node":count}}}});
    std::fs::write(&nested_path, serde_json::to_vec(&nested)?)?;
    assert!(export_project_package(&path, &output).is_err());
    assert!(!output.exists());
    std::fs::write(&nested_path, original)?;
    export_project_package(&path, &output)?;
    std::fs::remove_dir_all(root)?;
    let materialized = packaged_game::MaterializedPackage::open(&output)?;
    let mut restored = App::from_project(&materialized.project_path()?)?;
    restored.authoring.prefab_assets = restored.authoring.authoring_project.prefabs()?;
    restored.authoring.prefab_choice = restored
        .authoring
        .prefab_assets
        .iter()
        .position(|asset| asset.0 == "nested")
        .unwrap();
    restored.place_prefab()?;
    restored.start_standalone()?;
    let part = restored
        .scene
        .components::<ModelPart>()
        .next()
        .unwrap()
        .1
        .node;
    assert_eq!(part, 1);
    assert!(
        restored
            .catalog
            .snapshot(&AssetId("assembly".into()))
            .unwrap()
            .value()
            .mesh_for(Some(part))
            .is_some()
    );
    restored.advance_game(1. / 60.)?;
    assert_eq!(restored.play.simulation_ticks, 1);
    restored.stop_workers()?;
    Ok(())
}

#[test]
fn deferred_prefab_animation_selection_uses_native_playback_and_retarget_source()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, mut leaf) = temp.setup()?;
    let root = path.parent().unwrap();
    std::fs::write(
        root.join("main.scene.json"),
        br#"{"version":1,"objects":[]}"#,
    )?;
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/foot-contact.glb"),
    )?;
    std::fs::write(root.join("rig.glb"), &bytes)?;
    let json_length = u32::from_le_bytes(bytes[12..16].try_into()?) as usize;
    let mut spec: serde_json::Value = serde_json::from_slice(&bytes[20..20 + json_length])?;
    spec.as_object_mut().unwrap().remove("animations");
    crate::scene3d_tests::write_glb(&root.join("target.glb"), &spec, &bytes[28 + json_length..]);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("assets.json"))?)?;
    manifest["assets"].as_array_mut().unwrap().extend([
        serde_json::json!({"asset":"rig","source":"rig.glb"}),
        serde_json::json!({"asset":"target","source":"target.glb"}),
    ]);
    std::fs::write(root.join("assets.json"), serde_json::to_vec(&manifest)?)?;
    let mut probe = App::from_manifest(&root.join("assets.json"), AssetId("rig".into()), false)?;
    let until = Instant::now() + Duration::from_secs(10);
    while probe.catalog.snapshot(&AssetId("rig".into())).is_none() {
        probe.tick()?;
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let published = probe.catalog.snapshot(&AssetId("rig".into())).unwrap();
    let model = published.value().animated.as_ref().unwrap();
    let count = model.animations.len();
    let name = model.animations[0].name().to_owned();
    let joints = u16::try_from(model.skeleton.joints().len())?;
    probe.stop_workers()?;
    leaf["objects"][0]["components"]["editor.model.v1"] = "rig".into();
    let output = temp.0.join("clips.vpak");
    for settings in [
        ModelAnimation {
            clip: Some(count),
            ..Default::default()
        },
        ModelAnimation {
            clip_name: "missing-clip".into(),
            ..Default::default()
        },
        ModelAnimation {
            speed: -1.,
            ..Default::default()
        },
        ModelAnimation {
            root_motion_joint: joints,
            ..Default::default()
        },
        ModelAnimation {
            root_motion_bone: "missing-bone".into(),
            ..Default::default()
        },
        ModelAnimation {
            morph_weights: [("missing-morph-node".into(), vec![0.5])].into(),
            ..Default::default()
        },
    ] {
        leaf["objects"][0]["components"]["editor.model-animation.v1"] =
            serde_json::to_value(settings)?;
        std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
        assert!(
            export_project_package(&path, &output).is_err(),
            "deferred animation settings were not validated against imported model"
        );
        assert!(!output.exists());
    }
    leaf["objects"][0]["components"]["editor.model.v1"] = "mesh".into();
    leaf["objects"][0]["components"]["editor.model-animation.v1"] =
        serde_json::to_value(ModelAnimation::default())?;
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    assert!(export_project_package(&path, &output).is_err());
    assert!(!output.exists());
    leaf["objects"][0]["components"]["editor.model-animation.v1"] =
        serde_json::to_value(ModelAnimation {
            clip: None,
            ..Default::default()
        })?;
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    export_project_package(&path, &temp.0.join("bind-pose.vpak"))?;
    leaf["objects"][0]["components"]["editor.model.v1"] = "rig".into();
    let settings = ModelAnimation {
        clip: Some(9999),
        clip_name: name.clone(),
        ..Default::default()
    };
    leaf["objects"][0]["components"]["editor.model-animation.v1"] =
        serde_json::to_value(&settings)?;
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    let nested_path = root.join("nested.prefab");
    let original = std::fs::read(&nested_path)?;
    let mut nested: serde_json::Value = serde_json::from_slice(&original)?;
    let invalid = ModelAnimation {
        clip_name: "missing-override-clip".into(),
        ..Default::default()
    };
    nested["instances"][0]["overrides"] = serde_json::json!({"root":{"components":{"editor.model-animation.v1":serde_json::to_value(invalid)?}}});
    std::fs::write(&nested_path, serde_json::to_vec(&nested)?)?;
    assert!(export_project_package(&path, &output).is_err());
    assert!(!output.exists());
    std::fs::write(&nested_path, original)?;
    let locomotion = ModelLocomotion {
        idle: name.clone(),
        moving: name.clone(),
        rising: name.clone(),
        falling: name.clone(),
        speed_threshold: 0.01,
        transition_seconds: 0.05,
    };
    for field in [
        "idle",
        "moving",
        "rising",
        "falling",
        "speed_threshold",
        "transition_seconds",
    ] {
        let mut invalid = serde_json::to_value(&locomotion)?;
        invalid[field] = if matches!(field, "speed_threshold" | "transition_seconds") {
            serde_json::json!(-1)
        } else {
            "missing-locomotion-clip".into()
        };
        leaf["objects"][0]["components"]["editor.model-locomotion.v1"] = invalid;
        std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
        assert!(
            export_project_package(&path, &output).is_err(),
            "deferred locomotion {field} was not validated"
        );
        assert!(!output.exists());
    }
    leaf["objects"][0]["components"]["editor.model-locomotion.v1"] =
        serde_json::to_value(locomotion)?;
    let animation = leaf["objects"][0]["components"]
        .as_object_mut()
        .unwrap()
        .remove("editor.model-animation.v1")
        .unwrap();
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    assert!(export_project_package(&path, &output).is_err());
    assert!(!output.exists());
    leaf["objects"][0]["components"]["editor.model-animation.v1"] = animation;
    leaf["objects"][0]["components"]["game.character.v1"] =
        serde_json::to_value(voxy_gameplay::CharacterBody {
            half_extents: [0.1; 3],
            speed: 1.,
            gravity: -9.8,
            jump_speed: 3.,
        })?;
    leaf["objects"][0]["components"]["editor.model.v1"] = "target".into();
    leaf["objects"][0]["components"]["editor.model-retarget.v1"] =
        serde_json::json!({"source":"rig","joints":[{"source":"hip","target":"hip"}]});
    leaf["objects"][0]["components"]["editor.model-part.v1"] = serde_json::json!({"node":u32::MAX});
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    let error = export_project_package(&path, &output)
        .expect_err("deferred locomotion on imported hierarchy part was packaged");
    assert!(error.to_string().contains("logical model owner"), "{error}");
    assert!(!output.exists());
    leaf["objects"][0]["components"]
        .as_object_mut()
        .unwrap()
        .remove("editor.model-part.v1");
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    let original = std::fs::read(&nested_path)?;
    let mut nested: serde_json::Value = serde_json::from_slice(&original)?;
    nested["instances"][0]["overrides"] =
        serde_json::json!({"root":{"components":{"editor.model-part.v1":{"node":u32::MAX}}}});
    std::fs::write(&nested_path, serde_json::to_vec(&nested)?)?;
    let error = export_project_package(&path, &output)
        .expect_err("nested override admitted locomotion on imported hierarchy part");
    assert!(error.to_string().contains("logical model owner"), "{error}");
    assert!(!output.exists());
    std::fs::write(&nested_path, original)?;
    export_project_package(&path, &output)?;
    std::fs::remove_dir_all(root)?;
    let materialized = packaged_game::MaterializedPackage::open(&output)?;
    let mut restored = App::from_project(&materialized.project_path()?)?;
    restored.authoring.prefab_assets = restored.authoring.authoring_project.prefabs()?;
    restored.authoring.prefab_choice = restored
        .authoring
        .prefab_assets
        .iter()
        .position(|asset| asset.0 == "nested")
        .unwrap();
    restored.place_prefab()?;
    restored.start_standalone()?;
    restored.advance_game(1. / 60.)?;
    assert_eq!(restored.play.simulation_ticks, 1);
    let target = restored
        .catalog
        .snapshot(&AssetId("target".into()))
        .unwrap();
    let target = target.value().animated.as_ref().unwrap();
    assert!(target.animations.is_empty());
    let owner = restored
        .scene
        .components::<ModelAnimation>()
        .next()
        .unwrap()
        .0;
    assert!(restored.play.animations.frame(owner, target).is_some());
    assert_eq!(
        restored
            .scene
            .component::<ModelAnimation>(owner)?
            .unwrap()
            .clip_name,
        name
    );
    restored.stop_workers()?;
    Ok(())
}

#[test]
fn deferred_prefab_morph_weights_validate_target_and_survive_package()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = crate::locomotion_tests::Temp::new();
    let (_, bytes) = crate::locomotion_tests::morph_project(&fixture)?;
    let glb = gltf::binary::Glb::from_slice(&bytes)?;
    let mut document: serde_json::Value = serde_json::from_slice(&glb.json)?;
    let node = document["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node.get("mesh").is_some())
        .unwrap();
    document["nodes"][node]["name"] = "TargetMorph".into();
    let bytes = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&document)?.into(),
        bin: glb.bin,
    }
    .to_vec()?;
    let temp = Temp::new();
    let (path, mut leaf) = temp.setup()?;
    let root = path.parent().unwrap();
    std::fs::write(
        root.join("main.scene.json"),
        br#"{"version":1,"objects":[]}"#,
    )?;
    std::fs::write(root.join("morph.glb"), bytes)?;
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("assets.json"))?)?;
    manifest["assets"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"asset":"morph","source":"morph.glb"}));
    std::fs::write(root.join("assets.json"), serde_json::to_vec(&manifest)?)?;
    leaf["objects"][0]["components"]["editor.model.v1"] = "morph".into();
    let output = temp.0.join("morph.vpak");
    for weights in [vec![0.5], vec![0.5, 0.25, 0.75]] {
        let settings = ModelAnimation {
            clip: None,
            morph_weights: [("TargetMorph".into(), weights)].into(),
            ..Default::default()
        };
        leaf["objects"][0]["components"]["editor.model-animation.v1"] =
            serde_json::to_value(settings)?;
        std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
        assert!(export_project_package(&path, &output).is_err());
        assert!(!output.exists());
    }
    let weights = vec![1.25, -0.75];
    let settings = ModelAnimation {
        clip: None,
        morph_weights: [("TargetMorph".into(), weights.clone())].into(),
        ..Default::default()
    };
    leaf["objects"][0]["components"]["editor.model-animation.v1"] = serde_json::to_value(settings)?;
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    export_project_package(&path, &output)?;
    std::fs::remove_dir_all(root)?;
    let materialized = packaged_game::MaterializedPackage::open(&output)?;
    let mut restored = App::from_project(&materialized.project_path()?)?;
    restored.authoring.prefab_assets = restored.authoring.authoring_project.prefabs()?;
    restored.authoring.prefab_choice = restored
        .authoring
        .prefab_assets
        .iter()
        .position(|asset| asset.0 == "nested")
        .unwrap();
    restored.place_prefab()?;
    restored.start_standalone()?;
    restored.advance_game(1. / 60.)?;
    let published = restored.catalog.snapshot(&AssetId("morph".into())).unwrap();
    let model = published.value().animated.as_ref().unwrap();
    let owner = restored
        .scene
        .components::<ModelAnimation>()
        .next()
        .unwrap()
        .0;
    let frame = restored.play.animations.frame(owner, model).unwrap();
    assert!(
        model
            .frame_weights(&frame)?
            .iter()
            .any(|actual| actual == &weights)
    );
    restored.stop_workers()?;
    Ok(())
}

#[test]
fn deferred_prefab_liquid_material_is_admitted_before_package_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, mut leaf) = temp.setup()?;
    let root = path.parent().unwrap();
    std::fs::write(
        root.join("main.scene.json"),
        br#"{"version":1,"objects":[]}"#,
    )?;
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("assets.json"))?)?;
    manifest["assets"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"asset":"water","source":"water.json"}));
    std::fs::write(root.join("assets.json"), serde_json::to_vec(&manifest)?)?;
    leaf["objects"][0]["components"]["game.liquid-source.v1"] =
        serde_json::to_value(voxy_gameplay::LiquidSource {
            pulses: vec![voxy_gameplay::LiquidPulse {
                start_s: 0.,
                duration_s: 1. / 60.,
                volume_m3: 0.008,
                speed_m_s: 0.,
            }],
            density_kg_m3: 1000.,
            particle_volume_m3: 0.008,
            nozzle_radius_m: 0.,
            direction: [0., 0., -1.],
            material_asset: "water".into(),
        })?;
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    let output = temp.0.join("liquid.vpak");
    for invalid in [
        "not json",
        r#"{"rest_density":-1,"sound_speed":20,"viscosity":0.001}"#,
        r#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001,"optics":[3,0.5,0.2,0.5]}"#,
    ] {
        std::fs::write(root.join("water.json"), invalid)?;
        assert!(
            export_project_package(&path, &output).is_err(),
            "deferred liquid material was not admitted"
        );
        assert!(!output.exists());
    }
    std::fs::write(
        root.join("water.json"),
        br#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001,"optics":[3,0.5,0.2,1.333]}"#,
    )?;
    let original = std::fs::read(root.join("water.json"))?;
    for replacement in [
        b"corrupt material after declared prefab validation".as_slice(),
        br#"{"rest_density":1000,"sound_speed":21,"viscosity":0.001,"optics":[3,0.5,0.2,1.333]}"#
            .as_slice(),
    ] {
        let material = root.join("water.json");
        std::fs::write(&material, &original)?;
        let replacement = replacement.to_vec();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let called = calls.clone();
        let mut host = GameHost::new(editor_component_registry()?);
        host.behaviors = Some(BehaviorSetup::new(move |_, _| {
            called.fetch_add(1, Ordering::SeqCst);
            std::fs::write(&material, &replacement)?;
            Ok(())
        }));
        let error = export_project_package_with_host(&path, &output, host)
            .expect_err("deferred material changed after admission was packaged");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(
            error
                .to_string()
                .contains("water.json changed during package export"),
            "{error}"
        );
        assert!(!output.exists());
    }
    std::fs::write(root.join("water.json"), &original)?;
    export_project_package(&path, &output)?;
    std::fs::remove_dir_all(root)?;
    let package = packaged_game::MaterializedPackage::open(&output)?;
    let mut restored = App::from_project(&package.project_path()?)?;
    restored.authoring.prefab_assets = restored.authoring.authoring_project.prefabs()?;
    restored.authoring.prefab_choice = restored
        .authoring
        .prefab_assets
        .iter()
        .position(|asset| asset.0 == "nested")
        .unwrap();
    restored.place_prefab()?;
    restored.start_standalone()?;
    restored.advance_game(1. / 60.)?;
    assert_eq!(
        restored
            .play
            .liquid
            .as_ref()
            .unwrap()
            .liquid()
            .particles()
            .len(),
        1
    );
    restored.stop_workers()?;
    Ok(())
}

#[test]
fn deferred_prefab_audio_and_import_settings_are_admitted_before_build()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, mut leaf) = temp.setup()?;
    let root = path.parent().unwrap();
    std::fs::write(
        root.join("main.scene.json"),
        br#"{"version":1,"objects":[]}"#,
    )?;
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("assets.json"))?)?;
    manifest["assets"].as_array_mut().unwrap().extend([
        serde_json::json!({"asset":"tone","source":"tone.wav"}),
        serde_json::json!({"asset":"audio-settings","source":"audio.json"}),
    ]);
    std::fs::write(root.join("assets.json"), serde_json::to_vec(&manifest)?)?;
    leaf["objects"][0]["components"]["game.audio-source.v1"] =
        serde_json::to_value(voxy_gameplay::AudioSource {
            asset: "tone".into(),
            import_settings: Some("audio-settings".into()),
            bus: 0,
            gain: 1.,
            looping: true,
            spatial: false,
            near: 0.,
            far: 10.,
        })?;
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    let settings =
        br#"{"version":1,"max_input_bytes":1024,"max_frames":1024,"max_filter_evaluations":66560}"#;
    std::fs::write(root.join("audio.json"), settings)?;
    std::fs::write(root.join("tone.wav"), b"invalid WAV")?;
    let output = temp.0.join("audio.vpak");
    assert!(
        export_project_package(&path, &output).is_err(),
        "deferred audio was not decoded during build"
    );
    assert!(!output.exists());
    let wav = crate::audio_session::checkpoint::tests::wav();
    std::fs::write(root.join("tone.wav"), &wav)?;
    for invalid in [
        b"invalid settings".as_slice(),
        br#"{"version":1,"max_input_bytes":1024,"max_frames":2,"max_filter_evaluations":66560}"#
            .as_slice(),
    ] {
        std::fs::write(root.join("audio.json"), invalid)?;
        assert!(export_project_package(&path, &output).is_err());
        assert!(!output.exists());
    }
    std::fs::write(root.join("audio.json"), settings)?;
    let nested_path = root.join("nested.prefab");
    let original_nested = std::fs::read(&nested_path)?;
    let mut nested: serde_json::Value = serde_json::from_slice(&original_nested)?;
    let mut other = leaf["objects"][0].clone();
    other["id"] = "other-audio".into();
    other["components"]["game.audio-source.v1"]["import_settings"] = serde_json::Value::Null;
    nested["objects"] = serde_json::json!([other]);
    std::fs::write(&nested_path, serde_json::to_vec(&nested)?)?;
    assert!(export_project_package(&path, &output).is_err());
    assert!(!output.exists());
    std::fs::write(&nested_path, original_nested)?;
    for (file, replacement) in [
        ("tone.wav", b"corrupt WAV after deferred import".as_slice()),
        ("audio.json", br#"{"version":1,"max_input_bytes":1024,"max_frames":1023,"max_filter_evaluations":66560}"#.as_slice()),
    ] {
        let source = root.join(file);
        let original = std::fs::read(&source)?;
        let changed = source.clone();
        let replacement = replacement.to_vec();
        let mut host = GameHost::new(editor_component_registry()?);
        host.behaviors = Some(BehaviorSetup::new(move |_, _| {
            std::fs::write(&changed, &replacement)?;
            Ok(())
        }));
        let error = export_project_package_with_host(&path, &output, host)
            .expect_err("changed deferred audio inputs were packaged");
        assert!(error.to_string().contains(&format!("{file} changed during package export")), "{error}");
        assert!(!output.exists());
        std::fs::write(&source, original)?;
    }
    export_project_package(&path, &output)?;
    std::fs::remove_dir_all(root)?;
    let package = packaged_game::MaterializedPackage::open(&output)?;
    let mut restored = App::from_project(&package.project_path()?)?;
    restored.audio.set_output_mode(AudioOutputMode::Offline);
    restored.authoring.prefab_assets = restored.authoring.authoring_project.prefabs()?;
    restored.authoring.prefab_choice = restored
        .authoring
        .prefab_assets
        .iter()
        .position(|asset| asset.0 == "nested")
        .unwrap();
    restored.place_prefab()?;
    restored.start_standalone()?;
    restored.advance_game(1. / 60.)?;
    let audio = restored.audio.play().unwrap();
    assert!(audio.frames > 0);
    assert!(audio.peak > 0.);
    restored.stop_workers()?;
    Ok(())
}

#[test]
fn deferred_prefab_ui_font_is_validated_observed_and_available_without_sources()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let (path, mut leaf) = temp.setup()?;
    let root = path.parent().unwrap();
    std::fs::write(
        root.join("main.scene.json"),
        br#"{"version":1,"objects":[]}"#,
    )?;
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("assets.json"))?)?;
    manifest["assets"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"asset":"interface","source":"font.ttf"}));
    std::fs::write(root.join("assets.json"), serde_json::to_vec(&manifest)?)?;
    leaf["objects"][0]["components"]["game.ui-element.v1"] =
        serde_json::to_value(voxy_gameplay::UiElement {
            origin: [0.; 2],
            size: [1.; 2],
            color: [1.; 4],
            layer: 0,
            enabled: true,
            action: None,
            text: Some(voxy_gameplay::UiText {
                font: "interface".into(),
                content: "Voxy: Привет".into(),
                size: 24.,
                color: [1.; 4],
            }),
        })?;
    std::fs::write(root.join("leaf.prefab"), serde_json::to_vec(&leaf)?)?;
    std::fs::write(root.join("font.ttf"), b"invalid font")?;
    let output = temp.0.join("ui.vpak");
    assert!(
        export_project_package(&path, &output).is_err(),
        "deferred UI font was not decoded during build"
    );
    assert!(!output.exists());
    let font = [
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "C:/Windows/Fonts/arial.ttf",
    ]
    .iter()
    .find_map(|path| std::fs::read(path).ok())
    .ok_or("integration test requires Unicode TrueType font")?;
    std::fs::write(root.join("font.ttf"), &font)?;
    let changed = root.join("font.ttf");
    let mut host = GameHost::new(editor_component_registry()?);
    host.behaviors = Some(BehaviorSetup::new(move |_, _| {
        std::fs::write(&changed, b"font changed after prefab admission")?;
        Ok(())
    }));
    let error = export_project_package_with_host(&path, &output, host)
        .expect_err("font changed after deferred admission was packaged");
    assert!(
        error
            .to_string()
            .contains("font.ttf changed during package export"),
        "{error}"
    );
    assert!(!output.exists());
    std::fs::write(root.join("font.ttf"), &font)?;
    export_project_package(&path, &output)?;
    std::fs::remove_dir_all(root)?;
    let package = packaged_game::MaterializedPackage::open(&output)?;
    let mut restored = App::from_project(&package.project_path()?)?;
    restored.authoring.prefab_assets = restored.authoring.authoring_project.prefabs()?;
    restored.authoring.prefab_choice = restored
        .authoring
        .prefab_assets
        .iter()
        .position(|asset| asset.0 == "nested")
        .unwrap();
    restored.place_prefab()?;
    let text = ui_text::UiTextPreparation::prepare_import(
        &restored.scene,
        &restored.authoring.authoring_project,
        [1280., 720.],
    )?;
    assert!(!text.value().runs.is_empty());
    restored.start_standalone()?;
    restored.advance_game(1. / 60.)?;
    assert_eq!(restored.play.simulation_ticks, 1);
    restored.stop_workers()?;
    Ok(())
}
