#![allow(clippy::float_cmp)] // Exact accounting and unchanged-state assertions.
use physics::liquid::{
    Config, Error, Liquid, LiquidField, Material, Particle, ParticleInput, PhaseChange,
    TransportMaterial,
};
fn particle(mass: f64) -> Particle {
    Particle {
        position: [mass, 0.0, 0.0],
        velocity: [2.0, 0.0, 0.0],
        mass,
        material: 0,
    }
}
fn fluid() -> Liquid {
    Liquid::new(
        vec![particle(1.0), particle(2.0)],
        vec![Material::WATER],
        Config {
            max_particles: 3,
            ..Config::default()
        },
    )
    .unwrap()
}
fn source(mass: f64) -> ParticleInput {
    ParticleInput {
        particle: particle(mass),
        field: None,
        phase_fraction: None,
    }
}
#[test]
fn replacement_reports_mass_momentum_energy_and_preserves_survivor_order() {
    let mut fluid = fluid();
    let report = fluid.exchange_particles(&[0], &[source(3.0)]).unwrap();
    assert_eq!(report.removed.mass, 1.0);
    assert_eq!(report.added.mass, 3.0);
    assert_eq!(report.removed.momentum, [2.0, 0.0, 0.0]);
    assert_eq!(report.added.kinetic_energy, 6.0);
    assert_eq!(fluid.mass(), 3.0 + report.added.mass - report.removed.mass);
    assert_eq!(fluid.particles()[0].mass, 2.0);
    assert_eq!(fluid.particles()[1].mass, 3.0);
    assert_eq!(report.added.thermal_energy, None);
}
#[test]
fn sources_and_drains_keep_temperature_solute_and_latent_energy_attached() {
    let mut fluid = fluid();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 10.0,
                    concentration: 0.1,
                },
                LiquidField {
                    temperature: 10.0,
                    concentration: 0.7,
                },
            ],
            vec![TransportMaterial {
                specific_heat: 1.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    fluid
        .configure_phase_change(
            vec![Some(PhaseChange {
                temperature: 10.0,
                latent_heat: 100.0,
                high_phase: Material::OIL,
            })],
            vec![0.2, 0.8],
        )
        .unwrap();
    let initial = fluid.transport_totals().unwrap().unwrap();
    let input = ParticleInput {
        particle: particle(3.0),
        field: Some(LiquidField {
            temperature: 10.0,
            concentration: 0.5,
        }),
        phase_fraction: Some(0.5),
    };
    let report = fluid.exchange_particles(&[0], &[input]).unwrap();
    let totals = fluid.transport_totals().unwrap().unwrap();
    assert!(
        (totals.0
            - (initial.0 + report.added.thermal_energy.unwrap()
                - report.removed.thermal_energy.unwrap()))
        .abs()
            < 1e-10
    );
    assert!(
        (totals.1
            - (initial.1 + report.added.dissolved_mass.unwrap()
                - report.removed.dissolved_mass.unwrap()))
        .abs()
            < 1e-10
    );
    assert_eq!(fluid.phase_fractions().unwrap(), &[0.8, 0.5]);
    assert_eq!(fluid.fields().unwrap()[0].concentration, 0.7);
    fluid.exchange_particles(&[1, 0], &[]).unwrap();
    assert_eq!(fluid.mass(), 0.0);
    assert!(fluid.fields().unwrap().is_empty());
    fluid.exchange_particles(&[], &[input]).unwrap();
    assert_eq!(fluid.mass(), 3.0);
    fluid.step(0.01, None).unwrap();
}
#[test]
fn invalid_indices_budget_and_source_roll_back_removal() {
    let mut fluid = fluid();
    let before = fluid.clone();
    for indices in [&[0, 0][..], &[2][..]] {
        assert_eq!(
            fluid.exchange_particles(indices, &[]),
            Err(Error::InvalidParticle)
        );
        assert_eq!(fluid, before);
    }
    assert_eq!(
        fluid.exchange_particles(&[], &[source(1.0), source(1.0)]),
        Err(Error::ParticleBudget)
    );
    assert_eq!(fluid, before);
    assert_eq!(
        fluid.exchange_particles(&[0], &[source(-1.0)]),
        Err(Error::InvalidParticle)
    );
    assert_eq!(fluid, before);
    let mut invalid = source(1.0);
    invalid.field = Some(LiquidField {
        temperature: 10.0,
        concentration: 0.0,
    });
    assert_eq!(
        fluid.exchange_particles(&[0], &[invalid]),
        Err(Error::InvalidTransport)
    );
    assert_eq!(fluid, before);
}
#[test]
fn invalid_thermal_and_phase_sources_are_atomic_after_a_valid_removal() {
    let mut fluid = fluid();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 10.0,
                    concentration: 0.0
                };
                2
            ],
            vec![TransportMaterial::default()],
        )
        .unwrap();
    fluid
        .configure_phase_change(
            vec![Some(PhaseChange {
                temperature: 10.0,
                latent_heat: 100.0,
                high_phase: Material::OIL,
            })],
            vec![0.0; 2],
        )
        .unwrap();
    let before = fluid.clone();
    let invalid = ParticleInput {
        particle: particle(1.0),
        field: Some(LiquidField {
            temperature: 5.0,
            concentration: 0.2,
        }),
        phase_fraction: Some(0.5),
    };
    assert_eq!(
        fluid.exchange_particles(&[0], &[invalid]),
        Err(Error::InvalidPhaseChange)
    );
    assert_eq!(fluid, before);
    let mut missing = invalid;
    missing.field = None;
    assert_eq!(
        fluid.exchange_particles(&[0], &[missing]),
        Err(Error::InvalidTransport)
    );
    assert_eq!(fluid, before);
    let mut overflow = invalid;
    overflow.particle = particle(1e308);
    overflow.particle.velocity = [0.0; 3];
    overflow.particle.position = [0.0; 3];
    overflow.field = Some(LiquidField {
        temperature: 0.0,
        concentration: 0.0,
    });
    overflow.phase_fraction = Some(0.0);
    assert_eq!(
        fluid.exchange_particles(&[0], &[overflow]),
        Err(Error::NumericalFailure)
    );
    assert_eq!(fluid, before);
}
