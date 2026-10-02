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
