//! Geometric optics at a smooth, lossless, unpolarized dielectric interface.
//! Returns power fractions, NOT radiance multipliers or a traced scene hit.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DielectricBoundarySample {
    reflected: [f64; 3],
    transmitted: Option<[f64; 3]>,
    reflectance: f64,
    transmittance: f64,
}
impl DielectricBoundarySample {
    #[must_use]
    pub fn reflected_direction(self) -> [f64; 3] {
        self.reflected
    }
    #[must_use]
    pub fn transmitted_direction(self) -> Option<[f64; 3]> {
        self.transmitted
    }
    #[must_use]
    pub fn reflected_power_fraction(self) -> f64 {
        self.reflectance
    }
    #[must_use]
    pub fn transmitted_power_fraction(self) -> f64 {
        self.transmittance
    }
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn unit(v: [f64; 3]) -> Result<[f64; 3], &'static str> {
    if v.iter().any(|x| !x.is_finite()) {
        return Err("invalid dielectric direction");
    }
    let length = v.iter().fold(0_f64, |a, v| a.hypot(*v));
    if !length.is_finite() || (length - 1.).abs() > 1e-10 {
        return Err("dielectric direction must be unit length");
    }
    Ok(v.map(|x| x / length))
}
/// Incident direction travels TOWARD the boundary. Normal points into the
/// incident medium; its dot product with the incident direction must be <=0.
/// Refractive indices are explicit for the incident and transmitted media.
/// # Errors
/// Nonunit/nonfinite directions, reversed normal, nonpositive/nonfinite indices
/// or an index ratio that cannot be represented in f64.
pub fn dielectric_boundary_sample(
    incident: [f64; 3],
    normal: [f64; 3],
    incident_ior: f64,
    transmitted_ior: f64,
) -> Result<DielectricBoundarySample, &'static str> {
    let d = unit(incident)?;
    let n = unit(normal)?;
    if !incident_ior.is_finite()
        || !transmitted_ior.is_finite()
        || incident_ior <= 0.
        || transmitted_ior <= 0.
    {
        return Err("invalid dielectric refractive index");
    }
    let cosine = -dot(d, n);
    if cosine < 0. {
        return Err("dielectric normal points into transmitted medium");
    }
    let cosine = cosine.min(1.);
    let reflected = std::array::from_fn(|i| d[i] + 2. * cosine * n[i]);
    if incident_ior == transmitted_ior {
        return Ok(DielectricBoundarySample {
            reflected,
            transmitted: Some(d),
            reflectance: 0.,
            transmittance: 1.,
        });
    }
    let eta = incident_ior / transmitted_ior;
    if !eta.is_finite() || eta == 0. {
        return Err("unrepresentable dielectric index ratio");
    }
    // Tangential norm avoids 1-cos^2 cancellation at almost normal incidence.
    let tangent: [f64; 3] = std::array::from_fn(|i| d[i] + cosine * n[i]);
    let sine = tangent.iter().fold(0_f64, |a, v| a.hypot(*v));
    let transmitted_sine = eta * sine;
    if transmitted_sine >= 1. {
        return Ok(DielectricBoundarySample {
            reflected,
            transmitted: None,
            reflectance: 1.,
            transmittance: 0.,
        });
    }
    let transmitted_cosine = (1. - transmitted_sine * transmitted_sine).max(0.).sqrt();
    let transmitted = std::array::from_fn(|i| eta * tangent[i] - transmitted_cosine * n[i]);
    // Normalize indices before Fresnel sums/products to avoid overflowing them.
    let scale = incident_ior.max(transmitted_ior);
    let ni = incident_ior / scale;
    let nt = transmitted_ior / scale;
    let polarization = |a: f64, b: f64| {
        let denominator = a + b;
        let r = (a - b) / denominator;
        // Compute transmitted power directly: subtracting R from one would
        // destroy a small but representable transmission for high contrast.
        (r * r, (2. * a / denominator) * (2. * b / denominator))
    };
    let s = polarization(ni * cosine, nt * transmitted_cosine);
    let p = polarization(nt * cosine, ni * transmitted_cosine);
    let result = DielectricBoundarySample {
        reflected,
        transmitted: Some(transmitted),
        reflectance: 0.5 * (s.0 + p.0),
        transmittance: 0.5 * (s.1 + p.1),
    };
    if !result.reflectance.is_finite()
        || !result.transmittance.is_finite()
        || transmitted.iter().any(|x| !x.is_finite())
    {
        return Err("dielectric boundary arithmetic overflow");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn direction(angle: f64) -> [f64; 3] {
        [angle.sin(), 0., -angle.cos()]
    }
    #[test]
    fn normal_incidence_identity_and_brewster_angle() {
        let sample = dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], 1., 1.5).unwrap();
        assert_eq!(sample.reflected_direction(), [0., 0., 1.]);
        assert_eq!(sample.transmitted_direction(), Some([0., 0., -1.]));
        assert!((sample.reflectance - 0.04).abs() < 1e-15);
        assert!((sample.transmittance - 0.96).abs() < 1e-15);
        let grazing = dielectric_boundary_sample([1., 0., 0.], [0., 0., 1.], 1.333, 1.333).unwrap();
        assert_eq!(grazing.transmitted_direction(), Some([1., 0., 0.]));
        assert_eq!(grazing.reflectance, 0.);
        let brewster =
            dielectric_boundary_sample(direction(1.5_f64.atan()), [0., 0., 1.], 1., 1.5).unwrap();
        // At Brewster angle p-polarized reflectance vanishes; independent s value.
        let rs = ((1. - 1.5_f64.powi(2)) / (1. + 1.5_f64.powi(2))).powi(2);
        assert!((brewster.reflectance - 0.5 * rs).abs() < 1e-14);
    }
    #[test]
    fn snell_flux_and_reciprocity_for_entering_and_exiting_media() {
        for (ni, nt) in [(1., 1.333), (1.333, 1.), (1., 2.42), (2.42, 1.)] {
            for degrees in [0_f64, 10., 30., 45., 60., 80., 89.] {
                let d = direction(degrees.to_radians());
                let s = dielectric_boundary_sample(d, [0., 0., 1.], ni, nt).unwrap();
                assert!((s.reflectance + s.transmittance - 1.).abs() < 1e-14);
                assert!((dot(s.reflected, s.reflected) - 1.).abs() < 1e-14);
                if let Some(t) = s.transmitted {
                    assert!((dot(t, t) - 1.).abs() < 1e-14);
                    assert!((ni * d[0] - nt * t[0]).abs() < 1e-14);
                    let reverse =
                        dielectric_boundary_sample(t.map(|x| -x), [0., 0., -1.], nt, ni).unwrap();
                    let returned = reverse.transmitted.unwrap();
                    for axis in 0..3 {
                        assert!((returned[axis] + d[axis]).abs() < 1e-13);
                    }
                    assert!((reverse.reflectance - s.reflectance).abs() < 1e-13);
                } else {
                    assert!(ni * d[0] >= nt);
                    assert_eq!(s.reflectance, 1.);
                }
            }
        }
        let critical = (1_f64 / 1.5).asin();
        assert!(
            dielectric_boundary_sample(direction(critical - 1e-8), [0., 0., 1.], 1.5, 1.)
                .unwrap()
                .transmitted
                .is_some()
        );
        assert!(
            dielectric_boundary_sample(direction(critical + 1e-8), [0., 0., 1.], 1.5, 1.)
                .unwrap()
                .transmitted
                .is_none()
        );
    }
    #[test]
    fn invalid_inputs_and_high_contrast_do_not_silently_default() {
        for bad in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], bad, 1.).is_err());
            assert!(dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], 1., bad).is_err());
        }
        assert!(dielectric_boundary_sample([0., 0., -2.], [0., 0., 1.], 1., 1.5).is_err());
        assert!(dielectric_boundary_sample([0., 0., -1.], [0., 0., -1.], 1., 1.5).is_err());
        assert!(
            dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], f64::MAX, f64::MIN_POSITIVE)
                .is_err()
        );
        let high = dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], 1., 1e300).unwrap();
        assert!(high.transmittance > 0.);
        assert!((high.transmittance / 4e-300 - 1.).abs() < 1e-14);
        let scaled =
            dielectric_boundary_sample(direction(0.5), [0., 0., 1.], 1e300, 1.5e300).unwrap();
        let ordinary = dielectric_boundary_sample(direction(0.5), [0., 0., 1.], 1., 1.5).unwrap();
        assert!((scaled.reflectance - ordinary.reflectance).abs() < 1e-14);
    }
}
