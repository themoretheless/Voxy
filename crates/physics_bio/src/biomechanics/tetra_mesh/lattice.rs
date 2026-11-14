//! Explicit axis-aligned cell union, independent of world voxels and materials.
use super::*;
use std::collections::BTreeSet;
impl TetraMesh {
    /// Conforming six-tetrahedron split of each authored occupied lattice cell.
    /// Coordinates are metres. Cells are canonically sorted; shared lattice nodes
    /// have one index and all cubes use the same face diagonals. No smoothing,
    /// anatomical inference, mass, support placement or constitutive law is added.
    /// # Errors
    /// Empty/duplicate cells, invalid dimensions, coordinate/resource overflow,
    /// or geometry outside the existing VXTM admission bounds.
    pub fn from_lattice_cells(
        origin: Vec3,
        spacing: Vec3,
        occupied: &[[u32; 3]],
    ) -> Result<Self, &'static str> {
        if origin.iter().any(|v| !v.is_finite())
            || spacing.iter().any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("invalid tissue lattice dimensions");
        }
        if occupied.is_empty() || occupied.len() > 250_000 / 6 {
            return Err("tissue lattice cell limit");
        }
        let ordered: BTreeSet<_> = occupied.iter().copied().collect();
        if ordered.len() != occupied.len() {
            return Err("duplicate tissue lattice cell");
        }
        let mut indices = BTreeMap::new();
        let mut points = Vec::new();
        let mut cells = Vec::with_capacity(6 * occupied.len());
        for base in ordered {
            for permutation in [
                [0, 1, 2],
                [0, 2, 1],
                [1, 0, 2],
                [1, 2, 0],
                [2, 0, 1],
                [2, 1, 0],
            ] {
                let mut corner = base;
                let mut tet = [0; 4];
                for step in 0..4 {
                    if step > 0 {
                        let axis = permutation[step - 1];
                        corner[axis] = corner[axis]
                            .checked_add(1)
                            .ok_or("tissue lattice coordinate overflow")?;
                    }
                    tet[step] = *indices.entry(corner).or_insert_with(|| {
                        let index = points.len();
                        points.push(std::array::from_fn(|axis| {
                            origin[axis] + spacing[axis] * f64::from(corner[axis])
                        }));
                        index
                    });
                }
                let [a, b, c, d] = tet.map(|i| points[i]);
                if det(columns(sub(b, a), sub(c, a), sub(d, a))) < 0. {
                    tet.swap(1, 2);
                }
                cells.push(tet);
            }
        }
        Self::from_tetrahedra(points, cells)
    }
}
