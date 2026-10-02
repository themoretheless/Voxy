//! World-axis handles for the camera viewport.
use glam::{Mat4, Vec2, Vec3};
use voxy_render::{SceneError, SceneMesh, SceneVertex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DragAxis {
    Plane,
    X,
    Y,
    Z,
}

pub(crate) fn hit_axis(origin: Vec3, cursor: Vec2, size: Vec2) -> Option<DragAxis> {
    if !origin.is_finite()
        || !cursor.is_finite()
        || !size.is_finite()
        || size.min_element() <= 0.0
        || cursor.min_element() < 0.0
        || cursor.x >= size.x
        || cursor.y >= size.y
        || !(0.0..=1.0).contains(&origin.z)
    {
        return None;
    }
    let center = Vec2::new(
        (origin.x + 1.0) * size.x * 0.5,
        (1.0 - origin.y) * size.y * 0.5,
    );
    let relative = cursor - center;
    if relative.abs().max_element() <= 10.0 {
        return Some(DragAxis::Z);
    }
    if relative.x >= 12.0 && relative.x <= size.x * 0.125 && relative.y.abs() <= 8.0 {
        return Some(DragAxis::X);
    }
    if -relative.y >= 12.0 && -relative.y <= size.y * 0.125 && relative.x.abs() <= 8.0 {
        return Some(DragAxis::Y);
    }
    None
}

pub(crate) fn hit_axis_camera(
    vp: Mat4,
    origin: Vec3,
    cursor: Vec2,
    size: Vec2,
) -> Option<DragAxis> {
    if vp == Mat4::IDENTITY {
        return hit_axis(origin, cursor, size);
    }
    let project = |point| {
        let p = vp.project_point3(point);
        Vec2::new(p.x + 1., 1. - p.y) * size * 0.5
    };
    let ndc = vp.project_point3(origin);
    if !ndc.is_finite() || !(0.0..=1.0).contains(&ndc.z) {
        return None;
    }
    let center = project(origin);
    if cursor.distance(center) <= 10. {
        return Some(DragAxis::Z);
    }
    for (axis, direction) in [(DragAxis::X, Vec3::X), (DragAxis::Y, Vec3::Y)] {
        let segment = project(origin + direction * 0.25) - center;
        let length = segment.length_squared();
        if length <= 1. {
            continue;
        }
        let t = (cursor - center).dot(segment) / length;
        if (0.0..=1.0).contains(&t) && cursor.distance(center + t * segment) <= 8. {
            return Some(axis);
        }
    }
    None
}

pub(crate) fn axis_delta(
    vp: Mat4,
    origin: Vec3,
    axis: Vec3,
    cursor_delta: Vec2,
    size: Vec2,
) -> Vec3 {
    let project = |point| {
        let p = vp.project_point3(point);
        Vec2::new(p.x + 1., 1. - p.y) * size * 0.5
    };
    let screen_axis = project(origin + axis * 0.25) - project(origin);
    if screen_axis.length_squared() < 1. {
        return axis * (-cursor_delta.y / size.y * 2.);
    }
    axis * (cursor_delta.dot(screen_axis) / screen_axis.length_squared() * 0.25)
}

/// Creates colored world-axis translation handles: X red, Y green, depth blue.
/// The blue center handle changes depth by dragging vertically.
/// # Errors
/// Reports invalid generated mesh geometry.
pub fn translation_gizmo() -> Result<SceneMesh, SceneError> {
    let mut vertices = Vec::with_capacity(12);
    let mut indices = Vec::with_capacity(18);
    for (min, max, color) in [
        ([0.0, -0.006], [0.25, 0.006], [1.0, 0.1, 0.1, 1.0]),
        ([-0.006, 0.0], [0.006, 0.25], [0.1, 1.0, 0.1, 1.0]),
        ([-0.018, -0.018], [0.018, 0.018], [0.1, 0.3, 1.0, 1.0]),
    ] {
        let base =
            u32::try_from(vertices.len()).map_err(|_| SceneError::GeometryCapacityExceeded)?;
        for position in [
            [min[0], min[1], 0.0],
            [max[0], min[1], 0.0],
            [max[0], max[1], 0.0],
            [min[0], max[1], 0.0],
        ] {
            vertices.push(SceneVertex {
                position,
                uv: [0.0; 2],
                color,
            });
        }
        indices.extend([0, 1, 2, 0, 2, 3].map(|index| base + index));
    }
    SceneMesh::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axis_hits_use_physical_viewport_coordinates() {
        let size = Vec2::new(640.0, 480.0);
        let origin = Vec3::new(0.0, 0.0, 0.5);
        assert_eq!(
            hit_axis(origin, Vec2::new(360.0, 240.0), size),
            Some(DragAxis::X)
        );
        assert_eq!(
            hit_axis(origin, Vec2::new(320.0, 200.0), size),
            Some(DragAxis::Y)
        );
        assert_eq!(
            hit_axis(origin, Vec2::new(320.0, 240.0), size),
            Some(DragAxis::Z)
        );
        assert_eq!(hit_axis(origin, Vec2::new(100.0, 100.0), size), None);
        assert_eq!(hit_axis(origin, Vec2::new(640.0, 240.0), size), None);
    }
}
