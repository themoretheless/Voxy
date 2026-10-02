use physics::wear::{Layer, Material};
#[test]
fn worn_material_becomes_mass_volume_and_momentum_in_suspended_grains() {
    let material = Material::new(1e8, 1e-3).unwrap();
    let mut layer = Layer::new(0.01, 0.002, 2500.).unwrap();
    let initial = layer.remaining_mass_kg();
    let velocity = [0.01, -0.02, 0.03];
    let emitted = layer
        .advance_dust(material, 100., 0.001, &[[0.; 3]; 16], velocity)
        .unwrap();
    let mass: f64 = emitted.particles.iter().map(|p| p.mass_kg()).sum();
    let volume: f64 = emitted.particles.iter().map(|p| p.volume_m3()).sum();
    assert!((mass - emitted.wear.mass_kg).abs() < 1e-20);
    assert!((volume - emitted.wear.volume_m3).abs() < 1e-24);
    assert!((layer.remaining_mass_kg() + mass - initial).abs() < 1e-16);
    for axis in 0..3 {
        let momentum: f64 = emitted
            .particles
            .iter()
            .map(|p| p.mass_kg() * p.velocity_m_s()[axis])
            .sum();
        assert!((momentum - emitted.wear.mass_kg * velocity[axis]).abs() < 1e-20);
    }
    let kinetic: f64 = emitted
        .particles
        .iter()
        .map(|p| 0.5 * p.mass_kg() * p.velocity_m_s().iter().map(|v| v * v).sum::<f64>())
        .sum();
    assert!((kinetic - emitted.translational_kinetic_j).abs() < 1e-20);
}
#[test]
fn failed_emission_preserves_layer_and_zero_wear_emits_nothing() {
    let material = Material::new(1e8, 1e-3).unwrap();
    let mut layer = Layer::new(0.01, 0.002, 2500.).unwrap();
    let initial = layer.remaining_mass_kg();
    let thickness = layer.thickness_m();
    assert!(
        layer
            .advance_dust(material, 100., 0.001, &[], [0.; 3])
            .is_err()
    );
    assert!(
        layer
            .advance_dust(material, 100., 0.001, &[[0.; 3]], [1e300, 0., 0.])
            .is_err()
    );
    assert_eq!(layer.remaining_mass_kg(), initial);
    assert_eq!(layer.thickness_m(), thickness);
    assert_eq!(layer.debris_mass_kg(), 0.);
    assert!(
        layer
            .advance_dust(material, 0., 0.001, &[], [0.; 3])
            .unwrap()
            .particles
            .is_empty()
    );
}
#[test]
fn emitted_wear_grains_feed_back_into_liquid_instead_of_losing_mass() {
    use physics::liquid::{
        Config, Liquid, LiquidField, Material as FluidMaterial, Particle, TransportMaterial,
    };
    let mut layer = Layer::new(0.01, 0.002, 2500.).unwrap();
    let emitted = layer
        .advance_dust(
            Material::new(1e8, 1e-3).unwrap(),
            100.,
            1e-7,
            &[[0.; 3]; 16],
            [0.01, 0., 0.],
        )
        .unwrap();
    let mut grains = emitted.particles;
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 1e-6,
            material: 0,
        }],
        vec![FluidMaterial {
            rest_density: 1000.,
            sound_speed: 2.,
            viscosity: 0.001,
        }],
        Config::default(),
    )
    .unwrap();
    liquid
        .configure_transport(
            vec![LiquidField {
                temperature: 0.001,
                concentration: 0.,
            }],
            vec![TransportMaterial {
                specific_heat: 1.,
                conductivity: 0.,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    let initial_momentum = emitted.wear.mass_kg * 0.01;
    let old_heat = liquid.transport_totals().unwrap().unwrap().0;
    let report = liquid
        .exchange_suspension_cell(0, &mut grains, 1e-6)
        .unwrap();
    let mass: f64 = grains.iter().map(|p| p.mass_kg()).sum();
    assert!((mass - emitted.wear.mass_kg).abs() < 1e-24);
    let momentum = 1e-6 * liquid.particles()[0].velocity[0]
        + grains
            .iter()
            .map(|p| p.mass_kg() * p.velocity_m_s()[0])
            .sum::<f64>();
    assert!((momentum - initial_momentum).abs() < 1e-24);
    assert!(liquid.particles()[0].velocity[0] > 0.);
    let added_heat = liquid.transport_totals().unwrap().unwrap().0 - old_heat;
    assert!((added_heat - report.viscous_heat_j).abs() < 1e-24);
    let density = liquid.suspension_inventory(&grains).unwrap()[0];
    assert!((density.solid_mass_kg - emitted.wear.mass_kg).abs() < 1e-24);
}
#[test]
fn finer_dust_costs_more_surface_energy_and_insufficient_budget_is_atomic() {
    use physics::wear::SurfaceEnergy;
    let material = Material::new(1e8, 1e-3).unwrap();
    let original = Layer::new(0.01, 0.002, 2500.).unwrap();
    let mut coarse = original.clone();
    let mut fine = original.clone();
    let mut coarse_bank = SurfaceEnergy::new(1e-6, 2.).unwrap();
    let mut fine_bank = coarse_bank.clone();
    let coarse_report = coarse
        .advance_dust_with_surface_energy(
            material,
            100.,
            1e-7,
            &[[0.; 3]],
            [0.; 3],
            &mut coarse_bank,
        )
        .unwrap();
    let fine_report = fine
        .advance_dust_with_surface_energy(
            material,
            100.,
            1e-7,
            &[[0.; 3]; 64],
            [0.; 3],
            &mut fine_bank,
        )
        .unwrap();
    assert!((fine_report.created_surface_m2 / coarse_report.created_surface_m2 - 4.).abs() < 1e-12);
    assert!((fine_report.surface_energy_j - 2. * fine_report.created_surface_m2).abs() < 1e-25);
    assert!((fine_bank.available_j() + fine_report.surface_energy_j - 1e-6).abs() < 1e-21);
    assert!((coarse_report.dust.wear.mass_kg - fine_report.dust.wear.mass_kg).abs() < 1e-24);
    let mut layer = original.clone();
    let mut limited = SurfaceEnergy::new(0.5 * fine_report.surface_energy_j, 2.).unwrap();
    let old_energy = limited.available_j();
    assert!(
        layer
            .advance_dust_with_surface_energy(
                material,
                100.,
                1e-7,
                &[[0.; 3]; 64],
                [0.; 3],
                &mut limited
            )
            .is_err()
    );
    assert_eq!(layer.remaining_mass_kg(), original.remaining_mass_kg());
    assert_eq!(layer.debris_mass_kg(), 0.);
    assert_eq!(limited.available_j(), old_energy);
    let mut unrepresentable = SurfaceEnergy::new(1e300, 2.).unwrap();
    assert!(
        layer
            .advance_dust_with_surface_energy(
                material,
                100.,
                1e-7,
                &[[0.; 3]],
                [0.; 3],
                &mut unrepresentable
            )
            .is_err()
    );
    assert_eq!(unrepresentable.available_j(), 1e300);
    assert_eq!(layer.remaining_mass_kg(), original.remaining_mass_kg());
    let accepted = layer
        .advance_dust_with_surface_energy(material, 100., 1e-7, &[[0.; 3]], [0.; 3], &mut limited)
        .unwrap();
    assert!((limited.available_j() + accepted.surface_energy_j - old_energy).abs() < 1e-25);
}
#[test]
fn friction_funds_surface_and_heat_only_over_distance_consumed_by_layer() {
    use physics::wear::DustFriction;
    let mut layer = Layer::new(1., 0.001, 2000.).unwrap();
    let material = Material::new(1000., 0.1).unwrap();
    let friction = DustFriction {
        tangential_force_n: 100.,
        surface_energy_j_m2: 1.,
    };
    // Rate=0.001 m^3 per metre: exhaustion consumes exactly 1 of 3 metres.
    let report = layer
        .advance_dust_friction(material, 10., 3., &[[0.; 3]], [0.; 3], friction)
        .unwrap();
    assert_eq!(report.formation.dust.wear.consumed_sliding_distance_m, 1.);
    assert_eq!(report.formation.dust.wear.remaining_sliding_distance_m, 2.);
    assert_eq!(report.friction_work_j, 100.);
    assert!((report.friction_heat_j + report.formation.surface_energy_j - 100.).abs() < 1e-12);
    assert!(report.friction_heat_j > 0.);
    let again = layer
        .advance_dust_friction(material, 0., 2., &[], [0.; 3], friction)
        .unwrap();
    assert_eq!(again.friction_work_j, 0.);
    assert_eq!(again.friction_heat_j, 0.);
    assert_eq!(again.formation.dust.wear.remaining_sliding_distance_m, 2.);
    let mut untouched = Layer::new(1., 0.001, 2000.).unwrap();
    let initial = untouched.remaining_mass_kg();
    assert!(
        untouched
            .advance_dust_friction(
                material,
                10.,
                3.,
                &[[0.; 3]],
                [0.; 3],
                DustFriction {
                    tangential_force_n: 0.,
                    ..friction
                }
            )
            .is_err()
    );
    assert_eq!(untouched.remaining_mass_kg(), initial);
    assert_eq!(untouched.debris_mass_kg(), 0.);
}
#[test]
fn coulomb_contact_physical_dissipation_funds_dust_and_commits_history_atomically() {
    use physics::friction::{Material as Contact, Mode, State};
    use physics::wear::ContactDustSettings;
    let contact = Contact::new(1e9, 1e9, 0.5).unwrap();
    let mut layer = Layer::new(0.01, 0.002, 2500.).unwrap();
    let mut state = State::default();
    let settings = ContactDustSettings {
        wear_material: Material::new(1e8, 1e-3).unwrap(),
        surface_energy_j_m2: 100.,
        emission_positions_m: &[[0.; 3]; 16],
        inherited_velocity_m_s: [0.; 3],
    };
    let result = layer
        .advance_contact_dust(
            contact,
            &mut state,
            [-1e-4, 1e-4, 0.],
            [1., 0., 0.],
            settings,
        )
        .unwrap();
    assert_eq!(result.contact.mode, Mode::Slip);
    assert!((result.friction_work_j - state.dissipated_j_m2() * 0.01).abs() < 1e-15);
    assert!((result.formation.dust.wear.volume_m3 - 5e-13).abs() < 1e-24);
    assert!(
        (result.friction_heat_j + result.formation.surface_energy_j - result.friction_work_j).abs()
            < 1e-15
    );
    assert!(result.friction_heat_j < result.friction_work_j);
    // Holding the pose generates neither new physical slip nor new dust/work.
    let held = layer
        .advance_contact_dust(
            contact,
            &mut state,
            [-1e-4, 1e-4, 0.],
            [1., 0., 0.],
            settings,
        )
        .unwrap();
    assert_eq!(held.friction_work_j, 0.);
    assert!(held.formation.dust.particles.is_empty());
    let old_state = state;
    let old_mass = layer.remaining_mass_kg();
    assert!(
        layer
            .advance_contact_dust(
                contact,
                &mut state,
                [-1e-4, 2e-4, 0.],
                [1., 0., 0.],
                ContactDustSettings {
                    surface_energy_j_m2: 1e30,
                    ..settings
                }
            )
            .is_err()
    );
    assert_eq!(state, old_state);
    assert_eq!(layer.remaining_mass_kg(), old_mass);
}
#[test]
fn contact_dust_heat_reaches_latent_enthalpy_with_three_state_rollback() {
    use physics::friction::{Material as Contact, State};
    use physics::liquid::{
        Config, Liquid, LiquidField, Material as FluidMaterial, Particle, PhaseChange,
        TransportMaterial,
    };
    use physics::wear::{ContactDustSettings, LiquidHeatSink};
    let material = FluidMaterial {
        rest_density: 1000.,
        sound_speed: 2.,
        viscosity: 0.001,
    };
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 1.,
            material: 0,
        }],
        vec![material],
        Config::default(),
    )
    .unwrap();
    liquid
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
    liquid
        .configure_phase_change(
            vec![Some(PhaseChange {
                temperature: 10.,
                latent_heat: 1.,
                high_phase: material,
            })],
            vec![0.],
        )
        .unwrap();
    let mut layer = Layer::new(0.01, 0.002, 2500.).unwrap();
    let mut state = State::default();
    let contact = Contact::new(1e9, 1e9, 0.5).unwrap();
    let settings = ContactDustSettings {
        wear_material: Material::new(1e8, 1e-3).unwrap(),
        surface_energy_j_m2: 100.,
        emission_positions_m: &[[0.; 3]; 16],
        inherited_velocity_m_s: [0.; 3],
    };
    let before = liquid.transport_totals().unwrap().unwrap().0;
    let report = layer
        .advance_contact_dust_heated(
            contact,
            &mut state,
            [-1e-4, 1e-4, 0.],
            [1., 0., 0.],
            settings,
            LiquidHeatSink {
                liquid: &mut liquid,
                weights: &[1.],
            },
        )
        .unwrap();
    let added = liquid.transport_totals().unwrap().unwrap().0 - before;
    assert!((added - report.friction_heat_j).abs() < 1e-14);
    assert!((added + report.formation.surface_energy_j - report.friction_work_j).abs() < 1e-14);
    assert_eq!(liquid.fields().unwrap()[0].temperature, 10.);
    assert!((liquid.phase_fractions().unwrap()[0] - report.friction_heat_j).abs() < 1e-14);
    // Heat below enthalpy resolution is retained; invalid partition still rolls back.
    liquid.add_heat(&[1e300]).unwrap();
    let huge = liquid.transport_totals().unwrap().unwrap().0;
    let deferred = layer
        .advance_contact_dust_heated(
            contact,
            &mut state,
            [-1e-4, 2e-4, 0.],
            [1., 0., 0.],
            settings,
            LiquidHeatSink {
                liquid: &mut liquid,
                weights: &[1.],
            },
        )
        .unwrap();
    assert_eq!(liquid.transport_totals().unwrap().unwrap().0, huge);
    assert_eq!(liquid.suspension_heat_buffer()[0], deferred.friction_heat_j);
    let old_liquid = liquid.clone();
    let old_state = state;
    let old_mass = layer.remaining_mass_kg();
    assert!(
        layer
            .advance_contact_dust_heated(
                contact,
                &mut state,
                [-1e-4, 2e-4, 0.],
                [1., 0., 0.],
                settings,
                LiquidHeatSink {
                    liquid: &mut liquid,
                    weights: &[0.]
                }
            )
            .is_err()
    );
    assert_eq!(liquid, old_liquid);
    assert_eq!(state, old_state);
    assert_eq!(layer.remaining_mass_kg(), old_mass);
}
