//! Experimental generalized-Newtonian shear thinning from the central SPH strain-rate moments.
use super::{
    Error, Liquid, Material, Particle, density_kernel_gradient, finite, norm, pairs, positive, sub,
};

/// Regularized yield-stress law: `mu=mu_ref*(rate/ref)^(n-1)+yield_stress/rate`.
/// Rate flooring and viscosity caps permit creep; this is not an exact solid plug.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HerschelBulkley {
    pub shear_thinning: ShearThinning,
    /// Stress in Pa when using SI units; finite and nonnegative.
    pub yield_stress: f64,
}

/// Homogenized illustrative foam, without resolved bubbles or elastic memory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WhippedCreamProfile {
    pub material: Material,
    pub rheology: HerschelBulkley,
}
impl WhippedCreamProfile {
    /// Demonstration coefficients, not a fit to a measured product.
    pub const DEMO: Self = Self {
        material: Material {
            rest_density: 500.0,
            sound_speed: 20.0,
            viscosity: 5.0,
        },
        rheology: HerschelBulkley {
            shear_thinning: ShearThinning {
                reference_rate: 1.0,
                flow_index: 0.5,
                minimum_rate: 0.01,
                minimum_viscosity: 0.01,
                maximum_viscosity: 10_000.0,
            },
            yield_stress: 30.0,
        },
    };
    /// Sets bulk density from liquid density and fractional volume overrun.
    /// E.g. overrun=1 doubles volume, assuming negligible gas mass.
    /// # Errors
    /// Rejects invalid coefficients and density overflow/underflow.
    pub fn with_overrun(mut self, liquid_density: f64, overrun: f64) -> Result<Self, Error> {
        if !positive(liquid_density) || !overrun.is_finite() || overrun < 0.0 {
            return Err(Error::InvalidMaterial);
        }
        let density = liquid_density / (1.0 + overrun);
        if !positive(density) {
            return Err(Error::InvalidMaterial);
        }
        self.material.rest_density = density;
        Ok(self)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShearThinning {
    /// Rate where the material's current viscosity is the reference viscosity (1/time).
    pub reference_rate: f64,
    /// Power-law flow index, 0 < n <= 1. n=1 preserves Newtonian viscosity.
    pub flow_index: f64,
    /// Positive regularization near zero shear (1/time).
    pub minimum_rate: f64,
    pub minimum_viscosity: f64,
    pub maximum_viscosity: f64,
}
impl ShearThinning {
    /// Illustrative condensed-milk-like flow; not a measured brand calibration.
    pub const CONDENSED_MILK_DEMO: Self = Self {
        reference_rate: 1.0,
        flow_index: 0.75,
        minimum_rate: 0.01,
        minimum_viscosity: 0.05,
        maximum_viscosity: 100.0,
    };
    fn valid(self) -> bool {
        positive(self.reference_rate)
            && positive(self.flow_index)
            && self.flow_index <= 1.0
            && positive(self.minimum_rate)
            && self.minimum_viscosity.is_finite()
            && self.minimum_viscosity >= 0.0
            && positive(self.maximum_viscosity)
            && self.maximum_viscosity >= self.minimum_viscosity
    }
}
impl Liquid {
    /// Configures per-material shear thinning. Temperature/phase/solute responses
    /// supply the reference viscosity first. Failure preserves all configuration.
    /// This estimates strain from real fluid neighbors; wall-gradient correction,
    /// viscoelastic memory is not modelled. Optional structural kinetics is configured separately.
    /// # Errors
    /// Rejects invalid coefficients/count, numerical overflow and neighbor budgets.
    pub fn configure_shear_thinning(
        &mut self,
        models: Vec<Option<ShearThinning>>,
    ) -> Result<(), Error> {
        if models.len() != self.materials.len() || models.iter().flatten().any(|m| !m.valid()) {
            return Err(Error::InvalidPropertyResponse);
        }
        let mut candidate = self.clone();
        candidate.shear_thinning = models;
        candidate.yield_stresses.fill(0.0);
        candidate.effective_materials()?;
        self.shear_thinning = candidate.shear_thinning;
        self.yield_stresses = candidate.yield_stresses;
        Ok(())
    }
    /// Atomically replaces all shear laws with regularized Herschel–Bulkley models.
    /// Material viscosity is the consistency term at the configured reference rate.
    /// Temperature/solute laws affect that term; yield stress remains constant.
    /// # Errors
    /// Invalid coefficients/count, constitutive overflow or neighbor budgets.
    pub fn configure_herschel_bulkley(
        &mut self,
        models: &[Option<HerschelBulkley>],
    ) -> Result<(), Error> {
        if models.len() != self.materials.len()
            || models.iter().flatten().any(|m| {
                !m.shear_thinning.valid() || !m.yield_stress.is_finite() || m.yield_stress < 0.0
            })
        {
            return Err(Error::InvalidPropertyResponse);
        }
        let mut candidate = self.clone();
        candidate.shear_thinning = models.iter().map(|m| m.map(|v| v.shear_thinning)).collect();
        candidate.yield_stresses = models
            .iter()
            .map(|m| m.map_or(0.0, |v| v.yield_stress))
            .collect();
        candidate.effective_materials()?;
        self.shear_thinning = candidate.shear_thinning;
        self.yield_stresses = candidate.yield_stresses;
        Ok(())
    }
    pub(super) fn apply_shear_thinning(
        &self,
        particles: &[Particle],
        properties: &mut [Material],
    ) -> Result<(), Error> {
        if self.shear_thinning.iter().all(Option::is_none) {
            return Ok(());
        }
        let rates = self.strain_rates(particles, properties)?;
        for (i, property) in properties.iter_mut().enumerate() {
            let Some(model) = self.shear_thinning[particles[i].material] else {
                continue;
            };
            let rate = rates[i].max(model.minimum_rate);
            let viscosity = property.viscosity
                * (rate / model.reference_rate).powf(model.flow_index - 1.0)
                + self.yield_stresses[particles[i].material] * self.structure_yield_factor(i)
                    / rate;
            if !viscosity.is_finite() {
                return Err(Error::NumericalFailure);
            }
            property.viscosity = viscosity.clamp(model.minimum_viscosity, model.maximum_viscosity);
        }
        Ok(())
    }
    pub(super) fn strain_rates(
        &self,
        particles: &[Particle],
        properties: &[Material],
    ) -> Result<Vec<f64>, Error> {
        let mut neighbors = pairs(
            particles,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )?;
        self.retain_carrier_pairs(&mut neighbors);
        let density = self.particle_densities(particles, &neighbors, properties);
        if density.iter().any(|rho| !positive(*rho)) {
            return Err(Error::NumericalFailure);
        }
        let mut gradients = vec![[[0.0; 3]; 3]; particles.len()];
        let mut fits = vec![StrainFit::default(); particles.len()];
        for (i, j) in neighbors {
            let displacement = sub(particles[i].position, particles[j].position);
            let radius = norm(displacement);
            if radius <= 1e-12 {
                continue;
            }
            let direction = displacement.map(|r| r / radius);
            let relative = sub(particles[i].velocity, particles[j].velocity);
            let radial_speed: f64 = relative.iter().zip(direction).map(|(v, n)| v * n).sum();
            let kernel_gradient =
                density_kernel_gradient(self.formulation, self.config.smoothing_radius, radius);
            let magnitude = kernel_gradient * radial_speed;
            fits[i].add(
                direction,
                radial_speed / radius,
                particles[j].mass / density[j] * kernel_gradient * radius,
            );
            fits[j].add(
                direction,
                radial_speed / radius,
                particles[i].mass / density[i] * kernel_gradient * radius,
            );
            for (a, normal_a) in direction.into_iter().enumerate() {
                for (b, normal_b) in direction.into_iter().enumerate() {
                    let moment = magnitude * normal_a * normal_b;
                    gradients[i][a][b] += particles[j].mass / density[j] * moment;
                    gradients[j][a][b] += particles[i].mass / density[i] * moment;
                }
            }
        }
        let mut rates = Vec::with_capacity(particles.len());
        for i in 0..particles.len() {
            // Isotropic kernel fourth moment: A=(2D+tr(D)I)/5.
            // Radial speeds vanish for rigid rotation even on irregular clouds.
            let trace = gradients[i][0][0] + gradients[i][1][1] + gradients[i][2][2];
            let mut squared = 0.0;
            for (a, row) in gradients[i].iter().enumerate() {
                for (b, moment) in row.iter().enumerate() {
                    let strain = 0.5 * (5.0 * moment - if a == b { trace } else { 0.0 });
                    squared += 2.0 * strain * strain;
                }
            }
            if !squared.is_finite() || gradients[i].iter().any(|r| !finite(*r)) {
                return Err(Error::NumericalFailure);
            }
            let squared = fits[i].strain_squared()?.unwrap_or(squared);
            rates.push(squared.sqrt());
        }
        Ok(rates)
    }
}

#[derive(Clone, Default)]
struct StrainFit {
    system: [[f64; 7]; 6],
}
impl StrainFit {
    fn add(&mut self, normal: [f64; 3], radial_gradient: f64, weight: f64) {
        let [x, y, z] = normal;
        let row = [x * x, y * y, z * z, 2.0 * x * y, 2.0 * x * z, 2.0 * y * z];
        for (a, coefficient) in row.into_iter().enumerate() {
            for (b, other) in row.into_iter().enumerate() {
                self.system[a][b] += weight * coefficient * other;
            }
            self.system[a][6] += weight * coefficient * radial_gradient;
        }
    }
    fn strain_squared(&self) -> Result<Option<f64>, Error> {
        if self.system.iter().flatten().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        let scale = self
            .system
            .iter()
            .flat_map(|r| r[..6].iter())
            .map(|v| v.abs())
            .fold(0.0, f64::max);
        if scale == 0.0 {
            return Ok(None);
        }
        let mut matrix = self.system;
        for column in 0..6 {
            let pivot = (column..6)
                .max_by(|a, b| {
                    matrix[*a][column]
                        .abs()
                        .total_cmp(&matrix[*b][column].abs())
                })
                .ok_or(Error::NumericalFailure)?;
            if matrix[pivot][column].abs() <= 1e-10 * scale {
                return Ok(None);
            }
            matrix.swap(column, pivot);
            let divisor = matrix[column][column];
            for value in &mut matrix[column] {
                *value /= divisor;
            }
            let pivot_row = matrix[column];
            for (index, row) in matrix.iter_mut().enumerate() {
                if index == column {
                    continue;
                }
                let factor = row[column];
                for (value, pivot_value) in row.iter_mut().zip(pivot_row) {
                    *value -= factor * pivot_value;
                }
            }
        }
        let squared = 2.0
            * (matrix[..3].iter().map(|r| r[6].powi(2)).sum::<f64>()
                + 2.0 * matrix[3..].iter().map(|r| r[6].powi(2)).sum::<f64>());
        if !squared.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(Some(squared))
    }
}

#[cfg(test)]
mod tests {
    use super::StrainFit;
    #[test]
    fn cream_flow_curve_density_and_atomic_configuration() {
        use super::{HerschelBulkley, WhippedCreamProfile};
        use crate::liquid::{Config, Liquid, Particle};
        let profile = WhippedCreamProfile::DEMO.with_overrun(1000.0, 1.0).unwrap();
        assert!((profile.material.rest_density - 500.0).abs() < 1e-12);
        assert!(profile.with_overrun(1000.0, -1.0).is_err());
        for rate in [0.0_f64, 1.0, 100.0] {
            let mut particles = Vec::new();
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        particles.push(Particle {
                            position: [f64::from(x) * 0.1, f64::from(y) * 0.1, f64::from(z) * 0.1],
                            velocity: [rate * f64::from(y) * 0.1, 0.0, 0.0],
                            mass: 0.5,
                            material: 0,
                        });
                    }
                }
            }
            let mut fluid = Liquid::new(
                particles,
                vec![profile.material],
                Config {
                    smoothing_radius: 1.0,
                    ..Config::default()
                },
            )
            .unwrap();
            fluid
                .configure_herschel_bulkley(&[Some(profile.rheology)])
                .unwrap();
            let gamma = rate.max(0.01);
            let expected = 5.0 * gamma.powf(-0.5) + 30.0 / gamma;
            for material in fluid.effective_materials().unwrap() {
                assert!((material.viscosity - expected).abs() < 1e-8);
            }
            let before = fluid.clone();
            assert!(
                fluid
                    .configure_herschel_bulkley(&[Some(HerschelBulkley {
                        yield_stress: f64::NAN,
                        ..profile.rheology
                    })])
                    .is_err()
            );
            assert_eq!(fluid, before);
            if rate < 0.01 {
                assert!(
                    fluid
                        .configure_herschel_bulkley(&[Some(HerschelBulkley {
                            yield_stress: f64::MAX,
                            ..profile.rheology
                        })])
                        .is_err()
                );
                assert_eq!(fluid, before);
            }
            fluid
                .configure_shear_thinning(vec![Some(profile.rheology.shear_thinning)])
                .unwrap();
            for material in fluid.effective_materials().unwrap() {
                assert!((material.viscosity - 5.0 * gamma.powf(-0.5)).abs() < 1e-8);
            }
        }
    }
    #[test]
    fn rank_deficient_strain_fit_requests_fallback_and_overflow_is_rejected() {
        let mut fit = StrainFit::default();
        fit.add([1.0, 0.0, 0.0], 1.0, 1.0);
        assert!(fit.strain_squared().unwrap().is_none());
        fit.add([1.0, 0.0, 0.0], 1.0, f64::MAX);
        fit.add([1.0, 0.0, 0.0], 1.0, f64::MAX);
        assert!(fit.strain_squared().is_err());
    }
}
