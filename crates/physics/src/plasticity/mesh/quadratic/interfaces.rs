//! Quadratic mesh interface ownership, assembly and exposure.
use super::{
    Evaluation, QuadraticBody, QuadraticCohesiveFace, QuadraticCohesiveTrial, QuadraticFace, Vec3,
    dot,
};
impl QuadraticBody {
    /// Attach matched, separately indexed boundary faces before any loading.
    /// Minus winding must point out of its solid into the plus solid.
    /// # Errors
    /// Invalid geometry, histories, winding, ownership or already paired boundary.
    pub fn add_cohesive_interface(
        &mut self,
        minus: [usize; 6],
        plus: [usize; 6],
        material: crate::cohesive::Material,
    ) -> Result<usize, &'static str> {
        if self.positions != self.rest
            || self.cells.iter().any(|c| {
                c.states
                    .iter()
                    .any(|s| *s != crate::plasticity::State::default())
            })
            || self.interfaces.iter().any(|f| {
                f.states()
                    .iter()
                    .any(|s| *s != crate::cohesive::State::default())
            })
            || self.interfaces.len() >= 8192
        {
            return Err("invalid quadratic cohesive insertion state");
        }
        let interface = QuadraticCohesiveFace::new(&self.rest, minus, plus, material)?;
        let faces = self.reference_faces()?;
        let key = |nodes: [usize; 6]| {
            let mut nodes = nodes;
            nodes.sort_unstable();
            nodes
        };
        let minus_face = faces
            .iter()
            .find(|f| key(f.nodes) == key(minus))
            .ok_or("quadratic cohesive minus must be boundary")?;
        if !faces.iter().any(|f| key(f.nodes) == key(plus)) {
            return Err("quadratic cohesive plus must be boundary");
        }
        let a = self.rest[minus[0]];
        let e1 = super::sub(self.rest[minus[1]], a);
        let e2 = super::sub(self.rest[minus[2]], a);
        if dot(crate::plasticity::mesh::cross(e1, e2), minus_face.normal) <= 0. {
            return Err("quadratic cohesive winding must point outward");
        }
        let owner = self
            .cells
            .iter()
            .find(|c| plus[..3].iter().all(|n| c.nodes[..4].contains(n)))
            .ok_or("missing quadratic cohesive owner")?;
        let opposite = owner.nodes[..4]
            .iter()
            .find(|n| !plus[..3].contains(n))
            .ok_or("invalid quadratic cohesive owner")?;
        if dot(super::sub(self.rest[*opposite], a), minus_face.normal) <= 0. {
            return Err("quadratic cohesive solids must lie on opposite sides");
        }
        for other in &self.interfaces {
            let (m, p) = other.sides();
            if [key(m), key(p)]
                .iter()
                .any(|k| *k == key(minus) || *k == key(plus))
            {
                return Err("quadratic cohesive boundary already paired");
            }
        }
        let index = self.interfaces.len();
        self.interfaces.push(interface);
        Ok(index)
    }
    #[must_use]
    pub fn cohesive_interfaces(&self) -> &[QuadraticCohesiveFace] {
        &self.interfaces
    }
    /// Constitutive candidates at the supplied geometry, without committing history.
    /// # Errors
    /// Invalid geometry or cohesive response.
    pub fn cohesive_trials_at(
        &self,
        positions: &[Vec3],
    ) -> Result<Vec<QuadraticCohesiveTrial>, &'static str> {
        if positions.len() != self.rest.len() || positions.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid quadratic cohesive positions");
        }
        self.interfaces
            .iter()
            .map(|f| f.trial_at(positions))
            .collect()
    }
    /// Reference-area boundary faces exposed at current trial damage. A paired
    /// face becomes exposed only when all six cohesive integration points fail.
    /// This is an integration-point topology criterion, not crack-front geometry.
    /// # Errors
    /// Invalid boundary or constitutive response.
    pub fn exposed_faces_at(&self, positions: &[Vec3]) -> Result<Vec<QuadraticFace>, &'static str> {
        let mut hidden = Vec::new();
        for (face, trial) in self
            .interfaces
            .iter()
            .zip(self.cohesive_trials_at(positions)?)
        {
            if trial.quadrature.iter().any(|q| q.damage < 1.) {
                let (mut a, mut b) = face.sides();
                a.sort_unstable();
                b.sort_unstable();
                hidden.extend([a, b]);
            }
        }
        Ok(self
            .reference_faces()?
            .into_iter()
            .filter(|face| {
                let mut nodes = face.nodes;
                nodes.sort_unstable();
                !hidden.contains(&nodes)
            })
            .collect())
    }
    pub(super) fn assemble_cohesive(
        &self,
        x: &[Vec3],
        dofs: &[[Option<usize>; 3]],
        tangent: bool,
        result: &mut Evaluation,
    ) -> Result<(), &'static str> {
        for face in &self.interfaces {
            let trial = face.trial_at(x)?;
            for (f, addition) in result.internal.iter_mut().zip(&trial.internal_n) {
                for axis in 0..3 {
                    f[axis] += addition[axis];
                }
            }
            if tangent {
                let (minus, plus) = face.sides();
                let nodes: Vec<_> = minus.into_iter().chain(plus).collect();
                for &node in &nodes {
                    for axis in 0..3 {
                        let Some(column) = dofs[node][axis] else {
                            continue;
                        };
                        let h = face.derivative_step(x);
                        let mut positive = x.to_vec();
                        let mut negative = x.to_vec();
                        positive[node][axis] += h;
                        negative[node][axis] -= h;
                        let width = positive[node][axis] - negative[node][axis];
                        if !width.is_finite() || width <= 0. {
                            return Err("quadratic cohesive tangent resolution");
                        }
                        let positive = face.trial_at(&positive)?;
                        let negative = face.trial_at(&negative)?;
                        for &row_node in &nodes {
                            for (i, &row) in dofs[row_node].iter().enumerate() {
                                if let Some(row) = row {
                                    result.tangent[row][column] += (positive.internal_n[row_node]
                                        [i]
                                        - negative.internal_n[row_node][i])
                                        / width;
                                }
                            }
                        }
                    }
                }
            }
            result.interfaces.push(trial.candidate);
        }
        Ok(())
    }
}
