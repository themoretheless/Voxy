//! Durable follow target plus frame-rate-independent first-order presentation smoothing.
use crate::GameplayError;
use glam::DVec3;
use serde::{Deserialize, Serialize};
use voxy_scene::ObjectId;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FollowCamera {
    pub target: ObjectId,
    pub offset: [f32; 3],
    pub half_life: f64,
}
impl FollowCamera {
    #[must_use]
    pub fn valid(&self) -> bool {
        !self.target.0.is_empty()
            && self.offset.iter().all(|v| v.is_finite())
            && self.half_life.is_finite()
            && self.half_life > 0.
    }
}
/// Exact first-order response to a piecewise linear sampled target (up to f64 rounding).
/// Stores presentation history separately from scene/physics transforms.
#[derive(Clone, Debug)]
pub struct FollowSmoother {
    position: DVec3,
    previous_target: DVec3,
}
/// Exact presentation history; integer words preserve every finite f64 bit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FollowSmootherCheckpoint {
    version: u32,
    position: [u64; 3],
    previous_target: [u64; 3],
}
impl FollowSmootherCheckpoint {
    /// Rebuilds independent history without advancing the camera.
    /// # Errors
    /// Rejects unsupported versions and nonfinite state.
    pub fn prepare(&self) -> Result<FollowSmoother, GameplayError> {
        if self.version != 1 {
            return Err("unsupported follow smoother checkpoint version".into());
        }
        let position = DVec3::from_array(self.position.map(f64::from_bits));
        let previous_target = DVec3::from_array(self.previous_target.map(f64::from_bits));
        if !position.is_finite() || !previous_target.is_finite() {
            return Err("nonfinite follow smoother checkpoint".into());
        }
        Ok(FollowSmoother {
            position,
            previous_target,
        })
    }
}
impl FollowSmoother {
    /// Captures both the filtered position and prior sampled target.
    #[must_use]
    pub fn checkpoint(&self) -> FollowSmootherCheckpoint {
        FollowSmootherCheckpoint {
            version: 1,
            position: self.position.to_array().map(f64::to_bits),
            previous_target: self.previous_target.to_array().map(f64::to_bits),
        }
    }
    /// # Errors
    /// Rejects a nonfinite initial target.
    pub fn new(target: DVec3) -> Result<Self, GameplayError> {
        if !target.is_finite() {
            return Err("nonfinite camera follow target".into());
        }
        Ok(Self {
            position: target,
            previous_target: target,
        })
    }
    /// # Errors
    /// Rejects invalid time/target/half-life or overflow without changing history.
    pub fn advance(
        &mut self,
        target: DVec3,
        half_life: f64,
        dt: f64,
    ) -> Result<DVec3, GameplayError> {
        if !target.is_finite()
            || !half_life.is_finite()
            || half_life <= 0.
            || !dt.is_finite()
            || dt < 0.
        {
            return Err("invalid camera follow sample".into());
        }
        if dt == 0. {
            return Ok(self.position);
        }
        let x = std::f64::consts::LN_2 * (dt / half_life);
        let (decay, old_weight, new_weight) = if x.is_infinite() {
            // The exponential vanishes, but 1/x can remain representable even
            // when dt/half_life overflowed. Preserve that prior-target weight.
            let inverse = (half_life / dt) / std::f64::consts::LN_2;
            (0., inverse, 1. - inverse)
        } else if x == 0. {
            (1., 0., 0.)
        } else {
            let decay = (-x).exp();
            let response = -(-x).exp_m1();
            if x < 0.001 {
                // Stable series for 1-(1-exp(-x))/x near zero.
                let ramp = x
                    * (0.5
                        + x * (-1. / 6.
                            + x * (1. / 24. + x * (-1. / 120. + x * (1. / 720. - x / 5040.)))));
                (decay, response - ramp, ramp)
            } else {
                let inverse_response = response / x;
                (decay, inverse_response - decay, 1. - inverse_response)
            }
        };
        // Weighted form retains a tiny prior-target contribution when the new
        // target is zero; subtracting two large positions would erase it.
        let position =
            self.position * decay + self.previous_target * old_weight + target * new_weight;
        if !position.is_finite() {
            return Err("camera follow overflow".into());
        }
        self.position = position;
        self.previous_target = target;
        Ok(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linear_target_response_matches_across_frame_partitions_and_analytic_solution() {
        let half_life = 0.2;
        let rate = std::f64::consts::LN_2 / half_life;
        let expected = 2. - (1. - (-rate * 2.).exp()) / rate;
        for hz in [30, 60, 120, 160, 240, 1000] {
            let mut filter = FollowSmoother::new(DVec3::ZERO).unwrap();
            let mut value = DVec3::ZERO;
            for frame in 1..=2 * hz {
                value = filter
                    .advance(
                        DVec3::new(f64::from(frame) / f64::from(hz), 0., 0.),
                        half_life,
                        1. / f64::from(hz),
                    )
                    .unwrap();
            }
            assert!(
                (value.x - expected).abs() < 1e-12,
                "hz={hz} value={value:?}"
            );
        }
    }
    #[test]
    fn tiny_residual_is_preserved_for_fast_response_to_zero() {
        let mut filter = FollowSmoother::new(DVec3::X).unwrap();
        let result = filter
            .advance(DVec3::ZERO, std::f64::consts::LN_2 * 1e-18, 1.)
            .unwrap();
        assert!((result.x / 1e-18 - 1.).abs() < 1e-14);
        let mut filter = FollowSmoother::new(DVec3::X).unwrap();
        let result = filter.advance(DVec3::ZERO, f64::MIN_POSITIVE, 10.).unwrap();
        let expected = (f64::MIN_POSITIVE / 10.) / std::f64::consts::LN_2;
        assert!((result.x / expected - 1.).abs() < 1e-14);
    }
    #[test]
    fn invalid_samples_preserve_history_and_extreme_half_lives_stay_finite() {
        let mut filter = FollowSmoother::new(DVec3::ZERO).unwrap();
        let control = filter.clone();
        for (target, half, dt) in [
            (DVec3::NAN, 1., 1.),
            (DVec3::X, 0., 1.),
            (DVec3::X, 1., -1.),
            (DVec3::X, 1., f64::NAN),
        ] {
            assert!(filter.advance(target, half, dt).is_err());
        }
        assert_eq!(filter.advance(DVec3::X, 1., 0.).unwrap(), DVec3::ZERO);
        let mut control = control;
        assert_eq!(
            filter.advance(DVec3::X, 1., 0.1).unwrap(),
            control.advance(DVec3::X, 1., 0.1).unwrap()
        );
        assert!(
            filter
                .advance(DVec3::X, f64::MAX, 1e-9)
                .unwrap()
                .is_finite()
        );
        let result = filter.advance(DVec3::Y, f64::MIN_POSITIVE, 1.).unwrap();
        // An almost instantaneous response still retains the representable
        // contribution of the prior target; it must not be rounded to zero.
        let residual = f64::MIN_POSITIVE / std::f64::consts::LN_2;
        assert!((result.x / residual - 1.).abs() < 1e-14);
        assert_eq!(result.y, 1.);
        assert_eq!(result.z, 0.);
    }
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    #[test]
    fn restored_history_preserves_every_continuation_bit_and_rejects_corruption() {
        let mut original = FollowSmoother::new(DVec3::new(-0., 2., -3.)).unwrap();
        original
            .advance(DVec3::new(4., -5., 6.), 0.13, 0.019)
            .unwrap();
        let saved = original.checkpoint();
        let json = serde_json::to_string(&saved).unwrap();
        let decoded: FollowSmootherCheckpoint = serde_json::from_str(&json).unwrap();
        let mut restored = decoded.prepare().unwrap();
        assert_eq!(saved, restored.checkpoint());
        for frame in 0..720 {
            let t = f64::from(frame) * 0.017;
            let target = DVec3::new(t.sin(), t.cos(), t * 0.3);
            let dt = [0., 0.003, 0.021, 0.037][frame as usize % 4];
            let a = original.advance(target, 0.13, dt).unwrap();
            let b = restored.advance(target, 0.13, dt).unwrap();
            assert_eq!(
                a.to_array().map(f64::to_bits),
                b.to_array().map(f64::to_bits)
            );
            assert_eq!(original.checkpoint(), restored.checkpoint());
        }
        for word in [f64::NAN.to_bits(), f64::INFINITY.to_bits()] {
            let mut bad = saved.clone();
            bad.previous_target[0] = word;
            assert!(bad.prepare().is_err());
            bad = saved.clone();
            bad.position[2] = word;
            assert!(bad.prepare().is_err());
        }
        let mut bad = saved;
        bad.version = 2;
        assert!(bad.prepare().is_err());
    }
}
