//! Two-component native triangular transforms. The canonical f64 factor and
//! physical admission rules remain unchanged; coefficient tails travel intact.
use super::direct::BAND;

/// A finite coefficient with a retained low component. No implicit f64 conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HairResponseCoefficient {
    parts: [f64; 2],
}
impl HairResponseCoefficient {
    /// Exact finite f64 source. Rejects nonfinite data before any transform.
    pub fn from_f64(value: f64) -> Result<Self, &'static str> {
        if !value.is_finite() {
            return Err("nonfinite expanded hair coefficient");
        }
        Ok(Self { parts: [value, 0.] })
    }
    /// High and low parts; consumers must preserve their sum, not just the high part.
    pub fn parts(self) -> [f64; 2] {
        self.parts
    }
    fn sum(a: f64, b: f64) -> [f64; 2] {
        let s = a + b;
        let v = s - a;
        [s, (a - (s - v)) + (b - v)]
    }
    fn checked(parts: [f64; 2]) -> Result<Self, &'static str> {
        if parts.iter().any(|v| !v.is_finite()) {
            return Err("expanded hair transform overflow");
        }
        Ok(Self { parts })
    }
    fn sub(self, other: Self) -> Result<Self, &'static str> {
        let [hi, lo] = Self::sum(self.parts[0], -other.parts[0]);
        let [tail, rest] = Self::sum(self.parts[1], -other.parts[1]);
        let [middle, last] = Self::sum(lo, tail);
        let [hi, lo] = Self::sum(hi, middle);
        Self::checked(Self::sum(hi, lo + (rest + last)))
    }
    fn mul(self, value: f64) -> Result<Self, &'static str> {
        let safe_product = |a: f64, b: f64| -> Result<f64, &'static str> {
            let product = a * b;
            // FMA residuals need room below the product. Reject the extreme
            // exponent range rather than silently erase unrepresentable tails.
            if a != 0. && b != 0. && product.abs() < f64::MIN_POSITIVE / f64::EPSILON {
                return Err("expanded hair product exceeds precision exponent range");
            }
            Ok(product)
        };
        let hi = safe_product(self.parts[0], value)?;
        let error = self.parts[0].mul_add(value, -hi);
        let low = safe_product(self.parts[1], value)?;
        let low_error = self.parts[1].mul_add(value, -low);
        let [middle, tail] = Self::sum(error, low);
        let [hi, lo] = Self::sum(hi, middle);
        Self::checked(Self::sum(hi, lo + (tail + low_error)))
    }
    fn div(self, divisor: f64) -> Result<Self, &'static str> {
        if !divisor.is_finite() || divisor == 0. {
            return Err("invalid expanded hair diagonal");
        }
        let checked_quotient = |v: f64| -> Result<f64, &'static str> {
            let q = v / divisor;
            if v != 0. && (q == 0. || q.is_subnormal()) {
                return Err("expanded hair quotient underflow");
            }
            Ok(q)
        };
        let q1 = Self::from_f64(checked_quotient(self.parts[0])?)?;
        let remainder = self.sub(q1.mul(divisor)?)?;
        let q2 = Self::from_f64(checked_quotient(remainder.parts[0])?)?;
        let remainder = remainder.sub(q2.mul(divisor)?)?;
        let q3 = checked_quotient(remainder.parts[0])?;
        let [tail, rest] = Self::sum(q2.parts[0], q3);
        let [hi, lo] = Self::sum(q1.parts[0], tail);
        Self::checked(Self::sum(hi, lo + rest))
    }
}

pub(super) fn lower(
    factor: &[f64],
    values: &mut [HairResponseCoefficient],
    active: std::ops::Range<usize>,
) -> Result<(), &'static str> {
    for i in active.clone() {
        let mut value = values[i];
        for j in active.start.max(i.saturating_sub(BAND - 1))..i {
            value = value.sub(values[j].mul(factor[i * BAND + i - j])?)?;
        }
        values[i] = value.div(factor[i * BAND])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_rational::BigRational;
    fn exact(v: f64) -> BigRational {
        BigRational::from_float(v).unwrap()
    }
    fn rational(v: HairResponseCoefficient) -> BigRational {
        exact(v.parts[0]) + exact(v.parts[1])
    }
    #[test]
    fn cancellation_and_division_keep_the_tail() {
        let one = HairResponseCoefficient::from_f64(1.).unwrap();
        let tail = HairResponseCoefficient::from_f64(2f64.powi(-80)).unwrap();
        let composed = one
            .sub(HairResponseCoefficient {
                parts: [-tail.parts[0], 0.],
            })
            .unwrap();
        assert_eq!(rational(composed.sub(one).unwrap()), rational(tail));
        let quotient = composed.div(3.).unwrap();
        let error = rational(quotient) * exact(3.) - rational(composed);
        assert!(error.clone() <= exact(2f64.powi(-100)) && error >= -exact(2f64.powi(-100)));
    }
    #[test]
    fn nonfinite_sources_and_overflow_fail_closed() {
        assert!(HairResponseCoefficient::from_f64(f64::NAN).is_err());
        assert!(HairResponseCoefficient::from_f64(f64::INFINITY).is_err());
        assert!(
            HairResponseCoefficient::from_f64(f64::MAX)
                .unwrap()
                .mul(2.)
                .is_err()
        );
        assert!(
            HairResponseCoefficient::from_f64(1.)
                .unwrap()
                .div(0.)
                .is_err()
        );
        assert!(
            HairResponseCoefficient::from_f64(f64::MIN_POSITIVE)
                .unwrap()
                .mul(0.5)
                .is_err()
        );
        assert!(
            HairResponseCoefficient::from_f64(f64::MIN_POSITIVE)
                .unwrap()
                .div(2.)
                .is_err()
        );
    }
}
