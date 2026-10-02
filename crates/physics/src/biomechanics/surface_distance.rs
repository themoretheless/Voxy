//! Closest features of disjoint triangles; crossing triangles have zero distance.
use super::{Vec3, add, cross, dot, scale, sub};
#[derive(Clone, Copy, Debug)]
pub(super) struct Closest {
    pub a: [f64; 3],
    pub b: [f64; 3],
    pub delta: Vec3,
    pub distance: f64,
}
fn point(t: [Vec3; 3], weights: [f64; 3]) -> Vec3 {
    add(
        add(scale(t[0], weights[0]), scale(t[1], weights[1])),
        scale(t[2], weights[2]),
    )
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
    let delta = sub(p, point(t, w));
    (delta, w, dot(delta, delta).sqrt())
}
pub(super) fn edge_edge_closest(x: [Vec3; 4]) -> (Vec3, [f64; 4], f64) {
    let (s, t) = edge_parameters(x[0], x[1], x[2], x[3]);
    let delta = sub(
        add(scale(x[0], 1. - s), scale(x[1], s)),
        add(scale(x[2], 1. - t), scale(x[3], t)),
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
        let delta = sub(point(a, wa), point(b, wb));
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
        let delta = sub(point(a, wa), point(b, wb));
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
