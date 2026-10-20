//! CPU CDLOD quadtree selection for the ocean grid (allocation free per frame).

/// Quads per node edge; the index buffer draws `GRID * GRID` quads per instance.
pub(super) const GRID: u32 = 32;
/// Finest node size in metres (6.25 cm vertex spacing).
const LEAF: f64 = 2.0;
/// LOD 0 morph range.
pub(super) const R0: f64 = 8.0 * LEAF;
const LODS: usize = 17;
const MAX_DIST: f64 = 160_000.0;
pub(super) const MAX_NODES: usize = 8192;
/// Vertical bound of the displaced surface.
const HB: f64 = 4.0;
/// Horizontal displacement margin for culling.
const XB: f64 = 4.0;

#[derive(Debug)]
pub(super) struct Cdlod {
    ranges: [f64; LODS],
    planes: [f64; 16],
    camera: [f64; 3],
    /// Per node: x0, z0, size, lod.
    pub(super) nodes: Vec<[f32; 4]>,
}

impl Cdlod {
    pub(super) fn new() -> Self {
        let mut ranges = [0.0; LODS];
        for (level, range) in ranges.iter_mut().enumerate() {
            *range = R0 * f64::powi(2.0, level as i32);
        }
        Self {
            ranges,
            planes: [0.0; 16],
            camera: [0.0; 3],
            nodes: Vec::with_capacity(MAX_NODES),
        }
    }

    /// Selects nodes around `camera` against the side planes of the camera-relative
    /// column-major view-projection `view_proj`.
    pub(super) fn select(&mut self, camera: [f64; 3], view_proj: &[f32; 16]) {
        self.nodes.clear();
        self.camera = camera;
        let m = view_proj.map(f64::from);
        for i in 0..4 {
            let r = i >> 1;
            let s = if i & 1 == 1 { -1.0 } else { 1.0 };
            self.planes[i * 4] = m[3] + s * m[r];
            self.planes[i * 4 + 1] = m[7] + s * m[4 + r];
            self.planes[i * 4 + 2] = m[11] + s * m[8 + r];
            self.planes[i * 4 + 3] = m[15] + s * m[12 + r];
        }
        let top = LODS - 1;
        let ts = LEAF * f64::powi(2.0, top as i32);
        let [cx, _, cz] = camera;
        let x0 = ((cx - MAX_DIST) / ts).floor() as i64;
        let x1 = ((cx + MAX_DIST) / ts).floor() as i64;
        let z0 = ((cz - MAX_DIST) / ts).floor() as i64;
        let z1 = ((cz + MAX_DIST) / ts).floor() as i64;
        for zi in z0..=z1 {
            for xi in x0..=x1 {
                self.select_node(xi as f64 * ts, zi as f64 * ts, ts, top);
            }
        }
    }

    fn sphere_hit(&self, x0: f64, z0: f64, size: f64, r: f64) -> bool {
        let [cx, cy, cz] = self.camera;
        let dx = (x0 - cx).max(0.0).max(cx - (x0 + size));
        let dz = (z0 - cz).max(0.0).max(cz - (z0 + size));
        let dy = (-HB - cy).max(0.0).max(cy - HB);
        dx * dx + dy * dy + dz * dz <= r * r
    }

    fn in_frustum(&self, x0: f64, z0: f64, size: f64) -> bool {
        let [cx, cy, cz] = self.camera;
        let (minx, maxx) = (x0 - XB - cx, x0 + size + XB - cx);
        let (minz, maxz) = (z0 - XB - cz, z0 + size + XB - cz);
        let (miny, maxy) = (-HB - cy, HB - cy);
        self.planes.chunks_exact(4).all(|p| {
            let px = if p[0] > 0.0 { maxx } else { minx };
            let py = if p[1] > 0.0 { maxy } else { miny };
            let pz = if p[2] > 0.0 { maxz } else { minz };
            p[0] * px + p[1] * py + p[2] * pz + p[3] >= 0.0
        })
    }

    fn add_node(&mut self, x0: f64, z0: f64, size: f64, lod: usize) {
        if self.nodes.len() < MAX_NODES {
            self.nodes
                .push([x0 as f32, z0 as f32, size as f32, lod as f32]);
        }
    }

    fn select_node(&mut self, x0: f64, z0: f64, size: f64, lod: usize) -> bool {
        if !self.sphere_hit(x0, z0, size, self.ranges[lod]) {
            return false;
        }
        if !self.sphere_hit(x0, z0, size, MAX_DIST) || !self.in_frustum(x0, z0, size) {
            return true;
        }
        if lod == 0 || !self.sphere_hit(x0, z0, size, self.ranges[lod - 1]) {
            self.add_node(x0, z0, size, lod);
            return true;
        }
        let h = size * 0.5;
        for i in 0..4 {
            let nx = x0 + (i & 1) as f64 * h;
            let nz = z0 + (i >> 1) as f64 * h;
            if !self.select_node(nx, nz, h, lod - 1)
                && self.sphere_hit(nx, nz, h, MAX_DIST)
                && self.in_frustum(nx, nz, h)
            {
                self.add_node(nx, nz, h, lod - 1);
            }
        }
        true
    }
}
