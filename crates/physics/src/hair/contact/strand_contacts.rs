//! Strand-to-strand (self) contacts: normals, gathering and refresh.
//!
//! Split out of `contact.rs` (plan step B4) with unchanged bodies.

use super::*;

pub(in crate::hair) fn self_contacts(rods: &mut [HairRod], radius: f64) -> Vec<StrandResponse> {
    gather_strand_contacts(rods, radius, true)
}
/// Query witnesses must be refreshed before use. A pair owns one clearance
/// budget; splitting its positional targets does not split that budget.
pub(in crate::hair) fn strand_geometry_admitted(
    rods: &[HairRod],
    responses: &[StrandResponse],
    radius: f64,
) -> Result<bool, &'static str> {
    for response in responses {
        let (ra, ia, s) = response.a;
        let (rb, ib, t) = response.b;
        let a = add(mul(rods[ra].x[ia], 1. - s), mul(rods[ra].x[ia + 1], s));
        let b = add(mul(rods[rb].x[ib], 1. - t), mul(rods[rb].x[ib + 1], t));
        let gap = len(sub(a, b)) - 2. * radius;
        if !gap.is_finite() {
            return Err("joint strand contact geometry overflow");
        }
        if gap < -1e-10 {
            return Ok(false);
        }
    }
    Ok(true)
}
/// Re-query the current geometry without moving it. Historical projections supply
/// friction load only; old normals and released pairs must not constrain velocity.
pub(in crate::hair) fn refresh_strand_responses(
    rods: &mut [HairRod],
    radius: f64,
    history: &[StrandResponse],
) -> Vec<StrandResponse> {
    let mut current = gather_strand_contacts(rods, radius, false);
    if history.is_empty() {
        return current;
    }
    let mut witnesses: std::collections::BTreeMap<((usize, usize), (usize, usize)), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, response) in current.iter().enumerate() {
        witnesses
            .entry(((response.a.0, response.a.1), (response.b.0, response.b.1)))
            .or_default()
            .push(index);
    }
    // A historical projection supplies one pressure budget, not a new copy
    // for every point of the current manifold. Preserve unchanged witnesses
    // exactly; transfer a moved/released witness to the closest current point
    // of the same segment pair. This is a friction history hint; the coupled
    // pressure recovery solve owns newly solved physical contact reactions.
    for old in history {
        let Some(indices) = witnesses.get(&((old.a.0, old.a.1), (old.b.0, old.b.1))) else {
            continue;
        };
        let a = sub(rods[old.a.0].x[old.a.1 + 1], rods[old.a.0].x[old.a.1]);
        let b = sub(rods[old.b.0].x[old.b.1 + 1], rods[old.b.0].x[old.b.1]);
        let metric = |index: usize| {
            let new = &current[index];
            (new.a.2 - old.a.2).powi(2) * dot(a, a) + (new.b.2 - old.b.2).powi(2) * dot(b, b)
        };
        if let Some(&index) = indices
            .iter()
            .min_by(|&&a, &&b| metric(a).total_cmp(&metric(b)))
        {
            current[index].impulse += dot(old.normal, current[index].normal).max(0.) * old.impulse;
        }
    }
    current
}
// Scale before normalizing: squaring a nonzero separation can underflow,
// but that must not erase its geometric direction.
pub(super) fn normalized_contact_direction(vector: V) -> Option<V> {
    if !finite(vector) {
        return None;
    }
    let scale = vector.into_iter().map(f64::abs).fold(0f64, f64::max);
    if scale == 0. {
        return None;
    }
    Some(unit(vector.map(|value| value / scale)))
}
pub(super) fn feature_strand_normal(
    a: &HairRod,
    ia: usize,
    s: f64,
    b: &HairRod,
    ib: usize,
    t: f64,
    delta: V,
) -> V {
    let ta = normalized_contact_direction(sub(a.x[ia + 1], a.x[ia]));
    let tb = normalized_contact_direction(sub(b.x[ib + 1], b.x[ib]));
    let interior_a = s > 0. && s < 1.;
    let interior_b = t > 0. && t < 1.;
    if interior_a && interior_b {
        if let (Some(ta), Some(tb)) = (ta, tb) {
            let perpendicular = cross(ta, tb);
            if len(perpendicular) > 64. * f64::EPSILON {
                // Interior closest-point separation is perpendicular to both
                // segments. Subtracting rounded projected points can corrupt
                // its direction at tiny gaps or large world offsets; use their original tangents.
                let normal = unit(perpendicular);
                let side = normalized_contact_direction(delta)
                    .map_or(0., |direction| dot(direction, normal));
                if side > 0. {
                    return normal;
                }
                if side < 0. {
                    return mul(normal, -1.);
                }
                return coincident_strand_normal(a, ia, s, b, ib, t);
            }
        }
    }
    let mut direction = delta;
    for (interior, tangent) in [(interior_a, ta), (interior_b, tb)] {
        if interior {
            if let Some(tangent) = tangent {
                direction = cross(tangent, cross(direction, tangent));
            }
        }
    }
    normalized_contact_direction(direction)
        .unwrap_or_else(|| coincident_strand_normal(a, ia, s, b, ib, t))
}
pub(super) fn coincident_strand_normal(
    a: &HairRod,
    ia: usize,
    s: f64,
    b: &HairRod,
    ib: usize,
    t: f64,
) -> V {
    let ta = normalized_contact_direction(sub(a.x[ia + 1], a.x[ia]));
    let tb = normalized_contact_direction(sub(b.x[ib + 1], b.x[ib]));
    let old_a = add(mul(a.old_x[ia], 1. - s), mul(a.old_x[ia + 1], s));
    let old_b = add(mul(b.old_x[ib], 1. - t), mul(b.old_x[ib + 1], t));
    let history = normalized_contact_direction(sub(old_a, old_b));
    if let (Some(ta), Some(tb)) = (ta, tb) {
        let cross_direction = cross(ta, tb);
        // Only a resolvable angular difference defines a unique cross normal.
        // This dimensionless roundoff threshold does not change contact gaps
        // or admission. Parallel cases use the covariant material frame.
        if len(cross_direction) > 64. * f64::EPSILON {
            let mut normal = unit(cross_direction);
            if history.is_some_and(|old| dot(old, normal) < 0.) {
                normal = mul(normal, -1.);
            }
            return normal;
        }
    }
    let tangent = ta.or(tb);
    let transverse = |direction: V| {
        if let Some(tangent) = tangent {
            cross(tangent, cross(direction, tangent))
        } else {
            direction
        }
    };
    if let Some(history) = history {
        let direction = transverse(history);
        if len(direction) > 64. * f64::EPSILON {
            if let Some(normal) = normalized_contact_direction(direction) {
                return normal;
            }
        }
    }
    for basis in [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] {
        let direction = transverse(rotate(a.q[ia], basis));
        if len(direction) > 64. * f64::EPSILON {
            if let Some(normal) = normalized_contact_direction(direction) {
                return normal;
            }
        }
    }
    // Invalid/collapsed states have no reliable material direction. Keep a
    // finite transverse geometric direction; physical state admission still
    // owns rejection of the invalid pose, not this query helper.
    for basis in [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] {
        if let Some(normal) = normalized_contact_direction(transverse(basis)) {
            return normal;
        }
    }
    [1., 0., 0.]
}
pub(super) fn gather_strand_contacts(
    rods: &mut [HairRod],
    radius: f64,
    project_positions: bool,
) -> Vec<StrandResponse> {
    gather_strand_contacts_impl(rods, radius, project_positions, !project_positions)
}
#[cfg(test)]
pub(in crate::hair) fn reference_strand_refresh(
    rods: &mut [HairRod],
    radius: f64,
) -> Vec<StrandResponse> {
    gather_strand_contacts_impl(rods, radius, false, false)
}
pub(super) fn gather_strand_contacts_impl(
    rods: &mut [HairRod],
    radius: f64,
    project_positions: bool,
    filter_before_sort: bool,
) -> Vec<StrandResponse> {
    let mut responses = Vec::new();
    const CONTACT_TOLERANCE: f64 = 1e-10;
    let query_radius = radius + CONTACT_TOLERANCE * 0.5;
    for rod in rods.iter_mut() {
        rod.contacts
            .retain(|contact| matches!(contact.source, ContactSource::Mesh(_)));
    }
    // Segment AABBs, not just particles, enter the spatial hash. Adjacent segments are excluded.
    let cell = 0.008f64.max(radius * 4.);
    let total_segments: usize = rods.iter().map(|rod| rod.x.len().saturating_sub(1)).sum();
    let mut grid: ContactGrid = HashMap::with_capacity(total_segments.saturating_mul(2));
    let mut oversized = Vec::new();
    for (r, rod) in rods.iter().enumerate() {
        for (i, p) in rod.x.windows(2).enumerate() {
            let min: [i32; 3] = std::array::from_fn(|a| {
                ((p[0][a].min(p[1][a]) - query_radius) / cell).floor() as i32
            });
            let max: [i32; 3] = std::array::from_fn(|a| {
                ((p[0][a].max(p[1][a]) + query_radius) / cell).floor() as i32
            });
            // Keep hash insertion bounded without discarding physical pairs.
            // Oversized segments use the existing AABB hierarchy below.
            if (0..3).any(|a| i64::from(max[a]) - i64::from(min[a]) > 32) {
                oversized.push((r, i));
                continue;
            }
            for x in min[0]..=max[0] {
                for y in min[1]..=max[1] {
                    for z in min[2]..=max[2] {
                        grid.entry(cell_key([x, y, z]))
                            .or_default()
                            .push(((r, i), min));
                    }
                }
            }
        }
    }
    let query_separation = radius * 2. + CONTACT_TOLERANCE;
    // A refresh does not move geometry: discard disjoint capsules before
    // allocation/sorting, preserving exactly the same surviving pair order.
    // Sequential projection can move later pairs into range, so it retains
    // the original broad candidate list and checks live bounds below.
    let mut pairs = if !filter_before_sort {
        unique_cell_pairs(&grid)
    } else {
        unique_cell_pairs_filtered(&grid, |((ra, ia), (rb, ib))| {
            !(0..3).any(|axis| {
                let a0 = rods[ra].x[ia][axis];
                let a1 = rods[ra].x[ia + 1][axis];
                let b0 = rods[rb].x[ib][axis];
                let b1 = rods[rb].x[ib + 1][axis];
                a0.min(a1) - query_separation > b0.max(b1)
                    || b0.min(b1) - query_separation > a0.max(a1)
            })
        })
    };
    if !oversized.is_empty() {
        let segments: Vec<_> = rods
            .iter()
            .enumerate()
            .flat_map(|(r, rod)| (0..rod.x.len() - 1).map(move |i| (r, i)))
            .collect();
        let mut append = |a: SegmentId, b: SegmentId| {
            if a.0 == b.0 && a.1.abs_diff(b.1) <= 2 {
                return;
            }
            pairs.push(if a < b { (a, b) } else { (b, a) });
        };
        if project_positions {
            // Sequential projections can create later contacts. Preserve all
            // oversized candidates here, then check live capsule bounds in
            // the deterministic loop, rather than pruning stale geometry.
            for &a in &oversized {
                for &b in &segments {
                    append(a, b);
                }
            }
        } else {
            let bounds: Vec<_> = segments
                .iter()
                .map(|&(r, i)| {
                    let a = rods[r].x[i];
                    let b = rods[r].x[i + 1];
                    (
                        std::array::from_fn(|axis| a[axis].min(b[axis]) - query_radius),
                        std::array::from_fn(|axis| a[axis].max(b[axis]) + query_radius),
                    )
                })
                .collect();
            for (a, b) in continuous::bound_pairs(&bounds) {
                let a = segments[a];
                let b = segments[b];
                if oversized.binary_search(&a).is_ok() || oversized.binary_search(&b).is_ok() {
                    append(a, b);
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
    }
    for ((ra, ia), (rb, ib)) in pairs {
        let separation = radius * 2.;
        let query_separation = separation + CONTACT_TOLERANCE;
        // Grid cells are broader than a fibre. Reject disjoint current capsule
        // bounds before the more expensive segment-distance calculation.
        if !filter_before_sort
            && (0..3).any(|axis| {
                let a0 = rods[ra].x[ia][axis];
                let a1 = rods[ra].x[ia + 1][axis];
                let b0 = rods[rb].x[ib][axis];
                let b1 = rods[rb].x[ib + 1][axis];
                a0.min(a1) - query_separation > b0.max(b1)
                    || b0.min(b1) - query_separation > a0.max(a1)
            })
        {
            continue;
        }
        let a = rods[ra].x[ia];
        let b = rods[ra].x[ia + 1];
        let c = rods[rb].x[ib];
        let d = rods[rb].x[ib + 1];
        let (s, t, p, q) = segment_pair(a, b, c, d);
        if len(sub(p, q)) > query_separation {
            continue;
        }
        // A parallel capsule pair has an interval of closest points. The
        // minimum alone cannot constrain relative velocity at its other end.
        // Endpoint-to-segment projections expose both interval endpoints and
        // preserve valid endpoint contacts for oblique finite segments too.
        let project_fraction = |point: V, origin: V, end: V| {
            let direction = sub(end, origin);
            let square = dot(direction, direction);
            if square == 0. {
                0.
            } else {
                (dot(sub(point, origin), direction) / square).clamp(0., 1.)
            }
        };
        let witnesses = [
            (s, t),
            (0., project_fraction(a, c, d)),
            (1., project_fraction(b, c, d)),
            (project_fraction(c, a, b), 0.),
            (project_fraction(d, a, b), 1.),
        ];
        for (witness, &(s, t)) in witnesses.iter().enumerate() {
            if witnesses[..witness].contains(&(s, t)) {
                continue;
            }
            // Legacy sequential projection changes the live endpoints. Recheck
            // each secondary witness against that updated geometry, so the same
            // initial penetration is not projected repeatedly.
            let (p, q) = if witness == 0 {
                (p, q)
            } else {
                (
                    add(
                        rods[ra].x[ia],
                        mul(sub(rods[ra].x[ia + 1], rods[ra].x[ia]), s),
                    ),
                    add(
                        rods[rb].x[ib],
                        mul(sub(rods[rb].x[ib + 1], rods[rb].x[ib]), t),
                    ),
                )
            };
            let delta = sub(p, q);
            let distance = len(delta);
            if distance > query_separation {
                continue;
            }
            // Shared/nearby follicles are allowed to overlap at the pinned boundary.
            if ia == 0 && ib == 0 && s < 0.2 && t < 0.2 {
                continue;
            }
            // Interior witnesses require a normal perpendicular to their
            // segment tangents even at ordinary gaps. Subtracting projected
            // world points adds tangential roundoff after large translations.
            // Endpoint/endpoint witnesses retain their separation direction.
            let normal = if (s == 0. || s == 1.) && (t == 0. || t == 1.) && distance > 1e-12 {
                mul(delta, 1. / distance)
            } else {
                feature_strand_normal(&rods[ra], ia, s, &rods[rb], ib, t, delta)
            };
            let wa = contact_weight(&rods[ra], ia, s);
            let wb = contact_weight(&rods[rb], ib, t);
            if wa + wb < 1e-30 {
                continue;
            }
            let depth = separation - distance;
            let impulse = if project_positions {
                pair_impulse(rods, (ra, ia, s), (rb, ib, t), depth)
            } else {
                0.
            };
            // Keep the physical separation target from before projection. The
            // displacement limiter controls this iteration, not the contact gap:
            // an incomplete projection must still enter the structural solve.
            responses.push(StrandResponse {
                a: (ra, ia, s),
                b: (rb, ib, t),
                normal,
                impulse,
            });
            let target_a = add(p, mul(normal, depth * wa / (wa + wb)));
            let target_b = add(q, mul(normal, -depth * wb / (wa + wb)));
            if wa > 0.
                && (!project_positions || project(&mut rods[ra], ia, s, normal, impulse * wa, None))
            {
                rods[ra].record_contact(
                    ia,
                    s,
                    normal,
                    target_a,
                    ContactSource::Strand {
                        other_rod: rb,
                        other_segment: ib,
                    },
                );
            }
            if wb > 0.
                && (!project_positions
                    || project(&mut rods[rb], ib, t, mul(normal, -1.), impulse * wb, None))
            {
                rods[rb].record_contact(
                    ib,
                    t,
                    mul(normal, -1.),
                    target_b,
                    ContactSource::Strand {
                        other_rod: ra,
                        other_segment: ia,
                    },
                );
            }
        }
    }
    responses
}
