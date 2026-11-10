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
fn oversized_fracture_step_rejects_before_any_history_or_geometry_commit() {
    let (mut body, upper, _, _) = coupon();
    let old = body.positions().to_vec();
    let states = body.cohesive_interfaces()[0].states();
    let fixed: Vec<_> = upper
        .iter()
        .map(|u| [Some(0.), Some(0.), Some(if *u { 0.02 } else { 0. })])
        .collect();
    assert!(
        body.equilibrate_with_interface_work(&vec![[0.; 3]; old.len()], &fixed, 1, 1e-8, 0.001)
            .is_err()
    );
    assert_eq!(body.positions(), old);
    assert_eq!(body.cohesive_interfaces()[0].states(), states);
}
fn fracture(divisions: usize) -> f64 {
    let (mut body, upper, _, _) = coupon();
    let n = body.positions().len();
    let mut endpoint = 0.;
    let mut fracture = 0.;
    let mut error = 0.;
    let mut absolute_error = 0.;
    for i in 1..=divisions {
        let fixed: Vec<_> = upper
            .iter()
            .map(|u| {
                [
                    Some(0.),
                    Some(0.),
                    Some(if *u {
                        0.02 * i as f64 / divisions as f64
                    } else {
                        0.
                    }),
                ]
            })
            .collect();
        let work = body
            .equilibrate_with_interface_work(&vec![[0.; 3]; n], &fixed, 1, 1e-8, 0.1)
            .unwrap()
            .work
            .unwrap();
        endpoint += work.endpoint_work_j;
        fracture += work.interfaces[0].fracture_work_j;
        error += work.interface_error_j;
        absolute_error += work.interface_error_j.abs();
        assert_eq!(work.interfaces[0].friction_heat_j, 0.);
    }
    assert!(body.cohesive_interfaces()[0].is_fully_broken());
    assert!((fracture - 5.).abs() < 1e-10);
    assert!((endpoint - fracture - error).abs() < 1e-10);
    let fixed = vec![[Some(0.); 3]; n];
    let closed = body
        .equilibrate_with_interface_work(&vec![[0.; 3]; n], &fixed, 1, 1e-8, 1e-8)
        .unwrap()
        .work
        .unwrap();
    assert_eq!(closed.interfaces[0].fracture_work_j, 0.);
    assert!(body.cohesive_interfaces()[0].is_fully_broken());
    absolute_error
}
#[test]
fn fracture_energy_area_and_signed_work_error_converge_without_healing() {
    let coarse = fracture(128);
    let fine = fracture(512);
    assert!(fine < coarse * 0.3);
}
