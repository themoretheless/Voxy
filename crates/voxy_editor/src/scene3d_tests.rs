#![allow(clippy::float_cmp)]
use crate::*;
fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/scene3d")
}
fn ready(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.catalog.snapshot(&app.id).is_none() {
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
    let resource = app.catalog.snapshot(&app.id).unwrap();
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
        &app.catalog.snapshot(&app.id).unwrap()
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
    let resource = app.catalog.snapshot(&app.id).unwrap();
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
        &app.catalog.snapshot(&app.id).unwrap()
    ));
    assert_eq!(app.authoring_document().unwrap(), expected);
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

fn write_glb(path: &std::path::Path, spec: &serde_json::Value, binary: &[u8]) {
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
