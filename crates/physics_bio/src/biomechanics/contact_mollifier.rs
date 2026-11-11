//! IPC parallel-edge mollifier building block, not a complete contact potential.
//! Formulation: https://github.com/ipc-sim/ipc-toolkit/blob/main/src/ipc/distance/edge_edge_mollifier.hpp
use super::{Vec3, cross, dot, scale, sub};

/// Return the parallel-edge multiplier and its spatial gradient at fixed rest
/// geometry. Edges are [0,1] and [2,3]. No barrier or CCD is replaced by this API.
pub fn edge_contact_mollifier(
    rest: [Vec3; 4],
    x: [Vec3; 4],
) -> Result<(f64, [Vec3; 4]), &'static str> {
    if rest
        .iter()
        .chain(x.iter())
        .flatten()
        .any(|v| !v.is_finite())
    {
        return Err("nonfinite mollifier geometry");
    }
    let a = sub(rest[1], rest[0]);
    let b = sub(rest[3], rest[2]);
    let threshold = 1e-3 * dot(a, a) * dot(b, b);
    if !threshold.is_finite() || threshold <= 0. {
        return Err("invalid mollifier reference edges");
    }
    let u = sub(x[1], x[0]);
    let v = sub(x[3], x[2]);
    if dot(u, u) <= 0. || dot(v, v) <= 0. {
        return Err("collapsed mollifier edge");
    }
    let w = cross(u, v);
    let squared = dot(w, w);
    if !squared.is_finite() {
        return Err("mollifier overflow");
    }
    if squared >= threshold {
        return Ok((1., [[0.; 3]; 4]));
    }
    let ratio = squared / threshold;
    let derivative = 2. * (1. - ratio) / threshold;
    let du = scale(cross(v, w), 2. * derivative);
    let dv = scale(cross(w, u), 2. * derivative);
    let gradient = [scale(du, -1.), du, scale(dv, -1.), dv];
    if gradient.iter().flatten().any(|v| !v.is_finite()) {
        return Err("mollifier gradient overflow");
    }
    Ok((ratio * (2. - ratio), gradient))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observed_plateau_edge_pair_is_outside_parallel_mollification() {
        let rest = [
            [0.004592201188381078, 0.003325966317040632, 0.025],
            [0.004592201188381078, 0.003325966317040632, 0.03],
            [0.004783542904563624, 0.003464548246917325, 0.025],
            [0., 0.00375, 0.03],
        ];
        let current = [
            [
                0.0045560012995217,
                0.0030645197607002445,
                0.02503470106865289,
            ],
            [
                0.004574920135752974,
                0.003090634334835454,
                0.030057689781413727,
            ],
            [
                0.004756792289036654,
                0.0030809101455090545,
                0.025160701909512466,
            ],
            [
                0.000035671744578608064,
                0.003143184945410893,
                0.030198316860307387,
            ],
        ];
        assert_eq!(
            edge_contact_mollifier(rest, current).unwrap(),
            (1., [[0.; 3]; 4])
        );
    }
    #[test]
    fn gradient_matches_differences_and_balances_resultant() {
        let rest = [[0., 0., 0.], [1., 0., 0.], [0., 0., 0.1], [1., 0., 0.1]];
        let mut x = rest;
        x[3][1] = 0.01;
        let (m, g) = edge_contact_mollifier(rest, x).unwrap();
        assert!((m - 0.19).abs() < 1e-14);
        for i in 0..4 {
            for k in 0..3 {
                let mut plus = x;
                let mut minus = x;
                plus[i][k] += 1e-7;
                minus[i][k] -= 1e-7;
                let fd = (edge_contact_mollifier(rest, plus).unwrap().0
                    - edge_contact_mollifier(rest, minus).unwrap().0)
                    / 2e-7;
                assert!((fd - g[i][k]).abs() < 1e-7);
            }
        }
        for k in 0..3 {
            assert!(g.iter().map(|g| g[k]).sum::<f64>().abs() < 1e-14);
        }
        let shifted = x.map(|p| [p[0] + 2., p[1] - 1., p[2] + 3.]);
        assert!((edge_contact_mollifier(rest, shifted).unwrap().0 - m).abs() < 1e-13);
    }
    #[test]
    fn parallel_and_threshold_limits_and_invalid_geometry() {
        let rest = [[0., 0., 0.], [1., 0., 0.], [0., 0., 0.1], [1., 0., 0.1]];
        assert_eq!(
            edge_contact_mollifier(rest, rest).unwrap(),
            (0., [[0.; 3]; 4])
        );
        let mut x = rest;
        x[3][1] = 0.1;
        assert_eq!(edge_contact_mollifier(rest, x).unwrap(), (1., [[0.; 3]; 4]));
        x[3][1] = (1e-3_f64).sqrt();
        let (m, g) = edge_contact_mollifier(rest, x).unwrap();
        assert!((m - 1.).abs() < 1e-14);
        assert!(g.iter().flatten().all(|v| v.abs() < 1e-12));
        x[1] = x[0];
        assert!(edge_contact_mollifier(rest, x).is_err());
        x = rest;
        x[0][0] = f64::NAN;
        assert!(edge_contact_mollifier(rest, x).is_err());
    }
}
