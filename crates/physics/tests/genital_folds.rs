use physics::skin::*;
fn material() -> SkinMaterial {
    SkinMaterial {
        layers: vec![Layer {
            thickness: 0.0003,
            density: 1000.,
            shear_modulus: 1000.,
            collagen_modulus: 2000.,
            collagen_exponent: 4.,
            dispersion: 0.1,
            fiber_angle: 0.5,
            relaxation: vec![Relaxation {
                modulus: 500.,
                time: 0.02,
            }],
        }],
    }
}
fn prepuce() -> PrepuceGeometry {
    PrepuceGeometry {
        inner_radius_m: 0.009,
        outer_radius_m: 0.012,
        length_m: 0.02,
        sectors: 12,
        rows_per_section: 4,
    }
}
fn labia() -> LabiaMinoraGeometry {
    LabiaMinoraGeometry {
        length_m: 0.04,
        fold_width_m: 0.008,
        fold_height_m: 0.004,
        separation_m: 0.014,
        longitudinal_segments: 6,
        transverse_segments: 4,
    }
}
#[test]
fn folded_specimens_have_continuous_topology_and_stress_free_rest() {
    let p = prepuce().build(material()).unwrap();
    assert_eq!(p.skin.positions().len(), 156);
    assert_eq!(p.skin.triangles().len(), 288);
    let mut edges = std::collections::BTreeMap::new();
    for face in p.skin.triangles() {
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    assert!(edges.values().all(|n| *n == 1 || *n == 2));
    assert_eq!(edges.values().filter(|n| **n == 1).count(), 24);
    let pair = labia().build(material()).unwrap();
    for fold in [&p, &pair[0], &pair[1]] {
        assert!(fold.skin.stored_energy().unwrap().abs() < 1e-12);
        assert!(
            fold.skin
                .internal_forces()
                .unwrap()
                .iter()
                .flatten()
                .all(|x| x.abs() < 1e-9)
        );
        assert!(fold.skin.masses().iter().all(|m| *m > 0.));
        assert!(!fold.crest_nodes.is_empty());
    }
    for (a, b) in pair[0]
        .skin
        .positions()
        .iter()
        .zip(pair[1].skin.positions())
    {
        assert_eq!(a[0], -b[0]);
        assert_eq!(a[1], b[1]);
        assert_eq!(a[2], b[2]);
    }
    let mut bad = prepuce();
    bad.outer_radius_m = bad.inner_radius_m;
    assert!(bad.build(material()).is_err());
    let mut bad = labia();
    bad.separation_m = bad.fold_width_m;
    assert!(bad.build(material()).is_err());
}
#[test]
fn both_folds_deform_under_load_keep_roots_and_release_without_inversion() {
    let pair = labia().build(material()).unwrap();
    let specimens = [
        prepuce().build(material()).unwrap(),
        pair[0].clone(),
        pair[1].clone(),
    ];
    for mut fold in specimens {
        let initial = fold.skin.positions().to_vec();
        let mut forces = vec![[0.; 3]; initial.len()];
        for &node in &fold.crest_nodes {
            forces[node][2] = 0.0001 / fold.crest_nodes.len() as f64;
        }
        for _ in 0..4 {
            let r = fold
                .skin
                .step(0.002, [0.; 3], &forces, &[], SolverConfig::default())
                .unwrap();
            assert!(r.min_area_ratio > 0.);
        }
        assert!(
            fold.crest_nodes
                .iter()
                .any(|&i| fold.skin.positions()[i][2] > initial[i][2] + 1e-9)
        );
        for &i in &fold.root_nodes {
            assert_eq!(fold.skin.positions()[i], initial[i]);
        }
        forces.fill([0.; 3]);
        for _ in 0..4 {
            fold.skin
                .step(0.002, [0.; 3], &forces, &[], SolverConfig::default())
                .unwrap();
        }
        assert!(
            fold.skin
                .surface_metrics()
                .unwrap()
                .iter()
                .all(|m| m.area_ratio > 0. && m.thickness > 0.)
        );
        let accepted = fold.skin.positions().to_vec();
        let velocity = fold.skin.velocities().to_vec();
        assert!(
            fold.skin
                .step(0., [0.; 3], &forces, &[], SolverConfig::default())
                .is_err()
        );
        assert_eq!(fold.skin.positions(), accepted);
        assert_eq!(fold.skin.velocities(), velocity);
    }
}
#[test]
fn labial_fold_uses_positive_gap_triangle_contact() {
    let mut fold = labia().build(material()).unwrap()[0].clone();
    let tip = fold.skin.positions()[fold.crest_nodes[2]];
    let sphere = ContactSphere {
        center: [tip[0], tip[1], tip[2] + 0.003],
        radius: 0.001,
        velocity: [0.; 3],
    };
    let contacts = ContactScene {
        spheres: vec![sphere],
        planes: vec![],
        distance: 0.003,
        stiffness: 10000.,
    };
    let force = fold.skin.contact_forces(&contacts).unwrap();
    assert!(force.iter().any(|f| f[2] < 0.));
    fold.skin
        .step_with_contacts(
            0.001,
            [0.; 3],
            &vec![[0.; 3]; fold.skin.positions().len()],
            &[],
            &contacts,
            SolverConfig::default(),
        )
        .unwrap();
    assert!(fold.skin.positions().iter().all(|p| {
        p.iter()
            .zip(sphere.center)
            .map(|(x, c)| (x - c).powi(2))
            .sum::<f64>()
            .sqrt()
            > sphere.radius
    }));
    let saved = fold.skin.positions().to_vec();
    let bad = ContactScene {
        spheres: vec![ContactSphere {
            center: tip,
            radius: 0.01,
            velocity: [0.; 3],
        }],
        ..contacts
    };
    assert!(
        fold.skin
            .step_with_contacts(
                0.001,
                [0.; 3],
                &vec![[0.; 3]; saved.len()],
                &[],
                &bad,
                SolverConfig::default()
            )
            .is_err()
    );
    assert_eq!(fold.skin.positions(), saved);
}

#[test]
fn majora_volumetric_cores_compress_with_fixed_bases_and_refine_volume() {
    use physics::biomechanics::{LabiaMajoraGeometry, Material};
    let geometry = LabiaMajoraGeometry {
        length_m: 0.05,
        width_m: 0.018,
        height_m: 0.008,
        separation_m: 0.03,
        sectors: 12,
        latitude_rings: 3,
    };
    let material = Material::from_young_poisson(3000., 0.45).unwrap();
    let mut pads = geometry.build(material.clone()).unwrap();
    let volume = |b: &physics::biomechanics::Body| {
        b.stresses_at(b.positions())
            .unwrap()
            .iter()
            .map(|e| e.reference_volume_m3)
            .sum::<f64>()
    };
    let exact = 2. / 3. * std::f64::consts::PI * geometry.width_m / 2. * geometry.length_m / 2.
        * geometry.height_m;
    let mut refined = geometry;
    refined.sectors = 24;
    refined.latitude_rings = 6;
    let fine = refined.build(material).unwrap();
    assert!((volume(&fine[0].body) - exact).abs() < (volume(&pads[0].body) - exact).abs() / 3.);
    for pad in &mut pads {
        let original = pad.body.positions().to_vec();
        pad.body.set_force(pad.apex_node, [0., 0., -0.001]).unwrap();
        let report = pad.body.equilibrate(20000, 1e-7).unwrap();
        assert!(report.converged && report.min_j > 0.);
        assert!(pad.body.positions()[pad.apex_node][2] < original[pad.apex_node][2]);
        for &i in &pad.base_nodes {
            assert_eq!(pad.body.positions()[i], original[i]);
        }
        let depressed = pad.body.positions()[pad.apex_node][2];
        pad.body.set_force(pad.apex_node, [0.; 3]).unwrap();
        assert!(pad.body.equilibrate(20000, 1e-7).unwrap().converged);
        assert!(pad.body.positions()[pad.apex_node][2] > depressed);
    }
}

#[test]
fn labia_majora_gait_contact_simulation_responds_to_walk_and_run_and_preserves_cleft_gap() {
    use physics::biomechanics::{LabiaMajoraGeometry, Material, PelvisGaitKinematics};
    let geometry = LabiaMajoraGeometry {
        length_m: 0.05,
        width_m: 0.018,
        height_m: 0.008,
        separation_m: 0.022,
        sectors: 8,
        latitude_rings: 2,
    };
    let mat = Material::from_young_poisson(3000., 0.45).unwrap();
    let walk = PelvisGaitKinematics {
        stride_frequency_hz: 1.0,
        vertical_amplitude_m: 0.001,
        lateral_amplitude_m: 0.0005,
        sagittal_amplitude_m: 0.0005,
        phase_rad: 0.0,
    };
    let run = PelvisGaitKinematics {
        stride_frequency_hz: 1.8,
        vertical_amplitude_m: 0.0025,
        lateral_amplitude_m: 0.001,
        sagittal_amplitude_m: 0.0012,
        phase_rad: 0.0,
    };
    let mut peak_kinetic = [0.0_f64; 2];
    for (idx, gait) in [walk, run].iter().enumerate() {
        let mut sim = geometry
            .build_gait_contact_simulation(
                [mat.clone(), mat.clone()],
                [1000.; 2],
                0.0001,
                0.0015,
                1.0,
            )
            .unwrap();
        for _ in 0..6 {
            let report = sim.step_gait(gait, 5e-5, 1e-4).unwrap();
            assert!(report.energy_defect_j.is_finite() && report.energy_defect_j.abs() <= 1e-4);
            let diag = sim.assembly.body.diagnostics().unwrap();
            peak_kinetic[idx] = peak_kinetic[idx].max(diag.kinetic_j);
        }
        assert!(sim.min_contact_gap_m() > 0.0001);
        assert!(sim.time_s() > 0.);
    }
    assert!(
        peak_kinetic[1] > peak_kinetic[0],
        "running must induce higher labia majora kinetic energy than walking: {peak_kinetic:?}"
    );
}

#[test]
fn labia_minora_gait_and_surface_film_smearing_conserve_mass_and_invert_no_elements() {
    use physics::biomechanics::PelvisGaitKinematics;
    use physics::friction;
    use physics::surface_film::{Material as FilmMaterial, SmearContact, SmearRegime};

    let mut sim = labia().build_gait_simulation(material()).unwrap();
    let gait = PelvisGaitKinematics {
        stride_frequency_hz: 1.4,
        vertical_amplitude_m: 0.0015,
        lateral_amplitude_m: 0.0008,
        sagittal_amplitude_m: 0.0008,
        phase_rad: 0.0,
    };
    for _ in 0..4 {
        let reports = sim
            .step_gait(
                &gait,
                0.001,
                [0., -9.81, 0.],
                &ContactScene::default(),
                SolverConfig::default(),
            )
            .unwrap();
        for r in reports {
            assert!(r.min_area_ratio > 0.9);
            assert!(r.energy.is_finite());
        }
    }
    assert!(sim.time_s() > 0.);

    let mut films = sim
        .build_surface_films(FilmMaterial {
            density: 1000.,
            viscosity: 0.05,
            wetting: 1e-5,
            surface_tension: 0.05,
        })
        .unwrap();
    for film in &mut films {
        let cells = film.triangles().len();
        film.deposit(0, 1e-8).unwrap();
        film.deposit(cells / 2, 1e-8).unwrap();
        let mass_before = film.total_mass();
        let friction_mat = friction::Material::new(1e6, 1e6, 0.4).unwrap();
        let mut states = vec![friction::State::default(); cells];
        let pressures = vec![500.0; cells];
        let report = film
            .advance_smear(
                0.002,
                SmearContact {
                    normal: [0., 0., 1.],
                    body_velocity_m_s: [0., 0.02, 0.],
                },
                &pressures,
                &friction_mat,
                &mut states,
                [0., -9.81, 0.],
                0.001,
            )
            .unwrap();
        assert!(report.regimes.contains(&SmearRegime::Lubricated));
        assert!(report.slip_work_j > 0.);
        assert!((film.total_mass() - mass_before).abs() < 1e-14);
    }
}

#[test]
fn tube_insertion_parts_labia_minora_and_majora_laterally_without_element_inversion() {
    use physics::biomechanics::{LabiaMajoraGeometry, Material};

    let mut minora_sim = labia().build_gait_simulation(material()).unwrap();
    let initial_minora_sep = minora_sim.mean_crest_separation_m();

    let tube = LabiaTubeInsertion {
        radius_m: 0.0035,
        length_m: 0.028,
        axis_xy_m: [0.0, 0.0],
        start_tip_z_m: 0.010,
        max_depth_m: 0.014,
        frequency_hz: 4.0,
    };

    let mut peak_minora_sep = initial_minora_sep;
    for _ in 0..10 {
        let reports = minora_sim
            .step_tube_insertion(
                &tube,
                0.002,
                [0.0, 0.0, -9.81],
                6,
                0.0015,
                2500.0,
                0.0012,
                SolverConfig::default(),
            )
            .unwrap();
        for r in reports {
            assert!(r.min_area_ratio > 0.85);
            assert!(r.energy.is_finite());
        }
        peak_minora_sep = peak_minora_sep.max(minora_sim.mean_crest_separation_m());
    }
    assert!(
        peak_minora_sep > initial_minora_sep + 1e-6,
        "tube insertion must part labia minora crests laterally: initial={initial_minora_sep} peak={peak_minora_sep}"
    );

    let majora_geom = LabiaMajoraGeometry {
        length_m: 0.05,
        width_m: 0.018,
        height_m: 0.008,
        separation_m: 0.022,
        sectors: 8,
        latitude_rings: 2,
    };
    let mat = Material::from_young_poisson(3000., 0.45).unwrap();
    let mut majora_sim = majora_geom
        .build_gait_contact_simulation([mat.clone(), mat], [1000.; 2], 0.0001, 0.0015, 1.0)
        .unwrap();
    let initial_majora_sep = majora_sim.apex_separation_m();
    for _ in 0..4 {
        majora_sim
            .step_tube_insertion(0.85, -0.04, 0.025, 5e-5, 1e-4)
            .unwrap();
    }
    assert!(
        majora_sim.apex_separation_m() > initial_majora_sep,
        "tube insertion must part labia majora apices laterally"
    );
}
