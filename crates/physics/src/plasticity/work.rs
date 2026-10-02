//! Endpoint work partition for the existing small-strain associative J2 return.
use super::{Material, Matrix, Response, State, symmetric};

#[derive(Clone, Copy, Debug)]
pub struct WorkStep {
    /// Candidate only: commit after the global mechanical step is accepted.
    pub state: State,
    pub response: Response,
    pub endpoint_work_j_m3: f64,
    pub elastic_energy_change_j_m3: f64,
    pub hardening_energy_change_j_m3: f64,
    /// Physical irreversible plastic heat; excludes hardening storage.
    pub plastic_heat_j_m3: f64,
    /// Backward-Euler endpoint-work excess; do not deposit as physical heat.
    pub numerical_loss_j_m3: f64,
    pub energy_defect_j_m3: f64,
}
impl Material {
    /// Audit a candidate total-strain increment against its last accepted history.
    /// All tensors and history share the fixed small-strain material frame.
    /// Energies are densities: multiply by reference integration volume for joules.
    /// # Errors
    /// Invalid tensors, incompatible starting history, overflow or energy imbalance.
    pub fn response_with_work(
        &self,
        old: &State,
        previous_strain: Matrix,
        strain: Matrix,
    ) -> Result<WorkStep, &'static str> {
        let previous_strain = symmetric(previous_strain)?;
        let strain = symmetric(strain)?;
        let (_, before) = self.response(old, previous_strain)?;
        if before.plastic_increment != 0. {
            return Err("starting strain requires unaccepted plastic flow");
        }
        let (state, response) = self.response(old, strain)?;
        let delta: Matrix =
            std::array::from_fn(|i| std::array::from_fn(|j| strain[i][j] - previous_strain[i][j]));
        let elastic_delta: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                delta[i][j] - (state.plastic_strain[i][j] - old.plastic_strain[i][j])
            })
        });
        let dot = |a: Matrix, b: Matrix| {
            a.iter()
                .flatten()
                .zip(b.iter().flatten())
                .map(|(a, b)| a * b)
                .sum::<f64>()
        };
        let endpoint_work = dot(response.stress.cauchy_pa, delta);
        let mean_stress = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                before.stress.cauchy_pa[i][j].midpoint(response.stress.cauchy_pa[i][j])
            })
        });
        let elastic_change = dot(mean_stress, elastic_delta);
        let dp = response.plastic_increment;
        let hardening_change = self.hardening_pa
            * dp
            * old
                .equivalent_plastic_strain
                .midpoint(state.equivalent_plastic_strain);
        let heat = self.yield_pa * dp;
        let trace: f64 = (0..3).map(|i| elastic_delta[i][i]).sum();
        let deviator: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| elastic_delta[i][j] - if i == j { trace / 3. } else { 0. })
        });
        let numerical = self.shear_pa * dot(deviator, deviator)
            + 0.5 * self.bulk_pa * trace * trace
            + 0.5 * self.hardening_pa * dp * dp;
        let defect = endpoint_work - elastic_change - hardening_change - heat - numerical;
        let scale =
            endpoint_work.abs() + elastic_change.abs() + hardening_change.abs() + heat + numerical;
        if ![
            endpoint_work,
            elastic_change,
            hardening_change,
            heat,
            numerical,
            defect,
            scale,
        ]
        .iter()
        .all(|v| v.is_finite())
            || heat < 0.
            || numerical < 0.
            || defect.abs() > 1e-10 * scale.max(f64::MIN_POSITIVE)
        {
            return Err("plastic incremental work balance failure");
        }
        Ok(WorkStep {
            state,
            response,
            endpoint_work_j_m3: endpoint_work,
            elastic_energy_change_j_m3: elastic_change,
            hardening_energy_change_j_m3: hardening_change,
            plastic_heat_j_m3: heat,
            numerical_loss_j_m3: numerical,
            energy_defect_j_m3: defect,
        })
    }
}
