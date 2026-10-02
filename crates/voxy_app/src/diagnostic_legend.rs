//! Shared unlit diagnostic gradient for native and offscreen rendering.
use voxy_render::{SceneMesh, SceneVertex};
fn row_mesh(labels: [&str; 3]) -> Result<SceneMesh, voxy_render::SceneError> {
    let mut vertices = vec![
        SceneVertex {
            position: [-0.5, -0.15, 0.0],
            uv: [-4.0, 0.0],
            color: [1.0; 4],
        },
        SceneVertex {
            position: [0.0, -0.15, 0.0],
            uv: [-4.0, 0.5],
            color: [1.0; 4],
        },
        SceneVertex {
            position: [0.5, -0.15, 0.0],
            uv: [-4.0, 1.0],
            color: [1.0; 4],
        },
        SceneVertex {
            position: [-0.5, 0.15, 0.0],
            uv: [-4.0, 0.0],
            color: [1.0; 4],
        },
        SceneVertex {
            position: [0.0, 0.15, 0.0],
            uv: [-4.0, 0.5],
            color: [1.0; 4],
        },
        SceneVertex {
            position: [0.5, 0.15, 0.0],
            uv: [-4.0, 1.0],
            color: [1.0; 4],
        },
    ];
    let mut indices = vec![0, 1, 3, 1, 4, 3, 1, 2, 4, 2, 5, 4];
    for (label, anchor) in labels.into_iter().zip([-0.5, 0.0, 0.5]) {
        let width = label.len() as f32 * 0.052;
        let start = anchor - width * 0.5;
        for (column, ch) in label.chars().enumerate() {
            let rows = match ch {
                '0' => [7, 5, 5, 5, 7],
                '1' => [2, 6, 2, 2, 7],
                '5' => [7, 4, 7, 1, 7],
                '%' => [5, 1, 2, 4, 5],
                'm' => [0, 7, 7, 5, 5],
                'u' => [0, 5, 5, 5, 7],
                '+' => [0, 2, 7, 2, 0],
                _ => [0; 5],
            };
            for (row, bits) in rows.into_iter().enumerate() {
                for bit in 0..3 {
                    if bits & (1 << (2 - bit)) == 0 {
                        continue;
                    }
                    let x = start + column as f32 * 0.052 + bit as f32 * 0.013;
                    let y = -0.25 - row as f32 * 0.045;
                    let base = vertices.len() as u32;
                    for position in [
                        [x, y, 0.0],
                        [x + 0.013, y, 0.0],
                        [x + 0.013, y - 0.045, 0.0],
                        [x, y - 0.045, 0.0],
                    ] {
                        vertices.push(SceneVertex {
                            position,
                            uv: [-5.0, 0.0],
                            color: [1.0; 4],
                        });
                    }
                    indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
                }
            }
        }
    }
    SceneMesh::new(vertices, indices)
}

pub(crate) fn mesh(rows: &[[&str; 3]]) -> Result<SceneMesh, voxy_render::SceneError> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (row, labels) in rows.iter().enumerate() {
        let mesh = row_mesh(*labels)?;
        let base = vertices.len() as u32;
        vertices.extend(mesh.vertices().iter().map(|v| {
            let mut vertex = *v;
            vertex.position[1] -= row as f32 * 0.85;
            vertex
        }));
        indices.extend(mesh.indices().iter().map(|i| base + i));
    }
    // Upload a valid inactive placeholder when no diagnostic is selected.
    if rows.is_empty() {
        return row_mesh(["", "", ""]);
    }
    SceneMesh::new(vertices, indices)
}
