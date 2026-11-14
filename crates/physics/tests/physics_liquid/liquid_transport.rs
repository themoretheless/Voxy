use physics::liquid::{Config, Error, Liquid, LiquidField, Material, Particle, TransportMaterial};
fn system() -> Liquid {
    Liquid::new(
        vec![
            Particle {
                position: [-0.1, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.1, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 2.0,
                material: 1,
            },
        ],
        vec![Material::WATER, Material::OIL],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap()
}
fn fields() -> Vec<LiquidField> {
    vec![
        LiquidField {
            temperature: 300.0,
            concentration: 0.0,
        },
        LiquidField {
            temperature: 600.0,
            concentration: 1.0,
        },
    ]
}
fn properties(group: u32) -> Vec<TransportMaterial> {
    vec![
        TransportMaterial {
            specific_heat: 2.0,
            conductivity: 1e6,
            diffusivity: 1e6,
            mixing_group: group,
        },
        TransportMaterial {
            specific_heat: 4.0,
            conductivity: 1e6,
            diffusivity: 1e6,
            mixing_group: group,
        },
    ]
}
#[test]
fn heat_and_dissolved_mass_are_conserved_with_unequal_capacities() {
    let mut liquid = system();
    liquid.configure_transport(fields(), properties(1)).unwrap();
    let before = liquid.transport_totals().unwrap().unwrap();
    liquid.step(0.01, None).unwrap();
    let after = liquid.transport_totals().unwrap().unwrap();
    assert!((after.0 - before.0).abs() < 1e-9);
    assert!((after.1 - before.1).abs() < 1e-12);
    for field in liquid.fields().unwrap() {
        assert!((field.temperature - 540.0).abs() < 1e-6);
        assert!((field.concentration - 2.0 / 3.0).abs() < 1e-10);
    }
}
#[test]
fn immiscible_groups_exchange_heat_but_keep_composition_separate() {
    let mut liquid = system();
    liquid.configure_transport(fields(), properties(0)).unwrap();
    liquid.step(0.01, None).unwrap();
    let values = liquid.fields().unwrap();
    assert!(values[0].temperature > 300.0 && values[1].temperature < 600.0);
    assert!(values[0].concentration.abs() < 1e-12);
    assert!((values[1].concentration - 1.0).abs() < 1e-12);
}
#[test]
fn passive_fields_advect_with_particles_without_diffusion() {
    let mut particle = system().particles()[0];
    particle.velocity = [1.0, 0.0, 0.0];
    let mut liquid = Liquid::new(
        vec![particle],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    let value = LiquidField {
        temperature: 330.0,
        concentration: 0.25,
    };
    liquid
        .configure_transport(
            vec![value],
            vec![TransportMaterial {
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid.step(0.01, None).unwrap();
    assert_eq!(liquid.fields().unwrap(), &[value]);
    assert!((liquid.particles()[0].position[0] - particle.position[0] - 0.01).abs() < 1e-12);
}
#[test]
fn failure_after_transport_exchange_rolls_back_fields_and_motion() {
    let original = system();
    let mut liquid = Liquid::new(
        original.particles().to_vec(),
        vec![Material::WATER, Material::OIL],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            gravity: [0.0; 3],
            max_substeps: 1,
            ..Config::default()
        },
    )
    .unwrap();
    liquid.configure_transport(fields(), properties(1)).unwrap();
    let before = liquid.clone();
    assert_eq!(liquid.step(0.1, None), Err(Error::SubstepBudget));
    assert_eq!(liquid, before);
    let mut invalid = fields();
    invalid[0].concentration = 1.1;
    assert_eq!(
        liquid.configure_transport(invalid, properties(1)),
        Err(Error::InvalidTransport)
    );
    assert_eq!(liquid, before);
}
#[test]
fn zero_conductivity_insulates_and_large_rates_do_not_overshoot() {
    let mut liquid = system();
    let mut materials = properties(1);
    materials[0].conductivity = 0.0;
    liquid.configure_transport(fields(), materials).unwrap();
    liquid.step(0.01, None).unwrap();
    let values = liquid.fields().unwrap();
    assert!((values[0].temperature - 300.0).abs() < 1e-12);
    assert!((values[1].temperature - 600.0).abs() < 1e-12);
    assert!(
        values
            .iter()
            .all(|f| (0.0..=1.0).contains(&f.concentration))
    );
}

#[test]
fn connected_cloud_converges_to_mass_weighted_mixture_without_losing_heat() {
    let particles = (0..5)
        .map(|i| Particle {
            position: [f64::from(i) * 0.1, 0.0, 0.0],
            velocity: [0.0; 3],
            mass: f64::from(i + 1),
            material: 0,
        })
        .collect();
    let mut liquid = Liquid::new(
        particles,
        vec![Material::WATER],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    let fields = (0..5)
        .map(|i| LiquidField {
            temperature: 300.0 + f64::from(i) * 50.0,
            concentration: f64::from(i) / 4.0,
        })
        .collect();
    liquid
        .configure_transport(
            fields,
            vec![TransportMaterial {
                specific_heat: 2.0,
                conductivity: 1e12,
                diffusivity: 1e12,
                mixing_group: 1,
            }],
        )
        .unwrap();
    let before = liquid.transport_totals().unwrap().unwrap();
    let mean = before.1 / liquid.mass();
    let mean_temperature = before.0 / (2.0 * liquid.mass());
    for _ in 0..20 {
        liquid.step(0.01, None).unwrap();
        let totals = liquid.transport_totals().unwrap().unwrap();
        assert!((totals.0 - before.0).abs() < 1e-8);
        assert!((totals.1 - before.1).abs() < 1e-12);
        for field in liquid.fields().unwrap() {
            assert!((300.0..=500.0).contains(&field.temperature));
            assert!((0.0..=1.0).contains(&field.concentration));
        }
    }
    for field in liquid.fields().unwrap() {
        assert!((field.concentration - mean).abs() < 1e-8);
        assert!((field.temperature - mean_temperature).abs() < 1e-5);
    }
}

#[test]
fn overflowing_heat_capacity_is_rejected_without_publishing_fields() {
    let mut liquid = system();
    let before = liquid.clone();
    let mut materials = properties(1);
    materials[1].specific_heat = f64::MAX;
    assert_eq!(
        liquid.configure_transport(fields(), materials),
        Err(Error::NumericalFailure)
    );
    assert_eq!(liquid, before);
}
