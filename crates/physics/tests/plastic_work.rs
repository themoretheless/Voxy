use physics::{
    biomechanics::Matrix,
    plasticity::{Material, State},
};
fn shear(g: f64) -> Matrix {
    [[0., g / 2., 0.], [g / 2., 0., 0.], [0.; 3]]
}
#[test]
fn elastic_endpoint_excess_is_numerical_not_heat() {
    let m = Material::new(210e9, 0.3, 250e6, 1e9).unwrap();
    let step = m
        .response_with_work(&State::default(), shear(0.), shear(1e-4))
        .unwrap();
    let stored = 0.5 * (210e9 / 2.6) * 1e-8;
    assert!((step.elastic_energy_change_j_m3 - stored).abs() < stored * 1e-12);
    assert!((step.numerical_loss_j_m3 - stored).abs() < stored * 1e-12);
    assert_eq!(step.plastic_heat_j_m3, 0.);
    assert_eq!(step.state, State::default());
}
fn cycle(divisions: usize) -> (f64, f64) {
    let m = Material::new(210e9, 0.3, 250e6, 1e9).unwrap();
    let mut state = State::default();
    let mut previous = 0.;
    let (mut work, mut heat, mut numerical) = (0., 0., 0.);
    let mut last = None;
    for target in [0.01, 0., -0.01, 0.] {
        let start = previous;
        for i in 1..=divisions {
            let strain = start + (target - start) * i as f64 / divisions as f64;
            let s = m
                .response_with_work(&state, shear(previous), shear(strain))
                .unwrap();
            work += s.endpoint_work_j_m3;
            heat += s.plastic_heat_j_m3;
            numerical += s.numerical_loss_j_m3;
            assert!(s.plastic_heat_j_m3 >= 0.);
            state = s.state;
            previous = strain;
            last = Some(s);
        }
    }
    let s = last.unwrap();
    let storage = s.response.elastic_energy_j_m3 + s.response.hardening_energy_j_m3;
    assert!((work - storage - heat - numerical).abs() < 1e-10 * work.abs());
    assert!((heat - state.dissipated_j_m3()).abs() < 1e-10 * heat);
    assert!(state.equivalent_plastic_strain() > 0.);
    (numerical, heat)
}
#[test]
fn cyclic_reverse_flow_balances_work_and_refinement_reduces_numerical_loss() {
    let coarse = cycle(16);
    let fine = cycle(64);
    let finer = cycle(256);
    assert!(fine.0 < coarse.0 && finer.0 < fine.0);
    assert!((finer.1 - fine.1).abs() < finer.1 * 0.01);
}
#[test]
fn incompatible_history_and_invalid_trial_do_not_commit() {
    let m = Material::new(210e9, 0.3, 250e6, 1e9).unwrap();
    let old = State::default();
    assert!(m.response_with_work(&old, shear(0.01), shear(0.)).is_err());
    assert!(
        m.response_with_work(&old, shear(0.), shear(f64::NAN))
            .is_err()
    );
    assert_eq!(old, State::default());
}

#[test]
fn analytical_plastic_partition_preserves_hardening_storage() {
    let g = 210e9 / 2.6;
    let yield_pa = 250e6;
    let hardening = 1e9;
    let m = Material::new(210e9, 0.3, yield_pa, hardening).unwrap();
    let strain = shear(2. * yield_pa / (3_f64.sqrt() * g));
    let s = m
        .response_with_work(&State::default(), shear(0.), strain)
        .unwrap();
    let dp = yield_pa / (3. * g + hardening);
    assert!((s.plastic_heat_j_m3 - yield_pa * dp).abs() < 1e-12 * yield_pa * dp);
    assert!(
        (s.hardening_energy_change_j_m3 - 0.5 * hardening * dp * dp).abs()
            < 1e-12 * hardening * dp * dp
    );
    let plastic_work = (yield_pa + hardening * dp) * dp;
    assert!(
        (plastic_work
            - s.plastic_heat_j_m3
            - s.hardening_energy_change_j_m3
            - 0.5 * hardening * dp * dp)
            .abs()
            < 1e-12 * plastic_work
    );
}
#[test]
fn hydrostatic_compression_produces_no_j2_plastic_heat() {
    let m = Material::new(210e9, 0.3, 250e6, 1e9).unwrap();
    let strain = [[-0.001, 0., 0.], [0., -0.001, 0.], [0., 0., -0.001]];
    let s = m
        .response_with_work(&State::default(), [[0.; 3]; 3], strain)
        .unwrap();
    assert_eq!(s.state, State::default());
    assert_eq!(s.plastic_heat_j_m3, 0.);
    assert_eq!(s.hardening_energy_change_j_m3, 0.);
    let expected = 0.5 * (210e9 / (3. * (1. - 2. * 0.3))) * 0.003_f64.powi(2);
    assert!((s.elastic_energy_change_j_m3 - expected).abs() < 1e-12 * expected);
}
