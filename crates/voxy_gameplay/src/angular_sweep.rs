//! Conservative advancement over authored rigid trajectories and angular paths.
use super::{
    PhysicsError,
    convex::{AffineBox, reframe_rotation, rotate_vector, rotation_coordinate_preimage},
};
use glam::{DQuat, DVec3};
mod exact_gap;
mod rigid_pair;
pub(crate) use rigid_pair::{RigidBoxMotion, sweep_nominal_rigid_contact, sweep_rigid_pair};
mod gap;
type PointBoxes = [[[f64; 2]; 3]; 8];

/// Final stored-pose gate after grounding/relocation and scene narrowing.
pub(crate) fn certify_published_pose(
    center: DVec3,
    edges: [DVec3; 3],
    boxes: &[AffineBox],
    queries: &mut usize,
) -> Result<(), PhysicsError> {
    gap::certify_pose(center, edges, boxes, queries)
}

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
            Ok((center, edges.map(|edge| rotate_vector(rotation, edge))))
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
    advance_with_clearance(
        candidates,
        sample,
        speed_bound,
        radius,
        rotation_radius,
        0.,
        steps,
        queries,
    )
}

// A uniform Hausdorff envelope inflates every nominal pose by this radius.
// Advancing by the nominal speed is sufficient: the fixed envelope's boundary
// moves with that speed too. This is not a true-contact witness or an evaluation
// error certificate; callers must prove the envelope before physical admission.
#[allow(clippy::too_many_arguments)]
fn advance_with_clearance(
    candidates: &[&AffineBox],
    sample: impl Fn(f64) -> Result<(DVec3, [DVec3; 3]), PhysicsError>,
    speed_bound: f64,
    radius: f64,
    rotation_radius: f64,
    clearance: f64,
    steps: &mut usize,
    queries: &mut usize,
) -> Result<Hit, PhysicsError> {
    advance_with_enclosures(
        candidates,
        sample,
        speed_bound,
        radius,
        rotation_radius,
        0.,
        clearance,
        steps,
        queries,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn advance_with_enclosures(
    candidates: &[&AffineBox],
    sample: impl Fn(f64) -> Result<(DVec3, [DVec3; 3]), PhysicsError>,
    speed_bound: f64,
    radius: f64,
    rotation_radius: f64,
    contact_reserve: f64,
    clearance: f64,
    steps: &mut usize,
    queries: &mut usize,
    point_sample: Option<&dyn Fn(f64) -> Result<PointBoxes, PhysicsError>>,
    contact_tolerance: Option<f64>,
) -> Result<Hit, PhysicsError> {
    if contact_tolerance.is_some_and(|v| !v.is_finite() || v < 0.)
        || !clearance.is_finite()
        || clearance < 0.
        || !contact_reserve.is_finite()
        || contact_reserve < 0.
    {
        return Err(PhysicsError::InvalidMotion);
    }
    if candidates.is_empty() || (speed_bound == 0. && clearance == 0. && point_sample.is_none()) {
        return Ok(Hit {
            fraction: 1.,
            normal: None,
        });
    }
    let mut time = 0.;
    while *steps > 0 {
        *steps -= 1;
        let (center, current) = sample(time)?;
        let enclosed_points = point_sample.map(|sample| sample(time)).transpose()?;
        let mut distance = f64::INFINITY;
        let mut contact = DVec3::ZERO;
        let mut tolerance = 0.;
        for obstacle in candidates {
            query(queries)?;
            let relative = center - obstacle.center;
            let mut separation = f64::NEG_INFINITY;
            let mut normal = DVec3::ZERO;
            for axis in obstacle.axes_for(current) {
                let gap = if let Some(points) = &enclosed_points {
                    gap::lower_points(points, obstacle, axis, clearance)?
                } else if clearance > 0. || contact_tolerance.is_some() {
                    gap::lower(center, current, obstacle, axis, clearance)?
                } else {
                    relative.dot(axis).abs()
                        - obstacle.radius(axis)
                        - current.iter().map(|edge| edge.dot(axis).abs()).sum::<f64>()
                };
                if gap > separation {
                    separation = gap;
                    normal = axis * if relative.dot(axis) < 0. { -1. } else { 1. };
                }
            }
            if separation < distance {
                distance = separation;
                contact = normal;
                tolerance = contact_tolerance.unwrap_or(
                    128. * f64::EPSILON
                        * (1.
                            + center.abs().max_element()
                            + obstacle.center.abs().max_element()
                            + radius)
                        + 1e-8 * rotation_radius
                        + contact_reserve,
                );
            }
        }
        if distance <= tolerance {
            return Ok(Hit {
                fraction: time,
                normal: Some(contact),
            });
        }
        if speed_bound == 0. {
            return Ok(Hit {
                fraction: 1.,
                normal: None,
            });
        }
        // A projection gap is a lower bound on Euclidean separation. Every body
        // point travels at most speed_bound over the normalized unit interval.
        let next = if clearance > 0. || point_sample.is_some() || contact_tolerance.is_some() {
            gap::advance_time(time, (distance - contact_reserve).max(0.), speed_bound)?
        } else {
            (time + 0.8 * (distance - contact_reserve).max(0.) / speed_bound).min(1.)
        };
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

/// Immutable world-corner enclosure for one accepted canonical prefix.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CanonicalWorldPose {
    corners: PointBoxes,
}
impl CanonicalWorldPose {
    pub(crate) fn evaluation_error(
        &self,
        center: DVec3,
        edges: [DVec3; 3],
    ) -> Result<([f64; 3], f64), PhysicsError> {
        gap::world_pose_error(self.corners, center, edges)
    }
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
    pub pose_evaluation_error: Option<([f64; 3], f64)>,
    pub canonical_world_pose: Option<CanonicalWorldPose>,
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
    let displacement =
        |rotation: DQuat| rotate_vector(orientation, pivot - rotate_vector(rotation, pivot));
    let anchor = center + rotate_vector(orientation, pivot);
    if boxes.is_empty() {
        *queries = queries
            .checked_sub(path.spans().len())
            .ok_or(PhysicsError::SweepBudget)?;
        return Ok(PathHit {
            displacement: displacement(reframe_rotation(basis, path.end_rotation())),
            rotation: reframe_rotation(basis, path.end_rotation()),
            normal: None,
            completed_spans: path.spans().len(),
            span_fraction: 1.,
            path_fraction: 1.,
            complete: true,
            advancement_iterations: 0,
            trajectory_queries: path.spans().len(),
            pose_evaluation_error: None,
            canonical_world_pose: None,
        });
    }
    let vertices =
        corners(rest_edges).map(|vertex| rotate_vector(basis.conjugate(), vertex - pivot));
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
                anchor - rotate_vector(rotation, pivot),
                rest_edges.map(|edge| rotate_vector(rotation, edge)),
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
                            .projection_bounds(vertex, rotate_vector(left.conjugate(), normal))
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
                pose_evaluation_error: None,
                canonical_world_pose: None,
            });
        }
    }
    Ok(PathHit {
        displacement: displacement(reframe_rotation(basis, path.end_rotation())),
        rotation: reframe_rotation(basis, path.end_rotation()),
        normal: None,
        completed_spans: path.spans().len(),
        span_fraction: 1.,
        path_fraction: 1.,
        complete: true,
        advancement_iterations: iterations - steps,
        trajectory_queries: initial_queries - *queries,
        pose_evaluation_error: None,
        canonical_world_pose: None,
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
    sweep_rigid_path_with_clearance(
        center,
        rest_edges,
        orientation,
        path,
        basis,
        origin,
        scale,
        0.,
        boxes,
        iterations,
        queries,
    )
}

fn rigid_vertices(
    edges: [DVec3; 3],
    basis: DQuat,
    origin: DVec3,
    scale: f64,
) -> Result<[DVec3; 8], PhysicsError> {
    if !basis.is_finite()
        || !basis.is_normalized()
        || !origin.is_finite()
        || !scale.is_finite()
        || scale == 0.
        || edges.iter().any(|v| !v.is_finite())
    {
        return Err(PhysicsError::InvalidMotion);
    }
    let vertices = corners(edges).map(|v| rotate_vector(basis.conjugate(), v - origin) / scale);
    if vertices.iter().any(|v| !v.is_finite()) {
        return Err(PhysicsError::InvalidMotion);
    }
    Ok(vertices)
}

fn rigid_vertex_enclosures(
    edges: [DVec3; 3],
    basis: DQuat,
    origin: DVec3,
    scale: f64,
) -> Result<[[[f64; 2]; 3]; 8], PhysicsError> {
    let frame =
        voxy_animation::RootRigidEnclosure::from_transform(voxy_animation::RootRigidTransform {
            translation: origin,
            rotation: basis,
        })
        .map_err(|_| PhysicsError::InvalidMotion)?;
    let mut vertices = [[[0.; 2]; 3]; 8];
    for signs in 0..8 {
        let points = std::array::from_fn::<_, 3, _>(|i| {
            if signs & (1 << i) == 0 {
                -edges[i]
            } else {
                edges[i]
            }
        });
        vertices[signs] = frame
            .inverse_similarity_point_sum_bounds(&points, scale)
            .map_err(|_| PhysicsError::InvalidMotion)?;
    }
    Ok(vertices)
}

fn rigid_world_frame(
    center: DVec3,
    orientation: DQuat,
    basis: DQuat,
    origin: DVec3,
) -> Result<voxy_animation::RootRigidEnclosure, PhysicsError> {
    let actor =
        voxy_animation::RootRigidEnclosure::from_transform(voxy_animation::RootRigidTransform {
            translation: center,
            rotation: orientation,
        })
        .map_err(|_| PhysicsError::InvalidMotion)?;
    let source =
        voxy_animation::RootRigidEnclosure::from_transform(voxy_animation::RootRigidTransform {
            translation: origin,
            rotation: basis,
        })
        .map_err(|_| PhysicsError::InvalidMotion)?;
    actor
        .compose(&source)
        .map_err(|_| PhysicsError::InvalidMotion)
}

/// Rounding-only bounds for all body vertices throughout the canonical path.
/// Earlier path evaluation/compiler errors must be accounted for separately.
#[cfg_attr(not(test), allow(dead_code))]
fn rigid_path_publication_errors(
    center: DVec3,
    edges: [DVec3; 3],
    orientation: DQuat,
    cache: &voxy_animation::RootScrewEnclosurePath<'_>,
    basis: DQuat,
    origin: DVec3,
    scale: f64,
) -> Result<([f64; 3], f64), PhysicsError> {
    let frame = rigid_world_frame(center, orientation, basis, origin)?;
    let mut axes = [0_f64; 3];
    let mut radius = 0_f64;
    for vertex in rigid_vertex_enclosures(edges, basis, origin, scale)? {
        let source = cache
            .whole_path_point_box_bounds(vertex)
            .map_err(|_| PhysicsError::InvalidMotion)?;
        let world = frame
            .similarity_point_box_bounds(source, scale)
            .map_err(|_| PhysicsError::InvalidMotion)?;
        let (error, whole) =
            voxy_animation::RootRigidEnclosure::enclosed_f32_publication_error(world)
                .map_err(|_| PhysicsError::InvalidMotion)?;
        for axis in 0..3 {
            axes[axis] = axes[axis].max(error[axis]);
        }
        radius = radius.max(whole);
    }
    Ok((axes, radius))
}

fn rigid_body_clearance(
    approximation: &voxy_animation::RootRigidApproximation,
    edges: [DVec3; 3],
    basis: DQuat,
    origin: DVec3,
    scale: f64,
    evaluation_radius: f64,
) -> Result<f64, PhysicsError> {
    if !evaluation_radius.is_finite() || evaluation_radius < 0. {
        return Err(PhysicsError::InvalidMotion);
    }
    let mut error = 0_f64;
    for vertex in rigid_vertex_enclosures(edges, basis, origin, scale)? {
        error = error.max(
            approximation
                .enclosed_world_point_box_error_bound(vertex, scale, evaluation_radius)
                .map_err(|_| PhysicsError::InvalidMotion)?,
        );
    }
    Ok(error)
}

/// Sweep of an approximate field with automatically derived controller pose
/// evaluation bounds. evaluation_radius is an additional world allowance.
/// Source-field approximation error is carried separately by the approximation;
/// grounding and final scene publication are checked at the transaction boundary.
#[cfg_attr(not(test), allow(dead_code))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn sweep_rigid_approximation(
    center: DVec3,
    rest_edges: [DVec3; 3],
    orientation: DQuat,
    approximation: &voxy_animation::RootRigidApproximation,
    basis: DQuat,
    origin: DVec3,
    scale: f64,
    evaluation_radius: f64,
    boxes: &[AffineBox],
    iterations: usize,
    queries: &mut usize,
) -> Result<PathHit, PhysicsError> {
    sweep_rigid_approximation_with_coordinate_certificate(
        center,
        rest_edges,
        orientation,
        approximation,
        basis,
        origin,
        scale,
        evaluation_radius,
        boxes,
        iterations,
        queries,
        None,
    )
}

/// Read-only acceptance query for one immutable assembled fade. The directional
/// proof is constructed from the same owned source domains as the trajectory.
#[cfg_attr(not(test), allow(dead_code))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn sweep_certified_rigid_fade(
    center: DVec3,
    rest_edges: [DVec3; 3],
    orientation: DQuat,
    fade: &voxy_animation::RootRigidCertifiedFadeInterval,
    coordinate_axis: usize,
    basis: DQuat,
    origin: DVec3,
    scale: f64,
    evaluation_radius: f64,
    boxes: &[AffineBox],
    iterations: usize,
    queries: &mut usize,
) -> Result<PathHit, PhysicsError> {
    sweep_certified_rigid_fade_with_axis_errors(
        center,
        rest_edges,
        orientation,
        fade,
        coordinate_axis,
        basis,
        origin,
        scale,
        evaluation_radius,
        None,
        boxes,
        iterations,
        queries,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn sweep_certified_rigid_fade_with_axis_errors(
    center: DVec3,
    rest_edges: [DVec3; 3],
    orientation: DQuat,
    fade: &voxy_animation::RootRigidCertifiedFadeInterval,
    coordinate_axis: usize,
    basis: DQuat,
    origin: DVec3,
    scale: f64,
    evaluation_radius: f64,
    evaluation_axes: Option<[f64; 3]>,
    boxes: &[AffineBox],
    iterations: usize,
    queries: &mut usize,
) -> Result<PathHit, PhysicsError> {
    let certificate = fade
        .coordinate_certificate(coordinate_axis, 4096)
        .map_err(|_| PhysicsError::InvalidMotion)?;
    sweep_rigid_approximation_with_axis_errors(
        center,
        rest_edges,
        orientation,
        fade.approximation(),
        basis,
        origin,
        scale,
        evaluation_radius,
        evaluation_axes.unwrap_or([evaluation_radius; 3]),
        boxes,
        iterations,
        queries,
        certificate.as_ref(),
    )
}

#[cfg_attr(not(test), allow(dead_code))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn sweep_rigid_approximation_with_coordinate_certificate(
    center: DVec3,
    rest_edges: [DVec3; 3],
    orientation: DQuat,
    approximation: &voxy_animation::RootRigidApproximation,
    basis: DQuat,
    origin: DVec3,
    scale: f64,
    evaluation_radius: f64,
    boxes: &[AffineBox],
    iterations: usize,
    queries: &mut usize,
    coordinate_certificate: Option<&voxy_animation::RootRigidCoordinateCertificate<'_>>,
) -> Result<PathHit, PhysicsError> {
    sweep_rigid_approximation_with_axis_errors(
        center,
        rest_edges,
        orientation,
        approximation,
        basis,
        origin,
        scale,
        evaluation_radius,
        [evaluation_radius; 3],
        boxes,
        iterations,
        queries,
        coordinate_certificate,
    )
}

/// Both the whole-body radius and world-axis bounds are caller proof obligations.
/// Axis bounds apply in world coordinates, after actor and authored-frame mapping.
#[allow(clippy::too_many_arguments)]
fn sweep_rigid_approximation_with_axis_errors(
    center: DVec3,
    rest_edges: [DVec3; 3],
    orientation: DQuat,
    approximation: &voxy_animation::RootRigidApproximation,
    basis: DQuat,
    origin: DVec3,
    scale: f64,
    evaluation_radius: f64,
    evaluation_axes: [f64; 3],
    boxes: &[AffineBox],
    iterations: usize,
    queries: &mut usize,
    coordinate_certificate: Option<&voxy_animation::RootRigidCoordinateCertificate<'_>>,
) -> Result<PathHit, PhysicsError> {
    if !evaluation_radius.is_finite()
        || evaluation_radius < 0.
        || evaluation_axes
            .iter()
            .any(|value| !value.is_finite() || *value < 0. || *value > evaluation_radius)
    {
        return Err(PhysicsError::InvalidMotion);
    }
    if coordinate_certificate.is_some_and(|proof| !std::ptr::eq(proof.path(), &approximation.path))
    {
        return Err(PhysicsError::InvalidMotion);
    }
    // Derive the controller's rounding allowance from the same immutable cache
    // used by every candidate evaluation. Caller allowances add to it.
    let enclosed_path = approximation
        .path
        .prepare_screw_enclosures(4096)
        .map_err(|_| PhysicsError::InvalidMotion)?;
    let (mut runtime_axes, runtime_radius) = enclosed_path
        .physical_body_selection_error_bounds(center, rest_edges, basis, origin, orientation, scale)
        .map_err(|_| PhysicsError::InvalidMotion)?;
    for axis in 0..3 {
        if exact_stationary_physical_projection(&enclosed_path, orientation, basis, axis) {
            runtime_axes[axis] = 0.;
        }
    }
    // Exact source fields and zero additional directional uncertainty may use
    // support monotonicity instead of a symmetric error ball. The actual
    // coordinate arithmetic is monotone, including pivot cancellation.
    let exact_projection_axes = std::array::from_fn(|axis| {
        approximation.origin_error_bound == 0.
            && approximation.angular_error_bound == 0.
            && evaluation_axes[axis] == 0.
            && exact_physical_projection_frame(orientation, basis, axis).is_some()
    });
    let evaluation_radius = add_evaluation_allowances(evaluation_radius, runtime_radius)?;
    let mut combined_axes = [0.; 3];
    for axis in 0..3 {
        combined_axes[axis] = add_evaluation_allowances(evaluation_axes[axis], runtime_axes[axis])?;
    }
    let evaluation_axes = combined_axes;
    let coordinate_certificate = coordinate_certificate
        .map(|proof| {
            let mut margins = [0.; 3];
            for axis in 0..3 {
                margins[axis] = proof
                    .enclosed_scaled_error_bound(scale, evaluation_axes[axis])
                    .map_err(|_| PhysicsError::InvalidMotion)?;
            }
            Ok::<_, PhysicsError>((proof, margins))
        })
        .transpose()?;
    let clearance = rigid_body_clearance(
        approximation,
        rest_edges,
        basis,
        origin,
        scale,
        evaluation_radius,
    )?;
    let vertices = rigid_vertex_enclosures(rest_edges, basis, origin, scale)?;

    let speeds = enclosed_path
        .point_speed_bounds(&vertices, scale)
        .map_err(|_| PhysicsError::InvalidMotion)?;
    let mut hit = sweep_rigid_path_with_bounds(
        center,
        rest_edges,
        orientation,
        &approximation.path,
        basis,
        origin,
        scale,
        clearance,
        boxes,
        iterations,
        queries,
        Some(&speeds),
        Some(&enclosed_path),
        coordinate_certificate,
        exact_projection_axes,
    )?;
    // Mirror the physical controller's proposed orientation/edge update. This
    // checks the rounded proposal itself, not merely the canonical field pose.
    let proposed_orientation = (orientation * hit.rotation).normalize();
    let proposed_edges = rest_edges.map(|edge| rotate_vector(proposed_orientation, edge));
    let canonical = if approximation.path.spans().is_empty() {
        voxy_animation::RootRigidEnclosure::IDENTITY
    } else {
        let index = if hit.complete {
            approximation.path.spans().len() - 1
        } else {
            hit.completed_spans
        };
        enclosed_path
            .sample(index, hit.span_fraction)
            .map_err(|_| PhysicsError::InvalidMotion)?
    };
    let frame = rigid_world_frame(center, orientation, basis, origin)?;
    let mut world = [[[0.; 2]; 3]; 8];
    for i in 0..8 {
        let point = canonical
            .transform_point_box_bounds(vertices[i])
            .map_err(|_| PhysicsError::InvalidMotion)?;
        world[i] = frame
            .similarity_point_box_bounds(point, scale)
            .map_err(|_| PhysicsError::InvalidMotion)?;
    }
    let witness = CanonicalWorldPose { corners: world };
    hit.pose_evaluation_error =
        Some(witness.evaluation_error(center + hit.displacement, proposed_edges)?);
    hit.canonical_world_pose = Some(witness);
    let before = *queries;
    gap::certify_pose(center + hit.displacement, proposed_edges, boxes, queries)?;
    hit.trajectory_queries = hit
        .trajectory_queries
        .checked_add(before - *queries)
        .ok_or(PhysicsError::SweepBudget)?;
    Ok(hit)
}

#[allow(clippy::too_many_arguments)]
fn sweep_rigid_path_with_clearance(
    center: DVec3,
    rest_edges: [DVec3; 3],
    orientation: DQuat,
    path: &voxy_animation::RootRigidPath,
    basis: DQuat,
    origin: DVec3,
    scale: f64,
    clearance: f64,
    boxes: &[AffineBox],
    iterations: usize,
    queries: &mut usize,
) -> Result<PathHit, PhysicsError> {
    sweep_rigid_path_with_bounds(
        center,
        rest_edges,
        orientation,
        path,
        basis,
        origin,
        scale,
        clearance,
        boxes,
        iterations,
        queries,
        None,
        None,
        None,
        [false; 3],
    )
}

fn add_evaluation_allowances(a: f64, b: f64) -> Result<f64, PhysicsError> {
    // Preserve an exact zero directional allowance. Positive independent
    // allowances add with outward rounding, never merely take their maximum.
    let sum = if a == 0. {
        b
    } else if b == 0. {
        a
    } else {
        (a + b).next_up()
    };
    if sum.is_finite() {
        Ok(sum)
    } else {
        Err(PhysicsError::InvalidMotion)
    }
}

// A stationary source coordinate survives the actual evaluation chain exactly
// when the basis maps every row structurally. Cached screw arithmetic retains
// zero orthogonal quaternion components and zero coordinate translation. Basis
// reframing therefore retains a single imaginary axis. Right multiplication of
// the actor by this axis quaternion uses at most two nonzero products per
// component; its signed coordinate-row identities have identical products and
// sums (up to sign/order), so normalization preserves them as well. The cross
// evaluator then copies the coordinate directly. Pivot cancellation is exact.
fn exact_physical_projection_frame(
    orientation: DQuat,
    basis: DQuat,
    world_axis: usize,
) -> Option<usize> {
    let (actor_axis, _) = rotation_coordinate_preimage(orientation, world_axis)?;
    let (source_axis, _) = rotation_coordinate_preimage(basis, actor_axis)?;
    let permutation = (0..3).all(|axis| rotation_coordinate_preimage(basis, axis).is_some());
    // A quaternion about this same coordinate (or a half turn orthogonal to
    // it) also maps an axial imaginary vector without off-axis arithmetic.
    let q = basis.to_array();
    let axial = source_axis == actor_axis
        && ((0..3).all(|axis| axis == source_axis || q[axis] == 0.)
            || (q[source_axis] == 0. && q[3] == 0.));
    (permutation || axial).then_some(source_axis)
}
fn exact_stationary_physical_projection(
    cache: &voxy_animation::RootScrewEnclosurePath<'_>,
    orientation: DQuat,
    basis: DQuat,
    world_axis: usize,
) -> bool {
    exact_physical_projection_frame(orientation, basis, world_axis)
        .is_some_and(|axis| cache.coordinate_velocity_range(axis) == Some([0., 0.]))
}

// Structural coordinate-plane certificate for an exact canonical screw field.
// Both real frame rotations must transport an exact coordinate row. Uniform nonzero
// source error clearance requires a matching certificate and outward margin.
// Pure candidate rounding may use the derived exact projection-frame policy.
fn invariant_projection_separates(
    center: DVec3,
    edges: [DVec3; 3],
    obstacle: &AffineBox,
    normal: DVec3,
    orientation: DQuat,
    basis: DQuat,
    scale: f64,
    cache: &voxy_animation::RootScrewEnclosurePath<'_>,
    coordinate_certificate: Option<(
        &voxy_animation::RootRigidCoordinateCertificate<'_>,
        [f64; 3],
    )>,
) -> Result<bool, PhysicsError> {
    let Some(coordinate) = (0..3).find(|i| normal[*i] != 0.) else {
        return Ok(false);
    };
    if (0..3).any(|i| i != coordinate && normal[i] != 0.) {
        return Ok(false);
    }
    let Some((actor_coordinate, actor_sign)) =
        rotation_coordinate_preimage(orientation, coordinate)
    else {
        return Ok(false);
    };
    let Some((source_coordinate, source_sign)) =
        rotation_coordinate_preimage(basis, actor_coordinate)
    else {
        return Ok(false);
    };
    if coordinate_certificate.is_some_and(|proof| proof.0.axis() != source_coordinate) {
        return Ok(false);
    }
    let Some(range) = cache.coordinate_velocity_range(source_coordinate) else {
        return Ok(false);
    };
    let projected_scale = scale * actor_sign * source_sign;
    let away = if center[coordinate] > obstacle.center[coordinate] {
        if projected_scale > 0. {
            range[0] >= 0.
        } else {
            range[1] <= 0.
        }
    } else if center[coordinate] < obstacle.center[coordinate] {
        if projected_scale > 0. {
            range[1] <= 0.
        } else {
            range[0] >= 0.
        }
    } else {
        false
    };
    if !away {
        return Ok(false);
    }
    let margin = coordinate_certificate.map_or(0., |proof| proof.1[coordinate]);
    let separated = if margin == 0. {
        exact_gap::sign(center, edges, obstacle, normal)? >= 0
    } else {
        gap::lower(center, edges, obstacle, normal, margin)? > 0.
    };
    Ok(separated)
}

#[allow(clippy::too_many_arguments)]
fn sweep_rigid_path_with_bounds(
    center: DVec3,
    rest_edges: [DVec3; 3],
    orientation: DQuat,
    path: &voxy_animation::RootRigidPath,
    basis: DQuat,
    origin: DVec3,
    scale: f64,
    clearance: f64,
    boxes: &[AffineBox],
    iterations: usize,
    queries: &mut usize,
    enclosed_speeds: Option<&[f64]>,
    enclosed_path: Option<&voxy_animation::RootScrewEnclosurePath<'_>>,
    coordinate_certificate: Option<(
        &voxy_animation::RootRigidCoordinateCertificate<'_>,
        [f64; 3],
    )>,
    exact_projection_axes: [bool; 3],
) -> Result<PathHit, PhysicsError> {
    if !clearance.is_finite()
        || clearance < 0.
        || !center.is_finite()
        || !orientation.is_finite()
        || !orientation.is_normalized()
    {
        return Err(PhysicsError::InvalidMotion);
    }
    if coordinate_certificate.is_some_and(|proof| !std::ptr::eq(proof.0.path(), path)) {
        return Err(PhysicsError::InvalidMotion);
    }
    if enclosed_speeds.is_some() != enclosed_path.is_some()
        || enclosed_path.is_some_and(|cache| !std::ptr::eq(cache.path(), path))
    {
        return Err(PhysicsError::InvalidMotion);
    }
    if enclosed_speeds.is_some_and(|v| {
        v.len() != path.spans().len() || v.iter().any(|s| !s.is_finite() || *s < 0.)
    }) {
        return Err(PhysicsError::InvalidMotion);
    }
    let world_frame = if enclosed_speeds.is_some() {
        Some(rigid_world_frame(center, orientation, basis, origin)?)
    } else {
        None
    };
    let vertex_boxes = if world_frame.is_some() {
        Some(rigid_vertex_enclosures(rest_edges, basis, origin, scale)?)
    } else {
        None
    };
    let initial_queries = *queries;
    let left = (orientation * basis).normalize();
    let anchor = center + rotate_vector(orientation, origin);
    let vertices = rigid_vertices(rest_edges, basis, origin, scale)?;
    let radius = rest_edges.iter().map(|e| e.length()).sum::<f64>();
    let transform = |motion: voxy_animation::RootRigidTransform| {
        let rotation = reframe_rotation(basis, motion.rotation);
        let displacement = rotate_vector(
            orientation,
            scale * rotate_vector(basis, motion.translation) + origin
                - rotate_vector(rotation, origin),
        );
        (displacement, rotation)
    };
    let evaluated_motion = |index: usize, fraction: f64| {
        match enclosed_path {
            Some(cache) => cache.sample_evaluated(index, fraction).map(|v| v.0),
            None => path
                .spans()
                .get(index)
                .ok_or(voxy_animation::AnimationError::InvalidSampleTime)
                .and_then(|span| span.sample(fraction)),
        }
        .map_err(|_| PhysicsError::InvalidMotion)
    };
    // Prefer a candidate before the f32 grid boundary. This is an early-stop
    // policy, not a certified numeric-error margin; final stored poses are
    // independently checked against every obstacle before publication.
    let contact_reserve = if clearance == 0. && enclosed_path.is_none() {
        0.5 * f64::from(f32::EPSILON) * (anchor.abs().max_element() + radius).max(1.)
    } else {
        // Enclosed/clearance queries carry the derived numeric envelope and
        // any additional caller allowance; avoid a second heuristic reserve.
        0.
    };
    let mut steps = iterations;
    if path.spans().is_empty() && (clearance > 0. || world_frame.is_some()) {
        let candidates: Vec<_> = boxes.iter().collect();
        let point_sample = |_| -> Result<PointBoxes, PhysicsError> {
            let frame = world_frame.as_ref().ok_or(PhysicsError::InvalidMotion)?;
            let input = vertex_boxes.as_ref().ok_or(PhysicsError::InvalidMotion)?;
            let mut output = [[[0.; 2]; 3]; 8];
            for i in 0..8 {
                output[i] = frame
                    .similarity_point_box_bounds(input[i], scale)
                    .map_err(|_| PhysicsError::InvalidMotion)?;
            }
            Ok(output)
        };
        let hit = advance_with_enclosures(
            &candidates,
            |_| {
                Ok((
                    center,
                    rest_edges.map(|edge| rotate_vector(orientation, edge)),
                ))
            },
            0.,
            radius,
            radius,
            contact_reserve,
            clearance,
            &mut steps,
            queries,
            if world_frame.is_some() {
                Some(&point_sample)
            } else {
                None
            },
            None,
        )?;
        if hit.fraction < 1. {
            return Ok(PathHit {
                displacement: DVec3::ZERO,
                rotation: DQuat::IDENTITY,
                normal: hit.normal,
                completed_spans: 0,
                span_fraction: 0.,
                path_fraction: 0.,
                complete: false,
                advancement_iterations: iterations - steps,
                trajectory_queries: initial_queries - *queries,
                pose_evaluation_error: None,
                canonical_world_pose: None,
            });
        }
    }
    for (index, span) in path.spans().iter().enumerate() {
        query(queries)?;
        let sample = |fraction| -> Result<(DVec3, [DVec3; 3]), PhysicsError> {
            let motion = evaluated_motion(index, fraction)?;
            let (displacement, rotation) = transform(motion);
            Ok((
                center + displacement,
                rest_edges.map(|e| rotate_vector((orientation * rotation).normalize(), e)),
            ))
        };
        let point_sample = |fraction| -> Result<PointBoxes, PhysicsError> {
            let motion = enclosed_path
                .ok_or(PhysicsError::InvalidMotion)?
                .sample(index, fraction)
                .map_err(|_| PhysicsError::InvalidMotion)?;
            let frame = world_frame.as_ref().ok_or(PhysicsError::InvalidMotion)?;
            let input = vertex_boxes.as_ref().ok_or(PhysicsError::InvalidMotion)?;
            let mut output = [[[0.; 2]; 3]; 8];
            for i in 0..8 {
                let point = motion
                    .transform_point_box_bounds(input[i])
                    .map_err(|_| PhysicsError::InvalidMotion)?;
                output[i] = frame
                    .similarity_point_box_bounds(point, scale)
                    .map_err(|_| PhysicsError::InvalidMotion)?;
            }
            Ok(output)
        };
        let initial_points = if world_frame.is_some() {
            Some(point_sample(0.)?)
        } else {
            None
        };
        let (initial_center, initial) = sample(0.)?;
        let speed_bound = if let Some(bounds) = enclosed_speeds {
            bounds[index]
        } else {
            let mut bound = 0_f64;
            for vertex in vertices {
                bound = bound.max(
                    span.point_speed_bound(vertex)
                        .map_err(|_| PhysicsError::InvalidMotion)?,
                );
            }
            bound * scale.abs()
        };
        if !speed_bound.is_finite() {
            return Err(PhysicsError::InvalidMotion);
        }
        let mut candidates = Vec::new();
        if speed_bound > 0. || clearance > 0. || initial_points.is_some() {
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
                    let exact_numeric_projection = (0..3).any(|axis| {
                        exact_projection_axes[axis]
                            && normal[axis] != 0.
                            && (0..3).all(|other| other == axis || normal[other] == 0.)
                    });
                    if clearance == 0.
                        || coordinate_certificate.is_some()
                        || exact_numeric_projection
                    {
                        if let Some(cache) = enclosed_path {
                            // The whole field is monotone away from this plane;
                            // exact initial touching is safe throughout it.
                            if invariant_projection_separates(
                                center,
                                rest_edges.map(|e| rotate_vector(orientation, e)),
                                obstacle,
                                normal,
                                orientation,
                                basis,
                                scale,
                                cache,
                                if clearance == 0. || exact_numeric_projection {
                                    None
                                } else {
                                    coordinate_certificate
                                },
                            )? {
                                separated = true;
                                break;
                            }
                        }
                    }
                    if clearance > 0. || initial_points.is_some() {
                        // Every point moves at most speed_bound over this unit
                        // interval. Reject only if the certified initial gap
                        // exceeds that entire excursion; no projection extrema
                        // or normalized-axis assumptions enter this decision.
                        let gap = if let Some(points) = &initial_points {
                            gap::lower_points(points, obstacle, normal, clearance)?
                        } else {
                            gap::lower(initial_center, initial, obstacle, normal, clearance)?
                        };
                        if gap > speed_bound {
                            separated = true;
                            break;
                        }
                        continue;
                    }
                    let offset = (anchor - obstacle.center).dot(normal);
                    let mut lower = f64::INFINITY;
                    let mut upper = f64::NEG_INFINITY;
                    for vertex in vertices {
                        let bounds = span
                            .projection_bounds(
                                vertex,
                                scale * (rotate_vector(left.conjugate(), normal)),
                            )
                            .map_err(|_| PhysicsError::InvalidMotion)?;
                        lower = lower.min(bounds[0] + offset);
                        upper = upper.max(bounds[1] + offset);
                    }
                    let support = obstacle.radius(normal);
                    let separated_axis = lower >= support - epsilon || upper <= -support + epsilon;
                    if separated_axis {
                        separated = true;
                        break;
                    }
                }
                if !separated {
                    candidates.push(obstacle);
                }
            }
        }
        let hit = advance_with_enclosures(
            &candidates,
            sample,
            speed_bound,
            radius,
            radius,
            contact_reserve,
            clearance,
            &mut steps,
            queries,
            if world_frame.is_some() {
                Some(&point_sample)
            } else {
                None
            },
            None,
        )?;
        if hit.fraction < 1. {
            let accepted = evaluated_motion(index, hit.fraction)?;
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
                pose_evaluation_error: None,
                canonical_world_pose: None,
            });
        }
    }
    let end = match path.spans().len().checked_sub(1) {
        Some(index) => evaluated_motion(index, 1.)?,
        None => path.end_transform(),
    };
    let (displacement, rotation) = transform(end);
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
        pose_evaluation_error: None,
        canonical_world_pose: None,
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
    #[test]
    fn clearance_stops_before_nominal_contact_and_checks_stationary_envelopes() {
        let edges = [DVec3::X * 0.1, DVec3::Y * 0.1, DVec3::Z * 0.1];
        let obstacle = AffineBox {
            center: DVec3::X * 2.,
            edges,
        };
        let candidates = [&obstacle];
        let run = |clearance, speed: f64, center: DVec3| {
            advance_with_clearance(
                &candidates,
                |t| Ok((center + DVec3::X * speed * t, edges)),
                speed,
                0.3,
                0.3,
                clearance,
                &mut 256,
                &mut 4096,
            )
        };
        let nominal = run(0., 3., DVec3::ZERO).unwrap();
        let inflated = run(0.2, 3., DVec3::ZERO).unwrap();
        assert!((nominal.fraction - 0.6).abs() < 1e-8);
        assert!((inflated.fraction - (1.6 / 3.)).abs() < 1e-8);
        assert!(inflated.fraction < nominal.fraction);
        // A truly displaced body can reach the wall here, while the nominal
        // body still has a 0.2 gap. The envelope must not advance beyond it.
        let front = 3. * inflated.fraction + 0.1 + 0.2;
        assert!(front <= 1.9 + 1e-8);
        assert!(front >= 1.9 - 1e-8);
        assert_eq!(run(0.2, 0., DVec3::X * 1.7).unwrap().fraction, 0.);
        assert_eq!(run(0.2, 0., DVec3::ZERO).unwrap().fraction, 1.);
        assert!(run(-0.1, 3., DVec3::ZERO).is_err());
        assert!(run(f64::NAN, 3., DVec3::ZERO).is_err());
    }

    #[test]
    fn rigid_envelope_broadphase_retains_contacts_outside_the_nominal_sweep() {
        use voxy_animation::{RootRigidPath, RootRigidTwist};
        let edges = [DVec3::X * 0.1, DVec3::Y * 0.1, DVec3::Z * 0.1];
        let wall = AffineBox {
            center: DVec3::X,
            edges: [DVec3::X * 0.1, DVec3::Y * 2., DVec3::Z * 2.],
        };
        let path = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::X * 0.7,
                    angular: DVec3::ZERO,
                },
                1.,
            )],
            1,
        )
        .unwrap();
        let run = |clearance, queries: &mut usize| {
            sweep_rigid_path_with_clearance(
                DVec3::ZERO,
                edges,
                DQuat::IDENTITY,
                &path,
                DQuat::IDENTITY,
                DVec3::ZERO,
                1.,
                clearance,
                &[wall],
                256,
                queries,
            )
        };
        assert!(run(0., &mut 4096).unwrap().complete);
        let hit = run(0.2, &mut 4096).unwrap();
        assert!(!hit.complete);
        assert!((hit.path_fraction - 0.6 / 0.7).abs() < 1e-8);
        let far = AffineBox {
            center: DVec3::X * 10.,
            ..wall
        };
        let far_hit = sweep_rigid_path_with_clearance(
            DVec3::ZERO,
            edges,
            DQuat::IDENTITY,
            &path,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            0.2,
            &[far],
            0,
            &mut 4096,
        )
        .unwrap();
        assert!(far_hit.complete);
        // The near wall must survive broadphase even when no advancement work
        // is available: budget failure, rather than a falsely complete path.
        assert!(matches!(
            sweep_rigid_path_with_clearance(
                DVec3::ZERO,
                edges,
                DQuat::IDENTITY,
                &path,
                DQuat::IDENTITY,
                DVec3::ZERO,
                1.,
                0.2,
                &[wall],
                0,
                &mut 4096
            ),
            Err(PhysicsError::SweepBudget)
        ));
        assert!((hit.displacement.x - 0.6).abs() < 1e-8);
        assert_eq!(hit.completed_spans, 0);
        assert!(matches!(run(0.2, &mut 1), Err(PhysicsError::SweepBudget)));
        assert!(run(f64::NAN, &mut 4096).is_err());
        let stationary = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::ZERO,
                    angular: DVec3::ZERO,
                },
                1.,
            )],
            1,
        )
        .unwrap();
        let hit = sweep_rigid_path_with_clearance(
            DVec3::X * 0.7,
            edges,
            DQuat::IDENTITY,
            &stationary,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            0.2,
            &[wall],
            256,
            &mut 4096,
        )
        .unwrap();
        assert!(!hit.complete);
        assert_eq!(hit.path_fraction, 0.);
    }

    #[test]
    fn whole_body_clearance_accounts_for_pivot_signed_scale_and_evaluation_error() {
        use voxy_animation::{RootRigidApproximation, RootRigidPath, RootRigidTwist};
        let edges = [DVec3::X * 0.4, DVec3::Y * 0.1, DVec3::Z * 0.02];
        let origin = DVec3::new(0.6, -0.1, 0.2);
        let basis = DQuat::from_rotation_z(0.4);
        let approximation = RootRigidApproximation {
            path: RootRigidPath::from_twists(
                &[(
                    RootRigidTwist {
                        linear: DVec3::ZERO,
                        angular: DVec3::ZERO,
                    },
                    1.,
                )],
                1,
            )
            .unwrap(),
            origin_error_bound: 0.01,
            angular_error_bound: 0.2,
        };
        let physical_radius = corners(edges)
            .into_iter()
            .map(|v| (v - origin).length())
            .fold(0_f64, f64::max);
        for scale in [-2., 0.5, 2.] {
            let clearance =
                rigid_body_clearance(&approximation, edges, basis, origin, scale, 0.003).unwrap();
            let expected = scale.abs() * 0.01 + 2. * (0.1_f64).sin() * physical_radius + 0.003;
            assert!((clearance - expected).abs() < 1e-12);
            for corner in corners(edges) {
                let source = basis.conjugate() * (corner - origin) / scale;
                let actual = scale
                    * (basis * (DQuat::from_rotation_y(0.2) * source + DVec3::Z * 0.01))
                    + origin;
                assert!((actual - corner).length() <= clearance);
            }
        }
        assert!(rigid_body_clearance(&approximation, edges, basis, origin, 0., 0.).is_err());
        assert!(rigid_body_clearance(&approximation, edges, basis, origin, 1., f64::NAN).is_err());
        let moving = RootRigidApproximation {
            path: RootRigidPath::from_twists(
                &[(
                    RootRigidTwist {
                        linear: DVec3::X * 0.7,
                        angular: DVec3::ZERO,
                    },
                    1.,
                )],
                1,
            )
            .unwrap(),
            origin_error_bound: 0.15,
            angular_error_bound: 0.,
        };
        let body = [DVec3::X * 0.1, DVec3::Y * 0.1, DVec3::Z * 0.1];
        let wall = AffineBox {
            center: DVec3::X,
            edges: [DVec3::X * 0.1, DVec3::Y * 2., DVec3::Z * 2.],
        };
        let hit = sweep_rigid_approximation(
            DVec3::ZERO,
            body,
            DQuat::IDENTITY,
            &moving,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            0.05,
            &[wall],
            256,
            &mut 4096,
        )
        .unwrap();
        assert!(!hit.complete);
        assert!((hit.path_fraction - 0.6 / 0.7).abs() < 1e-8);
        let empty = RootRigidApproximation {
            path: RootRigidPath::from_twists(&[], 0).unwrap(),
            origin_error_bound: 0.2,
            angular_error_bound: 0.,
        };
        let hit = sweep_rigid_approximation(
            DVec3::X * 0.7,
            body,
            DQuat::IDENTITY,
            &empty,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            0.,
            &[wall],
            256,
            &mut 4096,
        )
        .unwrap();
        assert!(!hit.complete);
        assert_eq!(hit.path_fraction, 0.);
    }

    #[test]
    fn exact_conditional_screw_motion_preserves_floor_contact_and_jump() {
        use voxy_animation::{RootRigidApproximation, RootRigidPath, RootRigidTwist};
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
        let floor = AffineBox {
            center: DVec3::Y * (-0.5),
            edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
        };
        for vertical in [0., 0.1, -0.1] {
            let path = RootRigidPath::from_twists(
                &[(
                    RootRigidTwist {
                        linear: DVec3::new(0.3, vertical, 0.1),
                        angular: DVec3::Y * 0.4,
                    },
                    0.5,
                )],
                1,
            )
            .unwrap();
            let approximation = RootRigidApproximation {
                path,
                origin_error_bound: 0.,
                angular_error_bound: 0.,
            };
            for scale in [-2., 1.] {
                let hit = sweep_rigid_approximation(
                    DVec3::Y * 0.125,
                    edges,
                    DQuat::from_rotation_y(0.2),
                    &approximation,
                    DQuat::from_rotation_y(0.3),
                    DVec3::new(0.6, 0.2, -0.3),
                    scale,
                    0.,
                    &[floor],
                    256,
                    &mut 4096,
                )
                .unwrap();
                assert_eq!(hit.complete, vertical * scale >= 0.);
                if vertical * scale < 0. {
                    assert_eq!(hit.path_fraction, 0.);
                }
            }
        }
        // A wall is not discarded by the floor certificate.
        let wall = AffineBox {
            center: DVec3::X,
            edges: [DVec3::X * 0.125, DVec3::Y * 2., DVec3::Z * 2.],
        };
        let path = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::X * 1.2,
                    angular: DVec3::ZERO,
                },
                1.,
            )],
            1,
        )
        .unwrap();
        let approximation = RootRigidApproximation {
            path,
            origin_error_bound: 0.,
            angular_error_bound: 0.,
        };
        let hit = sweep_rigid_approximation(
            DVec3::Y * 0.125,
            edges,
            DQuat::IDENTITY,
            &approximation,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            0.,
            &[floor, wall],
            256,
            &mut 4096,
        )
        .unwrap();
        assert!(!hit.complete);
        assert!((hit.path_fraction - 0.75 / 1.2).abs() < 1e-8);
    }

    #[test]
    fn supported_screw_contact_survives_source_and_actor_axis_permutations() {
        use voxy_animation::{RootRigidApproximation, RootRigidPath, RootRigidTwist};
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
        let floor = AffineBox {
            center: DVec3::Y * (-0.5),
            edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
        };
        let quarter = DQuat::from_xyzw(0., 0., 0.5, 0.5).normalize();
        for (actor, basis) in [
            (DQuat::from_rotation_y(0.2), quarter),
            (quarter, DQuat::from_rotation_x(0.3)),
        ] {
            for scale in [-2., 1.] {
                let path = RootRigidPath::from_twists(
                    &[(
                        RootRigidTwist {
                            linear: DVec3::new(0., 0.3, 0.1),
                            angular: DVec3::X * 0.4,
                        },
                        0.5,
                    )],
                    1,
                )
                .unwrap();
                let approximation = RootRigidApproximation {
                    path,
                    origin_error_bound: 0.,
                    angular_error_bound: 0.,
                };
                let hit = sweep_rigid_approximation(
                    DVec3::Y * 0.125,
                    edges,
                    actor,
                    &approximation,
                    basis,
                    DVec3::new(0.6, 0.2, -0.3),
                    scale,
                    0.,
                    &[floor],
                    256,
                    &mut 4096,
                )
                .unwrap();
                assert!(hit.complete);
            }
        }
    }
}

#[cfg(test)]
mod directional_fade_tests {
    use super::*;
    use voxy_animation::{
        RootRigidApproximation, RootRigidFadeFieldInterval, RootRigidFieldInterval, RootRigidPath,
        RootRigidTransform, RootRigidTwist,
    };
    #[test]
    fn directional_fade_proof_admits_floor_contact_but_preserves_wall_and_evaluation_margin() {
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
        let floor = AffineBox {
            center: DVec3::Y * (-0.5),
            edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
        };
        let wall = AffineBox {
            center: DVec3::X,
            edges: [DVec3::X * 0.125, DVec3::Y * 2., DVec3::Z * 2.],
        };
        let path = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::X * 1.2,
                    angular: DVec3::ZERO,
                },
                1.,
            )],
            1,
        )
        .unwrap();
        let approximation = RootRigidApproximation {
            path,
            origin_error_bound: 0.02,
            angular_error_bound: 0.1,
        };
        let span = &approximation.path.spans()[0];
        let source = RootRigidFieldInterval::from_span(
            span,
            [0., 1.],
            [0., 1.],
            RootRigidTransform::IDENTITY,
            1.,
        )
        .unwrap()
        .unwrap();
        let field = RootRigidFadeFieldInterval::new(source, source, [0., 1.], [0., 1.]).unwrap();
        let proof = approximation
            .path
            .enclose_fade_coordinate_error(&[field], 1, 1)
            .unwrap()
            .unwrap();
        assert_eq!(proof.error_bound(), 0.);
        let query = |boxes: &[AffineBox], evaluation, certificate| {
            sweep_rigid_approximation_with_coordinate_certificate(
                DVec3::Y * 0.125,
                edges,
                DQuat::IDENTITY,
                &approximation,
                DQuat::IDENTITY,
                DVec3::ZERO,
                1.,
                evaluation,
                boxes,
                256,
                &mut 4096,
                certificate,
            )
        };
        assert!(query(&[floor], 0., Some(&proof)).unwrap().complete);
        let wall_hit = query(&[floor, wall], 0., Some(&proof)).unwrap();
        assert!(
            !wall_hit.complete
                && wall_hit.path_fraction > 0.
                && wall_hit.path_fraction < 0.75 / 1.2
        );
        assert!(!query(&[floor], 0., None).unwrap().complete);
        assert!(!query(&[floor], 0.001, Some(&proof)).unwrap().complete);
        let other = approximation.path.clone();
        let wrong = other
            .enclose_fade_coordinate_error(&[field], 1, 1)
            .unwrap()
            .unwrap();
        assert!(query(&[floor], 0., Some(&wrong)).is_err());
    }
}

#[cfg(test)]
mod directional_margin_tests {
    use super::*;
    use voxy_animation::{
        RootRigidApproximation, RootRigidFadeFieldInterval, RootRigidFieldInterval, RootRigidPath,
        RootRigidTransform, RootRigidTwist,
    };
    #[test]
    fn positive_directional_error_requires_a_proven_initial_gap_including_numeric_radius() {
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
        let floor = AffineBox {
            center: DVec3::Y * (-0.5),
            edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
        };
        let nominal = RootRigidTwist {
            linear: DVec3::X * 0.3,
            angular: DVec3::ZERO,
        };
        let path = RootRigidPath::from_twists(&[(nominal, 1.)], 1).unwrap();
        let source_path = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: nominal.linear - DVec3::Y * 0.002,
                    ..nominal
                },
                1.,
            )],
            1,
        )
        .unwrap();
        let source = RootRigidFieldInterval::from_span(
            &source_path.spans()[0],
            [0., 1.],
            [0., 1.],
            RootRigidTransform::IDENTITY,
            1.,
        )
        .unwrap()
        .unwrap();
        let field = RootRigidFadeFieldInterval::new(source, source, [0., 1.], [0., 1.]).unwrap();
        let approximation = RootRigidApproximation {
            path,
            origin_error_bound: 0.05,
            angular_error_bound: 0.1,
        };
        let proof = approximation
            .path
            .enclose_fade_coordinate_error(&[field], 1, 1)
            .unwrap()
            .unwrap();
        assert!(proof.error_bound() >= 0.002);
        for scale in [-2., 1.] {
            let query = |height, certificate| {
                sweep_rigid_approximation_with_coordinate_certificate(
                    DVec3::Y * (0.125 + height),
                    edges,
                    DQuat::IDENTITY,
                    &approximation,
                    DQuat::IDENTITY,
                    DVec3::ZERO,
                    scale,
                    0.001,
                    &[floor],
                    256,
                    &mut 4096,
                    certificate,
                )
                .unwrap()
            };
            assert!(query(0.01, Some(&proof)).complete);
            assert!(!query(0.001, Some(&proof)).complete);
            assert!(!query(0.01, None).complete);
            assert!(
                proof.enclosed_scaled_error_bound(scale, 0.001).unwrap()
                    >= 0.002 * scale.abs() + 0.001
            );
        }
        assert!(proof.enclosed_scaled_error_bound(1., -0.001).is_err());
    }
}

#[cfg(test)]
mod assembled_fade_tests {
    use super::*;
    use voxy_animation::{
        RootRigidCertifiedFadeInterval, RootRigidMappedField, RootRigidPath, RootRigidTransform,
        RootRigidTwist,
    };
    #[test]
    fn original_source_fade_assembly_and_collision_query_preserve_floor_and_wall() {
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
        let floor = AffineBox {
            center: DVec3::Y * (-0.5),
            edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
        };
        let wall = AffineBox {
            center: DVec3::X,
            edges: [DVec3::X * 0.125, DVec3::Y * 2., DVec3::Z * 2.],
        };
        for speed in [0.3, 3.] {
            let source = RootRigidPath::from_twists(
                &[(
                    RootRigidTwist {
                        linear: DVec3::X * speed,
                        angular: DVec3::Y * 0.4,
                    },
                    1.,
                )],
                1,
            )
            .unwrap();
            let field = RootRigidMappedField::new(
                &source.spans()[0],
                [0., 1.],
                RootRigidTransform::IDENTITY,
                1.,
            )
            .unwrap();
            let fade = RootRigidCertifiedFadeInterval::integrate(
                None,
                field,
                [0., 1.],
                1.,
                0.01,
                0.01,
                4096,
            )
            .unwrap();
            assert!(fade.approximation().origin_error_bound > 0.);
            let hit = sweep_certified_rigid_fade(
                DVec3::Y * 0.125,
                edges,
                DQuat::IDENTITY,
                &fade,
                1,
                DQuat::IDENTITY,
                DVec3::ZERO,
                1.,
                0.,
                &[floor],
                256,
                &mut 16384,
            )
            .unwrap();
            assert!(hit.complete);
            assert_eq!(hit.displacement.y, 0.);
            if speed == 3. {
                let hit = sweep_certified_rigid_fade(
                    DVec3::Y * 0.125,
                    edges,
                    DQuat::IDENTITY,
                    &fade,
                    1,
                    DQuat::IDENTITY,
                    DVec3::ZERO,
                    1.,
                    0.,
                    &[floor, wall],
                    256,
                    &mut 16384,
                )
                .unwrap();
                assert!(!hit.complete && hit.path_fraction > 0.);
            }
        }
    }
}

#[cfg(test)]
mod automatic_fade_tests {
    use super::*;
    use voxy_animation::{
        RootRigidCertifiedFadeInterval, RootRigidMappedPath, RootRigidPath, RootRigidTransform,
        RootRigidTwist,
    };
    #[test]
    fn automatically_discovered_key_guard_preserves_floor_during_translating_yaw_fade() {
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
        let floor = AffineBox {
            center: DVec3::Y * (-0.5),
            edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
        };
        let target = RootRigidPath::from_twists(
            &[
                (
                    RootRigidTwist {
                        linear: DVec3::X * 0.3,
                        angular: DVec3::Y * 0.4,
                    },
                    1.,
                ),
                (
                    RootRigidTwist {
                        linear: DVec3::X * 0.6,
                        angular: DVec3::Y * 0.2,
                    },
                    2.,
                ),
            ],
            2,
        )
        .unwrap();
        let target = RootRigidMappedPath::new(&target, RootRigidTransform::IDENTITY, 1.).unwrap();
        let fade = RootRigidCertifiedFadeInterval::integrate_paths(
            None,
            target,
            [0., 1.],
            1.,
            0.01,
            0.01,
            4096,
        )
        .unwrap();
        let hit = sweep_certified_rigid_fade(
            DVec3::Y * 0.125,
            edges,
            DQuat::IDENTITY,
            &fade,
            1,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            0.,
            &[floor],
            512,
            &mut 16384,
        )
        .unwrap();
        assert!(hit.complete);
        assert_eq!(hit.displacement.y, 0.);
        assert!(hit.displacement.x > 0.);
    }
}

#[cfg(test)]
mod whole_tick_fade_tests {
    use super::*;
    use voxy_animation::{
        RootRigidCertifiedFadeInterval, RootRigidMappedPath, RootRigidPath, RootRigidTransform,
        RootRigidTwist,
    };
    #[test]
    fn completion_keeps_floor_contact_and_wall_stops_motion_after_fade() {
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
        let floor = AffineBox {
            center: DVec3::Y * (-0.5),
            edges: [DVec3::X * 8., DVec3::Y * 0.5, DVec3::Z * 8.],
        };
        let wall = AffineBox {
            center: DVec3::X,
            edges: [DVec3::X * 0.125, DVec3::Y * 2., DVec3::Z * 8.],
        };
        let target = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::X * 0.1,
                    angular: DVec3::Y * 0.1,
                },
                0.25,
            )],
            1,
        )
        .unwrap();
        let tail = RootRigidPath::from_twists(
            &[
                (
                    RootRigidTwist {
                        linear: DVec3::X * 0.5,
                        angular: DVec3::Y * 0.1,
                    },
                    1.,
                ),
                (
                    RootRigidTwist {
                        linear: DVec3::X,
                        angular: DVec3::Y * 0.1,
                    },
                    2.,
                ),
            ],
            2,
        )
        .unwrap();
        let mapped = RootRigidMappedPath::new(&target, RootRigidTransform::IDENTITY, 1.).unwrap();
        let completion = RootRigidMappedPath::from_enclosed_frame(
            &tail,
            target.continuous_end_enclosure(1).unwrap(),
            1.,
        )
        .unwrap();
        let tick = RootRigidCertifiedFadeInterval::integrate_paths_with_completion(
            None,
            mapped,
            [0., 1.],
            0.25,
            Some((completion, 1.)),
            0.02,
            0.01,
            4096,
        )
        .unwrap();
        let query = |boxes: &[AffineBox], evaluation| {
            sweep_certified_rigid_fade(
                DVec3::Y * 0.125,
                edges,
                DQuat::IDENTITY,
                &tick,
                1,
                DQuat::IDENTITY,
                DVec3::ZERO,
                1.,
                evaluation,
                boxes,
                8192,
                &mut 65536,
            )
            .unwrap()
        };
        let free = query(&[floor], 0.);
        assert!(free.complete);
        assert_eq!(free.displacement.y, 0.);
        assert!(free.displacement.x > 1.);
        let blocked = query(&[floor, wall], 0.);
        assert!(!blocked.complete);
        assert!(blocked.path_fraction > 0.25 && blocked.path_fraction < 1.);
        assert!(blocked.displacement.x < 0.75);
        assert_eq!(blocked.displacement.y, 0.);
        assert!(!query(&[floor], 0.001).complete);
        let certificate = tick.coordinate_certificate(1, 4096).unwrap().unwrap();
        let axis_query = |axes| {
            sweep_rigid_approximation_with_axis_errors(
                DVec3::Y * 0.125,
                edges,
                DQuat::IDENTITY,
                tick.approximation(),
                DQuat::IDENTITY,
                DVec3::ZERO,
                1.,
                0.002,
                axes,
                &[floor],
                8192,
                &mut 65536,
                Some(&certificate),
            )
        };
        assert!(axis_query([0.001, 0., 0.001]).unwrap().complete);
        assert!(!axis_query([0.001, 0.001, 0.]).unwrap().complete);
        assert!(axis_query([0., -0.001, 0.]).is_err());
        assert!(axis_query([0., f64::NAN, 0.]).is_err());
        assert!(axis_query([0.003, 0., 0.]).is_err());
    }
}

#[cfg(test)]
mod publication_error_tests {
    use super::*;
    #[test]
    fn world_publication_bounds_cover_body_vertices_signed_scale_and_offset_origin() {
        let path = voxy_animation::RootRigidPath::from_twists(
            &[
                (
                    voxy_animation::RootRigidTwist {
                        linear: DVec3::X * 0.5,
                        angular: DVec3::Y * 0.1,
                    },
                    0.25,
                ),
                (
                    voxy_animation::RootRigidTwist {
                        linear: DVec3::Z * 0.25,
                        angular: -DVec3::Y * 0.2,
                    },
                    0.25,
                ),
            ],
            2,
        )
        .unwrap();
        let cache = path.prepare_screw_enclosures(2).unwrap();
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.0625];
        let center = DVec3::new(16777216., 1., -2.);
        let orientation = DQuat::from_xyzw(0., 1., 0., 0.);
        let basis = DQuat::from_xyzw(1., 0., 0., 0.);
        let origin = DVec3::new(0.25, 0., 0.5);
        for scale in [-2., 0.5, 1.] {
            let (axes, radius) = rigid_path_publication_errors(
                center,
                edges,
                orientation,
                &cache,
                basis,
                origin,
                scale,
            )
            .unwrap();
            for span in path.spans() {
                for step in 0..=32 {
                    let motion = span.sample(f64::from(step) / 32.).unwrap();
                    for vertex in rigid_vertices(edges, basis, origin, scale).unwrap() {
                        let world = center
                            + orientation
                                * (origin
                                    + scale
                                        * (basis
                                            * (motion.translation + motion.rotation * vertex)));
                        let error = world - world.as_vec3().as_dvec3();
                        for axis in 0..3 {
                            assert!(error[axis].abs() <= axes[axis]);
                        }
                        assert!(error.length() <= radius);
                    }
                }
            }
            assert!(axes[0] >= 1.);
        }
    }
}

#[cfg(test)]
mod published_world_error_tests {
    use super::*;
    #[test]
    fn canonical_witness_covers_f32_publication_and_ground_relocation() {
        let center = DVec3::new(65536.009765625, 2., -3.);
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.5];
        let witness = CanonicalWorldPose {
            corners: std::array::from_fn(|i| {
                let point = center + corners(edges)[i];
                point.to_array().map(|x| [x, x])
            }),
        };
        let (_, proposal) = witness.evaluation_error(center, edges).unwrap();
        assert!(proposal < 1e-8);
        let published = center.as_vec3().as_dvec3();
        let (axes, radius) = witness.evaluation_error(published, edges).unwrap();
        assert!(axes[0] >= 1. / 512. && axes[0] < 1. / 512. + 1e-8);
        assert!(radius >= 1. / 512. && radius < 1. / 512. + 1e-8);
        println!("PUBLISHED_WORLD_ERROR {:?}", (axes, radius));
        let relocated = published + DVec3::Y * 0.125;
        let (axes, radius) = witness.evaluation_error(relocated, edges).unwrap();
        assert!(axes[1] >= 0.125 && radius >= 0.125 + 1. / 512.);
        assert!(
            witness
                .evaluation_error(DVec3::splat(f64::NAN), edges)
                .is_err()
        );
    }
}

#[cfg(test)]
mod physical_rotation_error_tests {
    use super::*;
    #[test]
    fn physical_controller_reframe_and_orientation_update_fit_uniform_rotation_caps() {
        let path = voxy_animation::RootRigidPath::from_twists(
            &[
                (
                    voxy_animation::RootRigidTwist {
                        linear: DVec3::new(1., 0., 0.5),
                        angular: DVec3::Y * 0.7,
                    },
                    0.5,
                ),
                (
                    voxy_animation::RootRigidTwist {
                        linear: DVec3::new(0., 0.25, 0.),
                        angular: DVec3::X * -0.6,
                    },
                    0.25,
                ),
            ],
            2,
        )
        .unwrap();
        let cache = path.prepare_screw_enclosures(2).unwrap();
        let basis = DQuat::from_array([0.5, 0.5, 0.5, 0.5]);
        let orientation = DQuat::from_array([0.5, -0.5, 0.5, 0.5]);
        let caps = cache
            .physical_rotation_selection_error_bounds(basis, orientation)
            .unwrap();
        assert!(caps.iter().all(|v| v.is_finite() && *v > 0. && *v < 1e-9));
        for span in 0..2 {
            for fraction in [0., 0.1, 0.3, 0.5, 0.875, 1.] {
                let local = cache
                    .sample_evaluated_with_errors(span, fraction)
                    .unwrap()
                    .pose();
                let actual = (orientation * reframe_rotation(basis, local.rotation)).normalize();
                println!(
                    "PHYSICAL_ROTATION_ERROR {:?}",
                    (span, fraction, actual.to_array(), caps)
                );
            }
        }
        assert!(
            cache
                .physical_rotation_selection_error_bounds(
                    DQuat::from_array([f64::NAN; 4]),
                    orientation
                )
                .is_err()
        );
    }
}

#[cfg(test)]
mod physical_body_error_tests {
    use super::*;
    #[test]
    fn actual_controller_center_and_edges_fit_uniform_whole_body_caps() {
        let path = voxy_animation::RootRigidPath::from_twists(
            &[
                (
                    voxy_animation::RootRigidTwist {
                        linear: DVec3::new(1., 0., 0.5),
                        angular: DVec3::Y * 0.7,
                    },
                    0.5,
                ),
                (
                    voxy_animation::RootRigidTwist {
                        linear: DVec3::new(0., 0.25, 0.),
                        angular: DVec3::X * -0.6,
                    },
                    0.25,
                ),
            ],
            2,
        )
        .unwrap();
        let cache = path.prepare_screw_enclosures(2).unwrap();
        let center = DVec3::new(65536.009765625, 2., -3.);
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.5];
        let basis = DQuat::from_array([0.5, 0.5, 0.5, 0.5]);
        let actor = DQuat::from_array([0.5, -0.5, 0.5, 0.5]);
        let origin = DVec3::new(0.25, -0.125, 0.5);
        let scale = -2.;
        let (axes, radius) = cache
            .physical_body_selection_error_bounds(center, edges, basis, origin, actor, scale)
            .unwrap();
        assert!(
            axes.iter()
                .all(|v| v.is_finite() && *v > 0. && *v <= radius)
        );
        assert!(radius < 1e-7);
        for span in 0..2 {
            for fraction in [0., 0.1, 0.5, 0.875, 1.] {
                let local = cache
                    .sample_evaluated_with_errors(span, fraction)
                    .unwrap()
                    .pose();
                let rotation = reframe_rotation(basis, local.rotation);
                let displacement = rotate_vector(
                    actor,
                    scale * rotate_vector(basis, local.translation) + origin
                        - rotate_vector(rotation, origin),
                );
                let actual_center = center + displacement;
                let orientation = (actor * rotation).normalize();
                let actual_edges = edges.map(|edge| rotate_vector(orientation, edge));
                for corner in 0..8 {
                    let actual = actual_center + corners(actual_edges)[corner];
                    println!(
                        "PHYSICAL_BODY_ERROR {:?}",
                        (span, fraction, corner, actual.to_array(), axes, radius)
                    );
                }
            }
        }
        assert!(
            cache
                .physical_body_selection_error_bounds(center, edges, basis, origin, actor, 0.)
                .is_err()
        );
    }
}

#[cfg(test)]
mod automatic_physical_allowance_tests {
    use super::*;

    #[test]
    fn stationary_coordinate_survives_actual_cache_reframe_pivot_and_body_update() {
        let actors = [
            DQuat::IDENTITY,
            DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5),
            DQuat::from_xyzw(0.123, 0.123, -0.7, 0.7).normalize(),
            DQuat::from_xyzw(0.5, -0.5, 0.5, 0.5),
        ];
        let bases = [
            DQuat::IDENTITY,
            DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5),
            DQuat::from_rotation_y(0.37),
            DQuat::from_xyzw(0.123, 0., 0.7, 0.).normalize(),
        ];
        let origin = DVec3::new(0.123, -0.731, 0.417);
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.5];
        let mut cases = 0;
        for orientation in actors {
            for basis in bases {
                for world_axis in 0..3 {
                    let Some((actor_axis, actor_sign)) =
                        rotation_coordinate_preimage(orientation, world_axis)
                    else {
                        continue;
                    };
                    let Some((source_axis, _)) = rotation_coordinate_preimage(basis, actor_axis)
                    else {
                        continue;
                    };
                    let mut angular = DVec3::ZERO;
                    angular[source_axis] = 0.7;
                    let mut linear = DVec3::new(0.3, -0.2, 0.1);
                    linear[source_axis] = 0.;
                    let path = voxy_animation::RootRigidPath::from_twists(
                        &[(voxy_animation::RootRigidTwist { linear, angular }, 0.5)],
                        1,
                    )
                    .unwrap();
                    let cache = path.prepare_screw_enclosures(1).unwrap();
                    if !exact_stationary_physical_projection(&cache, orientation, basis, world_axis)
                    {
                        continue;
                    }
                    for fraction in [0., 0.1, 0.3, 0.5, 0.9, 1.] {
                        let pose = cache.sample_evaluated(0, fraction).unwrap().0;
                        let rotation = reframe_rotation(basis, pose.rotation);
                        let body_rotation = (orientation * rotation).normalize();
                        assert_eq!(
                            rotation_coordinate_preimage(body_rotation, world_axis),
                            Some((actor_axis, actor_sign))
                        );
                        for scale in [-2., 1.] {
                            let displacement = rotate_vector(
                                orientation,
                                scale * rotate_vector(basis, pose.translation) + origin
                                    - rotate_vector(rotation, origin),
                            );
                            assert_eq!(displacement[world_axis], 0.);
                            for edge in edges {
                                assert_eq!(
                                    rotate_vector(body_rotation, edge)[world_axis],
                                    rotate_vector(orientation, edge)[world_axis]
                                );
                            }
                        }
                    }
                    cases += 1;
                }
            }
        }
        assert!(cases >= 20, "only {cases} admitted frame cases");
    }

    #[test]
    fn zero_requested_allowance_still_reserves_derived_body_error_before_wall() {
        let approximation = voxy_animation::RootRigidApproximation {
            path: voxy_animation::RootRigidPath::from_twists(
                &[(
                    voxy_animation::RootRigidTwist {
                        linear: DVec3::X * 0.25,
                        angular: DVec3::ZERO,
                    },
                    1.,
                )],
                1,
            )
            .unwrap(),
            origin_error_bound: 0.,
            angular_error_bound: 0.,
        };
        let center = DVec3::ZERO;
        let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
        let cache = approximation.path.prepare_screw_enclosures(1).unwrap();
        let (_, radius) = cache
            .physical_body_selection_error_bounds(
                center,
                edges,
                DQuat::IDENTITY,
                DVec3::ZERO,
                DQuat::IDENTITY,
                1.,
            )
            .unwrap();
        assert!(radius > 0. && radius < 1e-6);
        let wall = AffineBox {
            center: DVec3::X * (0.5 + radius * 0.25),
            edges,
        };
        assert!(wall.center.x > 0.5);
        let run = |allowance, queries: &mut usize| {
            sweep_rigid_approximation(
                center,
                edges,
                DQuat::IDENTITY,
                &approximation,
                DQuat::IDENTITY,
                DVec3::ZERO,
                1.,
                allowance,
                &[wall],
                256,
                queries,
            )
        };
        let vertices = rigid_vertex_enclosures(edges, DQuat::IDENTITY, DVec3::ZERO, 1.).unwrap();
        let speeds = cache.point_speed_bounds(&vertices, 1.).unwrap();
        let control = sweep_rigid_path_with_bounds(
            center,
            edges,
            DQuat::IDENTITY,
            &approximation.path,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            0.,
            &[wall],
            256,
            &mut 4096,
            Some(&speeds),
            Some(&cache),
            None,
            [false; 3],
        )
        .unwrap();
        assert!(
            control.complete,
            "zero-clearance control must reach the endpoint"
        );
        let hit = run(0., &mut 4096).unwrap();
        assert!(!hit.complete);
        assert!(hit.displacement.x < 0.25 && hit.displacement.x > 0.249);
        println!(
            "AUTOMATIC_PHYSICAL_MARGIN {:?}",
            (radius, wall.center.x, hit.displacement.x)
        );
        for invalid in [f64::NAN, f64::INFINITY, -1.] {
            assert!(matches!(
                run(invalid, &mut 4096),
                Err(PhysicsError::InvalidMotion)
            ));
        }
        assert_eq!(add_evaluation_allowances(0., 0.).unwrap(), 0.);
        assert!(add_evaluation_allowances(0.25, 0.5).unwrap() >= 0.75);
        assert!(add_evaluation_allowances(f64::MAX, f64::MAX).is_err());
    }

    #[test]
    fn moving_projection_and_unproved_basis_do_not_get_zero_error() {
        let path = voxy_animation::RootRigidPath::from_twists(
            &[(
                voxy_animation::RootRigidTwist {
                    linear: DVec3::Y * 0.01,
                    angular: DVec3::Y * 0.7,
                },
                0.5,
            )],
            1,
        )
        .unwrap();
        let cache = path.prepare_screw_enclosures(1).unwrap();
        assert!(!exact_stationary_physical_projection(
            &cache,
            DQuat::IDENTITY,
            DQuat::IDENTITY,
            1
        ));
        assert!(!exact_stationary_physical_projection(
            &cache,
            DQuat::from_rotation_x(0.37),
            DQuat::IDENTITY,
            1
        ));
    }
}
