//! Outward-rounded small-angle exponential enclosures.
//! Assumes IEEE-754 round-to-nearest basic operations and gradual underflow.
//! No platform sin/cos/atan or platform argument-reduction assumptions are used.
use super::*;
mod compilation;
mod twist;
pub(crate) use compilation::{
    quaternion_composition_evaluation_error_bounds, quaternion_composition_uniform_error,
    quaternion_cubic_control_error_bounds, quaternion_cubic_interval_evaluation_error_bounds,
    quaternion_cubic_normalized_error_bounds, quaternion_cubic_phase_evaluation_error_bounds,
    quaternion_cubic_restriction_error_bounds, quaternion_cubic_source_angular_bounds,
    quaternion_cubic_source_speed_bound, quaternion_normalization_error_bounds,
    quaternion_normalized_composition_uniform_error, translation_coefficient_error_bounds,
    translation_interval_evaluation_error_bounds, translation_phase_evaluation_error_bounds,
    translation_piece_error_bounds, translation_source_position_bounds,
    translation_source_velocity_bounds,
};
mod accumulation;
mod automatic_fade;
mod cache;
mod coordinate;
mod cubic;
mod fade;
mod fade_point;
mod integrated_fade;
mod linear_source;
mod path_field;
mod points;
mod rates;
mod source_fade;
pub(crate) use linear_source::{
    source_key_rotation_bounds, source_linear_angular_bounds, source_linear_fraction,
    source_linear_rotation_bounds, source_relative_rotation_bounds,
};
mod wall_partition;
pub use accumulation::RootRigidErrorAccumulator;
pub use automatic_fade::RootRigidMappedPath;
pub use cache::{RootRigidEvaluatedPose, RootScrewEnclosurePath};
pub use coordinate::RootRigidCoordinateCertificate;
pub use fade::{RootRigidFadeFieldInterval, RootRigidFieldInterval};
pub use fade_point::RootRigidFadePointCertificate;
pub use integrated_fade::{
    RootRigidCertifiedFadeInterval, RootRigidFadeDomain, RootRigidMappedField,
};
pub use rates::RootAngularDerivativeBounds;
pub use source_fade::RootRigidSourceField;
pub use twist::{RootRigidTwistEnclosure, RootTwistErrorBounds};
pub use wall_partition::{RootRigidWallInterval, RootRigidWallPartition};

/// Exact-source enclosure of a cycle/key boundary relative to a path origin.
#[derive(Clone, Copy, Debug)]
pub struct RootTimeCutEnclosure {
    evaluated: f64,
    source: [f64; 2],
    error: f64,
}
impl RootTimeCutEnclosure {
    pub fn new(cycle: u64, duration: f32, key: f64, origin: f64) -> Result<Self, AnimationError> {
        if cycle > 9007199254740991
            || !duration.is_finite()
            || duration <= 0.
            || !key.is_finite()
            || key < 0.
            || key > f64::from(duration)
            || !origin.is_finite()
            || origin < 0.
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let d = f64::from(duration);
        let evaluated = (cycle as f64 * d + key) - origin;
        if !evaluated.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let source = Scalar::exact(cycle as f64)
            .mul(Scalar::exact(d))?
            .add(Scalar::exact(key))?
            .sub(Scalar::exact(origin))?;
        let discrepancy = source.sub(Scalar::exact(evaluated))?;
        Ok(Self {
            evaluated,
            source: source.array(),
            error: discrepancy.0.abs().max(discrepancy.1.abs()),
        })
    }
    fn mapped(self, source: Scalar, evaluated: f64) -> Result<Self, AnimationError> {
        if !evaluated.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let error = source.sub(Scalar::exact(evaluated))?;
        Ok(Self {
            evaluated,
            source: source.array(),
            error: error.0.abs().max(error.1.abs()),
        })
    }
    pub fn retimed(self, old_duration: f64, new_duration: f64) -> Result<Self, AnimationError> {
        if !old_duration.is_finite()
            || old_duration <= 0.
            || !new_duration.is_finite()
            || new_duration < 0.
        {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        self.mapped(
            Scalar(self.source[0], self.source[1])
                .div_positive(old_duration)?
                .mul(Scalar::exact(new_duration))?,
            (self.evaluated / old_duration) * new_duration,
        )
    }
    pub fn shifted(self, offset: f64) -> Result<Self, AnimationError> {
        if !offset.is_finite() || offset < 0. {
            return Err(AnimationError::InvalidSampleTime);
        }
        self.mapped(
            Scalar(self.source[0], self.source[1]).add(Scalar::exact(offset))?,
            self.evaluated + offset,
        )
    }
    pub fn evaluated(self) -> f64 {
        self.evaluated
    }
    pub fn exact_source_bounds(self) -> [f64; 2] {
        self.source
    }
    pub fn absolute_error_bound(self) -> f64 {
        self.error
    }
    /// Converts clock discrepancy to displacement for a continuous field.
    /// Supplied speeds must bound the entire exact/evaluated time corridor,
    /// in one fixed spatial frame. STEP impulses require separate event handling.
    pub fn continuous_motion_error(
        self,
        linear_speed: f64,
        angular_speed: f64,
        point_radius: f64,
    ) -> Result<[f64; 2], AnimationError> {
        if [linear_speed, angular_speed, point_radius]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.)
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let dt = Scalar::exact(self.error);
        let angular = dt.mul(Scalar::exact(angular_speed))?;
        let point_speed = Scalar::exact(linear_speed)
            .add(Scalar::exact(angular_speed).mul(Scalar::exact(point_radius))?)?;
        Ok([dt.mul(point_speed)?.1, angular.1])
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RootRigidEnclosure {
    translation: [[f64; 2]; 3],
    /// Quaternion components in x,y,z,w order. No floating normalization applied.
    rotation: [[f64; 2]; 4],
}
/// Outward enclosure of products of stored signed uniform scales.
#[derive(Clone, Copy, Debug)]
pub struct RootUniformScaleEnclosure {
    value: Scalar,
}
impl RootUniformScaleEnclosure {
    pub const ONE: Self = Self {
        value: Scalar(1., 1.),
    };
    pub fn from_scale(scale: f64) -> Result<Self, AnimationError> {
        if !scale.is_finite() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        Ok(Self {
            value: Scalar::exact(scale),
        })
    }
    pub fn multiplied(self, other: Self) -> Result<Self, AnimationError> {
        Ok(Self {
            value: self.value.mul(other.value)?,
        })
    }
    pub fn bounds(self) -> [f64; 2] {
        self.value.array()
    }
    fn absolute_upper(self) -> f64 {
        self.value.0.abs().max(self.value.1.abs())
    }
    fn invertible(self) -> bool {
        self.value.0 > 0. || self.value.1 < 0.
    }
}

#[derive(Clone, Copy, Debug)]
struct Scalar(f64, f64);
impl Scalar {
    fn exact(v: f64) -> Self {
        Self(v, v)
    }
    fn rounded(lo: f64, hi: f64) -> Result<Self, AnimationError> {
        let result = Self(lo.next_down(), hi.next_up());
        if !result.0.is_finite() || !result.1.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok(result)
    }
    fn is_zero(self) -> bool {
        self.0 == 0. && self.1 == 0.
    }
    fn is_finite(self) -> bool {
        self.0.is_finite() && self.1.is_finite()
    }
    fn add(self, b: Self) -> Result<Self, AnimationError> {
        if self.is_zero() && b.is_finite() {
            return Ok(b);
        }
        if b.is_zero() && self.is_finite() {
            return Ok(self);
        }
        Self::rounded(self.0 + b.0, self.1 + b.1)
    }
    fn sub(self, b: Self) -> Result<Self, AnimationError> {
        if b.is_zero() && self.is_finite() {
            return Ok(self);
        }
        if self.is_zero() && b.is_finite() {
            return Ok(Self(-b.1, -b.0));
        }
        if self.is_finite() && self.0 == self.1 && self.0 == b.0 && b.0 == b.1 {
            return Ok(Self::exact(0.));
        }
        Self::rounded(self.0 - b.1, self.1 - b.0)
    }
    fn mul(self, b: Self) -> Result<Self, AnimationError> {
        if (self.is_zero() && b.is_finite()) || (b.is_zero() && self.is_finite()) {
            return Ok(Self::exact(0.));
        }
        let values = [self.0 * b.0, self.0 * b.1, self.1 * b.0, self.1 * b.1];
        if values.iter().any(|v| !v.is_finite()) {
            return Err(AnimationError::NumericalOverflow);
        }
        Self::rounded(
            values.into_iter().fold(f64::INFINITY, f64::min),
            values.into_iter().fold(f64::NEG_INFINITY, f64::max),
        )
    }
    fn div_positive(self, b: f64) -> Result<Self, AnimationError> {
        if !b.is_finite() || b <= 0. {
            return Err(AnimationError::NumericalOverflow);
        }
        if self.is_zero() {
            return Ok(self);
        }
        Self::rounded(self.0 / b, self.1 / b)
    }
    fn sqrt_positive(self) -> Result<Self, AnimationError> {
        if self.0 <= 0. {
            return Err(AnimationError::NumericalOverflow);
        }
        Self::rounded(self.0.sqrt(), self.1.sqrt())
    }
    fn div_interval_positive(self, b: Self) -> Result<Self, AnimationError> {
        if !b.is_finite() || b.0 <= 0. {
            return Err(AnimationError::NumericalOverflow);
        }
        if self.is_zero() {
            return Ok(self);
        }
        let values = [self.0 / b.0, self.0 / b.1, self.1 / b.0, self.1 / b.1];
        if values.iter().any(|v| !v.is_finite()) {
            return Err(AnimationError::NumericalOverflow);
        }
        Self::rounded(
            values.into_iter().fold(f64::INFINITY, f64::min),
            values.into_iter().fold(f64::NEG_INFINITY, f64::max),
        )
    }
    fn square(self) -> Result<Self, AnimationError> {
        if self.is_zero() {
            return Ok(self);
        }
        let lo = if self.0 <= 0. && self.1 >= 0. {
            0.
        } else {
            {
                let v = self.0.abs().min(self.1.abs());
                v * v
            }
        };
        let hi = {
            let v = self.0.abs().max(self.1.abs());
            v * v
        };
        let mut result = Self::rounded(lo, hi)?;
        result.0 = result.0.max(0.);
        Ok(result)
    }
    fn array(self) -> [f64; 2] {
        [self.0, self.1]
    }
}
// Shared expression tree for ordinary enclosures and temporal width families.
trait EnclosureArithmetic: Copy {
    fn exact(value: f64) -> Self;
    fn domain(self) -> Scalar;
    fn add(self, other: Self) -> Result<Self, AnimationError>;
    fn sub(self, other: Self) -> Result<Self, AnimationError>;
    fn mul(self, other: Self) -> Result<Self, AnimationError>;
    fn square(self) -> Result<Self, AnimationError>;
    fn div_positive(self, value: f64) -> Result<Self, AnimationError>;
    fn negate(self) -> Self;
    fn nonnegative(self) -> Self;
    fn symmetric_remainder(self) -> Result<Self, AnimationError>;
}
impl EnclosureArithmetic for Scalar {
    fn exact(value: f64) -> Self {
        Self::exact(value)
    }
    fn domain(self) -> Scalar {
        self
    }
    fn add(self, other: Self) -> Result<Self, AnimationError> {
        self.add(other)
    }
    fn sub(self, other: Self) -> Result<Self, AnimationError> {
        self.sub(other)
    }
    fn mul(self, other: Self) -> Result<Self, AnimationError> {
        self.mul(other)
    }
    fn square(self) -> Result<Self, AnimationError> {
        self.square()
    }
    fn div_positive(self, value: f64) -> Result<Self, AnimationError> {
        self.div_positive(value)
    }
    fn negate(self) -> Self {
        Self(-self.1, -self.0)
    }
    fn nonnegative(self) -> Self {
        Self(self.0.max(0.), self.1)
    }
    fn symmetric_remainder(self) -> Result<Self, AnimationError> {
        let radius = self.0.abs().max(self.1.abs());
        Ok(Self(-radius, radius))
    }
}
fn cross<S: EnclosureArithmetic>(a: [S; 3], b: [S; 3]) -> Result<[S; 3], AnimationError> {
    Ok([
        a[1].mul(b[2])?.sub(a[2].mul(b[1])?)?,
        a[2].mul(b[0])?.sub(a[0].mul(b[2])?)?,
        a[0].mul(b[1])?.sub(a[1].mul(b[0])?)?,
    ])
}
fn rotate<S: EnclosureArithmetic>(q: [S; 4], v: [S; 3]) -> Result<[S; 3], AnimationError> {
    let vector = [q[0], q[1], q[2]];
    let c = cross(vector, v)?;
    let twice = [
        c[0].mul(S::exact(2.))?,
        c[1].mul(S::exact(2.))?,
        c[2].mul(S::exact(2.))?,
    ];
    let second = cross(vector, twice)?;
    Ok([
        v[0].add(q[3].mul(twice[0])?)?.add(second[0])?,
        v[1].add(q[3].mul(twice[1])?)?.add(second[1])?,
        v[2].add(q[3].mul(twice[2])?)?.add(second[2])?,
    ])
}
fn compose_components<S: EnclosureArithmetic>(
    at: [S; 3],
    a: [S; 4],
    bt: [S; 3],
    b: [S; 4],
) -> Result<([S; 3], [S; 4]), AnimationError> {
    let rotated = rotate(a, bt)?;
    let translation = [
        at[0].add(rotated[0])?,
        at[1].add(rotated[1])?,
        at[2].add(rotated[2])?,
    ];
    let rotation = [
        a[3].mul(b[0])?
            .add(a[0].mul(b[3])?)?
            .add(a[1].mul(b[2])?)?
            .sub(a[2].mul(b[1])?)?,
        a[3].mul(b[1])?
            .sub(a[0].mul(b[2])?)?
            .add(a[1].mul(b[3])?)?
            .add(a[2].mul(b[0])?)?,
        a[3].mul(b[2])?
            .add(a[0].mul(b[1])?)?
            .sub(a[1].mul(b[0])?)?
            .add(a[2].mul(b[3])?)?,
        a[3].mul(b[3])?
            .sub(a[0].mul(b[0])?)?
            .sub(a[1].mul(b[1])?)?
            .sub(a[2].mul(b[2])?)?,
    ];
    Ok((translation, rotation))
}

impl RootRigidEnclosure {
    /// Uniform rounding cap for glam 0.33.7 DQuat::normalize on stored f64
    /// inputs in these boxes, relative to exact normalization of those inputs.
    /// Does not include source selection, composition or publication error.
    /// Rejects boxes whose nonzero norm and finite operations cannot be proved.
    pub fn stored_quaternion_normalization_error_bounds(
        input: [[f64; 2]; 4],
    ) -> Result<[f64; 4], AnimationError> {
        compilation::stored_quaternion_normalization_error(input)
    }

    pub const IDENTITY: Self = Self {
        translation: [[0., 0.]; 3],
        rotation: [[0., 0.], [0., 0.], [0., 0.], [1., 1.]],
    };
    /// Outward intervals; fields stay private to preserve the unit-rotation proof.
    pub fn translation_bounds(&self) -> [[f64; 2]; 3] {
        self.translation
    }
    pub fn rotation_bounds(&self) -> [[f64; 2]; 4] {
        self.rotation
    }
    fn vectors(&self) -> ([Scalar; 3], [Scalar; 4]) {
        (
            self.translation.map(|v| Scalar(v[0], v[1])),
            self.rotation.map(|v| Scalar(v[0], v[1])),
        )
    }
    /// Exact stored translation and real normalization of the stored quaternion.
    /// The input must satisfy RootRigidTransform's finite/unit-near validation.
    /// Outward sqrt/division includes quaternion normalization uncertainty.
    pub fn from_transform(transform: RootRigidTransform) -> Result<Self, AnimationError> {
        transform.checked()?;
        let q = transform.rotation.to_array().map(Scalar::exact);
        let norm = q[0]
            .square()?
            .add(q[1].square()?)?
            .add(q[2].square()?)?
            .add(q[3].square()?)?
            .sqrt_positive()?;
        Ok(Self {
            translation: transform.translation.to_array().map(|v| [v, v]),
            rotation: [
                q[0].div_interval_positive(norm)?.array(),
                q[1].div_interval_positive(norm)?.array(),
                q[2].div_interval_positive(norm)?.array(),
                q[3].div_interval_positive(norm)?.array(),
            ],
        })
    }
    /// Changes translation units under a nonzero signed uniform scale while
    /// retaining the same proper rotation. Products remain outward enclosed.
    pub fn with_translation_scale(&self, scale: f64) -> Result<Self, AnimationError> {
        if !scale.is_finite() || scale == 0. {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        self.with_translation_scale_enclosed(RootUniformScaleEnclosure::from_scale(scale)?)
    }
    pub fn with_translation_scale_enclosed(
        &self,
        scale: RootUniformScaleEnclosure,
    ) -> Result<Self, AnimationError> {
        if !scale.invertible() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        let mut translation = [[0.; 2]; 3];
        for (target, source) in translation.iter_mut().zip(self.translation) {
            *target = Scalar(source[0], source[1]).mul(scale.value)?.array();
        }
        Ok(Self {
            translation,
            rotation: self.rotation,
        })
    }

    /// Encloses the inverse of the represented real unit rigid transforms.
    pub fn inverse(&self) -> Result<Self, AnimationError> {
        let (translation, q) = self.vectors();
        let conjugate = [
            Scalar(-q[0].1, -q[0].0),
            Scalar(-q[1].1, -q[1].0),
            Scalar(-q[2].1, -q[2].0),
            q[3],
        ];
        let moved = rotate(conjugate, translation)?;
        Ok(Self {
            translation: moved.map(|v| [-v.1, -v.0]),
            rotation: conjugate.map(Scalar::array),
        })
    }
    /// Transports an authored-to-body frame between two body-to-world poses,
    /// preserving its world anchor: B_next*C_next = B_previous*C_previous.
    /// Includes pose normalization and composition uncertainty retained by the
    /// enclosure owners. Choosing when an anchor follows input is caller policy.
    pub fn transported_body_reference(
        &self,
        previous_body_to_world: &Self,
        next_body_to_world: &Self,
    ) -> Result<Self, AnimationError> {
        next_body_to_world
            .inverse()?
            .compose(&previous_body_to_world.compose(self)?)
    }
    /// Similarity x_target = scale * frame.rotation * x_source + frame.translation.
    /// Signed uniform scale supports reflection; zero retains shifted rotation.
    /// Encloses all frame conjugation arithmetic, without treating frame bounds
    /// or quaternion normalization as exact stored floating values.
    pub fn transformed(&self, frame: &Self, scale: f64) -> Result<Self, AnimationError> {
        if !scale.is_finite() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        let (translation, _) = self.vectors();
        let scale = Scalar::exact(scale);
        let scaled = Self {
            translation: [
                translation[0].mul(scale)?.array(),
                translation[1].mul(scale)?.array(),
                translation[2].mul(scale)?.array(),
            ],
            rotation: self.rotation,
        };
        frame.compose(&scaled)?.compose(&frame.inverse()?)
    }
    /// Same order as RootRigidTransform::compose; every represented exact rotation
    /// remains unit under composition. No approximate quaternion normalization.
    pub fn compose(&self, other: &Self) -> Result<Self, AnimationError> {
        let (at, a) = self.vectors();
        let (bt, b) = other.vectors();
        let (translation, rotation) = compose_components(at, a, bt, b)?;
        Ok(Self {
            translation: translation.map(Scalar::array),
            rotation: rotation.map(Scalar::array),
        })
    }

    /// Encloses a transformed exact stored point, including prefix arithmetic.
    pub fn transform_point(&self, point: DVec3) -> Result<[[f64; 2]; 3], AnimationError> {
        if !point.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let (translation, rotation) = self.vectors();
        let rotated = rotate(rotation, point.to_array().map(Scalar::exact))?;
        Ok([
            translation[0].add(rotated[0])?.array(),
            translation[1].add(rotated[1])?.array(),
            translation[2].add(rotated[2])?.array(),
        ])
    }
}
// Eight alternating terms plus the magnitude of the ninth. For x in [0,1]
// successive term magnitudes decrease; the alternating remainder is bounded
// by the first omitted term. Every recurrence operation is outward rounded.
fn series<S: EnclosureArithmetic>(
    x: S,
    initial: S,
    denominator: impl Fn(u32) -> u32,
) -> Result<S, AnimationError> {
    let mut term = initial;
    let mut sum = initial;
    for n in 1..=7 {
        term = term.mul(x)?.div_positive(f64::from(denominator(n)))?;
        term = term.negate();
        sum = sum.add(term)?;
    }
    let remainder = term.mul(x)?.div_positive(f64::from(denominator(8)))?;
    sum.add(remainder.symmetric_remainder()?)
}
impl RootRigidTwist {
    /// Encloses the real SE(3) exponential of these exact stored f64 inputs.
    /// Supports |angular*duration| <= 1 radian, proven by interval arithmetic.
    /// Larger angles require subdivision; overflow/invalid inputs reject.
    /// This encloses one increment only, not accumulated path prefixes, frame
    /// conversion or collision arithmetic. It is not a complete sweep certificate.
    pub fn increment_enclosure(self, duration: f64) -> Result<RootRigidEnclosure, AnimationError> {
        if !duration.is_finite() || duration < 0. {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        self.increment_interval_enclosure(Scalar::exact(duration))
    }
    pub(super) fn increment_between_enclosure(
        self,
        start: f64,
        end: f64,
    ) -> Result<RootRigidEnclosure, AnimationError> {
        if !start.is_finite() || !end.is_finite() || start < 0. || end <= start {
            return Err(AnimationError::InvalidAnimationTimeStep);
        }
        let mut duration = Scalar::exact(end).sub(Scalar::exact(start))?;
        duration.0 = duration.0.max(0.);
        self.increment_interval_enclosure(duration)
    }
    fn increment_interval_enclosure(
        self,
        dt: Scalar,
    ) -> Result<RootRigidEnclosure, AnimationError> {
        let (translation, rotation) = increment_components(self, dt)?;
        Ok(RootRigidEnclosure {
            translation: translation.map(Scalar::array),
            rotation: rotation.map(Scalar::array),
        })
    }
}

fn increment_components<S: EnclosureArithmetic>(
    twist: RootRigidTwist,
    dt: S,
) -> Result<([S; 3], [S; 4]), AnimationError> {
    if !twist.linear.is_finite() || !twist.angular.is_finite() {
        return Err(AnimationError::NumericalOverflow);
    }
    if dt.domain().is_zero() {
        return Ok((
            [S::exact(0.); 3],
            [S::exact(0.), S::exact(0.), S::exact(0.), S::exact(1.)],
        ));
    }
    let mut omega = [S::exact(0.); 3];
    let mut displacement = omega;
    for i in 0..3 {
        omega[i] = S::exact(twist.angular[i]).mul(dt)?;
        displacement[i] = S::exact(twist.linear[i]).mul(dt)?;
    }
    let mut x = omega[0]
        .square()?
        .add(omega[1].square()?)?
        .add(omega[2].square()?)?;
    x = x.nonnegative();
    if x.domain().1 > 1. {
        return Err(AnimationError::RootRigidBudget);
    }
    let a = series(x, S::exact(0.5), |n| (2 * n + 1) * (2 * n + 2))?;
    let b = series(x, S::exact(1.).div_positive(6.)?, |n| {
        (2 * n + 2) * (2 * n + 3)
    })?;
    let q = series(x, S::exact(0.5), |n| 4 * (2 * n) * (2 * n + 1))?;
    let w = series(x, S::exact(1.), |n| 4 * (2 * n - 1) * (2 * n))?;
    let first = cross(omega, displacement)?;
    let second = cross(omega, first)?;
    let mut translation = [S::exact(0.); 3];
    let mut rotation = [S::exact(0.); 4];
    for i in 0..3 {
        translation[i] = displacement[i]
            .add(a.mul(first[i])?)?
            .add(b.mul(second[i])?)?;
        rotation[i] = omega[i].mul(q)?;
    }
    rotation[3] = w;
    Ok((translation, rotation))
}

impl RootRigidPath {
    /// Encloses the canonical ordered spatial field of stored screw rates and
    /// exact differences of stored timestamps. Floating stored initial prefixes
    /// are recomputed outward, not assumed exact. This does not enclose upstream
    /// curve extraction/blending errors or the runtime's entire numeric pipeline.
    /// STEP and polynomial prefixes, larger angular spans and exhausted work reject.
    pub fn screw_field_enclosure(
        &self,
        span_index: usize,
        fraction: f64,
        max_spans: usize,
    ) -> Result<RootRigidEnclosure, AnimationError> {
        if !fraction.is_finite()
            || !(0. ..=1.).contains(&fraction)
            || span_index >= self.spans.len()
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        if span_index >= max_spans.min(MAX_ROOT_ROTATION_SPANS) {
            return Err(AnimationError::RootRigidBudget);
        }
        let mut prefix = RootRigidEnclosure::IDENTITY;
        for (index, span) in self.spans[..=span_index].iter().enumerate() {
            let (twist, _) = span
                .screw
                .ok_or(AnimationError::RootRotationTransitionUnsupported)?;
            if span.end() <= span.start() {
                return Err(AnimationError::RootRotationTransitionUnsupported);
            }
            let mut duration = Scalar::exact(span.end()).sub(Scalar::exact(span.start()))?;
            duration.0 = duration.0.max(0.);
            if index == span_index {
                if fraction == 0. {
                    return Ok(prefix);
                }
                duration = duration.mul(Scalar::exact(fraction))?;
                duration.0 = duration.0.max(0.);
            }
            prefix = twist
                .increment_interval_enclosure(duration)?
                .compose(&prefix)?;
        }
        Ok(prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_zero_identities_preserve_constraints_without_masking_invalid_values() {
        let z = Scalar::exact(0.);
        let b = Scalar(-3., 7.);
        assert_eq!(z.add(b).unwrap().array(), b.array());
        assert_eq!(b.sub(z).unwrap().array(), b.array());
        assert_eq!(z.sub(b).unwrap().array(), [-7., 3.]);
        assert_eq!(z.mul(b).unwrap().array(), [0., 0.]);
        assert_eq!(z.div_positive(3.).unwrap().array(), [0., 0.]);
        assert_eq!(
            z.div_interval_positive(Scalar(2., 3.)).unwrap().array(),
            [0., 0.]
        );
        assert_eq!(z.square().unwrap().array(), [0., 0.]);
        assert_eq!(
            Scalar::exact(3.).sub(Scalar::exact(3.)).unwrap().array(),
            [0., 0.]
        );
        let d = b.sub(b).unwrap();
        assert!(d.0 < -10. && d.1 > 10.);
        assert!(z.mul(Scalar::exact(f64::INFINITY)).is_err());
        assert!(z.div_positive(0.).is_err());
        assert!(
            z.div_interval_positive(Scalar::exact(f64::INFINITY))
                .is_err()
        );
    }
    #[test]
    fn small_angle_increment_enclosures_cover_axial_and_offset_pivot_motion() {
        for (linear, angular, dt) in [
            (DVec3::new(0.2, -0.1, 0.3), DVec3::ZERO, 0.3),
            (DVec3::Y, DVec3::Z, 0.4),
            (DVec3::new(0.7, -0.2, 0.1), DVec3::new(0.3, -0.5, 0.7), 0.7),
            (DVec3::new(0.7, -0.2, 0.1), DVec3::new(0.3, -0.5, 0.7), 1e-9),
            (DVec3::new(0.7, -0.2, 0.1), DVec3::new(0.3, -0.5, 0.7), 0.),
        ] {
            let e = RootRigidTwist { linear, angular }
                .increment_enclosure(dt)
                .unwrap();
            assert!(
                e.translation
                    .iter()
                    .chain(&e.rotation)
                    .all(|b| b[0].is_finite() && b[0] <= b[1])
            );
            println!(
                "SCREW_ENCLOSURE {:?}",
                (
                    linear.to_array(),
                    angular.to_array(),
                    dt,
                    e.translation,
                    e.rotation
                )
            );
        }
        assert!(
            RootRigidTwist {
                linear: DVec3::ZERO,
                angular: DVec3::Y * 2.
            }
            .increment_enclosure(1.)
            .is_err()
        );
        assert!(
            RootRigidTwist {
                linear: DVec3::NAN,
                angular: DVec3::ZERO
            }
            .increment_enclosure(0.)
            .is_err()
        );
    }
    #[test]
    fn ordered_enclosure_composition_carries_prefix_and_point_arithmetic() {
        let first = RootRigidTwist {
            linear: DVec3::new(0.3, 0.1, -0.2),
            angular: DVec3::X * 0.7,
        };
        let second = RootRigidTwist {
            linear: DVec3::new(-0.2, 0.4, 0.1),
            angular: DVec3::Y * 0.8,
        };
        let point = DVec3::new(0.4, -0.1, 0.7);
        for count in [2, 16, 64] {
            let mut enclosure = RootRigidEnclosure::IDENTITY;
            for i in 0..count {
                let twist = if i % 2 == 0 { first } else { second };
                enclosure = twist
                    .increment_enclosure(0.03)
                    .unwrap()
                    .compose(&enclosure)
                    .unwrap();
            }
            let transformed = enclosure.transform_point(point).unwrap();
            assert!(transformed.iter().all(|v| v[1] - v[0] < 1e-8));
            println!(
                "SCREW_PREFIX_ENCLOSURE {:?}",
                (
                    count,
                    first.linear.to_array(),
                    first.angular.to_array(),
                    second.linear.to_array(),
                    second.angular.to_array(),
                    0.03,
                    point.to_array(),
                    enclosure.translation_bounds(),
                    enclosure.rotation_bounds(),
                    transformed
                )
            );
        }
        assert!(
            RootRigidEnclosure::IDENTITY
                .transform_point(DVec3::NAN)
                .is_err()
        );
        let a = first.increment_enclosure(0.3).unwrap();
        let b = second.increment_enclosure(0.4).unwrap();
        let ordered = b.compose(&a).unwrap();
        let reversed = a.compose(&b).unwrap();
        assert!(ordered.rotation_bounds()[2][1] < reversed.rotation_bounds()[2][0]);
    }
    #[test]
    fn screw_path_enclosures_use_exact_stored_timestamp_differences() {
        let a = RootRigidTwist {
            linear: DVec3::new(0.3, 0.1, -0.2),
            angular: DVec3::X * 0.7,
        };
        let b = RootRigidTwist {
            linear: DVec3::new(-0.2, 0.4, 0.1),
            angular: DVec3::Y * 0.8,
        };
        let path =
            RootRigidPath::from_twists(&[(a, 0.1), (b, 0.2), (a, 0.3), (b, 0.1)], 4).unwrap();
        let definitions: Vec<_> = path
            .spans()
            .iter()
            .map(|span| {
                let (twist, _) = span.screw.unwrap();
                (
                    twist.linear.to_array(),
                    twist.angular.to_array(),
                    span.start(),
                    span.end(),
                )
            })
            .collect();
        let point = DVec3::new(0.4, -0.1, 0.7);
        let cache = path.prepare_screw_enclosures(4).unwrap();
        assert!(std::ptr::eq(cache.path(), &path));
        assert!(path.prepare_screw_enclosures(3).is_err());
        assert!(cache.sample(4, 0.5).is_err());
        assert!(cache.sample(0, f64::NAN).is_err());
        for index in 0..4 {
            for fraction in [0., 0.37, 1.] {
                let enclosure = path.screw_field_enclosure(index, fraction, 4).unwrap();
                let prepared = cache.sample(index, fraction).unwrap();
                println!(
                    "CACHED_SCREW_ENCLOSURE {:?}",
                    (
                        &definitions,
                        index,
                        fraction,
                        point.to_array(),
                        prepared.translation_bounds(),
                        prepared.rotation_bounds(),
                        prepared.transform_point(point).unwrap()
                    )
                );
                let transformed = enclosure.transform_point(point).unwrap();
                assert!(transformed.iter().all(|v| v[1] - v[0] < 1e-10));
                println!(
                    "SCREW_PATH_ENCLOSURE {:?}",
                    (
                        &definitions,
                        index,
                        fraction,
                        point.to_array(),
                        enclosure.translation_bounds(),
                        enclosure.rotation_bounds(),
                        transformed
                    )
                );
            }
        }
        assert!(path.screw_field_enclosure(1, 0.5, 1).is_err());
        assert!(path.screw_field_enclosure(4, 0.5, 4).is_err());
        assert!(path.screw_field_enclosure(0, f64::NAN, 4).is_err());
        assert!(path.screw_field_enclosure(0, -0.1, 4).is_err());
    }
    #[test]
    fn frame_enclosures_preserve_similarity_inverse_and_signed_scales() {
        let first = RootRigidTwist {
            linear: DVec3::new(0.3, 0.1, -0.2),
            angular: DVec3::X * 0.7,
        };
        let second = RootRigidTwist {
            linear: DVec3::new(-0.2, 0.4, 0.1),
            angular: DVec3::Y * 0.8,
        };
        let source = second
            .increment_enclosure(0.2)
            .unwrap()
            .compose(&first.increment_enclosure(0.3).unwrap())
            .unwrap();
        let transform = RootRigidTransform {
            translation: DVec3::new(0.4, -0.2, 0.3),
            rotation: DQuat::from_rotation_y(0.7),
        };
        let frame = RootRigidEnclosure::from_transform(transform).unwrap();
        let point = DVec3::new(0.4, -0.1, 0.7);
        let identity = frame
            .compose(&frame.inverse().unwrap())
            .unwrap()
            .transform_point(point)
            .unwrap();
        for i in 0..3 {
            assert!(identity[i][0] <= point[i] && point[i] <= identity[i][1]);
        }
        for scale in [-2., 0., 0.5, 2.] {
            let mapped = source.transformed(&frame, scale).unwrap();
            let transformed = mapped.transform_point(point).unwrap();
            assert!(transformed.iter().all(|v| v[1] - v[0] < 1e-10));
            println!(
                "SCREW_FRAME_ENCLOSURE {:?}",
                (
                    (first.linear.to_array(), first.angular.to_array(), 0.3),
                    (second.linear.to_array(), second.angular.to_array(), 0.2),
                    (
                        transform.translation.to_array(),
                        transform.rotation.to_array()
                    ),
                    scale,
                    point.to_array(),
                    mapped.translation_bounds(),
                    mapped.rotation_bounds(),
                    transformed
                )
            );
        }
        assert!(source.transformed(&frame, f64::NAN).is_err());
        assert!(
            RootRigidEnclosure::from_transform(RootRigidTransform {
                translation: DVec3::ZERO,
                rotation: DQuat::from_xyzw(0., 0., 0., 0.)
            })
            .is_err()
        );
    }
}

impl RootRigidCurve {
    /// Spatial velocity of the exact source pose on one continuous local key interval.
    /// For a pose `(t, Q)`, this bounds `omega` and `v = t' - omega cross t`.
    /// Key endpoints use one-sided derivatives; isolated velocity changes do
    /// not change the integrated continuous pose. Pose STEP impulses reject.
    /// Loop prefixes, retiming and fixed-frame adjoints must be applied separately.
    pub fn source_phase_twist_enclosure(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
    ) -> Result<Option<RootRigidTwistEnclosure>, AnimationError> {
        let Some(pose) = self.source_phase_interval_enclosure(times, axes)? else {
            return Ok(None);
        };
        if self.0.rotation.source_rotation_is_constant() {
            let velocity = self
                .0
                .translation
                .source_velocity_bounds(times[0], times[1])?;
            return Ok(Some(RootRigidTwistEnclosure::from_parts(
                std::array::from_fn(|i| {
                    if axes[i] {
                        Scalar(velocity[i][0], velocity[i][1])
                    } else {
                        Scalar::exact(0.)
                    }
                }),
                [Scalar::exact(0.); 3],
            )));
        }
        let Some(angular_bounds) = self
            .0
            .rotation
            .source_angular_velocity_bounds(times[0], times[1])?
        else {
            return Ok(None);
        };
        let angular = angular_bounds.map(|v| Scalar(v[0], v[1]));
        let velocity = self
            .0
            .translation
            .source_velocity_bounds(times[0], times[1])?
            .map(|v| Scalar(v[0], v[1]));
        let position = self
            .0
            .translation
            .source_position_bounds(times[0], times[1])?;
        let mut adjusted = [Scalar::exact(0.); 3];
        let mut pivot_velocity = velocity;
        for i in 0..3 {
            adjusted[i] = Scalar(position[i][0], position[i][1]);
            if axes[i] {
                adjusted[i] = adjusted[i].add(Scalar::exact(self.0.bind[i]))?;
                pivot_velocity[i] = Scalar::exact(0.);
            } else {
                adjusted[i] = adjusted[i].add(Scalar::exact(self.0.origin[i]))?;
            }
        }
        // t=a-Q*b: t'=a'-omega cross (Q*b)-Q*b'.
        // Subtracting omega cross t cancels the pivot position exactly:
        // v=p'-Q*b'-omega cross a. Keep that cancellation before interval
        // arithmetic to preserve signed translation derivatives.
        let rotated = rotate(
            pose.rotation_bounds().map(|q| Scalar(q[0], q[1])),
            pivot_velocity,
        )?;
        let coupling = cross(angular, adjusted)?;
        let mut linear = velocity;
        for i in 0..3 {
            linear[i] = linear[i].sub(rotated[i])?.sub(coupling[i])?;
        }
        Ok(Some(RootRigidTwistEnclosure::from_parts(linear, angular)))
    }

    /// Source point-speed cap for one translation/cubic-or-LINEAR-rotation interval.
    /// Includes extraction masks and moving pivots; no loop-prefix or parent-frame proof.
    /// Radius bounds the fixed authored point relative to the local frame origin.
    pub fn source_point_speed_bound(
        &self,
        start: f64,
        end: f64,
        axes: [bool; 3],
        radius: f64,
    ) -> Result<Option<f64>, AnimationError> {
        if !radius.is_finite() || radius < 0. {
            return Err(AnimationError::InvalidSampleTime);
        }
        let velocity = self.0.translation.source_velocity_bounds(start, end)?;
        let position = self.0.translation.source_position_bounds(start, end)?;
        let Some(angular) = self.0.rotation.source_angular_speed_bound(start, end)? else {
            return Ok(None);
        };
        let mut translation_speed = Scalar::exact(0.);
        let mut pivot_radius = Scalar::exact(0.);
        for axis in 0..3 {
            let speed = velocity[axis][0].abs().max(velocity[axis][1].abs());
            translation_speed = translation_speed.add(
                Scalar::exact(speed).mul(Scalar::exact(if axes[axis] { 1. } else { 2. }))?,
            )?;
            let pivot = if axes[axis] {
                Scalar::exact(self.0.bind[axis])
            } else {
                Scalar(position[axis][0], position[axis][1])
                    .add(Scalar::exact(self.0.origin[axis]))?
            };
            pivot_radius = pivot_radius.add(Scalar::exact(pivot.0.abs().max(pivot.1.abs())))?;
        }
        Ok(Some(
            translation_speed
                .add(Scalar::exact(angular).mul(pivot_radius.add(Scalar::exact(radius))?)?)?
                .1,
        ))
    }
}

impl RootRigidCurve {
    /// Local source speed mapped through a fixed enclosed signed similarity.
    /// The target box contains fixed points; moving parent frames are unsupported.
    pub fn mapped_source_point_speed_bound(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
        frame: &RootRigidEnclosure,
        scale: RootUniformScaleEnclosure,
        points: [[f64; 2]; 3],
    ) -> Result<Option<f64>, AnimationError> {
        let source = frame.inverse_similarity_point_box_bounds_enclosed(points, scale)?;
        let mut radius = Scalar::exact(0.);
        for axis in source {
            radius = radius.add(Scalar::exact(axis[0].abs().max(axis[1].abs())))?;
        }
        let Some(speed) = self.source_point_speed_bound(times[0], times[1], axes, radius.1)? else {
            return Ok(None);
        };
        Ok(Some(
            Scalar::exact(speed)
                .mul(Scalar::exact(scale.absolute_upper()))?
                .1,
        ))
    }
}

impl RootRigidCurve {
    /// Source factor at an exact local phase, including the extraction pivot.
    /// Cubic, LINEAR or proved constant rotation; represents real unit rotations.
    pub fn source_phase_enclosure(
        &self,
        phase: f64,
        axes: [bool; 3],
    ) -> Result<Option<RootRigidEnclosure>, AnimationError> {
        if !phase.is_finite() || phase < 0. || phase > self.0.duration {
            return Err(AnimationError::InvalidSampleTime);
        }
        let position = self.0.translation.source_position_bounds(phase, phase)?;
        let rotation = if self.0.rotation.source_rotation_is_constant() {
            [0., 0., 0., 1.].map(Scalar::exact)
        } else if let Some(bounds) = self
            .0
            .rotation
            .linear_relative_source_phase_rotation_bounds(phase)?
        {
            bounds.map(|q| Scalar(q[0], q[1]))
        } else {
            let Some(error) = self
                .0
                .rotation
                .cubic_relative_phase_evaluation_error_bounds(phase)?
            else {
                return Ok(None);
            };
            let q = self.0.rotation.phase_rotation(phase)?.to_array();
            let mut rotation = [Scalar::exact(0.); 4];
            for i in 0..4 {
                rotation[i] = if error[i] == 0. {
                    Scalar::exact(q[i])
                } else {
                    Scalar::exact(q[i]).add(Scalar(-error[i], error[i]))?
                };
            }
            rotation
        };
        let mut adjusted = [Scalar::exact(0.); 3];
        let mut pivot = [Scalar::exact(0.); 3];
        for i in 0..3 {
            let p = Scalar(position[i][0], position[i][1]).add(Scalar::exact(self.0.origin[i]))?;
            pivot[i] = if axes[i] {
                Scalar::exact(self.0.bind[i])
            } else {
                p
            };
            adjusted[i] = if axes[i] {
                p.sub(Scalar::exact(self.0.origin[i]).sub(Scalar::exact(self.0.bind[i]))?)?
            } else {
                p
            };
        }
        let rotated = rotate(rotation, pivot)?;
        for i in 0..3 {
            adjusted[i] = adjusted[i].sub(rotated[i])?;
        }
        Ok(Some(RootRigidEnclosure {
            translation: adjusted.map(Scalar::array),
            rotation: rotation.map(Scalar::array),
        }))
    }
    /// Enclosed exact-source loop power. No floating quaternion renormalization.
    pub fn source_cycle_prefix_enclosure(
        &self,
        cycles: u64,
        axes: [bool; 3],
    ) -> Result<Option<RootRigidEnclosure>, AnimationError> {
        if cycles > 9007199254740991 || (self.0.playback == Playback::Clamp && cycles != 0) {
            return Err(AnimationError::RootRigidBudget);
        }
        let Some(mut base) = self.source_phase_enclosure(self.0.duration, axes)? else {
            return Ok(None);
        };
        let mut power = cycles;
        let mut result = RootRigidEnclosure::IDENTITY;
        while power != 0 {
            if power & 1 != 0 {
                result = result.compose(&base)?;
            }
            power >>= 1;
            if power != 0 {
                base = base.compose(&base)?;
            }
        }
        Ok(Some(result))
    }
    pub fn source_sample_enclosure(
        &self,
        time: f64,
        axes: [bool; 3],
    ) -> Result<Option<RootRigidEnclosure>, AnimationError> {
        let clock = crate::enclose_root_cycle_phase(time, self.0.duration as f32, self.0.playback)?;
        if clock.exact_phase_bounds() != [clock.phase(); 2] {
            return Err(AnimationError::RootRigidBudget);
        }
        let Some(phase) = self.source_phase_enclosure(clock.phase(), axes)? else {
            return Ok(None);
        };
        if clock.cycle() == 0 {
            return Ok(Some(phase));
        }
        let Some(prefix) = self.source_cycle_prefix_enclosure(clock.cycle(), axes)? else {
            return Ok(None);
        };
        Ok(Some(prefix.compose(&phase)?))
    }
}

impl RootRigidCurve {
    /// Source movement relative to the authored factor at the interval start.
    /// Includes source loop powers and starting-frame inversion/composition.
    pub fn source_delta_enclosure(
        &self,
        start: f64,
        end: f64,
        axes: [bool; 3],
    ) -> Result<Option<RootRigidEnclosure>, AnimationError> {
        if end < start {
            return Err(AnimationError::InvalidSampleTime);
        }
        let Some(a) = self.source_sample_enclosure(start, axes)? else {
            return Ok(None);
        };
        if end == start {
            return Ok(Some(RootRigidEnclosure::IDENTITY));
        }
        let Some(b) = self.source_sample_enclosure(end, axes)? else {
            return Ok(None);
        };
        Ok(Some(a.inverse()?.compose(&b)?))
    }
    /// Pointwise source-versus-evaluated interval motion discrepancy.
    /// This does not bound intermediate poses or certify a collision sweep.
    pub fn source_delta_point_error(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        evaluated: DVec3,
    ) -> Result<Option<([f64; 3], f64)>, AnimationError> {
        let Some(source) = self.source_delta_enclosure(times[0], times[1], axes)? else {
            return Ok(None);
        };
        Ok(Some(
            source.enclosed_point_evaluation_error(point, 1., evaluated)?,
        ))
    }
}

impl RootRigidCurve {
    /// Uniform source pose enclosure on one continuous local key interval.
    /// Integrates proved speed caps rather than checking selected samples.
    pub fn source_phase_interval_enclosure(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
    ) -> Result<Option<RootRigidEnclosure>, AnimationError> {
        if self.0.translation.jump(times[1]).is_some() {
            return Err(AnimationError::RootRotationTransitionUnsupported);
        }
        let Some(initial) = self.source_phase_enclosure(times[0], axes)? else {
            return Ok(None);
        };
        let Some(speed) = self.source_point_speed_bound(times[0], times[1], axes, 0.)? else {
            return Ok(None);
        };
        let Some(angular) = self
            .0
            .rotation
            .source_angular_speed_bound(times[0], times[1])?
        else {
            return Ok(None);
        };
        let elapsed = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
        let position_error = elapsed.mul(Scalar::exact(speed))?.1;
        let quaternion_error = elapsed
            .mul(Scalar::exact(angular))?
            .mul(Scalar::exact(0.5))?
            .1;
        let mut result = initial;
        for axis in 0..3 {
            result.translation[axis] =
                Scalar(initial.translation[axis][0], initial.translation[axis][1])
                    .add(Scalar(-position_error, position_error))?
                    .array();
        }
        for axis in 0..4 {
            let expanded = Scalar(initial.rotation[axis][0], initial.rotation[axis][1])
                .add(Scalar(-quaternion_error, quaternion_error))?;
            result.rotation[axis] = [expanded.0.max(-1.), expanded.1.min(1.)];
        }
        Ok(Some(result))
    }
    /// Uniform interval motion from a fixed source reference time.
    /// The requested interval must stay in one key interval and one loop cell;
    /// its right endpoint may be the next seam. Reference can precede many loops.
    pub fn source_delta_interval_enclosure(
        &self,
        reference: f64,
        times: [f64; 2],
        axes: [bool; 3],
    ) -> Result<Option<RootRigidEnclosure>, AnimationError> {
        if reference > times[0] {
            return Err(AnimationError::InvalidSampleTime);
        }
        let phases = crate::root_clock::root_segment_phases(
            times[0],
            times[1],
            self.0.duration as f32,
            self.0.playback,
        )?;
        let clock =
            crate::enclose_root_cycle_phase(times[0], self.0.duration as f32, self.0.playback)?;
        let Some(local) = self.source_phase_interval_enclosure(phases, axes)? else {
            return Ok(None);
        };
        let Some(prefix) = self.source_cycle_prefix_enclosure(clock.cycle(), axes)? else {
            return Ok(None);
        };
        let Some(initial) = self.source_sample_enclosure(reference, axes)? else {
            return Ok(None);
        };
        Ok(Some(initial.inverse()?.compose(&prefix.compose(&local)?)?))
    }
}

impl RootRigidCurve {
    /// Exact source spatial field relative to a fixed source reference.
    /// Includes every loop prefix and both channels' key domains. The returned
    /// hull covers the whole interval, without a derivative or axis constraint.
    /// STEP impulses reject; clamp completion tails have zero velocity.
    pub fn source_delta_twist_enclosure(
        &self,
        reference: f64,
        times: [f64; 2],
        axes: [bool; 3],
        limit: usize,
    ) -> Result<Option<RootRigidTwistEnclosure>, AnimationError> {
        if !reference.is_finite() || reference < 0. || reference > times[0] {
            return Err(AnimationError::InvalidSampleTime);
        }
        let cuts = self.source_partition_cuts(times, limit)?;
        let Some(initial) = self.source_sample_enclosure(reference, axes)? else {
            return Ok(None);
        };
        let inverse = initial.inverse()?;
        let zero = RootRigidTwist {
            linear: DVec3::ZERO,
            angular: DVec3::ZERO,
        }
        .enclosure()?;
        let mut result: Option<RootRigidTwistEnclosure> = None;
        // A point domain retains its source derivative; wall retiming supplies
        // the zero clock rate for paused playback.
        let cells = if times[0] == times[1] {
            vec![times]
        } else {
            cuts.windows(2).map(|cell| [cell[0], cell[1]]).collect()
        };
        for cell in cells {
            let field = if self.0.playback == Playback::Clamp && cell[0] >= self.0.duration {
                zero
            } else {
                let phases = crate::root_clock::root_segment_phases(
                    cell[0],
                    cell[1],
                    self.0.duration as f32,
                    self.0.playback,
                )?;
                let Some(local) = self.source_phase_twist_enclosure(phases, axes)? else {
                    return Ok(None);
                };
                let clock = crate::enclose_root_cycle_phase(
                    cell[0],
                    self.0.duration as f32,
                    self.0.playback,
                )?;
                let Some(prefix) = self.source_cycle_prefix_enclosure(clock.cycle(), axes)? else {
                    return Ok(None);
                };
                local.transformed(&inverse.compose(&prefix)?, 1.)?
            };
            result = Some(match result {
                Some(previous) => previous.hull(&field),
                None => field,
            });
        }
        Ok(result)
    }

    fn source_partition_cuts(
        &self,
        times: [f64; 2],
        limit: usize,
    ) -> Result<Vec<f64>, AnimationError> {
        if times.into_iter().any(|v| !v.is_finite() || v < 0.)
            || times[1] < times[0]
            || limit == 0
            || limit > MAX_ROOT_ROTATION_SPANS
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let first =
            crate::enclose_root_cycle_phase(times[0], self.0.duration as f32, self.0.playback)?;
        let last =
            crate::enclose_root_cycle_phase(times[1], self.0.duration as f32, self.0.playback)?;
        if last.cycle() - first.cycle() > limit as u64 {
            return Err(AnimationError::RootRigidBudget);
        }
        let mut keys =
            self.0
                .translation
                .cuts(0., self.0.duration, crate::MAX_ROOT_ROTATION_CACHE_KEYS)?;
        keys.extend(self.0.rotation.phase_key_times());
        keys.push(self.0.duration);
        keys.sort_by(f64::total_cmp);
        keys.dedup();
        let mut cuts = vec![times[0], times[1]];
        for cycle in first.cycle()..=last.cycle() {
            for &key in &keys {
                let absolute = cycle as f64 * self.0.duration + key;
                if absolute < times[0] || absolute > times[1] {
                    continue;
                }
                if self.0.playback == Playback::Loop {
                    let clock = crate::enclose_root_cycle_phase(
                        absolute,
                        self.0.duration as f32,
                        Playback::Loop,
                    )?;
                    let expected = if key == self.0.duration {
                        (cycle + 1, 0.)
                    } else {
                        (cycle, key)
                    };
                    if (clock.cycle(), clock.phase()) != expected
                        || clock.exact_phase_bounds() != [clock.phase(); 2]
                    {
                        return Err(AnimationError::RootRigidBudget);
                    }
                }
                cuts.push(absolute);
                if cuts.len() > 2 * (limit + 1) {
                    return Err(AnimationError::RootRigidBudget);
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        if cuts.len() - 1 > limit {
            return Err(AnimationError::RootRigidBudget);
        }
        Ok(cuts)
    }

    /// Uniform source-motion cells split at both channels' keys and loop seams.
    /// Nonrepresentable authored boundaries reject rather than moving an event.
    pub fn source_delta_partition(
        &self,
        reference: f64,
        times: [f64; 2],
        axes: [bool; 3],
        limit: usize,
    ) -> Result<Option<Vec<([f64; 2], RootRigidEnclosure)>>, AnimationError> {
        if !reference.is_finite() || reference < 0. || reference > times[0] {
            return Err(AnimationError::InvalidSampleTime);
        }
        let cuts = self.source_partition_cuts(times, limit)?;
        let mut result = Vec::with_capacity(cuts.len() - 1);
        for cell in cuts.windows(2) {
            let times = [cell[0], cell[1]];
            let Some(enclosure) = self.source_delta_interval_enclosure(reference, times, axes)?
            else {
                return Ok(None);
            };
            result.push((times, enclosure));
        }
        Ok(Some(result))
    }
}

impl RootRigidCurve {
    /// Uniform geometric point discrepancy against a canonical screw reference.
    /// Covers source compilation and loop factors, but not runtime evaluation,
    /// world mapping, f32 publication or a velocity-blended fade of two clips.
    /// Independent hull subtraction is conservative and may require refinement.
    pub fn source_screw_point_error(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        candidate: &RootScrewEnclosurePath<'_>,
        limit: usize,
    ) -> Result<Option<([f64; 3], f64)>, AnimationError> {
        if candidate.path().duration() != times[1] - times[0] {
            return Err(AnimationError::InvalidSampleTime);
        }
        self.source_screw_point_error_refined(times, axes, point, candidate, limit, 1)
    }
}

impl RootRigidCurve {
    /// Synchronized source/candidate hull comparison with bounded subdivision.
    /// Canonical candidate time is mapped affinely over the exact source duration.
    pub fn source_screw_point_error_refined(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        candidate: &RootScrewEnclosurePath<'_>,
        limit: usize,
        subdivisions: usize,
    ) -> Result<Option<([f64; 3], f64)>, AnimationError> {
        if subdivisions == 0
            || subdivisions > MAX_ROOT_ROTATION_SPANS
            || candidate.path().duration() != times[1] - times[0]
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let Some(partition) = self.source_delta_partition(times[0], times, axes, limit)? else {
            return Ok(None);
        };
        if partition
            .len()
            .checked_mul(subdivisions)
            .is_none_or(|n| n > MAX_ROOT_ROTATION_SPANS)
        {
            return Err(AnimationError::RootRigidBudget);
        }
        let mut error = [0_f64; 3];
        let duration = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
        let mut compare =
            |source: RootRigidEnclosure, cell: [f64; 2]| -> Result<(), AnimationError> {
                let source_box = source.transform_point_box_bounds(point)?;
                let candidate_times = if times[0] == times[1] {
                    [0., 0.]
                } else {
                    let mapped = Scalar(cell[0], cell[1])
                        .sub(Scalar::exact(times[0]))?
                        .div_interval_positive(duration)?
                        .mul(Scalar::exact(candidate.path().duration()))?;
                    [mapped.0.max(0.), mapped.1.min(candidate.path().duration())]
                };
                let candidate_box = candidate.point_box_bounds_between(candidate_times, point)?;
                for axis in 0..3 {
                    let delta = Scalar(source_box[axis][0], source_box[axis][1])
                        .sub(Scalar(candidate_box[axis][0], candidate_box[axis][1]))?;
                    error[axis] = error[axis].max(delta.0.abs().max(delta.1.abs()));
                }
                Ok(())
            };
        if partition.is_empty() {
            compare(RootRigidEnclosure::IDENTITY, times)?;
        }
        for (cell, _) in partition {
            let mut start = cell[0];
            for index in 1..=subdivisions {
                let end = if index == subdivisions {
                    cell[1]
                } else {
                    cell[0] + (cell[1] - cell[0]) * (index as f64 / subdivisions as f64)
                };
                if end <= start {
                    return Err(AnimationError::RootRigidBudget);
                }
                let Some(source) =
                    self.source_delta_interval_enclosure(times[0], [start, end], axes)?
                else {
                    return Ok(None);
                };
                compare(source, [start, end])?;
                start = end;
            }
        }
        let mut radius = Scalar::exact(0.);
        for axis in error {
            radius = radius.add(Scalar::exact(axis))?;
        }
        Ok(Some((error, radius.1)))
    }
}

/// Borrowed uniform geometric source/reference discrepancy proof.
/// Runtime pose evaluation and world publication errors are separate obligations.
#[derive(Debug)]
pub struct RootSourceScrewPointCertificate<'a> {
    source: &'a RootRigidCurve,
    candidate: &'a RootScrewEnclosurePath<'a>,
    times: [f64; 2],
    extraction_axes: [bool; 3],
    point: [[f64; 2]; 3],
    axis_error: [f64; 3],
    radius: f64,
    subdivisions: usize,
    evaluated_cells: usize,
}
impl<'a> RootSourceScrewPointCertificate<'a> {
    pub fn source(&self) -> &RootRigidCurve {
        self.source
    }
    pub fn candidate(&self) -> &RootScrewEnclosurePath<'a> {
        self.candidate
    }
    pub fn times(&self) -> [f64; 2] {
        self.times
    }
    pub fn extraction_axes(&self) -> [bool; 3] {
        self.extraction_axes
    }
    pub fn point_bounds(&self) -> [[f64; 2]; 3] {
        self.point
    }
    pub fn axis_error_bounds(&self) -> [f64; 3] {
        self.axis_error
    }
    pub fn radius(&self) -> f64 {
        self.radius
    }
    pub fn subdivisions(&self) -> usize {
        self.subdivisions
    }
    pub fn evaluated_cells(&self) -> usize {
        self.evaluated_cells
    }
}
impl RootRigidCurve {
    /// Refines uniform geometric discrepancy until tolerance is proved.
    /// Budget counts all source comparison cells across refinement attempts.
    pub fn certify_source_screw_point_error<'a>(
        &'a self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        candidate: &'a RootScrewEnclosurePath<'a>,
        tolerance: f64,
        max_cells: usize,
    ) -> Result<Option<RootSourceScrewPointCertificate<'a>>, AnimationError> {
        if !tolerance.is_finite()
            || tolerance < 0.
            || max_cells == 0
            || max_cells > MAX_ROOT_ROTATION_SPANS
        {
            return Err(AnimationError::InvalidSampleTime);
        }
        let Some(partition) = self.source_delta_partition(times[0], times, axes, max_cells)? else {
            return Ok(None);
        };
        let count = partition.len().max(1);
        let mut subdivisions = 1_usize;
        let mut evaluated_cells = 0_usize;
        loop {
            let work = count
                .checked_mul(subdivisions)
                .ok_or(AnimationError::RootRigidBudget)?;
            evaluated_cells = evaluated_cells
                .checked_add(work)
                .ok_or(AnimationError::RootRigidBudget)?;
            if evaluated_cells > max_cells {
                return Err(AnimationError::RootRigidBudget);
            }
            let Some((axis_error, radius)) = self.source_screw_point_error_refined(
                times,
                axes,
                point,
                candidate,
                max_cells,
                subdivisions,
            )?
            else {
                return Ok(None);
            };
            if radius <= tolerance {
                return Ok(Some(RootSourceScrewPointCertificate {
                    source: self,
                    candidate,
                    times,
                    extraction_axes: axes,
                    point,
                    axis_error,
                    radius,
                    subdivisions,
                    evaluated_cells,
                }));
            }
            subdivisions = subdivisions
                .checked_mul(2)
                .ok_or(AnimationError::RootRigidBudget)?;
        }
    }
}

impl RootSourceScrewPointCertificate<'_> {
    /// Geometric discrepancy plus direct f32 rounding in a fixed common frame.
    /// Does not include runtime quaternion/matrix evaluation before conversion.
    pub fn mapped_f32_geometric_error(
        &self,
        frame: &RootRigidEnclosure,
        scale: RootUniformScaleEnclosure,
    ) -> Result<([f64; 3], f64), AnimationError> {
        let (_, rotation) = frame.vectors();
        let local = self.axis_error.map(|error| Scalar(-error, error));
        let mut difference = rotate(rotation, local)?;
        for axis in &mut difference {
            *axis = axis.mul(scale.value)?;
        }
        let candidate = self.candidate.whole_path_point_box_bounds(self.point)?;
        let world = frame.similarity_point_box_bounds_enclosed(candidate, scale)?;
        let (publication, _) = RootRigidEnclosure::enclosed_f32_publication_error(world)?;
        let mut axes = [0.; 3];
        let mut radius = Scalar::exact(0.);
        for axis in 0..3 {
            axes[axis] = Scalar::exact(difference[axis].0.abs().max(difference[axis].1.abs()))
                .add(Scalar::exact(publication[axis]))?
                .1;
            radius = radius.add(Scalar::exact(axes[axis]))?;
        }
        Ok((axes, radius.1))
    }
}

impl RootRigidEnclosure {
    /// Uniform rounding cap for glam 0.33.7 DQuat*DVec3, then scale and offset.
    /// Inputs are stored f64 values within these boxes. The reference is the exact
    /// homogeneous quaternion polynomial, not a separately normalized rotation.
    /// Quaternion compilation/normalization errors must be bounded separately.
    pub fn stored_similarity_evaluation_error(
        q: [[f64; 2]; 4],
        point: [[f64; 2]; 3],
        scale: [f64; 2],
        offset: [[f64; 2]; 3],
    ) -> Result<[f64; 3], AnimationError> {
        compilation::stored_similarity_evaluation_error(q, point, scale, offset)
    }
}

impl RootRigidCurve {
    /// Uniform source-to-runtime image error for phase() then translation+q*point.
    /// One continuous shared local-key interval; loop prefix evaluation is separate.
    pub fn phase_point_evaluation_error_bounds(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
    ) -> Result<Option<[f64; 3]>, AnimationError> {
        let Some(source) = self.source_phase_interval_enclosure(times, axes)? else {
            return Ok(None);
        };
        let position = self
            .0
            .translation
            .source_position_bounds(times[0], times[1])?;
        let position_error = self
            .0
            .translation
            .interval_evaluation_error_bounds(times[0], times[1])?;
        let Some(rotation_error) = self
            .0
            .rotation
            .cubic_relative_interval_evaluation_error_bounds(times[0], times[1])?
        else {
            return Ok(None);
        };
        Ok(Some(compilation::root_phase_point_error(
            position,
            position_error,
            source.rotation_bounds(),
            rotation_error,
            self.0.origin,
            self.0.bind,
            axes,
            point,
        )?))
    }
}

impl RootRigidCurve {
    /// Fixed runtime rigid loop-prefix discrepancy against its exact source.
    pub fn cycle_prefix_evaluation_error_bounds(
        &self,
        cycles: u64,
        axes: [bool; 3],
    ) -> Result<Option<([f64; 3], [f64; 4])>, AnimationError> {
        let Some(source) = self.source_cycle_prefix_enclosure(cycles, axes)? else {
            return Ok(None);
        };
        let evaluated = power(self.phase(self.0.duration, axes)?, cycles)?;
        let mut translation = [0.; 3];
        let mut rotation = [0.; 4];
        for i in 0..3 {
            let delta = Scalar(source.translation[i][0], source.translation[i][1])
                .sub(Scalar::exact(evaluated.translation[i]))?;
            translation[i] = delta.0.abs().max(delta.1.abs());
        }
        for i in 0..4 {
            let delta = Scalar(source.rotation[i][0], source.rotation[i][1])
                .sub(Scalar::exact(evaluated.rotation.to_array()[i]))?;
            rotation[i] = delta.0.abs().max(delta.1.abs());
        }
        Ok(Some((translation, rotation)))
    }
    /// Uniform sample() then translation+rotation*point error at an explicit cycle.
    /// Wall-clock phase and world/f32 publication remain separate.
    pub fn cycle_point_evaluation_error_bounds(
        &self,
        cycles: u64,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
    ) -> Result<Option<[f64; 3]>, AnimationError> {
        let Some(prefix) = self.source_cycle_prefix_enclosure(cycles, axes)? else {
            return Ok(None);
        };
        let Some((prefix_t_error, prefix_q_error)) =
            self.cycle_prefix_evaluation_error_bounds(cycles, axes)?
        else {
            return Ok(None);
        };
        let Some(local) = self.source_phase_interval_enclosure(times, axes)? else {
            return Ok(None);
        };
        let Some(local_t_error) =
            self.phase_point_evaluation_error_bounds(times, axes, [[0.; 2]; 3])?
        else {
            return Ok(None);
        };
        let Some(local_q_error) = self
            .0
            .rotation
            .cubic_relative_interval_evaluation_error_bounds(times[0], times[1])?
        else {
            return Ok(None);
        };
        Ok(Some(compilation::composed_rigid_point_error(
            &prefix,
            prefix_t_error,
            prefix_q_error,
            &local,
            local_t_error,
            local_q_error,
            point,
        )?))
    }
}

impl RootRigidCurve {
    /// Uniform source-to-published point discrepancy for an explicit loop cell.
    /// Matches sample point evaluation, then actual_scale*(actual_frame.rotation*p)
    /// +actual_frame.translation, then direct f32 cast. Fixed source frames only.
    pub fn cycle_world_point_evaluation_error_bounds(
        &self,
        cycles: u64,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        source_frame: &RootRigidEnclosure,
        source_scale: RootUniformScaleEnclosure,
        actual_frame: RootRigidTransform,
        actual_scale: f64,
    ) -> Result<Option<([f64; 3], f64)>, AnimationError> {
        let Some(prefix) = self.source_cycle_prefix_enclosure(cycles, axes)? else {
            return Ok(None);
        };
        let Some(local) = self.source_phase_interval_enclosure(times, axes)? else {
            return Ok(None);
        };
        let source_point = prefix.compose(&local)?.transform_point_box_bounds(point)?;
        let Some(point_error) =
            self.cycle_point_evaluation_error_bounds(cycles, times, axes, point)?
        else {
            return Ok(None);
        };
        let (before_cast, actual_box) = compilation::mapped_point_runtime_error(
            source_point,
            point_error,
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
        Ok(Some((axes, radius.1)))
    }
}

impl RootRigidCurve {
    /// Uniform sample/map/f32 error on an absolute wall-clock interval.
    /// Splits channel keys/loops and refines unresolved normalization domains.
    /// Seam endpoints also certify the actual next-cycle sampling branch.
    pub fn world_point_evaluation_error_bounds(
        &self,
        times: [f64; 2],
        axes: [bool; 3],
        point: [[f64; 2]; 3],
        source_frame: &RootRigidEnclosure,
        source_scale: RootUniformScaleEnclosure,
        actual_frame: RootRigidTransform,
        actual_scale: f64,
        max_cells: usize,
    ) -> Result<Option<([f64; 3], f64)>, AnimationError> {
        let Some(partition) = self.source_delta_partition(times[0], times, axes, max_cells)? else {
            return Ok(None);
        };
        let mut pending: Vec<[f64; 2]> = partition.into_iter().map(|(cell, _)| cell).collect();
        if pending.is_empty() {
            pending.push(times);
        }
        let mut result = [0_f64; 3];
        let mut visited = 0_usize;
        let mut merge = |error: [f64; 3]| {
            for i in 0..3 {
                result[i] = result[i].max(error[i]);
            }
        };
        while let Some(cell) = pending.pop() {
            visited += 1;
            if visited > max_cells {
                return Err(AnimationError::RootRigidBudget);
            }
            let clock =
                crate::enclose_root_cycle_phase(cell[0], self.0.duration as f32, self.0.playback)?;
            let phases = crate::root_clock::root_segment_phases(
                cell[0],
                cell[1],
                self.0.duration as f32,
                self.0.playback,
            )?;
            match self.cycle_world_point_evaluation_error_bounds(
                clock.cycle(),
                phases,
                axes,
                point,
                source_frame,
                source_scale,
                actual_frame,
                actual_scale,
            ) {
                Ok(Some((error, _))) => merge(error),
                Ok(None) => return Ok(None),
                Err(AnimationError::InvalidRootRotationCurve) => {
                    let mid = cell[0] + (cell[1] - cell[0]) * 0.5;
                    if mid <= cell[0] || mid >= cell[1] {
                        return Err(AnimationError::RootRigidBudget);
                    }
                    pending.push([mid, cell[1]]);
                    pending.push([cell[0], mid]);
                    continue;
                }
                Err(error) => return Err(error),
            }
            let end_clock =
                crate::enclose_root_cycle_phase(cell[1], self.0.duration as f32, self.0.playback)?;
            if end_clock.cycle() != clock.cycle() {
                visited += 1;
                if visited > max_cells {
                    return Err(AnimationError::RootRigidBudget);
                }
                let Some((error, _)) = self.cycle_world_point_evaluation_error_bounds(
                    end_clock.cycle(),
                    [end_clock.phase(); 2],
                    axes,
                    point,
                    source_frame,
                    source_scale,
                    actual_frame,
                    actual_scale,
                )?
                else {
                    return Ok(None);
                };
                merge(error);
            }
        }
        let mut radius = Scalar::exact(0.);
        for error in result {
            radius = radius.add(Scalar::exact(error))?;
        }
        Ok(Some((result, radius.1)))
    }
}

#[cfg(test)]
mod stored_normalization_tests {
    use super::*;
    #[test]
    fn stored_normalization_rounding_covers_uniform_boxes_and_rejects_singular_inputs() {
        let domain = [[-0.125, 0.125], [-0.25, 0.25], [-0.5, 0.5], [0.75, 1.25]];
        let cap = RootRigidEnclosure::stored_quaternion_normalization_error_bounds(domain).unwrap();
        assert!(cap.iter().all(|v| v.is_finite() && *v > 0. && *v < 1e-12));
        for input in [
            [0., 1e-20, 0., 1.],
            [0.125, -0.25, 0.5, 0.75],
            [-0.125, 0.25, -0.5, 1.25],
            [0., 0., 0., 1.],
        ] {
            let actual = DQuat::from_array(input).normalize().to_array();
            println!("STORED_NORMALIZATION {:?}", (input, actual, cap));
        }
        assert!(
            RootRigidEnclosure::stored_quaternion_normalization_error_bounds([[0.; 2]; 4]).is_err()
        );
        assert!(
            RootRigidEnclosure::stored_quaternion_normalization_error_bounds([[-1., 1.]; 4])
                .is_err()
        );
        assert!(
            RootRigidEnclosure::stored_quaternion_normalization_error_bounds([[f64::NAN; 2]; 4])
                .is_err()
        );
        assert!(
            RootRigidEnclosure::stored_quaternion_normalization_error_bounds([[f64::MAX; 2]; 4])
                .is_err()
        );
    }
}

#[cfg(test)]
mod normalized_composition_rounding_tests {
    use super::*;
    #[test]
    fn normalized_composition_uses_relational_norm_proof_for_sign_changing_boxes() {
        let b = DQuat::from_array([0.5, -0.5, 0.5, 0.5]);
        let cap = quaternion_normalized_composition_uniform_error([0.; 4], b.to_array(), [0.; 4])
            .unwrap();
        assert!(cap.iter().all(|v| v.is_finite() && *v > 0. && *v < 1e-12));
        for a in [
            DQuat::IDENTITY,
            DQuat::from_array([0.5, 0.5, -0.5, 0.5]),
            DQuat::from_array([-0.5, 0.5, 0.5, -0.5]),
        ] {
            let actual = (a * b).normalize().to_array();
            println!(
                "NORMALIZED_COMPOSITION {:?}",
                (a.to_array(), b.to_array(), actual, cap)
            );
        }
        let perturbed =
            quaternion_normalized_composition_uniform_error([1e-4; 4], b.to_array(), [0.; 4])
                .unwrap();
        assert!(perturbed.iter().zip(cap).all(|(v, clean)| *v > clean));
        assert!(
            quaternion_normalized_composition_uniform_error([0.1; 4], b.to_array(), [0.; 4])
                .is_err()
        );
    }
}
