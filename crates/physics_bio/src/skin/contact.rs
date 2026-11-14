//! Barrier contact against rigid analytic surfaces. All geometry uses SI units.
use super::{
    Point,
    dual::{D, dot, sub},
};
/// A moving rigid sphere. Center is at the start of the step.
#[derive(Clone, Copy, Debug)]
pub struct ContactSphere {
    pub center: Point,
    pub radius: f64,
    pub velocity: Point,
}
/// A moving halfspace n.x >= offset. Normal must be unit length.
#[derive(Clone, Copy, Debug)]
pub struct ContactPlane {
    pub normal: Point,
    pub offset: f64,
    pub velocity: Point,
}
/// Barrier activates inside `distance`; stiffness in N/m³, integrated over reference area. Positive gaps are mandatory.
/// Triangle/sphere contact covers face interiors and edges, not only vertices.
#[derive(Clone, Debug)]
pub struct ContactScene {
    pub spheres: Vec<ContactSphere>,
    pub planes: Vec<ContactPlane>,
    pub distance: f64,
    pub stiffness: f64,
}
impl Default for ContactScene {
    fn default() -> Self {
        Self {
            spheres: vec![],
            planes: vec![],
            distance: 0.001,
            stiffness: 10_000.0,
        }
    }
}
impl ContactScene {
    pub(super) fn validate(&self) -> Result<(), &'static str> {
        let finite = |p: Point| p.iter().all(|v| v.is_finite());
        if !self.distance.is_finite()
            || self.distance <= 0.0
            || !self.stiffness.is_finite()
            || self.stiffness <= 0.0
            || self.spheres.iter().any(|s| {
                !finite(s.center) || !finite(s.velocity) || !s.radius.is_finite() || s.radius <= 0.0
            })
            || self.planes.iter().any(|p| {
                !finite(p.normal)
                    || !finite(p.velocity)
                    || !p.offset.is_finite()
                    || (p.normal.iter().map(|x| x * x).sum::<f64>() - 1.0).abs() > 1e-8
            })
        {
            return Err("invalid skin contacts");
        }
        Ok(())
    }
    pub(super) fn energy<const N: usize>(&self, gap: D<N>) -> Result<D<N>, &'static str> {
        if !gap.v.is_finite() || gap.v <= 0.0 {
            return Err("skin contact overlap");
        }
        if gap.v >= self.distance {
            return Ok(D::c(0.0));
        }
        Ok(-(gap - D::c(self.distance)).square()
            * (gap * (1.0 / self.distance)).ln()
            * self.stiffness)
    }
}
/// Conservative triangle AABB rejection for the finite-range sphere barrier.
/// This does not replace swept collision certification.
pub(super) fn sphere_in_range(
    positions: &[Point],
    ids: [usize; 3],
    sphere: &ContactSphere,
    dt: f64,
    margin: f64,
) -> bool {
    let center: Point = std::array::from_fn(|i| sphere.center[i] + sphere.velocity[i] * dt);
    let radius = sphere.radius + margin;
    if !radius.is_finite() || center.iter().any(|v| !v.is_finite()) {
        return true;
    }
    let gap: Point = std::array::from_fn(|axis| {
        let low = ids
            .iter()
            .map(|&i| positions[i][axis])
            .fold(f64::INFINITY, f64::min);
        let high = ids
            .iter()
            .map(|&i| positions[i][axis])
            .fold(f64::NEG_INFINITY, f64::max);
        (low - center[axis]).max(center[axis] - high).max(0.)
    });
    gap[0].hypot(gap[1]).hypot(gap[2]) <= radius + 1e-12
}
#[cfg(test)]
mod broadphase_tests {
    use super::*;
    #[test]
    fn bounding_box_never_discards_an_active_closest_feature() {
        let mut state = 42_u64;
        let mut sample = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 32) as f64 / u32::MAX as f64) * 4. - 2.
        };
        let mut active = 0;
        let mut discarded = 0;
        for _ in 0..2000 {
            let p: [Point; 3] = std::array::from_fn(|_| std::array::from_fn(|_| sample()));
            let sphere = ContactSphere {
                center: std::array::from_fn(|_| sample()),
                radius: 0.3,
                velocity: [0.; 3],
            };
            let delta = sub(
                closest(p.map(|v| v.map(D::<0>::c)), sphere.center.map(D::c)),
                sphere.center.map(D::c),
            );
            let distance = dot(delta, delta).v.sqrt();
            let retained = sphere_in_range(&p, [0, 1, 2], &sphere, 0., 0.01);
            if distance <= 0.31 {
                active += 1;
                assert!(retained, "active feature discarded at distance {distance}");
            }
            discarded += usize::from(!retained);
        }
        assert!(active > 20 && discarded > 20);
    }
}
/// Ericson closest-feature tests, differentiating the active feature exactly.
pub(super) fn closest<const N: usize>(p: [[D<N>; 3]; 3], q: [D<N>; 3]) -> [D<N>; 3] {
    let [a, b, c] = p;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(q, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1.v <= 0.0 && d2.v <= 0.0 {
        return a;
    }
    let bp = sub(q, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3.v >= 0.0 && d4.v <= d3.v {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc.v <= 0.0 && d1.v >= 0.0 && d3.v <= 0.0 {
        let v = d1 * (d1 - d3).reciprocal();
        return std::array::from_fn(|i| a[i] + ab[i] * v);
    }
    let cp = sub(q, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6.v >= 0.0 && d5.v <= d6.v {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb.v <= 0.0 && d2.v >= 0.0 && d6.v <= 0.0 {
        let w = d2 * (d2 - d6).reciprocal();
        return std::array::from_fn(|i| a[i] + ac[i] * w);
    }
    let va = d3 * d6 - d5 * d4;
    if va.v <= 0.0 && (d4.v - d3.v) >= 0.0 && (d5.v - d6.v) >= 0.0 {
        let w = (d4 - d3) * ((d4 - d3) + (d5 - d6)).reciprocal();
        return std::array::from_fn(|i| b[i] + (c[i] - b[i]) * w);
    }
    let inverse = (va + vb + vc).reciprocal();
    let v = vb * inverse;
    let w = vc * inverse;
    std::array::from_fn(|i| a[i] + ab[i] * v + ac[i] * w)
}

fn vsub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
fn vdot(a: Point, b: Point) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn norm(a: Point) -> f64 {
    vdot(a, a).sqrt()
}
fn vcross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn lerp(a: Point, b: Point, t: f64) -> Point {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}
/// Certifies the complete linear trajectory by conservative advancement;
/// an exhausted certificate fails closed instead of assuming no collision.
pub(super) fn safe_path(
    a: &[Point],
    b: &[Point],
    triangles: &[[usize; 3]],
    scene: &ContactScene,
    half_thickness: f64,
    start: f64,
    end: f64,
) -> bool {
    for triangle in triangles {
        let from = triangle.map(|i| a[i]);
        let to = triangle.map(|i| b[i]);
        let e1 = vsub(from[1], from[0]);
        let e2 = vsub(from[2], from[0]);
        let de1 = vsub(vsub(to[1], to[0]), e1);
        let de2 = vsub(vsub(to[2], to[0]), e2);
        let max_area_speed =
            norm(vcross(de1, e2)) + norm(vcross(e1, de2)) + 2.0 * norm(vcross(de1, de2));
        if !advance(
            |t| {
                let p: [Point; 3] = std::array::from_fn(|i| lerp(from[i], to[i], t));
                norm(vcross(vsub(p[1], p[0]), vsub(p[2], p[0]))) - 1e-12
            },
            max_area_speed,
        ) {
            return false;
        }
        for s in &scene.spheres {
            let center0 = std::array::from_fn(|i| s.center[i] + s.velocity[i] * start);
            let center1 = std::array::from_fn(|i| s.center[i] + s.velocity[i] * end);
            let speed = (0..3)
                .map(|i| norm(vsub(vsub(to[i], from[i]), vsub(center1, center0))))
                .fold(0.0, f64::max);
            if !advance(
                |t| {
                    let p = std::array::from_fn(|i| lerp(from[i], to[i], t).map(D::<0>::c));
                    let center = lerp(center0, center1, t).map(D::c);
                    let delta = sub(closest(p, center), center);
                    dot(delta, delta).v.sqrt() - s.radius - half_thickness
                },
                speed,
            ) {
                return false;
            }
        }
    }
    for plane in &scene.planes {
        for (old, new) in a.iter().zip(b) {
            let gap0 = vdot(*old, plane.normal)
                - plane.offset
                - vdot(plane.velocity, plane.normal) * start
                - half_thickness;
            let gap1 = vdot(*new, plane.normal)
                - plane.offset
                - vdot(plane.velocity, plane.normal) * end
                - half_thickness;
            if gap0 <= 0.0 || gap1 <= 0.0 {
                return false;
            }
        }
    }
    true
}
fn advance(mut gap: impl FnMut(f64) -> f64, speed: f64) -> bool {
    let mut t = 0.0;
    for _ in 0..512 {
        let distance = gap(t);
        if !distance.is_finite() || distance <= 0.0 {
            return false;
        }
        if distance > speed * (1.0 - t) {
            return true;
        }
        let increment = 0.9 * distance / speed;
        if increment < 1e-12 {
            return false;
        }
        t += increment;
        if t >= 1.0 {
            return gap(1.0) > 0.0;
        }
    }
    false
}
