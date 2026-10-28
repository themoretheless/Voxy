use super::*;
use voxy_model::{ModelAsset, ModelGeometry, ModelLimits};
use voxy_render::{LodArchiveLimits, LodSurface};

#[test]
fn accepted_geometry_and_animation_files_restore_without_sources_and_match_all_clips() {
    use voxy_render::{GeometryArchiveLimits, ImageCheckpoint, SceneMeshCheckpoint};
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "voxy-model-geometry-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let sources = root.join("sources");
    std::fs::create_dir(&sources).unwrap();
    std::fs::write(
        sources.join("fox.glb"),
        include_bytes!("../../../voxy_render/examples/assets/fox/Fox.glb"),
    )
    .unwrap();
    let provider = FileInputs::new(&sources).unwrap();
    let mut inputs = ImportInputs::new(4, 4 * 1024 * 1024);
    let asset = load(&SourcePath::new("fox.glb").unwrap(), &provider, &mut inputs).unwrap();
    let imported = inputs
        .finish(asset, |id, max| provider.read(id, max))
        .unwrap();
    let asset = imported.value();
    let asset_limits = super::asset_checkpoint::EditorAssetCheckpointLimits::default();
    let asset_record = super::asset_checkpoint::EditorAssetCheckpoint::capture(
        &[std::sync::Arc::new(asset.clone())],
        asset_limits,
    )
    .unwrap();
    let asset_bytes = asset_record.to_bytes(asset_limits).unwrap();
    std::fs::write(root.join("editor-asset.archive"), &asset_bytes).unwrap();
    super::asset_checkpoint::retain_evidence("fox.voxyeda", &asset_bytes);
    let limits = GeometryArchiveLimits::default();
    let base = asset.mesh.checkpoint();
    std::fs::write(root.join("base.geometry"), base.to_bytes(limits).unwrap()).unwrap();
    let model = asset.animated.as_ref().unwrap();
    let model_limits = voxy_model::ModelAssetCheckpointLimits::default();
    let model_record =
        voxy_model::ModelAssetCheckpoint::capture(std::slice::from_ref(model), model_limits)
            .unwrap();
    std::fs::write(
        root.join("model.archive"),
        model_record.to_bytes(model_limits).unwrap(),
    )
    .unwrap();
    let animation_limits = voxy_animation::AnimationAssetLimits::default();
    let animations = voxy_animation::AnimationAssetCheckpoint::capture(
        std::slice::from_ref(&model.skeleton),
        &model.animations,
        animation_limits,
    )
    .unwrap();
    std::fs::write(
        root.join("animation.json"),
        animations.to_json(animation_limits.max_bytes).unwrap(),
    )
    .unwrap();
    let primitive_limits = voxy_model::MorphArchiveLimits::default();
    for (index, primitive) in model.primitives.iter().enumerate() {
        let record = primitive.checkpoint(primitive_limits).unwrap();
        std::fs::write(
            root.join(format!("primitive-{index}.geometry")),
            record.as_bytes(),
        )
        .unwrap();
    }
    let skin_limits = voxy_render::SkinnedMeshSetLimits::default();
    let skin_sources: Vec<_> = model
        .primitives
        .iter()
        .filter_map(|p| match &p.geometry {
            ModelGeometry::Skinned(mesh) => Some(mesh.clone()),
            ModelGeometry::Static(_) => None,
        })
        .collect();
    let skins = voxy_render::SkinnedMeshSetCheckpoint::capture(&skin_sources, skin_limits).unwrap();
    std::fs::write(
        root.join("skin-streams.geometry"),
        skins.to_bytes(skin_limits).unwrap(),
    )
    .unwrap();
    assert!(!asset.images.is_empty());
    for (index, image) in asset.images.iter().enumerate() {
        std::fs::write(
            root.join(format!("image-{index}.geometry")),
            image.checkpoint().to_bytes(limits).unwrap(),
        )
        .unwrap();
    }
    std::fs::remove_dir_all(&sources).unwrap();
    let loaded_assets = super::asset_checkpoint::EditorAssetCheckpoint::from_bytes(
        &std::fs::read(root.join("editor-asset.archive")).unwrap(),
        asset_limits,
    )
    .unwrap()
    .prepare(asset_limits)
    .unwrap();
    assert_eq!(
        super::asset_checkpoint::EditorAssetCheckpoint::capture(&loaded_assets, asset_limits)
            .unwrap(),
        asset_record
    );
    assert_eq!(loaded_assets[0].mesh.checkpoint(), asset.mesh.checkpoint());
    assert_eq!(
        loaded_assets[0]
            .images
            .iter()
            .map(ImageAsset::checkpoint)
            .collect::<Vec<_>>(),
        asset
            .images
            .iter()
            .map(ImageAsset::checkpoint)
            .collect::<Vec<_>>()
    );
    let loaded_models = voxy_model::ModelAssetCheckpoint::from_bytes(
        &std::fs::read(root.join("model.archive")).unwrap(),
        model_limits,
    )
    .unwrap()
    .prepare(model_limits)
    .unwrap();
    let loaded_model = loaded_assets[0].animated.as_ref().unwrap();
    assert_eq!(
        loaded_model.animations.len(),
        loaded_models[0].animations.len()
    );
    assert_eq!(
        voxy_model::ModelAssetCheckpoint::capture(&loaded_models, model_limits).unwrap(),
        model_record
    );
    let restored_base = SceneMeshCheckpoint::from_bytes(
        &std::fs::read(root.join("base.geometry")).unwrap(),
        limits,
    )
    .unwrap()
    .prepare(limits)
    .unwrap();
    assert_eq!(restored_base.checkpoint(), base);
    let mut restored = Vec::new();
    for index in 0..model.primitives.len() {
        let bytes = std::fs::read(root.join(format!("primitive-{index}.geometry"))).unwrap();
        restored.push(
            voxy_model::ModelPrimitiveCheckpoint::from_bytes(&bytes, primitive_limits)
                .unwrap()
                .prepare(primitive_limits)
                .unwrap(),
        );
        assert_eq!(
            restored[index].checkpoint(primitive_limits).unwrap(),
            model.primitives[index]
                .checkpoint(primitive_limits)
                .unwrap()
        );
    }
    let skin_record = voxy_render::SkinnedMeshSetCheckpoint::from_bytes(
        &std::fs::read(root.join("skin-streams.geometry")).unwrap(),
        skin_limits,
    )
    .unwrap();
    let skin_meshes = skin_record.prepare(skin_limits).unwrap();
    assert_eq!(
        voxy_render::SkinnedMeshSetCheckpoint::capture(&skin_meshes, skin_limits).unwrap(),
        skins
    );
    let mut skin_meshes = skin_meshes.into_iter();
    for primitive in &mut restored {
        if let ModelGeometry::Skinned(mesh) = &mut primitive.geometry {
            let shared = skin_meshes.next().unwrap();
            assert_eq!(shared.checkpoint(), mesh.checkpoint());
            *mesh = shared;
        }
    }
    assert!(skin_meshes.next().is_none());
    for (index, image) in asset.images.iter().enumerate() {
        let restored = ImageCheckpoint::from_bytes(
            &std::fs::read(root.join(format!("image-{index}.geometry"))).unwrap(),
            limits,
        )
        .unwrap()
        .prepare(limits)
        .unwrap();
        assert_eq!(restored.checkpoint(), image.checkpoint());
        assert_eq!(
            restored
                .mip_chain()
                .iter()
                .map(voxy_render::ImageAsset::checkpoint)
                .collect::<Vec<_>>(),
            image
                .mip_chain()
                .iter()
                .map(voxy_render::ImageAsset::checkpoint)
                .collect::<Vec<_>>()
        );
    }
    let animations = voxy_animation::AnimationAssetCheckpoint::from_json(
        &std::fs::read_to_string(root.join("animation.json")).unwrap(),
        animation_limits.max_bytes,
    )
    .unwrap()
    .prepare(animation_limits)
    .unwrap();
    assert_eq!(animations.clips.len(), model.animations.len());
    for clip in 0..model.animations.len() {
        for frame in 0..120 {
            let pose = model
                .sample_pose_phase(Some(clip), f64::from(frame) / 119.)
                .unwrap();
            let palette = model.skin_matrices(&pose).unwrap();
            let restored_pose = loaded_model
                .sample_pose_phase(Some(clip), f64::from(frame) / 119.)
                .unwrap();
            assert_eq!(
                pose.checkpoint(),
                restored_pose.checkpoint(),
                "clip {clip} frame {frame}"
            );
            let restored_palette = loaded_model.skin_matrices(&restored_pose).unwrap();
            let words = |palette: &[Mat4]| {
                palette
                    .iter()
                    .map(|m| m.to_cols_array().map(f32::to_bits))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                words(&palette),
                words(&restored_palette),
                "clip {clip} frame {frame}"
            );
            for (original, restored) in model.primitives.iter().zip(&loaded_model.primitives) {
                match (&original.geometry, &restored.geometry) {
                    (ModelGeometry::Skinned(a), ModelGeometry::Skinned(b)) => {
                        assert_eq!(
                            a.validate_render_pose(&palette, Mat4::IDENTITY),
                            b.validate_render_pose(&restored_palette, Mat4::IDENTITY)
                        );
                        let a = a
                            .posed_scene_mesh(&palette, Mat4::IDENTITY, original.color)
                            .unwrap();
                        let b = b
                            .posed_scene_mesh(&restored_palette, Mat4::IDENTITY, restored.color)
                            .unwrap();
                        assert_eq!(a.checkpoint(), b.checkpoint(), "frame {frame}");
                    }
                    (ModelGeometry::Static(a), ModelGeometry::Static(b)) => {
                        assert_eq!(a.checkpoint(), b.checkpoint())
                    }
                    _ => panic!("primitive kind changed"),
                }
            }
        }
    }
}

#[test]
fn observed_skeletal_lod_import_keeps_clips_source_and_last_good() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "voxy-animated-import-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let glb = include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb");
    let parsed = gltf::Gltf::from_slice(glb).unwrap();
    let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let mut document: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    document["buffers"][0]["uri"] = "motion.bin".into();
    let json = serde_json::to_vec(&document).unwrap();
    let buffer = parsed.blob.unwrap();
    std::fs::write(root.join("motion.gltf"), &json).unwrap();
    std::fs::write(root.join("motion.bin"), &buffer).unwrap();
    let model = ModelAsset::parse(&json, &[&buffer], ModelLimits::default()).unwrap();
    let ModelGeometry::Skinned(mesh) = &model.primitives[0].geometry else {
        panic!("skin fixture required");
    };
    let positions: Vec<_> = mesh
        .vertices()
        .iter()
        .map(|vertex| vertex.position)
        .collect();
    let surface = LodSurface {
        positions: &positions,
        indices: mesh.indices(),
    };
    let proof = voxy_render::generate_subdivided_lod_witnesses(surface, surface, 0, 100).unwrap();
    let limits = LodArchiveLimits {
        bytes: 4096,
        positions: 3,
        levels: 2,
        indices: 6,
        cells: 2,
    };
    let certificate = voxy_render::encode_lod_archive(
        &positions,
        mesh.indices(),
        &[voxy_render::CertifiedLodSubdivisionVariant {
            indices: mesh.indices().to_vec(),
            source_to_variant: proof.source_to_approximation,
            variant_to_source: proof.approximation_to_source,
        }],
        limits,
    )
    .unwrap();
    std::fs::write(root.join("proof.lod"), &certificate).unwrap();
    std::fs::write(
        root.join("model.vmodel"),
        br#"{"version":1,"base":"motion.gltf","certificate":"proof.lod"}"#,
    )
    .unwrap();
    let provider = FileInputs::new(&root).unwrap();
    let source = SourcePath::new("model.vmodel").unwrap();
    let mut inputs = ImportInputs::new(4, 65536);
    let asset = load(&source, &provider, &mut inputs).unwrap();
    let imported = inputs
        .finish(asset, |id, limit| provider.read(id, limit))
        .unwrap();
    let dependencies: std::collections::BTreeSet<_> = imported
        .inputs()
        .observations()
        .keys()
        .map(|dependency| dependency.0.as_str())
        .collect();
    assert_eq!(
        dependencies,
        std::collections::BTreeSet::from([
            "model.vmodel",
            "motion.gltf",
            "motion.bin",
            "proof.lod"
        ])
    );
    let asset = imported.value();
    let whole_limits = super::asset_checkpoint::EditorAssetCheckpointLimits::default();
    let whole_checkpoint = super::asset_checkpoint::EditorAssetCheckpoint::capture(
        &[std::sync::Arc::new(asset.clone())],
        whole_limits,
    )
    .unwrap();
    std::fs::write(
        root.join("whole-skinned.voxyeda"),
        whole_checkpoint.to_bytes(whole_limits).unwrap(),
    )
    .unwrap();
    super::asset_checkpoint::retain_evidence(
        "skeletal-lod.voxyeda",
        &whole_checkpoint.to_bytes(whole_limits).unwrap(),
    );
    assert!(asset.lod.is_none()); // Never apply rest-space metadata as a static pose guarantee.
    let animated = asset.animated.as_ref().unwrap();
    let lod = asset.skinned_lod.as_ref().unwrap();
    let checkpoint_limits = voxy_render::SkinnedLodCheckpointLimits::default();
    let lod_checkpoint = lod.checkpoint(checkpoint_limits).unwrap();
    std::fs::write(
        root.join("accepted-skinned-lod.json"),
        lod_checkpoint.to_json(checkpoint_limits.max_bytes).unwrap(),
    )
    .unwrap();
    let model_limits = voxy_model::ModelAssetCheckpointLimits::default();
    let model_checkpoint =
        voxy_model::ModelAssetCheckpoint::capture(std::slice::from_ref(animated), model_limits)
            .unwrap();
    let mut shared_skins = voxy_model::ModelAssetCheckpoint::source_skin_meshes(
        std::slice::from_ref(animated),
        model_limits,
    )
    .unwrap();
    let model_skin_count = shared_skins.len();
    shared_skins.push(lod.source_mesh().as_ref().clone());
    let shared_skin_record =
        voxy_render::SkinnedMeshSetCheckpoint::capture(&shared_skins, model_limits.skins).unwrap();
    std::fs::write(
        root.join("accepted-shared-skins.archive"),
        shared_skin_record.to_bytes(model_limits.skins).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("accepted-model.archive"),
        model_checkpoint.to_bytes(model_limits).unwrap(),
    )
    .unwrap();
    assert!(!animated.animations.is_empty());
    let palette = animated
        .skin_matrices(&animated.skeleton.bind_pose())
        .unwrap();
    let pose = lod.prepare(&palette, Mat4::IDENTITY).unwrap();
    for (actual, expected) in asset.mesh.vertices().iter().zip(pose.positions()) {
        assert_eq!(
            actual.position.map(f32::to_bits),
            expected.map(f32::to_bits)
        );
    }
    assert_eq!(lod.indices(0).unwrap(), mesh.indices());
    // Two owners share the imported source, but evaluate distinct clips/clocks
    // and instance transforms before certifying world-space error.
    let mut evaluated = Vec::new();
    for (time, instance) in [
        (0.25, Mat4::IDENTITY),
        (0.75, Mat4::from_translation(Vec3::new(4.0, 2.0, 1.0))),
    ] {
        let sampled = animated.sample_pose(Some(0), time).unwrap();
        let palette = animated.skin_matrices(&sampled).unwrap();
        let prepared = lod.prepare(&palette, instance).unwrap();
        let baked = animated.scene_meshes(&sampled).unwrap();
        for (actual, vertex) in prepared.positions().iter().zip(baked[0].vertices()) {
            let expected = instance.transform_point3(Vec3::from_array(vertex.position));
            assert!(Vec3::from_array(*actual).abs_diff_eq(expected, 1e-6));
        }
        evaluated.push(prepared);
    }
    assert_ne!(evaluated[0].positions(), evaluated[1].positions());
    assert!(animated.sample_pose(Some(usize::MAX), 0.0).is_err());
    assert!(animated.sample_pose(Some(0), f32::NAN).is_err());
    // Ordinary import/catalog publication must retain the previous complete asset
    // when either a dependency or the certified source identity changes.
    let mut app = crate::App::new(&root.join("model.vmodel"), false).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app
        .catalog
        .snapshot(app.id.as_ref().expect("model-only fixture"))
        .is_none()
    {
        app.tick().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let before = app
        .catalog
        .snapshot(app.id.as_ref().expect("model-only fixture"))
        .unwrap();
    assert!(before.value().skinned_lod.is_some());
    std::fs::write(
        root.join("proof.lod"),
        &certificate[..certificate.len() - 1],
    )
    .unwrap();
    while app.failed_at.is_none() {
        app.tick().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let retained = app
        .catalog
        .snapshot(app.id.as_ref().expect("model-only fixture"))
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(&before, &retained));
    assert!(
        !retained
            .value()
            .animated
            .as_ref()
            .unwrap()
            .animations
            .is_empty()
    );
    std::fs::write(root.join("proof.lod"), &certificate).unwrap();
    while std::sync::Arc::ptr_eq(
        &before,
        &app.catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .unwrap(),
    ) {
        app.tick().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let repaired = app
        .catalog
        .snapshot(app.id.as_ref().expect("model-only fixture"))
        .unwrap();
    assert!(app.recovered);
    assert!(repaired.value().skinned_lod.is_some());
    app.stop_workers().unwrap();
    for filename in ["model.vmodel", "motion.gltf", "motion.bin", "proof.lod"] {
        std::fs::remove_file(root.join(filename)).unwrap();
    }
    let shared_skins = voxy_render::SkinnedMeshSetCheckpoint::from_bytes(
        &std::fs::read(root.join("accepted-shared-skins.archive")).unwrap(),
        model_limits.skins,
    )
    .unwrap()
    .prepare(model_limits.skins)
    .unwrap();
    let restored_models = voxy_model::ModelAssetCheckpoint::from_bytes(
        &std::fs::read(root.join("accepted-model.archive")).unwrap(),
        model_limits,
    )
    .unwrap()
    .prepare_with_skin_meshes(model_limits, &shared_skins[..model_skin_count])
    .unwrap();
    assert_eq!(
        voxy_model::ModelAssetCheckpoint::capture(&restored_models, model_limits).unwrap(),
        model_checkpoint
    );
    let whole_assets = super::asset_checkpoint::EditorAssetCheckpoint::from_bytes(
        &std::fs::read(root.join("whole-skinned.voxyeda")).unwrap(),
        whole_limits,
    )
    .unwrap()
    .prepare(whole_limits)
    .unwrap();
    assert_eq!(
        super::asset_checkpoint::EditorAssetCheckpoint::capture(&whole_assets, whole_limits)
            .unwrap(),
        whole_checkpoint
    );
    let restored_model = whole_assets[0].animated.as_ref().unwrap();
    let ModelGeometry::Skinned(restored_mesh) = &restored_model.primitives[0].geometry else {
        panic!("skin required")
    };
    let joined_lod = voxy_render::SkinnedLodCheckpoint::from_json(
        &std::fs::read_to_string(root.join("accepted-skinned-lod.json")).unwrap(),
        checkpoint_limits.max_bytes,
    )
    .unwrap()
    .prepare_with_mesh(
        std::sync::Arc::new(shared_skins[model_skin_count].clone()),
        checkpoint_limits,
    )
    .unwrap();
    assert_eq!(
        joined_lod.source_mesh().vertices().as_ptr(),
        shared_skins[0].vertices().as_ptr()
    );
    let restored_lod = whole_assets[0].skinned_lod.as_ref().unwrap();
    assert_eq!(
        restored_lod.source_mesh().vertices().as_ptr(),
        restored_mesh.vertices().as_ptr()
    );
    assert_eq!(
        restored_lod.source_mesh().indices().as_ptr(),
        restored_mesh.indices().as_ptr()
    );
    for phase in 0..120 {
        let phase = f64::from(phase) / 119.;
        let original_pose = animated.sample_pose_phase(Some(0), phase).unwrap();
        let restored_pose = restored_model.sample_pose_phase(Some(0), phase).unwrap();
        let model = Mat4::from_translation(Vec3::new(phase as f32, 0., 1.));
        let a = lod
            .prepare(&animated.skin_matrices(&original_pose).unwrap(), model)
            .unwrap();
        let b = restored_lod
            .prepare(
                &restored_model.skin_matrices(&restored_pose).unwrap(),
                model,
            )
            .unwrap();
        let positions = |p: &[[f32; 3]]| p.iter().map(|p| p.map(f32::to_bits)).collect::<Vec<_>>();
        assert_eq!(positions(a.positions()), positions(b.positions()));
        let levels = |v: &[voxy_render::LodLevel]| {
            v.iter()
                .map(|v| (v.object_error.to_bits(), v.index_count))
                .collect::<Vec<_>>()
        };
        assert_eq!(levels(a.levels()), levels(b.levels()));
        for level in 0..2 {
            assert_eq!(
                a.posed_scene_mesh(level, animated.primitives[0].color)
                    .unwrap()
                    .checkpoint(),
                b.posed_scene_mesh(level, restored_model.primitives[0].color)
                    .unwrap()
                    .checkpoint()
            );
        }
    }
    assert_eq!(
        restored_lod.checkpoint(checkpoint_limits).unwrap(),
        lod_checkpoint
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fox_texture_import_observes_external_image_and_preserves_animation() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("voxy-fox-texture-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let glb = include_bytes!("../../../voxy_render/examples/assets/fox/Fox.glb");
    let gltf = gltf::Gltf::from_slice(glb).unwrap();
    let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let mut document: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    let gltf::image::Source::View { view, .. } = gltf.images().next().unwrap().source() else {
        panic!("embedded image required");
    };
    let buffer = gltf.blob.as_ref().unwrap();
    let image = buffer[view.offset()..view.offset() + view.length()].to_vec();
    document["buffers"][0]["uri"] = "fox.bin".into();
    document["images"][0]
        .as_object_mut()
        .unwrap()
        .remove("bufferView");
    document["images"][0]["uri"] = "fox.png".into();
    std::fs::write(
        root.join("fox.gltf"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    std::fs::write(root.join("fox.bin"), buffer).unwrap();
    std::fs::write(root.join("fox.png"), &image).unwrap();
    let provider = FileInputs::new(&root).unwrap();
    let source = SourcePath::new("fox.gltf").unwrap();
    let mut inputs = ImportInputs::new(4, 4 * 1024 * 1024);
    let asset = load(&source, &provider, &mut inputs).unwrap();
    let imported = inputs
        .finish(asset, |id, limit| provider.read(id, limit))
        .unwrap();
    assert_eq!(
        imported
            .inputs()
            .observations()
            .keys()
            .map(|id| id.0.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["fox.gltf", "fox.bin", "fox.png"])
    );
    let asset = imported.value();
    assert!(asset.nodes.is_empty()); // Preserve the animated owner rather than creating static parts.
    assert_eq!(asset.images.len(), 1);
    let model = asset.animated.as_ref().unwrap();
    assert_eq!(model.animations.len(), 3);
    assert_eq!(model.primitives[0].base_color_texture.unwrap().image, 0);
    assert_eq!(asset.mesh.vertices().len(), 1728);
    assert!(
        asset
            .mesh
            .authored_normals()
            .unwrap()
            .iter()
            .all(|normal| *normal == [0.; 3])
    );
    std::fs::write(root.join("fox.png"), &image[..image.len() / 2]).unwrap();
    let mut inputs = ImportInputs::new(4, 4 * 1024 * 1024);
    assert!(load(&source, &provider, &mut inputs).is_err());
    document["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["texCoord"] = 1.into();
    assert!(
        ModelAsset::parse(
            &serde_json::to_vec(&document).unwrap(),
            &[buffer],
            ModelLimits::default()
        )
        .is_err()
    );
    std::fs::remove_dir_all(&root).unwrap();
}
