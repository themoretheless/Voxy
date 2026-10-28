//! Explicit droplet eligibility and compensated deferred dissipation inventory.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopulationHeatCheckpoint {
    pub version: u32,
    pub particle_materials: Vec<usize>,
    pub population: Option<Vec<bool>>,
    /// High and low parts stay separate; either may carry a signed rounding residue.
    pub heat: Vec<[u64; 2]>,
}
impl Liquid {
    /// # Errors
    /// Rejects in-flight carrier/body transactions; only accepted states are saved.
    pub fn population_heat_checkpoint(&self) -> Result<PopulationHeatCheckpoint, Error> {
        if self.carrier_droplet_stage || self.boundary_coupling.is_some() {
            return Err(Error::InvalidConfig);
        }
        Ok(PopulationHeatCheckpoint {
            version: 1,
            particle_materials: self.particles.iter().map(|p| p.material).collect(),
            population: self.droplet_population.clone(),
            heat: self
                .suspension_heat_buffer
                .iter()
                .zip(&self.suspension_heat_correction)
                .map(|(high, low)| [high.to_bits(), low.to_bits()])
                .collect(),
        })
    }
    /// Restores exact deferred inventory without depositing heat into fields.
    /// None population remains distinct from an explicit all-false mask.
    /// # Errors
    /// Rejects layout/limits, nonfinite inventory, missing thermal ownership and in-flight state.
    pub fn restore_population_heat_checkpoint(
        &mut self,
        saved: &PopulationHeatCheckpoint,
        max_particles: usize,
    ) -> Result<(), Error> {
        if saved.version != 1
            || self.carrier_droplet_stage
            || self.boundary_coupling.is_some()
            || saved.heat.len() > max_particles
            || saved.heat.len() != self.particles.len()
            || saved.particle_materials.len() != self.particles.len()
            || saved
                .particle_materials
                .iter()
                .zip(&self.particles)
                .any(|(m, p)| *m != p.material)
            || saved
                .population
                .as_ref()
                .is_some_and(|p| p.len() != self.particles.len())
        {
            return Err(Error::InvalidConfig);
        }
        let mut high = Vec::with_capacity(saved.heat.len());
        let mut low = Vec::with_capacity(saved.heat.len());
        for bits in &saved.heat {
            let [h, l] = bits.map(f64::from_bits);
            if !h.is_finite()
                || !l.is_finite()
                || !(h + l).is_finite()
                || (self.transport.is_none() && (h != 0. || l != 0.))
            {
                return Err(Error::InvalidTransport);
            }
            high.push(h);
            low.push(l);
        }
        let mut candidate = self.clone();
        candidate.configure_droplet_population(saved.population.clone())?;
        candidate.suspension_heat_buffer = high;
        candidate.suspension_heat_correction = low;
        *self = candidate;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sub_ulp_inventory_and_population_survive_restore_and_later_heat_deposit() {
        for mask in [None, Some(vec![true, false]), Some(vec![false, false])] {
            let config = Config {
                gravity: [0., -0.1, 0.],
                max_particles: 8,
                max_pairs: 16,
                max_neighbor_checks: 64,
                max_substeps: 256,
                ..Config::default()
            };
            let mut original = Liquid::new(
                vec![
                    Particle {
                        position: [0.; 3],
                        velocity: [0.; 3],
                        mass: 1.,
                        material: 0,
                    },
                    Particle {
                        position: [0.05, 0., 0.],
                        velocity: [0.; 3],
                        mass: 1.,
                        material: 0,
                    },
                ],
                vec![Material {
                    sound_speed: 2.,
                    ..Material::WATER
                }],
                config,
            )
            .unwrap();
            original
                .configure_transport(
                    vec![
                        LiquidField {
                            temperature: 10.,
                            concentration: 0.,
                        },
                        LiquidField {
                            temperature: 11.,
                            concentration: 0.1,
                        },
                    ],
                    vec![TransportMaterial {
                        specific_heat: 1.,
                        conductivity: 0.,
                        diffusivity: 0.,
                        mixing_group: 0,
                    }],
                )
                .unwrap();
            original.configure_droplet_population(mask.clone()).unwrap();
            original.add_heat(&[1e100, 0.]).unwrap();
            original.deposit_dissipation_heat(&[1., 0.]).unwrap();
            let increment = f64::EPSILON / 4.;
            original.deposit_dissipation_heat(&[increment, 0.]).unwrap();
            let saved = original.population_heat_checkpoint().unwrap();
            assert_eq!(f64::from_bits(saved.heat[0][0]), 1.);
            assert_eq!(f64::from_bits(saved.heat[0][1]), increment);
            let limits = LiquidCheckpointLimits {
                max_particles: 8,
                max_materials: 1,
                max_pairs: 16,
                max_neighbor_checks: 64,
                max_substeps: 256,
            };
            let mut restored = original.particle_checkpoint().prepare_base(limits).unwrap();
            restored
                .restore_thermal_checkpoint(&original.thermal_checkpoint().unwrap(), 8, 1)
                .unwrap();
            restored
                .configure_droplet_population(Some(vec![false, true]))
                .unwrap();
            restored
                .restore_population_heat_checkpoint(&saved, 8)
                .unwrap();
            assert_eq!(restored.population_heat_checkpoint().unwrap(), saved);
            for frame in 0..120 {
                if frame == 60 {
                    let energy = original.transport_totals().unwrap().unwrap().0;
                    original.add_heat(&[-energy, 0.]).unwrap();
                    restored.add_heat(&[-energy, 0.]).unwrap();
                }
                original.deposit_dissipation_heat(&[increment, 0.]).unwrap();
                restored.deposit_dissipation_heat(&[increment, 0.]).unwrap();
                if frame == 60 {
                    let temperature = original.fields().unwrap()[0].temperature;
                    assert!(
                        (0.9..1.1).contains(&temperature),
                        "deferred inventory was not deposited"
                    );
                    assert_eq!(
                        original.suspension_heat_buffer()[0].to_bits(),
                        0_f64.to_bits()
                    );
                }
                if mask.is_some() {
                    assert_eq!(
                        original.step_carrier_and_droplets(0.0001, None).unwrap(),
                        restored.step_carrier_and_droplets(0.0001, None).unwrap()
                    );
                } else {
                    assert_eq!(
                        original.step(0.0001, None).unwrap(),
                        restored.step(0.0001, None).unwrap()
                    );
                }
                assert_eq!(
                    original.particle_checkpoint(),
                    restored.particle_checkpoint()
                );
                assert_eq!(original.thermal_checkpoint(), restored.thermal_checkpoint());
                assert_eq!(
                    original.population_heat_checkpoint().unwrap(),
                    restored.population_heat_checkpoint().unwrap()
                );
            }
            assert!(original.fields().unwrap()[0].temperature < 1e100);
            let before = restored.clone();
            let mut bad = saved.clone();
            bad.heat[0][1] = f64::NAN.to_bits();
            assert!(
                restored
                    .restore_population_heat_checkpoint(&bad, 8)
                    .is_err()
            );
            assert_eq!(restored, before);
            let mut bad = saved.clone();
            bad.population = Some(vec![true]);
            assert!(
                restored
                    .restore_population_heat_checkpoint(&bad, 8)
                    .is_err()
            );
            assert_eq!(restored, before);
            assert!(
                restored
                    .restore_population_heat_checkpoint(&saved, 1)
                    .is_err()
            );
            let mut active = restored;
            active.carrier_droplet_stage = true;
            assert!(active.population_heat_checkpoint().is_err());
        }
    }
}
