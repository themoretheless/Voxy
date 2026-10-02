use super::*;
use voxy_render::{LodArchiveLimits, LodSurface, ModelAsset, ModelGeometry, ModelLimits};

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
    assert!(asset.lod.is_none()); // Never apply rest-space metadata as a static pose guarantee.
    let animated = asset.animated.as_ref().unwrap();
    let lod = asset.skinned_lod.as_ref().unwrap();
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
    while app.catalog.snapshot(&app.id).is_none() {
        app.tick().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let before = app.catalog.snapshot(&app.id).unwrap();
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
    let retained = app.catalog.snapshot(&app.id).unwrap();
    assert!(std::sync::Arc::ptr_eq(&before, &retained));
    assert!(retained.value().animated.as_ref().unwrap().animations.len() > 0);
    std::fs::write(root.join("proof.lod"), &certificate).unwrap();
    while std::sync::Arc::ptr_eq(&before, &app.catalog.snapshot(&app.id).unwrap()) {
        app.tick().unwrap();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let repaired = app.catalog.snapshot(&app.id).unwrap();
    assert!(app.recovered);
    assert!(repaired.value().skinned_lod.is_some());
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
