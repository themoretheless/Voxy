//! Positive conservative sparse transport iteration shared across physics owners.
pub(crate) fn solve_mass(
    rhs_mass: &[f64],
    diagonal: &[f64],
    incoming: &[Vec<(usize, f64)>],
    external_out: &[f64],
) -> Result<Vec<f64>, &'static str> {
    let scale = rhs_mass.iter().copied().fold(0_f64, f64::max);
    if scale == 0. {
        return Ok(vec![0.; rhs_mass.len()]);
    }
    let rhs: Vec<_> = rhs_mass.iter().map(|m| m / scale).collect();
    let total: f64 = rhs.iter().sum();
    let mut mass = rhs.clone();
    // Positive Gauss-Seidel iteration for the conservative transport M-matrix.
    // No clipping or post-hoc mass redistribution is used.
    for _ in 0..20_000 {
        for i in 0..mass.len() {
            mass[i] =
                (rhs[i] + incoming[i].iter().map(|(j, c)| c * mass[*j]).sum::<f64>()) / diagonal[i];
        }
        let residual: f64 = (0..mass.len())
            .map(|i| {
                (diagonal[i] * mass[i]
                    - incoming[i].iter().map(|(j, c)| c * mass[*j]).sum::<f64>()
                    - rhs[i])
                    .abs()
            })
            .sum();
        let drift = (mass
            .iter()
            .zip(external_out)
            .map(|(m, c)| m * (1. + c))
            .sum::<f64>()
            - total)
            .abs();
        if !residual.is_finite() || mass.iter().any(|m| !m.is_finite() || *m < 0.) {
            return Err("implicit positive transport iteration overflow");
        }
        if residual <= 1e-14 * total && drift <= 1e-14 * total {
            let output: Vec<_> = mass.into_iter().map(|m| m * scale).collect();
            if output.iter().any(|m| !m.is_finite()) {
                return Err("implicit positive transport inventory overflow");
            }
            return Ok(output);
        }
    }
    Err("implicit positive transport did not converge")
}
