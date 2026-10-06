use physics::biomechanics::{EmbeddedTriangleContact, PrescribedTriangleSurface, RelativeSkinPose};
const REST: [[f64; 3]; 4] = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
const SKIN: [[f64; 3]; 3] = [[0.1, 0.1, 0.015], [0.15, 0.1, 0.04], [0.1, 0.15, 0.04]];
fn contact() -> EmbeddedTriangleContact {
    EmbeddedTriangleContact::new(&REST, &[[0, 1, 2, 3]], &SKIN, vec![[0, 1, 2]]).unwrap()
}
fn obstacle() -> PrescribedTriangleSurface {
    PrescribedTriangleSurface::new(
        vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
        vec![[0, 1, 2]],
        0.001,
        0.03,
        100.,
    )
    .unwrap()
}
fn pose<'a>(
    nodes: &'a [[f64; 3]],
    reference: &'a [[f64; 3]],
    base: &'a [[f64; 3]],
) -> RelativeSkinPose<'a> {
    RelativeSkinPose {
        nodes,
        reference,
        base,
    }
}
#[test]
fn actual_contact_potential_matches_all_tissue_rig_and_obstacle_forces() {
    let contact = contact();
    let obstacle = obstacle();
    let base = SKIN.map(|p| [p[0] + 0.004, p[1], p[2] + 0.003]);
    let response = contact
        .response(pose(&REST, &REST, &base), &obstacle)
        .unwrap();
    assert!(response.potential_j > 0.);
    let h = 1e-7;
    for component in 0..3 {
        let count = if component == 2 { 3 } else { 4 };
        for node in 0..count {
            for axis in 0..3 {
                let mut nodes_plus = REST;
                let mut nodes_minus = REST;
                let mut ref_plus = REST;
                let mut ref_minus = REST;
                let mut base_plus = base;
                let mut base_minus = base;
                let expected = match component {
                    0 => {
                        nodes_plus[node][axis] += h;
                        nodes_minus[node][axis] -= h;
                        response.skin_loads.nodal_forces_n()[node][axis]
                    }
                    1 => {
                        ref_plus[node][axis] += h;
                        ref_minus[node][axis] -= h;
                        response.skin_loads.reference_forces_n()[node][axis]
                    }
                    _ => {
                        base_plus[node][axis] += h;
                        base_minus[node][axis] -= h;
                        response.skin_loads.base_forces_n()[node][axis]
                    }
                };
                let plus = contact
                    .response(pose(&nodes_plus, &ref_plus, &base_plus), &obstacle)
                    .unwrap()
                    .potential_j;
                let minus = contact
                    .response(pose(&nodes_minus, &ref_minus, &base_minus), &obstacle)
                    .unwrap()
                    .potential_j;
                assert!(
                    ((plus - minus) / (2. * h) + expected).abs() < 1e-6 * (1. + expected.abs()),
                    "component={component} node={node} axis={axis}"
                );
            }
        }
    }
    for node in 0..3 {
        for axis in 0..3 {
            let mut plus = obstacle.positions().to_vec();
            let mut minus = plus.clone();
            plus[node][axis] += h;
            minus[node][axis] -= h;
            let a = contact
                .response(
                    pose(&REST, &REST, &base),
                    &obstacle.with_positions(plus).unwrap(),
                )
                .unwrap()
                .potential_j;
            let b = contact
                .response(
                    pose(&REST, &REST, &base),
                    &obstacle.with_positions(minus).unwrap(),
                )
                .unwrap()
                .potential_j;
            let force = response.obstacle_forces_n[node][axis];
            assert!(((a - b) / (2. * h) + force).abs() < 1e-6 * (1. + force.abs()));
        }
    }
    let (rig_force, _) = response
        .skin_loads
        .rig_wrench_about(&REST, &base, [0.; 3])
        .unwrap();
    for axis in 0..3 {
        let sum: f64 = response
            .skin_loads
            .nodal_forces_n()
            .iter()
            .chain(&response.obstacle_forces_n)
            .map(|f| f[axis])
            .sum();
        assert!((sum + rig_force[axis]).abs() < 1e-12);
    }
}
#[test]
fn embedded_ccd_rejects_crossing_even_with_open_endpoints() {
    let contact = contact();
    let obstacle = obstacle();
    let end = SKIN.map(|p| [p[0], p[1], p[2] - 0.1]);
    let start = pose(&REST, &REST, &SKIN);
    let end = pose(&REST, &REST, &end);
    assert!(contact.response(start, &obstacle).is_ok());
    assert!(contact.response(end, &obstacle).is_ok());
    assert!(
        !contact
            .path_is_open(start, end, &obstacle, &obstacle)
            .unwrap()
    );
    assert!(
        contact
            .path_response(start, end, &obstacle, &obstacle, 1e-10)
            .is_err()
    );
}
#[test]
fn simultaneous_tissue_rig_and_obstacle_motion_passes_work_budget() {
    let contact = contact();
    let obstacle = obstacle();
    let mut nodes = REST;
    nodes[3][2] += 0.0003;
    let reference = REST.map(|p| [p[0], p[1], p[2] + 0.0004]);
    let base = SKIN.map(|p| [p[0], p[1], p[2] + 0.001]);
    let next = obstacle
        .with_positions(
            obstacle
                .positions()
                .iter()
                .map(|p| [p[0], p[1], p[2] + 0.0002])
                .collect(),
        )
        .unwrap();
    let start = pose(&REST, &REST, &SKIN);
    let end = pose(&nodes, &reference, &base);
    let report = contact
        .path_response(start, end, &obstacle, &next, 1e-10)
        .unwrap();
    assert!(report.potential_change_j.abs() > 1e-6);
    assert!(report.work_defect_j.abs() <= 1e-10);
    assert!((1..=128).contains(&report.quadrature_panels));
    let delta_reference = [[0., 0., 0.0004]; 4];
    let delta_base = [[0., 0., 0.001]; 3];
    assert!(
        report
            .average_skin_loads
            .actuator_work_j(&delta_reference, &delta_base)
            .unwrap()
            .abs()
            > 1e-6
    );
    assert!(
        contact
            .path_response(start, end, &obstacle, &next, 0.)
            .is_err()
    );
    assert!(
        contact
            .path_is_open(start, end, &obstacle, &self::obstacle())
            .is_err()
    );
}
#[test]
fn invalid_embedded_contact_topology_never_enters_evaluation() {
    assert!(EmbeddedTriangleContact::new(&REST, &[[0, 1, 2, 3]], &SKIN, vec![[0, 1, 3]]).is_err());
    assert!(
        EmbeddedTriangleContact::new(&REST, &[[0, 1, 2, 3]], &SKIN, vec![[0, 1, 2], [2, 1, 0]])
            .is_err()
    );
    assert!(
        EmbeddedTriangleContact::new(&REST, &[[0, 1, 2, 3]], &[[2.; 3]; 3], vec![[0, 1, 2]])
            .is_err()
    );
    assert!(
        contact()
            .response(pose(&REST[..3], &REST, &SKIN), &obstacle())
            .is_err()
    );
}

#[test]
fn embedded_normal_metric_is_symmetric_positive_and_preserves_tangent_nullspace() {
    let contact = contact();
    let obstacle = obstacle();
    let pose = pose(&REST, &REST, &SKIN);
    let u = [
        [0.1, -0.2, 0.3],
        [-0.2, 0.4, 0.1],
        [0.3, 0.2, -0.4],
        [-0.1, 0.3, 0.2],
    ];
    let v = [
        [-0.3, 0.2, 0.1],
        [0.2, 0.1, -0.3],
        [-0.4, 0.1, 0.2],
        [0.2, -0.1, 0.3],
    ];
    let au = contact.normal_metric_apply(pose, &obstacle, &u).unwrap();
    let av = contact.normal_metric_apply(pose, &obstacle, &v).unwrap();
    let dot = |a: &[[f64; 3]], b: &[[f64; 3]]| {
        a.iter()
            .zip(b)
            .flat_map(|(x, y)| (0..3).map(move |i| x[i] * y[i]))
            .sum::<f64>()
    };
    assert!(dot(&u, &au) > 0.);
    assert!(dot(&v, &av) > 0.);
    assert!((dot(&u, &av) - dot(&v, &au)).abs() < 1e-12);
    let tangent = [[1., -2., 0.]; 4];
    let action = contact
        .normal_metric_apply(pose, &obstacle, &tangent)
        .unwrap();
    assert!(action.iter().flatten().all(|x| x.abs() < 1e-12));
    assert!(
        contact
            .normal_metric_apply(pose, &obstacle, &u[..3])
            .is_err()
    );
}

fn inertial_skin(velocity: [f64; 3]) -> physics::biomechanics::InertialBody {
    use physics::biomechanics::{Body, InertialBody, Material};
    let body = Body::new(
        REST.to_vec(),
        vec![false; 4],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 500.,
                bulk_pa: 1000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    InertialBody::new(body, &[1000.], vec![velocity; 4]).unwrap()
}
fn stationary() -> physics::biomechanics::StationaryEmbeddedContact {
    use std::sync::Arc;
    physics::biomechanics::StationaryEmbeddedContact::new(
        Arc::new(contact()),
        REST.to_vec(),
        SKIN.to_vec(),
        Arc::new(obstacle()),
    )
    .unwrap()
}
#[test]
fn existing_inertial_step_responds_to_embedded_contact_and_books_parameter_work_once() {
    use physics::biomechanics::PlaneContact;
    let mut baseline = inertial_skin([0.; 3]);
    let mut body = baseline.clone();
    let initial = body.diagnostics().unwrap().potential_j;
    let work = body
        .set_stationary_embedded_contact(Some(stationary()))
        .unwrap();
    assert!(work > 0.);
    let installed = body.diagnostics().unwrap();
    assert!((installed.potential_j - initial - work).abs() < 1e-14);
    assert!((installed.contact_j - work).abs() < 1e-14);
    // An inactive plane must not add the embedded potential a second time.
    body.set_plane_contact(Some(PlaneContact::new([0., 0., 1.], -1., 100.).unwrap()))
        .unwrap();
    assert_eq!(
        body.diagnostics().unwrap().potential_j,
        installed.potential_j
    );
    let plane_work = body
        .set_plane_contact(Some(PlaneContact::new([0., 0., 1.], 0.1, 100.).unwrap()))
        .unwrap();
    assert!((plane_work - 1.5).abs() < 1e-12);
    let combined = body.diagnostics().unwrap();
    assert!((combined.potential_j - installed.potential_j - 1.5).abs() < 1e-12);
    assert!((combined.contact_j - installed.contact_j - 1.5).abs() < 1e-12);
    body.set_plane_contact(None).unwrap();

    baseline.step(1e-4, 1e-8).unwrap();
    let defect = body.step(1e-4, 1e-8).unwrap();
    assert!(defect.abs() <= 1e-8);
    assert!(body.velocities().iter().map(|v| v[2]).sum::<f64>() > 0.);
    assert!(baseline.velocities().iter().flatten().all(|v| *v == 0.));
    let contact_energy = body
        .body()
        .stationary_embedded_contact()
        .unwrap()
        .response(body.body().positions())
        .unwrap()
        .potential_j;
    let removed = body.set_stationary_embedded_contact(None).unwrap();
    assert!((removed + contact_energy).abs() < 1e-12);
}
#[test]
fn existing_inertial_step_rejects_embedded_tunnelling_without_mutating_state() {
    let mut body = inertial_skin([0., 0., -100.]);
    body.set_stationary_embedded_contact(Some(stationary()))
        .unwrap();
    let before = format!("{body:?}");
    assert_eq!(
        body.step(0.01, 1e-4).unwrap_err(),
        "inertial tissue gap path crossing"
    );
    assert_eq!(format!("{body:?}"), before);
}
#[test]
fn wrong_rest_contact_owner_is_rejected_atomically() {
    use physics::biomechanics::{Body, Material};
    let shifted = REST.map(|p| [p[0] + 1., p[1], p[2]]);
    let mut body = Body::new(
        shifted.to_vec(),
        vec![false; 4],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 500.,
                bulk_pa: 1000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    let before = format!("{body:?}");
    assert_eq!(
        body.set_stationary_embedded_contact(Some(stationary()))
            .unwrap_err(),
        "embedded contact rest owner mismatch"
    );
    assert_eq!(format!("{body:?}"), before);
}

#[test]
fn existing_implicit_solver_includes_stationary_embedded_contact_in_averaged_material_path() {
    use std::sync::Arc;
    let mut body = inertial_skin([0.; 3]);
    body.set_stationary_embedded_contact(Some(stationary()))
        .unwrap();
    let original = obstacle();
    let far = Arc::new(
        original
            .with_positions(
                original
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 10.])
                    .collect(),
            )
            .unwrap(),
    );
    body.set_prescribed_surface(Some(far.clone())).unwrap();
    let result = body
        .step_implicit_with_surface_motion(None, far, 1e-3, 1e-8)
        .unwrap();
    assert!(result.energy_defect_j.abs() <= 1e-8);
    assert!(body.velocities().iter().map(|v| v[2]).sum::<f64>() > 0.);
    assert!(body.diagnostics().unwrap().contact_j > 0.);
}

#[test]
fn embedded_contact_requires_actual_material_cell_membership() {
    use physics::biomechanics::{Body, Material, StationaryEmbeddedContact};
    use std::sync::Arc;
    let mut points = REST.to_vec();
    points.push([0., 0., 2.]);
    let contact =
        EmbeddedTriangleContact::new(&points, &[[0, 1, 2, 4]], &SKIN, vec![[0, 1, 2]]).unwrap();
    let stationary = StationaryEmbeddedContact::new(
        Arc::new(contact),
        points.clone(),
        SKIN.to_vec(),
        Arc::new(obstacle()),
    )
    .unwrap();
    let mut body = Body::new(
        points,
        vec![false, false, false, false, true],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 500.,
                bulk_pa: 1000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    let before = format!("{body:?}");
    assert_eq!(
        body.set_stationary_embedded_contact(Some(stationary))
            .unwrap_err(),
        "embedded contact rest owner mismatch"
    );
    assert_eq!(format!("{body:?}"), before);
}

#[test]
fn staged_skin_motion_uses_actual_nodes_instead_of_rest_pose_for_admission() {
    use std::sync::Arc;
    let original = Arc::new(obstacle());
    let start = physics::biomechanics::StationaryEmbeddedContact::new(
        Arc::new(contact()),
        REST.to_vec(),
        SKIN.to_vec(),
        original.clone(),
    )
    .unwrap();
    let base = SKIN.map(|p| [p[0], p[1], p[2] - 0.015]);
    let next = start
        .with_pose(REST.to_vec(), base.to_vec(), original.clone())
        .unwrap();
    assert!(next.response(&REST).is_err());
    let mut body = inertial_skin([0.; 3]);
    let before = format!("{body:?}");
    assert!(
        body.set_stationary_embedded_contact(Some(next.clone()))
            .is_err()
    );
    assert_eq!(format!("{body:?}"), before);
    let mut moving = inertial_skin([0., 0., 1.]);
    moving.step(0.02, 1e-6).unwrap();
    assert!(
        moving
            .set_stationary_embedded_contact(Some(next.clone()))
            .unwrap()
            > 0.
    );

    let nodes = REST.map(|p| [p[0], p[1], p[2] + 0.02]);
    assert!(next.response(&nodes).is_ok());
    let report = start.path_response_to(&next, &REST, &nodes, 1e-9).unwrap();
    assert!(report.work_defect_j.abs() <= 1e-9);
    let foreign = stationary();
    assert_eq!(
        start
            .path_response_to(&foreign, &REST, &REST, 1e-9)
            .unwrap_err(),
        "embedded skin binding owner changed"
    );
    assert!(
        start
            .with_pose(REST.to_vec(), SKIN.to_vec(), Arc::new(obstacle()))
            .is_err()
    );
    assert!(
        start
            .with_pose(
                REST[..3].to_vec(),
                SKIN.to_vec(),
                Arc::new(self::obstacle())
            )
            .is_err()
    );
}

#[test]
fn existing_step_books_moving_skin_rig_and_obstacle_work() {
    use std::sync::Arc;
    let start = stationary();
    let mut body = inertial_skin([0.; 3]);
    body.set_stationary_embedded_contact(Some(start.clone()))
        .unwrap();
    let before = body.diagnostics().unwrap();
    let reference = REST.map(|p| [p[0], p[1], p[2] + 0.0000004]);
    let base = SKIN.map(|p| [p[0], p[1], p[2] + 0.000001]);
    let obstacle = Arc::new(
        start
            .obstacle()
            .with_positions(
                start
                    .obstacle()
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 0.0000002])
                    .collect(),
            )
            .unwrap(),
    );
    let next = start
        .with_pose(reference.to_vec(), base.to_vec(), obstacle)
        .unwrap();
    let (report, skin_work) = body
        .step_with_embedded_skin_motion(None, next, 1e-3, 1e-8)
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert!(skin_work.rig_work_j.abs() > 1e-10);
    assert!(skin_work.obstacle_work_j.abs() > 1e-10);
    assert!(
        (report.surface_work_j - skin_work.rig_work_j - skin_work.obstacle_work_j).abs() < 1e-14
    );
    let independent = after.kinetic_j - before.kinetic_j + after.potential_j
        - before.potential_j
        - report.surface_work_j;
    assert!((independent - report.energy_defect_j).abs() < 1e-14);
    assert!(independent.abs() <= 1e-8);
}
#[test]
fn moving_skin_crossing_and_work_failure_preserve_complete_state() {
    use std::sync::Arc;
    let start = stationary();
    let mut body = inertial_skin([0.; 3]);
    body.set_stationary_embedded_contact(Some(start.clone()))
        .unwrap();
    let before = format!("{body:?}");
    let crossing = SKIN.map(|p| [p[0], p[1], p[2] - 0.1]);
    let next = start
        .with_pose(
            REST.to_vec(),
            crossing.to_vec(),
            Arc::new(start.obstacle().clone()),
        )
        .unwrap();
    assert_eq!(
        body.step_with_embedded_skin_motion(None, next, 1e-3, 1e-8)
            .unwrap_err(),
        "inertial embedded skin path crossing"
    );
    assert_eq!(format!("{body:?}"), before);
    let displacement = SKIN.map(|p| [p[0], p[1], p[2] + 0.005]);
    let next = start
        .with_pose(
            REST.to_vec(),
            displacement.to_vec(),
            Arc::new(start.obstacle().clone()),
        )
        .unwrap();
    assert!(
        body.step_with_embedded_skin_motion(None, next, 1e-3, 1e-30)
            .is_err()
    );
    assert_eq!(format!("{body:?}"), before);
}
#[test]
fn simultaneous_comotion_uses_moving_ccd_instead_of_old_stationary_skin_guard() {
    use std::sync::Arc;
    let start = stationary();
    let mut body = inertial_skin([0., 0., -40.]);
    body.set_stationary_embedded_contact(Some(start.clone()))
        .unwrap();
    let shift = |p: [f64; 3]| [p[0], p[1], p[2] - 0.04];
    let obstacle = Arc::new(
        start
            .obstacle()
            .with_positions(
                start
                    .obstacle()
                    .positions()
                    .iter()
                    .copied()
                    .map(shift)
                    .collect(),
            )
            .unwrap(),
    );
    let next = start
        .with_pose(REST.map(shift).to_vec(), SKIN.map(shift).to_vec(), obstacle)
        .unwrap();
    let (report, work) = body
        .step_with_embedded_skin_motion(None, next, 1e-3, 1e-6)
        .unwrap();
    assert!(report.energy_defect_j.abs() <= 1e-6);
    assert!(work.obstacle_work_j.abs() > 1e-4);
    assert!(body.body().positions()[0][2] < -0.03);
}

#[test]
fn moving_skin_energy_discrepancy_converges_under_time_refinement() {
    use std::sync::Arc;
    let mut errors = Vec::new();
    for subdivisions in [1, 2, 4] {
        let mut body = inertial_skin([0.; 3]);
        body.set_stationary_embedded_contact(Some(stationary()))
            .unwrap();
        let mut absolute_error = 0.;
        for step in 1..=subdivisions {
            let current = body.body().stationary_embedded_contact().unwrap().clone();
            let base = SKIN.map(|p| {
                [
                    p[0],
                    p[1],
                    p[2] + 0.005 * (step as f64) / (subdivisions as f64),
                ]
            });
            let next = current
                .with_pose(
                    REST.to_vec(),
                    base.to_vec(),
                    Arc::new(current.obstacle().clone()),
                )
                .unwrap();
            // A loose admission budget here measures truncation, not production tolerances.
            let (report, _) = body
                .step_with_embedded_skin_motion(None, next, 1e-3 / (subdivisions as f64), 1.)
                .unwrap();
            absolute_error += report.energy_defect_j.abs();
        }
        errors.push(absolute_error);
    }
    eprintln!("MOVING_SKIN_REFINEMENT absolute_error_j={errors:?}");
    assert!(errors[0] > 1e-12);
    assert!(errors[1] < 0.4 * errors[0]);
    assert!(errors[2] < 0.4 * errors[1]);
}

fn maxwell_skin() -> physics::biomechanics::InertialBody {
    use physics::biomechanics::{
        Body, InertialBody, Material, MaxwellBranch, OgdenTerm, ViscoelasticOgden,
    };
    let mut body = Body::new(
        REST.to_vec(),
        vec![false; 4],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 100.,
                bulk_pa: 1000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    let law = ViscoelasticOgden::new(
        vec![OgdenTerm {
            shear_pa: 100.,
            exponent: 2.,
        }],
        1000.,
        vec![MaxwellBranch {
            shear_pa: 700.,
            relaxation_seconds: 0.2,
        }],
    )
    .unwrap();
    body.set_viscoelastic_ogden(0, law).unwrap();
    let mut strained = REST;
    strained[2][0] = 0.1;
    body.restore_diagnostic_positions(&strained).unwrap();
    let mut dynamics =
        InertialBody::new_viscoelastic_with_supports(body, &[1000.], vec![[0.; 3]; 4]).unwrap();
    dynamics.enable_maxwell_thermal(&[2.], &[300.]).unwrap();
    dynamics
        .set_stationary_embedded_contact(Some(stationary()))
        .unwrap();
    dynamics
}
#[test]
fn moving_skin_maxwell_transaction_balances_mechanics_heat_and_actuator_work() {
    use std::sync::Arc;
    let mut body = maxwell_skin();
    let before = body.diagnostics().unwrap();
    let thermal_before = body.maxwell_sensible_energy_j().unwrap()[0];
    let start = body.body().stationary_embedded_contact().unwrap().clone();
    let base = SKIN.map(|p| [p[0], p[1], p[2] + 1e-6]);
    let next = start
        .with_pose(
            REST.to_vec(),
            base.to_vec(),
            Arc::new(start.obstacle().clone()),
        )
        .unwrap();
    let (report, work) = body
        .step_viscoelastic_with_embedded_skin_motion(None, next, 1e-3, 1e-6)
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert!(report.viscous_heat_j > 0.);
    assert!(work.rig_work_j.abs() > 1e-10);
    let independent = after.kinetic_j - before.kinetic_j + after.potential_j - before.potential_j
        + report.viscous_heat_j
        - report.support.surface_work_j;
    assert!(independent.abs() <= 1e-6);
    assert!(report.total_energy_defect_j.abs() <= 1e-6);
    let thermal_change = body.maxwell_sensible_energy_j().unwrap()[0] - thermal_before;
    assert!((thermal_change - report.viscous_heat_j).abs() < 1e-9);
    assert!(body.maxwell_temperatures_kelvin().unwrap()[0] > 300.);
}
#[test]
fn moving_skin_failure_rolls_back_maxwell_history_and_thermal_storage() {
    use std::sync::Arc;
    let mut body = maxwell_skin();
    let before = format!("{body:?}");
    let start = body.body().stationary_embedded_contact().unwrap().clone();
    let base = SKIN.map(|p| [p[0], p[1], p[2] - 0.1]);
    let next = start
        .with_pose(
            REST.to_vec(),
            base.to_vec(),
            Arc::new(start.obstacle().clone()),
        )
        .unwrap();
    assert_eq!(
        body.step_viscoelastic_with_embedded_skin_motion(None, next, 1e-3, 1e-6)
            .unwrap_err(),
        "inertial embedded skin path crossing"
    );
    assert_eq!(format!("{body:?}"), before);
    let base = SKIN.map(|p| [p[0], p[1], p[2] + 0.005]);
    let next = start
        .with_pose(
            REST.to_vec(),
            base.to_vec(),
            Arc::new(start.obstacle().clone()),
        )
        .unwrap();
    assert_eq!(
        body.step_viscoelastic_with_embedded_skin_motion(None, next, 1e-3, 1e-6)
            .unwrap_err(),
        "finite-deformation inertial energy defect"
    );
    assert_eq!(format!("{body:?}"), before);
}

#[test]
fn implicit_moving_skin_admits_independent_energy_and_commits_pose_atomically() {
    use std::sync::Arc;
    let start = stationary();
    let mut body = inertial_skin([0.; 3]);
    body.set_stationary_embedded_contact(Some(start.clone()))
        .unwrap();
    // Native owner is required by this coupled API. Isolate embedded work here.
    let original = obstacle();
    let far = Arc::new(
        original
            .with_positions(
                original
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 10.])
                    .collect(),
            )
            .unwrap(),
    );
    body.set_prescribed_surface(Some(far.clone())).unwrap();
    let next = start
        .with_pose(
            REST.map(|p| [p[0], p[1], p[2] + 4e-7]).to_vec(),
            SKIN.map(|p| [p[0], p[1], p[2] + 1e-6]).to_vec(),
            Arc::new(
                start
                    .obstacle()
                    .with_positions(
                        start
                            .obstacle()
                            .positions()
                            .iter()
                            .map(|p| [p[0], p[1], p[2] + 2e-7])
                            .collect(),
                    )
                    .unwrap(),
            ),
        )
        .unwrap();
    let before = body.diagnostics().unwrap();
    let expected_pose = format!("{next:?}");
    let result = body
        .step_implicit_with_surface_and_skin_motion(None, far.clone(), next, 1e-3, 1e-8)
        .unwrap();
    let after = body.diagnostics().unwrap();
    let defect = after.kinetic_j - before.kinetic_j + after.potential_j
        - before.potential_j
        - result.surface_work_j;
    assert!(result.surface_work_j.abs() > 1e-10);
    assert!(defect.abs() <= 1e-8);
    assert!((defect - result.energy_defect_j).abs() < 1e-14);
    assert_eq!(
        expected_pose,
        format!("{:?}", body.body().stationary_embedded_contact().unwrap())
    );
    let saved = format!("{body:?}");
    let current = body.body().stationary_embedded_contact().unwrap().clone();
    let crossing = current
        .with_pose(
            REST.to_vec(),
            SKIN.map(|p| [p[0], p[1], p[2] - 0.1]).to_vec(),
            Arc::new(current.obstacle().clone()),
        )
        .unwrap();
    assert!(
        body.step_implicit_with_surface_and_skin_motion(None, far, crossing, 1e-3, 1e-8)
            .is_err()
    );
    assert_eq!(saved, format!("{body:?}"));
}

#[test]
fn implicit_moving_skin_stiffness_sweep_and_work_rejection_are_transactional() {
    use physics::biomechanics::StationaryEmbeddedContact;
    use std::sync::Arc;
    for stiffness in [100., 10_000., 1_000_000.] {
        let skin_obstacle = Arc::new(
            PrescribedTriangleSurface::new(
                obstacle().positions().to_vec(),
                vec![[0, 1, 2]],
                0.001,
                0.03,
                stiffness,
            )
            .unwrap(),
        );
        let start = StationaryEmbeddedContact::new(
            Arc::new(contact()),
            REST.to_vec(),
            SKIN.to_vec(),
            skin_obstacle,
        )
        .unwrap();
        let mut body = inertial_skin([0.; 3]);
        body.set_stationary_embedded_contact(Some(start.clone()))
            .unwrap();
        let original = obstacle();
        let far = Arc::new(
            original
                .with_positions(
                    original
                        .positions()
                        .iter()
                        .map(|p| [p[0], p[1], p[2] + 10.])
                        .collect(),
                )
                .unwrap(),
        );
        body.set_prescribed_surface(Some(far.clone())).unwrap();
        let next = start
            .with_pose(
                REST.to_vec(),
                SKIN.map(|p| [p[0], p[1], p[2] + 1e-5]).to_vec(),
                Arc::new(start.obstacle().clone()),
            )
            .unwrap();
        let before = body.diagnostics().unwrap();
        let result = body
            .step_implicit_with_surface_and_skin_motion(None, far.clone(), next, 1e-3, 1e-8)
            .unwrap_or_else(|e| panic!("stiffness={stiffness} {e}"));
        let after = body.diagnostics().unwrap();
        let defect = after.kinetic_j - before.kinetic_j + after.potential_j
            - before.potential_j
            - result.surface_work_j;
        assert!(
            defect.abs() <= 1e-8,
            "stiffness={stiffness} defect={defect}"
        );
        let saved = format!("{body:?}");
        let current = body.body().stationary_embedded_contact().unwrap().clone();
        let next = current
            .with_pose(
                REST.to_vec(),
                SKIN.map(|p| [p[0], p[1], p[2] + 0.005]).to_vec(),
                Arc::new(current.obstacle().clone()),
            )
            .unwrap();
        assert!(
            body.step_implicit_with_surface_and_skin_motion(None, far, next, 1e-3, 1e-30)
                .is_err()
        );
        assert_eq!(saved, format!("{body:?}"));
    }
}

#[test]
fn implicit_maxwell_moving_skin_and_native_contact_balance_heat_and_both_actuators() {
    use std::sync::Arc;
    let mut body = maxwell_skin();
    // Tilted surface has a unique closest body vertex; both contacts are active.
    let native = Arc::new(
        PrescribedTriangleSurface::new(
            vec![[-3., -3., 0.135], [3., -3., 0.015], [0., 3., -0.105]],
            vec![[0, 1, 2]],
            0.001,
            0.03,
            100.,
        )
        .unwrap(),
    );
    body.set_prescribed_surface(Some(native.clone())).unwrap();
    let next_native = Arc::new(
        native
            .with_positions(
                native
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 2e-7])
                    .collect(),
            )
            .unwrap(),
    );
    let before = body.diagnostics().unwrap();
    let thermal_before = body.maxwell_sensible_energy_j().unwrap()[0];
    let start = body.body().stationary_embedded_contact().unwrap().clone();
    let next = start
        .with_pose(
            REST.to_vec(),
            SKIN.map(|p| [p[0], p[1], p[2] + 1e-6]).to_vec(),
            Arc::new(
                start
                    .obstacle()
                    .with_positions(
                        start
                            .obstacle()
                            .positions()
                            .iter()
                            .map(|p| [p[0], p[1], p[2] + 1e-7])
                            .collect(),
                    )
                    .unwrap(),
            ),
        )
        .unwrap();
    let expected = format!("{next:?}");
    let (report, work) = body
        .step_viscoelastic_implicit_with_surface_and_skin_motion(
            None,
            next_native,
            next,
            1e-3,
            1e-6,
        )
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert!(report.viscous_heat_j > 0.);
    assert!(work.rig_work_j.abs() > 1e-10 && work.obstacle_work_j.abs() > 1e-10);
    let native_work = report.support.surface_work_j - work.rig_work_j - work.obstacle_work_j;
    assert!(native_work.abs() > 1e-10);
    let independent = after.kinetic_j - before.kinetic_j + after.potential_j - before.potential_j
        + report.viscous_heat_j
        - report.support.surface_work_j;
    assert!(independent.abs() <= 1e-6, "defect={independent}");
    assert!((independent - report.total_energy_defect_j).abs() < 1e-9);
    assert!(
        (body.maxwell_sensible_energy_j().unwrap()[0] - thermal_before - report.viscous_heat_j)
            .abs()
            < 1e-9
    );
    assert!(body.maxwell_temperatures_kelvin().unwrap()[0] > 300.);
    assert_eq!(
        expected,
        format!("{:?}", body.body().stationary_embedded_contact().unwrap())
    );
    // A crossing fails after the first relaxation, with every owner rolled back.
    let saved = format!("{body:?}");
    let current = body.body().stationary_embedded_contact().unwrap().clone();
    let crossing = current
        .with_pose(
            REST.to_vec(),
            SKIN.map(|p| [p[0], p[1], p[2] - 0.1]).to_vec(),
            Arc::new(current.obstacle().clone()),
        )
        .unwrap();
    let native = Arc::new(body.prescribed_surface().unwrap().clone());
    let error = body
        .step_viscoelastic_implicit_with_surface_and_skin_motion(None, native, crossing, 1e-3, 1e-6)
        .unwrap_err();
    assert_ne!(error, "viscoelastic relaxation energy defect");
    assert_eq!(saved, format!("{body:?}"));
}

#[test]
fn implicit_maxwell_parallel_native_feature_quadrature_closes_full_step() {
    use std::sync::Arc;
    let mut body = maxwell_skin();
    let original = obstacle();
    let native = Arc::new(
        original
            .with_positions(
                original
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] - 0.015])
                    .collect(),
            )
            .unwrap(),
    );
    body.set_prescribed_surface(Some(native.clone())).unwrap();
    let next_native = Arc::new(
        native
            .with_positions(
                native
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 2e-7])
                    .collect(),
            )
            .unwrap(),
    );
    let start = body.body().stationary_embedded_contact().unwrap().clone();
    let next = start
        .with_pose(
            REST.to_vec(),
            SKIN.map(|p| [p[0], p[1], p[2] + 1e-6]).to_vec(),
            Arc::new(
                start
                    .obstacle()
                    .with_positions(
                        start
                            .obstacle()
                            .positions()
                            .iter()
                            .map(|p| [p[0], p[1], p[2] + 1e-7])
                            .collect(),
                    )
                    .unwrap(),
            ),
        )
        .unwrap();
    let before = body.diagnostics().unwrap();
    let thermal_before = body.maxwell_sensible_energy_j().unwrap()[0];
    let expected = format!("{next:?}");
    let (report, work) = body
        .step_viscoelastic_implicit_with_surface_and_skin_motion(
            None,
            next_native.clone(),
            next,
            1e-3,
            1e-6,
        )
        .unwrap();
    let after = body.diagnostics().unwrap();
    let independent = after.kinetic_j - before.kinetic_j + after.potential_j - before.potential_j
        + report.viscous_heat_j
        - report.support.surface_work_j;
    assert!(independent.abs() <= 1e-6);
    assert!(report.absolute_energy_defect_j <= 1e-6);
    assert!(
        (body.maxwell_sensible_energy_j().unwrap()[0] - thermal_before - report.viscous_heat_j)
            .abs()
            < 1e-9
    );
    assert!(work.rig_work_j.abs() > 1e-10 && work.obstacle_work_j.abs() > 1e-10);
    assert_eq!(
        body.prescribed_surface().unwrap().positions(),
        next_native.positions()
    );
    assert_eq!(
        expected,
        format!("{:?}", body.body().stationary_embedded_contact().unwrap())
    );
    eprintln!(
        "REFINED_PARALLEL_FULL_STEP independent_defect_j={independent:.17e} absolute_defects_j={:.17e}",
        report.absolute_energy_defect_j
    );
}

#[test]
fn parallel_native_terminal_feature_kink_has_distinct_one_sided_derivatives() {
    let start = [[0., 0., 0.], [1., 0., 0.], [0.1, 1., 0.], [0., 0., 1.]];
    let end = [
        [
            1.5988285628445842e-7,
            1.5995303673943793e-7,
            7.998402483563607e-8,
        ],
        [
            1.00000000069816,
            -1.6065113127211587e-7,
            4.839065317682925e-8,
        ],
        [
            0.09999983941879118,
            1.0000000006980625,
            6.102800645147528e-8,
        ],
        [
            1.647786858593217e-13,
            1.5289551000209675e-13,
            1.0000000154309676,
        ],
    ];
    let original = obstacle();
    let current = original
        .with_positions(
            original
                .positions()
                .iter()
                .map(|p| [p[0], p[1], p[2] - 0.015])
                .collect(),
        )
        .unwrap();
    let next = current
        .with_positions(
            current
                .positions()
                .iter()
                .map(|p| [p[0], p[1], p[2] + 2e-7])
                .collect(),
        )
        .unwrap();
    let faces = maxwell_skin().body().surface();
    let response = current.path_response(&next, &start, &end, &faces).unwrap();
    for t in [
        0.046910077030668_f64,
        0.230765344947158,
        0.5,
        0.769234655052842,
        0.953089922969332,
    ] {
        let world: Vec<[f64; 3]> = start
            .iter()
            .zip(end)
            .map(|(a, b)| std::array::from_fn(|k| t.mul_add(b[k] - a[k], a[k])))
            .collect();
        let posed = current
            .with_positions(
                current
                    .positions()
                    .iter()
                    .zip(next.positions())
                    .map(|(a, b)| std::array::from_fn(|k| t.mul_add(b[k] - a[k], a[k])))
                    .collect(),
            )
            .unwrap();
        let r = posed.response(&world, &faces).unwrap();
        let branch_counts: Vec<_> = posed
            .contact_branch_bundles(&world, &faces)
            .unwrap()
            .iter()
            .map(Vec::len)
            .collect();
        eprintln!("PARALLEL_BRANCH_COUNTS t={t} counts={branch_counts:?}");
        let mut lower = world[2][2] - 1e-10;
        let mut upper = world[2][2] + 1e-10;
        let slope_at = |z: f64| {
            let mut trial = world.clone();
            trial[2][2] = z;
            posed.response(&trial, &faces).unwrap().body_gradient_n[2][2]
        };
        assert!(slope_at(lower) < -5. && slope_at(upper) > -5.);
        let mut iterations = 0;
        for _ in 0..96 {
            let middle = lower + 0.5 * (upper - lower);
            if middle == lower || middle == upper {
                break;
            }
            if slope_at(middle) < -5. {
                lower = middle;
            } else {
                upper = middle;
            }
            iterations += 1;
        }
        assert_eq!(lower.next_up(), upper, "event bracket is not adjacent");
        assert!(slope_at(upper) - slope_at(lower) > 3.);
        eprintln!(
            "PARALLEL_EVENT t={t:.17e} lower_z={lower:.17e} upper_z={upper:.17e} width_m={:.17e} lower_slope_n={:.17e} upper_slope_n={:.17e} iterations={iterations}",
            upper - lower,
            slope_at(lower),
            slope_at(upper)
        );
        let mut plus = world.clone();
        let mut minus = world;
        let h = 1e-10;
        plus[2][2] += h;
        minus[2][2] -= h;
        let plus_energy = posed.response(&plus, &faces).unwrap().potential_j;
        let minus_energy = posed.response(&minus, &faces).unwrap().potential_j;
        let forward = (plus_energy - r.potential_j) / h;
        let backward = (r.potential_j - minus_energy) / h;
        let fd = (forward + backward) * 0.5;
        assert!((forward - backward).abs() > 3., "feature kink disappeared");
        assert!(
            r.body_gradient_n[2][2] >= forward.min(backward) - 1e-4
                && r.body_gradient_n[2][2] <= forward.max(backward) + 1e-4
        );
        eprintln!("PARALLEL_ONE_SIDED t={t} forward_n={forward} backward_n={backward}");
        eprintln!(
            "PARALLEL_SAMPLE t={t} fd_n={fd} gradient_n={} error_n={}",
            r.body_gradient_n[2][2],
            fd - r.body_gradient_n[2][2]
        );
    }

    for h in [1e-9, 1e-10, 1e-11, 1e-12] {
        let mut maximum: f64 = 0.;
        let mut worst = (0, 0, 0., 0.);
        for node in 0..4 {
            for axis in 0..3 {
                let mut plus = end;
                let mut minus = end;
                // Search variable is midpoint displacement, endpoint derivative factor 2.
                plus[node][axis] += 2. * h;
                minus[node][axis] -= 2. * h;
                let fd = (current
                    .path_response(&next, &start, &plus, &faces)
                    .unwrap()
                    .midpoint_objective_j
                    - current
                        .path_response(&next, &start, &minus, &faces)
                        .unwrap()
                        .midpoint_objective_j)
                    / (2. * h);
                let error = (fd - response.body_gradient_n[node][axis]).abs();
                if error > maximum {
                    maximum = error;
                    worst = (node, axis, fd, response.body_gradient_n[node][axis]);
                }
            }
        }
        assert!(
            (0.4..0.6).contains(&maximum),
            "known potential/gradient mismatch changed: {maximum}"
        );
        eprintln!("PARALLEL_GRADIENT h={h:.17e} maximum_error_n={maximum:.17e} worst={worst:?}");
    }
}

#[test]
fn same_pose_native_branch_bundles_preserve_selected_response_and_force_balance() {
    let surface = obstacle();
    let points = [[0., 0., 0.015], [0.2, 0., 0.015], [0., 0.2, 0.015]];
    let faces = [[0, 1, 2]];
    let before = format!("{surface:?}");
    let bundles = surface.contact_branch_bundles(&points, &faces).unwrap();
    assert_eq!(bundles.len(), 1);
    assert!(bundles[0].len() >= 3);
    let original = surface.response(&points, &faces).unwrap();
    let selected = &bundles[0][0];
    let (body, obstacle_gradient) = selected.gradients();
    assert_eq!(
        selected.potential_j.to_bits(),
        original.potential_j.to_bits()
    );
    for node in 0..3 {
        for axis in 0..3 {
            assert!((body[node][axis] - original.body_gradient_n[node][axis]).abs() < 1e-14);
            assert!(
                (obstacle_gradient[node][axis] - original.obstacle_gradient_n[node][axis]).abs()
                    < 1e-14
            );
        }
    }
    for branch in &bundles[0] {
        assert_eq!(branch.gap_m.to_bits(), selected.gap_m.to_bits());
        assert_eq!(branch.potential_j.to_bits(), selected.potential_j.to_bits());
        let (g, b) = branch.gradients();
        for axis in 0..3 {
            let resultant: f64 = g.iter().chain(&b).map(|p| p[axis]).sum();
            assert!(resultant.abs() < 1e-13);
        }
        let cross = |p: [f64; 3], f: [f64; 3]| {
            [
                p[1] * f[2] - p[2] * f[1],
                p[2] * f[0] - p[0] * f[2],
                p[0] * f[1] - p[1] * f[0],
            ]
        };
        let mut torque = [0.; 3];
        for (p, f) in points
            .iter()
            .zip(g)
            .chain(surface.positions().iter().zip(b))
        {
            let t = cross(*p, f);
            for axis in 0..3 {
                torque[axis] += t[axis];
            }
        }
        assert!(torque.iter().all(|t| t.abs() < 1e-13));
    }
    // A tilt selects a unique lower vertex; distant features are never bundled.
    let tilted = [[0., 0., 0.015], [0.2, 0., 0.02], [0., 0.2, 0.025]];
    let unique = surface.contact_branch_bundles(&tilted, &faces).unwrap();
    assert_eq!(unique[0].len(), 1);
    assert_eq!(before, format!("{surface:?}"));
    assert!(
        surface
            .contact_branch_bundles(&points, &[[0, 1, 99]])
            .is_err()
    );
}

#[test]
fn adaptive_implicit_maxwell_admits_full_parallel_contact_interval_and_rolls_back_limit() {
    use std::sync::Arc;
    let mut body = maxwell_skin();
    let original = obstacle();
    let native = Arc::new(
        original
            .with_positions(
                original
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] - 0.015])
                    .collect(),
            )
            .unwrap(),
    );
    body.set_prescribed_surface(Some(native.clone())).unwrap();
    let next_native = Arc::new(
        native
            .with_positions(
                native
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 2e-7])
                    .collect(),
            )
            .unwrap(),
    );
    let start = body.body().stationary_embedded_contact().unwrap().clone();
    let next = start
        .with_pose(
            REST.to_vec(),
            SKIN.map(|p| [p[0], p[1], p[2] + 1e-6]).to_vec(),
            Arc::new(
                start
                    .obstacle()
                    .with_positions(
                        start
                            .obstacle()
                            .positions()
                            .iter()
                            .map(|p| [p[0], p[1], p[2] + 1e-7])
                            .collect(),
                    )
                    .unwrap(),
            ),
        )
        .unwrap();
    let saved = format!("{body:?}");
    assert!(
        body.step_viscoelastic_implicit_adaptive_with_surface_and_skin_motion(
            None,
            next_native.clone(),
            next.clone(),
            1e-3,
            1e-30,
            1
        )
        .is_err()
    );
    assert_eq!(saved, format!("{body:?}"));
    for invalid in [0, 3, 512] {
        assert!(
            body.step_viscoelastic_implicit_adaptive_with_surface_and_skin_motion(
                None,
                next_native.clone(),
                next.clone(),
                1e-3,
                1e-6,
                invalid
            )
            .is_err()
        );
        assert_eq!(saved, format!("{body:?}"));
    }
    let before = body.diagnostics().unwrap();
    let thermal_before = body.maxwell_sensible_energy_j().unwrap()[0];
    let expected = format!("{next:?}");
    let result = body
        .step_viscoelastic_implicit_adaptive_with_surface_and_skin_motion(
            None,
            next_native.clone(),
            next,
            1e-3,
            1e-6,
            256,
        )
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert_eq!(result.substeps, 1);
    assert!(result.absolute_energy_defect_j <= 1e-6);
    let independent = after.kinetic_j - before.kinetic_j + after.potential_j - before.potential_j
        + result.step.viscous_heat_j
        - result.step.support.support_work_j
        - result.step.support.surface_work_j;
    assert!(independent.abs() <= 1e-6);
    assert!(result.step.viscous_heat_j > 0.);
    assert!(
        (body.maxwell_sensible_energy_j().unwrap()[0]
            - thermal_before
            - result.step.viscous_heat_j)
            .abs()
            < 1e-9
    );
    assert_eq!(
        body.prescribed_surface().unwrap().positions(),
        next_native.positions()
    );
    assert_eq!(
        expected,
        format!("{:?}", body.body().stationary_embedded_contact().unwrap())
    );
    eprintln!(
        "ADAPTIVE_PARALLEL substeps={} nominal_dt_s=1e-3 independent_defect_j={independent:.17e} absolute_defects_j={:.17e}",
        result.substeps, result.absolute_energy_defect_j
    );
}

fn parallel_contact_trajectory(intervals: usize, adaptive: bool) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
    parallel_contact_trajectory_with_budget(intervals, adaptive, 8e-6).unwrap()
}
fn parallel_contact_trajectory_with_budget(
    intervals: usize,
    adaptive: bool,
    budget: f64,
) -> Result<(Vec<[f64; 3]>, Vec<[f64; 3]>), (usize, &'static str)> {
    use std::sync::Arc;
    let mut body = maxwell_skin();
    let original = obstacle();
    let native = Arc::new(
        original
            .with_positions(
                original
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] - 0.015])
                    .collect(),
            )
            .unwrap(),
    );
    body.set_prescribed_surface(Some(native.clone())).unwrap();
    let skin = body.body().stationary_embedded_contact().unwrap().clone();
    let before = body.diagnostics().unwrap();
    let thermal_before = body.maxwell_sensible_energy_j().unwrap()[0];
    let mut heat = 0.;
    let mut work = 0.;
    let mut absolute = 0.;
    let mut substeps = 0;
    let duration = 0.008;
    let mut kinematic_absolute = 0.;
    for index in 1..=intervals {
        let time = duration * index as f64 / intervals as f64;
        let next_native = Arc::new(
            native
                .with_positions(
                    native
                        .positions()
                        .iter()
                        .map(|p| [p[0], p[1], p[2] + 2e-4 * time])
                        .collect(),
                )
                .unwrap(),
        );
        let next = skin
            .with_pose(
                REST.to_vec(),
                SKIN.map(|p| [p[0], p[1], p[2] + 1e-3 * time]).to_vec(),
                Arc::new(
                    skin.obstacle()
                        .with_positions(
                            skin.obstacle()
                                .positions()
                                .iter()
                                .map(|p| [p[0], p[1], p[2] + 1e-4 * time])
                                .collect(),
                        )
                        .unwrap(),
                ),
            )
            .unwrap();
        let snapshot = (budget < 8e-6).then(|| format!("{body:?}"));
        let old_positions = body.body().positions().to_vec();
        let old_velocities = body.velocities().to_vec();
        let (result, substep_count) = if adaptive {
            let result = body
                .step_viscoelastic_implicit_adaptive_with_surface_and_skin_motion(
                    None,
                    next_native,
                    next,
                    duration / intervals as f64,
                    budget / intervals as f64,
                    256,
                )
                .map_err(|e| {
                    if let Some(before)=snapshot.as_ref() {assert_eq!(*before,format!("{body:?}"));}
                    eprintln!("TRAJECTORY_REJECTION intervals={intervals} index={index} budget_j={budget} accepted_kinematic_absolute_m={kinematic_absolute:.17e} reason={e:?}");
                    (index,e)
                })?;
            (result.step, result.substeps)
        } else {
            let (result, _) = body
                .step_viscoelastic_implicit_with_surface_and_skin_motion(
                    None,
                    next_native,
                    next,
                    duration / intervals as f64,
                    budget / intervals as f64,
                )
                .map_err(|e| {
                    if let Some(before)=snapshot.as_ref() {assert_eq!(*before,format!("{body:?}"));}
                    eprintln!("TRAJECTORY_REJECTION intervals={intervals} index={index} budget_j={budget} accepted_kinematic_absolute_m={kinematic_absolute:.17e} reason={e:?}");
                    (index,e)
                })?;
            (result, 1)
        };
        for node in 0..old_positions.len() {
            let norm = (0..3)
                .map(|axis| {
                    let dx = body.body().positions()[node][axis] - old_positions[node][axis];
                    let midpoint_dx = 0.5
                        * (body.velocities()[node][axis] + old_velocities[node][axis])
                        * duration
                        / intervals as f64;
                    (dx - midpoint_dx).powi(2)
                })
                .sum::<f64>()
                .sqrt();
            kinematic_absolute += norm;
        }
        heat += result.viscous_heat_j;
        work += result.support.support_work_j
            + result.support.plane_work_j
            + result.support.surface_work_j;
        absolute += result.absolute_energy_defect_j;
        substeps += substep_count;
    }
    let after = body.diagnostics().unwrap();
    let independent =
        after.kinetic_j - before.kinetic_j + after.potential_j - before.potential_j + heat - work;
    assert!(independent.abs() <= budget);
    assert!(absolute <= budget);
    assert!((body.maxwell_sensible_energy_j().unwrap()[0] - thermal_before - heat).abs() < 1e-9);
    assert!(heat > 0.);
    eprintln!(
        "CONTACT_TRAJECTORY budget_j={budget:.17e} kinematic_absolute_m={kinematic_absolute:.17e} adaptive={adaptive} intervals={intervals} duration_s={duration} substeps={substeps} independent_defect_j={independent:.17e} absolute_defects_j={absolute:.17e}"
    );
    Ok((body.body().positions().to_vec(), body.velocities().to_vec()))
}
fn trajectory_distance(
    a: &(Vec<[f64; 3]>, Vec<[f64; 3]>),
    b: &(Vec<[f64; 3]>, Vec<[f64; 3]>),
) -> (f64, f64) {
    let norm = |a: &[[f64; 3]], b: &[[f64; 3]]| {
        a.iter()
            .zip(b)
            .flat_map(|(a, b)| (0..3).map(move |k| (a[k] - b[k]).powi(2)))
            .sum::<f64>()
            .sqrt()
    };
    (norm(&a.0, &b.0), norm(&a.1, &b.1))
}
#[test]
fn adaptive_implicit_parallel_contact_trajectory_refines_with_contact_quadrature() {
    let states = [8, 16, 32].map(|n| parallel_contact_trajectory(n, true));
    let coarse = trajectory_distance(&states[0], &states[2]);
    let fine = trajectory_distance(&states[1], &states[2]);
    assert!(fine.0 < coarse.0);
    eprintln!("UPDATED_ADAPTIVE_ERRORS coarse={coarse:?} fine={fine:?}");
    assert!(fine.1 < coarse.1);
}
#[test]
fn fixed_implicit_parallel_contact_trajectory_records_refinement_limit() {
    let states = [512, 1024, 2048, 4096, 8192].map(|n| parallel_contact_trajectory(n, false));
    for index in 0..4 {
        let discrepancy = trajectory_distance(&states[index], &states[4]);
        eprintln!(
            "FIXED_REFERENCE intervals={} reference_intervals=8192 position_m={:.17e} velocity_m_s={:.17e}",
            512 * (1 << index),
            discrepancy.0,
            discrepancy.1
        );
        assert!(discrepancy.0.is_finite() && discrepancy.1.is_finite());
    }
    // Fixed schedules exclude adaptive scheduling as the sole cause.
    assert!(trajectory_distance(&states[1], &states[4]).1.is_finite());
}

#[test]
fn fixed_parallel_contact_stricter_budget_comparison() {
    let original = parallel_contact_trajectory_with_budget(8192, false, 8e-6).unwrap();
    let strict = parallel_contact_trajectory_with_budget(8192, false, 8e-8).unwrap();
    let reference = parallel_contact_trajectory_with_budget(16384, false, 8e-8).unwrap();
    let original_error = trajectory_distance(&original, &reference);
    let strict_error = trajectory_distance(&strict, &reference);
    eprintln!(
        "STRICT_BUDGET_ADMITTED original_error={original_error:?} strict_error={strict_error:?}"
    );
    assert!(strict_error.1.is_finite());
}

#[test]
fn mixed_triangle_contact_preserves_force_chain_rule_and_prescribed_ccd() {
    // Two vertices deliberately outside the tissue volume are skeletal-owned.
    let skin = [SKIN[0], [1.15, 0.1, 0.04], [0.1, 1.15, 0.04]];
    let contact = EmbeddedTriangleContact::new_mixed(
        &REST,
        &[[0, 1, 2, 3]],
        &skin,
        vec![[0, 1, 2]],
        &[true, false, false],
    )
    .unwrap();
    let obstacle = PrescribedTriangleSurface::new(
        vec![[-4., -4., 0.], [4., -4., 0.], [0., 4., 0.]],
        vec![[0, 1, 2]],
        0.001,
        0.03,
        100.,
    )
    .unwrap();
    let response = contact
        .response(pose(&REST, &REST, &skin), &obstacle)
        .unwrap();
    assert!(response.potential_j > 0.);
    assert!(
        response
            .skin_loads
            .nodal_forces_n()
            .iter()
            .flatten()
            .any(|f| f.abs() > 1e-6)
    );
    let h = 1e-7;
    for component in 0..3 {
        for index in 0..if component == 2 { 3 } else { 4 } {
            for axis in 0..3 {
                let mut plus = [REST.to_vec(), REST.to_vec(), skin.to_vec()];
                let mut minus = plus.clone();
                plus[component][index][axis] += h;
                minus[component][index][axis] -= h;
                let energy = |p: &[Vec<[f64; 3]>; 3]| {
                    contact
                        .response(pose(&p[0], &p[1], &p[2]), &obstacle)
                        .unwrap()
                        .potential_j
                };
                let force = match component {
                    0 => response.skin_loads.nodal_forces_n(),
                    1 => response.skin_loads.reference_forces_n(),
                    _ => response.skin_loads.base_forces_n(),
                }[index][axis];
                assert!(
                    ((energy(&plus) - energy(&minus)) / (2. * h) + force).abs()
                        < 1e-6 * (1. + force.abs())
                );
            }
        }
    }
    let nodes = REST.map(|p| [p[0], p[1], p[2] + 0.001]);
    let positions = contact.positions(pose(&nodes, &REST, &skin)).unwrap();
    assert_eq!(positions[1], skin[1]);
    assert_eq!(positions[2], skin[2]);
    assert!((positions[0][2] - skin[0][2] - 0.001).abs() < 1e-14);
    // A prescribed vertex can cross while tissue nodes remain fixed. CCD must
    // inspect that part of a mixed triangle as well as its tissue-owned vertex.
    let mut crossing = skin;
    crossing[1][2] = -0.1;
    assert!(
        !contact
            .path_is_open(
                pose(&REST, &REST, &skin),
                pose(&REST, &REST, &crossing),
                &obstacle,
                &obstacle
            )
            .unwrap()
    );
}

#[test]
fn mixed_skin_uses_existing_implicit_motion_work_and_atomic_rollback() {
    use physics::biomechanics::StationaryEmbeddedContact;
    use std::sync::Arc;
    let skin = [SKIN[0], [1.15, 0.1, 0.04], [0.1, 1.15, 0.04]];
    let obstacle = Arc::new(
        PrescribedTriangleSurface::new(
            vec![[-4., -4., 0.], [4., -4., 0.], [0., 4., 0.]],
            vec![[0, 1, 2]],
            0.001,
            0.03,
            100.,
        )
        .unwrap(),
    );
    let start = StationaryEmbeddedContact::new(
        Arc::new(
            EmbeddedTriangleContact::new_mixed(
                &REST,
                &[[0, 1, 2, 3]],
                &skin,
                vec![[0, 1, 2]],
                &[true, false, false],
            )
            .unwrap(),
        ),
        REST.to_vec(),
        skin.to_vec(),
        obstacle.clone(),
    )
    .unwrap();
    let far = Arc::new(
        obstacle
            .with_positions(
                obstacle
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 10.])
                    .collect(),
            )
            .unwrap(),
    );
    let mut body = inertial_skin([0.; 3]);
    body.set_prescribed_surface(Some(far.clone())).unwrap();
    body.set_stationary_embedded_contact(Some(start.clone()))
        .unwrap();
    let next_base = skin.map(|p| [p[0], p[1], p[2] + 1e-5]);
    let next = start
        .with_pose(
            REST.map(|p| [p[0], p[1], p[2] + 4e-6]).to_vec(),
            next_base.to_vec(),
            Arc::new(
                obstacle
                    .with_positions(
                        obstacle
                            .positions()
                            .iter()
                            .map(|p| [p[0], p[1], p[2] + 2e-6])
                            .collect(),
                    )
                    .unwrap(),
            ),
        )
        .unwrap();
    let before = body.diagnostics().unwrap();
    let receipt = body
        .step_implicit_with_surface_and_skin_motion(None, far.clone(), next, 1e-3, 1e-8)
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert!(receipt.surface_work_j.abs() > 1e-9);
    let independent = after.kinetic_j - before.kinetic_j + after.potential_j
        - before.potential_j
        - receipt.surface_work_j;
    assert!(independent.abs() <= 1e-8, "independent={independent}");
    assert!(body.velocities().iter().flatten().any(|v| v.abs() > 1e-9));
    let saved = format!("{body:?}");
    let current = body.body().stationary_embedded_contact().unwrap();
    let mut crossing = next_base;
    crossing[1][2] = -0.1;
    let crossing = current
        .with_pose(
            REST.to_vec(),
            crossing.to_vec(),
            Arc::new(current.obstacle().clone()),
        )
        .unwrap();
    assert!(
        body.step_implicit_with_surface_and_skin_motion(None, far, crossing, 1e-3, 1e-8)
            .is_err()
    );
    assert_eq!(saved, format!("{body:?}"));
}

#[test]
fn shared_skin_triangle_couples_two_dynamic_fem_regions_in_existing_solver() {
    use physics::biomechanics::{Body, InertialBody, Material, StationaryEmbeddedContact};
    use std::sync::Arc;
    let rest: Vec<_> = REST
        .into_iter()
        .chain(REST.map(|p| [p[0] + 2., p[1], p[2]]))
        .collect();
    let cells = [[0, 1, 2, 3], [4, 5, 6, 7]];
    let skin = [[0.1, 0.1, 0.015], [2.1, 0.1, 0.018], [1., 2., 0.04]];
    let contact = Arc::new(
        EmbeddedTriangleContact::new_mixed(
            &rest,
            &cells,
            &skin,
            vec![[0, 1, 2]],
            &[true, true, false],
        )
        .unwrap(),
    );
    // Unique edge/vertex feature: the contact point lies between the two
    // dynamic owners. Both receive the same interaction through W^T.
    let obstacle = Arc::new(
        PrescribedTriangleSurface::new(
            vec![[0.95, -0.1, 0.], [1.25, -0.1, 0.], [1.1, 0.09, 0.]],
            vec![[0, 1, 2]],
            0.001,
            0.05,
            100.,
        )
        .unwrap(),
    );
    let response = contact
        .response(pose(&rest, &rest, &skin), &obstacle)
        .unwrap();
    assert!(
        !contact
            .path_is_open(
                pose(&rest, &rest, &skin),
                pose(&rest, &rest, &[skin[0], skin[1], [1.1, 0., -0.1]]),
                &obstacle,
                &obstacle,
            )
            .unwrap()
    );
    let force = response.skin_loads.nodal_forces_n();
    for range in [0..4, 4..8] {
        assert!(force[range].iter().flatten().any(|f| f.abs() > 1e-6));
    }
    // A direction in region zero produces metric action in region one.
    // Independent region solves would omit this off-diagonal coupling.
    let mut direction = vec![[0.; 3]; 8];
    direction[0][2] = 1.;
    let action = contact
        .normal_metric_apply(pose(&rest, &rest, &skin), &obstacle, &direction)
        .unwrap();
    assert!(action[4..].iter().flatten().any(|f| f.abs() > 1e-6));
    let h = 1e-7;
    for node in 0..8 {
        for axis in 0..3 {
            let mut plus = rest.clone();
            let mut minus = rest.clone();
            plus[node][axis] += h;
            minus[node][axis] -= h;
            let ep = contact
                .response(pose(&plus, &rest, &skin), &obstacle)
                .unwrap()
                .potential_j;
            let em = contact
                .response(pose(&minus, &rest, &skin), &obstacle)
                .unwrap()
                .potential_j;
            assert!(
                ((ep - em) / (2. * h) + force[node][axis]).abs()
                    < 1e-6 * (1. + force[node][axis].abs())
            );
        }
    }
    let parts: Vec<_> = (0..2)
        .map(|i| {
            let solid = Body::new(
                rest[i * 4..i * 4 + 4].to_vec(),
                vec![false; 4],
                vec![(
                    [0, 1, 2, 3],
                    Material {
                        shear_pa: 500. + 500. * i as f64,
                        bulk_pa: 1000.,
                        fibers: vec![],
                    },
                )],
            )
            .unwrap();
            InertialBody::new(solid, &[1000. + 200. * i as f64], vec![[0.; 3]; 4]).unwrap()
        })
        .collect();
    let assembly = InertialBody::assemble_tissues(&parts).unwrap();
    assert_eq!(assembly.node_ranges, vec![0..4, 4..8]);
    let mut body = assembly.body;
    let far = Arc::new(
        obstacle
            .with_positions(
                obstacle
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 10.])
                    .collect(),
            )
            .unwrap(),
    );
    body.set_prescribed_surface(Some(far.clone())).unwrap();
    let start =
        StationaryEmbeddedContact::new(contact, rest.clone(), skin.to_vec(), obstacle.clone())
            .unwrap();
    body.set_stationary_embedded_contact(Some(start.clone()))
        .unwrap();
    let reference: Vec<_> = rest
        .iter()
        .enumerate()
        .map(|(i, p)| [p[0], p[1], p[2] + if i < 4 { 4e-6 } else { -3e-6 }])
        .collect();
    let base = skin
        .into_iter()
        .zip([1e-5, 8e-6, 2e-5])
        .map(|(p, d)| [p[0], p[1], p[2] + d])
        .collect();
    let next = start.with_pose(reference, base, obstacle).unwrap();
    let before = body.diagnostics().unwrap();
    let receipt = body
        .step_implicit_with_surface_and_skin_motion(None, far.clone(), next, 1e-3, 1e-8)
        .unwrap();
    let after = body.diagnostics().unwrap();
    let independent = after.kinetic_j - before.kinetic_j + after.potential_j
        - before.potential_j
        - receipt.surface_work_j;
    assert!(independent.abs() <= 1e-8, "independent={independent}");
    for range in [0..4, 4..8] {
        assert!(
            body.velocities()[range]
                .iter()
                .flatten()
                .any(|v| v.abs() > 1e-9)
        );
    }
    let saved = format!("{body:?}");
    let current = body.body().stationary_embedded_contact().unwrap();
    // The prescribed third vertex passes through the obstacle; the successful
    // first interval must not authorize a partially committed second interval.
    let crossing = current
        .with_pose(
            rest,
            vec![skin[0], skin[1], [1.1, 0., -0.1]],
            Arc::new(current.obstacle().clone()),
        )
        .unwrap();
    let error = body
        .step_implicit_with_surface_and_skin_motion(None, far, crossing, 1e-3, 1e-8)
        .unwrap_err();
    assert_eq!(error, "implicit initial contact trajectory inadmissible");
    assert_eq!(saved, format!("{body:?}"));
    println!(
        "MULTI_REGION_SKIN_COUPLING independent_defect_j={independent:.17e} off_diagonal_max={:.17e}",
        action[4..]
            .iter()
            .flatten()
            .fold(0.0_f64, |m, x| m.max(x.abs()))
    );
}

#[test]
fn embedded_nearest_pair_reports_closed_gap_without_changing_contact() {
    use physics::biomechanics::StationaryEmbeddedContact;
    use std::sync::Arc;
    let contact = StationaryEmbeddedContact::new(
        Arc::new(contact()),
        REST.to_vec(),
        SKIN.to_vec(),
        Arc::new(obstacle()),
    )
    .unwrap();
    let initial = contact.nearest_active_contact(&REST).unwrap().unwrap();
    assert_eq!(initial.body_face, [0, 1, 2]);
    assert_eq!(initial.obstacle_face_index, 0);
    assert!((initial.distance_m - 0.015).abs() < 1e-12);
    let mut nodes = REST;
    for node in &mut nodes {
        node[2] -= 0.0145;
    }
    let before = format!("{contact:?}");
    let closed = contact.nearest_active_contact(&nodes).unwrap().unwrap();
    assert!(closed.gap_m < 0.);
    assert!((closed.distance_m - 0.0005).abs() < 1e-12);
    assert_eq!(
        contact.response(&nodes).unwrap_err(),
        "closed surface contact gap"
    );
    assert_eq!(before, format!("{contact:?}"));
    assert_eq!(
        contact
            .nearest_active_contact(&REST)
            .unwrap()
            .unwrap()
            .gap_m
            .to_bits(),
        initial.gap_m.to_bits()
    );
}

#[test]
fn adaptive_closed_trial_exhaustion_preserves_maxwell_thermal_state() {
    use std::sync::Arc;
    let mut body = maxwell_skin();
    let native = Arc::new(
        obstacle()
            .with_positions(
                obstacle()
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] - 10.])
                    .collect(),
            )
            .unwrap(),
    );
    body.set_prescribed_surface(Some(native.clone())).unwrap();
    let start = body.body().stationary_embedded_contact().unwrap().clone();
    let next = start
        .with_pose(
            REST.to_vec(),
            SKIN.map(|p| [p[0], p[1], p[2] - 0.0145]).to_vec(),
            Arc::new(start.obstacle().clone()),
        )
        .unwrap();
    assert!(
        start
            .nearest_active_contact(body.body().positions())
            .unwrap()
            .unwrap()
            .gap_m
            > 0.
    );
    assert!(
        next.nearest_active_contact(body.body().positions())
            .unwrap()
            .unwrap()
            .gap_m
            < 0.
    );
    let before = format!("{body:?}");
    for max_substeps in [1, 2] {
        assert!(
            body.step_viscoelastic_implicit_adaptive_with_surface_and_skin_motion(
                None,
                native.clone(),
                next.clone(),
                1e-8,
                1e-6,
                max_substeps,
            )
            .is_err()
        );
        assert_eq!(before, format!("{body:?}"));
    }
}

#[test]
fn prescribed_vertex_obstruction_is_distinct_from_a_movable_closed_trial() {
    use physics::biomechanics::StationaryEmbeddedContact;
    use std::sync::Arc;
    let skin = [SKIN[1], [-0.2, 0.1, 0.015], [0.1, -0.2, 0.015]];
    let start = StationaryEmbeddedContact::new(
        Arc::new(
            EmbeddedTriangleContact::new_mixed(
                &REST,
                &[[0, 1, 2, 3]],
                &skin,
                vec![[0, 1, 2]],
                &[true, false, false],
            )
            .unwrap(),
        ),
        REST.to_vec(),
        skin.to_vec(),
        Arc::new(obstacle()),
    )
    .unwrap();
    let next = start
        .with_pose(
            REST.to_vec(),
            skin.map(|p| [p[0], p[1], p[2] - 0.0145]).to_vec(),
            Arc::new(start.obstacle().clone()),
        )
        .unwrap();
    let obstruction = next.prescribed_contact_obstruction(&REST).unwrap().unwrap();
    assert_eq!(obstruction.body_weights[0], 0.);
    assert!(obstruction.gap_m < 0.);
    // Arbitrary tissue translations cannot move either prescribed edge vertex.
    for offset in [0.01, 1., 100.] {
        let nodes = REST.map(|p| [p[0], p[1], p[2] + offset]);
        assert!(next.response(&nodes).is_err());
    }
    let excluded = next.with_pose(
        REST.to_vec(),
        skin.map(|p| [p[0], p[1], p[2] - 0.0145]).to_vec(),
        Arc::new(
            start
                .obstacle()
                .with_body_contact_domains(vec![(vec![[0, 1, 2]], vec![false])])
                .unwrap(),
        ),
    );
    assert!(excluded.is_err()); // Domain changes cannot be smuggled through a pose.
    let fully_dynamic = stationary();
    let nodes = REST.map(|p| [p[0], p[1], p[2] - 0.0145]);
    assert!(fully_dynamic.response(&nodes).is_err());
    assert!(
        fully_dynamic
            .prescribed_contact_obstruction(&nodes)
            .unwrap()
            .is_none()
    );

    let mut body = maxwell_skin();
    body.set_stationary_embedded_contact(Some(start)).unwrap();
    let native = Arc::new(
        obstacle()
            .with_positions(
                obstacle()
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] - 10.])
                    .collect(),
            )
            .unwrap(),
    );
    body.set_prescribed_surface(Some(native.clone())).unwrap();
    let before = format!("{body:?}");
    assert_eq!(
        body.step_viscoelastic_implicit_adaptive_with_surface_and_skin_motion(
            None, native, next, 1e-3, 1e-6, 256,
        )
        .unwrap_err(),
        "prescribed skin obstacle gap is closed"
    );
    assert_eq!(before, format!("{body:?}"));
}
