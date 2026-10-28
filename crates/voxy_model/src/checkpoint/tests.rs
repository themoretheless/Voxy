use super::*;
fn imported() -> ModelAsset {
    let (document, buffer) = crate::tests::morph_import_fixture();
    ModelAsset::parse_animated(
        &serde_json::to_vec(&document).unwrap(),
        &[&buffer],
        ModelLimits::default(),
    )
    .unwrap()
}
fn words(values: &[Vec<f32>]) -> Vec<Vec<u32>> {
    values
        .iter()
        .map(|v| v.iter().map(|v| v.to_bits()).collect())
        .collect()
}

#[test]
fn standalone_playback_sources_share_model_rigs_clips_and_property_tracks_for_120_phases() {
    let model = Arc::new(imported());
    let independent = Arc::new(imported());
    model.animations[0].root_rigid_curve(0).unwrap();
    let clips = vec![
        model.animations[0].clone(),
        Arc::new(model.animations[0].as_ref().clone()),
        model.animations[0].clone(),
        independent.animations[0].clone(),
    ];
    let properties = vec![
        model.weight_clips()[0].clone(),
        independent.weight_clips()[0].clone(),
    ];
    let models = vec![model, independent];
    let limits = ModelAssetCheckpointLimits::default();
    let record =
        ModelAssetCheckpoint::capture_with_sources(&models, &clips, &properties, limits).unwrap();
    let bytes = record.to_bytes(limits).unwrap();
    if let Some(directory) = std::env::var_os("VOXY_MODEL_SOURCE_EVIDENCE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("playback-sources.voxymdl"))
            .unwrap();
        std::io::Write::write_all(&mut file, &bytes).unwrap();
    }
    let path = std::env::temp_dir().join(format!(
        "voxy-playback-sources-{}.archive",
        std::process::id()
    ));
    std::fs::write(&path, &bytes).unwrap();
    let read = std::fs::read(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let decoded = ModelAssetCheckpoint::from_bytes(&read, limits).unwrap();
    assert!(decoded.prepare(limits).is_err());
    let loaded = decoded.prepare_sources(limits).unwrap();
    assert!(Arc::ptr_eq(
        &loaded.clips[0],
        &loaded.models[0].animations[0]
    ));
    assert!(Arc::ptr_eq(&loaded.clips[0], &loaded.clips[2]));
    assert!(!Arc::ptr_eq(&loaded.clips[0], &loaded.clips[1]));
    assert!(!Arc::ptr_eq(&loaded.clips[0], &loaded.clips[3]));
    assert_eq!(
        loaded.properties[0].tracks().as_ptr(),
        loaded.models[0].weight_clips()[0].tracks().as_ptr()
    );
    assert_ne!(
        loaded.properties[0].tracks().as_ptr(),
        loaded.properties[1].tracks().as_ptr()
    );
    assert_eq!(
        ModelAssetCheckpoint::capture_with_sources(
            &loaded.models,
            &loaded.clips,
            &loaded.properties,
            limits
        )
        .unwrap(),
        record
    );
    let source_animation = AnimationAssetCheckpoint::capture(
        &models
            .iter()
            .map(|m| m.skeleton.clone())
            .collect::<Vec<_>>(),
        &clips,
        limits.animation,
    )
    .unwrap();
    let loaded_animation = AnimationAssetCheckpoint::capture(
        &loaded
            .models
            .iter()
            .map(|m| m.skeleton.clone())
            .collect::<Vec<_>>(),
        &loaded.clips,
        limits.animation,
    )
    .unwrap();
    assert_eq!(source_animation, loaded_animation);
    for phase in 0..120 {
        let phase = f64::from(phase) / 119.;
        for (index, (source, restored)) in clips.iter().zip(&loaded.clips).enumerate() {
            let rig = usize::from(index == 3);
            let matrices = |clip: &voxy_animation::AnimationClip,
                            rig: &voxy_animation::Skeleton| {
                clip.try_sample_phase64(rig, phase)
                    .unwrap()
                    .skin_matrices(rig)
                    .unwrap()
                    .iter()
                    .map(|m| m.to_cols_array().map(f64::to_bits))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                matrices(source, &models[rig].skeleton),
                matrices(restored, &loaded.models[rig].skeleton)
            );
        }
        for (source, restored) in properties.iter().zip(&loaded.properties) {
            assert_eq!(
                source.sample_phase(phase).unwrap(),
                restored.sample_phase(phase).unwrap()
            );
        }
    }
    let skins = ModelAssetCheckpoint::source_skin_meshes(&models, limits).unwrap();
    assert!(decoded.prepare_with_skin_meshes(limits, &skins).is_err());
    let joined = decoded
        .prepare_sources_with_skin_meshes(limits, &skins)
        .unwrap();
    assert_eq!(joined.clips.len(), 4);
    let tight = ModelAssetCheckpointLimits {
        animation: AnimationAssetLimits {
            max_references: 3,
            ..limits.animation
        },
        ..limits
    };
    assert!(decoded.prepare_sources(tight).is_err());
    let mut malformed = decoded.clone();
    malformed.manifest.external_clips += 1;
    assert!(malformed.prepare_sources(limits).is_err());
    let only = ModelAssetCheckpoint::capture_with_sources(&[], &clips, &properties, limits)
        .unwrap()
        .prepare_sources(limits)
        .unwrap();
    assert!(only.models.is_empty());
    assert_eq!(only.clips.len(), 4);
    assert_eq!(only.properties.len(), 2);
}
#[test]
fn complete_model_file_preserves_outer_morph_skin_rig_clip_and_name_aliases_for_120_frames() {
    let limits = ModelAssetCheckpointLimits::default();
    let mut model = imported();
    let shared = model.joint_names.iter().flatten().next().unwrap().clone();
    let data = Arc::make_mut(&mut model.morph);
    data.primitives.iter_mut().flatten().next().unwrap().name = Some(shared);
    // Supported clip identity lookup is not equivalent to matching vector positions.
    model.animations.push(model.animations[0].clone());
    let a = Arc::new(model);
    let b = Arc::new(a.as_ref().clone());
    let independent = Arc::new(imported());
    let sources = vec![a.clone(), b, a, independent];
    let record = ModelAssetCheckpoint::capture(&sources, limits).unwrap();
    let bytes = record.to_bytes(limits).unwrap();
    let root = std::env::temp_dir().join(format!(
        "voxy-model-graph-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("model.archive");
    std::fs::write(&path, &bytes).unwrap();
    let read = std::fs::read(&path).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let restored = ModelAssetCheckpoint::from_bytes(&read, limits)
        .unwrap()
        .prepare(limits)
        .unwrap();
    assert_eq!(
        ModelAssetCheckpoint::capture(&restored, limits).unwrap(),
        record
    );
    assert!(Arc::ptr_eq(&restored[0], &restored[2]));
    assert!(!Arc::ptr_eq(&restored[0], &restored[1]));
    assert!(Arc::ptr_eq(&restored[0].morph, &restored[1].morph));
    assert!(!Arc::ptr_eq(&restored[0].morph, &restored[3].morph));
    assert_eq!(
        restored[0].skeleton.joints().as_ptr(),
        restored[1].skeleton.joints().as_ptr()
    );
    assert_ne!(
        restored[0].skeleton.joints().as_ptr(),
        restored[3].skeleton.joints().as_ptr()
    );
    assert!(Arc::ptr_eq(
        &restored[0].animations[0],
        &restored[0].animations[1]
    ));
    assert!(Arc::ptr_eq(
        &restored[0].animations[0],
        &restored[0].morph.sources[0]
    ));
    let binding = restored[0]
        .morph
        .primitives
        .iter()
        .flatten()
        .next()
        .unwrap();
    assert!(Arc::ptr_eq(
        binding.name.as_ref().unwrap(),
        restored[0].joint_names.iter().flatten().next().unwrap()
    ));
    let ModelGeometry::Skinned(main) = &restored[0].primitives[0].geometry else {
        panic!("skin required")
    };
    let ModelGeometry::Skinned(morph) = &binding.geometry.source.geometry else {
        panic!("skin required")
    };
    assert_eq!(main.vertices().as_ptr(), morph.vertices().as_ptr());
    assert_eq!(main.indices().as_ptr(), morph.indices().as_ptr());
    assert_ne!(
        main.vertices().as_ptr(),
        match &restored[3].primitives[0].geometry {
            ModelGeometry::Skinned(m) => m.vertices().as_ptr(),
            ModelGeometry::Static(_) => panic!("skin required"),
        }
    );
    for phase in 0..120 {
        for (source, loaded) in sources.iter().zip(&restored) {
            let phase = f64::from(phase) / 119.;
            for clip in 0..source.animations.len() {
                let a = source.sample_frame_phase(Some(clip), phase).unwrap();
                let b = loaded.sample_frame_phase(Some(clip), phase).unwrap();
                assert_eq!(a.pose.checkpoint(), b.pose.checkpoint());
                assert_eq!(
                    words(&source.frame_weights(&a).unwrap()),
                    words(&loaded.frame_weights(&b).unwrap())
                );
                assert_eq!(
                    source
                        .scene_meshes_frame(&a)
                        .unwrap()
                        .iter()
                        .map(SceneMesh::checkpoint)
                        .collect::<Vec<_>>(),
                    loaded
                        .scene_meshes_frame(&b)
                        .unwrap()
                        .iter()
                        .map(SceneMesh::checkpoint)
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    assert_eq!(sources[0].animator(1).err(), restored[0].animator(1).err());
    assert!(restored[3].animator(0).is_ok());
}
#[test]
fn malformed_graph_metadata_and_late_native_errors_preserve_original_models() {
    let limits = ModelAssetCheckpointLimits::default();
    let source = Arc::new(imported());
    let original = ModelAssetCheckpoint::capture(std::slice::from_ref(&source), limits).unwrap();
    let mut cases = Vec::new();
    let mut r = original.clone();
    r.manifest.version = 9;
    cases.push(r);
    let mut r = original.clone();
    r.manifest.references[0] = usize::MAX;
    cases.push(r);
    let mut r = original.clone();
    r.manifest.models[0].morph = usize::MAX;
    cases.push(r);
    let mut r = original.clone();
    r.manifest.models[0].mesh_joint = usize::MAX;
    cases.push(r);
    let mut r = original.clone();
    r.manifest.models[0].joint_names.pop();
    cases.push(r);
    let mut r = original.clone();
    r.manifest.models[0].primitives[0].skin = Some(usize::MAX);
    cases.push(r);
    let mut r = original.clone();
    r.manifest.morphs[0].sources[0] = usize::MAX;
    cases.push(r);
    let mut r = original.clone();
    r.manifest.morphs[0]
        .primitives
        .iter_mut()
        .flatten()
        .next()
        .unwrap()
        .defaults[0] = f32::NAN.to_bits();
    cases.push(r);
    let mut r = original.clone();
    r.chunks.push(Vec::new());
    cases.push(r);
    let mut r = original.clone();
    let chunk = r.chunks.last_mut().unwrap();
    *chunk.last_mut().unwrap() ^= 1;
    cases.push(r);
    for record in cases {
        assert!(record.prepare(limits).is_err());
    }
    for limits in [
        ModelAssetCheckpointLimits {
            max_bytes: 1,
            ..limits
        },
        ModelAssetCheckpointLimits {
            max_models: 0,
            ..limits
        },
        ModelAssetCheckpointLimits {
            max_names: 0,
            ..limits
        },
        ModelAssetCheckpointLimits {
            max_primitives: 0,
            ..limits
        },
    ] {
        assert!(original.prepare(limits).is_err());
    }
    let mut repeated_chunk = original.clone();
    let duplicate = repeated_chunk.manifest.models[0].primitives[0].clone();
    repeated_chunk.manifest.models[0].primitives.push(duplicate);
    assert!(repeated_chunk.to_bytes(limits).is_err());
    assert!(repeated_chunk.prepare(limits).is_err());
    let mut unreachable = original.clone();
    unreachable.manifest.references.clear();
    assert!(unreachable.to_bytes(limits).is_err());
    let bytes = original.to_bytes(limits).unwrap();
    for n in [0, 7, 47, bytes.len() - 1] {
        assert!(ModelAssetCheckpoint::from_bytes(&bytes[..n], limits).is_err());
    }
    let mut corrupt = bytes.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(ModelAssetCheckpoint::from_bytes(&corrupt, limits).is_err());
    let mut hostile = bytes;
    hostile[48..56].copy_from_slice(&u64::MAX.to_le_bytes());
    let digest = blake3::hash(&hostile[48..]);
    hostile[16..48].copy_from_slice(digest.as_bytes());
    assert!(ModelAssetCheckpoint::from_bytes(&hostile, limits).is_err());
    assert_eq!(
        ModelAssetCheckpoint::capture(&[source], limits).unwrap(),
        original
    );
}
#[test]
fn legacy_rigid_and_empty_resource_graphs_restore_without_morph_publication() {
    let limits = ModelAssetCheckpointLimits::default();
    let bytes = include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb");
    let gltf = gltf::Gltf::from_slice(bytes).unwrap();
    let source = Arc::new(
        ModelAsset::parse(
            bytes,
            &[gltf.blob.as_deref().unwrap()],
            ModelLimits::default(),
        )
        .unwrap(),
    );
    let record = ModelAssetCheckpoint::capture(std::slice::from_ref(&source), limits).unwrap();
    let restored = record.prepare(limits).unwrap();
    assert!(!restored[0].has_morph());
    assert!(restored[0].morph.skeleton.is_none());
    assert_eq!(
        source.mesh_instance_count(),
        restored[0].mesh_instance_count()
    );
    assert_eq!(source.skinned, restored[0].skinned);
    for phase in 0..120 {
        let phase = f64::from(phase) / 119.;
        assert_eq!(
            source
                .sample_pose_phase(Some(0), phase)
                .unwrap()
                .checkpoint(),
            restored[0]
                .sample_pose_phase(Some(0), phase)
                .unwrap()
                .checkpoint()
        );
    }
    let empty = ModelAssetCheckpoint::capture(&[], limits).unwrap();
    assert!(
        ModelAssetCheckpoint::from_bytes(&empty.to_bytes(limits).unwrap(), limits)
            .unwrap()
            .prepare(limits)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn unskinned_static_geometry_retains_mesh_transform_and_material_streams() {
    let limits = ModelAssetCheckpointLimits::default();
    let (mut doc, buffer) = crate::tests::morph_import_fixture();
    doc.as_object_mut().unwrap().remove("skins");
    doc["nodes"][1].as_object_mut().unwrap().remove("skin");
    doc["nodes"][1].as_object_mut().unwrap().remove("weights");
    doc["nodes"][1]["translation"] = serde_json::json!([3, -2, 1]);
    doc["nodes"][1]["scale"] = serde_json::json!([-2, 3, 0.5]);
    doc["meshes"][0].as_object_mut().unwrap().remove("weights");
    let primitive = doc["meshes"][0]["primitives"][0].as_object_mut().unwrap();
    primitive.remove("targets");
    let attrs = primitive["attributes"].as_object_mut().unwrap();
    attrs.remove("JOINTS_0");
    attrs.remove("WEIGHTS_0");
    doc["animations"] = serde_json::json!([]);
    let source = Arc::new(
        ModelAsset::parse(
            &serde_json::to_vec(&doc).unwrap(),
            &[&buffer],
            ModelLimits::default(),
        )
        .unwrap(),
    );
    assert!(!source.skinned);
    assert!(matches!(
        source.primitives[0].geometry,
        ModelGeometry::Static(_)
    ));
    let record = ModelAssetCheckpoint::capture(std::slice::from_ref(&source), limits).unwrap();
    let loaded = record.prepare(limits).unwrap();
    let a = source.skeleton.bind_pose();
    let b = loaded[0].skeleton.bind_pose();
    assert_eq!(
        source
            .mesh_transform(&a)
            .unwrap()
            .to_cols_array()
            .map(f32::to_bits),
        loaded[0]
            .mesh_transform(&b)
            .unwrap()
            .to_cols_array()
            .map(f32::to_bits)
    );
    assert_eq!(
        source
            .scene_meshes(&a)
            .unwrap()
            .iter()
            .map(SceneMesh::checkpoint)
            .collect::<Vec<_>>(),
        loaded[0]
            .scene_meshes(&b)
            .unwrap()
            .iter()
            .map(SceneMesh::checkpoint)
            .collect::<Vec<_>>()
    );
}

#[test]
fn explicit_outer_skin_collection_keeps_model_and_morph_links_and_rejects_alias_changes() {
    let limits = ModelAssetCheckpointLimits::default();
    let a = Arc::new(imported());
    let sources = vec![
        a.clone(),
        Arc::new(a.as_ref().clone()),
        a,
        Arc::new(imported()),
    ];
    let record = ModelAssetCheckpoint::capture(&sources, limits).unwrap();
    let skins = ModelAssetCheckpoint::source_skin_meshes(&sources, limits).unwrap();
    let external = SkinnedMeshSetCheckpoint::capture(&skins, limits.skins)
        .unwrap()
        .prepare(limits.skins)
        .unwrap();
    let loaded = record.prepare_with_skin_meshes(limits, &external).unwrap();
    let ModelGeometry::Skinned(mesh) = &loaded[0].primitives[0].geometry else {
        panic!("skin required")
    };
    assert_eq!(mesh.vertices().as_ptr(), external[0].vertices().as_ptr());
    assert_eq!(mesh.indices().as_ptr(), external[0].indices().as_ptr());
    let morph = loaded[0].morph.primitives[0].as_ref().unwrap();
    let ModelGeometry::Skinned(mesh) = &morph.geometry.source.geometry else {
        panic!("skin required")
    };
    assert_eq!(mesh.vertices().as_ptr(), external[0].vertices().as_ptr());
    assert_eq!(
        ModelAssetCheckpoint::capture(&loaded, limits).unwrap(),
        record
    );
    let mut split = external.clone();
    split[1] = SkinnedMesh::new(
        split[1].vertices().to_vec(),
        split[1].indices().to_vec(),
        split[1].joint_count(),
    )
    .unwrap();
    assert!(record.prepare_with_skin_meshes(limits, &split).is_err());
    let mut merged = external.clone();
    merged[2] = merged[0].clone();
    merged[4] = merged[0].clone();
    assert!(record.prepare_with_skin_meshes(limits, &merged).is_err());
    assert!(record.prepare_with_skin_meshes(limits, &[]).is_err());
    assert_eq!(
        ModelAssetCheckpoint::capture(&sources, limits).unwrap(),
        record
    );
}
