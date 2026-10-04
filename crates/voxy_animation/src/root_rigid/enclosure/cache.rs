//! Path-borrowed canonical prefix cache. Preparation is linear; sample is constant work.
use super::*;
/// Frozen evaluated pose and component discrepancy against its canonical
/// source enclosure. Bounds include the actual midpoint/normalization result.
#[derive(Clone, Copy, Debug)]
pub struct RootRigidEvaluatedPose {
    pose: RootRigidTransform,
    source: RootRigidEnclosure,
    translation_error: [f64; 3],
    rotation_error: [f64; 4],
}
impl RootRigidEvaluatedPose {
    pub fn pose(&self) -> RootRigidTransform {
        self.pose
    }
    pub fn source(&self) -> RootRigidEnclosure {
        self.source
    }
    /// Per-component absolute errors, translation then quaternion components.
    /// This certifies this source enclosure, not all times in a trajectory.
    pub fn component_error_bounds(&self) -> ([f64; 3], [f64; 4]) {
        (self.translation_error, self.rotation_error)
    }

    /// Source-to-runtime point discrepancy for every stored point in this box.
    /// Matches pose.rotation * point + pose.translation (glam f64 operations).
    /// This is uniform over points at this pose, not over trajectory times.
    pub fn point_evaluation_error_bounds(
        &self,
        point: [[f64; 2]; 3],
    ) -> Result<([f64; 3], f64), AnimationError> {
        if point
            .iter()
            .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
        {
            return Err(AnimationError::NumericalOverflow);
        }
        let (axes, _) = compilation::mapped_point_runtime_error(
            point,
            [0.; 3],
            &self.source,
            RootUniformScaleEnclosure::from_scale(1.)?,
            self.pose,
            1.,
        )?;
        let mut radius = Scalar::exact(0.);
        for axis in axes {
            radius = radius.add(Scalar::exact(axis))?;
        }
        Ok((axes, radius.1))
    }

    /// Point-box discrepancy after local evaluation, fixed world-frame rotation,
    /// signed scale, offset, and direct f32 conversion. Matches that explicit
    /// operation sequence, not composed scene-matrix evaluation or ground snap.
    pub fn mapped_f32_point_error_bounds(
        &self,
        point: [[f64; 2]; 3],
        source_frame: &RootRigidEnclosure,
        source_scale: RootUniformScaleEnclosure,
        actual_frame: RootRigidTransform,
        actual_scale: f64,
    ) -> Result<([f64; 3], f64), AnimationError> {
        let (local_error, _) = self.point_evaluation_error_bounds(point)?;
        let source_point = self.source.transform_point_box_bounds(point)?;
        let (before_cast, actual_box) = compilation::mapped_point_runtime_error(
            source_point,
            local_error,
            source_frame,
            source_scale,
            actual_frame,
            actual_scale,
        )?;
        let (publication, _) = RootRigidEnclosure::enclosed_f32_publication_error(actual_box)?;
        let mut axes = [0.; 3];
        let mut radius = Scalar::exact(0.);
        for i in 0..3 {
            axes[i] = Scalar::exact(before_cast[i])
                .add(Scalar::exact(publication[i]))?
                .1;
            radius = radius.add(Scalar::exact(axes[i]))?;
        }
        Ok((axes, radius.1))
    }
}
impl RootRigidEnclosure {
    /// Evaluates this enclosure once and retains the exact stored result used
    /// for discrepancy accounting. Rejects nonfinite or degenerate results.
    pub fn evaluate_midpoint(&self) -> Result<RootRigidEvaluatedPose, AnimationError> {
        let midpoint = |v: [f64; 2]| v[0] * 0.5 + v[1] * 0.5;
        let q = DQuat::from_array(self.rotation_bounds().map(midpoint));
        if !q.is_finite() || q.length_squared() == 0. {
            return Err(AnimationError::NumericalOverflow);
        }
        let pose = RootRigidTransform {
            translation: DVec3::from_array(self.translation_bounds().map(midpoint)),
            rotation: q.normalize(),
        }
        .checked()?;
        let error = |bounds: [f64; 2], actual: f64| -> Result<f64, AnimationError> {
            let delta = Scalar(bounds[0], bounds[1]).sub(Scalar::exact(actual))?;
            Ok(delta.0.abs().max(delta.1.abs()))
        };
        let mut translation_error = [0.; 3];
        let mut rotation_error = [0.; 4];
        for i in 0..3 {
            translation_error[i] = error(self.translation[i], pose.translation[i])?;
        }
        for i in 0..4 {
            rotation_error[i] = error(self.rotation[i], pose.rotation.to_array()[i])?;
        }
        Ok(RootRigidEvaluatedPose {
            pose,
            source: *self,
            translation_error,
            rotation_error,
        })
    }
}

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
        let evaluated = self.sample_evaluated_with_errors(index, fraction)?;
        Ok((evaluated.pose(), evaluated.source()))
    }

    /// The same physical evaluator with immutable source/component error data.
    /// Prefix/path identity is retained by this borrowed canonical cache.
    pub fn sample_evaluated_with_errors(
        &self,
        index: usize,
        fraction: f64,
    ) -> Result<RootRigidEvaluatedPose, AnimationError> {
        self.sample(index, fraction)?.evaluate_midpoint()
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

// Domain contains all endpoint values across a time family; width bounds only
// each member's enclosure diameter. Temporal variation is not an error term.
#[derive(Clone, Copy)]
struct EnclosureFamily {
    domain: Scalar,
    width: Scalar,
}
fn adjacent_spacing(domain: Scalar) -> Result<Scalar, AnimationError> {
    let magnitude = domain.0.abs().max(domain.1.abs());
    let upper = magnitude.next_up();
    if !magnitude.is_finite() || !upper.is_finite() {
        return Err(AnimationError::NumericalOverflow);
    }
    Scalar::exact(upper).sub(Scalar::exact(magnitude))
}
impl EnclosureFamily {
    fn constant(domain: Scalar) -> Result<Self, AnimationError> {
        Ok(Self {
            domain,
            width: Scalar::exact(domain.1).sub(Scalar::exact(domain.0))?,
        })
    }
    fn add(self, other: Self) -> Result<Self, AnimationError> {
        if self.domain.is_zero() {
            return Ok(other);
        }
        if other.domain.is_zero() {
            return Ok(self);
        }
        let domain = self.domain.add(other.domain)?;
        let width = self
            .width
            .add(other.width)?
            .add(adjacent_spacing(domain)?.mul(Scalar::exact(4.))?)?;
        Ok(Self { domain, width })
    }
    fn mul(self, other: Self) -> Result<Self, AnimationError> {
        if self.domain.is_zero() || other.domain.is_zero() {
            return Self::constant(Scalar::exact(0.));
        }
        let domain = self.domain.mul(other.domain)?;
        let magnitude = |v: Scalar| Scalar::exact(v.0.abs().max(v.1.abs()));
        let width = magnitude(self.domain)
            .mul(other.width)?
            .add(magnitude(other.domain).mul(self.width)?)?
            .add(adjacent_spacing(domain)?.mul(Scalar::exact(4.))?)?;
        Ok(Self { domain, width })
    }
    fn selection_error(self) -> Result<f64, AnimationError> {
        // The exact midpoint lies within each member enclosure. Two halving
        // operations and one addition incur at most four adjacent spacings.
        Ok(self
            .width
            .add(adjacent_spacing(self.domain)?.mul(Scalar::exact(4.))?)?
            .1)
    }
}
impl RootScrewEnclosurePath<'_> {
    /// Uniform local component error of sample_evaluated for every fraction of
    /// this entire translation-only path. Measures enclosure width/selection
    /// rounding separately from temporal displacement. Nonzero angular rates
    /// return None; frame, scene publication and upstream field errors are separate.
    pub fn translation_selection_error_bounds(&self) -> Result<Option<[f64; 3]>, AnimationError> {
        if self.path.spans.iter().any(|span| {
            span.screw
                .is_none_or(|(twist, _)| twist.angular != DVec3::ZERO)
        }) {
            return Ok(None);
        }
        let mut caps = [0_f64; 3];
        for prefix in &self.prefixes {
            for (axis, cap) in caps.iter_mut().enumerate() {
                let bounds = prefix.translation[axis];
                *cap = cap.max(
                    EnclosureFamily::constant(Scalar(bounds[0], bounds[1]))?.selection_error()?,
                );
            }
        }
        for (index, span) in self.path.spans.iter().enumerate() {
            let (twist, _) = span.screw.ok_or(AnimationError::RootRigidBudget)?;
            let mut duration = Scalar::exact(span.end()).sub(Scalar::exact(span.start()))?;
            duration.0 = duration.0.max(0.);
            let elapsed = EnclosureFamily::constant(duration)?.mul(EnclosureFamily {
                domain: Scalar(0., 1.),
                width: Scalar::exact(0.),
            })?;
            for (axis, cap) in caps.iter_mut().enumerate() {
                let increment =
                    EnclosureFamily::constant(Scalar::exact(twist.linear[axis]))?.mul(elapsed)?;
                let prefix = self.prefixes[index].translation[axis];
                let pose =
                    increment.add(EnclosureFamily::constant(Scalar(prefix[0], prefix[1]))?)?;
                *cap = cap.max(pose.selection_error()?);
            }
        }
        Ok(Some(caps))
    }
}

impl EnclosureArithmetic for EnclosureFamily {
    fn exact(value: f64) -> Self {
        Self {
            domain: Scalar::exact(value),
            width: Scalar::exact(0.),
        }
    }
    fn domain(self) -> Scalar {
        self.domain
    }
    fn add(self, other: Self) -> Result<Self, AnimationError> {
        self.add(other)
    }
    fn mul(self, other: Self) -> Result<Self, AnimationError> {
        self.mul(other)
    }
    fn sub(self, other: Self) -> Result<Self, AnimationError> {
        if other.domain.is_zero() {
            return Ok(self);
        }
        let domain = self.domain.sub(other.domain)?;
        let width = self
            .width
            .add(other.width)?
            .add(adjacent_spacing(domain)?.mul(Scalar::exact(4.))?)?;
        Ok(Self { domain, width })
    }
    fn square(self) -> Result<Self, AnimationError> {
        if self.domain.is_zero() {
            return Ok(self);
        }
        let domain = self.domain.square()?;
        let magnitude = Scalar::exact(self.domain.0.abs().max(self.domain.1.abs()));
        let width = magnitude
            .mul(Scalar::exact(2.))?
            .mul(self.width)?
            .add(adjacent_spacing(domain)?.mul(Scalar::exact(4.))?)?;
        Ok(Self { domain, width })
    }
    fn div_positive(self, value: f64) -> Result<Self, AnimationError> {
        if self.domain.is_zero() {
            return Ok(self);
        }
        let domain = self.domain.div_positive(value)?;
        let width = self
            .width
            .div_positive(value)?
            .add(adjacent_spacing(domain)?.mul(Scalar::exact(4.))?)?;
        Ok(Self { domain, width })
    }
    fn negate(self) -> Self {
        Self {
            domain: Scalar(-self.domain.1, -self.domain.0),
            width: self.width,
        }
    }
    fn nonnegative(self) -> Self {
        Self {
            domain: Scalar(self.domain.0.max(0.), self.domain.1),
            width: self.width,
        }
    }
    fn symmetric_remainder(self) -> Result<Self, AnimationError> {
        let radius = self.domain.0.abs().max(self.domain.1.abs());
        Ok(Self {
            domain: Scalar(-radius, radius),
            width: Scalar::exact(radius).mul(Scalar::exact(2.))?,
        })
    }
}
impl RootScrewEnclosurePath<'_> {
    /// Uniform local translation/quaternion discrepancy of sample_evaluated
    /// over all fractions of every prepared screw span. Includes actual midpoint
    /// selection and normalization. Upstream field, frame and scene errors are
    /// separate; exhausted angular/normalization domains reject.
    pub fn selection_error_bounds(&self) -> Result<([f64; 3], [f64; 4]), AnimationError> {
        let mut translation = [0_f64; 3];
        let mut rotation = [0_f64; 4];
        let mut accumulate =
            |t: [EnclosureFamily; 3], q: [EnclosureFamily; 4]| -> Result<(), AnimationError> {
                for i in 0..3 {
                    translation[i] = translation[i].max(t[i].selection_error()?);
                }
                let mut raw = Scalar::exact(0.);
                for component in q {
                    raw = raw.add(Scalar::exact(component.selection_error()?))?;
                }
                let normalized = compilation::normalization_uniform_error(raw)?;
                for i in 0..4 {
                    rotation[i] = rotation[i].max(normalized[i]);
                }
                Ok(())
            };
        let family = |source: &RootRigidEnclosure| -> Result<([EnclosureFamily; 3], [EnclosureFamily; 4]), AnimationError> {
            let mut t = [EnclosureFamily::exact(0.); 3];
            let mut q = [EnclosureFamily::exact(0.); 4];
            for i in 0..3 { t[i] = EnclosureFamily::constant(Scalar(source.translation[i][0], source.translation[i][1]))?; }
            for i in 0..4 { q[i] = EnclosureFamily::constant(Scalar(source.rotation[i][0], source.rotation[i][1]))?; }
            Ok((t, q))
        };
        for prefix in &self.prefixes {
            let (t, q) = family(prefix)?;
            accumulate(t, q)?;
        }
        for (index, span) in self.path.spans.iter().enumerate() {
            let (twist, _) = span
                .screw
                .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
            let mut duration = Scalar::exact(span.end()).sub(Scalar::exact(span.start()))?;
            duration.0 = duration.0.max(0.);
            let elapsed = EnclosureFamily::constant(duration)?.mul(EnclosureFamily {
                domain: Scalar(0., 1.),
                width: Scalar::exact(0.),
            })?;
            let (t, q) = increment_components(twist, elapsed)?;
            let (pt, pq) = family(&self.prefixes[index])?;
            let (t, q) = compose_components(t, q, pt, pq)?;
            accumulate(t, q)?;
        }
        Ok((translation, rotation))
    }
}

impl RootScrewEnclosurePath<'_> {
    /// Uniform discrepancy for every stored point in a box, over every fraction
    /// of every prepared screw span. Matches evaluated.rotation * p + translation.
    /// Upstream source-field error and physical frame/controller operations differ.
    pub fn point_selection_error_bounds(
        &self,
        point: [[f64; 2]; 3],
    ) -> Result<([f64; 3], f64), AnimationError> {
        let (translation, rotation) = self.selection_error_bounds()?;
        let mut axes = [0_f64; 3];
        let mut add = |source: &RootRigidEnclosure| -> Result<(), AnimationError> {
            let errors =
                compilation::rigid_point_runtime_error(source, translation, rotation, point)?;
            for i in 0..3 {
                axes[i] = axes[i].max(errors[i]);
            }
            Ok(())
        };
        for prefix in &self.prefixes {
            add(prefix)?;
        }
        for i in 0..self.path.spans.len() {
            add(&self.span_fraction_enclosure(i, [0., 1.])?)?;
        }
        let mut radius = Scalar::exact(0.);
        for axis in axes {
            radius = radius.add(Scalar::exact(axis))?;
        }
        Ok((axes, radius.1))
    }

    /// Uniform canonical-to-published point-box error for the whole prepared path.
    /// Matches local evaluated pose, then fixed frame rotation, signed scale,
    /// offset and direct f32 cast. Does not cover composed scene matrices,
    /// grounding, runtime phase arithmetic or upstream field approximation.
    pub fn mapped_f32_point_selection_error_bounds(
        &self,
        point: [[f64; 2]; 3],
        source_frame: &RootRigidEnclosure,
        source_scale: RootUniformScaleEnclosure,
        actual_frame: RootRigidTransform,
        actual_scale: f64,
    ) -> Result<([f64; 3], f64), AnimationError> {
        let (local_error, _) = self.point_selection_error_bounds(point)?;
        let source_point = self.whole_path_point_box_bounds(point)?;
        let (before_cast, actual_box) = compilation::mapped_point_runtime_error(
            source_point,
            local_error,
            source_frame,
            source_scale,
            actual_frame,
            actual_scale,
        )?;
        let (publication, _) = RootRigidEnclosure::enclosed_f32_publication_error(actual_box)?;
        let mut axes = [0.; 3];
        let mut radius = Scalar::exact(0.);
        for i in 0..3 {
            axes[i] = Scalar::exact(before_cast[i])
                .add(Scalar::exact(publication[i]))?
                .1;
            radius = radius.add(Scalar::exact(axes[i]))?;
        }
        Ok((axes, radius.1))
    }
}

impl RootScrewEnclosurePath<'_> {
    /// Uniform quaternion component error for the physical controller's stored
    /// basis conjugation and actor orientation update. Matches cross-form basis
    /// rotation of the imaginary part, normalization, then
    /// (orientation * reframed).normalize(). The reference normalizes the same
    /// fixed stored basis/actor inputs exactly. Translation, edges, grounding
    /// and f32 publication are separate obligations.
    pub fn physical_rotation_selection_error_bounds(
        &self,
        basis: DQuat,
        orientation: DQuat,
    ) -> Result<[f64; 4], AnimationError> {
        let (_, selection) = self.selection_error_bounds()?;
        let mut result = [0_f64; 4];
        let mut add = |source: &RootRigidEnclosure| -> Result<(), AnimationError> {
            let error = compilation::physical_rotation_runtime_error(
                source.rotation_bounds(),
                selection,
                basis,
                orientation,
            )?;
            for i in 0..4 {
                result[i] = result[i].max(error[i]);
            }
            Ok(())
        };
        for prefix in &self.prefixes {
            add(prefix)?;
        }
        for i in 0..self.path.spans.len() {
            add(&self.span_fraction_enclosure(i, [0., 1.])?)?;
        }
        Ok(result)
    }
}

impl RootScrewEnclosurePath<'_> {
    /// Uniform whole-body error for the actual pre-grounding physical controller:
    /// basis/pivot displacement, actor orientation update, separate rest-edge
    /// rotations and affine corner sums. Stored frame inputs are interpreted by
    /// exact real normalization; all path fractions/material points are covered.
    /// Grounding, relocation, phase arithmetic and f32 scene matrices are separate.
    pub fn physical_body_selection_error_bounds(
        &self,
        center: DVec3,
        edges: [DVec3; 3],
        basis: DQuat,
        origin: DVec3,
        orientation: DQuat,
        scale: f64,
    ) -> Result<([f64; 3], f64), AnimationError> {
        let (translation, rotation) = self.selection_error_bounds()?;
        let mut result = [0_f64; 3];
        let mut add = |source: &RootRigidEnclosure| -> Result<(), AnimationError> {
            let error = compilation::physical_body_runtime_error(
                source,
                translation,
                rotation,
                center,
                edges,
                basis,
                origin,
                orientation,
                scale,
            )?;
            for i in 0..3 {
                result[i] = result[i].max(error[i]);
            }
            Ok(())
        };
        for prefix in &self.prefixes {
            add(prefix)?;
        }
        for i in 0..self.path.spans.len() {
            add(&self.span_fraction_enclosure(i, [0., 1.])?)?;
        }
        let mut radius = Scalar::exact(0.);
        for error in result {
            radius = radius.add(Scalar::exact(error))?;
        }
        Ok((result, radius.1))
    }
}

impl RootScrewEnclosurePath<'_> {
    /// Uniform whole-body numeric error after the physical rotation/displacement
    /// chain, any selected snap fraction in [0,1], and the current zero-anchor
    /// min/max center reconstruction. Reference is the canonical affine body
    /// translated by that exact stored snap fraction; the physical snap itself
    /// is not counted as numerical error. Does not qualify fraction selection,
    /// runtime phase arithmetic or composed f32 scene publication.
    #[allow(clippy::too_many_arguments)]
    pub fn physical_post_snap_selection_error_bounds(
        &self,
        center: DVec3,
        edges: [DVec3; 3],
        basis: DQuat,
        origin: DVec3,
        orientation: DQuat,
        scale: f64,
        snap: DVec3,
    ) -> Result<([f64; 3], f64), AnimationError> {
        let (translation, rotation) = self.selection_error_bounds()?;
        let mut result = [0_f64; 3];
        let mut add = |source: &RootRigidEnclosure| -> Result<(), AnimationError> {
            let (body, center_domain, half_domain) = compilation::physical_body_runtime_domains(
                source,
                translation,
                rotation,
                center,
                edges,
                basis,
                origin,
                orientation,
                scale,
            )?;
            let correction =
                compilation::snap_center_reconstruction_error(center_domain, half_domain, snap)?;
            for i in 0..3 {
                result[i] =
                    result[i].max(Scalar::exact(body[i]).add(Scalar::exact(correction[i]))?.1);
            }
            Ok(())
        };
        for prefix in &self.prefixes {
            add(prefix)?;
        }
        for i in 0..self.path.spans.len() {
            add(&self.span_fraction_enclosure(i, [0., 1.])?)?;
        }
        let mut radius = Scalar::exact(0.);
        for error in result {
            radius = radius.add(Scalar::exact(error))?;
        }
        Ok((result, radius.1))
    }
}
