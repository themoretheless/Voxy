//! Isochoric Ogden equilibrium elasticity and objective reference-strain Maxwell branches.
//! This is an explicit constitutive choice, not a reproduction of a calibrated brain dataset.
use super::{Body, IDENTITY, Matrix, Response, columns, det, dot, inverse, mm, sub, transpose};
#[derive(Clone, Copy, Debug)]
pub struct OgdenTerm {
    /// Small-strain shear modulus of this term, Pa.
    pub shear_pa: f64,
    /// Nonzero dimensionless exponent; negative exponents are permitted.
    pub exponent: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct MaxwellBranch {
    pub shear_pa: f64,
    pub relaxation_seconds: f64,
}
#[derive(Clone, Debug)]
pub struct ViscoelasticOgden {
    terms: Vec<OgdenTerm>,
    branches: Vec<MaxwellBranch>,
    bulk_pa: f64,
    memory: Vec<Matrix>,
    pub(super) trial_seconds: f64,
}
const ZERO: Matrix = [[0.; 3]; 3];
fn deviator(mut m: Matrix) -> Matrix {
    let mean = (m[0][0] + m[1][1] + m[2][2]) / 3.;
    for (i, row) in m.iter_mut().enumerate() {
        row[i] -= mean;
    }
    m
}
fn strain(f: Matrix) -> Result<(Matrix, Matrix, f64), &'static str> {
    let j = det(f);
    if !j.is_finite() || j <= 0. || f.iter().flatten().any(|v| !v.is_finite()) {
        return Err("invalid viscoelastic deformation");
    }
    let q = j.powf(-2. / 3.);
    let c = mm(transpose(f), f).map(|r| r.map(|v| q * v));
    let e = deviator(c.map(|r| r.map(|v| 0.5 * v)));
    if c.iter().flatten().any(|v| !v.is_finite()) {
        return Err("viscoelastic metric overflow");
    }
    Ok((c, e, j))
}
// Symmetric eigendecomposition. The complete tensor power is independent of the
// basis chosen in repeated eigenspaces. No eigenvector derivatives are needed.
fn spectral(c: Matrix) -> Result<([f64; 3], Matrix), &'static str> {
    let scale = c.iter().flatten().fold(0_f64, |a, b| a.max(b.abs()));
    if scale <= 0. || !scale.is_finite() {
        return Err("invalid Ogden metric");
    }
    let mut a = c.map(|r| r.map(|v| v / scale));
    let mut vectors = IDENTITY;
    for _ in 0..32 {
        let mut largest = 0_f64;
        for (i, k) in [(0, 1), (0, 2), (1, 2)] {
            largest = largest.max(a[i][k].abs());
            if a[i][k].abs() < 1e-16 {
                continue;
            }
            let angle = 0.5 * (2. * a[i][k]).atan2(a[k][k] - a[i][i]);
            let (s, c) = angle.sin_cos();
            let (ii, kk, ik) = (a[i][i], a[k][k], a[i][k]);
            a[i][i] = c * c * ii - 2. * s * c * ik + s * s * kk;
            a[k][k] = s * s * ii + 2. * s * c * ik + c * c * kk;
            a[i][k] = 0.;
            a[k][i] = 0.;
            let other = 3 - i - k;
            let (oi, ok) = (a[other][i], a[other][k]);
            a[other][i] = c * oi - s * ok;
            a[i][other] = a[other][i];
            a[other][k] = s * oi + c * ok;
            a[k][other] = a[other][k];
            for row in &mut vectors {
                let (vi, vk) = (row[i], row[k]);
                row[i] = c * vi - s * vk;
                row[k] = s * vi + c * vk;
            }
        }
        if largest < 1e-14 {
            break;
        }
    }
    let values = [a[0][0] * scale, a[1][1] * scale, a[2][2] * scale];
    if values.iter().any(|v| !v.is_finite() || *v <= 0.) {
        return Err("non-positive Ogden metric");
    }
    let reconstructed: Matrix = std::array::from_fn(|i| {
        std::array::from_fn(|k| {
            (0..3)
                .map(|n| vectors[i][n] * values[n] * vectors[k][n])
                .sum()
        })
    });
    if reconstructed
        .iter()
        .flatten()
        .zip(c.iter().flatten())
        .any(|(a, b)| (a - b).abs() > 1e-10 * scale)
    {
        return Err("Ogden eigensolver failed");
    }
    Ok((values, vectors))
}
fn retention(tau: f64, dt: f64) -> f64 {
    if dt <= tau {
        1. / (1. + dt / tau)
    } else {
        let ratio = tau / dt;
        ratio / (1. + ratio)
    }
}
fn exponential_remainder(z: f64) -> f64 {
    if z.abs() < 1e-3 {
        z * z * (0.5 + z * (1. / 6. + z * (1. / 24. + z * (1. / 120. + z / 720.))))
    } else {
        z.exp_m1() - z
    }
}
impl ViscoelasticOgden {
    /// Zero-history material in its stress-free reference configuration.
    /// # Errors
    /// Rejects nonpositive/nonfinite moduli, times and numerically singular exponents.
    pub fn new(
        terms: Vec<OgdenTerm>,
        bulk_pa: f64,
        branches: Vec<MaxwellBranch>,
    ) -> Result<Self, &'static str> {
        if terms.is_empty()
            || terms.len() > 16
            || branches.len() > 16
            || !bulk_pa.is_finite()
            || bulk_pa <= 0.
            || terms.iter().any(|t| {
                !t.shear_pa.is_finite()
                    || t.shear_pa <= 0.
                    || !t.exponent.is_finite()
                    || t.exponent.abs() < 1e-6
            })
            || branches.iter().any(|b| {
                !b.shear_pa.is_finite()
                    || b.shear_pa <= 0.
                    || !b.relaxation_seconds.is_finite()
                    || b.relaxation_seconds <= 0.
            })
        {
            return Err("invalid Ogden-Maxwell parameters");
        }
        let stiffness = bulk_pa
            + terms.iter().map(|t| t.shear_pa).sum::<f64>()
            + branches.iter().map(|b| b.shear_pa).sum::<f64>();
        if !stiffness.is_finite() {
            return Err("Ogden-Maxwell modulus overflow");
        }
        let material = Self {
            memory: vec![ZERO; branches.len()],
            terms,
            branches,
            bulk_pa,
            trial_seconds: 0.,
        };
        material.response(IDENTITY, 0.)?;
        Ok(material)
    }
    pub(super) fn stiffness(&self) -> f64 {
        self.bulk_pa
            + self.terms.iter().map(|t| t.shear_pa).sum::<f64>()
            + self.branches.iter().map(|b| b.shear_pa).sum::<f64>()
    }
    /// Incremental potential and its analytic derivative with old memory fixed.
    /// dt=0 returns instantaneous stored-energy response; dt>0 eliminates the
    /// implicit Maxwell update, including its dissipation potential.
    /// # Errors
    /// Rejects invalid time, inversion, spectral failure and constitutive overflow.
    pub fn response(&self, f: Matrix, dt: f64) -> Result<Response, &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid constitutive timestep");
        }
        let (c, e, j) = strain(f)?;
        // The exponent-two Ogden term is exactly isochoric neo-Hooke.
        // Reuse its stable invariant instead of decomposing a symmetric tensor
        // for every force/energy evaluation. Other exponents keep the spectral law.
        let eigensystem = if self.terms.iter().any(|term| term.exponent != 2.) {
            Some(spectral(c)?)
        } else {
            None
        };
        let inv_t = transpose(inverse(f)?);
        let mut energy = 0.5 * self.bulk_pa * (j - 1.).powi(2);
        let mut p = inv_t.map(|r| r.map(|v| self.bulk_pa * (j - 1.) * j * v));
        let q = j.powf(-2. / 3.);
        let trace = c[0][0] + c[1][1] + c[2][2];
        for term in &self.terms {
            if term.exponent == 2. {
                energy += 0.5 * term.shear_pa * super::invariants::isochoric_excess(f, q, trace);
                for i in 0..3 {
                    for k in 0..3 {
                        p[i][k] += term.shear_pa * (q * f[i][k] - trace / 3. * inv_t[i][k]);
                    }
                }
                continue;
            }
            let (eigen, vectors) = eigensystem.ok_or("missing Ogden spectrum")?;
            let logs = eigen.map(|v| 0.5 * v.ln());
            let mean_log = logs.iter().sum::<f64>() / 3.;
            let powers = eigen.map(|v| v.powf(term.exponent * 0.5));
            let sum = powers.iter().sum::<f64>();
            // The normalized principal logs sum to zero. Summing exp(z)-1-z
            // retains O(strain²) energy near rest without O(1) cancellation.
            let excess = logs
                .iter()
                .map(|v| exponential_remainder(term.exponent * (v - mean_log)))
                .sum::<f64>();
            energy += 2. * term.shear_pa / (term.exponent * term.exponent) * excess;
            let power: Matrix = std::array::from_fn(|i| {
                std::array::from_fn(|k| {
                    (0..3)
                        .map(|n| vectors[i][n] * powers[n] / eigen[n] * vectors[k][n])
                        .sum()
                })
            });
            let product = mm(f, power);
            for i in 0..3 {
                for k in 0..3 {
                    p[i][k] += 2. * term.shear_pa / term.exponent
                        * (q * product[i][k] - sum / 3. * inv_t[i][k]);
                }
            }
        }
        for (branch, memory) in self.branches.iter().zip(&self.memory) {
            let weight = retention(branch.relaxation_seconds, dt);
            let h: Matrix = std::array::from_fn(|i| {
                std::array::from_fn(|k| branch.shear_pa * weight * (e[i][k] - memory[i][k]))
            });
            let contraction: f64 = h
                .iter()
                .flatten()
                .zip(c.iter().flatten())
                .map(|(a, b)| a * b)
                .sum();
            energy += branch.shear_pa
                * weight
                * e.iter()
                    .flatten()
                    .zip(memory.iter().flatten())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>();
            let product = mm(f, h);
            for i in 0..3 {
                for k in 0..3 {
                    p[i][k] += 2. * q * product[i][k] - 2. / 3. * contraction * inv_t[i][k];
                }
            }
        }
        if !energy.is_finite() || p.iter().flatten().any(|v| !v.is_finite()) {
            return Err("Ogden-Maxwell overflow");
        }
        Ok(Response {
            energy_density: energy,
            first_piola: p,
            volume_ratio: j,
        })
    }
    /// Commit memory at prescribed accepted deformation. Returns nonnegative
    /// backward-Euler viscous dissipation per reference volume (J/m³).
    /// # Errors
    /// Invalid input leaves history unchanged.
    pub fn advance(&mut self, f: Matrix, dt: f64) -> Result<f64, &'static str> {
        if dt <= 0. || !dt.is_finite() {
            return Err("invalid relaxation timestep");
        }
        self.response(f, dt)?;
        let (_, e, _) = strain(f)?;
        let mut next = self.memory.clone();
        let mut dissipation = 0.;
        for ((branch, old), new) in self.branches.iter().zip(&self.memory).zip(&mut next) {
            let weight = retention(branch.relaxation_seconds, dt);
            for i in 0..3 {
                for k in 0..3 {
                    new[i][k] = weight * old[i][k] + (1. - weight) * e[i][k];
                    dissipation += 2.
                        * branch.shear_pa
                        * (branch.relaxation_seconds / dt)
                        * (new[i][k] - old[i][k]).powi(2);
                }
            }
        }
        if !dissipation.is_finite() || next.iter().flatten().flatten().any(|v| !v.is_finite()) {
            return Err("relaxation history overflow");
        }
        self.memory = next;
        Ok(dissipation)
    }
    /// Stored Maxwell branch energy per reference volume (J/m³), excluding
    /// equilibrium/bulk elasticity. Evaluated independently of the heat receipt.
    /// # Errors
    /// Invalid deformation or overflowing branch energy.
    pub fn maxwell_energy_density(&self, f: Matrix) -> Result<f64, &'static str> {
        let (_, strain, _) = strain(f)?;
        let energy = self
            .branches
            .iter()
            .zip(&self.memory)
            .map(|(branch, memory)| {
                branch.shear_pa
                    * strain
                        .iter()
                        .flatten()
                        .zip(memory.iter().flatten())
                        .map(|(a, b)| (a - b).powi(2))
                        .sum::<f64>()
            })
            .sum::<f64>();
        if !energy.is_finite() {
            return Err("Maxwell stored energy overflow");
        }
        Ok(energy)
    }
    /// Exact exponential Maxwell relaxation at a held deformation.
    /// Returns nonnegative released branch energy per reference volume (J/m³).
    /// The receipt uses the actual committed memory change; a rounded-away
    /// history increment cannot generate nominal heat without stored-energy loss.
    /// Equilibrium elasticity and bulk energy do not relax.
    /// # Errors
    /// Rejects invalid time/deformation or overflow without changing history.
    pub fn relax_exact(&mut self, f: Matrix, dt: f64) -> Result<f64, &'static str> {
        if !dt.is_finite() || dt <= 0. {
            return Err("invalid exact relaxation timestep");
        }
        self.response(f, 0.)?;
        let (_, e, _) = strain(f)?;
        let mut next = self.memory.clone();
        let mut released = 0.;
        for ((branch, old), new) in self.branches.iter().zip(&self.memory).zip(&mut next) {
            let fraction = -(-dt / branch.relaxation_seconds).exp_m1();
            for i in 0..3 {
                for k in 0..3 {
                    let target = e[i][k];
                    new[i][k] = (old[i][k] + fraction * (target - old[i][k]))
                        .clamp(old[i][k].min(target), old[i][k].max(target));
                    let before = target - old[i][k];
                    let after = target - new[i][k];
                    released += branch.shear_pa * (before - after) * (before + after);
                }
            }
        }
        if !released.is_finite()
            || released < 0.
            || next.iter().flatten().flatten().any(|v| !v.is_finite())
        {
            return Err("exact relaxation history overflow");
        }
        self.memory = next;
        Ok(released)
    }
}
impl Body {
    /// Assign time-dependent isotropic tissue mechanics to an element. Assignment
    /// resets that element's viscous history and replaces its cardiac law.
    /// # Errors
    /// Rejects invalid index, active muscle input or preconditioner overflow.
    pub fn set_viscoelastic_ogden(
        &mut self,
        index: usize,
        law: ViscoelasticOgden,
    ) -> Result<(), &'static str> {
        self.set_viscoelastic_ogden_batch(&[(index, law)])
    }
    /// Assign a heterogeneous material profile atomically with one body clone.
    /// Rejects duplicate/invalid indices; replaces only the assigned laws and memories.
    pub fn set_viscoelastic_ogden_batch(
        &mut self,
        assignments: &[(usize, ViscoelasticOgden)],
    ) -> Result<(), &'static str> {
        let mut seen = std::collections::BTreeSet::new();
        if assignments.iter().any(|(i, _)| {
            *i >= self.elements.len() || self.elements[*i].activation != 0. || !seen.insert(*i)
        }) {
            return Err("invalid viscoelastic assignment");
        }
        let mut trial = self.clone();
        for (index, law) in assignments {
            trial.elements[*index].myocardium = None;
            trial.elements[*index].viscoelastic = Some(law.clone());
            trial.elements[*index].viscoelastic_hgo = None;
        }
        trial.diagonal.fill(0.);
        for e in &trial.elements {
            let stiffness = if let Some(v) = &e.viscoelastic_hgo {
                v.stiffness()
            } else if let Some(v) = &e.viscoelastic {
                v.stiffness()
            } else if let Some(m) = e.myocardium {
                m.bulk_pa
                    + m.matrix.scale_pa
                    + 4. * (m.fiber.scale_pa + m.sheet.scale_pa + m.fiber_sheet.scale_pa)
                    + m.active_tension_pa
            } else {
                e.material.bulk_pa
                    + e.material.shear_pa
                    + e.material
                        .fibers
                        .iter()
                        .map(|f| 4. * f.stiffness_pa + f.active_pa)
                        .sum::<f64>()
            };
            for k in 0..4 {
                trial.diagonal[e.nodes[k]] +=
                    e.volume * stiffness * dot(e.gradients[k], e.gradients[k]);
            }
        }
        if trial.diagonal.iter().any(|v| !v.is_finite()) {
            return Err("viscoelastic preconditioner overflow");
        }
        *self = trial;
        Ok(())
    }
    /// Real-time relaxation increment with quasistatic equilibrium (no inertia).
    /// History is frozen throughout the nonlinear solve and committed only once
    /// after convergence. Failure leaves geometry and every branch unchanged.
    /// # Errors
    /// Rejects invalid dt, nonconvergence or constitutive failure.
    pub fn relax_step(
        &mut self,
        dt: f64,
        iterations: usize,
        tolerance_n: f64,
    ) -> Result<super::Equilibrium, &'static str> {
        if !dt.is_finite() || dt <= 0. {
            return Err("invalid tissue timestep");
        }
        self.relax_step_with(dt, |trial| trial.equilibrate(iterations, tolerance_n))
    }
    /// Transactional quasistatic physical timestep solved by L-BFGS. Branch
    /// memories stay frozen through every trial; advance once after convergence.
    pub fn relax_step_lbfgs_states(
        &mut self,
        dt: f64,
        iterations: usize,
        tolerance_n: f64,
        observe: impl FnMut(usize, f64, f64, f64, usize, f64, &[super::Vec3]),
    ) -> Result<super::Equilibrium, &'static str> {
        self.relax_step_with(dt, |trial| {
            trial.equilibrate_lbfgs_states(iterations, tolerance_n, observe)
        })
    }
    fn relax_step_with(
        &mut self,
        dt: f64,
        solve: impl FnOnce(&mut Body) -> Result<super::Equilibrium, &'static str>,
    ) -> Result<super::Equilibrium, &'static str> {
        if !dt.is_finite() || dt <= 0. {
            return Err("invalid tissue timestep");
        }
        let mut trial = self.clone();
        for e in &mut trial.elements {
            if let Some(v) = &mut e.viscoelastic {
                v.trial_seconds = dt;
            }
            if let Some(v) = &mut e.viscoelastic_hgo {
                v.trial_seconds = dt;
            }
        }
        let report = solve(&mut trial)?;
        if !report.converged {
            return Err("viscoelastic equilibrium did not converge");
        }
        for e in &mut trial.elements {
            if let Some(v) = &mut e.viscoelastic_hgo {
                let [a, b, c, d] = e.nodes.map(|i| trial.positions[i]);
                let f = mm(columns(sub(b, a), sub(c, a), sub(d, a)), e.inv_rest);
                v.advance(f, dt)?;
                v.trial_seconds = 0.;
            }
            if let Some(v) = &mut e.viscoelastic {
                let [a, b, c, d] = e.nodes.map(|i| trial.positions[i]);
                let f = mm(columns(sub(b, a), sub(c, a), sub(d, a)), e.inv_rest);
                v.advance(f, dt)?;
                v.trial_seconds = 0.;
            }
        }
        *self = trial;
        Ok(report)
    }
}
