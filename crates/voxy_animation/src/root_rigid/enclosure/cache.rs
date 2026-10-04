//! Path-borrowed canonical prefix cache. Preparation is linear; sample is constant work.
use super::*;
#[derive(Debug)]
pub struct RootScrewEnclosurePath<'a> {
    pub(super) path: &'a RootRigidPath,
    pub(super) prefixes: Vec<RootRigidEnclosure>,
    coordinate_ranges: [Option<[f64; 2]>; 3],
}
impl RootRigidPath {
    /// End pose of the stored continuous reference. Ordered screw paths use
    /// canonical prefixes rather than their rounded cached initial transforms.
    /// Compiled polynomial paths use their final stored absolute span. Mixed
    /// references and instantaneous events reject instead of silently switching.
    pub fn continuous_end_enclosure(
        &self,
        max_spans: usize,
    ) -> Result<RootRigidEnclosure, AnimationError> {
        if self.spans.len() > max_spans.min(MAX_ROOT_ROTATION_SPANS) {
            return Err(AnimationError::RootRigidBudget);
        }
        let Some(last) = self.spans.last() else {
            return Ok(RootRigidEnclosure::IDENTITY);
        };
        if self.spans.iter().any(|span| span.is_step()) {
            return Err(AnimationError::RootRotationTransitionUnsupported);
        }
        if self.spans.iter().all(|span| span.screw.is_some()) {
            return self
                .prepare_screw_enclosures(max_spans)?
                .sample(self.spans.len() - 1, 1.);
        }
        if self.spans.iter().any(|span| span.screw.is_some()) {
            return Err(AnimationError::RootRotationTransitionUnsupported);
        }
        last.continuous_pose_enclosure(1.)?
            .ok_or(AnimationError::RootRotationTransitionUnsupported)
    }

    /// Prepares all canonical real screw prefixes with outward arithmetic.
    /// The returned owner borrows this immutable path, preventing stale reuse
    /// after mutation. Unsupported fields/angles, overflow and capacity reject.
    pub fn prepare_screw_enclosures(
        &self,
        max_spans: usize,
    ) -> Result<RootScrewEnclosurePath<'_>, AnimationError> {
        if self.spans.len() > max_spans.min(MAX_ROOT_ROTATION_SPANS) {
            return Err(AnimationError::RootRigidBudget);
        }
        let mut prefixes = Vec::with_capacity(self.spans.len() + 1);
        prefixes.push(RootRigidEnclosure::IDENTITY);
        let mut coordinate_ranges = [Some([f64::INFINITY, f64::NEG_INFINITY]); 3];
        for span in &self.spans {
            let (twist, _) = span
                .screw
                .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
            let increment = twist.increment_between_enclosure(span.start(), span.end())?;
            let next =
                increment.compose(prefixes.last().ok_or(AnimationError::RootRigidBudget)?)?;
            for coordinate in 0..3 {
                if let Some(range) = &mut coordinate_ranges[coordinate] {
                    if (0..3).any(|i| i != coordinate && twist.angular[i] != 0.) {
                        coordinate_ranges[coordinate] = None;
                    } else {
                        range[0] = range[0].min(twist.linear[coordinate]);
                        range[1] = range[1].max(twist.linear[coordinate]);
                    }
                }
            }
            prefixes.push(next);
        }
        if self.spans.is_empty() {
            coordinate_ranges = [Some([0., 0.]); 3];
        }
        Ok(RootScrewEnclosurePath {
            path: self,
            prefixes,
            coordinate_ranges,
        })
    }
}
impl RootScrewEnclosurePath<'_> {
    pub fn path(&self) -> &RootRigidPath {
        self.path
    }
    /// Selects a finite rounded pose from the canonical source enclosure.
    /// Includes actual quaternion normalization in the selected stored pose.
    /// The returned enclosure supports point-specific discrepancy accounting;
    /// body/world mapping and f32 publication remain separate obligations.
    pub fn sample_evaluated(
        &self,
        index: usize,
        fraction: f64,
    ) -> Result<(RootRigidTransform, RootRigidEnclosure), AnimationError> {
        let source = self.sample(index, fraction)?;
        let midpoint = |v: [f64; 2]| v[0] * 0.5 + v[1] * 0.5;
        let q = DQuat::from_array(source.rotation_bounds().map(midpoint));
        if !q.is_finite() || q.length_squared() == 0. {
            return Err(AnimationError::NumericalOverflow);
        }
        let evaluated = RootRigidTransform {
            translation: DVec3::from_array(source.translation_bounds().map(midpoint)),
            rotation: q.normalize(),
        }
        .checked()?;
        Ok((evaluated, source))
    }

    /// Encloses every canonical pose over a closed fraction interval in a span.
    /// Uses the entire elapsed-time interval, without sampled extrema.
    pub fn span_fraction_enclosure(
        &self,
        index: usize,
        fractions: [f64; 2],
    ) -> Result<RootRigidEnclosure, AnimationError> {
        if fractions.iter().any(|value| !value.is_finite())
            || fractions[0] < 0.
            || fractions[1] > 1.
            || fractions[0] > fractions[1]
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let span = self
            .path
            .spans
            .get(index)
            .ok_or(AnimationError::InvalidSampleTime)?;
        let (twist, _) = span
            .screw
            .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
        let mut duration = Scalar::exact(span.end())
            .sub(Scalar::exact(span.start()))?
            .mul(Scalar(fractions[0], fractions[1]))?;
        duration.0 = duration.0.max(0.);
        twist
            .increment_interval_enclosure(duration)?
            .compose(&self.prefixes[index])
    }

    /// Hull of every image of every point in the supplied box for the whole path.
    /// The immutable prefix cache and interval exponentials cover each span.
    pub fn whole_path_point_box_bounds(
        &self,
        point: [[f64; 2]; 3],
    ) -> Result<[[f64; 2]; 3], AnimationError> {
        let mut hull = RootRigidEnclosure::IDENTITY.transform_point_box_bounds(point)?;
        for index in 0..self.path.spans.len() {
            let image = self
                .span_fraction_enclosure(index, [0., 1.])?
                .transform_point_box_bounds(point)?;
            for axis in 0..3 {
                hull[axis][0] = hull[axis][0].min(image[axis][0]);
                hull[axis][1] = hull[axis][1].max(image[axis][1]);
            }
        }
        Ok(hull)
    }

    /// Samples the canonical stored field without replaying earlier segments.
    /// Endpoint samples use their prepared enclosure directly.
    pub fn sample(
        &self,
        index: usize,
        fraction: f64,
    ) -> Result<RootRigidEnclosure, AnimationError> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(AnimationError::InvalidSampleTime);
        }
        let span = self
            .path
            .spans
            .get(index)
            .ok_or(AnimationError::InvalidSampleTime)?;
        if fraction == 0. {
            return Ok(self.prefixes[index]);
        }
        if fraction == 1. {
            return Ok(self.prefixes[index + 1]);
        }
        let (twist, _) = span
            .screw
            .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
        let mut duration = Scalar::exact(span.end())
            .sub(Scalar::exact(span.start()))?
            .mul(Scalar::exact(fraction))?;
        duration.0 = duration.0.max(0.);
        twist
            .increment_interval_enclosure(duration)?
            .compose(&self.prefixes[index])
    }
}

impl RootScrewEnclosurePath<'_> {
    /// Exact structural projection invariant: rotation fixes this coordinate
    /// whenever every spatial angular field has zero orthogonal components.
    /// Returns extrema of the stored coordinate translation velocity across all
    /// segments. These compare signs exactly; no sampled or rounded cross product
    /// is used. Prepared once with the prefixes; each lookup is constant work.
    /// Unsupported coordinate/rotation returns None.
    pub fn coordinate_velocity_range(&self, coordinate: usize) -> Option<[f64; 2]> {
        self.coordinate_ranges.get(coordinate).copied().flatten()
    }
}

impl RootScrewEnclosurePath<'_> {
    /// Canonical point images over a checked stored-time corridor, including holds.
    pub fn point_box_bounds_between(
        &self,
        times: [f64; 2],
        point: [[f64; 2]; 3],
    ) -> Result<[[f64; 2]; 3], AnimationError> {
        if times.iter().any(|t| !t.is_finite())
            || times[0] < 0.
            || times[1] < times[0]
            || times[1] > self.path.duration()
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        RootRigidEnclosure::IDENTITY.transform_point_box_bounds(point)?;
        let mut hull: Option<[[f64; 2]; 3]> = None;
        let mut add = |pose: RootRigidEnclosure| -> Result<(), AnimationError> {
            let image = pose.transform_point_box_bounds(point)?;
            if let Some(old) = &mut hull {
                for axis in 0..3 {
                    old[axis][0] = old[axis][0].min(image[axis][0]);
                    old[axis][1] = old[axis][1].max(image[axis][1]);
                }
            } else {
                hull = Some(image);
            }
            Ok(())
        };
        let mut previous = 0.;
        for (index, span) in self.path.spans.iter().enumerate() {
            if previous < span.start() && times[0] <= span.start() && times[1] >= previous {
                add(self.prefixes[index])?;
            }
            if span.end() >= times[0] && span.start() <= times[1] {
                let (twist, _) = span
                    .screw
                    .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
                let mut elapsed = Scalar(times[0].max(span.start()), times[1].min(span.end()))
                    .sub(Scalar::exact(span.start()))?;
                elapsed.0 = elapsed.0.max(0.);
                add(twist
                    .increment_interval_enclosure(elapsed)?
                    .compose(&self.prefixes[index])?)?;
            }
            previous = span.end();
        }
        if times[1] >= previous {
            add(*self
                .prefixes
                .last()
                .ok_or(AnimationError::RootRigidBudget)?)?;
        }
        hull.ok_or(AnimationError::RootRigidBudget)
    }
}
