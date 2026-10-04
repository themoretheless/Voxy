//! Outward accumulation of conditional frozen-field integration error.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct RootRigidErrorAccumulator {
    origin: f64,
    angular: f64,
}
impl RootRigidErrorAccumulator {
    pub const ZERO: Self = Self {
        origin: 0.,
        angular: 0.,
    };
    pub fn origin_bound(self) -> f64 {
        self.origin
    }
    pub fn angular_bound(self) -> f64 {
        self.angular
    }
    /// Reference clocks are exact stored endpoints. Rates must bound the original
    /// field throughout this interval, and prefix must enclose the nominal field
    /// at its start. Rejects invalid data/overflow without changing this owner.
    /// This encloses error arithmetic, not proof of the supplied derivative rates.
    pub fn append_frozen_interval(
        &self,
        start: f64,
        end: f64,
        rates: RootTwistRateBounds,
        sample_error: RootTwistErrorBounds,
        prefix: &RootRigidEnclosure,
        nominal: RootRigidTwist,
    ) -> Result<Self, AnimationError> {
        self.append_interval_error(start, end, rates, sample_error, prefix, nominal)
    }
    /// Uniform velocity discrepancy on the whole interval, without a derivative
    /// assumption. For delta_dot=omega_original cross delta + delta_omega cross
    /// x_nominal + delta_v, the skew term preserves norm and
    /// |x_nominal(t)| <= prefix_radius + |v_nominal|*t. Integrating this bound
    /// yields ev*h + ew*(radius*h + |v_nominal|*h*h/2), even at velocity jumps.
    pub fn append_bounded_field_interval(
        &self,
        start: f64,
        end: f64,
        whole_error: RootTwistErrorBounds,
        prefix: &RootRigidEnclosure,
        nominal: RootRigidTwist,
    ) -> Result<Self, AnimationError> {
        self.append_interval_error(
            start,
            end,
            RootTwistRateBounds {
                linear: 0.,
                angular: 0.,
            },
            whole_error,
            prefix,
            nominal,
        )
    }
    fn append_interval_error(
        &self,
        start: f64,
        end: f64,
        rates: RootTwistRateBounds,
        sample_error: RootTwistErrorBounds,
        prefix: &RootRigidEnclosure,
        nominal: RootRigidTwist,
    ) -> Result<Self, AnimationError> {
        if !start.is_finite()
            || !end.is_finite()
            || start < 0.
            || end < start
            || [rates.linear, rates.angular]
                .into_iter()
                .any(|v| !v.is_finite() || v < 0.)
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        nominal.enclosure()?;
        if start == end
            || (rates.linear == 0.
                && rates.angular == 0.
                && sample_error.linear_bound() == 0.
                && sample_error.angular_bound() == 0.)
        {
            return Ok(*self);
        }
        let h = Scalar::exact(end).sub(Scalar::exact(start))?;
        let radius = |bounds: [[f64; 2]; 3]| -> Result<Scalar, AnimationError> {
            let mut sum = Scalar::exact(0.);
            for value in bounds {
                sum = sum.add(Scalar::exact(value[0].abs().max(value[1].abs())))?;
            }
            Ok(sum)
        };
        let r = radius(prefix.translation_bounds())?;
        let v = radius(nominal.linear.to_array().map(|v| [v, v]))?;
        let lv = Scalar::exact(rates.linear);
        let lw = Scalar::exact(rates.angular);
        let ev = Scalar::exact(sample_error.linear_bound());
        let ew = Scalar::exact(sample_error.angular_bound());
        let h2 = h.mul(h)?;
        let h3 = h2.mul(h)?;
        let half_h2 = h2.div_positive(2.)?;
        let local = half_h2
            .mul(lv.add(lw.mul(r)?)?)?
            .add(lw.mul(v)?.mul(h3)?.div_positive(3.)?)?
            .add(ev.mul(h)?)?
            .add(ew.mul(r.mul(h)?.add(v.mul(half_h2)?)?)?)?;
        let angular = half_h2.mul(lw)?.add(ew.mul(h)?)?;
        Ok(Self {
            origin: Scalar::exact(self.origin).add(local)?.1,
            angular: if rates.angular == 0. && sample_error.angular_bound() == 0. {
                self.angular
            } else {
                Scalar::exact(self.angular).add(angular)?.1
            },
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_accumulation_encloses_exact_clock_and_prefix_arithmetic() {
        let nominal = RootRigidTwist {
            linear: DVec3::new(0.4, 0.1, -0.2),
            angular: DVec3::Y * 0.5,
        };
        let actual = RootRigidTwist {
            linear: DVec3::new(0.41, 0.09, -0.18),
            angular: DVec3::Y * 0.51,
        };
        let error = actual.enclosure().unwrap().error_bounds(nominal).unwrap();
        let transform = RootRigidTransform {
            translation: DVec3::new(1., -0.5, 0.3),
            rotation: DQuat::IDENTITY,
        };
        let prefix = RootRigidEnclosure::from_transform(transform).unwrap();
        let mut accumulated = RootRigidErrorAccumulator::ZERO;
        for (start, end) in [(0., 0.1), (0.1, 0.3), (0.3, 0.6)] {
            let rates = RootTwistRateBounds {
                linear: 0.7,
                angular: 0.2,
            };
            accumulated = accumulated
                .append_frozen_interval(start, end, rates, error, &prefix, nominal)
                .unwrap();
            println!(
                "ERROR_ACCUMULATION {:?}",
                (
                    start,
                    end,
                    transform.translation.to_array(),
                    nominal.linear.to_array(),
                    [rates.linear, rates.angular],
                    [error.linear_bound(), error.angular_bound()],
                    [accumulated.origin_bound(), accumulated.angular_bound()]
                )
            );
        }
        assert!(
            accumulated
                .append_frozen_interval(
                    1.,
                    0.,
                    RootTwistRateBounds {
                        linear: 0.,
                        angular: 0.
                    },
                    error,
                    &prefix,
                    nominal
                )
                .is_err()
        );
        let zero = RootRigidErrorAccumulator::ZERO
            .append_frozen_interval(
                0.,
                1.,
                RootTwistRateBounds {
                    linear: 0.,
                    angular: 0.,
                },
                RootTwistErrorBounds::ZERO,
                &prefix,
                nominal,
            )
            .unwrap();
        assert_eq!(zero.origin_bound(), 0.);
        assert_eq!(zero.angular_bound(), 0.);
        let linear = RootRigidErrorAccumulator::ZERO
            .append_frozen_interval(
                0.,
                1.,
                RootTwistRateBounds {
                    linear: 1.,
                    angular: 0.,
                },
                RootTwistErrorBounds::ZERO,
                &prefix,
                nominal,
            )
            .unwrap();
        assert_eq!(linear.angular_bound(), 0.);
        assert!(linear.origin_bound() >= 0.5);
    }
}
