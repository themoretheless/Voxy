//! Renderer-neutral selection against local bounds using cached world transforms.
use crate::{NodeId, SceneGraph, SceneGraphError};
use glam::{Mat4, Vec2, Vec3, Vec4};
#[derive(Clone, Copy, Debug)]
pub struct PickBounds {
    pub min: Vec3,
    pub max: Vec3,
    pub layers: u32,
}
/// Logical-pixel viewport rectangle inside a platform window.
#[derive(Clone, Copy, Debug)]
pub struct PickViewport {
    pub origin: Vec2,
    pub size: Vec2,
}
impl PickViewport {
    /// Converts physical window cursor coordinates into this logical viewport.
    /// Right/bottom bounds are exclusive, preventing overlap with adjacent panels.
    /// # Errors
    /// Rejects invalid scale, rectangle, cursor or camera. Outside clicks return None.
    pub fn ray(
        self,
        view_projection: Mat4,
        physical_cursor: Vec2,
        scale_factor: f32,
    ) -> Result<Option<PickRay>, PickError> {
        if !scale_factor.is_finite()
            || scale_factor <= 0.0
            || !self.origin.is_finite()
            || !self.size.is_finite()
            || self.size.x <= 0.0
            || self.size.y <= 0.0
            || !physical_cursor.is_finite()
        {
            return Err(PickError::InvalidRay);
        }
        let local = physical_cursor / scale_factor - self.origin;
        if !local.is_finite() {
            return Err(PickError::InvalidRay);
        }
        if local.x < 0.0 || local.y < 0.0 || local.x >= self.size.x || local.y >= self.size.y {
            return Ok(None);
        }
        Ok(Some(PickRay::from_viewport(
            view_projection,
            local,
            self.size,
        )?))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PickRay {
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PickError {
    InvalidRay,
    InvalidBounds,
    Scene(SceneGraphError),
}
impl std::fmt::Display for PickError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "picking error: {self:?}")
    }
}
impl std::error::Error for PickError {}
#[derive(Clone, Copy, Debug)]
pub struct PickHit {
    pub node: NodeId,
    pub distance: f32,
    pub position: Vec3,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct PickResult {
    pub hit: Option<PickHit>,
    /// Zero-scale or numerically non-invertible objects cannot be selected.
    pub skipped_singular: usize,
}
impl PickRay {
    /// Unprojects top-left-origin viewport coordinates using a right-handed
    /// view-projection with depth 0..1. The ray spans near to far clip planes;
    /// objects outside that frustum segment cannot be selected by this ray.
    /// Coordinates are relative to the viewport, not the whole window.
    /// # Errors
    /// Rejects invalid sizes/coordinates, singular/nonfinite matrices and invalid
    /// homogeneous points (including infinite far planes).
    pub fn from_viewport(
        view_projection: Mat4,
        cursor: Vec2,
        size: Vec2,
    ) -> Result<Self, PickError> {
        if !view_projection.is_finite()
            || !cursor.is_finite()
            || !size.is_finite()
            || size.x <= 0.0
            || size.y <= 0.0
            || cursor.x < 0.0
            || cursor.y < 0.0
            || cursor.x > size.x
            || cursor.y > size.y
        {
            return Err(PickError::InvalidRay);
        }
        let inverse = view_projection.inverse();
        if !inverse.is_finite() {
            return Err(PickError::InvalidRay);
        }
        let x = cursor.x / size.x * 2.0 - 1.0;
        let y = 1.0 - cursor.y / size.y * 2.0;
        let unproject = |depth| -> Result<Vec3, PickError> {
            let point = inverse * Vec4::new(x, y, depth, 1.0);
            if !point.is_finite() || point.w == 0.0 {
                return Err(PickError::InvalidRay);
            }
            let world = point.truncate() / point.w;
            if !world.is_finite() {
                return Err(PickError::InvalidRay);
            }
            Ok(world)
        };
        let near = unproject(0.0)?;
        let far = unproject(1.0)?;
        let delta = far - near;
        Self::new(near, delta, delta.length())
    }

    /// Direction is normalized so reported t is world-space distance.
    /// # Errors
    /// Rejects nonfinite values, zero/unrepresentable direction and negative range.
    pub fn new(origin: Vec3, direction: Vec3, max_distance: f32) -> Result<Self, PickError> {
        let length = direction.length();
        if !origin.is_finite()
            || !direction.is_finite()
            || !length.is_finite()
            || length <= 0.0
            || !max_distance.is_finite()
            || max_distance < 0.0
        {
            return Err(PickError::InvalidRay);
        }
        Ok(Self {
            origin,
            direction: direction / length,
            max_distance,
        })
    }
}
fn intersect(bounds: PickBounds, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<f32> {
    let mut near = 0.0_f32;
    let mut far = max_distance;
    for axis in 0..3 {
        if direction[axis] == 0.0 {
            if origin[axis] < bounds.min[axis] || origin[axis] > bounds.max[axis] {
                return None;
            }
        } else {
            let a = (bounds.min[axis] - origin[axis]) / direction[axis];
            let b = (bounds.max[axis] - origin[axis]) / direction[axis];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return None;
            }
        }
    }
    Some(near)
}
impl SceneGraph {
    /// Selects nearest active bounds matching the mask, with slot-order tie breaking.
    /// This is bounds picking, not triangle intersection or a spatial acceleration tree.
    /// # Errors
    /// Rejects invalid matching bounds and cached world-transform overflow.
    pub fn pick(&self, ray: PickRay, layers: u32) -> Result<PickResult, PickError> {
        let mut result = PickResult::default();
        for (node, bounds) in self.active_components::<PickBounds>() {
            if bounds.layers & layers == 0 {
                continue;
            }
            if !bounds.min.is_finite()
                || !bounds.max.is_finite()
                || bounds.min.cmpgt(bounds.max).any()
            {
                return Err(PickError::InvalidBounds);
            }
            let world = self.world_matrix(node).map_err(PickError::Scene)?;
            let inverse = world.inverse();
            if !inverse.is_finite() {
                result.skipped_singular += 1;
                continue;
            }
            let origin = inverse.transform_point3(ray.origin);
            let direction = inverse.transform_vector3(ray.direction);
            if !origin.is_finite() || !direction.is_finite() {
                result.skipped_singular += 1;
                continue;
            }
            // Preserve the transformed vector length: local t remains world distance.
            let limit = result.hit.map_or(ray.max_distance, |hit| hit.distance);
            if let Some(distance) = intersect(*bounds, origin, direction, limit)
                && result.hit.is_none_or(|hit| distance < hit.distance)
            {
                let position = ray.origin + ray.direction * distance;
                if !position.is_finite() {
                    return Err(PickError::InvalidRay);
                }
                result.hit = Some(PickHit {
                    node,
                    distance,
                    position,
                });
            }
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Transform;
    fn bounds(layers: u32) -> PickBounds {
        PickBounds {
            min: -Vec3::ONE,
            max: Vec3::ONE,
            layers,
        }
    }
    #[test]
    fn scaled_parent_preserves_world_hit_distance_and_activity_layers() {
        let mut scene = SceneGraph::new(3);
        let parent = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::Z * 10.0,
                    scale: Vec3::splat(2.0),
                    ..Transform::default()
                },
            )
            .unwrap();
        let child = scene.spawn(Some(parent), Transform::default()).unwrap();
        scene.insert_component(child, bounds(1)).unwrap();
        let ray = PickRay::new(Vec3::ZERO, Vec3::Z * 3.0, 20.0).unwrap();
        let hit = scene.pick(ray, 1).unwrap().hit.unwrap();
        assert_eq!(hit.node, child);
        assert!((hit.distance - 8.0).abs() < 1e-5);
        assert!(scene.pick(ray, 2).unwrap().hit.is_none());
        scene.set_active(parent, false).unwrap();
        assert!(scene.pick(ray, 1).unwrap().hit.is_none());
    }
    #[test]
    fn parallel_miss_inside_origin_limits_and_singular_bounds() {
        let mut scene = SceneGraph::new(2);
        let node = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(node, bounds(1)).unwrap();
        assert!(
            scene
                .pick(
                    PickRay::new(Vec3::new(2.0, 0.0, -3.0), Vec3::Z, 10.0).unwrap(),
                    1
                )
                .unwrap()
                .hit
                .is_none()
        );
        let inside = scene
            .pick(PickRay::new(Vec3::ZERO, Vec3::Z, 0.0).unwrap(), 1)
            .unwrap()
            .hit
            .unwrap();
        assert!(inside.distance.abs() < f32::EPSILON);
        assert!(
            scene
                .pick(PickRay::new(-Vec3::Z * 3.0, Vec3::Z, 1.0).unwrap(), 1)
                .unwrap()
                .hit
                .is_none()
        );
        scene
            .set_local(
                node,
                Transform {
                    scale: Vec3::ZERO,
                    ..Transform::default()
                },
            )
            .unwrap();
        assert_eq!(
            scene
                .pick(PickRay::new(Vec3::ZERO, Vec3::Z, 10.0).unwrap(), 1)
                .unwrap()
                .skipped_singular,
            1
        );
        assert!(PickRay::new(Vec3::ZERO, Vec3::ZERO, 1.0).is_err());
    }
    #[test]
    fn nearest_rotated_reflected_bounds_win_over_farther_objects() {
        let mut scene = SceneGraph::new(2);
        let far = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::Z * 20.0,
                    ..Transform::default()
                },
            )
            .unwrap();
        scene.insert_component(far, bounds(1)).unwrap();
        let near = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::Z * 10.0,
                    rotation: glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                    scale: Vec3::new(-2.0, 1.0, 1.0),
                },
            )
            .unwrap();
        scene.insert_component(near, bounds(1)).unwrap();
        let hit = scene
            .pick(PickRay::new(Vec3::ZERO, Vec3::Z, 100.0).unwrap(), 1)
            .unwrap()
            .hit
            .unwrap();
        assert_eq!(hit.node, near);
        assert!((hit.distance - 8.0).abs() < 1e-4);
    }
    #[test]
    fn viewport_unprojection_respects_top_left_origin_and_depth_segment() {
        let ray = PickRay::from_viewport(Mat4::IDENTITY, Vec2::new(0.0, 0.0), Vec2::splat(100.0))
            .unwrap();
        assert!(ray.origin.abs_diff_eq(Vec3::new(-1.0, 1.0, 0.0), 1e-6));
        assert!(ray.direction.abs_diff_eq(Vec3::Z, 1e-6));
        assert!((ray.max_distance - 1.0).abs() < 1e-6);
        assert!(PickRay::from_viewport(Mat4::ZERO, Vec2::ZERO, Vec2::ONE).is_err());
        assert!(PickRay::from_viewport(Mat4::IDENTITY, Vec2::ZERO, Vec2::ZERO).is_err());
        assert!(PickRay::from_viewport(Mat4::IDENTITY, Vec2::splat(2.0), Vec2::ONE).is_err());
    }
    #[test]
    fn dpi_offset_and_adjacent_panel_boundaries_map_consistently() {
        let viewport = PickViewport {
            origin: Vec2::new(100.0, 50.0),
            size: Vec2::new(400.0, 300.0),
        };
        for scale in [1.0, 1.5, 2.0] {
            let cursor = (viewport.origin + viewport.size * 0.5) * scale;
            let ray = viewport
                .ray(Mat4::IDENTITY, cursor, scale)
                .unwrap()
                .unwrap();
            assert!(ray.origin.abs_diff_eq(Vec3::ZERO, 1e-6));
            assert!(
                viewport
                    .ray(Mat4::IDENTITY, (viewport.origin - Vec2::ONE) * scale, scale)
                    .unwrap()
                    .is_none()
            );
            assert!(
                viewport
                    .ray(
                        Mat4::IDENTITY,
                        (viewport.origin + viewport.size) * scale,
                        scale
                    )
                    .unwrap()
                    .is_none()
            );
        }
        assert!(viewport.ray(Mat4::IDENTITY, Vec2::ZERO, 0.0).is_err());
    }
}
