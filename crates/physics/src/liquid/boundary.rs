use super::{Error, Liquid, Material, Particle, finite, norm, pairs, positive, sub};
use std::f64::consts::PI;
/// Static solid-volume quadrature sample. Samples must fill the wall's kernel support.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundarySample {
    pub position: [f64; 3],
    pub volume: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryDiagnostics {
    pub particle_densities: Vec<f64>,
    /// Pressure reaction force on each fixed sample, in configured sample order.
    pub reaction_forces: Vec<[f64; 3]>,
    pub viscous_reaction_forces: Vec<[f64; 3]>,
}
impl Liquid {
    /// Replaces static SPH boundary volume samples. Does not create collision geometry.
    /// # Errors
    /// Nonfinite positions, nonpositive volumes or more than `Config::max_particles` samples.
    pub fn configure_boundaries(&mut self, samples: Vec<BoundarySample>) -> Result<(), Error> {
        if (self.reflecting_box.is_some()
            || self.viscous_integrator != super::ViscousIntegrator::Sequential)
            && !samples.is_empty()
        {
            return Err(Error::InvalidBoundary);
        }
        if samples.len() > self.config.max_particles {
            return Err(Error::BoundaryBudget);
        }
        if samples
            .iter()
            .any(|sample| !finite(sample.position) || !positive(sample.volume))
        {
            return Err(Error::InvalidBoundary);
        }
        let mut grid = std::collections::BTreeMap::<_, Vec<usize>>::new();
        for (index, sample) in samples.iter().enumerate() {
            grid.entry(super::cell(sample.position, self.config.smoothing_radius)?)
                .or_default()
                .push(index);
        }
        self.boundary_velocities = vec![[0.0; 3]; samples.len()];
        self.boundaries = samples;
        self.boundary_grid = grid;
        Ok(())
    }
    /// Evaluates density support and pressure reaction without advancing time.
    /// # Errors
    /// Neighbor/pair/boundary budget excess, invalid field responses or numerical overflow.
    pub fn boundary_diagnostics(&self) -> Result<BoundaryDiagnostics, Error> {
        let properties = self.effective_materials()?;
        let neighbors = pairs(
            &self.particles,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )?;
        let (density, _, _) = self.forces(&self.particles, &neighbors, &properties)?;
        let boundary_pairs = self.boundary_pairs(&self.particles, neighbors.len())?;
        let mut acceleration = vec![[0.0; 3]; self.particles.len()];
        let reactions = self.add_boundary_pressure(
            &self.particles,
            &properties,
            &density,
            &boundary_pairs,
            &mut acceleration,
        )?;
        let viscous_reaction_forces = self.boundary_viscous_reactions(
            &self.particles,
            &properties,
            &density,
            &boundary_pairs,
        )?;
        Ok(BoundaryDiagnostics {
            particle_densities: density,
            reaction_forces: reactions,
            viscous_reaction_forces,
        })
    }
    pub(super) fn boundary_pairs(
        &self,
        particles: &[Particle],
        fluid_pairs: usize,
    ) -> Result<Vec<(usize, usize, f64)>, Error> {
        let mut result = Vec::new();
        if self.boundaries.is_empty() {
            return Ok(result);
        }
        let mut checks = 0usize;
        for (i, particle) in particles.iter().enumerate() {
            let center = super::cell(particle.position, self.config.smoothing_radius)?;
            let begin = result.len();
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        if let Some(indices) =
                            self.boundary_grid
                                .get(&[center[0] + x, center[1] + y, center[2] + z])
                        {
                            for &j in indices {
                                if checks >= self.config.max_neighbor_checks {
                                    return Err(Error::BoundaryBudget);
                                }
                                checks += 1;
                                let radius =
                                    norm(sub(particle.position, self.boundaries[j].position));
                                if radius < self.config.smoothing_radius {
                                    if result.len() >= self.config.max_pairs - fluid_pairs {
                                        return Err(Error::PairBudget);
                                    }
                                    result.push((i, j, radius));
                                }
                            }
                        }
                    }
                }
            }
            // Preserve configured sample order and the previous floating-point summation order.
            result[begin..].sort_unstable_by_key(|pair| pair.1);
        }
        Ok(result)
    }
    pub(super) fn add_boundary_density(
        &self,
        properties: &[Material],
        neighbors: &[(usize, usize, f64)],
        density: &mut [f64],
    ) {
        let support = self.config.smoothing_radius;
        for &(i, j, radius) in neighbors {
            density[i] += properties[i].rest_density
                * self.boundaries[j].volume
                * super::density_kernel(self.formulation, support, radius);
        }
    }
    pub(super) fn add_boundary_pressure(
        &self,
        particles: &[Particle],
        properties: &[Material],
        density: &[f64],
        neighbors: &[(usize, usize, f64)],
        acceleration: &mut [[f64; 3]],
    ) -> Result<Vec<[f64; 3]>, Error> {
        let support = self.config.smoothing_radius;
        let extrapolated = if self.static_pressure_extrapolation {
            Some(self.extrapolated_wall_pressure(particles, properties, density, neighbors)?)
        } else {
            None
        };
        let mut reactions = vec![[0.0; 3]; self.boundaries.len()];
        for &(i, j, radius) in neighbors {
            if radius <= 1e-12 {
                continue;
            }
            let delta = sub(particles[i].position, self.boundaries[j].position);
            let pressure = properties[i].sound_speed.powi(2)
                * (density[i] - properties[i].rest_density).max(0.0);
            let force = if let Some(walls) = &extrapolated {
                let (wall_pressure, wall_density) = walls[&(j, particles[i].material)];
                let gradient = if self.formulation == super::Formulation::MassDensity {
                    45.0 / (PI * support.powi(6)) * (support - radius).powi(2)
                } else {
                    super::density_kernel_gradient(self.formulation, support, radius)
                };
                particles[i].mass
                    * properties[i].rest_density
                    * self.boundaries[j].volume
                    * (pressure / density[i].powi(2) + wall_pressure / wall_density.powi(2))
                    * gradient
            } else {
                let gradient = match self.formulation {
                    // Retain the mirrored fluid neighbor used by the default formulation.
                    super::Formulation::MassDensity => {
                        2.0 * 45.0 / (PI * support.powi(6)) * (support - radius).powi(2)
                    }
                    // Only the fluid carries compression energy; solid volume is fixed.
                    super::Formulation::RestVolume | super::Formulation::RestVolumeWendland => {
                        super::density_kernel_gradient(self.formulation, support, radius)
                    }
                };
                particles[i].mass
                    * properties[i].rest_density
                    * self.boundaries[j].volume
                    * pressure
                    / density[i].powi(2)
                    * gradient
            };
            for (axis, component) in delta.into_iter().enumerate() {
                let value = force * component / radius;
                acceleration[i][axis] += value / particles[i].mass;
                reactions[j][axis] -= value;
            }
        }
        if acceleration
            .iter()
            .chain(&reactions)
            .any(|value| !finite(*value))
        {
            return Err(Error::NumericalFailure);
        }
        Ok(reactions)
    }
}
impl BoundarySample {
    /// Fills a solid box with midpoint volume samples; edge cells use their actual clipped volume.
    /// # Errors
    /// Invalid bounds/spacing, sample-budget excess or unrepresentable cell coordinates/volumes.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn box_grid(
        min: [f64; 3],
        max: [f64; 3],
        spacing: f64,
        max_samples: usize,
    ) -> Result<Vec<Self>, Error> {
        if !finite(min)
            || !finite(max)
            || !positive(spacing)
            || (0..3).any(|axis| !positive(max[axis] - min[axis]))
        {
            return Err(Error::InvalidBoundary);
        }
        let counts =
            std::array::from_fn::<_, 3, _>(|axis| ((max[axis] - min[axis]) / spacing).ceil());
        if counts
            .iter()
            .any(|count| !count.is_finite() || *count > max_samples as f64)
        {
            return Err(Error::BoundaryBudget);
        }
        let dims = counts.map(|count| count as usize);
        let count = dims
            .into_iter()
            .try_fold(1usize, usize::checked_mul)
            .ok_or(Error::BoundaryBudget)?;
        if count > max_samples {
            return Err(Error::BoundaryBudget);
        }
        let mut result = Vec::with_capacity(count);
        for x in 0..dims[0] {
            for y in 0..dims[1] {
                for z in 0..dims[2] {
                    let indices = [x, y, z];
                    let mut position = [0.0; 3];
                    let mut volume = 1.0;
                    for axis in 0..3 {
                        let lower = min[axis] + indices[axis] as f64 * spacing;
                        let upper = (lower + spacing).min(max[axis]);
                        let width = upper - lower;
                        position[axis] = lower + 0.5 * width;
                        volume *= width;
                    }
                    if !finite(position) || !positive(volume) {
                        return Err(Error::NumericalFailure);
                    }
                    result.push(Self { position, volume });
                }
            }
        }
        Ok(result)
    }
}
impl Liquid {
    pub(super) fn exchange_boundary_work(
        &self,
        particles: &mut [Particle],
        properties: &[Material],
        density: &[f64],
        fluid_pairs: usize,
        fields: &mut super::transport::Transport,
        dt: f64,
    ) -> Result<Vec<[f64; 3]>, Error> {
        let neighbors = self.boundary_pairs(particles, fluid_pairs)?;
        let mut acceleration = vec![[0.0; 3]; particles.len()];
        self.add_boundary_pressure(
            particles,
            properties,
            density,
            &neighbors,
            &mut acceleration,
        )?;
        for (index, (particle, force)) in particles.iter_mut().zip(&acceleration).enumerate() {
            if force.iter().all(|component| *component == 0.0) {
                continue;
            }
            let impulse = force.map(|component| particle.mass * component * dt);
            let work: f64 = impulse
                .iter()
                .zip(particle.velocity)
                .map(|(component, speed)| component * (speed + 0.5 * component / particle.mass))
                .sum();
            if !work.is_finite() {
                return Err(Error::NumericalFailure);
            }
            let energy = fields.energy(particle, &fields.fields[index], index)?;
            fields.set_energy(index, particle, energy - work)?;
            for (axis, component) in impulse.into_iter().enumerate() {
                particle.velocity[axis] += component / particle.mass;
            }
            if !finite(particle.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(acceleration)
    }
}
impl Liquid {
    pub(super) fn boundary_viscous_rates(
        &self,
        properties: &[Material],
        density: &[f64],
        neighbors: &[(usize, usize, f64)],
    ) -> Result<Vec<f64>, Error> {
        let mut rates = vec![0.0; density.len()];
        let support = self.config.smoothing_radius;
        let kernel = 45.0 / (PI * support.powi(6));
        for &(i, j, radius) in neighbors {
            rates[i] += properties[i].viscosity
                * self.boundaries[j].volume
                * kernel
                * (support - radius).max(0.0)
                / density[i];
        }
        if rates.iter().any(|rate| !rate.is_finite() || *rate < 0.0) {
            return Err(Error::NumericalFailure);
        }
        Ok(rates)
    }
    pub(super) fn dissipate_boundary_viscosity(
        &self,
        particles: &mut [Particle],
        fluid_pairs: usize,
        properties: &[Material],
        density: &[f64],
        fields: &mut super::transport::Transport,
        dt: f64,
    ) -> Result<(), Error> {
        let neighbors = self.boundary_pairs(particles, fluid_pairs)?;
        let motion = self.boundary_viscous_motion(properties, density, &neighbors)?;
        for (index, (particle, rate)) in particles.iter_mut().zip(motion.rates).enumerate() {
            if rate == 0.0 {
                continue;
            }
            let relative = sub(particle.velocity, motion.targets[index]);
            let fraction = -(-rate * dt).exp_m1();
            let heat = 0.5
                * particle.mass
                * relative.iter().map(|speed| speed * speed).sum::<f64>()
                * fraction
                * (2.0 - fraction)
                + particle.mass * motion.variance[index] * dt;
            if !heat.is_finite() {
                return Err(Error::NumericalFailure);
            }
            let energy = fields.energy(particle, &fields.fields[index], index)?;
            fields.set_energy(index, particle, energy + heat)?;
            particle.velocity = std::array::from_fn(|axis| {
                motion.targets[index][axis] + relative[axis] * (1.0 - fraction)
            });
            if !finite(particle.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(())
    }
}

impl Liquid {
    pub(super) fn add_boundary_forces(
        &self,
        particles: &[Particle],
        properties: &[Material],
        density: &[f64],
        neighbors: &[(usize, usize, f64)],
        acceleration: &mut [[f64; 3]],
        rates: &mut [f64],
    ) -> Result<(), Error> {
        self.add_boundary_pressure(particles, properties, density, neighbors, acceleration)?;
        let motion = self.boundary_viscous_motion(properties, density, neighbors)?;
        for (i, rate) in motion.rates.into_iter().enumerate() {
            rates[i] += rate;
            if !self.viscous_heating {
                for (axis, (component, speed)) in acceleration[i]
                    .iter_mut()
                    .zip(particles[i].velocity)
                    .enumerate()
                {
                    *component += rate * (motion.targets[i][axis] - speed);
                }
            }
        }
        Ok(())
    }
    fn boundary_viscous_reactions(
        &self,
        particles: &[Particle],
        properties: &[Material],
        density: &[f64],
        neighbors: &[(usize, usize, f64)],
    ) -> Result<Vec<[f64; 3]>, Error> {
        let support = self.config.smoothing_radius;
        let kernel = 45.0 / (PI * support.powi(6));
        let mut reactions = vec![[0.0; 3]; self.boundaries.len()];
        for &(i, j, radius) in neighbors {
            let rate = properties[i].viscosity
                * self.boundaries[j].volume
                * kernel
                * (support - radius).max(0.0)
                / density[i];
            for (component, speed) in reactions[j]
                .iter_mut()
                .zip(sub(particles[i].velocity, self.boundary_velocities[j]))
            {
                *component += particles[i].mass * rate * speed;
            }
        }
        if reactions.iter().any(|reaction| !finite(*reaction)) {
            return Err(Error::NumericalFailure);
        }
        Ok(reactions)
    }
}

struct WallMotion {
    rates: Vec<f64>,
    targets: Vec<[f64; 3]>,
    variance: Vec<f64>,
}
impl Liquid {
    /// Sets prescribed surface velocities for viscous exchange. Sample positions stay fixed;
    /// this supports tangential wall motion, not normal motion of collision geometry.
    /// Replacing geometry resets these velocities to zero.
    /// # Errors
    /// Wrong count or nonfinite velocity. Failure leaves all velocities unchanged.
    pub fn configure_boundary_velocities(
        &mut self,
        velocities: Vec<[f64; 3]>,
    ) -> Result<(), Error> {
        if velocities.len() != self.boundaries.len()
            || velocities.iter().any(|velocity| !finite(*velocity))
            || (self.static_pressure_extrapolation
                && velocities.iter().any(|v| v.iter().any(|x| *x != 0.0)))
        {
            return Err(Error::InvalidBoundary);
        }
        self.boundary_velocities = velocities;
        Ok(())
    }
    fn boundary_viscous_motion(
        &self,
        properties: &[Material],
        density: &[f64],
        neighbors: &[(usize, usize, f64)],
    ) -> Result<WallMotion, Error> {
        let rates = self.boundary_viscous_rates(properties, density, neighbors)?;
        let mut targets = vec![[0.0; 3]; density.len()];
        let mut variance = vec![0.0; density.len()];
        let support = self.config.smoothing_radius;
        let kernel = 45.0 / (PI * support.powi(6));
        for &(i, j, radius) in neighbors {
            let rate = properties[i].viscosity
                * self.boundaries[j].volume
                * kernel
                * (support - radius).max(0.0)
                / density[i];
            if rates[i] > 0.0 {
                for (target, velocity) in targets[i].iter_mut().zip(self.boundary_velocities[j]) {
                    *target += rate / rates[i] * velocity;
                }
            }
        }
        for &(i, j, radius) in neighbors {
            let rate = properties[i].viscosity
                * self.boundaries[j].volume
                * kernel
                * (support - radius).max(0.0)
                / density[i];
            variance[i] += rate
                * sub(self.boundary_velocities[j], targets[i])
                    .iter()
                    .map(|speed| speed * speed)
                    .sum::<f64>();
        }
        if targets.iter().any(|target| !finite(*target))
            || variance.iter().any(|value| !value.is_finite())
        {
            return Err(Error::NumericalFailure);
        }
        Ok(WallMotion {
            rates,
            targets,
            variance,
        })
    }
}

impl Liquid {
    pub(super) fn boundary_surface_speed(
        &self,
        particles: &[Particle],
        properties: &[Material],
        fluid_pairs: usize,
    ) -> Result<f64, Error> {
        let neighbors = self.boundary_pairs(particles, fluid_pairs)?;
        let mut maximum = 0.0_f64;
        for (i, j, _) in neighbors {
            if properties[i].viscosity > 0.0 {
                maximum = maximum.max(norm(self.boundary_velocities[j]));
            }
        }
        if !maximum.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(maximum)
    }
}
impl Liquid {
    /// Translates all SPH volume samples and rebuilds their spatial index atomically.
    /// Preserves volumes and prescribed surface velocities. Particle positions and collision
    /// geometry are unchanged; callers must coordinate collision movement separately.
    /// This is a geometry edit, not integration or a swept contact operation.
    /// # Errors
    /// Nonfinite displacement, overflowing positions or unsupported index coordinates.
    pub fn translate_boundaries(&mut self, displacement: [f64; 3]) -> Result<(), Error> {
        if !finite(displacement) {
            return Err(Error::InvalidBoundary);
        }
        let translated = self
            .boundaries
            .iter()
            .map(|sample| BoundarySample {
                position: std::array::from_fn(|axis| sample.position[axis] + displacement[axis]),
                volume: sample.volume,
            })
            .collect();
        let velocities = self.boundary_velocities.clone();
        self.configure_boundaries(translated)?;
        self.boundary_velocities = velocities;
        Ok(())
    }
}

impl Liquid {
    /// Current lab-space volume quadrature, including movement of attached bodies.
    #[must_use]
    pub fn boundary_samples(&self) -> &[BoundarySample] {
        &self.boundaries
    }
}
