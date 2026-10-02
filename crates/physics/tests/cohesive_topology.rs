use physics::{
    cohesive,
    plasticity::{
        Material,
        mesh::{Body, DynamicBody},
    },
};
fn solid() -> Material {
    Material::new(1e9, 0.3, 1e8, 0.).unwrap()
}
fn bond() -> cohesive::Material {
    cohesive::Material::new(1e9, 1e10, 1e5, 10.).unwrap()
}
fn points() -> Vec<[f64; 3]> {
    vec![
        [0.; 3],
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [0., 0., -1.],
    ]
}
#[test]
fn shared_face_is_bonded_then_breaks_into_two_independent_cells() {
    let mesh = Body::with_cohesive_faces(
        &points(),
        &vec![([0, 1, 2, 3], solid()), ([2, 0, 1, 4], solid())],
        bond(),
    )
    .unwrap();
    assert_eq!(mesh.source_nodes, [0, 1, 2, 3, 2, 0, 1, 4]);
    assert_eq!(mesh.cell_nodes, [[0, 1, 2, 3], [4, 5, 6, 7]]);
    let reports = mesh.body.interface_reports().unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(mesh.body.exposed_faces().unwrap().len(), 6);
    assert!((reports[0].area_m2 - 0.5).abs() < 1e-14);
    assert_eq!(reports[0].normal, [0., 0., -1.]);
    let initial = DynamicBody::new(mesh.body.clone(), &[1000.; 2], vec![[0.; 3]; 8]).unwrap();
    assert_eq!(initial.fragments().unwrap().len(), 1);
    let mut body = mesh.body;
    let mut prescribed = vec![[Some(0.); 3]; 8];
    for step in 1..=30 {
        let separation = f64::from(step) * 1e-5;
        for (cell, nodes) in mesh.cell_nodes.iter().enumerate() {
            for &node in nodes {
                for (axis, value) in prescribed[node].iter_mut().enumerate() {
                    *value = Some(
                        if cell == 0 { -0.5 } else { 0.5 } * separation * reports[0].normal[axis],
                    );
                }
            }
        }
        assert!(
            body.equilibrate(&[[0.; 3]; 8], &prescribed, 10, 1e-8)
                .unwrap()
                .converged
        );
    }
    assert_eq!(body.exposed_faces().unwrap().len(), 8);
    let report = &body.interface_reports().unwrap()[0];
    assert!((report.dissipated_j - 5.).abs() < 1e-9);
    assert!(report.quadrature.iter().all(|q| q.damage == 1.));
    assert!(
        body.responses()
            .unwrap()
            .iter()
            .all(|r| r.elastic_energy_j_m3 < 1e-16)
    );
    let fractured = DynamicBody::new(body, &[1000.; 2], vec![[0.; 3]; 8]).unwrap();
    let fragments = fractured.fragments().unwrap();
    assert_eq!(fragments.len(), 2);
    for fragment in fragments {
        assert_eq!(fragment.nodes.len(), 4);
        assert!((fragment.mass_kg - 1000. / 6.).abs() < 1e-10);
    }
}
#[test]
fn boundary_faces_are_unpaired_and_invalid_topologies_fail() {
    let single =
        Body::with_cohesive_faces(&points()[..4], &vec![([0, 1, 2, 3], solid())], bond()).unwrap();
    assert!(single.body.interface_reports().unwrap().is_empty());
    assert!(
        Body::with_cohesive_faces(
            &points(),
            &vec![
                ([0, 1, 2, 3], solid()),
                ([0, 1, 2, 4], solid()),
                ([0, 1, 2, 3], solid())
            ],
            bond()
        )
        .is_err()
    );
    assert!(
        Body::with_cohesive_faces(&points()[..4], &vec![([0, 1, 2, 3], solid()); 33], bond())
            .is_err()
    );
    assert!(
        Body::with_cohesive_faces(&points()[..4], &vec![([0, 1, 2, 3], solid()); 2], bond())
            .is_err()
    );
}

#[test]
fn source_load_mapping_preserves_force_and_reference_moment_and_checks_inputs() {
    let mut mesh = Body::with_cohesive_faces(
        &points(),
        &[([0, 1, 2, 3], solid()), ([2, 0, 1, 4], solid())],
        bond(),
    )
    .unwrap();
    let forces = [
        [3., -7., 11.],
        [12., -18., 24.],
        [-5., 17., 4.],
        [1., 2., 3.],
        [-2., -4., 8.],
    ];
    let expanded = mesh.split_nodal_forces(&forces).unwrap();
    for axis in 0..3 {
        let original: f64 = forces.iter().map(|f| f[axis]).sum();
        let duplicated: f64 = expanded.iter().map(|f| f[axis]).sum();
        assert!((original - duplicated).abs() < 1e-12);
        let j = (axis + 1) % 3;
        let k = (axis + 2) % 3;
        let original: f64 = points()
            .iter()
            .zip(forces)
            .map(|(p, f)| p[j] * f[k] - p[k] * f[j])
            .sum();
        let duplicated: f64 = mesh
            .body
            .positions()
            .iter()
            .zip(&expanded)
            .map(|(p, f)| p[j] * f[k] - p[k] * f[j])
            .sum();
        assert!((original - duplicated).abs() < 1e-12);
    }
    let prescribed = [[Some(0.), None, Some(0.2)]; 5];
    assert_eq!(
        mesh.expand_constraints(&prescribed).unwrap(),
        vec![prescribed[0]; 8]
    );
    assert!(mesh.split_nodal_forces(&forces[..4]).is_err());
    assert!(mesh.expand_constraints(&prescribed[..4]).is_err());
    let mut invalid = forces;
    invalid[0][0] = f64::NAN;
    assert!(mesh.split_nodal_forces(&invalid).is_err());
    mesh.source_nodes[0] = 5;
    assert!(mesh.split_nodal_forces(&forces).is_err());
}

#[test]
fn free_interface_nodes_transfer_applied_force_and_match_linear_work() {
    let mut mesh = Body::with_cohesive_faces(
        &points(),
        &[([0, 1, 2, 3], solid()), ([2, 0, 1, 4], solid())],
        bond(),
    )
    .unwrap();
    let mut source_load = [[0.; 3]; 5];
    source_load[4][2] = -100.;
    let loads = mesh.split_nodal_forces(&source_load).unwrap();
    let mut prescribed = vec![[Some(0.); 3]; 8];
    for &node in &mesh.cell_nodes[1] {
        prescribed[node][2] = None;
    }
    let result = mesh
        .body
        .equilibrate(&loads, &prescribed, 30, 1e-7)
        .unwrap();
    assert!(result.converged, "{result:?}");
    let report = &mesh.body.interface_reports().unwrap()[0];
    for q in report.quadrature {
        assert_eq!(q.damage, 0.);
    }
    let reaction: f64 = mesh.cell_nodes[0]
        .iter()
        .map(|&n| result.reactions_n[n][2])
        .sum();
    assert!((reaction - 100.).abs() < 1e-6);
    // A tetrahedral apex load is not uniform traction on its opposite face:
    // allow its interface nodes to move independently and use Clapeyron's
    // linear work identity rather than a uniform bar compliance approximation.
    let apex = mesh.cell_nodes[1][3];
    let work = 0.5 * -100. * (mesh.body.positions()[apex][2] + 1.);
    let energy = DynamicBody::new(mesh.body, &[1000.; 2], vec![[0.; 3]; 8])
        .unwrap()
        .diagnostics()
        .unwrap();
    assert!(work > 0.);
    assert!((energy.elastic_j + energy.interface_stored_j - work).abs() < 1e-10);
}
