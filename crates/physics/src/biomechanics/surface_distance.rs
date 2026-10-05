//! Closest features of disjoint triangles; crossing triangles have zero distance.
#[cfg(test)]
use super::add;
use super::{Vec3, cross, dot, sub};
/// Interpolate from the nearer endpoint, retaining authored endpoints exactly.
/// Fused multiply-add avoids a separately rounded displacement product.
pub(super) fn trajectory_point(start: Vec3, end: Vec3, time: f64) -> Vec3 {
    if time == 0. {
        return start;
    }
    if time == 1. {
        return end;
    }
    std::array::from_fn(|axis| {
        if time <= 0.5 {
            time.mul_add(end[axis] - start[axis], start[axis])
        } else {
            (1. - time).mul_add(start[axis] - end[axis], end[axis])
        }
    })
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Closest {
    pub a: [f64; 3],
    pub b: [f64; 3],
    pub delta: Vec3,
    pub distance: f64,
}
// Evaluate an affine feature difference before adding a world origin.
// Compensated products/sums retain small separations between oblique features.
fn feature_delta(a: [Vec3; 3], b: [Vec3; 3], wa: [f64; 3], wb: [f64; 3]) -> Vec3 {
    let anchor = sub(a[0], b[0]);
    let edges = [
        sub(a[1], a[0]),
        sub(a[2], a[0]),
        sub(b[1], b[0]),
        sub(b[2], b[0]),
    ];
    let weights = [wa[1], wa[2], -wb[1], -wb[2]];
    std::array::from_fn(|axis| {
        let mut sum = anchor[axis];
        let mut correction = 0.;
        for (edge, weight) in edges.iter().zip(weights) {
            let product = edge[axis] * weight;
            let next = sum + product;
            correction += if sum.abs() >= product.abs() {
                (sum - next) + product
            } else {
                (product - next) + sum
            };
            correction += weight.mul_add(edge[axis], -product);
            sum = next;
        }
        sum + correction
    })
}
fn triangle_weights(t: [Vec3; 3], q: Vec3) -> [f64; 3] {
    let [a, b, c] = t;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(q, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0. && d2 <= 0. {
        return [1., 0., 0.];
    }
    let bp = sub(q, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0. && d4 <= d3 {
        return [0., 1., 0.];
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        let v = d1 / (d1 - d3);
        return [1. - v, v, 0.];
    }
    let cp = sub(q, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0. && d5 <= d6 {
        return [0., 0., 1.];
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        let v = d2 / (d2 - d6);
        return [1. - v, 0., v];
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && d4 - d3 >= 0. && d5 - d6 >= 0. {
        let v = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return [0., 1. - v, v];
    }
    let inv = 1. / (va + vb + vc);
    let v = vb * inv;
    let w = vc * inv;
    [1. - v - w, v, w]
}
fn edge_parameters(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> (f64, f64) {
    let u = sub(b, a);
    let v = sub(d, c);
    let r = sub(a, c);
    let aa = dot(u, u);
    let bb = dot(u, v);
    let cc = dot(v, v);
    let dd = dot(u, r);
    let ee = dot(v, r);
    let denominator = aa * cc - bb * bb;
    let mut s = if denominator > 1e-14 * aa * cc {
        ((bb * ee - cc * dd) / denominator).clamp(0., 1.)
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
    (s, t)
}
pub(super) fn vertex_face_closest(p: Vec3, t: [Vec3; 3]) -> (Vec3, [f64; 3], f64) {
    let w = triangle_weights(t, p);
    let delta = feature_delta([p; 3], t, [1., 0., 0.], w);
    (delta, w, dot(delta, delta).sqrt())
}
pub(super) fn edge_edge_closest(x: [Vec3; 4]) -> (Vec3, [f64; 4], f64) {
    let (s, t) = edge_parameters(x[0], x[1], x[2], x[3]);
    let delta = feature_delta(
        [x[0], x[1], x[0]],
        [x[2], x[3], x[2]],
        [1. - s, s, 0.],
        [1. - t, t, 0.],
    );
    (delta, [1. - s, s, -(1. - t), -t], dot(delta, delta).sqrt())
}
fn segment_hits(a: Vec3, b: Vec3, t: [Vec3; 3]) -> bool {
    let direction = sub(b, a);
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let p = cross(direction, e2);
    let determinant = dot(e1, p);
    let bound = (dot(e1, e1) * dot(p, p)).sqrt();
    if determinant.abs() <= 1e-14 * bound {
        return false;
    }
    let q = sub(a, t[0]);
    let u = dot(q, p) / determinant;
    let r = cross(q, e1);
    let v = dot(direction, r) / determinant;
    let s = dot(e2, r) / determinant;
    (0. ..=1.).contains(&u) && v >= 0. && u + v <= 1. && (0. ..=1.).contains(&s)
}
pub(super) fn triangle_distance(a: [Vec3; 3], b: [Vec3; 3]) -> Result<Closest, &'static str> {
    for t in [a, b] {
        let normal = cross(sub(t[1], t[0]), sub(t[2], t[0]));
        if !dot(normal, normal).is_finite() || dot(normal, normal) <= 1e-30 {
            return Err("degenerate contact triangle");
        }
    }
    for i in 0..3 {
        if segment_hits(a[i], a[(i + 1) % 3], b) || segment_hits(b[i], b[(i + 1) % 3], a) {
            return Ok(Closest {
                a: [0.; 3],
                b: [0.; 3],
                delta: [0.; 3],
                distance: 0.,
            });
        }
    }
    let mut best = Closest {
        a: [0.; 3],
        b: [0.; 3],
        delta: [0.; 3],
        distance: f64::INFINITY,
    };
    let mut consider = |wa: [f64; 3], wb: [f64; 3]| {
        let delta = feature_delta(a, b, wa, wb);
        let distance = dot(delta, delta).sqrt();
        if distance < best.distance {
            best = Closest {
                a: wa,
                b: wb,
                delta,
                distance,
            };
        }
    };
    for i in 0..3 {
        let mut w = [0.; 3];
        w[i] = 1.;
        consider(w, triangle_weights(b, a[i]));
        consider(triangle_weights(a, b[i]), w);
    }
    for i in 0..3 {
        for j in 0..3 {
            let (s, t) = edge_parameters(a[i], a[(i + 1) % 3], b[j], b[(j + 1) % 3]);
            let mut wa = [0.; 3];
            wa[i] = 1. - s;
            wa[(i + 1) % 3] = s;
            let mut wb = [0.; 3];
            wb[j] = 1. - t;
            wb[(j + 1) % 3] = t;
            consider(wa, wb);
        }
    }
    if !best.distance.is_finite() {
        return Err("contact distance overflow");
    }
    Ok(best)
}

/// Individual point-triangle and edge-edge distances for diagnostic potentials.
pub(super) fn triangle_primitive_distances(
    a: [Vec3; 3],
    b: [Vec3; 3],
) -> Result<Vec<Closest>, &'static str> {
    if triangle_distance(a, b)?.distance <= 0. {
        return Err("intersecting diagnostic triangles");
    }
    let mut rows = Vec::new();
    let mut push = |wa, wb| {
        let delta = feature_delta(a, b, wa, wb);
        rows.push(Closest {
            a: wa,
            b: wb,
            delta,
            distance: dot(delta, delta).sqrt(),
        });
    };
    for i in 0..3 {
        let mut w = [0.; 3];
        w[i] = 1.;
        push(w, triangle_weights(b, a[i]));
        push(triangle_weights(a, b[i]), w);
    }
    for i in 0..3 {
        for j in 0..3 {
            let (s, t) = edge_parameters(a[i], a[(i + 1) % 3], b[j], b[(j + 1) % 3]);
            let mut wa = [0.; 3];
            wa[i] = 1. - s;
            wa[(i + 1) % 3] = s;
            let mut wb = [0.; 3];
            wb[j] = 1. - t;
            wb[(j + 1) % 3] = t;
            push(wa, wb);
        }
    }
    Ok(rows)
}

/// Lower bound from axis-aligned boxes and both triangle planes. Each bound is
/// no greater than true surface separation; intersecting triangles have bound zero.
pub(super) fn separation_lower_bound(a: [Vec3; 3], b: [Vec3; 3]) -> f64 {
    let mut squared = 0.;
    for k in 0..3 {
        let amin = a.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
        let amax = a.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
        let bmin = b.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
        let bmax = b.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
        let gap = (amin - bmax).max(bmin - amax).max(0.);
        squared += gap * gap;
    }
    let mut bound = squared.sqrt();
    for (plane, other) in [(a, b), (b, a)] {
        let normal = cross(sub(plane[1], plane[0]), sub(plane[2], plane[0]));
        let length = dot(normal, normal).sqrt();
        if length > 0. && length.is_finite() {
            let distances = other.map(|p| dot(normal, sub(p, plane[0])) / length);
            let lo = distances.into_iter().fold(f64::INFINITY, f64::min);
            let hi = distances.into_iter().fold(f64::NEG_INFINITY, f64::max);
            bound = bound.max(lo.max(-hi).max(0.));
        }
    }
    let scale = a
        .iter()
        .chain(&b)
        .flatten()
        .map(|v| v.abs())
        .fold(1e-12_f64, f64::max);
    (bound - 64. * f64::EPSILON * scale).max(0.)
}
/// Geometry cached once per energy evaluation; never reused after deformation.
#[derive(Debug)]
pub(super) struct PreparedTriangle {
    points: [Vec3; 3],
    lo: Vec3,
    hi: Vec3,
    normal: Vec3,
    normal_length: f64,
    coordinate_scale: f64,
}
impl PreparedTriangle {
    pub(super) fn new(points: [Vec3; 3]) -> Self {
        let normal = cross(sub(points[1], points[0]), sub(points[2], points[0]));
        Self {
            points,
            lo: std::array::from_fn(|k| points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min)),
            hi: std::array::from_fn(|k| {
                points
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max)
            }),
            normal,
            normal_length: dot(normal, normal).sqrt(),
            coordinate_scale: points
                .iter()
                .flatten()
                .map(|v| v.abs())
                .fold(1e-12_f64, f64::max),
        }
    }
    pub(super) fn separation_lower_bound(&self, other: &Self) -> f64 {
        let mut squared = 0.;
        for k in 0..3 {
            let gap = (self.lo[k] - other.hi[k])
                .max(other.lo[k] - self.hi[k])
                .max(0.);
            squared += gap * gap;
        }
        let mut bound = squared.sqrt();
        for (plane, other) in [(self, other), (other, self)] {
            if plane.normal_length > 0. && plane.normal_length.is_finite() {
                let distances = other
                    .points
                    .map(|p| dot(plane.normal, sub(p, plane.points[0])) / plane.normal_length);
                let lo = distances.into_iter().fold(f64::INFINITY, f64::min);
                let hi = distances.into_iter().fold(f64::NEG_INFINITY, f64::max);
                bound = bound.max(lo.max(-hi).max(0.));
            }
        }
        (bound - 64. * f64::EPSILON * self.coordinate_scale.max(other.coordinate_scale)).max(0.)
    }
}
#[cfg(test)]
mod lower_bound_tests {
    use super::*;
    #[test]
    fn bounds_never_exceed_exact_features_on_random_oblique_triangles() {
        let mut seed = 7_u64;
        let mut random = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 11) as f64 / (1_u64 << 53) as f64 - 0.5) * 0.02
        };
        for _ in 0..10000 {
            let a = std::array::from_fn(|_| std::array::from_fn(|_| random()));
            let b = std::array::from_fn(|_| std::array::from_fn(|_| random()));
            let closest = triangle_distance(a, b).unwrap();
            assert_eq!(
                separation_lower_bound(a, b),
                PreparedTriangle::new(a).separation_lower_bound(&PreparedTriangle::new(b))
            );
            let bound = separation_lower_bound(a, b);
            assert!(
                bound <= closest.distance + 1e-13,
                "bound={bound} distance={}",
                closest.distance
            );
            if closest.distance == 0. {
                assert!(bound < 1e-13);
            }
        }
    }
}

/// Conservative advancement of two linearly moving triangle primitives.
/// The fixed iteration budget rejects unresolved paths instead of admitting them.
pub(super) fn triangle_pair_path_is_open<const CULL: bool>(
    start_a: [Vec3; 3],
    end_a: [Vec3; 3],
    start_b: [Vec3; 3],
    end_b: [Vec3; 3],
    minimum: f64,
) -> bool {
    triangle_pair_path_rejection_time::<CULL>(start_a, end_a, start_b, end_b, minimum).is_none()
}

/// First sampled unresolved/closed time; None is a certified open path.
/// Shares the exact advancement used by the boolean admission API.
pub(super) fn triangle_pair_path_rejection_time<const CULL: bool>(
    start_a: [Vec3; 3],
    end_a: [Vec3; 3],
    start_b: [Vec3; 3],
    end_b: [Vec3; 3],
    minimum: f64,
) -> Option<f64> {
    // Preserve the first rejection location used for quadrature refinement.
    // Authored endpoints remain independently guarded in world coordinates.
    if match triangle_distance(start_a, start_b) {
        Ok(closest) => closest.distance <= minimum,
        Err(_) => true,
    } {
        return Some(0.);
    }
    let endpoint_rejected = match triangle_distance(end_a, end_b) {
        Ok(closest) => closest.distance <= minimum,
        Err(_) => true,
    };
    // Subtract the linearly moving first vertex before interpolation. This
    // preserves relative trajectories and avoids rounding away a narrow gap
    // during a large common translation. The omitted translation is rigid.
    let start_origin = start_a[0];
    let end_origin = end_a[0];
    let start_a = start_a.map(|p| sub(p, start_origin));
    let end_a = end_a.map(|p| sub(p, end_origin));
    let start_b = start_b.map(|p| sub(p, start_origin));
    let end_b = end_b.map(|p| sub(p, end_origin));
    let velocity_a: [Vec3; 3] = std::array::from_fn(|i| sub(end_a[i], start_a[i]));
    let velocity_b: [Vec3; 3] = std::array::from_fn(|i| sub(end_b[i], start_b[i]));
    // Feature velocities are convex combinations of vertex velocities. Their
    // relative norm is bounded by the largest cross-pair relative velocity.
    // Unlike summed absolute speeds this removes common rigid translation.
    let mut bound = 0.0_f64;
    let mut speed_scale = 0.0_f64;
    for a in velocity_a {
        speed_scale = speed_scale.max(dot(a, a).sqrt());
        for b in velocity_b {
            speed_scale = speed_scale.max(dot(b, b).sqrt());
            let relative = sub(a, b);
            bound = bound.max(dot(relative, relative).sqrt());
        }
    }
    bound += 64. * f64::EPSILON * speed_scale;
    let reject = |reason: &str, time: f64, gap: f64, steps: usize| {
        if std::env::var_os("VOXY_CCD_REJECTION_TRACE").is_some() {
            eprintln!(
                "CCD_REJECTION reason={reason:?} time={time:.17e} gap_m={gap:.17e} relative_speed_bound_m={bound:.17e} steps={steps}"
            );
        }
        Some(time)
    };
    let mut time = 0.;
    let mut last_time = 0.;
    let mut last_gap = f64::NAN;
    for step in 0..128 {
        let at = |start: [Vec3; 3], end: [Vec3; 3]| {
            std::array::from_fn(|i| trajectory_point(start[i], end[i], time))
        };
        let a = at(start_a, end_a);
        let b = at(start_b, end_b);
        for triangle in [a, b] {
            let n = cross(sub(triangle[1], triangle[0]), sub(triangle[2], triangle[0]));
            let area = dot(n, n);
            if !area.is_finite() || area <= 1e-30 {
                return reject("degenerate triangle", time, f64::NAN, step + 1);
            }
        }
        let lower = separation_lower_bound(a, b);
        if CULL && lower - minimum > bound * (1. - time) {
            return endpoint_rejected.then_some(1.);
        }
        let Ok(closest) = triangle_distance(a, b) else {
            return reject("distance evaluation", time, f64::NAN, step + 1);
        };
        let gap = closest.distance - minimum;
        last_time = time;
        last_gap = gap;
        if gap <= 0. {
            return reject("closed gap", time, gap, step + 1);
        }
        if bound == 0. || gap > bound * (1. - time) {
            return endpoint_rejected.then_some(1.);
        }
        let next = time + 0.8 * gap / bound;
        if !next.is_finite() || next <= time {
            return reject("time increment", time, gap, step + 1);
        }
        time = next.min(1.);
    }
    reject("iteration limit", last_time, last_gap, 128)
}

#[cfg(test)]
mod relative_motion_tests {
    use super::*;
    #[test]
    fn common_translation_preserves_a_narrow_open_gap() {
        let a = [[-0.1, -0.1, 0.], [0.1, -0.1, 0.], [0., 0.1, 0.]];
        let b = a.map(|p| [p[0], p[1], 0.000102]);
        for shift in [[1., -2., 3.], [-100., 50., -7.]] {
            let end_a = a.map(|p| add(p, shift));
            let end_b = b.map(|p| add(p, shift));
            // Both planes translate equally; exact separation is constant.
            assert!(triangle_pair_path_is_open::<false>(
                a, end_a, b, end_b, 0.0001
            ));
            assert!(triangle_pair_path_is_open::<true>(
                a, end_a, b, end_b, 0.0001
            ));
        }
    }
    #[test]
    fn relative_motion_still_rejects_a_swept_crossing() {
        let a = [[-0.1, -0.1, 0.], [0.1, -0.1, 0.], [0., 0.1, 0.]];
        let b = a.map(|p| [p[0], p[1], 0.001]);
        let end_b = b.map(|p| [p[0], p[1], -0.001]);
        for shift in [[0.; 3], [1., -2., 3.]] {
            assert!(!triangle_pair_path_is_open::<true>(
                a,
                a.map(|p| add(p, shift)),
                b,
                end_b.map(|p| add(p, shift)),
                0.0001
            ));
        }
    }
}

#[cfg(test)]
mod affine_precision_tests {
    use super::*;
    #[test]
    fn oblique_vertex_face_keeps_small_separation_after_large_translation() {
        let triangle = [[0., 0., 0.], [0.5, 0., 0.5], [0., 0.5, 0.5]];
        let gap = 2.0_f64.powi(-20);
        let p = [0.125, 0.125, 0.25 + gap];
        let expected = gap / 3.0_f64.sqrt();
        for shift in [[0.; 3], [1048576.; 3], [-1048576.; 3]] {
            let (delta, weights, distance) =
                vertex_face_closest(add(p, shift), triangle.map(|v| add(v, shift)));
            assert!(
                (distance - expected).abs() < 1e-15,
                "distance={distance} expected={expected}"
            );
            assert!(weights.iter().all(|v| *v > 0.));
            for (axis, sign) in [(0, -1.), (1, -1.), (2, 1.)] {
                assert!((delta[axis] - sign * gap / 3.).abs() < 1e-15);
            }
        }
    }
    #[test]
    fn oblique_triangle_and_edge_queries_share_affine_precision() {
        let gap = 2.0_f64.powi(-20);
        let b = [[0., 0., 0.], [0.5, 0., 0.5], [0., 0.5, 0.5]];
        let a = [
            [0.125, 0.125, 0.25 + gap],
            [0.125, 0.125, 0.3],
            [0.14, 0.125, 0.4],
        ];
        let edges = [
            [0., 0., 0.],
            [0.5, 0.5, 0.],
            [0.25, 0., gap],
            [0.25, 0.5, gap],
        ];
        for shift in [[0.; 3], [1048576.; 3]] {
            let closest =
                triangle_distance(a.map(|p| add(p, shift)), b.map(|p| add(p, shift))).unwrap();
            assert!((closest.distance - gap / 3.0_f64.sqrt()).abs() < 1e-15);
            let (_, _, distance) = edge_edge_closest(edges.map(|p| add(p, shift)));
            assert!((distance - gap).abs() < 1e-15);
        }
    }
}

#[cfg(test)]
mod trajectory_precision_tests {
    use super::*;
    #[test]
    fn authored_endpoints_survive_large_displacement_cancellation() {
        let start = [1e16, -1e16, 1.];
        let end = [1., -1., 1e16];
        assert_eq!(trajectory_point(start, end, 0.), start);
        assert_eq!(trajectory_point(start, end, 1.), end);
        // The old start + (end - start) loses both unit endpoints.
        assert_ne!(add(start, sub(end, start)), end);
    }
    #[test]
    fn reversed_trajectory_uses_identical_nearest_endpoint_arithmetic() {
        let start = [1e16, -1e16, 3.];
        let end = [1., -1., 1e16];
        for t in [0., 0.125, 0.25, 0.75, 0.875, 1.] {
            assert_eq!(
                trajectory_point(start, end, t),
                trajectory_point(end, start, 1. - t)
            );
        }
    }
}

#[cfg(test)]
mod moving_pair_frame_tests {
    use super::*;
    #[test]
    fn large_common_translation_keeps_a_binary_narrow_gap_open() {
        let a = [[0., 0., 1.], [0.25, 0., 1.], [0., 0.25, 1.]];
        let distance = 2_f64.powi(-20);
        let minimum = distance - 2_f64.powi(-40);
        let b = a.map(|p| [p[0], p[1], p[2] + distance]);
        let shift = [1048576.; 3];
        let end_a = a.map(|p| add(p, shift));
        let end_b = b.map(|p| add(p, shift));
        assert_eq!(end_b[0][2] - end_a[0][2], distance);
        assert!(triangle_pair_path_is_open::<true>(
            a, end_a, b, end_b, minimum
        ));
        assert!(triangle_pair_path_is_open::<false>(
            a, end_a, b, end_b, minimum
        ));
    }
    #[test]
    fn closed_authored_endpoint_remains_rejected_in_relative_frame() {
        let a = [[0., 0., 1.], [0.25, 0., 1.], [0., 0.25, 1.]];
        let b = a.map(|p| [p[0], p[1], p[2] + 0.01]);
        let time = triangle_pair_path_rejection_time::<true>(a, a, b, a, 0.001).unwrap();
        assert!((0. ..=1.).contains(&time));
    }
}
