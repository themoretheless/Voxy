use physics::biomechanics::{Body, Fiber, HgoMaterial, Material, ViscoelasticHgo};
fn body() -> Body {
    let mut b = Body::new(
        vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
        vec![true, false, true, true],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 3000.,
                bulk_pa: 50000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    b.set_viscoelastic_hgo_batch(&[(
        0,
        ViscoelasticHgo::new(
            HgoMaterial {
                shear_pa: 3000.,
                bulk_pa: 50000.,
                fibers: vec![Fiber {
                    direction: [1., 0., 0.],
                    stiffness_pa: 9000.,
                    exponent: 0.2,
                    active_pa: 0.,
                }],
            },
            &[(0.2, 0.3), (2., 0.5)],
        )
        .unwrap(),
    )])
    .unwrap();
    b.set_force(1, [0.01, 0., 0.]).unwrap();
    b
}
#[test]
fn physical_steps_preserve_equilibrium_and_reference() {
    let mut b = body();
    let reference = b.rest_positions().to_vec();
    for step in 1..=4 {
        let report = b
            .relax_step_lbfgs_states(0.1, 10000, 1e-8, |_, _, _, _, _, _, _| {})
            .unwrap();
        assert!(report.converged);
        let gradient = b.evaluate(b.positions()).unwrap().1;
        let residual = gradient[1].iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(residual <= 1e-8, "committed residual {residual}");
        println!(
            "hgo_fem_step,{:.1},{:.12e},{:.12e}",
            f64::from(step) * 0.1,
            b.positions()[1][0] - reference[1][0],
            residual
        );
        assert_eq!(b.rest_positions(), reference);
    }
}
#[test]
fn failed_step_and_invalid_assignment_preserve_history() {
    let mut b = body();
    let control = b.clone();
    let positions = b.positions().to_vec();
    let before = b.evaluate(&positions).unwrap();
    assert!(b.relax_step(0.1, 0, 1e-12).is_err());
    assert_eq!(b.positions(), positions);
    assert_eq!(b.evaluate(&positions).unwrap(), before);
    assert!(b.set_activation(0, 1.).is_err());
    let mut c = control;
    b.relax_step_lbfgs_states(0.1, 10000, 1e-8, |_, _, _, _, _, _, _| {})
        .unwrap();
    c.relax_step_lbfgs_states(0.1, 10000, 1e-8, |_, _, _, _, _, _, _| {})
        .unwrap();
    assert_eq!(b.positions(), c.positions());
    assert_eq!(
        b.evaluate(b.positions()).unwrap(),
        c.evaluate(c.positions()).unwrap()
    );
}
