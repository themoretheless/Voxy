//! Conservative advancement over authored rigid trajectories and angular paths.
use super::{
    PhysicsError,
    convex::{AffineBox, reframe_rotation, rotate_vector, rotation_coordinate_preimage},
};
use glam::{DQuat, DVec3};
mod exact_gap;
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
        clearance,
        steps,
        queries,
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
    clearance: f64,
    steps: &mut usize,
    queries: &mut usize,
    point_sample: Option<&dyn Fn(f64) -> Result<PointBoxes, PhysicsError>>,
) -> Result<Hit, PhysicsError> {
    if !clearance.is_finite() || clearance < 0. {
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
                } else if clearance > 0. {
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
        if speed_bound == 0. {
            return Ok(Hit {
                fraction: 1.,
                normal: None,
            });
        }
        // A projection gap is a lower bound on Euclidean separation. Every body
        // point travels at most speed_bound over the normalized unit interval.
        let next = if clearance > 0. || point_sample.is_some() {
            gap::advance_time(time, distance, speed_bound)?
        } else {
            (time + 0.8 * distance / speed_bound).min(1.)
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

/// Conditional sweep of an approximate field; evaluation_radius is a caller
/// proof obligation in world units, covering all numeric evaluation uncertainty.
/// Certified requests use this sweep within transactional candidate preparation;
/// ordinary editor dispatch still needs qualified caller error bounds.
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
    if evaluation_axes
        .iter()
        .any(|value| !value.is_finite() || *value < 0. || *value > evaluation_radius)
    {
        return Err(PhysicsError::InvalidMotion);
    }
    if coordinate_certificate.is_some_and(|proof| !std::ptr::eq(proof.path(), &approximation.path))
    {
        return Err(PhysicsError::InvalidMotion);
    }
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
    let enclosed_path = approximation
        .path
        .prepare_screw_enclosures(4096)
        .map_err(|_| PhysicsError::InvalidMotion)?;
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
    )?;
    // Mirror the physical controller's proposed orientation/edge update. This
    // checks the rounded proposal itself, not merely the canonical field pose.
    let proposed_orientation = (orientation * hit.rotation).normalize();
    let proposed_edges = rest_edges.map(|edge| rotate_vector(proposed_orientation, edge));
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
    )
}

// Structural coordinate-plane certificate for an exact canonical screw field.
// Both real frame rotations must transport an exact coordinate row. Uniform nonzero
// error clearance requires a matching certificate and its outward world margin.
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
            clearance,
            &mut steps,
            queries,
            if world_frame.is_some() {
                Some(&point_sample)
            } else {
                None
            },
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
            });
        }
    }
    for (index, span) in path.spans().iter().enumerate() {
        query(queries)?;
        let sample = |fraction| -> Result<(DVec3, [DVec3; 3]), PhysicsError> {
            let motion = span
                .sample(fraction)
                .map_err(|_| PhysicsError::InvalidMotion)?;
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
                    if clearance == 0. || coordinate_certificate.is_some() {
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
                                if clearance == 0. {
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
            clearance,
            &mut steps,
            queries,
            if world_frame.is_some() {
                Some(&point_sample)
            } else {
                None
            },
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
