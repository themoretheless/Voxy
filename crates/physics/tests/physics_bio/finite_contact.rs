use physics::biomechanics::{Body, InertialBody, Material, PlaneContact};
fn impact(stiffness: f64, dt: f64, steps: usize) -> (f64, f64, f64) {
    let points = vec![
        [0., 0.01, 0.],
        [0.1, 0.01, 0.],
        [0., 0.11, 0.],
        [0., 0.01, 0.1],
    ];
    let body = Body::new(
        points,
        vec![false; 4],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(1e5, 0.3).unwrap(),
        )],
    )
    .unwrap();
    let mut body = InertialBody::new(body, &[1000.], vec![[0., -1., 0.]; 4]).unwrap();
    assert_eq!(
        body.set_plane_contact(Some(
            PlaneContact::new([0., 1., 0.], 0., stiffness).unwrap()
        ))
        .unwrap(),
        0.
    );
    let initial = body.diagnostics().unwrap();
    let mut penetration = 0_f64;
    let mut defect = 0_f64;
    for _ in 0..steps {
        let before = body.diagnostics().unwrap().momentum_kg_m_s[1];
        let force_before: f64 = body
            .body()
            .positions()
            .iter()
            .map(|p| -stiffness * p[1].min(0.))
            .sum();
        body.step(dt, 1e-4).unwrap();
        let force_after: f64 = body
            .body()
            .positions()
            .iter()
            .map(|p| -stiffness * p[1].min(0.))
            .sum();
        let after = body.diagnostics().unwrap().momentum_kg_m_s[1];
        assert!((after - before - 0.5 * dt * (force_before + force_after)).abs() < 1e-11);
        for p in body.body().positions() {
            penetration = penetration.max(-p[1]);
        }
        let d = body.diagnostics().unwrap();
        defect =
            defect.max((d.kinetic_j + d.potential_j - initial.kinetic_j).abs() / initial.kinetic_j);
        assert!((d.momentum_kg_m_s[0] - initial.momentum_kg_m_s[0]).abs() < 1e-10);
        assert!((d.momentum_kg_m_s[2] - initial.momentum_kg_m_s[2]).abs() < 1e-10);
    }
    let momentum = body.diagnostics().unwrap().momentum_kg_m_s[1];
    (penetration, defect, momentum)
}
#[test]
fn frictionless_elastic_impact_rebounds_and_stiffer_contact_reduces_penetration() {
    let (soft, error_soft, momentum_soft) = impact(20_000., 1e-5, 6000);
    let (stiff, error_stiff, momentum_stiff) = impact(80_000., 1e-5, 6000);
    println!(
        "penetration soft={soft:e}, stiff={stiff:e}; energy deviations {error_soft:e}, {error_stiff:e}"
    );
    assert!(soft > 0. && stiff > 0. && stiff < 0.9 * soft);
    assert!(error_soft < 1e-3 && error_stiff < 1e-3);
    assert!(momentum_soft > 0. && momentum_stiff > 0.);
}
#[test]
fn invalid_plane_parameters_fail_before_contact_is_installed() {
    for normal in [[0.; 3], [0., 2., 0.], [f64::NAN, 0., 0.]] {
        assert!(PlaneContact::new(normal, 0., 1000.).is_err());
    }
    for stiffness in [0., -1., f64::INFINITY] {
        assert!(PlaneContact::new([0., 1., 0.], 0., stiffness).is_err());
    }
    assert!(PlaneContact::new([0., 1., 0.], f64::NAN, 1000.).is_err());
}

#[test]
fn contact_energy_error_reduces_with_timestep_refinement() {
    let (_, coarse, _) = impact(20_000., 2e-5, 3000);
    let (_, fine, _) = impact(20_000., 1e-5, 6000);
    assert!(fine < coarse / 2., "coarse={coarse:e}, fine={fine:e}");
}
