//! Coupled maximum-dissipation impulse solve with consistent (restricted) mass.
use super::super::ConsistentInertia;
use super::{QuadraticBody, QuadraticPlaneContact, Vec3, dot, evaluate_faces_with};
#[derive(Clone, Copy, Debug)]
pub struct QuadraticPlaneCoulomb {
    coefficient: f64,
    velocity_tolerance_m_s: f64,
    max_iterations: usize,
    max_samples: usize,
}
impl QuadraticPlaneCoulomb {
    /// # Errors
    /// Nonfinite/negative mu, invalid velocity tolerance or work limits.
    pub fn new(
        coefficient: f64,
        velocity_tolerance_m_s: f64,
        max_iterations: usize,
        max_samples: usize,
    ) -> Result<Self, &'static str> {
        if !coefficient.is_finite()
            || coefficient < 0.
            || !velocity_tolerance_m_s.is_finite()
            || velocity_tolerance_m_s <= 0.
            || max_iterations == 0
            || max_samples == 0
            || max_samples > 16384
        {
            return Err("invalid quadratic Coulomb limits");
        }
        Ok(Self {
            coefficient,
            velocity_tolerance_m_s,
            max_iterations,
            max_samples,
        })
    }
}
#[derive(Clone, Debug)]
pub struct QuadraticCoulombImpulse {
    pub velocities: Vec<Vec3>,
    pub nodal_impulse_n_s: Vec<Vec3>,
    pub support_impulse_n_s: Vec<Vec3>,
    pub iterations: usize,
    /// Projected-dual KKT residual scaled to velocity units.
    pub residual_m_s: f64,
    /// Endpoint traction work loss. Roundoff/solver negative values are clamped.
    pub endpoint_dissipated_j: f64,
    /// Backward impulse's kinetic decrement beyond endpoint traction work.
    pub numerical_dissipated_j: f64,
    pub energy_defect_j: f64,
}
struct Point {
    nodes: [usize; 6],
    shape: [f64; 6],
    weights: Vec<f64>,
    response: Vec<f64>,
    inverse_mass: f64,
    radius: f64,
    impulse: Vec3,
}
fn tangent(v: Vec3, normal: Vec3) -> Vec3 {
    let component = dot(v, normal) / dot(normal, normal);
    std::array::from_fn(|axis| v[axis] - component * normal[axis])
}
fn norm(v: Vec3) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn project(v: Vec3, radius: f64) -> Vec3 {
    let magnitude = norm(v);
    if magnitude > radius {
        v.map(|value| value * (radius / magnitude))
    } else {
        v
    }
}
impl QuadraticBody {
    /// Solve the convex Coulomb impulse problem on fixed contact geometry.
    /// Whole-node supports have zero velocity. Surface impulses are coupled by
    /// the consistent mass; this is not an independent clamp of nodal velocities.
    /// No history is modified. The plane's pressure is frozen for this kick.
    /// # Errors
    /// Invalid input/mass, sample limit, nonconvergence, or nonfinite diagnostics.
    #[allow(clippy::too_many_arguments)]
    pub fn coulomb_plane_impulse_at(
        &self,
        positions: &[Vec3],
        velocities: &[Vec3],
        densities: &[f64],
        pinned: &[bool],
        plane: QuadraticPlaneContact,
        law: QuadraticPlaneCoulomb,
        dt: f64,
    ) -> Result<QuadraticCoulombImpulse, &'static str> {
        let count = self.positions().len();
        if positions.len() != count
            || velocities.len() != count
            || pinned.len() != count
            || velocities.iter().flatten().any(|v| !v.is_finite())
            || pinned
                .iter()
                .zip(velocities)
                .any(|(p, v)| *p && v.iter().any(|x| *x != 0.))
            || !dt.is_finite()
            || dt <= 0.
        {
            return Err("invalid quadratic Coulomb input");
        }
        let mass = self.consistent_mass(densities)?;
        let free: Vec<_> = (0..count).filter(|&i| !pinned[i]).collect();
        let restricted: Vec<Vec<f64>> = free
            .iter()
            .map(|&i| free.iter().map(|&j| mass[i][j]).collect())
            .collect();
        let inertia = if free.is_empty() {
            None
        } else {
            Some(ConsistentInertia::factor(&restricted)?)
        };
        let mut points = Vec::new();
        let mut sample_overflow = false;
        evaluate_faces_with(
            positions,
            &self.exposed_faces_at(positions)?,
            plane,
            &mut |face, shape, gap, factor| {
                let radius = law.coefficient * factor * (-gap) * dt;
                if radius == 0. {
                    return;
                }
                if points.len() == law.max_samples {
                    sample_overflow = true;
                    return;
                }
                let weights: Vec<f64> = free
                    .iter()
                    .map(|node| {
                        face.nodes
                            .iter()
                            .zip(shape)
                            .filter(|(index, _)| *index == node)
                            .map(|(_, n)| n)
                            .sum()
                    })
                    .collect();
                points.push(Point {
                    nodes: face.nodes,
                    shape,
                    weights,
                    response: Vec::new(),
                    inverse_mass: 0.,
                    radius,
                    impulse: [0.; 3],
                });
            },
        )?;
        if sample_overflow {
            return Err("quadratic Coulomb sample limit reached");
        }
        for point in &mut points {
            if !point.radius.is_finite() {
                return Err("quadratic Coulomb impulse overflow");
            }
            if let Some(inertia) = &inertia {
                let forcing: Vec<_> = point.weights.iter().map(|&w| [w, 0., 0.]).collect();
                point.response = inertia
                    .accelerations(&forcing)?
                    .iter()
                    .map(|a| a[0])
                    .collect();
                point.inverse_mass = point
                    .weights
                    .iter()
                    .zip(&point.response)
                    .map(|(a, b)| a * b)
                    .sum();
                if !point.inverse_mass.is_finite() || point.inverse_mass < 0. {
                    return Err("invalid quadratic Coulomb inverse mass");
                }
            }
        }
        let mut result = QuadraticCoulombImpulse {
            velocities: velocities.to_vec(),
            nodal_impulse_n_s: vec![[0.; 3]; count],
            support_impulse_n_s: vec![[0.; 3]; count],
            iterations: 0,
            residual_m_s: 0.,
            endpoint_dissipated_j: 0.,
            numerical_dissipated_j: 0.,
            energy_defect_j: 0.,
        };
        solve_points(&mut points, &free, plane, law, &mut result)?;
        diagnostics(&points, &free, &mass, velocities, plane, &mut result)?;
        Ok(result)
    }
}

fn sampled_velocity(point: &Point, free: &[usize], velocities: &[Vec3], normal: Vec3) -> Vec3 {
    tangent(
        std::array::from_fn(|axis| {
            point
                .weights
                .iter()
                .zip(free)
                .map(|(w, &i)| w * velocities[i][axis])
                .sum()
        }),
        normal,
    )
}
fn solve_points(
    points: &mut [Point],
    free: &[usize],
    plane: QuadraticPlaneContact,
    law: QuadraticPlaneCoulomb,
    result: &mut QuadraticCoulombImpulse,
) -> Result<(), &'static str> {
    let initial_velocities = result.velocities.clone();
    warm_start(points, free, plane, result)?;
    let mut converged = points.is_empty() || free.is_empty();
    for iteration in 0..law.max_iterations.div_ceil(2) {
        if converged {
            break;
        }
        for point in points.iter_mut() {
            if point.inverse_mass == 0. {
                continue;
            }
            let velocity = sampled_velocity(point, free, &result.velocities, plane.normal);
            let proposed = project(
                std::array::from_fn(|axis| {
                    point.impulse[axis] - velocity[axis] / point.inverse_mass
                }),
                point.radius,
            );
            let delta: Vec3 = std::array::from_fn(|axis| proposed[axis] - point.impulse[axis]);
            for (&node, &response) in free.iter().zip(&point.response) {
                for (axis, &component) in delta.iter().enumerate() {
                    result.velocities[node][axis] += response * component;
                }
            }
            point.impulse = proposed;
        }
        result.iterations = iteration + 1;
        result.residual_m_s = 0.;
        for point in points.iter() {
            if point.inverse_mass == 0. {
                continue;
            }
            let velocity = sampled_velocity(point, free, &result.velocities, plane.normal);
            let projected = project(
                std::array::from_fn(|axis| {
                    point.impulse[axis] - velocity[axis] / point.inverse_mass
                }),
                point.radius,
            );
            let delta: Vec3 = std::array::from_fn(|axis| projected[axis] - point.impulse[axis]);
            result.residual_m_s = result.residual_m_s.max(norm(delta) * point.inverse_mass);
        }
        if !result.residual_m_s.is_finite() {
            return Err("quadratic Coulomb solve overflow");
        }
        converged = result.residual_m_s <= law.velocity_tolerance_m_s;
    }
    if !converged {
        converged = accelerated_solve(points, free, plane, law, &initial_velocities, result)?;
    }
    if !converged {
        return Err("quadratic Coulomb iteration limit reached");
    }
    Ok(())
}
fn reconstruct_velocity(
    points: &[Point],
    free: &[usize],
    impulses: &[Vec3],
    initial: &[Vec3],
) -> Vec<Vec3> {
    let mut velocity = initial.to_vec();
    for (point, impulse) in points.iter().zip(impulses) {
        for (&node, &response) in free.iter().zip(&point.response) {
            for (axis, &value) in impulse.iter().enumerate() {
                velocity[node][axis] += response * value;
            }
        }
    }
    velocity
}
/// FISTA on the same convex disk-constrained dual. Trace of the scalar
/// compliance matrix bounds its Lipschitz constant, without assembling it.
fn accelerated_solve(
    points: &mut [Point],
    free: &[usize],
    plane: QuadraticPlaneContact,
    law: QuadraticPlaneCoulomb,
    initial: &[Vec3],
    result: &mut QuadraticCoulombImpulse,
) -> Result<bool, &'static str> {
    let lipschitz: f64 = points.iter().map(|p| p.inverse_mass).sum();
    if !lipschitz.is_finite() || lipschitz <= 0. {
        return Err("quadratic Coulomb solve overflow");
    }
    let mut previous: Vec<Vec3> = points.iter().map(|p| p.impulse).collect();
    let mut extrapolated = previous.clone();
    let mut momentum = 1_f64;
    while result.iterations < law.max_iterations {
        let velocity = reconstruct_velocity(points, free, &extrapolated, initial);
        let next: Vec<Vec3> = points
            .iter()
            .zip(&extrapolated)
            .map(|(point, impulse)| {
                let gradient = sampled_velocity(point, free, &velocity, plane.normal);
                project(
                    std::array::from_fn(|axis| impulse[axis] - gradient[axis] / lipschitz),
                    point.radius,
                )
            })
            .collect();
        result.velocities = reconstruct_velocity(points, free, &next, initial);
        result.iterations += 1;
        result.residual_m_s = 0.;
        for (point, &impulse) in points.iter_mut().zip(&next) {
            point.impulse = impulse;
            if point.inverse_mass == 0. {
                continue;
            }
            let gradient = sampled_velocity(point, free, &result.velocities, plane.normal);
            let projected = project(
                std::array::from_fn(|axis| impulse[axis] - gradient[axis] / point.inverse_mass),
                point.radius,
            );
            let delta: Vec3 = std::array::from_fn(|axis| projected[axis] - impulse[axis]);
            result.residual_m_s = result.residual_m_s.max(norm(delta) * point.inverse_mass);
        }
        if !result.residual_m_s.is_finite()
            || result.velocities.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("quadratic Coulomb solve overflow");
        }
        if result.residual_m_s <= law.velocity_tolerance_m_s {
            return Ok(true);
        }
        let next_momentum = 0.5 * (1. + (1. + 4. * momentum * momentum).sqrt());
        let factor = (momentum - 1.) / next_momentum;
        extrapolated = next
            .iter()
            .zip(&previous)
            .map(|(a, b)| std::array::from_fn(|axis| a[axis] + factor * (a[axis] - b[axis])))
            .collect();
        previous = next;
        momentum = next_momentum;
    }
    Ok(false)
}
fn diagnostics(
    points: &[Point],
    free: &[usize],
    mass: &[Vec<f64>],
    velocities: &[Vec3],
    plane: QuadraticPlaneContact,
    result: &mut QuadraticCoulombImpulse,
) -> Result<(), &'static str> {
    let mut endpoint_work = 0.;
    for point in points {
        endpoint_work += dot(
            point.impulse,
            sampled_velocity(point, free, &result.velocities, plane.normal),
        );
        for (&node, &weight) in point.nodes.iter().zip(&point.shape) {
            for axis in 0..3 {
                result.nodal_impulse_n_s[node][axis] += weight * point.impulse[axis];
            }
        }
    }
    let kinetic = |v: &[Vec3]| -> f64 {
        mass.iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .enumerate()
                    .map(|(j, &m)| 0.5 * m * dot(v[i], v[j]))
                    .sum::<f64>()
            })
            .sum()
    };
    let delta: Vec<Vec3> = result
        .velocities
        .iter()
        .zip(velocities)
        .map(|(a, b)| std::array::from_fn(|axis| a[axis] - b[axis]))
        .collect();
    for (node, row) in mass.iter().enumerate() {
        for axis in 0..3 {
            result.support_impulse_n_s[node][axis] = row
                .iter()
                .zip(&delta)
                .map(|(m, v)| m * v[axis])
                .sum::<f64>()
                - result.nodal_impulse_n_s[node][axis];
        }
    }
    result.endpoint_dissipated_j = (-endpoint_work).max(0.);
    result.numerical_dissipated_j = kinetic(&delta);
    result.energy_defect_j = kinetic(&result.velocities) - kinetic(velocities)
        + result.endpoint_dissipated_j
        + result.numerical_dissipated_j;
    if !result.energy_defect_j.is_finite()
        || !result.endpoint_dissipated_j.is_finite()
        || result.numerical_dissipated_j < 0.
        || result
            .velocities
            .iter()
            .flatten()
            .chain(result.nodal_impulse_n_s.iter().flatten())
            .chain(result.support_impulse_n_s.iter().flatten())
            .any(|v| !v.is_finite())
    {
        return Err("quadratic Coulomb diagnostic overflow");
    }
    Ok(())
}

/// Minimize the same convex objective first over a common pressure-weighted
/// traction direction. This feasible initial guess resolves uniform applied
/// pressure-proportional traction without slow redundant point iterations.
fn warm_start(
    points: &mut [Point],
    free: &[usize],
    plane: QuadraticPlaneContact,
    result: &mut QuadraticCoulombImpulse,
) -> Result<(), &'static str> {
    let scale = points.iter().fold(0_f64, |r, p| r.max(p.radius));
    if scale == 0. || free.is_empty() {
        return Ok(());
    }
    let mut response = vec![0.; free.len()];
    let mut linear = [0.; 3];
    for point in points.iter() {
        let weight = point.radius / scale;
        for (sum, value) in response.iter_mut().zip(&point.response) {
            *sum += weight * value;
        }
        let velocity = sampled_velocity(point, free, &result.velocities, plane.normal);
        for axis in 0..3 {
            linear[axis] += weight * velocity[axis];
        }
    }
    let curvature: f64 = points
        .iter()
        .map(|p| {
            p.radius / scale
                * p.weights
                    .iter()
                    .zip(&response)
                    .map(|(w, r)| w * r)
                    .sum::<f64>()
        })
        .sum();
    if !curvature.is_finite() || linear.iter().any(|v| !v.is_finite()) {
        return Err("quadratic Coulomb solve overflow");
    }
    if curvature > 0. {
        let common = project(linear.map(|v| -v / curvature), scale);
        for point in points.iter_mut() {
            point.impulse = common.map(|v| v * (point.radius / scale));
        }
        for (&node, r) in free.iter().zip(response) {
            for (axis, &value) in common.iter().enumerate() {
                result.velocities[node][axis] += r * value;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod acceleration_tests {
    use super::{
        Point, QuadraticCoulombImpulse, QuadraticPlaneContact, QuadraticPlaneCoulomb,
        accelerated_solve, diagnostics,
    };
    #[test]
    fn accelerated_dual_recovers_coupled_analytic_stick_slide_solution() {
        // M=[[2,1],[1,2]], contact velocities initially (0.5,0.5).
        // Capacity J1=0.1 saturates: J1=-0.1. Sticking v0=0 then gives
        // J0=-0.8 and v1=0.7. This is an independent two-contact solution.
        let mut points = vec![
            Point {
                nodes: [0; 6],
                shape: [1., 0., 0., 0., 0., 0.],
                weights: vec![1., 0.],
                response: vec![2. / 3., -1. / 3.],
                inverse_mass: 2. / 3.,
                radius: 2.,
                impulse: [0.; 3],
            },
            Point {
                nodes: [1; 6],
                shape: [1., 0., 0., 0., 0., 0.],
                weights: vec![0., 1.],
                response: vec![-1. / 3., 2. / 3.],
                inverse_mass: 2. / 3.,
                radius: 0.1,
                impulse: [0.; 3],
            },
        ];
        let initial = vec![[0.5, 0., 0.]; 2];
        let mut report = QuadraticCoulombImpulse {
            velocities: initial.clone(),
            nodal_impulse_n_s: vec![[0.; 3]; 2],
            support_impulse_n_s: vec![[0.; 3]; 2],
            iterations: 0,
            residual_m_s: 0.,
            endpoint_dissipated_j: 0.,
            numerical_dissipated_j: 0.,
            energy_defect_j: 0.,
        };
        let plane = QuadraticPlaneContact::new([0., 1., 0.], 0., 1.).unwrap();
        let law = QuadraticPlaneCoulomb::new(1., 1e-10, 1000, 100).unwrap();
        assert!(
            accelerated_solve(&mut points, &[0, 1], plane, law, &initial, &mut report).unwrap()
        );
        assert!((points[0].impulse[0] + 0.8).abs() < 1e-8);
        assert!((points[1].impulse[0] + 0.1).abs() < 1e-8);
        assert!(report.velocities[0][0].abs() < 1e-8);
        assert!((report.velocities[1][0] - 0.7).abs() < 1e-8);
        diagnostics(
            &points,
            &[0, 1],
            &[vec![2., 1.], vec![1., 2.]],
            &initial,
            plane,
            &mut report,
        )
        .unwrap();
        assert!((report.endpoint_dissipated_j - 0.07).abs() < 1e-8);
        assert!((report.numerical_dissipated_j - 0.19).abs() < 1e-8);
        assert!(report.energy_defect_j.abs() < 1e-8);
    }
}
