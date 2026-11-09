//! Layered, anisotropic, nonlinear, viscoelastic skin shell in SI units.
//! Material energies are differentiated exactly and solved implicitly.
mod adaptive;
mod contact;
mod dual;
mod material;
mod shell;
pub use contact::*;
pub use material::*;
pub use shell::*;

mod embedding;
pub use embedding::{SkinEmbedding, SurfaceBinding};

mod genital_folds;
pub use genital_folds::{
    LabiaMinoraGaitSimulation, LabiaMinoraGeometry, LabiaTubeInsertion, OrificeDilationKernel,
    OrificePointResponse, PrepuceGeometry, TissueFold,
};

/// Computes the post-buckling micro-wrinkle crest amplitude (meters) perpendicular to
/// Langer cleavage lines when compressive strain $\varepsilon_c$ exceeds the critical
/// epidermal buckling threshold $\varepsilon_{\text{crit}} > 0$:
/// $A = A_{\max} \tanh\!\left(\sqrt{\max(\varepsilon_c / \varepsilon_{\text{crit}} - 1, 0)}\right)$.
///
/// # Errors
///
/// Rejects non-finite inputs, non-positive `critical_strain`, or negative `max_amplitude_m`.
pub fn langer_line_wrinkle_amplitude(
    compressive_strain: f64,
    critical_strain: f64,
    max_amplitude_m: f64,
) -> Result<f64, &'static str> {
    if !compressive_strain.is_finite()
        || !critical_strain.is_finite()
        || !max_amplitude_m.is_finite()
        || critical_strain <= 0.0
        || max_amplitude_m < 0.0
    {
        return Err("invalid Langer wrinkle buckling parameters");
    }
    if compressive_strain <= critical_strain || max_amplitude_m == 0.0 {
        return Ok(0.0);
    }
    let excess = (compressive_strain / critical_strain - 1.0).max(0.0).sqrt();
    Ok(max_amplitude_m * excess.tanh())
}

/// Performs antipodality-aware normalized Dual Quaternion Linear Blending (DLB)
/// of two unit dual quaternions `dq0` and `dq1` ($[q_r, q_d]$ in $[x, y, z, w]$ order)
/// with weights `w0`, `w1`, enforcing Plücker orthogonality $\langle q_r, q_d \rangle = 0$.
#[must_use]
pub fn dual_quaternion_screw_blend_norm(
    dq0: [[f64; 4]; 2],
    dq1: [[f64; 4]; 2],
    w0: f64,
    w1: f64,
) -> Option<[[f64; 4]; 2]> {
    if !w0.is_finite()
        || !w1.is_finite()
        || w0 < 0.0
        || w1 < 0.0
        || (w0 + w1) <= 1e-12
        || dq0.iter().flatten().any(|x| !x.is_finite())
        || dq1.iter().flatten().any(|x| !x.is_finite())
    {
        return None;
    }
    let dot_r: f64 = (0..4).map(|i| dq0[0][i] * dq1[0][i]).sum();
    let sign = if dot_r < 0.0 { -1.0 } else { 1.0 };
    let mut qr = [0.0; 4];
    let mut qd = [0.0; 4];
    for i in 0..4 {
        qr[i] = w0 * dq0[0][i] + w1 * sign * dq1[0][i];
        qd[i] = w0 * dq0[1][i] + w1 * sign * dq1[1][i];
    }
    let norm = qr.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm <= 1e-12 || !norm.is_finite() {
        return None;
    }
    for i in 0..4 {
        qr[i] /= norm;
        qd[i] /= norm;
    }
    let plucker: f64 = (0..4).map(|i| qr[i] * qd[i]).sum();
    for i in 0..4 {
        qd[i] -= plucker * qr[i];
    }
    Some([qr, qd])
}

#[cfg(test)]
mod wrinkle_tests {
    use super::*;

    #[test]
    fn langer_wrinkle_is_zero_below_buckling_threshold_and_grows_above() {
        assert_eq!(langer_line_wrinkle_amplitude(0.02, 0.05, 0.001).unwrap(), 0.0);
        let wrinkled = langer_line_wrinkle_amplitude(0.15, 0.05, 0.001).unwrap();
        assert!(wrinkled > 0.0 && wrinkled < 0.001);
        assert!(langer_line_wrinkle_amplitude(0.1, 0.0, 0.001).is_err());
    }

    #[test]
    fn dual_quaternion_blend_preserves_unit_norm_and_antipodal_hemisphere() {
        let dq0 = [[0.0, 0.0, 0.0, 1.0], [0.0, 0.5, 0.0, 0.0]];
        let dq1 = [[0.0, 0.0, 0.0, -1.0], [0.0, -0.5, 0.0, 0.0]];
        let blended = dual_quaternion_screw_blend_norm(dq0, dq1, 0.5, 0.5).unwrap();
        let norm_r: f64 = blended[0].iter().map(|x| x * x).sum::<f64>().sqrt();
        let plucker: f64 = (0..4).map(|i| blended[0][i] * blended[1][i]).sum();
        assert!((norm_r - 1.0).abs() < 1e-12);
        assert!(plucker.abs() < 1e-12);
    }
}
