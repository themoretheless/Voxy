//! Read-only support geometry. Physical contact and gait ownership stay outside.
use crate::AnimationError;
use glam::{DMat3, DVec3};
use std::sync::Arc;

pub const MAX_FOOT_SUPPORT_POINTS: usize = 4096;

/// Validated local reference points; a convex hull is an optional offline reduction.
#[derive(Clone, Debug)]
pub struct FootSupportPoints(Arc<[DVec3]>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootSupportPoint {
    pub index: usize,
    pub local: DVec3,
    /// Signed distance along normalized world normal, relative to bone origin.
    pub projection: f64,
}

impl FootSupportPoints {
    /// Number of projection evaluations required by a support query.
    pub fn point_count(&self) -> usize {
        self.0.len()
    }

    /// # Errors
    /// Rejects empty/oversized clouds and nonfinite or out-of-range coordinates.
    pub fn new(points: Vec<DVec3>) -> Result<Self, AnimationError> {
        if points.is_empty()
            || points.len() > MAX_FOOT_SUPPORT_POINTS
            || points
                .iter()
                .any(|p| !p.is_finite() || p.abs().max_element() > 1e6)
        {
            return Err(AnimationError::InvalidIkTarget);
        }
        Ok(Self(points.into()))
    }

    /// Finds the minimum point along a world-space surface normal. The supplied
    /// linear frame includes all bone/actor scales and reflections. Translation
    /// is deliberately absent. Exact ties retain source order; no contact is acquired.
    /// # Errors
    /// Rejects singular/nonfinite frames, zero/nonfinite normals and projection overflow.
    pub fn support(
        &self,
        linear: DMat3,
        normal: DVec3,
    ) -> Result<FootSupportPoint, AnimationError> {
        if !linear.is_finite()
            || !linear.inverse().is_finite()
            || !normal.is_finite()
            || normal.abs().max_element() == 0.
        {
            return Err(AnimationError::InvalidIkTarget);
        }
        let normal = (normal / normal.abs().max_element()).normalize();
        let local_direction = linear.transpose() * normal;
        let mut result = None;
        for (index, &local) in self.0.iter().enumerate() {
            let projection = local_direction.dot(local);
            if !projection.is_finite() {
                return Err(AnimationError::NumericalOverflow);
            }
            if result
                .as_ref()
                .is_none_or(|p: &FootSupportPoint| projection < p.projection)
            {
                result = Some(FootSupportPoint {
                    index,
                    local,
                    projection,
                });
            }
        }
        Ok(result.expect("validated nonempty support cloud"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DQuat;
    #[test]
    fn affine_support_matches_transformed_points_and_normal_scaling() {
        let points = vec![
            DVec3::new(0., -0.08, -0.04),
            DVec3::new(0.01, -0.07, 0.14),
            DVec3::ZERO,
        ];
        let shape = FootSupportPoints::new(points.clone()).unwrap();
        for angle in [-0.6, 0., 0.6] {
            let linear = DMat3::from_quat(DQuat::from_rotation_x(angle))
                * DMat3::from_diagonal(DVec3::new(-2., 0.5, 3.));
            let normal = DVec3::new(0.2, 1., -0.1).normalize();
            let actual = shape.support(linear, normal).unwrap();
            for p in &points {
                assert!(normal.dot(linear * *p) >= actual.projection - 1e-14);
            }
            assert!((normal.dot(linear * actual.local) - actual.projection).abs() < 1e-14);
            assert_eq!(
                shape.support(linear, normal * 1e200).unwrap().index,
                actual.index
            );
        }
        assert_eq!(shape.support(DMat3::IDENTITY, DVec3::X).unwrap().index, 0);
    }
    #[test]
    fn invalid_clouds_and_queries_reject_without_mutation() {
        assert!(FootSupportPoints::new(vec![]).is_err());
        assert!(FootSupportPoints::new(vec![DVec3::ZERO; MAX_FOOT_SUPPORT_POINTS + 1]).is_err());
        assert!(FootSupportPoints::new(vec![DVec3::splat(f64::NAN)]).is_err());
        let shape = FootSupportPoints::new(vec![DVec3::X]).unwrap();
        assert!(shape.support(DMat3::ZERO, DVec3::Y).is_err());
        assert!(shape.support(DMat3::IDENTITY, DVec3::ZERO).is_err());
        assert_eq!(
            shape.support(DMat3::IDENTITY, DVec3::X).unwrap().projection,
            1.
        );
    }
}
