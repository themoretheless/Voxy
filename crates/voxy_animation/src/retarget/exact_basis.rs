//! Exact predicates on stored f32 quaternion components. Normalization factors
//! cancel in proportionality and in zero tests of homogeneous rotation rows.
use glam::Quat;

#[derive(Clone, Default)]
struct Sum(Vec<f64>);
impl Sum {
    fn add(&mut self, mut value: f64) {
        let mut result = Vec::with_capacity(self.0.len() + 1);
        for &term in &self.0 {
            let sum = value + term;
            let virtual_term = sum - value;
            let error = (value - (sum - virtual_term)) + (term - virtual_term);
            if error != 0. {
                result.push(error);
            }
            value = sum;
        }
        if value != 0. {
            result.push(value);
        }
        self.0 = result;
    }
    fn scaled(&self, factor: f64) -> Self {
        let mut result = Self::default();
        for &term in &self.0 {
            let product = term * factor;
            result.add(term.mul_add(factor, -product));
            result.add(product);
        }
        result
    }
    fn append(&mut self, other: &Self) {
        for &term in &other.0 {
            self.add(term);
        }
    }
    fn zero(&self) -> bool {
        self.0.is_empty()
    }
}

// Four f32 factors have a dyadic lattice no finer than 2^-596, safely
// inside f64's normal range. Finite near-unit inputs keep every intermediate
// far below overflow. TwoSum and the fused product residual are therefore exact.
const HAMILTON: [[(usize, usize, f64); 4]; 4] = [
    [(3, 0, 1.), (0, 3, 1.), (1, 2, 1.), (2, 1, -1.)],
    [(3, 1, 1.), (0, 2, -1.), (1, 3, 1.), (2, 0, 1.)],
    [(3, 2, 1.), (0, 1, 1.), (1, 0, -1.), (2, 3, 1.)],
    [(3, 3, 1.), (0, 0, -1.), (1, 1, -1.), (2, 2, -1.)],
];
fn multiply(a: &[Sum; 4], b: [f64; 4]) -> [Sum; 4] {
    std::array::from_fn(|row| {
        let mut result = Sum::default();
        for (i, j, sign) in HAMILTON[row] {
            result.append(&a[i].scaled(sign * b[j]));
        }
        result
    })
}

pub(super) fn coherent(target: Quat, correction: Quat, source: Quat, basis: Quat) -> bool {
    let a = target.to_array().map(|x| {
        let mut sum = Sum::default();
        sum.add(f64::from(x));
        sum
    });
    let product = multiply(
        &multiply(&a, correction.to_array().map(f64::from)),
        source.conjugate().to_array().map(f64::from),
    );
    let b = basis.to_array().map(f64::from);
    (0..4).all(|i| {
        (i + 1..4).all(|j| {
            let mut cross = product[i].scaled(b[j]);
            cross.append(&product[j].scaled(-b[i]));
            cross.zero()
        })
    })
}

pub(super) fn rotation_nonzero(q: Quat) -> [[bool; 3]; 3] {
    let q = q.to_array().map(f64::from);
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            let mut sum = Sum::default();
            let mut product = |i: usize, j: usize, sign: f64| {
                // Two f32 mantissas multiply exactly in f64.
                sum.add(sign * q[i] * q[j]);
            };
            if row == column {
                for axis in 0..4 {
                    product(axis, axis, if axis == row || axis == 3 { 1. } else { -1. });
                }
            } else {
                let remaining = 3 - row - column;
                let cyclic = (row + 1) % 3 == column;
                product(row, column, 1.);
                product(3, remaining, if cyclic { -1. } else { 1. });
            }
            !sum.zero()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coherence_matches_integer_hamilton_oracle_for_dyadic_inputs() {
        // Independent integer arithmetic at a fixed 2^-16 input lattice;
        // products and proportionality determinants fit exactly in i128.
        fn product(a: [i128; 4], b: [i128; 4]) -> [i128; 4] {
            let [x, y, z, w] = a;
            let [u, v, s, t] = b;
            [
                w * u + x * t + y * s - z * v,
                w * v - x * s + y * t + z * u,
                w * s + x * v - y * u + z * t,
                w * t - x * u - y * v - z * s,
            ]
        }
        let quaternion =
            |a: [i128; 4]| Quat::from_array(a.map(|component| component as f32 / 65536.));
        let mut state = 17_u64;
        let mut vector = || {
            std::array::from_fn(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                i128::from((state >> 48) as i32 - 32768)
            })
        };
        for _ in 0..512 {
            let target = vector();
            let correction = vector();
            let source = vector();
            let basis = vector();
            let conjugate = [-source[0], -source[1], -source[2], source[3]];
            let p = product(product(target, correction), conjugate);
            let expected = (0..4).all(|i| (i + 1..4).all(|j| p[i] * basis[j] == p[j] * basis[i]));
            assert_eq!(
                coherent(
                    quaternion(target),
                    quaternion(correction),
                    quaternion(source),
                    quaternion(basis)
                ),
                expected
            );
            assert!(coherent(
                quaternion(source),
                Quat::IDENTITY,
                quaternion(source),
                Quat::IDENTITY
            ));
            let [x, y, z, w] = target;
            let expected_rows = [
                [
                    w * w + x * x - y * y - z * z,
                    2 * (x * y - w * z),
                    2 * (x * z + w * y),
                ],
                [
                    2 * (x * y + w * z),
                    w * w - x * x + y * y - z * z,
                    2 * (y * z - w * x),
                ],
                [
                    2 * (x * z - w * y),
                    2 * (y * z + w * x),
                    w * w - x * x - y * y + z * z,
                ],
            ]
            .map(|row| row.map(|component| component != 0));
            assert_eq!(rotation_nonzero(quaternion(target)), expected_rows);
        }
    }
    #[test]
    fn coherence_rejects_products_that_only_round_to_the_same_quaternion() {
        let tiny = f32::from_bits(1);
        let source = Quat::from_xyzw(tiny, 0., 0., 1.);
        let target = Quat::from_xyzw(tiny, tiny, 0., 1.);
        let rounded = (target * source.conjugate()).normalize();
        // The exact product has z=tiny^2, lost by the f32 multiplication.
        assert_eq!(rounded.z, 0.);
        assert!(!coherent(target, Quat::IDENTITY, source, rounded));
        assert!(coherent(source, Quat::IDENTITY, source, Quat::IDENTITY));
    }
    #[test]
    fn homogeneous_rows_preserve_quarter_turns_and_subnormal_terms() {
        let h = core::f32::consts::FRAC_1_SQRT_2;
        assert_eq!(
            rotation_nonzero(Quat::from_xyzw(0., 0., h, h)),
            [
                [false, true, false],
                [true, false, false],
                [false, false, true]
            ]
        );
        let tiny = f32::from_bits(1);
        let rows = rotation_nonzero(Quat::from_xyzw(tiny, tiny, 0., 1.));
        assert!(rows[0][1]); // tiny^2 must not disappear.
        assert!(rows[1][0]);
    }
}
