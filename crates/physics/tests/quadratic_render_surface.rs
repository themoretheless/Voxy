use physics::{
    cohesive::Material as Bond,
    plasticity::{Material, mesh::QuadraticBody},
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
fn surface_exposure_and_component_ids_follow_accepted_fracture() {
    let (mut body, upper, _, _) = coupon();
    let before = body.surface_triangles(0, 100).unwrap();
    assert_eq!(before.len(), 6);
    assert!(before.iter().all(|t| t.component == 0));
    let n = body.positions().len();
    let prescribed: Vec<_> = upper
        .iter()
        .map(|u| [Some(0.), Some(0.), Some(if *u { 0.02 } else { -0.02 })])
        .collect();
    assert!(
        body.equilibrate(&vec![[0.; 3]; n], &prescribed, 4, 1e-7)
            .unwrap()
            .converged
    );
    let after = body.surface_triangles(0, 100).unwrap();
    assert_eq!(after.len(), 8);
    assert_eq!(
        after.len(),
        body.exposed_faces_at(body.positions()).unwrap().len()
    );
    let components: std::collections::BTreeSet<_> = after.iter().map(|t| t.component).collect();
    assert_eq!(components.len(), 2);
    let positions = body.positions().to_vec();
    assert!(body.surface_triangles(3, 100).is_err());
    assert_eq!(body.positions(), positions);
}
#[test]
fn quadratic_curved_surface_vertices_and_normals_match_analytic_shear() {
    let m = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    let mut body = QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], m)],
    )
    .unwrap();
    let original = body.positions().to_vec();
    let n = original.len();
    let prescribed: Vec<_> = original
        .iter()
        .map(|p| [Some(0.), Some(0.), Some(0.2 * p[0] * p[0])])
        .collect();
    assert!(
        body.equilibrate(&vec![[0.; 3]; n], &prescribed, 4, 1e-7)
            .unwrap()
            .converged
    );
    let triangles = body.surface_triangles(2, 100).unwrap();
    assert_eq!(triangles.len(), 64);
    for triangle in triangles {
        for vertex in triangle.vertices {
            let p = vertex.position_m;
            let reference = [p[0], p[1], p[2] - 0.2 * p[0] * p[0]];
            let origin = original[triangle.face.nodes[0]];
            let normal = triangle.face.normal;
            let distance: f64 = (0..3).map(|a| normal[a] * (reference[a] - origin[a])).sum();
            assert!(distance.abs() < 1e-12);
            let expected = [normal[0] - 0.4 * p[0] * normal[2], normal[1], normal[2]];
            let norm = expected[0].hypot(expected[1]).hypot(expected[2]);
            for a in 0..3 {
                assert!((vertex.normal[a] - expected[a] / norm).abs() < 1e-12);
            }
            assert!((vertex.barycentric.iter().sum::<f64>() - 1.).abs() < 1e-12);
        }
    }
}
