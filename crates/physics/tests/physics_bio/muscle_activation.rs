use physics::biomechanics::*;
#[test]
fn activation_kinetics_has_exact_rise_fall_tone_and_time_composition() {
    let k = ActivationKinetics {
        rise_seconds: 0.2,
        fall_seconds: 0.5,
        tonic_activation: 0.1,
    };
    let a = k.advance(0.1, 1., 0.2).unwrap();
    assert!((a - (1. - 0.9 * (-1_f64).exp())).abs() < 1e-14);
    let b = k.advance(a, 0., 0.5).unwrap();
    assert!((b - (0.1 + (a - 0.1) * (-1_f64).exp())).abs() < 1e-14);
    let half = k.advance(0.1, 0.6, 0.05).unwrap();
    assert!(
        (k.advance(half, 0.6, 0.05).unwrap() - k.advance(0.1, 0.6, 0.1).unwrap()).abs() < 1e-14
    );
    assert_eq!(k.advance(0.1, 0., 1.).unwrap(), 0.1);
    assert!(k.advance(0., 1., 1e-15).unwrap() > 0.);
    assert!(k.advance(0., 1., 0.).is_err());
    assert!(k.advance(0., 2., 1.).is_err());
}
fn volume(body: &Body) -> f64 {
    body.cavities()[0]
        .faces
        .iter()
        .map(|f| {
            let [a, b, c] = f.map(|i| body.positions()[i]);
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.
        })
        .sum()
}
#[test]
fn layered_sphincter_kinetics_contract_actual_lumen_and_fail_atomically() {
    let mut body = sphincter_layers().unwrap();
    let initial = volume(&body);
    let mut drives = [0, 1].map(|region| MuscleRegionDrive {
        region,
        activation: 0.,
        excitation: if region == 0 { 0.2 } else { 0.1 },
        kinetics: ActivationKinetics {
            rise_seconds: 0.2,
            fall_seconds: 0.5,
            tonic_activation: 0.,
        },
    });
    let report = body
        .step_muscle_regions(&mut drives, 0.1, 50000, 1e-5)
        .unwrap();
    assert!(report.converged && report.min_j > 0.);
    let contracted = volume(&body);
    eprintln!(
        "sphincter initial_volume={initial} contracted_volume={contracted} IAS={} EAS={}",
        drives[0].activation, drives[1].activation
    );
    assert!(contracted < initial);
    for e in body.elements() {
        assert_eq!(e.activation, drives[e.region].activation);
    }
    let before = (
        body.positions().to_vec(),
        body.elements()
            .iter()
            .map(|e| e.activation)
            .collect::<Vec<_>>(),
        drives.map(|d| d.activation),
    );
    assert!(body.step_muscle_regions(&mut drives, 1., 1, 1e-20).is_err());
    assert_eq!(
        before,
        (
            body.positions().to_vec(),
            body.elements()
                .iter()
                .map(|e| e.activation)
                .collect::<Vec<_>>(),
            drives.map(|d| d.activation)
        )
    );
    drives[0].excitation = 0.;
    drives[1].excitation = 0.;
    body.step_muscle_regions(&mut drives, 0.5, 50000, 1e-5)
        .unwrap();
    assert!(volume(&body) > contracted);
    let before = body.positions().to_vec();
    let old = drives.map(|d| d.activation);
    drives[1].region = usize::MAX;
    assert!(
        body.step_muscle_regions(&mut drives, 0.1, 50000, 1e-5)
            .is_err()
    );
    assert_eq!(body.positions(), before);
    assert_eq!(old, drives.map(|d| d.activation));
}

#[test]
fn active_length_curve_stress_is_energy_derivative_and_objective() {
    let law = ActiveFiberLengthLaw {
        optimal_stretch: 1.,
        half_width: 0.5,
    };
    assert_eq!(law.response(1.).unwrap(), (1., 0.));
    assert_eq!(law.response(0.4).unwrap().0, 0.);
    assert_eq!(law.response(1.6).unwrap().0, 0.);
    let material = Material {
        shear_pa: 100.,
        bulk_pa: 1000.,
        fibers: vec![Fiber {
            direction: [1., 0., 0.],
            stiffness_pa: 30.,
            exponent: 2.,
            active_pa: 200.,
        }],
    };
    for stretch in [0.45, 0.8, 1., 1.1, 1.4, 1.65] {
        let f = [[stretch, 0.04, 0.], [0., 1.03, 0.02], [0., 0., 0.97]];
        let response = material
            .response_with_active_length(f, 0.8, Some(law))
            .unwrap();
        let passive = material.response(f, 0.).unwrap();
        assert!(
            (response.first_piola[0][0]
                - passive.first_piola[0][0]
                - 160. * law.response(stretch).unwrap().0)
                .abs()
                < 1e-10
        );
        for i in 0..3 {
            for j in 0..3 {
                let mut plus = f;
                let mut minus = f;
                plus[i][j] += 1e-6;
                minus[i][j] -= 1e-6;
                let derivative = (material
                    .response_with_active_length(plus, 0.8, Some(law))
                    .unwrap()
                    .energy_density
                    - material
                        .response_with_active_length(minus, 0.8, Some(law))
                        .unwrap()
                        .energy_density)
                    / 2e-6;
                assert!(
                    (derivative - response.first_piola[i][j]).abs()
                        < 1e-5 + 1e-6 * response.first_piola[i][j].abs(),
                    "stretch={stretch} i={i} j={j} numerical={derivative} analytic={}",
                    response.first_piola[i][j]
                );
            }
        }
        let rotated = [f[1].map(|x| -x), f[0], f[2]];
        let r = material
            .response_with_active_length(rotated, 0.8, Some(law))
            .unwrap();
        assert!((r.energy_density - response.energy_density).abs() < 1e-10);
        for k in 0..3 {
            assert!((r.first_piola[0][k] + response.first_piola[1][k]).abs() < 1e-10);
            assert!((r.first_piola[1][k] - response.first_piola[0][k]).abs() < 1e-10);
        }
    }
}
#[test]
fn length_dependent_sphincter_equilibrium_has_consistent_nodal_gradient() {
    let mut body = sphincter_layers().unwrap();
    for i in 0..body.elements().len() {
        body.set_active_fiber_length_law(
            i,
            ActiveFiberLengthLaw {
                optimal_stretch: 1.,
                half_width: 0.5,
            },
        )
        .unwrap();
    }
    let initial = volume(&body);
    let mut drives = [0, 1].map(|region| MuscleRegionDrive {
        region,
        activation: 0.,
        excitation: 0.2,
        kinetics: ActivationKinetics {
            rise_seconds: 0.2,
            fall_seconds: 0.5,
            tonic_activation: 0.,
        },
    });
    let report = body
        .step_muscle_regions(&mut drives, 0.1, 50000, 1e-5)
        .unwrap();
    assert!(report.converged && volume(&body) < initial);
    let positions = body.positions().to_vec();
    let (_, gradient) = body.evaluate(&positions).unwrap();
    for i in [0, 25, 100] {
        for axis in 0..3 {
            let mut plus = positions.clone();
            let mut minus = positions.clone();
            plus[i][axis] += 1e-8;
            minus[i][axis] -= 1e-8;
            let numerical =
                (body.evaluate(&plus).unwrap().0 - body.evaluate(&minus).unwrap().0) / 2e-8;
            assert!((numerical - gradient[i][axis]).abs() < 1e-7);
        }
    }
}

#[test]
fn velocity_law_matches_hill_and_objective_dissipative_fem_correction() {
    let law = ActiveFiberVelocityLaw {
        max_shortening_per_s: 2.,
        shortening_curvature: 0.25,
        eccentric_limit: 1.5,
        eccentric_rate_per_s: 1.,
    };
    for speed in [0., 0.1, 0.7, 1.9, 2.] {
        let force = law.factor(-speed).unwrap();
        // Hill: (P/P0 + c)(v + c*vmax) = c*vmax*(1+c).
        assert!(((force + 0.25) * (speed + 0.5) - 0.625).abs() < 1e-14);
    }
    assert_eq!(law.factor(-3.).unwrap(), 0.);
    assert_eq!(law.factor(0.).unwrap(), 1.);
    assert_eq!(law.factor(1.).unwrap(), 1.25);
    assert!(law.factor(f64::NAN).is_err());
    let mut body = sphincter_layers().unwrap();
    for i in 0..body.elements().len() {
        body.set_activation(i, 0.3).unwrap();
    }
    let x = body.positions();
    for rate in [-0.5, 0.5] {
        let v: Vec<_> = x.iter().map(|p| p.map(|a| rate * a)).collect();
        let result = body.active_velocity_forces(&v, law).unwrap();
        assert!(result.correction_power_w < 0.);
        let shifted: Vec<_> = v
            .iter()
            .map(|p| [p[0] + 0.7, p[1] - 0.3, p[2] + 0.2])
            .collect();
        let translated = body.active_velocity_forces(&shifted, law).unwrap();
        assert!((translated.correction_power_w - result.correction_power_w).abs() < 1e-11);
        let spinning: Vec<_> = v
            .iter()
            .zip(x)
            .map(|(v, p)| [v[0] - 2. * p[1], v[1] + 2. * p[0], v[2]])
            .collect();
        let rotated = body.active_velocity_forces(&spinning, law).unwrap();
        for (a, b) in rotated.forces_n.iter().zip(&result.forces_n) {
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() < 1e-12);
            }
        }
        for axis in 0..3 {
            assert!(result.forces_n.iter().map(|f| f[axis]).sum::<f64>().abs() < 1e-12);
        }
    }
    let rigid: Vec<_> = x.iter().map(|p| [-p[1] + 0.2, p[0] - 0.1, 0.3]).collect();
    let response = body.active_velocity_forces(&rigid, law).unwrap();
    assert!(response.forces_n.iter().flatten().all(|f| f.abs() < 1e-12));
    assert!(body.active_velocity_forces(&[], law).is_err());
}
