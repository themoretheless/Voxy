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

#[cfg(test)]
mod tests {
    use super::*;
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
