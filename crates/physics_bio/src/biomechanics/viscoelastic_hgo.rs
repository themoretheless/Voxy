//! Experimental HGO plus exponential memory conjugate to Cbar.
//! This explicit constitutive choice is not a reproduction of a published UMAT.
use super::{
    HgoMaterial, IDENTITY, Matrix, ReferenceStressMemory, Response, det, inverse, mm, transpose,
};

#[derive(Clone, Debug)]
pub struct ViscoelasticHgo {
    elastic: HgoMaterial,
    /// Unprojected stresses conjugate to Cbar, in the fixed material frame.
    branches: Vec<ReferenceStressMemory>,
    pub(super) trial_seconds: f64,
    stiffness_pa: f64,
}
impl ViscoelasticHgo {
    pub(super) fn stiffness(&self) -> f64 {
        self.stiffness_pa
    }
    /// Response with the last committed branch memory held fixed. No time
    /// increment is performed. The potential is normalized at Cbar=I.
    pub fn committed_response(&self, f: Matrix) -> Result<Response, &'static str> {
        let (cbar, mut stress, j) = self.metric(f)?;
        let (mut energy, _) = self.elastic.isochoric_metric_response(cbar)?;
        for branch in &self.branches {
            let memory = branch.memory();
            for i in 0..3 {
                for k in 0..3 {
                    energy += 0.5 * memory[i][k] * (cbar[i][k] - IDENTITY[i][k]);
                    stress[i][k] += memory[i][k];
                }
            }
        }
        let q = j.powf(-2. / 3.);
        let fs = mm(f, stress);
        let inv_t = transpose(inverse(f)?);
        let contraction = (0..3)
            .flat_map(|i| (0..3).map(move |k| cbar[i][k] * stress[i][k]))
            .sum::<f64>();
        let first_piola = std::array::from_fn(|i| {
            std::array::from_fn(|k| {
                q * fs[i][k] - contraction / 3. * inv_t[i][k]
                    + self.elastic.bulk_pa * (j - 1.) * j * inv_t[i][k]
            })
        });
        energy += 0.5 * self.elastic.bulk_pa * (j - 1.).powi(2);
        if !energy.is_finite() || first_piola.iter().flatten().any(|x| !x.is_finite()) {
            return Err("committed HGO response overflow");
        }
        Ok(Response {
            energy_density: energy,
            first_piola,
            volume_ratio: j,
        })
    }
    /// Equilibrium HGO coefficients plus additive branch (tau_seconds,beta).
    /// Initial history is the relaxed undeformed reference, not preconditioned tissue.
    pub fn new(elastic: HgoMaterial, spectrum: &[(f64, f64)]) -> Result<Self, &'static str> {
        if spectrum.len() > 16 {
            return Err("too many HGO memory branches");
        }
        let (_, initial) = elastic.isochoric_metric_response(IDENTITY)?;
        let branches = spectrum
            .iter()
            .map(|&(tau, beta)| ReferenceStressMemory::new(tau, beta, initial, [[0.; 3]; 3]))
            .collect::<Result<Vec<_>, _>>()?;
        let stiffness_pa = elastic.bulk_pa
            + (1. + spectrum.iter().map(|x| x.1).sum::<f64>())
                * (elastic.shear_pa
                    + 4. * elastic.fibers.iter().map(|f| f.stiffness_pa).sum::<f64>());
        if !stiffness_pa.is_finite() {
            return Err("HGO preconditioner overflow");
        }
        Ok(Self {
            elastic,
            branches,
            trial_seconds: 0.,
            stiffness_pa,
        })
    }
    fn metric(&self, f: Matrix) -> Result<(Matrix, Matrix, f64), &'static str> {
        self.elastic.response(f)?;
        let j = det(f);
        let q = j.powf(-2. / 3.);
        let cbar = mm(transpose(f), f).map(|r| r.map(|x| q * x));
        let (_, stress) = self.elastic.isochoric_metric_response(cbar)?;
        Ok((cbar, stress, j))
    }
    /// Frozen-history incremental energy and exact first Piola derivative.
    /// Branch convolution assumes linear unprojected elastic stress over dt.
    pub fn trial(&self, f: Matrix, seconds: f64) -> Result<Response, &'static str> {
        if !seconds.is_finite() || seconds <= 0. {
            return Err("invalid HGO physical interval");
        }
        let (cbar, elastic_stress, j) = self.metric(f)?;
        let (elastic_energy, _) = self.elastic.isochoric_metric_response(cbar)?;
        let (reference_energy, reference_stress) =
            self.elastic.isochoric_metric_response(IDENTITY)?;
        let mut energy = elastic_energy;
        let mut stress = elastic_stress;
        for branch in &self.branches {
            let frozen = branch.frozen_potential(seconds)?;
            let (e, s) = frozen.response(cbar, elastic_energy, elastic_stress)?;
            let reference = frozen
                .response(IDENTITY, reference_energy, reference_stress)?
                .0;
            energy += e - reference - elastic_energy;
            for i in 0..3 {
                for k in 0..3 {
                    stress[i][k] += s[i][k] - elastic_stress[i][k];
                }
            }
        }
        let q = j.powf(-2. / 3.);
        let fs = mm(f, stress);
        let inv_t = transpose(inverse(f)?);
        let contraction = (0..3)
            .flat_map(|i| (0..3).map(move |k| cbar[i][k] * stress[i][k]))
            .sum::<f64>();
        let first_piola = std::array::from_fn(|i| {
            std::array::from_fn(|k| {
                q * fs[i][k] - contraction / 3. * inv_t[i][k]
                    + self.elastic.bulk_pa * (j - 1.) * j * inv_t[i][k]
            })
        });
        energy += 0.5 * self.elastic.bulk_pa * (j - 1.).powi(2);
        if !energy.is_finite() || first_piola.iter().flatten().any(|x| !x.is_finite()) {
            return Err("viscoelastic HGO overflow");
        }
        Ok(Response {
            energy_density: energy,
            first_piola,
            volume_ratio: j,
        })
    }
    /// Commit all branch histories once after accepting a physical step.
    /// Any rejected input leaves every branch unchanged.
    pub fn advance(&mut self, f: Matrix, seconds: f64) -> Result<Response, &'static str> {
        let response = self.trial(f, seconds)?;
        let (_, stress, _) = self.metric(f)?;
        let mut candidate = self.clone();
        for branch in &mut candidate.branches {
            branch.advance(stress, seconds)?;
        }
        *self = candidate;
        Ok(response)
    }
}

impl super::Body {
    /// Atomically assign passive HGO history to selected elements. No reference
    /// rebasing or implicit history reset beyond each explicitly supplied law.
    pub fn set_viscoelastic_hgo_batch(
        &mut self,
        assignments: &[(usize, ViscoelasticHgo)],
    ) -> Result<(), &'static str> {
        let mut seen = std::collections::BTreeSet::new();
        if assignments.iter().any(|(i, _)| {
            *i >= self.elements.len() || self.elements[*i].activation != 0. || !seen.insert(*i)
        }) {
            return Err("invalid HGO element assignment");
        }
        let mut candidate = self.clone();
        for (index, law) in assignments {
            candidate.elements[*index].myocardium = None;
            candidate.elements[*index].viscoelastic = None;
            candidate.elements[*index].viscoelastic_hgo = Some(law.clone());
        }
        // Shared diagonal construction covers every retained material law.
        candidate.set_viscoelastic_ogden_batch(&[])?;
        *self = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biomechanics::Fiber;
    fn law() -> ViscoelasticHgo {
        ViscoelasticHgo::new(
            HgoMaterial {
                shear_pa: 3000.,
                bulk_pa: 50000.,
                fibers: vec![Fiber {
                    direction: [1., 0., 0.],
                    stiffness_pa: 9000.,
                    exponent: 0.2,
                    active_pa: 0.,
                }],
            },
            &[(0.2, 0.3), (2., 0.5)],
        )
        .unwrap()
    }
    #[test]
    fn physical_step_commit_preserves_end_stress_and_frozen_gradient() {
        let mut m = law();
        let f = [[1.2, 0.12, 0.03], [0.04, 0.9, 0.07], [0., 0.02, 1.05]];
        let accepted = m.advance(f, 0.1).unwrap();
        let committed = m.committed_response(f).unwrap();
        for i in 0..3 {
            for k in 0..3 {
                assert!((accepted.first_piola[i][k] - committed.first_piola[i][k]).abs() < 1e-8);
                let mut plus = f;
                let mut minus = f;
                let h = 1e-6;
                plus[i][k] += h;
                minus[i][k] -= h;
                let fd = (m.committed_response(plus).unwrap().energy_density
                    - m.committed_response(minus).unwrap().energy_density)
                    / (2. * h);
                assert!((fd - committed.first_piola[i][k]).abs() < 1e-4);
            }
        }
        assert_eq!(
            m.committed_response(f).unwrap().first_piola,
            committed.first_piola
        );
    }
    #[test]
    fn frozen_history_gradient_and_rotation() {
        let mut m = law();
        m.advance([[1.1, 0.02, 0.], [0., 0.97, 0.], [0., 0., 0.95]], 0.1)
            .unwrap();
        let f = [[1.2, 0.12, 0.03], [0.04, 0.9, 0.07], [0., 0.02, 1.05]];
        let r = m.trial(f, 0.05).unwrap();
        let h = 1e-6;
        for i in 0..3 {
            for k in 0..3 {
                let mut plus = f;
                let mut minus = f;
                plus[i][k] += h;
                minus[i][k] -= h;
                let fd = (m.trial(plus, 0.05).unwrap().energy_density
                    - m.trial(minus, 0.05).unwrap().energy_density)
                    / (2. * h);
                assert!((fd - r.first_piola[i][k]).abs() < 1e-4);
            }
        }
        let rotation = [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]];
        let rotated = m.trial(mm(rotation, f), 0.05).unwrap();
        assert!((r.energy_density - rotated.energy_density).abs() < 1e-8);
        let expected = mm(rotation, r.first_piola);
        for i in 0..3 {
            for k in 0..3 {
                assert!((rotated.first_piola[i][k] - expected[i][k]).abs() < 1e-8);
            }
        }
    }
    #[test]
    fn isotropic_volume_and_rejected_history() {
        let mut m = law();
        let control = m.clone();
        assert!(m.advance([[0.; 3]; 3], 0.1).is_err());
        let uniform = IDENTITY.map(|r| r.map(|x| 1.1 * x));
        let visc = m.trial(uniform, 0.1).unwrap();
        let elastic = m.elastic.response(uniform).unwrap();
        assert!((visc.energy_density - elastic.energy_density).abs() < 1e-9);
        for i in 0..3 {
            for k in 0..3 {
                assert!((visc.first_piola[i][k] - elastic.first_piola[i][k]).abs() < 1e-8);
            }
        }
        let f = [[1.1, 0., 0.], [0., 0.95, 0.], [0., 0., 0.96]];
        assert_eq!(
            m.trial(f, 0.1).unwrap().first_piola,
            control.trial(f, 0.1).unwrap().first_piola
        );
    }
    #[test]
    fn closed_imposed_stretch_cycles_have_positive_work_in_checked_protocols() {
        // A numerical protocol check, not a general dissipation theorem.
        for seconds in [0.01, 0.1] {
            let mut m = law();
            let mut previous_f = IDENTITY;
            let mut previous_p = m.trial(IDENTITY, seconds).unwrap().first_piola;
            let mut work = 0.;
            for k in 1..=400 {
                let fraction = if k <= 200 {
                    f64::from(k) / 200.
                } else {
                    f64::from(400 - k) / 200.
                };
                let stretch = 1. + 0.2 * fraction;
                let transverse = 1. / stretch.sqrt();
                let f = [
                    [stretch, 0., 0.],
                    [0., transverse, 0.],
                    [0., 0., transverse],
                ];
                let p = m.advance(f, seconds).unwrap().first_piola;
                for i in 0..3 {
                    for j in 0..3 {
                        work += 0.5 * (previous_p[i][j] + p[i][j]) * (f[i][j] - previous_f[i][j]);
                    }
                }
                previous_p = p;
                previous_f = f;
            }
            assert!(
                work.is_finite() && work > 0.,
                "cycle work {work} J/m^3 at dt={seconds}"
            );
            println!("imposed HGO cycle dt={seconds} s, net work={work} J/m^3");
        }
    }
}
