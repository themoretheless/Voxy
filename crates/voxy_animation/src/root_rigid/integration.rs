use super::*;

/// Caller-proved upper bounds on spatial twist derivatives in one fixed frame.
/// Bounds must cover every instant; STEP events must be split separately.
#[derive(Clone, Copy, Debug)]
pub struct RootTwistRateBounds {
    pub linear: f64,
    pub angular: f64,
}
/// Approximation plus real-arithmetic error bounds, conditional on rate bounds.
/// Floating-point roundoff is not certified. Physics must account for envelopes
/// before accepting this path as a representation of the original velocity field.
#[derive(Clone, Debug)]
pub struct RootRigidApproximation {
    pub path: RootRigidPath,
    pub origin_error_bound: f64,
    pub angular_error_bound: f64,
}
impl RootRigidApproximation {
    fn checked_errors(&self) -> Result<(), AnimationError> {
        if [self.origin_error_bound, self.angular_error_bound]
            .into_iter()
            .any(|value| !value.is_finite() || value < 0.)
        {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(())
    }
    fn rotation_chord(&self) -> f64 {
        2. * (0.5 * self.angular_error_bound.min(std::f64::consts::PI)).sin()
    }
    /// Appends a same-fixed-frame approximation and propagates both envelopes.
    /// Numerical evaluation error remains a separate proof obligation.
    pub fn append_spatial(&self, next: &Self, max_spans: usize) -> Result<Self, AnimationError> {
        self.checked_errors()?;
        next.checked_errors()?;
        let path = self.path.append_spatial(&next.path, max_spans)?;
        let origin_error_bound = self.origin_error_bound
            + next.origin_error_bound
            + next.rotation_chord() * self.path.end_transform().translation.length();
        let angular_error_bound = self.angular_error_bound + next.angular_error_bound;
        let result = Self {
            path,
            origin_error_bound,
            angular_error_bound,
        };
        result.checked_errors()?;
        Ok(result)
    }
    /// Conditional discretization error at a fixed point in path coordinates.
    /// # Errors
    /// Rejects invalid errors/points or overflowing bounds.
    pub fn point_error_bound(&self, point: DVec3) -> Result<f64, AnimationError> {
        self.checked_errors()?;
        if !point.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let bound = self.origin_error_bound + self.rotation_chord() * point.length();
        if !bound.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(bound)
    }
    /// Transports the approximate path and its conditional errors together.
    /// Angular discrepancy moves a shifted origin, including at scale zero.
    /// # Errors
    /// Rejects invalid coordinates/error metadata or overflowing results.
    pub fn transformed(
        &self,
        basis: DQuat,
        scale: f64,
        offset: DVec3,
    ) -> Result<Self, AnimationError> {
        self.checked_errors()?;
        let path = self.path.transformed(basis, scale, offset)?;
        let origin_error_bound =
            scale * self.origin_error_bound + self.rotation_chord() * offset.length();
        if !origin_error_bound.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(Self {
            path,
            origin_error_bound,
            angular_error_bound: self.angular_error_bound,
        })
    }
}
impl RootRigidPath {
    /// Integrates a continuous spatial velocity field using ordered screw spans.
    /// Refines uniformly until both error targets hold or span capacity is reached.
    /// The callback samples one fixed coordinate frame and must be deterministic.
    /// Derivative bounds are a caller proof obligation, not inferred from samples.
    /// # Errors
    /// Invalid bounds/tolerances, callback failures, overflow or exhausted capacity.
    pub fn integrate_spatial(
        duration: f64,
        rates: RootTwistRateBounds,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
        mut sample: impl FnMut(f64) -> Result<RootRigidTwist, AnimationError>,
    ) -> Result<RootRigidApproximation, AnimationError> {
        if !duration.is_finite()
            || duration < 0.
            || [
                rates.linear,
                rates.angular,
                origin_tolerance,
                angular_tolerance,
            ]
            .into_iter()
            .any(|value| !value.is_finite() || value < 0.)
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let capacity = max_spans.min(MAX_ROOT_ROTATION_SPANS);
        if duration == 0. {
            return Ok(RootRigidApproximation {
                path: Self::from_twists(&[], 0)?,
                origin_error_bound: 0.,
                angular_error_bound: 0.,
            });
        }
        if capacity == 0 {
            return Err(AnimationError::RootRigidBudget);
        }
        let mut count = 1;
        loop {
            let mut segments = Vec::with_capacity(count);
            let mut prefix = RootRigidTransform::IDENTITY;
            let mut origin_error = 0.;
            let mut angular_error = 0.;
            let mut start = 0.;
            for i in 0..count {
                let end = duration * ((i + 1) as f64 / count as f64);
                let dt = end - start;
                if !dt.is_finite() || dt <= 0. {
                    return Err(AnimationError::NumericalOverflow);
                }
                let twist = sample(start)?;
                let increment = twist.increment(dt)?;
                // Skew angular dynamics preserve prior error norm. Variation
                // over this span gives integral s*(Lv + Lw*|x_approx(s)|),
                // with |x_approx(s)| <= |prefix.translation| + |v0|*s.
                origin_error +=
                    0.5 * dt * dt * (rates.linear + rates.angular * prefix.translation.length())
                        + rates.angular * twist.linear.length() * dt * dt * dt / 3.;
                angular_error += 0.5 * rates.angular * dt * dt;
                if !origin_error.is_finite() || !angular_error.is_finite() {
                    return Err(AnimationError::NumericalOverflow);
                }
                prefix = increment.compose(prefix)?;
                segments.push((twist, dt));
                start = end;
            }
            if origin_error <= origin_tolerance && angular_error <= angular_tolerance {
                return Ok(RootRigidApproximation {
                    path: Self::from_twists(&segments, capacity)?,
                    origin_error_bound: origin_error,
                    angular_error_bound: angular_error,
                });
            }
            if count == capacity {
                return Err(AnimationError::RootRigidBudget);
            }
            count = count.saturating_mul(2).min(capacity);
        }
    }
}
