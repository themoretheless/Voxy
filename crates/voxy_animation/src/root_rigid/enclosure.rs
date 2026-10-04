//! Outward-rounded small-angle exponential enclosures.
//! Assumes IEEE-754 round-to-nearest basic operations and gradual underflow.
//! No platform sin/cos, argument reduction or sampled error estimates are used.
use super::*;
mod twist;
mod cubic;
mod accumulation;
mod rates;
mod points;
mod cache;
pub use cache::RootScrewEnclosurePath;
pub use rates::RootAngularDerivativeBounds;
pub use accumulation::RootRigidErrorAccumulator;
pub use twist::{RootRigidTwistEnclosure,RootTwistErrorBounds};

#[derive(Clone, Copy, Debug)]
pub struct RootRigidEnclosure {
    translation: [[f64; 2]; 3],
    /// Quaternion components in x,y,z,w order. No floating normalization applied.
    rotation: [[f64; 2]; 4],
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
    fn add(self, b: Self) -> Result<Self, AnimationError> {
        Self::rounded(self.0 + b.0, self.1 + b.1)
    }
    fn sub(self, b: Self) -> Result<Self, AnimationError> {
        Self::rounded(self.0 - b.1, self.1 - b.0)
    }
    fn mul(self, b: Self) -> Result<Self, AnimationError> {
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
        Self::rounded(self.0 / b, self.1 / b)
    }
    fn sqrt_positive(self) -> Result<Self, AnimationError> {
        if self.0 <= 0. {
            return Err(AnimationError::NumericalOverflow);
        }
        Self::rounded(self.0.sqrt(), self.1.sqrt())
    }
    fn div_interval_positive(self, b: Self) -> Result<Self, AnimationError> {
        if b.0 <= 0. {
            return Err(AnimationError::NumericalOverflow);
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
fn cross(a: [Scalar; 3], b: [Scalar; 3]) -> Result<[Scalar; 3], AnimationError> {
    Ok([
        a[1].mul(b[2])?.sub(a[2].mul(b[1])?)?,
        a[2].mul(b[0])?.sub(a[0].mul(b[2])?)?,
        a[0].mul(b[1])?.sub(a[1].mul(b[0])?)?,
    ])
}
fn rotate(q: [Scalar; 4], v: [Scalar; 3]) -> Result<[Scalar; 3], AnimationError> {
    let vector = [q[0], q[1], q[2]];
    let c = cross(vector, v)?;
    let twice = [
        c[0].mul(Scalar::exact(2.))?,
        c[1].mul(Scalar::exact(2.))?,
        c[2].mul(Scalar::exact(2.))?,
    ];
    let second = cross(vector, twice)?;
    Ok([
        v[0].add(q[3].mul(twice[0])?)?.add(second[0])?,
        v[1].add(q[3].mul(twice[1])?)?.add(second[1])?,
        v[2].add(q[3].mul(twice[2])?)?.add(second[2])?,
    ])
}
impl RootRigidEnclosure {
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
        let rotated = rotate(a, bt)?;
        let translation = [
            at[0].add(rotated[0])?.array(),
            at[1].add(rotated[1])?.array(),
            at[2].add(rotated[2])?.array(),
        ];
        let rotation = [
            a[3].mul(b[0])?
                .add(a[0].mul(b[3])?)?
                .add(a[1].mul(b[2])?)?
                .sub(a[2].mul(b[1])?)?
                .array(),
            a[3].mul(b[1])?
                .sub(a[0].mul(b[2])?)?
                .add(a[1].mul(b[3])?)?
                .add(a[2].mul(b[0])?)?
                .array(),
            a[3].mul(b[2])?
                .add(a[0].mul(b[1])?)?
                .sub(a[1].mul(b[0])?)?
                .add(a[2].mul(b[3])?)?
                .array(),
            a[3].mul(b[3])?
                .sub(a[0].mul(b[0])?)?
                .sub(a[1].mul(b[1])?)?
                .sub(a[2].mul(b[2])?)?
                .array(),
        ];
        Ok(Self {
            translation,
            rotation,
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
fn series(
    x: Scalar,
    initial: Scalar,
    denominator: impl Fn(u32) -> u32,
) -> Result<Scalar, AnimationError> {
    let mut term = initial;
    let mut sum = initial;
    for n in 1..=7 {
        term = term.mul(x)?.div_positive(f64::from(denominator(n)))?;
        term = Scalar(-term.1, -term.0);
        sum = sum.add(term)?;
    }
    let remainder = term.mul(x)?.div_positive(f64::from(denominator(8)))?;
    let radius = remainder.0.abs().max(remainder.1.abs());
    sum.add(Scalar(-radius, radius))
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
    pub(super) fn increment_between_enclosure(self, start: f64, end: f64) -> Result<RootRigidEnclosure, AnimationError> {
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
        if !self.linear.is_finite() || !self.angular.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        if dt.0 == 0. && dt.1 == 0. {
            return Ok(RootRigidEnclosure {
                translation: [[0., 0.]; 3],
                rotation: [[0., 0.], [0., 0.], [0., 0.], [1., 1.]],
            });
        }
        let mut omega = [Scalar::exact(0.); 3];
        let mut displacement = omega;
        for i in 0..3 {
            omega[i] = Scalar::exact(self.angular[i]).mul(dt)?;
            displacement[i] = Scalar::exact(self.linear[i]).mul(dt)?;
        }
        let mut x = omega[0]
            .square()?
            .add(omega[1].square()?)?
            .add(omega[2].square()?)?;
        x.0 = x.0.max(0.);
        if x.1 > 1. {
            return Err(AnimationError::RootRigidBudget);
        }
        let a = series(x, Scalar::exact(0.5), |n| (2 * n + 1) * (2 * n + 2))?;
        let b = series(x, Scalar::exact(1.).div_positive(6.)?, |n| {
            (2 * n + 2) * (2 * n + 3)
        })?;
        let q = series(x, Scalar::exact(0.5), |n| 4 * (2 * n) * (2 * n + 1))?;
        let w = series(x, Scalar::exact(1.), |n| 4 * (2 * n - 1) * (2 * n))?;
        let first = cross(omega, displacement)?;
        let second = cross(omega, first)?;
        let mut translation = [[0.; 2]; 3];
        let mut rotation = [[0.; 2]; 4];
        for i in 0..3 {
            translation[i] = displacement[i]
                .add(a.mul(first[i])?)?
                .add(b.mul(second[i])?)?
                .array();
            rotation[i] = omega[i].mul(q)?.array();
        }
        rotation[3] = w.array();
        Ok(RootRigidEnclosure {
            translation,
            rotation,
        })
    }
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
        let cache=path.prepare_screw_enclosures(4).unwrap();
        assert!(std::ptr::eq(cache.path(),&path));
        assert!(path.prepare_screw_enclosures(3).is_err());
        assert!(cache.sample(4,0.5).is_err());
        assert!(cache.sample(0,f64::NAN).is_err());
        for index in 0..4 {
            for fraction in [0., 0.37, 1.] {
                let enclosure = path.screw_field_enclosure(index, fraction, 4).unwrap();
                let prepared=cache.sample(index,fraction).unwrap();
                println!("CACHED_SCREW_ENCLOSURE {:?}",(&definitions,index,fraction,point.to_array(),
                    prepared.translation_bounds(),prepared.rotation_bounds(),prepared.transform_point(point).unwrap()));
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
