//! Static, two-sided authored triangles. Broad phase shares the engine triangle BVH.
//! Coordinates and capsule queries remain f64; no collision proxy replaces the mesh.
use crate::triangle_index::TriangleIndex;
type V = [f64; 3];
fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: V, s: f64) -> V {
    a.map(|v| v * s)
}
fn dot(a: V, b: V) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V) -> f64 {
    dot(a, a).sqrt()
}
fn segment_point(p: V, a: V, b: V) -> V {
    let d = sub(b, a);
    let n = dot(d, d);
    if n == 0. {
        a
    } else {
        add(a, mul(d, (dot(sub(p, a), d) / n).clamp(0., 1.)))
    }
}
fn closest(p: V, t: [V; 3]) -> V {
    let [a, b, c] = t;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0. && d2 <= 0. {
        return a;
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0. && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return add(a, mul(ab, d1 / (d1 - d3)));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0. && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return add(a, mul(ac, d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        return add(b, mul(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6))));
    }
    let denominator = va + vb + vc;
    if denominator == 0. {
        return [
            segment_point(p, a, b),
            segment_point(p, b, c),
            segment_point(p, c, a),
        ]
        .into_iter()
        .min_by(|a, b| dot(sub(p, *a), sub(p, *a)).total_cmp(&dot(sub(p, *b), sub(p, *b))))
        .unwrap();
    }
    add(a, add(mul(ab, vb / denominator), mul(ac, vc / denominator)))
}
fn pair(a: V, b: V, c: V, d: V) -> (V, V) {
    let u = sub(b, a);
    let v = sub(d, c);
    let w = sub(a, c);
    let aa = dot(u, u);
    let bb = dot(u, v);
    let cc = dot(v, v);
    let dd = dot(u, w);
    let ee = dot(v, w);
    let mut s = if aa == 0. {
        0.
    } else {
        let denom = aa * cc - bb * bb;
        if denom > 0. {
            ((bb * ee - cc * dd) / denom).clamp(0., 1.)
        } else {
            0.
        }
    };
    let mut t = if cc == 0. { 0. } else { (bb * s + ee) / cc };
    if t < 0. {
        t = 0.;
        s = if aa == 0. {
            0.
        } else {
            (-dd / aa).clamp(0., 1.)
        };
    } else if t > 1. {
        t = 1.;
        s = if aa == 0. {
            0.
        } else {
            ((bb - dd) / aa).clamp(0., 1.)
        };
    }
    (add(a, mul(u, s)), add(c, mul(v, t)))
}
fn segment_triangle(a: V, b: V, t: [V; 3]) -> (V, V) {
    let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
    let d = sub(b, a);
    let denom = dot(n, d);
    if denom != 0. {
        let s = dot(n, sub(t[0], a)) / denom;
        if (0. ..=1.).contains(&s) {
            let p = add(a, mul(d, s));
            let q = closest(p, t);
            if norm(sub(p, q)) <= 32. * f64::EPSILON * (1. + norm(p)) {
                return (p, p);
            }
        }
    }
    let mut best = (a, closest(a, t));
    let mut distance = dot(sub(best.0, best.1), sub(best.0, best.1));
    for candidate in
        std::iter::once((b, closest(b, t))).chain((0..3).map(|i| pair(a, b, t[i], t[(i + 1) % 3])))
    {
        let d = dot(sub(candidate.0, candidate.1), sub(candidate.0, candidate.1));
        if d < distance {
            best = candidate;
            distance = d;
        }
    }
    best
}
#[derive(Debug)]
pub struct StaticTriangleWorld {
    triangles: Vec<[V; 3]>,
    index: TriangleIndex,
}
#[derive(Clone, Copy, Debug)]
pub struct CapsuleHit {
    pub fraction: f64,
    pub normal: V,
    pub triangle: usize,
}
impl StaticTriangleWorld {
    pub fn new(triangles: Vec<[V; 3]>) -> Result<Self, &'static str> {
        if triangles.is_empty() || triangles.iter().flatten().flatten().any(|x| !x.is_finite()) {
            return Err("invalid static triangle world");
        }
        let index = TriangleIndex::new(&triangles);
        Ok(Self { triangles, index })
    }
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
    /// Conservative advancement uses the translation length as a Lipschitz bound.
    /// Exhaustion fails closed at the last safe fraction, rather than tunnelling.
    pub fn sweep_capsule(
        &self,
        a: V,
        b: V,
        radius: f64,
        delta: V,
    ) -> Result<Option<CapsuleHit>, &'static str> {
        if [a, b, delta].iter().flatten().any(|x| !x.is_finite())
            || !radius.is_finite()
            || radius <= 0.
        {
            return Err("invalid capsule sweep");
        }
        let end_a = add(a, delta);
        let end_b = add(b, delta);
        let lo = std::array::from_fn(|i| a[i].min(b[i]).min(end_a[i]).min(end_b[i]));
        let hi = std::array::from_fn(|i| a[i].max(b[i]).max(end_a[i]).max(end_b[i]));
        let mut candidates = Vec::new();
        self.index
            .query([lo, hi, lo], radius + 1e-8, &mut candidates);
        let speed = norm(delta);
        let mut best: Option<CapsuleHit> = None;
        for id in candidates {
            let triangle = self.triangles[id];
            let mut time = 0.;
            for iteration in 0..96 {
                if best.is_some_and(|hit| time > hit.fraction) {
                    break;
                }
                let (p, q) =
                    segment_triangle(add(a, mul(delta, time)), add(b, mul(delta, time)), triangle);
                let v = sub(p, q);
                let distance = norm(v);
                let normal = if distance > 0. {
                    mul(v, 1. / distance)
                } else {
                    let n = cross(sub(triangle[1], triangle[0]), sub(triangle[2], triangle[0]));
                    let len = norm(n);
                    if len == 0. {
                        break;
                    }
                    let n = mul(n, 1. / len);
                    if dot(n, delta) > 0. { mul(n, -1.) } else { n }
                };
                let gap = distance - radius;
                if gap <= 1e-8 || iteration == 95 {
                    if dot(normal, delta) < -1e-12 {
                        best = Some(CapsuleHit {
                            fraction: time,
                            normal,
                            triangle: id,
                        });
                    }
                    break;
                }
                if speed == 0. {
                    break;
                }
                let step = gap / speed;
                if time + step > 1. {
                    break;
                }
                if time + step == time {
                    best = Some(CapsuleHit {
                        fraction: time,
                        normal,
                        triangle: id,
                    });
                    break;
                }
                time += step;
            }
        }
        Ok(best)
    }
    /// Resolve an initially intersecting authored spawn without changing the mesh.
    pub fn recover_capsule(
        &self,
        mut feet: V,
        height: f64,
        radius: f64,
        preferred: V,
    ) -> Result<V, &'static str> {
        if !height.is_finite()
            || height < 2. * radius
            || !radius.is_finite()
            || radius <= 0.
            || [feet, preferred].iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid capsule recovery");
        }
        for _ in 0..64 {
            let a = add(feet, [0., radius, 0.]);
            let b = add(feet, [0., height - radius, 0.]);
            let mut candidates = Vec::new();
            self.index.query([a, b, a], radius + 1e-5, &mut candidates);
            let mut deepest: Option<(f64, V)> = None;
            for id in candidates {
                let triangle = self.triangles[id];
                let (p, q) = segment_triangle(a, b, triangle);
                let d = sub(p, q);
                let distance = norm(d);
                let penetration = radius + 1e-6 - distance;
                if penetration <= 1e-8 {
                    continue;
                }
                let normal = if distance > 1e-12 {
                    mul(d, 1. / distance)
                } else {
                    let n = cross(sub(triangle[1], triangle[0]), sub(triangle[2], triangle[0]));
                    let len = norm(n);
                    if len == 0. {
                        continue;
                    }
                    let n = mul(n, 1. / len);
                    if dot(n, preferred) < 0. {
                        mul(n, -1.)
                    } else {
                        n
                    }
                };
                if deepest.is_none_or(|(p, _)| penetration > p) {
                    deepest = Some((penetration, normal));
                }
            }
            let Some((depth, normal)) = deepest else {
                return Ok(feet);
            };
            feet = add(feet, mul(normal, depth + 1e-6));
        }
        Err("capsule spawn remains intersecting after recovery")
    }
    /// Sweeps, then projects only inward velocity onto contact tangent planes.
    pub fn move_capsule(
        &self,
        feet: V,
        height: f64,
        radius: f64,
        delta: V,
    ) -> Result<(V, bool), &'static str> {
        if !height.is_finite() || height < 2. * radius {
            return Err("invalid capsule height");
        }
        let mut position = feet;
        let mut remaining = delta;
        let mut grounded = false;
        for _ in 0..8 {
            let a = add(position, [0., radius, 0.]);
            let b = add(position, [0., height - radius, 0.]);
            let Some(hit) = self.sweep_capsule(a, b, radius, remaining)? else {
                position = add(position, remaining);
                break;
            };
            position = add(position, mul(remaining, hit.fraction));
            grounded |= hit.normal[1] > 0.65;
            remaining = mul(remaining, 1. - hit.fraction);
            let inward = dot(remaining, hit.normal);
            if inward < 0. {
                remaining = sub(remaining, mul(hit.normal, inward));
            }
            if norm(remaining) < 1e-10 {
                break;
            }
        }
        Ok((position, grounded))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn world() -> StaticTriangleWorld {
        StaticTriangleWorld::new(vec![
            [[-10., 0., -10.], [10., 0., -10.], [10., 0., 10.]],
            [[-10., 0., -10.], [10., 0., 10.], [-10., 0., 10.]],
            [[1., 0., -10.], [1., 10., -10.], [1., 10., 10.]],
            [[1., 0., -10.], [1., 10., 10.], [1., 0., 10.]],
        ])
        .unwrap()
    }
    #[test]
    fn full_speed_wall_sweep_does_not_tunnel() {
        let (p, _) = world()
            .move_capsule([0., 0., 0.], 1.8, 0.3, [100., 0., 0.])
            .unwrap();
        assert!((p[0] - 0.7).abs() < 1e-6);
    }
    #[test]
    fn floor_and_wall_slide() {
        let (p, grounded) = world()
            .move_capsule([0., 1., 0.], 1.8, 0.3, [2., -3., 2.])
            .unwrap();
        assert!(grounded);
        assert!(p[0] <= 0.700001 && p[1] >= -1e-6);
        assert!((p[2] - 2.).abs() < 1e-5);
    }
    #[test]
    fn invalid_queries_rejected_and_degenerate_retained() {
        let mut triangles = world().triangles;
        triangles.push([[0.; 3]; 3]);
        let w = StaticTriangleWorld::new(triangles).unwrap();
        assert_eq!(w.triangle_count(), 5);
        assert!(
            w.sweep_capsule([0.; 3], [0.; 3], f64::NAN, [0.; 3])
                .is_err()
        );
    }
}
