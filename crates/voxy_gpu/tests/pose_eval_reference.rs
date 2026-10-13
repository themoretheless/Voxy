//! Plan step 12: the wgpu pose evaluator against the CPU reference palette.
//! Needs a real adapter (lavapipe suffices).
use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};
use voxy_animation::{
    AnimationClip, GpuBlendJob, GpuClipTable, GpuSkeletonLayout, Joint, JointTrack, Playback,
    QuatKey, Skeleton, Transform, Vec3Key, cpu_reference_palette,
};
use voxy_gpu::GpuPoseEvalProgram;

fn rig() -> Skeleton {
    let joints = (0..48u16)
        .map(|i| Joint {
            name: Arc::from(format!("j{i}")),
            parent: if i == 0 { None } else { Some((i - 1) / 2) },
            bind_local: Transform {
                translation: Vec3::new(0.1 * f32::from(i % 3), 0.3, 0.0),
                ..Transform::IDENTITY
            },
            inverse_bind: Mat4::from_translation(Vec3::new(0.0, -0.3 * f32::from(i), 0.0)),
        })
        .collect();
    Skeleton::new(joints).unwrap()
}

fn clip(rig: &Skeleton, seed: f32, duration: f32) -> AnimationClip {
    let mut tracks = vec![JointTrack::default(); rig.joints().len()];
    for (j, track) in tracks.iter_mut().enumerate() {
        for i in 0..=8 {
            let t = duration * i as f32 / 8.0;
            let angle = (seed + j as f32 * 0.3 + t * 2.0).sin() * 0.6;
            track.rotations.push(QuatKey {
                time: t,
                value: Quat::from_rotation_x(angle) * Quat::from_rotation_y(angle * 0.4),
            });
            if j == 0 {
                track.translations.push(Vec3Key {
                    time: t,
                    value: Vec3::new(seed * 0.1, 1.0 + 0.1 * (t * 3.0).cos(), t),
                });
            }
        }
    }
    AnimationClip::new("clip", duration, Playback::Loop, tracks, rig).unwrap()
}

#[test]
fn wgpu_pose_eval_matches_the_cpu_reference_for_a_crowd() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("GPU required for the pose eval reference check");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program = pollster::block_on(GpuPoseEvalProgram::new(&device)).unwrap();

    let rig = rig();
    let tables: Vec<GpuClipTable> = [(0.0, 1.0), (1.7, 0.7), (3.1, 1.3)]
        .iter()
        .map(|&(seed, duration)| {
            GpuClipTable::bake(&clip(&rig, seed, duration), &rig, 30.0).unwrap()
        })
        .collect();
    let layout = GpuSkeletonLayout::new(&rig.to_soa());
    let clips = program.upload_clips(&tables).unwrap();

    let instance_count = 64u32;
    let jobs: Vec<GpuBlendJob> = (0..instance_count)
        .map(|i| {
            GpuBlendJob::new(
                i,
                i % 3,
                (i + 1) % 3,
                (i as f32 * 0.137) % 1.0,
                (i as f32 * 0.071) % 1.0,
            )
        })
        .collect();
    let palettes = program
        .evaluate(&queue, &clips, &layout, &jobs, instance_count)
        .unwrap();
    assert_eq!(
        palettes.len(),
        (instance_count as usize) * rig.joints().len()
    );

    let joint_count = rig.joints().len();
    let mut max_difference = 0.0f32;
    for job in &jobs {
        let expected = cpu_reference_palette(&layout, &tables, job).unwrap();
        let start = job.instance as usize * joint_count;
        for (gpu, cpu) in palettes[start..start + joint_count].iter().zip(&expected) {
            let difference = (gpu.to_cols_array().iter().zip(cpu.to_cols_array()))
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            max_difference = max_difference.max(difference);
        }
    }
    assert!(max_difference <= 1e-5, "max difference {max_difference}");
}
