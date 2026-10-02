//! Objective finite-deformation hyperelastic energy/force quadrature.
use super::{QuadraticBody, State, Vec3, deformation_at, dot};
use crate::biomechanics::{Material, Stress};
#[derive(Clone, Debug)]
pub struct FiniteElasticEvaluation {
    pub energy_j: f64,
    /// Energy gradient in N; the restoring force on a node is its negative.
    pub internal_n: Vec<Vec3>,
    /// Spatial Cauchy stress at all four integration points per cell.
    pub stresses: Vec<[Stress; 4]>,
}
impl QuadraticBody {
    /// Evaluate a supplied finite-deformation material per cell, without changing
    /// geometry or history. Reuses ten-node interpolation and four-point integration.
    /// Pure rotations are stress-free; forces and stresses transform objectively.
    /// Activation is zero. This elastic alternative cannot reinterpret J2 history.
    /// # Errors
    /// Wrong input counts, nonfinite/inverted geometry, plastic history or overflow.
    pub fn finite_elastic_at(
        &self,
        positions: &[Vec3],
        materials: &[Material],
    ) -> Result<FiniteElasticEvaluation, &'static str> {
        if positions.len() != self.rest.len()
            || materials.len() != self.cells.len()
            || positions.iter().flatten().any(|v| !v.is_finite())
            || self
                .cells
                .iter()
                .any(|c| c.states.iter().any(|s| *s != State::default()))
        {
            return Err("invalid quadratic finite-elastic input or plastic history");
        }
        let mut result = FiniteElasticEvaluation {
            energy_j: 0.,
            internal_n: vec![[0.; 3]; positions.len()],
            stresses: Vec::new(),
        };
        for (cell, material) in self.cells.iter().zip(materials) {
            let mut stresses = Vec::with_capacity(4);
            for g in &cell.gradients {
                let f = deformation_at(cell, g, positions, &self.rest)?;
                let response = material.response(f, 0.)?;
                let weight = cell.volume / 4.;
                result.energy_j += weight * response.energy_density;
                for (local, &node) in cell.nodes.iter().enumerate() {
                    for axis in 0..3 {
                        result.internal_n[node][axis] +=
                            weight * dot(response.first_piola[axis], g[local]);
                    }
                }
                stresses.push(material.stress(f, 0.)?);
            }
            result.stresses.push(
                stresses
                    .try_into()
                    .map_err(|_| "invalid finite stress count")?,
            );
        }
        if !result.energy_j.is_finite()
            || result.internal_n.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("quadratic finite-elastic overflow");
        }
        Ok(result)
    }
}
