//! Limited-memory energy minimization for stiff, nonlinear elastic/contact states.
use super::{Body, Equilibrium, Vec3, add, columns, det, dot, mm, scale, sub};
fn inner(a: &[Vec3], b: &[Vec3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| dot(*a, *b)).sum()
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
            let mut q = gradient.clone();
            let mut alphas = Vec::new();
            for (s, y, rho) in history.iter().rev() {
                let alpha = rho * inner(s, &q);
                alphas.push(alpha);
                for (q, y) in q.iter_mut().zip(y) {
                    *q = sub(*q, scale(*y, alpha));
                }
            }
            let mut gamma = 1.;
            if let Some((s, y, _)) = history.last() {
                let denominator: f64 = y
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !self.pinned[*i])
                    .map(|(i, y)| dot(*y, *y) / diagonal[i])
                    .sum();
                if denominator > 0. {
                    gamma = (inner(s, y) / denominator).clamp(1e-8, 1e8);
                }
            }
            let mut r: Vec<_> = q
                .iter()
                .enumerate()
                .map(|(i, q)| {
                    if self.pinned[i] {
                        [0.; 3]
                    } else {
                        scale(*q, gamma / diagonal[i])
                    }
                })
                .collect();
            for ((s, y, rho), alpha) in history.iter().zip(alphas.into_iter().rev()) {
                let beta = rho * inner(y, &r);
                for (r, s) in r.iter_mut().zip(s) {
                    *r = add(*r, scale(*s, alpha - beta));
                }
            }
            let mut direction: Vec<_> = r.iter().map(|r| scale(*r, -1.)).collect();
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
            let sy = inner(&s, &y);
            if sy.is_finite() && sy > 1e-12 * (inner(&s, &s) * inner(&y, &y)).sqrt() && sy > 0. {
                if history.len() == 12 {
                    history.remove(0);
                }
                history.push((s, y, 1. / sy));
            }
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
