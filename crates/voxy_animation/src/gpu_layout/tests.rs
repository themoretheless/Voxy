use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use super::*;
use crate::{AnimationClip, Joint, JointTrack, Playback, QuatKey, Transform, Vec3Key};

fn rig() -> Skeleton {
    let joints = (0..6u16)
        .map(|i| Joint {
            name: Arc::from(format!("j{i}")),
            parent: if i == 0 { None } else { Some((i - 1) / 2) },
            bind_local: Transform {
                translation: Vec3::new(0.0, 0.4, 0.1 * f32::from(i)),
                ..Transform::IDENTITY
            },
            inverse_bind: Mat4::from_translation(Vec3::new(0.0, -0.4 * f32::from(i), 0.0)),
        })
        .collect();
    Skeleton::new(joints).unwrap()
}

fn clip(rig: &Skeleton, duration: f32) -> AnimationClip {
    let mut tracks = vec![JointTrack::default(); 6];
    for (j, track) in tracks.iter_mut().enumerate() {
        for i in 0..=4 {
            let t = duration * i as f32 / 4.0;
            track.rotations.push(QuatKey {
                time: t,
                value: Quat::from_rotation_z((j as f32 * 0.5 + t).sin() * 0.7),
            });
        }
        if j == 0 {
            track.translations = vec![
                Vec3Key {
                    time: 0.0,
                    value: Vec3::ZERO,
                },
                Vec3Key {
                    time: duration,
                    value: Vec3::new(1.0, 0.0, 0.0),
                },
            ];
        }
    }
    AnimationClip::new("c", duration, Playback::Loop, tracks, rig).unwrap()
}

#[test]
fn reference_palette_matches_clip_skinning_at_sample_times() {
    let rig = rig();
    let clip = clip(&rig, 1.0);
    let table = GpuClipTable::bake(&clip, &rig, 8.0).unwrap();
    assert_eq!(table.sample_count, 9);
    assert_eq!(table.keys.len(), 9 * 6);
    let layout = GpuSkeletonLayout::new(&rig.to_soa());
    for s in 0..9u32 {
        let phase = s as f32 / 8.0;
        let job = GpuBlendJob::new(0, 0, 0, phase, 0.0);
        let palette = cpu_reference_palette(&layout, std::slice::from_ref(&table), &job).unwrap();
        let expected = clip
            .try_sample_phase(&rig, f64::from(phase))
            .unwrap()
            .skin_matrices(&rig)
            .unwrap();
        for (a, b) in palette.iter().zip(&expected) {
            assert!(a.abs_diff_eq(*b, 1e-5), "sample {s}: {a:?} vs {b:?}");
        }
    }
}

#[test]
fn blend_weight_endpoints_and_header_words() {
    let rig = rig();
    let a = GpuClipTable::bake(&clip(&rig, 1.0), &rig, 4.0).unwrap();
    let b = GpuClipTable::bake(&clip(&rig, 0.5), &rig, 4.0).unwrap();
    let layout = GpuSkeletonLayout::new(&rig.to_soa());
    let tables = [a.clone(), b];
    let only_a =
        cpu_reference_palette(&layout, &tables, &GpuBlendJob::new(0, 0, 1, 0.3, 0.0)).unwrap();
    let only_b =
        cpu_reference_palette(&layout, &tables, &GpuBlendJob::new(0, 0, 1, 0.3, 1.0)).unwrap();
    let a_self =
        cpu_reference_palette(&layout, &tables, &GpuBlendJob::new(0, 0, 0, 0.3, 0.5)).unwrap();
    let b_self =
        cpu_reference_palette(&layout, &tables, &GpuBlendJob::new(0, 1, 1, 0.3, 0.5)).unwrap();
    assert_eq!(only_a, a_self);
    assert_eq!(only_b, b_self);
    assert_ne!(only_a, only_b);
    let header = a.header(96);
    assert_eq!(header[0], 96);
    assert_eq!(header[1], a.sample_count);
    assert_eq!(header[2], 6);
    assert_eq!(header[3], 1);
    assert_eq!(f32::from_bits(header[4]), 1.0);
    assert_eq!(a.to_floats().len(), a.keys.len() * GpuJointKey::FLOATS);
    let words = GpuBlendJob::new(3, 1, 0, 0.25, 0.75).to_words();
    assert_eq!(&words[..3], &[3, 1, 0]);
    assert_eq!(f32::from_bits(words[3]), 0.25);
    assert!(cpu_reference_palette(&layout, &tables, &GpuBlendJob::new(0, 2, 0, 0.0, 0.0)).is_err());
    assert!(GpuClipTable::bake(&clip(&rig, 1.0), &rig, 0.0).is_err());
    let q = nlerp(Quat::from_rotation_x(0.2), -Quat::from_rotation_x(0.6), 0.5);
    assert!(q.angle_between(Quat::from_rotation_x(0.4)) < 1e-3);
    assert_eq!(layout.parents[0], GPU_NO_PARENT);
    assert_eq!(layout.parents[1], 0);
}

#[test]
fn dual_quat_skinning_preserves_radius_under_180_deg_twist() {
    let q0 = DualQuat::from_rotation_translation(Quat::IDENTITY, Vec3::new(0.0, 2.0, 0.0));
    let q1 = DualQuat::from_rotation_translation(
        Quat::from_rotation_z(3.0),
        Vec3::new(0.0, 2.0, 0.0),
    );
    let blended = DualQuat::blend_weighted(&[q0, q1], &[0.5, 0.5]);
    let p = Vec3::new(1.0, 0.0, 0.0);
    let out = blended.transform_point3(p);
    let radial_dist = (out - Vec3::new(0.0, 2.0, 0.0)).length();
    assert!((radial_dist - 1.0).abs() < 1e-5);
}
