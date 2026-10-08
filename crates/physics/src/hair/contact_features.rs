use super::*;

/// Geometry-welded topology; numerical sums refresh without allocating per pose.
#[derive(Clone, Debug)]
pub(super) struct FeatureNormals {
    vertices: Vec<[usize; 3]>,
    edges: Vec<[usize; 3]>,
    representatives: Vec<usize>,
    vertex_normals: Vec<V>,
    edge_normals: Vec<V>,
}

impl FeatureNormals {
    pub(super) fn new(triangles: &[Triangle]) -> Result<Self, &'static str> {
        let mut welded = HashMap::new();
        let mut representatives = Vec::new();
        let mut vertices = Vec::with_capacity(triangles.len());
        let mut edges = Vec::with_capacity(triangles.len());
        let mut edge_map: HashMap<[usize; 2], (usize, usize, i32)> = HashMap::new();
        for triangle in triangles {
            let ids = std::array::from_fn(|corner| {
                // Signed zero is the same geometric coordinate.
                let key = triangle.p[corner].map(|x| if x == 0. { 0 } else { x.to_bits() });
                *welded.entry(key).or_insert_with(|| {
                    let id = representatives.len();
                    representatives.push(triangle.ids[corner]);
                    id
                })
            });
            let edge_ids = std::array::from_fn(|corner| {
                let (a, b) = (ids[corner], ids[(corner + 1) % 3]);
                let key = [a.min(b), a.max(b)];
                let next = edge_map.len();
                let entry = edge_map.entry(key).or_insert((next, 0, 0));
                entry.1 += 1;
                entry.2 += if a < b { 1 } else { -1 };
                entry.0
            });
            vertices.push(ids);
            edges.push(edge_ids);
        }
        if edge_map
            .values()
            .any(|&(_, count, winding)| count != 2 || winding != 0)
        {
            return Err("feature normals require closed consistently oriented edges");
        }
        // Closed edges can still join disconnected face fans at a welded vertex.
        // Such a vertex has no single manifold pseudonormal (e.g. touching shells).
        let mut edge_faces = vec![[usize::MAX; 2]; edge_map.len()];
        let mut first_face = vec![usize::MAX; representatives.len()];
        let mut incident_count = vec![0usize; representatives.len()];
        for (face, ids) in vertices.iter().enumerate() {
            for &vertex in ids {
                first_face[vertex] = face;
                incident_count[vertex] += 1;
            }
            for &edge in &edges[face] {
                let slot = if edge_faces[edge][0] == usize::MAX {
                    0
                } else {
                    1
                };
                edge_faces[edge][slot] = face;
            }
        }
        let mut visited = vec![usize::MAX; triangles.len()];
        let mut pending = Vec::new();
        for vertex in 0..representatives.len() {
            pending.push(first_face[vertex]);
            let mut count = 0;
            while let Some(face) = pending.pop() {
                if visited[face] == vertex {
                    continue;
                }
                visited[face] = vertex;
                count += 1;
                for corner in 0..3 {
                    if vertices[face][corner] == vertex
                        || vertices[face][(corner + 1) % 3] == vertex
                    {
                        let adjacent = edge_faces[edges[face][corner]];
                        pending.push(if adjacent[0] == face {
                            adjacent[1]
                        } else {
                            adjacent[0]
                        });
                    }
                }
            }
            if count != incident_count[vertex] {
                return Err("feature normals require manifold vertex fans");
            }
        }
        let mut result = Self {
            vertex_normals: vec![[0.; 3]; representatives.len()],
            edge_normals: vec![[0.; 3]; edge_map.len()],
            vertices,
            edges,
            representatives,
        };
        result.refresh(triangles);
        Ok(result)
    }

    pub(super) fn validate_refit(&self, triangles: &[Triangle], positions: &[V]) -> bool {
        triangles.iter().enumerate().all(|(face, triangle)| {
            let points = triangle.ids.map(|i| positions[i]);
            let area_normal = cross(sub(points[1], points[0]), sub(points[2], points[0]));
            finite(area_normal)
                && len(area_normal) >= 1e-14
                && (0..3).all(|corner| {
                    positions[triangle.ids[corner]]
                        == positions[self.representatives[self.vertices[face][corner]]]
                })
        })
    }

    pub(super) fn refresh(&mut self, triangles: &[Triangle]) {
        self.vertex_normals.fill([0.; 3]);
        self.edge_normals.fill([0.; 3]);
        for (face, triangle) in triangles.iter().enumerate() {
            for corner in 0..3 {
                let a = sub(triangle.p[(corner + 1) % 3], triangle.p[corner]);
                let b = sub(triangle.p[(corner + 2) % 3], triangle.p[corner]);
                let angle = len(cross(a, b)).atan2(dot(a, b));
                let id = self.vertices[face][corner];
                self.vertex_normals[id] = add(self.vertex_normals[id], mul(triangle.normal, angle));
                let edge = self.edges[face][corner];
                self.edge_normals[edge] = add(self.edge_normals[edge], triangle.normal);
            }
        }
    }

    pub(super) fn normal(&self, face: usize, feature: ClosestFeature, fallback: V) -> V {
        let normal = match feature {
            ClosestFeature::Face => fallback,
            ClosestFeature::Vertex(corner) => self.vertex_normals[self.vertices[face][corner]],
            ClosestFeature::Edge(a, b) => {
                let corner = match (a, b) {
                    (0, 1) => 0,
                    (1, 2) => 1,
                    (0, 2) => 2,
                    _ => unreachable!(),
                };
                self.edge_normals[self.edges[face][corner]]
            }
        };
        if len(normal) > 1e-14 {
            unit(normal)
        } else {
            fallback
        }
    }
}
