//! Static triangle proximity for thin-film bridges; not continuous collision detection.
type V = [f64; 3];
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|k| a[k] - b[k])
}
fn add(a: V, b: V) -> V {
    std::array::from_fn(|k| a[k] + b[k])
}
fn mul(a: V, b: f64) -> V {
    a.map(|v| v * b)
}
fn dot(a: V, b: V) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn segment_point(p: V, a: V, b: V) -> f64 {
    let e = sub(b, a);
    let t = (dot(sub(p, a), e) / dot(e, e)).clamp(0., 1.);
    dot(sub(p, add(a, mul(e, t))), sub(p, add(a, mul(e, t))))
}
fn point_triangle(p: V, t: [V; 3]) -> f64 {
    let [a, b, c] = t;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let n = cross(ab, ac);
    let nn = dot(n, n);
    let distance = dot(sub(p, a), n);
    let q = sub(p, mul(n, distance / nn));
    let v = sub(q, a);
    let aa = dot(ab, ab);
    let bb = dot(ac, ac);
    let cc = dot(ab, ac);
    let av = dot(ab, v);
    let bv = dot(ac, v);
    let u = (av * bb - bv * cc) / nn;
    let w = (bv * aa - av * cc) / nn;
    if u >= 0. && w >= 0. && u + w <= 1. {
        distance * distance / nn
    } else {
        [
            segment_point(p, a, b),
            segment_point(p, b, c),
            segment_point(p, c, a),
        ]
        .into_iter()
        .fold(f64::INFINITY, f64::min)
    }
}
fn segment_segment(p: V, q: V, r: V, s: V) -> f64 {
    let d1 = sub(q, p);
    let d2 = sub(s, r);
    let v = sub(p, r);
    let a = dot(d1, d1);
    let e = dot(d2, d2);
    let b = dot(d1, d2);
    let c = dot(d1, v);
    let f = dot(d2, v);
    let den = a * e - b * b;
    let mut u = if den > 1e-20 * a * e {
        ((b * f - c * e) / den).clamp(0., 1.)
    } else {
        0.
    };
    let mut t = (b * u + f) / e;
    if t < 0. {
        t = 0.;
        u = (-c / a).clamp(0., 1.);
    } else if t > 1. {
        t = 1.;
        u = ((b - c) / a).clamp(0., 1.);
    }
    let diff = sub(add(p, mul(d1, u)), add(r, mul(d2, t)));
    dot(diff, diff)
}
fn intersects(p: V, q: V, t: [V; 3]) -> bool {
    let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
    let d = sub(q, p);
    let den = dot(n, d);
    if den.abs() <= 1e-14 * dot(n, n).sqrt() * dot(d, d).sqrt() {
        return false;
    }
    let f = dot(n, sub(t[0], p)) / den;
    if !(0. ..=1.).contains(&f) {
        return false;
    }
    let x = add(p, mul(d, f));
    point_triangle(x, t) <= 1e-24
}
pub(super) fn triangle_distance(a: [V; 3], b: [V; 3]) -> f64 {
    let mut result = f64::INFINITY;
    for i in 0..3 {
        if intersects(a[i], a[(i + 1) % 3], b) || intersects(b[i], b[(i + 1) % 3], a) {
            return 0.;
        }
        result = result
            .min(point_triangle(a[i], b))
            .min(point_triangle(b[i], a));
        for j in 0..3 {
            result = result.min(segment_segment(a[i], a[(i + 1) % 3], b[j], b[(j + 1) % 3]));
        }
    }
    result.sqrt()
}
pub(super) fn nearby(
    a: &[[V; 3]],
    b: &[[V; 3]],
    gap: f64,
    max_candidates: usize,
) -> Result<Vec<(usize, usize, f64)>, &'static str> {
    ProximityIndex::new(b).nearby_filtered(a, b, gap, max_candidates, |_| true, |_, _| true)
}

#[derive(Debug, Clone)]
pub(super) struct ProximityIndex {
    tree: crate::triangle_index::TriangleIndex,
}
impl ProximityIndex {
    pub fn new(triangles: &[[V; 3]]) -> Self {
        Self {
            tree: crate::triangle_index::TriangleIndex::new(triangles),
        }
    }
    pub fn refit(&mut self, triangles: &[[V; 3]]) {
        self.tree.refit(triangles);
    }
    pub fn nearby_filtered(
        &self,
        a: &[[V; 3]],
        b: &[[V; 3]],
        gap: f64,
        max_candidates: usize,
        active: impl Fn(usize) -> bool,
        include: impl Fn(usize, usize) -> bool,
    ) -> Result<Vec<(usize, usize, f64)>, &'static str> {
        let mut result = Vec::new();
        let mut tested = 0;
        for (i, &triangle) in a.iter().enumerate() {
            if !active(i) {
                continue;
            }
            let mut ids = Vec::new();
            self.tree.query(triangle, gap, &mut ids);
            for j in ids {
                if !include(i, j) {
                    continue;
                }
                tested += 1;
                if tested > max_candidates {
                    return Err("film proximity candidate budget exceeded");
                }
                let distance = triangle_distance(triangle, b[j]);
                if distance <= gap {
                    result.push((i, j, distance));
                }
            }
        }
        Ok(result)
    }
}

/// Projected overlap, excluding edge/point-only contacts from bridge conductance.
pub(super) fn overlap_area(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> f64 {
    let e = sub(a[1], a[0]);
    let e = mul(e, 1. / dot(e, e).sqrt());
    let n = cross(sub(a[1], a[0]), sub(a[2], a[0]));
    let n = mul(n, 1. / dot(n, n).sqrt());
    let f = cross(n, e);
    let project = |p| {
        let v = sub(p, a[0]);
        [dot(v, e), dot(v, f)]
    };
    let clip = a.map(project);
    let mut polygon = b.map(project).to_vec();
    let side = |a: [f64; 2], b: [f64; 2], p: [f64; 2]| {
        (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
    };
    for edge in 0..3 {
        if polygon.is_empty() {
            return 0.;
        }
        let start = clip[edge];
        let end = clip[(edge + 1) % 3];
        let input = std::mem::take(&mut polygon);
        let mut previous = *input.last().unwrap();
        let mut old = side(start, end, previous);
        for point in input {
            let value = side(start, end, point);
            if (value >= 0.) != (old >= 0.) {
                let t = old / (old - value);
                polygon.push(std::array::from_fn(|k| {
                    previous[k] + t * (point[k] - previous[k])
                }));
            }
            if value >= 0. {
                polygon.push(point);
            }
            previous = point;
            old = value;
        }
    }
    (0..polygon.len())
        .map(|i| {
            let a = polygon[i];
            let b = polygon[(i + 1) % polygon.len()];
            a[0] * b[1] - a[1] * b[0]
        })
        .sum::<f64>()
        .abs()
        * 0.5
}
