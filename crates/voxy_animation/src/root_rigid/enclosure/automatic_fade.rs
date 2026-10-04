//! Automatic original-clock fade assembly with explicit uncertain key domains.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct RootRigidMappedPath<'a> {
    path: &'a RootRigidPath,
    frame: RootRigidEnclosure,
    scale: f64,
}
impl<'a> RootRigidMappedPath<'a> {
    pub fn new(
        path: &'a RootRigidPath,
        frame: RootRigidTransform,
        scale: f64,
    ) -> Result<Self, AnimationError> {
        if !scale.is_finite() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        Ok(Self {
            path,
            frame: RootRigidEnclosure::from_transform(frame)?,
            scale,
        })
    }
    fn field(
        self,
        query: [f64; 2],
        duration: f64,
        index: Option<usize>,
        uncertain: bool,
    ) -> Result<RootRigidFieldInterval, AnimationError> {
        let zero = RootRigidTwist {
            linear: DVec3::ZERO,
            angular: DVec3::ZERO,
        }
        .enclosure()?;
        let factor = super::twist::retiming_factor_between_times(
            [0., self.path.duration()],
            [0., duration],
        )?;
        let clip = Scalar(query[0], query[1])
            .div_interval_positive(Scalar::exact(duration))?
            .mul(Scalar::exact(self.path.duration()))?;
        let mut times = [clip.0.max(0.), clip.1.min(self.path.duration())];
        let field = if uncertain {
            self.path
                .spatial_twist_enclosure_between(times)?
                .ok_or(AnimationError::RootRotationTransitionUnsupported)?
        } else if let Some(index) = index {
            // The outward wall partition proves the true clock remains in this
            // span. Intersect inverse-clock arithmetic with that domain proof.
            let span = &self.path.spans()[index];
            times[0] = times[0].max(span.start());
            times[1] = times[1].min(span.end());
            span.spatial_twist_enclosure_at_times(times)?
                .ok_or(AnimationError::RootRotationTransitionUnsupported)?
        } else {
            zero
        };
        Ok(RootRigidFieldInterval::from_enclosed_domain(
            field.scaled(factor)?.transformed(&self.frame, self.scale)?,
            query,
        ))
    }
    fn bounds(
        self,
        index: Option<usize>,
        duration: f64,
    ) -> Result<RootSpatialTwistBounds, AnimationError> {
        match index {
            Some(index) => self.path.spans()[index]
                .enclosed_twist_bounds()?
                .ok_or(AnimationError::RootRotationTransitionUnsupported)?
                .enclosed_retimed_between_times([0., self.path.duration()], [0., duration])?
                .enclosed_transformed(self.frame, self.scale),
            None => Ok(RootSpatialTwistBounds {
                linear_speed_bound: 0.,
                angular_speed_bound: 0.,
                rates: RootTwistRateBounds {
                    linear: 0.,
                    angular: 0.,
                },
            }),
        }
    }
}
impl RootRigidCertifiedFadeInterval {
    /// Assembles both original key streams on one stored global wall clock.
    /// Source/target mappings must target a shared fixed frame. None is frozen.
    /// Pose STEP events reject; no Animator clock or pose is published here.
    #[allow(clippy::too_many_arguments)]
    pub fn integrate_paths(
        source: Option<RootRigidMappedPath<'_>>,
        target: RootRigidMappedPath<'_>,
        weights: [f64; 2],
        duration: f64,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<Self, AnimationError> {
        let frozen = RootRigidPath::from_twists(&[], 0)?;
        let source_path = source.map_or(&frozen, |source| source.path);
        let partition = source_path.partition_wall_outward(target.path, duration, max_spans)?;
        let mut intervals = Vec::with_capacity(partition.intervals().len());
        for domain in partition.intervals() {
            let mode = if domain.key_uncertainty {
                RootRigidIntegrationDomain::WholeField
            } else {
                let source_bounds = match source {
                    Some(source) => source.bounds(domain.source_span, duration)?,
                    None => RootSpatialTwistBounds {
                        linear_speed_bound: 0.,
                        angular_speed_bound: 0.,
                        rates: RootTwistRateBounds {
                            linear: 0.,
                            angular: 0.,
                        },
                    },
                };
                RootRigidIntegrationDomain::Derivative(
                    source_bounds
                        .enclosed_blend(
                            target.bounds(domain.target_span, duration)?,
                            weights,
                            duration,
                        )?
                        .rates,
                )
            };
            intervals.push((domain.end, mode));
        }
        let zero = RootRigidTwist {
            linear: DVec3::ZERO,
            angular: DVec3::ZERO,
        }
        .enclosure()?;
        let field = |index: usize, query| -> Result<RootRigidFadeFieldInterval, AnimationError> {
            let domain = &partition.intervals()[index];
            let source = source
                .map(|source| {
                    source.field(query, duration, domain.source_span, domain.key_uncertainty)
                })
                .transpose()?
                .unwrap_or(RootRigidFieldInterval::from_enclosed_domain(zero, query));
            let target =
                target.field(query, duration, domain.target_span, domain.key_uncertainty)?;
            RootRigidFadeFieldInterval::new(source, target, [0., duration], weights)
        };
        let approximation = RootRigidPath::integrate_spatial_outward_domains(
            &intervals,
            origin_tolerance,
            angular_tolerance,
            max_spans,
            |index, query| {
                let enclosed = *field(index, query)?.velocity_enclosure();
                Ok((enclosed.nominal_midpoint(), enclosed))
            },
        )?;
        let mut fields = Vec::with_capacity(approximation.path.spans().len());
        let mut index = 0;
        for span in approximation.path.spans() {
            while span.start() >= partition.intervals()[index].end {
                index += 1;
            }
            fields.push(field(index, [span.start(), span.end()])?);
        }
        Ok(Self::from_integrated_domains(approximation, fields))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_nonrepresentable_key_clock_assembles_with_guard_and_global_weight() {
        let twist = |x| RootRigidTwist {
            linear: DVec3::X * x,
            angular: DVec3::ZERO,
        };
        let target = RootRigidPath::from_twists(&[(twist(1.), 1.), (twist(3.), 2.)], 2).unwrap();
        let mapped = RootRigidMappedPath::new(&target, RootRigidTransform::IDENTITY, 1.).unwrap();
        let fade = RootRigidCertifiedFadeInterval::integrate_paths(
            None,
            mapped,
            [0., 1.],
            1.,
            0.01,
            0.01,
            4096,
        )
        .unwrap();
        let result = fade.approximation();
        // Global retiming speed is 3. Integral is 3*(1/18+3*4/9)=25/6.
        assert!(result.origin_error_bound <= 0.01);
        println!(
            "AUTOMATIC_KEY_PHASE {:?}",
            (
                result.origin_error_bound,
                result.path.end_transform().translation.x,
                result
                    .path
                    .spans()
                    .iter()
                    .map(|span| {
                        let twist = span.screw.unwrap().0;
                        assert_eq!(twist.angular, DVec3::ZERO);
                        (span.start(), span.end(), twist.linear.x)
                    })
                    .collect::<Vec<_>>()
            )
        );
        assert_eq!(
            fade.coordinate_certificate(1, 4096)
                .unwrap()
                .unwrap()
                .error_bound(),
            0.
        );
        let empty = RootRigidPath::from_twists(&[], 0).unwrap();
        let partition = empty.partition_wall_outward(&target, 1., 4096).unwrap();
        for guard in partition
            .intervals()
            .iter()
            .filter(|domain| domain.key_uncertainty)
        {
            assert!(result
                .path
                .spans()
                .iter()
                .any(|span| span.start() == guard.start && span.end() == guard.end));
        }
        assert!(RootRigidCertifiedFadeInterval::integrate_paths(
            None,
            mapped,
            [0., 1.],
            1.,
            0.01,
            0.01,
            1
        )
        .is_err());
    }
}

#[cfg(test)]
mod paired_tests {
    use super::*;
    #[test]
    fn original_source_and_target_key_streams_are_discovered_automatically() {
        let twist = |linear| RootRigidTwist {
            linear,
            angular: DVec3::ZERO,
        };
        let source = RootRigidPath::from_twists(
            &[(twist(DVec3::X * 2.), 1.), (twist(DVec3::Z * 4.), 1.)],
            2,
        )
        .unwrap();
        let target =
            RootRigidPath::from_twists(&[(twist(DVec3::X), 1.), (twist(DVec3::X * 3.), 2.)], 2)
                .unwrap();
        let source = RootRigidMappedPath::new(&source, RootRigidTransform::IDENTITY, 1.).unwrap();
        let target = RootRigidMappedPath::new(&target, RootRigidTransform::IDENTITY, 1.).unwrap();
        let fade = RootRigidCertifiedFadeInterval::integrate_paths(
            Some(source),
            target,
            [0., 1.],
            1.,
            0.01,
            0.01,
            4096,
        )
        .unwrap();
        let result = fade.approximation();
        assert!(result.origin_error_bound <= 0.01);
        assert_eq!(
            fade.coordinate_certificate(1, 4096)
                .unwrap()
                .unwrap()
                .error_bound(),
            0.
        );
        println!(
            "AUTOMATIC_PAIRED_PHASE {:?}",
            (
                result.origin_error_bound,
                result
                    .path
                    .spans()
                    .iter()
                    .map(|span| {
                        let twist = span.screw.unwrap().0;
                        assert_eq!(twist.angular, DVec3::ZERO);
                        (span.start(), span.end(), twist.linear.to_array())
                    })
                    .collect::<Vec<_>>()
            )
        );
    }
}
