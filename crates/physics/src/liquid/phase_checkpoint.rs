//! Phase fractions and prescribed-pressure saturation without an energy remap.
use super::super::{SaturationCurve, saturation::SaturationState};
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaturationCheckpoint {
    pub curves: Vec<Option<[u64; 6]>>,
    pub pressures: Vec<u64>,
    pub temperatures: Vec<Option<u64>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhaseCheckpoint {
    pub version: u32,
    pub models: Vec<Option<[u64; 5]>>,
    pub fractions: Vec<u64>,
    pub particle_materials: Vec<usize>,
    pub saturation: Option<SaturationCheckpoint>,
}
impl Liquid {
    #[must_use]
    pub fn phase_checkpoint(&self) -> Option<PhaseCheckpoint> {
        let phase = self.transport.as_ref()?.phase.as_ref()?;
        Some(PhaseCheckpoint {
            version: 1,
            models: phase
                .models
                .iter()
                .map(|m| {
                    m.map(|m| {
                        [
                            m.temperature,
                            m.latent_heat,
                            m.high_phase.rest_density,
                            m.high_phase.sound_speed,
                            m.high_phase.viscosity,
                        ]
                        .map(f64::to_bits)
                    })
                })
                .collect(),
            fractions: phase.fractions.iter().map(|v| v.to_bits()).collect(),
            particle_materials: self.particles.iter().map(|p| p.material).collect(),
            saturation: phase.saturation.as_ref().map(|s| SaturationCheckpoint {
                curves: s
                    .curves
                    .iter()
                    .map(|c| {
                        c.map(|c| {
                            [
                                c.reference_temperature,
                                c.reference_pressure,
                                c.latent_heat,
                                c.vapor_gas_constant,
                                c.min_temperature,
                                c.max_temperature,
                            ]
                            .map(f64::to_bits)
                        })
                    })
                    .collect(),
                pressures: s.pressures.iter().map(|p| p.to_bits()).collect(),
                temperatures: s.temperatures.iter().map(|t| t.map(f64::to_bits)).collect(),
            }),
        })
    }
    /// Restores phase history into matching staged particle/thermal/species data.
    /// Cached saturation temperatures are recomputed for validation only; accepted
    /// words are retained. No heat/pressure remapping is run during restoration.
    /// # Errors
    /// Rejects incompatible layout, invalid models/fractions/pressure/cache and limits.
    pub fn restore_phase_checkpoint(
        &mut self,
        saved: &PhaseCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        self.restore_phase_checkpoint_inner(saved, max_particles, max_materials, true)
    }
    pub(in crate::liquid) fn stage_phase_checkpoint(
        &mut self,
        saved: &PhaseCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        self.restore_phase_checkpoint_inner(saved, max_particles, max_materials, false)
    }
    fn restore_phase_checkpoint_inner(
        &mut self,
        saved: &PhaseCheckpoint,
        max_particles: usize,
        max_materials: usize,
        evaluate: bool,
    ) -> Result<(), Error> {
        if saved.version != 1
            || saved.models.len() > max_materials
            || saved.models.len() != self.materials.len()
            || saved.fractions.len() > max_particles
            || saved.fractions.len() != self.particles.len()
            || saved.particle_materials.len() != self.particles.len()
            || saved
                .particle_materials
                .iter()
                .zip(&self.particles)
                .any(|(m, p)| *m != p.material)
        {
            return Err(Error::InvalidPhaseChange);
        }
        let models: Vec<_> = saved
            .models
            .iter()
            .map(|m| {
                m.map(|b| {
                    let [
                        temperature,
                        latent_heat,
                        rest_density,
                        sound_speed,
                        viscosity,
                    ] = b.map(f64::from_bits);
                    PhaseChange {
                        temperature,
                        latent_heat,
                        high_phase: Material {
                            rest_density,
                            sound_speed,
                            viscosity,
                        },
                    }
                })
            })
            .collect();
        if !models.iter().any(Option::is_some) {
            return Err(Error::InvalidPhaseChange);
        }
        for (model, gas) in models.iter().zip(&self.gas_equations) {
            if let Some(model) = model {
                model.validate()?;
                if gas.is_some() {
                    return Err(Error::InvalidPhaseChange);
                }
            }
        }
        let fractions: Vec<_> = saved
            .fractions
            .iter()
            .copied()
            .map(f64::from_bits)
            .collect();
        if fractions
            .iter()
            .any(|f| !f.is_finite() || !(0.0..=1.0).contains(f))
        {
            return Err(Error::InvalidPhaseChange);
        }
        let saturation = if let Some(s) = &saved.saturation {
            if s.curves.len() != self.materials.len()
                || s.pressures.len() != self.particles.len()
                || s.temperatures.len() != self.particles.len()
            {
                return Err(Error::InvalidPhaseChange);
            }
            let curves: Vec<_> = s
                .curves
                .iter()
                .map(|c| {
                    c.map(|b| {
                        let [
                            reference_temperature,
                            reference_pressure,
                            latent_heat,
                            vapor_gas_constant,
                            min_temperature,
                            max_temperature,
                        ] = b.map(f64::from_bits);
                        SaturationCurve {
                            reference_temperature,
                            reference_pressure,
                            latent_heat,
                            vapor_gas_constant,
                            min_temperature,
                            max_temperature,
                        }
                    })
                })
                .collect();
            if !curves.iter().any(Option::is_some) {
                return Err(Error::InvalidPhaseChange);
            }
            for (curve, model) in curves.iter().zip(&models) {
                if let Some(curve) = curve {
                    curve.validate()?;
                    let model = model.ok_or(Error::InvalidPhaseChange)?;
                    if (curve.latent_heat - model.latent_heat).abs()
                        > 64. * f64::EPSILON * model.latent_heat
                    {
                        return Err(Error::InvalidPhaseChange);
                    }
                }
            }
            let pressures: Vec<_> = s.pressures.iter().copied().map(f64::from_bits).collect();
            if pressures.iter().any(|p| !positive(*p)) {
                return Err(Error::InvalidPhaseChange);
            }
            for (i, p) in self.particles.iter().enumerate() {
                let expected = curves[p.material]
                    .map(|c| c.temperature(pressures[i]))
                    .transpose()?
                    .map(f64::to_bits);
                if expected != s.temperatures[i] {
                    return Err(Error::InvalidPhaseChange);
                }
            }
            Some(SaturationState {
                curves,
                pressures,
                temperatures: s
                    .temperatures
                    .iter()
                    .map(|t| t.map(f64::from_bits))
                    .collect(),
            })
        } else {
            None
        };
        let phase = PhaseState {
            models,
            fractions,
            saturation,
        };
        let mut candidate = self.transport.clone().ok_or(Error::InvalidPhaseChange)?;
        for (i, (particle, field)) in self.particles.iter().zip(&candidate.fields).enumerate() {
            let fraction = phase.fractions[i];
            if let Some(model) = phase.model(i, particle.material) {
                if !(candidate.specific_heat(i, particle.material)? * model.temperature
                    + model.latent_heat)
                    .is_finite()
                    || (field.temperature < model.temperature && fraction != 0.)
                    || (field.temperature > model.temperature && fraction != 1.)
                {
                    return Err(Error::InvalidPhaseChange);
                }
            } else if fraction != 0. {
                return Err(Error::InvalidPhaseChange);
            }
        }
        candidate.phase = Some(phase);
        if evaluate {
            let mut energy = 0.;
            for (i, (particle, field)) in self.particles.iter().zip(&candidate.fields).enumerate() {
                energy += candidate.energy(particle, field, i)?;
                if !energy.is_finite() {
                    return Err(Error::NumericalFailure);
                }
            }
            self.evaluate_materials(&self.particles, Some(&candidate))?;
        }
        self.transport = Some(candidate);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquid::{Config, LiquidCheckpointLimits, LiquidField, Particle, TransportMaterial};
    #[test]
    fn fixed_and_pressure_phase_history_rebuild_without_enthalpy_remap() {
        for saturated in [false, true] {
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
                        velocity: [0.; 3],
                        mass: 1.,
                        material: 0,
                    },
                    Particle {
                        position: [0.04, 0., 0.],
                        velocity: [0.; 3],
                        mass: 1.,
                        material: 1,
                    },
                ],
                vec![Material::WATER; 2],
                config,
            )
            .unwrap();
            original
                .configure_transport(
                    vec![
                        LiquidField {
                            temperature: 373.,
                            concentration: 0.2,
                        },
                        LiquidField {
                            temperature: 373.,
                            concentration: 0.8,
                        },
                    ],
                    vec![
                        TransportMaterial {
                            specific_heat: 4184.,
                            conductivity: 100.,
                            diffusivity: 0.001,
                            mixing_group: 7
                        };
                        2
                    ],
                )
                .unwrap();
            let model = PhaseChange {
                temperature: 373.,
                latent_heat: 2_260_000.,
                high_phase: Material {
                    rest_density: 700.,
                    sound_speed: 20.,
                    viscosity: 0.001,
                },
            };
            original
                .configure_phase_change(vec![Some(model); 2], vec![0.2, 0.7])
                .unwrap();
            if saturated {
                original
                    .configure_saturation(
                        vec![
                            Some(SaturationCurve {
                                reference_temperature: 373.,
                                reference_pressure: 100000.,
                                latent_heat: model.latent_heat,
                                vapor_gas_constant: 461.5,
                                min_temperature: 300.,
                                max_temperature: 500.,
                            }),
                            None,
                        ],
                        vec![100070., 100000.],
                    )
                    .unwrap();
            }
            for _ in 0..7 {
                original.add_heat(&[2., -1.]).unwrap();
                original.step(0.0001, None).unwrap();
            }
            let particles = original.particle_checkpoint();
            let thermal = original.thermal_checkpoint().unwrap();
            let phase = original.phase_checkpoint().unwrap();
            let limits = LiquidCheckpointLimits {
                max_particles: 8,
                max_materials: 2,
                max_pairs: 16,
                max_neighbor_checks: 64,
                max_substeps: 256,
            };
            let mut restored = particles.prepare_base(limits).unwrap();
            restored.restore_thermal_checkpoint(&thermal, 8, 2).unwrap();
            restored.restore_phase_checkpoint(&phase, 8, 2).unwrap();
            assert_eq!(restored.phase_checkpoint(), Some(phase.clone()));
            assert_eq!(restored.thermal_checkpoint(), Some(thermal));
            for frame in 0_u32..120 {
                if saturated {
                    let pressures = vec![100000. + f64::from(frame) * 20., 100000.];
                    original
                        .set_saturation_pressures(pressures.clone())
                        .unwrap();
                    restored.set_saturation_pressures(pressures).unwrap();
                }
                original.add_heat(&[0.02, -0.01]).unwrap();
                restored.add_heat(&[0.02, -0.01]).unwrap();
                assert_eq!(
                    original.step(0.0001, None).unwrap(),
                    restored.step(0.0001, None).unwrap()
                );
                assert_eq!(
                    original.particle_checkpoint(),
                    restored.particle_checkpoint()
                );
                assert_eq!(original.thermal_checkpoint(), restored.thermal_checkpoint());
                assert_eq!(original.phase_checkpoint(), restored.phase_checkpoint());
                let a = original.transport_totals().unwrap().unwrap();
                let b = restored.transport_totals().unwrap().unwrap();
                assert_eq!([a.0, a.1].map(f64::to_bits), [b.0, b.1].map(f64::to_bits));
            }
            let before = restored.clone();
            let mut bad = phase.clone();
            bad.fractions[0] = f64::NAN.to_bits();
            assert!(restored.restore_phase_checkpoint(&bad, 8, 2).is_err());
            assert_eq!(restored, before);
            if let Some(mut cache) = phase.saturation.clone() {
                cache.temperatures[0] = cache.temperatures[0].map(|bits| bits ^ 1);
                let bad = PhaseCheckpoint {
                    saturation: Some(cache),
                    ..phase.clone()
                };
                assert!(restored.restore_phase_checkpoint(&bad, 8, 2).is_err());
                assert_eq!(restored, before);
            }
            assert!(restored.restore_phase_checkpoint(&phase, 1, 2).is_err());
        }
    }
}
