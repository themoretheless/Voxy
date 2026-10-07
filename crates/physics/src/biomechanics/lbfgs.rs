//! Limited-memory energy minimization for stiff, nonlinear elastic/contact states.
use super::{Body, Equilibrium, Vec3, add, columns, det, dot, mm, scale, sub};
#[inline]
fn inner(a: &[Vec3], b: &[Vec3]) -> f64 {
    let mut sum = 0.0;
    for (va, vb) in a.iter().zip(b) {
        sum += va[0] * vb[0] + va[1] * vb[1] + va[2] * vb[2];
    }
    sum
}
// Shared secant algebra; owners keep their own objective, constraints and admission.
pub(super) type SecantPair = (Vec<Vec3>, Vec<Vec3>, f64);
/// Rayleigh scaling of the initial positive inverse, using the latest secant.
/// The owner supplies its inverse quadratic form without losing coupling.
pub(super) fn secant_scale(
    history: &[SecantPair],
    inverse_quadratic: impl FnOnce(&[Vec3]) -> f64,
) -> f64 {
    try_secant_scale::<std::convert::Infallible>(history, |q| Ok(inverse_quadratic(q)))
        .unwrap_or_else(|never| match never {})
}
pub(super) fn try_secant_scale<E>(
    history: &[SecantPair],
    inverse_quadratic: impl FnOnce(&[Vec3]) -> Result<f64, E>,
) -> Result<f64, E> {
    let Some((s, y, _)) = history.last() else {
        return Ok(1.);
    };
    let denominator = inverse_quadratic(y)?;
    let numerator = inner(s, y);
    if denominator.is_finite() && denominator > 0. && numerator.is_finite() && numerator > 0. {
        let ratio = numerator / denominator;
        if ratio.is_finite() && ratio > 0. {
            return Ok(ratio.clamp(1e-8, 1e8));
        }
    }
    Ok(1.)
}
pub(super) fn secant_direction(
    history: &[SecantPair],
    gradient: &[Vec3],
    inverse: impl FnOnce(&[Vec3]) -> Vec<Vec3>,
) -> Vec<Vec3> {
    try_secant_direction::<std::convert::Infallible>(history, gradient, |q| Ok(inverse(q)))
        .unwrap_or_else(|never| match never {})
}
pub(super) fn try_secant_direction<E>(
    history: &[SecantPair],
    gradient: &[Vec3],
    inverse: impl FnOnce(&[Vec3]) -> Result<Vec<Vec3>, E>,
) -> Result<Vec<Vec3>, E> {
    let mut q = gradient.to_vec();
    
    // Preallocate alphas with exact capacity (no reallocations)
    let mut alphas: Vec<f64> = Vec::with_capacity(history.len());
    
    for (s, y, rho) in history.iter().rev() {
        let alpha = rho * inner(s, &q);
        alphas.push(alpha);
        
        // In-place vector subtraction (daxpy) avoiding intermediate arrays
        for (qi, yi) in q.iter_mut().zip(y) {
            qi[0] -= yi[0] * alpha;
            qi[1] -= yi[1] * alpha;
            qi[2] -= yi[2] * alpha;
        }
    }
    
    let mut r = inverse(&q)?;
    
    // Reuse allocation for beta computation with in-place FMA
    for ((s, y, rho), alpha) in history.iter().zip(alphas.into_iter().rev()) {
        let beta = rho * inner(y, &r);
        let gamma = alpha - beta;
        for (ri, si) in r.iter_mut().zip(s) {
            ri[0] += si[0] * gamma;
            ri[1] += si[1] * gamma;
            ri[2] += si[2] * gamma;
        }
    }
    
    for ri in &mut r {
        ri[0] = -ri[0];
        ri[1] = -ri[1];
        ri[2] = -ri[2];
    }
    Ok(r)
}
pub(super) fn push_secant(
    history: &mut Vec<SecantPair>,
    s: Vec<Vec3>,
    y: Vec<Vec3>,
    capacity: usize,
) {
    let sy = inner(&s, &y);
    let reciprocal = 1. / sy;
    let correlation_scale = (inner(&s, &s) * inner(&y, &y)).sqrt();
    if sy.is_finite()
        && reciprocal.is_finite()
        && correlation_scale.is_finite()
        && sy > 1e-12 * correlation_scale
        && sy > 0.
        && capacity > 0
    {
        if history.len() == capacity {
            history.remove(0);
        }
        history.push((s, y, reciprocal));
    }
}
fn free_gradient(body: &Body, mut gradient: Vec<Vec3>) -> Vec<Vec3> {
    for (g, pinned) in gradient.iter_mut().zip(&body.pinned) {
        if *pinned {
            *g = [0.; 3];
        }
    }
    gradient
}
impl Body {
    /// Import a diagnostic configuration without rebasing reference geometry,
    /// material memory or loads. Validates energy and unchanged pinned nodes.
    /// This does not certify the path used to reach the imported configuration.
    pub fn restore_diagnostic_positions(&mut self, positions: &[Vec3]) -> Result<(), &'static str> {
        self.evaluate(positions)?;
        if self
            .pinned
            .iter()
            .enumerate()
            .any(|(i, p)| *p && positions[i] != self.positions[i])
        {
            return Err("diagnostic import moved a pinned node");
        }
        self.positions = positions.to_vec();
        Ok(())
    }
    /// Preconditioned L-BFGS with bounded memory and Armijo search. Iterations are
    /// numerical minimization, not elapsed physiological time. Supports stay fixed.
    /// # Errors
    /// Invalid controls, invalid initial state or nonconvergence leave state unchanged.
    pub fn equilibrate_lbfgs(
        &mut self,
        max_iterations: usize,
        tolerance_n: f64,
    ) -> Result<Equilibrium, &'static str> {
        let mut trial = self.clone();
        let report = trial.equilibrate_lbfgs_observed(max_iterations, tolerance_n, |_, _, _| {})?;
        if !report.converged {
            return Err("L-BFGS equilibrium did not converge");
        }
        self.positions = trial.positions;
        Ok(report)
    }

    /// Report initial and every accepted iterate: iteration, energy in joules,
    /// and free-node force residual in newtons. Observations do not commit
    /// intermediate geometry or change the minimization algorithm.
    /// Diagnostic API: commits the final iterate even when the returned report
    /// is unconverged. Use `equilibrate_lbfgs` for transactional equilibrium.
    pub fn equilibrate_lbfgs_observed(
        &mut self,
        max_iterations: usize,
        tolerance_n: f64,
        mut observe: impl FnMut(usize, f64, f64),
    ) -> Result<Equilibrium, &'static str> {
        self.equilibrate_lbfgs_steps(max_iterations, tolerance_n, |i, e, r, _, _, _| {
            observe(i, e, r)
        })
    }

    /// Observe accepted iteration, energy, residual, line-search multiplier,
    /// number of rejected trials and maximum accepted node displacement.
    /// Initial observation has zero step/displacement. No algorithm changes.
    pub fn equilibrate_lbfgs_steps(
        &mut self,
        max_iterations: usize,
        tolerance_n: f64,
        mut observe: impl FnMut(usize, f64, f64, f64, usize, f64),
    ) -> Result<Equilibrium, &'static str> {
        self.equilibrate_lbfgs_states(max_iterations, tolerance_n, |i, e, r, s, n, d, _| {
            observe(i, e, r, s, n, d)
        })
    }

    /// Observe the initial and accepted trial coordinates as read-only snapshots.
    /// Coordinates are valid solver iterates, not necessarily equilibria; the
    /// callback neither commits intermediate geometry nor advances tissue time.
    pub fn equilibrate_lbfgs_states(
        &mut self,
        max_iterations: usize,
        tolerance_n: f64,
        observe: impl FnMut(usize, f64, f64, f64, usize, f64, &[Vec3]),
    ) -> Result<Equilibrium, &'static str> {
        self.equilibrate_lbfgs_preconditioned_states(max_iterations, tolerance_n, false, observe)
    }

    /// Optional positive normal-contact curvature added to the material/pore
    /// diagonal at every accepted state. Changes numerical directions only;
    /// retains actual residual, energy, Armijo search and geometric path guard.
    /// Contact curvature requires the experimental primitive contact law.
    pub fn equilibrate_lbfgs_preconditioned_states(
        &mut self,
        max_iterations: usize,
        tolerance_n: f64,
        contact_curvature: bool,
        mut observe: impl FnMut(usize, f64, f64, f64, usize, f64, &[Vec3]),
    ) -> Result<Equilibrium, &'static str> {
        if contact_curvature
            && self.surface_contact_law != super::SurfaceContactLaw::ExperimentalPrimitiveSum
        {
            return Err("contact curvature requires primitive law");
        }
        if max_iterations == 0
            || max_iterations > 100000
            || !tolerance_n.is_finite()
            || tolerance_n <= 0.
        {
            return Err("invalid L-BFGS equilibrium options");
        }
        let base_diagonal = self.pore_preconditioner()?;
        let mut diagonal = base_diagonal.clone();
        let mut x = self.positions.clone();
        let (mut energy, g) = self.evaluate(&x)?;
        if contact_curvature {
            for (d, c) in diagonal.iter_mut().zip(self.primitive_curvature(&x)?) {
                *d += c;
            }
            if diagonal.iter().any(|d| !d.is_finite() || *d <= 0.) {
                return Err("contact preconditioner overflow");
            }
        }
        let mut gradient = free_gradient(self, g);
        let mut history: Vec<(Vec<Vec3>, Vec<Vec3>, f64)> = Vec::new();
        let mut iterations = 0;
        observe(0, energy, inner(&gradient, &gradient).sqrt(), 0., 0, 0., &x);
        let mut radius_squared = 0_f64;
        for p in &self.rest {
            radius_squared = radius_squared.max(dot(*p, *p));
        }
        let tiny_squared = radius_squared.max(1e-16) * f64::EPSILON;
        while iterations < max_iterations {
            let squared = inner(&gradient, &gradient);
            if !squared.is_finite() {
                return Err("L-BFGS residual overflow");
            }
            if squared.sqrt() <= tolerance_n {
                break;
            }
            let gamma = secant_scale(&history, |y| {
                y.iter()
                    .enumerate()
                    .filter(|(i, _)| !self.pinned[*i])
                    .map(|(i, y)| dot(*y, *y) / diagonal[i])
                    .sum()
            });
            let mut direction = secant_direction(&history, &gradient, |q| {
                q.iter()
                    .enumerate()
                    .map(|(i, q)| {
                        if self.pinned[i] {
                            [0.; 3]
                        } else {
                            scale(*q, gamma / diagonal[i])
                        }
                    })
                    .collect()
            });
            let mut slope = inner(&gradient, &direction);
            if !slope.is_finite() || slope >= 0. {
                history.clear();
                direction = gradient
                    .iter()
                    .enumerate()
                    .map(|(i, g)| {
                        if self.pinned[i] {
                            [0.; 3]
                        } else {
                            scale(*g, -1. / diagonal[i])
                        }
                    })
                    .collect();
                slope = inner(&gradient, &direction);
            }
            let mut accepted = None;
            let mut step = 1.;
            let mut rejected_trials = 0;
            for _ in 0..60 {
                let trial: Vec<_> = x
                    .iter()
                    .zip(&direction)
                    .map(|(x, d)| add(*x, scale(*d, step)))
                    .collect();
                if self.gap_path_is_open(&x, &trial) {
                    if let Ok((e, g)) = self.evaluate(&trial) {
                        let g = free_gradient(self, g);
                        let tiny = direction
                            .iter()
                            .all(|d| step * step * dot(*d, *d) <= tiny_squared);
                        let roundoff = 64. * f64::EPSILON * energy.abs().max(1e-12);
                        if e <= energy + 1e-4 * step * slope
                            || (tiny
                                && (e - energy).abs() <= roundoff
                                && inner(&g, &g) < squared * (1. - 1e-4))
                        {
                            accepted = Some((trial, e, g));
                            break;
                        }
                    }
                }
                step *= 0.5;
                rejected_trials += 1;
            }
            let Some((next, e, next_gradient)) = accepted else {
                break;
            };
            let s: Vec<_> = next.iter().zip(&x).map(|(a, b)| sub(*a, *b)).collect();
            let max_displacement = s.iter().map(|s| dot(*s, *s).sqrt()).fold(0., f64::max);
            let y: Vec<_> = next_gradient
                .iter()
                .zip(&gradient)
                .map(|(a, b)| sub(*a, *b))
                .collect();
            push_secant(&mut history, s, y, 12);
            x = next;
            energy = e;
            gradient = next_gradient;
            if contact_curvature {
                diagonal.clone_from(&base_diagonal);
                for (d, c) in diagonal.iter_mut().zip(self.primitive_curvature(&x)?) {
                    *d += c;
                }
                if diagonal.iter().any(|d| !d.is_finite() || *d <= 0.) {
                    return Err("contact preconditioner overflow");
                }
            }
            iterations += 1;
            observe(
                iterations,
                energy,
                inner(&gradient, &gradient).sqrt(),
                step,
                rejected_trials,
                max_displacement,
                &x,
            );
        }
        let residual_n = inner(&gradient, &gradient).sqrt();
        let mut min_j = f64::INFINITY;
        let mut max_j = f64::NEG_INFINITY;
        for e in &self.elements {
            let [a, b, c, d] = e.nodes.map(|i| x[i]);
            let j = det(mm(columns(sub(b, a), sub(c, a), sub(d, a)), e.inv_rest));
            min_j = min_j.min(j);
            max_j = max_j.max(j);
        }
        self.positions = x;
        Ok(Equilibrium {
            iterations,
            residual_n,
            converged: residual_n <= tolerance_n,
            min_j,
            max_j,
        })
    }
}

#[cfg(test)]
mod secant_tests {
    use super::*;
    fn product(a: [[f64; 3]; 3], x: Vec3) -> Vec3 {
        a.map(|row| dot(row, x))
    }
    #[test]
    fn two_loop_matches_independent_dense_inverse_bfgs_updates() {
        let initial = [[2., 0.3, 0.], [0.3, 1., 0.2], [0., 0.2, 0.5]];
        let stiffness = [[400., 99., 3.], [99., 40., 2.], [3., 2., 7.]];
        let mut dense = initial;
        let mut history = Vec::new();
        for displacement in [
            [0.01, -0.02, 0.03],
            [-0.03, 0.01, 0.02],
            [0.02, 0.03, -0.01],
        ] {
            let y = product(stiffness, displacement);
            let rho = 1. / dot(displacement, y);
            let left: [[f64; 3]; 3] = std::array::from_fn(|i| {
                std::array::from_fn(|j| f64::from(i == j) - rho * displacement[i] * y[j])
            });
            let old = dense;
            dense = std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    let mut value = rho * displacement[i] * displacement[j];
                    for k in 0..3 {
                        for l in 0..3 {
                            value += left[i][k] * old[k][l] * left[j][l];
                        }
                    }
                    value
                })
            });
            push_secant(&mut history, vec![displacement], vec![y], 12);
        }
        assert_eq!(history.len(), 3);
        let gradient = [0.7, -0.4, 0.2];
        let actual = secant_direction(&history, &[gradient], |q| vec![product(initial, q[0])]);
        let expected = product(dense, gradient).map(|v| -v);
        for axis in 0..3 {
            assert!((actual[0][axis] - expected[axis]).abs() < 1e-13);
        }
        assert!(dot(gradient, actual[0]) < 0.);
    }
    #[test]
    fn secant_admission_rejects_bad_curvature_and_bounds_memory() {
        let mut history = Vec::new();
        for y in [[-1., 0., 0.], [0.; 3], [f64::NAN, 0., 0.], [1e-320, 0., 0.]] {
            push_secant(&mut history, vec![[1., 0., 0.]], vec![y], 2);
            assert!(history.is_empty());
        }
        for value in [1., 2., 3.] {
            push_secant(
                &mut history,
                vec![[value, 0., 0.]],
                vec![[value * 2., 0., 0.]],
                2,
            );
        }
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].0[0][0], 2.);
        assert_eq!(history[1].0[0][0], 3.);
    }
    #[test]
    fn secant_scale_rejects_invalid_quadratic_forms_and_bounds_positive_ratios() {
        assert_eq!(
            secant_scale(&[], |_| panic!("empty history must not query metric")),
            1.
        );
        let history = vec![(vec![[1., 0., 0.]], vec![[2., 0., 0.]], 0.5)];
        for invalid in [0., -1., f64::NAN, f64::INFINITY] {
            assert_eq!(secant_scale(&history, |_| invalid), 1.);
        }
        assert_eq!(secant_scale(&history, |_| 4.), 0.5);
        assert_eq!(secant_scale(&history, |_| 1e-20), 1e8);
        assert_eq!(secant_scale(&history, |_| 1e20), 1e-8);
    }
}

#[cfg(test)]
mod backend_failure_tests {
    use super::*;
    #[test]
    fn secant_backend_errors_preserve_history_and_successful_arithmetic() {
        let mut history = Vec::new();
        push_secant(&mut history, vec![[1., 2., 3.]], vec![[2., 3., 5.]], 4);
        let before = format!("{history:?}");
        assert_eq!(
            try_secant_scale(&history, |_| Err("CUDA metric failure")),
            Err("CUDA metric failure")
        );
        assert_eq!(
            try_secant_direction(&history, &[[3., 4., 5.]], |_| Err("CUDA inverse failure")),
            Err("CUDA inverse failure")
        );
        assert_eq!(format!("{history:?}"), before);
        let native = secant_direction(&history, &[[3., 4., 5.]], |r| r.to_vec());
        let checked =
            try_secant_direction::<&str>(&history, &[[3., 4., 5.]], |r| Ok(r.to_vec())).unwrap();
        assert_eq!(
            native
                .iter()
                .flatten()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            checked
                .iter()
                .flatten()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            secant_scale(&history, |r| inner(r, r)),
            try_secant_scale::<&str>(&history, |r| Ok(inner(r, r))).unwrap()
        );
    }
}
