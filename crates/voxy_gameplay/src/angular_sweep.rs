//! Conservative advancement over authored rigid trajectories and angular paths.
use super::{PhysicsError, convex::AffineBox};
use glam::{DQuat, DVec3};

#[derive(Debug)]
pub(crate) struct Hit {
    pub fraction: f64,
    pub normal: Option<DVec3>,
}

// Exact extrema of sum_i |a_i cos(t) + b_i sin(t) + c_i| on a bounded arc.
// Split at sign changes, then evaluate endpoints and stationary points.
fn support_max(edges: [DVec3; 3], axis: DVec3, normal: DVec3, angle: f64) -> f64 {
    let end = angle.min(std::f64::consts::TAU);
    let terms = edges.map(|edge| {
        let axial = axis * edge.dot(axis);
        [
            normal.dot(edge - axial),
            normal.dot(axis.cross(edge)),
            normal.dot(axial),
        ]
    });
    let evaluate = |t: f64| {
        terms
            .iter()
            .map(|[a, b, c]| (a * t.cos() + b * t.sin() + c).abs())
            .sum::<f64>()
    };
    let mut cuts = vec![0., end];
    for [a, b, c] in terms {
        let radius = a.hypot(b);
        if radius == 0. || c.abs() > radius {
            continue;
        }
        let offset = b.atan2(a);
        let root = (-c / radius).clamp(-1., 1.).acos();
        for base in [offset - root, offset + root] {
            for period in -1..=2 {
                let t = base + f64::from(period) * std::f64::consts::TAU;
                if t > 0. && t < end {
                    cuts.push(t);
                }
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let mut maximum = cuts.iter().copied().map(evaluate).fold(0_f64, f64::max);
    for interval in cuts.windows(2) {
        let middle = (interval[0] + interval[1]) * 0.5;
        let (mut a, mut b) = (0., 0.);
        for [first, second, constant] in terms {
            let sign = if first * middle.cos() + second * middle.sin() + constant >= 0. {
                1.
            } else {
                -1.
            };
            a += sign * first;
            b += sign * second;
        }
        for period in -2..=4 {
            let t = b.atan2(a) + f64::from(period) * std::f64::consts::PI;
            if t > interval[0] && t < interval[1] {
                maximum = maximum.max(evaluate(t));
            }
        }
    }
    maximum + 64. * f64::EPSILON * edges.iter().map(|edge| edge.length()).sum::<f64>()
}

pub(crate) fn sweep(
    center: DVec3,
    edges: [DVec3; 3],
    angular: DVec3,
    boxes: &[AffineBox],
    iterations: usize,
) -> Result<Hit, PhysicsError> {
    let angle = angular.length();
    if angle == 0. || boxes.is_empty() {
        return Ok(Hit {
            fraction: 1.,
            normal: None,
        });
    }
    let axis = angular / angle;
    let mut radius = 0_f64;
    let mut rotation_radius = 0_f64;
    for x in [-1., 1.] {
        for y in [-1., 1.] {
            for z in [-1., 1.] {
                let corner = edges[0] * x + edges[1] * y + edges[2] * z;
                radius = radius.max(corner.length());
                rotation_radius = rotation_radius.max(corner.cross(axis).length());
            }
        }
    }
    let mut candidates = Vec::new();
    for obstacle in boxes {
        let relative = center - obstacle.center;
        let epsilon = 128.
            * f64::EPSILON
            * (1. + center.abs().max_element() + obstacle.center.abs().max_element() + radius);
        let separated = obstacle.axes_for(edges).any(|normal| {
            let space = relative.dot(normal).abs() - obstacle.radius(normal);
            space >= radius || space - support_max(edges, axis, normal, angle) >= -epsilon
        });
        if !separated {
            candidates.push(obstacle);
        }
    }
    let mut steps = iterations;
    let mut queries = usize::MAX;
    advance(
        &candidates,
        |time| {
            let rotation = DQuat::from_axis_angle(axis, angle * time);
            Ok((center, edges.map(|edge| rotation * edge)))
        },
        angle * rotation_radius,
        radius,
        rotation_radius,
        &mut steps,
        &mut queries,
    )
}

fn query(remaining: &mut usize) -> Result<(), PhysicsError> {
    *remaining = remaining.checked_sub(1).ok_or(PhysicsError::SweepBudget)?;
    Ok(())
}

// One advancement kernel serves constant-axis arcs and complete cubic spans.
#[allow(clippy::too_many_arguments)]
fn advance(
    candidates: &[&AffineBox],
    sample: impl Fn(f64) -> Result<(DVec3, [DVec3; 3]), PhysicsError>,
    speed_bound: f64,
    radius: f64,
    rotation_radius: f64,
    steps: &mut usize,
    queries: &mut usize,
) -> Result<Hit, PhysicsError> {
    if candidates.is_empty() || speed_bound == 0. {
        return Ok(Hit {
            fraction: 1.,
            normal: None,
        });
    }
    let mut time = 0.;
    while *steps > 0 {
        *steps -= 1;
        let (center, current) = sample(time)?;
        let mut distance = f64::INFINITY;
        let mut contact = DVec3::ZERO;
        let mut tolerance = 0.;
        for obstacle in candidates {
            query(queries)?;
            let relative = center - obstacle.center;
            let mut separation = f64::NEG_INFINITY;
            let mut normal = DVec3::ZERO;
            for axis in obstacle.axes_for(current) {
                let gap = relative.dot(axis).abs()
                    - obstacle.radius(axis)
                    - current.iter().map(|edge| edge.dot(axis).abs()).sum::<f64>();
                if gap > separation {
                    separation = gap;
                    normal = axis * if relative.dot(axis) < 0. { -1. } else { 1. };
                }
            }
            if separation < distance {
                distance = separation;
                contact = normal;
                tolerance = 128.
                    * f64::EPSILON
                    * (1.
                        + center.abs().max_element()
                        + obstacle.center.abs().max_element()
                        + radius)
                    + 1e-8 * rotation_radius;
            }
        }
        if distance <= tolerance {
            return Ok(Hit {
                fraction: time,
                normal: Some(contact),
            });
        }
        // A projection gap is a lower bound on Euclidean separation. Every body
        // point travels at most speed_bound over the normalized unit interval.
        let next = (time + 0.8 * distance / speed_bound).min(1.);
        if next >= 1. {
            return Ok(Hit {
                fraction: 1.,
                normal: None,
            });
        }
        if next <= time {
            return Err(PhysicsError::SweepBudget);
        }
        time = next;
    }
    Err(PhysicsError::SweepBudget)
}

#[derive(Debug)]
pub(crate) struct PathHit {
    pub displacement: DVec3,
    pub rotation: DQuat,
    pub normal: Option<DVec3>,
    pub completed_spans: usize,
    pub span_fraction: f64,
    pub path_fraction: f64,
    pub complete: bool,
    pub advancement_iterations: usize,
    pub trajectory_queries: usize,
}

fn corners(edges: [DVec3; 3]) -> [DVec3; 8] {
    std::array::from_fn(|index| {
        edges[0] * if index & 1 == 0 { -1. } else { 1. }
            + edges[1] * if index & 2 == 0 { -1. } else { 1. }
            + edges[2] * if index & 4 == 0 { -1. } else { 1. }
    })
}

/// Steps and queries are shared over all spans, never reset at a key boundary.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sweep_path(
    center: DVec3,
    rest_edges: [DVec3; 3],
    orientation: DQuat,
    path: &voxy_animation::RootRotationPath,
    basis: DQuat,
    pivot: DVec3,
    boxes: &[AffineBox],
    iterations: usize,
    queries: &mut usize,
) -> Result<PathHit, PhysicsError> {
    let initial_queries = *queries;
    let displacement = |rotation: DQuat| orientation * (pivot - rotation * pivot);
    let anchor = center + orientation * pivot;
    if boxes.is_empty() {
        *queries = queries
            .checked_sub(path.spans().len())
            .ok_or(PhysicsError::SweepBudget)?;
        return Ok(PathHit {
            displacement: displacement(
                (basis * path.end_rotation() * basis.conjugate()).normalize(),
            ),
            rotation: (basis * path.end_rotation() * basis.conjugate()).normalize(),
            normal: None,
            completed_spans: path.spans().len(),
            span_fraction: 1.,
            path_fraction: 1.,
            complete: true,
            advancement_iterations: 0,
            trajectory_queries: path.spans().len(),
        });
    }
    let vertices = corners(rest_edges).map(|vertex| basis.conjugate() * (vertex - pivot));
    let radius = vertices.iter().map(|v| v.length()).fold(0_f64, f64::max);
    let left = (orientation * basis).normalize();
    let mut steps = iterations;
    for (index, span) in path.spans().iter().enumerate() {
        query(queries)?;
        let sample = |fraction| -> Result<(DVec3, [DVec3; 3]), PhysicsError> {
            let rotation = (left
                * span
                    .sample(fraction)
                    .map_err(|_| PhysicsError::InvalidMotion)?
                * basis.conjugate())
            .normalize();
            Ok((
                anchor - rotation * pivot,
                rest_edges.map(|edge| rotation * edge),
            ))
        };
        let (_, initial) = sample(0.)?;
        let duration = span.end() - span.start();
        let angular_travel = span.angular_speed_bound().map_or_else(
            || {
                span.body_angular_displacement()
                    .map_or(0., |axis| axis.length())
            },
            |speed| speed * duration,
        );
        let mut speed_bound = 0_f64;
        for vertex in vertices {
            let speed = span
                .point_speed_bound(vertex)
                .map_err(|_| PhysicsError::InvalidMotion)?;
            let travel = speed.map_or_else(
                || {
                    span.body_angular_displacement()
                        .map_or(0., |axis| axis.cross(vertex).length())
                },
                |speed| speed * duration,
            );
            speed_bound = speed_bound.max(travel);
        }
        let motion_radius = if angular_travel > 0. {
            (speed_bound / angular_travel).min(radius)
        } else {
            0.
        };
        let mut candidates = Vec::new();
        if speed_bound > 0. {
            for obstacle in boxes {
                query(queries)?;
                let relative = anchor - obstacle.center;
                let epsilon = 4096.
                    * f64::EPSILON
                    * (1.
                        + center.abs().max_element()
                        + obstacle.center.abs().max_element()
                        + radius);
                let mut separated = false;
                for normal in obstacle.axes_for(initial) {
                    let space = relative.dot(normal).abs() - obstacle.radius(normal);
                    if space >= radius {
                        separated = true;
                        break;
                    }
                    let mut support = f64::NEG_INFINITY;
                    for vertex in vertices {
                        let bounds = span
                            .projection_bounds(vertex, left.conjugate() * normal)
                            .map_err(|_| PhysicsError::InvalidMotion)?;
                        support = support.max(bounds[0].abs().max(bounds[1].abs()));
                    }
                    if space - support >= -epsilon {
                        separated = true;
                        break;
                    }
                }
                if !separated {
                    candidates.push(obstacle);
                }
            }
        }
        let hit = advance(
            &candidates,
            sample,
            speed_bound,
            radius,
            motion_radius,
            &mut steps,
            queries,
        )?;
        if hit.fraction < 1. {
            let accepted_time = span.start() + duration * hit.fraction;
            return Ok(PathHit {
                displacement: sample(hit.fraction)?.0 - center,
                rotation: (basis
                    * span
                        .sample(hit.fraction)
                        .map_err(|_| PhysicsError::InvalidMotion)?
                    * basis.conjugate())
                .normalize(),
                normal: hit.normal,
                completed_spans: index,
                span_fraction: hit.fraction,
                path_fraction: if path.duration() > 0. {
                    (accepted_time / path.duration()).clamp(0., 1.)
                } else {
                    1.
                },
                complete: false,
                advancement_iterations: iterations - steps,
                trajectory_queries: initial_queries - *queries,
            });
        }
    }
    Ok(PathHit {
        displacement: displacement((basis * path.end_rotation() * basis.conjugate()).normalize()),
        rotation: (basis * path.end_rotation() * basis.conjugate()).normalize(),
        normal: None,
        completed_spans: path.spans().len(),
        span_fraction: 1.,
        path_fraction: 1.,
        complete: true,
        advancement_iterations: iterations - steps,
        trajectory_queries: initial_queries - *queries,
    })
}

/// Collision admission for simultaneous authored translation and rotation.
/// The source frame is conjugated as A M A^-1, including its translated origin.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sweep_rigid_path(
    center: DVec3,
    rest_edges: [DVec3; 3],
    orientation: DQuat,
    path: &voxy_animation::RootRigidPath,
    basis: DQuat,
    origin: DVec3,
    scale: f64,
    boxes: &[AffineBox],
    iterations: usize,
    queries: &mut usize,
) -> Result<PathHit, PhysicsError> {
    let initial_queries = *queries;
    let left = (orientation * basis).normalize();
    let anchor = center + orientation * origin;
    let vertices = corners(rest_edges).map(|v| (basis.conjugate() * (v - origin)) / scale);
    let radius = rest_edges.iter().map(|e| e.length()).sum::<f64>();
    let transform = |motion: voxy_animation::RootRigidTransform| {
        let rotation = (basis * motion.rotation * basis.conjugate()).normalize();
        let displacement =
            orientation * (scale * (basis * motion.translation) + origin - rotation * origin);
        (displacement, rotation)
    };
    let mut steps = iterations;
    for (index, span) in path.spans().iter().enumerate() {
        query(queries)?;
        let sample = |fraction| -> Result<(DVec3, [DVec3; 3]), PhysicsError> {
            let motion = span
                .sample(fraction)
                .map_err(|_| PhysicsError::InvalidMotion)?;
            let (displacement, rotation) = transform(motion);
            Ok((
                center + displacement,
                rest_edges.map(|e| orientation * rotation * e),
            ))
        };
        let (_, initial) = sample(0.)?;
        let mut speed_bound = 0_f64;
        for vertex in vertices {
            speed_bound = speed_bound.max(
                span.point_speed_bound(vertex)
                    .map_err(|_| PhysicsError::InvalidMotion)?,
            );
        }
        speed_bound *= scale.abs();
        if !speed_bound.is_finite() {
            return Err(PhysicsError::InvalidMotion);
        }
        let mut candidates = Vec::new();
        if speed_bound > 0. {
            for obstacle in boxes {
                query(queries)?;
                let epsilon = 16384.
                    * f64::EPSILON
                    * (1.
                        + anchor.abs().max_element()
                        + obstacle.center.abs().max_element()
                        + radius);
                let mut separated = false;
                for normal in obstacle.axes_for(initial) {
                    let offset = (anchor - obstacle.center).dot(normal);
                    let mut lower = f64::INFINITY;
                    let mut upper = f64::NEG_INFINITY;
                    for vertex in vertices {
                        let bounds = span
                            .projection_bounds(vertex, scale * (left.conjugate() * normal))
                            .map_err(|_| PhysicsError::InvalidMotion)?;
                        lower = lower.min(bounds[0] + offset);
                        upper = upper.max(bounds[1] + offset);
                    }
                    let support = obstacle.radius(normal);
                    if lower >= support - epsilon || upper <= -support + epsilon {
                        separated = true;
                        break;
                    }
                }
                if !separated {
                    candidates.push(obstacle);
                }
            }
        }
        let hit = advance(
            &candidates,
            sample,
            speed_bound,
            radius,
            radius,
            &mut steps,
            queries,
        )?;
        if hit.fraction < 1. {
            let accepted = span
                .sample(hit.fraction)
                .map_err(|_| PhysicsError::InvalidMotion)?;
            let (displacement, rotation) = transform(accepted);
            let time = span.start() + (span.end() - span.start()) * hit.fraction;
            return Ok(PathHit {
                displacement,
                rotation,
                normal: hit.normal,
                completed_spans: index,
                span_fraction: hit.fraction,
                path_fraction: if path.duration() > 0. {
                    (time / path.duration()).clamp(0., 1.)
                } else {
                    1.
                },
                complete: false,
                advancement_iterations: iterations - steps,
                trajectory_queries: initial_queries - *queries,
            });
        }
    }
    let (displacement, rotation) = transform(path.end_transform());
    Ok(PathHit {
        displacement,
        rotation,
        normal: None,
        completed_spans: path.spans().len(),
        span_fraction: 1.,
        path_fraction: 1.,
        complete: true,
        advancement_iterations: iterations - steps,
        trajectory_queries: initial_queries - *queries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn body() -> [DVec3; 3] {
        [DVec3::X * 0.4, DVec3::Y * 0.1, DVec3::Z * 0.02]
    }
    fn wall() -> AffineBox {
        AffineBox {
            center: DVec3::Z * 0.25,
            edges: [DVec3::X * 2., DVec3::Y * 2., DVec3::Z * 0.02],
        }
    }
    #[test]
    fn middle_of_half_and_full_turn_hits_even_when_endpoints_are_clear() {
        let edges = body();
        let obstacle = wall();
        let angle = (0.23 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
        for turn in [
            std::f64::consts::PI,
            std::f64::consts::TAU,
            4. * std::f64::consts::TAU,
        ] {
            let final_edges = edges.map(|edge| DQuat::from_rotation_y(turn) * edge);
            assert!(obstacle.penetration_affine(DVec3::ZERO, edges).is_none());
            assert!(
                obstacle
                    .penetration_affine(DVec3::ZERO, final_edges)
                    .is_none()
            );
            let hit = sweep(DVec3::ZERO, edges, DVec3::Y * turn, &[obstacle], 256).unwrap();
            assert!((hit.fraction * turn - angle).abs() < 1e-7, "{hit:?}");
            assert!(hit.fraction * turn <= angle);
            assert!(
                obstacle
                    .penetration_affine(
                        DVec3::ZERO,
                        edges.map(|edge| DQuat::from_rotation_y(turn * hit.fraction) * edge)
                    )
                    .is_none()
            );
        }
    }
    #[test]
    fn grounded_yaw_and_turning_away_are_certified_over_the_whole_arc() {
        let floor = AffineBox {
            center: DVec3::Y * -0.1,
            edges: [DVec3::X * 2., DVec3::Y * 0.1, DVec3::Z * 2.],
        };
        assert_eq!(
            sweep(
                DVec3::Y * 0.1,
                body(),
                DVec3::Y * 4. * std::f64::consts::TAU,
                &[floor],
                1
            )
            .unwrap()
            .fraction,
            1.
        );
        let obstacle = wall();
        let angle = (0.23 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
        let edges = body().map(|edge| DQuat::from_rotation_y(angle) * edge);
        assert_eq!(
            sweep(DVec3::ZERO, edges, -DVec3::Y * angle, &[obstacle], 1)
                .unwrap()
                .fraction,
            1.
        );
    }
    #[test]
    fn projection_extrema_bound_independent_quaternion_samples() {
        for index in 0..20 {
            let axis = DVec3::new(0.3 + f64::from(index) * 0.01, 0.7, -0.2).normalize();
            let normal = DVec3::new(-0.4, 0.1 + f64::from(index) * 0.03, 0.8).normalize();
            let angle = 0.1 + f64::from(index) * 0.7;
            let edges = [DVec3::new(0.4, 0.1, -0.03), DVec3::Y * 0.1, DVec3::Z * 0.02];
            let upper = support_max(edges, axis, normal, angle);
            let mut observed = 0_f64;
            for step in 0..=2000 {
                let rotation = DQuat::from_axis_angle(axis, angle * f64::from(step) / 2000.);
                let support = edges
                    .iter()
                    .map(|edge| (rotation * *edge).dot(normal).abs())
                    .sum::<f64>();
                assert!(support <= upper + 1e-14);
                observed = observed.max(support);
            }
            assert!(upper - observed < 1e-5);
        }
    }
    #[test]
    fn budget_failure_is_explicit_and_tall_yaw_uses_perpendicular_radius() {
        assert!(matches!(
            sweep(
                DVec3::ZERO,
                body(),
                DVec3::Y * std::f64::consts::PI,
                &[wall()],
                1
            ),
            Err(PhysicsError::SweepBudget)
        ));
        let mut edges = body();
        edges[1] = DVec3::Y * 1000.;
        let hit = sweep(
            DVec3::ZERO,
            edges,
            DVec3::Y * std::f64::consts::PI,
            &[wall()],
            256,
        )
        .unwrap();
        assert!((0.1..0.3).contains(&hit.fraction));
    }
}
