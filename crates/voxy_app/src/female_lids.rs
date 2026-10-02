//! Independent lid surfaces for retopology inspection; not yet stitched to the head.
use crate::female_face::LidContour;
use glam::Vec3;
use voxy_render::{SceneError, SceneMesh, SceneVertex};

const COLUMNS: usize = 33;
const ROWS: usize = 17;

fn mix_vertex(a: SceneVertex, b: SceneVertex, t: f32) -> SceneVertex {
    SceneVertex {
        position: std::array::from_fn(|k| a.position[k] + t * (b.position[k] - a.position[k])),
        uv: std::array::from_fn(|k| a.uv[k] + t * (b.uv[k] - a.uv[k])),
        color: std::array::from_fn(|k| a.color[k] + t * (b.color[k] - a.color[k])),
    }
}
fn subtract_quad(poly: Vec<SceneVertex>, mut quad: [Vec3; 4]) -> Vec<Vec<SceneVertex>> {
    let min = quad
        .iter()
        .fold(Vec3::splat(f32::INFINITY), |a, p| a.min(*p));
    let max = quad
        .iter()
        .fold(Vec3::splat(f32::NEG_INFINITY), |a, p| a.max(*p));
    if (0..2).any(|k| {
        poly.iter().all(|p| p.position[k] < min[k]) || poly.iter().all(|p| p.position[k] > max[k])
    }) {
        return vec![poly];
    }
    let signed_area: f32 = (0..4)
        .map(|i| (quad[i] - quad[0]).cross(quad[(i + 1) % 4] - quad[0]).z)
        .sum();
    if signed_area < 0. {
        quad.reverse();
    }
    let mut inside = poly;
    let mut result = Vec::new();
    for edge in 0..4 {
        if inside.is_empty() {
            break;
        }
        let a = quad[edge];
        let b = quad[(edge + 1) % 4];
        let distance = |p: SceneVertex| {
            (b.x - a.x) * (p.position[1] - a.y) - (b.y - a.y) * (p.position[0] - a.x)
        };
        let mut retained = Vec::new();
        let mut outside = Vec::new();
        for i in 0..inside.len() {
            let p = inside[i];
            let q = inside[(i + 1) % inside.len()];
            let d = distance(p);
            let next = distance(q);
            if d >= 0. {
                retained.push(p);
            } else {
                outside.push(p);
            }
            if (d >= 0.) != (next >= 0.) {
                let cut = mix_vertex(p, q, d / (d - next));
                retained.push(cut);
                outside.push(cut);
            }
        }
        if outside.len() >= 3 {
            result.push(outside);
        }
        inside = retained;
    }
    result
}

#[derive(Debug)]
pub(crate) struct LidSurface {
    outer: [[Vec3; COLUMNS]; 4],
    free: [[Vec3; COLUMNS]; 4],
    seam: [[Vec3; COLUMNS]; 2],
    bind_depth: [[f32; COLUMNS]; 4 * ROWS],
}
fn globe_front(x: f32, y: f32) -> f32 {
    0.1215
        + (0.0122_f32.powi(2) - (x.abs() - 0.03287).powi(2) - (y - 0.71242).powi(2))
            .max(0.)
            .sqrt()
        + 0.0006
}
fn front_at(vertices: &[SceneVertex], indices: &[u32], x: f32, y: f32) -> Option<f32> {
    let mut front: Option<f32> = None;
    for ids in indices.chunks_exact(3) {
        let p: [Vec3; 3] =
            std::array::from_fn(|k| Vec3::from_array(vertices[ids[k] as usize].position));
        let determinant =
            (p[1].y - p[2].y) * (p[0].x - p[2].x) + (p[2].x - p[1].x) * (p[0].y - p[2].y);
        if determinant.abs() < 1e-12 {
            continue;
        }
        let a = ((p[1].y - p[2].y) * (x - p[2].x) + (p[2].x - p[1].x) * (y - p[2].y)) / determinant;
        let b = ((p[2].y - p[0].y) * (x - p[2].x) + (p[0].x - p[2].x) * (y - p[2].y)) / determinant;
        let c = 1. - a - b;
        if a >= -1e-5 && b >= -1e-5 && c >= -1e-5 {
            let z = a * p[0].z + b * p[1].z + c * p[2].z;
            front = Some(front.map_or(z, |old| old.max(z)));
        }
    }
    front
}
impl LidSurface {
    /// Research replacement of the bind-space skin region, before rig integration.
    pub fn mesh_with_body(&self, body: &SceneMesh, closure: f32) -> Result<SceneMesh, SceneError> {
        let lids = self.mesh_with_canthi(closure)?;
        let mut vertices = body.vertices().to_vec();
        for vertex in &mut vertices {
            vertex.uv = crate::female_complexion::uv(Vec3::from_array(vertex.position));
            vertex.color = [0.72, 0.46, 0.34, 1.];
        }
        let mut indices = Vec::new();
        for ids in body.indices().chunks_exact(3) {
            let triangle: [SceneVertex; 3] = std::array::from_fn(|k| vertices[ids[k] as usize]);
            if triangle.iter().all(|p| p.position[2] < 0.118)
                || triangle.iter().all(|p| p.position[1] < 0.69)
                || triangle.iter().all(|p| p.position[1] > 0.74)
                || triangle.iter().all(|p| p.position[0] > 0.055)
                || triangle.iter().all(|p| p.position[0] < -0.055)
            {
                indices.extend_from_slice(ids);
                continue;
            }
            let mut pieces = vec![triangle.to_vec()];
            for side in 0..2 {
                for column in 0..COLUMNS - 1 {
                    let quad = [
                        self.outer[side * 2][column],
                        self.outer[side * 2][column + 1],
                        self.outer[side * 2 + 1][column + 1],
                        self.outer[side * 2 + 1][column],
                    ];
                    pieces = pieces
                        .into_iter()
                        .flat_map(|poly| subtract_quad(poly, quad))
                        .collect();
                }
            }
            if pieces.len() == 1
                && pieces[0].len() == 3
                && pieces[0]
                    .iter()
                    .zip(triangle)
                    .all(|(a, b)| a.position == b.position)
            {
                indices.extend_from_slice(ids);
                continue;
            }
            for poly in pieces {
                let base = vertices.len() as u32;
                for k in 1..poly.len() - 1 {
                    let points =
                        [poly[0], poly[k], poly[k + 1]].map(|v| Vec3::from_array(v.position));
                    if (points[1] - points[0])
                        .cross(points[2] - points[0])
                        .length()
                        > 1e-12
                    {
                        indices.extend([base, base + k as u32, base + k as u32 + 1]);
                    }
                }
                vertices.extend(poly);
            }
        }
        let base = vertices.len() as u32;
        // Reuse clipped skin nodes on each outer segment. Splitting the adjacent
        // lid triangle follows the original skin depth along the shared edge.
        let used: std::collections::HashSet<u32> = indices.iter().copied().collect();
        let mut borders = Vec::new();
        let mut aliases = std::collections::HashMap::<u32, u32>::new();
        let mut merged_nodes = 0;
        for border in &self.outer {
            for pair in border.windows(2) {
                let a = pair[0].as_dvec3();
                let edge = pair[1].as_dvec3() - a;
                let mut nodes = Vec::new();
                for &i in &used {
                    let p = Vec3::from_array(vertices[i as usize].position).as_dvec3();
                    let t =
                        (p - a).truncate().dot(edge.truncate()) / edge.truncate().length_squared();
                    if t > 1e-4
                        && t < 1.0 - 1e-4
                        && p.z >= 0.118
                        && (p - (a + t * edge)).truncate().length() < 2e-7
                        && front_at(body.vertices(), body.indices(), p.x as f32, p.y as f32)
                            .is_some_and(|z| (z as f64 - p.z).abs() < 2e-6)
                    {
                        nodes.push((t, i));
                    }
                }
                nodes.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                nodes.dedup_by(|a, b| {
                    vertices[a.1 as usize].position == vertices[b.1 as usize].position
                });
                // Clip arithmetic can produce separate nodes only nanometres
                // apart. Reuse one index on both sides instead of propagating
                // a near-zero-width column through the entire lid.
                let mut distinct: Vec<(f64, u32)> = Vec::new();
                for node in nodes {
                    if let Some(previous) = distinct.last_mut() {
                        let p = Vec3::from_array(vertices[previous.1 as usize].position);
                        let q = Vec3::from_array(vertices[node.1 as usize].position);
                        if p.distance(q) < 2e-7 {
                            let canonical = previous.1.min(node.1);
                            aliases.insert(previous.1.max(node.1), canonical);
                            if canonical == node.1 {
                                *previous = node;
                            }
                            merged_nodes += 1;
                            continue;
                        }
                    }
                    distinct.push(node);
                }
                borders.push((pair[0], pair[1], distinct));
            }
        }
        for index in &mut indices {
            while let Some(&canonical) = aliases.get(index) {
                *index = canonical;
            }
        }
        let mut retained = Vec::with_capacity(indices.len());
        for tri in indices.chunks_exact(3) {
            if tri[0] != tri[1] && tri[1] != tri[2] && tri[0] != tri[2] {
                retained.extend_from_slice(tri);
            }
        }
        indices = retained;
        #[cfg(test)]
        eprintln!("merged {merged_nodes} near-coincident exterior cut nodes");
        #[cfg(not(test))]
        let _ = merged_nodes;
        vertices.extend_from_slice(lids.vertices());
        let local_indices: Vec<u32> = body
            .indices()
            .chunks_exact(3)
            .filter(|tri| {
                let p: Vec<_> = tri
                    .iter()
                    .map(|&i| body.vertices()[i as usize].position)
                    .collect();
                !p.iter().all(|p| p[1] < 0.69)
                    && !p.iter().all(|p| p[1] > 0.74)
                    && !p.iter().all(|p| p[0].abs() > 0.055)
                    && !p.iter().all(|p| p[2] < 0.118)
            })
            .flatten()
            .copied()
            .collect();
        for part in 0..4 {
            for column in 0..COLUMNS - 1 {
                let nodes = &borders[part * (COLUMNS - 1) + column].2;
                let width = nodes.len() + 2;
                let mut grid = vec![0; ROWS * width];
                for row in 0..ROWS {
                    let a = base + ((part * ROWS + row) * COLUMNS + column) as u32;
                    let b = a + 1;
                    grid[row * width] = a;
                    grid[row * width + width - 1] = b;
                    for (k, &(t, skin_id)) in nodes.iter().enumerate() {
                        let id = if row == 0 {
                            skin_id
                        } else {
                            let t = t as f32;
                            let u = row as f32 / (ROWS - 1) as f32;
                            let va = vertices[a as usize];
                            let vb = vertices[b as usize];
                            let mut position = Vec3::from_array(va.position)
                                .lerp(Vec3::from_array(vb.position), t);
                            let bind = self.outer[part][column]
                                .lerp(self.free[part][column], u)
                                .lerp(
                                    self.outer[part][column + 1]
                                        .lerp(self.free[part][column + 1], u),
                                    t,
                                );
                            if row < ROWS - 1 {
                                let linear_depth = self.bind_depth[part * ROWS + row][column]
                                    * (1. - t)
                                    + self.bind_depth[part * ROWS + row][column + 1] * t;
                                let depth =
                                    front_at(body.vertices(), &local_indices, bind.x, bind.y)
                                        .unwrap_or(linear_depth);
                                // Evaluate the same bind-depth + motion rule as
                                // regular columns, then apply globe support once.
                                let free =
                                    self.free[part][column].lerp(self.free[part][column + 1], t);
                                let seam = self.seam[part / 2][column]
                                    .lerp(self.seam[part / 2][column + 1], t);
                                position.z = depth + u * u * closure * (seam.z - free.z);
                            }
                            position.z = position.z.max(globe_front(position.x, position.y));
                            let id = vertices.len() as u32;
                            vertices.push(SceneVertex {
                                position: position.to_array(),
                                uv: crate::female_complexion::uv(bind),
                                color: [0.72, 0.46, 0.34, 1.],
                            });
                            id
                        };
                        grid[row * width + k + 1] = id;
                    }
                }
                let flip = (part % 2 == 1) != (part / 2 == 1);
                for row in 0..ROWS - 1 {
                    for k in 0..width - 1 {
                        let a = grid[row * width + k];
                        let b = grid[row * width + k + 1];
                        let c = grid[(row + 1) * width + k];
                        let d = grid[(row + 1) * width + k + 1];
                        for (half, mut tri) in [[a, b, c], [b, d, c]].into_iter().enumerate() {
                            // Topology follows the bind parameters, never the pose.
                            if (column == 0 && k == 0 && half == 0)
                                || (column == COLUMNS - 2 && k == width - 2 && half == 1)
                            {
                                continue;
                            }
                            if flip {
                                tri.swap(1, 2);
                            }
                            indices.extend(tri);
                        }
                    }
                }
            }
        }
        // Canthal tissue remains independent of the skin grid.
        indices.extend(
            lids.indices()
                .chunks_exact(3)
                .filter(|tri| tri.iter().all(|&i| i as usize >= 4 * ROWS * COLUMNS))
                .flatten()
                .map(|i| i + base),
        );
        let mut normals = std::collections::HashMap::<[i32; 3], Vec3>::new();
        let key = |v: SceneVertex| v.position.map(|p| (p * 10_000_000.).round() as i32);
        for ids in indices.chunks_exact(3) {
            let p = ids
                .iter()
                .map(|&i| Vec3::from_array(vertices[i as usize].position))
                .collect::<Vec<_>>();
            let p: Vec<_> = p.iter().map(|p| p.as_dvec3()).collect();
            let Some(normal) = (p[1] - p[0]).cross(p[2] - p[0]).try_normalize() else {
                continue;
            };
            for (corner, &i) in ids.iter().enumerate() {
                let a = p[(corner + 1) % 3] - p[corner];
                let b = p[(corner + 2) % 3] - p[corner];
                let denominator = a.length() * b.length();
                if denominator == 0. {
                    continue;
                }
                let angle = (a.dot(b) / denominator).clamp(-1., 1.).acos();
                *normals.entry(key(vertices[i as usize])).or_default() +=
                    (normal * angle).as_vec3();
            }
        }
        for vertex in &mut vertices {
            if vertex.uv[0] < 0. {
                continue;
            }
            let normal = normals
                .get(&key(*vertex))
                .copied()
                .unwrap_or(Vec3::Z)
                .try_normalize()
                .unwrap_or(Vec3::Z);
            let illumination = 0.2
                + 0.6 * normal.dot(Vec3::new(-0.5, 0.7, 1.).normalize()).max(0.)
                + 0.2 * normal.dot(Vec3::new(0.8, 0.2, 0.5).normalize()).max(0.);
            for channel in &mut vertex.color[..3] {
                *channel *= illumination;
            }
        }
        SceneMesh::new(vertices, indices)
    }
    pub fn new(vertices: &[SceneVertex], indices: &[u32]) -> Result<Self, SceneError> {
        let contour = LidContour::new(vertices, indices);
        let local_indices: Vec<u32> = indices
            .chunks_exact(3)
            .filter(|ids| {
                let p = ids
                    .iter()
                    .map(|&i| vertices[i as usize].position)
                    .collect::<Vec<_>>();
                !p.iter().all(|p| p[1] < 0.69)
                    && !p.iter().all(|p| p[1] > 0.74)
                    && !p.iter().all(|p| p[0] < -0.055)
                    && !p.iter().all(|p| p[0] > 0.055)
                    && !p.iter().all(|p| p[2] < 0.118)
            })
            .flatten()
            .copied()
            .collect();
        let indices = local_indices.as_slice();
        let mut result = Self {
            outer: [[Vec3::ZERO; COLUMNS]; 4],
            free: [[Vec3::ZERO; COLUMNS]; 4],
            seam: [[Vec3::ZERO; COLUMNS]; 2],
            bind_depth: [[0.; COLUMNS]; 4 * ROWS],
        };
        for (side_index, side) in [1., -1.].into_iter().enumerate() {
            let corner_y = [0.019, 0.047].map(|x| {
                let e = contour.at(side * x);
                (e[0] + e[1]) * 0.5
            });
            for column in 0..COLUMNS {
                let t = column as f32 / (COLUMNS - 1) as f32;
                let arc = (std::f32::consts::PI * t).sin().max(0.);
                let x = side * (0.019 + 0.028 * t);
                let edge = contour.at(x);
                let common = corner_y[0] * (1. - t)
                    + corner_y[1] * t
                    + (0.7104 - (corner_y[0] + corner_y[1]) * 0.5) * arc;
                let corner = column == 0 || column == COLUMNS - 1;
                let seam_z = if corner {
                    front_at(vertices, indices, x, common).ok_or(SceneError::InvalidGeometry)?
                } else {
                    globe_front(x, common)
                        .max(front_at(vertices, indices, x, common).unwrap_or(f32::NEG_INFINITY))
                };
                result.seam[side_index][column] = Vec3::new(x, common, seam_z);
                for upper in [false, true] {
                    let part = side_index * 2 + usize::from(upper);
                    let y = if corner {
                        common
                    } else {
                        edge[usize::from(upper)]
                    };
                    let outer_y = y + if upper { 0.012 * arc } else { -0.012 * arc };
                    let outer_z = front_at(vertices, indices, x, outer_y)
                        .ok_or(SceneError::InvalidGeometry)?;
                    result.outer[part][column] = Vec3::new(x, outer_y, outer_z);
                    let edge_z = if corner {
                        outer_z
                    } else {
                        globe_front(x, y)
                            .max(front_at(vertices, indices, x, y).unwrap_or(f32::NEG_INFINITY))
                    };
                    result.free[part][column] = Vec3::new(x, y, edge_z);
                }
            }
        }
        for part in 0..4 {
            for row in 0..ROWS {
                for column in 0..COLUMNS {
                    let u = row as f32 / (ROWS - 1) as f32;
                    let p = result.outer[part][column].lerp(result.free[part][column], u);
                    result.bind_depth[part * ROWS + row][column] =
                        front_at(vertices, indices, p.x, p.y).unwrap_or(p.z);
                }
            }
        }
        Ok(result)
    }
    pub fn mesh(&self, closure: f32) -> Result<SceneMesh, SceneError> {
        if !closure.is_finite() || !(0. ..=1.).contains(&closure) {
            return Err(SceneError::InvalidGeometry);
        }
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for part in 0..4 {
            let base = vertices.len() as u32;
            for row in 0..ROWS {
                let u = row as f32 / (ROWS - 1) as f32;
                for column in 0..COLUMNS {
                    let outer = self.outer[part][column];
                    let inner = if closure == 1. {
                        self.seam[part / 2][column]
                    } else {
                        self.free[part][column].lerp(self.seam[part / 2][column], closure)
                    };
                    let mut p = outer.lerp(self.free[part][column], u);
                    let motion = u * u;
                    p.y += motion * (inner.y - self.free[part][column].y);
                    p.z = self.bind_depth[part * ROWS + row][column]
                        + motion * (inner.z - self.free[part][column].z);
                    p.x = outer.x;
                    if column == 0 || column == COLUMNS - 1 || row == 0 {
                        p = outer;
                    } else if row == ROWS - 1 {
                        p = inner;
                    }
                    if row > 0 && column > 0 && column < COLUMNS - 1 {
                        p.z = p.z.max(globe_front(p.x, p.y));
                    }
                    vertices.push(SceneVertex {
                        position: p.to_array(),
                        uv: crate::female_complexion::uv(
                            self.outer[part][column].lerp(self.free[part][column], u),
                        ),
                        color: [0.72, 0.46, 0.34, 1.],
                    });
                }
            }
            for row in 0..ROWS - 1 {
                for column in 0..COLUMNS - 1 {
                    let a = base + (row * COLUMNS + column) as u32;
                    let b = a + 1;
                    let c = a + COLUMNS as u32;
                    let d = c + 1;
                    let flip = (part % 2 == 1) != (part / 2 == 1);
                    // Collapsed canthus columns are shared points, not tiny faces.
                    if column > 0 {
                        if flip {
                            indices.extend([a, c, b]);
                        } else {
                            indices.extend([a, b, c]);
                        }
                    }
                    if column < COLUMNS - 2 {
                        if flip {
                            indices.extend([b, c, d]);
                        } else {
                            indices.extend([b, d, c]);
                        }
                    }
                }
            }
        }
        SceneMesh::new(vertices, indices)
    }
    pub fn mesh_with_canthi(&self, closure: f32) -> Result<SceneMesh, SceneError> {
        let mesh = self.mesh(closure)?;
        let mut vertices = mesh.vertices().to_vec();
        let mut indices = mesh.indices().to_vec();
        for side in 0..2 {
            for start in [0, 28] {
                let base = vertices.len() as u32;
                for column in start..=start + 4 {
                    let seam = self.seam[side][column];
                    let tissue_z = seam
                        .z
                        .min(self.free[side * 2][column].z)
                        .min(self.free[side * 2 + 1][column].z)
                        - 0.0008;
                    let margin =
                        0.00015 * (std::f32::consts::PI * column as f32 / 32.).sin().max(0.);
                    for upper in [false, true] {
                        let edge = self.free[side * 2 + usize::from(upper)][column];
                        let y = edge.y * (1. - closure)
                            + seam.y * closure
                            + if upper { margin } else { -margin };
                        vertices.push(SceneVertex {
                            position: [seam.x, y, tissue_z],
                            uv: [-1., 0.24],
                            color: if start == 0 {
                                [0.44, 0.19, 0.21, 1.]
                            } else {
                                [0.35, 0.16, 0.18, 1.]
                            },
                        });
                    }
                }
                for cell in 0..4 {
                    let a = base + cell * 2;
                    for mut tri in [[a, a + 2, a + 1], [a + 1, a + 2, a + 3]] {
                        if (start == 0 && cell == 0 && tri[0] == a)
                            || (start == 28 && cell == 3 && tri[0] == a + 1)
                        {
                            continue;
                        }
                        if side == 1 {
                            tri.swap(1, 2);
                        }
                        indices.extend(tri);
                    }
                }
            }
        }
        SceneMesh::new(vertices, indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn original_lid_base(mesh: &SceneMesh, lids: &SceneMesh) -> usize {
        mesh.vertices()
            .windows(lids.vertices().len())
            .position(|window| {
                window[0].position == lids.vertices()[0].position
                    && window
                        .iter()
                        .zip(lids.vertices())
                        .all(|(a, b)| a.position == b.position)
            })
            .unwrap()
    }
    #[test]
    fn added_neutral_lid_nodes_follow_source_depth() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let surface = LidSurface::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let lids = surface.mesh_with_canthi(0.).unwrap();
        let mesh = surface.mesh_with_body(&body.mesh, 0.).unwrap();
        let start = original_lid_base(&mesh, &lids) + lids.vertices().len();
        let local: Vec<u32> = body
            .mesh
            .indices()
            .chunks_exact(3)
            .filter(|tri| {
                let p: Vec<_> = tri
                    .iter()
                    .map(|&i| body.mesh.vertices()[i as usize].position)
                    .collect();
                !p.iter().all(|p| p[1] < 0.69)
                    && !p.iter().all(|p| p[1] > 0.74)
                    && !p.iter().all(|p| p[2] < 0.118)
            })
            .flatten()
            .copied()
            .collect();
        let mut checked = 0;
        for vertex in &mesh.vertices()[start..] {
            let p = Vec3::from_array(vertex.position);
            let column = (((p.x.abs() - 0.019) / 0.028 * (COLUMNS - 1) as f32).floor() as usize)
                .min(COLUMNS - 2);
            let side = usize::from(p.x < 0.);
            let t = (p.x - surface.outer[side * 2][column].x)
                / (surface.outer[side * 2][column + 1].x - surface.outer[side * 2][column].x);
            let upper =
                surface.free[side * 2 + 1][column].lerp(surface.free[side * 2 + 1][column + 1], t);
            let part = side * 2 + usize::from(p.y > upper.y);
            let free = surface.free[part][column].lerp(surface.free[part][column + 1], t);
            let outer = surface.outer[part][column].lerp(surface.outer[part][column + 1], t);
            let u = (p.y - outer.y) / (free.y - outer.y);
            if !(0.01..0.99).contains(&u) {
                continue;
            }
            if let Some(depth) = front_at(body.mesh.vertices(), &local, p.x, p.y) {
                assert!(
                    (p.z - depth.max(globe_front(p.x, p.y))).abs() < 2e-6,
                    "added neutral node departed from source skin: {p:?}"
                );
                checked += 1;
            }
        }
        assert!(checked > 1000);
    }

    #[test]
    #[ignore = "manual neutral globe-support displacement audit"]
    fn audit_globe_support_displacement() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let surface = LidSurface::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let mesh = surface.mesh(0.).unwrap();
        for row in 1..ROWS - 1 {
            let mut count = 0;
            let mut maximum = 0.0_f32;
            for part in 0..4 {
                for column in 1..COLUMNS - 1 {
                    let depth = mesh.vertices()[(part * ROWS + row) * COLUMNS + column].position[2];
                    let shift = depth - surface.bind_depth[part * ROWS + row][column];
                    if shift > 1e-6 {
                        count += 1;
                        maximum = maximum.max(shift);
                    }
                }
            }
            eprintln!("globe support row {row}: {count} displaced vertices; max {maximum:.9} m");
        }
    }
    #[test]
    #[ignore = "manual exterior seam face-normal audit"]
    fn audit_adaptive_closure_faces() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let surface = LidSurface::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let open = surface.mesh_with_body(&body.mesh, 0.).unwrap();
        let base = original_lid_base(&open, &surface.mesh_with_canthi(0.).unwrap());
        let split = open
            .indices()
            .chunks_exact(3)
            .position(|tri| tri.iter().any(|&i| i as usize >= base))
            .unwrap();
        let normal = |mesh: &SceneMesh, tri: &[u32]| {
            let p: [glam::DVec3; 3] = std::array::from_fn(|k| {
                Vec3::from_array(mesh.vertices()[tri[k] as usize].position).as_dvec3()
            });
            (p[1] - p[0]).cross(p[2] - p[0])
        };
        let mut report = String::from(
            "closure,skin_faces,zero_area,area_below_1e-12_m2,back_facing,normal_reversals_from_open\n",
        );
        let mut details = String::from("closure,a,b,c,x_m,y_m,z_m,normal_angle_degrees\n");
        for closure in [0., 0.5, 1.] {
            let mesh = if closure == 0. {
                open.clone()
            } else {
                surface.mesh_with_body(&body.mesh, closure).unwrap()
            };
            assert!(
                mesh.indices() == open.indices(),
                "adaptive topology changed at {closure}"
            );
            let (mut count, mut zero, mut tiny, mut back, mut reversed) = (0, 0, 0, 0, 0);
            for tri in mesh.indices()[split * 3..].chunks_exact(3) {
                if tri.iter().any(|&i| mesh.vertices()[i as usize].uv[0] < 0.) {
                    continue;
                }
                let n = normal(&mesh, tri);
                let area = n.length() * 0.5;
                count += 1;
                zero += usize::from(area == 0.);
                tiny += usize::from(area < 1e-12);
                back += usize::from(n.z < 0.);
                let bind_normal = normal(&open, tri);
                if n.dot(bind_normal) < 0. {
                    reversed += 1;
                    let center = tri
                        .iter()
                        .map(|&i| Vec3::from_array(mesh.vertices()[i as usize].position))
                        .sum::<Vec3>()
                        / 3.;
                    let angle = n
                        .normalize()
                        .dot(bind_normal.normalize())
                        .clamp(-1., 1.)
                        .acos()
                        .to_degrees();
                    details.push_str(&format!(
                        "{closure},{},{},{},{:.9},{:.9},{:.9},{angle:.3}\n",
                        tri[0], tri[1], tri[2], center.x, center.y, center.z
                    ));
                }
            }
            let row = format!("{closure},{count},{zero},{tiny},{back},{reversed}\n");
            eprint!("{row}");
            report.push_str(&row);
        }
        std::fs::write("/tmp/voxy-lid-adaptive-closure.csv", report).unwrap();
        std::fs::write("/tmp/voxy-lid-adaptive-reversals.csv", details).unwrap();
    }
    #[test]
    #[ignore = "manual exterior seam face-normal audit"]
    fn audit_exterior_seam_normals() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let surface = LidSurface::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let mesh = surface.mesh_with_body(&body.mesh, 0.).unwrap();
        let base = original_lid_base(&mesh, &surface.mesh_with_canthi(0.).unwrap());
        let split = mesh
            .indices()
            .chunks_exact(3)
            .position(|tri| tri.iter().any(|&i| i as usize >= base))
            .unwrap();
        let key = |p: Vec3| {
            p.to_array()
                .map(|v| (v as f64 * 10_000_000.).round() as i64)
        };
        let mut edges = std::collections::HashMap::<_, Vec<(bool, glam::DVec3, f64)>>::new();
        let mut collapsed_lid_faces = 0;
        for (index, tri) in mesh.indices().chunks_exact(3).enumerate() {
            let p: [Vec3; 3] = std::array::from_fn(|k| {
                Vec3::from_array(mesh.vertices()[tri[k] as usize].position)
            });
            let n = (p[1].as_dvec3() - p[0].as_dvec3()).cross(p[2].as_dvec3() - p[0].as_dvec3());
            let Some(n) = n.try_normalize() else {
                if index >= split {
                    collapsed_lid_faces += 1;
                }
                continue;
            };
            for k in 0..3 {
                let mut pair = [key(p[k]), key(p[(k + 1) % 3])];
                pair.sort();
                edges.entry(pair).or_default().push((
                    index >= split,
                    n,
                    p[k].distance(p[(k + 1) % 3]) as f64,
                ));
            }
        }
        let mut count = 0;
        let mut weighted = 0.0_f64;
        let mut length = 0.0_f64;
        let mut maximum = 0.0_f64;
        for faces in edges.values() {
            if faces.len() != 2 || faces[0].0 == faces[1].0 {
                continue;
            }
            let angle = faces[0]
                .1
                .dot(faces[1].1)
                .clamp(-1., 1.)
                .acos()
                .to_degrees();
            weighted += angle * faces[0].2;
            length += faces[0].2;
            maximum = maximum.max(angle);
            count += 1;
        }
        assert!(count > 0);
        eprintln!(
            "exterior seam: {count} paired edges; length-weighted face-normal angle {:.3} degrees; max {maximum:.3} degrees",
            weighted / length
        );
        eprintln!("assembled neutral lid has {collapsed_lid_faces} zero-area faces");
    }
    #[test]
    #[ignore = "manual replacement boundary topology audit"]
    fn audit_replacement_boundary_subdivision() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let surface = LidSurface::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let mesh = surface.mesh_with_body(&body.mesh, 0.).unwrap();
        let lid_base = original_lid_base(&mesh, &surface.mesh_with_canthi(0.).unwrap());
        let split = mesh
            .indices()
            .chunks_exact(3)
            .position(|tri| tri.iter().any(|&i| i as usize >= lid_base))
            .unwrap()
            * 3;
        let skin_indices = &mesh.indices()[..split];
        let lid_nodes: std::collections::HashSet<_> = mesh.indices()[split..]
            .iter()
            .map(|&i| {
                mesh.vertices()[i as usize]
                    .position
                    .map(|v| (v as f64 * 10_000_000.).round() as i64)
            })
            .collect();
        let mut used = std::collections::HashSet::new();
        for &i in skin_indices {
            used.insert(i);
        }
        let mut report = String::from(
            "part,column,missing_skin_nodes,max_xy_distance_m,max_depth_from_linear_border_m\n",
        );
        let mut total = 0;
        let mut maximum_depth = 0.0_f64;
        for (part, border) in surface.outer.iter().enumerate() {
            for (column, pair) in border.windows(2).enumerate() {
                let a = pair[0].as_dvec3();
                let edge = pair[1].as_dvec3() - a;
                let mut nodes = std::collections::HashSet::new();
                let mut maximum = 0.0_f64;
                let mut depth = 0.0_f64;
                for &i in &used {
                    let p = Vec3::from_array(mesh.vertices()[i as usize].position).as_dvec3();
                    let t =
                        (p - a).truncate().dot(edge.truncate()) / edge.truncate().length_squared();
                    let distance = (p - (a + edge * t)).truncate().length();
                    if t > 1e-4
                        && t < 1.0 - 1e-4
                        && p.z >= 0.118
                        && distance < 2e-7
                        && front_at(
                            body.mesh.vertices(),
                            body.mesh.indices(),
                            p.x as f32,
                            p.y as f32,
                        )
                        .is_some_and(|z| (z as f64 - p.z).abs() < 2e-6)
                    {
                        let key = p.to_array().map(|v| (v * 10_000_000.).round() as i64);
                        if !lid_nodes.contains(&key) {
                            nodes.insert(key);
                        }
                        maximum = maximum.max(distance);
                        depth = depth.max((p.z - (a + edge * t).z).abs());
                    }
                }
                total += nodes.len();
                maximum_depth = maximum_depth.max(depth);
                report.push_str(&format!(
                    "{part},{column},{},{maximum:.9},{depth:.9}\n",
                    nodes.len()
                ));
            }
        }
        std::fs::write("/tmp/voxy-lid-boundary-subdivision.csv", report).unwrap();
        eprintln!("replacement border: {total} interior skin nodes absent from regular lid border");
        eprintln!("source skin differs from linear border depth by up to {maximum_depth:.9} m");
        assert_eq!(
            total, 0,
            "skin boundary contains nodes absent from lid subdivision"
        );
    }
    #[test]
    fn outer_transition_row_follows_the_source_skin_profile() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let surface = LidSurface::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let mesh = surface.mesh(0.).unwrap();
        for part in 0..4 {
            for column in [5, 16, 27] {
                let p = Vec3::from_array(
                    mesh.vertices()[(part * ROWS + 1) * COLUMNS + column].position,
                );
                let reference =
                    front_at(body.mesh.vertices(), body.mesh.indices(), p.x, p.y).unwrap();
                assert!(
                    (p.z - reference).abs() < 1e-6,
                    "outer lid transition lost source profile"
                );
            }
        }
    }
    #[test]
    fn replacement_body_preserves_source_positions_and_constant_indices() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let surface = LidSurface::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let open = surface.mesh_with_body(&body.mesh, 0.).unwrap();
        let closed = surface.mesh_with_body(&body.mesh, 1.).unwrap();
        assert_eq!(open.indices(), closed.indices());
        assert!(open.vertices().iter().all(|v| {
            v.position
                .iter()
                .chain(v.color.iter())
                .all(|a| a.is_finite())
        }));
        for (a, b) in open.vertices().iter().zip(body.mesh.vertices()) {
            assert_eq!(a.position, b.position);
        }
        assert!(open.indices().len() > body.mesh.indices().len());
    }
    #[test]
    fn skin_region_subtraction_preserves_area_for_both_windings_and_corner_cells() {
        let vertex = |x, y| SceneVertex {
            position: [x, y, 0.],
            uv: [0.; 2],
            color: [1.; 4],
        };
        let triangle = vec![vertex(0., 0.), vertex(2., 0.), vertex(0., 2.)];
        let area = |pieces: Vec<Vec<SceneVertex>>| {
            pieces
                .iter()
                .map(|poly| {
                    (1..poly.len() - 1)
                        .map(|i| {
                            let a = Vec3::from_array(poly[0].position);
                            let b = Vec3::from_array(poly[i].position);
                            let c = Vec3::from_array(poly[i + 1].position);
                            (b - a).cross(c - a).length() * 0.5
                        })
                        .sum::<f32>()
                })
                .sum::<f32>()
        };
        let mut square = [Vec3::ZERO, Vec3::X, Vec3::X + Vec3::Y, Vec3::Y];
        assert!((area(subtract_quad(triangle.clone(), square)) - 1.).abs() < 1e-6);
        square.reverse();
        assert!((area(subtract_quad(triangle.clone(), square)) - 1.).abs() < 1e-6);
        assert!(
            (area(subtract_quad(
                triangle,
                [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::ZERO]
            )) - 1.5)
                .abs()
                < 1e-6
        );
    }
    #[test]
    fn canthal_tissue_keeps_topology_and_stays_behind_closed_lids() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let surface = LidSurface::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let reference = surface.mesh_with_canthi(0.).unwrap();
        let base_count = surface.mesh(0.).unwrap().indices().len();
        for closure in [0., 0.5, 1.] {
            let mesh = surface.mesh_with_canthi(closure).unwrap();
            assert_eq!(mesh.indices(), reference.indices());
            for ids in mesh.indices()[base_count..].chunks_exact(3) {
                let p: [Vec3; 3] = std::array::from_fn(|k| {
                    Vec3::from_array(mesh.vertices()[ids[k] as usize].position)
                });
                assert!((p[1] - p[0]).cross(p[2] - p[0]).length() > 1e-12);
                for lid_ids in mesh.indices()[..base_count].chunks_exact(3) {
                    let lid: [Vec3; 3] = std::array::from_fn(|k| {
                        Vec3::from_array(mesh.vertices()[lid_ids[k] as usize].position)
                    });
                    assert!(
                        !crate::female_face::tests::triangles_cross(p, lid),
                        "canthal strip crosses lid at closure {closure}: {p:?} / {lid:?}"
                    );
                }
                if closure == 1. {
                    let center = (p[0] + p[1] + p[2]) / 3.;
                    let lids = surface.mesh(1.).unwrap();
                    let front = front_at(lids.vertices(), lids.indices(), center.x, center.y)
                        .expect("canthal tissue exposed outside closed lid");
                    assert!(front > center.z + 0.0001);
                }
            }
        }
    }
    #[test]
    fn closed_surface_occludes_the_globe_front_over_the_iris_region() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let surface = LidSurface::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let open = surface.mesh(0.).unwrap();
        let closed = surface.mesh(1.).unwrap();
        for side in [-1., 1.] {
            assert!(front_at(open.vertices(), open.indices(), side * 0.03287, 0.71242).is_none());
            for dx in [-0.004, 0., 0.004] {
                for dy in [-0.003, 0., 0.003] {
                    let x = side * (0.03287 + dx);
                    let y = 0.71242 + dy;
                    let z = front_at(closed.vertices(), closed.indices(), x, y)
                        .expect("closed lid has a gap over the iris");
                    assert!(
                        z > globe_front(x, y) - 0.0002,
                        "lid lies behind the globe at {x}/{y}"
                    );
                }
            }
        }
    }
    #[test]
    fn continuous_lids_keep_borders_and_close_on_one_shared_edge() {
        let body = voxy_render::ObjAsset::parse(
            include_str!("../../../assets/characters/blender-female/body.obj"),
            voxy_render::ObjLimits::default(),
        )
        .unwrap();
        let surface = LidSurface::new(body.mesh.vertices(), body.mesh.indices()).unwrap();
        let neutral = surface.mesh(0.).unwrap();
        for step in 0..=10 {
            let mesh = surface.mesh(step as f32 / 10.).unwrap();
            assert_eq!(mesh.indices(), neutral.indices());
            for part in 0..4 {
                for column in 0..COLUMNS {
                    assert_eq!(
                        mesh.vertices()[part * ROWS * COLUMNS + column].position,
                        neutral.vertices()[part * ROWS * COLUMNS + column].position
                    );
                }
            }
            for ids in mesh.indices().chunks_exact(3) {
                let p: [Vec3; 3] = std::array::from_fn(|k| {
                    Vec3::from_array(mesh.vertices()[ids[k] as usize].position)
                });
                let normal = (p[1] - p[0]).cross(p[2] - p[0]);
                assert!(normal.length() > 1e-12);
                assert!(normal.z > 1e-12, "lid surface folds in frontal projection");
            }
            if [0, 5, 10].contains(&step) {
                let triangles: Vec<[Vec3; 3]> = mesh
                    .indices()
                    .chunks_exact(3)
                    .map(|ids| {
                        std::array::from_fn(|k| {
                            Vec3::from_array(mesh.vertices()[ids[k] as usize].position)
                        })
                    })
                    .collect();
                for i in 0..triangles.len() {
                    for j in i + 1..triangles.len() {
                        if triangles[i].iter().any(|p| triangles[j].contains(p)) {
                            continue;
                        }
                        assert!(
                            !crate::female_face::tests::triangles_cross(triangles[i], triangles[j]),
                            "transverse surface intersection {i}/{j} at closure step {step}: {:?} / {:?}",
                            triangles[i],
                            triangles[j]
                        );
                    }
                }
            }
        }
        let closed = surface.mesh(1.).unwrap();
        for side in 0..2 {
            for column in 0..COLUMNS {
                let lower = (side * 2 * ROWS + ROWS - 1) * COLUMNS + column;
                let upper = ((side * 2 + 1) * ROWS + ROWS - 1) * COLUMNS + column;
                assert_eq!(
                    closed.vertices()[lower].position,
                    closed.vertices()[upper].position
                );
            }
        }
    }
}
