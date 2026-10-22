//! Surface meshes for water bodies, in one vertex format shared by every shape.

use crate::bodies::{WaterBody, WaterShape, point_in_polygon};
use crate::river::RiverSample;
use bytemuck::{Pod, Zeroable};
use glam::Vec2;

/// 32-byte surface vertex.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct WaterVertex {
    /// Still-water surface position (world).
    pub position: [f32; 3],
    /// Distance along the flow (m); waves and foam scroll with it.
    pub along: f32,
    /// Surface current in the xz plane (m/s).
    pub flow: [f32; 2],
    /// Base white water, 0..1.
    pub foam: f32,
    /// 1 on a free-falling sheet (waterfall).
    pub falls: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WaterMesh {
    pub vertices: Vec<WaterVertex>,
    pub indices: Vec<u32>,
}

/// Grid cells are about this size; large bodies coarsen to stay under `MAX_CELLS`.
const BASIN_SPACING: f32 = 0.08;
const REGION_SPACING: f32 = 0.5;
const MAX_CELLS: u32 = 384;
const RIVER_COLUMNS: u32 = 8;

fn grid(
    lo: Vec2,
    hi: Vec2,
    spacing: f32,
    level: f32,
    keep: impl Fn(Vec2, f32) -> bool,
) -> WaterMesh {
    let size = hi - lo;
    let nx = ((size.x / spacing - 1e-3).ceil() as u32).clamp(1, MAX_CELLS);
    let nz = ((size.y / spacing - 1e-3).ceil() as u32).clamp(1, MAX_CELLS);
    let cell = Vec2::new(size.x / nx as f32, size.y / nz as f32);
    let mut mesh = WaterMesh::default();
    for j in 0..=nz {
        for i in 0..=nx {
            let p = lo + cell * Vec2::new(i as f32, j as f32);
            mesh.vertices.push(WaterVertex {
                position: [p.x, level, p.y],
                ..Default::default()
            });
        }
    }
    for j in 0..nz {
        for i in 0..nx {
            let centre = lo + cell * Vec2::new(i as f32 + 0.5, j as f32 + 0.5);
            if !keep(centre, cell.max_element()) {
                continue;
            }
            let a = i + j * (nx + 1);
            let (b, c) = (a + 1, a + nx + 1);
            mesh.indices.extend_from_slice(&[a, c, c + 1, a, c + 1, b]);
        }
    }
    mesh
}

/// Ribbon across the resampled centre line; falls become vertical sheets on their own.
pub fn river_mesh(samples: &[RiverSample]) -> WaterMesh {
    let mut mesh = WaterMesh::default();
    if samples.len() < 2 {
        return mesh;
    }
    for s in samples {
        let flow = s.tangent * s.speed;
        for c in 0..=RIVER_COLUMNS {
            let across = c as f32 / RIVER_COLUMNS as f32 - 0.5;
            let p = s.position + s.right * (across * s.width);
            // banks flow slower than the middle of the channel
            let profile = 1.0 - (2.0 * across).powi(2) * 0.6;
            mesh.vertices.push(WaterVertex {
                position: p.to_array(),
                along: s.along,
                flow: [flow.x * profile, flow.z * profile],
                foam: s.foam,
                falls: f32::from(u8::from(s.falls)),
            });
        }
    }
    let row = RIVER_COLUMNS + 1;
    for j in 0..samples.len() as u32 - 1 {
        for i in 0..RIVER_COLUMNS {
            let a = j * row + i;
            let (b, c) = (a + 1, a + row);
            mesh.indices.extend_from_slice(&[a, c, c + 1, a, c + 1, b]);
        }
    }
    mesh
}

/// The surface of `body`; `river_samples` are its resampled centre line (rivers only).
pub fn surface_mesh(body: &WaterBody, river_samples: &[RiverSample]) -> WaterMesh {
    match &body.shape {
        WaterShape::Basin { min, max } => grid(
            Vec2::new(min.x, min.z),
            Vec2::new(max.x, max.z),
            BASIN_SPACING,
            max.y,
            |_, _| true,
        ),
        WaterShape::Region { outline, level, .. } => {
            let (lo, hi) = outline.iter().fold(
                (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
                |(lo, hi), p| (lo.min(*p), hi.max(*p)),
            );
            let shore = body.shore.as_ref();
            grid(lo, hi, REGION_SPACING, *level, |c, cell| {
                // keep a cell of margin so the terrain, not the mesh edge, forms the shore
                let near = [Vec2::ZERO, Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y]
                    .iter()
                    .any(|o| point_in_polygon(c + *o * cell, outline));
                near && shore.is_none_or(|s| s.depth_at(c) > -0.5)
            })
        }
        WaterShape::River(_) => river_mesh(river_samples),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::river::{River, RiverNode};
    use glam::Vec3;

    #[test]
    fn basin_grid_covers_the_pool() {
        let m = surface_mesh(
            &WaterBody::pool(Vec3::new(0., -1., 0.), Vec3::new(4., 0.1, 10.)),
            &[],
        );
        assert_eq!(m.indices.len() as u32, 50 * 125 * 6);
        assert!(m.vertices.iter().all(|v| v.position[1] == 0.1));
    }

    #[test]
    fn lake_grid_follows_its_outline_and_river_ribbon_marks_falls() {
        let triangle = vec![Vec2::new(0., 0.), Vec2::new(40., 0.), Vec2::new(0., 40.)];
        let lake = surface_mesh(&WaterBody::lake(triangle, 0.0, -3.0, None), &[]);
        let full = 80 * 80 * 6;
        assert!(lake.indices.len() > full / 3 && lake.indices.len() < full * 2 / 3);
        let node = |z: f32, y: f32| RiverNode {
            position: Vec3::new(0., y, z),
            width: 3.0,
            depth: 0.5,
        };
        let river = River::new(
            vec![
                node(0., 4.),
                node(10., 3.9),
                node(10.5, 1.0),
                node(20., 0.9),
            ],
            1.0,
        );
        let samples = river.samples(0.5);
        let m = river_mesh(&samples);
        assert_eq!(m.vertices.len(), samples.len() * 9);
        assert!(
            m.vertices.iter().any(|v| v.falls == 1.0) && m.vertices.iter().any(|v| v.falls == 0.0)
        );
    }
}
