//! Gas EOS and coupled integration/energy policies; fields and boundaries are staged separately.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiquidEvolutionCheckpoint {
    pub version: u32,
    pub materials: Vec<[u64; 3]>,
    pub formulation: u8,
    pub gas_equations: Vec<Option<[u64; 2]>>,
    pub pressure_work: bool,
    pub viscous_heating: bool,
    pub viscous_integrator: u8,
    pub tensile_floor_pa: Option<u64>,
}
impl Liquid {
    #[must_use]
    pub fn evolution_checkpoint(&self) -> LiquidEvolutionCheckpoint {
        LiquidEvolutionCheckpoint {
            version: 1,
            materials: self
                .materials
                .iter()
                .map(|m| [m.rest_density, m.sound_speed, m.viscosity].map(f64::to_bits))
                .collect(),
            formulation: match self.formulation {
                Formulation::MassDensity => 0,
                Formulation::RestVolume => 1,
                Formulation::RestVolumeWendland => 2,
            },
            gas_equations: self
                .gas_equations
                .iter()
                .map(|m| m.map(|m| [m.gas_constant, m.heat_capacity_ratio].map(f64::to_bits)))
                .collect(),
            pressure_work: self.pressure_work,
            viscous_heating: self.viscous_heating,
            viscous_integrator: match self.viscous_integrator {
                ViscousIntegrator::Sequential => 0,
                ViscousIntegrator::Symmetric => 1,
                ViscousIntegrator::Midpoint => 2,
            },
            tensile_floor_pa: self.tensile_floor_pa.map(f64::to_bits),
        }
    }
    /// Restores policies through native EOS/energy admission on a private candidate.
    /// # Errors
    /// Rejects versions/layout/limits, invalid EOS, incompatible geometry or energy modes.
    pub fn restore_evolution_checkpoint(
        &mut self,
        saved: &LiquidEvolutionCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        if saved.version != 1
            || self.particles.len() > max_particles
            || saved.materials.len() > max_materials
            || saved.materials.len() != self.materials.len()
            || saved.gas_equations.len() != self.materials.len()
            || saved
                .materials
                .iter()
                .zip(&self.materials)
                .any(|(bits, m)| {
                    *bits != [m.rest_density, m.sound_speed, m.viscosity].map(f64::to_bits)
                })
        {
            return Err(Error::InvalidConfig);
        }
        let formulation = match saved.formulation {
            0 => Formulation::MassDensity,
            1 => Formulation::RestVolume,
            2 => Formulation::RestVolumeWendland,
            _ => return Err(Error::InvalidConfig),
        };
        let integrator = match saved.viscous_integrator {
            0 => ViscousIntegrator::Sequential,
            1 => ViscousIntegrator::Symmetric,
            2 => ViscousIntegrator::Midpoint,
            _ => return Err(Error::InvalidConfig),
        };
        let equations: Vec<_> = saved
            .gas_equations
            .iter()
            .map(|m| {
                m.map(|bits| IdealGas {
                    gas_constant: f64::from_bits(bits[0]),
                    heat_capacity_ratio: f64::from_bits(bits[1]),
                })
            })
            .collect();
        let mut candidate = self.clone();
        candidate.set_formulation(formulation);
        if equations.iter().any(Option::is_some) {
            candidate.configure_gas_equations(equations)?;
        } else {
            candidate.gas_equations = equations;
        }
        candidate.set_viscous_heating(saved.viscous_heating)?;
        candidate.set_pressure_work(saved.pressure_work)?;
        candidate.set_viscous_integrator(integrator)?;
        candidate.set_tensile_pressure_floor(saved.tensile_floor_pa.map(f64::from_bits))?;
        candidate.validate_gas_geometry()?;
        candidate.effective_materials()?;
        *self = candidate;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gas_and_liquid_energy_policies_continue_for_all_viscous_integrators() {
        for gas in [false, true] {
            for integrator in [
                ViscousIntegrator::Sequential,
                ViscousIntegrator::Symmetric,
                ViscousIntegrator::Midpoint,
            ] {
                let config = Config {
                    gravity: [0.; 3],
                    max_particles: 8,
                    max_pairs: 16,
                    max_neighbor_checks: 64,
                    max_substeps: 256,
                    ..Config::default()
                };
                let mut original = Liquid::new(
                    vec![
                        Particle {
                            position: [-0.04, 0., 0.],
                            velocity: [0.1, 0., 0.],
                            mass: 1.,
                            material: 0,
                        },
                        Particle {
                            position: [0.04, 0., 0.],
                            velocity: [-0.1, 0., 0.],
                            mass: 1.,
                            material: 0,
                        },
                    ],
                    vec![Material {
                        viscosity: 0.1,
                        ..Material::WATER
                    }],
                    config,
                )
                .unwrap();
                let eos = IdealGas {
                    gas_constant: 1.,
                    heat_capacity_ratio: 1.4,
                };
                let temperatures = if gas { [10., 20.] } else { [300., 330.] };
                original
                    .configure_transport(
                        temperatures
                            .into_iter()
                            .map(|temperature| LiquidField {
                                temperature,
                                concentration: 0.2,
                            })
                            .collect(),
                        vec![TransportMaterial {
                            specific_heat: if gas {
                                eos.specific_heat_cv().unwrap()
                            } else {
                                4184.
                            },
                            conductivity: 0.001,
                            diffusivity: 0.,
                            mixing_group: 0,
                        }],
                    )
                    .unwrap();
                if gas {
                    original.configure_gas_equations(vec![Some(eos)]).unwrap();
                } else {
                    original.set_formulation(Formulation::RestVolumeWendland);
                }
                original.set_pressure_work(true).unwrap();
                original.set_viscous_heating(true).unwrap();
                original.set_viscous_integrator(integrator).unwrap();
                original.set_tensile_pressure_floor(Some(100.)).unwrap();
                for _ in 0..7 {
                    original.step(0.00001, None).unwrap();
                }
                let saved = original.evolution_checkpoint();
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
                restored.restore_evolution_checkpoint(&saved, 8, 1).unwrap();
                assert_eq!(restored.evolution_checkpoint(), saved);
                for _ in 0..120 {
                    assert_eq!(
                        original.step(0.00001, None).unwrap(),
                        restored.step(0.00001, None).unwrap()
                    );
                    assert_eq!(
                        original.particle_checkpoint(),
                        restored.particle_checkpoint()
                    );
                    assert_eq!(original.thermal_checkpoint(), restored.thermal_checkpoint());
                    assert_eq!(
                        original.evolution_checkpoint(),
                        restored.evolution_checkpoint()
                    );
                    let a = original.transport_totals().unwrap().unwrap();
                    let b = restored.transport_totals().unwrap().unwrap();
                    assert_eq!([a.0, a.1].map(f64::to_bits), [b.0, b.1].map(f64::to_bits));
                }
                let before = restored.clone();
                let mut bad = saved.clone();
                bad.gas_equations[0] = Some([(-1_f64).to_bits(), 1.4_f64.to_bits()]);
                assert!(restored.restore_evolution_checkpoint(&bad, 8, 1).is_err());
                assert_eq!(restored, before);
                let mut bad = saved.clone();
                bad.formulation = 3;
                assert!(restored.restore_evolution_checkpoint(&bad, 8, 1).is_err());
                assert_eq!(restored, before);
                if gas {
                    let mut bad = saved;
                    bad.pressure_work = false;
                    assert!(restored.restore_evolution_checkpoint(&bad, 8, 1).is_err());
                    assert_eq!(restored, before);
                }
            }
        }
    }
}
