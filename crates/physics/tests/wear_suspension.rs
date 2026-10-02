use physics::friction::Material as Contact;
use physics::liquid::{Config, Liquid, LiquidField, Material, Particle, TransportMaterial};
use physics::wear::{Layer, Material as Wear, WearSuspension, WearSuspensionInput};
fn system() -> WearSuspension {
    system_with_velocity([0.; 3])
}
fn system_with_velocity(velocity: [f64; 3]) -> WearSuspension {
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.; 3],
            velocity,
            mass: 1e-5,
            material: 0,
        }],
        vec![Material {
            rest_density: 1000.,
            sound_speed: 2.,
            viscosity: 0.001,
        }],
        Config {
            gravity: [0.; 3],
            ..Config::default()
        },
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
    liquid.set_viscous_heating(true).unwrap();
    liquid.set_pressure_work(true).unwrap();
    WearSuspension::new(
        Layer::new(0.01, 0.002, 2500.).unwrap(),
        Contact::new(1e9, 1e9, 0.5).unwrap(),
        Wear::new(1e8, 1e-3).unwrap(),
        liquid,
        100.,
    )
    .unwrap()
}
#[test]
fn whole_frame_rolls_back_new_and_existing_cloud_after_late_fluid_failure() {
    let mut system = system();
    let input = WearSuspensionInput {
        gap_m: [-1e-4, 1e-4, 0.],
        normal: [1., 0., 0.],
        emission_positions_m: &[[0.; 3]; 16],
        inherited_velocity_m_s: [0.001, 0., 0.],
        heat_weights: &[1.],
        dt_s: 1e-4,
        coupling_steps: 1,
    };
    let initial = system.layer().remaining_mass_kg();
    let initial_energy = system.energy();
    let report = system.step_contact(input).unwrap();
    assert_eq!(report.emitted_grains, 16);
    assert_eq!(system.grains().len(), 16);
    let energy = system.energy();
    assert_eq!(energy.surface_energy_j, report.surface_energy_j);
    assert_eq!(energy.drag_numerical_loss_j, report.drag.numerical_loss_j);
    assert_eq!(energy.friction_input_j, report.friction_work_j);
    assert_eq!(energy.emission_kinetic_input_j, report.emitted_kinetic_j);
    let balance = energy.kinetic_j - initial_energy.kinetic_j + energy.enthalpy_j
        - initial_energy.enthalpy_j
        + energy.surface_energy_j
        + energy.drag_numerical_loss_j
        - energy.friction_input_j
        - energy.emission_kinetic_input_j;
    assert!(balance.abs() < 1e-14);
    assert!(report.energy_defect_j.abs() < 1e-14);
    let dust: f64 = system.grains().iter().map(|p| p.mass_kg()).sum();
    assert!((system.layer().remaining_mass_kg() + dust - initial).abs() < 1e-14);
    let old = system.clone();
    assert!(
        system
            .step_contact(WearSuspensionInput {
                gap_m: [-1e-4, 2e-4, 0.],
                dt_s: 0.2,
                ..input
            })
            .is_err()
    );
    assert_eq!(system.liquid(), old.liquid());
    assert_eq!(system.contact_state(), old.contact_state());
    assert_eq!(system.energy(), old.energy());
    assert_eq!(
        system.layer().remaining_mass_kg(),
        old.layer().remaining_mass_kg()
    );
    assert_eq!(system.grains().len(), old.grains().len());
    for (p, q) in system.grains().iter().zip(old.grains()) {
        assert_eq!(p.position_m(), q.position_m());
        assert_eq!(p.velocity_m_s(), q.velocity_m_s());
    }
    let continued = system
        .step_contact(WearSuspensionInput {
            gap_m: [-1e-4, 2e-4, 0.],
            ..input
        })
        .unwrap();
    assert_eq!(continued.emitted_grains, 16);
    assert_eq!(system.grains().len(), 32);
    assert!(
        (system.energy().surface_energy_j
            - old.energy().surface_energy_j
            - continued.surface_energy_j)
            .abs()
            < 1e-15
    );
    assert!(
        (system.energy().friction_input_j
            - old.energy().friction_input_j
            - continued.friction_work_j)
            .abs()
            < 1e-15
    );
}
#[test]
fn moving_carrier_checks_each_momentum_component_over_repeated_emission() {
    let fluid_velocity = [0.0005, -0.0004, 0.0002];
    let inherited = [0.001, -0.001, 0.0005];
    let mut system = system_with_velocity(fluid_velocity);
    let initial_mass = system.layer().remaining_mass_kg();
    let initial_energy = system.energy();
    let initial_fluid_mass = system.liquid().particles()[0].mass;
    let mut emitted_mass = 0.;
    for step in 0..10 {
        let report = system
            .step_contact(WearSuspensionInput {
                gap_m: [-1e-4, 1e-4 + f64::from(step) * 1e-5, 0.],
                normal: [1., 0., 0.],
                emission_positions_m: &[[0.; 3]; 16],
                inherited_velocity_m_s: inherited,
                heat_weights: &[1.],
                dt_s: 1e-4,
                coupling_steps: 1,
            })
            .unwrap();
        emitted_mass += report.wear.mass_kg;
        assert!(report.emitted_mass_defect_kg.abs() < 1e-23);
        for axis in 0..3 {
            let actual = system.liquid().particles()[0].mass
                * system.liquid().particles()[0].velocity[axis]
                + system
                    .grains()
                    .iter()
                    .map(|p| p.mass_kg() * p.velocity_m_s()[axis])
                    .sum::<f64>();
            let expected =
                initial_fluid_mass * fluid_velocity[axis] + emitted_mass * inherited[axis];
            assert!((actual - expected).abs() < 1e-21);
            assert!(report.momentum_defect_n_s[axis].abs() < 1e-21);
        }
        let dust_mass: f64 = system.grains().iter().map(|p| p.mass_kg()).sum();
        assert!((system.layer().remaining_mass_kg() + dust_mass - initial_mass).abs() < 1e-14);
        let energy = system.energy();
        assert!(
            (energy.kinetic_j - initial_energy.kinetic_j + energy.enthalpy_j
                - initial_energy.enthalpy_j
                + energy.surface_energy_j
                + energy.drag_numerical_loss_j
                - energy.friction_input_j
                - energy.emission_kinetic_input_j)
                .abs()
                < 1e-13
        );
    }
    assert_eq!(system.grains().len(), 160);
}
#[test]
fn finite_translating_parent_supplies_grain_mass_momentum_and_energy() {
    let fixture = system();
    let velocity = [0.001, -0.001, 0.0005];
    let mut system = WearSuspension::new_translating(
        fixture.layer().clone(),
        Contact::new(1e9, 1e9, 0.5).unwrap(),
        Wear::new(1e8, 1e-3).unwrap(),
        fixture.liquid().clone(),
        100.,
        velocity,
    )
    .unwrap();
    let initial_energy = system.energy();
    let initial_parent_momentum = system.parent_momentum_kg_m_s().unwrap();
    let initial_mass = system.layer().remaining_mass_kg();
    let input = WearSuspensionInput {
        gap_m: [-1e-4, 1e-4, 0.],
        normal: [1., 0., 0.],
        emission_positions_m: &[[0.; 3]; 16],
        inherited_velocity_m_s: velocity,
        heat_weights: &[1.],
        dt_s: 1e-4,
        coupling_steps: 1,
    };
    let report = system.step_contact(input).unwrap();
    let energy = system.energy();
    assert!(energy.parent_kinetic_j < initial_energy.parent_kinetic_j);
    assert_eq!(energy.emission_kinetic_input_j, 0.);
    assert!(
        (initial_energy.parent_kinetic_j - energy.parent_kinetic_j - report.emitted_kinetic_j)
            .abs()
            < 1e-22
    );
    let parent = system.parent_momentum_kg_m_s().unwrap();
    for axis in 0..3 {
        let combined = parent[axis]
            + system.liquid().particles()[0].mass * system.liquid().particles()[0].velocity[axis]
            + system
                .grains()
                .iter()
                .map(|p| p.mass_kg() * p.velocity_m_s()[axis])
                .sum::<f64>();
        assert!((combined - initial_parent_momentum[axis]).abs() < 1e-18);
    }
    let grains: f64 = system.grains().iter().map(|p| p.mass_kg()).sum();
    assert!((system.layer().remaining_mass_kg() + grains - initial_mass).abs() < 1e-14);
    let balance = energy.parent_kinetic_j - initial_energy.parent_kinetic_j + energy.kinetic_j
        - initial_energy.kinetic_j
        + energy.enthalpy_j
        - initial_energy.enthalpy_j
        + energy.surface_energy_j
        + energy.drag_numerical_loss_j
        - energy.friction_input_j;
    assert!(balance.abs() < 1e-14);
    let old = system.clone();
    assert!(
        system
            .step_contact(WearSuspensionInput {
                inherited_velocity_m_s: [0.; 3],
                ..input
            })
            .is_err()
    );
    assert_eq!(system.energy(), old.energy());
    assert_eq!(
        system.parent_momentum_kg_m_s(),
        old.parent_momentum_kg_m_s()
    );
    assert!(
        system
            .step_contact(WearSuspensionInput {
                gap_m: [-1e-4, 2e-4, 0.],
                dt_s: 0.2,
                ..input
            })
            .is_err()
    );
    assert_eq!(system.energy(), old.energy());
    assert_eq!(
        system.parent_momentum_kg_m_s(),
        old.parent_momentum_kg_m_s()
    );
    assert_eq!(system.grains().len(), old.grains().len());
}
#[test]
fn parent_impulses_change_inherited_motion_and_record_signed_work() {
    let fixture = system();
    let mut system = WearSuspension::new_translating(
        fixture.layer().clone(),
        Contact::new(1e9, 1e9, 0.5).unwrap(),
        Wear::new(1e8, 1e-3).unwrap(),
        fixture.liquid().clone(),
        100.,
        [0.; 3],
    )
    .unwrap();
    let initial = system.energy();
    let impulse = [5e-5, -2.5e-5, 1.25e-5];
    let work = system.apply_parent_impulse(impulse).unwrap();
    assert!(work > 0.);
    let momentum = system.parent_momentum_kg_m_s().unwrap();
    for i in 0..3 {
        assert!((momentum[i] - impulse[i]).abs() < 1e-20);
    }
    let velocity = system.parent_velocity_m_s().unwrap();
    let report = system
        .step_contact(WearSuspensionInput {
            gap_m: [-1e-4, 1e-4, 0.],
            normal: [1., 0., 0.],
            emission_positions_m: &[[0.; 3]; 16],
            inherited_velocity_m_s: velocity,
            heat_weights: &[1.],
            dt_s: 1e-4,
            coupling_steps: 1,
        })
        .unwrap();
    assert!(report.emitted_grains > 0);
    let energy = system.energy();
    assert_eq!(energy.parent_impulse_work_j, work);
    let balance = energy.parent_kinetic_j - initial.parent_kinetic_j + energy.kinetic_j
        - initial.kinetic_j
        + energy.enthalpy_j
        - initial.enthalpy_j
        + energy.surface_energy_j
        + energy.drag_numerical_loss_j
        - energy.friction_input_j
        - energy.parent_impulse_work_j;
    assert!(balance.abs() < 1e-14);
    let old_grains: Vec<_> = system.grains().iter().map(|p| p.velocity_m_s()).collect();
    let stop = system.parent_momentum_kg_m_s().unwrap().map(|p| -p);
    let braking = system.apply_parent_impulse(stop).unwrap();
    assert!(braking < 0.);
    assert!(system.energy().parent_kinetic_j < 1e-30);
    assert_eq!(
        old_grains,
        system
            .grains()
            .iter()
            .map(|p| p.velocity_m_s())
            .collect::<Vec<_>>()
    );
    let old = system.clone();
    assert!(system.apply_parent_impulse([f64::NAN, 0., 0.]).is_err());
    assert_eq!(system.energy(), old.energy());
    assert_eq!(system.parent_velocity_m_s(), old.parent_velocity_m_s());
}
#[test]
fn late_contact_frame_failure_rolls_back_parent_kick_and_work() {
    let fixture = system();
    let mut system = WearSuspension::new_translating(
        fixture.layer().clone(),
        Contact::new(1e9, 1e9, 0.5).unwrap(),
        Wear::new(1e8, 1e-3).unwrap(),
        fixture.liquid().clone(),
        100.,
        [0.; 3],
    )
    .unwrap();
    let input = WearSuspensionInput {
        gap_m: [-1e-4, 1e-4, 0.],
        normal: [1., 0., 0.],
        emission_positions_m: &[[0.; 3]; 16],
        inherited_velocity_m_s: [0.; 3],
        heat_weights: &[1.],
        dt_s: 0.2,
        coupling_steps: 1,
    };
    let impulse = [5e-5, -2.5e-5, 1.25e-5];
    let initial = system.clone();
    assert!(system.apply_parent_impulse([1e-300, 0., 0.]).is_err());
    assert_eq!(system.energy(), initial.energy());
    assert_eq!(system.parent_velocity_m_s(), initial.parent_velocity_m_s());
    assert!(
        system
            .step_contact_with_parent_impulse(impulse, input)
            .is_err()
    );
    assert_eq!(system.parent_velocity_m_s(), initial.parent_velocity_m_s());
    assert_eq!(system.energy(), initial.energy());
    assert_eq!(system.liquid(), initial.liquid());
    assert_eq!(system.contact_state(), initial.contact_state());
    assert_eq!(
        system.layer().remaining_mass_kg(),
        initial.layer().remaining_mass_kg()
    );
    assert!(system.grains().is_empty());
    let (work, report) = system
        .step_contact_with_parent_impulse(
            impulse,
            WearSuspensionInput {
                dt_s: 1e-4,
                ..input
            },
        )
        .unwrap();
    assert!(work > 0. && report.emitted_kinetic_j > 0.);
    assert_eq!(system.grains().len(), 16);
    for axis in 0..3 {
        let momentum = system.parent_momentum_kg_m_s().unwrap()[axis]
            + system.liquid().particles()[0].mass * system.liquid().particles()[0].velocity[axis]
            + system
                .grains()
                .iter()
                .map(|p| p.mass_kg() * p.velocity_m_s()[axis])
                .sum::<f64>();
        assert!((momentum - impulse[axis]).abs() < 1e-18);
    }
    let energy = system.energy();
    let balance = energy.parent_kinetic_j + energy.kinetic_j + energy.enthalpy_j
        - initial.energy().enthalpy_j
        + energy.surface_energy_j
        + energy.drag_numerical_loss_j
        - energy.friction_input_j
        - energy.parent_impulse_work_j;
    assert!(balance.abs() < 1e-14);
}
#[test]
fn dynamic_slider_funds_wear_heat_from_body_energy_without_external_work() {
    use physics::wear::WearSliderInput;
    let fixture = system();
    let mut system = WearSuspension::new_translating(
        fixture.layer().clone(),
        Contact::new(1e9, 1e9, 1e-6).unwrap(),
        Wear::new(1e8, 1e-3).unwrap(),
        fixture.liquid().clone(),
        0.01,
        [0., 0.001, 0.],
    )
    .unwrap();
    system
        .initialize_slider([-1e-4, 0., 0.], [1., 0., 0.])
        .unwrap();
    let initial = system.energy();
    let initial_mass = system.layer().remaining_mass_kg();
    let input = WearSliderInput {
        emission_positions_m: &[[0.; 3]; 16],
        heat_weights: &[1.],
        dt_s: 0.001,
        coupling_steps: 1,
    };
    let mut plane_impulse = [0.; 3];
    let mut recoil = false;
    for step in 0..200 {
        let result = system.step_sliding(input).unwrap();
        if step < 10 {
            assert!(result.formation.emitted_grains > 0);
            assert!(result.slider.physical_heat_j > 0.);
        }
        recoil |= result.slider.velocity_m_s[1] < 0.;
        for i in 0..3 {
            plane_impulse[i] -= result.slider.impulse_n_s[i];
        }
        let energy = system.energy();
        assert_eq!(energy.friction_input_j, 0.);
        assert_eq!(energy.parent_impulse_work_j, 0.);
        assert_eq!(energy.emission_kinetic_input_j, 0.);
        let balance = energy.parent_kinetic_j - initial.parent_kinetic_j + energy.kinetic_j
            - initial.kinetic_j
            + energy.enthalpy_j
            - initial.enthalpy_j
            + energy.surface_energy_j
            + energy.drag_numerical_loss_j
            + energy.contact_spring_j
            + energy.contact_numerical_loss_j
            + energy.parent_integration_numerical_loss_j
            + energy.suspension_heat_buffer_j
            - initial.suspension_heat_buffer_j
            + energy.accumulation_correction_j.iter().sum::<f64>()
            + system
                .energy_accumulation_tail()
                .iter()
                .flatten()
                .sum::<f64>();
        assert!(balance.abs() < 1e-17);
        let mass: f64 = system.grains().iter().map(|p| p.mass_kg()).sum();
        assert!((system.layer().remaining_mass_kg() + mass - initial_mass).abs() < 1e-14);
        let momentum = system.parent_momentum_kg_m_s().unwrap()[1]
            + system.liquid().particles()[0].mass * system.liquid().particles()[0].velocity[1]
            + system
                .grains()
                .iter()
                .map(|p| p.mass_kg() * p.velocity_m_s()[1])
                .sum::<f64>()
            + plane_impulse[1];
        assert!((momentum - initial_mass * 0.001).abs() < 1e-18);
    }
    assert!(recoil);
    assert!(system.parent_velocity_m_s().unwrap()[1].abs() < 1e-20);
    assert!(system.energy().enthalpy_j > initial.enthalpy_j);
    let old = system.clone();
    assert!(
        system
            .step_sliding(WearSliderInput { dt_s: 0.2, ..input })
            .is_err()
    );
    assert_eq!(system.energy(), old.energy());
    assert_eq!(
        system.energy_accumulation_tail(),
        old.energy_accumulation_tail()
    );
    assert_eq!(system.slider_gap_m(), old.slider_gap_m());
    assert_eq!(system.contact_state(), old.contact_state());
    assert_eq!(system.parent_velocity_m_s(), old.parent_velocity_m_s());
    assert_eq!(system.grains().len(), old.grains().len());
}
