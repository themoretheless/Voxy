//! World-axis handles for the camera viewport.
use glam::{Mat4, Vec2, Vec3};
use voxy_render::{SceneError, SceneMesh, SceneVertex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum GizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum GizmoSnapSpace {
    #[default]
    Displacement,
    WorldGrid,
}

/// Interactive gizmo snapping configuration across translation, rotation, and scale.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GizmoSnapConfig {
    pub enabled: bool,
    pub space: GizmoSnapSpace,
    pub translation_step: f32,
    pub angle_step_rad: f32,
    pub scale_step: f32,
}

impl Default for GizmoSnapConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            space: GizmoSnapSpace::Displacement,
            translation_step: 0.25,
            angle_step_rad: std::f32::consts::FRAC_PI_6 * 0.5, // 15 deg
            scale_step: 0.25,
        }
    }
}

impl GizmoSnapConfig {
    const LINEAR_PRESETS: [f32; 5] = [0.05, 0.1, 0.25, 0.5, 1.0];
    const ANGLE_DEGREES_PRESETS: [f32; 4] = [5.0, 15.0, 45.0, 90.0];

    fn cycle_preset(current: f32, presets: &[f32], up: bool) -> f32 {
        let mut best_idx = 0;
        let mut best_diff = f32::INFINITY;
        for (i, &preset) in presets.iter().enumerate() {
            let diff = (current - preset).abs();
            if diff < best_diff {
                best_diff = diff;
                best_idx = i;
            }
        }
        let next = if up {
            (best_idx + 1).min(presets.len() - 1)
        } else {
            best_idx.saturating_sub(1)
        };
        presets[next]
    }

    pub fn step_down(&mut self, mode: GizmoMode) {
        self.step(mode, false);
    }

    pub fn step_up(&mut self, mode: GizmoMode) {
        self.step(mode, true);
    }

    fn step(&mut self, mode: GizmoMode, up: bool) {
        match mode {
            GizmoMode::Translate => {
                self.translation_step =
                    Self::cycle_preset(self.translation_step, &Self::LINEAR_PRESETS, up);
            }
            GizmoMode::Scale => {
                self.scale_step = Self::cycle_preset(self.scale_step, &Self::LINEAR_PRESETS, up);
            }
            GizmoMode::Rotate => {
                let deg = Self::cycle_preset(
                    self.angle_step_rad.to_degrees(),
                    &Self::ANGLE_DEGREES_PRESETS,
                    up,
                );
                self.angle_step_rad = deg.to_radians();
            }
        }
    }
}

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

fn push_quad(
    vertices: &mut Vec<SceneVertex>,
    indices: &mut Vec<u32>,
    corners: [[f32; 3]; 4],
    color: [f32; 4],
) -> Result<(), SceneError> {
    let base = u32::try_from(vertices.len()).map_err(|_| SceneError::GeometryCapacityExceeded)?;
    for position in corners {
        vertices.push(SceneVertex {
            position,
            uv: [0.0; 2],
            color,
        });
    }
    indices.extend([0, 1, 2, 0, 2, 3].map(|index| base + index));
    Ok(())
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
        push_quad(
            &mut vertices,
            &mut indices,
            [
                [min[0], min[1], 0.0],
                [max[0], min[1], 0.0],
                [max[0], max[1], 0.0],
                [min[0], max[1], 0.0],
            ],
            color,
        )?;
    }
    SceneMesh::new(vertices, indices)
}

/// Creates a planar rotation ring gizmo with X/Y/Z colored quadrants.
/// # Errors
/// Reports invalid generated mesh geometry.
pub fn rotation_gizmo() -> Result<SceneMesh, SceneError> {
    const SEGMENTS: usize = 24;
    let mut vertices = Vec::with_capacity(SEGMENTS * 4);
    let mut indices = Vec::with_capacity(SEGMENTS * 6);
    let inner = 0.215_f32;
    let outer = 0.24_f32;
    for s in 0..SEGMENTS {
        let a0 = (s as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
        let a1 = ((s + 1) as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
        let color = if s % 3 == 0 {
            [1.0, 0.2, 0.2, 1.0]
        } else if s % 3 == 1 {
            [0.2, 1.0, 0.2, 1.0]
        } else {
            [0.2, 0.5, 1.0, 1.0]
        };
        let (s0, c0) = a0.sin_cos();
        let (s1, c1) = a1.sin_cos();
        push_quad(
            &mut vertices,
            &mut indices,
            [
                [c0 * inner, s0 * inner, 0.0],
                [c0 * outer, s0 * outer, 0.0],
                [c1 * outer, s1 * outer, 0.0],
                [c1 * inner, s1 * inner, 0.0],
            ],
            color,
        )?;
    }
    SceneMesh::new(vertices, indices)
}

/// Creates axis-aligned scale handles with end blocks on X (red), Y (green), and center uniform block (blue).
/// # Errors
/// Reports invalid generated mesh geometry.
pub fn scale_gizmo() -> Result<SceneMesh, SceneError> {
    let mut vertices = Vec::with_capacity(20);
    let mut indices = Vec::with_capacity(30);
    for (min, max, color) in [
        ([0.0, -0.005], [0.22, 0.005], [1.0, 0.1, 0.1, 1.0]),
        ([0.22, -0.018], [0.256, 0.018], [1.0, 0.3, 0.3, 1.0]),
        ([-0.005, 0.0], [0.005, 0.22], [0.1, 1.0, 0.1, 1.0]),
        ([-0.018, 0.22], [0.018, 0.256], [0.3, 1.0, 0.3, 1.0]),
        ([-0.02, -0.02], [0.02, 0.02], [0.2, 0.45, 1.0, 1.0]),
    ] {
        push_quad(
            &mut vertices,
            &mut indices,
            [
                [min[0], min[1], 0.0],
                [max[0], min[1], 0.0],
                [max[0], max[1], 0.0],
                [min[0], max[1], 0.0],
            ],
            color,
        )?;
    }
    SceneMesh::new(vertices, indices)
}

/// Builds the twelve edges of a unit trigger box as thin solid strips.
pub(crate) fn trigger_outline() -> Result<SceneMesh, SceneError> {
    let mut vertices = Vec::with_capacity(96);
    let mut indices = Vec::with_capacity(432);
    for axis in 0..3 {
        let cross = [(axis + 1) % 3, (axis + 2) % 3];
        for a in [-1., 1.] {
            for b in [-1., 1.] {
                let base = vertices.len() as u32;
                for corner in 0..8 {
                    let mut position = [0.; 3];
                    position[axis] = if corner & 1 == 0 { -1. } else { 1. };
                    position[cross[0]] = a * if corner & 2 == 0 { 1. } else { 0.984 };
                    position[cross[1]] = b * if corner & 4 == 0 { 1. } else { 0.984 };
                    vertices.push(SceneVertex {
                        position,
                        uv: [0.; 2],
                        color: [0.1, 0.9, 1., 1.],
                    });
                }
                for [a, b, c, d] in [
                    [0, 1, 3, 2],
                    [4, 6, 7, 5],
                    [0, 4, 5, 1],
                    [2, 3, 7, 6],
                    [0, 2, 6, 4],
                    [1, 5, 7, 3],
                ] {
                    indices.extend([a, b, c, a, c, d].map(|index| base + index));
                }
            }
        }
    }
    SceneMesh::new(vertices, indices)
}

/// Snaps a 3D world translation to the nearest uniform `grid_step` increment when `grid_step > 0.0`.
#[must_use]
pub fn snap_translation_to_grid(position: Vec3, grid_step: f32) -> Vec3 {
    if !position.is_finite() || !grid_step.is_finite() || grid_step <= 0.0 {
        return position;
    }
    (position / grid_step).round() * grid_step
}

/// Snaps a rotation angle in radians to the nearest `step_rad` increment when `step_rad > 0.0`.
#[must_use]
pub fn snap_angle_radians(angle_rad: f32, step_rad: f32) -> f32 {
    if !angle_rad.is_finite() || !step_rad.is_finite() || step_rad <= 0.0 {
        return angle_rad;
    }
    (angle_rad / step_rad).round() * step_rad
}

/// Snaps positive scale components to the nearest `step` increment (>= `0.05`) when `step > 0.0`.
#[must_use]
pub fn snap_scale_factor(scale: Vec3, step: f32) -> Vec3 {
    if !scale.is_finite() || !step.is_finite() || step <= 0.0 {
        return scale;
    }
    ((scale / step).round() * step).max(Vec3::splat(0.05))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snap_translation_angle_and_scale_quantize_to_step() {
        let p = snap_translation_to_grid(Vec3::new(1.24, -0.26, 0.51), 0.25);
        assert!((p - Vec3::new(1.25, -0.25, 0.50)).length() < 1e-6);
        assert_eq!(
            snap_translation_to_grid(Vec3::new(1.2, 2.3, 3.4), 0.0),
            Vec3::new(1.2, 2.3, 3.4)
        );
        let step = std::f32::consts::FRAC_PI_4;
        let a = snap_angle_radians(0.70, step);
        assert!((a - step).abs() < 1e-6);
        let s = snap_scale_factor(Vec3::new(1.13, 0.01, 2.38), 0.25);
        assert!((s - Vec3::new(1.25, 0.05, 2.5)).length() < 1e-6);
        assert!(rotation_gizmo().is_ok());
        assert!(scale_gizmo().is_ok());
        let mut cfg = GizmoSnapConfig::default();
        cfg.step_up(GizmoMode::Translate);
        assert!((cfg.translation_step - 0.5).abs() < 1e-6);
        cfg.step_down(GizmoMode::Translate);
        assert!((cfg.translation_step - 0.25).abs() < 1e-6);
    }
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
