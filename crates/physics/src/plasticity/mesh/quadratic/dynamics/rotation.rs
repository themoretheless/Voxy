use super::QuadraticDynamics;
use crate::plasticity::mesh::QuadraticFragmentRotation;
impl QuadraticDynamics {
    /// Centered spin and consistent inertia for every accepted deformable piece.
    /// Diagnostic projection only: no velocity, history or geometry mutation.
    pub fn fragment_rotations(&self) -> Result<Vec<QuadraticFragmentRotation>, &'static str> {
        self.fragments()?
            .into_iter()
            .map(|fragment| {
                super::super::fragments::project_rotation(
                    fragment,
                    &self.mass,
                    &self.body.positions,
                    &self.velocities,
                )
            })
            .collect()
    }
}

impl super::FiniteQuadraticDynamics {
    /// Same consistent inertia/spin projection for finite-elastic geometry.
    /// This diagnostic is independent of the constitutive law.
    pub fn fragment_rotations(&self) -> Result<Vec<QuadraticFragmentRotation>, &'static str> {
        self.inner.fragment_rotations()
    }
}
