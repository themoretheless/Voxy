//! Experimental fixed-obstacle geometric barrier Hessian; not a force law.
use super::surface_contact::{barrier_curvature, barrier_response};
use super::surface_distance::triangle_distance;
use super::{Vec3, dot, sub};
pub(super) type Matrix = [[f64; 9]; 9];

pub(super) fn body_hessian(
    a: [Vec3; 3],
    b: [Vec3; 3],
    minimum: f64,
    activation: f64,
    stiffness: f64,
) -> Result<Matrix, &'static str> {
    let closest = triangle_distance(a, b)?;
    let (_, slope) = barrier_response(closest.distance, minimum, activation, stiffness)?;
    if slope == 0. {
        return Ok([[0.; 9]; 9]);
    }
    let curvature = barrier_curvature(closest.distance - minimum, activation, stiffness);
    let mut parameters: Vec<(Vec3, [f64; 3])> = Vec::new();
    for (points, weights, body) in [(a, closest.a, true), (b, closest.b, false)] {
        let active: Vec<_> = (0..3).filter(|&i| weights[i] > 0.).collect();
        let base = *active.first().ok_or("missing active feature")?;
        for &node in &active[1..] {
            let edge = sub(points[node], points[base]);
            let mut variation = [0.; 3];
            if body {
                variation[base] = -1.;
                variation[node] = 1.;
            }
            parameters.push((if body { edge } else { edge.map(|x| -x) }, variation));
        }
    }
    if parameters.len() > 2 {
        return Err("unsupported closest-feature dimension");
    }
    let mut inverse = [[0.; 2]; 2];
    if parameters.len() == 1 {
        inverse[0][0] = 1. / dot(parameters[0].0, parameters[0].0);
    }
    if parameters.len() == 2 {
        let aa = dot(parameters[0].0, parameters[0].0);
        let bb = dot(parameters[0].0, parameters[1].0);
        let cc = dot(parameters[1].0, parameters[1].0);
        let determinant = aa.mul_add(cc, -bb * bb);
        if determinant <= 1e-14 * aa * cc {
            return Err("ill-conditioned closest-feature metric");
        }
        inverse = [
            [cc / determinant, -bb / determinant],
            [-bb / determinant, aa / determinant],
        ];
    }
    let gradient: [f64; 9] =
        std::array::from_fn(|i| closest.a[i / 3] * closest.delta[i % 3] / closest.distance);
    let coupling: [[f64; 2]; 9] = std::array::from_fn(|i| {
        std::array::from_fn(|p| {
            if p >= parameters.len() {
                0.
            } else {
                closest.a[i / 3] * parameters[p].0[i % 3]
                    + parameters[p].1[i / 3] * closest.delta[i % 3]
            }
        })
    });
    let h = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let mut reduced = if i % 3 == j % 3 {
                closest.a[i / 3] * closest.a[j / 3]
            } else {
                0.
            };
            for p in 0..parameters.len() {
                for q in 0..parameters.len() {
                    reduced -= coupling[i][p] * inverse[p][q] * coupling[j][q];
                }
            }
            curvature * gradient[i] * gradient[j]
                + slope / closest.distance * (reduced - gradient[i] * gradient[j])
        })
    });
    if h.iter().flatten().any(|x| !x.is_finite()) {
        return Err("geometric barrier Hessian overflow");
    }
    Ok(h)
}

/// Symmetric Jacobi eigensolve, followed by clamping negative eigenvalues.
pub(super) fn project_psd(mut h: Matrix) -> Result<Matrix, &'static str> {
    let scale = h.iter().flatten().map(|x| x.abs()).fold(0., f64::max);
    if !scale.is_finite() {
        return Err("invalid metric scale");
    }
    if scale == 0. {
        return Ok(h);
    }
    for i in 0..9 {
        for j in 0..9 {
            h[i][j] /= scale;
        }
    }
    let mut vectors: Matrix =
        std::array::from_fn(|i| std::array::from_fn(|j| if i == j { 1. } else { 0. }));
    let mut converged = false;
    for _ in 0..4096 {
        let mut pair = (0, 1);
        let mut largest = 0.;
        for i in 0..9 {
            for j in i + 1..9 {
                if h[i][j].abs() > largest {
                    largest = h[i][j].abs();
                    pair = (i, j);
                }
            }
        }
        if largest <= 1e-14 {
            converged = true;
            break;
        }
        let (p, q) = pair;
        let tau = (h[q][q] - h[p][p]) / (2. * h[p][q]);
        let t = if tau >= 0. {
            1. / (tau + tau.hypot(1.))
        } else {
            -1. / (-tau + tau.hypot(1.))
        };
        let c = 1. / (1. + t * t).sqrt();
        let s = t * c;
        let off = h[p][q];
        h[p][p] -= t * off;
        h[q][q] += t * off;
        h[p][q] = 0.;
        h[q][p] = 0.;
        for k in 0..9 {
            if k != p && k != q {
                let x = h[k][p];
                let y = h[k][q];
                h[k][p] = c * x - s * y;
                h[p][k] = h[k][p];
                h[k][q] = s * x + c * y;
                h[q][k] = h[k][q];
            }
            let x = vectors[k][p];
            let y = vectors[k][q];
            vectors[k][p] = c * x - s * y;
            vectors[k][q] = s * x + c * y;
        }
    }
    if !converged {
        return Err("metric eigensolve nonconvergence");
    }
    let result: Matrix = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..9)
                .map(|k| vectors[i][k] * h[k][k].max(0.) * vectors[j][k] * scale)
                .sum()
        })
    });
    if result.iter().flatten().any(|x| !x.is_finite()) {
        return Err("projected metric overflow");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn gradient(a: [Vec3; 3], b: [Vec3; 3]) -> [f64; 9] {
        let closest = triangle_distance(a, b).unwrap();
        let (_, slope) = barrier_response(closest.distance, 0.0001, 0.003, 100.).unwrap();
        std::array::from_fn(|i| slope * closest.a[i / 3] * closest.delta[i % 3] / closest.distance)
    }
    #[test]
    fn geometric_hessian_matches_force_derivative_for_stable_features() {
        // Interior edge-edge, body vertex/obstacle face, and body face/obstacle vertex.
        let cases = [
            (
                [[0., 0., 0.], [1., 0., 0.], [0., -1., -1.]],
                [[0.4, -0.4, 0.001], [0.4, 0.4, 0.001], [1.4, 0.4, 1.]],
            ),
            (
                [[0.2, 0.2, 0.001], [0.3, 0.2, 0.2], [0.2, 0.3, 0.2]],
                [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            ),
            (
                [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
                [[0.2, 0.2, 0.001], [0.3, 0.2, 0.2], [0.2, 0.3, 0.2]],
            ),
        ];
        for (a, b) in cases {
            let h = body_hessian(a, b, 0.0001, 0.003, 100.).unwrap();
            for j in 0..9 {
                let step = 1e-7;
                let mut plus = a;
                let mut minus = a;
                plus[j / 3][j % 3] += step;
                minus[j / 3][j % 3] -= step;
                let gp = gradient(plus, b);
                let gm = gradient(minus, b);
                for i in 0..9 {
                    let fd = (gp[i] - gm[i]) / (2. * step);
                    assert!(
                        (h[i][j] - fd).abs() < 2e-5 * (1. + fd.abs()),
                        "i={i} j={j} analytic={} fd={fd}",
                        h[i][j]
                    );
                    assert!((h[i][j] - h[j][i]).abs() < 1e-10);
                }
            }
            let p = project_psd(h).unwrap();
            for seed in 0..32 {
                let v: [f64; 9] = std::array::from_fn(|i| ((seed * 19 + i * 13) as f64).sin());
                let action: f64 = (0..9)
                    .map(|i| (0..9).map(|j| v[i] * p[i][j] * v[j]).sum::<f64>())
                    .sum();
                assert!(action >= -1e-10);
            }
        }
    }
    #[test]
    fn psd_projection_retains_positive_modes_and_removes_negative_modes() {
        let mut h = [[0.; 9]; 9];
        h[0][0] = 2.;
        h[1][1] = -3.;
        h[2][2] = 4.;
        h[2][3] = 1.;
        h[3][2] = 1.;
        h[3][3] = 4.;
        let p = project_psd(h).unwrap();
        assert!((p[0][0] - 2.).abs() < 1e-12);
        assert_eq!(p[1][1], 0.);
        for i in 2..4 {
            for j in 2..4 {
                assert!((p[i][j] - h[i][j]).abs() < 1e-12);
            }
        }
    }
}
