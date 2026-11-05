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
}
