use super::*;
use crate::{ImageAsset, ImageLimits, SceneMesh, SceneVertex, SkinnedMesh, SkinnedVertex};
use glam::{Mat4, Quat, Vec3};
fn limits() -> GeometryArchiveLimits {
    GeometryArchiveLimits {
        max_bytes: 4 * 1024 * 1024,
        max_vertices: 20000,
        max_indices: 60000,
        max_joints: 8,
        max_image_dimension: 64,
        max_pixel_bytes: 16384,
    }
}
fn skin(count: usize) -> SkinnedMesh {
    let vertices = (0..count)
        .map(|i| SkinnedVertex {
            position: [(i % 3) as f32 * 0.25, (i / 3) as f32 * 0.125, -0.0],
            normal: [-0.0, 0., 1.],
            uv: [0.125, (i % 3) as f32 * 0.25],
            joints: [0, 1, 2, u16::MAX],
            weights: [32768, 16384, 16383, 0],
        })
        .collect();
    SkinnedMesh::new(vertices, (0..count as u32).collect(), 3).unwrap()
}
#[test]
fn scene_stream_presence_materials_and_float_words_roundtrip_without_regeneration() {
    let base = SceneMesh::new(
        vec![
            SceneVertex {
                position: [-0.0, 0., 0.],
                uv: [-0.0, 0.125],
                color: [1., 0.25, 0.5, 1.],
            },
            SceneVertex {
                position: [1., 0., 0.],
                uv: [1., 0.],
                color: [1.; 4],
            },
            SceneVertex {
                position: [0., 1., 0.],
                uv: [0., 1.],
                color: [0.5; 4],
            },
        ],
        vec![0, 1, 2],
    )
    .unwrap();
    for source in [
        base.clone(),
        base.clone().with_prepared_upload_streams(),
        base.with_normals(vec![[f32::from_bits(1), -0.0, 2.]; 3])
            .unwrap()
            .with_material_coordinates(vec![[-0.0, 17.5, -2.]; 3])
            .unwrap()
            .with_material_parameters([-0.0, 1.333, 0.04, 0.125])
            .unwrap(),
    ] {
        let record = source.checkpoint();
        let bytes = record.to_bytes(limits()).unwrap();
        let decoded = SceneMeshCheckpoint::from_bytes(&bytes, limits()).unwrap();
        assert_eq!(decoded, record);
        let restored = decoded.prepare(limits()).unwrap();
        assert_eq!(restored.checkpoint(), record);
        assert_eq!(restored.checkpoint().to_bytes(limits()).unwrap(), bytes);
        assert_eq!(
            crate::scene::NormalCache::from_mesh(&source)
                .normals
                .iter()
                .flatten()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            crate::scene::NormalCache::from_mesh(&restored)
                .normals
                .iter()
                .flatten()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>()
        );
    }
}
#[test]
fn skin_integer_weights_exact_streams_and_120_pose_temporal_results_survive_archive() {
    let source = skin(3);
    let record = source.checkpoint();
    let bytes = record.to_bytes(limits()).unwrap();
    let restored = SkinnedMeshCheckpoint::from_bytes(&bytes, limits())
        .unwrap()
        .prepare(limits())
        .unwrap();
    assert_eq!(record, restored.checkpoint());
    assert_eq!(record.vertices[0].weights, [32768, 16384, 16383, 0]);
    for frame in 0..120 {
        let t = frame as f32 * 0.01;
        let palette = [
            Mat4::from_rotation_z(t),
            Mat4::from_translation(Vec3::new(t, 0., 0.)),
            Mat4::from_scale_rotation_translation(
                Vec3::new(1.125, 0.875, 1.),
                Quat::from_rotation_x(t * 0.5),
                Vec3::ZERO,
            ),
        ];
        let model = Mat4::from_translation(Vec3::new(0.25, t, -0.125));
        assert_eq!(
            source.validate_render_pose(&palette, model),
            restored.validate_render_pose(&palette, model)
        );
        let a = source
            .posed_scene_mesh(&palette, model, [0.1, 0.2, 0.3, 1.])
            .unwrap();
        let b = restored
            .posed_scene_mesh(&palette, model, [0.1, 0.2, 0.3, 1.])
            .unwrap();
        assert_eq!(a.checkpoint(), b.checkpoint());
        let a = source
            .previous_position_vertices([&palette, &palette], [model, Mat4::IDENTITY])
            .unwrap();
        let b = restored
            .previous_position_vertices([&palette, &palette], [model, Mat4::IDENTITY])
            .unwrap();
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(a.current.map(f32::to_bits), b.current.map(f32::to_bits));
            assert_eq!(a.previous.map(f32::to_bits), b.previous.map(f32::to_bits));
        }
    }
    let dense = skin(9000);
    let dense_record = dense.checkpoint();
    let dense_restored =
        SkinnedMeshCheckpoint::from_bytes(&dense_record.to_bytes(limits()).unwrap(), limits())
            .unwrap()
            .prepare(limits())
            .unwrap();
    assert_eq!(dense_restored.checkpoint(), dense_record);
    let palette = [Mat4::IDENTITY; 3];
    assert_eq!(
        dense.validate_render_pose(&palette, Mat4::IDENTITY),
        dense_restored.validate_render_pose(&palette, Mat4::IDENTITY)
    );
    let mut invalid = dense_record;
    invalid.vertices[0].joints[3] = u16::MAX;
    assert!(
        invalid.prepare(limits()).is_err(),
        "accepted inactive indices cannot be silently rewritten"
    );
}
#[test]
fn decoded_rgba_bytes_and_all_mip_levels_are_restored_without_image_source() {
    let rgba = (0..4 * 7 * 5).map(|n| (n * 37) as u8).collect();
    let source = ImageAsset::from_rgba(7, 5, rgba, ImageLimits::default()).unwrap();
    let record = source.checkpoint();
    let bytes = record.to_bytes(limits()).unwrap();
    let restored = ImageCheckpoint::from_bytes(&bytes, limits())
        .unwrap()
        .prepare(limits())
        .unwrap();
    assert_eq!(restored.checkpoint(), record);
    assert_eq!(
        source
            .mip_chain()
            .iter()
            .map(ImageAsset::checkpoint)
            .collect::<Vec<_>>(),
        restored
            .mip_chain()
            .iter()
            .map(ImageAsset::checkpoint)
            .collect::<Vec<_>>()
    );
}
#[test]
fn archive_integrity_lengths_types_and_raw_policy_limits_reject_before_native_publication() {
    let scene = SceneMesh::quad([1.; 4]).checkpoint();
    let skin = skin(3).checkpoint();
    let image = ImageAsset::from_rgba(1, 1, vec![1, 2, 3, 4], ImageLimits::default())
        .unwrap()
        .checkpoint();
    let variants = [
        scene.to_bytes(limits()).unwrap(),
        skin.to_bytes(limits()).unwrap(),
        image.to_bytes(limits()).unwrap(),
    ];
    for (kind, bytes) in variants.iter().enumerate() {
        let decode = |bytes: &[u8], limits: GeometryArchiveLimits| match kind {
            0 => SceneMeshCheckpoint::from_bytes(bytes, limits).map(|_| ()),
            1 => SkinnedMeshCheckpoint::from_bytes(bytes, limits).map(|_| ()),
            _ => ImageCheckpoint::from_bytes(bytes, limits).map(|_| ()),
        };
        for cut in [0, 7, 47, bytes.len() - 1] {
            assert!(decode(&bytes[..cut], limits()).is_err());
        }
        for index in [0, 8, 16, 48, bytes.len() - 1] {
            let mut bad = bytes.clone();
            bad[index] ^= 1;
            assert!(decode(&bad, limits()).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode(&trailing, limits()).is_err());
        let mut tight = limits();
        tight.max_bytes = bytes.len() - 1;
        assert!(decode(bytes, tight).is_err());
    }
    assert!(SceneMeshCheckpoint::from_bytes(&variants[1], limits()).is_err());
    assert!(SkinnedMeshCheckpoint::from_bytes(&variants[2], limits()).is_err());
    let mut tight = limits();
    tight.max_vertices = 2;
    assert!(scene.prepare(tight).is_err());
    assert!(skin.prepare(tight).is_err());
    tight = limits();
    tight.max_indices = 2;
    assert!(scene.prepare(tight).is_err());
    assert!(skin.prepare(tight).is_err());
    tight = limits();
    tight.max_joints = 2;
    assert!(skin.prepare(tight).is_err());
    tight = limits();
    tight.max_pixel_bytes = 3;
    assert!(image.prepare(tight).is_err());
    let before = scene.clone();
    let mut bad = scene.clone();
    bad.vertices[0][0] = f32::INFINITY.to_bits();
    assert!(bad.prepare(limits()).is_err());
    bad = scene.clone();
    bad.indices[0] = 999;
    assert!(bad.prepare(limits()).is_err());
    bad = scene.clone();
    bad.material_parameters[0] = f32::NAN.to_bits();
    assert!(bad.prepare(limits()).is_err());
    assert_eq!(scene, before);
    let mut bad = skin.clone();
    bad.vertices[0].weights[0] = 1;
    assert!(bad.prepare(limits()).is_err());
    let mut bad = skin;
    bad.joint_count = 0;
    assert!(bad.prepare(limits()).is_err());
    let mut bad = image;
    bad.width = u32::MAX;
    bad.height = u32::MAX;
    assert!(bad.prepare(limits()).is_err());
}

#[test]
fn rechecksummed_oversized_counts_reserved_flags_and_impossible_image_headers_reject() {
    fn resign(bytes: &mut [u8]) {
        let hash = blake3::hash(&bytes[HEADER..]);
        bytes[16..48].copy_from_slice(hash.as_bytes());
    }
    let scene = SceneMesh::quad([1.; 4])
        .checkpoint()
        .to_bytes(limits())
        .unwrap();
    let mut huge = scene.clone();
    huge[70..78].copy_from_slice(&u64::MAX.to_le_bytes());
    resign(&mut huge);
    assert!(SceneMeshCheckpoint::from_bytes(&huge, limits()).is_err());
    let mut wide = limits();
    wide.max_vertices = usize::MAX;
    assert!(SceneMeshCheckpoint::from_bytes(&huge, wide).is_err());
    let mut flags = scene;
    flags[69] = 4;
    resign(&mut flags);
    assert!(SceneMeshCheckpoint::from_bytes(&flags, limits()).is_err());
    let mut skin = skin(3).checkpoint().to_bytes(limits()).unwrap();
    skin[53..55].copy_from_slice(&0_u16.to_le_bytes());
    resign(&mut skin);
    assert!(SkinnedMeshCheckpoint::from_bytes(&skin, limits()).is_err());
    let image = ImageAsset::from_rgba(1, 1, vec![1; 4], ImageLimits::default())
        .unwrap()
        .checkpoint()
        .to_bytes(limits())
        .unwrap();
    let mut huge = image.clone();
    huge[53..57].copy_from_slice(&u32::MAX.to_le_bytes());
    huge[57..61].copy_from_slice(&u32::MAX.to_le_bytes());
    resign(&mut huge);
    wide.max_image_dimension = u32::MAX;
    wide.max_pixel_bytes = u64::MAX;
    assert!(ImageCheckpoint::from_bytes(&huge, wide).is_err());
    let mut mismatch = image;
    mismatch[53..57].copy_from_slice(&2_u32.to_le_bytes());
    resign(&mut mismatch);
    assert!(ImageCheckpoint::from_bytes(&mismatch, limits()).is_err());
}
