//! Reference-area adaptive quadratic surface penalty against a fixed plane.
mod coulomb;
use super::{QuadraticBody, QuadraticFace, Vec3, dot};
pub use coulomb::{QuadraticCoulombImpulse, QuadraticPlaneCoulomb};
#[derive(Clone, Copy, Debug)]
pub struct QuadraticPlaneContact {
    normal: Vec3,
    offset_m: f64,
    stiffness_n_m3: f64,
    refinement_depth: u8,
}
impl QuadraticPlaneContact {
    /// Allowed halfspace is `normal·x >= offset_m`; stiffness is N/m³ (Pa/m).
    /// # Errors
    /// Nonunit/nonfinite normal, invalid offset or nonpositive stiffness.
    pub fn new(normal: Vec3, offset_m: f64, stiffness_n_m3: f64) -> Result<Self, &'static str> {
        if normal.iter().any(|v| !v.is_finite())
            || (dot(normal, normal) - 1.).abs() > 1e-10
            || !offset_m.is_finite()
            || !stiffness_n_m3.is_finite()
            || stiffness_n_m3 <= 0.
        {
            return Err("invalid quadratic penalty plane");
        }
        Ok(Self {
            normal,
            offset_m,
            stiffness_n_m3,
            refinement_depth: 4,
        })
    }
    /// Maximum subdivision depth for partially active faces (default 4).
    /// Zero retains the original six-point rule; fully active faces need no split.
    /// # Errors
    /// Depth greater than 8 exceeds the bounded integration work limit.
    pub fn with_refinement_depth(mut self, depth: u8) -> Result<Self, &'static str> {
        if depth > 8 {
            return Err("quadratic contact refinement limit");
        }
        self.refinement_depth = depth;
        Ok(self)
    }
}
/// Velocity-regularized kinetic Coulomb traction. It tends to mu*p at high
/// slip speed; zero slip gives zero traction. This law does not model stiction.
#[derive(Clone, Copy, Debug)]
pub struct QuadraticPlaneFriction {
    coefficient: f64,
    regularization_m_s: f64,
}
impl QuadraticPlaneFriction {
    /// # Errors
    /// Nonfinite/negative coefficient or nonpositive regularization speed.
    pub fn new(coefficient: f64, regularization_m_s: f64) -> Result<Self, &'static str> {
        if !coefficient.is_finite()
            || coefficient < 0.
            || !regularization_m_s.is_finite()
            || regularization_m_s <= 0.
        {
            return Err("invalid quadratic plane friction");
        }
        Ok(Self {
            coefficient,
            regularization_m_s,
        })
    }
}
#[derive(Clone, Debug)]
pub struct QuadraticFrictionEvaluation {
    /// Forces on body nodes, N, integrated with quadratic shape functions.
    pub forces_n: Vec<Vec3>,
    pub resultant_n: Vec3,
    /// Instantaneous mechanical power, nonpositive (W).
    pub power_w: f64,
}
#[derive(Clone, Debug)]
pub struct QuadraticContactEvaluation {
    pub energy_j: f64,
    /// Conservative absolute quadrature energy error bound on mixed leaf cells,
    /// in exact arithmetic. Floating point extrema are not interval certified.
    pub partial_energy_error_bound_j: f64,
    /// Gradient of contact energy, N. Force on the body is the negative.
    pub gradient_n: Vec<Vec3>,
    pub force_n: Vec3,
    /// Minimum at the quadrature samples, not a certified geometric minimum.
    pub minimum_sample_gap_m: f64,
    /// Minimum over every quadratic boundary face, including edge/interior extrema.
    /// Analytic in exact arithmetic; floating-point result, not interval certified.
    pub minimum_surface_gap_m: f64,
}
impl QuadraticBody {
    /// Reference-area kinetic friction against the stationary penalty plane.
    /// Uses the same active-region subdivisions as normal contact. Velocities
    /// interpolate quadratically; normal velocity contributes no friction power.
    /// # Errors
    /// Invalid counts/data, geometry or force/power overflow.
    pub fn plane_friction_at(
        &self,
        positions: &[Vec3],
        velocities: &[Vec3],
        plane: QuadraticPlaneContact,
        friction: QuadraticPlaneFriction,
    ) -> Result<QuadraticFrictionEvaluation, &'static str> {
        if positions.len() != self.rest.len()
            || velocities.len() != self.rest.len()
            || velocities.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid quadratic friction input");
        }
        let mut result = QuadraticFrictionEvaluation {
            forces_n: vec![[0.; 3]; self.rest.len()],
            resultant_n: [0.; 3],
            power_w: 0.,
        };
        evaluate_faces_with(
            positions,
            &self.exposed_faces_at(positions)?,
            plane,
            &mut |face, shape, gap, factor| {
                let velocity: Vec3 = std::array::from_fn(|axis| {
                    face.nodes
                        .iter()
                        .zip(shape)
                        .map(|(&node, n)| velocities[node][axis] * n)
                        .sum()
                });
                let normal_speed = dot(plane.normal, velocity) / dot(plane.normal, plane.normal);
                let tangent: Vec3 =
                    std::array::from_fn(|axis| velocity[axis] - normal_speed * plane.normal[axis]);
                let speed = tangent[0].hypot(tangent[1]).hypot(tangent[2]);
                let denominator = speed.hypot(friction.regularization_m_s);
                let force: Vec3 =
                    tangent.map(|v| friction.coefficient * factor * gap * (v / denominator));
                result.power_w += dot(force, velocity);
                for (&node, n) in face.nodes.iter().zip(shape) {
                    for (axis, &component) in force.iter().enumerate() {
                        result.forces_n[node][axis] += n * component;
                    }
                }
            },
        )?;
        for force in &result.forces_n {
            for (axis, &component) in force.iter().enumerate() {
                result.resultant_n[axis] += component;
            }
        }
        if !result.power_w.is_finite()
            || result.power_w > 0.
            || result
                .forces_n
                .iter()
                .flatten()
                .chain(&result.resultant_n)
                .any(|v| !v.is_finite())
        {
            return Err("quadratic friction overflow");
        }
        Ok(result)
    }
    /// Adaptive six-point integration of frictionless reference-area contact.
    /// The clipped penalty is not exactly integrated for partially active faces;
    /// refine the surface/time discretization. No CCD or interval gap certification.
    /// # Errors
    /// Wrong count, invalid geometry or energy/force overflow.
    pub fn plane_contact_at(
        &self,
        positions: &[Vec3],
        plane: QuadraticPlaneContact,
    ) -> Result<QuadraticContactEvaluation, &'static str> {
        if positions.len() != self.rest.len() {
            return Err("invalid quadratic contact positions");
        }
        evaluate_faces(positions, &self.exposed_faces_at(positions)?, plane)
    }
}
pub(super) fn evaluate_faces(
    positions: &[Vec3],
    faces: &[QuadraticFace],
    plane: QuadraticPlaneContact,
) -> Result<QuadraticContactEvaluation, &'static str> {
    evaluate_faces_with(positions, faces, plane, &mut |_, _, _, _| {})
}
fn evaluate_faces_with(
    positions: &[Vec3],
    faces: &[QuadraticFace],
    plane: QuadraticPlaneContact,
    sample: &mut impl FnMut(&QuadraticFace, [f64; 6], f64, f64),
) -> Result<QuadraticContactEvaluation, &'static str> {
    if positions.iter().flatten().any(|v| !v.is_finite()) {
        return Err("nonfinite quadratic contact positions");
    }
    let mut result = QuadraticContactEvaluation {
        energy_j: 0.,
        partial_energy_error_bound_j: 0.,
        gradient_n: vec![[0.; 3]; positions.len()],
        force_n: [0.; 3],
        minimum_sample_gap_m: f64::INFINITY,
        minimum_surface_gap_m: f64::INFINITY,
    };
    // Positive degree-four triangle rule, weights normalized to unit face area.
    for face in faces {
        let gaps = face
            .nodes
            .map(|node| dot(plane.normal, positions[node]) - plane.offset_m);
        let minimum = face_minimum(gaps)?;
        result.minimum_surface_gap_m = result.minimum_surface_gap_m.min(minimum);
        integrate_triangle(
            &mut result,
            face,
            gaps,
            plane,
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            1.,
            plane.refinement_depth,
            sample,
        )?;
    }

    for gradient in &result.gradient_n {
        for (axis, &value) in gradient.iter().enumerate() {
            result.force_n[axis] -= value;
        }
    }
    if !result.energy_j.is_finite()
        || !result.partial_energy_error_bound_j.is_finite()
        || result
            .gradient_n
            .iter()
            .flatten()
            .chain(&result.force_n)
            .any(|v| !v.is_finite())
        || !result.minimum_sample_gap_m.is_finite()
        || !result.minimum_surface_gap_m.is_finite()
    {
        return Err("quadratic contact evaluation overflow");
    }
    Ok(result)
}

fn shape_at(l: Vec3) -> [f64; 6] {
    [
        l[0] * (2. * l[0] - 1.),
        l[1] * (2. * l[1] - 1.),
        l[2] * (2. * l[2] - 1.),
        4. * l[0] * l[1],
        4. * l[1] * l[2],
        4. * l[0] * l[2],
    ]
}
fn midpoint(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| 0.5 * a[i] + 0.5 * b[i])
}
#[allow(clippy::too_many_arguments)]
fn integrate_triangle(
    result: &mut QuadraticContactEvaluation,
    face: &QuadraticFace,
    gaps: [f64; 6],
    plane: QuadraticPlaneContact,
    vertices: [Vec3; 3],
    area_fraction: f64,
    depth: u8,
    sample: &mut impl FnMut(&QuadraticFace, [f64; 6], f64, f64),
) -> Result<(), &'static str> {
    let edge = [
        midpoint(vertices[0], vertices[1]),
        midpoint(vertices[1], vertices[2]),
        midpoint(vertices[0], vertices[2]),
    ];
    let gap_at = |l| {
        shape_at(l)
            .iter()
            .zip(gaps)
            .map(|(n, g)| n * g)
            .sum::<f64>()
    };
    let local = [
        gap_at(vertices[0]),
        gap_at(vertices[1]),
        gap_at(vertices[2]),
        gap_at(edge[0]),
        gap_at(edge[1]),
        gap_at(edge[2]),
    ];
    let minimum = face_minimum(local)?;
    let maximum = -face_minimum(local.map(|g| -g))?;
    let mixed = minimum < 0. && maximum > 0.;
    if mixed && depth > 0 {
        for child in [
            [vertices[0], edge[0], edge[2]],
            [edge[0], vertices[1], edge[1]],
            [edge[2], edge[1], vertices[2]],
            [edge[0], edge[1], edge[2]],
        ] {
            integrate_triangle(
                result,
                face,
                gaps,
                plane,
                child,
                area_fraction * 0.25,
                depth - 1,
                sample,
            )?;
        }
        return Ok(());
    }
    let stiffness_area = plane.stiffness_n_m3 * face.reference_area_m2 * area_fraction;
    if mixed {
        // Both true and positive-weight approximate energy lie in [0, cap].
        result.partial_energy_error_bound_j += 0.5 * stiffness_area * minimum * minimum;
    }
    for (a, b, weight) in [
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
    ] {
        for distinct in 0..3 {
            let local_l: Vec3 = std::array::from_fn(|i| if i == distinct { b } else { a });
            let l: Vec3 =
                std::array::from_fn(|axis| (0..3).map(|i| local_l[i] * vertices[i][axis]).sum());
            let shape = shape_at(l);
            let gap = gap_at(l);
            result.minimum_sample_gap_m = result.minimum_sample_gap_m.min(gap);
            if gap >= 0. {
                continue;
            }
            let factor = stiffness_area * weight;
            sample(face, shape, gap, factor);
            result.energy_j += 0.5 * factor * gap * gap;
            for (&node, n) in face.nodes.iter().zip(shape) {
                for (axis, &normal) in plane.normal.iter().enumerate() {
                    result.gradient_n[node][axis] += factor * gap * n * normal;
                }
            }
        }
    }
    Ok(())
}

/// Minimize a quadratic on the closed barycentric triangle. A singular convex
/// stationary set reaches an edge, so vertices/edge extrema cover that case.
fn face_minimum(gaps: [f64; 6]) -> Result<f64, &'static str> {
    if gaps.iter().any(|g| !g.is_finite()) {
        return Err("quadratic surface gap overflow");
    }
    let scale = gaps.iter().fold(0_f64, |maximum, g| maximum.max(g.abs()));
    if scale == 0. {
        return Ok(0.);
    }
    let g = gaps.map(|value| value / scale);
    let mut minimum = g[0].min(g[1]).min(g[2]);
    for (start, end, midpoint) in [(0, 1, 3), (1, 2, 4), (0, 2, 5)] {
        let curvature = 2. * g[start] + 2. * g[end] - 4. * g[midpoint];
        let slope = -3. * g[start] - g[end] + 4. * g[midpoint];
        if curvature > 0. {
            let t = -slope / (2. * curvature);
            if t > 0. && t < 1. {
                minimum = minimum.min((curvature * t + slope) * t + g[start]);
            }
        }
    }
    let uu = 2. * g[0] + 2. * g[1] - 4. * g[3];
    let vv = 2. * g[0] + 2. * g[2] - 4. * g[5];
    let uv = 4. * (g[0] - g[3] + g[4] - g[5]);
    let du = -3. * g[0] - g[1] + 4. * g[3];
    let dv = -3. * g[0] - g[2] + 4. * g[5];
    let determinant = 4. * uu * vv - uv * uv;
    if uu > 0. && vv > 0. && determinant > 0. {
        let u = (uv * dv - 2. * vv * du) / determinant;
        let v = (uv * du - 2. * uu * dv) / determinant;
        if u > 0. && v > 0. && u + v < 1. {
            minimum = minimum.min(g[0] + du * u + dv * v + uu * u * u + uv * u * v + vv * v * v);
        }
    }
    let result = minimum * scale;
    if !result.is_finite() {
        return Err("quadratic surface minimum overflow");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::face_minimum;
    fn minimum(coefficients: [f64; 6]) -> f64 {
        let value = |u: f64, v: f64| {
            coefficients[0]
                + coefficients[1] * u
                + coefficients[2] * v
                + coefficients[3] * u * u
                + coefficients[4] * u * v
                + coefficients[5] * v * v
        };
        face_minimum([
            value(0., 0.),
            value(1., 0.),
            value(0., 1.),
            value(0.5, 0.),
            value(0.5, 0.5),
            value(0., 0.5),
        ])
        .unwrap()
    }
    #[test]
    fn analytic_minimum_handles_edges_flat_singular_and_interior_cases() {
        // Convex minimum outside triangle: minimum lies on u+v=1.
        assert!((minimum([2., -2., -2., 1., 0., 1.]) - 0.5).abs() < 1e-14);
        // Rank-one Hessian: entire minimum line intersects the boundary.
        assert!(minimum([0.25, -1., -1., 1., 2., 1.]).abs() < 1e-14);
        assert_eq!(minimum([2., 0., 0., 0., 0., 0.]), 2.);
        assert!((minimum([0., 0., 0., -1., 0., -1.]) + 1.).abs() < 1e-14);
        // Positive definite Hessian with cross term, interior minimum (.2,.3).
        assert!(minimum([0.19, -0.7, -0.8, 1., 1., 1.]).abs() < 1e-14);
        assert!(
            (minimum([0.19e-200, -0.7e-200, -0.8e-200, 1e-200, 1e-200, 1e-200]) / 1e-200).abs()
                < 1e-14
        );
    }
    #[test]
    fn analytic_minimum_agrees_with_independent_dense_triangle_search() {
        for seed in 0..100 {
            let coefficients: [f64; 6] = std::array::from_fn(|index| {
                (f64::from(seed * 17 + i32::try_from(index).unwrap() * 31) * 0.73).sin()
            });
            let analytic = minimum(coefficients);
            let mut sampled = f64::INFINITY;
            for i in 0..=100 {
                for j in 0..=100 - i {
                    let u = f64::from(i) / 100.;
                    let v = f64::from(j) / 100.;
                    let value = coefficients[0]
                        + coefficients[1] * u
                        + coefficients[2] * v
                        + coefficients[3] * u * u
                        + coefficients[4] * u * v
                        + coefficients[5] * v * v;
                    sampled = sampled.min(value);
                }
            }
            assert!(analytic <= sampled + 1e-12, "seed={seed}");
            assert!(
                sampled - analytic < 0.001,
                "seed={seed}, difference={}",
                sampled - analytic
            );
        }
    }
}
