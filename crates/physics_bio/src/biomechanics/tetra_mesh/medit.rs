//! Geometry-only ASCII Medit volume interchange emitted by offline meshers.
use super::*;

impl TetraMesh {
    /// Derive an outward boundary from caller-supplied positively oriented cells.
    /// All ordinary volume, manifold and overlap admission checks remain active.
    /// # Errors
    /// Invalid indices, orientation, geometry or existing resource limits.
    pub fn from_tetrahedra(
        points: Vec<Vec3>,
        cells: Vec<[usize; 4]>,
    ) -> Result<Self, &'static str> {
        if points.len() > 1_000_000 || cells.len() > 250_000 {
            return Err("tetrahedral resource limit");
        }
        let mut faces = BTreeMap::<[usize; 3], Vec<[usize; 3]>>::new();
        for &[a, b, c, d] in &cells {
            for face in [[b, c, d], [a, d, c], [a, b, d], [a, c, b]] {
                let mut key = face;
                key.sort_unstable();
                let entry = faces.entry(key).or_default();
                entry.push(face);
                if entry.len() > 2 {
                    return Err("nonmanifold tetrahedral faces");
                }
            }
        }
        let boundary = faces
            .into_values()
            .filter(|f| f.len() == 1)
            .map(|f| f[0])
            .collect();
        let mesh = Self {
            points,
            cells,
            boundary,
        };
        mesh.validate()?;
        Ok(mesh)
    }

    /// Read the ASCII Medit v1 volume profile emitted by fTetWild: Dimension 3,
    /// Vertices, empty Triangles, Tetrahedra, End. Coordinates are metres.
    /// One-based indices are converted; reversed tetrahedral ordering is normalized
    /// without moving vertices. Boundary faces are derived from the volume cells.
    /// Only zero references are admitted: material/attachment assignment remains
    /// explicit in the caller, and unsupported region labels are never discarded.
    /// # Errors
    /// Unsupported sections/references, malformed input, nonfinite coordinates,
    /// degenerate/overlapping/nonmanifold cells or existing admission limits.
    pub fn from_medit_volume(text: &str) -> Result<Self, &'static str> {
        if text.len() > 64 * 1024 * 1024 {
            return Err("Medit tissue input byte limit");
        }
        let mut tokens = text
            .lines()
            .flat_map(|line| line.split('#').next().unwrap_or("").split_whitespace());
        let mut next = || tokens.next().ok_or("truncated Medit tissue input");
        for expected in ["MeshVersionFormatted", "1", "Dimension", "3", "Vertices"] {
            if next()? != expected {
                return Err("unsupported Medit tissue volume profile");
            }
        }
        let np = next()?
            .parse::<usize>()
            .map_err(|_| "invalid Medit vertex count")?;
        if !(4..=1_000_000).contains(&np) {
            return Err("tetrahedral resource limit");
        }
        let mut points = Vec::with_capacity(np);
        for _ in 0..np {
            let mut point = [0.; 3];
            for coordinate in &mut point {
                *coordinate = next()?
                    .parse::<f64>()
                    .map_err(|_| "invalid Medit coordinate")?;
                if !coordinate.is_finite() {
                    return Err("nonfinite tetrahedral point");
                }
            }
            if next()?.parse::<i32>().ok() != Some(0) {
                return Err("Medit tissue references require explicit mapping");
            }
            points.push(point);
        }
        for expected in ["Triangles", "0", "Tetrahedra"] {
            if next()? != expected {
                return Err("unsupported Medit tissue volume profile");
            }
        }
        let nc = next()?
            .parse::<usize>()
            .map_err(|_| "invalid Medit cell count")?;
        if !(1..=250_000).contains(&nc) {
            return Err("tetrahedral resource limit");
        }
        let mut cells = Vec::with_capacity(nc);
        for _ in 0..nc {
            let mut cell = [0; 4];
            for node in &mut cell {
                *node = next()?
                    .parse::<usize>()
                    .ok()
                    .and_then(|n| n.checked_sub(1))
                    .filter(|&n| n < np)
                    .ok_or("invalid tetrahedral node index")?;
            }
            if next()?.parse::<i32>().ok() != Some(0) {
                return Err("Medit tissue references require explicit mapping");
            }
            let [a, b, c, d] = cell.map(|n| points[n]);
            if det(columns(sub(b, a), sub(c, a), sub(d, a))) < 0. {
                cell.swap(1, 2);
            }
            cells.push(cell);
        }
        if next()? != "End" || tokens.next().is_some() {
            return Err("unexpected Medit tissue data");
        }
        Self::from_tetrahedra(points, cells)
    }
}
