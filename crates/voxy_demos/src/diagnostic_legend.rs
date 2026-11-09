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

pub fn mesh(rows: &[[&str; 3]]) -> Result<SceneMesh, voxy_render::SceneError> {
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

/// Evaluates a branchless polynomial fit of the perceptually uniform Viridis colormap
/// in linear sRGB `[r, g, b]` for normalized scalar `t` in `[0.0, 1.0]`.
#[must_use]
pub fn viridis_colormap_rgb(t: f32) -> [f32; 3] {
    let x = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
    let r = (0.280268
        + x * (-0.143511 + x * (2.225794 + x * (-5.076385 + x * (5.166253 - x * 1.45906)))))
        .clamp(0.0, 1.0);
    let g = (0.003362 + x * (1.268001 + x * (-0.270901 - x * 0.116085))).clamp(0.0, 1.0);
    let b = (0.33001 + x * (1.394112 + x * (-4.414325 + x * (4.106652 - x * 1.29812))))
        .clamp(0.0, 1.0);
    [r, g, b]
}

/// Evaluates Anton Mikhailov's `Turbo` rainbow colormap polynomial approximation in sRGB `[r, g, b]`
/// for normalized scalar `t` in `[0.0, 1.0]`, providing smooth luminance progression without false bands.
#[must_use]
pub fn turbo_colormap_rgb(t: f32) -> [f32; 3] {
    let x = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
    let r = (0.13572138
        + x * (4.6153926
            + x * (-42.660324
                + x * (132.13109 + x * (-152.94239 + x * 59.28638)))))
        .clamp(0.0, 1.0);
    let g = (0.09140261
        + x * (2.1941884
            + x * (4.8429665
                + x * (-14.185033 + x * (4.277299 - x * 2.8295665)))))
        .clamp(0.0, 1.0);
    let b = (0.1066733
        + x * (12.641946
            + x * (-60.582047
                + x * (110.36277 + x * (-89.90311 + x * 27.34825)))))
        .clamp(0.0, 1.0);
    [r, g, b]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viridis_colormap_spans_dark_purple_to_bright_yellow() {
        let low = viridis_colormap_rgb(0.0);
        let mid = viridis_colormap_rgb(0.5);
        let high = viridis_colormap_rgb(1.0);
        assert!(low[2] > low[1]);
        assert!(mid[1] > low[1] && high[1] > mid[1]);
        assert!(high[0] > 0.85 && high[1] > 0.80);
        let t_low = turbo_colormap_rgb(0.0);
        let t_mid = turbo_colormap_rgb(0.5);
        let t_high = turbo_colormap_rgb(1.0);
        assert!(t_mid[1] > t_low[1] && t_high[0] > t_low[0]);
    }
}
