use physics::{
    cohesive::Material as Bond,
    plasticity::{
        Material,
        mesh::{QuadraticBody, QuadraticDynamics},
    },
};
fn coupon() -> (QuadraticBody, Vec<bool>, [usize; 6], [usize; 6]) {
    let material = Material::new(1e7, 0.3, 1e9, 0.).unwrap();
    let mut body = QuadraticBody::from_linear(
        vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., -1.],
        ],
        vec![([0, 1, 2, 3], material), ([4, 5, 6, 7], material)],
    )
    .unwrap();
    let edges = body.edge_midpoints();
    let midpoint = |a: usize, b: usize| {
        edges
            .iter()
            .find(|(edge, _)| *edge == [a.min(b), a.max(b)])
            .unwrap()
            .1
    };
    let minus = [4, 5, 6, midpoint(4, 5), midpoint(5, 6), midpoint(4, 6)];
    let plus = [0, 1, 2, midpoint(0, 1), midpoint(1, 2), midpoint(0, 2)];
    let mut upper = vec![false; body.positions().len()];
    for value in &mut upper[..4] {
        *value = true;
    }
    for (edge, node) in edges {
        upper[node] = edge[0] < 4 && edge[1] < 4;
    }
    body.add_cohesive_interface(minus, plus, Bond::new(1e6, 2e6, 1000., 10.).unwrap())
        .unwrap();
    (body, upper, minus, plus)
}
#[test]
fn prescribed_fracture_commits_energy_and_exposes_both_sides_without_healing() {
    let (mut body, upper, minus, plus) = coupon();
    let count = body.positions().len();
    assert_eq!(body.reference_faces().unwrap().len(), 8);
    assert_eq!(body.exposed_faces_at(body.positions()).unwrap().len(), 6);
    assert!(
        body.add_cohesive_interface(minus, plus, Bond::new(1e6, 2e6, 1000., 10.).unwrap())
            .is_err()
    );
    let reverse = |n: [usize; 6]| [n[0], n[2], n[1], n[5], n[4], n[3]];
    assert_eq!(
        body.add_cohesive_interface(
            reverse(minus),
            reverse(plus),
            Bond::new(1e6, 2e6, 1000., 10.).unwrap()
        )
        .unwrap_err(),
        "quadratic cohesive winding must point outward"
    );
    let constraints: Vec<_> = upper
        .iter()
        .map(|u| [Some(0.), Some(0.), Some(if *u { 0.02 } else { 0. })])
        .collect();
    assert!(
        body.equilibrate(&vec![[0.; 3]; count], &constraints, 1, 1e-9)
            .unwrap()
            .converged
    );
    let reports = body.cohesive_trials_at(body.positions()).unwrap();
    assert!((reports[0].dissipated_j - 5.).abs() < 1e-12);
    assert_eq!(body.exposed_faces_at(body.positions()).unwrap().len(), 8);
    assert!(
        body.equilibrate(&vec![[0.; 3]; count], &vec![[Some(0.); 3]; count], 1, 1e-9)
            .unwrap()
            .converged
    );
    assert!(
        (body.cohesive_trials_at(body.positions()).unwrap()[0].dissipated_j - 5.).abs() < 1e-12
    );
}
#[test]
fn cohesive_newton_tangent_equilibrates_and_failed_solve_preserves_histories() {
    let (mut body, _, _, plus) = coupon();
    let count = body.positions().len();
    let node = plus[3];
    let mut loads = vec![[0.; 3]; count];
    loads[node][2] = 1.;
    let mut constraints = vec![[Some(0.); 3]; count];
    constraints[node][2] = None;
    let report = body.equilibrate(&loads, &constraints, 8, 1e-7).unwrap();
    assert!(report.converged && report.residual_n < 1e-7);
    let positions = body.positions().to_vec();
    let states = body.cohesive_interfaces()[0].states();
    let plastic = body.states();
    loads[node][2] = 1e5;
    assert!(
        !body
            .equilibrate(&loads, &constraints, 1, 1e-7)
            .unwrap()
            .converged
    );
    assert_eq!(body.positions(), positions);
    assert_eq!(body.cohesive_interfaces()[0].states(), states);
    assert_eq!(body.states(), plastic);
}
#[test]
fn dynamic_energy_guard_rejects_peak_skipping_fracture_and_rolls_back() {
    let (body, upper, _, _) = coupon();
    let initial = body.positions().to_vec();
    let velocities = upper
        .iter()
        .map(|u| [0., 0., if *u { 0.5 } else { -0.5 }])
        .collect();
    let mut dynamic = QuadraticDynamics::new(body, &[1000.; 2], velocities).unwrap();
    let before = dynamic.velocities().to_vec();
    assert!(dynamic.step(0.1, [0.; 3], 1e-8).is_err());
    assert_eq!(dynamic.body().positions(), initial);
    assert_eq!(dynamic.velocities(), before);
    assert!(
        dynamic.body().cohesive_interfaces()[0]
            .states()
            .iter()
            .all(|s| s.maximum_separation_m() == 0.)
    );
    assert_eq!(dynamic.energy().unwrap().fracture_dissipated_j, 0.);
}

fn finite_fracture_run(dt: f64, steps: usize) -> f64 {
    use physics::{biomechanics::Material as Elastic, plasticity::mesh::FiniteQuadraticDynamics};
    let (body, upper, _, _) = coupon();
    let count = body.positions().len();
    let velocities = upper
        .iter()
        .map(|u| [0., 0., if *u { 0.5 } else { -0.5 }])
        .collect();
    let material = Elastic::from_young_poisson(1e7, 0.3).unwrap();
    let mut dynamic = FiniteQuadraticDynamics::new(
        body,
        vec![material; 2],
        &[1000.; 2],
        velocities,
        &vec![false; count],
    )
    .unwrap();
    let initial = dynamic.energy().unwrap();
    let mut worst = 0_f64;
    for _ in 0..steps {
        dynamic.step(dt, [0.; 3], 1e-4).unwrap();
        let e = dynamic.energy().unwrap();
        worst = worst.max(
            (e.kinetic_j + e.elastic_j + e.cohesive_stored_j + e.fracture_dissipated_j
                - initial.kinetic_j)
                .abs(),
        );
        for axis in 0..3 {
            assert!((e.momentum_kg_m_s[axis] - initial.momentum_kg_m_s[axis]).abs() < 1e-7);
            assert!(
                (e.angular_momentum_kg_m2_s[axis] - initial.angular_momentum_kg_m2_s[axis]).abs()
                    < 1e-7
            );
        }
    }
    let final_energy = dynamic.energy().unwrap();
    let fragments = dynamic.fragments().unwrap();
    assert_eq!(fragments.len(), 2);
    assert!(
        (fragments.iter().map(|f| f.kinetic_j).sum::<f64>() - final_energy.kinetic_j).abs() < 1e-9
    );
    println!(
        "finite T10 fracture energy envelope={worst:e}, fracture work={:e}",
        final_energy.fracture_dissipated_j
    );
    assert!((final_energy.fracture_dissipated_j - 5.).abs() < 1e-10);
    assert!(worst < 1e-3);
    worst
}
#[test]
fn finite_dynamic_fracture_commits_gc_area_and_conserves_momenta() {
    let coarse = finite_fracture_run(2e-5, 1500);
    let fine = finite_fracture_run(1e-5, 3000);
    assert!(fine < coarse / 2.);
}

#[test]
fn fragment_connectivity_and_consistent_mass_diagnostics_survive_fracture_and_closure() {
    let (mut body, upper, _, _) = coupon();
    let count = body.positions().len();
    assert_eq!(body.fragment_nodes().len(), 1);
    for (opening, expected_count) in [(0.005, 1), (0.02, 2), (0., 2)] {
        let prescribed: Vec<_> = upper
            .iter()
            .map(|u| [Some(0.), Some(0.), Some(if *u { opening } else { 0. })])
            .collect();
        assert!(
            body.equilibrate(&vec![[0.; 3]; count], &prescribed, 1, 1e-8)
                .unwrap()
                .converged
        );
        assert_eq!(body.fragment_nodes().len(), expected_count);
    }
    let velocities: Vec<_> = body
        .positions()
        .iter()
        .zip(&upper)
        .map(|(p, u)| {
            let translation = if *u { [1., 0., 0.] } else { [-0.5, 0.2, 0.] };
            [
                translation[0] - 2. * (p[1] - 0.25),
                translation[1] + 2. * (p[0] - 0.25),
                0.,
            ]
        })
        .collect();
    let dynamic = QuadraticDynamics::new(body, &[1000., 2000.], velocities).unwrap();
    let fragments = dynamic.fragments().unwrap();
    assert_eq!(fragments.len(), 2);
    let energy = dynamic.energy().unwrap();
    let mass: f64 = fragments.iter().map(|f| f.mass_kg).sum();
    let kinetic: f64 = fragments.iter().map(|f| f.kinetic_j).sum();
    assert!((mass - energy.mass_kg).abs() < 1e-9 && (kinetic - energy.kinetic_j).abs() < 1e-9);
    for axis in 0..3 {
        let momentum: f64 = fragments.iter().map(|f| f.momentum_kg_m_s[axis]).sum();
        let angular: f64 = fragments
            .iter()
            .map(|f| f.angular_momentum_kg_m2_s[axis])
            .sum();
        assert!((momentum - energy.momentum_kg_m_s[axis]).abs() < 1e-9);
        assert!((angular - energy.angular_momentum_kg_m2_s[axis]).abs() < 1e-9);
    }
    for fragment in fragments {
        let is_upper = upper[fragment.nodes[0]];
        let expected_mass = if is_upper { 1000. / 6. } else { 2000. / 6. };
        let translation = if is_upper {
            [1., 0., 0.]
        } else {
            [-0.5, 0.2, 0.]
        };
        let center = [0.25, 0.25, if is_upper { 0.25 } else { -0.25 }];
        assert!((fragment.mass_kg - expected_mass).abs() < 1e-9);
        for axis in 0..3 {
            assert!((fragment.center_m[axis] - center[axis]).abs() < 1e-12);
            assert!((fragment.velocity_m_s[axis] - translation[axis]).abs() < 1e-12);
        }
        // A unit tetrahedron has centroidal Izz=0.075*m; omega=2 rad/s.
        let translation_ke = 0.5
            * expected_mass
            * (translation[0] * translation[0] + translation[1] * translation[1]);
        assert!((fragment.kinetic_j - translation_ke - 0.15 * expected_mass).abs() < 1e-9);
        let orbital =
            center[0] * fragment.momentum_kg_m_s[1] - center[1] * fragment.momentum_kg_m_s[0];
        assert!(
            (fragment.angular_momentum_kg_m2_s[2] - orbital - 0.15 * expected_mass).abs() < 1e-9
        );
    }
    let quadratic_velocities: Vec<_> = dynamic
        .body()
        .positions()
        .iter()
        .map(|p| [p[0] * p[0], 0., 0.])
        .collect();
    let quadratic = QuadraticDynamics::new(
        dynamic.body().clone(),
        &[1000., 2000.],
        quadratic_velocities,
    )
    .unwrap();
    for fragment in quadratic.fragments().unwrap() {
        // Uniform tetrahedron barycentric moments: E[x²]=1/10, E[x⁴]=1/35.
        // Row-sum mass lumping gives negative kinetic energy for this mode.
        assert!((fragment.momentum_kg_m_s[0] - fragment.mass_kg / 10.).abs() < 1e-9);
        assert!((fragment.kinetic_j - fragment.mass_kg / 70.).abs() < 1e-9);
    }
}

#[test]
fn automatic_quadratic_interfaces_preserve_source_loads_and_fracture_topology() {
    let material = Material::new(1e7, 0.3, 1e9, 0.).unwrap();
    let corners = [[0, 1, 2, 3], [0, 2, 1, 4]];
    let source = QuadraticBody::from_linear(
        vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0., 0., -1.],
        ],
        corners.iter().map(|&n| (n, material)).collect(),
    )
    .unwrap();
    let edges = source.edge_midpoints();
    let cells: Vec<_> = corners
        .iter()
        .map(|c| {
            let mut n = [0; 10];
            n[..4].copy_from_slice(c);
            for (k, (a, b)) in [(0, 1), (1, 2), (0, 2), (0, 3), (1, 3), (2, 3)]
                .into_iter()
                .enumerate()
            {
                n[4 + k] = edges
                    .iter()
                    .find(|(pair, _)| *pair == [c[a].min(c[b]), c[a].max(c[b])])
                    .unwrap()
                    .1;
            }
            (n, material)
        })
        .collect();
    let mut mesh = QuadraticBody::with_cohesive_faces(
        source.positions(),
        &cells,
        Bond::new(1e6, 2e6, 1000., 10.).unwrap(),
    )
    .unwrap();
    assert_eq!(mesh.body.positions().len(), 20);
    assert_eq!(mesh.body.cohesive_interfaces().len(), 1);
    assert_eq!(mesh.body.fragment_nodes().len(), 1);
    assert_eq!(
        mesh.body
            .exposed_faces_at(mesh.body.positions())
            .unwrap()
            .len(),
        6
    );
    let forces: Vec<_> = source
        .positions()
        .iter()
        .map(|p| [p[0] + 1., p[1] - 2., p[2] + 3.])
        .collect();
    let split = mesh.split_nodal_forces(&forces).unwrap();
    for axis in 0..3 {
        assert!(
            (split.iter().map(|f| f[axis]).sum::<f64>()
                - forces.iter().map(|f| f[axis]).sum::<f64>())
            .abs()
                < 1e-12
        );
    }
    let moment = |positions: &[[f64; 3]], loads: &[[f64; 3]]| {
        let mut total = [0.; 3];
        for (p, f) in positions.iter().zip(loads) {
            for axis in 0..3 {
                let a = (axis + 1) % 3;
                let b = (axis + 2) % 3;
                total[axis] += p[a] * f[b] - p[b] * f[a];
            }
        }
        total
    };
    let original = moment(source.positions(), &forces);
    let duplicated = moment(mesh.body.positions(), &split);
    for axis in 0..3 {
        assert!((original[axis] - duplicated[axis]).abs() < 1e-12);
    }
    let mass = |matrix: Vec<Vec<f64>>| matrix.iter().flatten().sum::<f64>();
    assert!(
        (mass(source.consistent_mass(&[1000., 2000.]).unwrap())
            - mass(mesh.body.consistent_mass(&[1000., 2000.]).unwrap()))
        .abs()
            < 1e-9
    );
    let source_constraints = vec![[Some(0.), None, Some(0.)]; source.positions().len()];
    let expanded = mesh.expand_constraints(&source_constraints).unwrap();
    assert_eq!(expanded, vec![[Some(0.), None, Some(0.)]; 20]);
    let mut prescribed = vec![[Some(0.); 3]; 20];
    for &node in &mesh.cell_nodes[0] {
        prescribed[node][2] = Some(0.02);
    }
    assert!(
        mesh.body
            .equilibrate(&vec![[0.; 3]; 20], &prescribed, 1, 1e-9)
            .unwrap()
            .converged
    );
    assert_eq!(mesh.body.fragment_nodes().len(), 2);
    assert!(
        (mesh.body.cohesive_trials_at(mesh.body.positions()).unwrap()[0].dissipated_j - 5.).abs()
            < 1e-12
    );
    assert!(mesh.split_nodal_forces(&[]).is_err());
    assert!(mesh.expand_constraints(&[]).is_err());
    // A third owner of the same face is a nonmanifold source, rejected atomically.
    let mut invalid = cells.clone();
    invalid.push(cells[0]);
    assert!(
        QuadraticBody::with_cohesive_faces(
            source.positions(),
            &invalid,
            Bond::new(1e6, 2e6, 1000., 10.).unwrap()
        )
        .is_err()
    );
}

#[test]
fn linear_entrypoint_bond_compliance_and_load_mapping_match_elevated_source() {
    let material = Material::new(1e7, 0.3, 1e9, 0.).unwrap();
    let rest = vec![
        [0.; 3],
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [0., 0., -1.],
    ];
    let cells = vec![([0, 1, 2, 3], material), ([0, 2, 1, 4], material)];
    let elevated = QuadraticBody::from_linear(rest.clone(), cells.clone()).unwrap();
    for stiffness in [1e6, 4e6] {
        let mesh = QuadraticBody::from_linear_with_cohesive_faces(
            rest.clone(),
            cells.clone(),
            Bond::new(stiffness, 2. * stiffness, 1000., 10.).unwrap(),
        )
        .unwrap();
        assert_eq!(
            mesh.source_reference_positions().unwrap(),
            elevated.positions()
        );
        assert_eq!(mesh.body.positions().len(), 20);
        // Independently prescribed rigid cell motions leave the bulk unstrained.
        // Opening 0.1 mm is below onset at both stiffness values.
        let mut positions = mesh.body.positions().to_vec();
        for &node in &mesh.cell_nodes[0] {
            positions[node][2] += 1e-4;
        }
        let report = mesh.body.cohesive_trials_at(&positions).unwrap().remove(0);
        assert!((report.stored_j - 0.5 * 0.5 * stiffness * 1e-8).abs() < 1e-12);
        assert_eq!(report.dissipated_j, 0.);
        let force: f64 = mesh.cell_nodes[0]
            .iter()
            .map(|&n| report.internal_n[n][2])
            .sum();
        assert!((force - 0.5 * stiffness * 1e-4).abs() < 1e-9);
        let result: f64 = report.internal_n.iter().map(|f| f[2]).sum();
        assert!(result.abs() < 1e-9);
        // Elevation adds physical DOFs: corner-only arrays must be rejected.
        assert!(mesh.split_nodal_forces(&vec![[0.; 3]; rest.len()]).is_err());
        assert!(
            mesh.expand_constraints(&vec![[None; 3]; rest.len()])
                .is_err()
        );
    }
    let mut mesh = QuadraticBody::from_linear_with_cohesive_faces(
        rest,
        cells,
        Bond::new(1e6, 2e6, 1000., 10.).unwrap(),
    )
    .unwrap();
    mesh.source_nodes[0] = usize::MAX;
    assert!(mesh.source_reference_positions().is_err());
}
