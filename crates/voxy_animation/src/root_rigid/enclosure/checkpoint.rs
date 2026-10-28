//! Exact stored interval endpoints; no recentering, normalization or widening.
use super::*;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootRigidEnclosureCheckpoint {
    version: u32,
    translation: [[u64; 2]; 3],
    rotation: [[u64; 2]; 4],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootUniformScaleCheckpoint {
    version: u32,
    bounds: [u64; 2],
}

fn ordered([lo, hi]: [f64; 2]) -> bool {
    lo.is_finite() && hi.is_finite() && lo <= hi
}
// A finite nonnegative binary64 value is an integer multiple of 2^-1074.
// Squaring the integer therefore places every squared component on 2^-2148,
// including the smallest subnormal; no rounded floating norm is used.
fn square_units(value: f64) -> BigUint {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as usize;
    let fraction = bits & ((1u64 << 52) - 1);
    let units = if exponent == 0 {
        BigUint::from(fraction)
    } else {
        BigUint::from(fraction | (1u64 << 52)) << (exponent - 1)
    };
    &units * &units
}
fn contains_unit(rotation: [[f64; 2]; 4]) -> bool {
    let mut min = BigUint::from(0u8);
    let mut max = BigUint::from(0u8);
    for [lo, hi] in rotation {
        let nearest = if lo <= 0. && hi >= 0. {
            0.
        } else {
            lo.abs().min(hi.abs())
        };
        let farthest = lo.abs().max(hi.abs());
        min += square_units(nearest);
        max += square_units(farthest);
    }
    let one = BigUint::from(1u8) << 2148usize;
    // Squared norm has a continuous range on the connected component box.
    // These exact extrema prove that its intersection with the unit sphere
    // is nonempty. Represented rotations remain restricted to that intersection.
    min <= one && one <= max
}
impl RootRigidEnclosure {
    #[must_use]
    pub fn checkpoint(&self) -> RootRigidEnclosureCheckpoint {
        RootRigidEnclosureCheckpoint {
            version: 1,
            translation: self.translation.map(|p| p.map(f64::to_bits)),
            rotation: self.rotation.map(|p| p.map(f64::to_bits)),
        }
    }
}
impl RootRigidEnclosureCheckpoint {
    /// Restore the exact box of translations and unit rotations. The enclosing
    /// save must verify the reference's asset/game identity and bound file IO.
    /// This admits a valid interval state; it does not authenticate its provenance.
    /// # Errors
    /// Rejects versions, nonfinite/reversed intervals and boxes containing no unit quaternion.
    pub fn prepare(&self) -> Result<RootRigidEnclosure, AnimationError> {
        let translation = self.translation.map(|p| p.map(f64::from_bits));
        let rotation = self.rotation.map(|p| p.map(f64::from_bits));
        if self.version != 1
            || !translation.into_iter().all(ordered)
            || !rotation.into_iter().all(ordered)
            || !contains_unit(rotation)
        {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        Ok(RootRigidEnclosure {
            translation,
            rotation,
        })
    }
}
impl RootUniformScaleEnclosure {
    #[must_use]
    pub fn checkpoint(&self) -> RootUniformScaleCheckpoint {
        RootUniformScaleCheckpoint {
            version: 1,
            bounds: self.bounds().map(f64::to_bits),
        }
    }
}
impl RootUniformScaleCheckpoint {
    /// Restore exact signed scale bounds. Existing use sites still own the
    /// nonzero/invertibility requirement; zero is valid for from_scale itself.
    /// # Errors
    /// Rejects unsupported versions and nonfinite/reversed endpoints.
    pub fn prepare(&self) -> Result<RootUniformScaleEnclosure, AnimationError> {
        let value = self.bounds.map(f64::from_bits);
        if self.version != 1 || !ordered(value) {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        Ok(RootUniformScaleEnclosure {
            value: Scalar(value[0], value[1]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bits(value: RootRigidEnclosure) -> ([[u64; 2]; 3], [[u64; 2]; 4]) {
        (
            value.translation_bounds().map(|p| p.map(f64::to_bits)),
            value.rotation_bounds().map(|p| p.map(f64::to_bits)),
        )
    }
    #[test]
    fn exact_interval_json_roundtrip_preserves_transport_and_scale_products() {
        let body = |i: f64| {
            RootRigidEnclosure::from_transform(RootRigidTransform {
                translation: DVec3::new(i * 0.01, -0., i * 0.003),
                rotation: DQuat::from_rotation_y(i * 0.007),
            })
            .unwrap()
        };
        let mut original = body(0.).inverse().unwrap().compose(&body(1.)).unwrap();
        let text = serde_json::to_string(&original.checkpoint()).unwrap();
        let saved: RootRigidEnclosureCheckpoint = serde_json::from_str(&text).unwrap();
        let mut restored = saved.prepare().unwrap();
        assert_eq!(bits(original), bits(restored));
        for i in 0..180 {
            original = original
                .transported_body_reference(&body(f64::from(i)), &body(f64::from(i + 1)))
                .unwrap();
            restored = restored
                .transported_body_reference(&body(f64::from(i)), &body(f64::from(i + 1)))
                .unwrap();
            assert_eq!(bits(original), bits(restored));
            assert_eq!(
                original
                    .transform_point(DVec3::new(0.1, 0.2, 0.3))
                    .unwrap()
                    .map(|p| p.map(f64::to_bits)),
                restored
                    .transform_point(DVec3::new(0.1, 0.2, 0.3))
                    .unwrap()
                    .map(|p| p.map(f64::to_bits))
            );
        }
        let scale = RootUniformScaleEnclosure::from_scale(-0.731)
            .unwrap()
            .multiplied(RootUniformScaleEnclosure::from_scale(1.137).unwrap())
            .unwrap();
        let saved: RootUniformScaleCheckpoint =
            serde_json::from_str(&serde_json::to_string(&scale.checkpoint()).unwrap()).unwrap();
        assert_eq!(
            scale.bounds().map(f64::to_bits),
            saved.prepare().unwrap().bounds().map(f64::to_bits)
        );
        assert_eq!(
            bits(RootRigidEnclosure::IDENTITY),
            bits(RootRigidEnclosure::IDENTITY.checkpoint().prepare().unwrap())
        );
    }
    #[test]
    fn exact_unit_sphere_check_detects_subnormal_excess_lost_by_float_norm() {
        let mut saved = RootRigidEnclosure::IDENTITY.checkpoint();
        // Point quaternion (smallest subnormal,0,0,1) has squared norm
        // exactly 1+2^-2148, although its binary64 norm rounds to one.
        saved.rotation[0] = [1, 1];
        assert_eq!(f64::from_bits(1) * f64::from_bits(1) + 1., 1.);
        assert!(saved.prepare().is_err());
        let mut invalid = RootRigidEnclosure::IDENTITY.checkpoint();
        invalid.rotation = [[0; 2]; 4];
        assert!(invalid.prepare().is_err());
        let mut invalid = RootRigidEnclosure::IDENTITY.checkpoint();
        invalid.translation[0] = [1f64.to_bits(), 0];
        assert!(invalid.prepare().is_err());
        let mut invalid = RootRigidEnclosure::IDENTITY.checkpoint();
        invalid.rotation[0] = [f64::NAN.to_bits(); 2];
        assert!(invalid.prepare().is_err());
        let mut invalid = RootUniformScaleEnclosure::ONE.checkpoint();
        invalid.bounds = [f64::INFINITY.to_bits(); 2];
        assert!(invalid.prepare().is_err());
    }
}
