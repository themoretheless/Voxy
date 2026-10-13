use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use super::*;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((self.0 >> 40) as f32) / ((1u64 << 24) as f32)
    }
}

/// Humanoid-sized chain of 24 joints with 0.3 m links.
fn rig() -> Skeleton {
    let joints = (0..24u16)
        .map(|i| Joint {
            name: Arc::from(format!("j{i}")),
            parent: if i == 0 { None } else { Some(i - 1) },
            bind_local: Transform {
                translation: if i == 0 {
                    Vec3::new(0.0, 1.0, 0.0)
                } else {
                    Vec3::new(0.0, 0.3, 0.0)
                },
                ..Transform::IDENTITY
            },
            inverse_bind: Mat4::IDENTITY,
        })
        .collect();
    Skeleton::new(joints).unwrap()
}

/// 60 Hz baked clip: smooth sinusoidal rotations, a few constant joints,
/// root translation drift.
fn baked_clip(rig: &Skeleton, seed: u64, duration: f32) -> AnimationClip {
    let mut rng = Lcg(seed);
    let samples = (duration * 60.0).round() as usize;
    let mut tracks = vec![JointTrack::default(); rig.joints().len()];
    for (j, track) in tracks.iter_mut().enumerate() {
        let constant = j % 5 == 4;
        let amp = 0.2 + rng.next() * 0.5;
        let freq = 1.0 + rng.next() * 2.0;
        let phase = rng.next() * std::f32::consts::TAU;
        for i in 0..=samples {
            let t = duration * i as f32 / samples as f32;
            let angle = if constant {
                0.3
            } else {
                amp * (freq * t + phase).sin()
            };
            track.rotations.push(QuatKey {
                time: t,
                value: Quat::from_rotation_x(angle) * Quat::from_rotation_z(angle * 0.5),
            });
            if j == 0 {
                track.translations.push(Vec3Key {
                    time: t,
                    value: Vec3::new(0.0, 1.0 + 0.05 * (3.0 * t).sin(), 0.8 * t),
                });
            }
        }
    }
    AnimationClip::new("baked", duration, Playback::Loop, tracks, rig).unwrap()
}

fn model_space_error(rig: &Skeleton, a: &Pose, b: &Pose) -> (f32, f32) {
    let (mut ga, mut gb, mut skin) = (Vec::new(), Vec::new(), Vec::new());
    a.skin_matrices_into(rig, &mut ga, &mut skin).unwrap();
    b.skin_matrices_into(rig, &mut gb, &mut skin).unwrap();
    ga.iter()
        .zip(&gb)
        .fold((0.0f32, 0.0f32), |(te, re), (x, y)| {
            let (_, rx, tx) = x.to_scale_rotation_translation();
            let (_, ry, ty) = y.to_scale_rotation_translation();
            (te.max((tx - ty).length()), re.max(quat_angle(rx, ry)))
        })
}

#[test]
fn measured_error_respects_bounds_across_random_clips() {
    let rig = rig();
    let settings = CompressionSettings {
        translation_error: 5e-3,
        rotation_error: 5e-3,
        scale_error: 1e-3,
        sample_rate: 30.0,
    };
    for seed in 1..=6u64 {
        let clip = baked_clip(&rig, seed, 2.0);
        let (compressed, report) = CompressedClip::compress(&clip, &settings).unwrap();
        assert!(
            report.max_translation_error <= settings.translation_error,
            "{report:?}"
        );
        assert!(
            report.max_rotation_error <= settings.rotation_error,
            "{report:?}"
        );
        // Independent probe at non-sample times.
        let mut worst = (0.0f32, 0.0f32);
        for k in 0..97 {
            let phase = f64::from(k) / 97.0;
            let a = clip.try_sample_phase(&rig, phase).unwrap();
            let b = compressed.try_sample_phase(&rig, phase).unwrap();
            let (te, re) = model_space_error(&rig, &a, &b);
            worst = (worst.0.max(te), worst.1.max(re));
        }
        // Between probes the error can exceed the bound slightly through
        // resampling; stay within a small multiple.
        assert!(
            worst.0 <= settings.translation_error * 3.0,
            "seed {seed}: {worst:?}"
        );
        assert!(
            worst.1 <= settings.rotation_error * 3.0,
            "seed {seed}: {worst:?}"
        );
    }
}

#[test]
fn constant_channels_collapse_and_ratio_exceeds_three() {
    let rig = rig();
    let clip = baked_clip(&rig, 9, 3.0);
    // 5 mm over a 7 m lever chain; the default 1 mm forces a 120 Hz resample.
    let settings = CompressionSettings {
        translation_error: 5e-3,
        rotation_error: 5e-3,
        ..CompressionSettings::default()
    };
    let (compressed, report) = CompressedClip::compress(&clip, &settings).unwrap();
    let constants = compressed
        .joints
        .iter()
        .filter(|j| matches!(j.rotation, QuatChannel::Constant(_)))
        .count();
    // Joints 4, 9, 14, 19 hold a constant rotation; all scales are constant.
    assert_eq!(constants, 4);
    assert!(
        compressed
            .joints
            .iter()
            .all(|j| matches!(j.scale, Vec3Channel::Constant(_)))
    );
    assert!(
        compressed
            .joints
            .iter()
            .skip(1)
            .all(|j| matches!(j.translation, Vec3Channel::Constant(_)))
    );
    let ratio = report.input_bytes as f32 / report.output_bytes as f32;
    assert!(ratio >= 3.0, "{report:?} ratio {ratio}");
    assert_eq!(compressed.byte_len(), report.output_bytes);
}

#[test]
fn decompressed_clip_samples_like_the_compressed_one() {
    let rig = rig();
    let clip = baked_clip(&rig, 3, 1.5);
    let (compressed, _) = CompressedClip::compress(&clip, &CompressionSettings::default()).unwrap();
    let restored = compressed.decompress().unwrap();
    assert_eq!(restored.name(), "baked");
    assert!(restored.is_compatible_with(&rig));
    for k in 0..40 {
        let phase = f64::from(k) / 40.0;
        let a = compressed.try_sample_phase(&rig, phase).unwrap();
        let b = restored.try_sample_phase(&rig, phase).unwrap();
        // The clip interpolates rotations its own way between kept keys, so
        // compare within the compression bound rather than bit-for-bit.
        for (x, y) in a.local().iter().zip(b.local()) {
            assert!((x.translation - y.translation).length() < 1e-5);
            assert!(quat_angle(x.rotation, y.rotation) < 5e-3);
            assert!((x.scale - y.scale).length() < 1e-6);
        }
    }
}

#[test]
fn smallest_three_round_trip_and_validation() {
    let mut rng = Lcg(5);
    for _ in 0..500 {
        let q = Quat::from_xyzw(
            rng.next() * 2.0 - 1.0,
            rng.next() * 2.0 - 1.0,
            rng.next() * 2.0 - 1.0,
            rng.next() * 2.0 - 1.0,
        )
        .normalize();
        let back = QuantizedQuat::new(q).get();
        assert!(quat_angle(back, q) < 1e-4, "{q:?} -> {back:?}");
    }
    let rig = rig();
    let clip = baked_clip(&rig, 1, 1.0);
    let bad = CompressionSettings {
        sample_rate: 0.0,
        ..CompressionSettings::default()
    };
    assert!(CompressedClip::compress(&clip, &bad).is_err());
    let (compressed, _) = CompressedClip::compress(&clip, &CompressionSettings::default()).unwrap();
    assert!(compressed.try_sample_phase(&rig, 1.5).is_err());
    let other = Skeleton::new(vec![Joint {
        name: Arc::from("solo"),
        parent: None,
        bind_local: Transform::IDENTITY,
        inverse_bind: Mat4::IDENTITY,
    }])
    .unwrap();
    assert!(compressed.try_sample_phase(&other, 0.5).is_err());
}
