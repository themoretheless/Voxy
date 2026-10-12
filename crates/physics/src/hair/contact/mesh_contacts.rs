//! Rod-to-mesh contact gathering, interval witnesses and envelope gradients.
//!
//! Split out of `contact.rs` (plan step B4) with unchanged bodies.

use super::*;

pub(in crate::hair) fn segment_contact_precedes(
    distance: f64,
    t: f64,
    key: [usize; 3],
    old_distance: f64,
    old_t: f64,
    old_key: [usize; 3],
    length: f64,
) -> bool {
    const ZERO_DISTANCE: f64 = 1e-10;
    if distance <= ZERO_DISTANCE && old_distance <= ZERO_DISTANCE {
        if (t - old_t).abs() * length <= ZERO_DISTANCE {
            key < old_key
        } else {
            t < old_t
        }
    } else {
        distance < old_distance || (distance == old_distance && key < old_key)
    }
}
// Feature pseudonormals classify the distance sign. Away from zero distance,
// the metric gradient follows the closest-point displacement, also from inside.
pub(super) fn distance_gradient(
    delta: V,
    distance: f64,
    signed: f64,
    fallback: V,
    epsilon: f64,
) -> V {
    if distance < epsilon {
        fallback
    } else {
        mul(delta, signed.signum() / distance)
    }
}
// A face-interior metric gradient is exactly its outward geometric normal.
// Reconstructing it from two rounded world points adds tangential force,
// especially when the surface gap is tiny or the world origin is distant.
// Edges/vertices retain the signed metric displacement (not a face normal).
pub(super) fn triangle_distance_gradient(
    triangle: &Triangle,
    point: V,
    closest: V,
    signed: f64,
    fallback: V,
    epsilon: f64,
) -> V {
    if signed.abs() >= epsilon
        && matches!(
            closest_triangle_feature(point, triangle).1,
            ClosestFeature::Face
        )
    {
        triangle.normal
    } else {
        distance_gradient(sub(point, closest), signed.abs(), signed, fallback, epsilon)
    }
}
pub(in crate::hair) fn mesh_contacts(rod: &mut HairRod, meshes: &[TriangleMesh], radius: f64) {
    gather_mesh_contacts(rod, meshes, radius, true);
}
// Approximate the deepest signed-distance witness in one interior interval.
// Coarse samples bracket candidate minima; bounded refinement retains every
// sampled candidate, including the old midpoint. This is constraint discovery,
// not a global segment-clearance certificate for arbitrary nonconvex meshes.
pub(super) fn deepest_interval_witness(
    mesh: &TriangleMesh,
    a: V,
    b: V,
    lo: f64,
    hi: f64,
) -> Result<(f64, f64, V), &'static str> {
    let evaluate = |t: f64| {
        mesh.signed_distance_closed(add(mul(a, 1. - t), mul(b, t)))
            .map(|(d, n)| (t, d, n))
    };
    let samples = (0..=8)
        .map(|i| evaluate(lo + (hi - lo) * i as f64 / 8.))
        .collect::<Result<Vec<_>, _>>()?;
    let segment_length = len(sub(b, a));
    let mut best = samples[4];
    for value in &samples {
        if value.1 < best.1 {
            best = *value;
        }
    }
    for i in 0..=8 {
        // A deepest endpoint sample can hide an even deeper medial peak in
        // the last sampling cell. Refine those boundary cells as well as
        // interior brackets; retain the endpoint itself in `best`.
        let (start, end) = if i == 0 {
            if samples[0].1 > samples[1].1 {
                continue;
            }
            (0, 1)
        } else if i == 8 {
            if samples[8].1 > samples[7].1 {
                continue;
            }
            (7, 8)
        } else {
            if samples[i].1 > samples[i - 1].1 || samples[i].1 > samples[i + 1].1 {
                continue;
            }
            (i - 1, i + 1)
        };
        let mut left = samples[start].0;
        let mut right = samples[end].0;
        let ratio = (5f64.sqrt() - 1.) * 0.5;
        let mut x = evaluate(right - ratio * (right - left))?;
        let mut y = evaluate(left + ratio * (right - left))?;
        for _ in 0..80 {
            for value in [x, y] {
                if value.1 < best.1 {
                    best = value;
                }
            }
            // Discovery precision must be finer than the 1e-10 m contact
            // admission gate, including long/scaled segments. Keep a bounded
            // search, but stop by physical interval width rather than count.
            if (right - left) * segment_length <= 1e-12 {
                break;
            }
            if x.1 <= y.1 {
                right = y.0;
                y = x;
                x = evaluate(right - ratio * (right - left))?;
            } else {
                left = x.0;
                x = y;
                y = evaluate(left + ratio * (right - left))?;
            }
        }
    }
    let position = add(mul(a, 1. - best.0), mul(b, best.0));
    let mut nearest = (f64::INFINITY, 0, [0.; 3]);
    mesh.nearest(position, 0, &mut nearest);
    // Feature pseudonormals determine sign, not the metric derivative at
    // concave edges. Use the actual closest-point distance gradient.
    best.2 = triangle_distance_gradient(
        &mesh.triangles[nearest.1],
        position,
        nearest.2,
        best.1,
        best.2,
        1e-10,
    );
    Ok(best)
}
// At a medial minimum of signed distance along a segment, combine the
// two observed one-sided metric gradients so their segment derivative is zero.
// Return unit force direction AND its original gradient magnitude separately.
pub(super) fn interior_envelope_gradient(
    mesh: &TriangleMesh,
    a: V,
    b: V,
    t: f64,
    lo: f64,
    hi: f64,
    normal: V,
    velocity: V,
) -> (V, f64, V) {
    let delta = (hi - lo) * 1e-6;
    if t - delta <= lo || t + delta >= hi {
        return (normal, 1., velocity);
    }
    let probe = |fraction: f64| -> Option<(V, V)> {
        let p = add(mul(a, 1. - fraction), mul(b, fraction));
        let (signed, pseudo) = mesh.signed_distance_closed(p).ok()?;
        let mut nearest = (f64::INFINITY, 0, [0.; 3]);
        mesh.nearest(p, 0, &mut nearest);
        Some((
            triangle_distance_gradient(
                &mesh.triangles[nearest.1],
                p,
                nearest.2,
                signed,
                pseudo,
                1e-10,
            ),
            mesh.surface_velocity(nearest.1, nearest.2),
        ))
    };
    let (Some((left, vl)), Some((right, vr))) = (probe(t - delta), probe(t + delta)) else {
        return (normal, 1., velocity);
    };
    let segment = sub(b, a);
    let dl = dot(left, segment);
    let dr = dot(right, segment);
    if !(dl < 0. && dr > 0.) {
        return (normal, 1., velocity);
    }
    let w = dr / (dr - dl);
    let g = add(mul(left, w), mul(right, 1. - w));
    let scale = len(g);
    // A zero generalized gradient needs a nonlocal escape, not division by zero.
    if !scale.is_finite() || scale < 1e-12 || scale > 1. + 1e-12 {
        return (normal, 1., velocity);
    }
    let direction = mul(g, 1. / scale);
    let blended = add(mul(vl, w), mul(vr, 1. - w));
    let normal_speed = (w * dot(left, vl) + (1. - w) * dot(right, vr)) / scale;
    let velocity = add(
        blended,
        mul(direction, normal_speed - dot(direction, blended)),
    );
    (direction, scale, velocity)
}
pub(in crate::hair) fn refresh_mesh_constraints(
    rod: &mut HairRod,
    meshes: &[TriangleMesh],
    radius: f64,
) {
    gather_mesh_contacts(rod, meshes, radius, false);
}
pub(super) fn gather_mesh_contacts(
    rod: &mut HairRod,
    meshes: &[TriangleMesh],
    radius: f64,
    project_positions: bool,
) {
    const CONTACT_TOLERANCE: f64 = 1e-10;
    let query_radius = radius + CONTACT_TOLERANCE;
    rod.contacts
        .retain(|contact| !matches!(contact.source, ContactSource::Mesh(_)));
    // Closest-surface projection also recovers nodes already inside the mesh,
    // which a narrow AABB overlap query alone cannot find.
    rod.nearest_cache.resize_with(meshes.len(), Vec::new);
    for cache in &mut rod.nearest_cache {
        cache.resize(rod.x.len(), usize::MAX);
    }
    rod.clearance_cache.resize_with(meshes.len(), Vec::new);
    for cache in &mut rod.clearance_cache {
        cache.resize(rod.x.len(), None);
    }
    for i in 1..rod.x.len() {
        for (mesh_index, mesh) in meshes.iter().enumerate() {
            if rod.clearance_cache[mesh_index][i]
                .is_some_and(|bound| bound.excludes(&[rod.x[i]], query_radius))
            {
                continue;
            }
            let cached = rod.nearest_cache[mesh_index][i];
            let mut best = if let Some(triangle) = mesh.triangles.get(cached) {
                let point = closest_triangle(rod.x[i], triangle);
                let delta = sub(rod.x[i], point);
                (dot(delta, delta), cached, point)
            } else {
                (f64::INFINITY, 0, [0.0; 3])
            };
            mesh.nearest(rod.x[i], 0, &mut best);
            rod.nearest_cache[mesh_index][i] = best.1;
            let contact_normal = mesh.contact_normal(best.1, rod.x[i]);
            let delta = sub(rod.x[i], best.2);
            let distance = best.0.sqrt();
            let signed = if dot(delta, contact_normal) < 0.0 {
                -distance
            } else {
                distance
            };
            rod.clearance_cache[mesh_index][i] = Some(Clearance {
                position: rod.x[i],
                distance: signed,
            });
            if signed <= query_radius {
                let normal = triangle_distance_gradient(
                    &mesh.triangles[best.1],
                    rod.x[i],
                    best.2,
                    signed,
                    contact_normal,
                    1e-12,
                );
                let depth = radius - signed;
                let target = add(rod.x[i], mul(normal, depth));
                let contact =
                    rod.record_point_contact(i, normal, target, ContactSource::Mesh(mesh_index));
                rod.contacts[contact].surface_velocity = mesh.surface_velocity(best.1, best.2);
                let correction = mul(normal, depth.max(0.).min(rod.lengths[i - 1] * 0.2));
                if project_positions {
                    rod.x[i] = add(rod.x[i], correction);
                    rod.record_surface_response(
                        i,
                        correction,
                        mesh.surface_velocity(best.1, best.2),
                    );
                }
            }
        }
    }
    let mut candidates = std::mem::take(&mut rod.contact_candidates);
    for i in 0..rod.lengths.len() {
        for (mesh_index, mesh) in meshes.iter().enumerate() {
            let a = rod.x[i];
            let b = rod.x[i + 1];
            let swept = [a, b, rod.old_x[i], rod.old_x[i + 1]];
            if [i, i + 1].into_iter().any(|point| {
                rod.clearance_cache[mesh_index][point]
                    .is_some_and(|bound| bound.excludes(&swept, query_radius))
            }) {
                continue;
            }
            // Swept bounds include previous endpoints; catches triangles crossed this substep.
            let min = std::array::from_fn(|j| {
                a[j].min(b[j]).min(rod.old_x[i][j]).min(rod.old_x[i + 1][j]) - query_radius
            });
            let max = std::array::from_fn(|j| {
                a[j].max(b[j]).max(rod.old_x[i][j]).max(rod.old_x[i + 1][j]) + query_radius
            });
            candidates.clear();
            mesh.query(min, max, 0, &mut candidates);
            let mut contact: Option<(f64, f64, V, f64, [usize; 3], V)> = None;
            let mut crossings = Vec::new();
            for &id in &candidates {
                let tri = &mesh.triangles[id];
                let (t, p, q) = segment_triangle(a, b, tri);
                // Follicle overlap is prescribed. Do not let an allowed root
                // contact hide a later collision on the free part of the strand.
                if i == 0 && t < 0.15 {
                    continue;
                }
                let contact_normal = mesh.contact_normal(id, p);
                let delta = sub(p, q);
                let distance = len(delta);
                if mesh.feature_normals.is_some() && distance < 1e-10 {
                    crossings.push(t);
                }
                let signed = if dot(delta, contact_normal) < 0. {
                    -distance
                } else {
                    distance
                };
                if distance > query_radius {
                    continue;
                }
                // Choose nearest surface, not every tessellation face independently.
                let normal = triangle_distance_gradient(tri, p, q, signed, contact_normal, 1e-10);
                if contact.as_ref().is_none_or(|c| {
                    segment_contact_precedes(distance, t, tri.ids, c.0, c.1, c.4, len(sub(b, a)))
                }) {
                    contact = Some((
                        distance,
                        t,
                        normal,
                        signed,
                        tri.ids,
                        mesh.surface_velocity(id, q),
                    ));
                }
            }
            // A segment with exterior endpoints can traverse a closed body.
            // Its boundary intersection has zero distance, hiding the actual
            // penetration. Add interior witnesses between surface crossings;
            // these discover constraints, never certify continuous clearance.
            // A segment entirely inside the volume has no boundary crossings.
            // It still needs an interior depth witness; endpoint constraints
            // alone can miss a deeper penetration in the middle.
            let free_start = if i == 0 { 0.15 } else { 0. };
            let inside_endpoint = mesh.feature_normals.is_some()
                && [free_start, 1.].into_iter().any(|t| {
                    let p = add(mul(a, 1. - t), mul(b, t));
                    let node = if t == 0. {
                        Some(i)
                    } else if t == 1. {
                        Some(i + 1)
                    } else {
                        None
                    };
                    if let Some(bound) =
                        node.and_then(|point| rod.clearance_cache[mesh_index][point])
                    {
                        if bound.excludes(&[p], query_radius) {
                            return false;
                        }
                        // Reuse the exact signed endpoint query above only at the
                        // same position. Moved/projected points must be re-queried.
                        if bound.position == p {
                            return bound.distance < -CONTACT_TOLERANCE;
                        }
                    }
                    mesh.signed_distance_closed(p)
                        .is_ok_and(|(distance, _)| distance < -CONTACT_TOLERANCE)
                });
            if !crossings.is_empty() || inside_endpoint {
                crossings.extend([if i == 0 { 0.15 } else { 0.0 }, 1.0]);
                crossings.retain(|t| *t >= if i == 0 { 0.15 } else { 0.0 });
                crossings.sort_by(f64::total_cmp);
                crossings.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
                for interval in crossings.windows(2) {
                    if let Ok((t, signed, normal)) =
                        deepest_interval_witness(mesh, a, b, interval[0], interval[1])
                    {
                        let position = add(mul(a, 1. - t), mul(b, t));
                        if signed >= -CONTACT_TOLERANCE {
                            continue;
                        }
                        let mut nearest = (f64::INFINITY, 0, [0.; 3]);
                        mesh.nearest(position, 0, &mut nearest);
                        let velocity = mesh.surface_velocity(nearest.1, nearest.2);
                        let (normal, scale, velocity) = interior_envelope_gradient(
                            mesh,
                            a,
                            b,
                            t,
                            interval[0],
                            interval[1],
                            normal,
                            velocity,
                        );
                        let depth = (radius - signed) / scale;
                        let index = rod.record_contact(
                            i,
                            t,
                            normal,
                            add(position, mul(normal, depth)),
                            ContactSource::Mesh(mesh_index),
                        );
                        rod.contacts[index].metric_scale = scale;
                        rod.contacts[index].surface_velocity = velocity;
                        if project_positions {
                            project(rod, i, t, normal, depth, Some(velocity));
                        }
                    }
                }
            }
            // A flat face supports an interval, not one arbitrary closest
            // point. A tiny tilt can switch the single witness between its
            // ends and cause a finite change in the velocity Jacobian. Keep
            // both supported endpoints as original physical constraints.
            // The tolerance only triggers discovery: each endpoint must still
            // be inside the exact projected face, within the original query
            // radius, and have this face as its globally nearest triangle.
            // Inspect every overlapping face: restricting the interval to the
            // selected minimum loses support across a tessellation boundary.
            if contact.is_some() {
                for &id in &candidates {
                    let tri = &mesh.triangles[id];
                    if dot(sub(b, a), tri.normal).abs() <= CONTACT_TOLERANCE {
                        if let Some((lo, hi)) = projected_face_interval(a, b, tri) {
                            for t in [lo.max(free_start), hi] {
                                if t < free_start || t > hi {
                                    continue;
                                }
                                let position = add(mul(a, 1. - t), mul(b, t));
                                let q = closest_triangle(position, tri);
                                let delta = sub(position, q);
                                let distance = len(delta);
                                if distance > query_radius {
                                    continue;
                                }
                                let mut nearest = (f64::INFINITY, 0, [0.; 3]);
                                mesh.nearest(position, 0, &mut nearest);
                                if nearest.1 != id {
                                    continue;
                                }
                                let pseudo = mesh.contact_normal(id, position);
                                let signed = if dot(delta, pseudo) < 0. {
                                    -distance
                                } else {
                                    distance
                                };
                                let normal = triangle_distance_gradient(
                                    tri, position, q, signed, pseudo, 1e-12,
                                );
                                let index = rod.record_contact(
                                    i,
                                    t,
                                    normal,
                                    add(position, mul(normal, radius - signed)),
                                    ContactSource::Mesh(mesh_index),
                                );
                                rod.contacts[index].surface_velocity = mesh.surface_velocity(id, q);
                            }
                        }
                    }
                }
            }
            if let Some((_, t, normal, signed, _, velocity)) = contact {
                let depth = radius - signed;
                let position = add(mul(a, 1. - t), mul(b, t));
                let contact = rod.record_contact(
                    i,
                    t,
                    normal,
                    add(position, mul(normal, depth)),
                    ContactSource::Mesh(mesh_index),
                );
                rod.contacts[contact].surface_velocity = velocity;
                if depth > 0. && project_positions {
                    project(rod, i, t, normal, depth, Some(velocity));
                }
            }
        }
    }
    rod.contact_candidates = candidates;
}
// An injective 96-bit cell key retains signed coordinates while avoiding
// the array hash's length prefix in the self-contact broad phase.
