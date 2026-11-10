use physics::lymph::{AdaptiveExchangeConfig, Exchange, FluidSpace, LymphNetwork};
fn space(p: f64, protein: f64) -> FluidSpace {
    FluidSpace {
        reference_volume_m3: 1.,
        reference_pressure_pa: p,
        compliance_m3_per_pa: 1.,
        initial_volume_m3: 1.,
        initial_protein_kg: protein,
        oncotic_pa_per_kg_m3: 1.,
    }
}
fn edge(from: usize, to: usize) -> Exchange {
    Exchange {
        from,
        to,
        hydraulic_m3_per_pa_s: 1.,
        reflection: 0.,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: 0.,
        valve: false,
    }
}
#[test]
fn hydraulic_relaxation_converges_to_exact_solution() {
    fn run(h: f64) -> f64 {
        let mut n =
            LymphNetwork::new(vec![space(1., 0.), space(0., 0.)], vec![edge(0, 1)]).unwrap();
        n.step(1., h).unwrap();
        let exact = 1. - 0.5 * (1. - (-2_f64).exp());
        assert!((n.total_volume() - 2.).abs() < 1e-13);
        (n.volumes()[0] - exact).abs()
    }
    let coarse = run(0.01);
    let fine = run(0.001);
    assert!(fine < coarse * 0.11);
}
#[test]
fn oncotic_balance_and_reflection_retain_protein() {
    let mut e = edge(0, 1);
    e.reflection = 1.;
    let n = LymphNetwork::new(vec![space(2., 2.), space(0., 0.)], vec![e]).unwrap();
    assert_eq!(n.rates().unwrap()[0], (0., 0.));
    let mut n = LymphNetwork::new(vec![space(3., 2.), space(0., 0.)], vec![e]).unwrap();
    n.step(0.01, 0.001).unwrap();
    assert!(n.volumes()[1] > 1.);
    assert_eq!(n.protein_masses(), &[2., 0.]);
}
#[test]
fn protein_diffusion_is_conservative_and_converges() {
    let mut e = edge(0, 1);
    e.hydraulic_m3_per_pa_s = 0.;
    e.protein_permeability_m3_per_s = 1.;
    let mut n = LymphNetwork::new(vec![space(0., 1.), space(0., 0.)], vec![e]).unwrap();
    let r = n.step(1., 0.0001).unwrap();
    assert!((n.protein_masses()[0] - (0.5 + 0.5 * (-2_f64).exp())).abs() < 2e-5);
    assert!(r.protein_drift_kg.abs() < 1e-13);
    assert_eq!(n.volumes(), &[1., 1.]);
}
#[test]
fn valve_closes_both_transport_paths_and_pump_reopens_it() {
    let mut e = edge(0, 1);
    e.valve = true;
    e.protein_permeability_m3_per_s = 1.;
    let n = LymphNetwork::new(vec![space(0., 2.), space(2., 0.)], vec![e]).unwrap();
    assert_eq!(n.rates().unwrap()[0], (0., 0.));
    e.pump_head_pa = 3.;
    let n = LymphNetwork::new(vec![space(0., 2.), space(2., 0.)], vec![e]).unwrap();
    let (q, j) = n.rates().unwrap()[0];
    assert!(q > 0. && j > 0.);
}
#[test]
fn closed_plasma_tissue_lymph_loop_preserves_both_totals() {
    let mut cap = edge(0, 1);
    cap.reflection = 0.8;
    cap.protein_permeability_m3_per_s = 0.01;
    let mut lymph = edge(1, 2);
    lymph.valve = true;
    let mut ret = edge(2, 0);
    ret.valve = true;
    ret.pump_head_pa = 4.;
    let mut n = LymphNetwork::new(
        vec![space(3., 2.), space(0., 0.2), space(-1., 0.1)],
        vec![cap, lymph, ret],
    )
    .unwrap();
    let v = n.total_volume();
    let m = n.total_protein();
    for _ in 0..100 {
        n.step(0.1, 0.001).unwrap();
        assert!(n.volumes().iter().all(|x| *x > 0.));
        assert!(n.protein_masses().iter().all(|x| *x >= 0.));
    }
    assert!((n.total_volume() - v).abs() < 1e-12);
    assert!((n.total_protein() - m).abs() < 1e-12);
}
#[test]
fn positivity_limit_and_failure_are_transactional() {
    let mut e = edge(0, 1);
    e.pump_head_pa = 1000.;
    let mut n = LymphNetwork::new(vec![space(0., 1.), space(0., 0.)], vec![e]).unwrap();
    n.step(0.00099, 1.).unwrap();
    assert!(n.volumes()[0] > 0. && n.protein_masses()[0] > 0.);
    let v = n.volumes().to_vec();
    let m = n.protein_masses().to_vec();
    assert!(n.step(1., 1e-7).is_err());
    assert_eq!(n.volumes(), v);
    assert_eq!(n.protein_masses(), m);
    assert!(n.step(f64::NAN, 1.).is_err());
    let mut bad = space(0., 0.);
    bad.compliance_m3_per_pa = 0.;
    assert!(LymphNetwork::new(vec![bad, space(0., 0.)], vec![e]).is_err());
}

#[test]
fn nonlinear_space_specific_osmosis_drives_conservative_valved_loop() {
    use physics::biomechanics::OsmoticPressureLaw;
    let laws = [
        OsmoticPressureLaw {
            linear: 1.,
            quadratic: 0.2,
            cubic: 0.01,
        },
        OsmoticPressureLaw {
            linear: 0.5,
            quadratic: 0.1,
            cubic: 0.,
        },
        OsmoticPressureLaw {
            linear: 0.,
            quadratic: 0.,
            cubic: 0.,
        },
    ];
    let mut cap = edge(0, 1);
    cap.reflection = 0.8;
    cap.protein_permeability_m3_per_s = 0.01;
    let mut inlet = edge(1, 2);
    inlet.valve = true;
    let mut ret = edge(2, 0);
    ret.valve = true;
    ret.pump_head_pa = 8.;
    let mut net = LymphNetwork::new(
        vec![space(4., 2.), space(1., 1.), space(0., 0.5)],
        vec![cap, inlet, ret],
    )
    .unwrap();
    let rates = net.rates_with_osmotic_laws(&laws).unwrap();
    let pi0 = 2. + 0.2 * 4. + 0.01 * 8.;
    let pi1 = 0.5 + 0.1;
    let q = 3. - 0.8 * (pi0 - pi1);
    assert!((rates[0].0 - q).abs() < 1e-14);
    assert!((rates[0].1 - (0.2 * q * 2. + 0.01)).abs() < 1e-14);
    let old = (net.total_volume(), net.total_protein());
    let report = net.step_with_osmotic_laws(0.1, 1e-4, &laws).unwrap();
    assert!((net.total_volume() - old.0).abs() < 1e-13);
    assert!((net.total_protein() - old.1).abs() < 1e-13);
    assert!(report.transferred_volume_m3.iter().all(|v| *v > 0.));
    assert!(net.volumes().iter().all(|v| *v > 0.));
    let before = (
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
    );
    for bad in [
        &laws[..2],
        &[OsmoticPressureLaw {
            linear: 0.,
            quadratic: f64::MAX,
            cubic: f64::MAX,
        }; 3][..],
    ] {
        assert!(net.step_with_osmotic_laws(0.1, 1e-4, bad).is_err());
        assert_eq!(
            before,
            (
                net.volumes().to_vec(),
                net.protein_masses().to_vec(),
                net.pressures(),
                net.rates().unwrap()
            )
        );
    }
}

#[test]
fn active_lymphatic_wall_pumps_without_edge_pressure_head() {
    use physics::biomechanics::OsmoticPressureLaw;
    use physics::lymph::LymphaticWallLaw;
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-8,
        length_m: 0.01,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: 0.,
        active_tension_n_per_m: 0.,
    };
    assert_eq!(wall.pressure_pa(1e-8).unwrap(), 0.);
    assert!(wall.pressure_pa(0.5e-8).unwrap() < 0.);
    assert!(wall.pressure_pa(2e-8).unwrap() > 0.);
    assert!(wall.pressure_pa(1e300).is_err());
    let radius = (1e-8_f64 / (std::f64::consts::PI * 0.01)).sqrt();
    let active = LymphaticWallLaw {
        active_tension_n_per_m: 0.01,
        ..wall
    };
    assert!((active.pressure_pa(1e-8).unwrap() - 0.01 / radius).abs() < 1e-12);
    let spaces = [(1e-7, 5.), (1e-8, 0.), (1e-7, 8.)].map(|(v, p)| FluidSpace {
        reference_volume_m3: v,
        reference_pressure_pa: p,
        compliance_m3_per_pa: 1e-8,
        initial_volume_m3: v,
        initial_protein_kg: 10. * v,
        oncotic_pa_per_kg_m3: 0.,
    });
    let mut inlet = edge(0, 1);
    inlet.hydraulic_m3_per_pa_s = 1e-12;
    inlet.valve = true;
    let mut outlet = edge(1, 2);
    outlet.hydraulic_m3_per_pa_s = 1e-12;
    outlet.valve = true;
    let mut ret = edge(2, 0);
    ret.hydraulic_m3_per_pa_s = 1e-12;
    let mut net = LymphNetwork::new(spaces.to_vec(), vec![inlet, outlet, ret]).unwrap();
    let initial_network = net.clone();
    let laws = [OsmoticPressureLaw {
        linear: 0.,
        quadratic: 0.,
        cubic: 0.,
    }; 3];
    let totals = (net.total_volume(), net.total_protein());
    for _ in 0..5 {
        let filling = net
            .step_with_wall_and_osmotic_laws(0.1, 1e-4, &laws, &[None, Some(wall), None])
            .unwrap();
        assert!(filling.transferred_volume_m3[0] > 0.);
        assert_eq!(filling.transferred_volume_m3[1], 0.);
        let old_volume = net.volumes()[1];
        let ejection = net
            .step_with_wall_and_osmotic_laws(0.1, 1e-4, &laws, &[None, Some(active), None])
            .unwrap();
        assert_eq!(ejection.transferred_volume_m3[0], 0.);
        assert!(ejection.transferred_volume_m3[1] > 0.);
        assert!(net.volumes()[1] < old_volume);
        assert!((net.pressures()[1] - active.pressure_pa(net.volumes()[1]).unwrap()).abs() < 1e-12);
        assert!((net.total_volume() - totals.0).abs() < 1e-12 * totals.0);
        assert!((net.total_protein() - totals.1).abs() < 1e-12 * totals.1);
        for (m, v) in net.protein_masses().iter().zip(net.volumes()) {
            assert!((m / v - 10.).abs() < 1e-10);
        }
    }
    let before = (
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
    );
    let invalid = LymphaticWallLaw {
        active_tension_n_per_m: f64::MAX,
        ..wall
    };
    assert!(
        net.step_with_wall_and_osmotic_laws(0.1, 1e-4, &laws, &[None, Some(invalid), None])
            .is_err()
    );
    assert_eq!(
        before,
        (
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap()
        )
    );
    // Initial wall evaluation is finite; overflow occurs only at the predictor.
    let mut retried = initial_network.clone();
    let retry_wall = LymphaticWallLaw {
        stiffening: 5e5,
        ..wall
    };
    let retry = retried
        .step_with_wall_and_osmotic_laws(0.1, 0.1, &laws, &[None, Some(retry_wall), None])
        .unwrap();
    assert!(
        retry.substeps > 1,
        "second-stage outgoing limit must shorten the initial step"
    );
    assert!((retried.total_volume() - 2.1e-7).abs() < 1e-19);
    assert!(retried.volumes().iter().all(|v| *v > 0.));
    for (m, v) in retried.protein_masses().iter().zip(retried.volumes()) {
        assert!((m / v - 10.).abs() < 1e-10);
    }
    let mut late = initial_network;
    let late_before = (
        late.volumes().to_vec(),
        late.protein_masses().to_vec(),
        late.pressures(),
        late.rates().unwrap(),
    );
    let stiff = LymphaticWallLaw {
        stiffening: 1e8,
        ..wall
    };
    assert!(stiff.pressure_pa(1e-8).unwrap().is_finite());
    assert_eq!(
        late.step_with_wall_and_osmotic_laws(0.1, 0.1, &laws, &[None, Some(stiff), None])
            .unwrap_err(),
        "lymphatic wall pressure overflow"
    );
    assert_eq!(
        late_before,
        (
            late.volumes().to_vec(),
            late.protein_masses().to_vec(),
            late.pressures(),
            late.rates().unwrap()
        )
    );
}

#[test]
fn active_wall_cycle_refines_toward_independent_runge_kutta_reference() {
    use physics::biomechanics::OsmoticPressureLaw;
    use physics::lymph::LymphaticWallLaw;
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-8,
        length_m: 0.01,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: 0.,
        active_tension_n_per_m: 0.,
    };
    // Reference independently implements the three water balance equations and
    // integrates output volume as a fourth variable; no production pressure/flow calls.
    let rhs = |v: [f64; 4], tension: f64| -> [f64; 4] {
        let stretch = (v[1] / 1e-8).sqrt();
        let radius = (v[1] / (std::f64::consts::PI * 0.01)).sqrt();
        let p = [
            5. + (v[0] - 1e-7) / 1e-8,
            20. * ((2. * (stretch - 1.)).exp() - stretch.powi(-3)) + tension / radius,
            8. + (v[2] - 1e-7) / 1e-8,
        ];
        let inlet = 1e-9 * (p[0] - p[1]).max(0.);
        let outlet = 1e-9 * (p[1] - p[2]).max(0.);
        let ret = 1e-9 * (p[2] - p[0]);
        [-inlet + ret, inlet - outlet, outlet - ret, outlet]
    };
    let rk4 = |h: f64| {
        let mut v = [1e-7, 1e-8, 1e-7, 0.];
        let n = (0.1 / h).round() as usize;
        let h = 0.1 / n as f64;
        for _ in 0..5 {
            for tension in [0., 0.03] {
                for _ in 0..n {
                    let a = rhs(v, tension);
                    let b = rhs(std::array::from_fn(|i| v[i] + 0.5 * h * a[i]), tension);
                    let c = rhs(std::array::from_fn(|i| v[i] + 0.5 * h * b[i]), tension);
                    let d = rhs(std::array::from_fn(|i| v[i] + h * c[i]), tension);
                    v = std::array::from_fn(|i| {
                        v[i] + h * (a[i] + 2. * b[i] + 2. * c[i] + d[i]) / 6.
                    });
                }
            }
        }
        v
    };
    let reference = rk4(1e-5);
    let finer = rk4(5e-6);
    assert!(
        reference
            .iter()
            .zip(finer)
            .all(|(a, b)| (a - b).abs() < 1e-16)
    );
    let mut errors = Vec::new();
    let mut adaptive_errors = Vec::new();
    for (h, tolerance) in [
        (0.01, None),
        (0.005, None),
        (0.0025, None),
        (0.1, Some(1e-3)),
        (0.1, Some(1e-4)),
        (0.1, Some(1e-5)),
    ] {
        let spaces = [(1e-7, 5.), (1e-8, 0.), (1e-7, 8.)].map(|(v, p)| FluidSpace {
            reference_volume_m3: v,
            reference_pressure_pa: p,
            compliance_m3_per_pa: 1e-8,
            initial_volume_m3: v,
            initial_protein_kg: 10. * v,
            oncotic_pa_per_kg_m3: 0.,
        });
        let edges = [(0, 1, true), (1, 2, true), (2, 0, false)].map(|(from, to, valve)| Exchange {
            from,
            to,
            hydraulic_m3_per_pa_s: 1e-9,
            reflection: 0.,
            protein_permeability_m3_per_s: 0.,
            pump_head_pa: 0.,
            valve,
        });
        let mut net = LymphNetwork::new(spaces.to_vec(), edges.to_vec()).unwrap();
        let laws = [OsmoticPressureLaw {
            linear: 0.,
            quadratic: 0.,
            cubic: 0.,
        }; 3];
        let mut output = 0.;
        let mut accepted = 0;
        let mut rejected = 0;
        for _ in 0..5 {
            for tension in [0., 0.03] {
                let active = LymphaticWallLaw {
                    active_tension_n_per_m: tension,
                    ..wall
                };
                if let Some(relative) = tolerance {
                    let report = net
                        .step_with_wall_and_osmotic_laws_adaptive(
                            0.1,
                            &laws,
                            &[None, Some(active), None],
                            AdaptiveExchangeConfig {
                                relative_tolerance: relative,
                                absolute_volume_tolerance_m3: 1e-18,
                                absolute_protein_tolerance_kg: 1e-17,
                                min_step_seconds: 1e-8,
                                max_step_seconds: h,
                                max_trials: 10_000,
                            },
                        )
                        .unwrap();
                    assert!(report.max_accepted_error_ratio <= 1.);
                    assert_eq!(report.exchange.substeps, 2 * report.accepted_steps);
                    accepted += report.accepted_steps;
                    rejected += report.rejected_steps;
                    output += report.exchange.transferred_volume_m3[1];
                } else {
                    output += net
                        .step_with_wall_and_osmotic_laws(0.1, h, &laws, &[None, Some(active), None])
                        .unwrap()
                        .transferred_volume_m3[1];
                }
            }
        }
        let state = [net.volumes()[0], net.volumes()[1], net.volumes()[2], output];
        let error = state
            .iter()
            .zip(reference)
            .map(|(a, b)| (a - b).abs())
            .fold(0., f64::max);
        if tolerance.is_some() {
            adaptive_errors.push(error);
            assert!(rejected > 0);
        } else {
            errors.push(error);
        }
        assert!((net.total_volume() - 2.1e-7).abs() < 1e-19);
        for (m, v) in net.protein_masses().iter().zip(net.volumes()) {
            assert!((m / v - 10.).abs() < 1e-10);
        }
        eprintln!(
            "active wall dt={h} tolerance={tolerance:?} accepted={accepted} rejected={rejected} max_state_error_m3={error:.12e} output_m3={output:.12e}"
        );
    }
    assert!(
        errors[1] < 0.3 * errors[0] && errors[2] < 0.3 * errors[1],
        "{errors:?}"
    );
    assert!(
        adaptive_errors[1] < adaptive_errors[0] && adaptive_errors[2] < adaptive_errors[1],
        "adaptive refinement {adaptive_errors:?}"
    );
    eprintln!("RK4 reference output_m3={:.12e}", reference[3]);
}

#[test]
fn adaptive_wall_limits_roll_back_even_after_an_accepted_trial() {
    use physics::biomechanics::OsmoticPressureLaw;
    let mut net = LymphNetwork::new(vec![space(5., 1.), space(0., 1.)], vec![edge(0, 1)]).unwrap();
    let walls = [
        None,
        Some(physics::lymph::LymphaticWallLaw {
            reference_volume_m3: 1.,
            length_m: 1.,
            passive_pressure_scale_pa: 20.,
            stiffening: 2.,
            external_pressure_pa: 0.,
            active_tension_n_per_m: 0.,
        }),
    ];
    let laws = [OsmoticPressureLaw {
        linear: 0.,
        quadratic: 0.,
        cubic: 0.,
    }; 2];
    let cfg = AdaptiveExchangeConfig {
        relative_tolerance: 1e-3,
        absolute_volume_tolerance_m3: 1e-9,
        absolute_protein_tolerance_kg: 1e-9,
        min_step_seconds: 1e-8,
        max_step_seconds: 0.001,
        max_trials: 1,
    };
    let before = (
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
    );
    // The first short trial is within tolerance, but the whole call exhausts its budget.
    let mut proof = net.clone();
    assert_eq!(
        proof
            .step_with_wall_and_osmotic_laws_adaptive(0.001, &laws, &walls, cfg)
            .unwrap()
            .accepted_steps,
        1
    );
    assert_eq!(
        net.step_with_wall_and_osmotic_laws_adaptive(0.002, &laws, &walls, cfg)
            .unwrap_err(),
        "adaptive exchange trial limit"
    );
    assert_eq!(
        before,
        (
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap()
        )
    );
    let strict = AdaptiveExchangeConfig {
        relative_tolerance: 0.,
        absolute_volume_tolerance_m3: 1e-30,
        absolute_protein_tolerance_kg: 1e-30,
        min_step_seconds: 0.001,
        ..cfg
    };
    assert_eq!(
        net.step_with_wall_and_osmotic_laws_adaptive(0.001, &laws, &walls, strict)
            .unwrap_err(),
        "adaptive exchange minimum step"
    );
    for invalid in [
        AdaptiveExchangeConfig {
            relative_tolerance: f64::NAN,
            ..cfg
        },
        AdaptiveExchangeConfig {
            absolute_protein_tolerance_kg: 0.,
            ..cfg
        },
    ] {
        assert!(
            net.step_with_wall_and_osmotic_laws_adaptive(0.001, &laws, &walls, invalid)
                .is_err()
        );
    }
    assert_eq!(
        before,
        (
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap()
        )
    );
}

#[test]
fn wall_tube_resistance_scales_with_radius_fourth_power_and_preserves_diffusion() {
    use physics::lymph::{
        LymphaticHydraulicAttachment, LymphaticWallLaw, apply_wall_hydraulic_resistances,
    };
    let pi = std::f64::consts::PI;
    let wall = LymphaticWallLaw {
        reference_volume_m3: pi,
        length_m: 1.,
        passive_pressure_scale_pa: 1.,
        stiffening: 1.,
        external_pressure_pa: 0.,
        active_tension_n_per_m: 0.,
    };
    let mut e = edge(0, 1);
    e.reflection = 0.25;
    e.protein_permeability_m3_per_s = 0.5;
    let link = LymphaticHydraulicAttachment {
        edge: 0,
        compartment: 1,
        segment_length_m: 1.,
        viscosity_pa_s: pi / 8.,
    };
    for radius in [0.5_f64, 1., 2.] {
        let volumes = [2. * pi, pi * radius * radius];
        let proteins = [4. * pi, 3. * volumes[1]];
        for q in [-4., 0., 4.] {
            let donor = if q >= 0. { 2. } else { 3. };
            let mut rates = [(q, 0.75 * q * donor - 0.5)];
            apply_wall_hydraulic_resistances(
                &volumes,
                &proteins,
                &[e],
                &[None, Some(wall)],
                &[link],
                &mut rates,
            )
            .unwrap();
            let expected = q / (1. + radius.powi(-4));
            assert!((rates[0].0 - expected).abs() < 1e-14);
            assert!((rates[0].1 - (0.75 * expected * donor - 0.5)).abs() < 1e-14);
        }
    }
    let before = [(4., 5.5)];
    let mut rates = before;
    let bad = LymphaticHydraulicAttachment {
        edge: usize::MAX,
        ..link
    };
    assert!(
        apply_wall_hydraulic_resistances(
            &[2. * pi, pi],
            &[4. * pi, 3. * pi],
            &[e],
            &[None, Some(wall)],
            &[link, bad],
            &mut rates
        )
        .is_err()
    );
    assert_eq!(rates, before);
    assert!(
        apply_wall_hydraulic_resistances(
            &[2. * pi, pi],
            &[4. * pi, 3. * pi],
            &[e],
            &[None, Some(wall)],
            &[link, link],
            &mut rates
        )
        .is_err()
    );
    assert_eq!(rates, before);
}

#[test]
fn tube_diagnostics_distinguish_low_reynolds_from_inertial_transients() {
    use physics::lymph::{LymphaticHydraulicAttachment, LymphaticWallLaw};
    let pi = std::f64::consts::PI;
    let wall = LymphaticWallLaw {
        reference_volume_m3: pi,
        length_m: 1.,
        passive_pressure_scale_pa: 1.,
        stiffening: 1.,
        external_pressure_pa: 0.,
        active_tension_n_per_m: 0.,
    };
    let link = LymphaticHydraulicAttachment {
        edge: 0,
        compartment: 0,
        segment_length_m: 2.,
        viscosity_pa_s: 1.,
    };
    let d = link.diagnostics(wall, pi, 3., -pi, 0., 2. * pi).unwrap();
    assert!((d.radius_m - 1.).abs() < 1e-14);
    assert!((d.mean_velocity_m_per_s + 1.).abs() < 1e-14);
    assert!((d.reynolds - 6.).abs() < 1e-14);
    assert!((d.womersley - 3_f64.sqrt()).abs() < 1e-14);
    assert!((d.tube_resistance_pa_s_per_m3 - 16. / pi).abs() < 1e-14);
    assert!((d.inertance_pa_s2_per_m3 - 6. / pi).abs() < 1e-14);
    assert!((d.relaxation_seconds - 3. / 8.).abs() < 1e-14);
    assert!((d.inertial_to_resistive_ratio - 3. / 8.).abs() < 1e-14);
    let faster = link.diagnostics(wall, pi, 3., 0., 0., 0.01).unwrap();
    assert_eq!(faster.reynolds, 0.);
    assert!(faster.inertial_to_resistive_ratio > 100.);
    let fixed = link
        .diagnostics(wall, pi, 3., -pi, 16. / pi, 2. * pi)
        .unwrap();
    assert!((fixed.relaxation_seconds - 0.5 * d.relaxation_seconds).abs() < 1e-14);
    assert!(link.diagnostics(wall, pi, -1., 0., 0., 1.).is_err());
    assert!(link.diagnostics(wall, pi, 3., 0., 0., 0.).is_err());
}

#[test]
fn inertial_lymph_segment_matches_transient_and_valve_momentum() {
    use physics::lymph::{LymphaticHydraulicAttachment, LymphaticWallLaw};
    let pi = std::f64::consts::PI;
    let wall = LymphaticWallLaw {
        reference_volume_m3: pi,
        length_m: 1.,
        passive_pressure_scale_pa: 1.,
        stiffening: 1.,
        external_pressure_pa: 0.,
        active_tension_n_per_m: 0.,
    };
    let link = LymphaticHydraulicAttachment {
        edge: 0,
        compartment: 0,
        segment_length_m: 2.,
        viscosity_pa_s: 1.,
    };
    // Independently: R=16/pi, L=6/pi, drive=R, steady Q=1.
    let r = 16. / pi;
    let l = 6. / pi;
    let exact = 1. - (-1_f64 / (l / r)).exp();
    let mut errors = Vec::new();
    for n in [20, 40, 80] {
        let h = 1. / f64::from(n);
        let mut q = 0.;
        for _ in 0..n {
            let old = q;
            let (next, tangent) = link
                .momentum_response(wall, pi, 3., 0., r, old, h, false)
                .unwrap();
            assert!((l * (next - old) / h + r * next - r).abs() < 1e-12);
            assert!((tangent - 1. / (r + l / h)).abs() < 1e-14);
            q = next;
        }
        errors.push((q - exact).abs());
    }
    assert!(errors[1] < 0.55 * errors[0] && errors[2] < 0.55 * errors[1]);
    // Reversed pressure does not instantly erase stored forward momentum.
    let (q, _) = link
        .momentum_response(wall, pi, 3., 0., -r, 1., 0.01, true)
        .unwrap();
    assert!(q > 0.);
    let (closed, tangent) = link
        .momentum_response(wall, pi, 3., 0., -r, q, 1., true)
        .unwrap();
    assert_eq!((closed, tangent), (0., 0.));
    assert!(
        link.momentum_response(wall, pi, 3., 0., r, -1., 0.01, true)
            .is_err()
    );
    assert!(
        link.momentum_response(wall, pi, 3., 0., r, 0., 0., false)
            .is_err()
    );
}

#[test]
fn inertial_wall_network_joint_inventory_momentum_and_rollback() {
    use physics::biomechanics::OsmoticPressureLaw;
    use physics::lymph::{LymphaticHydraulicAttachment, LymphaticWallLaw};
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1.,
        length_m: 1.,
        passive_pressure_scale_pa: 1.,
        stiffening: 2.,
        external_pressure_pa: 0.,
        active_tension_n_per_m: 0.,
    };
    let link = LymphaticHydraulicAttachment {
        edge: 0,
        compartment: 1,
        segment_length_m: 1.,
        viscosity_pa_s: 1.,
    };
    let laws = [OsmoticPressureLaw {
        linear: 0.,
        quadratic: 0.,
        cubic: 0.,
    }; 2];
    let walls = [None, Some(wall)];
    let mut n = LymphNetwork::new(vec![space(1., 2.), space(0., 1.)], vec![edge(0, 1)]).unwrap();
    let mut history = [0.];
    let dt = 0.001;
    let report = n
        .step_with_inertial_walls(
            dt,
            &laws,
            &walls,
            &[link],
            1.,
            &mut history,
            100,
            1e-13,
            1e-13,
        )
        .unwrap();
    assert!((n.total_volume() - 2.).abs() < 1e-14);
    assert!((n.total_protein() - 3.).abs() < 1e-14);
    let q = history[0];
    let r = wall.radius_m(n.volumes()[1]).unwrap();
    let resistance = 1. + 8. / (std::f64::consts::PI * r.powi(4));
    let inertance = 1. / (std::f64::consts::PI * r * r);
    let p = n.pressures();
    assert!((inertance * q / dt + resistance * q - (p[0] - p[1])).abs() < 1e-12);
    assert!((n.volumes()[1] - 1. - dt * q).abs() < 1e-13);
    assert!((n.protein_masses()[1] - 1. - report.transferred_protein_kg[0]).abs() < 1e-13);
    let expected_j = q * n.protein_masses()[0] / n.volumes()[0];
    assert!((n.rates().unwrap()[0].1 - expected_j).abs() < 1e-14);
    let before_v = n.volumes().to_vec();
    let before_m = n.protein_masses().to_vec();
    let before_q = history;
    assert!(
        n.step_with_inertial_walls(
            dt,
            &laws,
            &walls,
            &[link],
            1.,
            &mut history,
            1,
            1e-30,
            1e-30
        )
        .is_err()
    );
    assert_eq!(n.volumes(), before_v);
    assert_eq!(n.protein_masses(), before_m);
    assert_eq!(history, before_q);
}

#[test]
fn inertial_nonlinear_network_converges_to_independent_rk4() {
    use physics::biomechanics::OsmoticPressureLaw;
    use physics::lymph::{LymphaticHydraulicAttachment, LymphaticWallLaw};
    // Independent three-state ODE: V0, M0, Q; V1=2-V0, M1=3-M0.
    // Density=viscosity=length=1; area=V1, Rpipe=8*pi/V1^2, L=1/V1.
    fn derivative(x: [f64; 3]) -> [f64; 3] {
        let [v, m, q] = x;
        let v1 = 2. - v;
        let stretch = v1.sqrt();
        let p0 = v;
        let p1 = (2. * (stretch - 1.)).exp() - stretch.powi(-3);
        let c0 = m / v;
        let c1 = (3. - m) / v1;
        let osmotic = 0.2 * (c0 - c1) + 0.03 * (c0 * c0 - c1 * c1);
        let drive = p0 - p1 - 0.3 * osmotic;
        let resistance = 1. + 8. * std::f64::consts::PI / (v1 * v1);
        let j = 0.7 * q * if q >= 0. { c0 } else { c1 } + 0.02 * (c0 - c1);
        [-q, -j, (drive - resistance * q) * v1]
    }
    fn reference(n: usize) -> [f64; 3] {
        let h = 0.1 / n as f64;
        let mut x = [1., 2., 0.];
        for _ in 0..n {
            let a = derivative(x);
            let b = derivative(std::array::from_fn(|i| x[i] + 0.5 * h * a[i]));
            let c = derivative(std::array::from_fn(|i| x[i] + 0.5 * h * b[i]));
            let d = derivative(std::array::from_fn(|i| x[i] + h * c[i]));
            x = std::array::from_fn(|i| x[i] + h * (a[i] + 2. * b[i] + 2. * c[i] + d[i]) / 6.);
        }
        x
    }
    let expected = reference(10000);
    let refined = reference(20000);
    for i in 0..3 {
        assert!((expected[i] - refined[i]).abs() < 2e-13);
    }
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1.,
        length_m: 1.,
        passive_pressure_scale_pa: 1.,
        stiffening: 2.,
        external_pressure_pa: 0.,
        active_tension_n_per_m: 0.,
    };
    let link = LymphaticHydraulicAttachment {
        edge: 0,
        compartment: 1,
        segment_length_m: 1.,
        viscosity_pa_s: 1.,
    };
    let laws = [OsmoticPressureLaw {
        linear: 0.2,
        quadratic: 0.03,
        cubic: 0.,
    }; 2];
    let mut e = edge(0, 1);
    e.reflection = 0.3;
    e.protein_permeability_m3_per_s = 0.02;
    let mut errors = Vec::new();
    for n in [20, 40, 80] {
        let mut net = LymphNetwork::new(vec![space(1., 2.), space(0., 1.)], vec![e]).unwrap();
        let mut q = [0.];
        let mut ledger = [0.; 2];
        for _ in 0..n {
            let report = net
                .step_with_inertial_walls(
                    0.1 / f64::from(n),
                    &laws,
                    &[None, Some(wall)],
                    &[link],
                    1.,
                    &mut q,
                    100,
                    1e-14,
                    1e-14,
                )
                .unwrap();
            ledger[0] += report.transferred_volume_m3[0];
            ledger[1] += report.transferred_protein_kg[0];
        }
        let actual = [net.volumes()[0], net.protein_masses()[0], q[0]];
        let error = std::array::from_fn::<_, 3, _>(|i| (actual[i] - expected[i]).abs());
        eprintln!("inertial nonlinear n={n} error={error:?} state={actual:?}");
        errors.push(error);
        assert!((net.total_volume() - 2.).abs() < 1e-13);
        assert!((net.total_protein() - 3.).abs() < 1e-13);
        assert!((net.volumes()[0] + ledger[0] - 1.).abs() < 2e-12);
        assert!((net.protein_masses()[0] + ledger[1] - 2.).abs() < 2e-12);
    }
    for i in 0..3 {
        assert!(errors[1][i] < 0.6 * errors[0][i]);
        assert!(errors[2][i] < 0.6 * errors[1][i]);
    }
    let mut adaptive_errors = Vec::new();
    for relative in [1e-3, 1e-4] {
        let mut net = LymphNetwork::new(vec![space(1., 2.), space(0., 1.)], vec![e]).unwrap();
        let mut q = [0.];
        let cfg = AdaptiveExchangeConfig {
            relative_tolerance: relative,
            absolute_volume_tolerance_m3: 1e-10,
            absolute_protein_tolerance_kg: 1e-10,
            min_step_seconds: 1e-10,
            max_step_seconds: 0.005,
            max_trials: 100000,
        };
        let report = net
            .step_with_inertial_walls_adaptive(
                0.1,
                &laws,
                &[None, Some(wall)],
                &[link],
                1.,
                &mut q,
                cfg,
                1e-8,
                100,
                1e-14,
                1e-14,
            )
            .unwrap();
        let actual = [net.volumes()[0], net.protein_masses()[0], q[0]];
        let error = std::array::from_fn::<_, 3, _>(|i| (actual[i] - expected[i]).abs());
        eprintln!(
            "adaptive inertial rel={relative} accepted={} rejected={} error={error:?}",
            report.accepted_steps, report.rejected_steps
        );
        assert!(report.max_accepted_error_ratio <= 1.);
        assert_eq!(report.exchange.substeps, 2 * report.accepted_steps);
        assert!((net.total_volume() - 2.).abs() < 1e-13);
        assert!((net.total_protein() - 3.).abs() < 1e-13);
        adaptive_errors.push(error);
        let before = (net.volumes().to_vec(), net.protein_masses().to_vec(), q);
        let failed = AdaptiveExchangeConfig {
            max_trials: 1,
            ..cfg
        };
        assert!(
            net.step_with_inertial_walls_adaptive(
                0.1,
                &laws,
                &[None, Some(wall)],
                &[link],
                1.,
                &mut q,
                failed,
                1e-8,
                100,
                1e-14,
                1e-14
            )
            .is_err()
        );
        assert_eq!(
            before,
            (net.volumes().to_vec(), net.protein_masses().to_vec(), q)
        );
    }
    for i in 0..3 {
        assert!(adaptive_errors[1][i] < 0.5 * adaptive_errors[0][i]);
    }
}

#[test]
fn adaptive_inertial_retries_nonlinear_failure_and_preserves_atomicity() {
    use physics::biomechanics::OsmoticPressureLaw;
    use physics::lymph::{LymphaticHydraulicAttachment, LymphaticWallLaw};
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1.,
        length_m: 1.,
        passive_pressure_scale_pa: 1.,
        stiffening: 2.,
        external_pressure_pa: 0.,
        active_tension_n_per_m: 0.,
    };
    let walls = [None, Some(wall)];
    let links = [LymphaticHydraulicAttachment {
        edge: 0,
        compartment: 1,
        segment_length_m: 1.,
        viscosity_pa_s: 1.,
    }];
    let laws = [OsmoticPressureLaw {
        linear: 0.,
        quadratic: 0.,
        cubic: 0.,
    }; 2];
    let initial = LymphNetwork::new(vec![space(1., 2.), space(0., 1.)], vec![edge(0, 1)]).unwrap();
    let mut net = initial.clone();
    let mut history = [0.];
    assert_eq!(
        net.step_with_inertial_walls(
            0.01,
            &laws,
            &walls,
            &links,
            1.,
            &mut history,
            2,
            1e-10,
            1e-10
        )
        .unwrap_err(),
        "inertial lymph iteration limit"
    );
    assert_eq!(net.volumes(), initial.volumes());
    assert_eq!(history, [0.]);
    let cfg = AdaptiveExchangeConfig {
        relative_tolerance: 1e-3,
        absolute_volume_tolerance_m3: 1e-8,
        absolute_protein_tolerance_kg: 1e-8,
        min_step_seconds: 1e-9,
        max_step_seconds: 0.01,
        max_trials: 10000,
    };
    let report = net
        .step_with_inertial_walls_adaptive(
            0.01,
            &laws,
            &walls,
            &links,
            1.,
            &mut history,
            cfg,
            1e-6,
            2,
            1e-10,
            1e-10,
        )
        .unwrap();
    eprintln!(
        "nonlinear retry accepted={} rejected={}",
        report.accepted_steps, report.rejected_steps
    );
    assert!(report.rejected_steps > 0);
    assert!(net.volumes()[1] > 1. && history[0] > 0.);
    assert!((net.total_volume() - 2.).abs() < 1e-13);
    assert!((net.total_protein() - 3.).abs() < 1e-13);
    let before = (
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
        history,
    );
    let fail = AdaptiveExchangeConfig {
        min_step_seconds: 0.01,
        ..cfg
    };
    assert!(
        net.step_with_inertial_walls_adaptive(
            0.01,
            &laws,
            &walls,
            &links,
            1.,
            &mut history,
            fail,
            1e-6,
            1,
            1e-30,
            1e-30
        )
        .is_err()
    );
    assert_eq!(
        before,
        (
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap(),
            history
        )
    );
    // Invalid constitutive parameters are not silently treated as recoverable.
    assert_eq!(
        net.step_with_inertial_walls_adaptive(
            0.01,
            &laws,
            &walls,
            &links,
            -1.,
            &mut history,
            cfg,
            1e-6,
            2,
            1e-10,
            1e-10
        )
        .unwrap_err(),
        "invalid inertial lymph step"
    );
}

#[test]
fn radial_unsteady_pipe_converges_to_poiseuille_and_obeys_energy_balance() {
    use physics::lymph::profile::RadialPipe;
    let exact = std::f64::consts::PI / 8.;
    let mut errors = Vec::new();
    for n in [8, 16, 32] {
        let mut pipe = RadialPipe::new(1., 1., 1., n).unwrap();
        let mut old_energy = 0.;
        let mut flow = 0.;
        for _ in 0..20 {
            let report = pipe.step(1., 1.).unwrap();
            // Discrete BE work >= kinetic increment + nonnegative viscous loss.
            let work = report.flow_m3_per_s;
            assert!(
                report.kinetic_energy_j_per_m - old_energy + report.viscous_power_w_per_m
                    <= work + 1e-13
            );
            old_energy = report.kinetic_energy_j_per_m;
            flow = report.flow_m3_per_s;
        }
        let error = (flow - exact).abs();
        errors.push(error);
        eprintln!("radial Poiseuille n={n} flow={flow} error={error}");
        let before = pipe.velocities_m_per_s().to_vec();
        assert!(pipe.step(0., 1.).is_err());
        assert_eq!(before, pipe.velocities_m_per_s());
    }
    assert!(errors[1] < 0.3 * errors[0] && errors[2] < 0.3 * errors[1]);
    let mut forward = RadialPipe::new(1., 1., 1., 64).unwrap();
    let mut reverse = forward.clone();
    let a = forward.step(1e-5, 1.).unwrap();
    let b = reverse.step(1e-5, -1.).unwrap();
    assert!((a.flow_m3_per_s + b.flow_m3_per_s).abs() < 1e-16);
    assert!((a.flow_tangent - a.flow_m3_per_s).abs() < 1e-16);
    assert!(a.flow_m3_per_s < exact * 0.001);
    // After drive removal, stored radial momentum persists while viscosity dissipates it.
    let decay = forward.step(1e-5, 0.).unwrap();
    assert!(decay.flow_m3_per_s > 0. && decay.kinetic_energy_j_per_m < a.kinetic_energy_j_per_m);
}

#[test]
fn radial_pulsatile_flow_refines_to_independent_womersley_amplitude_and_phase() {
    use physics::lymph::profile::RadialPipe;
    #[derive(Clone, Copy)]
    struct C(f64, f64);
    impl C {
        fn add(self, b: Self) -> Self {
            Self(self.0 + b.0, self.1 + b.1)
        }
        fn mul(self, b: Self) -> Self {
            Self(self.0 * b.0 - self.1 * b.1, self.0 * b.1 + self.1 * b.0)
        }
        fn scale(self, a: f64) -> Self {
            Self(a * self.0, a * self.1)
        }
        fn div(self, b: Self) -> Self {
            self.mul(Self(b.0, -b.1))
                .scale(1. / (b.0 * b.0 + b.1 * b.1))
        }
        fn norm(self) -> f64 {
            self.0.hypot(self.1)
        }
    }
    // Independent power-series J0/J1; no radial discretization or production flow laws.
    fn bessel(z: C, order: usize) -> C {
        let mut term = if order == 0 { C(1., 0.) } else { z.scale(0.5) };
        let factor = z.mul(z).scale(-0.25);
        let mut sum = term;
        for k in 1..100 {
            term = term.mul(factor).scale(1. / (k * (k + order)) as f64);
            sum = sum.add(term);
            if term.norm() < 1e-16 * sum.norm() {
                return sum;
            }
        }
        panic!("independent Bessel series did not converge");
    }
    for omega in [4_f64, 100.] {
        // R=rho=mu=gradient amplitude=1, z^2=-i*omega.
        let a = (0.5 * omega).sqrt();
        let z = C(a, -a);
        let ratio = bessel(z, 1).scale(2.).div(z.mul(bessel(z, 0)));
        let expected = C(1. - ratio.0, -ratio.1).mul(C(0., -std::f64::consts::PI / omega));
        let mut errors = Vec::new();
        for (rings, samples) in [(32, 128), (64, 256), (128, 512)] {
            let mut pipe = RadialPipe::new(1., 1., 1., rings).unwrap();
            let dt = 2. * std::f64::consts::PI / omega / samples as f64;
            let mut measured = C(0., 0.);
            for k in 1..=40 * samples {
                let angle = omega * k as f64 * dt;
                let q = pipe.step(dt, angle.cos()).unwrap().flow_m3_per_s;
                if k > 39 * samples {
                    measured = measured
                        .add(C(q * angle.cos(), -q * angle.sin()).scale(2. / samples as f64));
                }
            }
            let error =
                C(measured.0 - expected.0, measured.1 - expected.1).norm() / expected.norm();
            let phase = (measured.1.atan2(measured.0) - expected.1.atan2(expected.0)).abs();
            eprintln!(
                "Womersley alpha={} rings={rings} samples={samples} relative_complex_error={error} phase_error_rad={phase}",
                omega.sqrt()
            );
            errors.push(error);
        }
        assert!(errors[1] < 0.65 * errors[0] && errors[2] < 0.65 * errors[1]);
        assert!(errors[2] < 0.015);
        assert!(expected.1 < 0.); // Flow lags cosine pressure forcing.
    }
}

#[test]
fn radial_profile_series_interface_preserves_momentum_and_pressure_balance() {
    use physics::lymph::profile::RadialPipe;
    let mut pipe = RadialPipe::new(1., 1., 1., 64).unwrap();
    let length = 2.;
    let resistance = 10.;
    let dt = 0.01;
    let mut old_energy = 0.;
    for drive in [1., 1., -0.1, 0.] {
        let old = pipe.clone();
        let mut free = old.clone();
        let free_report = free.step(dt, 0.).unwrap();
        let (report, tangent) = pipe
            .step_with_series_resistance(dt, drive, length, resistance)
            .unwrap();
        let work = dt * drive * report.flow_m3_per_s;
        let energy_and_loss = length * (report.kinetic_energy_j_per_m - old_energy)
            + dt * (length * report.viscous_power_w_per_m
                + resistance * report.flow_m3_per_s.powi(2));
        assert!(energy_and_loss <= work + 1e-13);
        old_energy = report.kinetic_energy_j_per_m;
        let gradient = (drive - resistance * report.flow_m3_per_s) / length;
        let mut independent = old.clone();
        let check = independent.step(dt, gradient).unwrap();
        assert!((check.flow_m3_per_s - report.flow_m3_per_s).abs() < 1e-14);
        for (a, b) in independent
            .velocities_m_per_s()
            .iter()
            .zip(pipe.velocities_m_per_s())
        {
            assert!((a - b).abs() < 1e-14);
        }
        assert!(
            (report.flow_m3_per_s
                - free_report.flow_m3_per_s
                - free_report.flow_tangent * gradient)
                .abs()
                < 1e-14
        );
        let eps = 1e-5;
        let mut plus = old.clone();
        let mut minus = old;
        let q_plus = plus
            .step_with_series_resistance(dt, drive + eps, length, resistance)
            .unwrap()
            .0
            .flow_m3_per_s;
        let q_minus = minus
            .step_with_series_resistance(dt, drive - eps, length, resistance)
            .unwrap()
            .0
            .flow_m3_per_s;
        assert!(((q_plus - q_minus) / (2. * eps) - tangent).abs() < 1e-12);
        if drive == 0. {
            assert!(report.flow_m3_per_s > 0.);
        }
    }
    let before = pipe.velocities_m_per_s().to_vec();
    assert!(
        pipe.step_with_series_resistance(dt, 1., length, -1.)
            .is_err()
    );
    assert_eq!(before, pipe.velocities_m_per_s());
    let mut dominant = RadialPipe::new(1., 1., 1., 64).unwrap();
    let q = dominant
        .step_with_series_resistance(dt, 1., length, 1e20)
        .unwrap()
        .0
        .flow_m3_per_s;
    assert!((q * 1e20 - 1.).abs() < 1e-12);
    let mut steady = RadialPipe::new(1., 1., 1., 128).unwrap();
    let mut q = 0.;
    for _ in 0..40 {
        q = steady
            .step_with_series_resistance(1., 1., length, resistance)
            .unwrap()
            .0
            .flow_m3_per_s;
    }
    let expected = 1. / (8. * length / std::f64::consts::PI + resistance);
    assert!((q / expected - 1.).abs() < 1e-4);
}

#[test]
fn radial_valve_closure_constrains_flux_without_erasing_profile_history() {
    use physics::lymph::profile::RadialPipe;
    let mut pipe = RadialPipe::new(1., 1., 1., 64).unwrap();
    for _ in 0..20 {
        pipe.step_with_ideal_valve(1., 1., 1., 1.).unwrap();
    }
    let (moving, tangent, reaction) = pipe.step_with_ideal_valve(1e-4, -0.01, 1., 1.).unwrap();
    assert!(moving.flow_m3_per_s > 0. && tangent > 0.);
    assert_eq!(reaction, 0.);
    let old = pipe.clone();
    let mut free = old.clone();
    let free_response = free.step(0.001, 0.).unwrap();
    let gradient = -free_response.flow_m3_per_s / free_response.flow_tangent;
    let (closed, tangent, reaction) = pipe.step_with_ideal_valve(0.001, -1e5, 1., 1.).unwrap();
    assert!(closed.flow_m3_per_s.abs() < 1e-13);
    assert_eq!(tangent, 0.);
    assert!(reaction > 0.);
    assert!((reaction - (gradient + 1e5)).abs() < 1e-10);
    assert!(closed.kinetic_energy_j_per_m > 0.);
    assert!(closed.kinetic_energy_j_per_m < moving.kinetic_energy_j_per_m);
    let mut independent = old;
    let checked = independent.step(0.001, gradient).unwrap();
    assert_eq!(closed.flow_m3_per_s, checked.flow_m3_per_s);
    assert_eq!(pipe.velocities_m_per_s(), independent.velocities_m_per_s());
    // Closed-valve constraint does no work at Q=0; viscous and BE loss dissipate history.
    assert!(
        closed.kinetic_energy_j_per_m - moving.kinetic_energy_j_per_m
            + 0.001 * closed.viscous_power_w_per_m
            <= 1e-12
    );
    let energy = closed.kinetic_energy_j_per_m;
    let (decay, _, r) = pipe.step_with_ideal_valve(0.001, -1e5, 1., 1.).unwrap();
    assert!(r > 0. && decay.flow_m3_per_s.abs() < 1e-13 && decay.kinetic_energy_j_per_m < energy);
    let (reopened, t, r) = pipe.step_with_ideal_valve(0.001, 1e5, 1., 1.).unwrap();
    assert!(reopened.flow_m3_per_s > 0. && t > 0.);
    assert_eq!(r, 0.);
    let before = pipe.velocities_m_per_s().to_vec();
    assert!(pipe.step_with_ideal_valve(0.001, f64::NAN, 1., 1.).is_err());
    assert_eq!(before, pipe.velocities_m_per_s());
}

#[test]
fn radial_network_couples_accepted_inventory_and_profile_atomically() {
    use physics::biomechanics::OsmoticPressureLaw;
    use physics::lymph::{RadialExchange, profile::RadialPipe};
    let laws = [OsmoticPressureLaw {
        linear: 0.2,
        quadratic: 0.03,
        cubic: 0.,
    }; 2];
    let mut e = edge(0, 1);
    e.reflection = 0.3;
    e.protein_permeability_m3_per_s = 0.02;
    let mut net = LymphNetwork::new(vec![space(1., 2.), space(0., 1.)], vec![e]).unwrap();
    let mut profiles = [RadialExchange {
        edge: 0,
        length_m: 1.,
        pipe: RadialPipe::new(1., 1., 1., 64).unwrap(),
    }];
    let dt = 0.001;
    for _ in 0..3 {
        let old = profiles[0].pipe.clone();
        let before_v = net.volumes().to_vec();
        let before_m = net.protein_masses().to_vec();
        let report = net
            .step_with_radial_profiles(dt, &laws, &mut profiles, 100, 1e-14, 1e-14)
            .unwrap();
        let q = net.rates().unwrap()[0].0;
        let c0 = net.protein_masses()[0] / net.volumes()[0];
        let c1 = net.protein_masses()[1] / net.volumes()[1];
        let drive = net.pressures()[0]
            - net.pressures()[1]
            - 0.3 * (0.2 * (c0 - c1) + 0.03 * (c0 * c0 - c1 * c1));
        let mut reference = old;
        let response = reference.step(dt, drive - q).unwrap();
        assert!((q - response.flow_m3_per_s).abs() < 1e-14);
        for (a, b) in reference
            .velocities_m_per_s()
            .iter()
            .zip(profiles[0].pipe.velocities_m_per_s())
        {
            assert!((a - b).abs() < 1e-14);
        }
        let j = 0.7 * q * c0 + 0.02 * (c0 - c1);
        assert!((net.rates().unwrap()[0].1 - j).abs() < 1e-14);
        assert!((net.volumes()[1] - before_v[1] - dt * q).abs() < 1e-14);
        assert!((net.protein_masses()[1] - before_m[1] - dt * j).abs() < 1e-14);
        assert!((report.transferred_volume_m3[0] - dt * q).abs() < 1e-16);
        assert!((net.total_volume() - 2.).abs() < 1e-13);
        assert!((net.total_protein() - 3.).abs() < 1e-13);
    }
    let before = (
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
        profiles[0].pipe.velocities_m_per_s().to_vec(),
    );
    assert!(
        net.step_with_radial_profiles(dt, &laws, &mut profiles, 1, 1e-30, 1e-30)
            .is_err()
    );
    assert_eq!(
        before,
        (
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap(),
            profiles[0].pipe.velocities_m_per_s().to_vec()
        )
    );
}

#[test]
fn closed_radial_network_valve_blocks_water_and_protein() {
    use physics::biomechanics::OsmoticPressureLaw;
    use physics::lymph::{RadialExchange, profile::RadialPipe};
    let laws = [OsmoticPressureLaw {
        linear: 0.,
        quadratic: 0.,
        cubic: 0.,
    }; 2];
    let mut e = edge(0, 1);
    e.valve = true;
    e.protein_permeability_m3_per_s = 0.1;
    let mut net = LymphNetwork::new(vec![space(0., 2.), space(1., 1.)], vec![e]).unwrap();
    let mut profiles = [RadialExchange {
        edge: 0,
        length_m: 1.,
        pipe: RadialPipe::new(1., 1., 1., 32).unwrap(),
    }];
    let before = (net.volumes().to_vec(), net.protein_masses().to_vec());
    let report = net
        .step_with_radial_profiles(0.01, &laws, &mut profiles, 100, 1e-14, 1e-14)
        .unwrap();
    assert_eq!(report.transferred_volume_m3, [0.]);
    assert_eq!(report.transferred_protein_kg, [0.]);
    assert_eq!(
        before,
        (net.volumes().to_vec(), net.protein_masses().to_vec())
    );
    assert!(
        profiles[0]
            .pipe
            .velocities_m_per_s()
            .iter()
            .all(|v| *v == 0.)
    );
}

#[test]
fn radial_harmonic_high_womersley_refines_to_independent_bessel_ratio() {
    use physics::lymph::profile::RadialPipe;
    fn mul(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
        [a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0]]
    }
    fn div(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
        let d = b[0] * b[0] + b[1] * b[1];
        [
            (a[0] * b[0] + a[1] * b[1]) / d,
            (a[1] * b[0] - a[0] * b[1]) / d,
        ]
    }
    // J_n/J_(n-1)=z/(2n-z*J_(n+1)/J_n), evaluated backwards.
    // Avoids catastrophic cancellation in J0/J1 power series at large complex z.
    fn exact(omega: f64, terms: usize) -> [f64; 2] {
        let a = (0.5 * omega).sqrt();
        let z = [a, -a];
        let mut ratio = [0.; 2];
        for n in (1..=terms).rev() {
            let zr = mul(z, ratio);
            ratio = div(z, [2. * n as f64 - zr[0], -zr[1]]);
        }
        let r = div([2. * ratio[0], 2. * ratio[1]], z);
        [
            -std::f64::consts::PI * r[1] / omega,
            -std::f64::consts::PI * (1. - r[0]) / omega,
        ]
    }
    for omega in [4., 100., 100000.] {
        let expected = exact(omega, 1000);
        let refined = exact(omega, 2000);
        assert!(
            (expected[0] - refined[0]).hypot(expected[1] - refined[1])
                < 1e-13 * expected[0].hypot(expected[1])
        );
        let mut errors = Vec::new();
        for rings in [256, 512, 1024] {
            let pipe = RadialPipe::new(1., 1., 1., rings).unwrap();
            let (velocities, q) = pipe.harmonic_response(omega, 1.).unwrap();
            assert!(velocities.iter().flatten().all(|x| x.is_finite()));
            assert!(pipe.velocities_m_per_s().iter().all(|x| *x == 0.));
            let error =
                (q[0] - expected[0]).hypot(q[1] - expected[1]) / expected[0].hypot(expected[1]);
            eprintln!(
                "harmonic alpha={} rings={rings} relative_error={error}",
                omega.sqrt()
            );
            errors.push(error);
        }
        assert!(errors[1] < 0.35 * errors[0] && errors[2] < 0.35 * errors[1]);
        assert!(errors[2] < 0.001);
    }
    let pipe = RadialPipe::new(1., 1., 1., 64).unwrap();
    assert!(pipe.harmonic_response(0., 1.).is_err());
    assert!(pipe.harmonic_response(1., f64::NAN).is_err());
}
