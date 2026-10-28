#![allow(clippy::float_cmp)]
use crate::*;
fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/scene3d")
}
fn ready(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app
        .catalog
        .snapshot(app.id.as_ref().expect("model-only fixture"))
        .is_none()
    {
        app.tick().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn static_gltf_hierarchy_material_camera_edit_and_persistence_share_resources() {
    let root = fixture();
    let mut app =
        App::from_manifest(&root.join("assets.json"), AssetId("assembly".into()), false).unwrap();
    ready(&mut app);
    let resource = app
        .catalog
        .snapshot(app.id.as_ref().expect("model-only fixture"))
        .unwrap();
    assert_eq!(resource.value().nodes.len(), 3);
    assert_eq!(resource.value().images.len(), 1);
    assert_eq!(resource.value().mesh.indices().len(), 72);
    app.expand_model().unwrap();
    assert_eq!(app.instances.len(), 4);
    assert_eq!(
        app.scene.parent(app.instances[2]).unwrap(),
        Some(app.instances[1])
    );
    assert_eq!(
        app.scene
            .component::<ModelPart>(app.instances[0])
            .unwrap()
            .unwrap()
            .node,
        u32::MAX
    );
    let size = Vec2::new(800., 600.);
    app.camera.orbit(Vec2::new(25., 15.));
    for perspective in [true, false] {
        app.camera.perspective = perspective;
        let node = app.instances[2];
        let point = app.scene.world_matrix(node).unwrap().w_axis.truncate();
        let projected = app.camera.matrix(size).unwrap().project_point3(point);
        let cursor = Vec2::new(projected.x + 1., 1. - projected.y) * size * 0.5;
        assert!(app.pick_model(cursor, size).unwrap());
        assert_eq!(app.selected, 2);
        let before = app.scene.local(node).unwrap();
        app.begin_drag(cursor, size).unwrap();
        app.drag.as_mut().unwrap().axis = DragAxis::X;
        app.preview_drag(cursor + Vec2::new(20., 0.)).unwrap();
        app.finish_drag(true).unwrap();
        let moved = app.scene.local(node).unwrap();
        assert!(moved.translation.x > before.translation.x);
        assert_eq!(moved.translation.y, before.translation.y);
        app.history_key(KeyCode::KeyZ).unwrap();
    }
    app.edit_key(KeyCode::KeyG).unwrap();
    app.edit_key(KeyCode::KeyM).unwrap();
    app.panel_action(panels::Action::Field(0)).unwrap();
    app.field = Some((0, "0.7".into()));
    app.field_key(KeyCode::Enter, None).unwrap();
    app.edit_key(KeyCode::KeyL).unwrap();
    let document = app.authoring_document().unwrap();
    let loaded = document.load(&model_registry().unwrap(), 128).unwrap();
    assert_eq!(loaded.graph.components::<EditorCamera>().count(), 1);
    assert_eq!(loaded.graph.components::<SceneMaterial>().count(), 1);
    assert_eq!(loaded.graph.components::<DirectionalLight>().count(), 1);
    assert!(Arc::ptr_eq(
        &resource,
        &app.catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .unwrap()
    ));
    app.toggle_play().unwrap();
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), document);
    app.stop_workers().unwrap();
}
#[test]
fn textured_static_import_rejects_truncation_and_observes_external_dependencies() {
    let root = fixture();
    let provider = FileInputs::new(&root).unwrap();
    let mut inputs = voxy_assets::ImportInputs::new(4, 32 * 1024 * 1024);
    let asset = import::load(
        &SourcePath::new("assembly.glb").unwrap(),
        &provider,
        &mut inputs,
    )
    .unwrap();
    assert_eq!(asset.images[0].width(), 4);
    let imported = inputs
        .finish(asset, |id, limit| provider.read(id, limit))
        .unwrap();
    assert_eq!(imported.value().nodes[2].parent, Some(0));
    let temporary = std::env::temp_dir().join(format!("voxy-glb-{}", std::process::id()));
    std::fs::create_dir_all(&temporary).unwrap();
    let bytes = std::fs::read(root.join("assembly.glb")).unwrap();
    std::fs::write(temporary.join("bad.glb"), &bytes[..bytes.len() - 10]).unwrap();
    let mut inputs = voxy_assets::ImportInputs::new(4, 32 * 1024 * 1024);
    assert!(
        import::load(
            &SourcePath::new("bad.glb").unwrap(),
            &FileInputs::new(&temporary).unwrap(),
            &mut inputs
        )
        .is_err()
    );
    let json_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let mut spec: serde_json::Value = serde_json::from_slice(&bytes[20..20 + json_length]).unwrap();
    let binary = &bytes[28 + json_length..];
    let image_view = &spec["bufferViews"][3];
    let image_offset = usize::try_from(image_view["byteOffset"].as_u64().unwrap()).unwrap();
    let image_length = usize::try_from(image_view["byteLength"].as_u64().unwrap()).unwrap();
    std::fs::write(
        temporary.join("texture.png"),
        &binary[image_offset..image_offset + image_length],
    )
    .unwrap();
    std::fs::write(temporary.join("mesh.bin"), binary).unwrap();
    spec["buffers"][0]["uri"] = "mesh.bin".into();
    spec["images"][0] = serde_json::json!({"uri":"texture.png"});
    std::fs::write(
        temporary.join("external.gltf"),
        serde_json::to_vec(&spec).unwrap(),
    )
    .unwrap();
    let provider = FileInputs::new(&temporary).unwrap();
    let mut inputs = voxy_assets::ImportInputs::new(4, 32 * 1024 * 1024);
    let decoded = import::load(
        &SourcePath::new("external.gltf").unwrap(),
        &provider,
        &mut inputs,
    )
    .unwrap();
    assert_eq!(inputs.observations().len(), 3);
    std::fs::write(temporary.join("texture.png"), b"changed after decode").unwrap();
    assert!(
        inputs
            .finish(decoded, |id, limit| provider.read(id, limit))
            .is_err()
    );
    std::fs::remove_dir_all(temporary).unwrap();
}

#[test]
fn external_texture_watcher_failure_and_recovery_preserve_authored_scene() {
    let directory = std::env::temp_dir().join(format!(
        "voxy-texture-recovery-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let bytes = std::fs::read(fixture().join("assembly.glb")).unwrap();
    let json_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let mut spec: serde_json::Value = serde_json::from_slice(&bytes[20..20 + json_length]).unwrap();
    let binary = &bytes[28 + json_length..];
    let image_view = &spec["bufferViews"][3];
    let offset = usize::try_from(image_view["byteOffset"].as_u64().unwrap()).unwrap();
    let length = usize::try_from(image_view["byteLength"].as_u64().unwrap()).unwrap();
    let texture = &binary[offset..offset + length];
    std::fs::write(directory.join("texture.png"), texture).unwrap();
    std::fs::write(directory.join("mesh.bin"), binary).unwrap();
    spec["buffers"][0]["uri"] = "mesh.bin".into();
    spec["images"][0] = serde_json::json!({"uri":"texture.png"});
    std::fs::write(
        directory.join("external.gltf"),
        serde_json::to_vec(&spec).unwrap(),
    )
    .unwrap();
    let mut app = App::new(&directory.join("external.gltf"), false).unwrap();
    ready(&mut app);
    app.expand_model().unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    let document = app.authoring_document().unwrap();
    let id = app.id.clone().unwrap();
    let accepted = app.catalog.snapshot(&id).unwrap();
    // Change only the external dependency; no manual reload request is issued.
    std::fs::write(directory.join("texture.png"), b"invalid PNG dependency").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.failed_at.is_none() {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "watcher did not reject corrupt texture"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()));
    assert_eq!(app.authoring_document().unwrap(), document);
    assert_eq!(app.catalog.snapshot(&id).unwrap().value().images.len(), 1);
    // Restoring just the dependency must recover through the same watcher/worker path.
    std::fs::write(directory.join("texture.png"), texture).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()) {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "watcher did not recover restored texture"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.recovered);
    assert_eq!(app.authoring_document().unwrap(), document);
    assert_eq!(app.catalog.snapshot(&id).unwrap().value().images.len(), 1);
    let recovered = app.catalog.snapshot(&id).unwrap();
    std::fs::remove_file(directory.join("texture.png")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !matches!(
        app.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Failed(_))
    ) {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "missing texture did not fail import"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(Arc::ptr_eq(&recovered, &app.catalog.snapshot(&id).unwrap()));
    assert_eq!(app.authoring_document().unwrap(), document);
    std::fs::write(directory.join("texture.png"), texture).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Arc::ptr_eq(&recovered, &app.catalog.snapshot(&id).unwrap()) {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "recreated texture did not recover"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.authoring_document().unwrap(), document);
    app.stop_workers().unwrap();
    // The recovered data must also survive the ordinary project build path.
    let project_root = directory.join("project");
    let project = project::create(&project_root, "External texture recovery").unwrap();
    for name in ["external.gltf", "mesh.bin", "texture.png"] {
        std::fs::copy(directory.join(name), project_root.join(name)).unwrap();
    }
    std::fs::write(
        project_root.join("assets.json"),
        br#"{"version":1,"assets":[{"asset":"external.gltf","source":"external.gltf"}]}"#,
    )
    .unwrap();
    std::fs::write(
        project_root.join("main.scene.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    let package_path = directory.with_extension("vpak");
    std::fs::write(project_root.join("texture.png"), b"corrupt build input").unwrap();
    assert!(export_project_package(&project, &package_path).is_err());
    assert!(!package_path.exists(), "failed import published a package");
    std::fs::write(project_root.join("texture.png"), texture).unwrap();
    export_project_package(&project, &package_path).unwrap();
    let accepted_package = std::fs::read(&package_path).unwrap();
    assert!(export_project_package(&project, &package_path).is_err());
    assert_eq!(std::fs::read(&package_path).unwrap(), accepted_package);
    let package = voxy_assets::ResourcePackage::from_bytes(
        &std::fs::read(&package_path).unwrap(),
        voxy_assets::PackageLimits {
            max_entries: 4096,
            max_payload_bytes: 64 * 1024 * 1024,
            max_document_bytes: 256 * 1024 * 1024,
        },
    )
    .unwrap();
    for name in ["external.gltf", "mesh.bin", "texture.png"] {
        assert!(package.paths().any(|path| path == name), "missing {name}");
    }
    std::fs::remove_dir_all(&directory).unwrap();
    assert!(!project.exists());
    let materialized = packaged_game::MaterializedPackage::open(&package_path).unwrap();
    let mut restored = App::from_project(&materialized.project_path().unwrap()).unwrap();
    assert_eq!(restored.authoring_document().unwrap(), document);
    let deadline = Instant::now() + Duration::from_secs(10);
    while restored.catalog.snapshot(&id).is_none() {
        restored.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "packaged external model did not load"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let restored_asset = restored.catalog.snapshot(&id).unwrap();
    assert_eq!(
        restored_asset.value().nodes.len(),
        accepted.value().nodes.len()
    );
    assert_eq!(
        restored_asset.value().images.len(),
        accepted.value().images.len()
    );
    for (restored_image, original_image) in restored_asset
        .value()
        .images
        .iter()
        .zip(&accepted.value().images)
    {
        assert_eq!(restored_image.width(), original_image.width());
        assert_eq!(restored_image.height(), original_image.height());
        assert_eq!(restored_image.rgba(), original_image.rgba());
    }
    restored.stop_workers().unwrap();
    drop(materialized);
    run_packaged_game(&package_path, ViewportMode::GameCheck).unwrap();
    std::fs::remove_file(package_path).unwrap();
}

#[test]
fn incompatible_gltf_hierarchy_reload_retains_resource_and_edited_owners() {
    let directory = std::env::temp_dir().join(format!("voxy-hierarchy-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::copy(
        fixture().join("assembly.glb"),
        directory.join("assembly.glb"),
    )
    .unwrap();
    let mut app = App::new(&directory.join("assembly.glb"), false).unwrap();
    ready(&mut app);
    app.expand_model().unwrap();
    let expected = app.authoring_document().unwrap();
    let resource = app
        .catalog
        .snapshot(app.id.as_ref().expect("model-only fixture"))
        .unwrap();
    let mut bytes = std::fs::read(directory.join("assembly.glb")).unwrap();
    let name = b"Textured cube";
    let index = bytes
        .windows(name.len())
        .position(|bytes| bytes == name)
        .unwrap();
    bytes[index..index + name.len()].copy_from_slice(b"Renamed model");
    std::fs::write(directory.join("assembly.glb"), bytes).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.failed_at.is_none() {
        app.tick().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(Arc::ptr_eq(
        &resource,
        &app.catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .unwrap()
    ));
    assert_eq!(app.authoring_document().unwrap(), expected);
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

pub(crate) fn write_glb(path: &std::path::Path, spec: &serde_json::Value, binary: &[u8]) {
    let mut json = serde_json::to_vec(spec).unwrap();
    json.resize(json.len().next_multiple_of(4), b' ');
    let mut data = binary.to_vec();
    data.resize(data.len().next_multiple_of(4), 0);
    let size = u32::try_from(12 + 8 + json.len() + 8 + data.len()).unwrap();
    let mut bytes = b"glTF".to_vec();
    bytes.extend(2_u32.to_le_bytes());
    bytes.extend(size.to_le_bytes());
    bytes.extend(u32::try_from(json.len()).unwrap().to_le_bytes());
    bytes.extend(b"JSON");
    bytes.extend(json);
    bytes.extend(u32::try_from(data.len()).unwrap().to_le_bytes());
    bytes.extend(b"BIN\0");
    bytes.extend(data);
    std::fs::write(path, bytes).unwrap();
}
#[test]
fn multi_material_nodes_preserve_parent_transforms_and_all_six_min_filters() {
    let bytes = std::fs::read(fixture().join("assembly.glb")).unwrap();
    let json_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let mut spec: serde_json::Value = serde_json::from_slice(&bytes[20..20 + json_length]).unwrap();
    let binary = &bytes[28 + json_length..];
    spec["accessors"][2]["count"] = 18.into();
    spec["accessors"].as_array_mut().unwrap().push(serde_json::json!({"bufferView":2,"byteOffset":36,"componentType":5123,"count":18,"type":"SCALAR"}));
    spec["materials"].as_array_mut().unwrap().push(serde_json::json!({"pbrMetallicRoughness":{"baseColorFactor":[0.1,0.5,1,1],"baseColorTexture":{"index":0},"metallicFactor":0}}));
    let mut second = spec["meshes"][0]["primitives"][0].clone();
    second["indices"] = 3.into();
    second["material"] = 1.into();
    spec["meshes"][0]["primitives"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let directory =
        std::env::temp_dir().join(format!("voxy-multi-material-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    for (code, min, mip, use_mips) in [
        (
            9728,
            voxy_render::TextureFilter::Nearest,
            voxy_render::TextureFilter::Linear,
            false,
        ),
        (
            9729,
            voxy_render::TextureFilter::Linear,
            voxy_render::TextureFilter::Linear,
            false,
        ),
        (
            9984,
            voxy_render::TextureFilter::Nearest,
            voxy_render::TextureFilter::Nearest,
            true,
        ),
        (
            9985,
            voxy_render::TextureFilter::Linear,
            voxy_render::TextureFilter::Nearest,
            true,
        ),
        (
            9986,
            voxy_render::TextureFilter::Nearest,
            voxy_render::TextureFilter::Linear,
            true,
        ),
        (
            9987,
            voxy_render::TextureFilter::Linear,
            voxy_render::TextureFilter::Linear,
            true,
        ),
    ] {
        spec["samplers"] =
            serde_json::json!([{"minFilter":code,"magFilter":9729,"wrapS":10497,"wrapT":33648}]);
        spec["textures"][0]["sampler"] = 0.into();
        write_glb(&directory.join("multi.glb"), &spec, binary);
        let provider = FileInputs::new(&directory).unwrap();
        let mut inputs = voxy_assets::ImportInputs::new(4, 32 * 1024 * 1024);
        let asset = import::load(
            &SourcePath::new("multi.glb").unwrap(),
            &provider,
            &mut inputs,
        )
        .unwrap();
        assert_eq!(asset.nodes.len(), 7);
        assert!(asset.nodes[1].mesh.is_none());
        assert!(asset.nodes[2].mesh.is_none());
        assert_eq!(asset.nodes[3].parent, Some(1));
        assert_eq!(asset.nodes[4].parent, Some(1));
        assert_eq!(asset.nodes[5].parent, Some(2));
        assert_eq!(asset.nodes[3].local, Transform::default());
        assert_eq!(asset.nodes[3].geometry_key, asset.nodes[5].geometry_key);
        assert_ne!(asset.nodes[3].geometry_key, asset.nodes[4].geometry_key);
        let color = asset.nodes[4].mesh.as_ref().unwrap().vertices()[0].color;
        assert_eq!(color, [0.1, 0.5, 1., 1.]);
        assert_eq!(asset.mesh.indices().len(), 72);
        assert_eq!(asset.nodes[3].sampling.min_filter, min);
        assert_eq!(asset.nodes[3].sampling.mipmap_filter, Some(mip));
        assert_eq!(asset.nodes[3].use_mips, use_mips);
        let limits = import::node_checkpoint::NodeCheckpointLimits::default();
        let checkpoint = import::node_checkpoint::ImportedNodesCheckpoint::capture(
            &asset.nodes,
            &asset.images,
            limits,
        )
        .unwrap();
        let path = directory.join(format!("nodes-{code}.json"));
        std::fs::write(&path, checkpoint.to_json(limits.max_bytes).unwrap()).unwrap();
        if let Some(output) = std::env::var_os("VOXY_NODE_CHECKPOINT_EVIDENCE_DIR") {
            let output = std::path::PathBuf::from(output);
            std::fs::create_dir_all(&output).unwrap();
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output.join(format!("nodes-{code}.json")))
                .unwrap();
            std::io::Write::write_all(&mut file, &std::fs::read(&path).unwrap()).unwrap();
        }
        let asset_limits = import::asset_checkpoint::EditorAssetCheckpointLimits::default();
        let asset_record = import::asset_checkpoint::EditorAssetCheckpoint::capture(
            &[std::sync::Arc::new(asset.clone())],
            asset_limits,
        )
        .unwrap();
        let asset_path = directory.join(format!("asset-{code}.voxyeda"));
        std::fs::write(&asset_path, asset_record.to_bytes(asset_limits).unwrap()).unwrap();
        import::asset_checkpoint::retain_evidence(
            &format!("static-{code}.voxyeda"),
            &asset_record.to_bytes(asset_limits).unwrap(),
        );
        std::fs::remove_file(directory.join("multi.glb")).unwrap();
        let restored_nodes = import::node_checkpoint::ImportedNodesCheckpoint::from_json(
            &std::fs::read_to_string(&path).unwrap(),
            limits.max_bytes,
        )
        .unwrap()
        .prepare(limits)
        .unwrap();
        let restored_assets = import::asset_checkpoint::EditorAssetCheckpoint::from_bytes(
            &std::fs::read(&asset_path).unwrap(),
            asset_limits,
        )
        .unwrap()
        .prepare(asset_limits)
        .unwrap();
        let restored = &restored_assets[0];
        assert_eq!(
            import::asset_checkpoint::EditorAssetCheckpoint::capture(
                &restored_assets,
                asset_limits
            )
            .unwrap(),
            asset_record
        );
        assert_eq!(
            import::node_checkpoint::ImportedNodesCheckpoint::capture(
                &restored_nodes.nodes,
                &restored_nodes.images,
                limits
            )
            .unwrap(),
            checkpoint
        );
        assert_eq!(
            import::node_checkpoint::ImportedNodesCheckpoint::capture(
                &restored.nodes,
                &restored.images,
                limits
            )
            .unwrap(),
            checkpoint
        );
        assert_eq!(
            restored.nodes[3].geometry_key,
            restored.nodes[5].geometry_key
        );
        for frame in 0..120 {
            let world = |nodes: &[import::ImportedNode]| {
                let instance = Mat4::from_rotation_y(frame as f32 / 119.);
                let mut worlds = Vec::new();
                for node in nodes {
                    let local = node.local.matrix().unwrap();
                    let parent: Mat4 = node.parent.map_or(instance, |p| worlds[p]);
                    worlds.push(parent * local);
                }
                worlds
                    .iter()
                    .map(|m| m.to_cols_array().map(f32::to_bits))
                    .collect::<Vec<_>>()
            };
            assert_eq!(world(&asset.nodes), world(&restored.nodes));
        }
        write_glb(&directory.join("multi.glb"), &spec, binary);
    }
    let mut app = App::new(&directory.join("multi.glb"), false).unwrap();
    ready(&mut app);
    app.expand_model().unwrap();
    assert_eq!(app.instances.len(), 8);
    let child = app.instances[4];
    let parent = app.instances[2];
    assert_eq!(app.scene.parent(child).unwrap(), Some(parent));
    assert_eq!(
        app.scene.world_matrix(child).unwrap(),
        app.scene.world_matrix(parent).unwrap()
    );
    let document = app.authoring_document().unwrap();
    app.toggle_play().unwrap();
    app.toggle_play().unwrap();
    assert_eq!(app.authoring_document().unwrap(), document);
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn gpu_residency_requires_active_renderable_descendants() {
    let root = fixture();
    let mut app =
        App::from_manifest(&root.join("assets.json"), AssetId("assembly".into()), false).unwrap();
    ready(&mut app);
    app.expand_model().unwrap();
    let id = AssetId("assembly".into());
    assert!(app.required_gpu_assets().contains(&id));
    let before = app.authoring_document().unwrap();
    let parent = app.instances[0];
    app.scene.set_active(parent, false).unwrap();
    assert!(app.required_gpu_assets().is_empty());
    app.scene.set_active(parent, true).unwrap();
    for node in &app.instances[2..] {
        app.scene.set_active(*node, false).unwrap();
    }
    assert!(app.required_gpu_assets().is_empty());
    app.scene.set_active(app.instances[2], true).unwrap();
    assert!(app.required_gpu_assets().contains(&id));
    for node in &app.instances[2..] {
        app.scene.set_active(*node, true).unwrap();
    }
    assert_eq!(app.authoring_document().unwrap(), before);
    app.imports.take().unwrap().close().join().unwrap();
}

#[test]
fn camera_and_light_without_model_round_trip_and_unknown_assets_reject() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let mut document = app.authoring_document().unwrap();
    let mut light = document.objects[0].clone();
    light.id = voxy_scene::ObjectId("standalone-light".into());
    light.name = "Standalone camera and light".into();
    light.components.clear();
    light.components.insert(
        "editor.light.v1".into(),
        serde_json::to_value(crate::DirectionalLight::default()).unwrap(),
    );
    light.components.insert(
        "editor.camera.v1".into(),
        serde_json::to_value(crate::EditorCamera::default()).unwrap(),
    );
    document.objects.push(light);
    let root = std::env::temp_dir().join(format!("voxy-nonrender-scene-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("scene.json");
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    app.configure_scene(&path).unwrap();
    assert_eq!(app.authoring_document().unwrap(), document);
    let light_node = *app.instances.last().unwrap();
    assert!(
        app.scene
            .component::<ModelInstance>(light_node)
            .unwrap()
            .is_none()
    );
    assert!(
        app.scene
            .component::<crate::DirectionalLight>(light_node)
            .unwrap()
            .is_some()
    );
    app.save_authoring().unwrap();
    app.load_authoring().unwrap();
    assert_eq!(app.authoring_document().unwrap(), document);
    let mut invalid = document.clone();
    invalid
        .objects
        .last_mut()
        .unwrap()
        .components
        .insert("editor.model.v1".into(), "unknown-resource".into());
    std::fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(app.load_authoring().is_err());
    assert_eq!(app.authoring_document().unwrap(), document);
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn geometry_picking_skips_fog_only_objects_and_preserves_their_document() {
    let root = fixture();
    let mut app =
        App::from_manifest(&root.join("assets.json"), AssetId("assembly".into()), false).unwrap();
    ready(&mut app);
    app.expand_model().unwrap();
    let fog = app.scene.spawn(None, Transform::default()).unwrap();
    app.scene.set_name(fog, "Fog").unwrap();
    app.scene
        .insert_component(fog, voxy_scene::FogVolume::default())
        .unwrap();
    app.instances.push(fog);
    app.object_ids.push(voxy_scene::ObjectId("fog-only".into()));
    app.commit_authoring().unwrap();
    app.camera.orbit(Vec2::new(25., 15.));
    let size = Vec2::new(800., 600.);
    for perspective in [true, false] {
        app.camera.perspective = perspective;
        let point = app
            .scene
            .world_matrix(app.instances[2])
            .unwrap()
            .w_axis
            .truncate();
        let projected = app.camera.matrix(size).unwrap().project_point3(point);
        let cursor = Vec2::new(projected.x + 1., 1. - projected.y) * size * 0.5;
        assert!(app.pick_model(cursor, size).unwrap());
        assert_eq!(app.selected, 2);
    }
    let document = app.authoring_document().unwrap();
    assert!(
        document
            .objects
            .iter()
            .any(|o| o.id.0 == "fog-only" && o.components.contains_key("scene.fog.v1"))
    );
    assert!(app.scene.component::<ModelInstance>(fog).unwrap().is_none());
}

#[test]
fn watcher_reloads_valid_model_sources_larger_than_manifest_budget() {
    let directory = std::env::temp_dir().join(format!(
        "voxy-large-source-watch-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let source = directory.join("large.obj");
    let model = |x| {
        format!(
            "v {x} 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n#{}\n",
            "padding".repeat(10000)
        )
    };
    std::fs::write(&source, model(0)).unwrap();
    let mut app = App::new(&source, false).unwrap();
    ready(&mut app);
    // Settle at least two real scans, so an unknown initial fingerprint cannot
    // accidentally make a broken watcher appear to detect the later edit.
    let settle = Instant::now() + Duration::from_millis(650);
    while Instant::now() < settle {
        app.tick().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    let id = app.id.clone().unwrap();
    let accepted = app.catalog.snapshot(&id).unwrap();
    assert_eq!(accepted.value().mesh.vertices()[0].position[0], 0.);
    std::fs::write(&source, model(2)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()) {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "watcher missed a valid source above 64 KiB"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        app.catalog.snapshot(&id).unwrap().value().mesh.vertices()[0].position[0],
        2.
    );
    let last_good = app.catalog.snapshot(&id).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&source)
        .unwrap()
        .set_len(u64::try_from(import::SOURCE_BYTES).unwrap() + 1)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.failed_at.is_none() {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "oversized source was not rejected"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(Arc::ptr_eq(&last_good, &app.catalog.snapshot(&id).unwrap()));
    std::fs::write(&source, model(3)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Arc::ptr_eq(&last_good, &app.catalog.snapshot(&id).unwrap()) {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "source did not recover below budget"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        app.catalog.snapshot(&id).unwrap().value().mesh.vertices()[0].position[0],
        3.
    );
    assert!(app.recovered);
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn manifest_retarget_replaces_watched_dependencies_after_failure_without_scene_edits() {
    let directory = std::env::temp_dir().join(format!(
        "voxy-manifest-retarget-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let model = |x| format!("v {x} 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n");
    let manifest = |source| {
        serde_json::to_vec(&serde_json::json!({
            "version":1,"assets":[{"asset":"mesh","source":source}]
        }))
        .unwrap()
    };
    std::fs::write(directory.join("a.obj"), model(0)).unwrap();
    std::fs::write(directory.join("b.obj"), model(2)).unwrap();
    std::fs::write(directory.join("assets.json"), manifest("a.obj")).unwrap();
    let mut app = App::from_manifest(
        &directory.join("assets.json"),
        AssetId("mesh".into()),
        false,
    )
    .unwrap();
    ready(&mut app);
    app.edit_key(KeyCode::KeyD).unwrap();
    let document = app.authoring_document().unwrap();
    let settle = Instant::now() + Duration::from_millis(650);
    while Instant::now() < settle {
        app.tick().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    let id = AssetId("mesh".into());
    let accepted = app.catalog.snapshot(&id).unwrap();
    std::fs::write(directory.join("assets.json"), b"invalid manifest").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !matches!(
        app.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Failed(_))
    ) {
        app.tick().unwrap();
        assert!(Instant::now() < deadline, "invalid manifest did not fail");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()));
    assert_eq!(app.authoring_document().unwrap(), document);
    std::fs::write(directory.join("assets.json"), manifest("b.obj")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()) {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "manifest retarget did not recover"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        app.catalog.snapshot(&id).unwrap().value().mesh.vertices()[0].position[0],
        2.
    );
    assert!(app.sources.affected([AssetId("a.obj".into())]).is_empty());
    assert_eq!(
        app.sources.affected([AssetId("b.obj".into())]),
        vec![id.clone()]
    );
    let settle = Instant::now() + Duration::from_millis(650);
    while Instant::now() < settle {
        app.tick().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    let switched = app.catalog.snapshot(&id).unwrap();
    std::fs::write(directory.join("a.obj"), b"obsolete broken source").unwrap();
    let settle = Instant::now() + Duration::from_millis(650);
    while Instant::now() < settle {
        app.tick().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(Arc::ptr_eq(&switched, &app.catalog.snapshot(&id).unwrap()));
    assert!(matches!(
        app.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Ready)
    ));
    std::fs::write(directory.join("b.obj"), model(3)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Arc::ptr_eq(&switched, &app.catalog.snapshot(&id).unwrap()) {
        app.tick().unwrap();
        assert!(Instant::now() < deadline, "new dependency was not watched");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        app.catalog.snapshot(&id).unwrap().value().mesh.vertices()[0].position[0],
        3.
    );
    assert_eq!(app.authoring_document().unwrap(), document);
    let expanded = serde_json::json!({"version":1,"assets":[
        {"asset":"mesh","source":"b.obj"},
        {"asset":"new-model","source":"a.obj"},
        {"asset":"sound","source":"tone.wav"}
    ]});
    std::fs::write(
        directory.join("assets.json"),
        serde_json::to_vec(&expanded).unwrap(),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !app.available.contains(&AssetId("new-model".into())) {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "new manifest model is absent from inventory"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!app.available.contains(&AssetId("sound".into())));
    assert_eq!(app.available.len(), 2);
    // Inventory now arrives before model preparation completes. Observe the
    // accepted model only after the original resource barrier has drained it.
    app.begin_model_checkpoint().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !app.poll_model_checkpoint().unwrap() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let last_good = app.catalog.snapshot(&id).unwrap();
    assert!(app.resume_model_checkpoint().unwrap());
    let removed = serde_json::json!({"version":1,"assets":[
        {"asset":"new-model","source":"a.obj"}
    ]});
    std::fs::write(
        directory.join("assets.json"),
        serde_json::to_vec(&removed).unwrap(),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.available.contains(&id) {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "removed model remains in inventory"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.available, BTreeSet::from([AssetId("new-model".into())]));
    assert!(Arc::ptr_eq(&last_good, &app.catalog.snapshot(&id).unwrap()));
    assert_eq!(app.authoring_document().unwrap(), document);
    std::fs::write(
        directory.join("assets.json"),
        b"broken manifest after removal",
    )
    .unwrap();
    let settle = Instant::now() + Duration::from_millis(650);
    while Instant::now() < settle {
        app.tick().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.available, BTreeSet::from([AssetId("new-model".into())]));
    assert!(Arc::ptr_eq(&last_good, &app.catalog.snapshot(&id).unwrap()));
    assert_eq!(app.authoring_document().unwrap(), document);
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn empty_project_manifest_inventory_updates_without_model_import_and_respects_barrier() {
    let directory = std::env::temp_dir().join(format!(
        "voxy-empty-inventory-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let project = project::create(&directory, "Empty inventory").unwrap();
    let mut app = App::from_project(&project).unwrap();
    assert!(app.instances.is_empty() && app.id.is_none());
    let document = app.authoring_document().unwrap();
    let write_manifest = |name: &str| {
        std::fs::write(
            directory.join("assets.json"),
            serde_json::to_vec(&serde_json::json!({
                "version":1,"assets":[{"asset":name,"source":"not-created.obj"}]
            }))
            .unwrap(),
        )
        .unwrap();
    };
    write_manifest("new-model");
    let expected = BTreeSet::from([AssetId("new-model".into())]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.available != expected {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "empty project inventory did not refresh"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.catalog.snapshot(&AssetId("new-model".into())).is_none());
    assert_eq!(app.catalog.pending(), 0);
    assert!(app.reload.is_empty());
    std::fs::write(directory.join("assets.json"), b"invalid manifest").unwrap();
    let settle = Instant::now() + Duration::from_millis(650);
    while Instant::now() < settle {
        app.tick().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.available, expected);
    app.begin_model_checkpoint().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !app.poll_model_checkpoint().unwrap() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    write_manifest("after-barrier");
    for _ in 0..10 {
        app.tick().unwrap();
    }
    assert_eq!(app.available, expected);
    assert!(app.resume_model_checkpoint().unwrap());
    let expected = BTreeSet::from([AssetId("after-barrier".into())]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.available != expected {
        app.tick().unwrap();
        assert!(Instant::now() < deadline, "manifest watcher did not resume");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.authoring_document().unwrap(), document);
    assert_eq!(app.catalog.pending(), 0);
    assert!(app.instances.is_empty() && app.reload.is_empty());
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn sampler_only_reload_preserves_geometry_and_survives_source_free_project_package() {
    let temp = crate::project_prefab_tests::Temp::new();
    let root = temp.0.join("game");
    let project = project::create(&root, "Sampler reload").unwrap();
    let bytes = std::fs::read(fixture().join("assembly.glb")).unwrap();
    let json_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let mut spec: serde_json::Value = serde_json::from_slice(&bytes[20..20 + json_length]).unwrap();
    let binary = &bytes[28 + json_length..];
    spec["samplers"] =
        serde_json::json!([{"minFilter":9987,"magFilter":9729,"wrapS":10497,"wrapT":10497}]);
    spec["textures"][0]["sampler"] = 0.into();
    write_glb(&root.join("assembly.glb"), &spec, binary);
    std::fs::write(
        root.join("assets.json"),
        br#"{"version":1,"assets":[{"asset":"assembly","source":"assembly.glb"}]}"#,
    )
    .unwrap();
    let id = AssetId("assembly".into());
    let mut app = App::from_project(&project).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.catalog.snapshot(&id).is_none() {
        app.tick().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    app.edit_key(KeyCode::ArrowRight).unwrap();
    let document = app.authoring_document().unwrap();
    app.save_authoring().unwrap();
    let before = app.catalog.snapshot(&id).unwrap();
    let node = before
        .value()
        .nodes
        .iter()
        .position(|node| node.image.is_some())
        .unwrap();
    assert!(before.value().nodes[node].use_mips);
    spec["samplers"][0] =
        serde_json::json!({"minFilter":9728,"magFilter":9728,"wrapS":33071,"wrapT":33648});
    write_glb(&root.join("assembly.glb"), &spec, binary);
    let deadline = Instant::now() + Duration::from_secs(10);
    while Arc::ptr_eq(&before, &app.catalog.snapshot(&id).unwrap()) {
        app.tick().unwrap();
        assert!(Instant::now() < deadline, "sampler edit was not reloaded");
        std::thread::sleep(Duration::from_millis(5));
    }
    let accepted = app.catalog.snapshot(&id).unwrap();
    let sampling = accepted.value().nodes[node].sampling;
    assert_eq!(sampling.min_filter, voxy_render::TextureFilter::Nearest);
    assert_eq!(sampling.mag_filter, voxy_render::TextureFilter::Nearest);
    assert_eq!(sampling.wrap_u, voxy_render::TextureWrap::Clamp);
    assert_eq!(sampling.wrap_v, voxy_render::TextureWrap::Mirror);
    assert!(!accepted.value().nodes[node].use_mips);
    let geometry = |asset: &import::EditorAsset| {
        asset
            .mesh
            .vertices()
            .iter()
            .map(|vertex| {
                (
                    vertex.position.map(f32::to_bits),
                    vertex.uv.map(f32::to_bits),
                    vertex.color.map(f32::to_bits),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(geometry(before.value()), geometry(accepted.value()));
    assert_eq!(
        before.value().mesh.indices(),
        accepted.value().mesh.indices()
    );
    for (old, new) in before.value().images.iter().zip(&accepted.value().images) {
        assert_eq!(old.rgba(), new.rgba());
    }
    assert_eq!(app.authoring_document().unwrap(), document);
    app.stop_workers().unwrap();
    let output = temp.0.join("sampler.vpak");
    export_project_package(&project, &output).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let materialized = packaged_game::MaterializedPackage::open(&output).unwrap();
    let mut restored = App::from_project(&materialized.project_path().unwrap()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while restored.catalog.snapshot(&id).is_none() {
        restored.tick().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let packaged = restored.catalog.snapshot(&id).unwrap();
    assert_eq!(geometry(packaged.value()), geometry(accepted.value()));
    assert_eq!(
        packaged.value().mesh.indices(),
        accepted.value().mesh.indices()
    );
    assert_eq!(packaged.value().nodes[node].sampling, sampling);
    assert!(!packaged.value().nodes[node].use_mips);
    assert_eq!(packaged.value().images.len(), accepted.value().images.len());
    for (original, packaged) in accepted.value().images.iter().zip(&packaged.value().images) {
        assert_eq!(original.rgba(), packaged.rgba());
    }
    assert_eq!(restored.authoring_document().unwrap(), document);
    restored.stop_workers().unwrap();
    run_packaged_game(&output, ViewportMode::GameCheck).unwrap();
}
