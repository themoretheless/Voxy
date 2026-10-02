use physics::plasticity::{Material, mesh::QuadraticBody};
fn points() -> Vec<[f64; 3]> {
    vec![
        [0.; 3],
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [0.5, 0., 0.],
        [0.5, 0.5, 0.],
        [0., 0.5, 0.],
        [0., 0., 0.5],
        [0.5, 0., 0.5],
        [0., 0.5, 0.5],
    ]
}
fn body(yield_pa: f64) -> QuadraticBody {
    QuadraticBody::new(
        points(),
        vec![(
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            Material::new(1e6, 0., yield_pa, 1000.).unwrap(),
        )],
    )
    .unwrap()
}
#[test]
fn quadratic_interpolation_reproduces_pure_bending_stress_distribution() {
    let body = body(1e9);
    let k = 1e-3;
    let x: Vec<_> = points()
        .iter()
        .map(|p| [p[0] - k * p[0] * p[2], p[1], p[2] + 0.5 * k * p[0] * p[0]])
        .collect();
    let responses = body.responses_at(&x).unwrap();
    let high = (5. + 3. * 5_f64.sqrt()) / 20.;
    let low = (5. - 5_f64.sqrt()) / 20.;
    for (point, response) in responses[0].iter().enumerate() {
        let z = if point == 3 { high } else { low };
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == 0 && j == 0 { -1e6 * k * z } else { 0. };
                assert!((response.stress.cauchy_pa[i][j] - expected).abs() < 1e-8);
            }
        }
    }
    assert!(
        body.states()
            .iter()
            .flatten()
            .all(|s| s.equivalent_plastic_strain() == 0.)
    );
}
#[test]
fn quadrature_plastic_histories_commit_together_and_failed_candidates_roll_back() {
    let mut body = body(100.);
    let initial = body.positions().to_vec();
    let states = body.states();
    let mut loads = vec![[0.; 3]; 10];
    loads[1][0] = 1000.;
    let mut prescribed = vec![[Some(0.); 3]; 10];
    prescribed[1][0] = None;
    assert!(
        !body
            .equilibrate(&loads, &prescribed, 1, 1e-8)
            .unwrap()
            .converged
    );
    assert_eq!(body.positions(), initial);
    assert_eq!(body.states(), states);
    for (p, bc) in points().iter().zip(&mut prescribed) {
        *bc = [Some(0.001 * p[0]), Some(0.), Some(0.)];
    }
    assert!(
        body.equilibrate(&vec![[0.; 3]; 10], &prescribed, 10, 1e-8)
            .unwrap()
            .converged
    );
    assert!(
        body.states()[0]
            .iter()
            .all(|s| s.equivalent_plastic_strain() > 0.)
    );
    let accepted = body.positions().to_vec();
    let history = body.states();
    for (p, bc) in points().iter().zip(&mut prescribed) {
        bc[0] = Some(-2. * p[0]);
    }
    assert!(
        body.equilibrate(&vec![[0.; 3]; 10], &prescribed, 10, 1e-8)
            .is_err()
    );
    assert_eq!(body.positions(), accepted);
    assert_eq!(body.states(), history);
}
#[test]
fn curved_reference_edges_and_invalid_response_counts_are_rejected() {
    let mut p = points();
    p[4][1] = 0.01;
    assert!(
        QuadraticBody::new(
            p,
            vec![(
                [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                Material::new(1e6, 0., 1e9, 0.).unwrap()
            )]
        )
        .is_err()
    );
    assert!(body(1e9).responses_at(&points()[..9]).is_err());
}

#[test]
fn elevation_preserves_corners_and_shares_every_common_edge_midpoint() {
    let material = Material::new(1e6, 0., 1e9, 0.).unwrap();
    let rest = vec![
        [0.; 3],
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [0., 0., -1.],
    ];
    let quadratic = QuadraticBody::from_linear(
        rest.clone(),
        vec![([0, 1, 2, 3], material), ([2, 1, 0, 4], material)],
    )
    .unwrap();
    assert_eq!(&quadratic.positions()[..5], rest);
    assert_eq!(quadratic.positions().len(), 14); // 5 corners + 9 distinct edges.
    let edges = quadratic.edge_midpoints();
    assert_eq!(edges.len(), 9);
    for ([a, b], mid) in edges {
        for axis in 0..3 {
            assert_eq!(
                quadratic.positions()[mid][axis],
                rest[a][axis].midpoint(rest[b][axis])
            );
        }
    }
    let mut prescribed = vec![[Some(0.); 3]; 14];
    for (p, bc) in quadratic.positions().iter().zip(&mut prescribed) {
        bc[0] = Some(0.001 * p[0]);
    }
    let mut quadratic = quadratic;
    assert!(
        quadratic
            .equilibrate(&vec![[0.; 3]; 14], &prescribed, 10, 1e-8)
            .unwrap()
            .converged
    );
    let responses = quadratic.responses_at(quadratic.positions()).unwrap();
    for response in responses.iter().flatten() {
        assert!((response.stress.cauchy_pa[0][0] - 1000.).abs() < 1e-8);
    }
}

#[test]
fn duplicate_nonmanifold_and_same_side_neighbor_cells_are_rejected() {
    let material = Material::new(1e6, 0., 1e9, 0.).unwrap();
    let rest = vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    assert!(QuadraticBody::from_linear(rest, vec![([0, 1, 2, 3], material); 2]).is_err());
    let rest = vec![
        [0.; 3],
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [0., 0., 2.],
    ];
    assert!(
        QuadraticBody::from_linear(
            rest,
            vec![([0, 1, 2, 3], material), ([0, 2, 1, 4], material)]
        )
        .is_err()
    );
    let rest = vec![
        [0.; 3],
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [0., 0., -1.],
        [0., 0., 2.],
    ];
    assert!(
        QuadraticBody::from_linear(
            rest,
            vec![
                ([0, 1, 2, 3], material),
                ([0, 1, 2, 4], material),
                ([0, 1, 2, 5], material)
            ]
        )
        .is_err()
    );
}
