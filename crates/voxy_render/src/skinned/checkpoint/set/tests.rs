use super::*;
use glam::Vec3;
fn fixture(count: usize) -> SkinnedMesh {
    SkinnedMesh::new(
        (0..count)
            .map(|i| SkinnedVertex {
                position: [(i % 31) as f32 * 0.01, -0., f32::from_bits(1)],
                normal: [0., 1., 0.],
                uv: [0., 0.],
                joints: [0, 1, 0, 0],
                weights: [32768, 32767, 0, 0],
            })
            .collect(),
        vec![0, 1, 2],
        2,
    )
    .unwrap()
}
#[test]
fn real_file_preserves_dense_groups_buffer_aliases_and_120_native_admissions() {
    let limits = SkinnedMeshSetLimits::default();
    let first = fixture(PARALLEL_ADMISSION_MIN_VERTICES + 1);
    assert!(first.groups.is_some());
    let variant = first.with_shared_indices(Arc::from([2, 1, 0])).unwrap();
    let independent = fixture(PARALLEL_ADMISSION_MIN_VERTICES + 1)
        .with_shared_indices(first.indices.clone())
        .unwrap();
    let source = vec![first.clone(), first, variant, independent];
    let record = SkinnedMeshSetCheckpoint::capture(&source, limits).unwrap();
    assert_eq!(record.vertices.len(), 2);
    assert_eq!(record.indices.len(), 2);
    assert_eq!(record.groups.len(), 2);
    let bytes = record.to_bytes(limits).unwrap();
    let path = std::env::temp_dir().join(format!(
        "voxy-skin-set-{}-{}.archive",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, &bytes).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let restored = SkinnedMeshSetCheckpoint::from_bytes(&bytes, limits)
        .unwrap()
        .prepare(limits)
        .unwrap();
    assert_eq!(
        SkinnedMeshSetCheckpoint::capture(&restored, limits).unwrap(),
        record
    );
    assert!(Arc::ptr_eq(&restored[0].vertices, &restored[1].vertices));
    assert!(Arc::ptr_eq(&restored[0].vertices, &restored[2].vertices));
    assert!(!Arc::ptr_eq(&restored[0].vertices, &restored[3].vertices));
    assert!(Arc::ptr_eq(&restored[0].indices, &restored[3].indices));
    assert!(!Arc::ptr_eq(&restored[0].indices, &restored[2].indices));
    assert!(Arc::ptr_eq(
        restored[0].groups.as_ref().unwrap(),
        restored[2].groups.as_ref().unwrap()
    ));
    assert!(!Arc::ptr_eq(
        restored[0].groups.as_ref().unwrap(),
        restored[3].groups.as_ref().unwrap()
    ));
    for phase in 0..120 {
        let t = phase as f32 / 119.;
        let palette = [
            Mat4::from_translation(Vec3::new(t, 0., 0.)),
            Mat4::from_rotation_z(t),
        ];
        for (a, b) in source.iter().zip(&restored) {
            assert_eq!(
                a.validate_render_pose(&palette, Mat4::IDENTITY),
                b.validate_render_pose(&palette, Mat4::IDENTITY)
            );
        }
    }
    let mut invalid = record.clone();
    invalid.groups[0][0] ^= 1;
    assert!(invalid.prepare(limits).is_err());
}
#[test]
fn exact_small_posed_geometry_and_empty_set_round_trip() {
    let limits = SkinnedMeshSetLimits::default();
    let first = fixture(3);
    let source = vec![first.clone(), first];
    let record = SkinnedMeshSetCheckpoint::capture(&source, limits).unwrap();
    let restored = record.prepare(limits).unwrap();
    for phase in 0..120 {
        let palette = [Mat4::IDENTITY, Mat4::from_rotation_x(phase as f32 / 119.)];
        for (a, b) in source.iter().zip(&restored) {
            assert_eq!(
                a.posed_scene_mesh(&palette, Mat4::IDENTITY, [1.; 4])
                    .unwrap()
                    .checkpoint(),
                b.posed_scene_mesh(&palette, Mat4::IDENTITY, [1.; 4])
                    .unwrap()
                    .checkpoint()
            );
        }
    }
    let empty = SkinnedMeshSetCheckpoint::capture(&[], limits).unwrap();
    assert!(
        SkinnedMeshSetCheckpoint::from_bytes(&empty.to_bytes(limits).unwrap(), limits)
            .unwrap()
            .prepare(limits)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn corrupted_tables_derived_claims_and_rechecksummed_counts_reject_atomically() {
    let limits = SkinnedMeshSetLimits::default();
    let source = fixture(3);
    let original =
        SkinnedMeshSetCheckpoint::capture(std::slice::from_ref(&source), limits).unwrap();
    let mut cases = Vec::new();
    let mut r = original.clone();
    r.version = 0;
    cases.push(r);
    let mut r = original.clone();
    r.meshes[0].vertices = usize::MAX;
    cases.push(r);
    let mut r = original.clone();
    r.meshes[0].coherent_weights = !r.meshes[0].coherent_weights;
    cases.push(r);
    let mut r = original.clone();
    r.meshes[0].max_position ^= 1;
    cases.push(r);
    let mut r = original.clone();
    r.indices.push(vec![0, 1, 2]);
    cases.push(r);
    let mut r = original.clone();
    r.vertices[0][0].attributes[0] = f32::NAN.to_bits();
    cases.push(r);
    let mut r = original.clone();
    r.meshes[0].groups = Some(0);
    r.groups.push([0; 32]);
    cases.push(r);
    for r in cases {
        assert!(r.prepare(limits).is_err());
    }
    assert!(
        original
            .prepare(SkinnedMeshSetLimits {
                geometry: GeometryArchiveLimits {
                    max_vertices: 2,
                    ..limits.geometry
                },
                ..limits
            })
            .is_err()
    );
    assert!(
        original
            .to_bytes(SkinnedMeshSetLimits {
                max_meshes: 0,
                ..limits
            })
            .is_err()
    );
    assert!(
        original
            .prepare(SkinnedMeshSetLimits {
                max_native_vertices: 2,
                ..limits
            })
            .is_err()
    );
    assert!(
        original
            .prepare(SkinnedMeshSetLimits {
                max_native_indices: 2,
                ..limits
            })
            .is_err()
    );
    let bytes = original.to_bytes(limits).unwrap();
    for n in [0, 47, 48, bytes.len() - 1] {
        assert!(SkinnedMeshSetCheckpoint::from_bytes(&bytes[..n], limits).is_err());
    }
    let mut corrupt = bytes.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(SkinnedMeshSetCheckpoint::from_bytes(&corrupt, limits).is_err());
    let mut hostile = bytes.clone();
    hostile[53..61].copy_from_slice(&u64::MAX.to_le_bytes());
    let digest = blake3::hash(&hostile[48..]);
    hostile[16..48].copy_from_slice(digest.as_bytes());
    assert!(SkinnedMeshSetCheckpoint::from_bytes(&hostile, limits).is_err());
    assert_eq!(
        SkinnedMeshSetCheckpoint::capture(&[source], limits).unwrap(),
        original
    );
}

#[test]
fn repeated_instances_share_native_work_budget_but_distinct_rigs_are_counted() {
    let source = fixture(3);
    let limits = SkinnedMeshSetLimits {
        max_native_vertices: 3,
        max_native_indices: 3,
        ..SkinnedMeshSetLimits::default()
    };
    let repeated = vec![source.clone(); 128];
    let record = SkinnedMeshSetCheckpoint::capture(&repeated, limits).unwrap();
    let loaded = record.prepare(limits).unwrap();
    assert_eq!(loaded.len(), 128);
    assert!(
        loaded
            .iter()
            .all(|m| Arc::ptr_eq(&m.vertices, &loaded[0].vertices)
                && Arc::ptr_eq(&m.indices, &loaded[0].indices))
    );
    let mut distinct = SkinnedMeshSetCheckpoint::capture(&[source], limits).unwrap();
    let mut entry = distinct.meshes[0].clone();
    entry.joint_count = 3;
    distinct.meshes.push(entry);
    assert!(matches!(
        distinct.prepare(limits),
        Err(GeometryCheckpointError::Capacity)
    ));
    let raised = SkinnedMeshSetLimits {
        max_native_vertices: 6,
        max_native_indices: 6,
        ..limits
    };
    let loaded = distinct.prepare(raised).unwrap();
    assert_eq!(loaded[1].joint_count(), 3);
    assert!(Arc::ptr_eq(&loaded[0].vertices, &loaded[1].vertices));
}
