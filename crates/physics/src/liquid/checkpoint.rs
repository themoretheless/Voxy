//! Complete current accepted Liquid owner, composed before joint physical evaluation.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct LiquidStateCheckpointLimits {
    pub core: LiquidCheckpointLimits,
    pub max_boundary_samples: usize,
    pub max_interfaces: usize,
    pub max_species: usize,
    pub max_name_bytes: usize,
    pub max_reaction_attempts: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiquidCheckpoint {
    pub version: u32,
    pub particles: LiquidParticleCheckpoint,
    pub polymer: PolymerCheckpoint,
    pub rheology: RheologyCheckpoint,
    pub thermal: Option<ThermalFieldCheckpoint>,
    pub phase: Option<PhaseCheckpoint>,
    pub species: Option<SpeciesCheckpoint>,
    pub field_response: FieldResponseCheckpoint,
    pub evolution: LiquidEvolutionCheckpoint,
    pub surface: SurfaceInteractionCheckpoint,
    pub boundary: LiquidBoundaryCheckpoint,
    pub population_heat: PopulationHeatCheckpoint,
}
impl Liquid {
    /// Captures all persistent fields at a public-step boundary.
    /// # Errors
    /// Rejects in-flight carrier/body transactions.
    pub fn checkpoint(&self) -> Result<LiquidCheckpoint, Error> {
        // Intentionally exhaustive: a new owner field requires an explicit save decision.
        let Self {
            particles: _,
            maxwell_fluids: _,
            conformation: _,
            polymer_heat: _,
            formulation: _,
            materials: _,
            config: _,
            surface_strengths: _,
            interface_penalties: _,
            wall_adhesion: _,
            viscous_heating: _,
            viscous_integrator: _,
            pressure_work: _,
            boundary_coupling: _,
            boundaries: _,
            boundary_grid: _,
            boundary_velocities: _,
            static_pressure_extrapolation: _,
            reflecting_box: _,
            reflecting_no_slip: _,
            transport: _,
            suspension_heat_buffer: _,
            suspension_heat_correction: _,
            property_responses: _,
            shear_thinning: _,
            yield_stresses: _,
            thixotropy: _,
            structure: _,
            droplet_population: _,
            carrier_droplet_stage: _,
            arrhenius_viscosity: _,
            gas_equations: _,
            tensile_floor_pa: _,
        } = self;
        self.validate_gas_geometry()?;
        self.effective_materials()?;
        self.polymer_energy()?;
        self.transport_totals()?;
        self.species_totals()?;
        self.chemical_energy()?;
        Ok(LiquidCheckpoint {
            version: 1,
            particles: self.particle_checkpoint(),
            polymer: self.polymer_checkpoint(),
            rheology: self.rheology_checkpoint(),
            thermal: self.thermal_checkpoint(),
            phase: self.phase_checkpoint(),
            species: self.species_checkpoint(),
            field_response: self.field_response_checkpoint(),
            evolution: self.evolution_checkpoint(),
            surface: self.surface_checkpoint(),
            boundary: self.boundary_checkpoint()?,
            population_heat: self.population_heat_checkpoint()?,
        })
    }
}
impl LiquidCheckpoint {
    /// Constructs an independent complete owner. Structural admission precedes
    /// coupled energy/property evaluation, avoiding false intermediate overflow.
    /// # Errors
    /// Rejects invalid blocks, limits, incompatible final physics or a nonexact roundtrip.
    pub fn prepare(&self, limits: LiquidStateCheckpointLimits) -> Result<Liquid, Error> {
        if self.version != 1
            || (self.thermal.is_none() && (self.phase.is_some() || self.species.is_some()))
        {
            return Err(Error::InvalidConfig);
        }
        let core = limits.core;
        let mut candidate = self.particles.prepare_base(core)?;
        if let Some(thermal) = &self.thermal {
            candidate.stage_thermal_checkpoint(thermal, core.max_particles, core.max_materials)?;
        }
        if let Some(species) = &self.species {
            candidate.stage_species_checkpoint(
                species,
                SpeciesCheckpointLimits {
                    max_particles: core.max_particles,
                    max_materials: core.max_materials,
                    max_species: limits.max_species,
                    max_name_bytes: limits.max_name_bytes,
                    max_reaction_attempts: limits.max_reaction_attempts,
                },
            )?;
        }
        candidate.stage_polymer_checkpoint(
            &self.polymer,
            core.max_particles,
            core.max_materials,
        )?;
        if let Some(phase) = &self.phase {
            candidate.stage_phase_checkpoint(phase, core.max_particles, core.max_materials)?;
        }
        candidate.stage_rheology_checkpoint(
            &self.rheology,
            core.max_particles,
            core.max_materials,
        )?;
        candidate.stage_field_response_checkpoint(
            &self.field_response,
            core.max_particles,
            core.max_materials,
        )?;
        candidate.restore_surface_checkpoint(
            &self.surface,
            core.max_particles,
            core.max_materials,
            limits.max_interfaces,
        )?;
        candidate.restore_boundary_checkpoint(
            &self.boundary,
            core.max_particles,
            limits.max_boundary_samples,
        )?;
        candidate.restore_population_heat_checkpoint(&self.population_heat, core.max_particles)?;
        // EOS/energy checks now see the complete set of fields, laws and geometry.
        candidate.restore_evolution_checkpoint(
            &self.evolution,
            core.max_particles,
            core.max_materials,
        )?;
        candidate.effective_materials()?;
        candidate.polymer_energy()?;
        candidate.transport_totals()?;
        candidate.species_totals()?;
        candidate.chemical_energy()?;
        if candidate.checkpoint()? != *self {
            return Err(Error::InvalidConfig);
        }
        Ok(candidate)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> LiquidStateCheckpointLimits {
        LiquidStateCheckpointLimits {
            core: LiquidCheckpointLimits {
                max_particles: 8,
                max_materials: 2,
                max_pairs: 64,
                max_neighbor_checks: 512,
                max_substeps: 256,
            },
            max_boundary_samples: 8,
            max_interfaces: 4,
            max_species: 3,
            max_name_bytes: 128,
            max_reaction_attempts: 4096,
        }
    }
    fn base() -> Liquid {
        Liquid::new(
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
            vec![Material::WATER],
            Config {
                gravity: [0.; 3],
                max_particles: 8,
                max_pairs: 64,
                max_neighbor_checks: 512,
                max_substeps: 256,
                ..Config::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn complete_owner_continues_combined_transport_chemistry_rheology_and_boundaries() {
        for density_change in [false, true] {
            let mut original = base();
            original
                .configure_transport(
                    vec![
                        LiquidField {
                            temperature: 300.,
                            concentration: 0.2,
                        },
                        LiquidField {
                            temperature: 330.,
                            concentration: 0.8,
                        },
                    ],
                    vec![TransportMaterial {
                        specific_heat: 4184.,
                        conductivity: 10.,
                        diffusivity: 0.001,
                        mixing_group: 1,
                    }],
                )
                .unwrap();
            original
                .configure_species(
                    vec!["fuel".into(), "oxidizer".into(), "product".into()],
                    vec![vec![0.3, 0.2, 0.5], vec![0.1, 0.4, 0.5]],
                )
                .unwrap();
            original
                .configure_species_heat_capacities(Some(vec![3000., 3500., 4000.]))
                .unwrap();
            original
                .configure_species_properties(Some(SpeciesProperties {
                    components: vec![Material::WATER; 3],
                    viscosity: ViscosityBlend::Logarithmic,
                }))
                .unwrap();
            original
                .configure_reaction(Some(Reaction {
                    fuel: 0,
                    oxidizer: 1,
                    product: 2,
                    oxidizer_ratio: 1.,
                    rate: 0.1,
                    chemical_energies: vec![1000., 500., 0.],
                }))
                .unwrap();
            original
                .configure_reaction_kinetics(Some(ReactionKinetics {
                    activation_temperature: 100.,
                    ignition_temperature: 100.,
                }))
                .unwrap();
            original
                .configure_property_response(vec![Some(PropertyResponse {
                    reference_temperature: 300.,
                    thermal_expansion: if density_change { 1e-5 } else { 0. },
                    viscosity_temperature_rate: 0.,
                    solute: None,
                })])
                .unwrap();
            original
                .configure_arrhenius_viscosity(vec![Some(ArrheniusViscosity {
                    reference_temperature: 300.,
                    activation_energy: 1000.,
                })])
                .unwrap();
            original
                .configure_herschel_bulkley(&[Some(HerschelBulkley {
                    shear_thinning: ShearThinning {
                        reference_rate: 1.,
                        flow_index: 0.8,
                        minimum_rate: 0.01,
                        minimum_viscosity: 0.0001,
                        maximum_viscosity: 10.,
                    },
                    yield_stress: 0.01,
                })])
                .unwrap();
            original
                .configure_thixotropy(
                    vec![Some(Thixotropy {
                        recovery_rate: 0.4,
                        breakdown: 0.8,
                        broken_viscosity_ratio: 0.1,
                        broken_yield_ratio: 0.2,
                    })],
                    vec![0.3, 0.7],
                )
                .unwrap();
            if !density_change {
                original
                    .set_maxwell_fluid(
                        0,
                        Some(MaxwellFluid {
                            modulus: 2.,
                            relaxation_time: 0.2,
                        }),
                    )
                    .unwrap();
                original
                    .set_conformations(vec![[[1.1, 0., 0.], [0., 0.95, 0.], [0., 0., 1.]]; 2])
                    .unwrap();
            }
            original
                .configure_boundaries(vec![BoundarySample {
                    position: [-0.13, 0., 0.],
                    volume: 0.001,
                }])
                .unwrap();
            original.set_surface_strength(0, 1e-7).unwrap();
            original
                .configure_droplet_population(Some(vec![false, false]))
                .unwrap();
            original.deposit_dissipation_heat(&[1e-30, 0.]).unwrap();
            original
                .deposit_dissipation_heat(&[1e-30 * f64::EPSILON / 4., 0.])
                .unwrap();
            original.set_pressure_work(true).unwrap();
            original.set_viscous_heating(true).unwrap();
            for _ in 0..7 {
                original.step(0.00001, None).unwrap();
            }
            let saved = original.checkpoint().unwrap();
            let bytes = saved.to_bytes(1_000_000).unwrap();
            let saved = LiquidCheckpoint::from_bytes(
                &bytes,
                LiquidWireLimits {
                    max_bytes: 1_000_000,
                    max_allocation_bytes: 1_000_000,
                    max_values: 100_000,
                },
            )
            .unwrap();
            assert_eq!(saved, original.checkpoint().unwrap());
            let mut restored = saved.prepare(limits()).unwrap();
            assert_eq!(restored, saved.prepare(limits()).unwrap());
            assert_eq!(restored, original);
            for _ in 0..120 {
                assert_eq!(
                    original.step(0.00001, None).unwrap(),
                    restored.step(0.00001, None).unwrap()
                );
                assert_eq!(
                    original.checkpoint().unwrap(),
                    restored.checkpoint().unwrap()
                );
                assert_eq!(original, restored);
            }
            let mut bad = saved;
            bad.population_heat.heat[0][1] = f64::NAN.to_bits();
            assert!(bad.prepare(limits()).is_err());
        }
    }
    #[test]
    fn complete_owner_covers_phase_gas_reflections_and_nonthermal_population() {
        for mode in 0..3 {
            let mut original = base();
            if mode < 2 {
                let gas = IdealGas {
                    gas_constant: 1.,
                    heat_capacity_ratio: 1.4,
                };
                original
                    .configure_transport(
                        vec![
                            LiquidField {
                                temperature: if mode == 0 { 300. } else { 10. },
                                concentration: 0.2
                            };
                            2
                        ],
                        vec![TransportMaterial {
                            specific_heat: if mode == 0 {
                                4184.
                            } else {
                                gas.specific_heat_cv().unwrap()
                            },
                            conductivity: 0.,
                            diffusivity: 0.,
                            mixing_group: 0,
                        }],
                    )
                    .unwrap();
                if mode == 0 {
                    original
                        .configure_phase_change(
                            vec![Some(PhaseChange {
                                temperature: 300.,
                                latent_heat: 2000.,
                                high_phase: Material {
                                    rest_density: 900.,
                                    ..Material::WATER
                                },
                            })],
                            vec![0.2, 0.7],
                        )
                        .unwrap();
                    original
                        .configure_saturation(
                            vec![Some(SaturationCurve {
                                reference_temperature: 300.,
                                reference_pressure: 100000.,
                                latent_heat: 2000.,
                                vapor_gas_constant: 1.,
                                min_temperature: 200.,
                                max_temperature: 400.,
                            })],
                            vec![100000.; 2],
                        )
                        .unwrap();
                    original.set_formulation(Formulation::RestVolume);
                } else {
                    original.configure_gas_equations(vec![Some(gas)]).unwrap();
                    original
                        .set_reflecting_box(Some(ReflectingBox {
                            min: [-0.1, -1., -1.],
                            max: [1.; 3],
                        }))
                        .unwrap();
                    original.set_reflecting_no_slip(true).unwrap();
                }
            } else {
                original
                    .configure_droplet_population(Some(vec![true, false]))
                    .unwrap();
            }
            for _ in 0..7 {
                if mode == 2 {
                    original.step_carrier_and_droplets(0.00001, None).unwrap();
                } else {
                    original.step(0.00001, None).unwrap();
                }
            }
            let saved = original.checkpoint().unwrap();
            let bytes = saved.to_bytes(1_000_000).unwrap();
            let saved = LiquidCheckpoint::from_bytes(
                &bytes,
                LiquidWireLimits {
                    max_bytes: 1_000_000,
                    max_allocation_bytes: 1_000_000,
                    max_values: 100_000,
                },
            )
            .unwrap();
            assert_eq!(saved, original.checkpoint().unwrap());
            let mut restored = saved.prepare(limits()).unwrap();
            assert_eq!(original, restored);
            for _ in 0..120 {
                if mode == 2 {
                    assert_eq!(
                        original.step_carrier_and_droplets(0.00001, None).unwrap(),
                        restored.step_carrier_and_droplets(0.00001, None).unwrap()
                    );
                } else {
                    assert_eq!(
                        original.step(0.00001, None).unwrap(),
                        restored.step(0.00001, None).unwrap()
                    );
                }
                assert_eq!(
                    original.checkpoint().unwrap(),
                    restored.checkpoint().unwrap()
                );
                assert_eq!(original, restored);
            }
        }
    }
    #[test]
    fn capture_rejects_currently_incompatible_physics_before_emitting_a_record() {
        let mut original = base();
        original
            .configure_transport(
                vec![
                    LiquidField {
                        temperature: 300.,
                        concentration: 0.
                    };
                    2
                ],
                vec![TransportMaterial::default()],
            )
            .unwrap();
        original
            .set_maxwell_fluid(
                0,
                Some(MaxwellFluid {
                    modulus: 1.,
                    relaxation_time: 0.2,
                }),
            )
            .unwrap();
        assert!(original.checkpoint().is_ok());
        original
            .configure_property_response(vec![Some(PropertyResponse {
                reference_temperature: 290.,
                thermal_expansion: 1e-4,
                viscosity_temperature_rate: 0.,
                solute: None,
            })])
            .unwrap();
        assert!(original.step(0.00001, None).is_err());
        assert!(original.checkpoint().is_err());
    }
    #[test]
    fn joint_staging_accepts_valid_mixture_with_overflowing_base_heat_reference() {
        let mut original = base();
        original
            .configure_transport(
                vec![
                    LiquidField {
                        temperature: 0.,
                        concentration: 0.
                    };
                    2
                ],
                vec![TransportMaterial {
                    specific_heat: 1e308,
                    conductivity: 0.,
                    diffusivity: 0.,
                    mixing_group: 0,
                }],
            )
            .unwrap();
        original
            .configure_species(vec!["component".into()], vec![vec![1.]; 2])
            .unwrap();
        original
            .configure_species_heat_capacities(Some(vec![1.]))
            .unwrap();
        original.add_heat(&[10., 20.]).unwrap();
        let saved = original.checkpoint().unwrap();
        let bytes = saved.to_bytes(1_000_000).unwrap();
        let saved = LiquidCheckpoint::from_bytes(
            &bytes,
            LiquidWireLimits {
                max_bytes: 1_000_000,
                max_allocation_bytes: 1_000_000,
                max_values: 100_000,
            },
        )
        .unwrap();
        assert_eq!(saved, original.checkpoint().unwrap());
        // Standalone base-transport admission has the wrong heat reference here.
        let mut standalone = original
            .particle_checkpoint()
            .prepare_base(limits().core)
            .unwrap();
        assert!(
            standalone
                .restore_thermal_checkpoint(saved.thermal.as_ref().unwrap(), 8, 2)
                .is_err()
        );
        let restored = saved.prepare(limits()).unwrap();
        assert_eq!(restored, original);
        assert_eq!(restored.checkpoint().unwrap(), saved);
    }
}
