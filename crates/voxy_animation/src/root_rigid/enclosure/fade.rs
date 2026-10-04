//! Owned snapshots of complete, commonly framed wall-time velocity domains.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct RootRigidFieldInterval {
    field: RootRigidTwistEnclosure,
    wall_times: [f64; 2],
}
impl RootRigidFieldInterval {
    pub(super) fn from_enclosed_domain(
        field: RootRigidTwistEnclosure,
        wall_times: [f64; 2],
    ) -> Self {
        Self { field, wall_times }
    }
    /// Snapshot a complete continuous source domain, retimed and mapped into
    /// the caller-selected common frame. Stored frame normalization is enclosed.
    pub fn from_span(
        span: &RootRigidSpan,
        clip_times: [f64; 2],
        wall_times: [f64; 2],
        frame: RootRigidTransform,
        scale: f64,
    ) -> Result<Option<Self>, AnimationError> {
        let frame = RootRigidEnclosure::from_transform(frame)?;
        if !scale.is_finite() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        let Some(field) = span.retimed_spatial_twist_enclosure_between(clip_times, wall_times)?
        else {
            return Ok(None);
        };
        Ok(Some(Self {
            field: field.transformed(&frame, scale)?,
            wall_times,
        }))
    }
    /// Frozen interruption source has zero spatial velocity in every frame.
    pub fn frozen(wall_times: [f64; 2]) -> Result<Self, AnimationError> {
        super::twist::retiming_factor_between_times([0., 0.], wall_times)?;
        Ok(Self {
            field: RootRigidTwist {
                linear: DVec3::ZERO,
                angular: DVec3::ZERO,
            }
            .enclosure()?,
            wall_times,
        })
    }
    pub fn wall_times(&self) -> [f64; 2] {
        self.wall_times
    }
    pub fn velocity_enclosure(&self) -> &RootRigidTwistEnclosure {
        &self.field
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RootRigidFadeFieldInterval {
    field: RootRigidFieldInterval,
}
impl RootRigidFadeFieldInterval {
    /// Both snapshots must cover exactly the same stored wall interval and
    /// already use the same caller-selected frame. STEP/tail events are split
    /// before construction. No arbitrary point-sample enclosure is accepted.
    pub fn new(
        source: RootRigidFieldInterval,
        target: RootRigidFieldInterval,
        fade_times: [f64; 2],
        weights: [f64; 2],
    ) -> Result<Self, AnimationError> {
        if source.wall_times != target.wall_times {
            return Err(AnimationError::InvalidSampleTime);
        }
        let field = source.field.blended_between_times(
            &target.field,
            weights,
            fade_times,
            source.wall_times,
        )?;
        Ok(Self {
            field: RootRigidFieldInterval {
                field,
                wall_times: source.wall_times,
            },
        })
    }
    pub fn wall_times(&self) -> [f64; 2] {
        self.field.wall_times
    }
    pub fn velocity_enclosure(&self) -> &RootRigidTwistEnclosure {
        &self.field.field
    }
    /// Bounds every prefix's coordinate displacement discrepancy against a
    /// frozen field for every material point. Initial discrepancy is separate;
    /// orthogonal angular components in either field reject the certificate.
    pub fn coordinate_displacement_error_bound(
        &self,
        axis: usize,
        reference: RootRigidTwist,
    ) -> Result<Option<f64>, AnimationError> {
        self.field.field.coordinate_displacement_error_between(
            self.field.wall_times,
            axis,
            reference,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_wall_domain_fade_preserves_height_with_shifted_yaw_frames() {
        let path = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::X,
                    angular: DVec3::Y * 2.,
                },
                1.,
            )],
            1,
        )
        .unwrap();
        let wall = [1e12, 1e12 + 0.5];
        let source = RootRigidFieldInterval::from_span(
            &path.spans()[0],
            [0., 0.7],
            wall,
            RootRigidTransform {
                translation: DVec3::new(2., 3., -4.),
                rotation: DQuat::from_rotation_y(0.8),
            },
            -2.,
        )
        .unwrap()
        .unwrap();
        let target = RootRigidFieldInterval::from_span(
            &path.spans()[0],
            [0.2, 0.9],
            wall,
            RootRigidTransform {
                translation: DVec3::new(-5., 7., 8.),
                rotation: DQuat::from_rotation_y(-0.4),
            },
            3.,
        )
        .unwrap()
        .unwrap();
        let fade =
            RootRigidFadeFieldInterval::new(source, target, [1e12, 1e12 + 1.], [0., 1.]).unwrap();
        let reference = RootRigidTwist {
            linear: DVec3::X * 80.,
            angular: -DVec3::Y * 40.,
        };
        assert_eq!(
            fade.coordinate_displacement_error_bound(1, reference)
                .unwrap(),
            Some(0.)
        );
        let biased = RootRigidTwist {
            linear: reference.linear + DVec3::Y * 0.25,
            ..reference
        };
        let bound = fade
            .coordinate_displacement_error_bound(1, biased)
            .unwrap()
            .unwrap();
        assert!(bound >= 0.125 && bound - 0.125 < 1e-12);
        let frozen = RootRigidFieldInterval::frozen(wall).unwrap();
        assert_eq!(
            RootRigidFadeFieldInterval::new(frozen, target, [1e12, 1e12 + 1.], [0., 1.])
                .unwrap()
                .coordinate_displacement_error_bound(1, reference)
                .unwrap(),
            Some(0.)
        );
        let mismatched = RootRigidFieldInterval::frozen([wall[0], wall[1] + 0.5]).unwrap();
        assert!(
            RootRigidFadeFieldInterval::new(source, mismatched, [1e12, 1e12 + 1.], [0., 1.])
                .is_err()
        );
        let tilted = RootRigidFieldInterval::from_span(
            &path.spans()[0],
            [0., 0.7],
            wall,
            RootRigidTransform {
                translation: DVec3::ZERO,
                rotation: DQuat::from_rotation_x(0.3),
            },
            1.,
        )
        .unwrap()
        .unwrap();
        assert!(
            RootRigidFadeFieldInterval::new(source, tilted, [1e12, 1e12 + 1.], [0., 1.])
                .unwrap()
                .coordinate_displacement_error_bound(1, reference)
                .unwrap()
                .is_none()
        );
    }
}
