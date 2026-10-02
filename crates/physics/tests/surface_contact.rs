use physics::biomechanics::*;
fn specimen() -> Body {
    let material = Material::from_young_poisson(8000., 0.3).unwrap();
    Body::new(
        vec![
            [0., 0., 0.],
            [0.01, 0., 0.],
            [0., 0.01, 0.],
            [0., 0., -0.005],
            [0.002, 0.002, 0.001],
            [0.004, 0.002, 0.003],
            [0.002, 0.004, 0.003],
            [0.002, 0.002, 0.006],
        ],
        vec![false; 8],
        vec![([0, 1, 2, 3], material.clone()), ([4, 5, 6, 7], material)],
    )
    .unwrap()
}
fn contact() -> TissueSurfaceContact {
    TissueSurfaceContact {
        faces: vec![[0, 1, 2], [4, 5, 6]],
        minimum_distance_m: 0.0001,
        activation_gap_m: 0.002,
        pair_stiffness_n_m: 10.,
    }
}
#[test]
fn unique_primitive_energy_matches_gradient() {
    let mut b = specimen();
    b.set_surface_contacts(vec![contact()]).unwrap();
    let x = b.positions().to_vec();
    let (e, g) = b.surface_primitive_energy_at(&x).unwrap();
    assert!(e > 0.);
    for node in 0..x.len() {
        for axis in 0..3 {
            let mut p = x.clone();
            let mut m = x.clone();
            p[node][axis] += 1e-8;
            m[node][axis] -= 1e-8;
            let fd = (b.surface_primitive_energy_at(&p).unwrap().0
                - b.surface_primitive_energy_at(&m).unwrap().0)
                / 2e-8;
            assert!(
                (fd - g[node][axis]).abs() < 1e-6,
                "node={node} axis={axis} fd={fd} g={}",
                g[node][axis]
            );
        }
    }
    for axis in 0..3 {
        assert!(g.iter().map(|g| g[axis]).sum::<f64>().abs() < 1e-12);
    }
}
#[test]
fn primitive_candidates_are_unique_and_orientation_independent() {
    let mut b = specimen();
    let mut c = contact();
    c.faces.push([0, 1, 3]);
    b.set_surface_contacts(vec![c.clone()]).unwrap();
    let stencils = b.surface_primitive_stencils_at(b.positions()).unwrap();
    assert_eq!(
        stencils
            .iter()
            .filter(|s| matches!(s, SurfacePrimitive::VertexFace { .. }))
            .count(),
        10
    );
    assert_eq!(
        stencils
            .iter()
            .filter(|s| matches!(s, SurfacePrimitive::EdgeEdge { .. }))
            .count(),
        15
    );
    c.faces.reverse();
    for f in &mut c.faces {
        f.swap(0, 2);
    }
    b.set_surface_contacts(vec![c]).unwrap();
    assert_eq!(
        b.surface_primitive_stencils_at(b.positions()).unwrap(),
        stencils
    );
    assert!(b.surface_primitive_stencils_at(&[]).is_err());
}
#[test]
fn diagnostic_primitive_sum_has_consistent_gradient_and_balanced_force() {
    let b = specimen();
    let x = b.positions().to_vec();
    let a = [0, 1, 2];
    let face = [4, 5, 6];
    let (energy, g) = b
        .surface_pair_primitive_barrier_at(&x, a, face, 0.0001, 0.002, 10.)
        .unwrap();
    assert!(energy > 0.);
    for (local, node) in a.into_iter().chain(face).enumerate() {
        for axis in 0..3 {
            let mut plus = x.clone();
            let mut minus = x.clone();
            plus[node][axis] += 1e-8;
            minus[node][axis] -= 1e-8;
            let ep = b
                .surface_pair_primitive_barrier_at(&plus, a, face, 0.0001, 0.002, 10.)
                .unwrap()
                .0;
            let em = b
                .surface_pair_primitive_barrier_at(&minus, a, face, 0.0001, 0.002, 10.)
                .unwrap()
                .0;
            assert!(((ep - em) / 2e-8 - g[local][axis]).abs() < 1e-6);
        }
    }
    for axis in 0..3 {
        assert!(g.iter().map(|g| g[axis]).sum::<f64>().abs() < 1e-12);
    }
}
#[test]
fn separating_tissue_groups_removes_only_the_cross_tissue_barrier() {
    let mut combined = specimen();
    combined.set_surface_contacts(vec![contact()]).unwrap();
    let mut separate = combined.clone();
    let groups = contact()
        .faces
        .into_iter()
        .map(|face| TissueSurfaceContact {
            faces: vec![face],
            ..contact()
        })
        .collect();
    separate.set_surface_contacts(groups).unwrap();
    let plain = specimen();
    let (combined_energy, combined_gradient) = combined.evaluate(combined.positions()).unwrap();
    let (separate_energy, separate_gradient) = separate.evaluate(separate.positions()).unwrap();
    let (plain_energy, plain_gradient) = plain.evaluate(plain.positions()).unwrap();
    assert_eq!(separate_energy, plain_energy);
    assert_eq!(separate_gradient, plain_gradient);
    let gap: f64 = 0.001 - 0.0001;
    let expected = -10. * (gap - 0.002).powi(2) * (gap / 0.002).ln();
    assert!((combined_energy - separate_energy - expected).abs() < 1e-15);
    let pairs = combined
        .active_surface_pairs_at(combined.positions())
        .unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0, 0);
    assert!((pairs[0].3 - 0.001).abs() < 1e-14);
    assert!((pairs[0].4 - expected).abs() < 1e-15);
    let (a, b, d) = combined
        .surface_pair_closest_at(combined.positions(), pairs[0].1, pairs[0].2)
        .unwrap();
    assert!((a.iter().sum::<f64>() - 1.).abs() < 1e-14);
    assert!((b.iter().sum::<f64>() - 1.).abs() < 1e-14);
    assert!((d - pairs[0].3).abs() < 1e-14);
    assert!(
        combined
            .surface_pair_closest_at(combined.positions(), [999, 1, 2], [4, 5, 6])
            .is_err()
    );
    assert!(
        separate
            .active_surface_pairs_at(separate.positions())
            .unwrap()
            .is_empty()
    );
    assert!(combined.active_surface_pairs_at(&[]).is_err());
    assert!(
        combined_gradient
            .iter()
            .zip(separate_gradient)
            .any(|(a, b)| (0..3).any(|k| (a[k] - b[k]).abs() > 1e-6))
    );
}
#[test]
fn vertex_face_energy_gradient_balances_force_and_torque_and_is_objective() {
    let plain = specimen();
    let mut b = plain.clone();
    b.set_surface_contacts(vec![contact()]).unwrap();
    assert!((b.minimum_surface_contact_distance().unwrap().unwrap() - 0.001).abs() < 1e-14);
    let x = b.positions().to_vec();
    let (e, g) = b.evaluate(&x).unwrap();
    let (e0, g0) = plain.evaluate(&x).unwrap();
    let gap: f64 = 0.001 - 0.0001;
    let offset = gap - 0.002;
    assert!((e - e0 + 10. * offset * offset * (gap / 0.002).ln()).abs() < 1e-15);
    for i in 0..8 {
        for k in 0..3 {
            let mut xp = x.clone();
            let mut xm = x.clone();
            xp[i][k] += 1e-8;
            xm[i][k] -= 1e-8;
            let fd = (b.evaluate(&xp).unwrap().0 - b.evaluate(&xm).unwrap().0) / 2e-8;
            assert!(
                (fd - g[i][k]).abs() < 1e-6,
                "node={i},axis={k},fd={fd},gradient={}",
                g[i][k]
            );
        }
    }
    let mut force = [0.; 3];
    let mut torque = [0.; 3];
    for i in 0..8 {
        let f: Vec3 = std::array::from_fn(|k| g[i][k] - g0[i][k]);
        for k in 0..3 {
            force[k] += f[k];
        }
        let p = x[i];
        torque[0] += p[1] * f[2] - p[2] * f[1];
        torque[1] += p[2] * f[0] - p[0] * f[2];
        torque[2] += p[0] * f[1] - p[1] * f[0];
    }
    assert!(force.into_iter().chain(torque).all(|v| v.abs() < 1e-12));
    let transformed: Vec<_> = x
        .iter()
        .map(|p| [-p[1] + 0.1, p[0] - 0.2, p[2] + 0.05])
        .collect();
    assert!(
        (b.evaluate(&transformed).unwrap().0 - plain.evaluate(&transformed).unwrap().0 - (e - e0))
            .abs()
            < 1e-14
    );
}
#[test]
fn crossing_triangle_is_rejected_and_invalid_configuration_is_atomic() {
    let mut b = specimen();
    b.set_surface_contacts(vec![contact()]).unwrap();
    let mut x = b.positions().to_vec();
    for p in &mut x[4..] {
        p[2] -= 0.002;
    }
    assert_eq!(b.evaluate(&x).unwrap_err(), "closed surface contact gap");
    let initial = b.evaluate(b.positions()).unwrap().0;
    let mut invalid = contact();
    invalid.faces.push([2, 1, 0]);
    assert!(b.set_surface_contacts(vec![invalid]).is_err());
    assert_eq!(b.evaluate(b.positions()).unwrap().0, initial);
    let assembly = Body::assemble_tissues(&[b.clone(), b]).unwrap().body;
    assert_eq!(assembly.surface_contacts().len(), 2);
    assert_eq!(assembly.surface_contacts()[1].faces[0], [8, 9, 10]);
    assert!((assembly.evaluate(assembly.positions()).unwrap().0 - 2. * initial).abs() < 1e-14);
}
#[test]
fn continuous_triangle_guard_rejects_tunnelling_with_disjoint_endpoints() {
    for law in [
        SurfaceContactLaw::TriangleMinimum,
        SurfaceContactLaw::ExperimentalPrimitiveSum,
    ] {
        let mut b = specimen();
        let mut c = contact();
        c.activation_gap_m = 0.0002;
        b.set_surface_contacts(vec![c]).unwrap();
        b.set_surface_contact_law(law).unwrap();
        let velocities = (0..8)
            .map(|i| [0., 0., if i < 4 { 0. } else { -0.006 }])
            .collect();
        let mut dynamic = InertialBody::new(b, &[1000.; 2], velocities).unwrap();
        let positions = dynamic.body().positions().to_vec();
        let velocity = dynamic.velocities().to_vec();
        assert_eq!(
            dynamic.step(1., 1.).unwrap_err(),
            "inertial tissue gap path crossing"
        );
        assert_eq!(dynamic.body().positions(), positions);
        assert_eq!(dynamic.velocities(), velocity);
    }
}

#[test]
fn edge_edge_closest_feature_distributes_balanced_contact_gradient() {
    let material = Material::from_young_poisson(8000., 0.3).unwrap();
    let plain = Body::new(
        vec![
            [-0.005, 0., 0.],
            [0.005, 0., 0.],
            [0., -0.005, 0.],
            [0., 0., -0.005],
            [0., 0.001, -0.005],
            [0., 0.001, 0.005],
            [0., 0.003, 0.],
            [0.005, 0.001, 0.],
        ],
        vec![false; 8],
        vec![([0, 1, 2, 3], material.clone()), ([4, 5, 6, 7], material)],
    )
    .unwrap();
    let mut b = plain.clone();
    b.set_surface_contacts(vec![contact()]).unwrap();
    let x = b.positions().to_vec();
    let g = b.evaluate(&x).unwrap().1;
    let g0 = plain.evaluate(&x).unwrap().1;
    assert!((g[0][1] - g0[0][1] - (g[1][1] - g0[1][1])).abs() < 1e-12);
    assert!((g[4][1] - g0[4][1] - (g[5][1] - g0[5][1])).abs() < 1e-12);
    for i in [2, 3, 6, 7] {
        for k in 0..3 {
            assert!((g[i][k] - g0[i][k]).abs() < 1e-12);
        }
    }
    for i in 0..8 {
        for k in 0..3 {
            let mut xp = x.clone();
            let mut xm = x.clone();
            xp[i][k] += 1e-8;
            xm[i][k] -= 1e-8;
            let fd = (b.evaluate(&xp).unwrap().0 - b.evaluate(&xm).unwrap().0) / 2e-8;
            assert!((fd - g[i][k]).abs() < 1e-6);
        }
    }
}

#[test]
fn experimental_solver_law_preserves_other_forces_and_validates_gradients() {
    let plain = specimen();
    let mut b = plain.clone();
    b.set_surface_contacts(vec![contact()]).unwrap();
    assert_eq!(b.surface_contact_law(), SurfaceContactLaw::TriangleMinimum);
    b.set_surface_contact_law(SurfaceContactLaw::ExperimentalPrimitiveSum)
        .unwrap();
    let mut x = b.positions().to_vec();
    x[7][2] += 0.00002;
    let (ep, gp) = plain.evaluate(&x).unwrap();
    let (ec, gc) = b.surface_primitive_energy_at(&x).unwrap();
    let (e, g) = b.evaluate(&x).unwrap();
    assert!((e - ep - ec).abs() < 1e-14);
    for i in 0..x.len() {
        for axis in 0..3 {
            assert!((g[i][axis] - gp[i][axis] - gc[i][axis]).abs() < 1e-12);
            let mut plus = x.clone();
            let mut minus = x.clone();
            plus[i][axis] += 1e-8;
            minus[i][axis] -= 1e-8;
            let fd = (b.evaluate(&plus).unwrap().0 - b.evaluate(&minus).unwrap().0) / 2e-8;
            assert!((fd - g[i][axis]).abs() < 1e-6);
        }
    }
    let assembled = Body::assemble_tissues(&[b.clone()]).unwrap();
    assert_eq!(
        assembled.body.surface_contact_law(),
        b.surface_contact_law()
    );
    assert_eq!(
        assembled.body.evaluate(&x).unwrap(),
        b.evaluate(&x).unwrap()
    );
    assert!(Body::assemble_tissues(&[b.clone(), plain]).is_err());
    let mut bad = x.clone();
    for i in 4..8 {
        bad[i][2] -= 0.001;
    }
    assert!(b.evaluate(&bad).is_err());
    b.set_surface_contact_law(SurfaceContactLaw::TriangleMinimum)
        .unwrap();
    assert_eq!(b.surface_contact_law(), SurfaceContactLaw::TriangleMinimum);
}

#[test]
fn normal_contact_preconditioning_is_optional_and_energy_preserving() {
    let mut b = specimen();
    b.set_surface_contacts(vec![contact()]).unwrap();
    assert!(
        b.equilibrate_lbfgs_preconditioned_states(2000, 1e-7, true, |_, _, _, _, _, _, _| {})
            .is_err()
    );
    b.set_surface_contact_law(SurfaceContactLaw::ExperimentalPrimitiveSum)
        .unwrap();
    let original = b.evaluate(b.positions()).unwrap();
    let c = b.surface_primitive_curvature_at(b.positions()).unwrap();
    assert!(c.iter().all(|v| v.is_finite() && *v >= 0.));
    assert!(c.iter().any(|v| *v > 0.));
    assert_eq!(b.evaluate(b.positions()).unwrap(), original);
    let mut plain = b.clone();
    let mut preconditioned = b;
    let a = plain.equilibrate_lbfgs(2000, 1e-7).unwrap();
    let r = preconditioned
        .equilibrate_lbfgs_preconditioned_states(2000, 1e-7, true, |_, _, _, _, _, _, _| {})
        .unwrap();
    assert!(
        a.converged && r.converged,
        "plain={} curvature={}",
        a.residual_n,
        r.residual_n
    );
    let ea = plain.evaluate(plain.positions()).unwrap().0;
    let er = preconditioned
        .evaluate(preconditioned.positions())
        .unwrap()
        .0;
    assert!((ea - er).abs() < 1e-10);
}
