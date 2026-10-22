//! Authored closed pinna shells. The front is a concave bowl, not a solid cone.
use super::{AnatomySurface, BreedAnatomy, Ear};
use glam::Vec3;

pub(super) fn append(mesh: &mut AnatomySurface, a: BreedAnatomy) {
    mesh.inner_ear.resize(mesh.positions.len(), false);
    mesh.inner_ear_weight.resize(mesh.positions.len(), 0.0);
    let Ear::Upright { height } = a.ears else {
        return;
    };
    const ROWS: usize = 18;
    const COLS: usize = 20;
    let hw = a.skull_width * 0.5;
    let skull = a.head_length - a.muzzle_length;
    let origin = a.head_origin();
    mesh.inner_ear.resize(mesh.positions.len(), false);
    mesh.inner_ear_weight.resize(mesh.positions.len(), 0.0);
    for side in [-1.0, 1.0] {
        let base = mesh.positions.len();
        let first_index = mesh.indices.len();
        for back in [false, true] {
            for row in 0..=ROWS {
                let t = row as f32 / ROWS as f32;
                let width = hw * 0.40 * (1.0 - t).powf(0.72) + height * 0.006;
                for col in 0..=COLS {
                    let u = col as f32 / COLS as f32 * 2.0 - 1.0;
                    let envelope = (std::f32::consts::PI * t).sin().max(0.);
                    let rim = origin.z + height * (0.22 * (1.0 - t) - 0.08 * t);
                    let bowl = height * 0.20 * (1. - u * u).max(0.).powf(0.75) * envelope;
                    let rolled_edge =
                        height * 0.035 * (-((u.abs() - 0.86) / 0.13).powi(2)).exp() * envelope;
                    let fold_envelope = if t < 0.55 {
                        (std::f32::consts::PI * t / 0.55).sin()
                    } else {
                        0.
                    };
                    let fold = height
                        * 0.070
                        * (-((u - side * (0.24 - t * 0.30)) / 0.18).powi(2)).exp()
                        * fold_envelope;
                    let front = rim - bowl + rolled_edge + fold;
                    // The back follows the bowl with positive cartilage
                    // thickness; the former planar back crossed the deep front.
                    let z = front
                        - if back {
                            height * (0.012 + 0.025 * (1. - t))
                        } else {
                            0.
                        };
                    mesh.positions.push([
                        side * (hw * 0.78 + height * t * 0.16) + width * u,
                        origin.y + skull * 0.22 + height * t,
                        z,
                    ]);
                    mesh.normals.push([0.0; 3]);
                    let skin = if back {
                        0.
                    } else {
                        ((1. - u * u).max(0.) * envelope).powf(0.65)
                    };
                    mesh.inner_ear.push(skin > 0.25);
                    mesh.inner_ear_weight.push(skin);
                }
            }
        }
        let plane = (ROWS + 1) * (COLS + 1);
        let idx = |back: usize, row: usize, col: usize| {
            (base + back * plane + row * (COLS + 1) + col) as u32
        };
        for back in 0..2 {
            for row in 0..ROWS {
                for col in 0..COLS {
                    let [a, b, c, d] = [
                        idx(back, row, col),
                        idx(back, row, col + 1),
                        idx(back, row + 1, col),
                        idx(back, row + 1, col + 1),
                    ];
                    if back == 0 {
                        mesh.indices.extend([a, b, c, b, d, c]);
                    } else {
                        mesh.indices.extend([a, c, b, b, c, d]);
                    }
                }
            }
        }
        for row in 0..ROWS {
            for col in [0, COLS] {
                let [a, b, c, d] = [
                    idx(0, row, col),
                    idx(0, row + 1, col),
                    idx(1, row, col),
                    idx(1, row + 1, col),
                ];
                if col == 0 {
                    mesh.indices.extend([a, b, c, b, d, c]);
                } else {
                    mesh.indices.extend([a, c, b, b, c, d]);
                }
            }
        }
        for col in 0..COLS {
            for row in [0, ROWS] {
                let [a, b, c, d] = [
                    idx(0, row, col),
                    idx(1, row, col),
                    idx(0, row, col + 1),
                    idx(1, row, col + 1),
                ];
                if row == 0 {
                    mesh.indices.extend([a, b, c, b, d, c]);
                } else {
                    mesh.indices.extend([a, c, b, b, c, d]);
                }
            }
        }
        // Keep the body's analytic normals; accumulate only this shell's faces.
        for tri in mesh.indices[first_index..].chunks_exact(3) {
            let [i, j, k] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let normal = (Vec3::from(mesh.positions[j]) - Vec3::from(mesh.positions[i]))
                .cross(Vec3::from(mesh.positions[k]) - Vec3::from(mesh.positions[i]));
            for v in [i, j, k] {
                mesh.normals[v] = (Vec3::from(mesh.normals[v]) + normal).to_array();
            }
        }
        for normal in &mut mesh.normals[base..] {
            *normal = Vec3::from(*normal).normalize_or_zero().to_array();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    #[test]
    fn pinnae_are_closed_thin_shells_with_concave_fronts() {
        for breed in super::super::Breed::ALL {
            let a = breed.anatomy();
            let Ear::Upright { height } = a.ears else {
                continue;
            };
            let mut mesh = AnatomySurface::default();
            append(&mut mesh, a);
            let mut edges = HashMap::new();
            for tri in mesh.indices.chunks_exact(3) {
                for k in 0..3 {
                    let (i, j) = (tri[k], tri[(k + 1) % 3]);
                    *edges.entry((i.min(j), i.max(j))).or_insert(0usize) += 1;
                }
                let p = tri.map_positions(&mesh);
                assert!((p[1] - p[0]).cross(p[2] - p[0]).length_squared() > 1e-20);
            }
            assert!(edges.values().all(|&n| n == 2));
            assert!(
                mesh.normals
                    .iter()
                    .all(|&n| (Vec3::from(n).length() - 1.0).abs() < 0.001)
            );
            assert!(mesh.inner_ear.iter().any(|&v| v));
            let plane = 19 * 21;
            for i in 0..plane {
                let front = Vec3::from(mesh.positions[i]);
                let back = Vec3::from(mesh.positions[i + plane]);
                assert!(
                    front.z > back.z,
                    "pinna surfaces cross at {i}: {}",
                    front.z - back.z
                );
            }
            let stride = 21;
            let rim = Vec3::from(mesh.positions[9 * stride]);
            let centre = Vec3::from(mesh.positions[9 * stride + 10]);
            assert!(rim.z - centre.z > height * 0.1);
        }
    }
    trait Positions {
        fn map_positions(&self, mesh: &AnatomySurface) -> [Vec3; 3];
    }
    impl Positions for [u32] {
        fn map_positions(&self, mesh: &AnatomySurface) -> [Vec3; 3] {
            std::array::from_fn(|i| Vec3::from(mesh.positions[self[i] as usize]))
        }
    }
}
