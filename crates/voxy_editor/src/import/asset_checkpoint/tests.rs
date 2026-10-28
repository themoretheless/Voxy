use super::*;
use voxy_model::ModelGeometry;
use voxy_render::CertifiedLodIndexSet;
fn animated() -> EditorAsset {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "voxy-editor-asset-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("mesh.glb"),
        include_bytes!("../../../../voxy_render/examples/assets/animated-triangle.glb"),
    )
    .unwrap();
    let provider = FileInputs::new(&root).unwrap();
    let mut inputs = ImportInputs::new(4, 4 * 1024 * 1024);
    let mut asset = load(
        &SourcePath::new("mesh.glb").unwrap(),
        &provider,
        &mut inputs,
    )
    .unwrap();
    let model = asset.animated.as_ref().unwrap();
    let ModelGeometry::Skinned(mesh) = &model.primitives[0].geometry else {
        panic!("skin required")
    };
    let positions: Vec<_> = mesh.vertices().iter().map(|v| v.position).collect();
    let surface = voxy_render::LodSurface {
        positions: &positions,
        indices: mesh.indices(),
    };
    let p = voxy_render::generate_subdivided_lod_witnesses(surface, surface, 0, 100).unwrap();
    asset.skinned_lod = Some(Arc::new(
        SkinnedLodMesh::new_subdivided(
            Arc::new(mesh.clone()),
            vec![voxy_render::CertifiedLodSubdivisionVariant {
                indices: mesh.indices().to_vec(),
                source_to_variant: p.source_to_approximation,
                variant_to_source: p.approximation_to_source,
            }],
        )
        .unwrap(),
    ));
    std::fs::remove_dir_all(root).unwrap();
    asset
}
fn static_asset() -> EditorAsset {
    let mesh = SceneMesh::new(
        vec![
            SceneVertex {
                position: [-0., 0., 0.],
                uv: [0.; 2],
                color: [1.; 4],
            },
            SceneVertex {
                position: [1., 0., 0.],
                uv: [0.; 2],
                color: [1.; 4],
            },
            SceneVertex {
                position: [0., 1., 0.],
                uv: [0.; 2],
                color: [1.; 4],
            },
        ],
        vec![0, 1, 2],
    )
    .unwrap();
    let positions: Vec<_> = mesh.vertices().iter().map(|v| v.position).collect();
    let surface = voxy_render::LodSurface {
        positions: &positions,
        indices: mesh.indices(),
    };
    let p = voxy_render::generate_lod_witnesses(surface, surface, 100).unwrap();
    let lod = CertifiedLodIndexSet::new(
        positions,
        mesh.indices().to_vec(),
        vec![voxy_render::CertifiedLodVariant {
            indices: mesh.indices().to_vec(),
            source_to_variant: p.source_to_approximation,
            variant_to_source: p.approximation_to_source,
        }],
    )
    .unwrap();
    let images =
        vec![ImageAsset::from_rgba(1, 1, vec![0, 255, 127, 255], ImageLimits::default()).unwrap()];
    let node = ImportedNode {
        name: "part".into(),
        parent: None,
        local: Transform::default(),
        mesh: Some(mesh.clone()),
        image: Some(0),
        sampling: TextureSampling::default(),
        use_mips: true,
        geometry_key: Some((4, 2)),
    };
    EditorAsset {
        mesh,
        lod: Some(Arc::new(lod)),
        animated: None,
        skinned_lod: None,
        nodes: vec![node],
        images,
    }
}
#[test]
fn entire_file_preserves_asset_model_lod_source_mesh_and_independent_equal_ownership() {
    let limits = EditorAssetCheckpointLimits::default();
    let a = Arc::new(animated());
    let b = Arc::new(a.as_ref().clone());
    let lod = a.skinned_lod.as_ref().unwrap();
    let mut c = a.as_ref().clone();
    c.skinned_lod = Some(Arc::new(lod.as_ref().clone()));
    let mut d = a.as_ref().clone();
    d.skinned_lod = Some(Arc::new(
        lod.checkpoint(limits.skinned)
            .unwrap()
            .prepare_with_mesh(lod.source_mesh().clone(), limits.skinned)
            .unwrap(),
    ));
    let independent = Arc::new(animated());
    let static_a = Arc::new(static_asset());
    let static_b = Arc::new(static_a.as_ref().clone());
    let input = vec![
        a.clone(),
        b,
        a,
        Arc::new(c),
        Arc::new(d),
        independent,
        static_a,
        static_b,
    ];
    let record = EditorAssetCheckpoint::capture(&input, limits).unwrap();
    let bytes = record.to_bytes(limits).unwrap();
    retain_evidence("identity-graph.voxyeda", &bytes);
    let path = std::env::temp_dir().join(format!(
        "voxy-editor-assets-{}-{}.bin",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, bytes).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let loaded = EditorAssetCheckpoint::from_bytes(&bytes, limits)
        .unwrap()
        .prepare(limits)
        .unwrap();
    assert_eq!(
        EditorAssetCheckpoint::capture(&loaded, limits).unwrap(),
        record
    );
    assert!(Arc::ptr_eq(&loaded[0], &loaded[2]));
    assert!(!Arc::ptr_eq(&loaded[0], &loaded[1]));
    assert!(Arc::ptr_eq(
        loaded[0].animated.as_ref().unwrap(),
        loaded[3].animated.as_ref().unwrap()
    ));
    let source = |i: usize| loaded[i].skinned_lod.as_ref().unwrap();
    assert!(Arc::ptr_eq(source(0), source(1)));
    assert!(!Arc::ptr_eq(source(0), source(3)));
    assert!(source(0).shares_source_with(source(3)));
    assert!(!source(0).shares_source_with(source(4)));
    assert!(Arc::ptr_eq(
        source(0).source_mesh(),
        source(4).source_mesh()
    ));
    assert!(!Arc::ptr_eq(
        source(0).source_mesh(),
        source(5).source_mesh()
    ));
    let ModelGeometry::Skinned(mesh) = &loaded[0].animated.as_ref().unwrap().primitives[0].geometry
    else {
        panic!("skin required")
    };
    assert_eq!(
        mesh.vertices().as_ptr(),
        source(0).mesh().vertices().as_ptr()
    );
    assert_eq!(mesh.indices().as_ptr(), source(0).mesh().indices().as_ptr());
    assert!(Arc::ptr_eq(
        loaded[6].lod.as_ref().unwrap(),
        loaded[7].lod.as_ref().unwrap()
    ));
    assert_ne!(
        loaded[6].images[0].rgba().as_ptr(),
        loaded[7].images[0].rgba().as_ptr()
    );
    for phase in 0..120 {
        for (a, b) in input.iter().zip(&loaded).take(6) {
            let a_model = a.animated.as_ref().unwrap();
            let b_model = b.animated.as_ref().unwrap();
            let a_pose = a_model
                .sample_pose_phase(Some(0), f64::from(phase) / 119.)
                .unwrap();
            let b_pose = b_model
                .sample_pose_phase(Some(0), f64::from(phase) / 119.)
                .unwrap();
            assert_eq!(a_pose.checkpoint(), b_pose.checkpoint());
            let a = a
                .skinned_lod
                .as_ref()
                .unwrap()
                .prepare(&a_model.skin_matrices(&a_pose).unwrap(), Mat4::IDENTITY)
                .unwrap();
            let b = b
                .skinned_lod
                .as_ref()
                .unwrap()
                .prepare(&b_model.skin_matrices(&b_pose).unwrap(), Mat4::IDENTITY)
                .unwrap();
            assert_eq!(
                a.levels()
                    .iter()
                    .map(|v| v.object_error.to_bits())
                    .collect::<Vec<_>>(),
                b.levels()
                    .iter()
                    .map(|v| v.object_error.to_bits())
                    .collect::<Vec<_>>()
            );
            for level in 0..2 {
                assert_eq!(
                    a.posed_scene_mesh(level, [1.; 4]).unwrap().checkpoint(),
                    b.posed_scene_mesh(level, [1.; 4]).unwrap().checkpoint()
                );
            }
        }
    }
}
#[test]
fn metadata_integrity_late_native_failures_limits_and_unreachable_owners_reject_atomically() {
    let limits = EditorAssetCheckpointLimits::default();
    let a = Arc::new(animated());
    let record = EditorAssetCheckpoint::capture(std::slice::from_ref(&a), limits).unwrap();
    let mut cases = Vec::new();
    let mut r = record.clone();
    r.manifest.version = 0;
    cases.push(r);
    let mut r = record.clone();
    r.manifest.references[0] = usize::MAX;
    cases.push(r);
    let mut r = record.clone();
    r.manifest.assets[0].nodes = r.manifest.assets[0].mesh;
    cases.push(r);
    let mut r = record.clone();
    r.manifest.sources[0].mesh = usize::MAX;
    cases.push(r);
    let mut r = record.clone();
    r.manifest.model_skins = usize::MAX;
    cases.push(r);
    let mut r = record.clone();
    r.manifest.allocations.push(0);
    cases.push(r);
    let mut r = record.clone();
    r.chunks.push(Vec::new());
    cases.push(r);
    let mut r = record.clone();
    let bytes = &mut r.chunks[record.manifest.assets[0].mesh];
    *bytes.last_mut().unwrap() ^= 1;
    cases.push(r);
    for r in cases {
        assert!(r.prepare(limits).is_err());
    }
    for limits in [
        EditorAssetCheckpointLimits {
            max_bytes: 1,
            ..limits
        },
        EditorAssetCheckpointLimits {
            max_assets: 0,
            ..limits
        },
        EditorAssetCheckpointLimits {
            max_vertices: 0,
            ..limits
        },
    ] {
        assert!(record.prepare(limits).is_err());
    }
    let bytes = record.to_bytes(limits).unwrap();
    for n in [0, 7, 47, bytes.len() - 1] {
        assert!(EditorAssetCheckpoint::from_bytes(&bytes[..n], limits).is_err());
    }
    let mut bad = bytes.clone();
    *bad.last_mut().unwrap() ^= 1;
    assert!(EditorAssetCheckpoint::from_bytes(&bad, limits).is_err());
    let mut bad = bytes;
    bad[48..56].copy_from_slice(&u64::MAX.to_le_bytes());
    let hash = blake3::hash(&bad[48..]);
    bad[16..48].copy_from_slice(hash.as_bytes());
    assert!(EditorAssetCheckpoint::from_bytes(&bad, limits).is_err());
    assert_eq!(
        EditorAssetCheckpoint::capture(&[a], limits).unwrap(),
        record
    );
}
#[test]
fn empty_graph_is_a_valid_self_contained_archive() {
    let limits = EditorAssetCheckpointLimits::default();
    let record = EditorAssetCheckpoint::capture(&[], limits).unwrap();
    assert!(
        EditorAssetCheckpoint::from_bytes(&record.to_bytes(limits).unwrap(), limits)
            .unwrap()
            .prepare(limits)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn external_models_preserve_catalog_aliases_retired_roots_and_shared_skin_buffers() {
    let asset = Arc::new(animated());
    let current = asset.animated.as_ref().unwrap().clone();
    let retired = Arc::new(current.as_ref().clone());
    let independent = animated().animated.unwrap();
    let roots = vec![current.clone(), retired, independent, current];
    let limits = EditorAssetCheckpointLimits::default();
    let record = EditorAssetCheckpoint::capture_with_models(&[asset], &roots, limits).unwrap();
    let bytes = record.to_bytes(limits).unwrap();
    retain_evidence("external-model-roots.voxyeda", &bytes);
    let decoded = EditorAssetCheckpoint::from_bytes(&bytes, limits).unwrap();
    assert!(decoded.prepare(limits).is_err());
    let loaded = decoded.prepare_with_models(limits).unwrap();
    assert!(Arc::ptr_eq(
        &loaded.models[0],
        loaded.assets[0].animated.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(&loaded.models[0], &loaded.models[3]));
    assert!(!Arc::ptr_eq(&loaded.models[0], &loaded.models[1]));
    assert!(!Arc::ptr_eq(&loaded.models[0], &loaded.models[2]));
    let mesh = |i: usize| match &loaded.models[i].primitives[0].geometry {
        ModelGeometry::Skinned(mesh) => mesh,
        ModelGeometry::Static(_) => panic!("skin required"),
    };
    assert_eq!(mesh(0).vertices().as_ptr(), mesh(1).vertices().as_ptr());
    assert_eq!(
        mesh(0).vertices().as_ptr(),
        loaded.assets[0]
            .skinned_lod
            .as_ref()
            .unwrap()
            .mesh()
            .vertices()
            .as_ptr()
    );
    assert_ne!(mesh(0).vertices().as_ptr(), mesh(2).vertices().as_ptr());
    assert_eq!(
        EditorAssetCheckpoint::capture_with_models(&loaded.assets, &loaded.models, limits).unwrap(),
        record
    );
    let tight = EditorAssetCheckpointLimits {
        models: ModelAssetCheckpointLimits {
            max_references: 4,
            ..limits.models
        },
        ..limits
    };
    assert!(decoded.prepare_with_models(tight).is_err());
    let mut malformed = decoded.clone();
    malformed.manifest.external_models += 1;
    assert!(malformed.prepare_with_models(limits).is_err());
    let only = EditorAssetCheckpoint::capture_with_models(&[], &roots, limits).unwrap();
    let only = EditorAssetCheckpoint::from_bytes(&only.to_bytes(limits).unwrap(), limits)
        .unwrap()
        .prepare_with_models(limits)
        .unwrap();
    assert!(only.assets.is_empty());
    assert_eq!(only.models.len(), 4);
}

#[test]
fn partial_model_lod_vertex_and_index_sharing_is_preserved_without_content_deduplication() {
    let limits = EditorAssetCheckpointLimits::default();
    for share_vertices in [false, true] {
        let mut asset = animated();
        let original_lod = asset
            .skinned_lod
            .as_ref()
            .unwrap()
            .checkpoint(limits.skinned)
            .unwrap();
        let mut model = asset.animated.as_ref().unwrap().as_ref().clone();
        let ModelGeometry::Skinned(mesh) = &model.primitives[0].geometry else {
            panic!("skin required")
        };
        let mut record =
            SkinnedMeshSetCheckpoint::capture(&[mesh.clone(), mesh.clone()], limits.skins).unwrap();
        if share_vertices {
            record.indices.push(record.indices[0].clone());
            record.meshes[1].indices = 1;
        } else {
            record.vertices.push(record.vertices[0].clone());
            record.meshes[1].vertices = 1;
        }
        let values = record.prepare(limits.skins).unwrap();
        model.primitives[0].geometry = ModelGeometry::Skinned(values[0].clone());
        asset.animated = Some(Arc::new(model));
        asset.skinned_lod = Some(Arc::new(
            original_lod
                .prepare_with_mesh(Arc::new(values[1].clone()), limits.skinned)
                .unwrap(),
        ));
        let source = Arc::new(asset);
        let checkpoint =
            EditorAssetCheckpoint::capture(std::slice::from_ref(&source), limits).unwrap();
        let loaded = checkpoint.prepare(limits).unwrap();
        let ModelGeometry::Skinned(model_mesh) =
            &loaded[0].animated.as_ref().unwrap().primitives[0].geometry
        else {
            panic!("skin required")
        };
        let lod_mesh = loaded[0].skinned_lod.as_ref().unwrap().mesh();
        assert_eq!(
            model_mesh.vertices().as_ptr() == lod_mesh.vertices().as_ptr(),
            share_vertices
        );
        assert_eq!(
            model_mesh.indices().as_ptr() == lod_mesh.indices().as_ptr(),
            !share_vertices
        );
        assert_eq!(
            EditorAssetCheckpoint::capture(&loaded, limits).unwrap(),
            checkpoint
        );
    }
}
