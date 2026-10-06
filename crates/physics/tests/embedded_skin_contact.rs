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
