//! Objective matched T6 cohesive face with six-point reference-area integration.
use super::{Vec3, dot};
use crate::cohesive::{Material, Response, State};
const COHESIVE_RULE: [(f64, f64, f64); 2] = [
    (
        0.445_948_490_915_965,
        0.108_103_018_168_070,
        0.223_381_589_678_011,
    ),
    (
        0.091_576_213_509_771,
        0.816_847_572_980_459,
        0.109_951_743_655_322,
    ),
];
#[derive(Clone, Debug)]
pub struct QuadraticCohesiveFace {
    minus: [usize; 6],
    plus: [usize; 6],
    area: f64,
    material: Material,
    states: [State; 6],
    count: usize,
}
#[derive(Clone, Debug)]
pub struct QuadraticCohesiveTrial {
    /// Virtual-work energy gradient; physical forces have the opposite sign.
    pub internal_n: Vec<Vec3>,
    pub quadrature: [Response; 6],
    pub normals: [Vec3; 6],
    pub stored_j: f64,
    pub dissipated_j: f64,
    pub friction_dissipated_j: f64,
    pub friction_numerical_j: f64,
    pub friction_released_j: f64,
    /// Candidate history. Publish only after the containing solve is accepted.
    pub candidate: QuadraticCohesiveFace,
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: Vec3) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
pub(super) fn basis(l: Vec3) -> ([f64; 6], [f64; 6], [f64; 6]) {
    (
        [
            l[0] * (2. * l[0] - 1.),
            l[1] * (2. * l[1] - 1.),
            l[2] * (2. * l[2] - 1.),
            4. * l[0] * l[1],
            4. * l[1] * l[2],
            4. * l[0] * l[2],
        ],
        [
            1. - 4. * l[0],
            4. * l[1] - 1.,
            0.,
            4. * (l[0] - l[1]),
            4. * l[2],
            -4. * l[2],
        ],
        [
            1. - 4. * l[0],
            0.,
            4. * l[2] - 1.,
            -4. * l[1],
            4. * l[1],
            4. * (l[0] - l[2]),
        ],
    )
}
struct Frame {
    t1: Vec3,
    t2: Vec3,
    t3: Vec3,
    edge1: Vec3,
    edge2: Vec3,
    length: f64,
    scale: f64,
    product_length: f64,
}
impl Frame {
    fn new(mean: &[Vec3; 6], du: [f64; 6], dv: [f64; 6]) -> Result<Self, &'static str> {
        let interpolate = |weights: [f64; 6]| -> Vec3 {
            std::array::from_fn(|axis| {
                mean.iter()
                    .zip(weights)
                    .map(|(p, n)| (p[axis] - mean[0][axis]) * n)
                    .sum()
            })
        };
        let e1 = interpolate(du);
        let e2 = interpolate(dv);
        let length = norm(e1);
        let scale = length.max(norm(e2));
        if !scale.is_finite() || length <= 0. {
            return Err("singular quadratic cohesive frame");
        }
        let edge1 = e1.map(|v| v / scale);
        let edge2 = e2.map(|v| v / scale);
        let product = cross(edge1, edge2);
        let product_length = norm(product);
        if !product_length.is_finite() || product_length <= 64. * f64::EPSILON {
            return Err("singular quadratic cohesive frame");
        }
        let t1 = e1.map(|v| v / length);
        let t3 = product.map(|v| v / product_length);
        let t2 = cross(t3, t1);
        Ok(Self {
            t1,
            t2,
            t3,
            edge1,
            edge2,
            length,
            scale,
            product_length,
        })
    }
    fn world(&self, tau: Vec3) -> Vec3 {
        std::array::from_fn(|axis| {
            tau[0] * self.t1[axis] + tau[1] * self.t2[axis] + tau[2] * self.t3[axis]
        })
    }
    fn derivatives(&self, jump: Vec3, tau: Vec3) -> (Vec3, Vec3) {
        let adj1: Vec3 =
            std::array::from_fn(|axis| tau[0] * jump[axis] + tau[1] * cross(jump, self.t3)[axis]);
        let adj3: Vec3 =
            std::array::from_fn(|axis| tau[2] * jump[axis] + tau[1] * cross(self.t1, jump)[axis]);
        let first: Vec3 = std::array::from_fn(|axis| {
            (adj1[axis] - self.t1[axis] * dot(adj1, self.t1)) / self.length
        });
        let normal_adj: Vec3 = std::array::from_fn(|axis| {
            (adj3[axis] - self.t3[axis] * dot(adj3, self.t3)) / self.product_length / self.scale
        });
        let de1: Vec3 =
            std::array::from_fn(|axis| first[axis] + cross(self.edge2, normal_adj)[axis]);
        (de1, cross(normal_adj, self.edge1))
    }
}
impl QuadraticCohesiveFace {
    /// Corners followed by edge midpoints 01,12,02, on separately indexed faces.
    /// Matched straight reference faces only. Solid ownership/boundary pairing
    /// must be validated by the containing mesh before attaching this kernel.
    /// # Errors
    /// Invalid indices, shared nodes, nonmatching/misordered edges or degeneracy.
    pub fn new(
        rest: &[Vec3],
        minus: [usize; 6],
        plus: [usize; 6],
        material: Material,
    ) -> Result<Self, &'static str> {
        if rest.iter().flatten().any(|v| !v.is_finite())
            || minus.iter().chain(&plus).any(|&i| i >= rest.len())
        {
            return Err("invalid quadratic cohesive reference");
        }
        let mut indices: Vec<_> = minus.iter().chain(&plus).copied().collect();
        indices.sort_unstable();
        indices.dedup();
        if indices.len() != 12 {
            return Err("quadratic cohesive faces require separate nodes");
        }
        let e1 = sub(rest[minus[1]], rest[minus[0]]);
        let e2 = sub(rest[minus[2]], rest[minus[0]]);
        let scale = norm(e1).max(norm(e2));
        if !scale.is_finite() || scale <= 0. {
            return Err("degenerate quadratic cohesive reference");
        }
        let product = cross(e1.map(|v| v / scale), e2.map(|v| v / scale));
        let area = 0.5 * norm(product) * scale * scale;
        if !area.is_finite() || area <= 0. || norm(product) <= 64. * f64::EPSILON {
            return Err("degenerate quadratic cohesive reference");
        }
        for (&m, &p) in minus.iter().zip(&plus) {
            if norm(sub(rest[m], rest[p])) > scale * 1e-12 {
                return Err("quadratic cohesive nodes must match");
            }
        }
        for (edge, (a, b)) in [(0, 1), (1, 2), (0, 2)].into_iter().enumerate() {
            let midpoint: Vec3 =
                std::array::from_fn(|axis| 0.5 * rest[minus[a]][axis] + 0.5 * rest[minus[b]][axis]);
            if norm(sub(rest[minus[edge + 3]], midpoint)) > scale * 1e-12 {
                return Err("quadratic cohesive reference edges must be straight");
            }
        }
        Ok(Self {
            minus,
            plus,
            area,
            material,
            states: [State::default(); 6],
            count: rest.len(),
        })
    }
    #[must_use]
    pub fn sides(&self) -> ([usize; 6], [usize; 6]) {
        (self.minus, self.plus)
    }
    pub(super) fn derivative_step(&self, x: &[Vec3]) -> f64 {
        let magnitude = self
            .minus
            .iter()
            .chain(&self.plus)
            .flat_map(|&i| x[i])
            .fold(0_f64, |m, v| m.max(v.abs()));
        (f64::EPSILON.cbrt() * self.material.onset_m()).max(64. * f64::EPSILON * magnitude)
    }
    /// Accepted full failure; independent of current surface geometry/closure.
    #[must_use]
    pub fn is_fully_broken(&self) -> bool {
        // Full damage is an exact clamped constitutive endpoint, not a tolerance.
        #[allow(clippy::float_cmp)]
        self.states.iter().all(|s| self.material.damage(s) == 1.)
    }
    #[must_use]
    pub fn states(&self) -> [State; 6] {
        self.states
    }
    #[must_use]
    pub fn area_m2(&self) -> f64 {
        self.area
    }
    /// Corotated traction and frame virtual work on the mean quadratic surface.
    /// Reference area stays fixed; the moving frame is differentiated, so forces
    /// preserve frame covariance and angular momentum even in crack closure.
    /// Six-point integration is approximate for nonlinear damage variations.
    /// # Errors
    /// Invalid positions, singular current surface frames or constitutive overflow.
    pub fn trial_at(&self, positions: &[Vec3]) -> Result<QuadraticCohesiveTrial, &'static str> {
        if positions.len() != self.count || positions.iter().flatten().any(|v| !v.is_finite()) {
            return Err("invalid quadratic cohesive positions");
        }
        let mut forces = vec![[0.; 3]; self.count];
        let mut candidate = self.clone();
        let mut responses = Vec::with_capacity(6);
        let mut normals = Vec::with_capacity(6);
        let mut energies = [0.; 5];
        for (a, b, weight) in COHESIVE_RULE {
            for distinct in 0..3 {
                let l = std::array::from_fn(|i| if i == distinct { b } else { a });
                let (shape, du, dv) = basis(l);
                let mean: [Vec3; 6] = std::array::from_fn(|i| {
                    std::array::from_fn(|axis| {
                        0.5 * positions[self.minus[i]][axis] + 0.5 * positions[self.plus[i]][axis]
                    })
                });
                let frame = Frame::new(&mean, du, dv)?;
                let jump: Vec3 = std::array::from_fn(|axis| {
                    self.minus
                        .iter()
                        .zip(&self.plus)
                        .zip(shape)
                        .map(|((&m, &p), n)| (positions[p][axis] - positions[m][axis]) * n)
                        .sum()
                });
                let local = [
                    dot(jump, frame.t1),
                    dot(jump, frame.t2),
                    dot(jump, frame.t3),
                ];
                let index = responses.len();
                let (state, response) =
                    self.material
                        .response(&self.states[index], local, [0., 0., 1.])?;
                candidate.states[index] = state;
                let world = frame.world(response.traction_pa);
                let (de1, de2) = frame.derivatives(jump, response.traction_pa);
                let factor = self.area * weight;
                for i in 0..6 {
                    for axis in 0..3 {
                        let frame = 0.5 * (du[i] * de1[axis] + dv[i] * de2[axis]);
                        forces[self.minus[i]][axis] += factor * (frame - shape[i] * world[axis]);
                        forces[self.plus[i]][axis] += factor * (frame + shape[i] * world[axis]);
                    }
                }
                for (sum, value) in energies.iter_mut().zip([
                    response.stored_j_m2,
                    response.dissipated_j_m2,
                    response.friction_dissipated_j_m2,
                    response.friction_numerical_j_m2,
                    response.friction_released_j_m2,
                ]) {
                    *sum += factor * value;
                }
                responses.push(response);
                normals.push(frame.t3);
            }
        }
        if forces
            .iter()
            .flatten()
            .chain(&energies)
            .any(|v| !v.is_finite())
        {
            return Err("quadratic cohesive assembly overflow");
        }
        Ok(QuadraticCohesiveTrial {
            internal_n: forces,
            quadrature: responses
                .try_into()
                .map_err(|_| "quadratic cohesive quadrature count")?,
            normals: normals
                .try_into()
                .map_err(|_| "quadratic cohesive quadrature count")?,
            stored_j: energies[0],
            dissipated_j: energies[1],
            friction_dissipated_j: energies[2],
            friction_numerical_j: energies[3],
            friction_released_j: energies[4],
            candidate,
        })
    }
}

impl QuadraticCohesiveFace {
    /// Change a frictionless law at accepted fixed geometry without healing or
    /// resetting quadrature fracture work. Returns external parameter work in J.
    /// # Errors
    /// Invalid pose, new loading, healing law or failed point migration. Atomic.
    pub fn update_material_at(
        &mut self,
        positions: &[Vec3],
        material: Material,
    ) -> Result<f64, &'static str> {
        // Validate complete geometry before touching any history.
        self.trial_at(positions)?;
        let mut next = self.clone();
        let mut work = 0.;
        let mut index = 0;
        for (a, b, weight) in COHESIVE_RULE {
            for distinct in 0..3 {
                let l = std::array::from_fn(|i| if i == distinct { b } else { a });
                let (shape, du, dv) = basis(l);
                let mean: [Vec3; 6] = std::array::from_fn(|i| {
                    std::array::from_fn(|axis| {
                        0.5 * positions[self.minus[i]][axis] + 0.5 * positions[self.plus[i]][axis]
                    })
                });
                let frame = Frame::new(&mean, du, dv)?;
                let jump: Vec3 = std::array::from_fn(|axis| {
                    self.minus
                        .iter()
                        .zip(&self.plus)
                        .zip(shape)
                        .map(|((&m, &p), n)| (positions[p][axis] - positions[m][axis]) * n)
                        .sum()
                });
                let local = [
                    dot(jump, frame.t1),
                    dot(jump, frame.t2),
                    dot(jump, frame.t3),
                ];
                let (state, point_work) = material.migrate_history(
                    &self.material,
                    &self.states[index],
                    local,
                    [0., 0., 1.],
                )?;
                next.states[index] = state;
                work += self.area * weight * point_work;
                index += 1;
            }
        }
        if !work.is_finite() {
            return Err("quadratic cohesive parameter work overflow");
        }
        next.material = material;
        next.trial_at(positions)?;
        *self = next;
        Ok(work)
    }
}
