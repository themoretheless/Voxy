//! Preinserted zero-thickness triangular interfaces, with 3-point quadrature.
use super::{Body, Vec3, cross, dot, sub};
use crate::cohesive::{Material, Response, State};
const WEIGHTS: [[f64; 3]; 3] = [
    [2. / 3., 1. / 6., 1. / 6.],
    [1. / 6., 2. / 3., 1. / 6.],
    [1. / 6., 1. / 6., 2. / 3.],
];
#[derive(Clone, Debug)]
pub(super) struct Interface {
    minus: [usize; 3],
    plus: [usize; 3],
    area: f64,
    normal: Vec3,
    material: Material,
    pub(super) states: [State; 3],
}
#[derive(Clone, Debug)]
pub struct InterfaceReport {
    pub minus: [usize; 3],
    pub plus: [usize; 3],
    pub area_m2: f64,
    pub normal: Vec3,
    pub quadrature: [Response; 3],
    pub stored_j: f64,
    pub dissipated_j: f64,
    pub friction_dissipated_j: f64,
    pub friction_numerical_j: f64,
    pub friction_released_j: f64,
}
impl Body {
    /// Insert a fixed matched interface between two boundary faces before loading.
    /// Faces need separate vertex indices at matching rest positions. Minus-side
    /// winding points into the plus solid. Arbitrary crack paths are not inserted.
    /// # Errors
    /// Rejects nonmatching, shared, duplicate, nonboundary or inverted pairs,
    /// invalid indices, degenerate faces, and insertion into a deformed body.
    pub fn add_interface(
        &mut self,
        minus: [usize; 3],
        plus: [usize; 3],
        material: Material,
    ) -> Result<usize, &'static str> {
        if self.positions != self.rest
            || self
                .elements
                .iter()
                .any(|e| e.state != crate::plasticity::State::default())
            || self.interfaces.iter().any(|interface| {
                interface
                    .states
                    .iter()
                    .any(|state| state.maximum_separation_m() > 0.)
            })
            || self.interfaces.len() >= 8192
            || minus.iter().chain(&plus).any(|&i| i >= self.rest.len())
        {
            return Err("invalid interface insertion");
        }
        let mut nodes: Vec<_> = minus.iter().chain(&plus).copied().collect();
        nodes.sort_unstable();
        nodes.dedup();
        if nodes.len() != 6 {
            return Err("interface requires six distinct nodes");
        }
        let [a, b, c] = minus.map(|i| self.rest[i]);
        let edges = [sub(b, a), sub(c, a)];
        let scale = edges.iter().flatten().fold(0_f64, |a, v| a.max(v.abs()));
        if !scale.is_finite() || scale == 0. {
            return Err("degenerate interface");
        }
        let product = cross(edges[0].map(|x| x / scale), edges[1].map(|x| x / scale));
        let norm = product.iter().fold(0_f64, |a, v| a.hypot(*v));
        let area = 0.5 * norm * scale * scale;
        if norm <= 64. * f64::EPSILON || !area.is_finite() || area <= 0. {
            return Err("degenerate interface");
        }
        let normal = product.map(|v| v / norm);
        for (&m, &p) in minus.iter().zip(&plus) {
            if sub(self.rest[m], self.rest[p])
                .iter()
                .any(|v| v.abs() > scale * 1e-12)
            {
                return Err("interface vertices must match in rest space");
            }
        }
        let opposite = |face: [usize; 3]| -> Result<usize, &'static str> {
            let cells: Vec<_> = self
                .elements
                .iter()
                .filter(|e| face.iter().all(|i| e.nodes.contains(i)))
                .collect();
            if cells.len() != 1 {
                return Err("interface must pair boundary faces");
            }
            cells[0]
                .nodes
                .iter()
                .find(|i| !face.contains(i))
                .copied()
                .ok_or("invalid boundary face")
        };
        let minus_opposite = opposite(minus)?;
        let plus_opposite = opposite(plus)?;
        if dot(sub(self.rest[minus_opposite], a), normal) >= 0.
            || dot(sub(self.rest[plus_opposite], a), normal) <= 0.
        {
            return Err("interface winding must point from minus into plus solid");
        }
        let mut keys = [minus, plus];
        for key in &mut keys {
            key.sort_unstable();
        }
        for other in &self.interfaces {
            let mut existing = [other.minus, other.plus];
            for key in &mut existing {
                key.sort_unstable();
            }
            if existing.iter().any(|key| keys.contains(key)) {
                return Err("interface boundary already paired");
            }
        }
        let index = self.interfaces.len();
        self.interfaces.push(Interface {
            minus,
            plus,
            area,
            normal,
            material,
            states: [State::default(); 3],
        });
        Ok(index)
    }
    #[must_use]
    pub fn interface_states(&self) -> Vec<[State; 3]> {
        self.interfaces.iter().map(|i| i.states).collect()
    }
    /// Diagnostics in joules integrate energy density over the reference face.
    /// # Errors
    /// Rejects invalid cohesive response or overflow.
    pub fn interface_reports(&self) -> Result<Vec<InterfaceReport>, &'static str> {
        self.interfaces
            .iter()
            .map(|interface| {
                let responses = interface.responses(&self.positions, &self.rest)?.1;
                let stored = responses
                    .iter()
                    .map(|r| r.stored_j_m2 * (interface.area / 3.))
                    .sum::<f64>();
                let dissipated = responses
                    .iter()
                    .map(|r| r.dissipated_j_m2 * (interface.area / 3.))
                    .sum::<f64>();
                let integrate = |f: fn(&Response) -> f64| {
                    responses
                        .iter()
                        .map(|r| f(r) * (interface.area / 3.))
                        .sum::<f64>()
                };
                let friction_dissipated = integrate(|r| r.friction_dissipated_j_m2);
                let friction_numerical = integrate(|r| r.friction_numerical_j_m2);
                let friction_released = integrate(|r| r.friction_released_j_m2);
                if [
                    stored,
                    dissipated,
                    friction_dissipated,
                    friction_numerical,
                    friction_released,
                ]
                .iter()
                .any(|v| !v.is_finite())
                {
                    return Err("interface energy overflow");
                }
                Ok(InterfaceReport {
                    minus: interface.minus,
                    plus: interface.plus,
                    area_m2: interface.area,
                    normal: interface.normal,
                    quadrature: responses,
                    stored_j: stored,
                    dissipated_j: dissipated,
                    friction_dissipated_j: friction_dissipated,
                    friction_numerical_j: friction_numerical,
                    friction_released_j: friction_released,
                })
            })
            .collect()
    }
}
impl Interface {
    fn responses(
        &self,
        x: &[Vec3],
        rest: &[Vec3],
    ) -> Result<([State; 3], [Response; 3]), &'static str> {
        let mut values = Vec::with_capacity(3);
        for (q, weights) in WEIGHTS.iter().enumerate() {
            let jump: Vec3 = std::array::from_fn(|i| {
                (0..3)
                    .map(|a| {
                        weights[a]
                            * ((x[self.plus[a]][i] - rest[self.plus[a]][i])
                                - (x[self.minus[a]][i] - rest[self.minus[a]][i]))
                    })
                    .sum()
            });
            values.push(self.material.response(&self.states[q], jump, self.normal)?);
        }
        Ok((
            std::array::from_fn(|q| values[q].0),
            std::array::from_fn(|q| values[q].1),
        ))
    }
    pub(super) fn assemble(
        &self,
        x: &[Vec3],
        rest: &[Vec3],
        dofs: &[[Option<usize>; 3]],
        internal: &mut [Vec3],
        stiffness: &mut [Vec<f64>],
        tangent: bool,
    ) -> Result<([State; 3], [Response; 3]), &'static str> {
        let (states, responses) = self.responses(x, rest)?;
        let nodes = [
            self.minus[0],
            self.minus[1],
            self.minus[2],
            self.plus[0],
            self.plus[1],
            self.plus[2],
        ];
        for (q, response) in responses.iter().enumerate() {
            let shape: [f64; 6] =
                std::array::from_fn(|a| WEIGHTS[q][a % 3] * if a < 3 { -1. } else { 1. });
            for (a, &node_a) in nodes.iter().enumerate() {
                for i in 0..3 {
                    internal[node_a][i] += self.area / 3. * shape[a] * response.traction_pa[i];
                    if tangent && let Some(row) = dofs[node_a][i] {
                        for (b, &node_b) in nodes.iter().enumerate() {
                            for (j, &column) in dofs[node_b].iter().enumerate() {
                                if let Some(column) = column {
                                    stiffness[row][column] += self.area / 3.
                                        * shape[a]
                                        * shape[b]
                                        * response.tangent_pa_m[i][j];
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok((states, responses))
    }
}
