use physics::{
    biomechanics::Material as Elastic,
    plasticity::{
        Material,
        mesh::{
            FiniteQuadraticDynamics, QuadraticBody, QuadraticClosestLimits, QuadraticSurfaceContact,
        },
    },
};
fn fixture() -> (FiniteQuadraticDynamics, ([usize; 6], [usize; 6])) {
    moving_fixture([0.; 3], [0.; 3])
}
fn moving_fixture(
    top: [f64; 3],
    bottom: [f64; 3],
) -> (FiniteQuadraticDynamics, ([usize; 6], [usize; 6])) {
    moving_gap_fixture(top, bottom, 0.01)
}
fn moving_gap_fixture(
    top: [f64; 3],
    bottom: [f64; 3],
    gap: f64,
) -> (FiniteQuadraticDynamics, ([usize; 6], [usize; 6])) {
    let material = Material::new(1e5, 0.3, 1e9, 0.).unwrap();
    let body = QuadraticBody::from_linear(
        vec![
            [0., 0., gap],
            [1., 0., gap],
            [0., 1., gap],
            [0., 0., 1. + gap],
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., -1.],
        ],
        vec![([0, 1, 2, 3], material), ([4, 5, 6, 7], material)],
    )
    .unwrap();
    let faces = body.reference_faces().unwrap();
    let a = faces
        .iter()
        .find(|f| f.nodes[..3].iter().all(|n| [0, 1, 2].contains(n)))
        .unwrap()
        .nodes;
    let b = faces
        .iter()
        .find(|f| f.nodes[..3].iter().all(|n| [4, 5, 6].contains(n)))
        .unwrap()
        .nodes;
    let count = body.positions().len();
    let velocities = (0..count)
        .map(|i| {
            if i < 4 || (8..14).contains(&i) {
                top
            } else {
                bottom
            }
        })
        .collect();
    (
        FiniteQuadraticDynamics::new(
            body,
            vec![Elastic::from_young_poisson(1e5, 0.3).unwrap(); 2],
            &[1000., 1000.],
            velocities,
            &vec![false; count],
        )
        .unwrap(),
        (a, b),
    )
}
fn law() -> QuadraticSurfaceContact {
    QuadraticSurfaceContact::new(
        0.02,
        1000.,
        QuadraticClosestLimits {
            distance_tolerance_m: 1e-6,
            max_patches: 4096,
            max_depth: 20,
        },
    )
    .unwrap()
}
fn run(dt: f64, steps: usize) -> f64 {
    let (mut body, pair) = fixture();
    let parameter_work = body.set_surface_contact(Some(law()), &[pair]).unwrap();
    assert!((parameter_work - 0.025).abs() < 1e-12);
    let initial = body.energy().unwrap();
    let mut envelope = 0_f64;
    for _ in 0..steps {
        body.step(dt, [0.; 3], 1e-3).unwrap();
        let e = body.energy().unwrap();
        envelope = envelope.max(
            (e.kinetic_j + e.elastic_j + e.surface_contact_j - initial.surface_contact_j).abs(),
        );
        for axis in 0..3 {
            assert!((e.momentum_kg_m_s[axis] - initial.momentum_kg_m_s[axis]).abs() < 1e-9);
            assert!(
                (e.angular_momentum_kg_m2_s[axis] - initial.angular_momentum_kg_m2_s[axis]).abs()
                    < 1e-9
            );
        }
    }
    let fragments = body.fragments().unwrap();
    assert_eq!(fragments.len(), 2);
    let duration = dt * steps as f64;
    assert!((fragments[0].momentum_kg_m_s[2] - 5. * duration).abs() < 0.02 * 5. * duration);
    assert!((fragments[0].momentum_kg_m_s[2] + fragments[1].momentum_kg_m_s[2]).abs() < 1e-9);
    assert!(body.energy().unwrap().surface_contact_j < initial.surface_contact_j);
    envelope
}
#[test]
fn free_surface_contact_exchanges_impulse_and_energy_refines() {
    let coarse = run(2e-4, 10);
    let fine = run(1e-4, 20);
    println!("moving surface contact energy envelopes: {coarse:e}, {fine:e}");
    assert!(fine < coarse / 2. && fine < 1e-6);
}
#[test]
fn contact_parameters_and_rejected_steps_are_atomic() {
    let (mut body, pair) = fixture();
    body.set_surface_contact(Some(law()), &[pair]).unwrap();
    let positions = body.positions().to_vec();
    let velocities = body.velocities().to_vec();
    let energy = body.energy().unwrap().surface_contact_j;
    assert!(
        body.set_surface_contact(Some(law()), &[pair, (pair.1, pair.0)])
            .is_err()
    );
    assert!(body.set_surface_contact(None, &[pair]).is_err());
    assert!(body.step(0.1, [0.; 3], 1e-15).is_err());
    assert_eq!(body.positions(), positions);
    assert_eq!(body.velocities(), velocities);
    assert_eq!(body.energy().unwrap().surface_contact_j, energy);
    assert!((body.set_surface_contact(None, &[]).unwrap() + energy).abs() < 1e-12);
    assert_eq!(body.energy().unwrap().surface_contact_j, 0.);
}

#[test]
fn selected_bonded_faces_activate_only_after_full_fracture() {
    use physics::{cohesive::Material as Bond, plasticity::mesh::QuadraticDynamics};
    let material = Material::new(1e5, 0.3, 1e9, 0.).unwrap();
    let mesh = QuadraticBody::from_linear_with_cohesive_faces(
        vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0., 0., -1.],
        ],
        vec![([0, 1, 2, 3], material), ([0, 2, 1, 4], material)],
        Bond::new(1e6, 2e6, 1000., 10.).unwrap(),
    )
    .unwrap();
    let pair = mesh.body.cohesive_interfaces()[0].sides();
    let mut intact =
        QuadraticDynamics::new(mesh.body.clone(), &[1000., 1000.], vec![[0.; 3]; 20]).unwrap();
    assert_eq!(
        intact.set_surface_contact(Some(law()), &[pair]).unwrap(),
        0.
    );
    intact.step(1e-4, [0.; 3], 1e-12).unwrap();
    assert_eq!(intact.energy().unwrap().surface_contact_j, 0.);
    assert_eq!(
        intact.set_automatic_surface_contact(Some(law())).unwrap(),
        0.
    );
    intact.step(1e-4, [0.; 3], 1e-12).unwrap();
    assert_eq!(intact.energy().unwrap().surface_contact_j, 0.);
    let mut broken = mesh.body;
    let mut prescribed = vec![[Some(0.); 3]; 20];
    for &node in &mesh.cell_nodes[0] {
        prescribed[node][2] = Some(0.02);
    }
    assert!(
        broken
            .equilibrate(&vec![[0.; 3]; 20], &prescribed, 1, 1e-9)
            .unwrap()
            .converged
    );
    let mut separated = QuadraticDynamics::new(broken, &[1000., 1000.], vec![[0.; 3]; 20]).unwrap();
    let thicker = QuadraticSurfaceContact::new(
        0.03,
        1000.,
        QuadraticClosestLimits {
            distance_tolerance_m: 1e-6,
            max_patches: 4096,
            max_depth: 20,
        },
    )
    .unwrap();
    assert!(
        (separated
            .set_surface_contact(Some(thicker), &[pair])
            .unwrap()
            - 0.025)
            .abs()
            < 1e-10
    );
    assert_eq!(separated.fragments().unwrap().len(), 2);
    assert!(
        separated
            .set_automatic_surface_contact(Some(thicker))
            .unwrap()
            .abs()
            < 1e-10
    );
    separated.step(1e-4, [0.; 3], 1e-6).unwrap();
    assert!(separated.fragments().unwrap()[0].momentum_kg_m_s[2] > 0.);
}

#[test]
fn automatic_fragments_match_selected_contact_without_manual_pairs() {
    let (mut automatic, pair) = fixture();
    let mut selected = automatic.clone();
    let a = automatic
        .set_automatic_surface_contact(Some(law()))
        .unwrap();
    let b = selected.set_surface_contact(Some(law()), &[pair]).unwrap();
    assert!((a - b).abs() < 1e-12);
    for _ in 0..10 {
        automatic.step(1e-4, [0.; 3], 1e-6).unwrap();
        selected.step(1e-4, [0.; 3], 1e-6).unwrap();
    }
    for (a, b) in automatic.velocities().iter().zip(selected.velocities()) {
        for axis in 0..3 {
            assert!((a[axis] - b[axis]).abs() < 1e-10);
        }
    }
    let old = automatic.energy().unwrap().surface_contact_j;
    assert!((automatic.set_automatic_surface_contact(None).unwrap() + old).abs() < 1e-12);
}

#[test]
fn fast_surface_motion_is_rejected_even_with_loose_energy_tolerance() {
    for automatic in [false, true] {
        let (mut body, pair) = moving_fixture([0., 0., -2.], [0., 0., 2.]);
        if automatic {
            body.set_automatic_surface_contact(Some(law())).unwrap();
        } else {
            body.set_surface_contact(Some(law()), &[pair]).unwrap();
        }
        let positions = body.positions().to_vec();
        let velocities = body.velocities().to_vec();
        assert_eq!(
            body.step(0.005, [0.; 3], 1e6).unwrap_err(),
            "quadratic surface motion limit reached"
        );
        assert_eq!(body.positions(), positions);
        assert_eq!(body.velocities(), velocities);
    }
}
#[test]
fn adaptive_motion_guard_refines_and_common_translation_does_not_reduce_step() {
    use physics::plasticity::mesh::QuadraticAdvanceLimits;
    let (mut body, _) = moving_fixture([0., 0., 10.], [0., 0., -10.]);
    body.set_automatic_surface_contact(Some(law())).unwrap();
    let mut reference = body.clone();
    let loads = vec![[0.; 3]; body.positions().len()];
    let report = body
        .advance_loaded(
            0.002,
            &loads,
            [0.; 3],
            QuadraticAdvanceLimits {
                minimum_dt_s: 1e-8,
                maximum_dt_s: 0.002,
                max_attempts: 256,
                energy_tolerance_j: 0.01,
            },
        )
        .unwrap();
    assert!(report.attempts > report.substeps.len());
    assert!(report.substeps.iter().all(|s| s.dt_s < 0.002));
    for step in &report.substeps {
        reference.step(step.dt_s, [0.; 3], 0.01).unwrap();
    }
    assert_eq!(body.positions(), reference.positions());
    assert_eq!(body.velocities(), reference.velocities());
    let (mut translated, pair) = moving_fixture([1000., 0., 0.], [1000., 0., 0.]);
    translated
        .set_surface_contact(Some(law()), &[pair])
        .unwrap();
    let (mut stationary, pair) = fixture();
    stationary
        .set_surface_contact(Some(law()), &[pair])
        .unwrap();
    translated.step(1e-4, [0.; 3], 1e-6).unwrap();
    stationary.step(1e-4, [0.; 3], 1e-6).unwrap();
    for (a, b) in translated.velocities().iter().zip(stationary.velocities()) {
        assert!((a[0] - b[0] - 1000.).abs() < 1e-9);
        for axis in 1..3 {
            assert!((a[axis] - b[axis]).abs() < 1e-9);
        }
    }
}

fn sweep_law(intervals: usize) -> QuadraticSurfaceContact {
    use physics::plasticity::mesh::QuadraticSweepLimits;
    let closest = QuadraticClosestLimits {
        distance_tolerance_m: 1e-9,
        max_patches: 8192,
        max_depth: 24,
    };
    QuadraticSurfaceContact::new(0.02, 1000., closest)
        .unwrap()
        .with_sweep_guard(
            1e-8,
            QuadraticSweepLimits {
                closest,
                minimum_time_fraction: 1e-10,
                max_intervals: intervals,
            },
        )
        .unwrap()
}
#[test]
fn continuous_guard_rejects_small_drift_crossings_in_both_selection_modes() {
    for automatic in [false, true] {
        let (mut body, pair) = moving_gap_fixture([0., 0., -0.01], [0., 0., 0.01], 1e-6);
        if automatic {
            body.set_automatic_surface_contact(Some(sweep_law(2048)))
                .unwrap();
        } else {
            body.set_surface_contact(Some(sweep_law(2048)), &[pair])
                .unwrap();
        }
        let positions = body.positions().to_vec();
        let velocities = body.velocities().to_vec();
        assert_eq!(
            body.step(1e-4, [0.; 3], 1e6).unwrap_err(),
            "quadratic surface sweep clearance reached"
        );
        assert_eq!(body.positions(), positions);
        assert_eq!(body.velocities(), velocities);
    }
}
#[test]
fn unresolved_guard_is_rejected_and_safe_motion_matches_unguarded_dynamics() {
    let (mut body, pair) = moving_gap_fixture([0., 0., -0.01], [0., 0., 0.01], 1e-6);
    body.set_surface_contact(Some(sweep_law(1)), &[pair])
        .unwrap();
    assert_eq!(
        body.step(1.5e-4, [0.; 3], 1e6).unwrap_err(),
        "quadratic surface sweep unresolved"
    );
    let (mut guarded, _) = fixture();
    let mut unguarded = guarded.clone();
    guarded
        .set_automatic_surface_contact(Some(sweep_law(2048)))
        .unwrap();
    unguarded
        .set_automatic_surface_contact(Some(law()))
        .unwrap();
    for _ in 0..3 {
        guarded.step(1e-4, [0.; 3], 1e-6).unwrap();
        unguarded.step(1e-4, [0.; 3], 1e-6).unwrap();
    }
    for (a, b) in guarded.velocities().iter().zip(unguarded.velocities()) {
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 1e-9);
        }
    }
    assert!(
        law()
            .with_sweep_guard(
                0.,
                physics::plasticity::mesh::QuadraticSweepLimits {
                    closest: QuadraticClosestLimits {
                        distance_tolerance_m: 1e-9,
                        max_patches: 8192,
                        max_depth: 24
                    },
                    minimum_time_fraction: 1e-10,
                    max_intervals: 2048
                }
            )
            .is_err()
    );
}

#[test]
fn guarded_adaptive_interval_rolls_back_when_penalty_cannot_stop_approach() {
    use physics::plasticity::mesh::QuadraticAdvanceLimits;
    let (mut body, pair) = moving_gap_fixture([0., 0., -0.01], [0., 0., 0.01], 1e-6);
    body.set_surface_contact(Some(sweep_law(2048)), &[pair])
        .unwrap();
    let positions = body.positions().to_vec();
    let velocities = body.velocities().to_vec();
    let loads = vec![[0.; 3]; positions.len()];
    let error = body
        .advance_loaded(
            1e-4,
            &loads,
            [0.; 3],
            QuadraticAdvanceLimits {
                minimum_dt_s: 1e-8,
                maximum_dt_s: 1e-4,
                max_attempts: 512,
                energy_tolerance_j: 1.,
            },
        )
        .unwrap_err();
    assert_eq!(error, "quadratic adaptive minimum timestep reached");
    assert_eq!(body.positions(), positions);
    assert_eq!(body.velocities(), velocities);
}
