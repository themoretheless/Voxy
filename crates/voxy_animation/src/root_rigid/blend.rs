use super::*;

/// Magnitude and derivative bounds for spatial twists in one fixed frame.
#[derive(Clone, Copy, Debug)]
pub struct RootSpatialTwistBounds {
    pub linear_speed_bound: f64,
    pub angular_speed_bound: f64,
    pub rates: RootTwistRateBounds,
}
impl RootSpatialTwistBounds {
    fn checked(self) -> Result<Self, AnimationError> {
        if [
            self.linear_speed_bound,
            self.angular_speed_bound,
            self.rates.linear,
            self.rates.angular,
        ]
        .into_iter()
        .any(|v| !v.is_finite() || v < 0.)
        {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(self)
    }
    /// Transports fixed-frame spatial speed/rate bounds through a similarity map.
    /// The shifted origin couples angular magnitude/rate into the linear bounds.
    pub fn transformed(
        self,
        basis: DQuat,
        scale: f64,
        offset: DVec3,
    ) -> Result<Self, AnimationError> {
        self.checked()?;
        validate_coordinates(basis, scale, offset)?;
        let radius = offset.length();
        Self {
            linear_speed_bound: scale * self.linear_speed_bound + self.angular_speed_bound * radius,
            angular_speed_bound: self.angular_speed_bound,
            rates: RootTwistRateBounds {
                linear: scale * self.rates.linear + self.rates.angular * radius,
                angular: self.rates.angular,
            },
        }
        .checked()
    }
    /// Constant nonnegative playback rate: speed scales by s, derivatives by s^2.
    pub fn retimed(self, speed: f64) -> Result<Self, AnimationError> {
        self.checked()?;
        if !speed.is_finite() || speed < 0. {
            return Err(AnimationError::InvalidPlaybackSpeed);
        }
        Self {
            linear_speed_bound: self.linear_speed_bound * speed,
            angular_speed_bound: self.angular_speed_bound * speed,
            rates: RootTwistRateBounds {
                linear: (self.rates.linear * speed) * speed,
                angular: (self.rates.angular * speed) * speed,
            },
        }
        .checked()
    }
    /// Bounds a linear-weight blend on a key-free common wall-time interval.
    /// Source/target coordinates and rates must already agree.
    pub fn blend(
        self,
        target: Self,
        from: f64,
        to: f64,
        duration: f64,
    ) -> Result<Self, AnimationError> {
        self.checked()?;
        target.checked()?;
        if [from, to]
            .into_iter()
            .any(|w| !w.is_finite() || !(0. ..=1.).contains(&w))
        {
            return Err(AnimationError::InvalidBlendWeight);
        }
        if !duration.is_finite() || duration <= 0. {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let weight_rate = (to - from).abs() / duration;
        let weighted = |a: f64, b: f64| ((1. - from) * a + from * b).max((1. - to) * a + to * b);
        Self {
            linear_speed_bound: weighted(self.linear_speed_bound, target.linear_speed_bound),
            angular_speed_bound: weighted(self.angular_speed_bound, target.angular_speed_bound),
            rates: RootTwistRateBounds {
                linear: weighted(self.rates.linear, target.rates.linear)
                    + weight_rate * self.linear_speed_bound
                    + weight_rate * target.linear_speed_bound,
                angular: weighted(self.rates.angular, target.rates.angular)
                    + weight_rate * self.angular_speed_bound
                    + weight_rate * target.angular_speed_bound,
            },
        }
        .checked()
    }
}
impl RootRigidTwist {
    /// Fixed-frame spatial similarity: angular'=B*angular,
    /// linear'=scale*B*linear - angular' cross offset.
    pub fn transformed(
        self,
        basis: DQuat,
        scale: f64,
        offset: DVec3,
    ) -> Result<Self, AnimationError> {
        validate_coordinates(basis, scale, offset)?;
        self.increment(0.)?;
        let angular = basis * self.angular;
        let result = Self {
            angular,
            linear: scale * (basis * self.linear) - angular.cross(offset),
        };
        result.increment(0.)?;
        Ok(result)
    }
    pub fn retimed(self, speed: f64) -> Result<Self, AnimationError> {
        if !speed.is_finite() || speed < 0. {
            return Err(AnimationError::InvalidPlaybackSpeed);
        }
        self.increment(0.)?;
        let result = Self {
            linear: self.linear * speed,
            angular: self.angular * speed,
        };
        result.increment(0.)?;
        Ok(result)
    }
    /// Velocity blend in an explicitly shared fixed coordinate frame.
    pub fn blend(self, target: Self, weight: f64) -> Result<Self, AnimationError> {
        if !weight.is_finite() || !(0. ..=1.).contains(&weight) {
            return Err(AnimationError::InvalidBlendWeight);
        }
        self.increment(0.)?;
        target.increment(0.)?;
        let result = Self {
            linear: (1. - weight) * self.linear + weight * target.linear,
            angular: (1. - weight) * self.angular + weight * target.angular,
        };
        result.increment(0.)?;
        Ok(result)
    }
}
impl RootRigidSpan {
    /// Spatial twist speed/rate bounds valid within this span, not across keys.
    pub fn twist_bounds(&self) -> Result<Option<RootSpatialTwistBounds>, AnimationError> {
        let Some(rates) = self.twist_rate_bounds()? else {
            return Ok(None);
        };
        if let Some((twist, _)) = self.screw {
            return Ok(Some(
                RootSpatialTwistBounds {
                    linear_speed_bound: twist.linear.length(),
                    angular_speed_bound: twist.angular.length(),
                    rates,
                }
                .checked()?,
            ));
        }
        let dt = self.end() - self.start();
        let first = |c: [DVec3; 4]| {
            c.windows(2)
                .map(|p| 3. * (p[1] - p[0]).length())
                .fold(0_f64, f64::max)
                / dt
        };
        let angular_speed_bound = self
            .rotation
            .angular_speed_bound()
            .ok_or(AnimationError::RootRotationBudget)?;
        let position = self
            .additive
            .iter()
            .map(|v| v.length())
            .fold(0_f64, f64::max);
        let linear = first(self.additive) + angular_speed_bound * position + first(self.pivot);
        let guard = 4096. * f64::EPSILON * (linear + position / dt);
        Ok(Some(
            RootSpatialTwistBounds {
                linear_speed_bound: linear + guard,
                angular_speed_bound,
                rates,
            }
            .checked()?,
        ))
    }
}

fn validate_coordinates(basis: DQuat, scale: f64, offset: DVec3) -> Result<(), AnimationError> {
    if !basis.is_finite()
        || !basis.is_normalized()
        || !scale.is_finite()
        || scale < 0.
        || !offset.is_finite()
    {
        return Err(AnimationError::InvalidRetargetBinding);
    }
    Ok(())
}
