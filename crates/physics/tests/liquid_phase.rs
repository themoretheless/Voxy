#![allow(clippy::float_cmp)] // Exact plateau and atomic-state assertions.
use physics::liquid::Particle;
use physics::liquid::{
    Config, Error, Liquid, LiquidField, Material, PhaseChange, TransportMaterial,
};
fn model() -> PhaseChange {
    PhaseChange {
        temperature: 10.0,
        latent_heat: 100.0,
        high_phase: Material {
            rest_density: 500.0,
            sound_speed: 2.0,
            viscosity: 0.2,
        },
    }
}
fn fluid(temperatures: &[f64], fractions: &[f64], conductivity: f64) -> Liquid {
    let particles = temperatures
        .iter()
        .enumerate()
        .map(|(index, _)| Particle {
            position: [if index == 0 { 0.0 } else { 0.2 }, 0.0, 0.0],
            velocity: [0.0; 3],
            mass: if index == 0 { 1.0 } else { 2.0 },
            material: 0,
        })
        .collect();
    let mut liquid = Liquid::new(
        particles,
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 2.0,
            viscosity: 0.0,
        }],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    liquid
        .configure_transport(
            temperatures
                .iter()
                .map(|temperature| LiquidField {
                    temperature: *temperature,
                    concentration: 0.25,
                })
                .collect(),
            vec![TransportMaterial {
                specific_heat: 10.0,
                conductivity,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid
        .configure_phase_change(vec![Some(model())], fractions.to_vec())
        .unwrap();
    liquid
}
fn heat(liquid: &Liquid) -> f64 {
    liquid.transport_totals().unwrap().unwrap().0
}
#[test]
fn heating_and_cooling_use_latent_plateau_and_are_reversible() {
    let mut liquid = fluid(&[5.0], &[0.0], 0.0);
    liquid.add_heat(&[100.0]).unwrap();
    assert_eq!(liquid.fields().unwrap()[0].temperature, 10.0);
    assert_eq!(liquid.phase_fractions().unwrap()[0], 0.5);
    assert_eq!(heat(&liquid), 150.0);
    let mechanical = liquid.effective_materials().unwrap()[0];
    assert!((mechanical.rest_density - 2000.0 / 3.0).abs() < 1e-10);
    assert_eq!(mechanical.viscosity, 0.1);
    liquid.add_heat(&[100.0]).unwrap();
    assert_eq!(liquid.fields().unwrap()[0].temperature, 15.0);
    assert_eq!(liquid.phase_fractions().unwrap()[0], 1.0);
    liquid.add_heat(&[-200.0]).unwrap();
    assert_eq!(liquid.fields().unwrap()[0].temperature, 5.0);
    assert_eq!(liquid.phase_fractions().unwrap()[0], 0.0);
}
#[test]
fn conductive_transition_conserves_total_enthalpy_without_temperature_overshoot() {
    let mut liquid = fluid(&[30.0, 10.0], &[1.0, 0.0], 1e12);
    let before = heat(&liquid);
    let solute = liquid.transport_totals().unwrap().unwrap().1;
    for _ in 0..20 {
        liquid.step(0.01, None).unwrap();
    }
    let fields = liquid.fields().unwrap();
    assert!(
        fields
            .iter()
            .all(|field| field.temperature >= 10.0 && field.temperature <= 30.0)
    );
    assert!((fields[0].temperature - fields[1].temperature).abs() < 1e-5);
    assert!(liquid.phase_fractions().unwrap()[1] > 0.99);
    assert!((heat(&liquid) - before).abs() < 1e-8);
    assert_eq!(liquid.transport_totals().unwrap().unwrap().1, solute);
    assert_eq!(liquid.mass(), 3.0);
}
#[test]
fn cooling_releases_latent_heat_into_a_colder_particle() {
    let mut liquid = fluid(&[10.0, 0.0], &[1.0, 0.0], 1e10);
    // Same model here: initially cold material is below transition and the hot particle condenses.
    let before = heat(&liquid);
    liquid.step(0.1, None).unwrap();
    assert!(liquid.phase_fractions().unwrap()[0] < 0.01);
    assert!(liquid.fields().unwrap()[1].temperature > 0.0);
    assert!((heat(&liquid) - before).abs() < 1e-8);
}
#[test]
fn invalid_phase_state_and_negative_energy_leave_all_state_unchanged() {
    let mut liquid = fluid(&[5.0], &[0.0], 0.0);
    let before = liquid.clone();
    assert_eq!(
        liquid.configure_phase_change(vec![Some(model())], vec![0.5]),
        Err(Error::InvalidPhaseChange)
    );
    assert_eq!(liquid, before);
    assert_eq!(liquid.add_heat(&[-51.0]), Err(Error::NumericalFailure));
    assert_eq!(liquid, before);
    assert_eq!(
        liquid.add_heat(&[f64::INFINITY]),
        Err(Error::InvalidPhaseChange)
    );
    assert_eq!(liquid, before);
}
#[test]
fn zero_conductivity_and_same_plateau_temperature_do_not_exchange_heat() {
    let mut liquid = fluid(&[10.0, 10.0], &[0.2, 0.8], 1e12);
    let fractions = liquid.phase_fractions().unwrap().to_vec();
    liquid.step(0.01, None).unwrap();
    assert_eq!(liquid.phase_fractions().unwrap(), fractions);
    let mut insulated = fluid(&[20.0, 0.0], &[1.0, 0.0], 0.0);
    let fields = insulated.fields().unwrap().to_vec();
    insulated.step(0.01, None).unwrap();
    assert_eq!(insulated.fields().unwrap(), fields);
}

#[test]
fn heat_injection_outside_property_domain_rolls_back_phase_and_temperature() {
    let mut liquid = fluid(&[10.0], &[0.5], 0.0);
    liquid
        .configure_property_response(vec![Some(physics::liquid::PropertyResponse {
            reference_temperature: 10.0,
            thermal_expansion: -0.2,
            viscosity_temperature_rate: 0.0,
            solute: None,
        })])
        .unwrap();
    let before = liquid.clone();
    assert_eq!(
        liquid.add_heat(&[100.0]),
        Err(Error::InvalidPropertyResponse)
    );
    assert_eq!(liquid, before);
}
#[test]
fn failed_outer_step_restores_already_exchanged_latent_heat() {
    let source = fluid(&[30.0, 10.0], &[1.0, 0.0], 1e8);
    let mut liquid = Liquid::new(
        source.particles().to_vec(),
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 200.0,
            viscosity: 0.0,
        }],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            max_substeps: 1,
            ..Config::default()
        },
    )
    .unwrap();
    liquid
        .configure_transport(
            source.fields().unwrap().to_vec(),
            vec![TransportMaterial {
                specific_heat: 10.0,
                conductivity: 1e8,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid
        .configure_phase_change(vec![Some(model())], vec![1.0, 0.0])
        .unwrap();
    let before = liquid.clone();
    assert_eq!(liquid.step(0.1, None), Err(Error::SubstepBudget));
    assert_eq!(liquid, before);
}

#[test]
fn implicit_heat_exchange_converges_toward_analytic_sensible_heat_solution() {
    let mut coarse = fluid(&[5.0, 0.0], &[0.0, 0.0], 1e4);
    let mut fine = coarse.clone();
    coarse.step(0.1, None).unwrap();
    for _ in 0..20 {
        fine.step(0.005, None).unwrap();
    }
    let conductance = 1e4_f64 * (1.0_f64 / 1000.0).powf(2.0 / 3.0) * 0.8;
    let analytic = 5.0 / 3.0 + 10.0 / 3.0 * (-conductance * (1.0 / 10.0 + 1.0 / 20.0) * 0.1).exp();
    let coarse_error = (coarse.fields().unwrap()[0].temperature - analytic).abs();
    let fine_error = (fine.fields().unwrap()[0].temperature - analytic).abs();
    assert!(fine_error < coarse_error / 5.0);
    assert!((heat(&fine) - 50.0).abs() < 1e-9);
}

#[test]
fn pressure_and_symmetric_viscous_heating_drive_latent_transition_with_energy_balance() {
    use physics::liquid::{Formulation, ViscousIntegrator};
    let mut liquid = Liquid::new(
        vec![
            Particle {
                position: [-0.1, 0.0, 0.0],
                velocity: [1.0, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.1, 0.0, 0.0],
                velocity: [-1.0, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
        ],
        vec![Material {
            rest_density: 0.8,
            sound_speed: 1.0,
            viscosity: 0.5,
        }],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    liquid.set_formulation(Formulation::RestVolumeWendland);
    liquid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 10.0,
                    concentration: 0.25
                };
                2
            ],
            vec![TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                diffusivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid
        .configure_phase_change(
            vec![Some(PhaseChange {
                temperature: 10.0,
                latent_heat: 0.2,
                high_phase: Material {
                    rest_density: 0.7,
                    sound_speed: 1.0,
                    viscosity: 0.02,
                },
            })],
            vec![0.1; 2],
        )
        .unwrap();
    liquid.set_pressure_work(true).unwrap();
    liquid.set_viscous_heating(true).unwrap();
    liquid
        .set_viscous_integrator(ViscousIntegrator::Symmetric)
        .unwrap();
    assert!(
        liquid
            .diagnostics()
            .unwrap()
            .densities
            .iter()
            .all(|rho| *rho > 0.8)
    );
    let initial = kinetic(&liquid) + heat(&liquid);
    let dissolved = liquid.transport_totals().unwrap().unwrap().1;
    for _ in 0..200 {
        liquid.step(0.0001, None).unwrap();
        assert!((kinetic(&liquid) + heat(&liquid) - initial).abs() < 1e-10);
        assert_eq!(liquid.transport_totals().unwrap().unwrap().1, dissolved);
        assert!(
            (liquid.particles()[0].velocity[0] + liquid.particles()[1].velocity[0]).abs() < 1e-12
        );
        for (field, fraction) in liquid
            .fields()
            .unwrap()
            .iter()
            .zip(liquid.phase_fractions().unwrap())
        {
            assert!((0.0..=1.0).contains(fraction));
            if *fraction > 0.0 && *fraction < 1.0 {
                assert_eq!(field.temperature, 10.0);
            }
        }
    }
    assert!(liquid.phase_fractions().unwrap().iter().any(|f| *f > 0.2));
    assert!(liquid.effective_materials().unwrap()[0].viscosity < 0.45);
}

fn kinetic(f: &Liquid) -> f64 {
    f.particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}

#[test]
fn reservoir_crosses_latent_plateau_with_exact_heating_and_cooling_times() {
    let mut liquid = fluid(&[5.0], &[0.0], 0.0);
    let duration = 1.5_f64.ln() + 1.0 + 0.5;
    let initial = heat(&liquid);
    let input = liquid
        .exchange_reservoir_heat(duration, 20.0, &[10.0])
        .unwrap();
    let expected = 20.0 - 10.0 * (-0.5_f64).exp();
    assert!((liquid.fields().unwrap()[0].temperature - expected).abs() < 1e-12);
    assert_eq!(liquid.phase_fractions().unwrap()[0], 1.0);
    assert!((heat(&liquid) - initial - input).abs() < 1e-12);
    let before = liquid.clone();
    let transition = (expected / 10.0).ln() + 1.0 + 0.5;
    let output = liquid
        .exchange_reservoir_heat(transition, 0.0, &[10.0])
        .unwrap();
    assert!((liquid.fields().unwrap()[0].temperature - 10.0 * (-0.5_f64).exp()).abs() < 1e-12);
    assert_eq!(liquid.phase_fractions().unwrap()[0], 0.0);
    assert!(output < 0.0);
    assert!((heat(&liquid) - heat(&before) - output).abs() < 1e-12);
}

#[test]
fn reservoir_exchange_is_partition_independent_and_leaves_motion_unchanged() {
    let base = fluid(&[5.0, 10.0], &[0.0, 0.5], 0.0);
    let mut one = base.clone();
    let mut many = base.clone();
    let whole = one
        .exchange_reservoir_heat(2.0, 20.0, &[10.0, 20.0])
        .unwrap();
    let mut sum = 0.0;
    for _ in 0..100 {
        sum += many
            .exchange_reservoir_heat(0.02, 20.0, &[10.0, 20.0])
            .unwrap();
    }
    assert!((sum - whole).abs() < 1e-10);
    assert_eq!(one.particles(), base.particles());
    assert_eq!(many.particles(), base.particles());
    for (a, b) in one.fields().unwrap().iter().zip(many.fields().unwrap()) {
        assert!((a.temperature - b.temperature).abs() < 1e-11);
        assert_eq!(a.concentration, b.concentration);
    }
    let before = one.clone();
    for (dt, temp, coefficients) in [
        (0.0, 20.0, vec![10.0, 20.0]),
        (1.0, -1.0, vec![10.0, 20.0]),
        (1.0, 20.0, vec![10.0]),
        (1.0, 20.0, vec![-1.0, 20.0]),
    ] {
        assert!(
            one.exchange_reservoir_heat(dt, temp, &coefficients)
                .is_err()
        );
        assert_eq!(one, before);
    }
    assert_eq!(
        one.exchange_reservoir_heat(1.0, 20.0, &[0.0; 2]).unwrap(),
        0.0
    );
    assert_eq!(one, before);
}

#[test]
fn reservoir_property_failure_rolls_back_and_plateau_equilibrium_has_zero_flux() {
    use physics::liquid::PropertyResponse;
    let mut plateau = fluid(&[10.0], &[0.5], 0.0);
    let before = plateau.clone();
    assert_eq!(
        plateau.exchange_reservoir_heat(1.0, 10.0, &[1e10]).unwrap(),
        0.0
    );
    assert_eq!(plateau, before);
    let mut liquid = fluid(&[5.0, 5.0], &[0.0, 0.0], 0.0);
    liquid
        .configure_property_response(vec![Some(PropertyResponse {
            reference_temperature: 5.0,
            thermal_expansion: -0.1,
            viscosity_temperature_rate: 0.0,
            solute: None,
        })])
        .unwrap();
    let before = liquid.clone();
    assert_eq!(
        liquid.exchange_reservoir_heat(10.0, 40.0, &[10.0, 20.0]),
        Err(Error::InvalidPropertyResponse)
    );
    assert_eq!(liquid, before);
}

#[test]
fn reservoir_total_enthalpy_overflow_is_atomic_even_when_each_particle_is_finite() {
    let mut liquid = fluid(&[f64::MAX / 40.0; 2], &[1.0; 2], 0.0);
    let before = liquid.clone();
    assert_eq!(
        liquid.exchange_reservoir_heat(1.0, f64::MAX / 25.0, &[1e100; 2]),
        Err(Error::NumericalFailure)
    );
    assert_eq!(liquid, before);
}
