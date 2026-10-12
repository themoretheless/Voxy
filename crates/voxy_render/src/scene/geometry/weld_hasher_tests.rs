use super::*;

/// Welding with an ordered map is the hash-independent reference.
fn reference_smooth_normals(mesh: &SceneMesh) -> Vec<[f32; 3]> {
    let mut lookup = std::collections::BTreeMap::new();
    let mut mapping = Vec::new();
    let mut sums = Vec::<glam::Vec3>::new();
    for vertex in &mesh.vertices {
        let key = vertex
            .position
            .map(|x| if x == 0. { 0 } else { x.to_bits() });
        let id = *lookup.entry(key).or_insert_with(|| {
            sums.push(glam::Vec3::ZERO);
            sums.len() - 1
        });
        mapping.push(id);
    }
    for triangle in mesh.indices.chunks_exact(3) {
        let [a, b, c]: [u32; 3] = triangle.try_into().unwrap();
        let p = |i: u32| glam::Vec3::from_array(mesh.vertices[i as usize].position);
        let normal = (p(b) - p(a)).cross(p(c) - p(a));
        for i in [a, b, c] {
            sums[mapping[i as usize]] += normal;
        }
    }
    for sum in &mut sums {
        *sum = sum.normalize_or_zero();
    }
    mapping.iter().map(|&i| sums[i].to_array()).collect()
}

#[test]
fn welded_normals_match_ordered_reference_on_seam_grid() {
    let size = 48_u32;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    // Each cell owns its four corners, so every interior position is shared by
    // four distinct render vertices and must weld into one fan.
    for y in 0..size {
        for x in 0..size {
            let base = u32::try_from(vertices.len()).unwrap();
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let fx = (x + dx) as f32 * 0.25;
                let fy = (y + dy) as f32 * 0.25;
                let height = (fx * 1.3).sin() * (fy * 0.7).cos();
                vertices.push(SceneVertex {
                    position: [fx, height, fy],
                    uv: [0.0; 2],
                    color: [1.0; 4],
                });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
        }
    }
    let mesh = SceneMesh::new(vertices, indices).unwrap();
    assert_eq!(smooth_normals(&mesh), reference_smooth_normals(&mesh));
}
