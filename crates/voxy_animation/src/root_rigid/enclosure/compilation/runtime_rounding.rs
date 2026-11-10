//! Runtime evaluation, normalization, retargeting and physical body rounding bounds.
use super::*;

pub(in crate::root_rigid) fn stored_similarity_evaluation_error(
    q: [[f64; 2]; 4],
    point: [[f64; 2]; 3],
    scale: [f64; 2],
    offset: [[f64; 2]; 3],
) -> Result<[f64; 3], AnimationError> {
    if q.iter()
        .chain(point.iter())
        .chain(std::iter::once(&scale))
        .chain(offset.iter())
        .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let input = |v: [f64; 2]| RoundedRange {
        value: Scalar(v[0], v[1]),
        error: Scalar::exact(0.),
    };
    let q = q.map(input);
    let p = point.map(input);
    let scale = input(scale);
    let offset = offset.map(input);
    let rotated = rounded_rotate(q, p)?;
    let mut result = [0.; 3];
    for axis in 0..3 {
        result[axis] = rotated[axis].mul(scale)?.add(offset[axis])?.error.1;
    }
    Ok(result)
}

/// Uniform runtime (a*b).normalize() discrepancy from a real-unit source product.
/// Each supplied error bounds a component of a/b against its real-unit source.
pub(crate) fn quaternion_normalized_composition_uniform_error(
    a_error: [f64; 4],
    b: [f64; 4],
    b_error: [f64; 4],
) -> Result<[f64; 4], AnimationError> {
    if b.iter().any(|v| !v.is_finite())
        || a_error
            .iter()
            .chain(b_error.iter())
            .any(|v| !v.is_finite() || *v < 0.)
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let sum = |values: [f64; 4]| -> Result<Scalar, AnimationError> {
        let mut total = Scalar::exact(0.);
        for value in values {
            total = total.add(Scalar::exact(value))?;
        }
        Ok(total)
    };
    let ea = sum(a_error)?;
    let eb = sum(b_error)?;
    let cap = Scalar::exact(1.).add(ea)?.1;
    let a = [RoundedRange {
        value: Scalar(-cap, cap),
        error: Scalar::exact(0.),
    }; 4];
    let b = b.map(RoundedRange::exact);
    let product = [
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
    let mut discrepancy = ea.add(eb)?.add(ea.mul(eb)?)?;
    for component in product {
        discrepancy = discrepancy.add(component.error)?;
    }
    normalization_uniform_error(discrepancy)
}

pub(in crate::root_rigid) fn normalization_uniform_error(
    discrepancy: Scalar,
) -> Result<[f64; 4], AnimationError> {
    if discrepancy.1 > 0.25 {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    // A raw product within discrepancy of a unit source has norm in
    // [1-discrepancy, 1+discrepancy]. Keep this relational proof even though
    // component boxes alone may all contain zero.
    let lower = Scalar::exact(1.).sub(discrepancy)?.0;
    let upper = Scalar::exact(1.).add(discrepancy)?.1;
    let q = [Scalar(-upper, upper); 4];
    let squared = Scalar(lower, upper).square()?;
    let rounding = normalization_rounding_for_domain(q, Some(squared))?;
    let selection = discrepancy.mul(Scalar::exact(2.))?.div_positive(lower)?;
    let mut result = [0.; 4];
    for i in 0..4 {
        result[i] = selection.add(Scalar::exact(rounding[i]))?.1;
    }
    Ok(result)
}

/// Actual f32 retarget Hamilton chain followed by f32 normalization. Fixed
/// inputs retain their stored values; source_error relates the animated input
/// to its ideal unit quaternion at the same time.
fn f32_rounding_cap(domain: Scalar) -> Result<Scalar, AnimationError> {
    let (axes, _) = RootRigidEnclosure::enclosed_f32_publication_error([domain.array(); 3])?;
    Ok(Scalar::exact(axes[0]))
}
pub(super) fn normalization_f32_uniform_error(
    raw_error: Scalar,
) -> Result<[f64; 4], AnimationError> {
    if raw_error.1 > 0.25 {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    // Relational norm proof: the exact ideal chain is unit even when every
    // component interval contains zero. Do not infer norm from those boxes.
    let lower = Scalar::exact(1.).sub(raw_error)?.0;
    let upper = Scalar::exact(1.).add(raw_error)?.1;
    let square_error = f32_rounding_cap(Scalar(0., Scalar::exact(upper).square()?.1))?;
    let partial = Scalar::exact(upper)
        .square()?
        .add(square_error)?
        .mul(Scalar::exact(8.))?
        .1;
    let dot_error = square_error
        .mul(Scalar::exact(4.))?
        .add(f32_rounding_cap(Scalar(0., partial))?.mul(Scalar::exact(3.))?)?;
    let norm2 = expanded(Scalar(lower, upper).square()?, dot_error.1)?;
    let sqrt = norm2.sqrt_positive()?;
    let sqrt_error = f32_rounding_cap(sqrt)?;
    let norm_error = dot_error
        .div_positive(Scalar::exact(lower).add(Scalar::exact(sqrt.0))?.0)?
        .add(sqrt_error)?;
    let actual_norm = expanded(sqrt, sqrt_error.1)?;
    let reciprocal = Scalar::exact(1.).div_interval_positive(actual_norm)?;
    let reciprocal_error = norm_error
        .div_positive(Scalar::exact(lower).mul(Scalar::exact(actual_norm.0))?.0)?
        .add(f32_rounding_cap(reciprocal)?)?;
    let actual_reciprocal = expanded(reciprocal, f32_rounding_cap(reciprocal)?.1)?;
    let multiplication_error = f32_rounding_cap(Scalar(-upper, upper).mul(actual_reciprocal)?)?;
    let normalization = Scalar::exact(upper)
        .mul(reciprocal_error)?
        .add(multiplication_error)?;
    let selection = raw_error.mul(Scalar::exact(2.))?.div_positive(lower)?;
    Ok([selection.add(normalization)?.1; 4])
}

pub(in crate::root_rigid) fn retarget_rotation_runtime_error(
    target: [f32; 4],
    correction: [f32; 4],
    source_bind: [f32; 4],
    source_error: [f64; 4],
) -> Result<[f64; 4], AnimationError> {
    fn sum(values: [f64; 4]) -> Result<Scalar, AnimationError> {
        if values.iter().any(|v| !v.is_finite() || *v < 0.) {
            return Err(AnimationError::NumericalOverflow);
        }
        values
            .into_iter()
            .try_fold(Scalar::exact(0.), |s, v| s.add(Scalar::exact(v)))
    }
    fn fixed(q: [f32; 4]) -> Result<Scalar, AnimationError> {
        let mut norm2 = Scalar::exact(0.);
        for v in q {
            norm2 = norm2.add(Scalar::exact(f64::from(v)).square()?)?;
        }
        let norm = norm2.sqrt_positive()?;
        let mut error = Scalar::exact(0.);
        for v in q {
            let raw = Scalar::exact(f64::from(v));
            let difference = raw.sub(raw.div_interval_positive(norm)?)?;
            error = error.add(Scalar::exact(absolute_upper(difference)))?;
        }
        Ok(error)
    }
    fn product(a: Scalar, b: Scalar) -> Result<Scalar, AnimationError> {
        let magnitude = Scalar::exact(1.).add(a)?.mul(Scalar::exact(1.).add(b)?)?.1;
        let term_error = f32_rounding_cap(Scalar(-magnitude, magnitude))?;
        let term_cap = Scalar::exact(magnitude).add(term_error)?;
        // Four products and three additions per component. Bound every
        // intermediate partial sum by eight rounded terms, including prior
        // addition rounding (three full f32 spacings), covering left-
        // associated scalar and pairwise SIMD trees; sign changes are exact.
        let partial = term_cap.mul(Scalar::exact(8.))?.1;
        let rounding = term_error
            .mul(Scalar::exact(4.))?
            .add(f32_rounding_cap(Scalar(-partial, partial))?.mul(Scalar::exact(3.))?)?;
        // A unit quaternion has L1 norm at most two (Cauchy-Schwarz).
        // Hamilton convolution therefore transports total errors by two,
        // rather than multiplying each component's worst case by four.
        a.mul(Scalar::exact(2.))?
            .add(b.mul(Scalar::exact(2.))?)?
            .add(a.mul(b)?)?
            .add(rounding.mul(Scalar::exact(4.))?)
    }
    let target = fixed(target)?;
    let correction = fixed(correction)?;
    let bind = fixed(source_bind)?; // conjugation changes signs exactly
    let delta = product(bind, sum(source_error)?)?;
    let prefix = product(target, correction)?;
    let raw_error = product(product(prefix, delta)?, correction)?;
    normalization_f32_uniform_error(raw_error)
}

/// Uniform raw Hamilton product error; deliberately performs no normalization.
pub(crate) fn quaternion_composition_uniform_error(
    a_error: [f64; 4],
    b_error: [f64; 4],
) -> Result<[f64; 4], AnimationError> {
    if a_error
        .iter()
        .chain(b_error.iter())
        .any(|v| !v.is_finite() || *v < 0.)
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let sum = |values: [f64; 4]| -> Result<Scalar, AnimationError> {
        let mut result = Scalar::exact(0.);
        for value in values {
            result = result.add(Scalar::exact(value))?;
        }
        Ok(result)
    };
    let ea = sum(a_error)?;
    let eb = sum(b_error)?;
    let domain = |error: Scalar| -> Result<RoundedRange, AnimationError> {
        let cap = Scalar::exact(1.).add(error)?.1;
        Ok(RoundedRange {
            value: Scalar(-cap, cap),
            error: Scalar::exact(0.),
        })
    };
    let a = domain(ea)?;
    let b = domain(eb)?;
    // Every Hamilton component has four products and three ordered additions
    // or subtractions; symmetric input domains give the same absolute RN cap.
    let term = a.mul(b)?;
    let component = term.add(term)?.add(term)?.add(term)?;
    let source_error = ea.add(eb)?.add(ea.mul(eb)?)?;
    let bound = source_error.add(component.error)?.1;
    Ok([bound; 4])
}

fn rounded_rotate(
    q: [RoundedRange; 4],
    p: [RoundedRange; 3],
) -> Result<[RoundedRange; 3], AnimationError> {
    let dot =
        |a: [RoundedRange; 3], b: [RoundedRange; 3]| -> Result<RoundedRange, AnimationError> {
            a[0].mul(b[0])?.add(a[1].mul(b[1])?)?.add(a[2].mul(b[2])?)
        };
    let b = [q[0], q[1], q[2]];
    let first = q[3].mul(q[3])?.sub(dot(b, b)?)?;
    let second = dot(p, b)?.mul(RoundedRange::exact(2.))?;
    let third = q[3].mul(RoundedRange::exact(2.))?;
    let cross = [
        b[1].mul(p[2])?.sub(p[1].mul(b[2])?)?,
        b[2].mul(p[0])?.sub(p[2].mul(b[0])?)?,
        b[0].mul(p[1])?.sub(p[0].mul(b[1])?)?,
    ];
    Ok([
        p[0].mul(first)?
            .add(b[0].mul(second)?)?
            .add(cross[0].mul(third)?)?,
        p[1].mul(first)?
            .add(b[1].mul(second)?)?
            .add(cross[1].mul(third)?)?,
        p[2].mul(first)?
            .add(b[2].mul(second)?)?
            .add(cross[2].mul(third)?)?,
    ])
}

pub(in crate::root_rigid) fn root_phase_point_error(
    position: [[f64; 2]; 3],
    position_error: [f64; 3],
    rotation: [[f64; 2]; 4],
    rotation_error: [f64; 4],
    origin: DVec3,
    bind: DVec3,
    axes: [bool; 3],
    point: [[f64; 2]; 3],
) -> Result<[f64; 3], AnimationError> {
    if point
        .iter()
        .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let mut p = [RoundedRange::exact(0.); 3];
    let mut pivot = p;
    let mut adjusted = p;
    for i in 0..3 {
        p[i] = RoundedRange {
            value: Scalar(position[i][0], position[i][1]),
            error: Scalar::exact(position_error[i]),
        }
        .add(RoundedRange::exact(origin[i]))?;
        pivot[i] = if axes[i] {
            RoundedRange::exact(bind[i])
        } else {
            p[i]
        };
        adjusted[i] = if axes[i] {
            p[i].sub(RoundedRange::exact(origin[i]).sub(RoundedRange::exact(bind[i]))?)?
        } else {
            p[i]
        };
    }
    let q = std::array::from_fn(|i| RoundedRange {
        value: Scalar(rotation[i][0], rotation[i][1]),
        error: Scalar::exact(rotation_error[i]),
    });
    let pivot_image = rounded_rotate(q, pivot)?;
    let image = rounded_rotate(
        q,
        point.map(|v| RoundedRange {
            value: Scalar(v[0], v[1]),
            error: Scalar::exact(0.),
        }),
    )?;
    let mut result = [0.; 3];
    for i in 0..3 {
        result[i] = adjusted[i].sub(pivot_image[i])?.add(image[i])?.error.1;
    }
    Ok(result)
}

pub(in crate::root_rigid) fn composed_rigid_point_error(
    prefix: &RootRigidEnclosure,
    prefix_t_error: [f64; 3],
    prefix_q_error: [f64; 4],
    local: &RootRigidEnclosure,
    local_t_error: [f64; 3],
    local_q_error: [f64; 4],
    point: [[f64; 2]; 3],
) -> Result<[f64; 3], AnimationError> {
    if point
        .iter()
        .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let input = |bounds: [[f64; 2]; 3], error: [f64; 3]| {
        std::array::from_fn(|i| RoundedRange {
            value: Scalar(bounds[i][0], bounds[i][1]),
            error: Scalar::exact(error[i]),
        })
    };
    let prefix_t = input(prefix.translation_bounds(), prefix_t_error);
    let local_t = input(local.translation_bounds(), local_t_error);
    let prefix_q = std::array::from_fn(|i| RoundedRange {
        value: Scalar(prefix.rotation[i][0], prefix.rotation[i][1]),
        error: Scalar::exact(prefix_q_error[i]),
    });
    let translated = rounded_rotate(prefix_q, local_t)?;
    let raw_error = quaternion_composition_uniform_error(prefix_q_error, local_q_error)?;
    let mut discrepancy = Scalar::exact(0.);
    for value in raw_error {
        discrepancy = discrepancy.add(Scalar::exact(value))?;
    }
    let q_error = normalization_uniform_error(discrepancy)?;
    let source = prefix.compose(local)?;
    let q = std::array::from_fn(|i| RoundedRange {
        value: Scalar(source.rotation[i][0], source.rotation[i][1]),
        error: Scalar::exact(q_error[i]),
    });
    let image = rounded_rotate(
        q,
        point.map(|v| RoundedRange {
            value: Scalar(v[0], v[1]),
            error: Scalar::exact(0.),
        }),
    )?;
    let mut result = [0.; 3];
    for i in 0..3 {
        result[i] = prefix_t[i].add(translated[i])?.add(image[i])?.error.1;
    }
    Ok(result)
}

pub(in crate::root_rigid) fn mapped_point_runtime_error(
    point: [[f64; 2]; 3],
    point_error: [f64; 3],
    frame: &RootRigidEnclosure,
    scale: RootUniformScaleEnclosure,
    actual: RootRigidTransform,
    actual_scale: f64,
) -> Result<([f64; 3], [[f64; 2]; 3]), AnimationError> {
    actual.checked()?;
    if !actual_scale.is_finite() {
        return Err(AnimationError::NumericalOverflow);
    }
    let discrepancy = |source: Scalar, value: f64| -> Result<Scalar, AnimationError> {
        let delta = source.sub(Scalar::exact(value))?;
        Ok(Scalar::exact(absolute_upper(delta)))
    };
    let mut q = [RoundedRange::exact(0.); 4];
    for i in 0..4 {
        let value = Scalar(frame.rotation[i][0], frame.rotation[i][1]);
        q[i] = RoundedRange {
            value,
            error: discrepancy(value, actual.rotation.to_array()[i])?,
        };
    }
    let p = std::array::from_fn(|i| RoundedRange {
        value: Scalar(point[i][0], point[i][1]),
        error: Scalar::exact(point_error[i]),
    });
    let rotated = rounded_rotate(q, p)?;
    let scale = RoundedRange {
        value: scale.value,
        error: discrepancy(scale.value, actual_scale)?,
    };
    let mut error = [0.; 3];
    let mut actual_box = [[0.; 2]; 3];
    for i in 0..3 {
        let value = Scalar(frame.translation[i][0], frame.translation[i][1]);
        let offset = RoundedRange {
            value,
            error: discrepancy(value, actual.translation[i])?,
        };
        let mapped = rotated[i].mul(scale)?.add(offset)?;
        error[i] = mapped.error.1;
        actual_box[i] = expanded(mapped.value, mapped.error.1)?.array();
    }
    Ok((error, actual_box))
}

pub(in crate::root_rigid) fn retarget_translation_runtime_error(
    position: [[f64; 2]; 3],
    source_error: [f64; 3],
    source_bind: DVec3,
    target_bind: DVec3,
    basis: DQuat,
    scale: f64,
) -> Result<([f64; 3], f64), AnimationError> {
    if position
        .iter()
        .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
        || source_error.iter().any(|v| !v.is_finite() || *v < 0.)
        || !source_bind.is_finite()
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let mut delta = [[0.; 2]; 3];
    let mut delta_error = [0.; 3];
    for axis in 0..3 {
        let value = RoundedRange {
            value: Scalar(position[axis][0], position[axis][1]),
            error: Scalar::exact(source_error[axis]),
        }
        .sub(RoundedRange::exact(source_bind[axis]))?;
        delta[axis] = value.value.array();
        delta_error[axis] = value.error.1;
    }
    let actual = RootRigidTransform {
        translation: target_bind,
        rotation: basis,
    };
    let frame = RootRigidEnclosure::from_transform(actual)?;
    let (evaluation, actual_box) = mapped_point_runtime_error(
        delta,
        delta_error,
        &frame,
        RootUniformScaleEnclosure::from_scale(scale)?,
        actual,
        scale,
    )?;
    let (publication, _) = RootRigidEnclosure::enclosed_f32_publication_error(actual_box)?;
    let mut result = [0.; 3];
    let mut radius = Scalar::exact(0.);
    for axis in 0..3 {
        result[axis] = Scalar::exact(evaluation[axis])
            .add(Scalar::exact(publication[axis]))?
            .1;
        radius = radius.add(Scalar::exact(result[axis]))?;
    }
    Ok((result, radius.1))
}

/// Uniform rounding discrepancy of glam 0.33.7 DQuat::normalize against
/// exact real normalization of the same stored quaternion. No source error.
pub(in crate::root_rigid) fn stored_quaternion_normalization_error(
    input: [[f64; 2]; 4],
) -> Result<[f64; 4], AnimationError> {
    if input
        .iter()
        .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let q = input.map(|v| Scalar(v[0], v[1]));
    normalization_rounding_for_domain(q, None)
}

// A supplied squared-norm interval must follow from a private relational proof;
// external callers cannot assert one for an arbitrary input box.
fn normalization_rounding_for_domain(
    q: [Scalar; 4],
    squared_norm: Option<Scalar>,
) -> Result<[f64; 4], AnimationError> {
    let mut products = [RoundedRange::exact(0.); 4];
    for i in 0..4 {
        let square = q[i].square()?;
        products[i] = RoundedRange {
            value: square,
            error: Scalar::exact(rounding_error(square)?),
        };
    }
    // glam uses four products followed by three left-associated additions.
    let dot = products[0]
        .add(products[1])?
        .add(products[2])?
        .add(products[3])?;
    let squared = squared_norm.unwrap_or(dot.value);
    let exact_norm = squared.sqrt_positive()?;
    let raw_norm = expanded(squared, dot.error.1)?.sqrt_positive()?;
    let sqrt_rounding = rounding_error(raw_norm)?;
    let norm_error = dot
        .error
        .div_interval_positive(Scalar::exact(exact_norm.0).add(Scalar::exact(raw_norm.0))?)?
        .add(Scalar::exact(sqrt_rounding))?;
    let actual_norm = expanded(raw_norm, sqrt_rounding)?;
    let reciprocal_range = Scalar::exact(1.).div_interval_positive(actual_norm)?;
    let reciprocal_rounding = rounding_error(reciprocal_range)?;
    let reciprocal_error = norm_error
        .div_interval_positive(Scalar::exact(exact_norm.0).mul(Scalar::exact(actual_norm.0))?)?
        .add(Scalar::exact(reciprocal_rounding))?;
    let actual_reciprocal = expanded(reciprocal_range, reciprocal_rounding)?;
    let mut result = [0.; 4];
    for i in 0..4 {
        result[i] = Scalar::exact(absolute_upper(q[i]))
            .mul(reciprocal_error)?
            .add(Scalar::exact(rounding_error(q[i].mul(actual_reciprocal)?)?))?
            .1;
    }
    Ok(result)
}

/// Uniform component/operation error for a family of evaluated rigid poses.
/// Caller-provided errors must bound those same canonical source pose components.
pub(in crate::root_rigid) fn rigid_point_runtime_error(
    source: &RootRigidEnclosure,
    translation_error: [f64; 3],
    rotation_error: [f64; 4],
    point: [[f64; 2]; 3],
) -> Result<[f64; 3], AnimationError> {
    if point
        .iter()
        .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
        || translation_error
            .iter()
            .chain(rotation_error.iter())
            .any(|v| !v.is_finite() || *v < 0.)
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let q = std::array::from_fn(|i| RoundedRange {
        value: Scalar(source.rotation[i][0], source.rotation[i][1]),
        error: Scalar::exact(rotation_error[i]),
    });
    let p = point.map(|v| RoundedRange {
        value: Scalar(v[0], v[1]),
        error: Scalar::exact(0.),
    });
    let rotated = rounded_rotate(q, p)?;
    let mut result = [0.; 3];
    for i in 0..3 {
        let offset = RoundedRange {
            value: Scalar(source.translation[i][0], source.translation[i][1]),
            error: Scalar::exact(translation_error[i]),
        };
        result[i] = rotated[i].add(offset)?.error.1;
    }
    Ok(result)
}

fn rounded_cross_rotate(
    q: [RoundedRange; 4],
    p: [RoundedRange; 3],
) -> Result<[RoundedRange; 3], AnimationError> {
    let v = [q[0], q[1], q[2]];
    let cross =
        |a: [RoundedRange; 3], b: [RoundedRange; 3]| -> Result<[RoundedRange; 3], AnimationError> {
            Ok([
                a[1].mul(b[2])?.sub(a[2].mul(b[1])?)?,
                a[2].mul(b[0])?.sub(a[0].mul(b[2])?)?,
                a[0].mul(b[1])?.sub(a[1].mul(b[0])?)?,
            ])
        };
    let first = cross(v, p)?;
    let twice = [
        first[0].mul(RoundedRange::exact(2.))?,
        first[1].mul(RoundedRange::exact(2.))?,
        first[2].mul(RoundedRange::exact(2.))?,
    ];
    let second = cross(v, twice)?;
    Ok([
        p[0].add(q[3].mul(twice[0])?)?.add(second[0])?,
        p[1].add(q[3].mul(twice[1])?)?.add(second[1])?,
        p[2].add(q[3].mul(twice[2])?)?.add(second[2])?,
    ])
}

pub(in crate::root_rigid) fn physical_rotation_runtime_error(
    source_rotation: [[f64; 2]; 4],
    selection_error: [f64; 4],
    basis: DQuat,
    orientation: DQuat,
) -> Result<[f64; 4], AnimationError> {
    Ok(physical_rotation_runtime_errors(source_rotation, selection_error, basis, orientation)?.1)
}

fn physical_rotation_runtime_errors(
    source_rotation: [[f64; 2]; 4],
    selection_error: [f64; 4],
    basis: DQuat,
    orientation: DQuat,
) -> Result<([f64; 4], [f64; 4]), AnimationError> {
    let source = |q: DQuat| {
        RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: DVec3::ZERO,
            rotation: q,
        })
    };
    let basis_source = source(basis)?;
    let actor_source = source(orientation)?;
    let discrepancy = |bounds: [f64; 2], actual: f64| -> Result<f64, AnimationError> {
        Ok(absolute_upper(
            Scalar(bounds[0], bounds[1]).sub(Scalar::exact(actual))?,
        ))
    };
    let mut b = [RoundedRange::exact(0.); 4];
    let mut actor_error = [0.; 4];
    for i in 0..4 {
        b[i] = RoundedRange {
            value: Scalar(basis_source.rotation[i][0], basis_source.rotation[i][1]),
            error: Scalar::exact(discrepancy(basis_source.rotation[i], basis.to_array()[i])?),
        };
        actor_error[i] = discrepancy(actor_source.rotation[i], orientation.to_array()[i])?;
    }
    let p = std::array::from_fn(|i| RoundedRange {
        value: Scalar(source_rotation[i][0], source_rotation[i][1]),
        error: Scalar::exact(selection_error[i]),
    });
    let vector = rounded_cross_rotate(b, p)?;
    let mut reframed_raw = Scalar::exact(selection_error[3]);
    for component in vector {
        reframed_raw = reframed_raw.add(component.error)?;
    }
    // Structural coordinate-row replacement in the controller is exact for
    // the real normalization of this same stored basis. Its only discrepancy
    // is the input component error, already included by the cross-form cap.
    let reframed = normalization_uniform_error(reframed_raw)?;
    let composed = quaternion_composition_uniform_error(actor_error, reframed)?;
    let mut raw = Scalar::exact(0.);
    for error in composed {
        raw = raw.add(Scalar::exact(error))?;
    }
    Ok((reframed, normalization_uniform_error(raw)?))
}

pub(in crate::root_rigid) fn physical_body_runtime_error(
    source: &RootRigidEnclosure,
    translation_error: [f64; 3],
    rotation_error: [f64; 4],
    center: DVec3,
    edges: [DVec3; 3],
    basis: DQuat,
    origin: DVec3,
    orientation: DQuat,
    scale: f64,
) -> Result<[f64; 3], AnimationError> {
    Ok(physical_body_runtime_domains(
        source,
        translation_error,
        rotation_error,
        center,
        edges,
        basis,
        origin,
        orientation,
        scale,
    )?
    .0)
}

pub(in crate::root_rigid) fn physical_body_runtime_domains(
    source: &RootRigidEnclosure,
    translation_error: [f64; 3],
    rotation_error: [f64; 4],
    center: DVec3,
    edges: [DVec3; 3],
    basis: DQuat,
    origin: DVec3,
    orientation: DQuat,
    scale: f64,
) -> Result<([f64; 3], [[f64; 2]; 3], [[f64; 2]; 3]), AnimationError> {
    if !center.is_finite()
        || edges.iter().any(|v| !v.is_finite())
        || !origin.is_finite()
        || !scale.is_finite()
        || scale == 0.
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let fixed = |q: DQuat| {
        RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: DVec3::ZERO,
            rotation: q,
        })
    };
    let basis_source = fixed(basis)?;
    let actor_source = fixed(orientation)?;
    let reference_rotation = basis_source
        .compose(source)?
        .compose(&basis_source.inverse()?)?;
    let body_rotation = actor_source.compose(&reference_rotation)?;
    let (reframed_error, body_error) =
        physical_rotation_runtime_errors(source.rotation, rotation_error, basis, orientation)?;
    let input_q = |reference: &RootRigidEnclosure, error: [f64; 4]| {
        std::array::from_fn(|i| RoundedRange {
            value: Scalar(reference.rotation[i][0], reference.rotation[i][1]),
            error: Scalar::exact(error[i]),
        })
    };
    let fixed_q = |reference: &RootRigidEnclosure,
                   actual: DQuat|
     -> Result<[RoundedRange; 4], AnimationError> {
        let mut error = [0.; 4];
        for i in 0..4 {
            error[i] = absolute_upper(
                Scalar(reference.rotation[i][0], reference.rotation[i][1])
                    .sub(Scalar::exact(actual.to_array()[i]))?,
            );
        }
        Ok(input_q(reference, error))
    };
    // A coordinate-row override evaluates the exact real normalization of its
    // stored quaternion. Its distance from the canonical unit source is at
    // most 2*L1(raw discrepancy)/(1-L1(raw discrepancy)). Inflating all component
    // errors by this bound also covers the ordinary cross-form branch.
    let structural_errors = |error: [f64; 4]| -> Result<[f64; 4], AnimationError> {
        let mut total = Scalar::exact(0.);
        for e in error {
            total = total.add(Scalar::exact(e))?;
        }
        let lower = Scalar::exact(1.).sub(total)?.0;
        Ok([total.mul(Scalar::exact(2.))?.div_positive(lower)?.1; 4])
    };
    let actor = fixed_q(&actor_source, orientation)?;
    let b = fixed_q(&basis_source, basis)?;
    let reframe = input_q(&reference_rotation, structural_errors(reframed_error)?);
    let body = input_q(&body_rotation, structural_errors(body_error)?);
    let translation = std::array::from_fn(|i| RoundedRange {
        value: Scalar(source.translation[i][0], source.translation[i][1]),
        error: Scalar::exact(translation_error[i]),
    });
    let mapped_translation = rounded_cross_rotate(b, translation)?;
    let pivot = origin.to_array().map(RoundedRange::exact);
    let pivot_image = rounded_cross_rotate(reframe, pivot)?;
    let mut local = [RoundedRange::exact(0.); 3];
    for i in 0..3 {
        local[i] = mapped_translation[i]
            .mul(RoundedRange::exact(scale))?
            .add(pivot[i])?
            .sub(pivot_image[i])?;
    }
    let displacement = rounded_cross_rotate(actor, local)?;
    let mut moved = [RoundedRange::exact(0.); 3];
    for i in 0..3 {
        moved[i] = RoundedRange::exact(center[i]).add(displacement[i])?;
    }
    let mut rotated_edges = [[RoundedRange::exact(0.); 3]; 3];
    for i in 0..3 {
        rotated_edges[i] =
            rounded_cross_rotate(body, edges[i].to_array().map(RoundedRange::exact))?;
    }
    let mut result = [0_f64; 3];
    for corner in 0..8 {
        for axis in 0..3 {
            let mut value = moved[axis];
            for (i, edge) in rotated_edges.iter().enumerate() {
                value = value.add(edge[axis].mul(RoundedRange::exact(
                    if corner & (1 << i) == 0 { -1. } else { 1. },
                ))?)?;
            }
            result[axis] = result[axis].max(value.error.1);
        }
    }
    let mut center_domain = [[0.; 2]; 3];
    let mut half_domain = [[0.; 2]; 3];
    for axis in 0..3 {
        center_domain[axis] = expanded(moved[axis].value, moved[axis].error.1)?.array();
        let mut half = Scalar::exact(0.);
        for edge in rotated_edges {
            let component = expanded(edge[axis].value, edge[axis].error.1)?;
            half = half.add(Scalar(0., absolute_upper(component)))?;
        }
        half_domain[axis] = [0., half.1];
    }
    Ok((result, center_domain, half_domain))
}

/// Numeric error relative to the exact stored-input snap displacement, for
/// every snap fraction in [0,1], followed by zero-anchor min/max reconstruction.
pub(in crate::root_rigid) fn snap_center_reconstruction_error(
    center: [[f64; 2]; 3],
    half: [[f64; 2]; 3],
    snap: DVec3,
) -> Result<[f64; 3], AnimationError> {
    if !snap.is_finite()
        || center
            .iter()
            .chain(half.iter())
            .any(|v| !v[0].is_finite() || !v[1].is_finite() || v[0] > v[1])
        || half.iter().any(|v| v[0] < 0.)
    {
        return Err(AnimationError::NumericalOverflow);
    }
    let input = |v: [f64; 2]| RoundedRange {
        value: Scalar(v[0], v[1]),
        error: Scalar::exact(0.),
    };
    let fraction = input([0., 1.]);
    let mut result = [0.; 3];
    for axis in 0..3 {
        let displacement = RoundedRange::exact(snap[axis]).mul(fraction)?;
        let moved = input(center[axis]).add(displacement)?;
        let minimum = moved.sub(input(half[axis]))?;
        let maximum = moved.add(input(half[axis]))?;
        let reconstructed = minimum.add(maximum)?.mul(RoundedRange::exact(0.5))?;
        result[axis] = reconstructed.error.1;
    }
    Ok(result)
}
