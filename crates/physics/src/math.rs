//! Shared `[f64; 3]` helpers (plan step B1).
//!
//! Every function here is the byte-for-byte body that the domain modules used
//! to define locally, so replacing a local copy with these does not change a
//! single floating-point result: `dot` accumulates left to right from zero,
//! `norm` is the nested `hypot`, `cross` keeps the textbook component order.

/// Row-major 3x3 matrix shared by the continuum, friction and cohesive models.
pub type Matrix = [[f64; 3]; 3];

/// Left-to-right accumulated dot product.
#[inline]
#[must_use]
pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Right-handed cross product in the textbook component order.
#[inline]
#[must_use]
pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
#[must_use]
pub fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}

#[inline]
#[must_use]
pub fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

#[inline]
#[must_use]
pub fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    a.map(|v| v * s)
}

/// Overflow-safe Euclidean length: uses a fast direct `sqrt` for normal
/// magnitudes and falls back to nested `hypot` for extreme or non-finite values.
#[inline]
#[must_use]
pub fn norm(v: [f64; 3]) -> f64 {
    let max_abs = v[0].abs().max(v[1].abs()).max(v[2].abs());
    if (1e-150..1e150).contains(&max_abs) {
        (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
    } else {
        v[0].hypot(v[1]).hypot(v[2])
    }
}

/// Returns `true` when every component of a 3-vector is finite.
#[inline]
#[must_use]
pub fn finite3(v: [f64; 3]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Row-major 3x3 matrix product `a * b`.
#[inline]
#[must_use]
pub fn mat3_mul(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| {
        std::array::from_fn(|j| a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j])
    })
}

/// Row-major 3x3 matrix transpose `a^T`.
#[inline]
#[must_use]
pub fn mat3_transpose(a: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| a[j][i]))
}

/// Row-major 3x3 determinant `det(a)`.
#[inline]
#[must_use]
pub fn mat3_det(a: Matrix) -> f64 {
    dot(a[0], cross(a[1], a[2]))
}

/// Row-major 3x3 matrix inverse when `|det(a)| > tol` and all entries are finite.
#[inline]
#[must_use]
pub fn mat3_inv(a: Matrix, tol: f64) -> Option<Matrix> {
    let c0 = cross(a[1], a[2]);
    let c1 = cross(a[2], a[0]);
    let c2 = cross(a[0], a[1]);
    let det = dot(a[0], c0);
    if !det.is_finite() || det.abs() <= tol {
        return None;
    }
    let inv_det = 1.0 / det;
    let inv = [
        [c0[0] * inv_det, c1[0] * inv_det, c2[0] * inv_det],
        [c0[1] * inv_det, c1[1] * inv_det, c2[1] * inv_det],
        [c0[2] * inv_det, c1[2] * inv_det, c2[2] * inv_det],
    ];
    inv.iter().flatten().all(|x| x.is_finite()).then_some(inv)
}

/// In-place Cholesky factorization + forward/backward substitution on a flat
/// `n x n` row-major symmetric positive-definite matrix `a` and right-hand side `b`.
///
/// Overwrites the lower triangle of `a` with `L` (`A = L * L^T`) and `b` with `x`.
#[must_use]
pub fn cholesky_solve_in_place(a: &mut [f64], b: &mut [f64], n: usize, diag_tol: f64) -> bool {
    if a.len() < n * n || b.len() < n {
        return false;
    }
    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            for k in 0..j {
                sum += a[i * n + k] * a[j * n + k];
            }
            let value = a[i * n + j] - sum;
            if i == j {
                if !value.is_finite() || value <= diag_tol {
                    return false;
                }
                a[i * n + i] = value.sqrt();
            } else {
                a[i * n + j] = value / a[j * n + j];
            }
        }
    }
    for i in 0..n {
        let mut sum = 0.0;
        for j in 0..i {
            sum += a[i * n + j] * b[j];
        }
        b[i] = (b[i] - sum) / a[i * n + i];
    }
    for i in (0..n).rev() {
        let mut sum = 0.0;
        for j in (i + 1)..n {
            sum += a[j * n + i] * b[j];
        }
        b[i] = (b[i] - sum) / a[i * n + i];
        if !b[i].is_finite() {
            return false;
        }
    }
    true
}

/// In-place dense Gaussian elimination with partial row pivoting on a flat
/// `n x n` row-major matrix `a` and right-hand side `b`.
///
/// Rejects pivots with `|pivot| <= pivot_tol` (strictly greater than `pivot_tol` required)
/// and non-finite back-substitution results. Overwrites `b` with `x`.
#[must_use]
pub fn gauss_pivoted_solve_in_place(
    a: &mut [f64],
    b: &mut [f64],
    n: usize,
    pivot_tol: f64,
) -> bool {
    if a.len() < n * n || b.len() < n {
        return false;
    }
    for col in 0..n {
        let Some(pivot) =
            (col..n).max_by(|&r0, &r1| a[r0 * n + col].abs().total_cmp(&a[r1 * n + col].abs()))
        else {
            return false;
        };
        if !a[pivot * n + col].is_finite() || a[pivot * n + col].abs() <= pivot_tol {
            return false;
        }
        if pivot != col {
            for k in col..n {
                a.swap(col * n + k, pivot * n + k);
            }
            b.swap(col, pivot);
        }
        let diag = a[col * n + col];
        for row in (col + 1)..n {
            let entry = a[row * n + col];
            if entry == 0.0 {
                continue;
            }
            let factor = entry / diag;
            for j in (col + 1)..n {
                a[row * n + j] -= factor * a[col * n + j];
            }
            b[row] -= factor * b[col];
        }
    }
    for i in (0..n).rev() {
        let mut sum = 0.0;
        for j in (i + 1)..n {
            sum += a[i * n + j] * b[j];
        }
        b[i] = (b[i] - sum) / a[i * n + i];
        if !b[i].is_finite() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_match_the_local_definitions_they_replace() {
        let a = [1.5, -2.25, 3.125];
        let b = [-0.5, 4.0, 0.75];
        assert_eq!(dot(a, b), (0..3).map(|k| a[k] * b[k]).sum::<f64>());
        assert_eq!(
            dot(a, b),
            a.into_iter().zip(b).map(|(x, y)| x * y).sum::<f64>()
        );
        assert_eq!(
            cross(a, b),
            [
                -2.25 * 0.75 - 3.125 * 4.0,
                3.125 * -0.5 - 1.5 * 0.75,
                1.5 * 4.0 - -2.25 * -0.5
            ]
        );
        assert_eq!(sub(a, b), std::array::from_fn(|k| a[k] - b[k]));
        assert_eq!(add(a, b), std::array::from_fn(|k| a[k] + b[k]));
        assert_eq!(scale(a, 2.5), a.map(|x| x * 2.5));
        assert!((norm(a) - a[0].hypot(a[1]).hypot(a[2])).abs() <= 2.0 * f64::EPSILON * norm(a));
        assert_eq!(norm([0.0, 0.0, 0.0]), 0.0);
        assert_eq!(norm([1e200, 1e200, 0.0]), 1e200 * 2f64.sqrt());
        assert_eq!(norm([1e-200, 1e-200, 0.0]), 1e-200 * 2f64.sqrt());
        assert!(norm([f64::NAN, 1.0, 0.0]).is_nan());
        assert_eq!(norm([f64::INFINITY, f64::NAN, 0.0]), f64::INFINITY);
        assert!(finite3(a));
        assert!(!finite3([1.0, f64::NAN, 0.0]));
        assert!(!finite3([1.0, f64::INFINITY, 0.0]));
    }

    #[test]
    fn mat3_and_flat_dense_solvers_roundtrip() {
        let m = [[4.0, 1.0, 2.0], [1.0, 5.0, 0.5], [2.0, 0.5, 3.0]];
        let inv = mat3_inv(m, 1e-12).expect("invertible 3x3");
        let prod = mat3_mul(m, inv);
        for (i, row) in prod.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((val - expected).abs() < 1e-12);
            }
        }
        assert_eq!(mat3_transpose(mat3_transpose(m)), m);
        assert!((mat3_det(m) * mat3_det(inv) - 1.0).abs() < 1e-12);
        assert!(mat3_inv([[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 1.0, 1.0]], 1e-12).is_none());

        let flat_spd = [4.0, 1.0, 2.0, 1.0, 5.0, 0.5, 2.0, 0.5, 3.0];
        let rhs = [9.0, 12.5, 11.0];
        let mut a_chol = flat_spd;
        let mut b_chol = rhs;
        assert!(cholesky_solve_in_place(&mut a_chol, &mut b_chol, 3, 1e-14));

        let mut a_gauss = [0.0, 1.0, 2.0, 4.0, 1.0, 2.0, 2.0, 0.5, 3.0];
        let mut b_gauss = [8.0, 9.0, 11.0];
        assert!(gauss_pivoted_solve_in_place(
            &mut a_gauss,
            &mut b_gauss,
            3,
            1e-14
        ));
        for i in 0..3 {
            let resid_spd = flat_spd[i * 3] * b_chol[0]
                + flat_spd[i * 3 + 1] * b_chol[1]
                + flat_spd[i * 3 + 2] * b_chol[2]
                - rhs[i];
            assert!(resid_spd.abs() < 1e-12);
        }
        assert!((b_gauss[1] + 2.0 * b_gauss[2] - 8.0).abs() < 1e-12);
        assert!((4.0 * b_gauss[0] + b_gauss[1] + 2.0 * b_gauss[2] - 9.0).abs() < 1e-12);
    }
}
