//! Renderer-neutral selection against local bounds using cached world transforms.
use crate::{NodeId, SceneGraph, SceneGraphError};
use glam::{Mat4, Vec2, Vec3, Vec4};
#[derive(Clone, Copy, Debug)]
pub struct PickBounds {
    pub min: Vec3,
    pub max: Vec3,
    pub layers: u32,
}
impl PickBounds {
    /// Returns the world/local AABB surface area `2 * (dx*dy + dy*dz + dz*dx)`, or `None` for invalid/inverted bounds.
    #[must_use]
    pub fn surface_area(self) -> Option<f32> {
        if !self.min.is_finite() || !self.max.is_finite() || self.min.cmpgt(self.max).any() {
            return None;
        }
        let d = self.max - self.min;
        let area = 2.0 * (d.x * d.y + d.y * d.z + d.z * d.x);
        area.is_finite().then_some(area)
    }

    /// Computes a deterministic 30-bit 3D Morton code (10 bits per axis) for the centroid of `self`
    /// normalized inside `scene_min..=scene_max`, returning `None` if bounds are invalid or degenerate.
    #[must_use]
    pub fn centroid_morton_code(self, scene_min: Vec3, scene_max: Vec3) -> Option<u32> {
        if !self.min.is_finite()
            || !self.max.is_finite()
            || self.min.cmpgt(self.max).any()
            || !scene_min.is_finite()
            || !scene_max.is_finite()
            || scene_min.cmpge(scene_max).any()
        {
            return None;
        }
        let centroid = 0.5 * (self.min + self.max);
        let extent = scene_max - scene_min;
        let norm = ((centroid - scene_min) / extent).clamp(Vec3::ZERO, Vec3::ONE);
        let qx = (norm.x * 1023.0).round() as u32;
        let qy = (norm.y * 1023.0).round() as u32;
        let qz = (norm.z * 1023.0).round() as u32;
        fn expand_bits_10(mut v: u32) -> u32 {
            v &= 0x0000_03ff;
            v = (v | (v << 16)) & 0x0300_00ff;
            v = (v | (v << 8)) & 0x0300_f00f;
            v = (v | (v << 4)) & 0x030c_30c3;
            v = (v | (v << 2)) & 0x0924_9249;
            v
        }
        Some(expand_bits_10(qx) | (expand_bits_10(qy) << 1) | (expand_bits_10(qz) << 2))
    }
}

/// Evaluates the Surface Area Heuristic (SAH) cost for splitting a parent bounding box into two child partitions.
#[must_use]
pub fn sah_split_cost(
    parent_area: f32,
    left_area: f32,
    left_count: u32,
    right_area: f32,
    right_count: u32,
    traversal_cost: f32,
    intersection_cost: f32,
) -> Option<f32> {
    if !parent_area.is_finite()
        || parent_area <= 0.0
        || !left_area.is_finite()
        || left_area < 0.0
        || !right_area.is_finite()
        || right_area < 0.0
        || !traversal_cost.is_finite()
        || traversal_cost < 0.0
        || !intersection_cost.is_finite()
        || intersection_cost <= 0.0
    {
        return None;
    }
    let cost = traversal_cost
        + intersection_cost
            * ((left_area * left_count as f32 + right_area * right_count as f32) / parent_area);
    cost.is_finite().then_some(cost)
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("picking error: {self:?}")]
pub enum PickError {
    InvalidRay,
    InvalidBounds,
    Scene(SceneGraphError),
}
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
fn world_aabb(bounds: PickBounds, world: Mat4) -> Option<PickBounds> {
    let center = 0.5 * (bounds.min + bounds.max);
    let half = 0.5 * (bounds.max - bounds.min);
    let world_center = world.transform_point3(center);
    let abs_x = world.x_axis.truncate().abs();
    let abs_y = world.y_axis.truncate().abs();
    let abs_z = world.z_axis.truncate().abs();
    let world_half = abs_x * half.x + abs_y * half.y + abs_z * half.z;
    let min = world_center - world_half;
    let max = world_center + world_half;
    (min.is_finite() && max.is_finite() && !min.cmpgt(max).any()).then_some(PickBounds {
        min,
        max,
        layers: bounds.layers,
    })
}

#[derive(Clone, Copy)]
struct BvhPrimitive {
    node: NodeId,
    slot_rank: usize,
    local_bounds: PickBounds,
    world_bounds: PickBounds,
    inverse: Mat4,
    morton: u32,
}

enum BvhNode {
    Leaf {
        bounds: PickBounds,
        start: usize,
        end: usize,
    },
    Internal {
        bounds: PickBounds,
        left: usize,
        right: usize,
    },
}

impl BvhNode {
    const fn bounds(&self) -> PickBounds {
        match *self {
            Self::Leaf { bounds, .. } | Self::Internal { bounds, .. } => bounds,
        }
    }
}

fn union_bounds(a: PickBounds, b: PickBounds) -> PickBounds {
    PickBounds {
        min: a.min.min(b.min),
        max: a.max.max(b.max),
        layers: a.layers | b.layers,
    }
}

fn build_bvh_recursive(
    prims: &[BvhPrimitive],
    start: usize,
    end: usize,
    nodes: &mut Vec<BvhNode>,
) -> usize {
    let slice = &prims[start..end];
    let mut bounds = slice[0].world_bounds;
    for prim in &slice[1..] {
        bounds = union_bounds(bounds, prim.world_bounds);
    }
    let count = end - start;
    if count <= 4 {
        let idx = nodes.len();
        nodes.push(BvhNode::Leaf { bounds, start, end });
        return idx;
    }

    let mut suffix_bounds = vec![slice[count - 1].world_bounds; count];
    for i in (0..count - 1).rev() {
        suffix_bounds[i] = union_bounds(slice[i].world_bounds, suffix_bounds[i + 1]);
    }
    let parent_area = bounds.surface_area().unwrap_or(0.0);
    let mut best_split = count / 2;
    if parent_area > 0.0 {
        let mut best_cost = f32::INFINITY;
        let mut prefix = slice[0].world_bounds;
        for split in 1..count {
            if split > 1 {
                prefix = union_bounds(prefix, slice[split - 1].world_bounds);
            }
            if let (Some(left_area), Some(right_area)) =
                (prefix.surface_area(), suffix_bounds[split].surface_area())
                && let Some(cost) = sah_split_cost(
                    parent_area,
                    left_area,
                    split as u32,
                    right_area,
                    (count - split) as u32,
                    1.0,
                    2.0,
                )
                && cost < best_cost
            {
                best_cost = cost;
                best_split = split;
            }
        }
    }

    let mid = start + best_split;
    let idx = nodes.len();
    nodes.push(BvhNode::Leaf { bounds, start, end });
    let left = build_bvh_recursive(prims, start, mid, nodes);
    let right = build_bvh_recursive(prims, mid, end, nodes);
    nodes[idx] = BvhNode::Internal {
        bounds,
        left,
        right,
    };
    idx
}

impl SceneGraph {
    /// Selects nearest active bounds matching the mask, with slot-order tie breaking.
    /// Uses a Morton-ordered SAH BVH when `>= 16` valid candidates match `layers`.
    /// # Errors
    /// Rejects invalid matching bounds and cached world-transform overflow.
    pub fn pick(&self, ray: PickRay, layers: u32) -> Result<PickResult, PickError> {
        let mut result = PickResult::default();
        let mut candidates: Vec<BvhPrimitive> = Vec::new();
        let mut scene_min = Vec3::splat(f32::INFINITY);
        let mut scene_max = Vec3::splat(f32::NEG_INFINITY);

        for (slot_rank, (node, bounds)) in self.active_components::<PickBounds>().enumerate() {
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
            if let Some(wb) = world_aabb(*bounds, world) {
                let centroid = 0.5 * (wb.min + wb.max);
                scene_min = scene_min.min(centroid);
                scene_max = scene_max.max(centroid);
                candidates.push(BvhPrimitive {
                    node,
                    slot_rank,
                    local_bounds: *bounds,
                    world_bounds: wb,
                    inverse,
                    morton: 0,
                });
            } else {
                // Extreme finite world extents: test local-space ray directly.
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
        }

        if candidates.len() < 16 {
            for prim in &candidates {
                let origin = prim.inverse.transform_point3(ray.origin);
                let direction = prim.inverse.transform_vector3(ray.direction);
                let limit = result.hit.map_or(ray.max_distance, |hit| hit.distance);
                if let Some(distance) = intersect(prim.local_bounds, origin, direction, limit)
                    && result.hit.is_none_or(|hit| distance < hit.distance)
                {
                    let position = ray.origin + ray.direction * distance;
                    if !position.is_finite() {
                        return Err(PickError::InvalidRay);
                    }
                    result.hit = Some(PickHit {
                        node: prim.node,
                        distance,
                        position,
                    });
                }
            }
            return Ok(result);
        }

        let padded_max = scene_max.max(scene_min + Vec3::splat(1e-4));
        for prim in &mut candidates {
            prim.morton = prim
                .world_bounds
                .centroid_morton_code(scene_min, padded_max)
                .unwrap_or(0);
        }
        candidates.sort_by_key(|prim| (prim.morton, prim.slot_rank));

        let mut bvh_nodes = Vec::with_capacity(candidates.len() * 2);
        let root = build_bvh_recursive(&candidates, 0, candidates.len(), &mut bvh_nodes);
        let mut best: Option<(f32, usize, PickHit)> = None;
        let mut stack = Vec::with_capacity(32);
        if let Some(root_t) = intersect(
            bvh_nodes[root].bounds(),
            ray.origin,
            ray.direction,
            ray.max_distance,
        ) {
            stack.push((root, root_t));
        }
        while let Some((idx, node_near)) = stack.pop() {
            let limit = best.map_or(ray.max_distance, |(dist, _, _)| dist);
            if node_near > limit {
                continue;
            }
            match bvh_nodes[idx] {
                BvhNode::Leaf { start, end, .. } => {
                    for prim in &candidates[start..end] {
                        let cur_limit = best.map_or(ray.max_distance, |(dist, _, _)| dist);
                        if intersect(prim.world_bounds, ray.origin, ray.direction, cur_limit)
                            .is_none()
                        {
                            continue;
                        }
                        let origin = prim.inverse.transform_point3(ray.origin);
                        let direction = prim.inverse.transform_vector3(ray.direction);
                        if let Some(distance) =
                            intersect(prim.local_bounds, origin, direction, cur_limit)
                        {
                            let wins = match best {
                                None => true,
                                Some((best_d, best_rank, _)) => {
                                    distance < best_d
                                        || (distance == best_d && prim.slot_rank < best_rank)
                                }
                            };
                            if wins {
                                let position = ray.origin + ray.direction * distance;
                                if !position.is_finite() {
                                    return Err(PickError::InvalidRay);
                                }
                                best = Some((
                                    distance,
                                    prim.slot_rank,
                                    PickHit {
                                        node: prim.node,
                                        distance,
                                        position,
                                    },
                                ));
                            }
                        }
                    }
                }
                BvhNode::Internal { left, right, .. } => {
                    let t_left =
                        intersect(bvh_nodes[left].bounds(), ray.origin, ray.direction, limit);
                    let t_right =
                        intersect(bvh_nodes[right].bounds(), ray.origin, ray.direction, limit);
                    match (t_left, t_right) {
                        (Some(tl), Some(tr)) => {
                            // Push farther child first so nearer child is popped first.
                            if tl <= tr {
                                stack.push((right, tr));
                                stack.push((left, tl));
                            } else {
                                stack.push((left, tl));
                                stack.push((right, tr));
                            }
                        }
                        (Some(tl), None) => stack.push((left, tl)),
                        (None, Some(tr)) => stack.push((right, tr)),
                        (None, None) => {}
                    }
                }
            }
        }
        if let Some((_, _, bvh_hit)) = best
            && result
                .hit
                .is_none_or(|fallback| bvh_hit.distance < fallback.distance)
        {
            result.hit = Some(bvh_hit);
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
    #[test]
    fn pick_bounds_surface_area_and_sah_split_cost_validate_inputs() {
        let unit = PickBounds {
            min: Vec3::ZERO,
            max: Vec3::ONE,
            layers: 1,
        };
        assert_eq!(unit.surface_area(), Some(6.0));
        let inverted = PickBounds {
            min: Vec3::ONE,
            max: Vec3::ZERO,
            layers: 1,
        };
        assert_eq!(inverted.surface_area(), None);
        let cost = sah_split_cost(10.0, 4.0, 5, 4.0, 5, 1.0, 2.0).unwrap();
        assert!((cost - 9.0).abs() < 1e-6);
        assert!(sah_split_cost(0.0, 4.0, 5, 4.0, 5, 1.0, 2.0).is_none());
        assert_eq!(
            PickBounds {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
                layers: 1
            }
            .centroid_morton_code(Vec3::ZERO, Vec3::splat(10.0)),
            Some(0)
        );
        assert_eq!(
            PickBounds {
                min: Vec3::splat(10.0),
                max: Vec3::splat(10.0),
                layers: 1
            }
            .centroid_morton_code(Vec3::ZERO, Vec3::splat(10.0)),
            Some(0x3fff_ffff)
        );
        assert!(unit.centroid_morton_code(Vec3::ONE, Vec3::ZERO).is_none());
    }

    #[test]
    fn bvh_pick_matches_nearest_and_slot_order_tie_breaking_over_many_objects() {
        let mut scene = SceneGraph::new(128);
        let mut nodes = Vec::new();
        for i in 0..64 {
            let z = 10.0 + (i / 4) as f32 * 5.0;
            let x = ((i % 4) as f32 - 1.5) * 4.0;
            let node = scene
                .spawn(
                    None,
                    Transform {
                        translation: Vec3::new(x, 0.0, z),
                        ..Transform::default()
                    },
                )
                .unwrap();
            scene.insert_component(node, bounds(1)).unwrap();
            nodes.push(node);
        }
        // Two overlapping identical bounds at z = 4.0: earliest slot must win on tie.
        let tie_first = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(-6.0, 0.0, 4.0),
                    ..Transform::default()
                },
            )
            .unwrap();
        scene.insert_component(tie_first, bounds(1)).unwrap();
        let tie_second = scene
            .spawn(
                None,
                Transform {
                    translation: Vec3::new(-6.0, 0.0, 4.0),
                    ..Transform::default()
                },
            )
            .unwrap();
        scene.insert_component(tie_second, bounds(1)).unwrap();

        let ray = PickRay::new(Vec3::new(-6.0, 0.0, 0.0), Vec3::Z, 200.0).unwrap();
        let hit = scene.pick(ray, 1).unwrap().hit.unwrap();
        assert_eq!(hit.node, tie_first);
        assert!((hit.distance - 3.0).abs() < 1e-5);

        let ray_grid = PickRay::new(Vec3::new(2.0, 0.0, 0.0), Vec3::Z, 200.0).unwrap();
        let hit_grid = scene.pick(ray_grid, 1).unwrap().hit.unwrap();
        assert_eq!(hit_grid.node, nodes[2]);
        assert!((hit_grid.distance - 9.0).abs() < 1e-5);
    }
}
