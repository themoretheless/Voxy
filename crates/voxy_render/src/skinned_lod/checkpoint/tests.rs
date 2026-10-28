use super::*;
use glam::Quat;
fn source(subdivided: bool) -> SkinnedLodMesh {
    let source = crate::skinned_lod::test_source();
    if !subdivided {
        return source;
    }
    let positions: Vec<_> = source
        .mesh()
        .vertices()
        .iter()
        .map(|v| v.position)
        .collect();
    let variant = source.indices(1).unwrap().to_vec();
    let proof = crate::generate_subdivided_lod_witnesses(
        LodSurface {
            positions: &positions,
            indices: source.mesh().indices(),
        },
        LodSurface {
            positions: &positions,
            indices: &variant,
        },
        1,
        1000,
    )
    .unwrap();
    SkinnedLodMesh::new_subdivided(
        source.source_mesh().clone(),
        vec![CertifiedLodSubdivisionVariant {
            indices: variant,
            source_to_variant: proof.source_to_approximation,
            variant_to_source: proof.approximation_to_source,
        }],
    )
    .unwrap()
}
fn camera(span: f32) -> SceneCamera {
    SceneCamera {
        eye: Vec3::new(0., 0., 10.),
        target: Vec3::ZERO,
        up: Vec3::Y,
        projection: crate::SceneProjection::Orthographic {
            left: -span / 2.,
            right: span / 2.,
            bottom: -span / 2.,
            top: span / 2.,
            near: 0.1,
            far: 100.,
        },
    }
}
#[test]
fn both_proof_kinds_restore_from_file_and_recertify_120_poses_with_external_mesh_identity() {
    let limits = SkinnedLodCheckpointLimits::default();
    for subdivided in [false, true] {
        let original = source(subdivided);
        let checkpoint = original.checkpoint(limits).unwrap();
        let json = checkpoint.to_json(limits.max_bytes).unwrap();
        let path = std::env::temp_dir().join(format!(
            "voxy-skinned-lod-{}-{}-{subdivided}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, json).unwrap();
        let json = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let record = SkinnedLodCheckpoint::from_json(&json, limits.max_bytes).unwrap();
        let restored = record.prepare(limits).unwrap();
        let bound = record
            .prepare_with_mesh(original.source_mesh().clone(), limits)
            .unwrap();
        assert!(Arc::ptr_eq(bound.source_mesh(), original.source_mesh()));
        assert!(!Arc::ptr_eq(restored.source_mesh(), original.source_mesh()));
        assert_eq!(restored.checkpoint(limits).unwrap(), checkpoint);
        let mut previous_a = None;
        let mut previous_b = None;
        let mut seen = [false; 2];
        let mut errors = Vec::new();
        for frame in 0..120 {
            let t = frame as f32 / 119. * std::f32::consts::TAU;
            let palette = [
                Mat4::IDENTITY,
                Mat4::from_translation(Vec3::new(0.1 * t.cos(), 0., 0.5 * t.sin())),
            ];
            let model = Mat4::from_scale_rotation_translation(
                Vec3::new(1.5, 0.5, 2.),
                Quat::from_rotation_y(t / 3.),
                Vec3::new(2., 0., 1.),
            );
            let a = original.prepare(&palette, model).unwrap();
            let b = restored.prepare(&palette, model).unwrap();
            let c = bound.prepare(&palette, model).unwrap();
            let words = |positions: &[[f32; 3]]| {
                positions
                    .iter()
                    .map(|p| p.map(f32::to_bits))
                    .collect::<Vec<_>>()
            };
            assert_eq!(words(a.positions()), words(b.positions()));
            assert_eq!(words(a.positions()), words(c.positions()));
            let levels = |v: &[LodLevel]| {
                v.iter()
                    .map(|v| (v.object_error.to_bits(), v.index_count))
                    .collect::<Vec<_>>()
            };
            assert_eq!(levels(a.levels()), levels(b.levels()));
            assert_eq!(levels(a.levels()), levels(c.levels()));
            errors.push(a.levels()[1].object_error.to_bits());
            for level in 0..2 {
                assert_eq!(
                    a.posed_scene_mesh(level, [1.; 4]).unwrap().checkpoint(),
                    b.posed_scene_mesh(level, [1.; 4]).unwrap().checkpoint()
                );
            }
            let view = camera(if frame % 2 == 0 { 2. } else { 2000. });
            let policy = LodPolicy {
                target_pixels: 2.,
                hysteresis: 0.1,
            };
            let a = a
                .select_for_camera(view, [800, 600], policy, previous_a)
                .unwrap();
            let b = b
                .select_for_camera(view, [800, 600], policy, previous_b)
                .unwrap();
            assert_eq!(a, b);
            previous_a = Some(a);
            previous_b = Some(b);
            seen[a] = true;
        }
        assert!(errors.windows(2).any(|v| v[0] != v[1]));
        assert!(seen.into_iter().all(|v| v));
    }
}
#[test]
fn invalid_mesh_binding_proofs_and_policies_preserve_original_ready_source() {
    let limits = SkinnedLodCheckpointLimits::default();
    let original = source(true);
    let checkpoint = original.checkpoint(limits).unwrap();
    let mut vertices = original.mesh().vertices().to_vec();
    vertices[0].joints = [0, 1, 0, 0];
    vertices[0].weights = [32768, 32767, 0, 0];
    let other =
        Arc::new(SkinnedMesh::new(vertices, original.mesh().indices().to_vec(), 2).unwrap());
    assert!(checkpoint.prepare_with_mesh(other, limits).is_err());
    let mut bad = checkpoint.clone();
    bad.version = 0;
    assert!(bad.prepare(limits).is_err());
    let mut bad = checkpoint.clone();
    *bad.mesh.last_mut().unwrap() ^= 1;
    assert!(bad.prepare(limits).is_err());
    let mut value = serde_json::to_value(&checkpoint).unwrap();
    value["certificate"]["levels"][1]["error"] = serde_json::json!(0);
    let bad =
        SkinnedLodCheckpoint::from_json(&serde_json::to_string(&value).unwrap(), limits.max_bytes)
            .unwrap();
    assert!(bad.prepare(limits).is_err());
    assert!(
        checkpoint
            .prepare(SkinnedLodCheckpointLimits {
                max_bytes: 1,
                ..limits
            })
            .is_err()
    );
    assert!(
        checkpoint
            .prepare(SkinnedLodCheckpointLimits {
                lod: LodArchiveLimits {
                    cells: 0,
                    ..limits.lod
                },
                ..limits
            })
            .is_err()
    );
    assert!(
        original
            .checkpoint(SkinnedLodCheckpointLimits {
                lod: LodArchiveLimits {
                    cells: 0,
                    ..limits.lod
                },
                ..limits
            })
            .is_err()
    );
    let json = checkpoint.to_json(limits.max_bytes).unwrap();
    assert!(SkinnedLodCheckpoint::from_json(&json, json.len() - 1).is_err());
    assert!(
        SkinnedLodCheckpoint::from_json(
            &json.replacen("\"version\":1", "\"unexpected\":1,\"version\":1", 1),
            limits.max_bytes
        )
        .is_err()
    );
    let ready = original
        .prepare(&[Mat4::IDENTITY; 2], Mat4::IDENTITY)
        .unwrap();
    assert!(original.prepare(&[Mat4::IDENTITY], Mat4::IDENTITY).is_err());
    assert_eq!(
        ready.positions(),
        original
            .prepare(&[Mat4::IDENTITY; 2], Mat4::IDENTITY)
            .unwrap()
            .positions()
    );
    assert_eq!(original.checkpoint(limits).unwrap(), checkpoint);
}
#[test]
fn base_only_sources_restore_without_inventing_a_variant() {
    let limits = SkinnedLodCheckpointLimits::default();
    let mesh = source(false).source_mesh().clone();
    for original in [
        SkinnedLodMesh::new(mesh.clone(), vec![]).unwrap(),
        SkinnedLodMesh::new_subdivided(mesh, vec![]).unwrap(),
    ] {
        let record = original.checkpoint(limits).unwrap();
        let restored = record.prepare(limits).unwrap();
        assert!(restored.indices(1).is_none());
        assert_eq!(restored.checkpoint(limits).unwrap(), record);
    }
}

#[test]
fn source_identity_distinguishes_clones_from_equal_independent_sources_on_one_mesh() {
    let original = source(false);
    let cloned = original.clone();
    let independent = original
        .checkpoint(SkinnedLodCheckpointLimits::default())
        .unwrap()
        .prepare_with_mesh(
            original.source_mesh().clone(),
            SkinnedLodCheckpointLimits::default(),
        )
        .unwrap();
    assert!(original.shares_source_with(&cloned));
    assert!(!original.shares_source_with(&independent));
    assert!(Arc::ptr_eq(
        original.source_mesh(),
        independent.source_mesh()
    ));
    assert_eq!(
        original
            .checkpoint(SkinnedLodCheckpointLimits::default())
            .unwrap(),
        independent
            .checkpoint(SkinnedLodCheckpointLimits::default())
            .unwrap()
    );
}
