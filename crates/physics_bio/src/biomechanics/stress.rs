//! Spatial stress diagnostics on tetrahedral finite-element bodies.
use super::{Body, ElementStress, Stress, columns, det, mm, sub, transpose};

impl Body {
    /// One spatial stress per constant-strain tetrahedron, without nodal smoothing.
    /// # Errors
    /// Rejects invalid positions, inverted elements and stress overflow.
    pub fn stresses_at(&self, positions: &[[f64; 3]]) -> Result<Vec<ElementStress>, &'static str> {
        if positions.len() != self.positions.len()
            || positions.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid stress evaluation positions");
        }
        let pore_pressure = self.pore_fields_at(positions)?.0;
        self.elements
            .iter()
            .enumerate()
            .map(|(index, e)| {
                let [a, b, c, d] = e.nodes.map(|i| positions[i]);
                let f = mm(columns(sub(b, a), sub(c, a), sub(d, a)), e.inv_rest);
                Ok(ElementStress {
                    nodes: e.nodes,
                    reference_volume_m3: e.volume,
                    volume_ratio: det(f),
                    stress: {
                        let r = e.response(f)?;
                        let mut spatial = mm(r.first_piola, transpose(f))
                            .map(|row| row.map(|v| v / r.volume_ratio));
                        for (i, row) in spatial.iter_mut().enumerate() {
                            row[i] -= pore_pressure[index];
                        }
                        Stress::from_cauchy(spatial)?
                    },
                })
            })
            .collect()
    }
}
