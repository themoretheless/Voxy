//! Experimental stationary-wall pressure reconstruction with a gravity correction.
use super::{Error, Liquid, Material, Particle, density_kernel, finite, pairs, positive, sub};
use std::collections::BTreeMap;
type WallValues = BTreeMap<(usize, usize), (f64, f64)>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallPressure {
    pub sample: usize,
    pub material: usize,
    pub pressure: f64,
    pub density: f64,
}
impl Liquid {
    /// Enables gravity-corrected, volume-weighted pressure extrapolation on fixed walls.
    /// This experimental boundary changes pressure forces, not density quadrature.
    /// It is not a full implementation of Adami et al. and has no rotating/moving-wall model.
    /// # Errors
    /// Moving prescribed samples or body coupling are unsupported. Failure is atomic.
    pub fn set_static_pressure_extrapolation(&mut self, enabled: bool) -> Result<(), Error> {
        if enabled
            && (self.reflecting_box.is_some()
                || self.boundary_coupling.is_some()
                || self
                    .boundary_velocities
                    .iter()
                    .any(|v| v.iter().any(|x| *x != 0.0)))
        {
            return Err(Error::InvalidBoundary);
        }
        self.static_pressure_extrapolation = enabled;
        Ok(())
    }
    /// Reconstructs pressure for each supported (sample, material) pair, in sorted order.
    /// # Errors
    /// Requires enabled stationary-wall extrapolation; usual material/neighbor budgets apply.
    pub fn static_wall_pressures(&self) -> Result<Vec<WallPressure>, Error> {
        if !self.static_pressure_extrapolation {
            return Err(Error::InvalidBoundary);
        }
        let properties = self.effective_materials()?;
        let fluid_pairs = pairs(
            &self.particles,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )?;
        let density = self.forces(&self.particles, &fluid_pairs, &properties)?.0;
        let neighbors = self.boundary_pairs(&self.particles, fluid_pairs.len())?;
        Ok(self
            .extrapolated_wall_pressure(&self.particles, &properties, &density, &neighbors)?
            .into_iter()
            .map(|((sample, material), (pressure, density))| WallPressure {
                sample,
                material,
                pressure,
                density,
            })
            .collect())
    }
    pub(super) fn extrapolated_wall_pressure(
        &self,
        particles: &[Particle],
        properties: &[Material],
        density: &[f64],
        neighbors: &[(usize, usize, f64)],
    ) -> Result<WallValues, Error> {
        if self.boundary_coupling.is_some()
            || self
                .boundary_velocities
                .iter()
                .any(|v| v.iter().any(|x| *x != 0.0))
        {
            return Err(Error::InvalidBoundary);
        }
        let mut sums = BTreeMap::<_, [f64; 4]>::new();
        for &(i, j, radius) in neighbors {
            let p = particles[i];
            let m = properties[i];
            let weight = p.mass / density[i]
                * density_kernel(self.formulation, self.config.smoothing_radius, radius);
            let displacement = sub(self.boundaries[j].position, p.position);
            let head: f64 = self
                .config
                .gravity
                .iter()
                .zip(displacement)
                .map(|(g, x)| g * x)
                .sum();
            let pressure = m.sound_speed.powi(2) * (density[i] - m.rest_density).max(0.0);
            let sum = sums.entry((j, p.material)).or_insert([0.0; 4]);
            sum[0] += weight;
            sum[1] += weight * (pressure + density[i] * head);
            // Responses may differ between particles of one material: average the
            // effective rest density and compliance rather than choosing a neighbor.
            sum[2] += weight * m.rest_density;
            sum[3] += weight / m.sound_speed.powi(2);
        }
        sums.into_iter()
            .map(|(key, sum)| {
                if !positive(sum[0]) || !finite([sum[1], sum[2], sum[3]]) {
                    return Err(Error::NumericalFailure);
                }
                let pressure = (sum[1] / sum[0]).max(0.0);
                let density = sum[2] / sum[0] + pressure * sum[3] / sum[0];
                if !pressure.is_finite() || !positive(density) {
                    return Err(Error::NumericalFailure);
                }
                Ok((key, (pressure, density)))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquid::{BoundarySample, Config, Formulation};
    #[test]
    fn hydrostatic_affine_pressure_is_reproduced_for_irregular_neighbor_volumes() {
        for formulation in [
            Formulation::MassDensity,
            Formulation::RestVolume,
            Formulation::RestVolumeWendland,
        ] {
            for gravity in [0.0, -2.0] {
                let particles = vec![
                    Particle {
                        position: [0.1, 0.1, 0.0],
                        velocity: [0.0; 3],
                        mass: 1.0,
                        material: 0,
                    },
                    Particle {
                        position: [-0.1, 0.2, 0.05],
                        velocity: [0.0; 3],
                        mass: 2.7,
                        material: 0,
                    },
                ];
                let mut liquid = Liquid::new(
                    particles.clone(),
                    vec![Material::WATER],
                    Config {
                        smoothing_radius: 1.0,
                        gravity: [0.0, gravity, 0.0],
                        ..Config::default()
                    },
                )
                .unwrap();
                liquid.set_formulation(formulation);
                let wall = [-0.05, -0.1, 0.0];
                liquid
                    .configure_boundaries(vec![BoundarySample {
                        position: wall,
                        volume: 0.8,
                    }])
                    .unwrap();
                let properties: Vec<_> = particles
                    .iter()
                    .map(|p| {
                        let pressure = 500.0 + 1000.0 * gravity * p.position[1];
                        Material {
                            rest_density: 1000.0 - pressure / 10000.0,
                            sound_speed: 100.0,
                            viscosity: 0.0,
                        }
                    })
                    .collect();
                let neighbors: Vec<_> = particles
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i, 0, super::super::norm(sub(p.position, wall))))
                    .collect();
                let fields = liquid
                    .extrapolated_wall_pressure(&particles, &properties, &[1000.0; 2], &neighbors)
                    .unwrap();
                assert!((fields[&(0, 0)].0 - (500.0 + 1000.0 * gravity * wall[1])).abs() < 1e-8);
            }
        }
    }
}
