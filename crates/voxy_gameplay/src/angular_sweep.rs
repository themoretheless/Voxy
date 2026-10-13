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
    pub feature: Option<super::convex::AxisFeature>,
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
    // At most 2 endpoints + 3 terms x 2 bases x 4 periods = 26 cuts, so a fixed
    // array replaces a heap allocation per (obstacle, axis) evaluation.
    let mut cuts = [0_f64; 26];
    let mut count = 2;
    cuts[1] = end;
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
                    debug_assert!(count < cuts.len());
                    cuts[count] = t;
                    count += 1;
                }
            }
        }
    }
    let cuts = &mut cuts[..count];
    cuts.sort_by(f64::total_cmp);
    let mut kept = 1;
    for index in 1..cuts.len() {
        if cuts[index] != cuts[kept - 1] {
            cuts[kept] = cuts[index];
            kept += 1;
        }
    }
    let cuts = &cuts[..kept];
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
            feature: None,
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
            feature: None,
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
        let mut contact_feature = None;
        let mut tolerance = 0.;
        for obstacle in candidates {
            query(queries)?;
            let relative = center - obstacle.center;
            let mut separation = f64::NEG_INFINITY;
            let mut normal = DVec3::ZERO;
            let mut feature = None;
            for source in obstacle.support_axes(current) {
                let axis = source.direction;
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
                    feature = Some(source.feature);
                    normal = axis * if relative.dot(axis) < 0. { -1. } else { 1. };
                }
            }
            if separation < distance {
                distance = separation;
                contact = normal;
                contact_feature = feature;
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
                feature: contact_feature,
                fraction: time,
                normal: Some(contact),
            });
        }
        if speed_bound == 0. {
            return Ok(Hit {
                feature: None,
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
                feature: None,
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
                    .map_or(0., glam::DVec3::length)
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
/// evaluation bounds. `evaluation_radius` is an additional world allowance.
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
    if coordinate_certificate
        .is_some_and(|proof| !std::ptr::eq(proof.path(), &raw const approximation.path))
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
                    if (clearance == 0.
                        || coordinate_certificate.is_some()
                        || exact_numeric_projection)
                        && let Some(cache) = enclosed_path
                    {
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
mod tests;

#[cfg(test)]
mod directional_fade_tests;

#[cfg(test)]
mod directional_margin_tests;

#[cfg(test)]
mod assembled_fade_tests;

#[cfg(test)]
mod automatic_fade_tests;

#[cfg(test)]
mod whole_tick_fade_tests;

#[cfg(test)]
mod publication_error_tests;

#[cfg(test)]
mod published_world_error_tests;

#[cfg(test)]
mod physical_rotation_error_tests;

#[cfg(test)]
mod physical_body_error_tests;

#[cfg(test)]
mod automatic_physical_allowance_tests;
