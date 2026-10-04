//! Composed root motion: translation and rotation share one authored interval.
use super::{
    AnimationError, MAX_ROOT_ROTATION_SPANS, Playback, RootRotationCurve, RootRotationSpan,
};
use crate::root_curve::RootCurve;
use glam::{DQuat, DVec3, Vec3};
use std::sync::Arc;
mod enclosure;
mod integration;
pub use enclosure::{
    RootAngularDerivativeBounds, RootRigidCertifiedFadeInterval, RootRigidCoordinateCertificate,
    RootRigidEnclosure, RootRigidErrorAccumulator, RootRigidFadeDomain, RootRigidFadeFieldInterval,
    RootRigidFieldInterval, RootRigidMappedField, RootRigidMappedPath, RootRigidTwistEnclosure,
    RootRigidWallInterval, RootRigidWallPartition, RootScrewEnclosurePath,
    RootSourceScrewPointCertificate, RootTimeCutEnclosure, RootTwistErrorBounds,
    RootUniformScaleEnclosure,
};
pub(crate) use enclosure::{
    quaternion_composition_evaluation_error_bounds, quaternion_composition_uniform_error,
    quaternion_cubic_control_error_bounds, quaternion_cubic_interval_evaluation_error_bounds,
    quaternion_cubic_normalized_error_bounds, quaternion_cubic_phase_evaluation_error_bounds,
    quaternion_cubic_restriction_error_bounds, quaternion_cubic_source_speed_bound,
    quaternion_normalization_error_bounds, quaternion_normalized_composition_uniform_error,
    translation_coefficient_error_bounds, translation_interval_evaluation_error_bounds,
    translation_phase_evaluation_error_bounds, translation_piece_error_bounds,
    translation_source_position_bounds, translation_source_velocity_bounds,
};
mod blend;
pub use blend::RootSpatialTwistBounds;
mod partition;
pub use integration::{RootRigidApproximation, RootRigidIntegrationDomain, RootTwistRateBounds};
pub use partition::{RootMotionInterval, RootMotionPartition, RootMotionStep};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RootRigidTransform {
    pub translation: DVec3,
    pub rotation: DQuat,
}
impl RootRigidTransform {
    pub const IDENTITY: Self = Self {
        translation: DVec3::ZERO,
        rotation: DQuat::IDENTITY,
    };
    fn checked(self) -> Result<Self, AnimationError> {
        if !self.translation.is_finite()
            || !self.rotation.is_finite()
            || !self.rotation.is_normalized()
        {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(self)
    }
    /// Right composition: `a.compose(b)` applies b in a's rotated frame.
    /// # Errors
    /// Rejects invalid transforms or overflowing translation.
    pub fn compose(self, other: Self) -> Result<Self, AnimationError> {
        self.checked()?;
        other.checked()?;
        Self {
            translation: self.translation + self.rotation * other.translation,
            rotation: (self.rotation * other.rotation).normalize(),
        }
        .checked()
    }
    /// # Errors
    /// Rejects invalid transforms or overflowing translation.
    pub fn inverse(self) -> Result<Self, AnimationError> {
        self.checked()?;
        let rotation = self.rotation.conjugate();
        Self {
            translation: rotation * -self.translation,
            rotation,
        }
        .checked()
    }
    /// # Errors
    /// Rejects invalid transforms, points or numerical overflow.
    pub fn transform_point(self, point: DVec3) -> Result<DVec3, AnimationError> {
        self.checked()?;
        let result = self.translation + self.rotation * point;
        if !point.is_finite() || !result.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(result)
    }
}
fn power(
    mut value: RootRigidTransform,
    mut cycles: u64,
) -> Result<RootRigidTransform, AnimationError> {
    let mut result = RootRigidTransform::IDENTITY;
    while cycles > 0 {
        if cycles & 1 != 0 {
            result = result.compose(value)?;
        }
        cycles >>= 1;
        if cycles > 0 {
            value = value.compose(value)?;
        }
    }
    Ok(result)
}
#[derive(Debug)]
struct Curve {
    translation: Arc<RootCurve>,
    rotation: RootRotationCurve,
    origin: DVec3,
    bind: DVec3,
    duration: f64,
    playback: Playback,
}
/// Immutable selected-joint coefficients shared by clips and owners. Masks do
/// not create additional compiled key arrays; this object contains no clock.
#[derive(Clone, Debug)]
pub struct RootRigidCurve(Arc<Curve>);
/// Origin linear velocity and angular velocity in the interval-start frame,
/// measured per clip second. This is not a body-coordinate twist.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RootRigidVelocity {
    pub linear: DVec3,
    pub angular: DVec3,
}
/// Spatial twist: t_dot = angular cross t + linear, in one fixed frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RootRigidTwist {
    pub linear: DVec3,
    pub angular: DVec3,
}
impl RootRigidTwist {
    /// Exact constant-twist increment. Left-compose it with the starting transform.
    /// # Errors
    /// Rejects invalid duration, nonfinite twist or numerical overflow.
    pub fn increment(self, duration: f64) -> Result<RootRigidTransform, AnimationError> {
        if !duration.is_finite() || duration < 0. {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        if !self.linear.is_finite() || !self.angular.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let displacement = self.linear * duration;
        let scaled = self.angular * duration;
        if !displacement.is_finite() || !scaled.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let maximum = scaled.abs().max_element();
        if maximum == 0. {
            return RootRigidTransform {
                translation: displacement,
                rotation: DQuat::IDENTITY,
            }
            .checked();
        }
        let norm = (scaled / maximum).length();
        let angle = maximum * norm;
        if !angle.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let axis = (scaled / maximum) / norm;
        let (a, b) = if angle < 1e-3 {
            let x = angle * angle;
            (
                angle * (0.5 - x / 24. + x * x / 720. - x * x * x / 40320.),
                x * (1. / 6. - x / 120. + x * x / 5040. - x * x * x / 362880.),
            )
        } else {
            ((1. - angle.cos()) / angle, 1. - angle.sin() / angle)
        };
        let cross = axis.cross(displacement);
        RootRigidTransform {
            translation: displacement + a * cross + b * axis.cross(cross),
            rotation: DQuat::from_axis_angle(axis, angle).normalize(),
        }
        .checked()
    }
}
impl RootRigidVelocity {
    /// Converts origin derivative to a spatial twist at the supplied transform.
    /// # Errors
    /// Rejects invalid transform/velocity or overflowing origin conversion.
    pub fn spatial_twist(
        self,
        transform: RootRigidTransform,
    ) -> Result<RootRigidTwist, AnimationError> {
        transform.checked()?;
        if !self.linear.is_finite() || !self.angular.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let linear = self.linear - self.angular.cross(transform.translation);
        if !linear.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(RootRigidTwist {
            linear,
            angular: self.angular,
        })
    }
    /// Transports velocity through x_target = scale * basis * x_source + offset.
    /// `rotation` is the source path rotation at this velocity's sampling time.
    /// The shifted target origin contributes velocity even when scale is zero.
    /// # Errors
    /// Rejects invalid coordinates, source velocity and numerical overflow.
    pub fn transformed(
        self,
        rotation: DQuat,
        basis: DQuat,
        scale: f64,
        offset: DVec3,
    ) -> Result<Self, AnimationError> {
        if !basis.is_finite()
            || !basis.is_normalized()
            || !rotation.is_finite()
            || !rotation.is_normalized()
            || !scale.is_finite()
            || scale < 0.
            || !offset.is_finite()
        {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        if !self.linear.is_finite() || !self.angular.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let angular = basis * self.angular;
        let target_rotation = (basis * rotation * basis.conjugate()).normalize();
        let linear = scale * (basis * self.linear) - angular.cross(target_rotation * offset);
        if !linear.is_finite() || !angular.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(Self { linear, angular })
    }
}
#[derive(Clone, Debug)]
pub struct RootRigidSpan {
    screw: Option<(RootRigidTwist, RootRigidTransform)>,
    rotation: RootRotationSpan,
    additive: [DVec3; 4],
    pivot: [DVec3; 4],
}
impl RootRigidSpan {
    #[must_use]
    pub fn start(&self) -> f64 {
        self.rotation.start()
    }
    #[must_use]
    pub fn end(&self) -> f64 {
        self.rotation.end()
    }
    #[must_use]
    pub fn is_step(&self) -> bool {
        self.start() == self.end()
    }
    #[must_use]
    pub fn rotation(&self) -> &RootRotationSpan {
        &self.rotation
    }
    /// Samples the actual curve. STEP bridges its simultaneous translation and
    /// shortest rotation event; replacing a continuous span by endpoint poses loses it.
    /// # Errors
    /// Rejects invalid fractions or numerical overflow.
    pub fn sample(&self, fraction: f64) -> Result<RootRigidTransform, AnimationError> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(AnimationError::InvalidSampleTime);
        }
        if let Some((twist, initial)) = self.screw {
            return twist
                .increment((self.end() - self.start()) * fraction)?
                .compose(initial);
        }
        let rotation = self.rotation.sample(fraction)?;
        RootRigidTransform {
            translation: bezier(self.additive, fraction) - rotation * bezier(self.pivot, fraction),
            rotation,
        }
        .checked()
    }
    /// Analytic velocity of the simultaneous translation/rotation curve.
    /// STEP events have no finite clip-time derivative and return None.
    /// # Errors
    /// Rejects invalid fractions, singular rotation curves or numerical overflow.
    pub fn velocity(&self, fraction: f64) -> Result<Option<RootRigidVelocity>, AnimationError> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(AnimationError::InvalidSampleTime);
        }
        if self.is_step() {
            return Ok(None);
        }
        if let Some((twist, _)) = self.screw {
            let transform = self.sample(fraction)?;
            let linear = twist.angular.cross(transform.translation) + twist.linear;
            if !linear.is_finite() {
                return Err(AnimationError::NumericalOverflow);
            }
            return Ok(Some(RootRigidVelocity {
                linear,
                angular: twist.angular,
            }));
        }
        let Some(angular) = self.rotation.angular_velocity(fraction)? else {
            return Ok(None);
        };
        let rotation = self.rotation.sample(fraction)?;
        let duration = self.end() - self.start();
        let derivative = |control: [DVec3; 4]| {
            let a = 3. * (control[1] - control[0]);
            let b = 3. * (control[2] - control[1]);
            let c = 3. * (control[3] - control[2]);
            a.lerp(b, fraction).lerp(b.lerp(c, fraction), fraction) / duration
        };
        let pivot = rotation * bezier(self.pivot, fraction);
        let linear =
            derivative(self.additive) - angular.cross(pivot) - rotation * derivative(self.pivot);
        if !linear.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(Some(RootRigidVelocity { linear, angular }))
    }
    /// Within-span derivative bounds for the spatial twist, per clip second.
    /// Key boundaries and STEP events must be partitioned before integration.
    /// # Errors
    /// Rejects unproved rotation bounds or overflowing coefficient derivatives.
    pub fn twist_rate_bounds(&self) -> Result<Option<RootTwistRateBounds>, AnimationError> {
        if self.is_step() {
            return Ok(None);
        }
        if self.screw.is_some() {
            return Ok(Some(RootTwistRateBounds {
                linear: 0.,
                angular: 0.,
            }));
        }
        let Some(angular) = self.rotation.angular_acceleration_bound()? else {
            return Ok(None);
        };
        let omega = self
            .rotation
            .angular_speed_bound()
            .ok_or(AnimationError::RootRotationBudget)?;
        let dt = self.end() - self.start();
        let position = self
            .additive
            .iter()
            .map(|v| v.length())
            .fold(0_f64, f64::max);
        let first = |c: [DVec3; 4]| {
            c.windows(2)
                .map(|p| 3. * (p[1] - p[0]).length())
                .fold(0_f64, f64::max)
                / dt
        };
        let second = |c: [DVec3; 4]| {
            c.windows(3)
                .map(|p| 6. * (p[2] - 2. * p[1] + p[0]).length())
                .fold(0_f64, f64::max)
                / (dt * dt)
        };
        // Spatial linear twist is a_dot - omega cross a - R*p_dot;
        // its derivative retains normalization, moving-pivot and cross terms.
        let linear = second(self.additive)
            + angular * position
            + omega * (first(self.additive) + first(self.pivot))
            + second(self.pivot);
        let control_norm = self
            .additive
            .iter()
            .chain(&self.pivot)
            .map(|v| v.length())
            .fold(0_f64, f64::max);
        let guarded = linear + 4096. * f64::EPSILON * (linear + control_norm / (dt * dt));
        if !guarded.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(Some(RootTwistRateBounds {
            linear: guarded,
            angular,
        }))
    }
    /// Encloses n dot (sample(u) * point) over the whole span. The position
    /// polynomial and rotated pivot retain matching Bernstein weights. Bounds
    /// combine proven constant-point rotation hulls, never sampled extrema.
    /// # Errors
    /// Rejects invalid points/normals or overflowing bounds.
    pub fn projection_bounds(
        &self,
        point: DVec3,
        normal: DVec3,
    ) -> Result<[f64; 2], AnimationError> {
        if self.screw.is_some() {
            if !point.is_finite() || !normal.is_finite() {
                return Err(AnimationError::NumericalOverflow);
            }
            let center = normal.dot(self.sample(0.5)?.transform_point(point)?);
            let radius = normal.length() * self.point_speed_bound(point)? * 0.5;
            let guard = 4096. * f64::EPSILON * (center.abs() + radius);
            let bounds = [center - radius - guard, center + radius + guard];
            if !bounds.into_iter().all(f64::is_finite) {
                return Err(AnimationError::NumericalOverflow);
            }
            return Ok(bounds);
        }
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for i in 0..4 {
            let bounds = self
                .rotation
                .projection_bounds(point - self.pivot[i], normal)?;
            let additive = normal.dot(self.additive[i]);
            low = low.min(additive + bounds[0]);
            high = high.max(additive + bounds[1]);
        }
        let guard = 4096.
            * f64::EPSILON
            * (low.abs().max(high.abs())
                + normal.length()
                    * (point.length()
                        + self
                            .additive
                            .iter()
                            .chain(&self.pivot)
                            .map(|v| v.length())
                            .fold(0_f64, f64::max)));
        let result = [low - guard, high + guard];
        if !result.into_iter().all(f64::is_finite) {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(result)
    }
    /// Speed per normalized span fraction, including simultaneous STEP bridges.
    /// # Errors
    /// Rejects invalid points or overflowing derivative bounds.
    pub fn point_speed_bound(&self, point: DVec3) -> Result<f64, AnimationError> {
        if let Some((twist, initial)) = self.screw {
            let position = initial.transform_point(point)?;
            let speed = (twist.angular.cross(position) + twist.linear).length()
                * (self.end() - self.start());
            let guarded = speed * (1. + 4096. * f64::EPSILON);
            if !guarded.is_finite() {
                return Err(AnimationError::NumericalOverflow);
            }
            return Ok(guarded);
        }
        let mut angular = 0_f64;
        for pivot in self.pivot {
            let vector = point - pivot;
            let bound = self.rotation.point_speed_bound(vector)?;
            let speed = if let Some(speed) = bound {
                speed * (self.end() - self.start())
            } else {
                self.rotation
                    .body_angular_displacement()
                    .unwrap_or_default()
                    .cross(vector)
                    .length()
            };
            angular = angular.max(speed);
        }
        let derivative = |c: [DVec3; 4]| {
            c.windows(2)
                .map(|pair| 3. * (pair[1] - pair[0]).length())
                .fold(0_f64, f64::max)
        };
        let result = angular + derivative(self.additive) + derivative(self.pivot);
        if !result.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(result)
    }
}
fn bezier(c: [DVec3; 4], t: f64) -> DVec3 {
    let a = c[0].lerp(c[1], t);
    let b = c[1].lerp(c[2], t);
    let d = c[2].lerp(c[3], t);
    a.lerp(b, t).lerp(b.lerp(d, t), t)
}
#[derive(Clone, Debug)]
pub struct RootRigidPath {
    translation_cuts: Vec<RootTimeCutEnclosure>,
    spans: Vec<RootRigidSpan>,
    duration: f64,
    end: RootRigidTransform,
}
impl RootRigidPath {
    /// Enclosed translation-key and loop cut clocks. Not a total motion error certificate.
    pub fn translation_cut_enclosures(&self) -> &[RootTimeCutEnclosure] {
        &self.translation_cuts
    }

    /// Builds ordered, exact constant-spatial-twist segments in one fixed frame.
    /// This does not approximate or certify a time-varying velocity field.
    /// # Errors
    /// Rejects invalid/zero durations, span capacity and numerical overflow.
    pub fn from_twists(
        segments: &[(RootRigidTwist, f64)],
        max_spans: usize,
    ) -> Result<Self, AnimationError> {
        if segments.len() > max_spans.min(MAX_ROOT_ROTATION_SPANS) {
            return Err(AnimationError::RootRigidBudget);
        }
        let mut spans = Vec::with_capacity(segments.len());
        let mut end = RootRigidTransform::IDENTITY;
        let mut time = 0.;
        for &(twist, duration) in segments {
            if !duration.is_finite() || duration <= 0. {
                return Err(AnimationError::InvalidAnimationTimeStep);
            }
            let next = time + duration;
            if !next.is_finite() || next <= time {
                return Err(AnimationError::NumericalOverflow);
            }
            let rotation =
                RootRotationSpan::constant_velocity(time, next, end.rotation, twist.angular)?;
            let span = RootRigidSpan {
                screw: Some((twist, end)),
                rotation,
                additive: [DVec3::ZERO; 4],
                pivot: [DVec3::ZERO; 4],
            };
            end = span.sample(1.)?;
            spans.push(span);
            time = next;
        }
        Ok(Self {
            spans,
            duration: time,
            end,
            translation_cuts: vec![],
        })
    }
    /// Changes total elapsed time while retaining every geometric span/event.
    /// Moving spans cannot collapse to zero duration; stationary paths can acquire
    /// a wall duration without inventing velocity or motion.
    pub fn retimed(&self, duration: f64) -> Result<Self, AnimationError> {
        if !duration.is_finite() || duration < 0. {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let mut spans = Vec::with_capacity(self.spans.len());
        for original in &self.spans {
            let start = (original.start() / self.duration) * duration;
            let end = (original.end() / self.duration) * duration;
            let rotation = original.rotation.with_times(start, end)?;
            let screw = match original.screw {
                Some((twist, initial)) => Some((
                    twist.retimed((original.end() - original.start()) / (end - start))?,
                    initial,
                )),
                None => None,
            };
            spans.push(RootRigidSpan {
                rotation,
                screw,
                additive: original.additive,
                pivot: original.pivot,
            });
        }
        Ok(Self {
            spans,
            duration,
            end: self.end,
            translation_cuts: self
                .translation_cuts
                .iter()
                .map(|cut| cut.retimed(self.duration, duration))
                .collect::<Result<_, _>>()?,
        })
    }
    /// Appends a path expressed in the same fixed spatial frame.
    /// Each following sample left-composes this path's accepted endpoint.
    /// # Errors
    /// Rejects aggregate span capacity, collapsed shifted times or overflow.
    pub fn append_spatial(&self, next: &Self, max_spans: usize) -> Result<Self, AnimationError> {
        let count = self
            .spans
            .len()
            .checked_add(next.spans.len())
            .ok_or(AnimationError::RootRigidBudget)?;
        if count > max_spans.min(MAX_ROOT_ROTATION_SPANS) {
            return Err(AnimationError::RootRigidBudget);
        }
        let duration = self.duration + next.duration;
        if !duration.is_finite() || (next.duration > 0. && duration <= self.duration) {
            return Err(AnimationError::NumericalOverflow);
        }
        let prefix = self.end;
        let mut spans = self.spans.clone();
        for original in &next.spans {
            let rotation = original
                .rotation
                .shifted_and_postcomposed(self.duration, prefix.rotation)?;
            let screw = if let Some((twist, initial)) = original.screw {
                let rate =
                    (original.end() - original.start()) / (rotation.end() - rotation.start());
                Some((twist.retimed(rate)?, initial.compose(prefix)?))
            } else {
                None
            };
            let pivot = original
                .pivot
                .map(|value| prefix.rotation.conjugate() * (value - prefix.translation));
            if !pivot.iter().all(|v| v.is_finite()) {
                return Err(AnimationError::NumericalOverflow);
            }
            spans.push(RootRigidSpan {
                rotation,
                screw,
                pivot,
                additive: original.additive,
            });
        }
        let mut translation_cuts = self.translation_cuts.clone();
        translation_cuts.extend(
            next.translation_cuts
                .iter()
                .map(|cut| cut.shifted(self.duration))
                .collect::<Result<Vec<_>, _>>()?,
        );
        Ok(Self {
            spans,
            duration,
            end: next.end.compose(prefix)?,
            translation_cuts,
        })
    }
    /// Changes coordinates by x_target = scale * basis * x_source + offset.
    /// Retains every ordered span, cubic coefficient and STEP event.
    /// # Errors
    /// Rejects nonunit/nonfinite bases, negative/nonfinite scales and overflow.
    pub fn transformed(
        &self,
        basis: DQuat,
        scale: f64,
        offset: DVec3,
    ) -> Result<Self, AnimationError> {
        if !basis.is_finite()
            || !basis.is_normalized()
            || !scale.is_finite()
            || scale < 0.
            || !offset.is_finite()
        {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        let map = |value: DVec3| -> Result<DVec3, AnimationError> {
            let result = scale * (basis * value) + offset;
            if !result.is_finite() {
                return Err(AnimationError::NumericalOverflow);
            }
            Ok(result)
        };
        let mut spans = Vec::with_capacity(self.spans.len());
        for span in &self.spans {
            let mut additive = [DVec3::ZERO; 4];
            let mut pivot = [DVec3::ZERO; 4];
            for i in 0..4 {
                additive[i] = map(span.additive[i])?;
                pivot[i] = map(span.pivot[i])?;
            }
            let screw = if let Some((twist, initial)) = span.screw {
                let twist = twist.transformed(basis, scale, offset)?;
                let rotation = (basis * initial.rotation * basis.conjugate()).normalize();
                let initial = RootRigidTransform {
                    rotation,
                    translation: scale * (basis * initial.translation) + offset - rotation * offset,
                }
                .checked()?;
                twist.increment(0.)?;
                Some((twist, initial))
            } else {
                None
            };
            spans.push(RootRigidSpan {
                screw,
                rotation: span.rotation.conjugated(basis),
                additive,
                pivot,
            });
        }
        let rotation = (basis * self.end.rotation * basis.conjugate()).normalize();
        let end = RootRigidTransform {
            translation: scale * (basis * self.end.translation) + offset - rotation * offset,
            rotation,
        }
        .checked()?;
        Ok(Self {
            spans,
            duration: self.duration,
            translation_cuts: self.translation_cuts.clone(),
            end,
        })
    }

    #[must_use]
    pub fn spans(&self) -> &[RootRigidSpan] {
        &self.spans
    }
    #[must_use]
    pub fn duration(&self) -> f64 {
        self.duration
    }
    #[must_use]
    pub fn end_transform(&self) -> RootRigidTransform {
        self.end
    }
    #[must_use]
    pub fn angular_travel_bound(&self) -> f64 {
        self.spans
            .iter()
            .map(|span| {
                span.rotation.angular_speed_bound().map_or_else(
                    || {
                        span.rotation
                            .body_angular_displacement()
                            .map_or(0., |v| v.length())
                    },
                    |speed| speed * (span.end() - span.start()),
                )
            })
            .sum()
    }
}
impl RootRigidCurve {
    /// Uniform position discrepancy of the stored translation polynomial from
    /// exact stored-key coefficient compilation, at equal normalized parameter.
    /// Parameter evaluation, Bernstein conversion, rotation and loop powers are
    /// separate proof obligations.
    pub fn translation_compilation_error_bounds(&self) -> Result<[f64; 3], AnimationError> {
        self.0.translation.compilation_error_bounds()
    }
    /// Exact stored-key translation velocity on one key interval, before rotation
    /// coupling or extraction masks. STEP impulses are separate from this derivative.
    pub fn translation_source_velocity_bounds(
        &self,
        start: f64,
        end: f64,
    ) -> Result<[[f64; 2]; 3], AnimationError> {
        self.0.translation.source_velocity_bounds(start, end)
    }
    pub fn rotation_cubic_cycle_interval_evaluation_error_bounds(
        &self,
        cycles: u64,
        start: f64,
        end: f64,
    ) -> Result<Option<[f64; 4]>, AnimationError> {
        self.0
            .rotation
            .cubic_cycle_interval_evaluation_error_bounds(cycles, start, end)
    }
    pub fn rotation_cubic_relative_interval_evaluation_error_bounds(
        &self,
        start: f64,
        end: f64,
    ) -> Result<Option<[f64; 4]>, AnimationError> {
        self.0
            .rotation
            .cubic_relative_interval_evaluation_error_bounds(start, end)
    }
    pub fn rotation_cubic_source_angular_speed_bound(
        &self,
        start: f64,
        end: f64,
    ) -> Result<Option<f64>, AnimationError> {
        self.0.rotation.cubic_source_angular_speed_bound(start, end)
    }
    /// Componentwise error of cached quaternion key normalization only.
    /// Arc logarithms, cubic controls, interval restriction and cycle composition
    /// require additional compilation proofs.
    pub fn rotation_key_normalization_error_bounds(&self) -> [f64; 4] {
        self.0.rotation.key_normalization_error_bounds()
    }
    /// Componentwise unnormalized cubic control compilation discrepancy.
    /// None indicates a non-cubic rotation channel.
    pub fn rotation_cubic_control_compilation_error_bounds(&self) -> Option<[f64; 4]> {
        self.0.rotation.cubic_control_compilation_error_bounds()
    }
    /// Whole-channel normalized cubic compilation discrepancy, requiring a
    /// positive norm proof. Does not cover rounded runtime pose evaluation.
    pub fn rotation_cubic_normalized_compilation_error_bounds(
        &self,
    ) -> Result<Option<[f64; 4]>, AnimationError> {
        self.0.rotation.cubic_normalized_compilation_error_bounds()
    }
    /// Normalized cubic source/cached discrepancy through interval restriction.
    /// Rejects crossing rotation keys; relative frames and loops are separate.
    pub fn rotation_cubic_piece_compilation_error_bounds(
        &self,
        start: f64,
        end: f64,
    ) -> Result<Option<[f64; 4]>, AnimationError> {
        self.0
            .rotation
            .cubic_piece_compilation_error_bounds(start, end)
    }
    /// Exact supplied local phase versus the actually evaluated cubic quaternion.
    /// This pointwise proof does not include phase selection or relative frames.
    pub fn rotation_cubic_phase_evaluation_error_bounds(
        &self,
        phase: f64,
    ) -> Result<Option<[f64; 4]>, AnimationError> {
        self.0.rotation.cubic_phase_evaluation_error_bounds(phase)
    }
    /// Pointwise exact source translation versus actual local Horner evaluation.
    /// The supplied local phase is exact; extraction/loops remain separate.
    pub fn translation_phase_evaluation_error_bounds(
        &self,
        phase: f64,
    ) -> Result<[f64; 3], AnimationError> {
        self.0.translation.phase_evaluation_error_bounds(phase)
    }
    /// Uniform source-to-runtime translation error within one key interval.
    /// Covers parameter and Horner rounding; source phase/extraction are separate.
    pub fn translation_interval_evaluation_error_bounds(
        &self,
        start: f64,
        end: f64,
    ) -> Result<[f64; 3], AnimationError> {
        self.0
            .translation
            .interval_evaluation_error_bounds(start, end)
    }
    /// Uniform source-to-runtime cubic quaternion error within one key interval.
    /// Includes scaled normalization rounding; relative frames/loops are separate.
    pub fn rotation_cubic_interval_evaluation_error_bounds(
        &self,
        start: f64,
        end: f64,
    ) -> Result<Option<[f64; 4]>, AnimationError> {
        self.0
            .rotation
            .cubic_interval_evaluation_error_bounds(start, end)
    }
    /// Pointwise relative cubic quaternion proof, including origin composition.
    pub fn rotation_cubic_relative_phase_evaluation_error_bounds(
        &self,
        phase: f64,
    ) -> Result<Option<[f64; 4]>, AnimationError> {
        self.0
            .rotation
            .cubic_relative_phase_evaluation_error_bounds(phase)
    }
    /// Cubic rotation error through explicit cycle powers and local phase.
    /// Wall-time cycle/phase mapping is not included.
    pub fn rotation_cubic_cycle_phase_evaluation_error_bounds(
        &self,
        cycles: u64,
        phase: f64,
    ) -> Result<Option<[f64; 4]>, AnimationError> {
        self.0
            .rotation
            .cubic_cycle_phase_evaluation_error_bounds(cycles, phase)
    }
    /// Position discrepancy from exact stored-key compilation through one
    /// restricted Bernstein piece. Rejects crossing a translation key boundary.
    /// Rotation/extraction and cycle composition are not included.
    pub fn translation_piece_compilation_error_bounds(
        &self,
        start: f64,
        end: f64,
    ) -> Result<[f64; 3], AnimationError> {
        self.0.translation.piece_error_bounds(start, end)
    }

    pub(super) fn new(
        translation: Arc<RootCurve>,
        rotation: RootRotationCurve,
        origin: Vec3,
        bind: Vec3,
        duration: f32,
        playback: Playback,
    ) -> Result<Self, AnimationError> {
        Ok(Self(Arc::new(Curve {
            translation,
            rotation,
            origin: origin.as_dvec3(),
            bind: bind.as_dvec3(),
            duration: f64::from(duration),
            playback,
        })))
    }
    fn constant(&self, axes: [bool; 3]) -> bool {
        self.0.rotation.is_constant() && !self.0.translation.has_motion(axes)
    }
    fn pivot(&self, position: DVec3, axes: [bool; 3]) -> DVec3 {
        DVec3::from_array(std::array::from_fn(|i| {
            if axes[i] { self.0.bind[i] } else { position[i] }
        }))
    }
    fn adjusted_position(&self, position: DVec3, axes: [bool; 3]) -> DVec3 {
        position
            - DVec3::from_array(std::array::from_fn(|i| {
                if axes[i] {
                    self.0.origin[i] - self.0.bind[i]
                } else {
                    0.
                }
            }))
    }
    fn phase(&self, time: f64, axes: [bool; 3]) -> Result<RootRigidTransform, AnimationError> {
        let rotation = self.0.rotation.phase_rotation(time)?;
        let position = self.0.origin + self.0.translation.position(time);
        RootRigidTransform {
            rotation,
            translation: self.adjusted_position(position, axes)
                - rotation * self.pivot(position, axes),
        }
        .checked()
    }
    fn cycle_phase(&self, time: f64) -> Result<(u64, f64), AnimationError> {
        let proof = crate::enclose_root_cycle_phase(time, self.0.duration as f32, self.0.playback)
            .map_err(|error| {
                if error == AnimationError::RootRigidBudget {
                    AnimationError::RootRigidBudget
                } else {
                    error
                }
            })?;
        Ok((proof.cycle(), proof.phase()))
    }
    /// Unwrapped composed transform relative to the first authored root factor.
    /// Each cycle transports the following cycle's displacement through its turn.
    /// Unselected translation axes remain in the displayed pose and become a
    /// moving rotation pivot; selected axes are extracted into the actor motion.
    /// # Errors
    /// Rejects invalid clocks, unrepresentable cycle indices or numerical overflow.
    pub fn sample(&self, time: f64, axes: [bool; 3]) -> Result<RootRigidTransform, AnimationError> {
        if !time.is_finite() || time < 0. {
            return Err(AnimationError::InvalidSampleTime);
        }
        if self.constant(axes) {
            return Ok(RootRigidTransform::IDENTITY);
        }
        let (cycles, phase) = self.cycle_phase(time)?;
        let prefix = if cycles == 0 {
            RootRigidTransform::IDENTITY
        } else {
            power(self.phase(self.0.duration, axes)?, cycles)?
        };
        prefix.compose(self.phase(phase, axes)?)
    }
    fn span(
        &self,
        rotation: RootRotationSpan,
        positions: [DVec3; 4],
        prefix: RootRigidTransform,
        axes: [bool; 3],
    ) -> RootRigidSpan {
        RootRigidSpan {
            screw: None,
            rotation,
            additive: positions
                .map(|p| prefix.translation + prefix.rotation * self.adjusted_position(p, axes)),
            pivot: positions.map(|p| self.pivot(p, axes)),
        }
    }
    fn prefix(
        &self,
        absolute: f64,
        initial_inverse: RootRigidTransform,
        cycle: RootRigidTransform,
    ) -> Result<(RootRigidTransform, f64), AnimationError> {
        let (n, phase) = self.cycle_phase(absolute)?;
        Ok((initial_inverse.compose(power(cycle, n)?)?, phase))
    }
    /// Complete ordered trajectory with exact polynomial/rotation spans. STEP
    /// events use (start,end], and simultaneous channel jumps form one event.
    /// A common old cycle prefix cancels before interval generation, avoiding
    /// subtraction of huge world displacements to recover a small tick.
    /// # Errors
    /// Invalid time, singular rotations, collapsed key times or span/loop work
    /// exhaustion reject the whole path without an accepted prefix.
    #[allow(clippy::too_many_lines)]
    pub fn path(
        &self,
        start: f64,
        end: f64,
        axes: [bool; 3],
        max_spans: usize,
    ) -> Result<RootRigidPath, AnimationError> {
        if !start.is_finite() || !end.is_finite() || start < 0. || end < start {
            return Err(AnimationError::InvalidSampleTime);
        }
        if !(1..=MAX_ROOT_ROTATION_SPANS).contains(&max_spans) {
            return Err(AnimationError::RootRigidBudget);
        }
        let elapsed = end - start;
        if elapsed == 0. {
            self.sample(start, axes)?;
        }
        if self.constant(axes)
            || elapsed == 0.
            || (self.0.playback == Playback::Clamp && start >= self.0.duration)
        {
            return Ok(RootRigidPath {
                spans: vec![],
                translation_cuts: vec![],
                duration: elapsed,
                end: RootRigidTransform::IDENTITY,
            });
        }
        let (_, start) = self.cycle_phase(start)?;
        let end = start + elapsed;
        if !end.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let rotations = self.0.rotation.path(start, end, max_spans)?;
        let initial = self.phase(start, axes)?.inverse()?;
        let cycle = if self.0.playback == Playback::Loop {
            self.phase(self.0.duration, axes)?
        } else {
            RootRigidTransform::IDENTITY
        };
        let end_transform = self
            .prefix(end, initial, cycle)?
            .0
            .compose(self.phase(self.cycle_phase(end)?.1, axes)?)?;
        let mut translation_cuts = Vec::new();
        let mut cuts = vec![0., elapsed];
        cuts.extend(
            rotations
                .spans()
                .iter()
                .flat_map(|span| [span.start(), span.end()]),
        );
        let loops = if self.0.playback == Playback::Loop {
            usize::try_from(self.cycle_phase(end)?.0)
                .map_err(|_| AnimationError::RootRigidBudget)?
        } else {
            0
        };
        if loops > max_spans {
            return Err(AnimationError::RootRigidBudget);
        }
        for n in 0..=loops {
            let base = n as f64 * self.0.duration;
            let from = (start - base).clamp(0., self.0.duration);
            let to = (end - base).clamp(0., self.0.duration);
            for key in self.0.translation.cuts(from, to, max_spans)? {
                let absolute = base + key;
                if key > 0.
                    && key < self.0.duration
                    && (absolute <= base || absolute >= base + self.0.duration)
                {
                    return Err(AnimationError::RootRigidBudget);
                }
                let proof =
                    RootTimeCutEnclosure::new(n as u64, self.0.duration as f32, key, start)?;
                let relative = proof.evaluated();
                if relative > 0. && relative <= elapsed {
                    cuts.push(relative);
                    translation_cuts.push(proof);
                }
            }
            let proof = RootTimeCutEnclosure::new(
                n as u64,
                self.0.duration as f32,
                self.0.duration,
                start,
            )?;
            let seam = proof.evaluated();
            if seam > 0. && seam < elapsed {
                cuts.push(seam);
                translation_cuts.push(proof);
            }
            if cuts.len() > 4 * max_spans + 4 {
                return Err(AnimationError::RootRigidBudget);
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        let mut spans = Vec::new();
        let mut cursor = 0;
        for pair in cuts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if b <= a {
                return Err(AnimationError::RootRigidBudget);
            }
            while rotations
                .spans()
                .get(cursor)
                .is_some_and(|span| span.is_step() || span.end() <= a)
            {
                cursor += 1;
            }
            let rotation = rotations
                .spans()
                .get(cursor)
                .ok_or(AnimationError::RootRigidBudget)?
                .restricted(a, b)?;
            let interval_start = start + a;
            let (prefix, _) = self.prefix(interval_start, initial, cycle)?;
            let [from, to] = crate::root_clock::root_segment_phases(
                interval_start,
                start + b,
                self.0.duration as f32,
                self.0.playback,
            )?;
            let positions = self
                .0
                .translation
                .checked_piece(from, to)?
                .map(|p| p + self.0.origin);
            spans.push(self.span(rotation, positions, prefix, axes));
            let absolute = start + b;
            let (mut n, mut phase) = self.cycle_phase(absolute)?;
            if self.0.playback == Playback::Loop && phase == 0. && n > 0 {
                n -= 1;
                phase = self.0.duration;
            }
            let event = rotations
                .spans()
                .get(cursor + 1)
                .filter(|span| span.is_step() && span.start() == b);
            let jump = self.0.translation.jump(phase);
            if event.is_some() || jump.is_some() {
                let prefix = initial.compose(power(cycle, n)?)?;
                let rotation = if let Some(event) = event {
                    event.clone()
                } else {
                    RootRotationSpan::held(
                        b,
                        b,
                        prefix.rotation * self.0.rotation.phase_rotation(phase)?,
                    )
                };
                let values = jump.unwrap_or([self.0.translation.position(phase); 2]);
                let from = values[0] + self.0.origin;
                let to = values[1] + self.0.origin;
                let positions = [from, from.lerp(to, 1. / 3.), from.lerp(to, 2. / 3.), to];
                spans.push(self.span(rotation, positions, prefix, axes));
            }
            if spans.len() > max_spans {
                return Err(AnimationError::RootRigidBudget);
            }
        }
        Ok(RootRigidPath {
            spans,
            duration: elapsed,
            translation_cuts,
            end: end_transform,
        })
    }
}

#[cfg(test)]
mod tests;
