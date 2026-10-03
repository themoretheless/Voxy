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
impl RootRigidTwistEnclosure {
    pub(super) fn from_parts(linear: [Scalar; 3], angular: [Scalar; 3]) -> Self {
        Self { linear, angular }
    }
    pub fn linear_bounds(&self) -> [[f64; 2]; 3] {
        self.linear.map(Scalar::array)
    }
    pub fn angular_bounds(&self) -> [[f64; 2]; 3] {
        self.angular.map(Scalar::array)
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
    fn scaled(&self, factor: Scalar) -> Result<Self, AnimationError> {
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
    /// Linear-weight blend, including interpolation and (1-weight) subtraction.
    /// Stored weights/progress are exact reference inputs; frames must agree.
    pub fn blended(
        &self,
        target: &Self,
        weights: [f64; 2],
        progress: f64,
    ) -> Result<Self, AnimationError> {
        if weights
            .into_iter()
            .chain([progress])
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(&v))
        {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let weight = Scalar::exact(weights[0]).add(
            Scalar::exact(weights[1])
                .sub(Scalar::exact(weights[0]))?
                .mul(Scalar::exact(progress))?,
        )?;
        let source = self.scaled(Scalar::exact(1.).sub(weight)?)?;
        let target = target.scaled(weight)?;
        let mut result = source;
        for i in 0..3 {
            result.linear[i] = source.linear[i].add(target.linear[i])?;
            result.angular[i] = source.angular[i].add(target.angular[i])?;
        }
        Ok(result)
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
        let (offset, rotation) = frame.vectors();
        let angular = rotate(rotation, self.angular)?;
        let linear = rotate(rotation, self.linear)?;
        let coupling = cross(angular, offset)?;
        Ok(Self {
            angular,
            linear: [
                linear[0].mul(Scalar::exact(scale))?.sub(coupling[0])?,
                linear[1].mul(Scalar::exact(scale))?.sub(coupling[1])?,
                linear[2].mul(Scalar::exact(scale))?.sub(coupling[2])?,
            ],
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
