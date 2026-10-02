//! Calorically perfect gas EOS, using transported internal energy and SPH mass density.
use super::{Error, Formulation, Liquid, Material, Particle, positive, transport::Transport};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IdealGas {
    /// Specific gas constant, J/(kg K) in SI; positive and finite.
    pub gas_constant: f64,
    /// Constant cp/cv, strictly greater than one.
    pub heat_capacity_ratio: f64,
}
impl IdealGas {
    /// Internal-energy capacity cv = R/(gamma-1), not constant-pressure cp.
    /// # Errors
    /// Invalid coefficients or unrepresentable heat capacity.
    pub fn specific_heat_cv(self) -> Result<f64, Error> {
        if !positive(self.gas_constant)
            || !self.heat_capacity_ratio.is_finite()
            || self.heat_capacity_ratio <= 1.0
        {
            return Err(Error::InvalidMaterial);
        }
        let cv = self.gas_constant / (self.heat_capacity_ratio - 1.0);
        if !positive(cv) {
            return Err(Error::NumericalFailure);
        }
        Ok(cv)
    }
    /// # Errors
    /// Invalid coefficients, nonpositive temperature or pressure overflow.
    pub fn pressure(self, density: f64, temperature: f64) -> Result<f64, Error> {
        self.specific_heat_cv()?;
        let pressure = density * self.gas_constant * temperature;
        if !positive(density) || !positive(temperature) || !positive(pressure) {
            return Err(Error::NumericalFailure);
        }
        Ok(pressure)
    }
}
impl Liquid {
    #[must_use]
    pub(super) fn gas_active(&self) -> bool {
        self.gas_equations.iter().any(Option::is_some)
    }
    /// Enables a calorically perfect EOS per material. Transport capacities must be cv.
    /// Enables pressure work and viscous heating for internal-energy bookkeeping.
    /// No energy is injected; effective gas densities use the current SPH mass density.
    /// # Errors
    /// Invalid arrays/coefficient/cv, absent fields, latent-model conflict, unsupported
    /// sampled boundaries or non-mass-density formulation. Atomic.
    pub fn configure_gas_equations(&mut self, models: Vec<Option<IdealGas>>) -> Result<(), Error> {
        if models.iter().any(Option::is_some)
            && self
                .transport
                .as_ref()
                .and_then(|t| t.species.as_ref())
                .is_some_and(|s| s.heat_capacities.is_some())
        {
            return Err(Error::InvalidTransport);
        }
        if models.len() != self.materials.len() {
            return Err(Error::InvalidMaterial);
        }
        let fields = self.transport.as_ref().ok_or(Error::InvalidTransport)?;
        for (i, model) in models.iter().enumerate() {
            if let Some(model) = model {
                let cv = model.specific_heat_cv()?;
                if (fields.materials[i].specific_heat - cv).abs() > 64.0 * f64::EPSILON * cv {
                    return Err(Error::InvalidTransport);
                }
                if fields.phase.as_ref().is_some_and(|p| p.models[i].is_some()) {
                    return Err(Error::InvalidPhaseChange);
                }
            }
        }
        let mut candidate = self.clone();
        candidate.gas_equations = models;
        candidate.validate_gas_geometry()?;
        if candidate.gas_equations.iter().any(Option::is_some) {
            candidate.pressure_work = true;
            candidate.viscous_heating = true;
        }
        candidate.effective_materials()?;
        *self = candidate;
        Ok(())
    }
    pub(super) fn validate_gas_geometry(&self) -> Result<(), Error> {
        if self.gas_equations.iter().any(Option::is_some) {
            if self.formulation != Formulation::MassDensity {
                return Err(Error::InvalidConfig);
            }
            if !self.boundaries.is_empty() || self.boundary_coupling.is_some() {
                return Err(Error::InvalidBoundary);
            }
        }
        Ok(())
    }
    pub(super) fn apply_gas_properties(
        &self,
        particles: &[Particle],
        properties: &mut [Material],
        fields: Option<&Transport>,
    ) -> Result<(), Error> {
        for (i, (particle, property)) in particles.iter().zip(properties.iter_mut()).enumerate() {
            if let Some(model) = self.gas_equations[particle.material] {
                let fields = fields.ok_or(Error::InvalidTransport)?;
                let cv = model.specific_heat_cv()?;
                if (fields.materials[particle.material].specific_heat - cv).abs()
                    > 64.0 * f64::EPSILON * cv
                {
                    return Err(Error::InvalidTransport);
                }
                let temperature = fields.fields[i].temperature;
                let squared = model.heat_capacity_ratio * model.gas_constant * temperature;
                if !positive(temperature) || !positive(squared) {
                    return Err(Error::NumericalFailure);
                }
                property.sound_speed = squared.sqrt();
            }
        }
        if self.gas_equations.iter().any(Option::is_some) {
            self.validate_gas_geometry()?;
            let neighbors = super::pairs(
                particles,
                self.config.smoothing_radius,
                self.config.max_pairs,
                self.config.max_neighbor_checks,
            )?;
            let mut densities = self.particle_densities(particles, &neighbors, properties);
            let images = self.image_pairs(particles, neighbors.len())?;
            self.image_density(particles, properties, &images, &mut densities);
            for (i, particle) in particles.iter().enumerate() {
                if self.gas_equations[particle.material].is_some() {
                    if !positive(densities[i]) {
                        return Err(Error::NumericalFailure);
                    }
                    // Transport volume and kinematic viscosity use current gas density.
                    properties[i].rest_density = densities[i];
                }
            }
        }
        Ok(())
    }
    pub(super) fn thermodynamic_pressure_magnitude(
        &self,
        mass: [f64; 2],
        density: [f64; 2],
        pressure: [f64; 2],
        reference: [f64; 2],
        geometry: [f64; 2],
        materials: [usize; 2],
    ) -> f64 {
        let [support, radius] = geometry;
        if self.formulation == Formulation::MassDensity
            && materials.iter().any(|i| self.gas_equations[*i].is_some())
        {
            mass[0]
                * mass[1]
                * (pressure[0] / density[0].powi(2) + pressure[1] / density[1].powi(2))
                * super::density_kernel_gradient(self.formulation, support, radius)
        } else {
            super::pressure_magnitude(
                mass,
                density,
                pressure,
                reference,
                support,
                radius,
                self.formulation,
            )
        }
    }
    pub(super) fn material_pressure(
        &self,
        index: usize,
        material: Material,
        density: f64,
    ) -> Result<f64, Error> {
        let pressure = if let Some(model) = self.gas_equations[index] {
            density * (material.sound_speed.powi(2) / model.heat_capacity_ratio)
        } else {
            material.sound_speed.powi(2) * (density - material.rest_density).max(0.0)
        };
        if !pressure.is_finite() || pressure < 0.0 {
            return Err(Error::NumericalFailure);
        }
        Ok(pressure)
    }
}

// Each record represents the work associated with one density owner's pressure.
// Its force also acts on the neighbor (or reflected owner), so energy ownership
// differs from the particle receiving the impulse at a wall.
struct PressureWorkConnection {
    owner: usize,
    first: usize,
    second: usize,
    force: [f64; 3],
    sign: [f64; 3],
}
impl Liquid {
    pub(super) fn exchange_gas_pressure_work(
        &self,
        particles: &mut [Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
        density: &[f64],
        fields: &mut Transport,
        dt: f64,
    ) -> Result<Vec<[f64; 3]>, Error> {
        let connections = self.pressure_connections(particles, pairs, properties, density)?;
        let acceleration = Self::pressure_accelerations_from(particles, &connections)?;
        let mut work = vec![0.0; particles.len()];
        for connection in connections {
            let i = connection.first;
            let j = connection.second;
            for (axis, force) in connection.force.iter().enumerate() {
                let first = particles[i].velocity[axis] + 0.5 * dt * acceleration[i][axis];
                let second = particles[j].velocity[axis] + 0.5 * dt * acceleration[j][axis];
                work[connection.owner] += dt * *force * (first - connection.sign[axis] * second);
            }
        }
        for (index, (particle, a)) in particles.iter_mut().zip(&acceleration).enumerate() {
            let energy = fields.energy(particle, &fields.fields[index], index)?;
            fields.set_energy(index, particle, energy - work[index])?;
            for (velocity, acceleration) in particle.velocity.iter_mut().zip(a) {
                *velocity += dt * acceleration;
            }
            if !super::finite(particle.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(acceleration)
    }
    fn pressure_connections(
        &self,
        particles: &[Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
        density: &[f64],
    ) -> Result<Vec<PressureWorkConnection>, Error> {
        self.validate_gas_geometry()?;
        let mut connections = Vec::new();
        let h = self.config.smoothing_radius;
        for &(i, j) in pairs {
            let delta = super::sub(particles[i].position, particles[j].position);
            let radius = super::norm(delta);
            let direction = if radius > 1e-12 {
                delta.map(|v| v / radius)
            } else {
                [1.0, 0.0, 0.0]
            };
            let pressure = [
                self.material_pressure(particles[i].material, properties[i], density[i])?,
                self.material_pressure(particles[j].material, properties[j], density[j])?,
            ];
            let magnitude = self.thermodynamic_pressure_magnitude(
                [particles[i].mass, particles[j].mass],
                [density[i], density[j]],
                pressure,
                [properties[i].rest_density, properties[j].rest_density],
                [h, radius],
                [particles[i].material, particles[j].material],
            );
            let weights = [
                pressure[0] / density[i].powi(2),
                pressure[1] / density[j].powi(2),
            ];
            let sum = weights[0] + weights[1];
            let first_magnitude = if sum > 0.0 {
                magnitude * (weights[0] / sum)
            } else {
                0.0
            };
            for (owner, value) in [(i, first_magnitude), (j, magnitude - first_magnitude)] {
                connections.push(PressureWorkConnection {
                    owner,
                    first: i,
                    second: j,
                    force: direction.map(|v| value * v),
                    sign: [1.0; 3],
                });
            }
        }
        let images = self.image_pairs(particles, pairs.len())?;
        for pair in images {
            if pair.radius == 0.0 {
                continue;
            }
            let i = pair.particle;
            let j = pair.image.owner;
            let pressure =
                self.material_pressure(particles[i].material, properties[i], density[i])?;
            let magnitude = particles[i].mass * particles[j].mass * pressure / density[i].powi(2)
                * super::density_kernel_gradient(self.formulation, h, pair.radius);
            let delta = super::sub(particles[i].position, pair.image.position);
            connections.push(PressureWorkConnection {
                owner: i,
                first: i,
                second: j,
                force: delta.map(|v| magnitude * v / pair.radius),
                sign: pair.image.sign,
            });
        }
        Ok(connections)
    }
    fn pressure_accelerations_from(
        particles: &[Particle],
        connections: &[PressureWorkConnection],
    ) -> Result<Vec<[f64; 3]>, Error> {
        let mut acceleration = vec![[0.0; 3]; particles.len()];
        for connection in connections {
            for (axis, force) in connection.force.iter().enumerate() {
                acceleration[connection.first][axis] += *force / particles[connection.first].mass;
                acceleration[connection.second][axis] -=
                    connection.sign[axis] * *force / particles[connection.second].mass;
            }
        }
        if acceleration.iter().any(|v| !super::finite(*v)) {
            return Err(Error::NumericalFailure);
        }
        Ok(acceleration)
    }
    pub(super) fn gas_pressure_accelerations(
        &self,
        particles: &[Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
        density: &[f64],
    ) -> Result<Vec<[f64; 3]>, Error> {
        let connections = self.pressure_connections(particles, pairs, properties, density)?;
        Self::pressure_accelerations_from(particles, &connections)
    }
}
