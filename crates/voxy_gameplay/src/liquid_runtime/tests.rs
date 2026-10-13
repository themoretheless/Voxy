use super::*;
use crate::LiquidPulse;
use voxy_scene::Transform;
#[test]
fn authored_supported_point_load_advances_phase_and_rolls_back_publication() {
    let mut scene = support_floor();
    let node = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(0., 0.125, 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            node,
            crate::BoxCollider {
                half_extents: [0.125; 3],
            },
        )
        .unwrap();
    scene
        .insert_component(
            node,
            crate::LiquidBody {
                mass_kg: 2.,
                initial_velocity_m_s: [0.; 3],
            },
        )
        .unwrap();
    scene
        .insert_component(
            node,
            crate::LiquidMassDistribution {
                parts: vec![crate::LiquidMassPart {
                    mass_kg: 2.,
                    center_m: [0.; 3],
                    half_edges_m: [[0.125, 0., 0.], [0., 0.125, 0.], [0., 0., 0.125]],
                }],
            },
        )
        .unwrap();
    scene
        .insert_component(
            node,
            crate::RigidPointLoads {
                points: vec![crate::RigidPointForce {
                    root_point_m: [0.025, 0., 0.],
                    world_force_n: [2., -1., 0.],
                    world_force_rate_n_s: [0., -0.4, 0.],
                }],
            },
        )
        .unwrap();
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let admission = physics::liquid::SupportedWorldConfig {
        max_interval_s: 0.0005,
        reaction_jerk_tolerance: Some(1e-10),
        ..Default::default()
    };
    for tick in 0..2 {
        let report = runtime
            .tick_and_publish_with_supported_dynamics(
                &mut scene,
                0.001,
                None,
                Default::default(),
                rigid_config(),
                admission,
            )
            .unwrap();
        let start = tick as f64 * 0.001;
        let expected_impulse = -(21. + 0.4 * start) * 0.001 - 0.2 * 0.001_f64.powi(2);
        let mechanical = report.support.unwrap();
        assert!((mechanical.environment_reaction_impulse[1] - expected_impulse).abs() < 1e-10);
        let expected_work = 2. * start * 0.001 + 0.001_f64.powi(2);
        assert!((mechanical.rigid.external_work - expected_work).abs() < 1e-10);
        assert!(mechanical.reaction_work.abs() < 1e-10);
        assert!(mechanical.rigid.integration_energy_residual.abs() < 1e-10);
        assert!((runtime.body[0].state.motion.velocity[0] - start - 0.001).abs() < 1e-10);
        assert!((runtime.body[0].point_elapsed_s - (start + 0.001)).abs() < 1e-14);
        assert!((scene.local(node).unwrap().translation.y - 0.125).abs() < 1e-7);
    }
    let saved = runtime.clone();
    let pose = scene.local(node).unwrap();
    assert!(
        runtime
            .tick_and_publish_with_supported_dynamics(
                &mut scene,
                0.01,
                None,
                Default::default(),
                rigid_config(),
                physics::liquid::SupportedWorldConfig {
                    max_intervals: 1,
                    ..admission
                },
            )
            .is_err()
    );
    assert_eq!(runtime, saved);
    assert_eq!(scene.local(node).unwrap(), pose);
}
#[test]
fn supported_motion_scene_owner_publishes_and_restores_after_late_failure() {
    let mut scene = support_floor();
    let body = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(0., 0.125, 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            body,
            crate::BoxCollider {
                half_extents: [0.125; 3],
            },
        )
        .unwrap();
    scene
        .insert_component(
            body,
            crate::LiquidBody {
                mass_kg: 2.,
                initial_velocity_m_s: [1., 0., 0.],
            },
        )
        .unwrap();
    scene
        .insert_component(
            body,
            crate::LiquidMassDistribution {
                parts: vec![crate::LiquidMassPart {
                    mass_kg: 2.,
                    center_m: [0.; 3],
                    half_edges_m: [[0.125, 0., 0.], [0., 0.125, 0.], [0., 0., 0.125]],
                }],
            },
        )
        .unwrap();
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let before = runtime.clone();
    let pose = scene.local(body).unwrap();
    assert!(
        runtime
            .tick_and_publish_with_supported_dynamics(
                &mut scene,
                0.1,
                None,
                Default::default(),
                rigid_config(),
                physics::liquid::SupportedWorldConfig {
                    max_intervals: 2,
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(runtime, before);
    assert_eq!(scene.local(body).unwrap(), pose);
    for tick in 1..=2 {
        let report = runtime
            .tick_and_publish_with_supported_dynamics(
                &mut scene,
                0.1,
                None,
                Default::default(),
                rigid_config(),
                Default::default(),
            )
            .unwrap();
        assert!((scene.local(body).unwrap().translation.x as f64 - 0.1 * tick as f64).abs() < 1e-7);
        assert!((scene.local(body).unwrap().translation.y as f64 - 0.125).abs() < 1e-8);
        assert!((report.support.unwrap().environment_reaction_impulse[1] + 2.).abs() < 1e-10);
    }
}
#[test]
fn supported_motion_retains_small_spin_and_rejects_unresolved_real_rotation() {
    use physics::contact::ContactWrench;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let mut bodies = [rigid_body([0., 0.125, 0.], [0.; 3], 1e-12)];
    let report = liquid
        .step_with_supported_rigid_body_forces(
            0.01,
            &mut bodies,
            &world,
            Default::default(),
            1,
            rigid_config(),
            &[ContactWrench::default()],
            Default::default(),
        )
        .unwrap();
    assert_ne!(bodies[0].spin.unwrap().angular_momentum[2], 0.);
    assert_ne!(bodies[0].spin.unwrap().orientation[2], 0.);
    assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    let mut rotating = [rigid_body([0., 0.125, 0.], [0.; 3], 1.)];
    let before = rotating;
    let fluid_before = liquid.clone();
    assert!(
        liquid
            .step_with_supported_rigid_body_forces(
                0.01,
                &mut rotating,
                &world,
                Default::default(),
                1,
                rigid_config(),
                &[ContactWrench::default()],
                Default::default()
            )
            .is_err()
    );
    assert_eq!(rotating, before);
    assert_eq!(liquid, fluid_before);
}
#[test]
fn rotating_edge_normal_advances_a_real_edge_patch_and_closes_internal_moment() {
    use glam::{DQuat, DVec3};
    use physics::contact::ContactWrench;
    use physics::liquid::LiquidBodyWorld;
    let scene = voxy_scene::SceneGraph::new(16);
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]; 2],
    };
    let q = DQuat::from_rotation_z(0.45) * DQuat::from_rotation_y(0.35);
    let c1 = (DVec3::Y - DVec3::Z) * 0.125;
    let c2 = -(q * ((DVec3::X + DVec3::Y) * 0.125));
    let mut bodies = [
        rigid_body(c1.to_array(), DVec3::Z.cross(c1).to_array(), 1.),
        rigid_body(c2.to_array(), DVec3::Z.cross(c2).to_array(), 1.),
    ];
    bodies[1].spin.as_mut().unwrap().orientation = q.to_array();
    let initial = bodies;
    let points = world
        .rigid_pair_support_contacts_with_error(0, &bodies[0], 1, &bodies[1], 128, 1e-10)
        .unwrap();
    assert!(!points.is_empty());
    assert!(points.iter().all(|point| matches!(
        FeatureKey::decode(point.feature.unwrap()).unwrap().axis,
        crate::convex::AxisFeature::Edges(_, _)
    )));
    let n = DVec3::from_array(points[0].support.contact.normal);
    let force = n * 0.3;
    let loads = [
        ContactWrench {
            force: (DVec3::Z.cross(DVec3::Z.cross(c1)) - force).to_array(),
            torque: c1.cross(force).to_array(),
        },
        ContactWrench {
            force: (DVec3::Z.cross(DVec3::Z.cross(c2)) + force).to_array(),
            torque: (-c2.cross(force)).to_array(),
        },
    ];
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap();
    let dt = 1e-4;
    let report = liquid
        .step_with_supported_rigid_body_forces(
            dt,
            &mut bodies,
            &world,
            Default::default(),
            2,
            rigid_config(),
            &loads,
            physics::liquid::SupportedWorldConfig {
                reaction_jerk_tolerance: Some(1e-10),
                ..Default::default()
            },
        )
        .unwrap();
    for (body, start) in bodies.iter().zip(initial) {
        let expected =
            DQuat::from_rotation_z(dt) * DQuat::from_array(start.spin.unwrap().orientation);
        let actual = DQuat::from_array(body.spin.unwrap().orientation);
        assert!(
            actual
                .to_array()
                .iter()
                .zip(expected.to_array())
                .all(|(a, b)| (a - b).abs() < 1e-8)
        );
        assert!((body.spin.unwrap().angular_momentum[2] - 1.).abs() < 1e-7);
    }
    assert!(
        report
            .reaction_angular_balance_residual
            .iter()
            .all(|x| x.abs() < 1e-12)
    );
    assert!(report.reaction_work.abs() < 1e-12);
    assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    assert_eq!(report.supported_intervals, 1);
    assert!(report.max_support_error_m <= 1e-10);
}

#[test]
fn material_point_loads_advance_a_rotating_face_pair_without_internal_couple() {
    use physics::contact::ContactWrench;
    use physics::liquid::LiquidBodyWorld;
    let scene = voxy_scene::SceneGraph::new(16);
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]; 2],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap();
    let mut bodies = [
        rigid_body([0., 0.125, 0.], [-0.125, 0., 0.], 1.),
        rigid_body([0., -0.125, 0.], [0.125, 0., 0.], 1.),
    ];
    let loads = [
        ContactWrench {
            force: [0., -0.5, 0.],
            torque: [0.; 3],
        },
        ContactWrench {
            force: [0., 0.5, 0.],
            torque: [0.; 3],
        },
    ];
    let supports = world
        .rigid_pair_support_contacts_with_error(0, &bodies[0], 1, &bodies[1], 128, 1e-10)
        .unwrap();
    assert!(!supports.is_empty());
    assert!(supports.iter().any(|point| matches!(
        point.support.plane,
        physics::contact::SupportPlane::First | physics::contact::SupportPlane::Second
    )));
    let points = [
        vec![physics::rigid_motion::MaterialPointForce {
            local: [0.025, -0.125, 0.],
            force: [0., -0.3, 0.],
            force_rate: [0.; 3],
        }],
        vec![physics::rigid_motion::MaterialPointForce {
            local: [0.025, 0.125, 0.],
            force: [0., 0.3, 0.],
            force_rate: [0.; 3],
        }],
    ];
    let dt: f64 = 1e-4;
    let report = liquid
        .step_with_supported_rigid_body_point_forces(
            dt,
            &mut bodies,
            &world,
            Default::default(),
            2,
            rigid_config(),
            &loads,
            &points,
            physics::liquid::SupportedWorldConfig {
                reaction_jerk_tolerance: Some(1e-10),
                ..Default::default()
            },
        )
        .unwrap();
    let expected = glam::DQuat::from_rotation_z(dt);
    for body in bodies {
        let actual = glam::DQuat::from_array(body.spin.unwrap().orientation);
        assert!(
            actual
                .to_array()
                .iter()
                .zip(expected.to_array())
                .all(|(a, b)| (a - b).abs() < 1e-10)
        );
        // Individual spin momenta can change: the pressure normal rotates.
        // Conservation is checked below using spin plus orbital momentum.
    }
    let total_momentum = bodies
        .iter()
        .map(|body| {
            glam::DVec3::from_array(body.spin.unwrap().angular_momentum)
                + glam::DVec3::from_array(body.motion.position)
                    .cross(glam::DVec3::from_array(body.motion.velocity) * body.motion.mass)
        })
        .sum::<glam::DVec3>();
    // Initial total Lz is 2 spins plus 2*(0.125²) orbital terms.
    // Constant COM loads give sum(v cross F) = 0.125 along Z.
    // Opposite point forces at the common initial world point cancel
    // through acceleration order; remaining integral terms are O(dt³).
    let expected_lz = 2. + 2. * 0.125_f64.powi(2) + 0.125 * dt.powi(2) / 2.;
    assert!(
        (total_momentum.z - expected_lz).abs() < 1e-12,
        "total={total_momentum:?}, expected Lz={expected_lz}"
    );
    assert!(total_momentum.x.abs().max(total_momentum.y.abs()) < 1e-12);
    assert!((bodies[0].motion.position[1] - (0.125 - 0.125 * dt * dt / 2.)).abs() < 1e-12);
    assert!((bodies[1].motion.position[1] - (-0.125 + 0.125 * dt * dt / 2.)).abs() < 1e-12);
    assert!(
        report
            .reaction_angular_balance_residual
            .iter()
            .all(|x| x.abs() < 1e-12)
    );
    assert!((bodies[0].motion.velocity[0] + bodies[1].motion.velocity[0]).abs() < 1e-12);
    assert!((bodies[0].motion.velocity[1] + bodies[1].motion.velocity[1]).abs() < 1e-12);
    assert!(report.reaction_work.abs() < 1e-10);
    assert!(report.rigid.integration_energy_residual.abs() < 1e-10);
    assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    assert_eq!(report.supported_intervals, 1);
    assert!(report.max_support_error_m <= 1e-10);
}

#[test]
fn rotating_face_normal_advances_a_pressed_pair_without_freezing_spin() {
    use physics::contact::ContactWrench;
    use physics::liquid::LiquidBodyWorld;
    let scene = voxy_scene::SceneGraph::new(16);
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]; 2],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap();
    let mut bodies = [
        rigid_body([0., 0.125, 0.], [-0.125, 0., 0.], 1.),
        rigid_body([0., -0.125, 0.], [0.125, 0., 0.], 1.),
    ];
    let loads = [
        ContactWrench {
            force: [0., -0.5, 0.],
            torque: [0.; 3],
        },
        ContactWrench {
            force: [0., 0.5, 0.],
            torque: [0.; 3],
        },
    ];
    let supports = world
        .rigid_pair_support_contacts_with_error(0, &bodies[0], 1, &bodies[1], 128, 1e-10)
        .unwrap();
    assert!(!supports.is_empty());
    assert!(supports.iter().any(|point| matches!(
        point.support.plane,
        physics::contact::SupportPlane::First | physics::contact::SupportPlane::Second
    )));
    let dt: f64 = 1e-4;
    let report = liquid
        .step_with_supported_rigid_body_forces(
            dt,
            &mut bodies,
            &world,
            Default::default(),
            2,
            rigid_config(),
            &loads,
            physics::liquid::SupportedWorldConfig {
                reaction_jerk_tolerance: Some(1e-10),
                ..Default::default()
            },
        )
        .unwrap();
    let expected = glam::DQuat::from_rotation_z(dt);
    for body in bodies {
        let actual = glam::DQuat::from_array(body.spin.unwrap().orientation);
        assert!(
            actual
                .to_array()
                .iter()
                .zip(expected.to_array())
                .all(|(a, b)| (a - b).abs() < 1e-10)
        );
        assert!((body.spin.unwrap().angular_momentum[2] - 1.).abs() < 1e-10);
    }
    assert!((bodies[0].motion.position[1] - (0.125 - 0.125 * dt * dt / 2.)).abs() < 1e-12);
    assert!((bodies[1].motion.position[1] - (-0.125 + 0.125 * dt * dt / 2.)).abs() < 1e-12);
    assert!(
        report
            .reaction_angular_balance_residual
            .iter()
            .all(|x| x.abs() < 1e-12)
    );
    assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    assert_eq!(report.supported_intervals, 1);
    assert!(report.max_support_error_m <= 1e-10);
}

#[test]
fn supported_material_point_yaw_is_sustained_over_one_second() {
    use physics::contact::ContactWrench;
    use physics::rigid_motion::MaterialPointForce;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let mut bodies = [rigid_body([0., 0.125, 0.], [0.; 3], 0.)];
    bodies[0].spin.as_mut().unwrap().angular_momentum = [0., 1., 0.];
    let points = [vec![MaterialPointForce {
        local: [0.05, 0., 0.],
        force: [0., -0.3, 0.],
        force_rate: [0., -0.2, 0.],
    }]];
    let dt = 0.01;
    let mut impulse = 0.;
    let mut external_work = 0.;
    let mut reaction_work = 0.;
    let mut residual = 0.;
    let mut intervals = 0;
    let mut max_error = 0_f64;
    let mut max_balance = 0_f64;
    for tick in 0..100 {
        let start = tick as f64 * dt;
        let shifted = [vec![points[0][0].shifted(start).unwrap()]];
        let report = liquid
            .step_with_supported_rigid_body_point_forces(
                dt,
                &mut bodies,
                &world,
                Default::default(),
                1,
                rigid_config(),
                &[ContactWrench::default()],
                &shifted,
                physics::liquid::SupportedWorldConfig {
                    reaction_jerk_tolerance: Some(1e-10),
                    ..Default::default()
                },
            )
            .unwrap_or_else(|error| panic!("tick={tick} error={error:?}"));
        impulse += report.environment_reaction_impulse[1];
        external_work += report.rigid.external_work;
        reaction_work += report.reaction_work;
        residual += report.rigid.integration_energy_residual;
        intervals += report.supported_intervals;
        max_error = max_error.max(report.max_support_error_m);
        max_balance = max_balance.max(
            report
                .reaction_angular_balance_residual
                .iter()
                .map(|x| x.abs())
                .fold(0_f64, f64::max),
        );
        assert!((bodies[0].motion.position[1] - 0.125).abs() < 1e-10);
        assert!(report.max_support_error_m <= 1e-10);
        assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    }
    let expected = glam::DQuat::from_rotation_y(1.);
    let actual = glam::DQuat::from_array(bodies[0].spin.unwrap().orientation);
    assert!(
        actual
            .to_array()
            .iter()
            .zip(expected.to_array())
            .all(|(a, b)| (a - b).abs() < 1e-8)
    );
    assert!((impulse + 10.4).abs() < 1e-8);
    let momentum = glam::DVec3::from_array(bodies[0].spin.unwrap().angular_momentum);
    assert!((momentum - glam::DVec3::Y).length() < 1e-8);
    assert!(external_work.abs() < 1e-9);
    assert!(reaction_work.abs() < 1e-9);
    assert!(residual.abs() < 1e-9);
    assert!(max_balance < 1e-12);
    eprintln!(
        "sustained point yaw: intervals={intervals} max_gap={max_error:e} max_angular_balance={max_balance:e} external_work={external_work:e} reaction_work={reaction_work:e} energy_residual={residual:e} momentum_error={:e}",
        (momentum - glam::DVec3::Y).length()
    );
}
#[test]
fn affine_material_point_pressure_releases_at_zero_force() {
    use physics::contact::ContactWrench;
    use physics::rigid_motion::MaterialPointForce;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap();
    let mut bodies = [rigid_body([0., 0.125, 0.], [0.; 3], 0.)];
    bodies[0].spin.as_mut().unwrap().angular_momentum = [0., 1., 0.];
    let points = [vec![MaterialPointForce {
        local: [0.; 3],
        force: [0., -0.1, 0.],
        force_rate: [0., 2., 0.],
    }]];
    let report = liquid
        .step_with_supported_rigid_body_point_forces(
            0.1,
            &mut bodies,
            &world,
            Default::default(),
            1,
            rigid_config(),
            &[ContactWrench::default()],
            &points,
            physics::liquid::SupportedWorldConfig {
                reaction_jerk_tolerance: Some(1e-10),
                ..Default::default()
            },
        )
        .unwrap();
    // Pressure integral from zero to 0.05: integral(-0.1 + 2t) dt = -0.0025.
    // After release, a=2(t-0.05): v=0.0025, displacement=1/24000.
    assert!((report.environment_reaction_impulse[1] + 0.0025).abs() < 1e-9);
    assert!((bodies[0].motion.velocity[1] - 0.0025).abs() < 1e-9);
    assert!((bodies[0].motion.position[1] - 0.125 - 1. / 24000.).abs() < 1e-9);
    assert!((report.rigid.external_work - 0.0025_f64.powi(2) / 2.).abs() < 1e-9);
    assert!(report.reaction_work.abs() < 1e-9);
    assert!(report.rigid.integration_energy_residual.abs() < 1e-9);
    assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    let actual = glam::DQuat::from_array(bodies[0].spin.unwrap().orientation);
    assert!(
        actual
            .to_array()
            .iter()
            .zip(glam::DQuat::from_rotation_y(0.1).to_array())
            .all(|(a, b)| (a - b).abs() < 1e-8)
    );
}
#[test]
fn supported_material_point_load_keeps_yaw_and_balances_pressure_work() {
    use physics::contact::ContactWrench;
    use physics::rigid_motion::MaterialPointForce;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let mut bodies = [rigid_body([0., 0.125, 0.], [0.; 3], 0.)];
    bodies[0].spin.as_mut().unwrap().angular_momentum = [0., 1., 0.];
    let points = [vec![MaterialPointForce {
        local: [0.05, 0., 0.],
        force: [0., -0.3, 0.],
        force_rate: [0., -0.2, 0.],
    }]];
    let dt = 1e-4;
    let report = liquid
        .step_with_supported_rigid_body_point_forces(
            dt,
            &mut bodies,
            &world,
            Default::default(),
            1,
            rigid_config(),
            &[ContactWrench::default()],
            &points,
            physics::liquid::SupportedWorldConfig {
                reaction_jerk_tolerance: Some(1e-10),
                ..Default::default()
            },
        )
        .unwrap();
    let expected = glam::DQuat::from_rotation_y(dt);
    let actual = glam::DQuat::from_array(bodies[0].spin.unwrap().orientation);
    assert!(
        actual
            .to_array()
            .iter()
            .zip(expected.to_array())
            .all(|(a, b)| (a - b).abs() < 1e-8)
    );
    assert!((bodies[0].motion.position[1] - 0.125).abs() < 1e-10);
    assert!((report.environment_reaction_impulse[1] + 10.3 * dt + 0.1 * dt * dt).abs() < 1e-10);
    assert!(report.rigid.external_work.abs() < 1e-10);
    assert!(report.reaction_work.abs() < 1e-10);
    assert!(
        report
            .reaction_angular_balance_residual
            .iter()
            .all(|v| v.abs() < 1e-12)
    );
    assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    assert!(report.max_support_error_m <= 1e-10);
    // A later interval-budget failure preserves the complete pre-tick state.
    let saved = liquid.clone();
    let original = bodies;
    assert!(
        liquid
            .step_with_supported_rigid_body_point_forces(
                0.01,
                &mut bodies,
                &world,
                Default::default(),
                1,
                rigid_config(),
                &[ContactWrench::default()],
                &points,
                physics::liquid::SupportedWorldConfig {
                    max_interval_s: 0.001,
                    max_intervals: 1,
                    reaction_jerk_tolerance: Some(1e-10),
                    ..Default::default()
                },
            )
            .is_err()
    );
    assert_eq!(liquid, saved);
    assert_eq!(bodies, original);
}
#[test]
fn supported_face_yaw_uses_cubic_corner_motion_without_freezing_spin() {
    use physics::contact::ContactWrench;
    use physics::liquid::LiquidBodyWorld;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let mut bodies = [rigid_body([0., 0.125, 0.], [0.; 3], 0.)];
    bodies[0].spin.as_mut().unwrap().angular_momentum = [0., 1., 0.];
    // Prove that this fixture actually selects the custom point branch.
    let snapshot = world
        .rigid_environment_support_contacts_with_error(0, &bodies[0], 128, 1e-10)
        .unwrap();
    let geometry = snapshot[0];
    let support = physics::contact::NetworkSupport {
        first: 0,
        second: None,
        support: geometry.support,
    };
    let model = world
        .rigid_support_point_motion(
            &bodies,
            &[ContactWrench::default()],
            &[ContactWrench::default()],
            support,
            geometry,
        )
        .unwrap()
        .unwrap();
    assert!(glam::DVec3::from_array(model.velocity).length() > 0.1);
    let mut intervals = 0;
    let mut impulse = 0.;
    for _ in 0..10 {
        let report = liquid
            .step_with_supported_rigid_body_forces(
                0.1,
                &mut bodies,
                &world,
                Default::default(),
                1,
                rigid_config(),
                &[ContactWrench::default()],
                physics::liquid::SupportedWorldConfig {
                    reaction_jerk_tolerance: Some(1e-10),
                    ..Default::default()
                },
            )
            .unwrap();
        intervals += report.supported_intervals;
        impulse += report.environment_reaction_impulse[1];
        assert!(report.max_support_error_m <= 1e-10);
        assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
        assert!(report.reaction_work.abs() < 1e-10);
    }
    let expected = glam::DQuat::from_rotation_y(1.);
    let actual = glam::DQuat::from_array(bodies[0].spin.unwrap().orientation);
    assert!(
        actual
            .to_array()
            .iter()
            .zip(expected.to_array())
            .all(|(a, b)| (a - b).abs() < 1e-9)
    );
    assert!((bodies[0].motion.position[1] - 0.125).abs() < 1e-10);
    assert!((bodies[0].spin.unwrap().angular_momentum[1] - 1.).abs() < 1e-10);
    assert!((impulse + 10.).abs() < 1e-9);
    assert!(intervals >= 100);
}

#[test]
fn supported_vertex_spin_keeps_real_yaw_over_one_second() {
    use physics::contact::ContactWrench;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let q = glam::DQuat::from_rotation_arc(glam::DVec3::splat(-1.).normalize(), -glam::DVec3::Y);
    let height = 3_f64.sqrt() * 0.125;
    let mut bodies = [rigid_body([0., height, 0.], [0.; 3], 0.)];
    let spin = bodies[0].spin.as_mut().unwrap();
    spin.orientation = q.to_array();
    spin.angular_momentum = [0., 1., 0.];
    let before = bodies[0];
    let mut impulse = 0.;
    let mut intervals = 0;
    for _ in 0..10 {
        let report = liquid
            .step_with_supported_rigid_body_forces(
                0.1,
                &mut bodies,
                &world,
                Default::default(),
                1,
                rigid_config(),
                &[ContactWrench::default()],
                physics::liquid::SupportedWorldConfig {
                    reaction_jerk_tolerance: Some(1e-10),
                    ..Default::default()
                },
            )
            .unwrap();
        impulse += report.environment_reaction_impulse[1];
        intervals += report.supported_intervals;
        assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    }
    let expected = glam::DQuat::from_rotation_y(1.) * q;
    let actual = glam::DQuat::from_array(bodies[0].spin.unwrap().orientation);
    assert!(
        actual
            .to_array()
            .iter()
            .zip(expected.to_array())
            .all(|(a, b)| (a - b).abs() < 1e-9)
    );
    assert!((bodies[0].motion.position[1] - height).abs() < 1e-10);
    assert!((bodies[0].spin.unwrap().angular_momentum[1] - 1.).abs() < 1e-10);
    assert!((impulse + 10.).abs() < 1e-9);
    assert!(intervals >= 100);
    assert_ne!(
        bodies[0].spin.unwrap().orientation,
        before.spin.unwrap().orientation
    );
}

#[test]
fn supported_motion_keeps_real_tangential_acceleration_and_separates_work() {
    use physics::contact::ContactWrench;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let mut bodies = [rigid_body([0., 0.125, 0.], [1., 0., 0.], 0.)];
    bodies[0].motion.mass = 2.;
    let report = liquid
        .step_with_supported_rigid_body_forces(
            0.1,
            &mut bodies,
            &world,
            Default::default(),
            1,
            rigid_config(),
            &[ContactWrench {
                force: [4., 0., 0.],
                torque: [0.; 3],
            }],
            Default::default(),
        )
        .unwrap();
    assert!((bodies[0].motion.position[0] - 0.11).abs() < 1e-12);
    assert!((bodies[0].motion.velocity[0] - 1.2).abs() < 1e-12);
    assert!((bodies[0].motion.position[1] - 0.125).abs() < 1e-12);
    assert!(bodies[0].motion.velocity[1].abs() < 1e-11);
    assert!((report.rigid.external_work - 0.44).abs() < 1e-11);
    assert!(report.reaction_work.abs() < 1e-11);
    assert!(report.rigid.integration_energy_residual.abs() < 1e-11);
    assert!((report.environment_reaction_impulse[1] + 2.).abs() < 1e-11);
    // Floor force is 20 N and x(t)=t+t². Its opposite angular impulse
    // about the world origin integrates the moving application point.
    let expected_moment = 20. * (0.1_f64.powi(2) / 2. + 0.1_f64.powi(3) / 3.);
    assert!((report.reaction_angular_impulse[2] - expected_moment).abs() < 1e-11);
    assert!((report.environment_reaction_angular_impulse[2] + expected_moment).abs() < 1e-11);
    assert!(
        report
            .reaction_angular_balance_residual
            .iter()
            .all(|x| x.abs() < 1e-11)
    );

    assert_eq!(
        report.environment_reaction_impulse,
        report.rigid.world.environment_impulse
    );
    assert_eq!(report.rigid.world.dynamics.contacts, 0);
    assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    assert!(report.supported_intervals > 0);
    assert!(report.max_support_error_m < 1e-11);
}
#[test]
fn cubic_collision_detects_inside_step_motion_and_preserves_world_feature() {
    use physics::{
        astrophysics_spin::TorquePolynomial,
        liquid::{GeometryHit, LiquidBodyWorld},
    };
    let mut scene = SceneGraph::new(2);
    let node = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            node,
            crate::BoxCollider {
                half_extents: [0.125, 1., 1.],
            },
        )
        .unwrap();
    let shape = support_box([0.125; 3]);
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![shape], vec![shape]],
    };
    let zero = TorquePolynomial::constant([0.; 3]);
    for (a, j, spin) in [(36., -72., true), (0., 36., false)] {
        let mut initial = rigid_body([-3., 0., 0.], [0.; 3], 0.);
        if !spin {
            initial.spin = None;
        }
        let path = initial
            .prepare_affine_motion([a, 0., 0.], [j, 0., 0.], zero, 1., rigid_config())
            .unwrap();
        let end = path.end();
        assert!((end.motion.position[0] - 3.).abs() < 1e-12);
        let original = path.clone();
        // Both endpoint poses are separated, yet the cubic path crosses the wall.
        let event = world.sweep_rigid_environment_event(0, &path, 256).unwrap();
        let GeometryHit::Contact { fraction, normal } = event.contact.geometry else {
            panic!("missed cubic crossing: {event:?}");
        };
        let mut lo = 0.;
        let mut hi = 1.;
        for _ in 0..64 {
            let mid = (lo + hi) * 0.5;
            if -3. + a * mid * mid / 2. + j * mid * mid * mid / 6. < -0.25 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        assert!((fraction - (lo + hi) * 0.5).abs() < 1e-8);
        assert_eq!(normal, [-1., 0., 0.]);
        assert!((event.contact.witness.unwrap().point[0] + 0.125).abs() < 1e-8);
        assert_eq!(
            FeatureKey::decode(event.feature.unwrap()).unwrap().second,
            0
        );
        assert!(world.sweep_rigid_environment_event(0, &path, 0).is_err());
        assert_eq!(path, original);
        let fixed = rigid_body([0.; 3], [0.; 3], 0.)
            .prepare_motion([0.; 3], [0.; 3], 1., rigid_config())
            .unwrap();
        let pair = world
            .sweep_rigid_pair_event(0, &path, 1, &fixed, 256)
            .unwrap();
        let GeometryHit::Contact {
            fraction: pair_fraction,
            normal: pair_normal,
        } = pair.contact.geometry
        else {
            panic!("missed cubic pair");
        };
        assert!((pair_fraction - fraction).abs() < 1e-8);
        assert_eq!(pair_normal, normal);
        let compound = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![
                vec![shape],
                vec![crate::convex::AffineBox {
                    center: glam::DVec3::X * 0.5,
                    ..shape
                }],
            ],
        };
        let shifted = compound
            .sweep_rigid_pair_event(0, &path, 1, &fixed, 256)
            .unwrap();
        let GeometryHit::Contact {
            fraction: shifted_fraction,
            ..
        } = shifted.contact.geometry
        else {
            panic!("missed compound cubic pair");
        };
        let mut lo = 0.;
        let mut hi = 1.;
        for _ in 0..64 {
            let mid = (lo + hi) * 0.5;
            if -3. + a * mid * mid / 2. + j * mid * mid * mid / 6. < 0.25 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        assert!((shifted_fraction - (lo + hi) * 0.5).abs() < 1e-8);
        assert!((shifted.contact.witness.unwrap().point[0] - 0.375).abs() < 1e-8);
    }
}

#[test]
fn cubic_supported_interval_admits_release_and_rejects_hidden_penetration() {
    use physics::{
        astrophysics_spin::TorquePolynomial,
        liquid::{GeometryHit, LiquidBodyWorld},
    };
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]],
    };
    let initial = rigid_body([0., 0.125, 0.], [0.; 3], 0.);
    let supports = world
        .rigid_environment_support_contacts(0, &initial, 256)
        .unwrap();
    assert!(!supports.is_empty());
    let zero = TorquePolynomial::constant([0.; 3]);
    let release = initial
        .prepare_affine_motion([0.; 3], [0., 6., 0.], zero, 1., rigid_config())
        .unwrap();
    let result = world
        .sweep_supported_rigid_environment_event(0, &release, &supports, 256)
        .unwrap();
    assert!(matches!(result.event.contact.geometry, GeometryHit::Clear));
    // Loaded points must remain on the plane, rather than silently release.
    let loaded: Vec<_> = supports
        .iter()
        .map(|point| physics::liquid::RigidSupportPoint {
            carrying_reaction: true,
            ..*point
        })
        .collect();
    assert!(
        world
            .sweep_supported_rigid_environment_event(0, &release, &loaded, 256)
            .is_err()
    );
    assert_eq!(release.initial(), initial);
    let mut incoming = initial;
    incoming.motion.velocity[1] = 1.;
    // y displacement t - 3t² + 2t³ returns to zero but penetrates the floor.
    let crossing = incoming
        .prepare_affine_motion([0., -6., 0.], [0., 12., 0.], zero, 1., rigid_config())
        .unwrap();
    assert!((crossing.end().motion.position[1] - initial.motion.position[1]).abs() < 1e-12);
    assert!(
        world
            .sweep_supported_rigid_environment_event(0, &crossing, &supports, 256)
            .is_err()
    );
}

#[test]
fn supported_motion_finite_sliding_transmits_moving_arm_torque_without_internal_couple() {
    use physics::contact::ContactWrench;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![
            vec![support_box([0.125; 3])],
            vec![support_box([1., 0.125, 1.])],
        ],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let mut bodies = [
        rigid_body([0., 0.375, 0.], [1., 0., 0.], 0.),
        rigid_body([0., 0.125, 0.], [0.; 3], 0.),
    ];
    bodies[0].motion.mass = 2.;
    bodies[1].motion.mass = 3.;
    let before_query = bodies;
    let before_liquid = liquid.clone();
    let rate_report = liquid
        .rigid_world_reaction_rates(
            &bodies,
            &world,
            &[ContactWrench::default(); 2],
            &[ContactWrench::default(); 2],
            Default::default(),
            physics::contact::ReactionRateConfig {
                reaction: physics::liquid::SupportedWorldConfig::default().reaction,
                jerk_tolerance: 1e-10,
            },
        )
        .unwrap();
    let rates = rate_report.rate.as_ref().unwrap();
    for wrench in &rates.wrenches_rate {
        assert!(
            wrench
                .force
                .iter()
                .chain(&wrench.torque)
                .all(|x| x.abs() < 1e-9)
        );
    }
    let floor_moment_rate: f64 = rate_report
        .reactions
        .supports
        .iter()
        .zip(&rates.forces_rate)
        .filter(|(s, _)| s.second.is_none())
        .map(|(s, f)| s.support.contact.point[0] * f[1] - s.support.contact.point[1] * f[0])
        .sum();
    assert!((floor_moment_rate - 20.).abs() < 1e-9);
    assert!(
        rate_report
            .environment_force_rate
            .iter()
            .all(|x| x.abs() < 1e-9)
    );
    assert_eq!(bodies, before_query);
    assert_eq!(liquid, before_liquid);
    let dt = 1e-4;
    let mut affine_liquid = liquid.clone();
    let mut affine_bodies = bodies;
    let affine_report = affine_liquid
        .step_with_supported_rigid_body_forces(
            dt,
            &mut affine_bodies,
            &world,
            Default::default(),
            2,
            rigid_config(),
            &[ContactWrench::default(); 2],
            physics::liquid::SupportedWorldConfig {
                reaction_jerk_tolerance: Some(1e-10),
                ..Default::default()
            },
        )
        .unwrap();
    assert!((affine_bodies[0].motion.position[0] - dt).abs() < 1e-12);
    // Changing pressure cancels the moving-arm torque at the floor.
    assert!(affine_bodies[1].spin.unwrap().angular_momentum[2].abs() < 1e-12);
    assert!(
        affine_report
            .reaction_angular_balance_residual
            .iter()
            .all(|x| x.abs() < 1e-12)
    );
    assert!((affine_report.environment_reaction_impulse[1] + 50. * dt).abs() < 1e-12);
    assert!(affine_report.reaction_work.abs() < 1e-12);
    assert_eq!(affine_report.rigid.world.dynamics.dissipated_energy, 0.);
    let admission = physics::liquid::SupportedWorldConfig::default();
    let report = liquid
        .step_with_supported_rigid_body_forces(
            dt,
            &mut bodies,
            &world,
            Default::default(),
            2,
            rigid_config(),
            &[ContactWrench::default(); 2],
            admission,
        )
        .unwrap();
    assert!((bodies[0].motion.position[0] - dt).abs() < 1e-12);
    // Moving top application point imposes -20t torque on the lower body.
    assert!((bodies[1].spin.unwrap().angular_momentum[2] + 10. * dt * dt).abs() < 1e-12);
    assert!(
        report
            .reaction_angular_balance_residual
            .iter()
            .all(|x| x.abs() < 1e-12)
    );
    assert!((report.environment_reaction_impulse[1] + 50. * dt).abs() < 1e-12);
    assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
    assert!(report.max_support_error_m <= admission.max_geometry_error_m);
}
#[test]
fn affine_supported_sliding_reassembles_pressure_and_closes_long_interval_moments() {
    use physics::contact::ContactWrench;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![
            vec![support_box([0.125; 3])],
            vec![support_box([1., 0.125, 1.])],
        ],
    };
    for interval in [0.01, 0.005] {
        let mut liquid = Liquid::new(
            Vec::new(),
            vec![Material::WATER],
            Config {
                gravity: [0., -10., 0.],
                ..Default::default()
            },
        )
        .unwrap();
        let mut bodies = [
            rigid_body([0., 0.375, 0.], [1., 0., 0.], 0.),
            rigid_body([0., 0.125, 0.], [0.; 3], 0.),
        ];
        bodies[0].motion.mass = 2.;
        bodies[1].motion.mass = 3.;
        let mut angular = 0.;
        let mut impulse = 0.;
        let mut supported = 0;
        for _ in 0..10 {
            let report = liquid
                .step_with_supported_rigid_body_forces(
                    0.02,
                    &mut bodies,
                    &world,
                    Default::default(),
                    2,
                    rigid_config(),
                    &[ContactWrench::default(); 2],
                    physics::liquid::SupportedWorldConfig {
                        reaction_jerk_tolerance: Some(1e-10),
                        max_interval_s: interval,
                        ..Default::default()
                    },
                )
                .unwrap();
            angular += report.environment_reaction_angular_impulse[2];
            impulse += report.environment_reaction_impulse[1];
            supported += report.supported_intervals;
            assert!(
                report
                    .reaction_angular_balance_residual
                    .iter()
                    .all(|x| x.abs() < 1e-10)
            );
            assert!(report.reaction_work.abs() < 1e-10);
            assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
        }
        assert!(supported >= 20);
        assert!((bodies[0].motion.position[0] - 0.2).abs() < 1e-10);
        assert!((bodies[0].motion.position[1] - 0.375).abs() < 1e-10);
        assert!((bodies[0].motion.velocity[0] - 1.).abs() < 1e-10);
        assert!(
            bodies[1]
                .spin
                .unwrap()
                .angular_momentum
                .iter()
                .all(|x| x.abs() < 1e-10)
        );
        assert!((impulse + 10.).abs() < 1e-9);
        // Opposite floor moment balances gravity on the moving 2 kg top body:
        // integral -m*g*x(t) dt = -20*(0.2²/2).
        assert!((angular + 0.4).abs() < 1e-9);
    }
}

#[test]
fn off_center_material_load_transmits_force_and_moment_through_unequal_stack() {
    use physics::contact::ContactWrench;
    use physics::rigid_motion::MaterialPointForce;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.03125; 3])]; 3],
    };
    let liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    for order in [[0, 1, 2], [2, 0, 1]] {
        let mut liquid = liquid.clone();
        let originals = [(2., 0.15625), (3., 0.09375), (5., 0.03125)];
        let top = order.iter().position(|i| *i == 0).unwrap();
        let mut bodies: Vec<_> = order
            .map(|i| originals[i])
            .into_iter()
            .map(|(mass, y)| {
                let mut b = rigid_body([0., y, 0.], [0.; 3], 0.);
                b.motion.mass = mass;
                b
            })
            .collect();
        let before = bodies.clone();
        let recipe = MaterialPointForce {
            local: [0.01, 0., 0.],
            force: [0., -0.6, 0.],
            force_rate: [0., -0.4, 0.],
        };
        let dt = 0.05;
        let mut impulse = glam::DVec3::ZERO;
        let mut angular = glam::DVec3::ZERO;
        let mut intervals = 0;
        for tick in 0..10 {
            let mut points = [vec![], vec![], vec![]];
            points[top].push(recipe.shifted(tick as f64 * dt).unwrap());
            let report = liquid
                .step_with_supported_rigid_body_point_forces(
                    dt,
                    &mut bodies,
                    &world,
                    Default::default(),
                    3,
                    rigid_config(),
                    &[ContactWrench::default(); 3],
                    &points,
                    physics::liquid::SupportedWorldConfig {
                        reaction_jerk_tolerance: Some(1e-10),
                        ..Default::default()
                    },
                )
                .unwrap_or_else(|e| panic!("tick={tick}: {e:?}"));
            impulse += glam::DVec3::from_array(report.environment_reaction_impulse);
            angular += glam::DVec3::from_array(report.environment_reaction_angular_impulse);
            intervals += report.supported_intervals;
            assert!(
                report
                    .reaction_angular_balance_residual
                    .iter()
                    .all(|x| x.abs() < 1e-12)
            );
            assert!(report.reaction_work.abs() < 1e-10);
            assert!(report.rigid.external_work.abs() < 1e-10);
            assert!(report.rigid.integration_energy_residual.abs() < 1e-10);
            assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
            assert!(report.max_support_error_m <= 1e-10);
        }
        // Independent load integral: 100 N gravity + (0.6 + 0.4t) N.
        // The only external load with a world moment has a 0.01 m X arm.
        assert!((impulse - glam::DVec3::new(0., -50.35, 0.)).length() < 1e-9);
        assert!((angular - glam::DVec3::new(0., 0., -0.0035)).length() < 1e-9);
        for (a, b) in bodies.iter().zip(&before) {
            assert!(
                (glam::DVec3::from_array(a.motion.position)
                    - glam::DVec3::from_array(b.motion.position))
                .length()
                    < 1e-10
            );
            assert!(glam::DVec3::from_array(a.motion.velocity).length() < 1e-10);
            assert!(glam::DVec3::from_array(a.spin.unwrap().angular_momentum).length() < 1e-10);
        }
        eprintln!(
            "point-loaded stack: order={order:?} intervals={intervals} impulse={impulse:?} angular_impulse={angular:?}"
        );
    }
}
#[test]
fn supported_motion_stack_holds_and_releases_under_changed_load_without_rest_clamping() {
    use physics::contact::ContactWrench;
    for rate in [None, Some(1e-10)] {
        let admission = physics::liquid::SupportedWorldConfig {
            reaction_jerk_tolerance: rate,
            ..Default::default()
        };
        let scene = support_floor();
        let world = SceneBodyWorld {
            environment: SceneGeometry(crate::static_world(&scene).unwrap()),
            templates: vec![vec![support_box([0.03125; 3])]; 3],
        };
        let mut liquid = Liquid::new(
            Vec::new(),
            vec![Material::WATER],
            Config {
                gravity: [0., -10., 0.],
                ..Default::default()
            },
        )
        .unwrap();
        let mut bodies: Vec<_> = [(2., 0.15625), (3., 0.09375), (5., 0.03125)]
            .into_iter()
            .map(|(m, y)| {
                let mut b = rigid_body([0., y, 0.], [0.; 3], 0.);
                b.motion.mass = m;
                b
            })
            .collect();
        let before = bodies.clone();
        let mut impulse = 0.;
        for _ in 0..10 {
            let report = liquid
                .step_with_supported_rigid_body_forces(
                    0.1,
                    &mut bodies,
                    &world,
                    Default::default(),
                    3,
                    rigid_config(),
                    &[ContactWrench::default(); 3],
                    admission,
                )
                .unwrap();
            impulse += report.environment_reaction_impulse[1];
            assert_eq!(report.rigid.world.dynamics.contacts, 0);
            assert!(report.reaction_work.abs() < 1e-10);
        }
        assert!((impulse + 100.).abs() < 1e-9);
        for (a, b) in bodies.iter().zip(before) {
            assert!((a.motion.position[1] - b.motion.position[1]).abs() < 1e-11);
            assert!(glam::DVec3::from_array(a.motion.velocity).length() < 1e-10);
        }
        let report = liquid
            .step_with_supported_rigid_body_forces(
                0.1,
                &mut bodies,
                &world,
                Default::default(),
                3,
                rigid_config(),
                &[
                    ContactWrench {
                        force: [0., 40., 0.],
                        torque: [0.; 3],
                    },
                    ContactWrench::default(),
                    ContactWrench::default(),
                ],
                admission,
            )
            .unwrap();
        assert!((bodies[0].motion.position[1] - 0.20625).abs() < 1e-10);
        assert!((bodies[0].motion.velocity[1] - 1.).abs() < 1e-10);
        assert!((report.environment_reaction_impulse[1] + 8.).abs() < 1e-9);
    }
}
#[test]
fn supported_motion_searches_other_walls_and_rolls_back_late_interval_budget() {
    use physics::contact::ContactWrench;
    let mut scene = support_floor();
    let wall = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(0.5, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            wall,
            crate::BoxCollider {
                half_extents: [0.0625, 2., 1.],
            },
        )
        .unwrap();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0., -10., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let mut bodies = [rigid_body([0., 0.125, 0.], [5., 0., 0.], 0.)];
    bodies[0].motion.mass = 2.;
    let initial = bodies;
    let fluid_before = liquid.clone();
    let config = physics::liquid::DynamicWorldConfig {
        contact: physics::liquid::ContactConfig {
            restitution: 1.,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        liquid.step_with_supported_rigid_body_forces(
            0.1,
            &mut bodies,
            &world,
            config,
            1,
            rigid_config(),
            &[ContactWrench::default()],
            physics::liquid::SupportedWorldConfig {
                max_intervals: 2,
                ..Default::default()
            }
        ),
        Err(physics::liquid::Error::CollisionBudget)
    );
    assert_eq!(bodies, initial);
    assert_eq!(liquid, fluid_before);
    let report = liquid
        .step_with_supported_rigid_body_forces(
            0.1,
            &mut bodies,
            &world,
            config,
            1,
            rigid_config(),
            &[ContactWrench::default()],
            Default::default(),
        )
        .unwrap();
    assert_eq!(report.rigid.world.dynamics.contacts, 1);
    assert!((bodies[0].motion.velocity[0] + 5.).abs() < 1e-9);
    assert!((bodies[0].motion.position[0] - 0.125).abs() < 1e-8);
    assert!((report.rigid.world.environment_impulse[0] - 20.).abs() < 1e-9);
    assert!((report.rigid.world.environment_impulse[1] + 2.).abs() < 1e-9);
    assert_eq!(report.rigid.world.dynamics.dissipated_energy, 0.);
}
fn support_query_config() -> physics::contact::ReactionConfig {
    physics::contact::ReactionConfig {
        max_sweeps: 8192,
        acceleration_tolerance: 1e-9,
        normal_velocity_tolerance: 1e-10,
    }
}
fn support_box(half: [f64; 3]) -> crate::convex::AffineBox {
    crate::convex::AffineBox {
        center: glam::DVec3::ZERO,
        edges: std::array::from_fn(|k| {
            [glam::DVec3::X, glam::DVec3::Y, glam::DVec3::Z][k] * half[k]
        }),
    }
}
fn support_floor() -> SceneGraph {
    let mut scene = SceneGraph::new(2);
    let node = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(0., -0.125, 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            node,
            crate::BoxCollider {
                half_extents: [4., 0.125, 4.],
            },
        )
        .unwrap();
    scene
}
#[test]
fn scene_support_query_uses_authored_owners_gravity_and_all_corner_branches_read_only() {
    use physics::contact::ContactWrench;
    let mut scene = SceneGraph::new(8);
    let wall = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            wall,
            crate::BoxCollider {
                half_extents: [1.; 3],
            },
        )
        .unwrap();
    let body = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(2., 2., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            body,
            crate::BoxCollider {
                half_extents: [1.; 3],
            },
        )
        .unwrap();
    scene
        .insert_component(
            body,
            crate::LiquidBody {
                mass_kg: 2.,
                initial_velocity_m_s: [0.; 3],
            },
        )
        .unwrap();
    scene
        .insert_component(
            body,
            crate::LiquidMassDistribution {
                parts: vec![crate::LiquidMassPart {
                    mass_kg: 2.,
                    center_m: [0.; 3],
                    half_edges_m: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                }],
            },
        )
        .unwrap();
    let runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [-10., -15., 0.],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let before = runtime.clone();
    let pose = scene.local(body).unwrap();
    let torque = ContactWrench {
        force: [0.; 3],
        torque: [0., 0., 10.],
    };
    let report = runtime
        .support_reactions(
            &scene,
            &[(body, torque)],
            Default::default(),
            support_query_config(),
        )
        .unwrap();
    let equivalent = runtime
        .support_reactions(
            &scene,
            &[(
                body,
                ContactWrench {
                    force: [2.4, -3.8, 0.],
                    torque: [0., 0., 10. - 0.86],
                },
            )],
            Default::default(),
            support_query_config(),
        )
        .unwrap();
    scene
        .insert_component(
            body,
            crate::RigidPointLoads {
                points: vec![crate::RigidPointForce {
                    root_point_m: [0.1, 0.2, 0.],
                    world_force_n: [2., -3., 0.],
                    world_force_rate_n_s: [1., -2., 0.],
                }],
            },
        )
        .unwrap();
    let mut loaded_runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [-10., -15., 0.],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    loaded_runtime.body[0].point_elapsed_s = 0.4;
    let loaded_before = loaded_runtime.clone();
    let loaded_report = loaded_runtime
        .support_reactions(
            &scene,
            &[(body, torque)],
            Default::default(),
            support_query_config(),
        )
        .unwrap();
    assert_eq!(loaded_report, equivalent);
    assert_eq!(loaded_runtime, loaded_before);
    scene
        .remove_component::<crate::RigidPointLoads>(body)
        .unwrap();
    assert_eq!(report.owners, vec![body]);
    assert_eq!(report.reactions.supports.len(), 4);
    assert!(
        report
            .reactions
            .supports
            .iter()
            .any(|s| s.support.contact.normal == [1., 0., 0.])
    );
    assert!(
        report
            .reactions
            .supports
            .iter()
            .any(|s| s.support.contact.normal == [0., 1., 0.])
    );
    let reaction = report.reactions.reaction.unwrap();
    assert!(reaction.acceleration_residual <= 1e-9);
    let wrench = reaction.wrenches[0];
    assert!((wrench.force[0] - 20.).abs() < 1e-7);
    assert!((wrench.force[1] - 30.).abs() < 1e-7);
    assert!((wrench.torque[2] + 10.).abs() < 1e-7);
    assert_eq!(runtime, before);
    assert_eq!(scene.local(body).unwrap(), pose);
    assert!(
        runtime
            .support_reactions(
                &scene,
                &[(body, torque), (body, torque)],
                Default::default(),
                support_query_config()
            )
            .is_err()
    );
    assert!(
        runtime
            .support_reactions(
                &scene,
                &[(wall, torque)],
                Default::default(),
                support_query_config()
            )
            .is_err()
    );
    scene.set_active(body, false).unwrap();
    assert!(
        runtime
            .support_reactions(
                &scene,
                &[(body, torque)],
                Default::default(),
                support_query_config()
            )
            .is_err()
    );
    assert_eq!(runtime, before);
    scene.set_active(body, true).unwrap();
    scene
        .component_mut::<crate::BoxCollider>(body)
        .unwrap()
        .unwrap()
        .half_extents[0] = 0.5;
    assert!(
        runtime
            .support_reactions(&scene, &[], Default::default(), support_query_config())
            .is_err()
    );
    assert_eq!(runtime, before);
}
#[test]
fn scene_support_network_discovers_all_stack_patches_and_load_paths() {
    use physics::contact::ContactWrench;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.03125; 3])]; 3],
    };
    let bodies: Vec<_> = [(2., 0.15625), (3., 0.09375), (5., 0.03125)]
        .into_iter()
        .map(|(mass, y)| {
            let mut body = rigid_body([0., y, 0.], [0.; 3], 0.);
            body.motion.mass = mass;
            body
        })
        .collect();
    for order in [[0, 1, 2], [2, 0, 1]] {
        let input: Vec<_> = order.map(|i| bodies[i]).into_iter().collect();
        let before = input.clone();
        let external: Vec<_> = input
            .iter()
            .map(|b| ContactWrench {
                force: [0., -10. * b.motion.mass, 0.],
                torque: [0.; 3],
            })
            .collect();
        let report = physics::liquid::resolve_rigid_world_reactions(
            &input,
            &world,
            &external,
            Default::default(),
            support_query_config(),
        )
        .unwrap();
        assert_eq!(report.queries, 6);
        assert_eq!(report.supports.len(), 12);
        assert_eq!(input, before);
        let reaction = report.reaction.unwrap();
        assert!(reaction.acceleration_residual <= 1e-9);
        assert_eq!(reaction.instantaneous_power, 0.);
        assert!((report.environment_force[1] + 100.).abs() < 1e-7);
        for (wrench, load) in reaction.wrenches.iter().zip(&external) {
            for k in 0..3 {
                assert!((wrench.force[k] + load.force[k]).abs() < 1e-7);
                assert!((wrench.torque[k] + load.torque[k]).abs() < 1e-7);
            }
        }
        let strength = |mass: f64| -> f64 {
            report
                .supports
                .iter()
                .zip(&reaction.forces)
                .filter(|(s, _)| {
                    input[s.first].motion.mass == mass
                        && s.second.is_some()
                        && input[s.second.unwrap()].motion.mass > mass
                })
                .map(|(_, f)| f[1])
                .sum()
        };
        if order == [0, 1, 2] {
            assert!((strength(2.) - 20.).abs() < 1e-7);
            assert!((strength(3.) - 50.).abs() < 1e-7);
        }
    }
}
#[test]
fn scene_support_network_is_covariant_under_a_proper_world_frame_permutation() {
    use physics::contact::ContactWrench;
    let q = glam::DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5);
    let mut scene = support_floor();
    let floor = scene.components::<crate::BoxCollider>().next().unwrap().0;
    let mut pose = scene.local(floor).unwrap();
    pose.translation = (q * pose.translation.as_dvec3()).as_vec3();
    pose.rotation = q.as_quat();
    scene.set_local(floor, pose).unwrap();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.03125; 3])]; 3],
    };
    let bodies: Vec<_> = [(2., 0.15625), (3., 0.09375), (5., 0.03125)]
        .into_iter()
        .map(|(mass, y)| {
            let mut b = rigid_body((q * glam::DVec3::Y * y).to_array(), [0.; 3], 0.);
            b.motion.mass = mass;
            b.spin.as_mut().unwrap().orientation = q.to_array();
            b
        })
        .collect();
    let external: Vec<_> = bodies
        .iter()
        .map(|b| ContactWrench {
            force: (q * glam::DVec3::Y * (-10. * b.motion.mass)).to_array(),
            torque: [0.; 3],
        })
        .collect();
    let report = physics::liquid::resolve_rigid_world_reactions(
        &bodies,
        &world,
        &external,
        Default::default(),
        support_query_config(),
    )
    .unwrap();
    assert_eq!(report.supports.len(), 12);
    assert!(
        (glam::DVec3::from_array(report.environment_force) + q * glam::DVec3::Y * 100.).length()
            < 1e-7
    );
    for (w, load) in report.reaction.unwrap().wrenches.iter().zip(external) {
        assert!(
            (glam::DVec3::from_array(w.force) + glam::DVec3::from_array(load.force)).length()
                < 1e-7
        );
        assert!(glam::DVec3::from_array(w.torque).length() < 1e-7);
    }
}
#[test]
fn scene_support_network_transmits_beam_force_and_torque_to_finite_supports() {
    use physics::contact::ContactWrench;
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![
            vec![support_box([1.25, 0.125, 0.25])],
            vec![support_box([0.125, 0.125, 0.25])],
            vec![support_box([0.125, 0.125, 0.25])],
        ],
    };
    let mut bodies = [
        rigid_body([0., 0.375, 0.], [0.; 3], 0.),
        rigid_body([-1., 0.125, 0.], [0.; 3], 0.),
        rigid_body([1., 0.125, 0.], [0.; 3], 0.),
    ];
    for (b, m) in bodies.iter_mut().zip([2., 3., 5.]) {
        b.motion.mass = m;
    }
    let before = bodies;
    let external = [
        ContactWrench {
            force: [0., -20., 0.],
            torque: [0., 0., 6.],
        },
        ContactWrench {
            force: [0., -30., 0.],
            torque: [0.; 3],
        },
        ContactWrench {
            force: [0., -50., 0.],
            torque: [0.; 3],
        },
    ];
    let report = physics::liquid::resolve_rigid_world_reactions(
        &bodies,
        &world,
        &external,
        Default::default(),
        support_query_config(),
    )
    .unwrap();
    assert_eq!(report.supports.len(), 16);
    let reaction = report.reaction.unwrap();
    assert_eq!(bodies, before);
    assert!((report.environment_force[1] + 100.).abs() < 1e-7);
    for (wrench, load) in reaction.wrenches.iter().zip(&external) {
        for k in 0..3 {
            assert!((wrench.force[k] + load.force[k]).abs() < 1e-7);
            assert!((wrench.torque[k] + load.torque[k]).abs() < 1e-7);
        }
    }
    let mut ground_moment = [0.; 3];
    for (entry, force) in report.supports.iter().zip(&reaction.forces) {
        if entry.second.is_none() {
            let moment = glam::DVec3::from_array(entry.support.contact.point)
                .cross(-glam::DVec3::from_array(*force));
            for k in 0..3 {
                ground_moment[k] += moment[k];
            }
        }
    }
    let applied_moment: glam::DVec3 = bodies
        .iter()
        .zip(&external)
        .map(|(body, load)| {
            glam::DVec3::from_array(body.motion.position).cross(glam::DVec3::from_array(load.force))
                + glam::DVec3::from_array(load.torque)
        })
        .sum();
    assert_eq!(applied_moment.z, -14.);
    assert!((glam::DVec3::from_array(ground_moment) - applied_moment).length() < 1e-7);
}
#[test]
fn scene_support_network_rejects_overlap_and_late_budgets_without_state_writes() {
    use physics::contact::ContactWrench;
    use physics::liquid::{DynamicWorldConfig, Error};
    let scene = support_floor();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![support_box([0.125; 3])]; 2],
    };
    let bodies = [
        rigid_body([0., 0.375, 0.], [0.; 3], 0.),
        rigid_body([0., 0.125, 0.], [0.; 3], 0.),
    ];
    let before = bodies;
    let external = [ContactWrench {
        force: [0., -10., 0.],
        torque: [0.; 3],
    }; 2];
    for limits in [
        DynamicWorldConfig {
            max_queries: 2,
            ..Default::default()
        },
        DynamicWorldConfig {
            max_contacts: 7,
            ..Default::default()
        },
    ] {
        assert_eq!(
            physics::liquid::resolve_rigid_world_reactions(
                &bodies,
                &world,
                &external,
                limits,
                support_query_config()
            ),
            Err(Error::CollisionBudget)
        );
        assert_eq!(bodies, before);
    }
    let mut overlapping = bodies;
    overlapping[1].motion.position[1] -= 0.01;
    let bad_before = overlapping;
    assert_eq!(
        physics::liquid::resolve_rigid_world_reactions(
            &overlapping,
            &world,
            &external,
            Default::default(),
            support_query_config()
        ),
        Err(Error::InitialOverlap)
    );
    assert_eq!(overlapping, bad_before);
    let mut separated = bodies;
    separated[0].motion.position[1] += 0.01;
    separated[1].motion.position[1] += 0.01;
    let report = physics::liquid::resolve_rigid_world_reactions(
        &separated,
        &world,
        &external,
        Default::default(),
        support_query_config(),
    )
    .unwrap();
    assert_eq!(report.supports.len(), 4);
    assert_eq!(report.environment_force, [0.; 3]);
    let mut outgoing = bodies;
    outgoing[0].motion.velocity[1] = 1.;
    let report = physics::liquid::resolve_rigid_world_reactions(
        &outgoing,
        &world,
        &external,
        Default::default(),
        support_query_config(),
    )
    .unwrap();
    assert!((report.environment_force[1] + 10.).abs() < 1e-7);
}
fn rigid_config() -> physics::spin_path::Config {
    physics::spin_path::Config {
        max_angular_error_rad: 1e-5,
        min_step_s: 1e-9,
        max_arcs: 10000,
        max_trials: 30000,
    }
}
fn rigid_body(position: [f64; 3], velocity: [f64; 3], z: f64) -> physics::contact::ContactBody {
    physics::contact::ContactBody {
        motion: physics::gravity::Body {
            mass: 1.,
            position,
            velocity,
        },
        spin: Some(physics::astrophysics_spin::Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0., z],
            inertia: [1.; 3],
        }),
    }
}
fn rigid_liquid(particles: Vec<physics::liquid::Particle>) -> Liquid {
    Liquid::new(
        particles,
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            max_particles: 8,
            ..Config::default()
        },
    )
    .unwrap()
}
fn elastic_rigid() -> physics::liquid::DynamicWorldConfig {
    physics::liquid::DynamicWorldConfig {
        contact: physics::liquid::ContactConfig {
            restitution: 1.,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn rigid_shapes() -> [crate::convex::AffineBox; 2] {
    [
        crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [
                glam::DVec3::X * 0.04,
                glam::DVec3::Y * 0.02,
                glam::DVec3::Z * 0.02,
            ],
        },
        crate::convex::AffineBox {
            center: glam::DVec3::X * 0.04,
            edges: [
                glam::DVec3::X * 0.04,
                glam::DVec3::Y * 2.,
                glam::DVec3::Z * 0.02,
            ],
        },
    ]
}
#[test]
fn scene_geometry_and_shared_core_advance_real_off_center_rigid_collision() {
    let scene = SceneGraph::new(1);
    let shapes = rigid_shapes();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: shapes.map(|s| vec![s]).to_vec(),
    };
    let mut liquid = rigid_liquid(Vec::new());
    let mut bodies = [
        rigid_body([-0.1, 1., 0.], [3., 0., 0.], 0.),
        rigid_body([0.; 3], [0.; 3], 0.),
    ];
    let before = bodies.iter().map(|b| b.energy().unwrap()).sum::<f64>();
    let report = liquid
        .step_with_rigid_body_world(0.1, &mut bodies, &world, elastic_rigid(), 2, rigid_config())
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    assert!((bodies[0].motion.velocity[0] - 1.).abs() < 1e-11);
    assert!((bodies[1].motion.velocity[0] - 2.).abs() < 1e-11);
    assert!((bodies[1].spin.unwrap().angular_momentum[2] + 2.).abs() < 1e-11);
    assert!((bodies[1].spin.unwrap().orientation[2] - (-0.08_f64).sin()).abs() < 1e-11);
    assert!(
        (bodies.iter().map(|b| b.energy().unwrap()).sum::<f64>()
            + report.dynamics.dissipated_energy
            - before)
            .abs()
            < 1e-11
    );
}
#[test]
fn scene_geometry_particle_recoil_rotates_the_body_and_restores_on_budget_failure() {
    let scene = SceneGraph::new(1);
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![rigid_shapes()[1]]],
    };
    let mut liquid = rigid_liquid(vec![physics::liquid::Particle {
        position: [-0.07, 1., 0.],
        velocity: [3., 0., 0.],
        mass: 1.,
        material: 0,
    }]);
    let mut bodies = [rigid_body([0.; 3], [0.; 3], 0.)];
    let originals = (liquid.clone(), bodies);
    let mut limited = elastic_rigid();
    limited.contact.max_candidates = 0;
    assert!(
        liquid
            .step_with_rigid_body_world(0.03, &mut bodies, &world, limited, 1, rigid_config())
            .is_err()
    );
    assert_eq!((liquid.clone(), bodies), originals);
    let report = liquid
        .step_with_rigid_body_world(
            0.03,
            &mut bodies,
            &world,
            elastic_rigid(),
            1,
            rigid_config(),
        )
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    assert!((liquid.particles()[0].velocity[0] + bodies[0].motion.velocity[0] - 3.).abs() < 1e-11);
    assert!((bodies[0].spin.unwrap().angular_momentum[2] + 2.).abs() < 1e-11);
    assert_ne!(bodies[0].spin.unwrap().orientation, [0., 0., 0., 1.]);
}
#[test]
fn authored_static_wall_reflects_a_spinning_body_with_boundary_impulse_balance() {
    let mut scene = SceneGraph::new(1);
    let node = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(0.04, 0., 0.),
                ..Transform::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            node,
            crate::BoxCollider {
                half_extents: [0.04, 2., 0.02],
            },
        )
        .unwrap();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![rigid_shapes()[0]]],
    };
    let mut liquid = rigid_liquid(Vec::new());
    let mut bodies = [rigid_body([-0.1, 1., 0.], [3., 0., 0.], 0.3)];
    let energy = bodies[0].energy().unwrap();
    let report = liquid
        .step_with_rigid_body_world(0.1, &mut bodies, &world, elastic_rigid(), 1, rigid_config())
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    assert!(bodies[0].motion.velocity[0] < 0.);
    assert!((bodies[0].motion.velocity[0] + report.environment_impulse[0] - 3.).abs() < 1e-11);
    assert!(
        (bodies[0].energy().unwrap() + report.dynamics.dissipated_energy - energy).abs() < 1e-11
    );
    assert!((bodies[0].spin.unwrap().angular_momentum[2] - 0.3).abs() > 1e-3);
}
#[test]
fn moving_body_contact_witness_is_transported_to_contact_time() {
    use physics::liquid::LiquidBodyWorld;
    let scene = SceneGraph::new(1);
    let shape = crate::convex::AffineBox {
        center: glam::DVec3::ZERO,
        edges: [
            glam::DVec3::X * 0.5,
            glam::DVec3::Y * 0.5,
            glam::DVec3::Z * 0.5,
        ],
    };
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![shape], vec![shape]],
    };
    let a = physics::liquid::TranslatingBody {
        mass: 1.,
        position: [-3., 0., 0.],
        velocity: [4., 0., 0.],
    };
    let b = physics::liquid::TranslatingBody {
        mass: 1.,
        position: [0.; 3],
        velocity: [1., 0., 0.],
    };
    let hit = world.sweep_body_pair_contact(0, &a, 1, &b, 1., 10).unwrap();
    let physics::liquid::GeometryHit::Contact { fraction, .. } = hit.geometry else {
        panic!("contact expected");
    };
    assert!((fraction - 2. / 3.).abs() < 1e-12);
    let witness = hit.witness.unwrap();
    assert!((witness.point[0] - (-0.5 + fraction)).abs() < witness.tolerance_m + 1e-12);
    let particle = physics::liquid::Particle {
        position: [-3., 0., 0.],
        velocity: [4., 0., 0.],
        mass: 1.,
        material: 0,
    };
    let hit = world
        .sweep_particle_body_contact(&particle, 0.5, 1, &b, 1., 10)
        .unwrap();
    assert_eq!(
        hit.geometry,
        world
            .sweep_particle_body(&particle, 0.5, 1, &b, 1., 10)
            .unwrap()
    );
    assert!((hit.witness.unwrap().point[0] - witness.point[0]).abs() < 1e-12);
}

fn fixture(max_particles: usize) -> (SceneGraph, NodeId, NodeId, SceneLiquidRuntime) {
    let mut scene = SceneGraph::new(2);
    let mut nodes = Vec::new();
    for x in [0., 10.] {
        let node = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(x, 0., 0.),
                    ..Transform::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                node,
                LiquidSource {
                    pulses: vec![LiquidPulse {
                        start_s: 0.,
                        duration_s: 1.,
                        volume_m3: 0.001,
                        speed_m_s: 2.,
                    }],
                    density_kg_m3: 1000.,
                    particle_volume_m3: 0.001,
                    nozzle_radius_m: 0.,
                    direction: [1., 0., 0.],
                    material_asset: "water".into(),
                },
            )
            .unwrap();
        nodes.push(node);
    }
    let runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("water".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            max_particles,
            ..Config::default()
        },
        2,
    )
    .unwrap();
    (scene, nodes[0], nodes[1], runtime)
}

#[test]
fn shared_world_emits_at_authored_origins_and_pauses_inactive_sources() {
    let (mut scene, first, second, mut runtime) = fixture(10);
    let report = runtime.tick(&scene, 0.001, None).unwrap();
    assert_eq!(report.emissions.len(), 2);
    let particles = runtime.liquid().particles();
    assert_eq!(particles.len(), 2);
    assert!((runtime.liquid().mass() - 0.002).abs() < 1e-15);
    for (p, x) in particles.iter().zip([0., 10.]) {
        assert!((p.position[0] - x - 0.002).abs() < 1e-12);
        assert_eq!(p.velocity, [2., 0., 0.]);
    }
    scene.set_active(second, false).unwrap();
    runtime.tick(&scene, 0.001, None).unwrap();
    assert_eq!(runtime.source_elapsed(first), Some(0.002));
    assert_eq!(runtime.source_elapsed(second), Some(0.001));
    assert_eq!(runtime.liquid().particles().len(), 3);
    assert!((runtime.liquid().particles()[1].position[0] - 10.004).abs() < 1e-12);
}

#[test]
fn moving_nozzle_transfers_world_velocity_and_admitted_preflight_succeeds() {
    let (mut scene, first, second, mut runtime) = fixture(10);
    crate::validate_game_descriptors_with_liquid_runtime(&scene, 2, &runtime).unwrap();
    assert!(crate::validate_game_descriptors(&scene, 2).is_err());
    scene.set_active(second, false).unwrap();
    let mut transform = scene.local(first).unwrap();
    transform.translation.x = 0.01;
    scene.set_local(first, transform).unwrap();
    runtime.tick(&scene, 0.01, None).unwrap();
    let p = runtime.liquid().particles()[0];
    let nozzle_velocity = f64::from(transform.translation.x) / 0.01;
    assert!((p.velocity[0] - 2. - nozzle_velocity).abs() < 1e-12);
    assert!(
        (p.position[0] - f64::from(transform.translation.x) - p.velocity[0] * 0.01).abs() < 1e-12
    );
}

#[test]
fn rotated_wall_uses_exact_geometry_and_oblique_normal() {
    let (mut scene, first, second, _) = fixture(10);
    scene.set_active(second, false).unwrap();
    scene.remove_component::<LiquidSource>(second).unwrap();
    scene
        .set_local(
            second,
            Transform {
                translation: glam::Vec3::new(0.1, 0., 0.),
                rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                ..Transform::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            second,
            crate::BoxCollider {
                half_extents: [0.01, 2., 2.],
            },
        )
        .unwrap();
    scene.set_active(second, true).unwrap();
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("water".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            ..Config::default()
        },
        1,
    )
    .unwrap();
    runtime.tick(&scene, 0.05, None).unwrap();
    let p = runtime.liquid().particles()[0];
    assert!((p.velocity[0] - 1.).abs() < 1e-6);
    assert!((p.velocity[1] + 1.).abs() < 1e-6);
    assert!(p.position[1] < 0.);
    assert_eq!(runtime.source_elapsed(first), Some(0.05));
    assert!((runtime.liquid().mass() - 0.05).abs() < 1e-12);
    let before = runtime.clone();
    scene
        .component_mut::<crate::BoxCollider>(second)
        .unwrap()
        .unwrap()
        .half_extents = [1., 1., 1.];
    assert!(runtime.tick(&scene, 0.001, None).is_err());
    assert_eq!(runtime, before);
}

#[test]
fn finite_translating_body_uses_scene_affine_template_and_recoils() {
    use physics::liquid::{DynamicWorldConfig, Particle, TranslatingBody};
    let mut scene = SceneGraph::new(1);
    let node = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(0.1, 0., 0.),
                rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            node,
            crate::BoxCollider {
                half_extents: [0.01, 2., 2.],
            },
        )
        .unwrap();
    let original = scene.local(node).unwrap();
    let geometry = SceneGeometry(crate::static_world(&scene).unwrap());
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.; 3],
            velocity: [2., 0., 0.],
            mass: 1.,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap();
    let mut body = TranslatingBody {
        position: [0.; 3],
        velocity: [0.; 3],
        mass: 3.,
    };
    let report = liquid
        .step_with_dynamic_geometry(0.05, &mut body, &geometry, DynamicWorldConfig::default())
        .unwrap();
    assert_eq!(report.contacts, 1);
    let p = liquid.particles()[0];
    assert!((p.velocity[0] - 1.25).abs() < 1e-6);
    assert!((p.velocity[1] + 0.75).abs() < 1e-6);
    assert!((body.velocity[0] - 0.25).abs() < 1e-6);
    assert!((body.velocity[1] - 0.25).abs() < 1e-6);
    for a in 0..3 {
        assert!((p.velocity[a] + 3. * body.velocity[a] - [2., 0., 0.][a]).abs() < 1e-12);
    }
    assert!((report.dissipated_energy - 0.75).abs() < 1e-6);
    assert!(body.position[0] > 0. && body.position[1] > 0.);
    assert_eq!(scene.local(node).unwrap(), original);
}

#[test]
fn authored_body_recoil_publishes_pose_and_failure_preserves_all_owners() {
    let (mut scene, first, second, _) = fixture(10);
    scene.remove_component::<LiquidSource>(second).unwrap();
    scene
        .component_mut::<LiquidSource>(first)
        .unwrap()
        .unwrap()
        .pulses[0]
        .duration_s = 0.05;
    scene
        .set_local(
            second,
            Transform {
                translation: glam::Vec3::new(0.1, 0., 0.),
                rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            second,
            crate::BoxCollider {
                half_extents: [0.01, 2., 2.],
            },
        )
        .unwrap();
    scene
        .insert_component(
            second,
            crate::LiquidBody {
                mass_kg: 3.,
                initial_velocity_m_s: [0.; 3],
            },
        )
        .unwrap();
    let initial = scene.local(second).unwrap();
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("water".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
        1,
    )
    .unwrap();
    let report = runtime.tick_and_publish(&mut scene, 0.05, None).unwrap();
    assert_eq!(report.dynamics.unwrap().dynamics.contacts, 1);
    let state = runtime.body_state().unwrap().1;
    assert!((state.velocity[0] - 0.25).abs() < 1e-6);
    assert!((state.velocity[1] - 0.25).abs() < 1e-6);
    let pose = scene.local(second).unwrap();
    assert!(pose.translation.x > initial.translation.x && pose.translation.y > 0.);
    assert_eq!(pose.rotation, initial.rotation);
    assert_eq!(pose.scale, initial.scale);
    runtime.validate_bindings(&scene).unwrap();
    let before = runtime.clone();
    let pose_before = scene.local(second).unwrap();
    scene
        .component_mut::<LiquidSource>(first)
        .unwrap()
        .unwrap()
        .particle_volume_m3 = 1e-10;
    assert!(runtime.tick_and_publish(&mut scene, 0.001, None).is_err());
    assert_eq!(runtime, before);
    assert_eq!(scene.local(second).unwrap(), pose_before);
}

#[test]
fn body_publication_overflow_and_initial_overlap_are_admitted_atomically() {
    let mut scene = SceneGraph::new(2);
    let body = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(body, crate::BoxCollider::default())
        .unwrap();
    scene
        .insert_component(
            body,
            crate::LiquidBody {
                mass_kg: 3.,
                initial_velocity_m_s: [1e100, 0., 0.],
            },
        )
        .unwrap();
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
        1,
    )
    .unwrap();
    let before = runtime.clone();
    let pose = scene.local(body).unwrap();
    assert!(runtime.tick_and_publish(&mut scene, 0.001, None).is_err());
    assert_eq!(runtime, before);
    assert_eq!(scene.local(body).unwrap(), pose);
    let wall = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(wall, crate::BoxCollider::default())
        .unwrap();
    assert!(
        SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config::default(),
            1
        )
        .is_err()
    );
}

#[test]
fn late_emission_and_physics_failures_roll_back_every_owner() {
    let (scene, _, _, mut runtime) = fixture(1);
    let before = runtime.clone();
    assert!(runtime.tick(&scene, 0.001, None).is_err());
    assert_eq!(runtime, before);
    let (scene, _, _, mut runtime) = fixture(10);
    let before = runtime.clone();
    assert!(
        runtime
            .tick(
                &scene,
                0.001,
                Some(Container {
                    min: [1.; 3],
                    max: [0.; 3],
                    restitution: 0.,
                    friction: 0.
                })
            )
            .is_err()
    );
    assert_eq!(runtime, before);
}

#[test]
fn foreign_scene_edits_unresolved_material_and_budget_are_rejected() {
    let (mut scene, first, _, mut runtime) = fixture(10);
    let before = runtime.clone();
    assert!(runtime.tick(&SceneGraph::new(2), 0.001, None).is_err());
    scene
        .component_mut::<LiquidSource>(first)
        .unwrap()
        .unwrap()
        .density_kg_m3 = 500.;
    assert!(runtime.tick(&scene, 0.001, None).is_err());
    assert_eq!(runtime, before);
    assert!(
        SceneLiquidRuntime::new(
            &scene,
            vec![("oil".into(), Material::OIL)],
            Config::default(),
            2
        )
        .is_err()
    );
    assert!(
        SceneLiquidRuntime::new(
            &scene,
            vec![("water".into(), Material::WATER)],
            Config::default(),
            1
        )
        .is_err()
    );
}
fn body_pair_fixture() -> (SceneGraph, Vec<NodeId>, SceneLiquidRuntime) {
    let mut scene = SceneGraph::new(8);
    let mut nodes = Vec::new();
    for (x, velocity) in [(-0.2, 3.), (0., 0.), (2., 7.)] {
        let node = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(x, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                node,
                crate::BoxCollider {
                    half_extents: [0.05; 3],
                },
            )
            .unwrap();
        scene
            .insert_component(
                node,
                crate::LiquidBody {
                    mass_kg: 1.,
                    initial_velocity_m_s: [velocity, 0., 0.],
                },
            )
            .unwrap();
        nodes.push(node);
    }
    scene.set_active(nodes[2], false).unwrap();
    let runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    (scene, nodes, runtime)
}

#[test]
fn multiple_scene_bodies_exchange_impulse_publish_and_pause_inactive_owner() {
    let (mut scene, nodes, mut runtime) = body_pair_fixture();
    let paused = scene.local(nodes[2]).unwrap();
    let report = runtime
        .tick_and_publish(&mut scene, 0.1, None)
        .unwrap()
        .dynamics
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    let states: Vec<_> = runtime.body_states().collect();
    assert_eq!(states.len(), 3);
    for (_, state) in &states[..2] {
        assert!((state.velocity[0] - 1.5).abs() < 1e-12);
    }
    assert!((report.dynamics.dissipated_energy - 2.25).abs() < 1e-12);
    assert_eq!(states[2].1.position, [2., 0., 0.]);
    assert_eq!(states[2].1.velocity, [7., 0., 0.]);
    assert_eq!(scene.local(nodes[2]).unwrap(), paused);
    for (node, state) in &states[..2] {
        assert_eq!(
            scene.local(*node).unwrap().translation,
            glam::DVec3::from_array(state.position).as_vec3()
        );
    }
    runtime.validate_bindings(&scene).unwrap();
    runtime.tick_and_publish(&mut scene, 0.01, None).unwrap();
}

#[test]
fn later_body_publication_failure_preserves_all_scene_poses() {
    let (mut scene, nodes, mut runtime) = body_pair_fixture();
    let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
    runtime.body[0].state.motion.position[0] += 1.;
    runtime.body[1].state.motion.position[0] = 1e100;
    let before = runtime.clone();
    assert!(runtime.publish_body_pose(&mut scene).is_err());
    assert_eq!(runtime, before);
    for (node, pose) in nodes.iter().zip(poses) {
        assert_eq!(scene.local(*node).unwrap(), pose);
    }
}
#[test]
fn child_only_compound_hits_another_body_and_shape_edits_reject_atomically() {
    let (mut scene, nodes, _) = body_pair_fixture();
    scene
        .remove_component::<crate::BoxCollider>(nodes[0])
        .unwrap();
    let child = scene
        .spawn(
            Some(nodes[0]),
            Transform {
                translation: glam::Vec3::new(0.1, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            child,
            crate::BoxCollider {
                half_extents: [0.02; 3],
            },
        )
        .unwrap();
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let report = runtime
        .tick_and_publish(&mut scene, 0.1, None)
        .unwrap()
        .dynamics
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    let states: Vec<_> = runtime.body_states().collect();
    assert!((states[0].1.velocity[0] - 1.5).abs() < 1e-12);
    assert!((states[1].1.velocity[0] - 1.5).abs() < 1e-12);
    assert_eq!(scene.local(child).unwrap().translation.x, 0.1);
    runtime.validate_bindings(&scene).unwrap();
    let before = runtime.clone();
    let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
    scene
        .set_local(
            child,
            Transform {
                translation: glam::Vec3::new(0.2, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(runtime.tick_and_publish(&mut scene, 0.01, None).is_err());
    assert_eq!(runtime, before);
    for (node, pose) in nodes.iter().zip(poses) {
        assert_eq!(scene.local(*node).unwrap(), pose);
    }
}

#[test]
fn compound_gap_is_not_filled_by_a_bounding_box() {
    let (mut scene, nodes, _) = body_pair_fixture();
    scene
        .remove_component::<crate::BoxCollider>(nodes[0])
        .unwrap();
    for y in [-1., 1.] {
        let child = scene
            .spawn(
                Some(nodes[0]),
                Transform {
                    translation: glam::Vec3::new(0., y, 0.),
                    rotation: glam::Quat::from_rotation_z(0.4),
                    scale: glam::Vec3::new(1., 2., 1.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                child,
                crate::BoxCollider {
                    half_extents: [0.05; 3],
                },
            )
            .unwrap();
    }
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let report = runtime
        .tick_and_publish(&mut scene, 0.1, None)
        .unwrap()
        .dynamics
        .unwrap();
    assert_eq!(report.dynamics.contacts, 0);
    let states: Vec<_> = runtime.body_states().collect();
    assert_eq!(states[0].1.velocity, [3., 0., 0.]);
    assert_eq!(states[1].1.velocity, [0.; 3]);
    assert!((states[0].1.position[0] - 0.1).abs() < 1e-7);
}
#[test]
fn compound_child_strikes_static_wall_and_balances_external_impulse() {
    let (mut scene, nodes, _) = body_pair_fixture();
    scene.set_active(nodes[1], false).unwrap();
    scene
        .remove_component::<crate::BoxCollider>(nodes[0])
        .unwrap();
    let child = scene
        .spawn(
            Some(nodes[0]),
            Transform {
                translation: glam::Vec3::new(0.1, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            child,
            crate::BoxCollider {
                half_extents: [0.02; 3],
            },
        )
        .unwrap();
    let wall = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(0.2, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            wall,
            crate::BoxCollider {
                half_extents: [0.02; 3],
            },
        )
        .unwrap();
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let report = runtime
        .tick_and_publish(&mut scene, 0.1, None)
        .unwrap()
        .dynamics
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    assert_eq!(runtime.body_state().unwrap().1.velocity, [0.; 3]);
    assert!((report.environment_impulse[0] - 3.).abs() < 1e-12);
    assert!((report.dynamics.dissipated_energy - 4.5).abs() < 1e-12);
    runtime.tick_and_publish(&mut scene, 0.01, None).unwrap();
}

#[test]
fn root_relative_compound_template_preserves_small_offsets_far_from_origin() {
    let (mut scene, nodes, _) = body_pair_fixture();
    scene
        .set_local(
            nodes[0],
            Transform {
                translation: glam::Vec3::new(1e5, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    let child = scene
        .spawn(
            Some(nodes[0]),
            Transform {
                translation: glam::Vec3::new(0.03, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            child,
            crate::BoxCollider {
                half_extents: [0.01; 3],
            },
        )
        .unwrap();
    let runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config::default(),
        0,
    )
    .unwrap();
    let body = &runtime.body[0];
    let collider = body.colliders.iter().find(|c| c.node == child).unwrap();
    let template = collider_template(body, collider).unwrap();
    assert_eq!(template.center.x, f64::from(0.03_f32));
}
#[test]
fn compound_candidates_are_bounded_and_invalid_extents_reject() {
    use physics::liquid::LiquidBodyWorld;
    let (mut scene, nodes, runtime) = body_pair_fixture();
    let template = collider_template(&runtime.body[0], &runtime.body[0].colliders[0]).unwrap();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![template; 2], vec![template; 2]],
    };
    assert_eq!(
        world.sweep_body_pair(
            0,
            &runtime.body_states().next().unwrap().1,
            1,
            &runtime.body_states().nth(1).unwrap().1,
            0.1,
            3
        ),
        Err(physics::liquid::Error::CollisionBudget)
    );
    assert_eq!(
        world.sweep_body_environment(0, &runtime.body_states().next().unwrap().1, 0.1, 1),
        Err(physics::liquid::Error::CollisionBudget)
    );
    scene
        .component_mut::<crate::BoxCollider>(nodes[0])
        .unwrap()
        .unwrap()
        .half_extents = [-0.05; 3];
    assert!(
        SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config::default(),
            0
        )
        .is_err()
    );
}
#[test]
fn liquid_impulse_recoils_child_only_compound_owner() {
    let mut scene = SceneGraph::new(2);
    let root = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            root,
            crate::LiquidBody {
                mass_kg: 1.,
                initial_velocity_m_s: [0.; 3],
            },
        )
        .unwrap();
    let child = scene
        .spawn(
            Some(root),
            Transform {
                translation: glam::Vec3::new(0.1, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            child,
            crate::BoxCollider {
                half_extents: [0.02; 3],
            },
        )
        .unwrap();
    let config = Config {
        gravity: [0.; 3],
        particle_radius: 0.01,
        ..Default::default()
    };
    let mut runtime =
        SceneLiquidRuntime::new(&scene, vec![("water".into(), Material::WATER)], config, 0)
            .unwrap();
    runtime.liquid = Liquid::new(
        vec![physics::liquid::Particle {
            position: [0.; 3],
            velocity: [3., 0., 0.],
            mass: 1.,
            material: 0,
        }],
        vec![Material::WATER],
        config,
    )
    .unwrap();
    let report = runtime
        .tick_and_publish(&mut scene, 0.05, None)
        .unwrap()
        .dynamics
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    let body = runtime.body_state().unwrap().1;
    let particle = &runtime.liquid.particles()[0];
    assert!((particle.velocity[0] - 1.5).abs() < 1e-12);
    assert!((body.velocity[0] - 1.5).abs() < 1e-12);
    assert!((report.dynamics.dissipated_energy - 2.25).abs() < 1e-12);
    assert_eq!(report.environment_impulse, [0.; 3]);
    assert_eq!(scene.local(child).unwrap().translation.x, 0.1);
}
#[test]
fn common_gravity_preserves_particle_body_relative_motion_across_sph_substeps() {
    let scene = SceneGraph::new(1);
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![rigid_shapes()[0]]],
    };
    let particle = physics::liquid::Particle {
        position: [0.; 3],
        velocity: [0.; 3],
        mass: 2.,
        material: 0,
    };
    let mut liquid = Liquid::new(
        vec![particle],
        vec![Material::WATER],
        Config {
            gravity: [0., -2., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let mut initial = rigid_body([10., 0., 0.], [0.; 3], 0.);
    initial.spin = None;
    initial.motion.mass = 3.;
    let mut bodies = [initial];
    let report = liquid
        .step_with_rigid_body_forces(
            0.01,
            &mut bodies,
            &world,
            Default::default(),
            1,
            rigid_config(),
            &[physics::contact::ContactWrench::default()],
        )
        .unwrap();
    assert!(report.world.dynamics.fluid.substeps > 1);
    assert_eq!(report.world.dynamics.contacts, 0);
    let p = liquid.particles()[0];
    for actual in [p.position[1], bodies[0].motion.position[1]] {
        assert!((actual + 0.0001).abs() < 1e-14);
    }
    for actual in [p.velocity[1], bodies[0].motion.velocity[1]] {
        assert!((actual + 0.02).abs() < 1e-14);
    }
    assert!((p.position[1] - bodies[0].motion.position[1]).abs() < 1e-14);
    assert!((report.external_work - 0.0006).abs() < 1e-14);
}

#[test]
fn common_acceleration_event_transports_the_real_world_witness_quadratically() {
    use physics::liquid::{GeometryHit, LiquidBodyWorld};
    let scene = SceneGraph::new(1);
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![rigid_shapes()[1]]],
    };
    let p = physics::liquid::Particle {
        position: [-0.1, 0., 0.],
        velocity: [3., 0., 0.],
        mass: 1.,
        material: 0,
    };
    let particle = physics::contact::ContactBody {
        motion: physics::gravity::Body {
            mass: p.mass,
            position: p.position,
            velocity: p.velocity,
        },
        spin: None,
    }
    .prepare_motion([0., -2., 0.], [0.; 3], 0.04, rigid_config())
    .unwrap();
    let mut initial = rigid_body([0.; 3], [0.; 3], 0.);
    initial.spin = None;
    initial.motion.mass = 3.;
    let body = initial
        .prepare_motion([0., -6., 0.], [0.; 3], 0.04, rigid_config())
        .unwrap();
    let event = world
        .sweep_particle_motion_body_event(&p, 0.01, &particle, 0, &body, 1)
        .unwrap();
    let GeometryHit::Contact { fraction, normal } = event.contact.geometry else {
        panic!("contact required")
    };
    let time = fraction * 0.04;
    assert!((time - 0.03).abs() < 1e-14);
    assert_eq!(normal, [-1., 0., 0.]);
    assert!((event.contact.witness.unwrap().point[1] + time * time).abs() < 1e-14);
}

#[test]
fn unresolved_resting_gravity_support_preserves_complete_public_state() {
    let mut scene = SceneGraph::new(2);
    let wall = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(0.03125, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            wall,
            crate::BoxCollider {
                half_extents: [0.03125, 1., 0.03125],
            },
        )
        .unwrap();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![crate::convex::AffineBox {
            center: glam::DVec3::ZERO,
            edges: [glam::DVec3::X, glam::DVec3::Y, glam::DVec3::Z].map(|e| e * 0.03125),
        }]],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [20., 0., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let initial_fluid = liquid.clone();
    let mut bodies = [rigid_body([-0.03125, 0., 0.], [0.; 3], 0.)];
    let before = bodies;
    assert_eq!(
        liquid.step_with_rigid_body_forces(
            0.01,
            &mut bodies,
            &world,
            Default::default(),
            1,
            rigid_config(),
            &[physics::contact::ContactWrench::default()]
        ),
        Err(physics::liquid::Error::CollisionBudget)
    );
    assert_eq!(before, bodies);
    assert_eq!(liquid.particles(), initial_fluid.particles());
}

#[test]
fn accelerated_nonspinning_body_uses_real_parabolic_wall_query_and_remainder() {
    let mut scene = SceneGraph::new(2);
    let wall = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(0.04, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            wall,
            crate::BoxCollider {
                half_extents: [0.04, 2., 0.02],
            },
        )
        .unwrap();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![rigid_shapes()[0]]],
    };
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [20., 0., 0.],
            ..Default::default()
        },
    )
    .unwrap();
    let mut body = rigid_body([-0.1, 0., 0.], [0.; 3], 0.);
    body.spin = None;
    let initial = body.energy().unwrap();
    // Keep the real publication array so endpoint comparisons prove the
    // actual geometry/core path rather than a separate prepared reference.
    let mut bodies = [body];
    let report = liquid
        .step_with_rigid_body_forces(
            0.1,
            &mut bodies,
            &world,
            elastic_rigid(),
            1,
            rigid_config(),
            &[physics::contact::ContactWrench::default()],
        )
        .unwrap();
    let impact = (2. * 0.06 / 20_f64).sqrt();
    let remainder = 0.1 - impact;
    let expected_x = -0.04 - 20. * impact * remainder + 10. * remainder * remainder;
    let expected_v = -20. * impact + 20. * remainder;
    assert_eq!(report.world.dynamics.contacts, 1);
    assert!((bodies[0].motion.position[0] - expected_x).abs() < 1e-10);
    assert!((bodies[0].motion.velocity[0] - expected_v).abs() < 1e-10);
    assert!((report.world.environment_impulse[0] - 40. * impact).abs() < 1e-10);
    assert_eq!(report.world.dynamics.dissipated_energy, 0.);
    assert!(
        (bodies[0].energy().unwrap()
            - initial
            - report.external_work
            - report.integration_energy_residual)
            .abs()
            < 1e-10
    );
}

#[test]
fn real_simultaneous_rigid_faces_resolve_as_one_network() {
    let scene = SceneGraph::new(1);
    let shape = rigid_shapes()[0];
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![shape]; 3],
    };
    let original = [
        rigid_body([-0.1, 0., 0.], [3., 0., 0.], 0.),
        rigid_body([0.; 3], [0.; 3], 0.),
        rigid_body([0.1, 0., 0.], [-3., 0., 0.], 0.),
    ];
    for order in [[0, 1, 2], [2, 0, 1]] {
        let mut states = order.map(|i| original[i]);
        let mut liquid = rigid_liquid(Vec::new());
        {
            use physics::liquid::{GeometryHit, LiquidBodyWorld};
            let paths: Vec<_> = states
                .iter()
                .map(|b| {
                    b.prepare_motion([0.; 3], [0.; 3], 0.02, rigid_config())
                        .unwrap()
                })
                .collect();
            let mut fractions = Vec::new();
            for i in 0..3 {
                for j in i + 1..3 {
                    let event = world
                        .sweep_rigid_pair_event(i, &paths[i], j, &paths[j], 4096)
                        .unwrap();
                    assert!(event.feature.is_some() && event.contact.witness.is_some());
                    let GeometryHit::Contact { fraction, .. } = event.contact.geometry else {
                        panic!("initial approaching pair required")
                    };
                    fractions.push(fraction);
                }
            }
            fractions.sort_by(f64::total_cmp);
            assert_eq!(fractions[0], fractions[1]);
            assert!(fractions[2] > fractions[0]);
        }
        let report = liquid
            .step_with_rigid_body_world(
                0.02,
                &mut states,
                &world,
                Default::default(),
                3,
                rigid_config(),
            )
            .unwrap();
        assert_eq!(report.dynamics.contacts, 2);
        assert!((report.dynamics.dissipated_energy - 9.).abs() < 1e-11);
        assert_eq!(report.environment_impulse, [0.; 3]);
        for (body, i) in states.iter().zip(order) {
            assert!(body.motion.velocity.iter().all(|v| v.abs() < 1e-12));
            assert!(
                body.spin
                    .unwrap()
                    .angular_momentum
                    .iter()
                    .all(|v| v.abs() < 1e-12)
            );
            assert!((body.motion.position[0] - [-0.08, 0., 0.08][i]).abs() < 1e-14);
        }
    }
}

#[test]
fn compound_event_key_preserves_exact_shape_pair_and_support_source() {
    use physics::liquid::LiquidBodyWorld;
    let scene = SceneGraph::new(1);
    let shapes = rigid_shapes();
    let first_far = crate::convex::AffineBox {
        center: glam::DVec3::new(5., 10., 0.),
        ..shapes[0]
    };
    let second_far = crate::convex::AffineBox {
        center: glam::DVec3::new(10., -10., 0.),
        ..shapes[1]
    };
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![first_far, shapes[0]], vec![second_far, shapes[1]]],
    };
    let a = rigid_body([-0.1, 1., 0.], [3., 0., 0.], 0.)
        .prepare_motion([0.; 3], [0.; 3], 0.03, rigid_config())
        .unwrap();
    let b = rigid_body([0.; 3], [0.; 3], 0.)
        .prepare_motion([0.; 3], [0.; 3], 0.03, rigid_config())
        .unwrap();
    let event = world.sweep_rigid_pair_event(0, &a, 1, &b, 16).unwrap();
    assert_eq!(
        event.contact,
        world.sweep_rigid_pair_contact(0, &a, 1, &b, 16).unwrap()
    );
    let key = FeatureKey::decode(event.feature.unwrap()).unwrap();
    assert_eq!(
        (key.first, key.second, key.axis),
        (1, 1, crate::convex::AxisFeature::BodyFace(0))
    );
    let physics::liquid::GeometryHit::Contact { fraction, normal } = event.contact.geometry else {
        panic!("contact required")
    };
    let mut sa = a.sample(fraction * a.duration()).unwrap();
    let mut sb = b.sample(fraction * b.duration()).unwrap();
    sa.spin.as_mut().unwrap().angular_momentum = [0., 0., 0.25];
    sb.spin.as_mut().unwrap().angular_momentum = [0., 0., -0.6];
    let points = world
        .rigid_pair_supports(
            0,
            &sa,
            1,
            &sb,
            event.contact.witness.unwrap(),
            normal,
            event.feature.unwrap(),
            1,
        )
        .unwrap();
    assert_eq!(points.len(), 4);
    assert!(
        points
            .iter()
            .all(|s| s.plane == physics::contact::SupportPlane::First)
    );
    let bad = FeatureKey { first: 99, ..key }.encode().unwrap();
    assert_eq!(
        world.rigid_pair_supports(
            0,
            &sa,
            1,
            &sb,
            event.contact.witness.unwrap(),
            normal,
            bad,
            1
        ),
        Err(physics::liquid::Error::InvalidCollision)
    );
    assert_eq!(
        world.rigid_pair_supports(
            0,
            &sa,
            1,
            &sb,
            event.contact.witness.unwrap(),
            normal,
            255,
            1
        ),
        Err(physics::liquid::Error::InvalidCollision)
    );
    assert_eq!(
        world.rigid_pair_supports(
            0,
            &sa,
            1,
            &sb,
            event.contact.witness.unwrap(),
            normal,
            event.feature.unwrap(),
            0
        ),
        Err(physics::liquid::Error::CollisionBudget)
    );
}

#[test]
fn edge_event_source_rebuilds_normal_rate_from_post_impact_spin() {
    use glam::{DQuat, DVec3};
    use physics::liquid::LiquidBodyWorld;
    let h = 0.02;
    let (s, c) = 0.35_f64.sin_cos();
    let (t, d) = 0.45_f64.sin_cos();
    let first_shape = crate::convex::AffineBox {
        center: DVec3::ZERO,
        edges: [
            DVec3::Y,
            DVec3::new(c, 0., -s) * h,
            DVec3::new(s, 0., c) * h,
        ],
    };
    let second_shape = crate::convex::AffineBox {
        center: DVec3::ZERO,
        edges: [
            DVec3::Z,
            DVec3::new(d, t, 0.) * h,
            DVec3::new(-t, d, 0.) * h,
        ],
    };
    let separation = first_shape.radius(DVec3::X) + second_shape.radius(DVec3::X);
    let scene = SceneGraph::new(1);
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![first_shape], vec![second_shape]],
    };
    let a = rigid_body([-separation - 0.03, 0., 0.], [3., 0., 0.], 0.4)
        .prepare_motion([0.; 3], [0.; 3], 0.02, rigid_config())
        .unwrap();
    let mut initial_b = rigid_body([0.; 3], [0.; 3], 0.);
    initial_b.spin.as_mut().unwrap().angular_momentum = [-0.6, 0., 0.];
    let b = initial_b
        .prepare_motion([0.; 3], [0.; 3], 0.02, rigid_config())
        .unwrap();
    let event = world.sweep_rigid_pair_event(0, &a, 1, &b, 64).unwrap();
    let key = FeatureKey::decode(event.feature.unwrap()).unwrap();
    assert_eq!(key.axis, crate::convex::AxisFeature::Edges(0, 0));
    let physics::liquid::GeometryHit::Contact { fraction, normal } = event.contact.geometry else {
        panic!("contact required")
    };
    let mut sa = a.sample(fraction * a.duration()).unwrap();
    let mut sb = b.sample(fraction * b.duration()).unwrap();
    let previous = world
        .rigid_pair_supports(
            0,
            &sa,
            1,
            &sb,
            event.contact.witness.unwrap(),
            normal,
            event.feature.unwrap(),
            1,
        )
        .unwrap();
    assert_eq!(previous.len(), 1);
    sa.spin.as_mut().unwrap().angular_momentum = [0., 0., 0.8];
    sb.spin.as_mut().unwrap().angular_momentum = [-0.2, 0., 0.];
    let updated = world
        .rigid_pair_supports(
            0,
            &sa,
            1,
            &sb,
            event.contact.witness.unwrap(),
            normal,
            event.feature.unwrap(),
            1,
        )
        .unwrap();
    let physics::contact::SupportPlane::Rate { normal_rate } = updated[0].plane else {
        panic!("edge normal rate required")
    };
    assert_ne!(updated[0].plane, previous[0].plane);
    let ea = DQuat::from_array(sa.spin.unwrap().orientation) * first_shape.edges[0];
    let eb = DQuat::from_array(sb.spin.unwrap().orientation) * second_shape.edges[0];
    let wa = DVec3::from_array(sa.spin.unwrap().angular_velocity().unwrap());
    let wb = DVec3::from_array(sb.spin.unwrap().angular_velocity().unwrap());
    let sign = if ea.cross(eb).dot(DVec3::from_array(normal)) < 0. {
        -1.
    } else {
        1.
    };
    let rotate = |edge: DVec3, omega: DVec3, time: f64| {
        let speed = omega.length();
        let axis = omega / speed;
        let (s, c) = (speed * time).sin_cos();
        edge * c + axis.cross(edge) * s + axis * axis.dot(edge) * (1. - c)
    };
    let n = |time| rotate(ea, wa, time).cross(rotate(eb, wb, time)).normalize() * sign;
    let delta = 1e-5;
    let numeric = (n(delta) - n(-delta)) / (2. * delta);
    assert!((numeric - DVec3::from_array(normal_rate)).length() < 1e-8);
    assert!(
        DVec3::from_array(normal)
            .dot(DVec3::from_array(normal_rate))
            .abs()
            < 1e-12
    );
}

#[test]
fn environment_event_keeps_wall_identity_and_world_plane() {
    use glam::{DQuat, DVec3};
    use physics::liquid::LiquidBodyWorld;
    let mut scene = SceneGraph::new(3);
    for x in [10., 0.04] {
        let wall = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::new(x, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(
                wall,
                crate::BoxCollider {
                    half_extents: [0.04, 2., 0.02],
                },
            )
            .unwrap();
    }
    let q = DQuat::from_rotation_z(0.4);
    let mut shape = rigid_shapes()[0];
    shape.edges = shape.edges.map(|e| q * e);
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![shape]],
    };
    let expected_wall = world
        .environment
        .0
        .0
        .iter()
        .position(|w| w.shape.center.x < 1.)
        .unwrap();
    let path = rigid_body([-0.1, 0., 0.], [3., 0., 0.], 0.)
        .prepare_motion([0.; 3], [0.; 3], 0.03, rigid_config())
        .unwrap();
    let event = world.sweep_rigid_environment_event(0, &path, 64).unwrap();
    let token = event.feature.unwrap();
    let key = FeatureKey::decode(token).unwrap();
    assert_eq!(key.second, expected_wall);
    assert_eq!(key.axis, crate::convex::AxisFeature::ObstacleFace(0));
    let physics::liquid::GeometryHit::Contact { fraction, normal } = event.contact.geometry else {
        panic!("contact required")
    };
    let mut body = path.sample(fraction * path.duration()).unwrap();
    body.spin.as_mut().unwrap().angular_momentum = [0., 0., 0.8];
    let supports = world
        .rigid_environment_supports(0, &body, event.contact.witness.unwrap(), normal, token, 1)
        .unwrap();
    assert!(!supports.is_empty());
    assert!(
        supports
            .iter()
            .all(|s| s.plane == physics::contact::SupportPlane::World)
    );
    let bad = FeatureKey { second: 99, ..key }.encode().unwrap();
    assert_eq!(
        world.rigid_environment_supports(0, &body, event.contact.witness.unwrap(), normal, bad, 1),
        Err(physics::liquid::Error::InvalidCollision)
    );
    assert_eq!(
        world.rigid_environment_supports(
            0,
            &body,
            event.contact.witness.unwrap(),
            normal,
            token,
            0
        ),
        Err(physics::liquid::Error::CollisionBudget)
    );
    // A fixed wall's normal must not inherit the moving body's angular rate.
    let gap = physics::contact::normal_gap_acceleration(
        &body,
        None,
        supports[0],
        physics::contact::ContactWrench {
            force: [0.; 3],
            torque: [0.; 3],
        },
        None,
    )
    .unwrap();
    assert!(gap.is_finite());
    assert!(DVec3::from_array(normal).is_finite());
    // Run actual scene feature admission through the physical impact and
    // reaction solvers. Rebuild supports after impact changes angular momentum.
    let points: Vec<_> = supports.iter().map(|s| s.contact).collect();
    physics::contact::resolve_normal_manifold(
        &mut body,
        None,
        &points,
        physics::contact::ManifoldConfig {
            max_sweeps: 100,
            velocity_tolerance: 1e-11,
        },
    )
    .unwrap();
    let post = world
        .rigid_environment_supports(0, &body, event.contact.witness.unwrap(), normal, token, 1)
        .unwrap();
    let external = physics::contact::ContactWrench {
        force: [3., 0., 0.],
        torque: [0.; 3],
    };
    let snapshot = body;
    let reaction = physics::contact::resolve_normal_reactions(
        &body,
        None,
        &post,
        external,
        None,
        physics::contact::ReactionConfig {
            max_sweeps: 100,
            acceleration_tolerance: 1e-10,
            normal_velocity_tolerance: 1e-10,
        },
    )
    .unwrap();
    assert_eq!(snapshot, body);
    assert!(reaction.first_wrench.force[0] < 0.);
    assert!(reaction.second_wrench.is_none());
    assert!(reaction.normal_accelerations.iter().all(|g| *g >= -1e-10));
    assert!(reaction.acceleration_residual <= 1e-10);
    assert!(reaction.instantaneous_power.abs() < 1e-9);
    // Independently evaluate the material-point acceleration for the spherical
    // inertia fixture. The wall normal has no Coriolis contribution.
    let omega = DVec3::from_array(body.spin.unwrap().angular_velocity().unwrap());
    let alpha = DVec3::from_array(reaction.first_wrench.torque);
    let acceleration = (DVec3::from_array(external.force)
        + DVec3::from_array(reaction.first_wrench.force))
        / body.motion.mass;
    let com = DVec3::from_array(body.motion.position);
    let n = DVec3::from_array(normal);
    for (support, actual) in post.iter().zip(&reaction.normal_accelerations) {
        let r = DVec3::from_array(support.contact.point) - com;
        let expected = n.dot(acceleration + alpha.cross(r) + omega.cross(omega.cross(r)));
        assert!((expected - actual).abs() < 1e-11);
    }
}

#[test]
fn authored_wall_patch_stops_a_rigid_body_and_closes_boundary_ledger() {
    let mut scene = SceneGraph::new(2);
    let wall = scene
        .spawn(
            None,
            Transform {
                translation: glam::Vec3::new(0.04, 0., 0.),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            wall,
            crate::BoxCollider {
                half_extents: [0.04, 2., 0.02],
            },
        )
        .unwrap();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: vec![vec![rigid_shapes()[0]]],
    };
    let mut liquid = rigid_liquid(Vec::new());
    let mut bodies = [rigid_body([-0.1, 0., 0.], [3., 0., 0.], 0.)];
    let energy = bodies[0].energy().unwrap();
    let report = liquid
        .step_with_rigid_body_world(
            0.03,
            &mut bodies,
            &world,
            Default::default(),
            1,
            rigid_config(),
        )
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    assert!(bodies[0].motion.velocity.iter().all(|v| v.abs() < 1e-11));
    assert!(
        bodies[0]
            .spin
            .unwrap()
            .angular_momentum
            .iter()
            .all(|v| v.abs() < 1e-11)
    );
    assert!((report.environment_impulse[0] - 3.).abs() < 1e-11);
    assert!(
        (bodies[0].energy().unwrap() + report.dynamics.dissipated_energy - energy).abs() < 1e-11
    );
    let before = (liquid.clone(), bodies);
    let next = liquid
        .step_with_rigid_body_world(
            0.001,
            &mut bodies,
            &world,
            Default::default(),
            1,
            rigid_config(),
        )
        .unwrap();
    assert_eq!(next.dynamics.contacts, 0);
    assert_eq!(bodies[0].motion.velocity, before.1[0].motion.velocity);
    assert_eq!(
        bodies[0].spin.unwrap().angular_momentum,
        before.1[0].spin.unwrap().angular_momentum
    );
    assert!(
        (glam::DVec3::from_array(bodies[0].motion.position)
            - glam::DVec3::from_array(before.1[0].motion.position))
        .length()
            < 1e-14
    );
    assert!(
        bodies[0].spin.unwrap().orientation[..3]
            .iter()
            .all(|v| v.abs() < 1e-15)
    );
}

#[test]
fn scene_patch_solves_contact_velocities_but_free_rotation_requires_sustained_constraint() {
    use physics::liquid::LiquidBodyWorld;
    let scene = SceneGraph::new(1);
    let shapes = rigid_shapes();
    let world = SceneBodyWorld {
        environment: SceneGeometry(crate::static_world(&scene).unwrap()),
        templates: shapes.map(|s| vec![s]).to_vec(),
    };
    let mut a = rigid_body([-0.04, 1., 0.], [3., 0., 0.], 0.);
    let mut b = rigid_body([0.; 3], [0.; 3], 0.);
    let contacts = world
        .rigid_pair_patch(
            0,
            &a,
            1,
            &b,
            ContactWitness {
                point: [0., 1., 0.],
                tolerance_m: 1e-13,
            },
            [-1., 0., 0.],
            4,
        )
        .unwrap();
    assert_eq!(contacts.len(), 4);
    let report = physics::contact::resolve_normal_manifold(
        &mut a,
        Some(&mut b),
        &contacts,
        physics::contact::ManifoldConfig {
            max_sweeps: 100,
            velocity_tolerance: 1e-11,
        },
    )
    .unwrap();
    assert!(report.velocity_residual <= 1e-11);
    for c in contacts {
        assert!(
            -(a.point_velocity(c.point).unwrap()[0] - b.point_velocity(c.point).unwrap()[0])
                >= -1e-11
        );
    }
    let dt = 0.001;
    let a = a
        .prepare_motion([0.; 3], [0.; 3], dt, rigid_config())
        .unwrap()
        .end();
    let b = b
        .prepare_motion([0.; 3], [0.; 3], dt, rigid_config())
        .unwrap()
        .end();
    let qa = glam::DQuat::from_array(a.spin.unwrap().orientation);
    let qb = glam::DQuat::from_array(b.spin.unwrap().orientation);
    let obstacle = crate::convex::AffineBox {
        center: glam::DVec3::from_array(b.motion.position) + qb * shapes[1].center,
        edges: shapes[1].edges.map(|e| qb * e),
    };
    assert!(
        obstacle
            .penetration_affine(
                glam::DVec3::from_array(a.motion.position) + qa * shapes[0].center,
                shapes[0].edges.map(|e| qa * e)
            )
            .is_some(),
        "a zero normal velocity at impact does not imply a clear free-rotation remainder"
    );
    assert_eq!(
        world.rigid_pair_patch(
            0,
            &a,
            1,
            &b,
            ContactWitness {
                point: [0., 1., 0.],
                tolerance_m: 1e-13
            },
            [-1., 0., 0.],
            0
        ),
        Err(physics::liquid::Error::CollisionBudget)
    );
}

#[test]
fn authored_rigid_owners_rotate_publish_persist_pause_and_rollback() {
    let mut scene = SceneGraph::new(8);
    let mut nodes = Vec::new();
    for (position, velocity, center, half) in [
        ([-0.1, 1., 0.], [3., 0., 0.], 0., [0.04, 0.02, 0.02]),
        ([0.; 3], [0.; 3], 0.04, [0.04, 2., 0.02]),
    ] {
        let root = scene
            .spawn(
                None,
                Transform {
                    translation: glam::Vec3::from_array(position),
                    ..Default::default()
                },
            )
            .unwrap();
        let child = scene
            .spawn(
                Some(root),
                Transform {
                    translation: glam::Vec3::new(center, 0., 0.),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .insert_component(child, crate::BoxCollider { half_extents: half })
            .unwrap();
        scene
            .insert_component(
                root,
                crate::LiquidBody {
                    mass_kg: 1.,
                    initial_velocity_m_s: velocity,
                },
            )
            .unwrap();
        // Mass distribution is explicit and independent of collision proxies.
        let h = 1.5_f64.sqrt();
        scene
            .insert_component(
                root,
                crate::LiquidMassDistribution {
                    parts: vec![crate::LiquidMassPart {
                        mass_kg: 1.,
                        center_m: [0.; 3],
                        half_edges_m: [[h, 0., 0.], [0., h, 0.], [0., 0., h]],
                    }],
                },
            )
            .unwrap();
        nodes.push(root);
    }
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let energy: f64 = runtime
        .body_rigid_states()
        .map(|(_, s)| s.energy().unwrap())
        .sum();
    let before = runtime.clone();
    let poses_before: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
    assert!(
        runtime
            .tick_and_publish(&mut scene, 0.03, None)
            .unwrap_err()
            .to_string()
            .contains("CollisionBudget")
    );
    assert_eq!(runtime, before);
    assert_eq!(
        nodes
            .iter()
            .map(|n| scene.local(*n).unwrap())
            .collect::<Vec<_>>(),
        poses_before
    );
    let report = runtime
        .tick_and_publish_with_dynamics(&mut scene, 0.03, None, elastic_rigid(), rigid_config())
        .unwrap()
        .dynamics
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    let states: Vec<_> = runtime.body_rigid_states().collect();
    assert!((states[0].1.motion.velocity[0] - 1.).abs() < 1e-7);
    assert!((states[1].1.motion.velocity[0] - 2.).abs() < 1e-7);
    assert!((states[1].1.spin.unwrap().angular_momentum[2] + 2.).abs() < 1e-7);
    let after: f64 = states.iter().map(|(_, s)| s.energy().unwrap()).sum();
    assert!((after + report.dynamics.dissipated_energy - energy).abs() < 1e-11);
    // Published child geometry follows the persistent principal template,
    // within the same admitted root-pose point error used by publication.
    for owner in &runtime.body {
        let spin = owner.state.spin.unwrap();
        let q = glam::DQuat::from_array(spin.orientation);
        let com = glam::DVec3::from_array(owner.state.motion.position);
        for (collider, shape) in owner
            .colliders
            .iter()
            .zip(owner.principal_templates.as_ref().unwrap())
        {
            let actual =
                crate::affine_box(&scene, collider.node, collider.collider.half_extents).unwrap();
            assert!((actual.center - (com + q * shape.center)).length() < 1e-5);
            for k in 0..3 {
                assert!((actual.edges[k] - q * shape.edges[k]).length() < 1e-5);
            }
        }
    }
    let rotation = scene.local(nodes[1]).unwrap().rotation;
    assert!(rotation.z.abs() > 0.001);
    runtime.validate_bindings(&scene).unwrap();
    runtime.tick_and_publish(&mut scene, 0.001, None).unwrap();
    assert_ne!(scene.local(nodes[1]).unwrap().rotation, rotation);
    for node in &nodes {
        scene.set_active(*node, false).unwrap();
    }
    let states: Vec<_> = runtime.body_rigid_states().collect();
    runtime.tick_and_publish(&mut scene, 0.001, None).unwrap();
    assert_eq!(runtime.body_rigid_states().collect::<Vec<_>>(), states);
    let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
    runtime.body[0].state.motion.position[0] += 1.;
    runtime.body[1].state.motion.position[0] = 1e100;
    let before = runtime.clone();
    assert!(runtime.publish_body_pose(&mut scene).is_err());
    assert_eq!(runtime, before);
    assert_eq!(
        nodes
            .iter()
            .map(|n| scene.local(*n).unwrap())
            .collect::<Vec<_>>(),
        poses
    );
}

#[test]
fn explicit_mass_distribution_admits_scaled_tensor_and_edits_rollback() {
    let (mut scene, nodes, _) = body_pair_fixture();
    let distribution = crate::LiquidMassDistribution {
        parts: vec![crate::LiquidMassPart {
            mass_kg: 1.,
            center_m: [0.25, 0., 0.],
            half_edges_m: [[0.1, 0., 0.], [0., 0.2, 0.], [0., 0., 0.3]],
        }],
    };
    scene
        .insert_component(nodes[0], distribution.clone())
        .unwrap();
    let mut pose = scene.local(nodes[0]).unwrap();
    pose.scale = glam::Vec3::new(2., 1., 1.);
    scene.set_local(nodes[0], pose).unwrap();
    // Duplicate collision proxies do not duplicate explicitly authored mass.
    let duplicate = scene
        .spawn(Some(nodes[0]), voxy_scene::Transform::default())
        .unwrap();
    scene
        .insert_component(duplicate, crate::BoxCollider::default())
        .unwrap();
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    let properties = runtime.body_mass_properties().next().unwrap().1.unwrap();
    assert_eq!(properties.center, [0.5, 0., 0.]);
    assert!((properties.inertia[0][0] - (0.04 + 0.09) / 3.).abs() < 1e-12);
    assert!((properties.inertia[1][1] - (0.04 + 0.09) / 3.).abs() < 1e-12);
    assert!(runtime.body_mass_properties().nth(1).unwrap().1.is_none());
    runtime.tick_and_publish(&mut scene, 0.001, None).unwrap();
    let before = runtime.clone();
    let poses: Vec<_> = nodes.iter().map(|n| scene.local(*n).unwrap()).collect();
    scene
        .component_mut::<crate::LiquidMassDistribution>(nodes[0])
        .unwrap()
        .unwrap()
        .parts[0]
        .mass_kg = 2.;
    assert!(runtime.tick_and_publish(&mut scene, 0.001, None).is_err());
    assert_eq!(runtime, before);
    for (node, pose) in nodes.iter().zip(poses) {
        assert_eq!(scene.local(*node).unwrap(), pose);
    }
    assert!(
        SceneLiquidRuntime::new(
            &scene,
            vec![("unused".into(), Material::WATER)],
            Config::default(),
            0
        )
        .is_err()
    );
    let orphan = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
    scene.insert_component(orphan, distribution).unwrap();
    let error = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config::default(),
        0,
    )
    .unwrap_err();
    assert!(error.to_string().contains("requires a liquid body owner"));
}
#[test]
fn authored_point_loads_tick_publish_pause_and_rollback_with_original_phase() {
    use voxy_scene::Transform;
    let mut scene = SceneGraph::new(4);
    let node = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            node,
            crate::BoxCollider {
                half_extents: [0.1; 3],
            },
        )
        .unwrap();
    scene
        .insert_component(
            node,
            crate::LiquidBody {
                mass_kg: 1.,
                initial_velocity_m_s: [0.; 3],
            },
        )
        .unwrap();
    let h = 1.5_f64.sqrt();
    scene
        .insert_component(
            node,
            crate::LiquidMassDistribution {
                parts: vec![crate::LiquidMassPart {
                    mass_kg: 1.,
                    center_m: [0.; 3],
                    half_edges_m: [[h, 0., 0.], [0., h, 0.], [0., 0., h]],
                }],
            },
        )
        .unwrap();
    let loads = crate::RigidPointLoads {
        points: vec![crate::RigidPointForce {
            root_point_m: [1., 0., 0.],
            world_force_n: [0., 0.3, 0.],
            world_force_rate_n_s: [0., 0.2, 0.],
        }],
    };
    scene.insert_component(node, loads.clone()).unwrap();
    let mut runtime = SceneLiquidRuntime::new(
        &scene,
        vec![("unused".into(), Material::WATER)],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
        0,
    )
    .unwrap();
    crate::validate_game_descriptors_with_liquid_runtime(&scene, 4, &runtime).unwrap();
    for _ in 0..2 {
        let owner = &runtime.body[0];
        let frame = owner.rigid_frame.unwrap();
        let expected = owner
            .state
            .prepare_material_load_motion(
                loads.prepare(frame, owner.point_elapsed_s, 128).unwrap(),
                [],
                0.02,
                rigid_config(),
            )
            .unwrap()
            .end();
        runtime
            .tick_and_publish_with_dynamics(&mut scene, 0.02, None, elastic_rigid(), rigid_config())
            .unwrap();
        assert_eq!(runtime.body[0].state, expected);
    }
    assert!((runtime.body[0].point_elapsed_s - 0.04).abs() < 1e-14);
    scene.set_active(node, false).unwrap();
    runtime
        .tick_and_publish_with_dynamics(&mut scene, 0.02, None, elastic_rigid(), rigid_config())
        .unwrap();
    assert!((runtime.body[0].point_elapsed_s - 0.04).abs() < 1e-14);
    scene.set_active(node, true).unwrap();
    let before_failure = runtime.clone();
    let pose_before = scene.local(node).unwrap();
    assert!(
        runtime
            .tick_and_publish_with_dynamics(
                &mut scene,
                0.02,
                None,
                elastic_rigid(),
                physics::spin_path::Config {
                    max_arcs: 1,
                    max_trials: 1,
                    max_angular_error_rad: 1e-15,
                    ..rigid_config()
                }
            )
            .is_err()
    );
    assert_eq!(runtime, before_failure);
    assert_eq!(scene.local(node).unwrap(), pose_before);
    let saved = runtime.clone();
    scene
        .component_mut::<crate::RigidPointLoads>(node)
        .unwrap()
        .unwrap()
        .points[0]
        .world_force_n[0] = 1.;
    assert!(
        runtime
            .tick_and_publish_with_dynamics(&mut scene, 0.02, None, elastic_rigid(), rigid_config())
            .unwrap_err()
            .to_string()
            .contains("rebind")
    );
    assert_eq!(runtime, saved);
}
