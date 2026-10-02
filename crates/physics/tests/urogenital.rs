use physics::biomechanics::*;
fn materials() -> [Material; 4] {
    let passive = Material {
        shear_pa: 500.,
        bulk_pa: 5000.,
        fibers: vec![],
    };
    let muscle = |direction| Material {
        fibers: vec![Fiber {
            direction,
            stiffness_pa: 50.,
            exponent: 2.,
            active_pa: 1000.,
        }],
        ..passive.clone()
    };
    [
        passive.clone(),
        muscle([0., 0., 1.]),
        muscle([1., 0., 0.]),
        muscle([1., 0., 0.]),
    ]
}
fn volume(body: &Body) -> f64 {
    body.cavities()[0]
        .faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|i| body.positions()[i]);
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.
        })
        .sum()
}
fn length(body: &Body) -> f64 {
    body.positions()
        .iter()
        .map(|p| p[2])
        .fold(f64::NEG_INFINITY, f64::max)
}
#[test]
fn urethral_and_vaginal_layers_have_distinct_muscle_actions_and_pressure_response() {
    for (geometry, urethra) in [
        (
            UrogenitalWallGeometry {
                radii_m: [0.0015, 0.0018, 0.0022, 0.0026, 0.003],
                length_m: 0.02,
                sectors: 8,
                segments: 2,
            },
            true,
        ),
        (
            UrogenitalWallGeometry {
                radii_m: [0.008, 0.009, 0.01, 0.011, 0.012],
                length_m: 0.03,
                sectors: 8,
                segments: 2,
            },
            false,
        ),
    ] {
        let build = || {
            if urethra {
                geometry.urethra(materials())
            } else {
                geometry.vagina(materials())
            }
        };
        let original = build().unwrap();
        let mut circular = original.clone();
        for i in 0..circular.elements().len() {
            if circular.elements()[i].region == 2 {
                circular.set_activation(i, 0.03).unwrap();
            }
        }
        assert!(circular.equilibrate(50000, 1e-7).unwrap().converged);
        assert!(volume(&circular) < volume(&original));
        let mut longitudinal = original.clone();
        for i in 0..longitudinal.elements().len() {
            if longitudinal.elements()[i].region == 1 {
                longitudinal.set_activation(i, 0.03).unwrap();
            }
        }
        assert!(longitudinal.equilibrate(50000, 1e-7).unwrap().converged);
        assert!(length(&longitudinal) < length(&original));
        let mut loaded = original.clone();
        loaded.set_pressure(0, 20.).unwrap();
        assert!(loaded.equilibrate(50000, 1e-7).unwrap().converged);
        assert!(volume(&loaded) > volume(&original));
        for body in [&circular, &longitudinal, &loaded] {
            assert!(
                body.stresses_at(body.positions())
                    .unwrap()
                    .iter()
                    .all(|e| e.volume_ratio > 0.)
            );
            for (p, rest) in body.positions().iter().zip(original.positions()).take(40) {
                assert_eq!(p, rest);
            }
        }
    }
}
fn clitoris() -> ClitoralGeometry {
    ClitoralGeometry {
        corpus_radius_m: 0.002,
        crus_length_m: 0.018,
        body_length_m: 0.01,
        root_half_separation_m: 0.012,
        body_half_separation_m: 0.003,
        glans_radii_m: [0.004, 0.003, 0.004],
        bulb_radii_m: [0.005, 0.007, 0.01],
        bulb_half_separation_m: 0.015,
        sectors: 8,
        segments: 3,
    }
}
#[test]
fn clitoral_complex_contains_continuous_crura_corpora_and_deformable_glans_bulbs() {
    let material = Material::from_young_poisson(3000., 0.4).unwrap();
    let mut complex = clitoris()
        .build(material.clone(), material.clone(), material)
        .unwrap();
    for body in complex
        .corpora_crura
        .iter_mut()
        .chain(std::iter::once(&mut complex.glans))
        .chain(complex.vestibular_bulbs.iter_mut())
    {
        let initial = body.positions().to_vec();
        let tip = (0..initial.len())
            .max_by(|&a, &b| initial[a][2].total_cmp(&initial[b][2]))
            .unwrap();
        body.set_force(tip, [0., 0., -0.0001]).unwrap();
        let report = body.equilibrate(50000, 1e-7).unwrap();
        assert!(report.converged && report.min_j > 0.);
        assert!(body.positions()[tip][2] < initial[tip][2]);
        let mut edges = std::collections::BTreeMap::new();
        for face in body.surface() {
            for (u, v) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
                *edges.entry((u.min(v), u.max(v))).or_insert(0) += 1;
            }
        }
        assert!(edges.values().all(|n| *n == 2));
    }
    let mut bad = clitoris();
    bad.body_half_separation_m = bad.corpus_radius_m;
    let m = materials()[0].clone();
    assert!(bad.build(m.clone(), m.clone(), m).is_err());
}

#[test]
fn corpus_fiber_frames_follow_curved_crus_centerline() {
    let mut m = materials()[0].clone();
    m.fibers = vec![Fiber {
        direction: [0., 0., 1.],
        stiffness_pa: 100.,
        exponent: 2.,
        active_pa: 10.,
    }];
    let complex = clitoris()
        .build(m, materials()[0].clone(), materials()[0].clone())
        .unwrap();
    for body in &complex.corpora_crura {
        let p = body.positions();
        for row in 0..6 {
            let first = p[row * 9];
            let second = p[(row + 1) * 9];
            let delta: [f64; 3] = std::array::from_fn(|k| second[k] - first[k]);
            let norm = delta.iter().map(|x| x * x).sum::<f64>().sqrt();
            for e in &body.elements()[row * 24..(row + 1) * 24] {
                for k in 0..3 {
                    assert!((e.material.fibers[0].direction[k] - delta[k] / norm).abs() < 1e-14);
                }
            }
        }
    }
}

#[test]
fn objective_tissue_bonds_have_consistent_energy_gradient_and_atomic_installation() {
    let material = Material::from_young_poisson(1000., 0.3).unwrap();
    let tet = |offset: f64| {
        Body::new(
            vec![
                [offset, 0., 0.],
                [offset + 0.01, 0., 0.],
                [offset, 0.01, 0.],
                [offset, 0., 0.01],
            ],
            vec![false; 4],
            vec![([0, 1, 2, 3], material.clone())],
        )
        .unwrap()
    };
    let mut a = Body::assemble_tissues(&[tet(0.), tet(0.02)]).unwrap();
    assert_eq!(a.node_ranges, vec![0..4, 4..8]);
    let no_bond = a.body.clone();
    a.body.add_tissue_bonds(&[([0, 4], 100.)]).unwrap();
    let mut x = a.body.positions().to_vec();
    for p in &mut x[4..] {
        p[0] += 0.001;
    }
    let (e, g) = a.body.evaluate(&x).unwrap();
    let (base, _) = no_bond.evaluate(&x).unwrap();
    assert!((e - base - 0.5 * 100. * 0.001_f64.powi(2)).abs() < 1e-14);
    assert!((g[0][0] + 0.1).abs() < 1e-12 && (g[4][0] - 0.1).abs() < 1e-12);
    for i in [0, 4] {
        for axis in 0..3 {
            let mut plus = x.clone();
            let mut minus = x.clone();
            plus[i][axis] += 1e-7;
            minus[i][axis] -= 1e-7;
            let numerical =
                (a.body.evaluate(&plus).unwrap().0 - a.body.evaluate(&minus).unwrap().0) / 2e-7;
            assert!((numerical - g[i][axis]).abs() < 1e-8);
        }
    }
    let rotated: Vec<_> = x
        .iter()
        .map(|p| [-p[1] + 0.3, p[0] - 0.2, p[2] + 0.1])
        .collect();
    let (re, rg) = a.body.evaluate(&rotated).unwrap();
    assert!((re - e).abs() < 1e-12);
    for (original, rotated) in g.iter().zip(rg) {
        assert!((rotated[0] + original[1]).abs() < 1e-11);
        assert!((rotated[1] - original[0]).abs() < 1e-11);
        assert!((rotated[2] - original[2]).abs() < 1e-11);
    }
    assert!(
        a.body
            .add_tissue_bonds(&[([1, 5], 100.), ([2, 99], 100.)])
            .is_err()
    );
    assert_eq!(a.body.tissue_bonds().len(), 1);
    assert!(a.body.add_tissue_bonds(&[([4, 0], 100.)]).is_err());
}
#[test]
fn bonded_clitoral_parts_transfer_load_to_crus_roots() {
    let m = Material::from_young_poisson(3000., 0.4).unwrap();
    let complex = clitoris().build(m.clone(), m.clone(), m).unwrap();
    let mut assembly = complex.coupled(1., 6).unwrap();
    let original = assembly.body.positions().to_vec();
    let range = assembly.node_ranges[2].clone();
    let tip = range
        .max_by(|&a, &b| original[a][2].total_cmp(&original[b][2]))
        .unwrap();
    assembly.body.set_force(tip, [0., 0., -0.0001]).unwrap();
    let report = assembly.body.equilibrate(100000, 1e-7).unwrap();
    println!(
        "coupled clitoral residual={}, iterations={}, min_j={}",
        report.residual_n, report.iterations, report.min_j
    );
    assert!(report.converged && report.min_j > 0.);
    assert!(assembly.body.positions()[tip][2] < original[tip][2]);
    let corpus_nodes = assembly.node_ranges[0]
        .clone()
        .chain(assembly.node_ranges[1].clone());
    assert!(
        corpus_nodes
            .clone()
            .any(|i| (assembly.body.positions()[i][2] - original[i][2]).abs() > 1e-9)
    );
    let gradient = assembly.body.evaluate(assembly.body.positions()).unwrap().1;
    // Only the two corpus/crus basal sections retain pins (9 nodes each).
    let roots = (0..9).chain(assembly.node_ranges[1].start..assembly.node_ranges[1].start + 9);
    let mut reaction = [0.; 3];
    for i in roots {
        assert_eq!(assembly.body.positions()[i], original[i]);
        for k in 0..3 {
            reaction[k] += gradient[i][k];
        }
    }
    assert!((reaction[2] - 0.0001).abs() < 5e-6);
    assert!(reaction[0].abs() < 5e-6 && reaction[1].abs() < 5e-6);
}

#[test]
fn tissue_bonds_participate_in_inertial_motion_without_internal_momentum_loss() {
    let material = Material::from_young_poisson(1000., 0.3).unwrap();
    let tet = |offset: f64| {
        Body::new(
            vec![
                [offset, 0., 0.],
                [offset + 0.01, 0., 0.],
                [offset, 0.01, 0.],
                [offset, 0., 0.01],
            ],
            vec![false; 4],
            vec![([0, 1, 2, 3], material.clone())],
        )
        .unwrap()
    };
    let mut assembly = Body::assemble_tissues(&[tet(0.), tet(0.02)]).unwrap();
    assembly.body.add_tissue_bonds(&[([0, 4], 10.)]).unwrap();
    let velocities = (0..8)
        .map(|i| [if i < 4 { -0.01 } else { 0.01 }, 0., 0.])
        .collect();
    let mut b = InertialBody::new(assembly.body, &[1000.; 2], velocities).unwrap();
    let before = b.diagnostics().unwrap();
    for _ in 0..100 {
        b.step(1e-5, 1e-9).unwrap();
    }
    let after = b.diagnostics().unwrap();
    for axis in 0..3 {
        assert!((after.momentum_kg_m_s[axis] - before.momentum_kg_m_s[axis]).abs() < 1e-14);
        assert!(
            (after.angular_momentum_kg_m2_s[axis] - before.angular_momentum_kg_m2_s[axis]).abs()
                < 1e-14
        );
    }
    assert!(
        (after.kinetic_j + after.potential_j - before.kinetic_j - before.potential_j).abs() < 1e-9
    );
    assert!(b.velocities()[0][0] > -0.01);
}

#[test]
fn oval_wall_has_exact_polygon_volume_and_orthonormal_muscle_frames() {
    let geometry = UrogenitalWallGeometry {
        radii_m: [0.008, 0.009, 0.01, 0.011, 0.012],
        length_m: 0.03,
        sectors: 8,
        segments: 2,
    };
    let scales = [1., 0.3];
    let body = geometry.oval_wall(scales, materials()).unwrap();
    let expected = 0.5
        * geometry.sectors as f64
        * (std::f64::consts::TAU / geometry.sectors as f64).sin()
        * geometry.radii_m[0].powi(2)
        * scales[0]
        * scales[1]
        * geometry.length_m;
    assert!((volume(&body) / expected - 1.).abs() < 1e-13);
    assert!(body.evaluate(body.positions()).unwrap().0.abs() < 1e-12);
    assert!(
        body.stresses_at(body.positions())
            .unwrap()
            .iter()
            .all(|s| (s.volume_ratio - 1.).abs() < 1e-13)
    );
    for (i, cell) in body.elements().iter().enumerate() {
        if cell.region == 2 {
            let sector = (i / 6) % geometry.sectors;
            let theta = (sector as f64 + 0.5) * std::f64::consts::TAU / geometry.sectors as f64;
            let tangent = [-scales[0] * theta.sin(), scales[1] * theta.cos(), 0.];
            let norm = tangent[0].hypot(tangent[1]);
            let fiber = cell.material.fibers[0].direction;
            for k in 0..3 {
                assert!((fiber[k] - tangent[k] / norm).abs() < 1e-13);
            }
            let normal = [theta.cos() / scales[0], theta.sin() / scales[1]];
            assert!((fiber[0] * normal[0] + fiber[1] * normal[1]).abs() < 1e-13);
        }
    }
    let circular = geometry.vagina(materials()).unwrap();
    let identity = geometry.oval_wall([1., 1.], materials()).unwrap();
    assert_eq!(circular.positions(), identity.positions());
    assert_eq!(volume(&circular), volume(&identity));
    for scales in [[0., 1.], [-1., 1.], [f64::NAN, 1.], [1., f64::INFINITY]] {
        assert!(geometry.oval_wall(scales, materials()).is_err());
    }
}
#[test]
fn oval_wall_pressure_and_muscles_act_without_reference_prestrain() {
    let geometry = UrogenitalWallGeometry {
        radii_m: [0.008, 0.009, 0.01, 0.011, 0.012],
        length_m: 0.03,
        sectors: 8,
        segments: 2,
    };
    let original = geometry.oval_wall([1., 0.3], materials()).unwrap();
    for region in [1, 2] {
        let mut body = original.clone();
        for i in 0..body.elements().len() {
            if body.elements()[i].region == region {
                body.set_activation(i, 0.03).unwrap();
            }
        }
        let result = body.equilibrate(100000, 1e-7).unwrap();
        assert!(result.converged && result.min_j > 0.);
        if region == 1 {
            assert!(length(&body) < length(&original));
        } else {
            assert!(volume(&body) < volume(&original));
        }
        for (i, position) in body.positions().iter().enumerate() {
            if original.positions()[i][2] == 0. {
                assert_eq!(position, &original.positions()[i]);
            }
        }
    }
    let mut loaded = original.clone();
    loaded.set_pressure(0, 20.).unwrap();
    let result = loaded.equilibrate(100000, 1e-7).unwrap();
    assert!(result.converged && result.min_j > 0.);
    assert!(volume(&loaded) > volume(&original));
}

#[test]
fn axial_material_profile_preserves_mesh_and_localizes_muscle_drive() {
    let geometry = UrogenitalWallGeometry {
        radii_m: [0.0015, 0.0018, 0.0022, 0.0026, 0.003],
        length_m: 0.02,
        sectors: 8,
        segments: 3,
    };
    let mut profiles = vec![materials(); 3];
    // Explicit synthetic middle-segment outer muscle; passive outer ends.
    profiles[0][3].fibers.clear();
    profiles[2][3].fibers.clear();
    profiles[2][0].shear_pa = 700.;
    let mut body = geometry.axial_wall([1., 0.6], &profiles).unwrap();
    let baseline = geometry.oval_wall([1., 0.6], materials()).unwrap();
    assert_eq!(body.rest_positions(), baseline.rest_positions());
    assert_eq!(body.surface(), baseline.surface());
    assert_eq!(volume(&body), volume(&baseline));
    for (i, element) in body.elements().iter().enumerate() {
        let zone = i / (geometry.sectors * 6);
        assert_eq!(element.region, zone);
        let material = &profiles[zone / 4][zone % 4];
        assert_eq!(element.material.shear_pa, material.shear_pa);
        assert_eq!(element.material.fibers.len(), material.fibers.len());
        assert_eq!(element.nodes, baseline.elements()[i].nodes);
    }
    let original = body.clone();
    let mut drives = [MuscleRegionDrive {
        region: 7,
        activation: 0.,
        excitation: 0.05,
        kinetics: ActivationKinetics {
            rise_seconds: 0.2,
            fall_seconds: 0.5,
            tonic_activation: 0.,
        },
    }];
    let report = body
        .step_muscle_regions(&mut drives, 0.2, 100000, 1e-7)
        .unwrap();
    assert!(report.converged && report.min_j > 0.);
    assert!(volume(&body) < volume(&original));
    for (i, position) in body.positions().iter().enumerate() {
        if original.rest_positions()[i][2] == 0. {
            assert_eq!(position, &original.positions()[i]);
        }
    }
    for element in body.elements() {
        assert_eq!(
            element.activation,
            if element.region == 7 {
                drives[0].activation
            } else {
                0.
            }
        );
    }
    assert!(geometry.axial_wall([1., 1.], &profiles[..2]).is_err());
    let mut invalid = profiles.clone();
    invalid[2][0].shear_pa = -1.;
    assert!(geometry.axial_wall([1., 1.], &invalid).is_err());
}
