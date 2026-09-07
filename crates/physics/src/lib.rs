//! Geometry-independent kinematic physics. No renderer, voxel world or ECS dependencies.
mod character;
pub mod planar;
pub use character::*;

/// Integer origin plus local floating-point bounds preserve precision in large worlds.
/// Units are chosen by the application, not tied to a voxel size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Origin {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchoredAabb {
    pub anchor: Origin,
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweepResult<O> {
    pub fraction: f64,
    /// Axis-aligned unit normal; zero denotes initial overlap.
    pub normal: [i8; 3],
    pub obstacle: Option<O>,
}

/// Backend contract for continuous AABB collision queries.
/// Return the earliest hit, fraction in [0,1], in the body's local frame.
/// Missing geometry must be represented as an obstacle or an error, never as empty space.
pub trait CollisionWorld {
    type Obstacle;
    type Error;
    /// # Errors
    /// Backend-specific failures, including exhausted query budgets.
    fn sweep_aabb(
        &self,
        body: AnchoredAabb,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<SweepResult<Self::Obstacle>, Self::Error>;
}

/// Sweeps against a static box in the same local frame (the anchor is not used).
/// Bounds and displacement must be finite, and each minimum must be below its maximum.
/// Equal contact times prefer X, then Y, then Z. Initial overlaps return a zero normal.
#[must_use]
pub fn sweep_box(
    aabb: AnchoredAabb,
    displacement: [f64; 3],
    min: [f64; 3],
    max: [f64; 3],
) -> Option<(f64, [i8; 3])> {
    let mut enter = f64::NEG_INFINITY;
    let mut exit = f64::INFINITY;
    let mut normal = [0_i8; 3];
    for (axis, &axis_displacement) in displacement.iter().enumerate() {
        let voxel_min = min[axis];
        let voxel_max = max[axis];
        let velocity = axis_displacement;
        if velocity == 0.0 {
            if aabb.max[axis] <= voxel_min || aabb.min[axis] >= voxel_max {
                return None;
            }
            continue;
        }
        let first = (voxel_min - aabb.max[axis]) / velocity;
        let second = (voxel_max - aabb.min[axis]) / velocity;
        let axis_enter = first.min(second);
        let axis_exit = first.max(second);
        if axis_enter > enter {
            enter = axis_enter;
            normal = [0; 3];
            normal[axis] = if velocity > 0.0 { -1 } else { 1 };
        }
        exit = exit.min(axis_exit);
        if enter > exit {
            return None;
        }
    }
    if exit < 0.0 || enter > 1.0 {
        None
    } else if enter < 0.0 {
        Some((0.0, [0; 3]))
    } else {
        Some((enter, normal))
    }
}
