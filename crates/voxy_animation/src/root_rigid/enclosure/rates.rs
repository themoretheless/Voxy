//! Whole-span angular bounds of the normalized stored polynomial reference.
use super::*;
use crate::RootRotationSpan;
#[derive(Clone, Copy, Debug)]
pub struct RootAngularDerivativeBounds {
    speed: f64,
    acceleration: f64,
}
impl RootAngularDerivativeBounds {
    pub fn speed_bound(self) -> f64 {
        self.speed
    }
    pub fn acceleration_bound(self) -> f64 {
        self.acceleration
    }
}
fn l1<const N: usize>(v: [Scalar; N]) -> Result<Scalar, AnimationError> {
    let mut total = Scalar::exact(0.);
    for value in v {
        total = total.add(Scalar::exact(value.0.abs().max(value.1.abs())))?;
    }
    Ok(total)
}
impl RootRotationSpan {
    /// Whole continuous span bound with outward coefficient/time arithmetic.
    /// Cubic norm positivity is proved by its Bernstein component hull. A hull
    /// that cannot exclude zero rejects; sampled norm checks are not a fallback.
    /// Frames are constant normalized real rotations; compilation is separate.
    pub fn enclosed_angular_derivative_bounds(
        &self,
    ) -> Result<Option<RootAngularDerivativeBounds>, AnimationError> {
        if self.end() <= self.start() || self.is_step() {
            return Ok(None);
        }
        let h = Scalar::exact(self.end()).sub(Scalar::exact(self.start()))?;
        if h.0 <= 0. {
            return Err(AnimationError::RootRotationBudget);
        }
        let Some((control, _, _)) = self.cubic_velocity_inputs() else {
            let Some((_, axis, _, _)) = self.arc_velocity_inputs() else {
                return Ok(None);
            };
            if axis == DVec3::ZERO {
                return Ok(Some(RootAngularDerivativeBounds {
                    speed: 0.,
                    acceleration: 0.,
                }));
            }
            let speed = l1(axis.to_array().map(Scalar::exact))?
                .div_interval_positive(h)?
                .1;
            return Ok(Some(RootAngularDerivativeBounds {
                speed,
                acceleration: 0.,
            }));
        };
        let mut norm_squared = Scalar::exact(0.);
        for j in 0..4 {
            let lo = control.iter().map(|v| v[j]).fold(f64::INFINITY, f64::min);
            let hi = control
                .iter()
                .map(|v| v[j])
                .fold(f64::NEG_INFINITY, f64::max);
            let nearest = if lo <= 0. && hi >= 0. {
                0.
            } else {
                lo.abs().min(hi.abs())
            };
            norm_squared = norm_squared.add(Scalar::exact(nearest).square()?)?;
        }
        if norm_squared.0 <= 0. {
            return Err(AnimationError::RootRotationBudget);
        }
        let minimum = Scalar::exact(norm_squared.0.sqrt().next_down());
        let c = control.map(|v| v.map(Scalar::exact));
        let mut first = 0_f64;
        let mut second = 0_f64;
        for i in 0..3 {
            let mut derivative = [Scalar::exact(0.); 4];
            for j in 0..4 {
                derivative[j] = c[i + 1][j]
                    .sub(c[i][j])?
                    .mul(Scalar::exact(3.))?
                    .div_interval_positive(h)?;
            }
            first = first.max(l1(derivative)?.1);
        }
        for i in 0..2 {
            let mut derivative = [Scalar::exact(0.); 4];
            for j in 0..4 {
                derivative[j] = c[i + 2][j]
                    .sub(c[i + 1][j].mul(Scalar::exact(2.))?)?
                    .add(c[i][j])?
                    .mul(Scalar::exact(6.))?
                    .div_interval_positive(h.mul(h)?)?;
            }
            second = second.max(l1(derivative)?.1);
        }
        let ratio = Scalar::exact(first).div_interval_positive(minimum)?;
        let speed = ratio.mul(Scalar::exact(2.))?.1;
        let acceleration = Scalar::exact(second)
            .div_interval_positive(minimum)?
            .mul(Scalar::exact(2.))?
            .add(ratio.mul(ratio)?.mul(Scalar::exact(4.))?)?
            .1;
        Ok(Some(RootAngularDerivativeBounds {
            speed,
            acceleration,
        }))
    }
}

fn polynomial_bounds(control: [DVec3; 4], h: Scalar) -> Result<[Scalar; 3], AnimationError> {
    let c = control.map(|v| v.to_array().map(Scalar::exact));
    let mut position = 0_f64;
    let mut first = 0_f64;
    let mut second = 0_f64;
    for v in c {
        position = position.max(l1(v)?.1);
    }
    for i in 0..3 {
        let mut d = [Scalar::exact(0.); 3];
        for j in 0..3 {
            d[j] = c[i + 1][j]
                .sub(c[i][j])?
                .mul(Scalar::exact(3.))?
                .div_interval_positive(h)?;
        }
        first = first.max(l1(d)?.1);
    }
    for i in 0..2 {
        let mut d = [Scalar::exact(0.); 3];
        for j in 0..3 {
            d[j] = c[i + 2][j]
                .sub(c[i + 1][j].mul(Scalar::exact(2.))?)?
                .add(c[i][j])?
                .mul(Scalar::exact(6.))?
                .div_interval_positive(h.mul(h)?)?;
        }
        second = second.max(l1(d)?.1);
    }
    Ok([
        Scalar::exact(position),
        Scalar::exact(first),
        Scalar::exact(second),
    ])
}
impl RootRigidSpan {
    /// Outward whole-span speed/rate bounds for the canonical stored spatial field.
    /// Bernstein derivative hulls retain all additive and moving-pivot terms.
    /// STEP has no finite field; an unproved rotation norm rejects.
    pub fn enclosed_twist_bounds(&self) -> Result<Option<RootSpatialTwistBounds>, AnimationError> {
        if self.end() <= self.start() {
            return Ok(None);
        }
        if let Some((twist, _)) = self.screw {
            return Ok(Some(RootSpatialTwistBounds {
                linear_speed_bound: l1(twist.linear.to_array().map(Scalar::exact))?.1,
                angular_speed_bound: l1(twist.angular.to_array().map(Scalar::exact))?.1,
                rates: RootTwistRateBounds {
                    linear: 0.,
                    angular: 0.,
                },
            }));
        }
        let Some(rotation) = self.rotation.enclosed_angular_derivative_bounds()? else {
            return Ok(None);
        };
        let h = Scalar::exact(self.end()).sub(Scalar::exact(self.start()))?;
        if h.0 <= 0. {
            return Err(AnimationError::RootRigidBudget);
        }
        let a = polynomial_bounds(self.additive, h)?;
        let p = polynomial_bounds(self.pivot, h)?;
        let omega = Scalar::exact(rotation.speed_bound());
        let alpha = Scalar::exact(rotation.acceleration_bound());
        let speed = a[1].add(omega.mul(a[0])?)?.add(p[1])?;
        let rate = a[2]
            .add(alpha.mul(a[0])?)?
            .add(omega.mul(a[1].add(p[1])?)?)?
            .add(p[2])?;
        Ok(Some(RootSpatialTwistBounds {
            linear_speed_bound: speed.1,
            angular_speed_bound: rotation.speed_bound(),
            rates: RootTwistRateBounds {
                linear: rate.1,
                angular: rotation.acceleration_bound(),
            },
        }))
    }
}
