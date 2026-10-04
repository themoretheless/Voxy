//! Certified continuous fade assembly; key/STEP/tail partition is external.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct RootRigidMappedField<'a> {
    span: &'a RootRigidSpan,
    clip_times: [f64; 2],
    frame: RootRigidEnclosure,
    scale: f64,
}
impl<'a> RootRigidMappedField<'a> {
    pub fn new(
        span: &'a RootRigidSpan,
        clip_times: [f64; 2],
        frame: RootRigidTransform,
        scale: f64,
    ) -> Result<Self, AnimationError> {
        Self::from_enclosed_frame(
            span,
            clip_times,
            RootRigidEnclosure::from_transform(frame)?,
            scale,
        )
    }
    /// Preserves an enclosed original-target endpoint for completion domains.
    pub fn from_enclosed_frame(
        span: &'a RootRigidSpan,
        clip_times: [f64; 2],
        frame: RootRigidEnclosure,
        scale: f64,
    ) -> Result<Self, AnimationError> {
        if !scale.is_finite() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        span.spatial_twist_enclosure_at_times(clip_times)?
            .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
        Ok(Self {
            span,
            clip_times,
            frame,
            scale,
        })
    }
    fn field(
        self,
        query: [f64; 2],
        wall_times: [f64; 2],
    ) -> Result<RootRigidFieldInterval, AnimationError> {
        if query[0] < wall_times[0] || query[1] > wall_times[1] || query[1] < query[0] {
            return Err(AnimationError::InvalidSampleTime);
        }
        let factor = super::twist::retiming_factor_between_times(self.clip_times, wall_times)?;
        let duration = Scalar::exact(wall_times[1]).sub(Scalar::exact(wall_times[0]))?;
        let progress = Scalar(query[0], query[1])
            .sub(Scalar::exact(wall_times[0]))?
            .div_interval_positive(duration)?;
        let clip = super::cubic::interpolate(
            Scalar::exact(self.clip_times[0]),
            Scalar::exact(self.clip_times[1]),
            Scalar(progress.0.max(0.), progress.1.min(1.)),
        )?;
        let field = self
            .span
            .spatial_twist_enclosure_at_times(clip.array())?
            .ok_or(AnimationError::RootRotationTransitionUnsupported)?
            .scaled(factor)?
            .transformed(&self.frame, self.scale)?;
        Ok(RootRigidFieldInterval::from_enclosed_domain(field, query))
    }
    fn bounds(self, wall_times: [f64; 2]) -> Result<RootSpatialTwistBounds, AnimationError> {
        self.span
            .enclosed_twist_bounds()?
            .ok_or(AnimationError::RootRotationTransitionUnsupported)?
            .enclosed_retimed_between_times(self.clip_times, wall_times)?
            .enclosed_transformed(self.frame, self.scale)
    }
}

/// One continuous clip-pair domain in a common frame, ending at stored wall time.
/// Starts are zero and the preceding domain endpoint. Pose STEP events are separate.
#[derive(Clone, Copy, Debug)]
pub struct RootRigidFadeDomain<'a> {
    pub end: f64,
    pub source: Option<RootRigidMappedField<'a>>,
    pub target: RootRigidMappedField<'a>,
}

#[derive(Debug)]
pub struct RootRigidCertifiedFadeInterval {
    approximation: RootRigidApproximation,
    fields: Vec<RootRigidFadeFieldInterval>,
}
impl RootRigidCertifiedFadeInterval {
    /// Conservative stored wall prefix for a physical span receipt. The lower
    /// endpoint never advances beyond the real stored-time interpolation.
    pub fn accepted_wall_time(
        &self,
        completed_spans: usize,
        span_fraction: f64,
        complete: bool,
    ) -> Result<f64, AnimationError> {
        let path = &self.approximation.path;
        if !span_fraction.is_finite() || !(0. ..=1.).contains(&span_fraction) {
            return Err(AnimationError::InvalidSampleTime);
        }
        if complete {
            if completed_spans != path.spans().len() || span_fraction != 1. {
                return Err(AnimationError::InvalidSampleTime);
            }
            return Ok(path.duration());
        }
        let span = path
            .spans()
            .get(completed_spans)
            .ok_or(AnimationError::InvalidSampleTime)?;
        if span_fraction == 0. {
            return Ok(span.start());
        }
        if span_fraction == 1. {
            return Ok(span.end());
        }
        let time = super::cubic::interpolate(
            Scalar::exact(span.start()),
            Scalar::exact(span.end()),
            Scalar::exact(span_fraction),
        )?;
        Ok(time.0.max(span.start()))
    }

    pub(super) fn from_integrated_domains(
        approximation: RootRigidApproximation,
        fields: Vec<RootRigidFadeFieldInterval>,
    ) -> Self {
        Self {
            approximation,
            fields,
        }
    }

    /// Integrates one continuous key-free interval in an explicitly common
    /// frame. None is the frozen interruption source. No animator is published.
    #[allow(clippy::too_many_arguments)]
    pub fn integrate(
        source: Option<RootRigidMappedField<'_>>,
        target: RootRigidMappedField<'_>,
        weights: [f64; 2],
        duration: f64,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<Self, AnimationError> {
        if !duration.is_finite() || duration <= 0. {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        Self::integrate_partitioned(
            &[RootRigidFadeDomain {
                end: duration,
                source,
                target,
            }],
            weights,
            origin_tolerance,
            angular_tolerance,
            max_spans,
        )
    }
    /// Builds the complete fade across explicitly mapped source/target key domains.
    /// Weight uses the global fade clock and never restarts at a domain cut.
    pub fn integrate_partitioned(
        domains: &[RootRigidFadeDomain<'_>],
        weights: [f64; 2],
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<Self, AnimationError> {
        let duration = domains
            .last()
            .ok_or(AnimationError::InvalidAnimationTimeStep)?
            .end;
        Self::integrate_partitioned_with_completion(
            domains,
            weights,
            duration,
            origin_tolerance,
            angular_tolerance,
            max_spans,
        )
    }

    /// Integrates a fade and target-only completion in one canonical trajectory
    /// and error accumulator. Domains must cut exactly at the stored fade end;
    /// continuation mappings must already retain the original target frame.
    #[allow(clippy::too_many_arguments)]
    pub fn integrate_partitioned_with_completion(
        domains: &[RootRigidFadeDomain<'_>],
        weights: [f64; 2],
        fade_duration: f64,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<Self, AnimationError> {
        let duration = domains
            .last()
            .ok_or(AnimationError::InvalidAnimationTimeStep)?
            .end;
        if !fade_duration.is_finite() || fade_duration <= 0. || fade_duration > duration {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        if duration > fade_duration
            && (weights[1] != 1. || !domains.iter().any(|domain| domain.end == fade_duration))
        {
            return Err(AnimationError::RootRotationTransitionUnsupported);
        }
        let zero = RootSpatialTwistBounds {
            linear_speed_bound: 0.,
            angular_speed_bound: 0.,
            rates: RootTwistRateBounds {
                linear: 0.,
                angular: 0.,
            },
        };
        let stationary = RootRigidTwist {
            linear: DVec3::ZERO,
            angular: DVec3::ZERO,
        }
        .enclosure()?;
        let mut prepared = Vec::with_capacity(domains.len());
        let mut clocks = Vec::with_capacity(domains.len());
        let mut intervals = Vec::with_capacity(domains.len());
        let mut start = 0.;
        for domain in domains {
            if !domain.end.is_finite() || domain.end <= start {
                return Err(AnimationError::InvalidSampleTime);
            }
            let wall = [start, domain.end];
            let completion = start >= fade_duration;
            if completion && domain.source.is_some() {
                return Err(AnimationError::InvalidRetargetBinding);
            }
            let (domain_weights, weight_duration) = if completion {
                ([1., 1.], duration)
            } else {
                (weights, fade_duration)
            };
            let rates = domain
                .source
                .map(|source| source.bounds(wall))
                .transpose()?
                .unwrap_or(zero)
                .enclosed_blend(domain.target.bounds(wall)?, domain_weights, weight_duration)?
                .rates;
            intervals.push((domain.end, rates));
            prepared.push(wall);
            clocks.push((domain_weights, weight_duration));
            start = domain.end;
        }
        let field = |index: usize, query| -> Result<RootRigidFadeFieldInterval, AnimationError> {
            let domain = &domains[index];
            let source = domain
                .source
                .map(|source| source.field(query, prepared[index]))
                .transpose()?
                .unwrap_or(RootRigidFieldInterval::from_enclosed_domain(
                    stationary, query,
                ));
            RootRigidFadeFieldInterval::new(
                source,
                domain.target.field(query, prepared[index])?,
                [0., clocks[index].1],
                clocks[index].0,
            )
        };
        let approximation = RootRigidPath::integrate_spatial_outward_partitioned(
            &intervals,
            origin_tolerance,
            angular_tolerance,
            max_spans,
            |index, time| {
                let sample = field(index, [time, time])?;
                let enclosed = *sample.velocity_enclosure();
                Ok((enclosed.nominal_midpoint(), enclosed))
            },
        )?;
        let mut fields = Vec::with_capacity(approximation.path.spans().len());
        let mut index = 0;
        for span in approximation.path.spans() {
            while span.start() >= domains[index].end {
                index += 1;
            }
            fields.push(field(index, [span.start(), span.end()])?);
        }
        Ok(Self {
            approximation,
            fields,
        })
    }
    pub fn approximation(&self) -> &RootRigidApproximation {
        &self.approximation
    }
    pub fn coordinate_certificate(
        &self,
        axis: usize,
        max_spans: usize,
    ) -> Result<Option<RootRigidCoordinateCertificate<'_>>, AnimationError> {
        self.approximation
            .path
            .enclose_fade_coordinate_error(&self.fields, axis, max_spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assembled_fade_integrates_translation_and_proves_height_from_same_sources() {
        let path = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::X,
                    angular: DVec3::ZERO,
                },
                1.,
            )],
            1,
        )
        .unwrap();
        let target =
            RootRigidMappedField::new(&path.spans()[0], [0., 1.], RootRigidTransform::IDENTITY, 1.)
                .unwrap();
        let fade =
            RootRigidCertifiedFadeInterval::integrate(None, target, [0., 1.], 1., 0.01, 0.01, 4096)
                .unwrap();
        let approximation = fade.approximation();
        assert!(
            (approximation.path.end_transform().translation.x - 0.5).abs()
                <= approximation.origin_error_bound
        );
        assert_eq!(
            fade.coordinate_certificate(1, 4096)
                .unwrap()
                .unwrap()
                .error_bound(),
            0.
        );
        assert!(
            RootRigidCertifiedFadeInterval::integrate(None, target, [0., 1.], 1., 0., 0., 1)
                .is_err()
        );
    }
}

#[cfg(test)]
mod angular_tests {
    use super::*;
    #[test]
    fn assembled_yaw_fade_bounds_the_analytic_half_radian_rotation() {
        let path = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::ZERO,
                    angular: DVec3::Y,
                },
                1.,
            )],
            1,
        )
        .unwrap();
        let target =
            RootRigidMappedField::new(&path.spans()[0], [0., 1.], RootRigidTransform::IDENTITY, 1.)
                .unwrap();
        let fade =
            RootRigidCertifiedFadeInterval::integrate(None, target, [0., 1.], 1., 0.01, 0.01, 4096)
                .unwrap();
        let approximation = fade.approximation();
        assert!(approximation.angular_error_bound <= 0.01);
        println!(
            "CERTIFIED_YAW_PHASE {:?}",
            (
                approximation.angular_error_bound,
                approximation
                    .path
                    .spans()
                    .iter()
                    .map(|span| {
                        let twist = span.screw.unwrap().0;
                        assert_eq!(twist.angular.x, 0.);
                        assert_eq!(twist.angular.z, 0.);
                        (span.start(), span.end(), twist.angular.y)
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
    }
}

#[cfg(test)]
mod partition_tests {
    use super::*;
    #[test]
    fn global_weight_does_not_restart_at_target_key_and_height_proof_covers_all_domains() {
        let path = RootRigidPath::from_twists(
            &[
                (
                    RootRigidTwist {
                        linear: DVec3::X,
                        angular: DVec3::ZERO,
                    },
                    0.25,
                ),
                (
                    RootRigidTwist {
                        linear: DVec3::X * 3.,
                        angular: DVec3::ZERO,
                    },
                    0.75,
                ),
            ],
            2,
        )
        .unwrap();
        let domains = path
            .spans()
            .iter()
            .map(|span| RootRigidFadeDomain {
                end: span.end(),
                source: None,
                target: RootRigidMappedField::new(
                    span,
                    [span.start(), span.end()],
                    RootRigidTransform::IDENTITY,
                    1.,
                )
                .unwrap(),
            })
            .collect::<Vec<_>>();
        let fade = RootRigidCertifiedFadeInterval::integrate_partitioned(
            &domains,
            [0., 1.],
            0.01,
            0.01,
            4096,
        )
        .unwrap();
        let approximation = fade.approximation();
        assert!(
            (approximation.path.end_transform().translation.x - 1.4375).abs()
                <= approximation.origin_error_bound
        );
        assert!(
            approximation
                .path
                .spans()
                .iter()
                .any(|span| span.end() == 0.25)
        );
        assert!(
            approximation
                .path
                .spans()
                .iter()
                .all(|span| !(span.start() < 0.25 && span.end() > 0.25))
        );
        assert_eq!(
            fade.coordinate_certificate(1, 4096)
                .unwrap()
                .unwrap()
                .error_bound(),
            0.
        );
        let mut invalid = domains.clone();
        invalid[1].end = 0.25;
        assert!(
            RootRigidCertifiedFadeInterval::integrate_partitioned(
                &invalid,
                [0., 1.],
                0.01,
                0.01,
                4096
            )
            .is_err()
        );
        assert!(
            RootRigidCertifiedFadeInterval::integrate_partitioned(
                &domains,
                [0., 1.],
                0.01,
                0.01,
                1
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod paired_key_tests {
    use super::*;
    #[test]
    fn distinct_source_and_target_keys_use_one_global_weight_clock() {
        let twist = |linear| RootRigidTwist {
            linear,
            angular: DVec3::ZERO,
        };
        let source = RootRigidPath::from_twists(
            &[(twist(DVec3::X * 2.), 0.5), (twist(DVec3::Z * 4.), 0.5)],
            2,
        )
        .unwrap();
        let target =
            RootRigidPath::from_twists(&[(twist(DVec3::X), 0.25), (twist(DVec3::X * 3.), 0.75)], 2)
                .unwrap();
        let cuts = [(0., 0.25, 0, 0), (0.25, 0.5, 0, 1), (0.5, 1., 1, 1)];
        let domains = cuts.map(|(start, end, a, b)| RootRigidFadeDomain {
            end,
            source: Some(
                RootRigidMappedField::new(
                    &source.spans()[a],
                    [start, end],
                    RootRigidTransform::IDENTITY,
                    1.,
                )
                .unwrap(),
            ),
            target: RootRigidMappedField::new(
                &target.spans()[b],
                [start, end],
                RootRigidTransform::IDENTITY,
                1.,
            )
            .unwrap(),
        });
        let fade = RootRigidCertifiedFadeInterval::integrate_partitioned(
            &domains,
            [0., 1.],
            0.01,
            0.01,
            4096,
        )
        .unwrap();
        let result = fade.approximation();
        assert!(
            (result.path.end_transform().translation - DVec3::new(2.1875, 0., 0.5)).length()
                <= result.origin_error_bound
        );
        for cut in [0.25, 0.5] {
            assert!(result.path.spans().iter().any(|span| span.end() == cut));
        }
        assert_eq!(
            fade.coordinate_certificate(1, 4096)
                .unwrap()
                .unwrap()
                .error_bound(),
            0.
        );
    }
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    #[test]
    fn fade_completion_uses_one_clock_prefix_and_global_error_budget() {
        let target = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::X * 2.,
                    angular: DVec3::ZERO,
                },
                1.,
            )],
            1,
        )
        .unwrap();
        let tail = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::X * 3.,
                    angular: DVec3::ZERO,
                },
                2.,
            )],
            1,
        )
        .unwrap();
        let mapped = |times| {
            RootRigidMappedField::new(&target.spans()[0], times, RootRigidTransform::IDENTITY, 1.)
                .unwrap()
        };
        let domains = [
            RootRigidFadeDomain {
                end: 0.5,
                source: None,
                target: mapped([0., 0.5]),
            },
            RootRigidFadeDomain {
                end: 1.,
                source: None,
                target: mapped([0.5, 1.]),
            },
            RootRigidFadeDomain {
                end: 3.,
                source: None,
                target: RootRigidMappedField::from_enclosed_frame(
                    &tail.spans()[0],
                    [0., 2.],
                    target.continuous_end_enclosure(1).unwrap(),
                    1.,
                )
                .unwrap(),
            },
        ];
        let assembled = RootRigidCertifiedFadeInterval::integrate_partitioned_with_completion(
            &domains,
            [0., 1.],
            1.,
            0.01,
            0.01,
            4096,
        )
        .unwrap();
        let result = assembled.approximation();
        assert_eq!(result.path.duration(), 3.);
        assert_eq!(
            assembled
                .accepted_wall_time(result.path.spans().len(), 1., true)
                .unwrap(),
            3.
        );
        let first = &result.path.spans()[0];
        assert_eq!(
            assembled.accepted_wall_time(0, 0., false).unwrap(),
            first.start()
        );
        assert_eq!(
            assembled.accepted_wall_time(0, 1., false).unwrap(),
            first.end()
        );
        let partial = assembled.accepted_wall_time(0, 0.5, false).unwrap();
        assert!(partial >= first.start() && partial <= (first.start() + first.end()) * 0.5);
        assert!(assembled.accepted_wall_time(0, 1., true).is_err());
        assert!(
            assembled
                .accepted_wall_time(result.path.spans().len(), 0., false)
                .is_err()
        );
        assert!(assembled.accepted_wall_time(0, f64::NAN, false).is_err());
        assert!(
            (result.path.end_transform().translation.x - 7.).abs() <= result.origin_error_bound
        );
        assert!(result.origin_error_bound <= 0.01);
        assert_eq!(
            assembled
                .coordinate_certificate(1, 4096)
                .unwrap()
                .unwrap()
                .error_bound(),
            0.
        );
        for cut in [0.5, 1.] {
            assert!(result.path.spans().iter().any(|span| span.end() == cut));
        }
        assert!(
            RootRigidCertifiedFadeInterval::integrate_partitioned_with_completion(
                &domains,
                [0., 0.9],
                1.,
                0.01,
                0.01,
                4096
            )
            .is_err()
        );
        assert!(
            RootRigidCertifiedFadeInterval::integrate_partitioned_with_completion(
                &domains,
                [0., 1.],
                0.75,
                0.01,
                0.01,
                4096
            )
            .is_err()
        );
        assert!(
            RootRigidCertifiedFadeInterval::integrate_partitioned_with_completion(
                &domains,
                [0., 1.],
                1.,
                0.01,
                0.01,
                2
            )
            .is_err()
        );
    }
}
