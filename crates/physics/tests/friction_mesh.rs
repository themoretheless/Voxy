#[path = "support/cohesive.rs"]
mod support;
use physics::{cohesive, friction::Mode};
#[test]
fn fractured_slabs_stick_then_slide_at_mu_pressure_and_keep_history_transactional() {
    for n in [1, 2] {
        let interface = cohesive::Material::new(1e9, 1e10, 1e5, 10.)
            .unwrap()
            .with_friction(0.5, 1e9)
            .unwrap();
        let (mut body, mut prescribed, end) = support::fixture(n, interface);
        let rest = body.positions().to_vec();
        let loads = vec![[0.; 3]; rest.len()];
        for &i in &end {
            prescribed[i][0] = Some(0.0002);
        }
        let broken = body.equilibrate(&loads, &prescribed, 50, 1e-6).unwrap();
        assert!(broken.converged);
        let pressure = 1e10 * 1e-5 / (1. + 2. * 0.1 * 1e10 / 1e9);
        for (i, p) in rest.iter().enumerate() {
            let displacement = if i < rest.len() / 2 {
                -pressure / 1e9 * (p[0] + 0.1)
            } else {
                -1e-5 + pressure / 1e9 * (0.1 - p[0])
            };
            let at_outer = prescribed[i][0].is_some();
            prescribed[i] = [
                Some(displacement),
                if at_outer { Some(0.) } else { None },
                Some(0.),
            ];
        }
        let closed = body.equilibrate(&loads, &prescribed, 50, 1e-6).unwrap();
        assert!(closed.converged, "{closed:?}");
        let positions = body.positions().to_vec();
        let history = body.interface_states();
        for &i in &end {
            prescribed[i][1] = Some(5e-5);
        }
        let failed = body.equilibrate(&loads, &prescribed, 1, 1e-6).unwrap();
        assert!(!failed.converged);
        assert_eq!(body.positions(), positions);
        assert_eq!(body.interface_states(), history);
        for step in 0..=10 {
            let displacement = f64::from(step) * 5e-6;
            for &i in &end {
                prescribed[i][1] = Some(displacement);
            }
            let result = body.equilibrate(&loads, &prescribed, 50, 1e-6).unwrap();
            assert!(result.converged, "n={n}, step={step}, {result:?}");
            let force: f64 = end.iter().map(|&i| result.reactions_n[i][1]).sum();
            let traction = (displacement / (1. / 1e9 + 2. * 0.1 / 5e8)).min(0.5 * pressure);
            assert!(
                (force - traction * 0.01).abs() < 1e-5,
                "{force} != {}",
                traction * 0.01
            );
        }
        let reports = body.interface_reports().unwrap();
        let dissipated: f64 = reports.iter().map(|r| r.friction_dissipated_j).sum();
        let slip = 5e-5 - 0.5 * pressure * (1. / 1e9 + 2. * 0.1 / 5e8);
        assert!((dissipated - 0.5 * pressure * slip * 0.01).abs() < 1e-9);
        assert!(reports.iter().all(|r| {
            r.quadrature.iter().all(|q| {
                q.damage > 1. - 1e-10
                    && q.friction_mode != Some(Mode::Open)
                    && (q.traction_pa[1] - 0.5 * pressure).abs() < 1e-5
            })
        }));
        let numerical: f64 = reports.iter().map(|r| r.friction_numerical_j).sum();
        let held = body.equilibrate(&loads, &prescribed, 50, 1e-6).unwrap();
        assert!(held.converged);
        let now: f64 = body
            .interface_reports()
            .unwrap()
            .iter()
            .map(|r| r.friction_dissipated_j)
            .sum();
        assert!((now - dissipated).abs() < 1e-10);
        let after_numerical: f64 = body
            .interface_reports()
            .unwrap()
            .iter()
            .map(|r| r.friction_numerical_j)
            .sum();
        assert!((after_numerical - numerical).abs() < 1e-10);
    }
}

#[test]
fn coupled_normal_and_shear_free_dofs_satisfy_global_equilibrium_and_coulomb_bound() {
    let interface = cohesive::Material::new(1e9, 1e10, 1e5, 10.)
        .unwrap()
        .with_friction(0.5, 1e9)
        .unwrap();
    let (mut body, mut prescribed, end) = support::fixture(1, interface);
    let loads = vec![[0.; 3]; body.positions().len()];
    for &index in &end {
        prescribed[index][0] = Some(0.0002);
    }
    assert!(
        body.equilibrate(&loads, &prescribed, 50, 1e-6)
            .unwrap()
            .converged
    );
    for &index in &end {
        prescribed[index][0] = Some(-1e-5);
    }
    for row in &mut prescribed {
        if row[0].is_none() {
            row[1] = None;
        }
    }
    assert!(
        body.equilibrate(&loads, &prescribed, 50, 1e-6)
            .unwrap()
            .converged
    );
    for step in 1..=10 {
        for &index in &end {
            prescribed[index][1] = Some(f64::from(step) * 4e-6);
        }
        let result = body.equilibrate(&loads, &prescribed, 100, 1e-6).unwrap();
        assert!(result.converged, "step={step}, {result:?}");
        let reports = body.interface_reports().unwrap();
        let normal_force: f64 = reports
            .iter()
            .flat_map(|r| {
                r.quadrature
                    .iter()
                    .map(|q| -q.traction_pa[0] * r.area_m2 / 3.)
            })
            .sum();
        let shear_force: f64 = reports
            .iter()
            .flat_map(|r| {
                r.quadrature
                    .iter()
                    .map(|q| q.traction_pa[1] * r.area_m2 / 3.)
            })
            .sum();
        let reaction: f64 = end.iter().map(|&index| result.reactions_n[index][1]).sum();
        assert!((reaction - shear_force).abs() < 1e-5);
        assert!(shear_force.abs() <= 0.5 * normal_force + 1e-5);
        assert!(normal_force > 0.);
    }
    let friction_work: f64 = body
        .interface_reports()
        .unwrap()
        .iter()
        .map(|r| r.friction_dissipated_j)
        .sum();
    assert!(friction_work > 0., "coupled test must reach actual sliding");
}
