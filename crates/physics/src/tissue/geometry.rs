//! Rounded volumetric geometry for the existing tissue owner.
use super::{Material, Tissue, V, finite, volume};
use std::collections::BTreeSet;

/// Creates an inscribed ellipsoid with a conforming tetrahedral volume mesh.
/// Refinement bisects boundary edges on the unit sphere before applying radii.
/// Node 0 is the center; nodes 1..=6 are +X, -X, +Y, -Y, +Z, -Z.
/// These identities remain stable across refinements. Additional nodes are free.
/// Lumped nodal masses use density times incident cell volume divided by four.
/// Pinned nodes retain geometry but have zero inverse mass in the solver.
/// Material compliance is used as supplied; it is not resolution calibrated.
/// # Errors
/// Rejects invalid dimensions, density, pins, refinement above three, overflow,
/// unrepresentable mass, degenerate cells and invalid material parameters.
pub fn ellipsoid(
    center: V,
    radii: V,
    density: f64,
    refinement: u32,
    pinned_axes: &[usize],
    material: Material,
) -> Result<Tissue, &'static str> {
    if !finite(center)
        || radii.iter().any(|r| !r.is_finite() || *r <= 0.)
        || !density.is_finite()
        || density <= 0.
        || refinement > 3
        || pinned_axes.iter().any(|&i| !(1..=6).contains(&i))
    {
        return Err("invalid tissue ellipsoid");
    }
    let mesh = crate::biomechanics::TetraMesh::ellipsoid(center, radii, refinement)?;
    let positions = mesh.points;
    let cells = mesh.cells;
    let mut mass = vec![0.; positions.len()];
    let mut edges = BTreeSet::new();
    for &cell in &cells {
        let quarter_mass = volume(&positions, cell).abs() * density * 0.25;
        if !quarter_mass.is_finite() || quarter_mass <= 0. {
            return Err("unrepresentable tissue ellipsoid mass");
        }
        for i in cell {
            mass[i] += quarter_mass;
        }
        for a in 0..4 {
            for b in a + 1..4 {
                edges.insert([cell[a].min(cell[b]), cell[a].max(cell[b])]);
            }
        }
    }
    if !mass.iter().sum::<f64>().is_finite() {
        return Err("tissue ellipsoid total mass overflow");
    }
    let mut inverse_mass = Vec::with_capacity(mass.len());
    for (i, m) in mass.into_iter().enumerate() {
        if !m.is_finite() || m <= 0. || !(1. / m).is_finite() || 1. / m == 0. {
            return Err("unrepresentable tissue ellipsoid mass");
        }
        inverse_mass.push(if pinned_axes.contains(&i) { 0. } else { 1. / m });
    }
    Tissue::new(
        positions,
        inverse_mass,
        edges.into_iter().map(|e| (e, false)).collect(),
        cells,
        material,
    )
}
