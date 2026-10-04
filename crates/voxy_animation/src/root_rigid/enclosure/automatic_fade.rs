//! Automatic original-clock fade assembly with explicit uncertain key domains.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct RootRigidMappedPath<'a> {
    path: &'a RootRigidPath,
    frame: RootRigidEnclosure,
    scale: RootUniformScaleEnclosure,
}
impl<'a> RootRigidMappedPath<'a> {
    pub fn new(
        path: &'a RootRigidPath,
        frame: RootRigidTransform,
        scale: f64,
    ) -> Result<Self, AnimationError> {
        Self::from_enclosed_frame(path, RootRigidEnclosure::from_transform(frame)?, scale)
    }
    /// Retains a previously enclosed continuation frame without reducing it to
    /// a rounded nominal pose. The private enclosure owner supplies validity.
    pub fn from_enclosed_frame(
        path: &'a RootRigidPath,
        frame: RootRigidEnclosure,
        scale: f64,
    ) -> Result<Self, AnimationError> {
        Self::from_enclosed_similarity(path,frame,RootUniformScaleEnclosure::from_scale(scale)?)
    }
    pub fn from_enclosed_similarity(
        path: &'a RootRigidPath, frame: RootRigidEnclosure, scale: RootUniformScaleEnclosure,
    ) -> Result<Self, AnimationError> {
        Ok(Self {path,frame,scale})
    }
    fn field(
        self,
        query: [f64; 2],
        duration: f64,
        index: Option<usize>,
        uncertain: bool,
    ) -> Result<RootRigidFieldInterval, AnimationError> {
        self.field_between(query, [0., duration], index, uncertain)
    }
    fn field_between(
        self,
        query: [f64; 2],
        wall: [f64; 2],
        index: Option<usize>,
        uncertain: bool,
    ) -> Result<RootRigidFieldInterval, AnimationError> {
        if query[0] < wall[0] || query[1] > wall[1] || query[1] < query[0] {
            return Err(AnimationError::InvalidSampleTime);
        }
        let zero = RootRigidTwist {
            linear: DVec3::ZERO,
            angular: DVec3::ZERO,
        }
        .enclosure()?;
        let factor = super::twist::retiming_factor_between_times(
            [0., self.path.duration()],
            wall,
        )?;
        let clip = Scalar(query[0], query[1])
            .sub(Scalar::exact(wall[0]))?
            .div_interval_positive(Scalar::exact(wall[1]).sub(Scalar::exact(wall[0]))?)?
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
            field.scaled(factor)?.transformed_enclosed_scale(&self.frame, self.scale)?,
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
                .enclosed_transformed(self.frame, self.scale.absolute_upper()),
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
        Self::integrate_paths_with_completion(source, target, weights, duration, None,
            origin_tolerance, angular_tolerance, max_spans)
    }
    /// Automatically encloses key crossings in a target-only completion tail.
    /// Tail domains use complete original fields and share the fade integrator's
    /// canonical prefix and global error accumulator. The endpoint is a stored
    /// whole-tick wall time; its difference from fade end is enclosed directly.
    #[allow(clippy::too_many_arguments)]
    pub fn integrate_paths_with_completion(
        source: Option<RootRigidMappedPath<'_>>,
        target: RootRigidMappedPath<'_>,
        weights: [f64; 2],
        duration: f64,
        completion: Option<(RootRigidMappedPath<'_>, f64)>,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<Self, AnimationError> {
        if let Some((_, end)) = completion {
            if !end.is_finite() || end <= duration || weights[1] != 1. {
                return Err(AnimationError::InvalidAnimationTimeStep);
            }
        }
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
            if index >= partition.intervals().len() {
                let (tail, end) = completion.ok_or(AnimationError::InvalidSampleTime)?;
                let target = tail.field_between(query, [duration, end], None, true)?;
                return RootRigidFadeFieldInterval::new(
                    RootRigidFieldInterval::from_enclosed_domain(zero, query),
                    target, [0., end], [1., 1.]);
            }
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
        let base_count = intervals.len();
        let mut tail_count = usize::from(completion.is_some());
        let approximation = loop {
            intervals.truncate(base_count);
            if let Some((_, end)) = completion {
                if base_count + tail_count > max_spans.min(MAX_ROOT_ROTATION_SPANS) {
                    return Err(AnimationError::RootRigidBudget);
                }
                let mut previous = duration;
                for index in 1..=tail_count {
                    let cut = if index == tail_count { end } else {
                        duration + (end - duration) * (index as f64 / tail_count as f64)
                    };
                    if cut <= previous || cut > end {
                        return Err(AnimationError::RootRigidBudget);
                    }
                    intervals.push((cut, RootRigidIntegrationDomain::WholeField));
                    previous = cut;
                }
            }
            match RootRigidPath::integrate_spatial_outward_domains(
                &intervals, origin_tolerance, angular_tolerance, max_spans,
                |index, query| {
                    let enclosed = *field(index, query)?.velocity_enclosure();
                    Ok((enclosed.nominal_midpoint(), enclosed))
                },
            ) {
                Ok(value) => break value,
                Err(AnimationError::RootRigidBudget) if completion.is_some() => {
                    tail_count = tail_count.checked_mul(2).ok_or(AnimationError::RootRigidBudget)?;
                }
                Err(error) => return Err(error),
            }
        };
        let mut fields = Vec::with_capacity(approximation.path.spans().len());
        let mut index = 0;
        for span in approximation.path.spans() {
            while span.start() >= intervals[index].0 {
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

#[cfg(test)]
mod completion_tests {
    use super::*;
    #[test]
    fn automatic_completion_encloses_original_tail_key_with_one_global_budget() {
        let fade = RootRigidPath::from_twists(&[(RootRigidTwist {
            linear:DVec3::X*2., angular:DVec3::ZERO,
        },1.)],1).unwrap();
        let tail = RootRigidPath::from_twists(&[
            (RootRigidTwist {linear:DVec3::X*3., angular:DVec3::ZERO},1.),
            (RootRigidTwist {linear:DVec3::X*6., angular:DVec3::ZERO},2.),
        ],2).unwrap();
        let mapped = |path| RootRigidMappedPath::new(path,RootRigidTransform::IDENTITY,1.).unwrap();
        let result = RootRigidCertifiedFadeInterval::integrate_paths_with_completion(
            None,mapped(&fade),[0.,1.],1.,Some((mapped(&tail),3.)),0.02,0.01,4096).unwrap();
        assert_eq!(result.approximation().path.duration(),3.);
        assert!((result.approximation().path.end_transform().translation.x-16.).abs()
            <= result.approximation().origin_error_bound);
        assert!(result.approximation().origin_error_bound<=0.02);
        assert_eq!(result.coordinate_certificate(1,4096).unwrap().unwrap().error_bound(),0.);
        assert!(RootRigidCertifiedFadeInterval::integrate_paths_with_completion(
            None,mapped(&fade),[0.,1.],1.,Some((mapped(&tail),3.)),0.02,0.01,2).is_err());
    }
}
