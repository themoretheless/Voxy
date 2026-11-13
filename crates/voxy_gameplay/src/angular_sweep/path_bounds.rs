use super::{
    AffineBox, DQuat, DVec3, PathHit, PhysicsError, PointBoxes, SweepEnclosureBounds, SweepQuery,
    advance_with_enclosures, exact_gap, gap, query, reframe_rotation, rigid_vertex_enclosures,
    rigid_vertices, rigid_world_frame, rotate_vector, rotation_coordinate_preimage,
};

// A stationary source coordinate survives the actual evaluation chain exactly
// when the basis maps every row structurally. Cached screw arithmetic retains
// zero orthogonal quaternion components and zero coordinate translation. Basis
// reframing therefore retains a single imaginary axis. Right multiplication of
// the actor by this axis quaternion uses at most two nonzero products per
// component; its signed coordinate-row identities have identical products and
// sums (up to sign/order), so normalization preserves them as well. The cross
// evaluator then copies the coordinate directly. Pivot cancellation is exact.
pub(super) fn exact_physical_projection_frame(
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

pub(super) fn exact_stationary_physical_projection(
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
#[expect(
    clippy::too_many_arguments,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
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

#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
pub(super) fn sweep_rigid_path_with_bounds(
    sweep: SweepQuery,
    path: &voxy_animation::RootRigidPath,
    clearance: f64,
    boxes: &[AffineBox],
    iterations: usize,
    queries: &mut usize,
    bounds: SweepEnclosureBounds<'_>,
) -> Result<PathHit, PhysicsError> {
    let SweepQuery {
        center,
        rest_edges,
        orientation,
        basis,
        origin,
        scale,
    } = sweep;
    let SweepEnclosureBounds {
        enclosed_speeds,
        enclosed_path,
        coordinate_certificate,
        exact_projection_axes,
    } = bounds;
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
