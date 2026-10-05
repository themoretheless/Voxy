//! Shared closed finite-capacity heat transfer, positive from a to b.
pub(crate) fn finite_pair_transfer(
    ta: f64,
    ca: f64,
    tb: f64,
    cb: f64,
    g: f64,
    dt: f64,
) -> Result<f64, &'static str> {
    if [ta, ca, tb, cb].iter().any(|v| !v.is_finite() || *v <= 0.)
        || !g.is_finite()
        || g < 0.
        || !dt.is_finite()
        || dt < 0.
    {
        return Err("invalid finite heat exchange");
    }
    if g == 0. || dt == 0. {
        return Ok(0.);
    }
    let reduced = ca.min(cb) / (1. + ca.min(cb) / ca.max(cb));
    if reduced <= 0. {
        return Err("unrepresentable reduced heat capacity");
    }
    let product = g * dt;
    let exponent = if product.is_finite() && product > 0. {
        product / reduced
    } else {
        (g.ln() + dt.ln() - reduced.ln()).exp()
    };
    let fraction = -(-exponent).exp_m1();
    let difference = ta - tb;
    if difference == 0. {
        return Ok(0.);
    }
    let q = if exponent < 1e-8 {
        // The linear heat can be representable even when the relaxation
        // fraction underflows. Avoid multiplying a huge capacity by zero.
        let mut linear = (g * difference) * dt;
        if !linear.is_finite() || linear == 0. {
            linear = (g.ln() + difference.abs().ln() + dt.ln())
                .exp()
                .copysign(difference);
        }
        linear
            * if exponent == 0. {
                1.
            } else {
                fraction / exponent
            }
    } else {
        let full = reduced * difference;
        if full.is_finite() {
            full * fraction
        } else {
            (reduced * fraction) * difference
        }
    };
    if !q.is_finite() || q == 0. {
        return Err("finite heat exchange overflow");
    }
    Ok(q)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_heat_survives_underflowing_fraction_and_overflowing_intermediate_energy() {
        let small = finite_pair_transfer(1e300, 1e300, 1., 1e300, 1e-170, 1e-170).unwrap();
        assert!((small / 1e-40 - 1.).abs() < 1e-12);
        let large = finite_pair_transfer(1e100, 1e300, 1., 1e300, 1., 1.).unwrap();
        assert!((large / 1e100 - 1.).abs() < 1e-12);
        assert_eq!(
            finite_pair_transfer(300., 1., 300., 1., 1e300, 0.).unwrap(),
            0.
        );
    }
}
