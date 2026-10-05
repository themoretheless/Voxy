use physics::biomechanics::{
    Body, InertialBody, Material, MaxwellBranch, OgdenTerm, PlaneContact, SupportTarget,
    ViscoelasticOgden,
};

fn tetra(pins: bool, velocity_y: f64, maxwell: bool) -> InertialBody {
    let points = vec![
        [0., 0.01, 0.],
        [0.1, 0.01, 0.],
        [0., 0.11, 0.],
        [0., 0.01, 0.1],
    ];
    let mut body = Body::new(
        points,
        vec![pins; 4],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(1e5, 0.3).unwrap(),
        )],
    )
    .unwrap();
    let mut dynamics = if maxwell {
        body.set_viscoelastic_ogden_batch(&[(
            0,
            ViscoelasticOgden::new(
                vec![OgdenTerm {
                    shear_pa: 5000.,
                    exponent: 2.,
                }],
                1e6,
                vec![MaxwellBranch {
                    shear_pa: 10000.,
                    relaxation_seconds: 0.2,
                }],
            )
            .unwrap(),
        )])
        .unwrap();
        InertialBody::new_viscoelastic_with_supports(body, &[1000.], vec![[0., velocity_y, 0.]; 4])
            .unwrap()
    } else if pins {
        InertialBody::new_with_fixed_supports(body, &[1000.], vec![[0.; 3]; 4]).unwrap()
    } else {
        InertialBody::new(body, &[1000.], vec![[0., velocity_y, 0.]; 4]).unwrap()
    };
    dynamics
        .set_plane_contact(Some(PlaneContact::new([0., 1., 0.], 0., 20000.).unwrap()))
        .unwrap();
    dynamics
}

#[test]
fn stationary_nodes_receive_independently_derived_plane_work() {
    let mut body = tetra(true, 0., false);
    let untouched = format!("{body:?}");
    // A coarse segment crosses contact activation. Its independently computed
    // trapezoidal work must fail a strict budget, rather than being replaced by
    // the measured potential-energy change.
    assert!(
        body.step_with_moving_plane(None, 0.012, 0.1, 1e-10)
            .is_err()
    );
    assert_eq!(format!("{body:?}"), untouched);
    body.set_plane_contact(Some(PlaneContact::new([0., 1., 0.], 0.01, 20000.).unwrap()))
        .unwrap();
    let initial = body.diagnostics().unwrap();
    let receipt = body
        .step_with_moving_plane(None, 0.012, 0.1, 1e-10)
        .unwrap();
    let expected = 3. * 0.5 * 20000. * 0.002_f64.powi(2);
    assert!((receipt.plane_work_j - expected).abs() < 1e-12);
    assert_eq!(receipt.support_work_j, 0.);
    let after = body.diagnostics().unwrap();
    assert!(
        (after.potential_j - initial.potential_j - receipt.plane_work_j - receipt.energy_defect_j)
            .abs()
            < 1e-12
    );
}

#[test]
fn translating_plane_is_galilean_equivalent_and_work_matches_momentum() {
    let mut fixed = tetra(false, -1., false);
    let speed = 0.4;
    let mut moving = tetra(false, -1. + speed, false);
    let dt = 1e-5;
    let mut work = 0.;
    let initial = moving.diagnostics().unwrap();
    let mut defect = 0.;
    for step in 1..=6000 {
        let time = f64::from(step) * dt;
        fixed.step(dt, 1e-4).unwrap();
        let before_momentum = moving.diagnostics().unwrap().momentum_kg_m_s[1];
        let receipt = moving
            .step_with_moving_plane(None, speed * time, dt, 1e-4)
            .unwrap();
        let after = moving.diagnostics().unwrap();
        assert!(
            (receipt.plane_work_j - speed * (after.momentum_kg_m_s[1] - before_momentum)).abs()
                < 1e-12
        );
        assert_eq!(receipt.support_work_j, 0.);
        work += receipt.plane_work_j;
        defect += receipt.energy_defect_j;
        for (a, b) in fixed
            .body()
            .positions()
            .iter()
            .zip(moving.body().positions())
        {
            assert!((a[0] - b[0]).abs() < 1e-10 && (a[2] - b[2]).abs() < 1e-10);
            assert!((a[1] + speed * time - b[1]).abs() < 1e-10);
        }
        for (a, b) in fixed.velocities().iter().zip(moving.velocities()) {
            assert!((a[1] + speed - b[1]).abs() < 1e-9);
        }
    }
    let after = moving.diagnostics().unwrap();
    assert!(work > 0.);
    assert!(
        (after.kinetic_j + after.potential_j
            - initial.kinetic_j
            - initial.potential_j
            - work
            - defect)
            .abs()
            < 1e-10
    );
    println!("MOVING PLANE actuator work={work:.12} J; defect={defect:.12} J");
}

#[test]
fn moving_plane_rejection_rolls_back_offset_histories_and_heat() {
    let mut body = tetra(true, 0., true);
    body.enable_maxwell_thermal(&[3500.], &[310.15]).unwrap();
    let targets: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: [p[0] + 0.01 * p[1], p[1], p[2]],
        })
        .collect();
    body.step_viscoelastic_with_moving_plane(Some(&targets), 0., 0.001, 1e-6)
        .unwrap();
    let before = format!("{body:?}");
    for offset in [f64::NAN, f64::INFINITY, f64::MAX] {
        assert!(
            body.step_viscoelastic_with_moving_plane(Some(&targets), offset, 0.001, 1e-6)
                .is_err()
        );
        assert_eq!(format!("{body:?}"), before);
    }
    let receipt = body
        .step_viscoelastic_with_moving_plane(Some(&targets), 0., 0.001, 1e-6)
        .unwrap();
    assert!(receipt.viscous_heat_j > 0.);
    assert_eq!(receipt.support.plane_work_j, 0.);
}

#[test]
fn co_translating_pins_and_plane_cancel_contact_actuator_work() {
    let mut body = tetra(true, 0., false);
    body.set_plane_contact(Some(
        PlaneContact::new([0., 1., 0.], 0.012, 20000.).unwrap(),
    ))
    .unwrap();
    let before = body.diagnostics().unwrap();
    let targets: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: [p[0], p[1] + 0.003, p[2]],
        })
        .collect();
    let receipt = body
        .step_with_moving_plane(Some(&targets), 0.015, 0.1, 1e-10)
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert!((after.contact_j - before.contact_j).abs() < 1e-12);
    assert!(receipt.plane_work_j > 0.);
    assert!((receipt.reaction_work_j + receipt.plane_work_j).abs() < 1e-12);
    assert!(
        (receipt.support_work_j + receipt.plane_work_j - receipt.pin_kinetic_work_j).abs() < 1e-12
    );
}

#[test]
fn viscoelastic_moving_contact_keeps_actuator_work_separate_from_heat() {
    let mut body = tetra(true, 0., true);
    body.enable_maxwell_thermal(&[3500.], &[310.15]).unwrap();
    body.set_plane_contact(Some(PlaneContact::new([0., 1., 0.], 0.01, 20000.).unwrap()))
        .unwrap();
    let before = body.diagnostics().unwrap();
    let receipt = body
        .step_viscoelastic_with_moving_plane(None, 0.012, 0.001, 1e-10)
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert_eq!(receipt.viscous_heat_j, 0.);
    assert_eq!(receipt.support.support_work_j, 0.);
    assert!((receipt.support.plane_work_j - 0.12).abs() < 1e-12);
    assert!(
        (after.potential_j
            - before.potential_j
            - receipt.support.plane_work_j
            - receipt.total_energy_defect_j)
            .abs()
            < 1e-12
    );
    let mut missing = tetra(false, 0., false);
    missing.set_plane_contact(None).unwrap();
    let snapshot = format!("{missing:?}");
    assert_eq!(
        missing
            .step_with_moving_plane(None, 0.1, 0.001, 1e-6)
            .unwrap_err(),
        "moving plane requires installed contact"
    );
    assert_eq!(format!("{missing:?}"), snapshot);
}

#[test]
fn moving_contact_work_balance_converges_under_time_refinement() {
    let run = |dt: f64, steps: u32| {
        let mut body = tetra(false, -0.6, false);
        let initial = body.diagnostics().unwrap();
        let mut work = 0.;
        let mut envelope = 0_f64;
        for step in 1..=steps {
            work += body
                .step_with_moving_plane(None, 0.4 * f64::from(step) * dt, dt, 1e-4)
                .unwrap()
                .plane_work_j;
            let after = body.diagnostics().unwrap();
            envelope = envelope.max(
                (after.kinetic_j + after.potential_j
                    - initial.kinetic_j
                    - initial.potential_j
                    - work)
                    .abs(),
            );
        }
        envelope
    };
    let coarse = run(2e-5, 3000);
    let fine = run(1e-5, 6000);
    println!("MOVING PLANE convergence: coarse={coarse:e}, fine={fine:e}");
    assert!(fine < coarse * 0.5);
}

#[test]
fn rotating_plane_torque_work_converges_and_matches_potential_change() {
    let run = |steps: u32| {
        let mut body = tetra(true, 0., false);
        let before = body.diagnostics().unwrap();
        let mut work = 0.;
        let mut defect = 0.;
        for step in 1..=steps {
            let angle = 1.3 * f64::from(step) / f64::from(steps);
            let plane = PlaneContact::new([-angle.sin(), angle.cos(), 0.], 0., 20000.).unwrap();
            let receipt = body
                .step_with_plane_motion(None, plane, 1. / f64::from(steps), 1e-5)
                .unwrap();
            assert_eq!(receipt.plane_translation_work_j, 0.);
            assert_eq!(receipt.support_work_j, 0.);
            assert_eq!(receipt.plane_work_j, receipt.plane_rotation_work_j);
            work += receipt.plane_work_j;
            defect += receipt.energy_defect_j;
        }
        let after = body.diagnostics().unwrap();
        assert!(work > 0.);
        let error = after.potential_j - before.potential_j - work;
        assert!((error - defect).abs() < 1e-10);
        (work, error.abs())
    };
    let (coarse_work, coarse) = run(2000);
    let (fine_work, fine) = run(4000);
    println!(
        "ROTATING PLANE work coarse={coarse_work:e}, fine={fine_work:e}; errors={coarse:e},{fine:e}"
    );
    assert!(fine < coarse * 0.6);
}

#[test]
fn antipodal_motion_and_stiffness_change_reject_without_mutation() {
    let mut body = tetra(true, 0., true);
    body.enable_maxwell_thermal(&[3500.], &[310.15]).unwrap();
    let before = format!("{body:?}");
    for plane in [
        PlaneContact::new([0., -1., 0.], 0., 20000.).unwrap(),
        PlaneContact::new([0., 1., 0.], 0., 20001.).unwrap(),
    ] {
        assert!(
            body.step_viscoelastic_with_plane_motion(None, plane, 0.001, 1e-6)
                .is_err()
        );
        assert_eq!(format!("{body:?}"), before);
    }
    let mut rotation_work = 0.;
    for step in 1..=4000 {
        let angle = f64::from(step) * 0.00005;
        let receipt = body
            .step_viscoelastic_with_plane_motion(
                None,
                PlaneContact::new([-angle.sin(), angle.cos(), 0.], 0., 20000.).unwrap(),
                0.00005,
                1e-6,
            )
            .unwrap();
        assert_eq!(receipt.viscous_heat_j, 0.);
        rotation_work += receipt.support.plane_rotation_work_j;
    }
    assert!(rotation_work > 0.);
    assert_eq!(body.maxwell_temperatures_kelvin().unwrap(), &[310.15]);
}

#[test]
fn endpoint_open_rotating_plane_cannot_hide_intermediate_contact() {
    let body = Body::new(
        vec![
            [0., -0.1, 0.],
            [0.1, -0.1, 0.],
            [0., 0., 0.],
            [0., -0.1, 0.1],
        ],
        vec![true; 4],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(1e5, 0.3).unwrap(),
        )],
    )
    .unwrap();
    let mut body = InertialBody::new_with_fixed_supports(body, &[1000.], vec![[0.; 3]; 4]).unwrap();
    let angle = std::f64::consts::PI / 3.;
    let initial = PlaneContact::new([-angle.sin(), angle.cos(), 0.], -0.08, 20000.).unwrap();
    let next = PlaneContact::new([angle.sin(), angle.cos(), 0.], -0.08, 20000.).unwrap();
    body.set_plane_contact(Some(initial)).unwrap();
    // Node zero has positive endpoint gaps .03 m, but its mid-arc gap is -.02 m.
    let before = format!("{body:?}");
    assert_eq!(
        body.step_with_plane_motion(None, next, 0.01, 1e-6)
            .unwrap_err(),
        "unresolved swept rotating plane contact"
    );
    assert_eq!(format!("{body:?}"), before);
}

#[test]
fn combined_plane_motion_is_covariant_under_world_rotation() {
    let mut original = tetra(true, 0., false);
    // R(x,y,z)=(x,-z,y) is a proper 90-degree world rotation about X.
    let points = original
        .body()
        .positions()
        .iter()
        .map(|p| [p[0], -p[2], p[1]])
        .collect();
    let body = Body::new(
        points,
        vec![true; 4],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(1e5, 0.3).unwrap(),
        )],
    )
    .unwrap();
    let mut rotated =
        InertialBody::new_with_fixed_supports(body, &[1000.], vec![[0.; 3]; 4]).unwrap();
    rotated
        .set_plane_contact(Some(PlaneContact::new([0., 0., 1.], 0., 20000.).unwrap()))
        .unwrap();
    let mut angular_work = 0.;
    let mut linear_work = 0.;
    for step in 1..=2000 {
        let angle = 1.3 * f64::from(step) / 2000.;
        let offset = 0.001 * angle.sin();
        let a = original
            .step_with_plane_motion(
                None,
                PlaneContact::new([-angle.sin(), angle.cos(), 0.], offset, 20000.).unwrap(),
                0.0005,
                1e-5,
            )
            .unwrap();
        let b = rotated
            .step_with_plane_motion(
                None,
                PlaneContact::new([-angle.sin(), 0., angle.cos()], offset, 20000.).unwrap(),
                0.0005,
                1e-5,
            )
            .unwrap();
        assert!((a.plane_translation_work_j - b.plane_translation_work_j).abs() < 1e-12);
        assert!((a.plane_rotation_work_j - b.plane_rotation_work_j).abs() < 1e-12);
        assert!((a.energy_defect_j - b.energy_defect_j).abs() < 1e-11);
        assert!(
            (original.diagnostics().unwrap().contact_j - rotated.diagnostics().unwrap().contact_j)
                .abs()
                < 1e-11
        );
        angular_work += a.plane_rotation_work_j;
        linear_work += a.plane_translation_work_j;
    }
    assert!(angular_work > 0. && linear_work > 0.);
}

#[test]
fn tiny_normal_rotation_cannot_round_independent_torque_work_to_zero() {
    let mut body = tetra(true, 0., false);
    body.set_plane_contact(Some(
        PlaneContact::new([0., 1., 0.], 0.012, 20000.).unwrap(),
    ))
    .unwrap();
    let next = PlaneContact::new([1e-200, 1., 0.], 0.012, 20000.).unwrap();
    let before = format!("{body:?}");
    // Potential change rounds away, but torque work is still representable.
    assert!(
        body.step_with_plane_motion(None, next, 0.001, 1e-210)
            .is_err()
    );
    assert_eq!(format!("{body:?}"), before);
    let receipt = body
        .step_with_plane_motion(None, next, 0.001, 1e-190)
        .unwrap();
    assert!(receipt.plane_rotation_work_j < 0.);
    assert!((receipt.plane_rotation_work_j / -4e-200 - 1.).abs() < 1e-12);
    assert!(receipt.energy_defect_j > 0.);
}
