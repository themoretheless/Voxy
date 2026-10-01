use physics::astrophysics_star::{Scaling, lane_emden};
#[test]
fn known_surfaces_and_profiles() {
    for n in [0.0, 1.0, 5.0] {
        let p = lane_emden(n, 0.001, 10.0, 11000).unwrap();
        for q in &p.points {
            if q.theta == 0.0 {
                continue;
            }
            let exact = if n == 0.0 {
                1.0 - q.xi * q.xi / 6.0
            } else if n == 1.0 {
                q.xi.sin() / q.xi
            } else {
                (1.0 + q.xi * q.xi / 3.0).powf(-0.5)
            };
            let exact = if q.xi == 0.0 { 1.0 } else { exact };
            assert!(
                (q.theta - exact).abs() < 1e-7,
                "n {n} xi {} error {}",
                q.xi,
                q.theta - exact
            );
        }
        if n == 5.0 {
            assert!(p.surface.is_none());
        } else {
            let s = p.surface.unwrap();
            let r = if n == 0.0 {
                6_f64.sqrt()
            } else {
                std::f64::consts::PI
            };
            assert!((s.xi - r).abs() < 1e-7);
            let m = if n == 0.0 { r.powi(3) / 3.0 } else { r };
            assert!((s.mass - m).abs() < 1e-6);
        }
    }
}
#[test]
fn fractional_index_and_physical_mass() {
    let profile = lane_emden(1.5, 0.001, 5.0, 6000).unwrap();
    let s = profile.surface.unwrap();
    assert!((s.xi - 3.65375374).abs() < 2e-6);
    assert!((s.mass - 2.71405512).abs() < 2e-6);
    let p = lane_emden(0.0, 0.001, 3.0, 4000).unwrap();
    let scale = Scaling::new(0.0, 2.0, 3.0, 1.0).unwrap();
    let surface = scale.point(0.0, p.surface.unwrap()).unwrap();
    assert!(
        (surface.enclosed_mass - 4.0 * std::f64::consts::PI / 3.0 * 2.0 * surface.radius.powi(3))
            .abs()
            < 1e-9
    );
    assert_eq!(surface.pressure, 0.0);
}
#[test]
fn rejects_invalid_and_exhausted_budget() {
    assert!(lane_emden(-1.0, 0.001, 5.0, 10000).is_err());
    assert!(lane_emden(1.0, 0.001, 5.0, 1).is_err());
    assert!(Scaling::new(1.0, 0.0, 1.0, 1.0).is_err());
}
#[test]
fn refinement_converges_and_hydrostatic_balance_holds() {
    let reference = lane_emden(1.5, 0.0005, 5.0, 11000)
        .unwrap()
        .surface
        .unwrap();
    let coarse = lane_emden(1.5, 0.02, 5.0, 1000).unwrap().surface.unwrap();
    let fine = lane_emden(1.5, 0.01, 5.0, 1000).unwrap().surface.unwrap();
    assert!((fine.xi - reference.xi).abs() < (coarse.xi - reference.xi).abs());
    let profile = lane_emden(1.5, 0.001, 5.0, 6000).unwrap();
    let scale = Scaling::new(1.5, 2.0, 3.0, 1.0).unwrap();
    for trio in profile.points.windows(3).skip(10).take(3000).step_by(100) {
        let a = scale.point(1.5, trio[0]).unwrap();
        let b = scale.point(1.5, trio[1]).unwrap();
        let c = scale.point(1.5, trio[2]).unwrap();
        let gradient = (c.pressure - a.pressure) / (c.radius - a.radius);
        let gravity = -b.enclosed_mass * b.density / b.radius.powi(2);
        assert!((gradient - gravity).abs() < 1e-5);
    }
}

#[test]
fn sampled_profile_matches_analytic_solution_and_rejects_malformed_data() {
    let profile = lane_emden(1.0, 0.001, 4.0, 5000).unwrap();
    for xi in [0.0_f64, 0.1005, 0.7777, 1.2345, 2.9999] {
        let point = profile.sample(xi).unwrap();
        let theta = if xi == 0.0 { 1.0 } else { xi.sin() / xi };
        let mass = xi.sin() - xi * xi.cos();
        assert!((point.theta - theta).abs() < 1e-7);
        assert!((point.mass - mass).abs() < 5e-7);
    }
    let surface = profile.surface.unwrap();
    assert_eq!(profile.sample(surface.xi).unwrap(), surface);
    for xi in [-1.0, f64::NAN, surface.xi + 1e-8] {
        assert!(profile.sample(xi).is_err());
    }
    let mut malformed = profile.clone();
    malformed.points[3].xi = malformed.points[2].xi;
    assert!(malformed.sample(0.1).is_err());
    let mut malformed = profile;
    malformed.points[3].mass = -1.0;
    assert!(malformed.sample(0.1).is_err());
}

#[test]
fn volume_averages_match_analytic_polytropes_including_centre_and_surface() {
    let uniform = lane_emden(0.0, 0.0005, 3.0, 7000).unwrap();
    for (a, b) in [
        (0.0_f64, 0.5_f64),
        (0.7, 1.3),
        (2.0, uniform.surface.unwrap().xi),
    ] {
        let avg = uniform.shell_average(a, b, 64).unwrap();
        let mean_r2 = 0.6 * (b.powi(5) - a.powi(5)) / (b.powi(3) - a.powi(3));
        assert!((avg.density_over_central - 1.0).abs() < 1e-12);
        assert!((avg.pressure_over_central - (1.0 - mean_r2 / 6.0)).abs() < 2e-8);
    }
    let p = lane_emden(1.0, 0.0005, 4.0, 9000).unwrap();
    let surface = p.surface.unwrap().xi;
    let mut integrated_mass = 0.0;
    for i in 0..16 {
        let a = surface * i as f64 / 16.0;
        let b = surface * (i + 1) as f64 / 16.0;
        let avg = p.shell_average(a, b, 64).unwrap();
        let expected =
            3.0 * ((b.sin() - b * b.cos()) - (a.sin() - a * a.cos())) / (b.powi(3) - a.powi(3));
        assert!((avg.density_over_central - expected).abs() < 3e-8);
        assert!(avg.pressure_over_central > 0.0);
        integrated_mass += avg.density_over_central * (b.powi(3) - a.powi(3)) / 3.0;
    }
    assert!((integrated_mass / p.surface.unwrap().mass - 1.0).abs() < 1e-7);
    assert!(p.shell_average(0.0, 1.0, 0).is_err());
    assert!(p.shell_average(1.0, 1.0, 1).is_err());
    assert!(p.shell_average(0.0, surface + 0.01, 1).is_err());
}
