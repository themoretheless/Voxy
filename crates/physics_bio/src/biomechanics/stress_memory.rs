//! Exponential stress-memory convolution for prescribed reference stress.
//! Not a complete HGO law or an experimental tissue calibration.
use super::Matrix;

/// One branch satisfying Q' + Q/tau = beta*S'. All tensors use one fixed
/// reference stress measure. Spatial stress tensors require separate transport.
#[derive(Clone, Debug)]
pub struct ReferenceStressMemory {
    tau_seconds: f64,
    beta: f64,
    previous_stress: Matrix,
    memory: Matrix,
}
/// Fixed-history potential in the right Cauchy-Green tensor C. The caller
/// supplies a consistent passive elastic energy and S=2*dPsi/dC.
#[derive(Clone, Debug)]
pub struct FrozenStressPotential {
    elastic_multiplier: f64,
    history: Matrix,
}
impl FrozenStressPotential {
    /// Returns incremental energy density and total second Piola stress.
    /// Does not prove that the supplied elastic stress differentiates its energy.
    pub fn response(
        &self,
        c: Matrix,
        elastic_energy: f64,
        elastic_second_piola: Matrix,
    ) -> Result<(f64, Matrix), &'static str> {
        if !finite(c)
            || !finite(elastic_second_piola)
            || !elastic_energy.is_finite()
            || !(0..3).all(|i| {
                (0..3).all(|j| {
                    c[i][j] == c[j][i] && elastic_second_piola[i][j] == elastic_second_piola[j][i]
                })
            })
        {
            return Err("invalid symmetric reference potential input");
        }
        let determinant = c[0][0] * (c[1][1] * c[2][2] - c[1][2] * c[2][1])
            - c[0][1] * (c[1][0] * c[2][2] - c[1][2] * c[2][0])
            + c[0][2] * (c[1][0] * c[2][1] - c[1][1] * c[2][0]);
        if c[0][0] <= 0.
            || c[0][0] * c[1][1] - c[0][1] * c[1][0] <= 0.
            || !determinant.is_finite()
            || determinant <= 0.
        {
            return Err("nonpositive reference metric");
        }
        let energy = self.elastic_multiplier * elastic_energy
            + 0.5
                * (0..3)
                    .flat_map(|i| (0..3).map(move |j| self.history[i][j] * c[i][j]))
                    .sum::<f64>();
        let stress = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                self.elastic_multiplier * elastic_second_piola[i][j] + self.history[i][j]
            })
        });
        if !energy.is_finite() || !finite(stress) {
            return Err("reference potential overflow");
        }
        Ok((energy, stress))
    }
}
fn finite(a: Matrix) -> bool {
    a.iter().flatten().all(|x| x.is_finite())
}
impl ReferenceStressMemory {
    /// Initial stress and memory are explicit: no assumed preconditioning.
    pub fn new(
        tau_seconds: f64,
        beta: f64,
        initial_stress: Matrix,
        initial_memory: Matrix,
    ) -> Result<Self, &'static str> {
        if !tau_seconds.is_finite()
            || tau_seconds <= 0.
            || !beta.is_finite()
            || beta < 0.
            || !finite(initial_stress)
            || !finite(initial_memory)
        {
            return Err("invalid reference stress-memory parameters");
        }
        Ok(Self {
            tau_seconds,
            beta,
            previous_stress: initial_stress,
            memory: initial_memory,
        })
    }
    #[must_use]
    pub fn memory(&self) -> Matrix {
        self.memory
    }
    /// Evaluate a solver candidate without committing constitutive history.
    /// Each trial must start from the same last committed reference stress.
    pub fn trial(&self, next_stress: Matrix, seconds: f64) -> Result<Matrix, &'static str> {
        let mut candidate = self.clone();
        candidate.advance(next_stress, seconds)
    }
    /// Freeze one branch for solver trials. Potential is valid for symmetric
    /// second Piola history, driven by the same supplied passive elastic law.
    pub fn frozen_potential(&self, seconds: f64) -> Result<FrozenStressPotential, &'static str> {
        if !seconds.is_finite()
            || seconds <= 0.
            || !(0..3).all(|i| {
                (0..3).all(|j| {
                    self.memory[i][j] == self.memory[j][i]
                        && self.previous_stress[i][j] == self.previous_stress[j][i]
                })
            })
        {
            return Err("invalid frozen second Piola history");
        }
        let z = seconds / self.tau_seconds;
        let decay = (-z).exp();
        let gain = if z == 0. { 1. } else { -(-z).exp_m1() / z };
        let elastic_multiplier = 1. + self.beta * gain;
        let history = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                decay * self.memory[i][j] - self.beta * gain * self.previous_stress[i][j]
            })
        });
        if !elastic_multiplier.is_finite() || !finite(history) {
            return Err("frozen history overflow");
        }
        Ok(FrozenStressPotential {
            elastic_multiplier,
            history,
        })
    }
    /// Exact for linearly varying elastic stress over this interval.
    /// Rejecting an invalid/overflowing update preserves both stored tensors.
    pub fn advance(&mut self, next_stress: Matrix, seconds: f64) -> Result<Matrix, &'static str> {
        if !seconds.is_finite() || seconds <= 0. || !finite(next_stress) {
            return Err("invalid reference stress-memory increment");
        }
        let z = seconds / self.tau_seconds;
        let decay = (-z).exp();
        let gain = if z == 0. { 1. } else { -(-z).exp_m1() / z };
        let candidate = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                decay * self.memory[i][j]
                    + self.beta * gain * (next_stress[i][j] - self.previous_stress[i][j])
            })
        });
        if !finite(candidate) {
            return Err("reference stress-memory overflow");
        }
        self.memory = candidate;
        self.previous_stress = next_stress;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ZERO: Matrix = [[0.; 3]; 3];
    #[test]
    fn frozen_potential_differentiates_to_trial_stress() {
        let prior = [[110., 13., -8.], [13., 95., 3.], [-8., 3., 108.]];
        let memory = [[21., -2., 1.], [-2., 17., 4.], [1., 4., 19.]];
        let branch = ReferenceStressMemory::new(31.75, 0.24, prior, memory).unwrap();
        let frozen = branch.frozen_potential(0.25).unwrap();
        let c: Matrix = [[1.3, 0.12, -0.03], [0.12, 0.9, 0.08], [-0.03, 0.08, 1.2]];
        let evaluate = |c: Matrix| {
            let energy = 25. * c.iter().flatten().map(|x| x * x).sum::<f64>();
            frozen
                .response(c, energy, c.map(|row| row.map(|x| 100. * x)))
                .unwrap()
        };
        let (_, stress) = evaluate(c);
        let elastic = c.map(|row| row.map(|x| 100. * x));
        let q = branch.trial(elastic, 0.25).unwrap();
        for i in 0..3 {
            for j in i..3 {
                assert!((stress[i][j] - elastic[i][j] - q[i][j]).abs() < 1e-12);
                let mut plus = c;
                let mut minus = c;
                let h = 1e-5;
                plus[i][j] += h;
                minus[i][j] -= h;
                if i != j {
                    plus[j][i] += h;
                    minus[j][i] -= h;
                }
                let derivative = (evaluate(plus).0 - evaluate(minus).0) / (2. * h);
                let expected = if i == j {
                    0.5 * stress[i][j]
                } else {
                    stress[i][j]
                };
                assert!((derivative - expected).abs() < 1e-7);
            }
        }
        assert_eq!(branch.memory(), memory);
        assert!(frozen.response(ZERO, 0., ZERO).is_err());
        let mut nonsymmetric = prior;
        nonsymmetric[0][1] += 1.;
        assert!(
            ReferenceStressMemory::new(1., 1., nonsymmetric, ZERO)
                .unwrap()
                .frozen_potential(1.)
                .is_err()
        );
    }
    #[test]
    fn exact_ramp_and_hold_under_refinement() {
        let tau = 31.75_f64;
        let beta = 0.24;
        let duration = 120.;
        let rate = 100.;
        let exact = beta * rate * tau * (-(-duration / tau).exp_m1());
        for n in [1, 12, 120, 1200] {
            let mut branch = ReferenceStressMemory::new(tau, beta, ZERO, ZERO).unwrap();
            for k in 1..=n {
                let mut stress = ZERO;
                stress[0][0] = rate * duration * f64::from(k) / f64::from(n);
                branch.advance(stress, duration / f64::from(n)).unwrap();
            }
            assert!((branch.memory()[0][0] - exact).abs() < 1e-9);
            let mut stress = ZERO;
            stress[0][0] = rate * duration;
            let after = branch.advance(stress, 900.).unwrap()[0][0];
            assert!((after - exact * (-900. / tau).exp()).abs() < 1e-20);
        }
    }
    #[test]
    fn invalid_update_preserves_future_history() {
        let mut branch = ReferenceStressMemory::new(1., 0.5, ZERO, ZERO).unwrap();
        let mut control = branch.clone();
        let mut bad = ZERO;
        bad[1][2] = f64::NAN;
        assert!(branch.advance(bad, 1.).is_err());
        assert!(branch.advance(ZERO, 0.).is_err());
        let mut stress = ZERO;
        stress[0][1] = 3.;
        stress[1][0] = 3.;
        assert_eq!(
            branch.advance(stress, 0.1).unwrap(),
            control.advance(stress, 0.1).unwrap()
        );
    }
    #[test]
    fn rejected_solver_trials_do_not_change_committed_history() {
        let mut branch = ReferenceStressMemory::new(2., 0.3, ZERO, ZERO).unwrap();
        let mut control = branch.clone();
        let mut rejected = ZERO;
        rejected[0][0] = 10000.;
        let trial = branch.trial(rejected, 0.2).unwrap();
        assert!(trial[0][0] > 0.);
        assert_eq!(branch.memory(), ZERO);
        assert_eq!(branch.trial(rejected, 0.2).unwrap(), trial);
        let mut accepted = ZERO;
        accepted[0][0] = 10.;
        let expected = branch.trial(accepted, 0.2).unwrap();
        assert_eq!(branch.advance(accepted, 0.2).unwrap(), expected);
        assert_eq!(branch.memory(), control.advance(accepted, 0.2).unwrap());
    }
    #[test]
    fn tiny_intervals_and_extreme_time_ratio_are_finite() {
        let mut branch = ReferenceStressMemory::new(31.75, 0.24, ZERO, ZERO).unwrap();
        let mut stress = ZERO;
        stress[2][2] = 1.;
        assert!((branch.advance(stress, 1e-12).unwrap()[2][2] - 0.24).abs() < 1e-13);
        let mut fast = ReferenceStressMemory::new(f64::MIN_POSITIVE, 1., ZERO, ZERO).unwrap();
        assert_eq!(fast.advance(stress, f64::MAX).unwrap(), ZERO);
    }
}
