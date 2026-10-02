use physics::skin::{Layer, Relaxation, Skin, SkinMaterial, SolverConfig, patch};
fn matrix_material() -> SkinMaterial {
    SkinMaterial {
        layers: vec![Layer {
            thickness: 0.001,
            density: 1000.0,
            shear_modulus: 20_000.0,
            collagen_modulus: 0.0,
            collagen_exponent: 8.0,
            dispersion: 0.0,
            fiber_angle: 0.0,
            relaxation: vec![],
        }],
    }
}
fn triangle(material: SkinMaterial, pins: &[usize]) -> Skin {
    Skin::new(
        vec![[0.0, 0.0, 0.0], [0.02, 0.0, 0.0], [0.0, 0.02, 0.0]],
        vec![[0, 1, 2]],
        pins,
        material,
        vec![[1.0, 0.0, 0.0]],
    )
    .unwrap()
}
fn norm(v: [f64; 3]) -> f64 {
    v.iter().map(|v| v * v).sum::<f64>().sqrt()
}
#[test]
fn material_gradient_and_tangent_match_finite_differences() {
    let m = SkinMaterial::default();
    let c = [1.4, 0.9, 0.08];
    let r = m.response(c).unwrap();
    let h = 1e-5;
    for i in 0..3 {
        let mut plus = c;
        let mut minus = c;
        plus[i] += h;
        minus[i] -= h;
        let p = m.response(plus).unwrap();
        let n = m.response(minus).unwrap();
        let numerical = (p.energy - n.energy) / (2.0 * h);
        assert!((numerical - r.gradient[i]).abs() < 1e-5 * numerical.abs().max(1.0));
        for j in 0..3 {
            let numerical = (p.gradient[j] - n.gradient[j]) / (2.0 * h);
            assert!((numerical - r.tangent[j][i]).abs() < 1e-5 * numerical.abs().max(1.0));
        }
    }
}
#[test]
fn exact_rest_has_zero_energy_and_force() {
    let s = patch(3, 3, 0.01, SkinMaterial::default()).unwrap();
    assert!(s.stored_energy().unwrap().abs() < 1e-12);
    assert!(s.internal_forces().unwrap().iter().all(|v| norm(*v) < 1e-9));
}
#[test]
fn nonlinear_fibers_are_tension_only_and_directional() {
    let mut m = matrix_material();
    let l = &mut m.layers[0];
    l.collagen_modulus = 100_000.0;
    l.dispersion = 0.0;
    l.fiber_angle = 0.0;
    let along = m.response([1.44, 1.0, 0.0]).unwrap();
    let across = m.response([1.0, 1.44, 0.0]).unwrap();
    assert!(along.energy > across.energy * 5.0);
    let low = m.response([1.1, 1.0, 0.0]).unwrap();
    let high = m.response([1.5, 1.0, 0.0]).unwrap();
    assert!(high.tangent[0][0] > low.tangent[0][0] * 3.0);
    assert!(
        (m.response([0.8, 1.0, 0.0]).unwrap().energy
            - matrix_material().response([0.8, 1.0, 0.0]).unwrap().energy)
            .abs()
            < 1e-12
    );
}
#[test]
fn incompressibility_tracks_thickness() {
    let mut s = triangle(matrix_material(), &[]);
    let p = s
        .positions()
        .iter()
        .map(|p| [p[0] * 1.2, p[1] * 1.1, p[2]])
        .collect();
    s.set_state(p, vec![[0.0; 3]; 3]).unwrap();
    assert!((s.thicknesses().unwrap()[0] * 1.2 * 1.1 - 0.001).abs() < 1e-12);
}
#[test]
fn forces_are_objective_and_conserve_linear_and_angular_momentum() {
    let mut s = patch(3, 3, 0.01, SkinMaterial::default()).unwrap();
    let p: Vec<_> = s
        .positions()
        .iter()
        .map(|p| [p[0] * 1.15, p[1] * 0.98, 0.002 * (p[0] * 100.0).sin()])
        .collect();
    s.set_state(p.clone(), vec![[0.0; 3]; 9]).unwrap();
    let f = s.internal_forces().unwrap();
    let e = s.stored_energy().unwrap();
    let mut total = [0.0; 3];
    let mut torque = [0.0; 3];
    for (&p, &f) in p.iter().zip(&f) {
        for i in 0..3 {
            total[i] += f[i];
        }
        torque[0] += p[1] * f[2] - p[2] * f[1];
        torque[1] += p[2] * f[0] - p[0] * f[2];
        torque[2] += p[0] * f[1] - p[1] * f[0];
    }
    assert!(norm(total) < 1e-8);
    assert!(norm(torque) < 1e-9);
    let rotate = |p: [f64; 3]| [p[2], p[0], p[1]];
    let rotated = p
        .iter()
        .map(|&p| {
            let r = rotate(p);
            [r[0] + 2.0, r[1] - 3.0, r[2] + 1.0]
        })
        .collect();
    s.set_state(rotated, vec![[0.0; 3]; 9]).unwrap();
    assert!((s.stored_energy().unwrap() - e).abs() < 1e-9);
    let rf = s.internal_forces().unwrap();
    for (a, b) in f.into_iter().map(rotate).zip(rf) {
        assert!(norm(std::array::from_fn(|i| a[i] - b[i])) < 1e-7);
    }
}
#[test]
fn shell_energy_gradient_matches_finite_difference_including_hinges() {
    let mut s = patch(3, 3, 0.01, SkinMaterial::default()).unwrap();
    let p: Vec<_> = s
        .positions()
        .iter()
        .map(|p| [p[0] * 1.1, p[1], 0.002 * (p[0] * 100.0).sin()])
        .collect();
    s.set_state(p.clone(), vec![[0.0; 3]; 9]).unwrap();
    let forces = s.internal_forces().unwrap();
    let h = 1e-7;
    for vertex in 0..9 {
        for axis in 0..3 {
            let mut a = p.clone();
            let mut b = p.clone();
            a[vertex][axis] += h;
            b[vertex][axis] -= h;
            s.set_state(a, vec![[0.0; 3]; 9]).unwrap();
            let ep = s.stored_energy().unwrap();
            s.set_state(b, vec![[0.0; 3]; 9]).unwrap();
            let en = s.stored_energy().unwrap();
            let numerical = -(ep - en) / (2.0 * h);
            assert!(
                (numerical - forces[vertex][axis]).abs() < 2e-5 * numerical.abs().max(1.0),
                "vertex {vertex} axis {axis}: {numerical} vs {}",
                forces[vertex][axis]
            );
        }
    }
}
#[test]
fn free_fall_and_rigid_translation_need_no_artificial_drag() {
    let mut s = triangle(matrix_material(), &[]);
    let start = s.positions().to_vec();
    let v = vec![[0.1, 0.0, 0.0]; 3];
    s.set_state(start.clone(), v).unwrap();
    let dt = 1.0 / 240.0;
    for _ in 0..24 {
        s.step(
            dt,
            [0.0, -9.81, 0.0],
            &[[0.0; 3]; 3],
            &[],
            SolverConfig::default(),
        )
        .unwrap();
    }
    for (i, p) in s.positions().iter().enumerate() {
        assert!((p[0] - start[i][0] - 0.01).abs() < 1e-9);
        let analytic = -9.81 * dt * dt * 24.0 * 25.0 / 2.0;
        assert!((p[1] - start[i][1] - analytic).abs() < 1e-9);
    }
}
#[test]
fn maxwell_stress_relaxes_at_fixed_strain() {
    let mut m = matrix_material();
    m.layers[0].relaxation = vec![Relaxation {
        modulus: 100_000.0,
        time: 0.1,
    }];
    let mut s = triangle(m, &[0, 1, 2]);
    let p = s
        .positions()
        .iter()
        .map(|p| [p[0] * 1.2, p[1], p[2]])
        .collect();
    s.set_state(p, vec![[0.0; 3]; 3]).unwrap();
    let first = norm(s.internal_forces().unwrap()[1]);
    let dt = 1.0 / 240.0;
    for _ in 0..240 {
        s.step(dt, [0.0; 3], &[[0.0; 3]; 3], &[], SolverConfig::default())
            .unwrap();
    }
    let final_force = norm(s.internal_forces().unwrap()[1]);
    let mut reference = triangle(matrix_material(), &[0, 1, 2]);
    reference
        .set_state(s.positions().to_vec(), vec![[0.0; 3]; 3])
        .unwrap();
    let relaxed = norm(reference.internal_forces().unwrap()[1]);
    assert!(first > final_force * 1.2);
    assert!((final_force - relaxed).abs() < 1e-3);
}
#[test]
fn invalid_and_nonconverged_steps_are_atomic() {
    let mut s = triangle(matrix_material(), &[0, 2]);
    let old = s.positions().to_vec();
    let old_v = s.velocities().to_vec();
    let e = s.stored_energy().unwrap();
    assert!(
        s.step(
            f64::NAN,
            [0.0; 3],
            &[[0.0; 3]; 3],
            &[],
            SolverConfig::default()
        )
        .is_err()
    );
    let config = SolverConfig {
        max_newton: 1,
        ..SolverConfig::default()
    };
    assert!(
        s.step(
            1.0 / 60.0,
            [0.0, -9.81, 0.0],
            &[[0.0, 100.0, 10.0]; 3],
            &[],
            config
        )
        .is_err()
    );
    assert_eq!(s.positions(), old);
    assert_eq!(s.velocities(), old_v);
    assert_eq!(s.stored_energy().unwrap(), e);
}
#[test]
fn rejects_nonmanifold_winding_and_unused_vertices() {
    let m = matrix_material();
    assert!(
        Skin::new(
            vec![
                [0.0; 3],
                [0.01, 0.0, 0.0],
                [0.0, 0.01, 0.0],
                [0.01, 0.01, 0.0]
            ],
            vec![[0, 1, 2], [0, 1, 3]],
            &[],
            m.clone(),
            vec![[1.0, 0.0, 0.0]; 2]
        )
        .is_err()
    );
    assert!(
        Skin::new(
            vec![
                [0.0; 3],
                [0.01, 0.0, 0.0],
                [0.0, 0.01, 0.0],
                [0.01, 0.01, 0.0]
            ],
            vec![[0, 1, 2]],
            &[],
            m,
            vec![[1.0, 0.0, 0.0]]
        )
        .is_err()
    );
}

#[test]
fn sphere_contacts_face_interior_when_all_vertices_are_clear() {
    use physics::skin::{ContactScene, ContactSphere};
    let s = triangle(matrix_material(), &[]);
    let scene = ContactScene {
        spheres: vec![ContactSphere {
            center: [0.006, 0.006, -0.005],
            radius: 0.003,
            velocity: [0.0; 3],
        }],
        distance: 0.003,
        stiffness: 100_000.0,
        ..ContactScene::default()
    };
    let forces = s.contact_forces(&scene).unwrap();
    assert!(forces.iter().all(|f| f[2] > 0.0));
    assert!(forces.iter().map(|f| f[2]).sum::<f64>() > 1e-4);
    assert!(forces.iter().map(|f| f[0]).sum::<f64>().abs() < 1e-10);
}
#[test]
fn plane_contact_stops_load_and_remains_nonpenetrating() {
    use physics::skin::{ContactPlane, ContactScene};
    let mut s = triangle(matrix_material(), &[]);
    let p = s.positions().iter().map(|p| [p[0], p[1], 0.002]).collect();
    s.set_state(p, vec![[0.0; 3]; 3]).unwrap();
    let scene = ContactScene {
        planes: vec![ContactPlane {
            normal: [0.0, 0.0, 1.0],
            offset: 0.0,
            velocity: [0.0; 3],
        }],
        distance: 0.003,
        stiffness: 100_000.0,
        ..ContactScene::default()
    };
    for _ in 0..120 {
        s.step_with_contacts(
            1.0 / 240.0,
            [0.0, 0.0, -9.81],
            &[[0.0; 3]; 3],
            &[],
            &scene,
            SolverConfig::default(),
        )
        .unwrap();
    }
    assert!(s.positions().iter().all(|p| p[2] > 0.0005));
    let force = s
        .contact_forces(&scene)
        .unwrap()
        .iter()
        .map(|f| f[2])
        .sum::<f64>();
    assert!((force - s.masses().iter().sum::<f64>() * 9.81).abs() < 0.001);
}
#[test]
fn swept_sphere_cannot_cross_a_face_with_clear_endpoints() {
    use physics::skin::{ContactScene, ContactSphere};
    let mut s = triangle(matrix_material(), &[0, 1, 2]);
    let old = s.positions().to_vec();
    let scene = ContactScene {
        spheres: vec![ContactSphere {
            center: [0.006, 0.006, -0.01],
            radius: 0.002,
            velocity: [0.0, 0.0, 4.8],
        }],
        ..ContactScene::default()
    };
    assert!(
        s.step_with_contacts(
            1.0 / 240.0,
            [0.0; 3],
            &[[0.0; 3]; 3],
            &[],
            &scene,
            SolverConfig::default()
        )
        .is_err()
    );
    assert_eq!(s.positions(), old);
}

#[test]
fn extreme_valid_relaxation_times_keep_committed_memory_finite() {
    for time in [1e-320, 1e300] {
        let mut material = matrix_material();
        material.layers[0].relaxation = vec![Relaxation {
            modulus: 100_000.,
            time,
        }];
        let mut skin = triangle(material, &[0, 1, 2]);
        let deformed = skin
            .rest_positions()
            .iter()
            .map(|p| [1.1 * p[0], p[1], p[2]])
            .collect();
        skin.set_state(deformed, vec![[0.; 3]; 3]).unwrap();
        skin.step(
            1. / 120.,
            [0.; 3],
            &[[0.; 3]; 3],
            &[],
            SolverConfig::default(),
        )
        .unwrap();
        assert!(skin.stored_energy().unwrap().is_finite());
        assert!(
            skin.internal_forces()
                .unwrap()
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
    }
}

#[test]
fn rejects_overflowing_laminate_totals_before_returning_a_response() {
    let mut material = matrix_material();
    material.layers[0].thickness = 1e308;
    material.layers[0].shear_modulus = 1e-308;
    material.layers[0].density = 1e-308;
    material.layers.push(material.layers[0].clone());
    assert!(material.response([1., 1., 0.]).is_err());
}

#[test]
fn attachments_follow_start_target_plus_velocity_without_artificial_drag() {
    use physics::skin::Attachment;
    let mut skin = triangle(matrix_material(), &[]);
    let start = skin.positions().to_vec();
    let velocity = [0.1, -0.2, 0.3];
    skin.set_state(start.clone(), vec![velocity; 3]).unwrap();
    let attachments: Vec<_> = start
        .iter()
        .enumerate()
        .map(|(vertex, &target)| Attachment {
            vertex,
            target,
            velocity,
            stiffness: 100.,
            viscosity: 10.,
        })
        .collect();
    let dt = 1. / 120.;
    skin.step(
        dt,
        [0.; 3],
        &[[0.; 3]; 3],
        &attachments,
        SolverConfig::default(),
    )
    .unwrap();
    for (point, rest) in skin.positions().iter().zip(&start) {
        for k in 0..3 {
            assert!((point[k] - rest[k] - dt * velocity[k]).abs() < 1e-10);
        }
    }
}

#[test]
fn rejects_disconnected_vertex_fans_even_when_all_edges_are_manifold() {
    assert!(
        Skin::new(
            vec![
                [0., 0., 0.],
                [0.01, 0., 0.],
                [0., 0.01, 0.],
                [-0.01, 0., 0.],
                [0., -0.01, 0.]
            ],
            vec![[0, 1, 2], [0, 3, 4]],
            &[],
            matrix_material(),
            vec![[1., 0., 0.]; 2],
        )
        .is_err()
    );
}

#[test]
fn principal_stretches_and_thickness_match_affine_deformation() {
    let mut skin = patch(3, 3, 0.01, matrix_material()).unwrap();
    let rest_thickness = skin.material().thickness();
    for rotation in [0., 0.7, 1.5] {
        let (s, c) = f64::sin_cos(rotation);
        let positions: Vec<_> = skin
            .rest_positions()
            .iter()
            .map(|p| {
                let x = 1.2 * p[0];
                let y = 0.8 * p[1];
                [c * x - s * y, s * x + c * y, 0.]
            })
            .collect();
        let velocities = vec![[0.; 3]; positions.len()];
        skin.set_state(positions, velocities).unwrap();
        for metric in skin.surface_metrics().unwrap() {
            assert!((metric.principal_stretches[0] - 0.8).abs() < 1e-12);
            assert!((metric.principal_stretches[1] - 1.2).abs() < 1e-12);
            assert!((metric.area_ratio - 0.96).abs() < 1e-12);
            assert!((metric.thickness * metric.area_ratio - rest_thickness).abs() < 1e-14);
        }
    }
}
