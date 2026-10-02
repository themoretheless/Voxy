use physics::liquid::{DryWallSplashOnset, ImpactNumbers};
#[test]
fn normal_impact_numbers_match_independent_si_values_and_scaling() {
    let n = ImpactNumbers::new(1000.0, 0.001, 0.072, 0.001, 2.0).unwrap();
    assert!((n.reynolds - 2000.0).abs() < 1e-10);
    assert!((n.weber - 55.55555555555556).abs() < 1e-12);
    assert!((n.ohnesorge - 0.0037267799624996494).abs() < 1e-15);
    assert!((n.splash_parameter - n.ohnesorge * n.reynolds.powf(1.25)).abs() < 1e-12);
    let viscous = ImpactNumbers::new(1000.0, 0.016, 0.072, 0.001, 2.0).unwrap();
    assert!((viscous.splash_parameter * 2.0 - n.splash_parameter).abs() < 1e-12);
    let faster = ImpactNumbers::new(1000.0, 0.001, 0.072, 0.001, 4.0).unwrap();
    assert!((faster.splash_parameter / n.splash_parameter - 2.0_f64.powf(1.25)).abs() < 1e-12);
    let criterion = DryWallSplashOnset {
        critical_parameter: (n.splash_parameter + viscous.splash_parameter) * 0.5,
    };
    assert!(criterion.permits(n).unwrap());
    assert!(!criterion.permits(viscous).unwrap());
    let rest = ImpactNumbers::new(1000.0, 0.001, 0.072, 0.001, 0.0).unwrap();
    assert_eq!(rest.splash_parameter, 0.0);
    assert!(!criterion.permits(rest).unwrap());
}
#[test]
fn invalid_and_overflowing_inputs_are_rejected() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(ImpactNumbers::new(bad, 0.001, 0.072, 0.001, 2.0).is_err());
        assert!(ImpactNumbers::new(1000.0, bad, 0.072, 0.001, 2.0).is_err());
        assert!(ImpactNumbers::new(1000.0, 0.001, bad, 0.001, 2.0).is_err());
        assert!(ImpactNumbers::new(1000.0, 0.001, 0.072, bad, 2.0).is_err());
    }
    assert!(ImpactNumbers::new(1000.0, 0.001, 0.072, 0.001, f64::MAX).is_err());
}
