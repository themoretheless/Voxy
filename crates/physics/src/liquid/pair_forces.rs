//! Fluid-fluid force stage, isolated from wall/image/polymer contributions.
//! Preserve pair order and scalar arithmetic: native state publication and all
//! physical model choices stay with Liquid, independent of a future GPU backend.
use super::{
    Error, Formulation, Liquid, Material, Particle, cohesion_kernel, color_normals, finite, norm,
    positive, sub, viscous_conductance,
};

impl Liquid {
    pub(super) fn particle_pair_forces(
        &self,
        particles: &[Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
        density: &[f64],
        pressure: &[f64],
        normal_override: Option<Vec<[f64; 3]>>,
        mut backend: Option<&mut dyn super::LiquidDensityBackend>,
    ) -> (Vec<[f64; 3]>, Vec<f64>) {
        let h = self.config.smoothing_radius;
        let normal_candidate = normal_override.or_else(|| {
            backend.as_mut().and_then(|backend| {
                if !backend.supports_normals() {
                    return None;
                }
                backend.normal_candidates(&super::LiquidNormalOperator {
                    particles,
                    pairs,
                    densities: density,
                    smoothing_radius: h,
                })
            })
        });
        let normals =
            normal_candidate.unwrap_or_else(|| color_normals(particles, pairs, density, h));
        if let Some(backend) = backend
            .as_mut()
            .filter(|backend| backend.supports_force_pipeline())
        {
            if !pairs.is_empty() {
                let inputs = pairs
                    .iter()
                    .map(|&(i, j)| {
                        self.particle_pair_force_input(
                            particles[i],
                            particles[j],
                            properties[i],
                            properties[j],
                            [density[i], density[j]],
                            [pressure[i], pressure[j]],
                            sub(normals[i], normals[j]),
                        )
                    })
                    .collect::<Vec<_>>();
                let operator = super::LiquidForcePipelineOperator {
                    particles,
                    pairs,
                    inputs: &inputs,
                    gravity: self.config.gravity,
                };
                if let Some(candidate) = backend.force_pipeline_candidate(&operator) {
                    return (
                        candidate.accumulation.accelerations,
                        candidate.accumulation.viscous_rates,
                    );
                }
            }
        }
        let mut candidates = backend
            .as_mut()
            .filter(|backend| backend.supports_pair_forces())
            .and_then(|backend| {
                if pairs.is_empty() {
                    return None;
                }
                let inputs = pairs
                    .iter()
                    .map(|&(i, j)| {
                        self.particle_pair_force_input(
                            particles[i],
                            particles[j],
                            properties[i],
                            properties[j],
                            [density[i], density[j]],
                            [pressure[i], pressure[j]],
                            sub(normals[i], normals[j]),
                        )
                    })
                    .collect::<Vec<_>>();
                backend.pair_force_candidates(&inputs)
            });
        if let Some(backend) = backend
            .as_mut()
            .filter(|backend| backend.supports_accumulation())
        {
            let forces = candidates.get_or_insert_with(|| {
                pairs
                    .iter()
                    .map(|&(i, j)| {
                        self.particle_pair_force_input(
                            particles[i],
                            particles[j],
                            properties[i],
                            properties[j],
                            [density[i], density[j]],
                            [pressure[i], pressure[j]],
                            sub(normals[i], normals[j]),
                        )
                        .evaluate_validated()
                    })
                    .collect()
            });
            let operator = super::LiquidAccumulationOperator {
                particles,
                pairs,
                forces,
                gravity: self.config.gravity,
            };
            if let Some(result) = backend.accumulation_candidate(&operator) {
                return (result.accelerations, result.viscous_rates);
            }
        }
        let mut acceleration = vec![self.config.gravity; particles.len()];
        let mut viscous_rates = vec![0.0_f64; particles.len()];
        for (index, &(i, j)) in pairs.iter().enumerate() {
            let a = particles[i];
            let b = particles[j];
            let input = self.particle_pair_force_input(
                a,
                b,
                properties[i],
                properties[j],
                [density[i], density[j]],
                [pressure[i], pressure[j]],
                sub(normals[i], normals[j]),
            );
            let result = candidates
                .as_ref()
                .map_or_else(|| input.evaluate_validated(), |results| results[index]);
            let [f0, f1, f2] = result.force;
            viscous_rates[i] += result.viscous_conductance / a.mass;
            viscous_rates[j] += result.viscous_conductance / b.mass;
            let inv_a = 1.0 / a.mass;
            let inv_b = 1.0 / b.mass;
            acceleration[i][0] += f0 * inv_a;
            acceleration[i][1] += f1 * inv_a;
            acceleration[i][2] += f2 * inv_a;
            acceleration[j][0] -= f0 * inv_b;
            acceleration[j][1] -= f1 * inv_b;
            acceleration[j][2] -= f2 * inv_b;
        }
        (acceleration, viscous_rates)
    }
}

/// Immutable original-model input for one fluid-fluid pair. Geometry, material
/// selection and color normals remain owned by physics; compute supplies a candidate.
#[derive(Clone, Copy, Debug)]
pub struct LiquidPairForceInput {
    pub delta: [f64; 3],
    pub relative_velocity: [f64; 3],
    pub normal_difference: [f64; 3],
    pub mass: [f64; 2],
    pub density: [f64; 2],
    pub pressure: [f64; 2],
    pub reference_density: [f64; 2],
    pub smoothing_radius: f64,
    pub viscosity: f64,
    pub surface_strength: f64,
    pub interface_coefficient: f64,
    pub formulation: Formulation,
    pub gas_pressure: bool,
    pub viscous_heating: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidPairForceResult {
    /// Force on the first particle; the second receives its exact negative.
    pub force: [f64; 3],
    pub viscous_conductance: f64,
}
impl LiquidPairForceInput {
    pub fn validate(&self) -> Result<(), Error> {
        if !finite(self.delta)
            || !finite(self.relative_velocity)
            || !finite(self.normal_difference)
            || !positive(self.smoothing_radius)
            || self
                .mass
                .iter()
                .chain(&self.density)
                .chain(&self.reference_density)
                .any(|&x| !positive(x))
            || self
                .pressure
                .iter()
                .chain([
                    &self.viscosity,
                    &self.surface_strength,
                    &self.interface_coefficient,
                ])
                .any(|&x| !x.is_finite() || x < 0.)
        {
            return Err(Error::NumericalFailure);
        }
        Ok(())
    }
    pub fn evaluate_native(&self) -> Result<LiquidPairForceResult, Error> {
        self.validate()?;
        let result = self.evaluate_validated();
        if !finite(result.force)
            || !result.viscous_conductance.is_finite()
            || result.viscous_conductance < 0.
        {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }
    pub fn admit(
        &self,
        candidate: LiquidPairForceResult,
        absolute: f64,
        relative: f64,
    ) -> Result<(), Error> {
        if !absolute.is_finite() || absolute < 0. || !relative.is_finite() || relative < 0. {
            return Err(Error::InvalidConfig);
        }
        let expected = self.evaluate_native()?;
        for (a, b) in candidate
            .force
            .into_iter()
            .chain([candidate.viscous_conductance])
            .zip(
                expected
                    .force
                    .into_iter()
                    .chain([expected.viscous_conductance]),
            )
        {
            let error = (a - b).abs();
            if !a.is_finite()
                || !error.is_finite()
                || (error > absolute && (b == 0. || error / b.abs() > relative))
            {
                return Err(Error::NumericalFailure);
            }
        }
        if candidate.viscous_conductance < 0. {
            return Err(Error::NumericalFailure);
        }
        Ok(())
    }
    fn evaluate_validated(&self) -> LiquidPairForceResult {
        let h = self.smoothing_radius;
        let r = norm(self.delta);
        let direction = if r > 1e-12 {
            self.delta.map(|x| x / r)
        } else {
            [1., 0., 0.]
        };
        let pressure_force = if self.formulation == Formulation::MassDensity && self.gas_pressure {
            self.mass[0]
                * self.mass[1]
                * (self.pressure[0] / self.density[0].powi(2)
                    + self.pressure[1] / self.density[1].powi(2))
                * super::density_kernel_gradient(self.formulation, h, r)
        } else {
            super::pressure_magnitude(
                self.mass,
                self.density,
                self.pressure,
                self.reference_density,
                h,
                r,
                self.formulation,
            )
        };
        let viscosity = viscous_conductance(self.mass, self.viscosity, self.density, r, h);
        let surface = self.surface_strength;
        let correction = (self.reference_density[0] + self.reference_density[1])
            / (self.density[0] + self.density[1]);
        let cohesion = if r > 1e-12 {
            surface * self.mass[0] * self.mass[1] * correction * cohesion_kernel(r, h)
        } else {
            0.
        };
        let curvature = surface
            * correction
            * (2. * self.mass[0] * self.mass[1] / (self.mass[0] + self.mass[1]));
        let dn = self.normal_difference;
        let curvature_normal = dn[0] * direction[0] + dn[1] * direction[1] + dn[2] * direction[2];
        let dv = self.relative_velocity;
        let relative_normal = dv[0] * direction[0] + dv[1] * direction[1] + dv[2] * direction[2];
        let interface_force = self.interface_coefficient * r * (h - r).max(0.).powi(2);
        let radial = pressure_force - cohesion + interface_force - curvature * curvature_normal;
        let visc = if self.viscous_heating {
            0.
        } else {
            viscosity * relative_normal
        };
        let total_mag = radial + visc;
        LiquidPairForceResult {
            force: direction.map(|x| total_mag * x),
            viscous_conductance: viscosity,
        }
    }
}
impl Liquid {
    pub(super) fn particle_pair_force_input(
        &self,
        a: Particle,
        b: Particle,
        ma: Material,
        mb: Material,
        density: [f64; 2],
        pressure: [f64; 2],
        normal_difference: [f64; 3],
    ) -> LiquidPairForceInput {
        LiquidPairForceInput {
            delta: sub(a.position, b.position),
            relative_velocity: sub(b.velocity, a.velocity),
            normal_difference,
            mass: [a.mass, b.mass],
            density,
            pressure,
            reference_density: [ma.rest_density, mb.rest_density],
            smoothing_radius: self.config.smoothing_radius,
            viscosity: 0.5 * (ma.viscosity + mb.viscosity),
            surface_strength: if a.material == b.material {
                self.surface_strengths[a.material]
            } else {
                0.
            },
            interface_coefficient: self.interface_coefficient(a, b),
            formulation: self.formulation,
            gas_pressure: self.gas_equations[a.material].is_some()
                || self.gas_equations[b.material].is_some(),
            viscous_heating: self.viscous_heating,
        }
    }
    /// Snapshot the current fluid-fluid force inputs in original neighbor order.
    /// Wall/image density is included; wall/image/polymer forces are separate.
    pub fn pair_force_inputs(&self) -> Result<Vec<LiquidPairForceInput>, Error> {
        self.validate_gas_geometry()?;
        let particles = &self.particles;
        let pairs = super::pairs(
            particles,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )?;
        let properties = self.effective_materials()?;
        let mut density = self.particle_densities(particles, &pairs, &properties);
        let boundaries = self.boundary_pairs(particles, pairs.len())?;
        self.add_boundary_density(&properties, &boundaries, &mut density);
        let images = self.image_pairs(particles, pairs.len())?;
        self.image_density(particles, &properties, &images, &mut density);
        let pressure = particles
            .iter()
            .zip(&properties)
            .zip(&density)
            .map(|((p, m), rho)| self.material_pressure(p.material, *m, *rho))
            .collect::<Result<Vec<_>, _>>()?;
        let normals = color_normals(particles, &pairs, &density, self.config.smoothing_radius);
        pairs
            .into_iter()
            .map(|(i, j)| {
                let input = self.particle_pair_force_input(
                    particles[i],
                    particles[j],
                    properties[i],
                    properties[j],
                    [density[i], density[j]],
                    [pressure[i], pressure[j]],
                    sub(normals[i], normals[j]),
                );
                input.validate()?;
                Ok(input)
            })
            .collect()
    }
}
