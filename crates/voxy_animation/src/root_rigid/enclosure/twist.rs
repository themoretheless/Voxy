//! Numerical enclosures for fixed-frame spatial velocity arithmetic.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct RootRigidTwistEnclosure {
    linear: [Scalar; 3],
    angular: [Scalar; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct RootTwistErrorBounds {
    linear: f64,
    angular: f64,
}
impl RootTwistErrorBounds {
    pub const ZERO: Self = Self {
        linear: 0.,
        angular: 0.,
    };
    pub fn linear_bound(self) -> f64 {
        self.linear
    }
    pub fn angular_bound(self) -> f64 {
        self.angular
    }
}
impl RootRigidTwist {
    /// Exact stored spatial velocity; subsequent operations round outward.
    pub fn enclosure(self) -> Result<RootRigidTwistEnclosure, AnimationError> {
        if !self.linear.is_finite() || !self.angular.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(RootRigidTwistEnclosure {
            linear: self.linear.to_array().map(Scalar::exact),
            angular: self.angular.to_array().map(Scalar::exact),
        })
    }
}
pub(super) fn retiming_factor_between_times(
    clip: [f64; 2],
    wall: [f64; 2],
) -> Result<Scalar, AnimationError> {
    if clip.into_iter().chain(wall).any(|v| !v.is_finite())
        || clip[1] < clip[0]
        || wall[1] <= wall[0]
    {
        return Err(AnimationError::InvalidPlaybackSpeed);
    }
    let mut numerator = Scalar::exact(clip[1]).sub(Scalar::exact(clip[0]))?;
    numerator.0 = numerator.0.max(0.);
    let denominator = Scalar::exact(wall[1]).sub(Scalar::exact(wall[0]))?;
    numerator.div_interval_positive(denominator)
}
impl RootRigidTwistEnclosure {
    pub(super) fn nominal_midpoint(&self) -> RootRigidTwist {
        let midpoint = |values: [Scalar; 3]| {
            DVec3::from_array(values.map(|value| value.0 * 0.5 + value.1 * 0.5))
        };
        RootRigidTwist {
            linear: midpoint(self.linear),
            angular: midpoint(self.angular),
        }
    }
    pub(super) fn from_parts(linear: [Scalar; 3], angular: [Scalar; 3]) -> Self {
        Self { linear, angular }
    }
    /// Componentwise hull containing both represented velocity domains.
    /// This does not establish a derivative bound across their boundary.
    pub fn hull(&self, other: &Self) -> Self {
        let hull = |a: Scalar, b: Scalar| Scalar(a.0.min(b.0), a.1.max(b.1));
        Self {
            linear: std::array::from_fn(|i| hull(self.linear[i], other.linear[i])),
            angular: std::array::from_fn(|i| hull(self.angular[i], other.angular[i])),
        }
    }
    pub fn linear_bounds(&self) -> [[f64; 2]; 3] {
        self.linear.map(Scalar::array)
    }
    pub fn angular_bounds(&self) -> [[f64; 2]; 3] {
        self.angular.map(Scalar::array)
    }
    /// Coordinate velocity bound when the angular field is parallel to that axis.
    /// Covers only the time domain represented by this enclosure; a point sample
    /// alone does not establish a constraint between samples.
    pub fn coordinate_velocity_range(&self, axis: usize) -> Option<[f64; 2]> {
        if axis >= 3 || (0..3).any(|i| i != axis && !self.angular[i].is_zero()) {
            return None;
        }
        Some(self.linear[axis].array())
    }
    pub(super) fn coordinate_displacement_error_between(
        &self,
        times: [f64; 2],
        axis: usize,
        reference: RootRigidTwist,
    ) -> Result<Option<f64>, AnimationError> {
        let nominal = reference.enclosure()?;
        let (Some(actual), Some(frozen)) = (
            self.coordinate_velocity_range(axis),
            nominal.coordinate_velocity_range(axis),
        ) else {
            return Ok(None);
        };
        let delta = Scalar(actual[0], actual[1]).sub(Scalar(frozen[0], frozen[1]))?;
        let speed_error = Scalar::exact(delta.0.abs().max(delta.1.abs()));
        let mut duration = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
        duration.0 = duration.0.max(0.);
        Ok(Some(speed_error.mul(duration)?.1))
    }
    /// Outward L1 bounds also bound Euclidean velocity discrepancy from reference.
    pub fn error_bounds(
        &self,
        reference: RootRigidTwist,
    ) -> Result<RootTwistErrorBounds, AnimationError> {
        reference.enclosure()?;
        let distance = |bounds: [Scalar; 3], point: DVec3| -> Result<f64, AnimationError> {
            let mut sum = Scalar::exact(0.);
            for i in 0..3 {
                if bounds[i].0 == point[i] && bounds[i].1 == point[i] {
                    continue;
                }
                let delta = bounds[i].sub(Scalar::exact(point[i]))?;
                let error = delta.0.abs().max(delta.1.abs());
                sum = sum.add(Scalar::exact(error))?;
            }
            Ok(sum.1)
        };
        Ok(RootTwistErrorBounds {
            linear: distance(self.linear, reference.linear)?,
            angular: distance(self.angular, reference.angular)?,
        })
    }
    pub(super) fn scaled(&self, factor: Scalar) -> Result<Self, AnimationError> {
        let mut result = *self;
        for i in 0..3 {
            result.linear[i] = self.linear[i].mul(factor)?;
            result.angular[i] = self.angular[i].mul(factor)?;
        }
        Ok(result)
    }
    /// Includes division error for the exact stored clip/wall duration ratio.
    /// Derivative bounds and any error in obtaining those clocks are separate.
    pub fn retimed_between(
        &self,
        clip_seconds: f64,
        wall_seconds: f64,
    ) -> Result<Self, AnimationError> {
        if !clip_seconds.is_finite()
            || clip_seconds < 0.
            || !wall_seconds.is_finite()
            || wall_seconds <= 0.
        {
            return Err(AnimationError::InvalidPlaybackSpeed);
        }
        self.scaled(Scalar::exact(clip_seconds).div_interval_positive(Scalar::exact(wall_seconds))?)
    }
    /// Retimes by the exact differences of stored clip and wall endpoints.
    /// Includes outward subtraction and division; reverse clip motion must be
    /// represented by a reversed field rather than a decreasing interval.
    pub fn retimed_between_times(
        &self,
        clip_times: [f64; 2],
        wall_times: [f64; 2],
    ) -> Result<Self, AnimationError> {
        self.scaled(retiming_factor_between_times(clip_times, wall_times)?)
    }
    /// Linear-weight blend, including interpolation and (1-weight) subtraction.
    /// Stored weights/progress are exact reference inputs; frames must agree.
    pub fn blended(
        &self,
        target: &Self,
        weights: [f64; 2],
        progress: f64,
    ) -> Result<Self, AnimationError> {
        self.blended_over_progress(target, weights, [progress, progress])
    }
    /// Encloses every linearly weighted blend in a closed progress interval.
    /// Source and target must enclose their complete corresponding time domains,
    /// expressed in the same spatial frame and playback clock. Progress bounds
    /// include any upstream clock conversion error; this method does not derive it.
    pub fn blended_over_progress(
        &self,
        target: &Self,
        weights: [f64; 2],
        progress: [f64; 2],
    ) -> Result<Self, AnimationError> {
        if weights
            .into_iter()
            .chain(progress)
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(&v))
            || progress[0] > progress[1]
        {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let weight = super::cubic::interpolate(
            Scalar::exact(weights[0]),
            Scalar::exact(weights[1]),
            Scalar(progress[0], progress[1]),
        )?;
        let mut result = *self;
        for i in 0..3 {
            result.linear[i] = super::cubic::interpolate(self.linear[i], target.linear[i], weight)?;
            result.angular[i] =
                super::cubic::interpolate(self.angular[i], target.angular[i], weight)?;
        }
        Ok(result)
    }
    /// Encloses a fade over stored wall-clock endpoints. Weights correspond to
    /// the fade's two endpoints. The query must lie inside the continuous fade;
    /// callers split completion/tail events before invoking this method.
    /// Both fields must already cover the query in the same frame and wall clock.
    pub fn blended_between_times(
        &self,
        target: &Self,
        weights: [f64; 2],
        fade_times: [f64; 2],
        query_times: [f64; 2],
    ) -> Result<Self, AnimationError> {
        if fade_times
            .into_iter()
            .chain(query_times)
            .any(|v| !v.is_finite())
            || fade_times[1] <= fade_times[0]
            || query_times[1] < query_times[0]
            || query_times[0] < fade_times[0]
            || query_times[1] > fade_times[1]
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let duration = Scalar::exact(fade_times[1]).sub(Scalar::exact(fade_times[0]))?;
        let progress = Scalar(query_times[0], query_times[1])
            .sub(Scalar::exact(fade_times[0]))?
            .div_interval_positive(duration)?;
        // The checked query domain proves true progress remains in [0,1].
        self.blended_over_progress(target, weights, [progress.0.max(0.), progress.1.min(1.)])
    }
    /// Similarity for a spatial field: omega'=R*omega,
    /// v'=scale*R*v - omega' cross frame.translation.
    pub fn transformed(
        &self,
        frame: &RootRigidEnclosure,
        scale: f64,
    ) -> Result<Self, AnimationError> {
        if !scale.is_finite() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        self.transformed_enclosed_scale(frame, RootUniformScaleEnclosure::from_scale(scale)?)
    }
    pub fn transformed_enclosed_scale(
        &self,
        frame: &RootRigidEnclosure,
        scale: RootUniformScaleEnclosure,
    ) -> Result<Self, AnimationError> {
        let (offset, rotation) = frame.vectors();
        let angular = rotate(rotation, self.angular)?;
        let linear = rotate(rotation, self.linear)?;
        let coupling = cross(angular, offset)?;
        Ok(Self {
            angular,
            linear: [
                linear[0].mul(scale.value)?.sub(coupling[0])?,
                linear[1].mul(scale.value)?.sub(coupling[1])?,
                linear[2].mul(scale.value)?.sub(coupling[2])?,
            ],
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planar_constraints_survive_retiming_and_blending() {
        let a = RootRigidTwist {
            linear: DVec3::X,
            angular: DVec3::Y,
        }
        .enclosure()
        .unwrap();
        let b = RootRigidTwist {
            linear: DVec3::Z,
            angular: -DVec3::Y,
        }
        .enclosure()
        .unwrap();
        let a = a.retimed_between(0.3, 0.7).unwrap();
        let b = b.retimed_between(0.2, 0.9).unwrap();
        for progress in [0., 0.37, 1.] {
            let mixed = a.blended(&b, [0.1, 0.9], progress).unwrap();
            assert_eq!(mixed.coordinate_velocity_range(1), Some([0., 0.]));
            assert_eq!(mixed.coordinate_velocity_range(3), None);
        }
        let tilted = RootRigidTwist {
            linear: DVec3::ZERO,
            angular: DVec3::new(1e-300, 1., 0.),
        }
        .enclosure()
        .unwrap();
        assert_eq!(tilted.coordinate_velocity_range(1), None);
    }
    #[test]
    fn twist_enclosures_cover_retiming_weight_interpolation_and_shifted_frame() {
        let a = RootRigidTwist {
            linear: DVec3::new(0.3, 0.1, -0.2),
            angular: DVec3::new(0.7, 0.1, -0.2),
        };
        let b = RootRigidTwist {
            linear: DVec3::new(-0.2, 0.4, 0.1),
            angular: DVec3::new(0.1, 0.8, -0.3),
        };
        let source = a.enclosure().unwrap().retimed_between(0.3, 0.2).unwrap();
        let target = b.enclosure().unwrap().retimed_between(0.7, 0.4).unwrap();
        let mixed = source.blended(&target, [0.1, 0.9], 0.37).unwrap();
        let transform = RootRigidTransform {
            translation: DVec3::new(0.4, -0.2, 0.3),
            rotation: DQuat::from_rotation_y(0.7),
        };
        let frame = RootRigidEnclosure::from_transform(transform).unwrap();
        for scale in [-2., 0., 0.5, 2.] {
            let e = mixed.transformed(&frame, scale).unwrap();
            assert!(
                e.linear_bounds()
                    .into_iter()
                    .chain(e.angular_bounds())
                    .all(|v| v[1] - v[0] < 1e-10)
            );
            println!(
                "TWIST_FRAME_ENCLOSURE {:?}",
                (
                    (a.linear.to_array(), a.angular.to_array(), 0.3, 0.2),
                    (b.linear.to_array(), b.angular.to_array(), 0.7, 0.4),
                    [0.1, 0.9],
                    0.37,
                    (
                        transform.translation.to_array(),
                        transform.rotation.to_array()
                    ),
                    scale,
                    e.linear_bounds(),
                    e.angular_bounds()
                )
            );
        }
        assert!(source.retimed_between(-0.1, 1.).is_err());
        assert!(source.retimed_between(0.1, 0.).is_err());
        assert!(source.blended(&target, [0.1, 0.9], f64::NAN).is_err());
        assert!(source.transformed(&frame, f64::NAN).is_err());
        assert!(
            RootRigidTwist {
                linear: DVec3::NAN,
                angular: DVec3::ZERO
            }
            .enclosure()
            .is_err()
        );
    }
}

#[cfg(test)]
mod range_tests {
    use super::*;
    #[test]
    fn whole_blend_range_encloses_reversing_weights_and_preserves_planarity() {
        let a = RootRigidTwist {
            linear: DVec3::new(2., 0., -3.),
            angular: DVec3::Y * 4.,
        }
        .enclosure()
        .unwrap();
        let b = RootRigidTwist {
            linear: DVec3::new(-5., 0., 7.),
            angular: -DVec3::Y * 2.,
        }
        .enclosure()
        .unwrap();
        for weights in [[0., 1.], [1., 0.], [0.3, 0.7], [0.5, 0.5]] {
            let whole = a.blended_over_progress(&b, weights, [0.1, 0.9]).unwrap();
            assert_eq!(whole.coordinate_velocity_range(1), Some([0., 0.]));
            for progress in [0.1, 0.3, 0.5, 0.9] {
                let weight = weights[0] * (1. - progress) + weights[1] * progress;
                let source = [2., 0., -3., 0., 4., 0.];
                let target = [-5., 0., 7., 0., -2., 0.];
                for (i, bounds) in whole
                    .linear_bounds()
                    .into_iter()
                    .chain(whole.angular_bounds())
                    .enumerate()
                {
                    let value = source[i] * (1. - weight) + target[i] * weight;
                    assert!(bounds[0] <= value && value <= bounds[1]);
                }
            }
        }
        for progress in [[0.8, 0.2], [-0.1, 0.5], [0., f64::NAN]] {
            assert!(a.blended_over_progress(&b, [0., 1.], progress).is_err());
        }
        let tilted = RootRigidTwist {
            linear: DVec3::ZERO,
            angular: DVec3::new(1e-300, 1., 0.),
        }
        .enclosure()
        .unwrap();
        assert!(
            a.blended_over_progress(&tilted, [0., 1.], [0., 1.])
                .unwrap()
                .coordinate_velocity_range(1)
                .is_none()
        );
    }
}

#[cfg(test)]
mod clock_tests {
    use super::*;
    #[test]
    fn clock_retiming_covers_long_elapsed_tiny_ticks_and_paused_clip() {
        let twist = RootRigidTwist {
            linear: DVec3::new(0., 2., 0.),
            angular: DVec3::Y * 3.,
        }
        .enclosure()
        .unwrap();
        let clip = [1e12, 1e12_f64.next_up()];
        let wall = [1e14, 1e14_f64.next_up()];
        let value = twist.retimed_between_times(clip, wall).unwrap();
        let exact_ratio = (clip[1] - clip[0]) / (wall[1] - wall[0]);
        let range = value.coordinate_velocity_range(1).unwrap();
        assert!(range[0] <= 2. * exact_ratio && range[1] >= 2. * exact_ratio);
        let paused = twist.retimed_between_times([0.3, 0.3], [0.1, 0.4]).unwrap();
        assert!(
            paused
                .linear_bounds()
                .into_iter()
                .chain(paused.angular_bounds())
                .all(|b| b == [0., 0.])
        );
        for (clip, wall) in [
            ([1., 0.], [0., 1.]),
            ([0., 1.], [1., 1.]),
            ([0., f64::NAN], [0., 1.]),
        ] {
            assert!(twist.retimed_between_times(clip, wall).is_err());
        }
        assert!(
            twist
                .retimed_between_times([0., 1.], [0., f64::from_bits(1)])
                .is_err()
        );
    }
}

#[cfg(test)]
mod fade_clock_tests {
    use super::*;
    #[test]
    fn fade_clocks_cover_long_elapsed_ticks_and_reject_unsplit_tails() {
        let source = RootRigidTwist {
            linear: DVec3::X * 2.,
            angular: DVec3::Y,
        }
        .enclosure()
        .unwrap();
        let target = RootRigidTwist {
            linear: DVec3::X * 6.,
            angular: -DVec3::Y,
        }
        .enclosure()
        .unwrap();
        let fade = [1e12, 1e12 + 1.];
        let query = [1e12 + 0.25, 1e12 + 0.75];
        for weights in [[0., 1.], [1., 0.]] {
            let blend = source
                .blended_between_times(&target, weights, fade, query)
                .unwrap();
            let bounds = blend.linear_bounds()[0];
            assert!(bounds[0] <= 3. && bounds[1] >= 5.);
            assert!(bounds[0] > 2.9 && bounds[1] < 5.1);
            assert_eq!(blend.coordinate_velocity_range(1), Some([0., 0.]));
        }
        let adjacent = [1e12, 1e12_f64.next_up()];
        let blend = source
            .blended_between_times(&target, [0., 1.], adjacent, adjacent)
            .unwrap();
        assert!(blend.linear_bounds()[0][0] <= 2. && blend.linear_bounds()[0][1] >= 6.);
        for (fade, query) in [
            ([0., 1.], [0., 1.1]),
            ([0., 0.], [0., 0.]),
            ([0., 1.], [0.8, 0.2]),
            ([0., 1.], [0., f64::NAN]),
        ] {
            assert!(
                source
                    .blended_between_times(&target, [0., 1.], fade, query)
                    .is_err()
            );
        }
    }
}
