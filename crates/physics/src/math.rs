//! Shared `[f64; 3]` helpers (plan step B1).
//!
//! Every function here is the byte-for-byte body that the domain modules used
//! to define locally, so replacing a local copy with these does not change a
//! single floating-point result: `dot` accumulates left to right from zero,
//! `norm` is the nested `hypot`, `cross` keeps the textbook component order.

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

/// Overflow-safe Euclidean length as the nested `hypot`.
#[inline]
#[must_use]
pub fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
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
        assert_eq!(norm(a), a[0].hypot(a[1]).hypot(a[2]));
        assert_eq!(norm([1e200, 1e200, 0.0]), 1e200 * 2f64.sqrt());
    }
}
