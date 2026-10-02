use physics::{cohesive::Material, plasticity::mesh::QuadraticCohesiveFace};
fn rest() -> Vec<[f64; 3]> {
    let face = [
        [0.; 3],
        [1., 0., 0.],
        [0., 1., 0.],
        [0.5, 0., 0.],
        [0.5, 0.5, 0.],
        [0., 0.5, 0.],
    ];
    face.into_iter().chain(face).collect()
}
fn face() -> QuadraticCohesiveFace {
    QuadraticCohesiveFace::new(
        &rest(),
        [0, 1, 2, 3, 4, 5],
        [6, 7, 8, 9, 10, 11],
        Material::new(1e6, 1e7, 1000., 10.).unwrap(),
    )
    .unwrap()
}
fn rotated(p: [f64; 3]) -> [f64; 3] {
    [p[2], p[1], -p[0]]
}
#[test]
fn uniform_fracture_spends_gc_area_and_history_is_transactional() {
    let face = face();
    let mut positions = rest();
    for p in &mut positions[6..] {
        p[2] += 0.02;
    }
    let trial = face.trial_at(&positions).unwrap();
    assert!((trial.dissipated_j - 5.).abs() < 1e-12);
    assert!(trial.internal_n.iter().flatten().all(|f| f.abs() < 1e-10));
    assert!(trial.quadrature.iter().all(|q| q.damage == 1.));
    assert!(
        face.states()
            .iter()
            .all(|state| state.maximum_separation_m() == 0.)
    );
    let broken = trial.candidate;
    let closed = broken.trial_at(&rest()).unwrap();
    assert!(closed.quadrature.iter().all(|q| q.damage == 1.));
    assert!((closed.dissipated_j - 5.).abs() < 1e-12);
    for p in &mut positions[6..] {
        p[2] = -0.0002;
    }
    let closure = broken.trial_at(&positions).unwrap();
    assert!(closure.stored_j > 0. && closure.quadrature.iter().all(|q| q.damage == 1.));
    assert!(broken.trial_at(&positions[..11]).is_err());
}
#[test]
fn curved_corotated_interface_forces_match_energy_and_preserve_momenta() {
    let face = face();
    let mut positions = rest();
    positions[3][2] = 0.03;
    positions[9][2] = 0.03;
    positions[4][2] = -0.02;
    positions[10][2] = -0.02;
    for (i, p) in positions[6..].iter_mut().enumerate() {
        p[0] += 0.0001 * (1. + i as f64 / 10.);
        p[1] -= 0.00005;
        p[2] -= 0.0002;
    }
    let trial = face.trial_at(&positions).unwrap();
    for node in 0..12 {
        for axis in 0..3 {
            let mut plus = positions.clone();
            let mut minus = positions.clone();
            plus[node][axis] += 1e-7;
            minus[node][axis] -= 1e-7;
            let gradient = (face.trial_at(&plus).unwrap().stored_j
                - face.trial_at(&minus).unwrap().stored_j)
                / 2e-7;
            assert!(
                (gradient - trial.internal_n[node][axis]).abs() < 1e-5,
                "node={node}, axis={axis}, numerical={gradient}, analytic={}",
                trial.internal_n[node][axis]
            );
        }
    }
    let mut force = [0.; 3];
    let mut torque = [0.; 3];
    for (p, f) in positions.iter().zip(&trial.internal_n) {
        for axis in 0..3 {
            force[axis] += f[axis];
        }
        torque[0] += p[1] * f[2] - p[2] * f[1];
        torque[1] += p[2] * f[0] - p[0] * f[2];
        torque[2] += p[0] * f[1] - p[1] * f[0];
    }
    assert!(force.iter().chain(&torque).all(|v| v.abs() < 1e-9));
    let transformed: Vec<_> = positions
        .iter()
        .map(|&p| {
            let p = rotated(p);
            [p[0] + 2., p[1] - 3., p[2] + 4.]
        })
        .collect();
    let moved = face.trial_at(&transformed).unwrap();
    assert!((moved.stored_j - trial.stored_j).abs() < 1e-10);
    for (a, b) in trial.internal_n.iter().zip(moved.internal_n) {
        let a = rotated(*a);
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-8));
    }
}
#[test]
fn invalid_reference_and_degenerate_current_frames_are_rejected() {
    let material = Material::new(1e6, 1e7, 1000., 10.).unwrap();
    let mut wrong = rest();
    wrong[9][0] += 0.1;
    assert!(
        QuadraticCohesiveFace::new(&wrong, [0, 1, 2, 3, 4, 5], [6, 7, 8, 9, 10, 11], material)
            .is_err()
    );
    assert!(
        QuadraticCohesiveFace::new(&rest(), [0, 1, 2, 3, 4, 5], [0, 1, 2, 3, 4, 5], material)
            .is_err()
    );
    assert!(face().trial_at(&vec![[0.; 3]; 12]).is_err());
}

#[test]
fn fractured_closure_friction_history_is_objective_under_rigid_rotation() {
    let material = Material::new(1e6, 1e7, 1000., 10.)
        .unwrap()
        .with_friction(0.4, 1e6)
        .unwrap();
    let face =
        QuadraticCohesiveFace::new(&rest(), [0, 1, 2, 3, 4, 5], [6, 7, 8, 9, 10, 11], material)
            .unwrap();
    let mut opening = rest();
    for p in &mut opening[6..] {
        p[2] += 0.02;
    }
    let broken = face.trial_at(&opening).unwrap().candidate;
    let mut closure = rest();
    for p in &mut closure[6..] {
        p[0] += 0.001;
        p[2] -= 0.0001;
    }
    let slipping = broken.trial_at(&closure).unwrap();
    assert!(slipping.friction_dissipated_j > 0.);
    let accepted = slipping.candidate;
    let held = accepted.trial_at(&closure).unwrap();
    let transformed: Vec<_> = closure
        .iter()
        .map(|&p| {
            let p = rotated(p);
            [p[0] + 2., p[1] - 3., p[2] + 4.]
        })
        .collect();
    let moved = accepted.trial_at(&transformed).unwrap();
    assert!((moved.stored_j - held.stored_j).abs() < 1e-10);
    assert!((moved.friction_dissipated_j - held.friction_dissipated_j).abs() < 1e-10);
    for (a, b) in held.internal_n.iter().zip(&moved.internal_n) {
        let a = rotated(*a);
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-8));
    }
    let mut torque = [0.; 3];
    for (p, f) in transformed.iter().zip(&moved.internal_n) {
        torque[0] += p[1] * f[2] - p[2] * f[1];
        torque[1] += p[2] * f[0] - p[0] * f[2];
        torque[2] += p[0] * f[1] - p[1] * f[0];
    }
    assert!(torque.iter().all(|v| v.abs() < 1e-8));
}

#[test]
fn wet_material_update_preserves_point_history_and_integrated_fracture_work() {
    let mut positions = rest();
    for p in &mut positions[6..] {
        p[2] = 0.0125;
    }
    let initial = face().trial_at(&positions).unwrap();
    let mut accepted = initial.candidate;
    let wet = Material::new(1e6, 1e7, 500., 2.5).unwrap();
    let work = accepted.update_material_at(&positions, wet).unwrap();
    let updated = accepted.trial_at(&positions).unwrap();
    assert!(accepted.is_fully_broken());
    assert!((updated.dissipated_j - initial.dissipated_j).abs() < 1e-12);
    assert!((updated.stored_j - initial.stored_j - work).abs() < 1e-12);
    let before = accepted.states();
    assert!(
        accepted
            .update_material_at(&positions, Material::new(1e6, 1e7, 1000., 10.).unwrap())
            .is_err()
    );
    assert_eq!(accepted.states(), before);
    let closed = accepted.trial_at(&rest()).unwrap();
    assert!(closed.quadrature.iter().all(|q| q.damage == 1.));
    assert!((closed.dissipated_j - initial.dissipated_j).abs() < 1e-12);
}
