//! Selection presentation is derived from immutable geometry, outside authoring data.
use glam::Vec3;
use voxy_render::{SceneError, SceneMesh, SceneVertex};

/// Creates an amber box outline in the model's local coordinates.
/// Thickness scales with the model's largest bound, not with screen pixels.
/// # Errors
/// Rejects nonfinite or unrepresentable bounds and invalid generated geometry.
pub fn selection_outline(mesh: &SceneMesh) -> Result<SceneMesh, SceneError> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for vertex in mesh.vertices() {
        let point = Vec3::from_array(vertex.position);
        min = min.min(point);
        max = max.max(point);
    }
    let half = ((max - min).max_element() * 0.006).max(0.001);
    if !min.is_finite() || !max.is_finite() || !half.is_finite() {
        return Err(SceneError::InvalidGeometry);
    }
    let mut vertices = Vec::with_capacity(96);
    let mut indices = Vec::with_capacity(432);
    for axis in 0..3 {
        let u = (axis + 1) % 3;
        let v = (axis + 2) % 3;
        for side in 0..4 {
            let mut start = min;
            start[u] = if side & 1 == 0 { min[u] } else { max[u] };
            start[v] = if side & 2 == 0 { min[v] } else { max[v] };
            let mut end = start;
            end[axis] = max[axis];
            if start == end {
                continue;
            }
            let low = start - Vec3::splat(half);
            let high = end + Vec3::splat(half);
            let base =
                u32::try_from(vertices.len()).map_err(|_| SceneError::GeometryCapacityExceeded)?;
            for corner in 0..8 {
                vertices.push(SceneVertex {
                    position: [
                        if corner & 1 == 0 { low.x } else { high.x },
                        if corner & 2 == 0 { low.y } else { high.y },
                        if corner & 4 == 0 { low.z } else { high.z },
                    ],
                    uv: [0.0; 2],
                    color: [1.0, 0.6, 0.0, 1.0],
                });
            }
            indices.extend(
                [
                    0, 2, 1, 1, 2, 3, 4, 5, 6, 5, 7, 6, 0, 1, 4, 1, 5, 4, 2, 6, 3, 3, 6, 7, 0, 4,
                    2, 2, 4, 6, 1, 3, 5, 3, 7, 5,
                ]
                .map(|index| base + index),
            );
        }
    }
    SceneMesh::new(vertices, indices)
}
