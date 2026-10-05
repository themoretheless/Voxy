//! Experimental compensated closest-feature gap, retaining affine residuals.
use super::{Vec3, surface_distance::triangle_distance};
#[derive(Clone, Copy, Debug)]
pub(super) struct Pair(f64, f64);
pub(super) type PrecisePoint = [Pair; 3];
fn sum(a: f64, b: f64) -> Pair {
    let hi = a + b;
    let z = hi - a;
    Pair(hi, (a - (hi - z)) + (b - z))
}
impl Pair {
    fn add(self, b: Self) -> Self {
        let s = sum(self.0, b.0);
        let t = sum(self.1, b.1);
        let u = sum(s.0, s.1 + t.0);
        sum(u.0, u.1 + t.1)
    }
    fn neg(self) -> Self {
        Self(-self.0, -self.1)
    }
    fn mul(self, b: Self) -> Self {
        let hi = self.0 * b.0;
        let low = self.0.mul_add(b.0, -hi) + self.0 * b.1 + self.1 * b.0 + self.1 * b.1;
        sum(hi, low)
    }
    fn value(self) -> f64 {
        self.0 + self.1
    }
}
fn difference(a: f64, b: f64) -> Pair {
    sum(a, -b)
}
/// Anchored affine residual avoids relying on a rounded sum of barycentric weights.
pub(super) fn feature_gap(
    a: [Vec3; 3],
    b: [Vec3; 3],
    wa: [f64; 3],
    wb: [f64; 3],
    minimum: f64,
    distance: f64,
) -> Result<f64, &'static str> {
    precise_feature_gap(
        a.map(|p| p.map(|v| Pair(v, 0.))),
        b.map(|p| p.map(|v| Pair(v, 0.))),
        wa,
        wb,
        minimum,
        distance,
    )
}
pub(super) fn precise_feature_gap(
    a: [PrecisePoint; 3],
    b: [PrecisePoint; 3],
    wa: [f64; 3],
    wb: [f64; 3],
    minimum: f64,
    distance: f64,
) -> Result<f64, &'static str> {
    let delta: [Pair; 3] = std::array::from_fn(|axis| {
        let mut r = a[0][axis].add(b[0][axis].neg());
        for node in 1..3 {
            r = r.add(a[node][axis].add(a[0][axis].neg()).mul(Pair(wa[node], 0.)));
            r = r.add(b[node][axis].add(b[0][axis].neg()).mul(Pair(-wb[node], 0.)));
        }
        r
    });
    let squared = delta.into_iter().fold(Pair(0., 0.), |s, r| s.add(r.mul(r)));
    let numerator = squared
        .add(Pair(minimum, 0.).mul(Pair(minimum, 0.)).neg())
        .value();
    let gap = numerator / (distance + minimum);
    if !gap.is_finite() {
        return Err("compensated gap overflow");
    }
    Ok(gap)
}
pub(super) fn trajectory(a: Vec3, b: Vec3, time: f64) -> PrecisePoint {
    std::array::from_fn(|i| {
        if time <= 0.5 {
            Pair(a[i], 0.).add(difference(b[i], a[i]).mul(Pair(time, 0.)))
        } else {
            Pair(b[i], 0.).add(difference(a[i], b[i]).mul(Pair(1. - time, 0.)))
        }
    })
}
#[derive(Clone, Copy)]
pub(super) struct PathCoordinates<'a> {
    pub body_start: &'a [Vec3],
    pub body_end: &'a [Vec3],
    pub obstacle_start: &'a [Vec3],
    pub obstacle_end: &'a [Vec3],
    pub time: f64,
}
impl PathCoordinates<'_> {
    pub fn gap(
        self,
        face: [usize; 3],
        obstacle: [usize; 3],
        closest: super::surface_distance::Closest,
        minimum: f64,
    ) -> Result<f64, &'static str> {
        precise_feature_gap(
            face.map(|i| trajectory(self.body_start[i], self.body_end[i], self.time)),
            obstacle.map(|i| trajectory(self.obstacle_start[i], self.obstacle_end[i], self.time)),
            closest.a,
            closest.b,
            minimum,
            closest.distance,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captured_edge_gaps_agree_with_independent_decimal_oracle() {
        let cases: [([Vec3; 3], [Vec3; 3], f64); 2] = [
            (
                [
                    [
                        -0.08636394707620143,
                        0.6729495720825177,
                        -0.09664506951503982,
                    ],
                    [
                        -0.0868566125052545,
                        0.6651068785250249,
                        -0.04455422811458455,
                    ],
                    [
                        -0.02298324888170589,
                        0.6443220598444104,
                        -0.061714878168300354,
                    ],
                ],
                [
                    [-0.126679841174613, 0.6754529331010063, -0.06863771476672106],
                    [
                        -0.12282226834155513,
                        0.7048274105447977,
                        -0.0649863831105906,
                    ],
                    [
                        -0.08518929683783355,
                        0.6728915965383779,
                        -0.09686607888652063,
                    ],
                ],
                3.1222638107997636e-17,
            ),
            (
                [
                    [
                        -0.09040878695685069,
                        0.676098120185722,
                        -0.09584294203487168,
                    ],
                    [
                        -0.08664409088032021,
                        0.6697253811738947,
                        -0.04286008638423716,
                    ],
                    [
                        -0.021494519618966916,
                        0.6519847318503608,
                        -0.06350163433106278,
                    ],
                ],
                [
                    [-0.1265575871900793, 0.6784980020630489, -0.0676192414267055],
                    [
                        -0.12272059172734148,
                        0.7075790279176801,
                        -0.06375824725408999,
                    ],
                    [
                        -0.08510898195703949,
                        0.6754584142288202,
                        -0.0962930433153297,
                    ],
                ],
                4.234388982451942e-18,
            ),
        ];
        for (a, b, oracle) in cases {
            let closest = triangle_distance(a, b).unwrap();
            let g = feature_gap(a, b, closest.a, closest.b, 0.0001, closest.distance).unwrap();
            eprintln!(
                "COMPENSATED_GAP old={:.17e} compensated={g:.17e} oracle={oracle:.17e}",
                closest.distance - 0.0001
            );
            assert!((g - oracle).abs() < 1e-28, "gap={g} oracle={oracle}");
        }
    }
    #[test]
    fn compensated_gap_keeps_exact_closed_floor_closed() {
        let a = [[0., 0., 0.0001], [1., 0., 0.0001], [0., 1., 0.0001]];
        let b = a.map(|p| [p[0], p[1], 0.]);
        let closest = triangle_distance(a, b).unwrap();
        assert_eq!(
            feature_gap(a, b, closest.a, closest.b, 0.0001, closest.distance).unwrap(),
            0.
        );
    }
}
