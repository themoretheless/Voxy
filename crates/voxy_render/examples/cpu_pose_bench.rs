//! CPU-only pose admission benchmark; does not create a GPU or window.
use glam::{Mat4, Quat, Vec3};
use std::{hint::black_box, time::Instant};
use voxy_render::{SkinnedMesh, SkinnedVertex};
fn main() {
    for (name, run) in [
        ("coherent", 64usize),
        ("alternating", 1),
        ("mixed_quarter", 0),
    ] {
        let vertices = (0usize..100_000)
            .map(|i| {
                let joint = (i
                    .checked_div(run)
                    .unwrap_or_else(|| i / 4 * 3 + (i % 4).saturating_sub(1))
                    % 16) as u16;
                SkinnedVertex {
                    position: [(i % 71) as f32 * 0.01, 0.2, 0.3],
                    normal: [0.2, 0.8, 0.1],
                    uv: [0.; 2],
                    joints: [joint, (joint + 1) % 16, 0, 0],
                    weights: [45000, 20535, 0, 0],
                }
            })
            .collect();
        let mesh = SkinnedMesh::new(vertices, vec![0, 1, 2], 16).unwrap();
        let joints: Vec<_> = (0..16)
            .map(|i| {
                Mat4::from_rotation_translation(
                    Quat::from_rotation_y(i as f32 * 0.01),
                    Vec3::Y * 0.1,
                )
            })
            .collect();
        for _ in 0..3 {
            mesh.validate_render_pose(&joints, Mat4::IDENTITY).unwrap();
        }
        let mut samples = Vec::new();
        for _ in 0..30 {
            let start = Instant::now();
            black_box(mesh.validate_render_pose(black_box(&joints), black_box(Mat4::IDENTITY)))
                .unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1e6);
        }
        samples.sort_by(f64::total_cmp);
        println!("{name},{:.3}", samples[15]);
    }
}
