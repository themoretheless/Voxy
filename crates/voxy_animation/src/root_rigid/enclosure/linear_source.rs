//! Source-key short-arc logarithms without platform atan/atan2 assumptions.
use super::*;
use std::sync::OnceLock;

// DLMF 4.24.3: alternating atan series. At |x|<=1/2 the remainder
// after 32 terms is bounded by the magnitude of the first omitted term.
fn atan_small(x: Scalar) -> Result<Scalar, AnimationError> {
    if !x.is_finite() || x.0 < -0.5 || x.1 > 0.5 {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let squared = x.square()?;
    let mut power = x;
    let mut sum = x;
    for index in 1..32 {
        power = power.mul(squared)?;
        let term = power.div_positive(f64::from(2 * index + 1))?;
        sum = if index % 2 == 0 {
            sum.add(term)?
        } else {
            sum.sub(term)?
        };
    }
    let next = power.mul(squared)?.div_positive(65.)?;
    let radius = next.0.abs().max(next.1.abs());
    sum.add(Scalar(-radius, radius))
}
fn pi() -> Result<Scalar, AnimationError> {
    static VALUE: OnceLock<Result<Scalar, AnimationError>> = OnceLock::new();
    VALUE
        .get_or_init(|| {
            // Machin identity, derived by tangent addition on the first-quadrant
            // branches: pi = 16 atan(1/5) - 4 atan(1/239).
            atan_small(Scalar::exact(1.).div_positive(5.)?)?
                .mul(Scalar::exact(16.))?
                .sub(atan_small(Scalar::exact(1.).div_positive(239.)?)?.mul(Scalar::exact(4.))?)
        })
        .clone()
}
fn atan_unit_point(x: f64) -> Result<Scalar, AnimationError> {
    if x <= 0.5 {
        return atan_small(Scalar::exact(x));
    }
    let x = Scalar::exact(x);
    let reduced = x
        .sub(Scalar::exact(1.))?
        .div_interval_positive(x.add(Scalar::exact(1.))?)?;
    pi()?.div_positive(4.)?.add(atan_small(reduced)?)
}
fn atan_unit(x: Scalar) -> Result<Scalar, AnimationError> {
    // Each caller has proved the true ratio belongs to [0,1].
    let lo = atan_unit_point(x.0.max(0.))?;
    let hi = atan_unit_point(x.1.min(1.))?;
    Ok(Scalar(lo.0, hi.1))
}
fn quadrant_angle(y: f64, x: f64) -> Result<Scalar, AnimationError> {
    if y == 0. {
        return Ok(Scalar::exact(0.));
    }
    if x == 0. {
        return pi()?.div_positive(2.);
    }
    if y <= x {
        atan_unit(Scalar::exact(y).div_positive(x)?)
    } else {
        pi()?
            .div_positive(2.)?
            .sub(atan_unit(Scalar::exact(x).div_positive(y)?)?)
    }
}

/// Positive acos at one exact stored argument via first-quadrant atan.
pub(super) fn positive_acos_point(input: f64) -> Result<Scalar, AnimationError> {
    if !input.is_finite() || !(0. ..=1.).contains(&input) {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    if input == 1. {
        return Ok(Scalar::exact(0.));
    }
    if input == 0. {
        return pi()?.div_positive(2.);
    }
    let x = Scalar::exact(input);
    // Stable near one; subtracting a rounded x*x could lose the positive gap.
    let y = Scalar::exact(1.)
        .sub(x)?
        .mul(Scalar::exact(1.).add(x)?)?
        .sqrt_positive()?;
    let lo = quadrant_angle(y.0, input)?;
    let hi = quadrant_angle(y.1, input)?;
    Ok(Scalar(lo.0, hi.1))
}

/// Constant spatial angular velocity of the exact normalized f32 source keys.
/// Raw quaternion product scale cancels before logarithm/axis normalization.
/// Ambiguous hemisphere or vanishing-axis corridors reject rather than pick
/// an unproved branch. Constant authored orientations return exactly zero.
pub(crate) fn source_linear_angular_bounds(
    keys: [[f32; 4]; 2],
    times: [f64; 2],
) -> Result<[[f64; 2]; 3], AnimationError> {
    if keys.iter().flatten().any(|v| !v.is_finite())
        || keys.iter().any(|q| q.iter().all(|v| *v == 0.))
    {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    if times.into_iter().any(|v| !v.is_finite()) || times[1] <= times[0] {
        return Err(AnimationError::InvalidSampleTime);
    }
    let [a, b] = keys.map(|q| q.map(f64::from));
    if (0..4).all(|i| (0..4).all(|j| a[i] * b[j] == a[j] * b[i])) {
        return Ok([[0.; 2]; 3]);
    }
    // These products are exact: two f32 significands fit within f64.
    let p = |i: usize, j: usize| Scalar::exact(b[i] * a[j]);
    let mut vector = [
        p(0, 3).sub(p(3, 0))?.sub(p(1, 2))?.add(p(2, 1))?,
        p(1, 3).sub(p(3, 1))?.sub(p(2, 0))?.add(p(0, 2))?,
        p(2, 3).sub(p(3, 2))?.sub(p(0, 1))?.add(p(1, 0))?,
    ];
    let mut dot = p(0, 0).add(p(1, 1))?.add(p(2, 2))?.add(p(3, 3))?;
    if dot.1 < 0. {
        dot = Scalar(-dot.1, -dot.0);
        vector = vector.map(|v| Scalar(-v.1, -v.0));
    } else if dot.0 < 0. {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let norm_squared = vector[0]
        .square()?
        .add(vector[1].square()?)?
        .add(vector[2].square()?)?;
    if norm_squared.0 <= 0. {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let norm = norm_squared.sqrt_positive()?;
    let lo = quadrant_angle(norm.0, dot.1)?;
    let hi = quadrant_angle(norm.1, dot.0)?;
    let angle = Scalar(lo.0.max(0.), hi.1).mul(Scalar::exact(2.))?;
    let duration = Scalar::exact(times[1]).sub(Scalar::exact(times[0]))?;
    let factor = angle
        .div_interval_positive(norm)?
        .div_interval_positive(duration)?;
    let mut result = [[0.; 2]; 3];
    for i in 0..3 {
        result[i] = vector[i].mul(factor)?.array();
    }
    Ok(result)
}

/// Exact source-key normalization. Raw f32 squares are exact f64 products.
pub(crate) fn source_key_rotation_bounds(key: [f32; 4]) -> Result<[[f64; 2]; 4], AnimationError> {
    if key.iter().any(|x| !x.is_finite()) || key.iter().all(|x| *x == 0.) {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let q = key.map(|x| Scalar::exact(f64::from(x)));
    let mut norm = Scalar::exact(0.);
    for x in key.map(f64::from) {
        norm = norm.add(Scalar::exact(x * x))?;
    }
    let norm = norm.sqrt_positive()?;
    let mut result = [[0.; 2]; 4];
    for i in 0..4 {
        result[i] = q[i].div_interval_positive(norm)?.array();
    }
    Ok(result)
}

// Alternating Taylor series with the first omitted term as uniform remainder.
// No platform trigonometric evaluations; |x|<=2 keeps the tail decreasing.
pub(super) fn sine_cosine(x: Scalar) -> Result<(Scalar, Scalar), AnimationError> {
    if !x.is_finite() || x.0 < 0. || x.1 > 2. {
        return Err(AnimationError::InvalidRootRotationCurve);
    }
    let square = x.square()?;
    let mut sine = x;
    let mut cosine = Scalar::exact(1.);
    let mut st = x;
    let mut ct = Scalar::exact(1.);
    for n in 1..32 {
        st = st
            .mul(square)?
            .div_positive(f64::from((2 * n) * (2 * n + 1)))?;
        ct = ct
            .mul(square)?
            .div_positive(f64::from((2 * n - 1) * (2 * n)))?;
        if n % 2 == 0 {
            sine = sine.add(st)?;
            cosine = cosine.add(ct)?;
        } else {
            sine = sine.sub(st)?;
            cosine = cosine.sub(ct)?;
        }
    }
    let sr = st.mul(square)?.div_positive(64. * 65.)?;
    let cr = ct.mul(square)?.div_positive(63. * 64.)?;
    Ok((
        sine.add(Scalar(-sr.1, sr.1))?,
        cosine.add(Scalar(-cr.1, cr.1))?,
    ))
}

/// Unit source slerp lift at an outward affine key fraction. This encloses
/// source normalization and the short-arc exponential, not the cached pose.
pub(crate) fn source_linear_rotation_bounds(
    keys: [[f32; 4]; 2],
    fraction: [f64; 2],
) -> Result<[[f64; 2]; 4], AnimationError> {
    if fraction.iter().any(|x| !x.is_finite())
        || fraction[0] < 0.
        || fraction[1] > 1.
        || fraction[1] < fraction[0]
    {
        return Err(AnimationError::InvalidSampleTime);
    }
    let first = source_key_rotation_bounds(keys[0])?;
    let angular = source_linear_angular_bounds(keys, [0., 1.])?.map(|x| Scalar(x[0], x[1]));
    if angular.iter().all(|x| x.is_zero()) || fraction == [0., 0.] {
        return Ok(first);
    }
    let norm = angular[0]
        .square()?
        .add(angular[1].square()?)?
        .add(angular[2].square()?)?
        .sqrt_positive()?;
    let angle = norm
        .mul(Scalar(fraction[0], fraction[1]))?
        .div_positive(2.)?;
    let (sine, cosine) = sine_cosine(angle)?;
    let mut delta = [[0.; 2]; 4];
    for i in 0..3 {
        delta[i] = angular[i].div_interval_positive(norm)?.mul(sine)?.array();
    }
    delta[3] = cosine.array();
    Ok(RootRigidEnclosure {
        translation: [[0.; 2]; 3],
        rotation: delta,
    }
    .compose(&RootRigidEnclosure {
        translation: [[0.; 2]; 3],
        rotation: first,
    })?
    .rotation_bounds())
}

pub(crate) fn source_relative_rotation_bounds(
    current: [[f64; 2]; 4],
    origin: [[f64; 2]; 4],
) -> Result<[[f64; 2]; 4], AnimationError> {
    let mut inverse = origin;
    for v in &mut inverse[..3] {
        *v = [-v[1], -v[0]];
    }
    Ok(RootRigidEnclosure {
        translation: [[0.; 2]; 3],
        rotation: current,
    }
    .compose(&RootRigidEnclosure {
        translation: [[0.; 2]; 3],
        rotation: inverse,
    })?
    .rotation_bounds())
}
pub(crate) fn source_linear_fraction(
    time: f64,
    keys: [f64; 2],
) -> Result<[f64; 2], AnimationError> {
    let fraction = Scalar::exact(time)
        .sub(Scalar::exact(keys[0]))?
        .div_interval_positive(Scalar::exact(keys[1]).sub(Scalar::exact(keys[0]))?)?;
    // The caller proves time lies inside this key cell.
    Ok([fraction.0.max(0.), fraction.1.min(1.)])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_linear_pose_encloses_short_arc_without_platform_trigonometry() {
        let cases = [
            [[0., 0., 0., 1.], [0., 1e-20, 0., 1.]],
            [[0., 0., 0., 1.], [0., 0.5, 0., 1.]],
            [[0., 0., 0., 1.], [0., 1., 0., 1.]],
            [[0., 0., 0., 1.], [0., 1., 0., 0.]],
            [[0., 0., 0., 1.], [0., 1., 0., -1.]],
            [[0.5, 0.5, 0.5, 0.5], [-0.5, 0.5, 0.5, 0.5]],
        ];
        for keys in cases {
            for u in [0., 0.25, 0.5, 0.75, 1.] {
                let bounds = source_linear_rotation_bounds(keys, [u, u]).unwrap();
                assert!(bounds.iter().all(|x| x[1] - x[0] < 1e-10));
                println!(
                    "SOURCE_LINEAR_POSE {:?}",
                    (keys.map(|q| q.map(f64::from)), u, bounds)
                );
            }
        }
        assert!(source_linear_rotation_bounds(cases[0], [-0.1, 0.]).is_err());
        assert!(source_linear_rotation_bounds(cases[0], [0., 1.1]).is_err());
    }
    #[test]
    fn source_linear_log_covers_small_large_antipodal_and_noncommuting_keys() {
        let cases = [
            [[0., 0., 0., 1.], [0., 1e-20, 0., 1.]],
            [[0., 0., 0., 1.], [0., 0.5, 0., 1.]],
            [[0., 0., 0., 1.], [0., 1., 0., 1.]],
            [[0., 0., 0., 1.], [0., 1., 0., 0.]],
            [[0., 0., 0., 1.], [0., 1., 0., -1.]],
            [[0.5, 0.5, 0.5, 0.5], [-0.5, 0.5, 0.5, 0.5]],
        ];
        for keys in cases {
            let times = [0.25, 0.75];
            let bounds = source_linear_angular_bounds(keys, times).unwrap();
            assert!(bounds.iter().all(|v| v[1] - v[0] < 1e-10));
            println!(
                "SOURCE_LINEAR_LOG {:?}",
                (keys.map(|q| q.map(f64::from)), times, bounds)
            );
        }
        assert_eq!(
            source_linear_angular_bounds([[0.5; 4], [-0.5; 4]], [0., 1.]).unwrap(),
            [[0.; 2]; 3]
        );
        assert!(source_linear_angular_bounds([[0.; 4], [0.; 4]], [0., 1.]).is_err());
        assert!(source_linear_angular_bounds(cases[0], [1., 0.]).is_err());
    }
}
