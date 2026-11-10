use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, ParticleInput, PropertyResponse,
    SaturationCurve, SolutionVaporInterface, SpeciesProperties, TransportMaterial, VaporCell,
    VaporExchangeAccuracy, VaporInterface, ViscosityBlend,
};
fn model(viscosity: ViscosityBlend) -> SpeciesProperties {
    SpeciesProperties {
        components: vec![
            Material {
                rest_density: 1000.0,
                sound_speed: 10.0,
                viscosity: 1.0,
            },
            Material {
                rest_density: 2000.0,
                sound_speed: 20.0,
                viscosity: 100.0,
            },
        ],
        viscosity,
    }
}
fn fluid(rows: Vec<Vec<f64>>) -> Liquid {
    let mut l = Liquid::new(
        rows.iter()
            .enumerate()
            .map(|(i, _)| Particle {
                position: [if i == 0 { 0.0 } else { 0.4 }, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            })
            .collect(),
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            ..Config::default()
        },
    )
    .unwrap();
    l.configure_transport(
        vec![
            LiquidField {
                temperature: 10.0,
                concentration: 0.0
            };
            rows.len()
        ],
        vec![TransportMaterial {
            specific_heat: 2.0,
            conductivity: 0.0,
            diffusivity: 20.0,
            mixing_group: 0,
        }],
    )
    .unwrap();
    l.configure_species(vec!["solvent".into(), "solute".into()], rows)
        .unwrap();
    l
}
#[test]
fn coefficients_match_additive_volume_wood_and_viscosity_closures() {
    let p = model(ViscosityBlend::Linear).evaluate(&[0.5, 0.5]).unwrap();
    let volume = 0.5 / 1000.0 + 0.5 / 2000.0;
    let density = 1.0 / volume;
    let first_volume = (0.5 / 1000.0) / volume;
    let second_volume = (0.5 / 2000.0) / volume;
    let compliance = first_volume / (1000.0 * 10.0 * 10.0) + second_volume / (2000.0 * 20.0 * 20.0);
    let sound = (1.0_f64 / (density * compliance)).sqrt();
    assert!((p.rest_density - density).abs() < 1e-10);
    assert!((p.sound_speed - sound).abs() < 1e-12);
    assert!((p.viscosity - 50.5).abs() < 1e-12);
    assert!(
        (model(ViscosityBlend::Logarithmic)
            .evaluate(&[0.5, 0.5])
            .unwrap()
            .viscosity
            - 10.0)
            .abs()
            < 1e-12
    );
    for (i, row) in [[1.0, 0.0], [0.0, 1.0]].iter().enumerate() {
        let p = model(ViscosityBlend::Linear).evaluate(row).unwrap();
        let expected = model(ViscosityBlend::Linear).components[i];
        assert!((p.rest_density - expected.rest_density).abs() < 1e-10);
        assert!((p.sound_speed - expected.sound_speed).abs() < 1e-12);
    }
}
#[test]
fn transported_composition_changes_mechanics_and_respects_thermal_response_order() {
    for symmetric in [false, true] {
        let mut l = fluid(vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
        let closure = model(ViscosityBlend::Linear);
        l.configure_species_properties(Some(closure.clone()))
            .unwrap();
        let before = l.species_totals().unwrap().unwrap();
        if symmetric {
            l.set_viscous_heating(true).unwrap();
            l.step_symmetric_free(0.001).unwrap();
        } else {
            l.step(0.001, None).unwrap();
        }
        let rows = l.species_fractions().unwrap();
        assert!(rows[0][1] > 0.0 && rows[1][0] > 0.0);
        let properties = l.effective_materials().unwrap();
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(properties[i], closure.evaluate(row).unwrap());
        }
        for (a, b) in l.species_totals().unwrap().unwrap().iter().zip(before) {
            assert!((a - b).abs() < 1e-12);
        }
    }
    let mut l = fluid(vec![vec![0.5, 0.5]]);
    l.configure_species_properties(Some(model(ViscosityBlend::Logarithmic)))
        .unwrap();
    l.configure_property_response(vec![Some(PropertyResponse {
        reference_temperature: 10.0,
        thermal_expansion: 0.01,
        viscosity_temperature_rate: 0.1,
        solute: None,
    })])
    .unwrap();
    l.add_heat(&[20.0]).unwrap();
    let p = l.effective_materials().unwrap()[0];
    let base = model(ViscosityBlend::Logarithmic)
        .evaluate(&[0.5, 0.5])
        .unwrap();
    assert!((p.rest_density - base.rest_density / 1.1).abs() < 1e-10);
    assert!((p.viscosity - base.viscosity * (-1.0_f64).exp()).abs() < 1e-12);
}
#[test]
fn selective_evaporation_increases_solution_density_and_viscosity() {
    let mut l = fluid(vec![vec![0.5, 0.5]]);
    l.configure_species_properties(Some(model(ViscosityBlend::Linear)))
        .unwrap();
    let before = l.effective_materials().unwrap()[0];
    let interface = SolutionVaporInterface {
        interface: VaporInterface {
            curve: SaturationCurve {
                reference_temperature: 10.0,
                reference_pressure: 10.0,
                latent_heat: 100.0,
                vapor_gas_constant: 1.0,
                min_temperature: 5.0,
                max_temperature: 20.0,
            },
            area: 0.1,
            accommodation: 0.01,
        },
        solvent: 0,
        molar_masses: vec![1.0, 2.0],
    };
    let mut vapor = VaporCell {
        mass: 0.05,
        volume: 1.0,
        temperature: 10.0,
        velocity: [0.0; 3],
        specific_heat_cv: 1.0,
    };
    l.exchange_solution_vapor(
        0,
        &mut vapor,
        &interface,
        1.0,
        VaporExchangeAccuracy::default(),
    )
    .unwrap();
    let after = l.effective_materials().unwrap()[0];
    assert!(after.rest_density > before.rest_density);
    assert!(after.viscosity > before.viscosity);
    assert!((l.species_totals().unwrap().unwrap()[1] - 0.5).abs() < 1e-12);
}
#[test]
fn sources_replacement_and_configuration_conflicts_are_atomic() {
    let mut l = fluid(vec![vec![0.5, 0.5]]);
    let closure = model(ViscosityBlend::Linear);
    l.configure_species_properties(Some(closure.clone()))
        .unwrap();
    let before = l.clone();
    assert!(
        l.configure_species(vec!["x".into()], vec![vec![1.0]])
            .is_err()
    );
    assert_eq!(l, before);
    assert!(
        l.configure_property_response(vec![Some(PropertyResponse {
            reference_temperature: 10.0,
            thermal_expansion: 0.0,
            viscosity_temperature_rate: 0.0,
            solute: Some(Material::WATER)
        })])
        .is_err()
    );
    assert_eq!(l, before);
    let mut bad = model(ViscosityBlend::Logarithmic);
    bad.components[1].viscosity = 0.0;
    assert!(l.configure_species_properties(Some(bad)).is_err());
    assert_eq!(l, before);
    let source = ParticleInput {
        particle: l.particles()[0],
        field: Some(l.fields().unwrap()[0]),
        phase_fraction: None,
    };
    l.exchange_particles_with_species(&[0], &[source], &[vec![0.25, 0.75]])
        .unwrap();
    assert_eq!(
        l.effective_materials().unwrap()[0],
        closure.evaluate(&[0.25, 0.75]).unwrap()
    );
    l.configure_transport(
        vec![l.fields().unwrap()[0]],
        vec![TransportMaterial {
            specific_heat: 2.0,
            ..TransportMaterial::default()
        }],
    )
    .unwrap();
    assert_eq!(
        l.effective_materials().unwrap()[0],
        closure.evaluate(&[0.25, 0.75]).unwrap()
    );
    l.configure_species_properties(None).unwrap();
    l.configure_species(vec!["x".into()], vec![vec![1.0]])
        .unwrap();
    assert_eq!(l.effective_materials().unwrap()[0], Material::WATER);
}

#[test]
fn composition_changes_actual_viscous_damping_without_energy_creation() {
    let mut losses = Vec::new();
    for row in [vec![1.0, 0.0], vec![0.0, 1.0]] {
        let mut l = fluid(vec![row.clone(), row.clone()]);
        let mut closure = model(ViscosityBlend::Linear);
        closure.components[1].rest_density = 1000.0;
        closure.components[1].sound_speed = 10.0;
        l.configure_species_properties(Some(closure)).unwrap();
        let sources = l
            .particles()
            .iter()
            .enumerate()
            .map(|(i, p)| ParticleInput {
                particle: Particle {
                    velocity: [if i == 0 { 1.0 } else { -1.0 }, 0.0, 0.0],
                    ..*p
                },
                field: Some(l.fields().unwrap()[i]),
                phase_fraction: None,
            })
            .collect::<Vec<_>>();
        l.exchange_particles_with_species(&[0, 1], &sources, &[row.clone(), row])
            .unwrap();
        l.set_viscous_heating(true).unwrap();
        let before = l.transport_totals().unwrap().unwrap().0;
        l.relax_viscosity(1.0).unwrap();
        let heat = l.transport_totals().unwrap().unwrap().0 - before;
        let kinetic = l
            .particles()
            .iter()
            .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>();
        assert!((heat + kinetic - 1.0).abs() < 1e-10);
        assert!((l.particles()[0].velocity[0] + l.particles()[1].velocity[0]).abs() < 1e-12);
        losses.push(heat);
    }
    assert!(losses[0] > 0.0 && losses[1] > losses[0]);
}
