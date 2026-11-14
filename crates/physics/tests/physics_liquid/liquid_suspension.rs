use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, PhaseChange, TransportMaterial,
};
use physics::suspension::Particle as Grain;
fn fixture(thermal: bool) -> (Liquid, Vec<Grain>) {
    let mut fluid = Liquid::new(
        vec![Particle {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 1e-10,
            material: 0,
        }],
        vec![Material {
            rest_density: 1000.,
            sound_speed: 2.,
            viscosity: 0.001,
        }],
        Config::default(),
    )
    .unwrap();
    if thermal {
        fluid
            .configure_transport(
                vec![LiquidField {
                    temperature: 10.,
                    concentration: 0.,
                }],
                vec![TransportMaterial {
                    specific_heat: 1.,
                    conductivity: 0.,
                    ..TransportMaterial::default()
                }],
            )
            .unwrap();
        fluid
            .configure_phase_change(
                vec![Some(PhaseChange {
                    temperature: 10.,
                    latent_heat: 0.001,
                    high_phase: Material {
                        rest_density: 1000.,
                        sound_speed: 2.,
                        viscosity: 0.001,
                    },
                })],
                vec![0.],
            )
            .unwrap();
    }
    (
        fluid,
        vec![Grain::new(1e-6, 2000., [0.; 3], [0.01, 0., 0.]).unwrap()],
    )
}
fn kinetic(fluid: &Liquid, grains: &[Grain]) -> f64 {
    fluid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum::<f64>()
        + grains
            .iter()
            .map(|p| 0.5 * p.mass_kg() * p.velocity_m_s().iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
}
#[test]
fn drag_feedback_conserves_momentum_and_heats_latent_plateau() {
    let (mut fluid, mut grains) = fixture(true);
    let momentum = grains[0].mass_kg() * grains[0].velocity_m_s()[0];
    let old_k = kinetic(&fluid, &grains);
    let old_heat = fluid.transport_totals().unwrap().unwrap().0;
    let report = fluid
        .exchange_suspension_cell(0, &mut grains, 1e-6)
        .unwrap();
    let accepted_p = fluid.particles()[0].mass * fluid.particles()[0].velocity[0]
        + grains[0].mass_kg() * grains[0].velocity_m_s()[0];
    assert!((accepted_p - momentum).abs() < 1e-12 * momentum);
    assert!(report.viscous_heat_j > 0. && report.numerical_loss_j > 0.);
    let added = fluid.transport_totals().unwrap().unwrap().0 - old_heat;
    assert!((added - report.viscous_heat_j).abs() < 1e-23);
    assert!((kinetic(&fluid, &grains) - old_k + added + report.numerical_loss_j).abs() < 1e-23);
    assert_eq!(fluid.fields().unwrap()[0].temperature, 10.);
    assert!((fluid.phase_fractions().unwrap()[0] - report.viscous_heat_j / 1e-13).abs() < 1e-10);
}
#[test]
fn invalid_carrier_or_regime_preserves_both_inventories() {
    for thermal in [false, true] {
        let (mut fluid, mut grains) = fixture(thermal);
        let old_fluid = fluid.clone();
        let old_grains = grains.clone();
        let result =
            fluid.exchange_suspension_cell(0, &mut grains, if thermal { -1. } else { 1e-6 });
        assert!(result.is_err());
        assert_eq!(fluid, old_fluid);
        for (grain, old) in grains.iter().zip(&old_grains) {
            assert_eq!(grain.position_m(), old.position_m());
            assert_eq!(grain.velocity_m_s(), old.velocity_m_s());
            assert_eq!(grain.mass_kg(), old.mass_kg());
        }
    }
}

fn spatial() -> Liquid {
    let mut fluid = Liquid::new(
        (0..2)
            .map(|i| Particle {
                position: [i as f64 * 0.001, 0., 0.],
                velocity: [0.01, 0., 0.],
                mass: 1e-10,
                material: 0,
            })
            .collect(),
        vec![Material {
            rest_density: 1000.,
            sound_speed: 2.,
            viscosity: 0.001,
        }],
        Config {
            smoothing_radius: 0.002,
            particle_radius: 0.0001,
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 10.,
                    concentration: 0.
                };
                2
            ],
            vec![TransportMaterial {
                specific_heat: 1.,
                conductivity: 0.,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    fluid
}
#[test]
fn spatial_drift_reassigns_carrier_without_losing_particle_mass() {
    let mut fluid = spatial();
    let mut cloud = vec![Grain::new(1e-6, 2000., [0.0004, 0., 0.], [0.01, 0., 0.]).unwrap()];
    let mass = cloud[0].mass_kg();
    let (first, _) = fluid.advance_suspension(&mut cloud, 0.02).unwrap();
    assert_eq!(first, vec![0]);
    assert!((cloud[0].position_m()[0] - 0.0006).abs() < 1e-15);
    let (second, _) = fluid.advance_suspension(&mut cloud, 0.02).unwrap();
    assert_eq!(second, vec![1]);
    assert_eq!(cloud[0].mass_kg(), mass);
    let inventory = fluid.suspension_inventory(&cloud).unwrap();
    assert_eq!(inventory[0].solid_mass_kg, 0.);
    assert_eq!(inventory[1].solid_mass_kg, mass);
    assert_eq!(inventory.iter().map(|c| c.solid_mass_kg).sum::<f64>(), mass);
}
#[test]
fn later_cell_failure_restores_prior_cell_feedback_and_drift() {
    let mut fluid = spatial();
    let mut cloud = vec![
        Grain::new(1e-6, 2000., [0., 0., 0.], [0.02, 0., 0.]).unwrap(),
        Grain::new(1e-6, 2000., [0.001, 0., 0.], [1., 0., 0.]).unwrap(),
    ];
    let old_fluid = fluid.clone();
    let old = cloud.clone();
    assert!(fluid.advance_suspension(&mut cloud, 1e-6).is_err());
    assert_eq!(fluid, old_fluid);
    for (grain, old) in cloud.iter().zip(old) {
        assert_eq!(grain.position_m(), old.position_m());
        assert_eq!(grain.velocity_m_s(), old.velocity_m_s());
    }
}
#[test]
fn spatial_two_cell_feedback_balances_global_energy_and_momentum() {
    let mut fluid = spatial();
    let mut cloud = vec![
        Grain::new(1e-6, 2000., [0., 0., 0.], [0.02, 0., 0.]).unwrap(),
        Grain::new(1e-6, 2000., [0.001, 0., 0.], [0., 0., 0.]).unwrap(),
    ];
    let momentum = |fluid: &Liquid, cloud: &[Grain]| {
        fluid
            .particles()
            .iter()
            .map(|p| p.mass * p.velocity[0])
            .sum::<f64>()
            + cloud
                .iter()
                .map(|p| p.mass_kg() * p.velocity_m_s()[0])
                .sum::<f64>()
    };
    let old_p = momentum(&fluid, &cloud);
    let old_k = kinetic(&fluid, &cloud);
    let old_heat = fluid.transport_totals().unwrap().unwrap().0;
    let (cells, report) = fluid.advance_suspension(&mut cloud, 1e-6).unwrap();
    assert_eq!(cells, vec![0, 1]);
    assert!((momentum(&fluid, &cloud) - old_p).abs() < 1e-12 * old_p);
    let added_heat = fluid.transport_totals().unwrap().unwrap().0 - old_heat;
    assert!((added_heat - report.viscous_heat_j).abs() < 1e-23);
    assert!((kinetic(&fluid, &cloud) - old_k + added_heat + report.numerical_loss_j).abs() < 1e-23);
    assert!(fluid.particles()[0].velocity[0] > 0.01);
    assert!(fluid.particles()[1].velocity[0] < 0.01);
}
#[test]
fn joint_free_interval_advects_fluid_and_cloud_and_preserves_outer_transaction() {
    let (base, _) = fixture(true);
    let mut fluid = Liquid::new(
        vec![Particle {
            position: [0.; 3],
            velocity: [0.01, 0., 0.],
            mass: 1e-10,
            material: 0,
        }],
        vec![Material {
            rest_density: 1000.,
            sound_speed: 2.,
            viscosity: 0.001,
        }],
        Config {
            gravity: [0.; 3],
            max_substeps: 4,
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_transport(
            base.fields().unwrap().to_vec(),
            vec![TransportMaterial {
                specific_heat: 1.,
                conductivity: 0.,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    fluid.set_viscous_heating(true).unwrap();
    fluid.set_pressure_work(true).unwrap();
    let mut cloud = vec![Grain::new(1e-6, 2000., [0.; 3], [0.01, 0., 0.]).unwrap()];
    let (stats, _) = fluid.step_suspension_free(&mut cloud, 0.01, 4).unwrap();
    assert!(stats.substeps >= 4);
    assert!((fluid.particles()[0].position[0] - 0.0001).abs() < 1e-15);
    assert!((cloud[0].position_m()[0] - 0.0001).abs() < 1e-15);
    let old_fluid = fluid.clone();
    let old_cloud = cloud.clone();
    // Each fluid interval needs an acoustic subdivision; aggregate budget fails
    // after preceding drag/drift stages, not only at argument validation.
    assert!(fluid.step_suspension_free(&mut cloud, 0.1, 4).is_err());
    assert_eq!(fluid, old_fluid);
    assert_eq!(cloud[0].position_m(), old_cloud[0].position_m());
    assert_eq!(cloud[0].velocity_m_s(), old_cloud[0].velocity_m_s());
}
#[test]
fn forced_carrier_thermal_bridge_balances_weight_and_enthalpy() {
    let (mut fluid, mut cloud) = fixture(true);
    let mass = fluid.particles()[0].mass + cloud[0].mass_kg();
    let old_p = cloud[0].mass_kg() * cloud[0].velocity_m_s()[0];
    let old_k = kinetic(&fluid, &cloud);
    let old_heat = fluid.transport_totals().unwrap().unwrap().0;
    let dt = 1e-6;
    let acceleration = [1., 0., 0.];
    let report = fluid
        .exchange_suspension_cell_forced(0, &mut cloud, dt, &[acceleration], acceleration)
        .unwrap();
    let new_p = fluid.particles()[0].mass * fluid.particles()[0].velocity[0]
        + cloud[0].mass_kg() * cloud[0].velocity_m_s()[0];
    assert!((new_p - old_p - dt * mass).abs() < 1e-25);
    let added_heat = fluid.transport_totals().unwrap().unwrap().0 - old_heat;
    assert!((added_heat - report.viscous_heat_j).abs() < 1e-23);
    assert!(
        (kinetic(&fluid, &cloud) - old_k + added_heat + report.numerical_loss_j
            - report.body_force_work_j)
            .abs()
            < 1e-23
    );
    let old_fluid = fluid.clone();
    let old_cloud = cloud.clone();
    assert!(
        fluid
            .exchange_suspension_cell_forced(0, &mut cloud, dt, &[], acceleration)
            .is_err()
    );
    assert_eq!(fluid, old_fluid);
    assert_eq!(cloud[0].position_m(), old_cloud[0].position_m());
    assert_eq!(cloud[0].velocity_m_s(), old_cloud[0].velocity_m_s());
}
#[test]
fn common_free_fall_generates_no_spurious_drag_heat_or_buoyancy() {
    let (mut fluid, _) = fixture(true);
    let mut cloud = vec![Grain::new(1e-6, 2000., [0.; 3], [0.; 3]).unwrap()];
    let old_heat = fluid.transport_totals().unwrap().unwrap().0;
    let gravity = [0., -9.81, 0.];
    let dt = 1e-4;
    let report = fluid
        .exchange_suspension_cell_forced(0, &mut cloud, dt, &[gravity], gravity)
        .unwrap();
    assert_eq!(fluid.particles()[0].velocity, cloud[0].velocity_m_s());
    assert_eq!(fluid.particles()[0].velocity[1], dt * gravity[1]);
    assert_eq!(report.viscous_heat_j, 0.);
    assert_eq!(fluid.transport_totals().unwrap().unwrap().0, old_heat);
    assert!(report.body_force_work_j > 0.);
    assert!(report.numerical_loss_j > 0.);
    assert_eq!(fluid.phase_fractions().unwrap()[0], 0.);
}
#[test]
fn mixture_inventory_counts_solid_volume_and_tracks_thermal_carrier_density() {
    let (mut fluid, cloud) = fixture(true);
    let grain_mass = cloud[0].mass_kg();
    let grain_volume = cloud[0].volume_m3();
    let cell = fluid.suspension_inventory(&cloud).unwrap()[0];
    let expected_density = (1e-10 + grain_mass) / (1e-13 + grain_volume);
    assert!((cell.mixture_density_kg_m3 - expected_density).abs() < 1e-10);
    assert_eq!(cell.solid_mass_kg, grain_mass);
    assert_eq!(cell.solid_volume_m3, grain_volume);
    assert!((cell.solid_volume_fraction - grain_volume / (1e-13 + grain_volume)).abs() < 1e-15);
    assert!(cell.mixture_density_kg_m3 > 1000.);
    fluid
        .configure_phase_change(
            vec![Some(PhaseChange {
                temperature: 10.,
                latent_heat: 0.001,
                high_phase: Material {
                    rest_density: 500.,
                    sound_speed: 2.,
                    viscosity: 0.001,
                },
            })],
            vec![0.],
        )
        .unwrap();
    fluid.add_heat(&[1e-13]).unwrap();
    let hot = fluid.suspension_inventory(&cloud).unwrap()[0];
    let density = fluid.effective_materials().unwrap()[0].rest_density;
    assert!((hot.liquid_volume_m3 - 1e-10 / density).abs() < 1e-28);
    assert!(
        (hot.mixture_density_kg_m3 - (1e-10 + grain_mass) / (1e-10 / density + grain_volume)).abs()
            < 1e-10
    );
    assert_eq!(hot.solid_mass_kg, cell.solid_mass_kg);
    assert_eq!(hot.solid_volume_m3, cell.solid_volume_m3);
    assert!(hot.solid_volume_fraction < cell.solid_volume_fraction);
    let old = fluid.clone();
    let dense = vec![Grain::new(1e-5, 2000., [0.; 3], [0.; 3]).unwrap()];
    assert!(fluid.suspension_inventory(&dense).is_err());
    assert_eq!(fluid, old);
}
#[test]
fn sub_ulp_drag_heat_is_owned_in_buffer_without_losing_particle_feedback() {
    let (mut fluid, mut cloud) = fixture(true);
    fluid.add_heat(&[1e100]).unwrap();
    let before = fluid.transport_totals().unwrap().unwrap().0;
    let report = fluid.exchange_suspension_cell(0, &mut cloud, 1e-6).unwrap();
    assert!(report.viscous_heat_j > 0.);
    assert_eq!(fluid.transport_totals().unwrap().unwrap().0, before);
    assert_eq!(fluid.suspension_heat_buffer()[0], report.viscous_heat_j);
    assert!(fluid.particles()[0].velocity[0] > 0.);
    let old = fluid.clone();
    assert!(fluid.exchange_suspension_cell(0, &mut cloud, -1.).is_err());
    assert_eq!(fluid, old);
    let buffered = fluid.suspension_heat_buffer()[0];
    fluid.add_heat(&[-before]).unwrap();
    let enthalpy = fluid.transport_totals().unwrap().unwrap().0;
    let next = fluid.exchange_suspension_cell(0, &mut cloud, 1e-6).unwrap();
    let deposited = fluid.transport_totals().unwrap().unwrap().0 - enthalpy;
    assert!((deposited - buffered - next.viscous_heat_j).abs() < 1e-25);
    assert_eq!(fluid.suspension_heat_buffer()[0], 0.);
}

#[test]
fn sub_ulp_buffer_increments_preserve_their_rounding_residue() {
    let (mut fluid, _) = fixture(true);
    fluid.add_heat(&[1e100]).unwrap();
    fluid.deposit_dissipation_heat(&[1.]).unwrap();
    let increment = f64::EPSILON / 4.;
    fluid.deposit_dissipation_heat(&[increment]).unwrap();
    assert_eq!(fluid.suspension_heat_buffer()[0], 1.);
    assert_eq!(fluid.suspension_heat_correction()[0], increment);
    for _ in 0..7 {
        fluid.deposit_dissipation_heat(&[increment]).unwrap();
    }
    assert_eq!(fluid.suspension_heat_buffer()[0], 1. + 2. * f64::EPSILON);
    assert_eq!(fluid.suspension_heat_correction()[0], 0.);
    let old = fluid.clone();
    assert!(fluid.deposit_dissipation_heat(&[f64::INFINITY]).is_err());
    assert_eq!(fluid, old);
}
