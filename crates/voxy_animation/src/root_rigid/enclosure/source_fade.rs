//! Whole-field fade integration directly from immutable authored channels.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct RootRigidSourceField<'a> {
    source: &'a RootRigidCurve,
    reference: f64,
    clip_times: [f64; 2],
    axes: [bool; 3],
    frame: RootRigidEnclosure,
    scale: RootUniformScaleEnclosure,
    max_keys: usize,
}
impl<'a> RootRigidSourceField<'a> {
    /// Bind one original clip clock to a fixed source reference and common
    /// frame. The frame must enclose a real unit rotation. STEP poses reject.
    pub fn new(
        source: &'a RootRigidCurve,
        reference: f64,
        clip_times: [f64; 2],
        axes: [bool; 3],
        frame: RootRigidEnclosure,
        scale: RootUniformScaleEnclosure,
        max_keys: usize,
    ) -> Result<Self, AnimationError> {
        source
            .source_delta_twist_enclosure(reference, clip_times, axes, max_keys)?
            .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
        Ok(Self {
            source,
            reference,
            clip_times,
            axes,
            frame,
            scale,
            max_keys,
        })
    }
    /// Enclose the exact affine original clock over the query before splitting
    /// at source keys/seams. Retiming always uses the full clock, so rounded
    /// subinterval endpoints cannot change the requested playback rate.
    pub fn field(
        &self,
        query: [f64; 2],
        wall: [f64; 2],
    ) -> Result<RootRigidFieldInterval, AnimationError> {
        let factor = super::twist::retiming_factor_between_times(self.clip_times, wall)?;
        if query.into_iter().any(|v| !v.is_finite())
            || query[0] < wall[0]
            || query[1] > wall[1]
            || query[1] < query[0]
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let duration = Scalar::exact(wall[1]).sub(Scalar::exact(wall[0]))?;
        let progress = Scalar(query[0], query[1])
            .sub(Scalar::exact(wall[0]))?
            .div_interval_positive(duration)?;
        let clip = super::cubic::interpolate(
            Scalar::exact(self.clip_times[0]),
            Scalar::exact(self.clip_times[1]),
            Scalar(progress.0.max(0.), progress.1.min(1.)),
        )?;
        let clip = [
            clip.0.max(self.clip_times[0]),
            clip.1.min(self.clip_times[1]),
        ];
        let field = self
            .source
            .source_delta_twist_enclosure(self.reference, clip, self.axes, self.max_keys)?
            .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
        Ok(RootRigidFieldInterval::from_enclosed_domain(
            field
                .scaled(factor)?
                .transformed_enclosed_scale(&self.frame, self.scale)?,
            query,
        ))
    }
}

impl RootRigidCertifiedFadeInterval {
    /// Refine complete original-source field domains until the canonical
    /// trajectory meets both tolerances. None denotes a frozen source.
    /// One global fade clock and one error accumulator span every key and loop.
    /// Floating playback/publication and moving common frames are separate.
    pub fn integrate_sources(
        source: Option<RootRigidSourceField<'_>>,
        target: RootRigidSourceField<'_>,
        weights: [f64; 2],
        duration: f64,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<Self, AnimationError> {
        Self::integrate_sources_with_completion(
            source,
            target,
            weights,
            duration,
            duration,
            origin_tolerance,
            angular_tolerance,
            max_spans,
        )
    }

    /// Finish a fade inside the wall interval, then continue the original
    /// target field. Both clip clocks still map over the full wall duration;
    /// only the blend weight uses fade_duration. The endpoint weight must be 1.
    pub fn integrate_sources_with_completion(
        source: Option<RootRigidSourceField<'_>>,
        target: RootRigidSourceField<'_>,
        weights: [f64; 2],
        duration: f64,
        fade_duration: f64,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<Self, AnimationError> {
        if !duration.is_finite() || duration <= 0. {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        if !fade_duration.is_finite() || fade_duration <= 0. || fade_duration > duration {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let completion = fade_duration < duration;
        if completion && weights[1] != 1. {
            return Err(AnimationError::RootRotationTransitionUnsupported);
        }
        let capacity = max_spans.min(MAX_ROOT_ROTATION_SPANS);
        if capacity < if completion { 2 } else { 1 } {
            return Err(AnimationError::RootRigidBudget);
        }
        let wall = [0., duration];
        let field = |query: [f64; 2]| -> Result<RootRigidFadeFieldInterval, AnimationError> {
            if query[0] >= fade_duration {
                return RootRigidFadeFieldInterval::new(
                    RootRigidFieldInterval::frozen(query)?,
                    target.field(query, wall)?,
                    wall,
                    [1., 1.],
                );
            }
            if query[1] > fade_duration {
                return Err(AnimationError::InvalidSampleTime);
            }
            let source = source
                .map(|value| value.field(query, wall))
                .transpose()?
                .unwrap_or(RootRigidFieldInterval::frozen(query)?);
            RootRigidFadeFieldInterval::new(
                source,
                target.field(query, wall)?,
                [0., fade_duration],
                weights,
            )
        };
        let mut count = if completion { 2 } else { 1 };
        loop {
            let mut domains = Vec::with_capacity(count);
            let fade_count = if completion {
                ((count as f64 * (fade_duration / duration)).round() as usize).clamp(1, count - 1)
            } else {
                count
            };
            let mut previous = 0.;
            for (start, end, cells) in [
                (0., fade_duration, fade_count),
                (fade_duration, duration, count - fade_count),
            ] {
                for index in 1..=cells {
                    let cut = if index == cells {
                        end
                    } else {
                        start + (end - start) * (index as f64 / cells as f64)
                    };
                    if cut <= previous || cut > end {
                        return Err(AnimationError::RootRigidBudget);
                    }
                    domains.push((cut, RootRigidIntegrationDomain::WholeField));
                    previous = cut;
                }
            }
            match RootRigidPath::integrate_spatial_outward_domains(
                &domains,
                origin_tolerance,
                angular_tolerance,
                capacity,
                |_, query| {
                    let enclosure = *field(query)?.velocity_enclosure();
                    Ok((enclosure.nominal_midpoint(), enclosure))
                },
            ) {
                Ok(approximation) => {
                    let fields = approximation
                        .path
                        .spans()
                        .iter()
                        .map(|span| field([span.start(), span.end()]))
                        .collect::<Result<Vec<_>, _>>()?;
                    return Ok(Self::from_integrated_domains(approximation, fields));
                }
                Err(AnimationError::RootRigidBudget) if count < capacity => {
                    count = count.saturating_mul(2).min(capacity);
                }
                Err(error) => return Err(error),
            }
        }
    }
}
