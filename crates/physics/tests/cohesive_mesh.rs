use physics::{cohesive, plasticity::mesh::Body};
#[path = "support/cohesive.rs"]
mod support;
fn fixture(n: usize) -> (Body, Vec<[Option<f64>; 3]>, Vec<usize>) {
    support::fixture(n, cohesive::Material::new(1e9, 1e10, 1e5, 10.).unwrap())
}
fn close(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() < tolerance, "{a} != {b}");
}
#[test]
fn displacement_loading_fracture_work_and_reclosure_are_mesh_independent() {
    for n in [1, 2] {
        let (mut body, mut prescribed, end) = fixture(n);
        let loads = vec![[0.; 3]; body.positions().len()];
        let mut work = 0.;
        let mut previous_force = 0.;
        let mut previous_displacement = 0.;
        for step in 0..=40 {
            let gap = f64::from(step) * 0.0002 / 40.;
            let traction = if step <= 20 {
                1e9 * gap
            } else {
                1e5 * (0.0002 - gap) / 0.0001
            };
            let displacement = gap + 2. * 0.1 * traction / 1e9;
            for &i in &end {
                prescribed[i][0] = Some(displacement);
            }
            let result = body.equilibrate(&loads, &prescribed, 50, 1e-6).unwrap();
            assert!(result.converged, "n={n}, step={step}: {result:?}");
            let force: f64 = end.iter().map(|&i| result.reactions_n[i][0]).sum();
            close(force, traction * 0.01, 1e-5);
            work += 0.5 * (force + previous_force) * (displacement - previous_displacement);
            previous_force = force;
            previous_displacement = displacement;
            let reports = body.interface_reports().unwrap();
            let dissipated: f64 = reports.iter().map(|r| r.dissipated_j).sum();
            let stored_interface: f64 = reports.iter().map(|r| r.stored_j).sum();
            let bulk_energy = traction * traction / (2. * 1e9) * 0.002;
            close(work, dissipated + stored_interface + bulk_energy, 1e-9);
        }
        close(work, 0.1, 1e-9);
        assert!(
            body.interface_reports()
                .unwrap()
                .iter()
                .all(|r| r.quadrature.iter().all(|q| q.damage > 1. - 1e-10))
        );
        let history = body.interface_states();
        for &i in &end {
            prescribed[i][0] = Some(-1e-5);
        }
        let closed = body.equilibrate(&loads, &prescribed, 50, 1e-6).unwrap();
        assert!(closed.converged, "{closed:?}");
        let expected_gap = -1e-5 / (1. + 2. * 0.1 * 1e10 / 1e9);
        let force: f64 = end.iter().map(|&i| closed.reactions_n[i][0]).sum();
        close(force, 1e10 * expected_gap * 0.01, 1e-5);
        assert_eq!(body.interface_states(), history);
        for report in body.interface_reports().unwrap() {
            for q in report.quadrature {
                assert!(q.traction_pa[0] < 0.);
            }
        }
    }
}
#[test]
fn invalid_interface_and_failed_equilibrium_preserve_geometry_and_history() {
    let (mut body, mut prescribed, end) = fixture(1);
    let report = body.interface_reports().unwrap()[0].clone();
    let material = cohesive::Material::new(1e9, 1e10, 1e5, 10.).unwrap();
    assert!(
        body.add_interface(report.minus, report.plus, material)
            .is_err()
    );
    let positions = body.positions().to_vec();
    let states = body.states();
    let interfaces = body.interface_states();
    for &i in &end {
        prescribed[i][0] = Some(0.00013);
    }
    let loads = vec![[0.; 3]; body.positions().len()];
    let result = body.equilibrate(&loads, &prescribed, 1, 1e-6).unwrap();
    assert!(!result.converged);
    assert_eq!(body.positions(), positions);
    assert_eq!(body.states(), states);
    assert_eq!(body.interface_states(), interfaces);
    prescribed[0][0] = Some(f64::NAN);
    assert!(body.equilibrate(&loads, &prescribed, 50, 1e-6).is_err());
    assert_eq!(body.positions(), positions);
    assert_eq!(body.interface_states(), interfaces);
}

#[test]
fn nonuniform_opening_uses_independent_quadrature_history_and_consistent_nodal_forces() {
    let (template, _, _) = fixture(1);
    let mut prescribed: Vec<_> = template
        .positions()
        .iter()
        .enumerate()
        .map(|(index, p)| {
            [
                Some(if index >= 8 {
                    0.00011 + 0.0002 * p[1] + 0.0003 * p[2]
                } else {
                    0.
                }),
                Some(0.),
                Some(0.),
            ]
        })
        .collect();
    let loads = vec![[0.; 3]; 16];
    let evaluate = |constraints: &[[Option<f64>; 3]]| {
        let mut body = template.clone();
        let equilibrium = body.equilibrate(&loads, constraints, 50, 1e-6).unwrap();
        assert!(equilibrium.converged);
        let bulk: f64 = body
            .responses()
            .unwrap()
            .iter()
            .map(|r| r.elastic_energy_j_m3 * (0.001 / 6.))
            .sum();
        let interfaces: f64 = body
            .interface_reports()
            .unwrap()
            .iter()
            .map(|r| r.stored_j + r.dissipated_j)
            .sum();
        (body, equilibrium, bulk + interfaces)
    };
    let (body, equilibrium, _) = evaluate(&prescribed);
    let report = body.interface_reports().unwrap()[0].clone();
    assert!((report.quadrature[0].damage - report.quadrature[1].damage).abs() > 0.01);
    for axis in 0..3 {
        let total: f64 = equilibrium.reactions_n.iter().map(|r| r[axis]).sum();
        close(total, 0., 1e-7);
    }
    for (node, axis) in [(8, 0), (10, 0), (8, 1)] {
        let h = 1e-9;
        let original = prescribed[node][axis].unwrap();
        prescribed[node][axis] = Some(original + h);
        let upper = evaluate(&prescribed).2;
        prescribed[node][axis] = Some(original - h);
        let lower = evaluate(&prescribed).2;
        prescribed[node][axis] = Some(original);
        close(
            (upper - lower) / (2. * h),
            equilibrium.reactions_n[node][axis],
            0.0001,
        );
    }
}
