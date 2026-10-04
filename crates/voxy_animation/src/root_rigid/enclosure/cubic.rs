//! Stored polynomial rotation and simultaneous moving-pivot velocity enclosures.
use super::*;
use crate::RootRotationSpan;
pub(super) fn interpolate(a: Scalar, b: Scalar, u: Scalar) -> Result<Scalar, AnimationError> {
    let mut value = a.mul(Scalar::exact(1.).sub(u)?)?.add(b.mul(u)?)?;
    // A second equivalent expression retains weight/complement correlation.
    // An overflowing alternate expression supplies no bound; the first remains
    // valid and is not discarded merely because an optional tightening failed.
    if let Ok(alternate) = b
        .sub(a)
        .and_then(|delta| delta.mul(u))
        .and_then(|delta| a.add(delta))
    {
        value.0 = value.0.max(alternate.0);
        value.1 = value.1.min(alternate.1);
    }
    // Both interval evaluation and the convex hull enclose (1-u)*a+u*b.
    // Intersect them only with the explicit nonnegative-weight domain proof.
    if u.0 >= 0. && u.1 <= 1. {
        value.0 = value.0.max(a.0.min(b.0));
        value.1 = value.1.min(a.1.max(b.1));
    }
    Ok(value)
}
fn bezier<const N: usize>(
    mut control: [[Scalar; N]; 4],
    degree: usize,
    u: Scalar,
) -> Result<[Scalar; N], AnimationError> {
    for width in (1..=degree).rev() {
        for i in 0..width {
            for j in 0..N {
                control[i][j] = interpolate(control[i][j], control[i + 1][j], u)?;
            }
        }
    }
    Ok(control[0])
}
fn polynomial<const N: usize>(
    control: [[Scalar; N]; 4],
    u: Scalar,
    duration: Scalar,
) -> Result<([Scalar; N], [Scalar; N]), AnimationError> {
    let value = bezier(control, 3, u)?;
    let mut derivative = control;
    for i in 0..3 {
        for j in 0..N {
            derivative[i][j] = control[i + 1][j]
                .sub(control[i][j])?
                .mul(Scalar::exact(3.))?
                .div_interval_positive(duration)?;
        }
    }
    Ok((value, bezier(derivative, 2, u)?))
}
impl RootRotationSpan {
    fn cubic_motion(
        &self,
        fraction: Scalar,
    ) -> Result<Option<(RootRigidEnclosure, [Scalar; 3])>, AnimationError> {
        let Some((control, left, right)) = self.cubic_velocity_inputs() else {
            return Ok(None);
        };
        if self.end() <= self.start() {
            return Ok(None);
        }
        let duration = Scalar::exact(self.end()).sub(Scalar::exact(self.start()))?;
        if duration.0 <= 0. {
            return Err(AnimationError::RootRotationBudget);
        }
        let (value, derivative) =
            polynomial(control.map(|v| v.map(Scalar::exact)), fraction, duration)?;
        let norm = value[0]
            .square()?
            .add(value[1].square()?)?
            .add(value[2].square()?)?
            .add(value[3].square()?)?;
        if norm.0 <= 0. {
            return Err(AnimationError::RootRotationBudget);
        }
        let c = cross(
            [derivative[0], derivative[1], derivative[2]],
            [value[0], value[1], value[2]],
        )?;
        let mut angular = [Scalar::exact(0.); 3];
        for i in 0..3 {
            angular[i] = derivative[i]
                .mul(value[3])?
                .sub(derivative[3].mul(value[i])?)?
                .sub(c[i])?
                .mul(Scalar::exact(2.))?
                .div_interval_positive(norm)?;
        }
        let left = RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: DVec3::ZERO,
            rotation: left,
        })?;
        let right = RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: DVec3::ZERO,
            rotation: right,
        })?;
        let (_, left_rotation) = left.vectors();
        let length = norm.sqrt_positive()?;
        let normalized = RootRigidEnclosure {
            translation: [[0., 0.]; 3],
            rotation: [
                value[0].div_interval_positive(length)?.array(),
                value[1].div_interval_positive(length)?.array(),
                value[2].div_interval_positive(length)?.array(),
                value[3].div_interval_positive(length)?.array(),
            ],
        };
        Ok(Some((
            left.compose(&normalized)?.compose(&right)?,
            rotate(left_rotation, angular)?,
        )))
    }
    fn rotation_motion_enclosure(
        &self,
        fraction: Scalar,
    ) -> Result<Option<(RootRigidEnclosure, [Scalar; 3])>, AnimationError> {
        if self.cubic_velocity_inputs().is_some() {
            return self.cubic_motion(fraction);
        }
        let Some((from, axis, left, right)) = self.arc_velocity_inputs() else {
            return Ok(None);
        };
        if self.end() <= self.start() {
            return Ok(None);
        }
        let duration = Scalar::exact(self.end()).sub(Scalar::exact(self.start()))?;
        if duration.0 <= 0. {
            return Err(AnimationError::RootRotationBudget);
        }
        let make = |rotation| {
            RootRigidEnclosure::from_transform(RootRigidTransform {
                translation: DVec3::ZERO,
                rotation,
            })
        };
        let initial = make(left)?.compose(&make(from)?)?;
        let (_, q) = initial.vectors();
        let angular = rotate(
            q,
            [
                Scalar::exact(axis.x).div_interval_positive(duration)?,
                Scalar::exact(axis.y).div_interval_positive(duration)?,
                Scalar::exact(axis.z).div_interval_positive(duration)?,
            ],
        )?;
        let mut exponential = RootRigidEnclosure::IDENTITY;
        if fraction.1 > 0. && axis != DVec3::ZERO {
            let radius = Scalar::exact(axis.x.abs())
                .add(Scalar::exact(axis.y.abs()))?
                .add(Scalar::exact(axis.z.abs()))?
                .mul(fraction)?
                .mul(Scalar::exact(2.))?;
            let count = radius.1.ceil().max(1.);
            if count > MAX_ROOT_ROTATION_SPANS as f64 {
                return Err(AnimationError::RootRotationBudget);
            }
            let step = RootRigidTwist {
                linear: DVec3::ZERO,
                angular: axis,
            }
            .increment_interval_enclosure(fraction.div_positive(count)?)?;
            for _ in 0..count as usize {
                exponential = step.compose(&exponential)?;
            }
        }
        Ok(Some((
            initial.compose(&exponential)?.compose(&make(right)?)?,
            angular,
        )))
    }
    /// Outward stored-field angular speed for HOLD, LINEAR or normalized CUBIC.
    /// STEP and zero-duration events have no finite derivative and return None.
    pub fn angular_velocity_bounds(
        &self,
        fraction: f64,
    ) -> Result<Option<[[f64; 2]; 3]>, AnimationError> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(AnimationError::InvalidSampleTime);
        }
        Ok(self
            .rotation_motion_enclosure(Scalar::exact(fraction))?
            .map(|(_, angular)| angular.map(Scalar::array)))
    }
    /// Encloses the rational angular derivative of normalized stored cubic data.
    /// Unsupported shapes return None. Compilation errors before the stored
    /// coefficients are separate from this arithmetic enclosure.
    pub fn cubic_angular_velocity_bounds(
        &self,
        fraction: f64,
    ) -> Result<Option<[[f64; 2]; 3]>, AnimationError> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(AnimationError::InvalidSampleTime);
        }
        Ok(self
            .cubic_motion(Scalar::exact(fraction))?
            .map(|(_, angular)| angular.map(Scalar::array)))
    }
}
impl RootRigidSpan {
    /// Encloses the continuous stored curve pose, including the moving pivot.
    /// This is the real normalized-control reference, not a bound derived from
    /// the rounded result of `sample`. STEP events have no continuous reference.
    pub fn continuous_pose_enclosure(
        &self,
        fraction: f64,
    ) -> Result<Option<RootRigidEnclosure>, AnimationError> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(AnimationError::InvalidSampleTime);
        }
        if let Some((twist, initial)) = self.screw {
            let duration = Scalar::exact(self.end())
                .sub(Scalar::exact(self.start()))?
                .mul(Scalar::exact(fraction))?;
            return Ok(Some(
                twist
                    .increment_interval_enclosure(duration)?
                    .compose(&RootRigidEnclosure::from_transform(initial)?)?,
            ));
        }
        let Some((rotation, _)) = self
            .rotation
            .rotation_motion_enclosure(Scalar::exact(fraction))?
        else {
            return Ok(None);
        };
        let additive = bezier(
            self.additive.map(|v| v.to_array().map(Scalar::exact)),
            3,
            Scalar::exact(fraction),
        )?;
        let pivot = bezier(
            self.pivot.map(|v| v.to_array().map(Scalar::exact)),
            3,
            Scalar::exact(fraction),
        )?;
        let (_, q) = rotation.vectors();
        let rotated = rotate(q, pivot)?;
        let mut translation = [[0.; 2]; 3];
        for i in 0..3 {
            translation[i] = additive[i].sub(rotated[i])?.array();
        }
        Ok(Some(RootRigidEnclosure {
            translation,
            rotation: rotation.rotation,
        }))
    }

    /// Stored-field spatial velocity enclosure for screw, HOLD, LINEAR or CUBIC spans.
    /// Includes moving-pivot coupling v=a_dot-omega cross a-R*p_dot.
    /// Unsupported interpolation and instantaneous events return None.
    pub fn spatial_twist_enclosure(
        &self,
        fraction: f64,
    ) -> Result<Option<RootRigidTwistEnclosure>, AnimationError> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(AnimationError::InvalidSampleTime);
        }
        self.spatial_twist_enclosure_range([fraction, fraction])
    }
    /// Converts a closed stored-time interval outward, including subtraction and
    /// division error. Times must stay in this continuous span; events are split
    /// by the caller. Intersecting with [0,1] uses that checked domain proof.
    pub fn spatial_twist_enclosure_at_times(
        &self,
        times: [f64; 2],
    ) -> Result<Option<RootRigidTwistEnclosure>, AnimationError> {
        if times.into_iter().any(|v| !v.is_finite())
            || times[1] < times[0]
            || times[0] < self.start()
            || times[1] > self.end()
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        if self.end() <= self.start() {
            return Ok(None);
        }
        let duration = Scalar::exact(self.end()).sub(Scalar::exact(self.start()))?;
        let fraction = Scalar(times[0], times[1])
            .sub(Scalar::exact(self.start()))?
            .div_interval_positive(duration)?;
        self.spatial_twist_enclosure_range([fraction.0.max(0.), fraction.1.min(1.)])
    }
    /// Encloses the complete clip-time field after a linear mapping onto wall
    /// time. Both stored endpoint differences and their ratio round outward.
    /// Source clip times must remain inside this span; STEP events stay separate.
    pub fn retimed_spatial_twist_enclosure_between(
        &self,
        clip_times: [f64; 2],
        wall_times: [f64; 2],
    ) -> Result<Option<RootRigidTwistEnclosure>, AnimationError> {
        let factor = super::twist::retiming_factor_between_times(clip_times, wall_times)?;
        self.spatial_twist_enclosure_at_times(clip_times)?
            .map(|field| field.scaled(factor))
            .transpose()
    }
    /// Uniform coordinate displacement error against a frozen spatial field.
    /// Both fields must have angular velocity parallel to the chosen axis over
    /// the complete stored-time interval. Then x_dot[axis] = v[axis] for every
    /// material point, and integrating the velocity discrepancy bounds every
    /// prefix of this interval. Initial coordinate discrepancy is separate.
    /// No claim is made about the other coordinates or floating pose evaluation.
    pub fn enclosed_coordinate_displacement_error_between(
        &self,
        times: [f64; 2],
        axis: usize,
        reference: RootRigidTwist,
    ) -> Result<Option<f64>, AnimationError> {
        reference.enclosure()?;
        let Some(field) = self.spatial_twist_enclosure_at_times(times)? else {
            return Ok(None);
        };
        field.coordinate_displacement_error_between(times, axis, reference)
    }
    /// Encloses every field value on a closed progress interval. This retains
    /// uncertainty from clock conversion rather than sampling rounded progress.
    /// An interval whose cubic quaternion norm cannot be proved positive rejects.
    pub fn spatial_twist_enclosure_range(
        &self,
        fractions: [f64; 2],
    ) -> Result<Option<RootRigidTwistEnclosure>, AnimationError> {
        if fractions
            .into_iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(&v))
            || fractions[1] < fractions[0]
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let fraction = Scalar(fractions[0], fractions[1]);
        if self.end() <= self.start() {
            return Ok(None);
        }
        if let Some((twist, _)) = self.screw {
            return Ok(Some(twist.enclosure()?));
        }
        let Some((rotation, angular)) = self.rotation.rotation_motion_enclosure(fraction)? else {
            return Ok(None);
        };
        let duration = Scalar::exact(self.end()).sub(Scalar::exact(self.start()))?;
        let (additive, derivative) = polynomial(
            self.additive.map(|v| v.to_array().map(Scalar::exact)),
            fraction,
            duration,
        )?;
        let (_, pivot_derivative) = polynomial(
            self.pivot.map(|v| v.to_array().map(Scalar::exact)),
            fraction,
            duration,
        )?;
        let (_, q) = rotation.vectors();
        let moved = rotate(q, pivot_derivative)?;
        let coupling = cross(angular, additive)?;
        Ok(Some(RootRigidTwistEnclosure::from_parts(
            [
                derivative[0].sub(coupling[0])?.sub(moved[0])?,
                derivative[1].sub(coupling[1])?.sub(moved[1])?,
                derivative[2].sub(coupling[2])?.sub(moved[2])?,
            ],
            angular,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interpolation_retains_convex_hull_with_uncertain_weights() {
        let value = interpolate(Scalar(2., 3.), Scalar(4., 5.), Scalar(0., 1.)).unwrap();
        assert_eq!(value.array(), [2., 5.]);
        let extrapolated =
            interpolate(Scalar::exact(2.), Scalar::exact(4.), Scalar::exact(2.)).unwrap();
        assert!(extrapolated.0 <= 6. && extrapolated.1 >= 6.);
        assert!(extrapolated.0 > 4.);
    }
}
