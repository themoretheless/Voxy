//! Audited anatomical tetrahedral mesh interchange, VXTM version 1, metres.
use super::{Body, Material, Vec3, columns, cross, det, dot, sub};
use std::collections::BTreeMap;
mod convex_surface;
mod ellipsoid;
mod lattice;
mod manifold;
mod medit;
mod overlap;
#[derive(Clone, Debug)]
pub struct TetraMesh {
    pub points: Vec<Vec3>,
    pub cells: Vec<[usize; 4]>,
    pub boundary: Vec<[usize; 3]>,
}

#[cfg(test)]
mod refinement_tests {
    use super::*;
    fn boundary(cells: &[[usize; 4]]) -> Vec<[usize; 3]> {
        let mut faces: BTreeMap<[usize; 3], Vec<[usize; 3]>> = BTreeMap::new();
        for &[a, b, c, d] in cells {
            for face in [[b, c, d], [a, d, c], [a, b, d], [a, c, b]] {
                let mut key = face;
                key.sort_unstable();
                faces.entry(key).or_default().push(face);
            }
        }
        faces
            .into_values()
            .filter(|f| f.len() == 1)
            .map(|f| f[0])
            .collect()
    }
    fn volume(mesh: &TetraMesh) -> f64 {
        mesh.cells
            .iter()
            .map(|cell| {
                let [a, b, c, d] = cell.map(|i| mesh.points[i]);
                det(columns(sub(b, a), sub(c, a), sub(d, a))) / 6.
            })
            .sum()
    }
    #[test]
    fn shared_faces_are_conforming_and_volume_is_preserved() {
        let cells = vec![[0, 1, 2, 3], [0, 2, 1, 4]];
        let mesh = TetraMesh {
            points: vec![
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., 1.],
                [0., 0., -1.],
            ],
            boundary: boundary(&cells),
            cells,
        };
        let refined = mesh.refined_once().unwrap();
        let (_, parents) = mesh.refined_once_with_parents().unwrap();
        let coarse_pins = [true, true, true, false, false];
        let fine_pins: Vec<_> = parents
            .iter()
            .map(|&[a, b]| coarse_pins[a] && coarse_pins[b])
            .collect();
        let values: [Vec3; 5] = [[0.; 3], [0.; 3], [0.; 3], [1., 2., 3.], [-1., 2., 1.]];
        for (i, &[a, b]) in parents.iter().enumerate() {
            for axis in 0..3 {
                assert_eq!(
                    refined.points[i][axis],
                    mesh.points[a][axis].midpoint(mesh.points[b][axis])
                );
                if fine_pins[i] {
                    assert_eq!(values[a][axis].midpoint(values[b][axis]), 0.);
                }
            }
        }
        assert_eq!(refined.points.len(), 14);
        assert_eq!(refined.cells.len(), 16);
        assert_eq!(refined.boundary.len(), 24);
        assert_eq!(&refined.points[..mesh.points.len()], mesh.points);
        assert!((volume(&mesh) - volume(&refined)).abs() < 1e-14);
        let second = refined.refined_once().unwrap();
        assert_eq!(second.cells.len(), 128);
        assert_eq!(second.boundary.len(), 96);
        assert!((volume(&mesh) - volume(&second)).abs() < 1e-14);
        let weights = mesh.reference_patch_weights(&[0]).unwrap();
        let finer = refined.reference_patch_weights(&[0, 1, 2, 3]).unwrap();
        assert!((weights.iter().sum::<f64>() - finer.iter().sum::<f64>()).abs() < 1e-14);
        for axis in 0..3 {
            let a = weights
                .iter()
                .zip(&mesh.points)
                .map(|(w, p)| w * p[axis])
                .sum::<f64>();
            let b = finer
                .iter()
                .zip(&refined.points)
                .map(|(w, p)| w * p[axis])
                .sum::<f64>();
            assert!((a - b).abs() < 1e-14);
        }
        assert!(mesh.reference_patch_weights(&[0, 0]).is_err());
        assert!(
            mesh.reference_patch_weights(&[mesh.boundary.len()])
                .is_err()
        );
    }
    #[test]
    fn atlas_refinement_preserves_reference_volume_and_boundary() {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/anatomy/hra-female/tetrahedra/right-ovary.vxtet"
        ))
        .unwrap();
        let mesh = TetraMesh::from_bytes(&bytes).unwrap();
        let refined = mesh.refined_once().unwrap();
        assert_eq!(refined.cells.len(), 8 * mesh.cells.len());
        assert_eq!(refined.boundary.len(), 4 * mesh.boundary.len());
        assert!((volume(&refined) / volume(&mesh) - 1.).abs() < 1e-12);
        println!(
            "atlas refined points={} cells={} faces={} volume_m3={:.12e}",
            refined.points.len(),
            refined.cells.len(),
            refined.boundary.len(),
            volume(&refined)
        );
    }
}
fn raw<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], &'static str> {
    let end = offset.checked_add(N).ok_or("tetrahedral offset overflow")?;
    bytes
        .get(offset..end)
        .ok_or("truncated tetrahedral value")?
        .try_into()
        .map_err(|_| "invalid tetrahedral value")
}
impl TetraMesh {
    /// Reference-area lumped weights for constant vector traction on selected
    /// boundary triangles. Units m²; force_i = weight_i * traction (Pa).
    /// Preserves the resultant and first moment of the continuous patch load.
    pub fn reference_patch_weights(&self, faces: &[usize]) -> Result<Vec<f64>, &'static str> {
        self.validate()?;
        if faces.is_empty() {
            return Err("empty surface load patch");
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut weights = vec![0.; self.points.len()];
        for &index in faces {
            if index >= self.boundary.len() || !seen.insert(index) {
                return Err("invalid or duplicate patch face");
            }
            let face = self.boundary[index];
            let [a, b, c] = face.map(|i| self.points[i]);
            let n = cross(sub(b, a), sub(c, a));
            let area = 0.5 * dot(n, n).sqrt();
            if !area.is_finite() || area <= 0. {
                return Err("invalid reference patch area");
            }
            for i in face {
                weights[i] += area / 3.;
            }
        }
        if weights.iter().any(|w| !w.is_finite()) {
            return Err("surface load weight overflow");
        }
        Ok(weights)
    }
    /// Conforming 1-to-8 midpoint refinement, preserving the piecewise-linear
    /// boundary exactly. Does not transfer tissue history or boundary forces.
    pub fn refined_once(&self) -> Result<Self, &'static str> {
        self.refined_once_with_parents().map(|(mesh, _)| mesh)
    }
    /// Parent vertices for exact linear field prolongation: each new node is
    /// their midpoint; retained vertices have two identical parent indices.
    pub fn refined_once_with_parents(&self) -> Result<(Self, Vec<[usize; 2]>), &'static str> {
        self.validate()?;
        if self.cells.len() > 250_000 / 8 {
            return Err("refined tetrahedral resource limit");
        }
        let mut points = self.points.clone();
        let mut parents: Vec<_> = (0..points.len()).map(|i| [i, i]).collect();
        let mut edges = BTreeMap::new();
        for cell in &self.cells {
            for i in 0..4 {
                for j in i + 1..4 {
                    let mut edge = [cell[i], cell[j]];
                    edge.sort_unstable();
                    edges.entry(edge).or_insert_with(|| {
                        let index = points.len();
                        parents.push(edge);
                        points.push(std::array::from_fn(|k| {
                            self.points[edge[0]][k].midpoint(self.points[edge[1]][k])
                        }));
                        index
                    });
                }
            }
        }
        if points.len() > 1_000_000 {
            return Err("refined tetrahedral node limit");
        }
        let midpoint = |a: usize, b: usize| {
            let mut edge = [a, b];
            edge.sort_unstable();
            edges[&edge]
        };
        let mut cells = Vec::with_capacity(self.cells.len() * 8);
        for &[a, b, c, d] in &self.cells {
            let ab = midpoint(a, b);
            let ac = midpoint(a, c);
            let ad = midpoint(a, d);
            let bc = midpoint(b, c);
            let bd = midpoint(b, d);
            let cd = midpoint(c, d);
            // Central octahedron split along ab--cd; boundary subdivision is
            // identical regardless of this internal diagonal choice.
            for mut cell in [
                [a, ab, ac, ad],
                [b, ab, bc, bd],
                [c, ac, bc, cd],
                [d, ad, bd, cd],
                [ab, cd, ac, ad],
                [ab, cd, ad, bd],
                [ab, cd, bd, bc],
                [ab, cd, bc, ac],
            ] {
                let [p, q, r, s] = cell.map(|i| points[i]);
                if det(columns(sub(q, p), sub(r, p), sub(s, p))) < 0. {
                    cell.swap(1, 2);
                }
                cells.push(cell);
            }
        }
        let mut boundary = Vec::with_capacity(self.boundary.len() * 4);
        for &[a, b, c] in &self.boundary {
            let ab = midpoint(a, b);
            let bc = midpoint(b, c);
            let ca = midpoint(c, a);
            boundary.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
        }
        let refined = Self {
            points,
            cells,
            boundary,
        };
        refined.validate_topology()?;
        Ok((refined, parents))
    }
    /// # Errors
    /// Rejects unknown format, truncation/trailing bytes, excessive counts,
    /// nonfinite/inverted/degenerate geometry and invalid boundary topology.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 20 || bytes.len() > 256 * 1024 * 1024 || &bytes[..4] != b"VXTM" {
            return Err("invalid tetrahedral header");
        }
        let u32_at = |offset| raw(bytes, offset).map(|value| u32::from_le_bytes(value) as usize);
        if u32_at(4)? != 1 {
            return Err("unsupported tetrahedral version");
        }
        let (np, nc, nf) = (u32_at(8)?, u32_at(12)?, u32_at(16)?);
        if !(4..=1_000_000).contains(&np) || !(1..=250_000).contains(&nc) || nf > 4 * nc {
            return Err("tetrahedral resource limit");
        }
        let expected = 20 + 24 * np + 16 * nc + 12 * nf;
        if bytes.len() != expected {
            return Err("invalid tetrahedral payload length");
        }
        let mut cursor = 20;
        let mut points = Vec::with_capacity(np);
        for _ in 0..np {
            let mut p = [0.; 3];
            for value in &mut p {
                *value = f64::from_le_bytes(raw(bytes, cursor)?);
                cursor += 8;
            }
            if p.iter().any(|v| !v.is_finite()) {
                return Err("nonfinite tetrahedral point");
            }
            points.push(p);
        }
        let mut index = || {
            let value = u32_at(cursor)?;
            cursor += 4;
            Ok::<usize, &'static str>(value)
        };
        let mut cells = Vec::with_capacity(nc);
        for _ in 0..nc {
            cells.push([index()?, index()?, index()?, index()?]);
        }
        let mut boundary = Vec::with_capacity(nf);
        for _ in 0..nf {
            boundary.push([index()?, index()?, index()?]);
        }
        let mesh = Self {
            points,
            cells,
            boundary,
        };
        mesh.validate()?;
        Ok(mesh)
    }
    /// Validate resource bounds, finite coordinates, topology and cell overlap.
    /// # Errors
    /// Invalid geometry or existing VXTM resource limits.
    pub fn validate(&self) -> Result<(), &'static str> {
        let (np, nc, nf) = (self.points.len(), self.cells.len(), self.boundary.len());
        if !(4..=1_000_000).contains(&np) || !(1..=250_000).contains(&nc) || nf > 4 * nc {
            return Err("tetrahedral resource limit");
        }
        if self.points.iter().flatten().any(|v| !v.is_finite()) {
            return Err("nonfinite tetrahedral point");
        }
        self.validate_topology()?;
        self.validate_boundary_manifold()?;
        self.reject_overlapping_cells()
    }
    /// Serialize an admitted mesh to the existing VXTM v1 interchange format.
    /// # Errors
    /// Invalid topology, nonfinite points or existing interchange resource limits.
    pub fn to_bytes(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()?;
        let (np, nc, nf) = (self.points.len(), self.cells.len(), self.boundary.len());
        let mut bytes = Vec::with_capacity(20 + 24 * np + 16 * nc + 12 * nf);
        bytes.extend_from_slice(b"VXTM");
        for value in [1, np, nc, nf] {
            bytes.extend_from_slice(&(value as u32).to_le_bytes());
        }
        for &point in &self.points {
            for value in point {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        for cell in &self.cells {
            for &index in cell {
                bytes.extend_from_slice(&(index as u32).to_le_bytes());
            }
        }
        for face in &self.boundary {
            for &index in face {
                bytes.extend_from_slice(&(index as u32).to_le_bytes());
            }
        }
        Ok(bytes)
    }
    pub(super) fn validate_topology(&self) -> Result<(), &'static str> {
        let points = &self.points;
        let cells = &self.cells;
        let boundary = &self.boundary;
        let np = points.len();
        let mut faces: BTreeMap<[usize; 3], Vec<[usize; 3]>> = BTreeMap::new();
        for cell in cells {
            if cell.iter().any(|i| *i >= np) {
                return Err("invalid tetrahedral node index");
            }
            let [a, b, c, d] = cell.map(|i| points[i]);
            let volume = det(columns(sub(b, a), sub(c, a), sub(d, a))) / 6.;
            if !volume.is_finite() || volume <= 1e-15 {
                return Err("nonpositive tetrahedral volume");
            }
            let [a, b, c, d] = *cell;
            for face in [[b, c, d], [a, d, c], [a, b, d], [a, c, b]] {
                let mut key = face;
                key.sort_unstable();
                let entries = faces.entry(key).or_default();
                entries.push(face);
                if entries.len() > 2 {
                    return Err("nonmanifold tetrahedral faces");
                }
            }
        }
        let cyclic =
            |a: [usize; 3], b: [usize; 3]| (0..3).any(|j| (0..3).all(|i| a[i] == b[(i + j) % 3]));
        let mut exterior = BTreeMap::new();
        for (key, adj) in faces {
            if adj.len() == 2 && cyclic(adj[0], adj[1]) {
                return Err("inconsistent tetrahedral interface orientation");
            }
            if adj.len() == 1 {
                exterior.insert(key, adj[0]);
            }
        }
        for face in boundary {
            let mut key = *face;
            key.sort_unstable();
            let expected = exterior
                .remove(&key)
                .ok_or("invalid or duplicate tetrahedral boundary face")?;
            if !cyclic(*face, expected) {
                return Err("inward tetrahedral boundary face");
            }
        }
        if !exterior.is_empty() {
            return Err("incomplete tetrahedral boundary");
        }
        Ok(())
    }
    /// Caller supplies anatomical anchoring and constitutive law explicitly.
    /// # Errors
    /// Rejects invalid material, pin count or mechanically invalid mesh.
    pub fn into_body(self, pinned: Vec<bool>, material: &Material) -> Result<Body, &'static str> {
        self.validate()?;
        Body::new(
            self.points,
            pinned,
            self.cells
                .into_iter()
                .map(|c| (c, material.clone()))
                .collect(),
        )
    }
}
