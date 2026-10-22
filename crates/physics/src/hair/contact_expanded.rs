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
    /// Normalize a finite two-component source without first rounding its sum.
    pub fn from_parts(parts: [f64; 2]) -> Result<Self, &'static str> {
        if parts.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite expanded hair coefficient");
        }
        Self::checked(Self::sum(parts[0], parts[1]))
    }
    /// Recover a GPU expansion without rounding its three terms to one scalar.
    pub fn from_scaled_terms(terms: [f64; 3], scales: [f64; 2]) -> Result<Self, &'static str> {
        Self::from_expansion_terms(terms, scales)
    }
    /// Recover an explicitly sized GPU expansion in the native two-component
    /// arithmetic, without first collapsing its terms to a single f64.
    pub fn from_expansion_terms<const N: usize>(
        terms: [f64; N],
        scales: [f64; 2],
    ) -> Result<Self, &'static str> {
        if N == 0 {
            return Err("empty expanded hair coefficient");
        }
        let mut result = Self::from_f64(0.)?;
        for term in terms {
            result = result.sub(Self::from_f64(-term)?)?;
        }
        for scale in scales {
            if !scale.is_finite() || scale <= 0. {
                return Err("invalid expanded hair scale");
            }
            result = result.mul(scale)?;
        }
        Ok(result)
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
        // A checked finite nonzero diagonal maps exact zero to exact zero.
        // Keep the diagonal validation above; a zero RHS cannot hide bad factors.
        if self.parts[0] == 0. && self.parts[1] == 0. {
            return Self::from_f64(0.);
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
            let coefficient = factor[i * BAND + i - j];
            // Exact zero products only. Nonfinite off-diagonals still fail,
            // including when the corresponding expanded RHS is zero.
            if coefficient == 0. || (coefficient.is_finite() && values[j].parts == [0., 0.]) {
                continue;
            }
            value = value.sub(values[j].mul(coefficient)?)?;
        }
        values[i] = value.div(factor[i * BAND])?;
    }
    Ok(())
}

pub(super) fn upper(
    factor: &[f64],
    values: &mut [HairResponseCoefficient],
    active: std::ops::Range<usize>,
) -> Result<(), &'static str> {
    for i in (active.start..active.end).rev() {
        let mut value = values[i];
        for j in i + 1..(i + BAND).min(active.end) {
            let coefficient = factor[j * BAND + j - i];
            if coefficient == 0. || (coefficient.is_finite() && values[j].parts == [0., 0.]) {
                continue;
            }
            value = value.sub(values[j].mul(coefficient)?)?;
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
    fn five_gpu_terms_retain_native_precision_through_upper_cancellation() {
        let terms = [
            1.,
            2f64.powi(-30),
            2f64.powi(-60),
            2f64.powi(-90),
            2f64.powi(-100),
        ];
        let source =
            HairResponseCoefficient::from_expansion_terms(terms, [2f64.powi(33), 2f64.powi(-33)])
                .unwrap();
        let expected = terms.iter().map(|&v| exact(v)).sum::<BigRational>();
        assert_eq!(rational(source), expected);
        let mut factor = vec![0.; 2 * BAND];
        factor[0] = 1.;
        factor[BAND] = 1.;
        factor[BAND + 1] = 1.;
        let mut values = vec![source, HairResponseCoefficient::from_f64(1.).unwrap()];
        upper(&factor, &mut values, 0..2).unwrap();
        assert_eq!(rational(values[0]), expected - exact(1.));
        for terms in [
            [1., 2f64.powi(-60), 2f64.powi(-100)],
            [-1., -2f64.powi(-60), -2f64.powi(-100)],
        ] {
            assert_eq!(
                HairResponseCoefficient::from_scaled_terms(terms, [2., 0.5])
                    .unwrap()
                    .parts()
                    .map(f64::to_bits),
                HairResponseCoefficient::from_expansion_terms(terms, [2., 0.5])
                    .unwrap()
                    .parts()
                    .map(f64::to_bits)
            );
        }
        assert!(HairResponseCoefficient::from_expansion_terms([], [1., 1.]).is_err());
        assert!(
            HairResponseCoefficient::from_expansion_terms([1., f64::NAN, 0., 0., 0.], [1., 1.])
                .is_err()
        );
        assert!(
            HairResponseCoefficient::from_expansion_terms([1., 0., 0., 0., 0.], [0., 1.]).is_err()
        );
    }

    #[test]
    fn gpu_terms_survive_physical_upper_cancellation() {
        let low = 2f64.powi(-60);
        let tail = 2f64.powi(-100);
        let source = HairResponseCoefficient::from_scaled_terms([1., low, tail], [1., 1.]).unwrap();
        assert_eq!(rational(source), exact(1.) + exact(low) + exact(tail));
        let mut factor = vec![0.; 2 * BAND];
        factor[0] = 1.;
        factor[BAND] = 1.;
        factor[BAND + 1] = 1.;
        let mut values = vec![source, HairResponseCoefficient::from_f64(1.).unwrap()];
        upper(&factor, &mut values, 0..2).unwrap();
        assert_eq!(rational(values[0]), exact(low) + exact(tail));
        assert!(HairResponseCoefficient::from_parts([1., f64::NAN]).is_err());
    }

    #[test]
    fn expanded_backend_reaches_unchanged_physical_admission() {
        use crate::hair::{
            HairExpandedCoordinateRequest, HairLinearSolver, HairLinearSystem, HairResponseSystem,
        };
        struct Backend {
            calls: usize,
        }
        impl HairLinearSolver for Backend {
            fn joint_contact_expanded_enabled(&self) -> bool {
                true
            }
            fn solve(&mut self, _: &[HairLinearSystem]) -> Result<Vec<Vec<f64>>, &'static str> {
                panic!("legacy solve unexpectedly used")
            }
            fn solve_joint_expanded_batch(
                &mut self,
                requests: &[HairExpandedCoordinateRequest<'_>],
            ) -> Option<Vec<(Vec<HairResponseCoefficient>, Vec<f64>)>> {
                self.calls += 1;
                Some(
                    requests
                        .iter()
                        .map(|r| {
                            assert_ne!(
                                r.columns[0][6].parts()[1],
                                0.,
                                "whitening tail was discarded"
                            );
                            (r.columns[0].clone(), vec![1.])
                        })
                        .collect(),
                )
            }
        }
        let mut matrix = vec![0.; 12 * BAND];
        for i in 0..12 {
            matrix[i * BAND] = 9.;
        }
        let mut load = vec![0.; 12];
        load[6] = 1.;
        let request = HairResponseSystem {
            system: HairLinearSystem {
                band_width: BAND,
                matrix,
                rhs: vec![0.; 12],
                active: 6..9,
            },
            loads: vec![load],
        };
        let mut backend = Backend { calls: 0 };
        let (responses, reactions, used) =
            HairResponseSystem::solve_joint_load_inequalities_expanded(
                std::slice::from_ref(&request),
                &[1. / 9.],
                1e-14,
                &mut backend,
            )
            .unwrap();
        HairResponseSystem::validate_joint_solution(
            &[request],
            &[1. / 9.],
            &responses,
            &reactions,
            1e-14,
        )
        .unwrap();
        assert!(used);
        assert_eq!(backend.calls, 1);
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
    fn exact_zero_transform_work_still_rejects_invalid_factors() {
        let zero = HairResponseCoefficient::from_f64(0.).unwrap();
        let mut valid = vec![0.; 2 * BAND];
        valid[0] = 3.;
        valid[BAND] = 7.;
        for transform in [lower, upper] {
            let mut values = vec![zero; 2];
            transform(&valid, &mut values, 0..2).unwrap();
            assert_eq!(values, vec![zero; 2]);
            for bad in [f64::NAN, f64::INFINITY] {
                let mut factor = valid.clone();
                factor[BAND + 1] = bad;
                assert!(
                    transform(&factor, &mut [zero; 2], 0..2).is_err(),
                    "zero RHS masked nonfinite off-diagonal"
                );
            }
            let mut factor = valid.clone();
            factor[BAND] = 0.;
            assert!(
                transform(&factor, &mut [zero; 2], 0..2).is_err(),
                "zero RHS masked invalid diagonal"
            );
        }
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
