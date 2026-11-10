use physics::{
    biomechanics::{Material as Elastic, Matrix},
    plasticity::{Material, mesh::QuadraticBody},
};
fn body() -> QuadraticBody {
    QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], Material::new(1e6, 0.3, 1e9, 0.).unwrap())],
    )
    .unwrap()
}
fn mv(m: Matrix, v: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| (0..3).map(|j| m[i][j] * v[j]).sum())
}
#[test]
fn quadratic_finite_elasticity_is_objective_and_force_matches_energy_gradient() {
    let b = body();
    let material = Elastic::from_young_poisson(1e6, 0.3).unwrap();
    let rotation = [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0., 0.6]];
    let rotated: Vec<_> = b
        .positions()
        .iter()
        .map(|&p| mv(rotation, p).map(|x| x + 0.2))
        .collect();
    let response = b
        .finite_elastic_at(&rotated, std::slice::from_ref(&material))
        .unwrap();
    assert!(response.energy_j.abs() < 1e-8);
    assert!(response.internal_n.iter().flatten().all(|f| f.abs() < 1e-7));
    let deformation = [[1.1, 0.2, 0.], [0., 0.94, 0.1], [0., 0., 1.03]];
    let x: Vec<_> = b.positions().iter().map(|&p| mv(deformation, p)).collect();
    let original = b
        .finite_elastic_at(&x, std::slice::from_ref(&material))
        .unwrap();
    let rotated: Vec<_> = x
        .iter()
        .map(|&p| mv(rotation, p).map(|v| v + 0.2))
        .collect();
    let transformed = b
        .finite_elastic_at(&rotated, std::slice::from_ref(&material))
        .unwrap();
    assert!((original.energy_j - transformed.energy_j).abs() < 1e-8);
    for (a, b) in original.internal_n.iter().zip(&transformed.internal_n) {
        for (u, v) in mv(rotation, *a).iter().zip(b) {
            assert!((u - v).abs() < 1e-7);
        }
    }
    for node in 0..10 {
        for axis in 0..3 {
            let mut plus = x.clone();
            let mut minus = x.clone();
            plus[node][axis] += 1e-6;
            minus[node][axis] -= 1e-6;
            let p = b
                .finite_elastic_at(&plus, std::slice::from_ref(&material))
                .unwrap()
                .energy_j;
            let m = b
                .finite_elastic_at(&minus, std::slice::from_ref(&material))
                .unwrap()
                .energy_j;
            assert!(((p - m) / 2e-6 - original.internal_n[node][axis]).abs() < 3e-4);
        }
    }
    assert!(
        b.finite_elastic_at(&x[..9], std::slice::from_ref(&material))
            .is_err()
    );
}

#[test]
fn finite_elastic_evaluation_rejects_inversion_and_existing_plastic_history() {
    let b = body();
    let elastic = Elastic::from_young_poisson(1e6, 0.3).unwrap();
    let inverted: Vec<_> = b.positions().iter().map(|p| [-p[0], p[1], p[2]]).collect();
    assert!(
        b.finite_elastic_at(&inverted, std::slice::from_ref(&elastic))
            .is_err()
    );
    let mut plastic = QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], Material::new(1e6, 0.3, 1., 1000.).unwrap())],
    )
    .unwrap();
    let prescribed: Vec<_> = plastic
        .positions()
        .iter()
        .map(|p| [Some(0.001 * p[0]), Some(0.), Some(0.)])
        .collect();
    assert!(
        plastic
            .equilibrate(&vec![[0.; 3]; 10], &prescribed, 10, 1e-8)
            .unwrap()
            .converged
    );
    assert!(
        plastic
            .finite_elastic_at(plastic.positions(), std::slice::from_ref(&elastic))
            .is_err()
    );
}
