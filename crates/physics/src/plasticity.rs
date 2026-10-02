//! Rate-independent small-strain J2 plasticity with linear isotropic hardening.
//! SI units: dimensionless tensorial strain, stress in Pa, energy density in J/m³.
//! Use a fixed material frame. Finite rotations, fracture and softening are excluded.
use crate::biomechanics::{Matrix, Stress};
mod work;
pub use work::WorkStep;

#[derive(Clone, Copy, Debug)]
#[allow(clippy::struct_field_names)] // Keep SI units explicit in material constants.
pub struct Material {
    shear_pa: f64,
    bulk_pa: f64,
    yield_pa: f64,
    hardening_pa: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    plastic_strain: Matrix,
    equivalent_plastic_strain: f64,
    dissipated_j_m3: f64,
}
impl State {
    #[must_use]
    pub fn plastic_strain(&self) -> Matrix {
        self.plastic_strain
    }
    #[must_use]
    pub fn equivalent_plastic_strain(&self) -> f64 {
        self.equivalent_plastic_strain
    }
    #[must_use]
    pub fn dissipated_j_m3(&self) -> f64 {
        self.dissipated_j_m3
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Response {
    pub stress: Stress,
    pub plastic_increment: f64,
    pub elastic_energy_j_m3: f64,
    pub hardening_energy_j_m3: f64,
    pub yield_stress_pa: f64,
}
impl Material {
    /// Consistent algorithmic tangent applied to a symmetric strain direction.
    /// Uses the same last accepted history as `response`.
    /// # Errors
    /// Rejects invalid strains and nonrepresentable tangent results.
    pub fn tangent_action(
        &self,
        old: &State,
        strain: Matrix,
        direction: Matrix,
    ) -> Result<Matrix, &'static str> {
        let direction = symmetric(direction)?;
        let (_, response) = self.response(old, strain)?;
        let trial_elastic: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| strain[i][j] - old.plastic_strain[i][j])
        });
        let trace: f64 = (0..3).map(|i| trial_elastic[i][i]).sum();
        let trial: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                2. * self.shear_pa * (trial_elastic[i][j] - if i == j { trace / 3. } else { 0. })
            })
        });
        let q = Stress::from_cauchy(trial)?.von_mises_pa;
        let direction_trace: f64 = (0..3).map(|i| direction[i][i]).sum();
        let increment = response.plastic_increment;
        let factor = if increment > 0. {
            1. - 3. * self.shear_pa * (increment / q)
        } else {
            1.
        };
        let dot: f64 = trial
            .iter()
            .flatten()
            .zip(direction.iter().flatten())
            .map(|(a, b)| a * b)
            .sum();
        let correction = if increment > 0. {
            -9. * (self.shear_pa / q).powi(2)
                * (1. / (3. * self.shear_pa + self.hardening_pa) - increment / q)
                * dot
        } else {
            0.
        };
        let result: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                2. * self.shear_pa
                    * factor
                    * (direction[i][j] - if i == j { direction_trace / 3. } else { 0. })
                    + correction * trial[i][j]
                    + if i == j {
                        self.bulk_pa * direction_trace
                    } else {
                        0.
                    }
            })
        });
        if result.iter().flatten().any(|v| !v.is_finite()) {
            return Err("plastic tangent overflow");
        }
        Ok(result)
    }
    /// # Errors
    /// Requires E > 0, -1 < nu < 0.5, yield > 0 and hardening >= 0;
    /// all parameters and derived moduli must be finite and representable.
    pub fn new(
        young_pa: f64,
        poisson: f64,
        yield_pa: f64,
        hardening_pa: f64,
    ) -> Result<Self, &'static str> {
        let elastic = crate::biomechanics::Material::from_young_poisson(young_pa, poisson)?;
        if !yield_pa.is_finite() || yield_pa <= 0. || !hardening_pa.is_finite() || hardening_pa < 0.
        {
            return Err("invalid plastic material");
        }
        let material = Self {
            shear_pa: elastic.shear_pa,
            bulk_pa: elastic.bulk_pa,
            yield_pa,
            hardening_pa,
        };
        if !(3. * material.shear_pa + hardening_pa).is_finite() {
            return Err("plastic modulus overflow");
        }
        Ok(material)
    }
    /// Backward-Euler radial return for a prescribed total strain tensor.
    /// Off-diagonal components are tensorial strains (engineering shear / 2).
    /// Returns a candidate state; callers must commit it only after global
    /// equilibrium/time-step acceptance. Every nonlinear trial uses the same
    /// last accepted state. Rejected trials cannot accumulate plastic strain.
    /// # Errors
    /// Rejects nonfinite/asymmetric strains and numerical overflow.
    pub fn response(&self, old: &State, strain: Matrix) -> Result<(State, Response), &'static str> {
        let strain = symmetric(strain)?;
        let elastic_trial: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| strain[i][j] - old.plastic_strain[i][j])
        });
        let trace: f64 = (0..3).map(|i| elastic_trial[i][i]).sum();
        let deviator: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| elastic_trial[i][j] - if i == j { trace / 3. } else { 0. })
        });
        let trial = deviator.map(|row| row.map(|v| 2. * self.shear_pa * v));
        let hydrostatic = self.bulk_pa * trace;
        let trial_mises = Stress::from_cauchy(trial)?.von_mises_pa;
        let old_yield = self.yield_pa + self.hardening_pa * old.equivalent_plastic_strain;
        // At an accepted yield surface, roundoff must not choose a plastic
        // loading tangent for an unloading Newton step or accumulate creep.
        let excess = trial_mises - old_yield;
        let roundoff = 64. * f64::EPSILON * trial_mises.max(old_yield);
        let increment = if excess > roundoff {
            excess / (3. * self.shear_pa + self.hardening_pa)
        } else {
            0.
        };
        let mut next = *old;
        if increment > 0. {
            for (i, row) in next.plastic_strain.iter_mut().enumerate() {
                for (j, value) in row.iter_mut().enumerate() {
                    *value += 1.5 * increment * (trial[i][j] / trial_mises);
                }
            }
            next.equivalent_plastic_strain += increment;
            next.dissipated_j_m3 += self.yield_pa * increment;
        }
        let factor = if increment == 0. {
            1.
        } else {
            1. - 3. * self.shear_pa * (increment / trial_mises)
        };
        let cauchy: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| factor * trial[i][j] + if i == j { hydrostatic } else { 0. })
        });
        let stress = Stress::from_cauchy(cauchy)?;
        let elastic_energy_j_m3 = 0.5
            * cauchy
                .iter()
                .flatten()
                .zip(
                    strain
                        .iter()
                        .flatten()
                        .zip(next.plastic_strain.iter().flatten()),
                )
                .map(|(s, (e, p))| s * (e - p))
                .sum::<f64>();
        let hardening_energy_j_m3 =
            0.5 * self.hardening_pa * next.equivalent_plastic_strain.powi(2);
        let yield_stress_pa = self.yield_pa + self.hardening_pa * next.equivalent_plastic_strain;
        if !trace.is_finite()
            || !hydrostatic.is_finite()
            || !old_yield.is_finite()
            || !increment.is_finite()
            || !next.equivalent_plastic_strain.is_finite()
            || !next.dissipated_j_m3.is_finite()
            || next.plastic_strain.iter().flatten().any(|v| !v.is_finite())
            || !elastic_energy_j_m3.is_finite()
            || elastic_energy_j_m3 < 0.
            || !hardening_energy_j_m3.is_finite()
            || !yield_stress_pa.is_finite()
        {
            return Err("plastic response overflow");
        }
        Ok((
            next,
            Response {
                stress,
                plastic_increment: increment,
                elastic_energy_j_m3,
                hardening_energy_j_m3,
                yield_stress_pa,
            },
        ))
    }
}
fn symmetric(strain: Matrix) -> Result<Matrix, &'static str> {
    if strain.iter().flatten().any(|v| !v.is_finite()) {
        return Err("nonfinite strain");
    }
    let scale = strain
        .iter()
        .flatten()
        .fold(0_f64, |a, b| a.max(b.abs()))
        .max(f64::MIN_POSITIVE);
    let mut result = strain;
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        if (strain[i][j] / scale - strain[j][i] / scale).abs() > 1e-12 {
            return Err("asymmetric strain");
        }
        let value = strain[i][j].midpoint(strain[j][i]);
        result[i][j] = value;
        result[j][i] = value;
    }
    Ok(result)
}

pub mod mesh;
