//! Stable strain invariant shared by constitutive material laws.
use super::{Matrix, det, mm, transpose};

pub(super) fn isochoric_excess(f: Matrix, q: f64, isochoric_trace: f64) -> f64 {
    isochoric_metric_excess(
        mm(transpose(f), f).map(|r| r.map(|v| q * v)),
        isochoric_trace,
    )
}

/// Reuse an already admitted isochoric metric without rebuilding F-transpose F.
pub(super) fn isochoric_metric_excess(metric: Matrix, isochoric_trace: f64) -> f64 {
    // det(Cbar)=1 implies tr(D)=-I2(D)-det(D), D=Cbar-I.
    // Near rest this avoids subtracting two O(1) values to recover O(strain²).
    let mut deviation = metric;
    for (i, row) in deviation.iter_mut().enumerate() {
        row[i] -= 1.;
    }
    if deviation.iter().flatten().all(|v| v.abs() < 0.1) {
        let second = deviation[0][0] * deviation[1][1]
            + deviation[0][0] * deviation[2][2]
            + deviation[1][1] * deviation[2][2]
            - deviation[0][1] * deviation[1][0]
            - deviation[0][2] * deviation[2][0]
            - deviation[1][2] * deviation[2][1];
        (-second - det(deviation)).max(0.)
    } else {
        isochoric_trace - 3.
    }
}

/// Computes the $\overline{F}$-projected deformation gradient
/// $\overline{F} = (\overline{J} / J)^{1/3} F$ (de Souza Neto et al.) to eliminate
/// volumetric locking in nearly incompressible 4-node tetrahedral patches where
/// $\overline{J} > 0$ is the patch-averaged Jacobian determinant.
///
/// # Errors
///
/// Rejects non-finite matrices, inverted elements ($J \le 0$), or non-positive $\overline{J}$.
pub fn f_bar_modified_gradient(f: Matrix, j_bar: f64) -> Result<Matrix, &'static str> {
    if f.iter().flatten().any(|v| !v.is_finite()) || !j_bar.is_finite() || j_bar <= 0.0 {
        return Err("invalid F-bar deformation input");
    }
    let j = det(f);
    if !j.is_finite() || j <= 0.0 {
        return Err("inverted element in F-bar projection");
    }
    let scale = (j_bar / j).cbrt();
    if !scale.is_finite() || scale <= 0.0 {
        return Err("F-bar scale overflow");
    }
    Ok(f.map(|row| row.map(|v| v * scale)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biomechanics::IDENTITY;

    #[test]
    fn f_bar_replaces_element_determinant_with_patch_average() {
        let f = [[1.2, 0.0, 0.0], [0.0, 0.9, 0.0], [0.0, 0.0, 1.1]];
        let modified = f_bar_modified_gradient(f, 1.0).unwrap();
        assert!((det(modified) - 1.0).abs() < 1e-12);
        assert!(f_bar_modified_gradient(IDENTITY, -0.5).is_err());
    }
}
