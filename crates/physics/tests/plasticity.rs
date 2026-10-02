use physics::biomechanics::Matrix;
use physics::plasticity::{Material, State};
const YOUNG: f64 = 210e9;
const POISSON: f64 = 0.3;
const YIELD: f64 = 250e6;
const HARDENING: f64 = 1e9;
fn shear(gamma: f64) -> Matrix {
    [[0., gamma / 2., 0.], [gamma / 2., 0., 0.], [0.; 3]]
}
fn close(a: f64, b: f64, relative: f64) {
    assert!((a - b).abs() <= relative * b.abs().max(1.), "{a} != {b}");
}
#[test]
fn analytical_return_mapping_and_residual_strain_after_unloading() {
    let m = Material::new(YOUNG, POISSON, YIELD, HARDENING).unwrap();
    let g = YOUNG / (2. * (1. + POISSON));
    let gamma_y = YIELD / (3_f64.sqrt() * g);
    let (state, response) = m.response(&State::default(), shear(2. * gamma_y)).unwrap();
    let expected_increment = YIELD / (3. * g + HARDENING);
    close(response.plastic_increment, expected_increment, 1e-12);
    close(
        response.stress.von_mises_pa,
        YIELD + HARDENING * expected_increment,
        1e-12,
    );
    close(
        response.stress.cauchy_pa[0][1],
        response.yield_stress_pa / 3_f64.sqrt(),
        1e-12,
    );
    close(
        state
            .plastic_strain()
            .iter()
            .enumerate()
            .map(|(i, r)| r[i])
            .sum(),
        0.,
        1e-14,
    );
    let (unloaded, r) = m.response(&state, state.plastic_strain()).unwrap();
    close(r.stress.von_mises_pa, 0., 1e-6);
    assert_eq!(unloaded, state);
    assert!(state.plastic_strain()[0][1] > 0.);
    close(state.dissipated_j_m3(), YIELD * expected_increment, 1e-12);
    close(
        r.hardening_energy_j_m3,
        0.5 * HARDENING * expected_increment.powi(2),
        1e-12,
    );
    // Hold the loaded strain: no creep or repeated history accumulation.
    let (held, _) = m.response(&state, shear(2. * gamma_y)).unwrap();
    close(
        held.equivalent_plastic_strain(),
        state.equivalent_plastic_strain(),
        1e-14,
    );
}
#[test]
fn monotonic_work_equals_elastic_hardening_and_irreversible_energy() {
    let m = Material::new(YOUNG, POISSON, YIELD, HARDENING).unwrap();
    let g = YOUNG / (2. * (1. + POISSON));
    let gamma_y = YIELD / (3_f64.sqrt() * g);
    let mut state = State::default();
    let mut last_stress = 0.;
    let mut work = 0.;
    for i in 1..=100 {
        let gamma = 2. * gamma_y * f64::from(i) / 100.;
        let (next, r) = m.response(&state, shear(gamma)).unwrap();
        let stress = r.stress.cauchy_pa[0][1];
        work += (last_stress + stress) * 0.5 * (2. * gamma_y / 100.);
        assert!(next.dissipated_j_m3() >= state.dissipated_j_m3());
        close(
            work,
            r.elastic_energy_j_m3 + r.hardening_energy_j_m3 + next.dissipated_j_m3(),
            1e-11,
        );
        last_stress = stress;
        state = next;
    }
    let (one, _) = m.response(&State::default(), shear(2. * gamma_y)).unwrap();
    close(
        state.equivalent_plastic_strain(),
        one.equivalent_plastic_strain(),
        1e-13,
    );
    for (a, b) in state
        .plastic_strain()
        .iter()
        .flatten()
        .zip(one.plastic_strain().iter().flatten())
    {
        close(*a, *b, 1e-13);
    }
}
#[test]
fn hydrostatic_load_does_not_yield_and_reverse_loading_does() {
    let m = Material::new(YOUNG, POISSON, YIELD, 0.).unwrap();
    let (hydro, r) = m
        .response(
            &State::default(),
            [[0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
        )
        .unwrap();
    assert_eq!(hydro, State::default());
    assert!(r.stress.von_mises_pa < 1e-5);
    let (forward, _) = m.response(&State::default(), shear(0.01)).unwrap();
    let (reverse, r) = m.response(&forward, shear(-0.01)).unwrap();
    assert!(reverse.equivalent_plastic_strain() > forward.equivalent_plastic_strain());
    assert!(reverse.dissipated_j_m3() > forward.dissipated_j_m3());
    close(r.stress.von_mises_pa, YIELD, 1e-12);
    assert!(r.stress.cauchy_pa[0][1] < 0.);
}
fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}
fn rotate(a: Matrix) -> Matrix {
    let rotation = [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0., 0.6]];
    let transpose = std::array::from_fn(|i| std::array::from_fn(|j| rotation[j][i]));
    multiply(multiply(rotation, a), transpose)
}
#[test]
fn tensor_basis_covariance_over_nonproportional_history() {
    let m = Material::new(YOUNG, POISSON, YIELD, HARDENING).unwrap();
    let mut a = State::default();
    let mut b = State::default();
    for strain in [
        shear(0.01),
        [[0.003, 0., 0.], [0., -0.002, 0.001], [0., 0.001, -0.001]],
        shear(-0.005),
    ] {
        let (next_a, r_a) = m.response(&a, strain).unwrap();
        let (next_b, r_b) = m.response(&b, rotate(strain)).unwrap();
        for (x, y) in r_b
            .stress
            .cauchy_pa
            .iter()
            .flatten()
            .zip(rotate(r_a.stress.cauchy_pa).iter().flatten())
        {
            close(*x, *y, 1e-10);
        }
        close(
            next_a.equivalent_plastic_strain(),
            next_b.equivalent_plastic_strain(),
            1e-12,
        );
        close(next_a.dissipated_j_m3(), next_b.dissipated_j_m3(), 1e-12);
        a = next_a;
        b = next_b;
    }
}
#[test]
fn invalid_trials_leave_accepted_history_untouched() {
    let m = Material::new(YOUNG, POISSON, YIELD, HARDENING).unwrap();
    let old = m.response(&State::default(), shear(0.01)).unwrap().0;
    let copy = old;
    for strain in [
        [[f64::NAN; 3]; 3],
        [[f64::MAX; 3]; 3],
        [[0., 1., 0.], [0.; 3], [0.; 3]],
    ] {
        assert!(m.response(&old, strain).is_err());
    }
    assert_eq!(old, copy);
    let trial_a = m.response(&old, shear(0.02)).unwrap();
    let trial_b = m.response(&old, shear(0.02)).unwrap();
    assert_eq!(trial_a.0, trial_b.0);
    for (yield_pa, h) in [
        (0., 0.),
        (-1., 0.),
        (1., -1.),
        (f64::INFINITY, 0.),
        (1., f64::NAN),
    ] {
        assert!(Material::new(YOUNG, POISSON, yield_pa, h).is_err());
    }
}

#[test]
fn consistent_tangent_matches_directional_stress_derivative() {
    let m = Material::new(YOUNG, POISSON, YIELD, HARDENING).unwrap();
    let old = m.response(&State::default(), shear(0.002)).unwrap().0;
    let direction = [[0.2, 0.3, 0.1], [0.3, -0.1, 0.2], [0.1, 0.2, -0.1]];
    for strain in [shear(0.001), shear(0.008)] {
        let delta = 1e-8;
        let plus: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| strain[i][j] + delta * direction[i][j])
        });
        let minus: Matrix = std::array::from_fn(|i| {
            std::array::from_fn(|j| strain[i][j] - delta * direction[i][j])
        });
        let upper = m.response(&old, plus).unwrap().1.stress.cauchy_pa;
        let lower = m.response(&old, minus).unwrap().1.stress.cauchy_pa;
        let tangent = m.tangent_action(&old, strain, direction).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                close(
                    tangent[i][j],
                    (upper[i][j] - lower[i][j]) / (2. * delta),
                    1e-8,
                );
            }
        }
    }
}
