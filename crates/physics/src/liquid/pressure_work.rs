use super::{Error, Liquid, Material, Particle, finite, norm, positive, sub, transport::Transport};
impl Liquid {
    /// Enables conservative pressure kicks with opposite energy exchange into enthalpy.
    /// This discrete work balance does not replace the weakly compressible equation of state.
    /// # Errors
    /// Enabling requires thermal fields. Failure leaves the setting unchanged.
    pub fn set_pressure_work(&mut self, enabled: bool) -> Result<(), Error> {
        if (!enabled && self.gas_equations.iter().any(Option::is_some))
            || (enabled && self.transport.is_none())
        {
            return Err(Error::InvalidTransport);
        }
        self.pressure_work = enabled;
        Ok(())
    }
    pub(super) fn exchange_pressure_work(
        &self,
        particles: &mut [Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
        density: &[f64],
        fields: &mut Transport,
        dt: f64,
    ) -> Result<Vec<[f64; 3]>, Error> {
        if self.gas_equations.iter().any(Option::is_some) {
            return self
                .exchange_gas_pressure_work(particles, pairs, properties, density, fields, dt);
        }
        let mut accelerations = vec![[0.0; 3]; particles.len()];
        let support = self.config.smoothing_radius;
        for &(i, j) in pairs {
            let first = particles[i];
            let second = particles[j];
            let displacement = sub(first.position, second.position);
            let radius = norm(displacement);
            let direction = if radius > 1e-12 {
                displacement.map(|value| value / radius)
            } else {
                [1.0, 0.0, 0.0]
            };
            let first_pressure =
                self.material_pressure(first.material, properties[i], density[i])?;
            let second_pressure =
                self.material_pressure(second.material, properties[j], density[j])?;
            let magnitude = self.thermodynamic_pressure_magnitude(
                [first.mass, second.mass],
                [density[i], density[j]],
                [first_pressure, second_pressure],
                [properties[i].rest_density, properties[j].rest_density],
                [support, radius],
                [first.material, second.material],
            );
            let reduced = 1.0 / (1.0 / first.mass + 1.0 / second.mass);
            if !magnitude.is_finite() || !positive(reduced) {
                return Err(Error::NumericalFailure);
            }
            let relative = sub(first.velocity, second.velocity);
            let impulse = direction.map(|component| magnitude * dt * component);
            let work: f64 = impulse
                .iter()
                .zip(relative)
                .map(|(component, speed)| component * (speed + 0.5 * component / reduced))
                .sum();
            if !work.is_finite() {
                return Err(Error::NumericalFailure);
            }
            for axis in 0..3 {
                let force = magnitude * direction[axis];
                accelerations[i][axis] += force / first.mass;
                accelerations[j][axis] -= force / second.mass;
                particles[i].velocity[axis] += impulse[axis] / first.mass;
                particles[j].velocity[axis] -= impulse[axis] / second.mass;
            }
            if !finite(particles[i].velocity) || !finite(particles[j].velocity) {
                return Err(Error::NumericalFailure);
            }
            let ratio = if self.formulation == super::Formulation::MassDensity {
                1.0
            } else {
                properties[i].rest_density / properties[j].rest_density
            };
            let first_weight = first_pressure / density[i].powi(2) * ratio;
            let second_weight = second_pressure / density[j].powi(2) / ratio;
            let first_share = if first_weight + second_weight > 0.0 {
                first_weight / (first_weight + second_weight)
            } else {
                0.5
            };
            for (index, share) in [(i, first_share), (j, 1.0 - first_share)] {
                let energy = fields.energy(&particles[index], &fields.fields[index], index)?;
                fields.set_energy(index, &particles[index], energy - share * work)?;
            }
        }
        if self.boundary_coupling.is_none() {
            let boundary = self.exchange_boundary_work(
                particles,
                properties,
                density,
                pairs.len(),
                fields,
                dt,
            )?;
            for (total, wall) in accelerations.iter_mut().zip(boundary) {
                for axis in 0..3 {
                    total[axis] += wall[axis];
                }
            }
        }
        if self.reflecting_box.is_some() {
            let reflected =
                self.exchange_image_work(particles, properties, density, pairs.len(), fields, dt)?;
            for (total, image) in accelerations.iter_mut().zip(reflected) {
                for axis in 0..3 {
                    total[axis] += image[axis];
                }
            }
        }
        if accelerations.iter().any(|value| !finite(*value)) {
            return Err(Error::NumericalFailure);
        }
        Ok(accelerations)
    }
}
