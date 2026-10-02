use physics::plasticity::{Material, State, mesh::QuadraticBody};
fn body() -> QuadraticBody {
    QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], Material::new(210e9, 0.3, 250e6, 1e9).unwrap())],
    )
    .unwrap()
}
#[test]
fn affine_plastic_body_commits_volume_heat_and_history_with_reactions() {
    let mut body = body();
    let rest = body.positions().to_vec();
    let mut scalar = State::default();
    let mut previous = 0.;
    let material = Material::new(210e9, 0.3, 250e6, 1e9).unwrap();
    for gamma in [0.01, 0., -0.01, 0.] {
        let prescribed: Vec<_> = rest
            .iter()
            .map(|p| [Some(gamma * p[1]), Some(0.), Some(0.)])
            .collect();
        let loads = vec![[0.; 3]; rest.len()];
        let step = body
            .equilibrate_with_work(&loads, &prescribed, 20, 1e-6)
            .unwrap();
        assert!(step.equilibrium.converged);
        let work = step.work.unwrap();
        let shear = |g: f64| [[0., g / 2., 0.], [g / 2., 0., 0.], [0.; 3]];
        let reference = material
            .response_with_work(&scalar, shear(previous), shear(gamma))
            .unwrap();
        let expected = reference.plastic_heat_j_m3 / 6.;
        assert!((work.plastic_heat_j - expected).abs() < 1e-10 * expected.max(1.));
        assert_eq!(work.cell_plastic_heat_j, vec![work.plastic_heat_j]);
        assert!(work.energy_defect_j.abs() < 1e-10 * work.endpoint_work_j.abs());
        for state in body.states()[0] {
            assert!(
                (state.equivalent_plastic_strain() - reference.state.equivalent_plastic_strain())
                    .abs()
                    < 1e-12
            );
        }
        scalar = reference.state;
        previous = gamma;
    }
}
#[test]
fn rejected_geometry_and_failed_solve_leave_positions_and_history_untouched() {
    let mut body = body();
    let positions = body.positions().to_vec();
    let history = body.states();
    let mut inverted = positions.clone();
    inverted[1][0] = -1.;
    assert!(body.bulk_work_at(&inverted).is_err());
    let prescribed = vec![[Some(0.); 3]; positions.len()];
    assert!(
        body.equilibrate_with_work(&vec![[0.; 3]; positions.len()], &prescribed, 0, 1e-6)
            .is_err()
    );
    assert_eq!(body.positions(), positions);
    assert_eq!(body.states(), history);
}
#[test]
fn unconverged_trial_exposes_no_heat_and_commits_nothing() {
    let mut body = body();
    let positions = body.positions().to_vec();
    let states = body.states();
    let mut loads = vec![[0.; 3]; positions.len()];
    loads[1][0] = 1e9;
    let mut fixed = vec![[Some(0.); 3]; positions.len()];
    fixed[1][0] = None;
    let step = body.equilibrate_with_work(&loads, &fixed, 1, 1e-6).unwrap();
    assert!(!step.equilibrium.converged);
    assert!(step.work.is_none());
    assert_eq!(body.positions(), positions);
    assert_eq!(body.states(), states);
}
