//! Passive incompressible uniaxial calibration with fixed fiber exponent.
use super::{Fiber, Material};
#[derive(Clone, Copy, Debug)]
pub struct TensilePoint {
    pub stretch: f64,
    pub nominal_stress_pa: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct FitError {
    pub rmse_pa: f64,
    pub max_error_pa: f64,
}
fn basis(stretch: f64, exponent: f64) -> Result<[f64; 2], &'static str> {
    if !stretch.is_finite() || stretch < 1. || !exponent.is_finite() || exponent <= 0. {
        return Err("invalid tensile input");
    }
    let e = stretch * stretch - 1.;
    let b = [
        stretch - stretch.powi(-2),
        2. * stretch * e * (exponent * e * e).exp(),
    ];
    if b.iter().any(|x| !x.is_finite()) {
        return Err("tensile overflow");
    }
    Ok(b)
}
/// Nominal uniaxial stress with traction-free lateral faces, J=1 and a single
/// longitudinal fiber family. It is a calibration test path, not a full-body FE solve.
/// # Errors
/// Rejects incompatible material parameters and invalid stretches.
pub fn tensile_stress(material: &Material, stretch: f64) -> Result<f64, &'static str> {
    material.response(super::IDENTITY, 0.)?;
    if material.fibers.len() != 1
        || (material.fibers[0].direction[0] - 1.).abs() > 1e-8
        || material.fibers[0].direction[1].abs() > 1e-8
        || material.fibers[0].direction[2].abs() > 1e-8
        || material.fibers[0].active_pa != 0.
    {
        return Err("requires single passive longitudinal fiber");
    }
    let f = material.fibers[0];
    let b = basis(stretch, f.exponent)?;
    let stress = material.shear_pa * b[0] + f.stiffness_pa * b[1];
    if !stress.is_finite() {
        return Err("stress overflow");
    }
    Ok(stress)
}
/// Fits shear and aligned fiber stiffness by nonnegative least squares. Bulk
/// modulus and exponent must be supplied; tensile data cannot identify bulk.
/// # Errors
/// Rejects insufficient/degenerate data, negative stresses or invalid parameters.
pub fn fit_tensile(
    points: &[TensilePoint],
    bulk_pa: f64,
    exponent: f64,
) -> Result<Material, &'static str> {
    if points.len() < 3 || !bulk_pa.is_finite() || bulk_pa <= 0. {
        return Err("invalid calibration setup");
    }
    let mut aa = 0.;
    let mut ab = 0.;
    let mut bb = 0.;
    let mut ay = 0.;
    let mut by = 0.;
    for p in points {
        if !p.nominal_stress_pa.is_finite() || p.nominal_stress_pa < 0. {
            return Err("invalid tensile measurement");
        }
        let [a, b] = basis(p.stretch, exponent)?;
        aa += a * a;
        ab += a * b;
        bb += b * b;
        ay += a * p.nominal_stress_pa;
        by += b * p.nominal_stress_pa;
    }
    let determinant = aa * bb - ab * ab;
    if !determinant.is_finite() || determinant <= 1e-10 * aa * bb || aa <= 0. || bb <= 0. {
        return Err("tensile data cannot identify parameters");
    }
    let unconstrained = [
        (ay * bb - by * ab) / determinant,
        (by * aa - ay * ab) / determinant,
    ];
    let [shear_pa, stiffness_pa] = if unconstrained[1] < 0. {
        [ay / aa, 0.]
    } else if unconstrained[0] <= 0. {
        // The nonnegative optimum lies on the zero-matrix boundary. Returning
        // an arbitrary positive shear would conceal non-identifiability.
        return Err("best fit has no positive matrix shear modulus");
    } else {
        unconstrained
    };
    let material = Material {
        shear_pa,
        bulk_pa,
        fibers: vec![Fiber {
            direction: [1., 0., 0.],
            stiffness_pa,
            exponent,
            active_pa: 0.,
        }],
    };
    material.response(super::IDENTITY, 0.)?;
    Ok(material)
}
/// Evaluate on an independently measured, held-out dataset.
/// # Errors
/// Rejects empty/invalid measurements or incompatible material models.
pub fn tensile_error(
    material: &Material,
    points: &[TensilePoint],
) -> Result<FitError, &'static str> {
    if points.is_empty() {
        return Err("empty validation data");
    }
    let mut sum = 0.;
    let mut max_error_pa: f64 = 0.;
    for p in points {
        if !p.nominal_stress_pa.is_finite() || p.nominal_stress_pa < 0. {
            return Err("invalid tensile measurement");
        }
        let error = tensile_stress(material, p.stretch)? - p.nominal_stress_pa;
        sum += error * error;
        max_error_pa = max_error_pa.max(error.abs());
    }
    let count = u32::try_from(points.len()).map_err(|_| "too many tensile measurements")?;
    let rmse_pa = (sum / f64::from(count)).sqrt();
    if !rmse_pa.is_finite() {
        return Err("validation overflow");
    }
    Ok(FitError {
        rmse_pa,
        max_error_pa,
    })
}
