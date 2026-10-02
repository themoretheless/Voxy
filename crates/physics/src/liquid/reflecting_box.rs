//! Conservative pressure images for a stationary axis-aligned box.
use super::{
    Error, Formulation, Liquid, Material, Particle, cell, density_kernel, density_kernel_gradient,
    finite, norm, pairs, positive, sub,
};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum WallViscosity {
    FreeSlip,
    NoSlip,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReflectingBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
}
#[derive(Clone, Debug, PartialEq)]
pub struct ReflectingDiagnostics {
    pub pressure_accelerations: Vec<[f64; 3]>,
    /// Lower/upper x, lower/upper y, lower/upper z pressure reactions.
    pub reaction_forces: [[f64; 3]; 6],
    pub reaction_torque_about_origin: [f64; 3],
}
/// Instantaneous viscous load on the entire stationary fixture.
/// Edge/corner image loads are not assigned to individual faces.
#[derive(Clone, Debug, PartialEq)]
pub struct ReflectingViscousDiagnostics {
    pub accelerations: Vec<[f64; 3]>,
    pub reaction_force: [f64; 3],
    pub reaction_torque_about_origin: [f64; 3],
    /// Nonnegative rate of conversion from kinetic energy into heat.
    pub dissipated_power: f64,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Image {
    pub(super) owner: usize,
    pub(super) position: [f64; 3],
    pub(super) sign: [f64; 3],
    faces: [Option<usize>; 3],
}
#[derive(Clone, Copy, Debug)]
pub(super) struct ImagePair {
    pub(super) particle: usize,
    pub(super) image: Image,
    pub(super) radius: f64,
}
impl Liquid {
    #[must_use]
    pub fn reflecting_box(&self) -> Option<ReflectingBox> {
        self.reflecting_box
    }
    /// Enables odd velocity images at stationary walls (zero wall velocity).
    /// This dissipative no-slip stencil is optional and requires a reflecting box.
    /// With viscous heating, all dissipated energy is deposited in fluid enthalpy;
    /// the stationary fixture receives momentum but performs no work.
    /// # Errors
    /// Enabling without a reflecting box leaves the setting unchanged.
    pub fn set_reflecting_no_slip(&mut self, enabled: bool) -> Result<(), Error> {
        if enabled && self.reflecting_box.is_none() {
            return Err(Error::InvalidBoundary);
        }
        self.reflecting_no_slip = if enabled {
            WallViscosity::NoSlip
        } else {
            WallViscosity::FreeSlip
        };
        Ok(())
    }
    /// Configures conservative pressure reflections, including edge and corner images.
    /// This is a stationary pressure boundary, not collision geometry. Use a matching
    /// Container to prevent crossing. Optional odd velocity images provide viscous wall coupling; heat transport uses real pairs.
    /// # Errors
    /// Requires a volume formulation or gas `MassDensity`, no sampled walls/body coupling/extrapolation,
    /// finite ordered bounds, support smaller than each box width, and particles inside.
    pub fn set_reflecting_box(&mut self, bounds: Option<ReflectingBox>) -> Result<(), Error> {
        if let Some(b) = bounds
            && ((self.formulation == Formulation::MassDensity && !self.gas_active())
                || !self.boundaries.is_empty()
                || self.static_pressure_extrapolation
                || self.boundary_coupling.is_some()
                || !finite(b.min)
                || !finite(b.max)
                || (0..3).any(|a| {
                    !positive(b.max[a] - b.min[a])
                        || self.config.smoothing_radius >= b.max[a] - b.min[a]
                })
                || self
                    .particles
                    .iter()
                    .any(|p| (0..3).any(|a| p.position[a] < b.min[a] || p.position[a] > b.max[a])))
        {
            return Err(Error::InvalidBoundary);
        }
        self.reflecting_box = bounds;
        if bounds.is_none() {
            self.reflecting_no_slip = WallViscosity::FreeSlip;
        }
        Ok(())
    }
    /// Evaluates the reflected pressure and total fixture reactions without advancing.
    /// # Errors
    /// Requires a reflecting box; usual property, geometry and neighbor budgets apply.
    pub fn reflecting_diagnostics(&self) -> Result<ReflectingDiagnostics, Error> {
        if self.reflecting_box.is_none() {
            return Err(Error::InvalidBoundary);
        }
        let properties = self.effective_materials()?;
        let real = pairs(
            &self.particles,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )?;
        let density = self.forces(&self.particles, &real, &properties)?.0;
        let images = self.image_pairs(&self.particles, real.len())?;
        self.image_pressure(&self.particles, &properties, &density, &images)
    }
    /// Evaluates odd-image viscous forces independently of the integration mode.
    /// Disabled no-slip coupling returns zero loads. This is an instantaneous rate,
    /// not the integrated impulse of sequential exact thermal relaxation.
    /// # Errors
    /// Requires a reflecting box and valid properties, geometry and neighbor budgets.
    pub fn reflecting_viscous_diagnostics(&self) -> Result<ReflectingViscousDiagnostics, Error> {
        if self.reflecting_box.is_none() {
            return Err(Error::InvalidBoundary);
        }
        let properties = self.effective_materials()?;
        let real = pairs(
            &self.particles,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )?;
        let density = self.forces(&self.particles, &real, &properties)?.0;
        let images = self.image_pairs(&self.particles, real.len())?;
        let mut result = ReflectingViscousDiagnostics {
            accelerations: vec![[0.0; 3]; self.particles.len()],
            reaction_force: [0.0; 3],
            reaction_torque_about_origin: [0.0; 3],
            dissipated_power: 0.0,
        };
        if self.reflecting_no_slip == WallViscosity::FreeSlip {
            return Ok(result);
        }
        for pair in &images {
            let first = pair.particle;
            let second = pair.image.owner;
            let (conductance, parity) =
                self.image_drag(&self.particles, &properties, &density, pair);
            let (direction, speed) = Self::image_velocity(&self.particles, pair, parity);
            result.dissipated_power += conductance * speed * speed;
            for (axis, normal) in direction.into_iter().enumerate() {
                let force = -conductance * speed * normal;
                result.accelerations[first][axis] += force / self.particles[first].mass;
                result.accelerations[second][axis] -= parity * force / self.particles[second].mass;
            }
        }
        for (particle, acceleration) in self.particles.iter().zip(&result.accelerations) {
            let force = acceleration.map(|value| value * particle.mass);
            for axis in 0..3 {
                let next = (axis + 1) % 3;
                let last = (axis + 2) % 3;
                result.reaction_force[axis] -= force[axis];
                result.reaction_torque_about_origin[axis] -=
                    particle.position[next] * force[last] - particle.position[last] * force[next];
            }
        }
        if !result.dissipated_power.is_finite()
            || result.dissipated_power < 0.0
            || !finite(result.reaction_force)
            || !finite(result.reaction_torque_about_origin)
            || result.accelerations.iter().any(|a| !finite(*a))
        {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }
    pub(super) fn validate_reflecting_positions(
        &self,
        particles: &[Particle],
    ) -> Result<(), Error> {
        if let Some(bounds) = self.reflecting_box
            && particles.iter().any(|p| {
                (0..3).any(|a| p.position[a] < bounds.min[a] || p.position[a] > bounds.max[a])
            })
        {
            return Err(Error::InvalidBoundary);
        }
        Ok(())
    }
    pub(super) fn image_pairs(
        &self,
        particles: &[Particle],
        real_pairs: usize,
    ) -> Result<Vec<ImagePair>, Error> {
        let Some(bounds) = self.reflecting_box else {
            return Ok(Vec::new());
        };
        if self.formulation == Formulation::MassDensity && !self.gas_active() {
            return Err(Error::InvalidBoundary);
        }
        let h = self.config.smoothing_radius;
        let mut images = Vec::new();
        let mut grid = BTreeMap::<[i64; 3], Vec<usize>>::new();
        let mut checks = 0usize;
        for (owner, particle) in particles.iter().enumerate() {
            if (0..3).any(|a| {
                particle.position[a] < bounds.min[a] || particle.position[a] > bounds.max[a]
            }) {
                return Err(Error::InvalidBoundary);
            }
            let choices: [Vec<Option<usize>>; 3] = std::array::from_fn(|a| {
                let mut options = vec![None];
                if particle.position[a] - bounds.min[a] < h {
                    options.push(Some(2 * a));
                }
                if bounds.max[a] - particle.position[a] < h {
                    options.push(Some(2 * a + 1));
                }
                options
            });
            for &x in &choices[0] {
                for &y in &choices[1] {
                    for &z in &choices[2] {
                        let faces = [x, y, z];
                        if faces.iter().all(Option::is_none) {
                            continue;
                        }
                        if checks >= self.config.max_neighbor_checks {
                            return Err(Error::NeighborBudget);
                        }
                        checks += 1;
                        let mut image = Image {
                            owner,
                            position: particle.position,
                            sign: [1.0; 3],
                            faces,
                        };
                        for (a, face) in faces.into_iter().enumerate() {
                            if let Some(face) = face {
                                let wall = if face % 2 == 0 {
                                    bounds.min[a]
                                } else {
                                    bounds.max[a]
                                };
                                image.position[a] = 2.0 * wall - particle.position[a];
                                image.sign[a] = -1.0;
                            }
                        }
                        grid.entry(cell(image.position, h)?)
                            .or_default()
                            .push(images.len());
                        images.push(image);
                    }
                }
            }
        }
        let remaining = self
            .config
            .max_pairs
            .checked_sub(real_pairs)
            .ok_or(Error::PairBudget)?;
        let mut result = Vec::new();
        for (i, particle) in particles.iter().enumerate() {
            let center = cell(particle.position, h)?;
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        if let Some(indices) =
                            grid.get(&[center[0] + x, center[1] + y, center[2] + z])
                        {
                            for &j in indices {
                                if checks >= self.config.max_neighbor_checks {
                                    return Err(Error::NeighborBudget);
                                }
                                checks += 1;
                                let image = images[j];
                                let radius = norm(sub(particle.position, image.position));
                                if radius < h {
                                    if result.len() >= remaining {
                                        return Err(Error::PairBudget);
                                    }
                                    result.push(ImagePair {
                                        particle: i,
                                        image,
                                        radius,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(result)
    }
    pub(super) fn image_density(
        &self,
        particles: &[Particle],
        properties: &[Material],
        images: &[ImagePair],
        density: &mut [f64],
    ) {
        for pair in images {
            let i = pair.particle;
            let j = pair.image.owner;
            let ratio = if self.formulation == Formulation::MassDensity {
                1.0
            } else {
                properties[i].rest_density / properties[j].rest_density
            };
            density[i] += ratio
                * particles[j].mass
                * density_kernel(self.formulation, self.config.smoothing_radius, pair.radius);
        }
    }
    pub(super) fn add_image_pressure(
        &self,
        particles: &[Particle],
        properties: &[Material],
        density: &[f64],
        images: &[ImagePair],
        acceleration: &mut [[f64; 3]],
    ) -> Result<(), Error> {
        if images.is_empty() {
            return Ok(());
        }
        let reflected = self.image_pressure(particles, properties, density, images)?;
        for (a, image) in acceleration
            .iter_mut()
            .zip(reflected.pressure_accelerations)
        {
            for axis in 0..3 {
                a[axis] += image[axis];
            }
        }
        Ok(())
    }
    pub(super) fn image_pressure(
        &self,
        particles: &[Particle],
        properties: &[Material],
        density: &[f64],
        images: &[ImagePair],
    ) -> Result<ReflectingDiagnostics, Error> {
        let mut result = ReflectingDiagnostics {
            pressure_accelerations: vec![[0.0; 3]; particles.len()],
            reaction_forces: [[0.0; 3]; 6],
            reaction_torque_about_origin: [0.0; 3],
        };
        for pair in images {
            let i = pair.particle;
            let j = pair.image.owner;
            if pair.radius == 0.0 {
                continue;
            }
            let delta = sub(particles[i].position, pair.image.position);
            let pressure =
                self.material_pressure(particles[i].material, properties[i], density[i])?;
            let ratio = if self.formulation == Formulation::MassDensity {
                1.0
            } else {
                properties[i].rest_density / properties[j].rest_density
            };
            let magnitude = particles[i].mass * particles[j].mass * ratio * pressure
                / density[i].powi(2)
                * density_kernel_gradient(
                    self.formulation,
                    self.config.smoothing_radius,
                    pair.radius,
                );
            for (a, d) in delta.into_iter().enumerate() {
                let force = magnitude * d / pair.radius;
                // The image follows its owner's position: differentiate both endpoints.
                result.pressure_accelerations[i][a] += force / particles[i].mass;
                result.pressure_accelerations[j][a] -=
                    pair.image.sign[a] * force / particles[j].mass;
                if let Some(face) = pair.image.faces[a] {
                    result.reaction_forces[face][a] -= 2.0 * force;
                }
            }
        }
        for (p, a) in particles.iter().zip(&result.pressure_accelerations) {
            for axis in 0..3 {
                let b = (axis + 1) % 3;
                let c = (axis + 2) % 3;
                result.reaction_torque_about_origin[axis] -=
                    p.mass * (p.position[b] * a[c] - p.position[c] * a[b]);
            }
        }
        if result
            .pressure_accelerations
            .iter()
            .chain(&result.reaction_forces)
            .any(|a| !finite(*a))
            || !finite(result.reaction_torque_about_origin)
        {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }
    pub(super) fn exchange_image_work(
        &self,
        particles: &mut [Particle],
        properties: &[Material],
        density: &[f64],
        real_pairs: usize,
        fields: &mut super::transport::Transport,
        dt: f64,
    ) -> Result<Vec<[f64; 3]>, Error> {
        if self.reflecting_box.is_none() {
            return Ok(vec![[0.0; 3]; particles.len()]);
        }
        let images = self.image_pairs(particles, real_pairs)?;
        let acceleration = self
            .image_pressure(particles, properties, density, &images)?
            .pressure_accelerations;
        for (index, (p, a)) in particles.iter_mut().zip(&acceleration).enumerate() {
            if a.iter().all(|v| *v == 0.0) {
                continue;
            }
            let impulse = a.map(|v| p.mass * v * dt);
            let work: f64 = impulse
                .iter()
                .zip(p.velocity)
                .map(|(j, v)| j * (v + 0.5 * j / p.mass))
                .sum();
            let energy = fields.energy(p, &fields.fields[index], index)?;
            fields.set_energy(index, p, energy - work)?;
            for (axis, j) in impulse.into_iter().enumerate() {
                p.velocity[axis] += j / p.mass;
            }
            if !finite(p.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(acceleration)
    }
}

impl Liquid {
    fn image_velocity(particles: &[Particle], pair: &ImagePair, parity: f64) -> ([f64; 3], f64) {
        if pair.radius <= 1e-12 {
            return ([0.0; 3], 0.0);
        }
        let first = particles[pair.particle];
        let second = particles[pair.image.owner];
        let direction = sub(first.position, pair.image.position).map(|v| v / pair.radius);
        let speed = direction
            .iter()
            .enumerate()
            .map(|(a, n)| n * (first.velocity[a] - parity * second.velocity[a]))
            .sum();
        (direction, speed)
    }
    fn image_drag(
        &self,
        particles: &[Particle],
        properties: &[Material],
        density: &[f64],
        pair: &ImagePair,
    ) -> (f64, f64) {
        let i = pair.particle;
        let j = pair.image.owner;
        let parity = if pair.image.faces.iter().flatten().count().is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        // Directed images contain both receiver/owner orderings. Half weight avoids
        // double counting. Use the same central projection and factor 5 as real pairs.
        let c = 0.5
            * super::viscous_conductance(
                [particles[i].mass, particles[j].mass],
                0.5 * (properties[i].viscosity + properties[j].viscosity),
                [density[i], density[j]],
                pair.radius,
                self.config.smoothing_radius,
            );
        (c, parity)
    }
    pub(super) fn add_image_forces(
        &self,
        particles: &[Particle],
        properties: &[Material],
        density: &[f64],
        images: &[ImagePair],
        acceleration: &mut [[f64; 3]],
        rates: &mut [f64],
    ) -> Result<(), Error> {
        self.add_image_pressure(particles, properties, density, images, acceleration)?;
        if self.reflecting_no_slip == WallViscosity::FreeSlip {
            return Ok(());
        }
        for pair in images {
            let i = pair.particle;
            let j = pair.image.owner;
            let (conductance, parity) = self.image_drag(particles, properties, density, pair);
            if !conductance.is_finite() || conductance < 0.0 {
                return Err(Error::NumericalFailure);
            }
            if i == j {
                rates[i] += conductance * (1.0 - parity).powi(2) / particles[i].mass;
            } else {
                // Absolute row sum bounds the explicit operator, including images.
                rates[i] += 2.0 * conductance / particles[i].mass;
                rates[j] += 2.0 * conductance / particles[j].mass;
            }
            if self.viscous_heating {
                continue;
            }
            let (direction, speed) = Self::image_velocity(particles, pair, parity);
            for (axis, normal) in direction.into_iter().enumerate() {
                let force = -conductance * speed * normal;
                acceleration[i][axis] += force / particles[i].mass;
                acceleration[j][axis] -= parity * force / particles[j].mass;
            }
        }
        Ok(())
    }
    pub(super) fn dissipate_image_viscosity(
        &self,
        particles: &mut [Particle],
        properties: &[Material],
        density: &[f64],
        real_pairs: usize,
        fields: &mut super::transport::Transport,
        dt: f64,
    ) -> Result<(), Error> {
        if self.reflecting_no_slip == WallViscosity::FreeSlip {
            return Ok(());
        }
        let images = self.image_pairs(particles, real_pairs)?;
        self.dissipate_image_pairs(particles, properties, density, &images, fields, dt)
    }
    pub(super) fn dissipate_image_pairs(
        &self,
        particles: &mut [Particle],
        properties: &[Material],
        density: &[f64],
        images: &[ImagePair],
        fields: &mut super::transport::Transport,
        dt: f64,
    ) -> Result<(), Error> {
        if self.reflecting_no_slip == WallViscosity::FreeSlip {
            return Ok(());
        }
        for pair in images {
            let i = pair.particle;
            let j = pair.image.owner;
            let (conductance, parity) = self.image_drag(particles, properties, density, pair);
            let inverse = if i == j {
                (1.0 - parity).powi(2) / particles[i].mass
            } else {
                1.0 / particles[i].mass + 1.0 / particles[j].mass
            };
            if inverse == 0.0 || conductance == 0.0 {
                continue;
            }
            let fraction = -(-conductance * inverse * dt).exp_m1();
            let (direction, speed) = Self::image_velocity(particles, pair, parity);
            let heat = 0.5 / inverse * speed.powi(2) * fraction * (2.0 - fraction);
            if !conductance.is_finite() || conductance < 0.0 || !heat.is_finite() {
                return Err(Error::NumericalFailure);
            }
            for (a, normal) in direction.into_iter().enumerate() {
                let impulse = fraction * speed * normal / inverse;
                particles[i].velocity[a] -= impulse / particles[i].mass;
                particles[j].velocity[a] += parity * impulse / particles[j].mass;
            }
            for index in [i, j] {
                let energy = fields.energy(&particles[index], &fields.fields[index], index)?;
                fields.set_energy(index, &particles[index], energy + 0.5 * heat)?;
            }
            if !finite(particles[i].velocity) || !finite(particles[j].velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(())
    }
}
