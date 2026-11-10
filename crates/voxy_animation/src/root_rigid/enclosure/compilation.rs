//! Stored-key compilation discrepancy proofs; runtime evaluation is separate.
use super::*;
mod runtime_rounding;
use runtime_rounding::normalization_f32_uniform_error;
pub(super) use runtime_rounding::{
    composed_rigid_point_error, mapped_point_runtime_error, normalization_uniform_error,
    physical_body_runtime_domains, physical_body_runtime_error, physical_rotation_runtime_error,
    retarget_rotation_runtime_error, retarget_translation_runtime_error, rigid_point_runtime_error,
    root_phase_point_error, snap_center_reconstruction_error,
    stored_quaternion_normalization_error, stored_similarity_evaluation_error,
};
pub(crate) use runtime_rounding::{
    quaternion_composition_uniform_error, quaternion_normalized_composition_uniform_error,
};
/// Exact-source polynomial coefficients, before rounded cache compilation.
pub(crate) fn translation_coefficient_error_bounds(
    origin: DVec3,
    value: DVec3,
    end: Option<DVec3>,
    times: [f64; 2],
    tangents: [DVec3; 2],
    mode: crate::Interpolation,
    stored: [DVec3; 4],
) -> Result<[f64; 3], AnimationError> {
    let mut error = [0_f64; 3];
    for axis in 0..3 {
        let value = Scalar::exact(value[axis]).sub(Scalar::exact(origin[axis]))?;
        let mut coefficients = [
            value,
            Scalar::exact(0.),
            Scalar::exact(0.),
            Scalar::exact(0.),
        ];
        if let Some(end) = end {
            let end = Scalar::exact(end[axis]).sub(Scalar::exact(origin[axis]))?;
            match mode {
                crate::Interpolation::Step => {}
                crate::Interpolation::Linear => {
                    coefficients[1] = end.sub(value)?;
                }
                crate::Interpolation::CubicSpline => {
                    let dt = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
                    let out = Scalar::exact(tangents[0][axis]).mul(dt)?;
                    let incoming = Scalar::exact(tangents[1][axis]).mul(dt)?;
                    coefficients[1] = out;
                    coefficients[2] = end
                        .sub(value)?
                        .mul(Scalar::exact(3.))?
                        .sub(out.mul(Scalar::exact(2.))?)?
                        .sub(incoming)?;
                    coefficients[3] = value
                        .sub(end)?
                        .mul(Scalar::exact(2.))?
                        .add(out)?
                        .add(incoming)?;
                }
            }
        }
        let mut bound = Scalar::exact(0.);
        for coefficient in 0..4 {
            let difference =
                coefficients[coefficient].sub(Scalar::exact(stored[coefficient][axis]))?;
            bound = bound.add(Scalar::exact(difference.0.abs().max(difference.1.abs())))?;
        }
        error[axis] = bound.1;
    }
    Ok(error)
}

/// Restriction and power-to-Bernstein discrepancy at exact supplied key times.
pub(crate) fn translation_piece_error_bounds(
    coefficients: [DVec3; 4],
    compilation: [f64; 3],
    key_times: [f64; 2],
    interval: [f64; 2],
    stored: [DVec3; 4],
) -> Result<[f64; 3], AnimationError> {
    let delta = Scalar::exact(key_times[1]).sub(Scalar::exact(key_times[0]))?;
    let u = Scalar::exact(interval[0])
        .sub(Scalar::exact(key_times[0]))?
        .div_interval_positive(delta)?;
    let v = Scalar::exact(interval[1])
        .sub(Scalar::exact(interval[0]))?
        .div_interval_positive(delta)?;
    let mut error = [0_f64; 3];
    for axis in 0..3 {
        if !compilation[axis].is_finite() || compilation[axis] < 0. {
            return Err(AnimationError::NumericalOverflow);
        }
        let mut c = [Scalar::exact(0.); 4];
        for i in 0..4 {
            let lower =
                Scalar::exact(coefficients[i][axis]).sub(Scalar::exact(compilation[axis]))?;
            let upper =
                Scalar::exact(coefficients[i][axis]).add(Scalar::exact(compilation[axis]))?;
            c[i] = Scalar(lower.0, upper.1);
        }
        let a = c[3]
            .mul(u)?
            .add(c[2])?
            .mul(u)?
            .add(c[1])?
            .mul(u)?
            .add(c[0])?;
        let b = c[1]
            .add(c[2].mul(Scalar::exact(2.))?.mul(u)?)?
            .add(c[3].mul(Scalar::exact(3.))?.mul(u)?.mul(u)?)?
            .mul(v)?;
        let d = c[2]
            .add(c[3].mul(Scalar::exact(3.))?.mul(u)?)?
            .mul(v)?
            .mul(v)?;
        let e = c[3].mul(v)?.mul(v)?.mul(v)?;
        let controls = [
            a,
            a.add(b.div_positive(3.)?)?,
            a.add(b.mul(Scalar::exact(2.))?.div_positive(3.)?)?
                .add(d.div_positive(3.)?)?,
            a.add(b)?.add(d)?.add(e)?,
        ];
        for i in 0..4 {
            let discrepancy = controls[i].sub(Scalar::exact(stored[i][axis]))?;
            error[axis] = error[axis].max(discrepancy.0.abs().max(discrepancy.1.abs()));
        }
    }
    Ok(error)
}

/// Discrepancy of rounded key normalization from the real normalized stored key.
pub(crate) fn quaternion_normalization_error_bounds(
    source: [f64; 4],
    evaluated: [f64; 4],
) -> Result<[f64; 4], AnimationError> {
    if source
        .iter()
        .chain(evaluated.iter())
        .any(|value| !value.is_finite())
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let q = source.map(Scalar::exact);
    let norm = q[0]
        .square()?
        .add(q[1].square()?)?
        .add(q[2].square()?)?
        .add(q[3].square()?)?
        .sqrt_positive()?;
    let mut errors = [0.; 4];
    for axis in 0..4 {
        let exact = q[axis].div_interval_positive(norm)?;
        let delta = exact.sub(Scalar::exact(evaluated[axis]))?;
        errors[axis] = delta.0.abs().max(delta.1.abs());
    }
    Ok(errors)
}

pub(crate) fn quaternion_cubic_control_error_bounds(
    keys: [[f64; 4]; 2],
    tangents: [[f64; 4]; 2],
    times: [f64; 2],
    stored: [[f64; 4]; 4],
) -> Result<[f64; 4], AnimationError> {
    let duration = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
    if duration.0 <= 0. {
        return Err(AnimationError::InvalidSampleTime);
    }
    let factor = duration.div_positive(3.)?;
    let mut errors = [0_f64; 4];
    for axis in 0..4 {
        let first = Scalar::exact(keys[0][axis]);
        let last = Scalar::exact(keys[1][axis]);
        let controls = [
            first,
            first.add(Scalar::exact(tangents[0][axis]).mul(factor)?)?,
            last.sub(Scalar::exact(tangents[1][axis]).mul(factor)?)?,
            last,
        ];
        for control in 0..4 {
            let delta = controls[control].sub(Scalar::exact(stored[control][axis]))?;
            errors[axis] = errors[axis].max(delta.0.abs().max(delta.1.abs()));
        }
    }
    Ok(errors)
}

/// Uniform source-vs-cached normalized polynomial discrepancy on [0,1].
pub(crate) fn quaternion_cubic_normalized_error_bounds(
    control: [[f64; 4]; 4],
    raw_error: [f64; 4],
) -> Result<[f64; 4], AnimationError> {
    if control.iter().flatten().any(|value| !value.is_finite())
        || raw_error
            .iter()
            .any(|value| !value.is_finite() || *value < 0.)
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let mut norm_squared = Scalar::exact(0.);
    let mut discrepancy = Scalar::exact(0.);
    for axis in 0..4 {
        let lo = control
            .iter()
            .map(|q| q[axis])
            .fold(f64::INFINITY, f64::min);
        let hi = control
            .iter()
            .map(|q| q[axis])
            .fold(f64::NEG_INFINITY, f64::max);
        let distance = if lo > 0. {
            lo
        } else if hi < 0. {
            -hi
        } else {
            0.
        };
        norm_squared = norm_squared.add(Scalar::exact(distance).square()?)?;
        discrepancy = discrepancy.add(Scalar::exact(raw_error[axis]))?;
    }
    let norm = norm_squared.sqrt_positive()?;
    // Bernstein convexity bounds raw source-cached discrepancy by its L1 cap.
    // Reverse triangle inequality then proves the source stays nonsingular too.
    if norm.0 <= discrepancy.1 {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let bound = discrepancy
        .mul(Scalar::exact(2.))?
        .div_interval_positive(norm)?
        .1;
    Ok(std::array::from_fn(|axis| {
        if raw_error[axis] == 0. && control.iter().all(|q| q[axis] == 0.) {
            0.
        } else {
            bound
        }
    }))
}

fn split_controls(
    control: [[Scalar; 4]; 4],
    t: Scalar,
) -> Result<([[Scalar; 4]; 4], [[Scalar; 4]; 4]), AnimationError> {
    let mut level = control;
    let mut left = control;
    let mut right = control;
    for depth in 1..4 {
        for index in 0..4 - depth {
            for component in 0..4 {
                level[index][component] =
                    cubic::interpolate(level[index][component], level[index + 1][component], t)?;
            }
        }
        left[depth] = level[0];
        right[3 - depth] = level[3 - depth];
    }
    Ok((left, right))
}

pub(crate) fn quaternion_cubic_restriction_error_bounds(
    control: [[f64; 4]; 4],
    raw_error: [f64; 4],
    times: [f64; 2],
    interval: [f64; 2],
    stored: [[f64; 4]; 4],
) -> Result<[f64; 4], AnimationError> {
    if interval[0] < times[0]
        || interval[1] > times[1]
        || interval[1] <= interval[0]
        || raw_error
            .iter()
            .any(|value| !value.is_finite() || *value < 0.)
    {
        return Err(AnimationError::InvalidSampleTime);
    }
    let duration = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
    let mut end = Scalar::exact(interval[1])
        .sub(Scalar::exact(times[0]))?
        .div_interval_positive(duration)?;
    end.0 = end.0.max(0.);
    end.1 = end.1.min(1.);
    let mut enclosed = [[Scalar::exact(0.); 4]; 4];
    for i in 0..4 {
        for axis in 0..4 {
            let lo = Scalar::exact(control[i][axis]).sub(Scalar::exact(raw_error[axis]))?;
            let hi = Scalar::exact(control[i][axis]).add(Scalar::exact(raw_error[axis]))?;
            enclosed[i][axis] = Scalar(lo.0, hi.1);
        }
    }
    let first = split_controls(enclosed, end)?.0;
    let restricted = if interval[0] == times[0] {
        first
    } else {
        let mut ratio = Scalar::exact(interval[0])
            .sub(Scalar::exact(times[0]))?
            .div_interval_positive(Scalar::exact(interval[1]).sub(Scalar::exact(times[0]))?)?;
        ratio.0 = ratio.0.max(0.);
        ratio.1 = ratio.1.min(1.);
        split_controls(first, ratio)?.1
    };
    let mut error = [0_f64; 4];
    for i in 0..4 {
        for axis in 0..4 {
            let delta = restricted[i][axis].sub(Scalar::exact(stored[i][axis]))?;
            error[axis] = error[axis].max(delta.0.abs().max(delta.1.abs()));
        }
    }
    quaternion_cubic_normalized_error_bounds(stored, error)
}

pub(crate) fn quaternion_cubic_phase_evaluation_error_bounds(
    control: [[f64; 4]; 4],
    raw_error: [f64; 4],
    times: [f64; 2],
    phase: f64,
    evaluated: [f64; 4],
) -> Result<[f64; 4], AnimationError> {
    if !phase.is_finite()
        || phase < times[0]
        || phase > times[1]
        || raw_error
            .iter()
            .any(|value| !value.is_finite() || *value < 0.)
        || evaluated.iter().any(|value| !value.is_finite())
    {
        return Err(AnimationError::InvalidSampleTime);
    }
    let mut u = Scalar::exact(phase)
        .sub(Scalar::exact(times[0]))?
        .div_interval_positive(Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?)?;
    u.0 = u.0.max(0.);
    u.1 = u.1.min(1.);
    let mut enclosed = [[Scalar::exact(0.); 4]; 4];
    for i in 0..4 {
        for axis in 0..4 {
            let lo = Scalar::exact(control[i][axis]).sub(Scalar::exact(raw_error[axis]))?;
            let hi = Scalar::exact(control[i][axis]).add(Scalar::exact(raw_error[axis]))?;
            enclosed[i][axis] = Scalar(lo.0, hi.1);
        }
    }
    let value = split_controls(enclosed, u)?.0[3];
    let norm = value[0]
        .square()?
        .add(value[1].square()?)?
        .add(value[2].square()?)?
        .add(value[3].square()?)?
        .sqrt_positive()?;
    let mut error = [0.; 4];
    for axis in 0..4 {
        let normalized = value[axis].div_interval_positive(norm)?;
        let discrepancy = normalized.sub(Scalar::exact(evaluated[axis]))?;
        error[axis] = discrepancy.0.abs().max(discrepancy.1.abs());
    }
    Ok(error)
}

pub(crate) fn translation_phase_evaluation_error_bounds(
    coefficients: [DVec3; 4],
    compilation: [f64; 3],
    times: [f64; 2],
    phase: f64,
    evaluated: DVec3,
) -> Result<[f64; 3], AnimationError> {
    if !phase.is_finite()
        || phase < times[0]
        || phase > times[1]
        || !evaluated.is_finite()
        || compilation
            .iter()
            .any(|error| !error.is_finite() || *error < 0.)
    {
        return Err(AnimationError::InvalidSampleTime);
    }
    let mut u = Scalar::exact(phase)
        .sub(Scalar::exact(times[0]))?
        .div_interval_positive(Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?)?;
    u.0 = u.0.max(0.);
    u.1 = u.1.min(1.);
    let mut error = [0.; 3];
    for axis in 0..3 {
        let mut c = [Scalar::exact(0.); 4];
        for i in 0..4 {
            let lo = Scalar::exact(coefficients[i][axis]).sub(Scalar::exact(compilation[axis]))?;
            let hi = Scalar::exact(coefficients[i][axis]).add(Scalar::exact(compilation[axis]))?;
            c[i] = Scalar(lo.0, hi.1);
        }
        let position = c[3]
            .mul(u)?
            .add(c[2])?
            .mul(u)?
            .add(c[1])?
            .mul(u)?
            .add(c[0])?;
        let delta = position.sub(Scalar::exact(evaluated[axis]))?;
        error[axis] = delta.0.abs().max(delta.1.abs());
    }
    Ok(error)
}

fn absolute_upper(value: Scalar) -> f64 {
    value.0.abs().max(value.1.abs())
}
fn expanded(value: Scalar, error: f64) -> Result<Scalar, AnimationError> {
    let lo = Scalar::exact(value.0).sub(Scalar::exact(error))?;
    let hi = Scalar::exact(value.1).add(Scalar::exact(error))?;
    Ok(Scalar(lo.0, hi.1))
}
/// A full adjacent f64 spacing bounds nearest rounding over this result range.
fn rounding_error(value: Scalar) -> Result<f64, AnimationError> {
    if !value.is_finite() {
        return Err(AnimationError::NumericalOverflow);
    }
    if value.0 == value.1 {
        return Ok(0.);
    }
    let magnitude = absolute_upper(value);
    let upper = magnitude.next_up();
    if !upper.is_finite() {
        return Err(AnimationError::NumericalOverflow);
    }
    Ok(upper - magnitude)
}

pub(crate) fn translation_interval_evaluation_error_bounds(
    coefficients: [DVec3; 4],
    compilation: [f64; 3],
    times: [f64; 2],
    interval: [f64; 2],
) -> Result<[f64; 3], AnimationError> {
    if interval
        .iter()
        .chain(times.iter())
        .any(|value| !value.is_finite())
        || interval[0] < times[0]
        || interval[1] > times[1]
        || interval[0] > interval[1]
        || compilation
            .iter()
            .any(|error| !error.is_finite() || *error < 0.)
    {
        return Err(AnimationError::InvalidSampleTime);
    }
    let tracked = rounded_parameter(times, interval)?;
    let parameter = tracked.value;
    let parameter_error = tracked.error.1;
    let mut result = [0.; 3];
    for axis in 0..3 {
        let mut value = Scalar::exact(coefficients[3][axis]);
        let mut error = Scalar::exact(0.);
        for coefficient in (0..3).rev() {
            let product = value.mul(parameter)?;
            let product_error = Scalar::exact(absolute_upper(value))
                .mul(Scalar::exact(parameter_error))?
                .add(Scalar::exact(absolute_upper(parameter)).mul(error)?)?
                .add(error.mul(Scalar::exact(parameter_error))?)?;
            error = product_error.add(Scalar::exact(rounding_error(expanded(
                product,
                product_error.1,
            )?)?))?;
            value = product.add(Scalar::exact(coefficients[coefficient][axis]))?;
            error = error.add(Scalar::exact(rounding_error(expanded(value, error.1)?)?))?;
        }
        result[axis] = error.add(Scalar::exact(compilation[axis]))?.1;
    }
    Ok(result)
}

#[derive(Clone, Copy)]
struct RoundedRange {
    value: Scalar,
    error: Scalar,
}
impl RoundedRange {
    fn exact(value: f64) -> Self {
        Self {
            value: Scalar::exact(value),
            error: Scalar::exact(0.),
        }
    }
    fn add(self, other: Self) -> Result<Self, AnimationError> {
        let value = self.value.add(other.value)?;
        let error = self.error.add(other.error)?;
        let error = error.add(Scalar::exact(rounding_error(expanded(value, error.1)?)?))?;
        Ok(Self { value, error })
    }
    fn sub(self, other: Self) -> Result<Self, AnimationError> {
        let value = self.value.sub(other.value)?;
        let error = self.error.add(other.error)?;
        let error = error.add(Scalar::exact(rounding_error(expanded(value, error.1)?)?))?;
        Ok(Self { value, error })
    }
    fn mul(self, other: Self) -> Result<Self, AnimationError> {
        let value = self.value.mul(other.value)?;
        let error = Scalar::exact(absolute_upper(self.value))
            .mul(other.error)?
            .add(Scalar::exact(absolute_upper(other.value)).mul(self.error)?)?
            .add(self.error.mul(other.error)?)?;
        let error = error.add(Scalar::exact(rounding_error(expanded(value, error.1)?)?))?;
        Ok(Self { value, error })
    }
}
impl RoundedRange {
    fn div_positive_f32(self, denominator: Self) -> Result<Self, AnimationError> {
        let actual_denominator = expanded(denominator.value, denominator.error.1)?;
        if denominator.value.0 <= 0. || actual_denominator.0 <= 0. {
            return Err(AnimationError::InvalidRootRotationCurve);
        }
        let value = self.value.div_interval_positive(denominator.value)?;
        let error = self.error.div_positive(actual_denominator.0)?.add(
            Scalar::exact(absolute_upper(self.value))
                .mul(denominator.error)?
                .div_positive(denominator.value.0)?
                .div_positive(actual_denominator.0)?,
        )?;
        Self { value, error }.rounded_f32()
    }
}

pub(super) fn f32_positive_division_error(
    numerator: [f64; 2],
    numerator_error: f64,
    denominator: [f64; 2],
    denominator_error: f64,
) -> Result<f64, AnimationError> {
    if numerator
        .iter()
        .chain(denominator.iter())
        .any(|v| !v.is_finite())
        || numerator[0] > numerator[1]
        || denominator[0] > denominator[1]
        || [numerator_error, denominator_error]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.)
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let result = RoundedRange {
        value: Scalar(numerator[0], numerator[1]),
        error: Scalar::exact(numerator_error),
    }
    .div_positive_f32(RoundedRange {
        value: Scalar(denominator[0], denominator[1]),
        error: Scalar::exact(denominator_error),
    })?;
    Ok(result.error.1)
}

#[cfg(test)]
mod positive_division_tests {
    use super::*;
    #[test]
    fn positive_f32_division_propagates_same_member_errors_and_rejects_zero_corridor() {
        for numerator in [-1_f32, 0., 1.] {
            for denominator in [0.5_f32, 1., 2.] {
                for errors in [[0., 0.], [0.03125, 0.015625]] {
                    for (n, d) in [
                        ([-2., 2.], [0.25, 2.]),
                        ([f64::from(numerator); 2], [f64::from(denominator); 2]),
                    ] {
                        let cap = f32_positive_division_error(n, errors[0], d, errors[1]).unwrap();
                        if errors == [0., 0.] {
                            assert!(cap < 1e-4);
                        }
                        let actual =
                            (numerator + errors[0] as f32) / (denominator + errors[1] as f32);
                        assert!(
                            (f64::from(actual) - f64::from(numerator) / f64::from(denominator))
                                .abs()
                                <= cap
                        );
                        println!(
                            "positive_division_reference={{\"numerator\":{:?},\"denominator\":{:?},\"actual\":{:?},\"cap\":{:?}}}",
                            numerator, denominator, actual, cap
                        );
                    }
                }
            }
        }
        for denominator in [[0., 1.], [-1., 2.], [0., 0.]] {
            assert!(f32_positive_division_error([-1., 1.], 0., denominator, 0.).is_err());
        }
        assert!(f32_positive_division_error([-1., 1.], 0., [0.25, 1.], 0.25).is_err());
        assert!(f32_positive_division_error([-1., 1.], 0., [0.25, 1.], 0.5).is_err());
        assert!(f32_positive_division_error([0., 1.], -1., [1., 2.], 0.).is_err());
        assert!(f32_positive_division_error([0., f64::NAN], 0., [1., 2.], 0.).is_err());
        assert!(
            f32_positive_division_error(
                [f64::from(f32::MAX); 2],
                0.,
                [f64::from(f32::from_bits(1)); 2],
                0.
            )
            .is_err()
        );
    }
}

impl RoundedRange {
    /// Add f32 operation rounding to the same-member f64 proof machinery.
    fn rounded_f32(self) -> Result<Self, AnimationError> {
        let actual = expanded(self.value, self.error.1)?;
        let (cap, _) = RootRigidEnclosure::enclosed_f32_publication_error([actual.array(); 3])?;
        Ok(Self {
            value: self.value,
            error: self.error.add(Scalar::exact(cap[0]))?,
        })
    }
}

/// Stationary original orientation with the actual normalized-lerp branch.
/// Real proportionality is exact on original f32 keys; runtime branch remains
/// independently checked because raw key norms can affect its threshold.
pub(super) fn stationary_nlerp_sample_error(
    keys: [[f32; 4]; 2],
) -> Result<Option<[f64; 4]>, AnimationError> {
    let [a, b] = keys.map(glam::Quat::from_array);
    if !a.is_finite() || !b.is_finite() || !a.is_normalized() || !b.is_normalized() {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    for i in 0..4 {
        for j in i + 1..4 {
            if f64::from(keys[0][i]) * f64::from(keys[1][j])
                != f64::from(keys[0][j]) * f64::from(keys[1][i])
            {
                return Ok(None);
            }
        }
    }
    if a.dot(b).abs() <= 1. - f32::EPSILON {
        return Ok(None);
    }
    let flip = a.dot(b) < 0.;
    let unit = super::linear_source::source_key_rotation_bounds(keys[0])?;
    let alpha = RoundedRange {
        value: Scalar(0., 1.),
        error: Scalar::exact(0.),
    };
    let complement = RoundedRange::exact(1.).sub(alpha)?.rounded_f32()?;
    let mut error = Scalar::exact(0.);
    for axis in 0..4 {
        let value = Scalar(unit[axis][0], unit[axis][1]);
        let operand = |raw: f64| -> Result<RoundedRange, AnimationError> {
            Ok(RoundedRange {
                value,
                error: Scalar::exact(absolute_upper(Scalar::exact(raw).sub(value)?)),
            })
        };
        let first = operand(f64::from(keys[0][axis]))?;
        let second = operand(f64::from(keys[1][axis]) * if flip { -1. } else { 1. })?;
        let blend = first
            .mul(complement)?
            .rounded_f32()?
            .add(second.mul(alpha)?.rounded_f32()?)?
            .rounded_f32()?;
        error = error.add(blend.error)?;
    }
    // Exact real unit keys coincide after the hemisphere flip; their real
    // affine blend is that same unit quaternion for every parameter in [0,1].
    let first = normalization_f32_uniform_error(error)?;
    let total = first
        .into_iter()
        .try_fold(Scalar::exact(0.), |s, v| s.add(Scalar::exact(v)))?;
    let mut result = normalization_f32_uniform_error(total)?;
    // The clip returns its raw first key at the first-key boundary. Include
    // that endpoint separately; a negative final authored key is not admitted
    // across the next-key boundary by the clip interval selector.
    for axis in 0..4 {
        let raw_error = absolute_upper(
            Scalar::exact(f64::from(keys[0][axis])).sub(Scalar(unit[axis][0], unit[axis][1]))?,
        );
        result[axis] = result[axis].max(raw_error);
    }
    Ok(Some(result))
}

#[cfg(test)]
mod stationary_nlerp_tests {
    use super::*;
    #[test]
    fn stationary_linear_keys_cover_both_runtime_normalizations() {
        let rotations = [
            glam::Quat::IDENTITY,
            glam::Quat::from_xyzw(0.5, 0.5, 0.5, 0.5),
            glam::Quat::from_euler(glam::EulerRot::XYZ, 0.2, -0.4, 0.7),
            glam::Quat::from_xyzw(0., 0., 0., 1_f32.next_up()),
        ];
        for q in rotations {
            for end in [q, -q] {
                let caps = stationary_nlerp_sample_error([q.to_array(), end.to_array()])
                    .unwrap()
                    .unwrap();
                for step in 0..=16 {
                    let actual = q.slerp(end, step as f32 / 16.).normalize();
                    println!(
                        "stationary_nlerp_reference={{\"source\":{:?},\"actual\":{:?},\"caps\":{:?}}}",
                        q.to_array(),
                        actual.to_array(),
                        caps
                    );
                }
            }
        }
        assert_eq!(
            stationary_nlerp_sample_error([
                glam::Quat::IDENTITY.to_array(),
                glam::Quat::from_rotation_y(0.001).to_array()
            ])
            .unwrap(),
            None
        );
        assert_eq!(
            stationary_nlerp_sample_error([[0., 0., 0., 1_f32.next_down()]; 2]).unwrap(),
            None
        );
        assert!(stationary_nlerp_sample_error([[f32::NAN; 4]; 2]).is_err());
    }
}

/// Complete spherical NEON SLERP plus the source sampler's final normalize.
/// Original normalized keys/time define the same-time unit quaternion source.
pub(super) fn neon_slerp_sample_error(
    keys: [[f32; 4]; 2],
    times: [f32; 2],
    interval: [f32; 2],
) -> Result<Option<[f64; 4]>, AnimationError> {
    let alpha = rounded_f32_parameter(times, interval)?;
    let Some((angle, angle_error)) = stored_slerp_key_angle_error(keys)? else {
        return Ok(None);
    };
    let [a, b] = keys.map(glam::Quat::from_array);
    let theta = 0.5 * a.angle_between(b);
    if theta <= 0. || theta > core::f32::consts::FRAC_PI_2 {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let angle = RoundedRange {
        value: Scalar(angle[0], angle[1]),
        error: Scalar::exact(angle_error),
    };
    let complementary = RoundedRange::exact(1.).sub(alpha)?.rounded_f32()?;
    let sine = |argument: RoundedRange, domain: [f32; 2]| -> Result<RoundedRange, AnimationError> {
        let ideal_argument = Scalar(argument.value.0.max(0.), argument.value.1);
        let value = super::linear_source::sine_cosine(ideal_argument)?.0;
        let error = argument
            .error
            .add(Scalar::exact(neon_slerp_sine_error(domain)?))?;
        // Mathematical sin is globally 1-Lipschitz, so argument error transfers
        // directly, separately from actual NEON same-argument evaluation error.
        Ok(RoundedRange { value, error })
    };
    // The actual source parameter is in [0,1] by monotonic rounded subtraction
    // and division on a validated key interval. Rounded multiplication retains
    // arguments in [0,stored theta], including the rounded complementary lane.
    let left = sine(angle.mul(complementary)?.rounded_f32()?, [0., theta])?;
    let right = sine(angle.mul(alpha)?.rounded_f32()?, [0., theta])?;
    let denominator = sine(angle, [theta, theta])?;
    let unit = keys.map(super::linear_source::source_key_rotation_bounds);
    let [unit_a, unit_b] = unit;
    let unit_a = unit_a?;
    let unit_b = unit_b?;
    let flip = a.dot(b) < 0.;
    let mut total = Scalar::exact(0.);
    for axis in 0..4 {
        let input =
            |raw: f32, domain: [f64; 2], negate: bool| -> Result<RoundedRange, AnimationError> {
                let mut value = Scalar(domain[0], domain[1]);
                let mut raw = f64::from(raw);
                if negate {
                    value = Scalar(-value.1, -value.0);
                    raw = -raw;
                }
                let error = Scalar::exact(absolute_upper(Scalar::exact(raw).sub(value)?));
                Ok(RoundedRange { value, error })
            };
        let qa = input(keys[0][axis], unit_a[axis], false)?;
        let qb = input(keys[1][axis], unit_b[axis], flip)?;
        let numerator = qa
            .mul(left)?
            .rounded_f32()?
            .add(qb.mul(right)?.rounded_f32()?)?
            .rounded_f32()?;
        let quotient = numerator.div_positive_f32(denominator)?;
        total = total.add(quotient.error)?;
    }
    // Original short-arc sine interpolation of unit keys is unit. Retain that
    // relational norm proof instead of inferring it from component interval boxes.
    normalization_f32_uniform_error(total).map(Some)
}

#[cfg(all(test, target_arch = "aarch64"))]
mod complete_neon_slerp_tests {
    use super::*;
    #[test]
    fn complete_neon_slerp_source_error_covers_original_time_and_normalization() {
        let _: core::arch::aarch64::float32x4_t = glam::Vec4::ZERO.into();
        let h = core::f32::consts::FRAC_1_SQRT_2;
        for initial in [
            glam::Quat::IDENTITY,
            glam::Quat::from_xyzw(0.5, 0.5, 0.5, 0.5),
        ] {
            for sign in [1., -1.] {
                let end = initial * glam::Quat::from_xyzw(0., h, 0., h) * sign;
                let keys = [initial.to_array(), end.to_array()];
                for times in [[0., 1.], [0.1, 0.9]] {
                    for interval in [
                        times,
                        [
                            times[0] + (times[1] - times[0]) * 0.25,
                            times[0] + (times[1] - times[0]) * 0.75,
                        ],
                    ] {
                        let caps = neon_slerp_sample_error(keys, times, interval)
                            .unwrap()
                            .unwrap();
                        assert!(caps.iter().all(|v| v.is_finite() && *v > 0. && *v < 0.001));
                        for step in 0..=16 {
                            let time = if step == 16 {
                                interval[1]
                            } else {
                                interval[0] + (interval[1] - interval[0]) * (step as f32 / 16.)
                            };
                            let alpha = (time - times[0]) / (times[1] - times[0]);
                            let actual = glam::Quat::from_array(keys[0])
                                .slerp(glam::Quat::from_array(keys[1]), alpha)
                                .normalize();
                            println!(
                                "complete_neon_slerp_reference={{\"initial\":{:?},\"times\":{:?},\"time\":{:?},\"actual\":{:?},\"caps\":{:?}}}",
                                initial.to_array(),
                                times,
                                time,
                                actual.to_array(),
                                caps
                            );
                        }
                    }
                }
            }
        }
        assert_eq!(
            neon_slerp_sample_error([[0., 0., 0., 1.]; 2], [0., 1.], [0., 1.]).unwrap(),
            None
        );
        assert!(
            neon_slerp_sample_error([[0., 0., 0., 1.], [0., h, 0., h]], [0., 1.], [0.5, 0.25])
                .is_err()
        );
    }
}

/// Uniform glam 0.33.7 NEON sine error over a stored positive input domain.
/// No range reduction/reflection is needed in [0,f32 FRAC_PI_2].
pub(super) fn neon_slerp_sine_error(domain: [f32; 2]) -> Result<f64, AnimationError> {
    if domain.iter().any(|v| !v.is_finite())
        || domain[0] < 0.
        || domain[1] < domain[0]
        || domain[1] > core::f32::consts::FRAC_PI_2
    {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let x = RoundedRange {
        value: Scalar(f64::from(domain[0]), f64::from(domain[1])),
        error: Scalar::exact(0.),
    };
    // glam reduces by round(input * stored_inv_tau). The complete rounded
    // quotient stays strictly between -1/2 and 1/2, so the integer is zero
    // and subtracting its multiple of tau preserves the stored input exactly.
    let quotient = x
        .mul(RoundedRange::exact(f64::from(0.159_154_94_f32)))?
        .rounded_f32()?;
    if absolute_upper(expanded(quotient.value, quotient.error.1)?) >= 0.5 {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    // input <= the exact stored reflection threshold; the NEON selector
    // retains the original reduced input on this whole domain.
    let square = x.mul(x)?.rounded_f32()?;
    let coefficients = [
        1_f32,
        -0.16666667,
        0.008_333_331,
        -0.00019840874,
        2.752_556_2e-6,
        -2.388_985_9e-8,
    ];
    let mut polynomial = RoundedRange::exact(f64::from(coefficients[5]));
    for coefficient in coefficients[..5].iter().rev() {
        polynomial = polynomial
            .mul(square)?
            .rounded_f32()?
            .add(RoundedRange::exact(f64::from(*coefficient)))?
            .rounded_f32()?;
    }
    let evaluated = polynomial.mul(x)?.rounded_f32()?;
    let radius = Scalar::exact(f64::from(domain[1]));
    let squared = radius.square()?;
    let mut power = radius;
    let denominators = [1., 6., 120., 5040., 362880., 39916800.];
    let mut approximation = Scalar::exact(0.);
    for i in 0..6 {
        let taylor =
            Scalar::exact(if i % 2 == 0 { 1. } else { -1. }).div_positive(denominators[i])?;
        let delta = Scalar::exact(f64::from(coefficients[i])).sub(taylor)?;
        approximation = approximation.add(Scalar::exact(absolute_upper(delta)).mul(power)?)?;
        power = power.mul(squared)?;
    }
    // Taylor degree 12 (its even coefficient is zero), derivative bounded by
    // one, gives the uniform x^13/13! remainder without a platform sine oracle.
    approximation = approximation.add(power.div_positive(6227020800.)?)?;
    evaluated.error.add(approximation).map(|v| v.1)
}

#[cfg(all(test, target_arch = "aarch64"))]
mod neon_sine_tests {
    use super::*;
    #[test]
    fn neon_slerp_sine_uniform_cap_covers_actual_vector_backend() {
        let upper = core::f32::consts::FRAC_PI_2;
        for domain in [
            [0., upper],
            [0., 0.01],
            [0.25, 0.5],
            [upper, upper],
            [0., 0.],
        ] {
            let cap = neon_slerp_sine_error(domain).unwrap();
            assert!(cap.is_finite() && cap >= 0. && cap < 1e-5);
            for step in 0..=16 {
                let input = if step == 16 {
                    domain[1]
                } else {
                    domain[0] + (domain[1] - domain[0]) * (step as f32 / 16.)
                };
                let actual = glam::Vec4::splat(input).sin().x;
                println!(
                    "neon_sine_reference={{\"domain\":{:?},\"input\":{:?},\"actual\":{:?},\"cap\":{:?}}}",
                    domain, input, actual, cap
                );
            }
        }
        for invalid in [[-1., 0.], [1., 0.], [0., upper.next_up()], [0., f32::NAN]] {
            assert!(neon_slerp_sine_error(invalid).is_err());
        }
    }
}

/// Source-key half-angle versus actual stored-dot acos, after exact
/// hemisphere qualification. None denotes glam's separate normalized lerp path.
pub(super) fn stored_slerp_key_angle_error(
    keys: [[f32; 4]; 2],
) -> Result<Option<([f64; 2], f64)>, AnimationError> {
    let [a, b] = keys.map(glam::Quat::from_array);
    if !a.is_finite() || !b.is_finite() || !a.is_normalized() || !b.is_normalized() {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let sign = crate::exact_quaternion::dot_sign(a, b);
    let runtime = a.dot(b);
    if sign == core::cmp::Ordering::Equal || (sign == core::cmp::Ordering::Less) != (runtime < 0.) {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let runtime = runtime.abs();
    if runtime > 1. - f32::EPSILON {
        return Ok(None);
    }
    let mut dot = Scalar::exact(0.);
    let mut norms = [Scalar::exact(0.); 2];
    for axis in 0..4 {
        let x = f64::from(keys[0][axis]);
        let y = f64::from(keys[1][axis]);
        dot = dot.add(Scalar::exact(x * y))?;
        norms[0] = norms[0].add(Scalar::exact(x * x))?;
        norms[1] = norms[1].add(Scalar::exact(y * y))?;
    }
    if sign == core::cmp::Ordering::Less {
        dot = Scalar(-dot.1, -dot.0);
    }
    let normalized =
        dot.div_interval_positive(norms[0].sqrt_positive()?.mul(norms[1].sqrt_positive()?)?)?;
    // Exact sign plus Cauchy-Schwarz prove the true normalized dot is in [0,1].
    let lo = super::linear_source::positive_acos_point(normalized.1.clamp(0., 1.))?;
    let hi = super::linear_source::positive_acos_point(normalized.0.clamp(0., 1.))?;
    let ideal = Scalar(lo.0, hi.1);
    let (rounded_ideal, approximation) = stored_slerp_acos_error(runtime)?;
    let shift = absolute_upper(Scalar(rounded_ideal[0], rounded_ideal[1]).sub(ideal)?);
    let cap = Scalar::exact(shift).add(Scalar::exact(approximation))?.1;
    Ok(Some((ideal.array(), cap)))
}

#[cfg(test)]
mod slerp_branch_tests {
    use super::*;
    #[test]
    fn original_hemisphere_and_dot_error_are_qualified_before_slerp_angle() {
        let h = core::f32::consts::FRAC_1_SQRT_2;
        let tiny = f32::from_bits(1);
        let a = glam::Quat::from_xyzw(h, h, tiny, 0.);
        let wrong = glam::Quat::from_xyzw(h, -h, -tiny, 0.);
        assert_eq!(a.dot(wrong), 0.);
        assert_eq!(
            crate::exact_quaternion::dot_sign(a, wrong),
            core::cmp::Ordering::Less
        );
        assert!(stored_slerp_key_angle_error([a.to_array(), wrong.to_array()]).is_err());
        let positive = glam::Quat::from_xyzw(h, -h, tiny, 0.);
        assert_eq!(a.dot(positive), 0.);
        assert!(
            stored_slerp_key_angle_error([a.to_array(), positive.to_array()])
                .unwrap()
                .is_some()
        );
        let rotations = [
            glam::Quat::IDENTITY,
            glam::Quat::from_rotation_y(0.3),
            glam::Quat::from_euler(glam::EulerRot::XYZ, 0.2, -0.4, 0.7),
        ];
        for a in rotations {
            for b in rotations {
                for b in [b, -b] {
                    let result =
                        stored_slerp_key_angle_error([a.to_array(), b.to_array()]).unwrap();
                    if let Some((ideal, cap)) = result {
                        let actual = 0.5 * f64::from(a.angle_between(b));
                        assert!(
                            absolute_upper(
                                Scalar::exact(actual)
                                    .sub(Scalar(ideal[0], ideal[1]))
                                    .unwrap()
                            ) <= cap
                        );
                        println!(
                            "slerp_key_angle_reference={{\"a\":{:?},\"b\":{:?},\"actual\":{:?},\"ideal\":{:?},\"cap\":{:?}}}",
                            a.to_array(),
                            b.to_array(),
                            actual,
                            ideal,
                            cap
                        );
                    } else {
                        assert!(a.dot(b).abs() > 1. - f32::EPSILON);
                    }
                }
            }
        }
        assert!(stored_slerp_key_angle_error([[0.; 4]; 2]).is_err());
    }
}

/// glam 0.33.7 positive acos minimax evaluation versus mathematical acos.
/// Fixed stored input only; source-key dot and hemisphere selection are separate.
#[allow(clippy::approx_constant)]
pub(super) fn stored_slerp_acos_error(input: f32) -> Result<([f64; 2], f64), AnimationError> {
    let ideal = super::linear_source::positive_acos_point(f64::from(input))?;
    if input == 1. {
        return Ok((ideal.array(), 0.));
    }
    let x = RoundedRange::exact(f64::from(input));
    let coefficients = [
        -0.001_262_491_1_f32,
        0.006_670_09,
        -0.017_088_126,
        0.030_891_88,
        -0.050_174_303,
        0.088_978_99,
        -0.214_598_8,
        1.570_796_3,
    ];
    let mut polynomial = RoundedRange::exact(f64::from(coefficients[0]));
    for coefficient in &coefficients[1..] {
        polynomial = polynomial
            .mul(x)?
            .rounded_f32()?
            .add(RoundedRange::exact(f64::from(*coefficient)))?
            .rounded_f32()?;
    }
    let omx = RoundedRange::exact(1.).sub(x)?.rounded_f32()?;
    let nominal = omx.value.sqrt_positive()?;
    let raw = expanded(omx.value, omx.error.1)?.sqrt_positive()?;
    let (rounding, _) = RootRigidEnclosure::enclosed_f32_publication_error([raw.array(); 3])?;
    let root = RoundedRange {
        value: nominal,
        error: omx
            .error
            .div_positive(Scalar::exact(nominal.0).add(Scalar::exact(raw.0))?.0)?
            .add(Scalar::exact(rounding[0]))?,
    };
    let evaluated = polynomial.mul(root)?.rounded_f32()?;
    let approximation = absolute_upper(evaluated.value.sub(ideal)?);
    let cap = evaluated.error.add(Scalar::exact(approximation))?.1;
    Ok((ideal.array(), cap))
}

#[cfg(test)]
mod slerp_acos_tests {
    use super::*;
    #[test]
    fn stored_slerp_acos_caps_cover_glam_and_stable_endpoint_domains() {
        for input in (0..=128)
            .map(|i| i as f32 / 128.)
            .chain([1_f32.next_down(), f32::from_bits(1)])
        {
            let (ideal, cap) = stored_slerp_acos_error(input).unwrap();
            let q =
                glam::Quat::from_xyzw((1. - f64::from(input).powi(2)).sqrt() as f32, 0., 0., input);
            let actual = 0.5 * f64::from(glam::Quat::IDENTITY.angle_between(q));
            assert!(cap.is_finite() && cap >= 0. && cap < 1e-5);
            let discrepancy = Scalar::exact(actual)
                .sub(Scalar(ideal[0], ideal[1]))
                .unwrap();
            assert!(absolute_upper(discrepancy) <= cap);
            println!(
                "slerp_acos_reference={{\"input\":{:?},\"actual\":{:?},\"ideal\":{:?},\"cap\":{:?}}}",
                input, actual, ideal, cap
            );
        }
        for input in [-1., 1_f32.next_up(), f32::NAN, f32::INFINITY] {
            assert!(stored_slerp_acos_error(input).is_err());
        }
    }
}

pub(super) fn source_linear_translation_local_time_error(
    from: [f32; 3],
    to: [f32; 3],
    keys: [f32; 2],
    interval: [f64; 2],
) -> Result<[f64; 3], AnimationError> {
    let sampled = interval.map(|v| v as f32);
    let sampling = source_linear_translation_sample_error(from, to, keys, sampled)?;
    let (time_error, _) = RootRigidEnclosure::enclosed_f32_publication_error([interval; 3])?;
    let duration = Scalar::exact(f64::from(keys[1])).sub(Scalar::exact(f64::from(keys[0])))?;
    let mut result = [0.; 3];
    for axis in 0..3 {
        let difference =
            Scalar::exact(f64::from(to[axis])).sub(Scalar::exact(f64::from(from[axis])))?;
        let speed = Scalar::exact(absolute_upper(difference)).div_interval_positive(duration)?;
        result[axis] = Scalar::exact(sampling[axis])
            .add(speed.mul(Scalar::exact(time_error[0]))?)?
            .1;
    }
    Ok(result)
}

fn rounded_f32_parameter(
    times: [f32; 2],
    interval: [f32; 2],
) -> Result<RoundedRange, AnimationError> {
    let times = times.map(f64::from);
    let interval = interval.map(f64::from);
    if interval.iter().chain(times.iter()).any(|v| !v.is_finite())
        || interval[0] < times[0]
        || interval[1] > times[1]
        || interval[0] > interval[1]
    {
        return Err(AnimationError::InvalidSampleTime);
    }
    let denominator = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
    let nominal = f64::from((times[1] as f32) - (times[0] as f32));
    if denominator.0 <= 0. || !nominal.is_finite() || nominal <= 0. {
        return Err(AnimationError::InvalidSampleTime);
    }
    let numerator = RoundedRange {
        value: Scalar(interval[0], interval[1]),
        error: Scalar::exact(0.),
    }
    .sub(RoundedRange::exact(times[0]))?
    .rounded_f32()?;
    let value = numerator.value.div_interval_positive(denominator)?;
    let denominator_error = absolute_upper(denominator.sub(Scalar::exact(nominal))?);
    let error = numerator.error.div_positive(nominal)?.add(
        Scalar::exact(absolute_upper(numerator.value))
            .mul(Scalar::exact(denominator_error))?
            .div_positive(denominator.0)?
            .div_positive(nominal)?,
    )?;
    let mut alpha = RoundedRange { value, error }.rounded_f32()?;
    alpha.value.0 = alpha.value.0.max(0.);
    alpha.value.1 = alpha.value.1.min(1.);
    Ok(alpha)
}

pub(super) fn source_linear_translation_sample_error(
    from: [f32; 3],
    to: [f32; 3],
    times: [f32; 2],
    interval: [f32; 2],
) -> Result<[f64; 3], AnimationError> {
    let alpha = rounded_f32_parameter(times, interval)?;
    let complement = RoundedRange::exact(1.).sub(alpha)?.rounded_f32()?;
    let mut result = [0.; 3];
    for axis in 0..3 {
        let a = RoundedRange::exact(f64::from(from[axis]))
            .mul(complement)?
            .rounded_f32()?;
        let b = RoundedRange::exact(f64::from(to[axis]))
            .mul(alpha)?
            .rounded_f32()?;
        result[axis] = a.add(b)?.rounded_f32()?.error.1;
    }
    Ok(result)
}

fn rounded_parameter(times: [f64; 2], interval: [f64; 2]) -> Result<RoundedRange, AnimationError> {
    let denominator = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
    let nominal = times[1] - times[0];
    if denominator.0 <= 0. || !nominal.is_finite() || nominal <= 0. {
        return Err(AnimationError::InvalidSampleTime);
    }
    let numerator = Scalar(interval[0], interval[1]).sub(Scalar::exact(times[0]))?;
    let numerator_error = rounding_error(numerator)?;
    let denominator_error = absolute_upper(denominator.sub(Scalar::exact(nominal))?);
    let mut value = numerator.div_interval_positive(denominator)?;
    value.0 = value.0.max(0.);
    value.1 = value.1.min(1.);
    let error = Scalar::exact(numerator_error).div_positive(nominal)?.add(
        Scalar::exact(absolute_upper(numerator))
            .mul(Scalar::exact(denominator_error))?
            .div_positive(denominator.0)?
            .div_positive(nominal)?,
    )?;
    let actual_quotient = expanded(numerator, numerator_error)?.div_positive(nominal)?;
    let error = error.add(Scalar::exact(rounding_error(actual_quotient)?))?;
    Ok(RoundedRange { value, error })
}

/// Rounding cap for unit(): divide by maximum component, dot/sqrt/reciprocal,
/// then component multiplication. Matches the inspected glam f64 operation path.
fn scaled_unit_rounding_error() -> Result<f64, AnimationError> {
    let ratio = Scalar::exact(rounding_error(Scalar(0., 1.))?);
    let dot = ratio
        .mul(Scalar::exact(4.))?
        .add(Scalar::exact(rounding_error(Scalar(0., 5.))?).mul(Scalar::exact(3.))?)?;
    if dot.1 >= 1. {
        return Err(AnimationError::NumericalOverflow);
    }
    // The maximum component divides to exactly +/-1; the rounded scaled
    // components have magnitude <=1, so both exact and rounded dot norms >=1.
    // sqrt sensitivity is <=1/2 and reciprocal sensitivity <=1 on that domain.
    Ok(ratio
        .mul(Scalar::exact(8.))?
        .add(dot.div_positive(2.)?)?
        .add(Scalar::exact(rounding_error(Scalar(1., 3.))?))?
        .add(ratio)?
        .add(ratio)?
        .1)
}

pub(crate) fn quaternion_cubic_interval_evaluation_error_bounds(
    control: [[f64; 4]; 4],
    raw_error: [f64; 4],
    times: [f64; 2],
    interval: [f64; 2],
) -> Result<[f64; 4], AnimationError> {
    if interval
        .iter()
        .chain(times.iter())
        .any(|value| !value.is_finite())
        || interval[0] < times[0]
        || interval[1] > times[1]
        || interval[0] > interval[1]
        || control.iter().flatten().any(|value| !value.is_finite())
        || raw_error
            .iter()
            .any(|error| !error.is_finite() || *error < 0.)
    {
        return Err(AnimationError::InvalidSampleTime);
    }
    let u = rounded_parameter(times, interval)?;
    let complement = RoundedRange::exact(1.).sub(u)?;
    let mut level = control.map(|q| q.map(RoundedRange::exact));
    for depth in 1..4 {
        for index in 0..4 - depth {
            for axis in 0..4 {
                let a = level[index][axis];
                let b = level[index + 1][axis];
                let mut next = a.mul(complement)?.add(b.mul(u)?)?;
                // Exact nominal interpolation remains inside its endpoint hull.
                next.value = cubic::interpolate(a.value, b.value, u.value)?;
                level[index][axis] = next;
            }
        }
    }
    let mut squared = Scalar::exact(0.);
    let mut total = Scalar::exact(0.);
    for axis in 0..4 {
        let source = expanded(level[0][axis].value, raw_error[axis])?;
        squared = squared.add(source.square()?)?;
        total = total
            .add(level[0][axis].error)?
            .add(Scalar::exact(raw_error[axis]))?;
    }
    if squared.0 <= 0. {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let norm = squared.sqrt_positive()?;
    if norm.0 <= total.1 {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let error = total
        .mul(Scalar::exact(2.))?
        .div_positive(norm.0)?
        .add(Scalar::exact(scaled_unit_rounding_error()?))?
        .1;
    Ok(std::array::from_fn(|axis| {
        if raw_error[axis] == 0. && control.iter().all(|q| q[axis] == 0.) {
            0.
        } else {
            error
        }
    }))
}

/// Inputs' errors must enclose real unit rotations. Compare their real Hamilton
/// product to the actual evaluated product (including runtime normalization).
pub(crate) fn quaternion_composition_evaluation_error_bounds(
    a: [f64; 4],
    a_error: [f64; 4],
    b: [f64; 4],
    b_error: [f64; 4],
    evaluated: [f64; 4],
) -> Result<[f64; 4], AnimationError> {
    if a.iter()
        .chain(b.iter())
        .chain(evaluated.iter())
        .any(|value| !value.is_finite())
        || a_error
            .iter()
            .chain(b_error.iter())
            .any(|error| !error.is_finite() || *error < 0.)
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let enclose =
        |value: [f64; 4], error: [f64; 4]| -> Result<RootRigidEnclosure, AnimationError> {
            let mut rotation = [[0.; 2]; 4];
            for axis in 0..4 {
                let interval = expanded(Scalar::exact(value[axis]), error[axis])?;
                rotation[axis] = interval.array();
            }
            Ok(RootRigidEnclosure {
                translation: [[0.; 2]; 3],
                rotation,
            })
        };
    let product = enclose(a, a_error)?.compose(&enclose(b, b_error)?)?;
    let mut result = [0.; 4];
    for axis in 0..4 {
        let bound = product.rotation[axis];
        let delta = Scalar(bound[0], bound[1]).sub(Scalar::exact(evaluated[axis]))?;
        result[axis] = absolute_upper(delta);
    }
    Ok(result)
}

/// Exact-source translation derivative over a single key interval.
pub(crate) fn translation_source_velocity_bounds(
    coefficients: [DVec3; 4],
    error: [f64; 3],
    key_times: [f64; 2],
    times: [f64; 2],
) -> Result<[[f64; 2]; 3], AnimationError> {
    let dt = Scalar::exact(key_times[1]).sub(Scalar::exact(key_times[0]))?;
    let u = Scalar(times[0], times[1])
        .sub(Scalar::exact(key_times[0]))?
        .div_interval_positive(dt)?;
    let u = Scalar(u.0.max(0.), u.1.min(1.));
    let mut result = [[0.; 2]; 3];
    for axis in 0..3 {
        let coefficient = |i: usize| -> Result<Scalar, AnimationError> {
            if error[axis] == 0. {
                Ok(Scalar::exact(coefficients[i][axis]))
            } else {
                Scalar::exact(coefficients[i][axis]).add(Scalar(-error[axis], error[axis]))
            }
        };
        let numerator = coefficient(3)?
            .mul(Scalar::exact(3.))?
            .mul(u)?
            .add(coefficient(2)?.mul(Scalar::exact(2.))?)?
            .mul(u)?
            .add(coefficient(1)?)?;
        result[axis] = numerator.div_interval_positive(dt)?.array();
    }
    Ok(result)
}

/// Componentwise exact-source spatial angular velocity. The normalization
/// derivative cancels in 2*vec(raw' * conjugate(raw))/|raw|².
pub(crate) fn quaternion_cubic_source_angular_bounds(
    control: [[f64; 4]; 4],
    error: [f64; 4],
    key_times: [f64; 2],
    times: [f64; 2],
) -> Result<[[f64; 2]; 3], AnimationError> {
    if control.iter().flatten().any(|v| !v.is_finite())
        || error.iter().any(|v| !v.is_finite() || *v < 0.)
    {
        return Err(AnimationError::NumericalOverflow);
    }
    if key_times.into_iter().chain(times).any(|v| !v.is_finite())
        || key_times[1] <= key_times[0]
        || times[0] < key_times[0]
        || times[1] > key_times[1]
        || times[1] < times[0]
    {
        return Err(AnimationError::InvalidSampleTime);
    }
    let dt = Scalar::exact(key_times[1]).sub(Scalar::exact(key_times[0]))?;
    let u = Scalar(times[0], times[1])
        .sub(Scalar::exact(key_times[0]))?
        .div_interval_positive(dt)?;
    let mut source = [[Scalar::exact(0.); 4]; 4];
    for i in 0..4 {
        for axis in 0..4 {
            source[i][axis] = if error[axis] == 0. {
                Scalar::exact(control[i][axis])
            } else {
                Scalar::exact(control[i][axis]).add(Scalar(-error[axis], error[axis]))?
            };
        }
    }
    let mut pending = vec![(Scalar(u.0.max(0.), u.1.min(1.)), 0_u32)];
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut visited = 0;
    while let Some((u, depth)) = pending.pop() {
        visited += 1;
        if visited > 8191 {
            return Err(AnimationError::RootRotationBudget);
        }
        let (value, derivative) = super::cubic::polynomial(source, u, dt)?;
        let mut norm_squared = Scalar::exact(0.);
        for component in value {
            norm_squared = norm_squared.add(component.square()?)?;
        }
        if norm_squared.0 <= 0. {
            let middle = u.0 * 0.5 + u.1 * 0.5;
            if depth == 12 || middle <= u.0 || middle >= u.1 {
                return Err(AnimationError::InvalidRootRotationCurve);
            }
            pending.push((Scalar(middle, u.1), depth + 1));
            pending.push((Scalar(u.0, middle), depth + 1));
            continue;
        }
        let product = cross(
            [derivative[0], derivative[1], derivative[2]],
            [value[0], value[1], value[2]],
        )?;
        for axis in 0..3 {
            let angular = derivative[axis]
                .mul(value[3])?
                .sub(derivative[3].mul(value[axis])?)?
                .sub(product[axis])?
                .mul(Scalar::exact(2.))?
                .div_interval_positive(norm_squared)?;
            hull[axis][0] = hull[axis][0].min(angular.0);
            hull[axis][1] = hull[axis][1].max(angular.1);
        }
    }
    Ok(hull)
}

#[cfg(test)]
mod source_angular_tests {
    use super::*;
    #[test]
    fn source_angular_components_cover_noncommuting_polynomial_and_reject_zero_norm() {
        // Bernstein controls of raw(u)=(u,u²,u³,1). The explicit error covers
        // the exact rational thirds before the stored f64 control rounding.
        let control = [
            [0., 0., 0., 1.],
            [1. / 3., 0., 0., 1.],
            [2. / 3., 1. / 3., 0., 1.],
            [1., 1., 1., 1.],
        ];
        for i in 0..=16 {
            let u = f64::from(i) / 16.;
            let bounds =
                quaternion_cubic_source_angular_bounds(control, [1e-15; 4], [0., 1.], [u, u])
                    .unwrap();
            assert!(bounds.iter().all(|r| r[1] - r[0] < 1e-9));
            if i == 8 {
                for (range, expected) in bounds.into_iter().zip([8. / 5., 96. / 85., 128. / 85.]) {
                    assert!(range[0] <= expected && range[1] >= expected);
                }
            }
            println!("SOURCE_ANGULAR_COMPONENTS {:?}", (u, bounds));
        }
        let singular = [
            [0., 0., 0., 1.],
            [0., 0., 0., 1.],
            [0., 0., 0., -1.],
            [0., 0., 0., -1.],
        ];
        assert!(matches!(
            quaternion_cubic_source_angular_bounds(singular, [0.; 4], [0., 1.], [0.5, 0.5]),
            Err(AnimationError::InvalidRootRotationCurve)
        ));
    }
}

/// |omega| <= 2 |raw quaternion derivative| / |raw quaternion|.
pub(crate) fn quaternion_cubic_source_speed_bound(
    control: [[f64; 4]; 4],
    error: [f64; 4],
    times: [f64; 2],
) -> Result<f64, AnimationError> {
    if control.iter().flatten().any(|v| !v.is_finite())
        || error.iter().any(|v| !v.is_finite() || *v < 0.)
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let dt = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
    let mut source = [[Scalar::exact(0.); 4]; 4];
    for i in 0..4 {
        for axis in 0..4 {
            source[i][axis] = if error[axis] == 0. {
                Scalar::exact(control[i][axis])
            } else {
                Scalar::exact(control[i][axis]).add(Scalar(-error[axis], error[axis]))?
            };
        }
    }
    let mut derivative_l1 = Scalar::exact(0.);
    for axis in 0..4 {
        let mut bound = 0_f64;
        for i in 0..3 {
            let derivative = source[i + 1][axis]
                .sub(source[i][axis])?
                .mul(Scalar::exact(3.))?
                .div_interval_positive(dt)?;
            bound = bound.max(derivative.0.abs().max(derivative.1.abs()));
        }
        derivative_l1 = derivative_l1.add(Scalar::exact(bound))?;
    }
    // Bernstein subdivision proves a positive norm over every covered cell.
    // Sampling a midpoint cannot establish this invariant.
    let mut pending = vec![(source, 0_u32)];
    let mut lower = f64::INFINITY;
    let mut cells = 0_usize;
    while let Some((control, depth)) = pending.pop() {
        cells += 1;
        if cells > 8191 {
            return Err(AnimationError::RootRotationBudget);
        }
        let mut squared = Scalar::exact(0.);
        for axis in 0..4 {
            let lo = control
                .iter()
                .map(|q| q[axis].0)
                .fold(f64::INFINITY, f64::min);
            let hi = control
                .iter()
                .map(|q| q[axis].1)
                .fold(f64::NEG_INFINITY, f64::max);
            let distance = if lo > 0. {
                lo
            } else if hi < 0. {
                -hi
            } else {
                0.
            };
            squared = squared.add(Scalar::exact(distance).square()?)?;
        }
        if squared.0 > 0. {
            let norm = squared.sqrt_positive()?;
            if norm.0 > 0. {
                lower = lower.min(norm.0);
                continue;
            }
        }
        if depth == 12 {
            return Err(AnimationError::InvalidRootRotationCurve);
        }
        let (left, right) = split_controls(control, Scalar::exact(0.5))?;
        pending.push((right, depth + 1));
        pending.push((left, depth + 1));
    }
    Ok(derivative_l1.mul(Scalar::exact(2.))?.div_positive(lower)?.1)
}

pub(crate) fn translation_source_position_bounds(
    coefficients: [DVec3; 4],
    error: [f64; 3],
    key_times: [f64; 2],
    times: [f64; 2],
) -> Result<[[f64; 2]; 3], AnimationError> {
    let dt = Scalar::exact(key_times[1]).sub(Scalar::exact(key_times[0]))?;
    let u = Scalar(times[0], times[1])
        .sub(Scalar::exact(key_times[0]))?
        .div_interval_positive(dt)?;
    let u = Scalar(u.0.max(0.), u.1.min(1.));
    let mut result = [[0.; 2]; 3];
    for axis in 0..3 {
        let c = |i: usize| -> Result<Scalar, AnimationError> {
            if error[axis] == 0. {
                Ok(Scalar::exact(coefficients[i][axis]))
            } else {
                Scalar::exact(coefficients[i][axis]).add(Scalar(-error[axis], error[axis]))
            }
        };
        result[axis] = c(3)?
            .mul(u)?
            .add(c(2)?)?
            .mul(u)?
            .add(c(1)?)?
            .mul(u)?
            .add(c(0)?)?
            .array();
    }
    Ok(result)
}
