use physics::{
    cohesive::State,
    moisture::{CohesiveCalibration, CohesiveProperties},
};
#[test]
fn calibrated_wet_interface_has_lower_peak_and_fracture_work() {
    let dry = CohesiveProperties {
        stiffness_pa_m: 1e9,
        closure_pa_m: 2e9,
        peak_pa: 1e5,
        fracture_j_m2: 10.,
    };
    let wet = CohesiveProperties {
        peak_pa: 5e4,
        fracture_j_m2: 2.5,
        ..dry
    };
    let calibration = CohesiveCalibration::new(dry, wet).unwrap();
    for (s, peak, toughness) in [(0., 1e5, 10.), (0.5, 7.5e4, 6.25), (1., 5e4, 2.5)] {
        let law = calibration.at(s).unwrap();
        let (_, onset) = law
            .response(&State::default(), [law.onset_m(), 0., 0.], [1., 0., 0.])
            .unwrap();
        assert!((onset.traction_pa[0] - peak).abs() < 1e-8);
        let (broken, failed) = law
            .response(&State::default(), [law.failure_m(), 0., 0.], [1., 0., 0.])
            .unwrap();
        assert_eq!(failed.damage, 1.);
        assert!((failed.dissipated_j_m2 - toughness).abs() < 1e-12);
        let (_, closed) = law.response(&broken, [0.; 3], [1., 0., 0.]).unwrap();
        assert_eq!(closed.damage, 1.);
    }
    assert!(calibration.at(f64::NAN).is_err());
    assert!(calibration.at(1.1).is_err());
}

#[test]
fn wet_law_migration_preserves_fracture_work_and_rejects_healing() {
    let dry = CohesiveProperties {
        stiffness_pa_m: 1e9,
        closure_pa_m: 2e9,
        peak_pa: 1e5,
        fracture_j_m2: 10.,
    };
    let wet = CohesiveProperties {
        peak_pa: 5e4,
        fracture_j_m2: 2.5,
        ..dry
    };
    let calibration = CohesiveCalibration::new(dry, wet).unwrap();
    let dry = calibration.at(0.).unwrap();
    let wet = calibration.at(1.).unwrap();
    let jump = [0.000125, 0., 0.];
    let normal = [1., 0., 0.];
    let (old, before) = dry.response(&State::default(), jump, normal).unwrap();
    let (migrated, work) = wet.migrate_history(&dry, &old, jump, normal).unwrap();
    let (_, after) = wet.response(&migrated, jump, normal).unwrap();
    assert_eq!(after.damage, 1.);
    assert!((after.dissipated_j_m2 - before.dissipated_j_m2).abs() < 1e-12);
    assert!((after.stored_j_m2 - before.stored_j_m2 - work).abs() < 1e-12);
    let (_, closed) = wet.response(&migrated, [0.; 3], normal).unwrap();
    assert_eq!(closed.damage, 1.);
    assert!((closed.dissipated_j_m2 - after.dissipated_j_m2).abs() < 1e-12);
    assert!(dry.migrate_history(&wet, &migrated, jump, normal).is_err());
    assert!(
        wet.migrate_history(&dry, &old, [0.001, 0., 0.], normal)
            .is_err()
    );
}

#[test]
fn calibrated_heating_migrates_damage_without_erasing_fracture_work() {
    use physics::moisture::ThermalCohesiveCalibration;
    let cold = CohesiveProperties {
        stiffness_pa_m: 1e9,
        closure_pa_m: 2e9,
        peak_pa: 1e5,
        fracture_j_m2: 10.,
    };
    let hot = CohesiveProperties {
        peak_pa: 5e4,
        fracture_j_m2: 2.5,
        ..cold
    };
    let law = ThermalCohesiveCalibration::new(
        300.,
        600.,
        CohesiveCalibration::new(cold, cold).unwrap(),
        CohesiveCalibration::new(hot, hot).unwrap(),
    )
    .unwrap();
    let cold = law.at_temperature(300.).unwrap().at(0.5).unwrap();
    let hot = law.at_temperature(600.).unwrap().at(0.5).unwrap();
    let jump = [0.000125, 0., 0.];
    let n = [1., 0., 0.];
    let (state, before) = cold.response(&State::default(), jump, n).unwrap();
    let (heated, work) = hot.migrate_history(&cold, &state, jump, n).unwrap();
    let (_, after) = hot.response(&heated, jump, n).unwrap();
    assert_eq!(after.damage, 1.);
    assert!((after.dissipated_j_m2 - before.dissipated_j_m2).abs() < 1e-12);
    assert!((after.stored_j_m2 - before.stored_j_m2 - work).abs() < 1e-12);
    assert!(cold.migrate_history(&hot, &heated, jump, n).is_err());
    assert!(law.at_temperature(601.).is_err());
    assert!(law.at_temperature(f64::NAN).is_err());
}
