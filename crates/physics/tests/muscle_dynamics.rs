use physics::biomechanics::*;
fn law() -> ActiveFiberVelocityLaw {
    ActiveFiberVelocityLaw {
        max_shortening_per_s: 2.,
        shortening_curvature: 0.25,
        eccentric_limit: 1.5,
        eccentric_rate_per_s: 1.,
    }
}
fn specimen() -> InertialBody {
    let x = vec![[0.; 3], [0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]];
    let v = x.iter().map(|p| [-0.5 * (p[0] - 0.025), 0., 0.]).collect();
    let mut body = Body::new(
        x,
        vec![false; 4],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 100.,
                bulk_pa: 1000.,
                fibers: vec![Fiber {
                    direction: [1., 0., 0.],
                    stiffness_pa: 30.,
                    exponent: 2.,
                    active_pa: 200.,
                }],
            },
        )],
    )
    .unwrap();
    body.set_activation(0, 0.5).unwrap();
    InertialBody::new(body, &[1000.], v).unwrap()
}
fn run(steps: usize) -> (InertialBody, f64) {
    let mut b = specimen();
    let initial = b.diagnostics().unwrap();
    let mut work = 0.;
    for _ in 0..steps {
        let report = b.step_muscle(0.02 / steps as f64, 1., law()).unwrap();
        assert!(report.correction_work_j <= 0.);
        work += report.correction_work_j;
    }
    let after = b.diagnostics().unwrap();
    for axis in 0..3 {
        assert!((after.momentum_kg_m_s[axis] - initial.momentum_kg_m_s[axis]).abs() < 1e-13);
    }
    let defect =
        after.kinetic_j + after.potential_j - initial.kinetic_j - initial.potential_j - work;
    assert!(work < 0.);
    (b, defect.abs())
}
fn difference(a: &InertialBody, b: &InertialBody) -> f64 {
    a.body()
        .positions()
        .iter()
        .flatten()
        .chain(a.velocities().iter().flatten())
        .zip(
            b.body()
                .positions()
                .iter()
                .flatten()
                .chain(b.velocities().iter().flatten()),
        )
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}
#[test]
fn rate_dependent_motion_and_work_balance_converge() {
    let (coarse, ec) = run(20);
    let (medium, em) = run(40);
    let (fine, ef) = run(80);
    let ratio = difference(&coarse, &medium) / difference(&medium, &fine);
    println!("trajectory ratio={ratio}, work defects={ec:e},{em:e},{ef:e}");
    assert!(ratio > 3.5 && ratio < 4.5);
    assert!(ec / em > 3. && em / ef > 3.);
    assert!(fine.body().positions()[1][0] < 0.1);
}
#[test]
fn rejected_muscle_steps_preserve_all_motion_state() {
    let mut b = specimen();
    let original = b.clone();
    assert!(b.step_muscle(0.01, 1e-30, law()).is_err());
    assert_eq!(b.body().positions(), original.body().positions());
    assert_eq!(b.velocities(), original.velocities());
    assert!(b.step_muscle(0., 1., law()).is_err());
    let mut bad = law();
    bad.shortening_curvature = 0.;
    assert!(b.step_muscle(0.001, 1., bad).is_err());
    assert_eq!(b.body().positions(), original.body().positions());
    assert_eq!(b.velocities(), original.velocities());
}

fn drive(excitation: f64) -> MuscleRegionDrive {
    MuscleRegionDrive {
        region: 0,
        activation: 0.5,
        excitation,
        kinetics: ActivationKinetics {
            rise_seconds: 0.01,
            fall_seconds: 0.03,
            tonic_activation: 0.,
        },
    }
}
fn driven_run(steps: usize) -> (InertialBody, f64) {
    let mut b = specimen();
    let initial = b.diagnostics().unwrap();
    let mut drives = [drive(0.9)];
    let mut work = 0.;
    let mut activation_work = 0.;
    for _ in 0..steps {
        let report = b
            .step_driven_muscle(&mut drives, 0.02 / steps as f64, 1., law())
            .unwrap();
        work += report.activation_work_j + report.correction_work_j;
        activation_work += report.activation_work_j;
    }
    assert!((drives[0].activation - (0.9 - 0.4 * (-2_f64).exp())).abs() < 1e-14);
    assert_eq!(b.body().elements()[0].activation, drives[0].activation);
    // Contracted active potential is negative; rising activation lowers it.
    assert!(activation_work < 0.);
    let final_state = b.diagnostics().unwrap();
    let defect = final_state.kinetic_j - initial.kinetic_j + final_state.potential_j
        - initial.potential_j
        - work;
    (b, defect.abs())
}
#[test]
fn excitation_activation_motion_split_has_second_order_work_balance() {
    let (a, ea) = driven_run(20);
    let (b, eb) = driven_run(40);
    let (c, ec) = driven_run(80);
    let ratio = difference(&a, &b) / difference(&b, &c);
    println!("driven trajectory ratio={ratio}, defects={ea:e},{eb:e},{ec:e}");
    assert!(ratio > 3.5 && ratio < 4.5);
    assert!(ea / eb > 3. && eb / ec > 3.);
    let (constant, _) = run(80);
    assert!(difference(&constant, &c) > 1e-5);
}
#[test]
fn driven_failures_rollback_histories_and_full_solid_state() {
    let mut b = specimen();
    let original = b.clone();
    let mut drives = [drive(0.9)];
    assert!(
        b.step_driven_muscle(&mut drives, 0.01, 1e-30, law())
            .is_err()
    );
    assert_eq!(drives[0].activation, 0.5);
    assert_eq!(b.body().positions(), original.body().positions());
    assert_eq!(b.velocities(), original.velocities());
    assert_eq!(b.body().elements()[0].activation, 0.5);
    let mut missing = drive(0.9);
    missing.region = 12;
    let mut late_invalid = [drive(0.9), missing];
    assert!(
        b.step_driven_muscle(&mut late_invalid, 0.001, 1., law())
            .is_err()
    );
    assert_eq!(late_invalid[0].activation, 0.5);
    assert_eq!(b.body().elements()[0].activation, 0.5);
    drives[0].activation = 0.4;
    assert!(b.step_driven_muscle(&mut drives, 0.001, 1., law()).is_err());
    assert_eq!(b.body().positions(), original.body().positions());
    assert_eq!(b.velocities(), original.velocities());
}

#[test]
fn constant_excitation_reduces_to_held_activation_and_release_is_exact() {
    let mut held = specimen();
    let mut driven = held.clone();
    let mut drives = [drive(0.5)];
    let report = driven
        .step_driven_muscle(&mut drives, 0.001, 1., law())
        .unwrap();
    held.step_muscle(0.001, 1., law()).unwrap();
    assert_eq!(report.activation_work_j, 0.);
    assert_eq!(driven.body().positions(), held.body().positions());
    assert_eq!(driven.velocities(), held.velocities());
    drives[0].excitation = 0.;
    driven
        .step_driven_muscle(&mut drives, 0.001, 1., law())
        .unwrap();
    assert!((drives[0].activation - 0.5 * (-0.001_f64 / 0.03).exp()).abs() < 1e-14);
}

fn lumen(b: &Body) -> f64 {
    b.cavities()[0]
        .faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|i| b.positions()[i]);
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.
        })
        .sum()
}
fn ring_run(steps: usize) -> (InertialBody, f64) {
    let mut tissue = sphincter_layers().unwrap();
    for i in 0..tissue.elements().len() {
        tissue
            .set_active_fiber_length_law(
                i,
                ActiveFiberLengthLaw {
                    optimal_stretch: 1.,
                    half_width: 0.5,
                },
            )
            .unwrap();
    }
    let initial = lumen(&tissue);
    let density = vec![1000.; tissue.elements().len()];
    let velocities = vec![[0.; 3]; tissue.positions().len()];
    let mut body = InertialBody::new_with_fixed_supports(tissue, &density, velocities).unwrap();
    let mut drives = [0, 1].map(|region| MuscleRegionDrive {
        region,
        activation: 0.,
        excitation: if region == 0 { 0.2 } else { 0.1 },
        kinetics: ActivationKinetics {
            rise_seconds: 0.002,
            fall_seconds: 0.005,
            tonic_activation: 0.,
        },
    });
    let mut worst = 0_f64;
    for _ in 0..steps {
        let r = body
            .step_driven_muscle(&mut drives, 0.0004 / steps as f64, 1e-7, law())
            .unwrap();
        worst = worst.max(r.energy_defect_j.abs());
    }
    assert!(lumen(body.body()) < initial);
    // Original tube's three outer midlength support nodes remain exactly fixed.
    for node in [120, 128, 136] {
        assert_eq!(
            body.body().positions()[node],
            body.body().rest_positions()[node]
        );
        assert_eq!(body.velocities()[node], [0.; 3]);
    }
    assert!(
        body.muscle_support_reactions(law())
            .unwrap()
            .iter()
            .flatten()
            .any(|x| x.abs() > 1e-8)
    );
    (body, worst)
}
#[test]
fn layered_sphincter_dynamic_contraction_preserves_supports_and_refines() {
    let (a, ea) = ring_run(100);
    let (b, eb) = ring_run(200);
    let (c, ec) = ring_run(400);
    let ratio = difference(&a, &b) / difference(&b, &c);
    println!(
        "ring trajectory ratio={ratio}, lumen={}, defects={ea:e},{eb:e},{ec:e}",
        lumen(c.body())
    );
    assert!(ratio > 3. && ratio < 5.);
}
#[test]
fn stationary_support_reaction_matches_weight_and_rejects_initial_motion() {
    let x = vec![[0.; 3], [0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]];
    let body = Body::new(
        x,
        vec![true, false, false, false],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(1000., 0.3).unwrap(),
        )],
    )
    .unwrap();
    let mut v = vec![[0.; 3]; 4];
    v[0][0] = 1.;
    assert!(InertialBody::new_with_fixed_supports(body.clone(), &[1000.], v).is_err());
    assert!(InertialBody::new(body.clone(), &[1000.], vec![[0.; 3]; 4]).is_err());
    let mut b = InertialBody::new_with_fixed_supports(body, &[1000.], vec![[0.; 3]; 4]).unwrap();
    b.set_uniform_acceleration([0., -9.81, 0.]).unwrap();
    let r = b.muscle_support_reactions(law()).unwrap();
    assert!((r[0][1] - b.masses()[0] * 9.81).abs() < 1e-14);
    assert!(r[1..].iter().all(|r| *r == [0.; 3]));
    b.step(1e-5, 1.).unwrap();
    assert_eq!(b.body().positions()[0], [0.; 3]);
    assert_eq!(b.velocities()[0], [0.; 3]);
}

fn ring_release_cycle(substeps: usize) -> (InertialBody, f64) {
    let (mut b, _) = ring_run(substeps);
    let start = b.diagnostics().unwrap();
    let mut drives = [0, 1].map(|region| MuscleRegionDrive {
        region,
        activation: if region == 0 { 0.2 } else { 0.1 } * (1. - (-0.2_f64).exp()),
        excitation: 0.,
        kinetics: ActivationKinetics {
            rise_seconds: 0.002,
            fall_seconds: 0.005,
            tonic_activation: 0.,
        },
    });
    // Reuse exact accepted histories, avoiding floating-point recomputation mismatch.
    for drive in &mut drives {
        drive.activation = b
            .body()
            .elements()
            .iter()
            .find(|e| e.region == drive.region)
            .unwrap()
            .activation;
    }
    let mut work = 0.;
    let mut minimum = f64::INFINITY;
    let mut volume_at_switch = 0.;
    for phase in 1..44 {
        drives[0].excitation = if phase < 4 { 0.2 } else { 0. };
        drives[1].excitation = if phase < 4 { 0.1 } else { 0. };
        for _ in 0..substeps {
            let r = b
                .step_driven_muscle(&mut drives, 0.0004 / substeps as f64, 1e-7, law())
                .unwrap();
            work += r.activation_work_j + r.correction_work_j;
        }
        let v = lumen(b.body());
        minimum = minimum.min(v);
        if phase == 3 {
            volume_at_switch = v;
        }
    }
    let expected = 0.2 * (1. - (-0.8_f64).exp()) * (-3.2_f64).exp();
    assert!((drives[0].activation - expected).abs() < 1e-13);
    assert!((drives[1].activation - expected / 2.).abs() < 1e-13);
    assert!(minimum < volume_at_switch);
    assert!(lumen(b.body()) > volume_at_switch);
    let end = b.diagnostics().unwrap();
    let defect = end.kinetic_j - start.kinetic_j + end.potential_j - start.potential_j - work;
    for node in [120, 128, 136] {
        assert_eq!(b.body().positions()[node], b.body().rest_positions()[node]);
        assert_eq!(b.velocities()[node], [0.; 3]);
    }
    assert!(
        b.body()
            .stresses_at(b.body().positions())
            .unwrap()
            .iter()
            .all(|e| e.volume_ratio > 0.)
    );
    (b, defect.abs())
}
#[test]
fn full_ring_excitation_release_cycle_recovers_and_temporally_converges() {
    let (a, ea) = ring_release_cycle(100);
    let (b, eb) = ring_release_cycle(200);
    let (c, ec) = ring_release_cycle(400);
    let ratio = difference(&a, &b) / difference(&b, &c);
    println!("release trajectory ratio={ratio}, defects={ea:e},{eb:e},{ec:e}");
    assert!(ratio > 3.5 && ratio < 4.5);
    assert!(ea / eb > 3. && eb / ec > 3.);
}

#[test]
fn full_muscle_cauchy_stress_matches_hill_tension_and_nodal_power() {
    let b = specimen();
    let tissue = b.body();
    let passive = tissue.stresses_at(tissue.positions()).unwrap();
    for rate in [-0.5, 0., 0.5] {
        let v: Vec<_> = tissue
            .positions()
            .iter()
            .map(|p| [rate * p[0], 0., 0.])
            .collect();
        let stresses = tissue.muscle_stresses(&v, law()).unwrap();
        let sigma = stresses[0].stress.cauchy_pa;
        assert!((sigma[0][0] - 100. * law().factor(rate).unwrap()).abs() < 1e-12);
        for i in 0..3 {
            for j in 0..3 {
                if (i, j) != (0, 0) {
                    assert!(sigma[i][j].abs() < 1e-12);
                }
            }
        }
        let stress_power = (sigma[0][0] - passive[0].stress.cauchy_pa[0][0])
            * rate
            * stresses[0].reference_volume_m3;
        let nodal = tissue.active_velocity_forces(&v, law()).unwrap();
        assert!((nodal.correction_power_w + stress_power).abs() < 1e-14);
        let spinning: Vec<_> = v
            .iter()
            .zip(tissue.positions())
            .map(|(v, p)| [v[0] - 2. * p[1] + 0.3, v[1] + 2. * p[0] - 0.1, v[2] + 0.2])
            .collect();
        let rigid_added = tissue.muscle_stresses(&spinning, law()).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                assert!((rigid_added[0].stress.cauchy_pa[i][j] - sigma[i][j]).abs() < 1e-12);
            }
        }
    }
    assert!(tissue.muscle_stresses(&[], law()).is_err());
}

#[test]
fn deformed_layered_ring_stress_power_matches_assembled_correction() {
    let (b, _) = ring_run(100);
    let tissue = b.body();
    let baseline = tissue.stresses_at(tissue.positions()).unwrap();
    // v=rate*x gives current spatial velocity gradient rate*I, even after deformation.
    for rate in [-0.5, 0.5] {
        let velocities: Vec<_> = tissue
            .positions()
            .iter()
            .map(|p| p.map(|x| rate * x))
            .collect();
        let total = tissue.muscle_stresses(&velocities, law()).unwrap();
        let stress_power: f64 = total
            .iter()
            .zip(&baseline)
            .map(|(s, old)| {
                let trace: f64 = (0..3)
                    .map(|i| s.stress.cauchy_pa[i][i] - old.stress.cauchy_pa[i][i])
                    .sum();
                rate * trace * s.volume_ratio * s.reference_volume_m3
            })
            .sum();
        let forces = tissue.active_velocity_forces(&velocities, law()).unwrap();
        assert!((stress_power + forces.correction_power_w).abs() < 1e-12);
        assert!(stress_power > 0.);
    }
}
