use super::*;
use crate::morph::tests::{primitive, targets};

fn metadata(mut primitive: ModelPrimitive) -> ModelPrimitive {
    primitive.color = [-0., f32::from_bits(1), 0.75, 1.];
    primitive.base_color_texture = Some(ModelTexture {
        image: 37,
        sampling: TextureSampling {
            wrap_u: TextureWrap::Mirror,
            wrap_v: TextureWrap::Repeat,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
            mipmap_filter: Some(TextureFilter::Linear),
            anisotropy: 16,
        },
        use_mips: true,
    });
    primitive
}
#[test]
fn static_and_skinned_sources_rebuild_exact_metadata_and_120_morph_results() {
    let limits = MorphArchiveLimits::default();
    for skin in [false, true] {
        let source = metadata(primitive(skin));
        let record = source.checkpoint(limits).unwrap();
        let restored = ModelPrimitiveCheckpoint::from_bytes(record.as_bytes(), limits)
            .unwrap()
            .prepare(limits)
            .unwrap();
        assert_eq!(restored.checkpoint(limits).unwrap(), record);
        let mut deltas = targets();
        deltas[1].positions.as_mut().unwrap()[0][2] = -0.;
        deltas[1].positions.as_mut().unwrap()[1][2] = f32::from_bits(1);
        let original = MorphGeometry::new(source, deltas, limits.max_delta_vectors).unwrap();
        let record = original.checkpoint(limits).unwrap();
        let restored = MorphGeometryCheckpoint::from_bytes(record.as_bytes(), limits)
            .unwrap()
            .prepare(limits)
            .unwrap();
        assert_eq!(restored.checkpoint(limits).unwrap(), record);
        for frame in 0..120 {
            let weights = [frame as f32 / 59. - 1., 1. - frame as f32 / 31.];
            let words = |v: Vec<[f64; 3]>| {
                v.into_iter()
                    .map(|v| v.map(f64::to_bits))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                words(original.positions64(&weights).unwrap()),
                words(restored.positions64(&weights).unwrap())
            );
            assert_eq!(
                original
                    .evaluate(&weights)
                    .unwrap()
                    .checkpoint(limits)
                    .unwrap(),
                restored
                    .evaluate(&weights)
                    .unwrap()
                    .checkpoint(limits)
                    .unwrap()
            );
        }
        assert_eq!(original.checkpoint(limits).unwrap(), record);
    }
}
fn seal(bytes: &mut [u8]) {
    let digest = blake3::hash(&bytes[HEADER..]);
    bytes[16..48].copy_from_slice(digest.as_bytes());
}
#[test]
fn malformed_headers_streams_enums_quotas_and_types_reject_without_changing_source() {
    let limits = MorphArchiveLimits::default();
    let source = MorphGeometry::new(metadata(primitive(false)), targets(), 9).unwrap();
    let record = source.checkpoint(limits).unwrap();
    for n in [0, 7, 16, 47, record.as_bytes().len() - 1] {
        assert!(MorphGeometryCheckpoint::from_bytes(&record.as_bytes()[..n], limits).is_err());
    }
    let mut bytes = record.as_bytes().to_vec();
    *bytes.last_mut().unwrap() ^= 1;
    assert!(MorphGeometryCheckpoint::from_bytes(&bytes, limits).is_err());
    assert!(ModelPrimitiveCheckpoint::from_bytes(record.as_bytes(), limits).is_err());
    let tight = MorphArchiveLimits {
        max_delta_vectors: 8,
        ..limits
    };
    assert!(source.checkpoint(tight).is_err());
    assert!(record.prepare(tight).is_err());
    assert!(
        record
            .prepare(MorphArchiveLimits {
                max_bytes: record.as_bytes().len() - 1,
                ..limits
            })
            .is_err()
    );
    let source_size = u64::from_le_bytes(record.as_bytes()[50..58].try_into().unwrap()) as usize;
    // body kind + geometry kind + geometry length + native geometry + color +
    // texture flag/image/sampler/anisotropy/use_mips.
    let texture = 58 + source_size + 16;
    let target_count = texture + 1 + 8 + 5 + 2 + 1;
    for (offset, replacement) in [
        (texture, vec![2]),
        (texture + 1 + 8, vec![255]),
        (target_count, u64::MAX.to_le_bytes().to_vec()),
        (target_count + 8, vec![2]),
        (target_count + 9, u64::MAX.to_le_bytes().to_vec()),
        (target_count + 17, f32::NAN.to_bits().to_le_bytes().to_vec()),
    ] {
        let mut bytes = record.as_bytes().to_vec();
        bytes[offset..offset + replacement.len()].copy_from_slice(&replacement);
        seal(&mut bytes);
        assert!(
            MorphGeometryCheckpoint::from_bytes(&bytes, limits)
                .unwrap()
                .prepare(limits)
                .is_err(),
            "offset {offset}"
        );
    }
    assert_eq!(source.checkpoint(limits).unwrap(), record);
}

#[test]
fn real_file_roundtrip_preserves_cancellation_and_negative_weight_accuracy() {
    let limits = MorphArchiveLimits::default();
    let source = primitive(true);
    let targets = vec![
        MorphTarget {
            positions: Some(vec![[1e20, -1e20, 1.]; 3]),
            normals: None,
        },
        MorphTarget {
            positions: Some(vec![[-1e20, 1e20, -1.]; 3]),
            normals: None,
        },
        MorphTarget {
            positions: Some(vec![[f32::from_bits(1), -0., 0.125]; 3]),
            normals: None,
        },
    ];
    let original = MorphGeometry::new(source, targets, 9).unwrap();
    let record = original.checkpoint(limits).unwrap();
    let path = std::env::temp_dir().join(format!(
        "voxy-morph-{}-{}.archive",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, record.as_bytes()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let restored = MorphGeometryCheckpoint::from_bytes(&bytes, limits)
        .unwrap()
        .prepare(limits)
        .unwrap();
    for weights in [[1., 1., 1.], [-1., -1., -2.], [0., 0., -0.]] {
        assert_eq!(
            original
                .evaluate(&weights)
                .unwrap()
                .checkpoint(limits)
                .unwrap(),
            restored
                .evaluate(&weights)
                .unwrap()
                .checkpoint(limits)
                .unwrap()
        );
        assert_eq!(
            original
                .positions64(&weights)
                .unwrap()
                .into_iter()
                .map(|v| v.map(f64::to_bits))
                .collect::<Vec<_>>(),
            restored
                .positions64(&weights)
                .unwrap()
                .into_iter()
                .map(|v| v.map(f64::to_bits))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn imported_gltf_morph_sources_rebuild_after_original_buffers_are_dropped() {
    let (document, buffer) = crate::tests::morph_import_fixture();
    let json = serde_json::to_vec(&document).unwrap();
    let model =
        crate::MorphModelAsset::parse(&json, &[&buffer], crate::ModelLimits::default()).unwrap();
    let limits = MorphArchiveLimits::default();
    let records: Vec<_> = model
        .morph
        .primitives
        .iter()
        .map(|binding| {
            binding
                .as_ref()
                .map(|b| b.geometry.checkpoint(limits).unwrap())
        })
        .collect();
    let property_limits = voxy_animation::PropertyAssetLimits::default();
    let animation_limits = voxy_animation::AnimationAssetLimits::default();
    let mut names: Vec<_> = model.joint_names().iter().flatten().cloned().collect();
    names.extend(
        model
            .morph
            .primitives
            .iter()
            .flatten()
            .filter_map(|b| b.name.clone()),
    );
    names.extend(voxy_animation::AnimationAssetCheckpoint::source_names(
        std::slice::from_ref(&model.skeleton),
        &model.animations,
    ));
    names.extend(voxy_animation::PropertyAssetCheckpoint::source_names(
        &model.morph.clips,
        std::slice::from_ref(&model.morph.defaults),
    ));
    let animation_record = voxy_animation::AnimationAssetCheckpoint::capture_with_names(
        std::slice::from_ref(&model.skeleton),
        &model.animations,
        &names,
        animation_limits,
    )
    .unwrap();
    let animation_json = animation_record
        .to_json(animation_limits.max_bytes)
        .unwrap();
    let skeletal = voxy_animation::AnimationAssetCheckpoint::from_json(
        &animation_json,
        animation_limits.max_bytes,
    )
    .unwrap()
    .prepare(animation_limits)
    .unwrap();
    let property_record = voxy_animation::PropertyAssetCheckpoint::capture_with_names(
        &model.morph.clips,
        std::slice::from_ref(&model.morph.defaults),
        &names,
        property_limits,
    )
    .unwrap();
    let property_json = property_record.to_json(property_limits.max_bytes).unwrap();
    let properties = voxy_animation::PropertyAssetCheckpoint::from_json(
        &property_json,
        property_limits.max_bytes,
    )
    .unwrap()
    .prepare_with_names(property_limits, &skeletal.names)
    .unwrap();
    assert!(
        properties
            .names
            .iter()
            .zip(&skeletal.names)
            .all(|(a, b)| std::sync::Arc::ptr_eq(a, b))
    );
    drop(document);
    drop(json);
    drop(buffer);
    let restored: Vec<_> = records
        .iter()
        .map(|record| {
            record.as_ref().map(|record| {
                MorphGeometryCheckpoint::from_bytes(record.as_bytes(), limits)
                    .unwrap()
                    .prepare(limits)
                    .unwrap()
            })
        })
        .collect();
    assert!(restored.iter().any(Option::is_some));
    for phase in 0..120 {
        let original_pose = model
            .sample_pose_phase(Some(0), f64::from(phase) / 119.)
            .unwrap();
        let restored_pose = skeletal.clips[0]
            .try_sample_phase(&skeletal.skeletons[0], f64::from(phase) / 119.)
            .unwrap();
        assert_eq!(original_pose.checkpoint(), restored_pose.checkpoint());
        let weights = model.sample_weights(Some(0), phase as f32 / 119.).unwrap();
        for ((source, restored), weights) in
            model.morph.primitives.iter().zip(&restored).zip(&weights)
        {
            if let (Some(source), Some(restored)) = (source, restored) {
                let sampled = properties.clips[0].sample(phase as f32 / 119.).unwrap();
                let restored_weights: Vec<_> = (0..weights.len())
                    .map(|i| {
                        let name = super::super::weight_name(source.node, i);
                        sampled
                            .get_float(&name)
                            .or_else(|| properties.sets[0].get_float(&name))
                            .unwrap()
                    })
                    .collect();
                assert_eq!(
                    weights.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    restored_weights
                        .iter()
                        .map(|v| v.to_bits())
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    source
                        .geometry
                        .evaluate(weights)
                        .unwrap()
                        .checkpoint(limits)
                        .unwrap(),
                    restored
                        .evaluate(&restored_weights)
                        .unwrap()
                        .checkpoint(limits)
                        .unwrap()
                );
                assert_eq!(
                    source
                        .geometry
                        .positions64(weights)
                        .unwrap()
                        .into_iter()
                        .map(|v| v.map(f64::to_bits))
                        .collect::<Vec<_>>(),
                    restored
                        .positions64(&restored_weights)
                        .unwrap()
                        .into_iter()
                        .map(|v| v.map(f64::to_bits))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}
