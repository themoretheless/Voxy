//! Exact mesh picking for the viewport's camera and 0..1 depth segment.
use glam::{Mat4, Vec2, Vec3};
use voxy_render::SceneMesh;
use voxy_scene::{NodeId, PickError, SceneGraph};

#[cfg(test)]
pub(crate) fn pick_mesh(
    scene: &SceneGraph,
    nodes: &[NodeId],
    mesh: &SceneMesh,
    cursor: Vec2,
    size: Vec2,
) -> Result<Option<usize>, PickError> {
    Ok(pick_mesh_depth(scene, nodes, mesh, cursor, size, Mat4::IDENTITY)?.map(|(index, _)| index))
}
pub(crate) fn pick_mesh_depth(
    scene: &SceneGraph,
    nodes: &[NodeId],
    mesh: &SceneMesh,
    cursor: Vec2,
    size: Vec2,
    view_projection: Mat4,
) -> Result<Option<(usize, f32)>, PickError> {
    if !cursor.is_finite() || !size.is_finite() {
        return Err(PickError::InvalidRay);
    }
    if size.x <= 0.0
        || size.y <= 0.0
        || cursor.x < 0.0
        || cursor.y < 0.0
        || cursor.x >= size.x
        || cursor.y >= size.y
    {
        return Ok(None);
    }
    let origin = crate::camera::unproject(view_projection, cursor, size, 0.0)
        .map_err(|_| PickError::InvalidRay)?;
    let end = crate::camera::unproject(view_projection, cursor, size, 1.0)
        .map_err(|_| PickError::InvalidRay)?;
    let mut nearest = 1.0;
    let mut selected = None;
    for (index, node) in nodes.iter().enumerate() {
        if !scene.active_in_hierarchy(*node).map_err(PickError::Scene)? {
            continue;
        }
        let inverse = scene
            .world_matrix(*node)
            .map_err(PickError::Scene)?
            .inverse();
        if !inverse.is_finite() {
            continue;
        }
        let local_origin = inverse.transform_point3(origin);
        let direction = inverse.transform_vector3(end - origin);
        for triangle in mesh.indices().chunks_exact(3) {
            let vertex_a = Vec3::from_array(mesh.vertices()[triangle[0] as usize].position);
            let vertex_b = Vec3::from_array(mesh.vertices()[triangle[1] as usize].position);
            let vertex_c = Vec3::from_array(mesh.vertices()[triangle[2] as usize].position);
            let edge1 = vertex_b - vertex_a;
            let edge2 = vertex_c - vertex_a;
            let cross = direction.cross(edge2);
            let determinant = edge1.dot(cross);
            if determinant == 0.0 || !determinant.is_finite() {
                continue;
            }
            let relative = local_origin - vertex_a;
            let barycentric_u = relative.dot(cross) / determinant;
            let cross_relative = relative.cross(edge1);
            let barycentric_v = direction.dot(cross_relative) / determinant;
            let depth = edge2.dot(cross_relative) / determinant;
            if barycentric_u >= 0.0
                && barycentric_v >= 0.0
                && barycentric_u + barycentric_v <= 1.0
                && depth >= 0.0
                && depth <= nearest
                && depth.is_finite()
                && (selected.is_none() || depth < nearest)
            {
                nearest = depth;
                selected = Some(index);
            }
        }
    }
    Ok(selected.map(|index| (index, nearest)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxy_render::SceneVertex;
    use voxy_scene::Transform;
    #[test]
    fn empty_triangle_bounds_do_not_hide_a_real_hit_behind() {
        let mesh = SceneMesh::new(
            [[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [-0.5, 0.5, 0.0]]
                .map(|position| SceneVertex {
                    position,
                    uv: [0.0; 2],
                    color: [1.0; 4],
                })
                .to_vec(),
            vec![0, 1, 2],
        )
        .unwrap();
        let mut scene = SceneGraph::new(2);
        let front = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(0.0, 0.0, 0.25),
                    ..Transform::default()
                },
            )
            .unwrap();
        let back = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(0.3, 0.3, 0.75),
                    ..Transform::default()
                },
            )
            .unwrap();
        let nodes = [front, back];
        assert_eq!(
            pick_mesh(
                &scene,
                &nodes,
                &mesh,
                Vec2::new(60.0, 40.0),
                Vec2::splat(100.0)
            )
            .unwrap(),
            Some(1)
        );
        assert_eq!(
            pick_mesh(
                &scene,
                &nodes,
                &mesh,
                Vec2::new(40.0, 60.0),
                Vec2::splat(100.0)
            )
            .unwrap(),
            Some(0)
        );
        scene.set_active(back, false).unwrap();
        assert_eq!(
            pick_mesh(
                &scene,
                &nodes,
                &mesh,
                Vec2::new(60.0, 40.0),
                Vec2::splat(100.0)
            )
            .unwrap(),
            None
        );
    }
}
