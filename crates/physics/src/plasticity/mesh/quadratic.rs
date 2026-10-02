//! Straight-sided ten-node tetrahedra, four-point volume integration.
use super::{Body, Equilibrium, Material, Matrix, State, Vec3, dot, reduced, solve_dense, sub};
mod closest;
mod impact;
mod sweep;
pub use impact::QuadraticNormalImpact;
pub use sweep::{QuadraticSweep, QuadraticSweepLimits};
mod contact;
pub use contact::{QuadraticSurfaceContact, QuadraticSurfaceContactEvaluation};
mod cohesive;
pub use closest::{QuadraticClosestLimits, QuadraticClosestPoint};
mod fragments;
mod topology;
pub use topology::QuadraticCohesiveMesh;
mod interfaces;
pub use cohesive::{QuadraticCohesiveFace, QuadraticCohesiveTrial};
pub use fragments::{QuadraticFragment, QuadraticFragmentRotation};
mod dynamics;
mod plane;
pub use plane::{
    QuadraticContactEvaluation, QuadraticCoulombImpulse, QuadraticFrictionEvaluation,
    QuadraticPlaneContact, QuadraticPlaneCoulomb, QuadraticPlaneFriction,
};
mod finite;
mod interface_work;
mod work;
pub use finite::FiniteElasticEvaluation;
pub use interface_work::{CoupledQuadraticWork, CoupledWorkEquilibrium, InterfaceWork};
pub use work::{QuadraticWork, QuadraticWorkEquilibrium};
mod surface;
pub use dynamics::{
    FiniteQuadraticDynamics, QuadraticAdvance, QuadraticAdvanceLimits, QuadraticCohesiveWetUpdate,
    QuadraticDynamics, QuadraticEnergy, QuadraticSubstep, QuadraticWetAdvance, QuadraticWetUpdate,
};
pub use surface::QuadraticFace;
const EDGES: [(usize, usize); 6] = [(0, 1), (1, 2), (0, 2), (0, 3), (1, 3), (2, 3)];
#[derive(Clone, Debug)]
struct Cell {
    nodes: [usize; 10],
    gradients: [[Vec3; 10]; 4],
    volume: f64,
    material: Material,
    states: [State; 4],
}
#[derive(Clone, Debug)]
pub struct QuadraticBody {
    rest: Vec<Vec3>,
    positions: Vec<Vec3>,
    cells: Vec<Cell>,
    interfaces: Vec<QuadraticCohesiveFace>,
}
struct Evaluation {
    internal: Vec<Vec3>,
    tangent: Vec<Vec<f64>>,
    states: Vec<[State; 4]>,
    interfaces: Vec<QuadraticCohesiveFace>,
}
impl QuadraticBody {
    /// Elevate a conforming linear tetrahedral mesh, preserving original corner
    /// indices and sharing one newly inserted midpoint per source edge.
    /// # Errors
    /// Invalid source mesh or expanded geometry exceeding the 512-node limit.
    pub fn from_linear(
        mut rest: Vec<Vec3>,
        cells: Vec<([usize; 4], Material)>,
    ) -> Result<Self, &'static str> {
        Body::new(rest.clone(), cells.clone())?;
        let mut edges = std::collections::BTreeMap::new();
        let mut quadratic = Vec::with_capacity(cells.len());
        for (corners, material) in cells {
            let mut nodes = [0; 10];
            nodes[..4].copy_from_slice(&corners);
            for (edge, &(a, b)) in EDGES.iter().enumerate() {
                let key = (corners[a].min(corners[b]), corners[a].max(corners[b]));
                let midpoint = if let Some(&index) = edges.get(&key) {
                    index
                } else {
                    if rest.len() >= 512 {
                        return Err("elevated quadratic mesh exceeds vertex limit");
                    }
                    let index = rest.len();
                    let position = std::array::from_fn(|axis| {
                        rest[corners[a]][axis].midpoint(rest[corners[b]][axis])
                    });
                    rest.push(position);
                    edges.insert(key, index);
                    index
                };
                nodes[4 + edge] = midpoint;
            }
            quadratic.push((nodes, material));
        }
        Self::new(rest, quadratic)
    }
    /// Sorted source-corner edge pairs and their shared midpoint indices.
    #[must_use]
    pub fn edge_midpoints(&self) -> Vec<([usize; 2], usize)> {
        let mut edges = std::collections::BTreeMap::new();
        for cell in &self.cells {
            for (edge, &(a, b)) in EDGES.iter().enumerate() {
                let first = cell.nodes[a];
                let second = cell.nodes[b];
                edges.insert([first.min(second), first.max(second)], cell.nodes[4 + edge]);
            }
        }
        edges.into_iter().collect()
    }
    /// Corners 0..4, then edge midpoints in order 01,12,02,03,13,23.
    /// Shared edges must share midpoint indices; curved reference cells are rejected.
    /// Four independent J2 histories per cell; cohesive faces have six histories.
    /// # Errors
    /// Invalid indices/geometry, unused nodes, incompatible edge topology or limits.
    pub fn new(rest: Vec<Vec3>, cells: Vec<([usize; 10], Material)>) -> Result<Self, &'static str> {
        if rest.is_empty()
            || rest.len() > 512
            || cells.is_empty()
            || cells.len() > 8192
            || rest.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid quadratic mesh");
        }
        let mut used = vec![false; rest.len()];
        let corners: std::collections::BTreeSet<_> = cells
            .iter()
            .flat_map(|(n, _)| n[..4].iter().copied())
            .collect();
        let mut edges = std::collections::BTreeMap::new();
        let mut result = Vec::new();
        for (nodes, material) in cells {
            if nodes.iter().any(|&n| n >= rest.len()) {
                return Err("invalid quadratic cell index");
            }
            let mut unique = nodes;
            unique.sort_unstable();
            if unique.windows(2).any(|p| p[0] == p[1]) {
                return Err("repeated quadratic cell node");
            }
            let local = Body::new(
                nodes[..4].iter().map(|&n| rest[n]).collect(),
                vec![([0, 1, 2, 3], material)],
            )?;
            let element = &local.elements[0];
            let scale = nodes[..4]
                .iter()
                .flat_map(|&n| sub(rest[n], rest[nodes[0]]))
                .fold(0_f64, |m, v| m.max(v.abs()));
            for (edge, &(a, b)) in EDGES.iter().enumerate() {
                let mid = nodes[4 + edge];
                if corners.contains(&mid) {
                    return Err("quadratic midpoint is also a corner");
                }
                for ((&mid_coordinate, &first), &second) in
                    rest[mid].iter().zip(&rest[nodes[a]]).zip(&rest[nodes[b]])
                {
                    if (mid_coordinate - first.midpoint(second)).abs() > scale * 1e-12 {
                        return Err("quadratic reference edge must be straight");
                    }
                }
                let key = (nodes[a].min(nodes[b]), nodes[a].max(nodes[b]));
                if edges
                    .insert(key, mid)
                    .is_some_and(|previous| previous != mid)
                {
                    return Err("nonconforming quadratic edge");
                }
            }
            let high = (5. + 3. * 5_f64.sqrt()) / 20.;
            let low = (5. - 5_f64.sqrt()) / 20.;
            let gradients = std::array::from_fn(|point| {
                let l: [f64; 4] = std::array::from_fn(|i| if i == point { high } else { low });
                let mut g = [[0.; 3]; 10];
                for i in 0..4 {
                    g[i] = element.gradients[i].map(|v| (4. * l[i] - 1.) * v);
                }
                for (edge, &(a, b)) in EDGES.iter().enumerate() {
                    g[4 + edge] = std::array::from_fn(|axis| {
                        4. * (l[a] * element.gradients[b][axis] + l[b] * element.gradients[a][axis])
                    });
                }
                g
            });
            for node in nodes {
                used[node] = true;
            }
            result.push(Cell {
                nodes,
                gradients,
                volume: element.volume,
                material,
                states: [State::default(); 4],
            });
        }
        if used.contains(&false) {
            return Err("unused quadratic mesh vertex");
        }
        let body = Self {
            positions: rest.clone(),
            rest,
            cells: result,
            interfaces: Vec::new(),
        };
        body.reference_faces()?;
        Ok(body)
    }
    #[must_use]
    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }
    #[must_use]
    pub fn states(&self) -> Vec<[State; 4]> {
        self.cells.iter().map(|c| c.states).collect()
    }
    /// Exact consistent scalar mass matrix `integral rho*N_i*N_j dV` in kg.
    /// Apply the same matrix independently to each velocity component. Four-point
    /// stiffness quadrature does not exactly integrate this degree-four integrand.
    /// Row sums at corner nodes can be negative: they are not lumped point masses.
    /// # Errors
    /// Invalid per-cell density or assembly overflow.
    pub fn consistent_mass(&self, densities: &[f64]) -> Result<Vec<Vec<f64>>, &'static str> {
        if densities.len() != self.cells.len()
            || densities.iter().any(|d| !d.is_finite() || *d <= 0.)
        {
            return Err("invalid quadratic mass density");
        }
        let mut mass = vec![vec![0.; self.rest.len()]; self.rest.len()];
        for (cell, &density) in self.cells.iter().zip(densities) {
            let scale = (cell.volume / 420.) * density;
            if !scale.is_finite() || scale <= 0. {
                return Err("quadratic mass scale overflow");
            }
            for (i, &row) in cell.nodes.iter().enumerate() {
                for (j, &column) in cell.nodes.iter().enumerate() {
                    let coefficient = mass_coefficient(i, j);
                    mass[row][column] += scale * coefficient;
                }
            }
        }
        if mass.iter().flatten().any(|m| !m.is_finite()) {
            return Err("quadratic mass assembly overflow");
        }
        Ok(mass)
    }
    /// Four quadrature responses per cell, evaluated without committing history.
    /// # Errors
    /// Invalid position count, nonfinite geometry or inverted integration points.
    pub fn responses_at(
        &self,
        positions: &[Vec3],
    ) -> Result<Vec<[super::Response; 4]>, &'static str> {
        if positions.len() != self.rest.len() || positions.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid quadratic response positions");
        }
        self.cells
            .iter()
            .map(|cell| {
                let mut responses = Vec::with_capacity(4);
                for (point, g) in cell.gradients.iter().enumerate() {
                    let strain = strain_at(cell, g, positions, &self.rest)?;
                    responses.push(cell.material.response(&cell.states[point], strain)?.1);
                }
                responses
                    .try_into()
                    .map_err(|_| "invalid quadratic response count")
            })
            .collect()
    }
    fn evaluate(
        &self,
        x: &[Vec3],
        dofs: &[[Option<usize>; 3]],
        count: usize,
        tangent: bool,
    ) -> Result<Evaluation, &'static str> {
        let mut result = Evaluation {
            internal: vec![[0.; 3]; x.len()],
            tangent: if tangent {
                vec![vec![0.; count]; count]
            } else {
                Vec::new()
            },
            states: Vec::new(),
            interfaces: Vec::new(),
        };
        if x.iter().flatten().any(|v| !v.is_finite()) {
            return Err("nonfinite quadratic position");
        }
        for cell in &self.cells {
            let mut states = cell.states;
            for (point, g) in cell.gradients.iter().enumerate() {
                let strain = strain_at(cell, g, x, &self.rest)?;
                let (state, response) = cell.material.response(&cell.states[point], strain)?;
                states[point] = state;
                let weight = cell.volume / 4.;
                for (a, &node) in cell.nodes.iter().enumerate() {
                    for i in 0..3 {
                        result.internal[node][i] +=
                            weight * dot(response.stress.cauchy_pa[i], g[a]);
                    }
                }
                if tangent {
                    for (b, &node) in cell.nodes.iter().enumerate() {
                        for axis in 0..3 {
                            let Some(column) = dofs[node][axis] else {
                                continue;
                            };
                            let direction: Matrix = std::array::from_fn(|i| {
                                std::array::from_fn(|j| {
                                    0.5 * ((if i == axis { g[b][j] } else { 0. })
                                        + (if j == axis { g[b][i] } else { 0. }))
                                })
                            });
                            let stress = cell.material.tangent_action(
                                &cell.states[point],
                                strain,
                                direction,
                            )?;
                            for (a, &node_a) in cell.nodes.iter().enumerate() {
                                for (i, row) in stress.iter().enumerate() {
                                    if let Some(index) = dofs[node_a][i] {
                                        result.tangent[index][column] += weight * dot(*row, g[a]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            result.states.push(states);
        }
        self.assemble_cohesive(x, dofs, tangent, &mut result)?;
        if result
            .internal
            .iter()
            .flatten()
            .chain(result.tangent.iter().flatten())
            .any(|v| !v.is_finite())
        {
            return Err("quadratic assembly overflow");
        }
        Ok(result)
    }
    /// Same SI loading/constraint and transactional-history contract as linear FEM.
    /// # Errors
    /// Invalid input, singular support/tangent, inversion or assembly overflow.
    #[allow(clippy::too_many_lines)] // Newton candidates and all quadrature histories commit together.
    pub fn equilibrate(
        &mut self,
        loads: &[Vec3],
        prescribed: &[[Option<f64>; 3]],
        max_iterations: usize,
        tolerance_n: f64,
    ) -> Result<Equilibrium, &'static str> {
        if loads.len() != self.rest.len()
            || prescribed.len() != self.rest.len()
            || max_iterations == 0
            || !tolerance_n.is_finite()
            || tolerance_n <= 0.
            || loads
                .iter()
                .flatten()
                .chain(prescribed.iter().flatten().flatten())
                .any(|v| !v.is_finite())
        {
            return Err("invalid quadratic solve input");
        }
        let mut count = 0;
        let dofs: Vec<_> = prescribed
            .iter()
            .map(|row| {
                row.map(|p| {
                    if p.is_some() {
                        None
                    } else {
                        let index = count;
                        count += 1;
                        Some(index)
                    }
                })
            })
            .collect();
        let mut x = self.positions.clone();
        for (node, row) in prescribed.iter().enumerate() {
            for (axis, &value) in row.iter().enumerate() {
                if let Some(v) = value {
                    x[node][axis] = self.rest[node][axis] + v;
                }
            }
        }
        for iteration in 0..=max_iterations {
            let evaluation = self.evaluate(&x, &dofs, count, true)?;
            let reactions: Vec<_> = evaluation
                .internal
                .iter()
                .zip(loads)
                .map(|(a, b)| sub(*a, *b))
                .collect();
            let residual = reduced(&reactions, &dofs, count);
            let norm = residual.iter().fold(0_f64, |m, v| m.hypot(*v));
            if !norm.is_finite() {
                return Err("quadratic residual overflow");
            }
            if norm <= tolerance_n {
                self.positions = x;
                for (cell, states) in self.cells.iter_mut().zip(evaluation.states) {
                    cell.states = states;
                }
                self.interfaces = evaluation.interfaces;
                return Ok(Equilibrium {
                    converged: true,
                    iterations: iteration,
                    residual_n: norm,
                    reactions_n: reactions,
                });
            }
            if iteration == max_iterations {
                return Ok(Equilibrium {
                    converged: false,
                    iterations: iteration,
                    residual_n: norm,
                    reactions_n: reactions,
                });
            }
            let direction = solve_dense(evaluation.tangent, residual.iter().map(|v| -v).collect())?;
            let mut fraction = 1.;
            let mut accepted = None;
            for _ in 0..32 {
                let trial: Vec<_> = x
                    .iter()
                    .enumerate()
                    .map(|(node, row)| {
                        std::array::from_fn(|axis| {
                            row[axis]
                                + dofs[node][axis].map_or(0., |index| fraction * direction[index])
                        })
                    })
                    .collect();
                if let Ok(candidate) = self.evaluate(&trial, &dofs, count, false) {
                    let forces: Vec<_> = candidate
                        .internal
                        .iter()
                        .zip(loads)
                        .map(|(a, b)| sub(*a, *b))
                        .collect();
                    let trial_norm = reduced(&forces, &dofs, count)
                        .iter()
                        .fold(0_f64, |m, v| m.hypot(*v));
                    if trial_norm.is_finite() && trial_norm < norm * (1. - 1e-4 * fraction) {
                        accepted = Some(trial);
                        break;
                    }
                }
                fraction *= 0.5;
            }
            let Some(trial) = accepted else {
                return Ok(Equilibrium {
                    converged: false,
                    iterations: iteration,
                    residual_n: norm,
                    reactions_n: reactions,
                });
            };
            x = trial;
        }
        unreachable!()
    }
}

fn kinematics_at(
    cell: &Cell,
    g: &[Vec3; 10],
    x: &[Vec3],
    rest: &[Vec3],
) -> Result<(Matrix, Matrix), &'static str> {
    let gradient: Matrix = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..10)
                .map(|a| (x[cell.nodes[a]][i] - rest[cell.nodes[a]][i]) * g[a][j])
                .sum()
        })
    });
    let f: Matrix = std::array::from_fn(|i| {
        std::array::from_fn(|j| gradient[i][j] + if i == j { 1. } else { 0. })
    });
    let determinant = dot(f[0], super::cross(f[1], f[2]));
    if !determinant.is_finite() || determinant <= 0. {
        return Err("inverted quadratic integration point");
    }
    Ok((gradient, f))
}

fn deformation_at(
    cell: &Cell,
    g: &[Vec3; 10],
    x: &[Vec3],
    rest: &[Vec3],
) -> Result<Matrix, &'static str> {
    Ok(kinematics_at(cell, g, x, rest)?.1)
}
fn strain_at(
    cell: &Cell,
    g: &[Vec3; 10],
    x: &[Vec3],
    rest: &[Vec3],
) -> Result<Matrix, &'static str> {
    let gradient = kinematics_at(cell, g, x, rest)?.0;
    Ok(std::array::from_fn(|i| {
        std::array::from_fn(|j| gradient[i][j].midpoint(gradient[j][i]))
    }))
}

/// Cached scaled Cholesky factor of the consistent quadratic mass matrix.
#[derive(Clone, Debug)]
pub struct ConsistentInertia {
    lower: Vec<Vec<f64>>,
    scale: f64,
}
impl ConsistentInertia {
    /// Solve `M*a = force` independently for each component, without changing
    /// the cached mass factor. Forces are nodal N and accelerations are m/s².
    /// # Errors
    /// Invalid force count, nonfinite input or solution overflow.
    pub fn accelerations(&self, forces: &[Vec3]) -> Result<Vec<Vec3>, &'static str> {
        let count = self.lower.len();
        if forces.len() != count || forces.iter().flatten().any(|f| !f.is_finite()) {
            return Err("invalid consistent inertia force");
        }
        let mut result = vec![[0.; 3]; count];
        for axis in 0..3 {
            for i in 0..count {
                let sum: f64 = (0..i).map(|j| self.lower[i][j] * result[j][axis]).sum();
                result[i][axis] = (forces[i][axis] / self.scale - sum) / self.lower[i][i];
            }
            for i in (0..count).rev() {
                let sum: f64 = (i + 1..count)
                    .map(|j| self.lower[j][i] * result[j][axis])
                    .sum();
                result[i][axis] = (result[i][axis] - sum) / self.lower[i][i];
            }
        }
        if result.iter().flatten().any(|a| !a.is_finite()) {
            return Err("consistent inertia solution overflow");
        }
        Ok(result)
    }
}
impl QuadraticBody {
    /// Factor the exact mass matrix once for repeated force-to-acceleration solves.
    /// # Errors
    /// Invalid densities, mass overflow, or numerically singular mass matrix.
    pub fn consistent_inertia(&self, densities: &[f64]) -> Result<ConsistentInertia, &'static str> {
        let mass = self.consistent_mass(densities)?;
        ConsistentInertia::factor(&mass)
    }
}

impl ConsistentInertia {
    fn factor(mass: &[Vec<f64>]) -> Result<Self, &'static str> {
        let scale = mass.iter().flatten().fold(0_f64, |m, v| m.max(v.abs()));
        if !scale.is_finite() || scale <= 0. {
            return Err("invalid consistent inertia scale");
        }
        let count = mass.len();
        let mut lower = vec![vec![0_f64; count]; count];
        for i in 0..count {
            for j in 0..=i {
                let sum: f64 = (0..j).map(|k| lower[i][k] * lower[j][k]).sum();
                let value = mass[i][j] / scale - sum;
                lower[i][j] = if i == j {
                    if !value.is_finite() || value <= 1e-14 {
                        return Err("numerically singular consistent inertia");
                    }
                    value.sqrt()
                } else {
                    value / lower[j][j]
                };
            }
        }
        Ok(ConsistentInertia { lower, scale })
    }
}

fn mass_coefficient(row: usize, column: usize) -> f64 {
    match (row < 4, column < 4) {
        (true, true) => {
            if row == column {
                6.
            } else {
                1.
            }
        }
        (true, false) => {
            if [EDGES[column - 4].0, EDGES[column - 4].1].contains(&row) {
                -4.
            } else {
                -6.
            }
        }
        (false, true) => {
            if [EDGES[row - 4].0, EDGES[row - 4].1].contains(&column) {
                -4.
            } else {
                -6.
            }
        }
        (false, false) => {
            let (first_a, first_b) = EDGES[row - 4];
            let (second_a, second_b) = EDGES[column - 4];
            if row == column {
                32.
            } else if [second_a, second_b].contains(&first_a)
                || [second_a, second_b].contains(&first_b)
            {
                16.
            } else {
                8.
            }
        }
    }
}

pub use dynamics::{QuadraticFractureAdvance, QuadraticFractureStep, QuadraticFractureSubstep};

mod fragment_sweep;
pub use fragment_sweep::{
    QuadraticFragmentFirstCandidate, QuadraticFragmentFirstClearance, QuadraticFragmentSweep,
};

mod first_sweep;
pub use first_sweep::QuadraticFirstClearance;

pub use dynamics::{QuadraticClearanceAdvance, QuadraticClearanceStep};

pub use dynamics::QuadraticAutomaticImpact;

mod multi_impact;
pub use multi_impact::{QuadraticImpactConstraint, QuadraticMultiImpact};

pub use dynamics::QuadraticFragmentImpactConstraint;

pub use dynamics::{QuadraticAutomaticMultiImpact, QuadraticDetectedFragmentContact};

mod render_surface;
pub use render_surface::{QuadraticSurfaceTriangle, QuadraticSurfaceVertex};
