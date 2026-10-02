use physics::{
    biomechanics::Material as Elastic,
    plasticity::{
        Material,
        mesh::{FiniteQuadraticDynamics, QuadraticBody},
    },
};
fn mesh() -> QuadraticBody {
    QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], Material::new(1e6, 0.3, 1e9, 0.).unwrap())],
    )
    .unwrap()
}
fn spin(dt: f64, steps: usize) -> (FiniteQuadraticDynamics, f64) {
    let mesh = mesh();
    let velocities = mesh
        .positions()
        .iter()
        .map(|p| [-4. * (p[1] - 0.25), 4. * (p[0] - 0.25), 0.])
        .collect();
    let mut body = FiniteQuadraticDynamics::new(
        mesh,
        vec![Elastic::from_young_poisson(1e6, 0.3).unwrap()],
        &[1000.],
        velocities,
        &[false; 10],
    )
    .unwrap();
    let initial = body.energy().unwrap();
    let mut worst = 0_f64;
    for _ in 0..steps {
        body.step(dt, [0.; 3], 1.).unwrap();
        let e = body.energy().unwrap();
        worst =
            worst.max((e.kinetic_j + e.elastic_j - initial.kinetic_j).abs() / initial.kinetic_j);
        for axis in 0..3 {
            assert!((e.momentum_kg_m_s[axis] - initial.momentum_kg_m_s[axis]).abs() < 1e-6);
            assert!(
                (e.angular_momentum_kg_m2_s[axis] - initial.angular_momentum_kg_m2_s[axis]).abs()
                    < 1e-6
            );
        }
    }
    (body, worst)
}
#[test]
fn large_quadratic_rotation_preserves_momenta_and_energy_refines() {
    let (_, coarse) = spin(0.0002, 2500);
    let (body, fine) = spin(0.0001, 5000);
    println!("finite quadratic spin energy envelopes: {coarse:e}, {fine:e}");
    assert!(fine < 0.01 && fine < coarse / 2.);
    let p = body.positions()[1];
    let initial = [0.75, -0.25];
    let current = [p[0] - 0.25, p[1] - 0.25];
    let angle = (initial[0] * current[1] - initial[1] * current[0])
        .atan2(initial[0] * current[0] + initial[1] * current[1]);
    assert!(angle > 1., "rotation={angle}");
}
#[test]
fn finite_quadratic_rejected_step_rolls_back_positions_and_velocities() {
    let (mut body, _) = spin(0.0001, 100);
    let positions = body.positions().to_vec();
    let velocities = body.velocities().to_vec();
    assert!(body.step(0.01, [0.; 3], 1e-15).is_err());
    assert_eq!(body.positions(), positions);
    assert_eq!(body.velocities(), velocities);
}
