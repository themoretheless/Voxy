//! Bridge current FEM surface snapshots into the existing scene renderer.
use physics::plasticity::mesh::QuadraticBody;
use voxy_render::{SceneMesh, SceneVertex};
/// Build a scene mesh from current exposed FEM geometry. color receives accepted
/// component ID and analytic current normal; returns linear straight-alpha RGBA.
/// origin/scale are display coordinates only and do not modify physics state.
pub fn fem_surface_scene_mesh(
    body: &QuadraticBody,
    subdivision_depth: u8,
    max_triangles: usize,
    origin_m: [f64; 3],
    display_scale: f64,
    mut color: impl FnMut(usize, [f64; 3]) -> [f32; 4],
) -> Result<SceneMesh, Box<dyn std::error::Error>> {
    if origin_m.iter().any(|x| !x.is_finite()) || !display_scale.is_finite() || display_scale <= 0.
    {
        return Err("invalid FEM display transform".into());
    }
    let triangles = body.surface_triangles(subdivision_depth, max_triangles)?;
    let count = triangles
        .len()
        .checked_mul(3)
        .ok_or("FEM scene vertex count overflow")?;
    if count > u32::MAX as usize {
        return Err("FEM scene index limit".into());
    }
    let mut vertices = Vec::with_capacity(count);
    let mut indices = Vec::with_capacity(count);
    for triangle in triangles {
        for vertex in triangle.vertices {
            indices.push(vertices.len() as u32);
            vertices.push(SceneVertex {
                position: std::array::from_fn(|a| {
                    ((vertex.position_m[a] - origin_m[a]) * display_scale) as f32
                }),
                uv: [vertex.barycentric[1] as f32, vertex.barycentric[2] as f32],
                color: color(triangle.component, vertex.normal),
            });
        }
    }
    Ok(SceneMesh::new(vertices, indices)?)
}
