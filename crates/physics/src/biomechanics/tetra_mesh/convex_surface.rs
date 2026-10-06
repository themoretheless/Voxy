//! Conforming radial tetrahedra for explicitly authored convex tissue boundaries.
use super::super::{Vec3, cross, dot, sub};
use super::TetraMesh;
use std::collections::{BTreeMap, BTreeSet};

impl TetraMesh {
    /// Fill a closed, outward-oriented convex triangle surface from an interior point.
    /// Original surface vertices and indices are retained; the interior node is appended.
    /// Each source triangle owns one tetrahedron. No convex hull, welding, extrapolation,
    /// anatomical fit, material, support or mass is inferred.
    ///
    /// # Errors
    /// Nonfinite/duplicate/unused points, excessive validation work, invalid topology,
    /// nonconvex/inward boundaries, a noninterior center or existing cell-volume limits.
    /// Half-space checks use f64 without a permissive geometry tolerance; this is not
    /// an exact-arithmetic convexity certificate.
    pub fn from_convex_surface(
        mut points: Vec<Vec3>,
        boundary: Vec<[usize; 3]>,
        interior: Vec3,
    ) -> Result<Self, &'static str> {
        if points.len() < 4
            || points.len() >= 1_000_000
            || boundary.len() < 4
            || boundary.len() > 250_000
            || points
                .len()
                .checked_mul(boundary.len())
                .is_none_or(|n| n > 16_000_000)
        {
            return Err("convex tissue surface resource limit");
        }
        if points
            .iter()
            .flatten()
            .chain(&interior)
            .any(|x| !x.is_finite())
        {
            return Err("nonfinite convex tissue surface");
        }
        let key = |p: Vec3| p.map(|x| if x == 0. { 0 } else { x.to_bits() });
        let mut unique = BTreeSet::new();
        if points.iter().any(|&p| !unique.insert(key(p))) {
            return Err("duplicate convex tissue surface point");
        }
        let mut used = vec![false; points.len()];
        let mut edges = BTreeMap::<[usize; 2], Vec<[usize; 2]>>::new();
        for &face in &boundary {
            if face.iter().any(|&i| i >= points.len())
                || face[0] == face[1]
                || face[1] == face[2]
                || face[2] == face[0]
            {
                return Err("invalid convex tissue surface face");
            }
            let [a, b, c] = face.map(|i| points[i]);
            let normal = cross(sub(b, a), sub(c, a));
            let center_side = dot(normal, sub(interior, a));
            if !center_side.is_finite() || center_side >= 0. {
                return Err("convex tissue center is not strictly interior");
            }
            for (index, &point) in points.iter().enumerate() {
                if face.contains(&index) {
                    continue;
                }
                let side = dot(normal, sub(point, a));
                if !side.is_finite() || side > 0. {
                    return Err("tissue surface is not convex");
                }
            }
            for vertex in face {
                used[vertex] = true;
            }
            for edge in [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]] {
                let mut edge_key = edge;
                edge_key.sort_unstable();
                edges.entry(edge_key).or_default().push(edge);
            }
        }
        if used.iter().any(|&v| !v)
            || edges
                .values()
                .any(|e| e.len() != 2 || e[0] != [e[1][1], e[1][0]])
            || points.len() + boundary.len() != edges.len() + 2
        {
            return Err("convex tissue boundary is not a closed sphere");
        }
        let mut adjacency = vec![Vec::new(); points.len()];
        for &[a, b] in edges.keys() {
            adjacency[a].push(b);
            adjacency[b].push(a);
        }
        let mut seen = vec![false; points.len()];
        let mut stack = vec![0];
        while let Some(node) = stack.pop() {
            if seen[node] {
                continue;
            }
            seen[node] = true;
            stack.extend(adjacency[node].iter().copied());
        }
        if seen.iter().any(|&v| !v) {
            return Err("disconnected convex tissue boundary");
        }
        let center = points.len();
        let cells = boundary
            .iter()
            .map(|&[a, b, c]| [center, a, b, c])
            .collect();
        points.push(interior);
        let mesh = Self {
            points,
            cells,
            boundary,
        };
        mesh.validate_topology()?;
        Ok(mesh)
    }
}
