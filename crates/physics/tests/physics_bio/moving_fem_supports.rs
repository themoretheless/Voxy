use physics::biomechanics::{Body, InertialBody, Material, SupportTarget};

fn specimen(pins: Vec<bool>) -> InertialBody {
    let points = vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let body = Body::new(
        points,
        pins,
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 10.,
                bulk_pa: 1000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    InertialBody::new_with_fixed_supports(body, &[1.], vec![[0.; 3]; 4]).unwrap()
}
fn targets(
    body: &InertialBody,
    nodes: &[usize],
    transform: impl Fn([f64; 3]) -> [f64; 3],
) -> Vec<SupportTarget> {
    nodes
        .iter()
        .map(|&node| SupportTarget {
            node,
            position_m: transform(body.body().positions()[node]),
        })
        .collect()
}

#[test]
fn empty_support_targets_match_analytic_free_fall_without_actuator_work() {
    let mut ordinary = specimen(vec![false; 4]);
    ordinary.set_uniform_acceleration([0., -9.81, 0.]).unwrap();
    let mut driven = ordinary.clone();
    let rest = ordinary.body().positions().to_vec();
    let dt = 0.001;
    for frame in 1..=100 {
        let ordinary_defect = ordinary.step(dt, 1e-10).unwrap();
        let report = driven.step_with_support_targets(&[], dt, 1e-10).unwrap();
        assert_eq!(driven.body().positions(), ordinary.body().positions());
        assert_eq!(driven.velocities(), ordinary.velocities());
        assert!((report.energy_defect_j - ordinary_defect).abs() < 1e-15);
        assert_eq!(report.support_work_j, 0.);
        assert_eq!(report.reaction_work_j, 0.);
        assert_eq!(report.pin_kinetic_work_j, 0.);
        assert_eq!(report.plane_work_j, 0.);
        assert_eq!(report.surface_work_j, 0.);
        let time = f64::from(frame) * dt;
        let diagnostic = driven.diagnostics().unwrap();
        // Unit right tetrahedron at density 1: analytic mass is 1/6 kg.
        assert!((diagnostic.mass_kg - 1. / 6.).abs() < 1e-15);
        let expected_kinetic = 0.5 / 6. * (9.81 * time).powi(2);
        assert!((diagnostic.kinetic_j - expected_kinetic).abs() < 1e-12);
        for ((position, velocity), initial) in driven
            .body()
            .positions()
            .iter()
            .zip(driven.velocities())
            .zip(&rest)
        {
            for axis in 0..3 {
                let acceleration = if axis == 1 { -9.81 } else { 0. };
                let expected = initial[axis] + 0.5 * acceleration * time * time;
                assert!((position[axis] - expected).abs() < 1e-12);
                assert!((velocity[axis] - acceleration * time).abs() < 1e-11);
            }
        }
    }
}

#[test]
fn prescribed_translation_books_gravity_and_pin_acceleration_work() {
    for gravity_m_s2 in [0., 9.81] {
        let mut body = specimen(vec![true; 4]);
        body.set_uniform_acceleration([0., -gravity_m_s2, 0.])
            .unwrap();
        let mass = body.diagnostics().unwrap().mass_kg;
        let controls = targets(&body, &[0, 1, 2, 3], |p| [p[0] + 0.001, p[1] + 0.002, p[2]]);
        let report = body
            .step_with_support_targets(&controls, 0.1, 1e-10)
            .unwrap();
        let kinetic = 0.5 * mass * (0.01_f64.powi(2) + 0.02_f64.powi(2));
        let gravity = mass * gravity_m_s2 * 0.002;
        assert!((report.pin_kinetic_work_j - kinetic).abs() < 1e-12);
        assert!((report.support_work_j - kinetic - gravity).abs() < 1e-12);
        assert!(report.energy_defect_j.abs() < 1e-12);
    }
}

#[test]
fn prescribed_shear_books_analytic_elastic_work_and_explicit_stopping() {
    let mut body = specimen(vec![true; 4]);
    let controls = targets(&body, &[0, 1, 2, 3], |p| [p[0] + 0.01 * p[1], p[1], p[2]]);
    let report = body
        .step_with_support_targets(&controls, 0.1, 1e-10)
        .unwrap();
    let elastic = 0.5 * 10. * 0.01_f64.powi(2) / 6.;
    assert!((report.support_work_j - report.pin_kinetic_work_j - elastic).abs() < 1e-12);
    let moving = format!("{body:?}");
    assert!(body.step(0.1, 1e-10).is_err());
    assert_eq!(format!("{body:?}"), moving);
    let stop = targets(&body, &[0, 1, 2, 3], |p| p);
    let braking = body.step_with_support_targets(&stop, 0.1, 1e-10).unwrap();
    assert!((braking.support_work_j + report.pin_kinetic_work_j).abs() < 1e-12);
    assert!(body.velocities().iter().flatten().all(|v| *v == 0.));
    assert_eq!(body.step(0.1, 1e-10).unwrap(), 0.);
}

#[test]
fn free_nodes_retain_inertia_and_support_work_defect_converges() {
    let mut defects = Vec::new();
    for dt in [0.01, 0.005] {
        let mut body = specimen(vec![true, true, true, false]);
        let controls = targets(&body, &[0, 1, 2], |p| [p[0] + 0.05 * dt, p[1], p[2]]);
        let report = body.step_with_support_targets(&controls, dt, 1e-6).unwrap();
        assert_eq!(body.body().positions()[3], [0., 0., 1.]);
        assert!(body.velocities()[3][0].abs() > 1e-8);
        assert_eq!(body.body().positions()[0], controls[0].position_m);
        defects.push(report.energy_defect_j.abs());
    }
    assert!(defects[0] > 1e-14);
    assert!(defects[1] < defects[0] * 0.2, "defects={defects:?}");
}

#[test]
fn invalid_targets_inversion_and_strict_work_guard_roll_back_all_state() {
    let mut body = specimen(vec![true, true, true, false]);
    let before = format!("{body:?}");
    let valid = targets(&body, &[0, 1, 2], |p| [p[0] + 0.001, p[1], p[2]]);
    let mut duplicate = valid.clone();
    duplicate[1] = duplicate[0];
    let mut free = valid.clone();
    free[1].node = 3;
    let mut nonfinite = valid.clone();
    nonfinite[1].position_m[0] = f64::NAN;
    for invalid in [vec![], valid[..2].to_vec(), duplicate, free, nonfinite] {
        assert!(
            body.step_with_support_targets(&invalid, 0.01, 1e-6)
                .is_err()
        );
        assert_eq!(format!("{body:?}"), before);
    }
    assert!(body.step_with_support_targets(&valid, 0.01, 1e-30).is_err());
    assert_eq!(format!("{body:?}"), before);
    let mut all = specimen(vec![true; 4]);
    let before_all = format!("{all:?}");
    let inverted = targets(&all, &[0, 1, 2, 3], |p| [p[0], p[1], -p[2]]);
    assert!(all.step_with_support_targets(&inverted, 0.1, 1.).is_err());
    assert_eq!(format!("{all:?}"), before_all);
    let nonlinear = targets(&all, &[0, 1, 2, 3], |p| [p[0], p[1], 0.9 * p[2]]);
    assert!(
        all.step_with_support_targets(&nonlinear, 0.1, 1e-12)
            .is_err()
    );
    assert_eq!(format!("{all:?}"), before_all);
}

#[test]
fn huge_pin_kinetic_energy_cannot_erase_reaction_work_and_pass_the_guard() {
    let mut body = specimen(vec![true; 4]);
    let before = format!("{body:?}");
    let controls = targets(&body, &[0, 1, 2, 3], |p| [p[0], p[1], 0.9 * p[2]]);
    assert_eq!(
        body.step_with_support_targets(&controls, 1e-12, 1e-12)
            .unwrap_err(),
        "unrepresentable prescribed support work"
    );
    assert_eq!(format!("{body:?}"), before);
}

#[test]
fn positive_endpoint_rotation_cannot_hide_mid_segment_collapse() {
    let mut supported = specimen(vec![true; 4]);
    let before = format!("{supported:?}");
    for scale in [[-1., -1., 1.], [-1., -2., 2.]] {
        let controls = targets(&supported, &[0, 1, 2, 3], |p| {
            std::array::from_fn(|axis| scale[axis] * p[axis])
        });
        assert_eq!(
            supported
                .step_with_support_targets(&controls, 0.1, 1.)
                .unwrap_err(),
            "inertial tetrahedral path collapse"
        );
        assert_eq!(format!("{supported:?}"), before);
    }
    let quarter_turn = targets(&supported, &[0, 1, 2, 3], |p| [-p[1], p[0], p[2]]);
    supported
        .step_with_support_targets(&quarter_turn, 0.1, 1e-9)
        .unwrap();
    assert!(supported.diagnostics().unwrap().potential_j.abs() < 1e-12);
    let points = vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let velocities = points
        .iter()
        .map(|p| [-20. * p[0], -20. * p[1], 0.])
        .collect();
    let body = Body::new(
        points,
        vec![false; 4],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 10.,
                bulk_pa: 1000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    let mut free = InertialBody::new(body, &[1.], velocities).unwrap();
    let before = format!("{free:?}");
    assert_eq!(
        free.step(0.1, 1.).unwrap_err(),
        "inertial tetrahedral path collapse"
    );
    assert_eq!(format!("{free:?}"), before);
}

#[test]
fn prescribed_trial_positions_drive_plane_contact_and_reaction_work() {
    use physics::biomechanics::PlaneContact;
    let mut body = specimen(vec![true; 4]);
    body.set_plane_contact(Some(PlaneContact::new([0., 1., 0.], 0., 100.).unwrap()))
        .unwrap();
    let initial = body.diagnostics().unwrap();
    assert_eq!(initial.contact_j, 0.);
    let controls = targets(&body, &[0, 1, 2, 3], |p| [p[0], p[1] - 0.002, p[2]]);
    let report = body
        .step_with_support_targets(&controls, 0.1, 1e-10)
        .unwrap();
    let after = body.diagnostics().unwrap();
    // Three boundary nodes enter the halfspace penalty; the top node remains open.
    let expected = 3. * 0.5 * 100. * 0.002_f64.powi(2);
    assert!((after.contact_j - expected).abs() < 1e-14);
    assert!((report.reaction_work_j - expected).abs() < 1e-12);
    assert!(
        (after.kinetic_j + after.potential_j
            - initial.kinetic_j
            - initial.potential_j
            - report.support_work_j)
            .abs()
            < 1e-12
    );
}
