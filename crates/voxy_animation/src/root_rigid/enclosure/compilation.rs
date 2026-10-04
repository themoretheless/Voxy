//! Stored-key compilation discrepancy proofs; runtime evaluation is separate.
use super::*;
/// Exact-source polynomial coefficients, before rounded cache compilation.
pub(crate) fn translation_coefficient_error_bounds(
    origin: DVec3, value: DVec3, end: Option<DVec3>,
    times: [f64;2], tangents: [DVec3;2], mode: crate::Interpolation,
    stored: [DVec3;4],
) -> Result<[f64;3],AnimationError> {
    let mut error = [0_f64;3];
    for axis in 0..3 {
        let value = Scalar::exact(value[axis]).sub(Scalar::exact(origin[axis]))?;
        let mut coefficients = [value,Scalar::exact(0.),Scalar::exact(0.),Scalar::exact(0.)];
        if let Some(end) = end {
            let end = Scalar::exact(end[axis]).sub(Scalar::exact(origin[axis]))?;
            match mode {
                crate::Interpolation::Step => {},
                crate::Interpolation::Linear => {coefficients[1]=end.sub(value)?;},
                crate::Interpolation::CubicSpline => {
                    let dt = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
                    let out = Scalar::exact(tangents[0][axis]).mul(dt)?;
                    let incoming = Scalar::exact(tangents[1][axis]).mul(dt)?;
                    coefficients[1] = out;
                    coefficients[2] = end.sub(value)?.mul(Scalar::exact(3.))?
                        .sub(out.mul(Scalar::exact(2.))?)?.sub(incoming)?;
                    coefficients[3] = value.sub(end)?.mul(Scalar::exact(2.))?
                        .add(out)?.add(incoming)?;
                }
            }
        }
        let mut bound = Scalar::exact(0.);
        for coefficient in 0..4 {
            let difference = coefficients[coefficient].sub(Scalar::exact(stored[coefficient][axis]))?;
            bound = bound.add(Scalar::exact(difference.0.abs().max(difference.1.abs())))?;
        }
        error[axis] = bound.1;
    }
    Ok(error)
}

/// Restriction and power-to-Bernstein discrepancy at exact supplied key times.
pub(crate) fn translation_piece_error_bounds(
    coefficients: [DVec3;4], compilation: [f64;3], key_times: [f64;2],
    interval: [f64;2], stored: [DVec3;4],
) -> Result<[f64;3],AnimationError> {
    let delta = Scalar::exact(key_times[1]).sub(Scalar::exact(key_times[0]))?;
    let u = Scalar::exact(interval[0]).sub(Scalar::exact(key_times[0]))?
        .div_interval_positive(delta)?;
    let v = Scalar::exact(interval[1]).sub(Scalar::exact(interval[0]))?
        .div_interval_positive(delta)?;
    let mut error = [0_f64;3];
    for axis in 0..3 {
        if !compilation[axis].is_finite() || compilation[axis]<0. {
            return Err(AnimationError::NumericalOverflow);
        }
        let mut c = [Scalar::exact(0.);4];
        for i in 0..4 {
            let lower = Scalar::exact(coefficients[i][axis]).sub(Scalar::exact(compilation[axis]))?;
            let upper = Scalar::exact(coefficients[i][axis]).add(Scalar::exact(compilation[axis]))?;
            c[i] = Scalar(lower.0,upper.1);
        }
        let a = c[3].mul(u)?.add(c[2])?.mul(u)?.add(c[1])?.mul(u)?.add(c[0])?;
        let b = c[1].add(c[2].mul(Scalar::exact(2.))?.mul(u)?)?
            .add(c[3].mul(Scalar::exact(3.))?.mul(u)?.mul(u)?)?.mul(v)?;
        let d = c[2].add(c[3].mul(Scalar::exact(3.))?.mul(u)?)?.mul(v)?.mul(v)?;
        let e = c[3].mul(v)?.mul(v)?.mul(v)?;
        let controls = [a,a.add(b.div_positive(3.)?)?,
            a.add(b.mul(Scalar::exact(2.))?.div_positive(3.)?)?.add(d.div_positive(3.)?)?,
            a.add(b)?.add(d)?.add(e)?];
        for i in 0..4 {
            let discrepancy = controls[i].sub(Scalar::exact(stored[i][axis]))?;
            error[axis] = error[axis].max(discrepancy.0.abs().max(discrepancy.1.abs()));
        }
    }
    Ok(error)
}

/// Discrepancy of rounded key normalization from the real normalized stored key.
pub(crate) fn quaternion_normalization_error_bounds(
    source: [f64;4], evaluated: [f64;4],
) -> Result<[f64;4],AnimationError> {
    if source.iter().chain(evaluated.iter()).any(|value|!value.is_finite()) {
        return Err(AnimationError::NumericalOverflow);
    }
    let q = source.map(Scalar::exact);
    let norm = q[0].square()?.add(q[1].square()?)?.add(q[2].square()?)?
        .add(q[3].square()?)?.sqrt_positive()?;
    let mut errors = [0.;4];
    for axis in 0..4 {
        let exact = q[axis].div_interval_positive(norm)?;
        let delta = exact.sub(Scalar::exact(evaluated[axis]))?;
        errors[axis] = delta.0.abs().max(delta.1.abs());
    }
    Ok(errors)
}

pub(crate) fn quaternion_cubic_control_error_bounds(
    keys: [[f64;4];2], tangents: [[f64;4];2], times: [f64;2],
    stored: [[f64;4];4],
) -> Result<[f64;4],AnimationError> {
    let duration = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
    if duration.0<=0. {return Err(AnimationError::InvalidSampleTime);}
    let factor = duration.div_positive(3.)?;
    let mut errors = [0_f64;4];
    for axis in 0..4 {
        let first = Scalar::exact(keys[0][axis]);
        let last = Scalar::exact(keys[1][axis]);
        let controls = [first,
            first.add(Scalar::exact(tangents[0][axis]).mul(factor)?)?,
            last.sub(Scalar::exact(tangents[1][axis]).mul(factor)?)?,last];
        for control in 0..4 {
            let delta = controls[control].sub(Scalar::exact(stored[control][axis]))?;
            errors[axis] = errors[axis].max(delta.0.abs().max(delta.1.abs()));
        }
    }
    Ok(errors)
}

/// Uniform source-vs-cached normalized polynomial discrepancy on [0,1].
pub(crate) fn quaternion_cubic_normalized_error_bounds(
    control: [[f64;4];4], raw_error: [f64;4],
) -> Result<[f64;4],AnimationError> {
    if control.iter().flatten().any(|value|!value.is_finite())
        || raw_error.iter().any(|value|!value.is_finite() || *value<0.) {
        return Err(AnimationError::NumericalOverflow);
    }
    let mut norm_squared = Scalar::exact(0.);
    let mut discrepancy = Scalar::exact(0.);
    for axis in 0..4 {
        let lo = control.iter().map(|q|q[axis]).fold(f64::INFINITY,f64::min);
        let hi = control.iter().map(|q|q[axis]).fold(f64::NEG_INFINITY,f64::max);
        let distance = if lo>0. {lo} else if hi<0. {-hi} else {0.};
        norm_squared = norm_squared.add(Scalar::exact(distance).square()?)?;
        discrepancy = discrepancy.add(Scalar::exact(raw_error[axis]))?;
    }
    let norm = norm_squared.sqrt_positive()?;
    // Bernstein convexity bounds raw source-cached discrepancy by its L1 cap.
    // Reverse triangle inequality then proves the source stays nonsingular too.
    if norm.0 <= discrepancy.1 {return Err(AnimationError::InvalidRootRotationCurve);}
    let bound = discrepancy.mul(Scalar::exact(2.))?.div_interval_positive(norm)?.1;
    Ok(std::array::from_fn(|axis| {
        if raw_error[axis]==0. && control.iter().all(|q|q[axis]==0.) {0.} else {bound}
    }))
}

fn split_controls(control: [[Scalar;4];4], t: Scalar)
    -> Result<([[Scalar;4];4],[[Scalar;4];4]),AnimationError> {
    let mut level = control;
    let mut left = control;
    let mut right = control;
    for depth in 1..4 {
        for index in 0..4-depth {
            for component in 0..4 {
                level[index][component] = cubic::interpolate(
                    level[index][component],level[index+1][component],t)?;
            }
        }
        left[depth] = level[0];
        right[3-depth] = level[3-depth];
    }
    Ok((left,right))
}

pub(crate) fn quaternion_cubic_restriction_error_bounds(
    control: [[f64;4];4], raw_error: [f64;4], times: [f64;2],
    interval: [f64;2], stored: [[f64;4];4],
) -> Result<[f64;4],AnimationError> {
    if interval[0]<times[0] || interval[1]>times[1] || interval[1]<=interval[0]
        || raw_error.iter().any(|value| !value.is_finite() || *value<0.) {
        return Err(AnimationError::InvalidSampleTime);
    }
    let duration = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
    let mut end = Scalar::exact(interval[1]).sub(Scalar::exact(times[0]))?
        .div_interval_positive(duration)?;
    end.0=end.0.max(0.);end.1=end.1.min(1.);
    let mut enclosed = [[Scalar::exact(0.);4];4];
    for i in 0..4 {
        for axis in 0..4 {
            let lo = Scalar::exact(control[i][axis]).sub(Scalar::exact(raw_error[axis]))?;
            let hi = Scalar::exact(control[i][axis]).add(Scalar::exact(raw_error[axis]))?;
            enclosed[i][axis]=Scalar(lo.0,hi.1);
        }
    }
    let first = split_controls(enclosed,end)?.0;
    let restricted = if interval[0]==times[0] {first} else {
        let mut ratio = Scalar::exact(interval[0]).sub(Scalar::exact(times[0]))?
            .div_interval_positive(Scalar::exact(interval[1]).sub(Scalar::exact(times[0]))?)?;
        ratio.0=ratio.0.max(0.);ratio.1=ratio.1.min(1.);
        split_controls(first,ratio)?.1
    };
    let mut error = [0_f64;4];
    for i in 0..4 {
        for axis in 0..4 {
            let delta = restricted[i][axis].sub(Scalar::exact(stored[i][axis]))?;
            error[axis] = error[axis].max(delta.0.abs().max(delta.1.abs()));
        }
    }
    quaternion_cubic_normalized_error_bounds(stored,error)
}

pub(crate) fn quaternion_cubic_phase_evaluation_error_bounds(
    control: [[f64;4];4], raw_error: [f64;4], times: [f64;2],
    phase: f64, evaluated: [f64;4],
) -> Result<[f64;4],AnimationError> {
    if !phase.is_finite() || phase<times[0] || phase>times[1]
        || raw_error.iter().any(|value|!value.is_finite() || *value<0.)
        || evaluated.iter().any(|value|!value.is_finite()) {
        return Err(AnimationError::InvalidSampleTime);
    }
    let mut u = Scalar::exact(phase).sub(Scalar::exact(times[0]))?
        .div_interval_positive(Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?)?;
    u.0=u.0.max(0.);u.1=u.1.min(1.);
    let mut enclosed = [[Scalar::exact(0.);4];4];
    for i in 0..4 {
        for axis in 0..4 {
            let lo = Scalar::exact(control[i][axis]).sub(Scalar::exact(raw_error[axis]))?;
            let hi = Scalar::exact(control[i][axis]).add(Scalar::exact(raw_error[axis]))?;
            enclosed[i][axis] = Scalar(lo.0,hi.1);
        }
    }
    let value = split_controls(enclosed,u)?.0[3];
    let norm = value[0].square()?.add(value[1].square()?)?.add(value[2].square()?)?
        .add(value[3].square()?)?.sqrt_positive()?;
    let mut error = [0.;4];
    for axis in 0..4 {
        let normalized = value[axis].div_interval_positive(norm)?;
        let discrepancy = normalized.sub(Scalar::exact(evaluated[axis]))?;
        error[axis] = discrepancy.0.abs().max(discrepancy.1.abs());
    }
    Ok(error)
}

pub(crate) fn translation_phase_evaluation_error_bounds(
    coefficients: [DVec3;4], compilation: [f64;3], times: [f64;2],
    phase: f64, evaluated: DVec3,
) -> Result<[f64;3],AnimationError> {
    if !phase.is_finite() || phase<times[0] || phase>times[1] || !evaluated.is_finite()
        || compilation.iter().any(|error|!error.is_finite() || *error<0.) {
        return Err(AnimationError::InvalidSampleTime);
    }
    let mut u = Scalar::exact(phase).sub(Scalar::exact(times[0]))?
        .div_interval_positive(Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?)?;
    u.0=u.0.max(0.);u.1=u.1.min(1.);
    let mut error = [0.;3];
    for axis in 0..3 {
        let mut c = [Scalar::exact(0.);4];
        for i in 0..4 {
            let lo = Scalar::exact(coefficients[i][axis]).sub(Scalar::exact(compilation[axis]))?;
            let hi = Scalar::exact(coefficients[i][axis]).add(Scalar::exact(compilation[axis]))?;
            c[i] = Scalar(lo.0,hi.1);
        }
        let position = c[3].mul(u)?.add(c[2])?.mul(u)?.add(c[1])?.mul(u)?.add(c[0])?;
        let delta = position.sub(Scalar::exact(evaluated[axis]))?;
        error[axis] = delta.0.abs().max(delta.1.abs());
    }
    Ok(error)
}

fn absolute_upper(value: Scalar) -> f64 {value.0.abs().max(value.1.abs())}
fn expanded(value: Scalar, error: f64) -> Result<Scalar,AnimationError> {
    let lo = Scalar::exact(value.0).sub(Scalar::exact(error))?;
    let hi = Scalar::exact(value.1).add(Scalar::exact(error))?;
    Ok(Scalar(lo.0,hi.1))
}
/// A full adjacent f64 spacing bounds nearest rounding over this result range.
fn rounding_error(value: Scalar) -> Result<f64,AnimationError> {
    if !value.is_finite() {return Err(AnimationError::NumericalOverflow);}
    if value.0==value.1 {return Ok(0.);}
    let magnitude = absolute_upper(value);
    let upper = magnitude.next_up();
    if !upper.is_finite() {return Err(AnimationError::NumericalOverflow);}
    Ok(upper-magnitude)
}

pub(crate) fn translation_interval_evaluation_error_bounds(
    coefficients: [DVec3;4], compilation: [f64;3], times: [f64;2],
    interval: [f64;2],
) -> Result<[f64;3],AnimationError> {
    if interval.iter().chain(times.iter()).any(|value|!value.is_finite())
        || interval[0]<times[0] || interval[1]>times[1] || interval[0]>interval[1]
        || compilation.iter().any(|error| !error.is_finite() || *error<0.) {
        return Err(AnimationError::InvalidSampleTime);
    }
    let tracked = rounded_parameter(times,interval)?;
    let parameter = tracked.value;
    let parameter_error = tracked.error.1;
    let mut result = [0.;3];
    for axis in 0..3 {
        let mut value = Scalar::exact(coefficients[3][axis]);
        let mut error = Scalar::exact(0.);
        for coefficient in (0..3).rev() {
            let product = value.mul(parameter)?;
            let product_error = Scalar::exact(absolute_upper(value)).mul(Scalar::exact(parameter_error))?
                .add(Scalar::exact(absolute_upper(parameter)).mul(error)?)?
                .add(error.mul(Scalar::exact(parameter_error))?)?;
            error = product_error.add(Scalar::exact(rounding_error(expanded(product,product_error.1)?)?))?;
            value = product.add(Scalar::exact(coefficients[coefficient][axis]))?;
            error = error.add(Scalar::exact(rounding_error(expanded(value,error.1)?)?))?;
        }
        result[axis] = error.add(Scalar::exact(compilation[axis]))?.1;
    }
    Ok(result)
}

#[derive(Clone,Copy)]
struct RoundedRange {value:Scalar,error:Scalar}
impl RoundedRange {
    fn exact(value:f64)->Self {Self {value:Scalar::exact(value),error:Scalar::exact(0.)}}
    fn add(self,other:Self)->Result<Self,AnimationError> {
        let value=self.value.add(other.value)?;
        let error=self.error.add(other.error)?;
        let error=error.add(Scalar::exact(rounding_error(expanded(value,error.1)?)?))?;
        Ok(Self {value,error})
    }
    fn sub(self,other:Self)->Result<Self,AnimationError> {
        let value=self.value.sub(other.value)?;
        let error=self.error.add(other.error)?;
        let error=error.add(Scalar::exact(rounding_error(expanded(value,error.1)?)?))?;
        Ok(Self {value,error})
    }
    fn mul(self,other:Self)->Result<Self,AnimationError> {
        let value=self.value.mul(other.value)?;
        let error=Scalar::exact(absolute_upper(self.value)).mul(other.error)?
            .add(Scalar::exact(absolute_upper(other.value)).mul(self.error)?)?
            .add(self.error.mul(other.error)?)?;
        let error=error.add(Scalar::exact(rounding_error(expanded(value,error.1)?)?))?;
        Ok(Self {value,error})
    }
}
fn rounded_parameter(times:[f64;2],interval:[f64;2])->Result<RoundedRange,AnimationError> {
    let denominator=Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
    let nominal=times[1]-times[0];
    if denominator.0<=0. || !nominal.is_finite() || nominal<=0. {
        return Err(AnimationError::InvalidSampleTime);
    }
    let numerator=Scalar(interval[0],interval[1]).sub(Scalar::exact(times[0]))?;
    let numerator_error=rounding_error(numerator)?;
    let denominator_error=absolute_upper(denominator.sub(Scalar::exact(nominal))?);
    let mut value=numerator.div_interval_positive(denominator)?;
    value.0=value.0.max(0.);value.1=value.1.min(1.);
    let error=Scalar::exact(numerator_error).div_positive(nominal)?
        .add(Scalar::exact(absolute_upper(numerator)).mul(Scalar::exact(denominator_error))?
            .div_positive(denominator.0)?.div_positive(nominal)?)?;
    let actual_quotient=expanded(numerator,numerator_error)?.div_positive(nominal)?;
    let error=error.add(Scalar::exact(rounding_error(actual_quotient)?))?;
    Ok(RoundedRange {value,error})
}

/// Rounding cap for unit(): divide by maximum component, dot/sqrt/reciprocal,
/// then component multiplication. Matches the inspected glam f64 operation path.
fn scaled_unit_rounding_error()->Result<f64,AnimationError> {
    let ratio=Scalar::exact(rounding_error(Scalar(0.,1.))?);
    let dot=ratio.mul(Scalar::exact(4.))?
        .add(Scalar::exact(rounding_error(Scalar(0.,5.))?).mul(Scalar::exact(3.))?)?;
    if dot.1>=1. {return Err(AnimationError::NumericalOverflow);}
    // The maximum component divides to exactly +/-1; the rounded scaled
    // components have magnitude <=1, so both exact and rounded dot norms >=1.
    // sqrt sensitivity is <=1/2 and reciprocal sensitivity <=1 on that domain.
    Ok(ratio.mul(Scalar::exact(8.))?
        .add(dot.div_positive(2.)?)?
        .add(Scalar::exact(rounding_error(Scalar(1.,3.))?))?
        .add(ratio)?.add(ratio)?.1)
}

pub(crate) fn quaternion_cubic_interval_evaluation_error_bounds(
    control:[[f64;4];4],raw_error:[f64;4],times:[f64;2],interval:[f64;2],
)->Result<[f64;4],AnimationError> {
    if interval.iter().chain(times.iter()).any(|value|!value.is_finite())
        || interval[0]<times[0] || interval[1]>times[1] || interval[0]>interval[1]
        || control.iter().flatten().any(|value|!value.is_finite())
        || raw_error.iter().any(|error|!error.is_finite() || *error<0.) {
        return Err(AnimationError::InvalidSampleTime);
    }
    let u=rounded_parameter(times,interval)?;
    let complement=RoundedRange::exact(1.).sub(u)?;
    let mut level=control.map(|q|q.map(RoundedRange::exact));
    for depth in 1..4 {
        for index in 0..4-depth {
            for axis in 0..4 {
                let a=level[index][axis];let b=level[index+1][axis];
                let mut next=a.mul(complement)?.add(b.mul(u)?)?;
                // Exact nominal interpolation remains inside its endpoint hull.
                next.value=cubic::interpolate(a.value,b.value,u.value)?;
                level[index][axis]=next;
            }
        }
    }
    let mut squared=Scalar::exact(0.);
    let mut total=Scalar::exact(0.);
    for axis in 0..4 {
        let source=expanded(level[0][axis].value,raw_error[axis])?;
        squared=squared.add(source.square()?)?;
        total=total.add(level[0][axis].error)?.add(Scalar::exact(raw_error[axis]))?;
    }
    let norm=squared.sqrt_positive()?;
    if norm.0<=total.1 {return Err(AnimationError::InvalidRootRotationCurve);}
    let error=total.mul(Scalar::exact(2.))?.div_positive(norm.0)?
        .add(Scalar::exact(scaled_unit_rounding_error()?))?.1;
    Ok(std::array::from_fn(|axis|
        if raw_error[axis]==0. && control.iter().all(|q|q[axis]==0.) {0.} else {error}))
}

/// Inputs' errors must enclose real unit rotations. Compare their real Hamilton
/// product to the actual evaluated product (including runtime normalization).
pub(crate) fn quaternion_composition_evaluation_error_bounds(
    a:[f64;4],a_error:[f64;4],b:[f64;4],b_error:[f64;4],evaluated:[f64;4],
)->Result<[f64;4],AnimationError> {
    if a.iter().chain(b.iter()).chain(evaluated.iter()).any(|value|!value.is_finite())
        || a_error.iter().chain(b_error.iter()).any(|error|!error.is_finite() || *error<0.) {
        return Err(AnimationError::NumericalOverflow);
    }
    let enclose=|value:[f64;4],error:[f64;4]| ->Result<RootRigidEnclosure,AnimationError> {
        let mut rotation=[[0.;2];4];
        for axis in 0..4 {
            let interval=expanded(Scalar::exact(value[axis]),error[axis])?;
            rotation[axis]=interval.array();
        }
        Ok(RootRigidEnclosure {translation:[[0.;2];3],rotation})
    };
    let product=enclose(a,a_error)?.compose(&enclose(b,b_error)?)?;
    let mut result=[0.;4];
    for axis in 0..4 {
        let bound=product.rotation[axis];
        let delta=Scalar(bound[0],bound[1]).sub(Scalar::exact(evaluated[axis]))?;
        result[axis]=absolute_upper(delta);
    }
    Ok(result)
}
