//! Exact zero predicate for the determinant of stored finite f64 coefficients.
//! An inexpensive floating filter handles well-separated determinants; the
//! fallback adds six exact dyadic triple products without changing the matrix.
use glam::DMat4;

fn parts(value: f64) -> (bool, u64, i32) {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 2047) as i32;
    let fraction = bits & ((1_u64 << 52) - 1);
    if exponent == 0 {
        (bits >> 63 != 0, fraction, -1074)
    } else {
        (bits >> 63 != 0, fraction | (1_u64 << 52), exponent - 1075)
    }
}

fn add_word(sum: &mut [u64; 100], mut index: usize, mut value: u64) {
    while value != 0 {
        let (word, carry) = sum[index].overflowing_add(value);
        sum[index] = word;
        value = u64::from(carry);
        index += 1;
    }
}

fn ldexp_f64(value: f64, mut exp: i32) -> f64 {
    if value == 0.0 || exp == 0 {
        return value;
    }
    let mut out = value;
    while exp > 0 {
        let step = exp.min(1023);
        let factor = f64::from_bits(((1023 + step) as u64) << 52);
        out *= factor;
        exp -= step;
    }
    while exp < 0 {
        let step = exp.max(-1022);
        let factor = f64::from_bits(((1023 + step) as u64) << 52);
        out *= factor;
        exp -= step;
    }
    out
}

fn normalize_column_pow2(col: [f64; 3]) -> Option<[f64; 3]> {
    let max_abs = col[0].abs().max(col[1].abs()).max(col[2].abs());
    if max_abs == 0.0 {
        return None;
    }
    let (_, m, e) = parts(max_abs);
    if m == 0 {
        return None;
    }
    let top_bit = 63 - m.leading_zeros() as i32;
    let shift = -(e + top_bit);
    let scaled = col.map(|v| ldexp_f64(v, shift));
    if scaled
        .iter()
        .zip(col)
        .all(|(&s, orig)| s.is_finite() && ((orig == 0.0) == (s == 0.0)))
    {
        Some(scaled)
    } else {
        Some(col)
    }
}

pub(super) fn usable(matrix: DMat4) -> bool {
    if !matrix.is_finite() {
        return false;
    }
    let Some(a) = normalize_column_pow2([matrix.x_axis.x, matrix.x_axis.y, matrix.x_axis.z]) else {
        return false;
    };
    let Some(b) = normalize_column_pow2([matrix.y_axis.x, matrix.y_axis.y, matrix.y_axis.z]) else {
        return false;
    };
    let Some(c) = normalize_column_pow2([matrix.z_axis.x, matrix.z_axis.y, matrix.z_axis.z]) else {
        return false;
    };
    let terms = [
        [a[0], b[1], c[2]],
        [b[0], c[1], a[2]],
        [c[0], a[1], b[2]],
        [a[2], b[1], c[0]],
        [b[2], c[1], a[0]],
        [c[2], a[1], b[0]],
    ];
    let mut determinant = 0.;
    let mut magnitude = 0.;
    let mut filtered = true;
    for (i, term) in terms.iter().enumerate() {
        if term.contains(&0.) {
            continue;
        }
        let pair = term[0] * term[1];
        let product = pair * term[2];
        filtered &= pair.is_finite()
            && pair.abs() >= f64::MIN_POSITIVE
            && product.is_finite()
            && product.abs() >= f64::MIN_POSITIVE;
        determinant += if i < 3 { product } else { -product };
        magnitude += product.abs();
    }
    if filtered && magnitude.is_finite() && determinant.abs() > 16. * f64::EPSILON * magnitude {
        return true;
    }
    // Exact fallback on original matrix entries when Sarrus filter is near-singular or underflowed.
    let oa = matrix.x_axis;
    let ob = matrix.y_axis;
    let oc = matrix.z_axis;
    let exact_terms = [
        [oa.x, ob.y, oc.z],
        [ob.x, oc.y, oa.z],
        [oc.x, oa.y, ob.z],
        [oa.z, ob.y, oc.x],
        [ob.z, oc.y, oa.x],
        [oc.z, oa.y, ob.x],
    ];
    // Smallest possible triple exponent is -3222. Largest product plus six
    // additions uses fewer than 6300 bits, so 100 u64 limbs cover every f64.
    let mut positive = [0_u64; 100];
    let mut negative = [0_u64; 100];
    for (i, term) in exact_terms.into_iter().enumerate() {
        let [(s0, m0, e0), (s1, m1, e1), (s2, m2, e2)] = term.map(parts);
        if m0 == 0 || m1 == 0 || m2 == 0 {
            continue;
        }
        let pair = u128::from(m0) * u128::from(m1);
        let low = u128::from(pair as u64) * u128::from(m2);
        let high = (pair >> 64) * u128::from(m2) + (low >> 64);
        let product = [low as u64, high as u64, (high >> 64) as u64];
        let shift = (e0 + e1 + e2 + 3222) as usize;
        let word = shift / 64;
        let bit = shift % 64;
        let sum = if s0 ^ s1 ^ s2 ^ (i >= 3) {
            &mut negative
        } else {
            &mut positive
        };
        for (k, limb) in product.into_iter().enumerate() {
            add_word(sum, word + k, limb << bit);
            if bit != 0 {
                add_word(sum, word + k + 1, limb >> (64 - bit));
            }
        }
    }
    positive != negative
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{DVec3, DVec4};
    #[test]
    fn exact_near_dependency_survives_all_representable_scales() {
        let next = 1_f64.next_up();
        for scale in [1., 8e307, f64::MIN_POSITIVE] {
            let matrix = DMat4::from_cols(
                DVec4::new(scale, scale, scale, 0.),
                DVec4::new(scale, scale * next, scale, 0.),
                DVec4::new(scale, scale, scale * next, 0.),
                DVec4::W,
            );
            assert!(usable(matrix));
            let singular = DMat4 {
                z_axis: matrix.y_axis,
                ..matrix
            };
            assert!(!usable(singular));
        }
        assert!(usable(DMat4::from_scale(DVec3::new(
            f64::MAX,
            f64::MAX,
            f64::from_bits(1)
        ))));
        assert!(usable(DMat4::from_scale(DVec3::splat(-1e-320))));
    }
    #[test]
    fn determinant_zero_matches_independent_integer_oracle() {
        let mut seed = 123_u64;
        for _ in 0..2048 {
            let values: [i64; 9] = std::array::from_fn(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                (seed % 21) as i64 - 10
            });
            let [a, b, c, d, e, f, g, h, i] = values;
            let determinant = a * e * i + d * h * c + g * b * f - c * e * g - f * h * a - i * b * d;
            let matrix = DMat4::from_cols(
                DVec4::new(a as f64, b as f64, c as f64, 0.),
                DVec4::new(d as f64, e as f64, f as f64, 0.),
                DVec4::new(g as f64, h as f64, i as f64, 0.),
                DVec4::W,
            );
            assert_eq!(usable(matrix), determinant != 0, "{values:?}");
        }
    }
}
