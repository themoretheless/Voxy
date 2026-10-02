//! Small CPU tetrahedral FEM with stateful plasticity and cohesive interfaces.
//! Dense solves are bounded to 512 vertices; optional dynamics use lumped mass.
use super::{Material, Response, State};
use crate::biomechanics::Matrix;
mod dynamics;
mod interfaces;
mod quadratic;
pub use quadratic::QuadraticNormalImpact;
pub use quadratic::{
    QuadraticClosestLimits, QuadraticClosestPoint, QuadraticCohesiveFace, QuadraticCohesiveMesh,
    QuadraticCohesiveTrial, QuadraticFragment,
};
pub use quadratic::{QuadraticCohesiveWetUpdate, QuadraticWetAdvance, QuadraticWetUpdate};
pub use quadratic::{QuadraticSurfaceContact, QuadraticSurfaceContactEvaluation};
pub use quadratic::{QuadraticSweep, QuadraticSweepLimits};
mod surface;
pub use quadratic::{
    ConsistentInertia, QuadraticBody, QuadraticDynamics, QuadraticEnergy, QuadraticFace,
};
pub use quadratic::{
    FiniteElasticEvaluation, FiniteQuadraticDynamics, QuadraticAdvance, QuadraticAdvanceLimits,
    QuadraticSubstep,
};
pub use quadratic::{
    QuadraticContactEvaluation, QuadraticCoulombImpulse, QuadraticFrictionEvaluation,
    QuadraticPlaneContact, QuadraticPlaneCoulomb, QuadraticPlaneFriction,
};
mod topology;
pub use dynamics::{AdvanceLimits, Diagnostics, DynamicBody, DynamicStep, Fragment};
use interfaces::Interface;
pub use interfaces::InterfaceReport;
pub use surface::SurfaceFace;
pub use topology::CohesiveMesh;
type Vec3 = [f64; 3];
#[derive(Clone, Debug)]
struct Element {
    nodes: [usize; 4],
    gradients: [Vec3; 4],
    volume: f64,
    material: Material,
    state: State,
}
#[derive(Clone, Debug)]
pub struct Body {
    rest: Vec<Vec3>,
    positions: Vec<Vec3>,
    elements: Vec<Element>,
    interfaces: Vec<Interface>,
}
#[derive(Clone, Debug)]
pub struct Equilibrium {
    pub converged: bool,
    pub iterations: usize,
    /// Euclidean norm over unconstrained degrees of freedom, in newtons.
    pub residual_n: f64,
    /// Internal force minus external force, including constrained reactions.
    pub reactions_n: Vec<Vec3>,
}
#[derive(Debug)]
struct Evaluation {
    states: Vec<State>,
    responses: Vec<Response>,
    internal: Vec<Vec3>,
    stiffness: Vec<Vec<f64>>,
    interface_states: Vec<[crate::cohesive::State; 3]>,
}
impl Body {
    /// # Errors
    /// Rejects invalid or degenerate meshes, unused vertices, and >512 vertices.
    pub fn new(rest: Vec<Vec3>, cells: Vec<([usize; 4], Material)>) -> Result<Self, &'static str> {
        if rest.is_empty()
            || rest.len() > 512
            || cells.is_empty()
            || cells.len() > 8192
            || rest.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid plastic mesh");
        }
        let mut used = vec![false; rest.len()];
        let mut elements = Vec::with_capacity(cells.len());
        for (nodes, material) in cells {
            if nodes.iter().any(|&i| i >= rest.len()) {
                return Err("invalid plastic cell index");
            }
            let [a, b, c, d] = nodes.map(|i| rest[i]);
            let edges = [sub(b, a), sub(c, a), sub(d, a)];
            let scale = edges.iter().flatten().fold(0_f64, |a, v| a.max(v.abs()));
            if !scale.is_finite() || scale == 0. {
                return Err("invalid plastic cell scale");
            }
            let edges = edges.map(|v| v.map(|x| x / scale));
            let determinant = dot(edges[0], cross(edges[1], edges[2]));
            if !determinant.is_finite() || determinant.abs() <= 64. * f64::EPSILON {
                return Err("degenerate plastic cell");
            }
            let inverse = [
                cross(edges[1], edges[2]),
                cross(edges[2], edges[0]),
                cross(edges[0], edges[1]),
            ]
            .map(|v| v.map(|x| (x / determinant) / scale));
            let gradients = [
                std::array::from_fn(|k| -inverse[0][k] - inverse[1][k] - inverse[2][k]),
                inverse[0],
                inverse[1],
                inverse[2],
            ];
            let volume = (determinant.abs() * scale / 6.) * scale * scale;
            if !volume.is_finite()
                || volume <= 0.
                || gradients.iter().flatten().any(|v| !v.is_finite())
            {
                return Err("plastic cell geometry overflow");
            }
            for i in nodes {
                used[i] = true;
            }
            elements.push(Element {
                nodes,
                gradients,
                volume,
                material,
                state: State::default(),
            });
        }
        if used.contains(&false) {
            return Err("unused plastic mesh vertex");
        }
        Ok(Self {
            positions: rest.clone(),
            rest,
            elements,
            interfaces: Vec::new(),
        })
    }
    #[must_use]
    pub fn positions(&self) -> &[Vec3] {
        &self.positions
    }
    #[must_use]
    pub fn states(&self) -> Vec<State> {
        self.elements.iter().map(|e| e.state).collect()
    }
    /// Re-evaluate stresses without committing material history.
    /// # Errors
    /// Rejects invalid material responses or numerical overflow.
    pub fn responses(&self) -> Result<Vec<Response>, &'static str> {
        let dofs = vec![[None; 3]; self.rest.len()];
        Ok(self.evaluate(&self.positions, &dofs, 0, false)?.responses)
    }
    /// Loads in N; prescribed displacement components in m relative to rest.
    /// `None` denotes a free degree of freedom. Unconverged/invalid solves leave
    /// positions AND all material histories unchanged. Newton trials always
    /// start from the last accepted plastic states, never previous trial states.
    /// # Errors
    /// Rejects invalid inputs, singular supports/tangents, inversion and overflow.
    #[allow(clippy::too_many_lines)] // Keep trial acceptance and atomic history commit together.
    pub fn equilibrate(
        &mut self,
        loads: &[Vec3],
        prescribed: &[[Option<f64>; 3]],
        max_iterations: usize,
        tolerance_n: f64,
    ) -> Result<Equilibrium, &'static str> {
        if loads.len() != self.rest.len()
            || prescribed.len() != self.rest.len()
            || loads.iter().flatten().any(|v| !v.is_finite())
            || prescribed
                .iter()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
            || max_iterations == 0
            || !tolerance_n.is_finite()
            || tolerance_n <= 0.
        {
            return Err("invalid plastic solve inputs");
        }
        let mut count = 0;
        let dofs: Vec<[Option<usize>; 3]> = prescribed
            .iter()
            .map(|row| {
                row.map(|value| {
                    if value.is_none() {
                        let index = count;
                        count += 1;
                        Some(index)
                    } else {
                        None
                    }
                })
            })
            .collect();
        let mut x = self.positions.clone();
        for (i, row) in prescribed.iter().enumerate() {
            for (k, value) in row.iter().enumerate() {
                if let Some(value) = value {
                    x[i][k] = self.rest[i][k] + value;
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
            let norm = residual.iter().fold(0_f64, |a, v| a.hypot(*v));
            if !norm.is_finite() {
                return Err("plastic residual overflow");
            }
            if norm <= tolerance_n {
                self.positions = x;
                for (e, state) in self.elements.iter_mut().zip(evaluation.states) {
                    e.state = state;
                }
                for (interface, states) in
                    self.interfaces.iter_mut().zip(evaluation.interface_states)
                {
                    interface.states = states;
                }
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
            let direction =
                solve_dense(evaluation.stiffness, residual.iter().map(|v| -v).collect())?;
            let mut fraction = 1.;
            let mut accepted = None;
            for _ in 0..32 {
                let trial: Vec<_> = x
                    .iter()
                    .enumerate()
                    .map(|(i, row)| {
                        std::array::from_fn(|k| {
                            row[k] + dofs[i][k].map_or(0., |j| fraction * direction[j])
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
                        .fold(0_f64, |a, v| a.hypot(*v));
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
    fn evaluate(
        &self,
        x: &[Vec3],
        dofs: &[[Option<usize>; 3]],
        count: usize,
        tangent: bool,
    ) -> Result<Evaluation, &'static str> {
        if x.iter().flatten().any(|v| !v.is_finite()) {
            return Err("nonfinite plastic position");
        }
        let mut result = Evaluation {
            states: Vec::with_capacity(self.elements.len()),
            responses: Vec::with_capacity(self.elements.len()),
            internal: vec![[0.; 3]; x.len()],
            stiffness: if tangent {
                vec![vec![0.; count]; count]
            } else {
                Vec::new()
            },
            interface_states: Vec::with_capacity(self.interfaces.len()),
        };
        for e in &self.elements {
            let gradient: Matrix = std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    (0..4)
                        .map(|a| (x[e.nodes[a]][i] - self.rest[e.nodes[a]][i]) * e.gradients[a][j])
                        .sum()
                })
            });
            let f: Matrix = std::array::from_fn(|i| {
                std::array::from_fn(|j| gradient[i][j] + if i == j { 1. } else { 0. })
            });
            let determinant = dot(f[0], cross(f[1], f[2]));
            if !determinant.is_finite() || determinant <= 0. {
                return Err("inverted plastic element");
            }
            let strain: Matrix = std::array::from_fn(|i| {
                std::array::from_fn(|j| gradient[i][j].midpoint(gradient[j][i]))
            });
            let (state, response) = e.material.response(&e.state, strain)?;
            for (a, &node) in e.nodes.iter().enumerate() {
                for i in 0..3 {
                    result.internal[node][i] +=
                        e.volume * dot(response.stress.cauchy_pa[i], e.gradients[a]);
                }
            }
            if tangent {
                for (b, &node_b) in e.nodes.iter().enumerate() {
                    for axis in 0..3 {
                        let Some(column) = dofs[node_b][axis] else {
                            continue;
                        };
                        let direction: Matrix = std::array::from_fn(|i| {
                            std::array::from_fn(|j| {
                                0.5 * ((if i == axis { e.gradients[b][j] } else { 0. })
                                    + (if j == axis { e.gradients[b][i] } else { 0. }))
                            })
                        });
                        let stress_direction =
                            e.material.tangent_action(&e.state, strain, direction)?;
                        for (a, &node_a) in e.nodes.iter().enumerate() {
                            for (i, row) in stress_direction.iter().enumerate() {
                                if let Some(index) = dofs[node_a][i] {
                                    result.stiffness[index][column] +=
                                        e.volume * dot(*row, e.gradients[a]);
                                }
                            }
                        }
                    }
                }
            }
            result.states.push(state);
            result.responses.push(response);
        }
        for interface in &self.interfaces {
            let (states, _) = interface.assemble(
                x,
                &self.rest,
                dofs,
                &mut result.internal,
                &mut result.stiffness,
                tangent,
            )?;
            result.interface_states.push(states);
        }
        if result.internal.iter().flatten().any(|v| !v.is_finite())
            || result.stiffness.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("plastic assembly overflow");
        }
        Ok(result)
    }
}
fn reduced(v: &[Vec3], dofs: &[[Option<usize>; 3]], count: usize) -> Vec<f64> {
    let mut result = vec![0.; count];
    for (i, row) in v.iter().enumerate() {
        for (k, value) in row.iter().enumerate() {
            if let Some(index) = dofs[i][k] {
                result[index] = *value;
            }
        }
    }
    result
}
#[allow(clippy::float_cmp)] // Exact structural zeros: skipping them preserves Gaussian elimination.
fn solve_dense(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Result<Vec<f64>, &'static str> {
    let scale = a.iter().flatten().fold(0_f64, |x, v| x.max(v.abs()));
    for i in 0..b.len() {
        let pivot = (i..b.len())
            .max_by(|&x, &y| a[x][i].abs().total_cmp(&a[y][i].abs()))
            .ok_or("empty tangent")?;
        if a[pivot][i].abs() <= scale * 1e-14 {
            return Err("singular plastic tangent or missing supports");
        }
        a.swap(i, pivot);
        b.swap(i, pivot);
        for j in i + 1..b.len() {
            if a[j][i] == 0. {
                continue;
            }
            let factor = a[j][i] / a[i][i];
            let (before, after) = a.split_at_mut(j);
            for (value, pivot_value) in after[0].iter_mut().zip(&before[i]).skip(i) {
                *value -= factor * pivot_value;
            }
            b[j] -= factor * b[i];
        }
    }
    for i in (0..b.len()).rev() {
        b[i] = (b[i] - (i + 1..b.len()).map(|j| a[i][j] * b[j]).sum::<f64>()) / a[i][i];
        if !b[i].is_finite() {
            return Err("plastic linear solve overflow");
        }
    }
    Ok(b)
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: Vec3, b: Vec3) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub use quadratic::{QuadraticWork, QuadraticWorkEquilibrium};

pub use quadratic::{CoupledQuadraticWork, CoupledWorkEquilibrium, InterfaceWork};

pub use quadratic::{QuadraticFractureAdvance, QuadraticFractureStep, QuadraticFractureSubstep};

pub use quadratic::QuadraticFragmentRotation;

pub use quadratic::QuadraticFragmentSweep;

pub use quadratic::QuadraticFirstClearance;

pub use quadratic::{QuadraticFragmentFirstCandidate, QuadraticFragmentFirstClearance};

pub use quadratic::{QuadraticClearanceAdvance, QuadraticClearanceStep};

pub use quadratic::QuadraticAutomaticImpact;

pub use quadratic::{QuadraticImpactConstraint, QuadraticMultiImpact};

pub use quadratic::QuadraticFragmentImpactConstraint;

pub use quadratic::{QuadraticAutomaticMultiImpact, QuadraticDetectedFragmentContact};

pub use quadratic::{QuadraticSurfaceTriangle, QuadraticSurfaceVertex};
