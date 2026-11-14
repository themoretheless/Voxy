use physics::plasticity::mesh::QuadraticAdvanceLimits;
#[test]
fn interval_partition_preserves_total_budget_without_energy_floor() {
    let limits = QuadraticAdvanceLimits {
        minimum_dt_s: 1e-8,
        maximum_dt_s: 0.01,
        max_attempts: 17,
        energy_tolerance_j: 1.,
    };
    let whole = limits.with_interval_energy_rate(0.125, 0.03125).unwrap();
    let child = limits
        .with_interval_energy_rate(0.125 / 64., 0.03125)
        .unwrap();
    assert_eq!(child.energy_tolerance_j * 64., whole.energy_tolerance_j);
    assert_eq!(whole.minimum_dt_s, limits.minimum_dt_s);
    assert_eq!(whole.maximum_dt_s, limits.maximum_dt_s);
    assert_eq!(whole.max_attempts, 17);
    assert!(
        limits
            .with_interval_energy_rate(f64::MIN_POSITIVE, f64::MIN_POSITIVE)
            .is_err()
    );
    assert!(limits.with_interval_energy_rate(f64::MAX, 2.).is_err());
    for invalid in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(limits.with_interval_energy_rate(invalid, 1.).is_err());
        assert!(limits.with_interval_energy_rate(1., invalid).is_err());
    }
}
