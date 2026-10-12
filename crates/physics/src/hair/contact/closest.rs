//! Closest-feature geometry between segments and triangles, plus pair impulses.
//!
//! Split out of `contact.rs` (plan step B4) with unchanged bodies.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClosestFeature {
    Vertex(usize),
    Edge(usize, usize),
    Face,
}
pub(super) fn closest_triangle(p: V, t: &Triangle) -> V {
    closest_triangle_feature(p, t).0
}
// Preserve the geometric feature selected by the closest-point region tests.
// A face normal alone does not define the distance sign at edges or vertices.
pub(super) fn closest_triangle_feature(p: V, t: &Triangle) -> (V, ClosestFeature) {
    let [a, b, c] = t.p;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0. && d2 <= 0. {
        return (a, ClosestFeature::Vertex(0));
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0. && d4 <= d3 {
        return (b, ClosestFeature::Vertex(1));
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return (add(a, mul(ab, d1 / (d1 - d3))), ClosestFeature::Edge(0, 1));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0. && d5 <= d6 {
        return (c, ClosestFeature::Vertex(2));
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return (add(a, mul(ac, d2 / (d2 - d6))), ClosestFeature::Edge(0, 2));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        return (
            add(b, mul(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)))),
            ClosestFeature::Edge(1, 2),
        );
    }
    let denom = 1. / (va + vb + vc);
    (
        add(a, add(mul(ab, vb * denom), mul(ac, vc * denom))),
        ClosestFeature::Face,
    )
}
pub(super) fn segment_pair(a: V, b: V, c: V, d: V) -> (f64, f64, V, V) {
    let u = sub(b, a);
    let v = sub(d, c);
    let w = sub(a, c);
    let aa = dot(u, u);
    let bb = dot(u, v);
    let cc = dot(v, v);
    let dd = dot(u, w);
    let ee = dot(v, w);
    if aa == 0. {
        let t = if cc > 0. { (ee / cc).clamp(0., 1.) } else { 0. };
        return (0., t, a, add(c, mul(v, t)));
    }
    if cc == 0. {
        let s = (-dd / aa).clamp(0., 1.);
        return (s, 0., add(a, mul(u, s)), c);
    }
    // |u x v|^2 is the same determinant as aa*cc-bb*bb, without
    // subtracting almost equal squared lengths for near-parallel segments.
    // The matching cross-product numerator retains their interior minimum.
    let uv = cross(u, v);
    let den = dot(uv, uv);
    let mut s = if den > 0. {
        (dot(cross(v, w), uv) / den).clamp(0., 1.)
    } else {
        0.
    };
    let mut t = (bb * s + ee) / cc;
    if t < 0. {
        t = 0.;
        s = (-dd / aa).clamp(0., 1.);
    } else if t > 1. {
        t = 1.;
        s = ((bb - dd) / aa).clamp(0., 1.);
    }
    (s, t, add(a, mul(u, s)), add(c, mul(v, t)))
}
pub(super) fn segment_triangle(a: V, b: V, tri: &Triangle) -> (f64, V, V) {
    let pa = closest_triangle(a, tri);
    let pb = closest_triangle(b, tri);
    let mut best = if len(sub(a, pa)) < len(sub(b, pb)) {
        (0., a, pa)
    } else {
        (1., b, pb)
    };
    let denominator = dot(sub(b, a), tri.normal);
    if denominator.abs() > 1e-20 {
        let s = dot(sub(tri.p[0], a), tri.normal) / denominator;
        if (0.0..=1.0).contains(&s) {
            let p = add(a, mul(sub(b, a), s));
            let (q, feature) = closest_triangle_feature(p, tri);
            if matches!(feature, ClosestFeature::Face) {
                // This is an interior plane/segment intersection, hence zero
                // separation. Reprojecting the same intersection introduces
                // ULP noise which makes flat trajectory minima pick unrelated
                // times and segment weights. Edge/vertex near misses retain
                // their actual distinct closest points below.
                return (s, p, p);
            }
            if len(sub(p, q)) < 1e-9 {
                return (s, p, q);
            }
        }
    }
    for i in 0..3 {
        let (s, _, p, q) = segment_pair(a, b, tri.p[i], tri.p[(i + 1) % 3]);
        if len(sub(p, q)) < len(sub(best.1, best.2)) {
            best = (s, p, q);
        }
    }
    best
}
// Clip the projection of a segment to the closed triangular face. Each
// half-plane is linear in its segment coordinate; normal displacement does
// not affect the cross-product test. No distance tolerance enlarges the face.
pub(super) fn projected_face_interval(a: V, b: V, tri: &Triangle) -> Option<(f64, f64)> {
    let mut lo = 0f64;
    let mut hi = 1f64;
    for edge in 0..3 {
        let origin = tri.p[edge];
        let direction = sub(tri.p[(edge + 1) % 3], origin);
        let left = dot(cross(direction, sub(a, origin)), tri.normal);
        let right = dot(cross(direction, sub(b, origin)), tri.normal);
        if left < 0. && right < 0. {
            return None;
        }
        if left < 0. {
            lo = lo.max(left / (left - right));
        }
        if right < 0. {
            hi = hi.min(left / (left - right));
        }
        if lo > hi {
            return None;
        }
    }
    Some((lo, hi))
}
pub(super) fn project(
    rod: &mut HairRod,
    i: usize,
    t: f64,
    normal: V,
    penetration: f64,
    surface_velocity: Option<V>,
) -> bool {
    if i == 0 && t < 0.15 {
        return false;
    }
    let a = 1. - t;
    let b = t;
    let denom = rod.inv_mass[i] * a * a + rod.inv_mass[i + 1] * b * b;
    if denom < 1e-30 {
        return false;
    }
    let max_weight = (rod.inv_mass[i] * a).max(rod.inv_mass[i + 1] * b);
    let impulse = (penetration / denom).min(rod.lengths[i] * 0.2 / max_weight.max(1e-30));
    let correction_a = mul(normal, impulse * rod.inv_mass[i] * a);
    let correction_b = mul(normal, impulse * rod.inv_mass[i + 1] * b);
    rod.x[i] = add(rod.x[i], correction_a);
    rod.x[i + 1] = add(rod.x[i + 1], correction_b);
    // Accumulate the applied generalized contact impulse, including fixed
    // endpoint weights and the displacement cap, rather than raw penetration.
    if let Some(velocity) = surface_velocity {
        rod.record_surface_response(i, correction_a, velocity);
        rod.record_surface_response(i + 1, correction_b, velocity);
    }
    true
}
pub(super) fn contact_weight(rod: &HairRod, i: usize, t: f64) -> f64 {
    if i == 0 && t < 0.15 {
        return 0.;
    }
    rod.inv_mass[i] * (1. - t).powi(2) + rod.inv_mass[i + 1] * t.powi(2)
}
pub(super) fn pair_impulse(
    rods: &[HairRod],
    a: (usize, usize, f64),
    b: (usize, usize, f64),
    depth: f64,
) -> f64 {
    let weight = contact_weight(&rods[a.0], a.1, a.2) + contact_weight(&rods[b.0], b.1, b.2);
    if weight < 1e-30 {
        return 0.;
    }
    let mut impulse = depth.max(0.) / weight;
    // One shared multiplier preserves equal/opposite generalized impulses.
    // Independently clamping either strand changes their momentum balance.
    for (rod, segment, fraction) in [a, b] {
        let rod = &rods[rod];
        if contact_weight(rod, segment, fraction) == 0. {
            continue;
        }
        let maximum =
            (rod.inv_mass[segment] * (1. - fraction)).max(rod.inv_mass[segment + 1] * fraction);
        impulse = impulse.min(rod.lengths[segment] * 0.2 / maximum);
    }
    impulse
}
