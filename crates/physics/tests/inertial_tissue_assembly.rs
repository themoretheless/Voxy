use physics::biomechanics::*;

fn region(offset: f64, steps: usize, thermal: bool) -> InertialBody {
    let rest = vec![
        [offset, 0., 0.],
        [offset + 0.02, 0., 0.],
        [offset, 0.02, 0.],
        [offset, 0., 0.02],
    ];
    let mut solid = Body::new(
        rest.clone(),
        vec![true, false, true, true],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 500.,
                bulk_pa: 5000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    solid
        .set_viscoelastic_ogden(
            0,
            ViscoelasticOgden::new(
                vec![OgdenTerm {
                    shear_pa: 500.,
                    exponent: 2.,
                }],
                5000.,
                vec![MaxwellBranch {
                    shear_pa: 1000.,
                    relaxation_seconds: 0.1,
                }],
            )
            .unwrap(),
        )
        .unwrap();
    let mut strained = rest;
    strained[1][0] += 0.001;
    solid.restore_diagnostic_positions(&strained).unwrap();
    let mut state = InertialBody::new_viscoelastic_with_supports(
        solid,
        &[1000. + offset * 1000.],
        vec![[0.; 3]; 4],
    )
    .unwrap();
    state.set_uniform_acceleration([0., -9.81, 0.]).unwrap();
    if thermal {
        state
            .enable_maxwell_thermal(&[2000. + offset * 1000.], &[300. + offset])
            .unwrap();
    }
    for _ in 0..steps {
        let targets: Vec<_> = [0, 2, 3]
            .into_iter()
            .map(|node| {
                let mut p = state.body().positions()[node];
                p[2] += 1e-7;
                SupportTarget {
                    node,
                    position_m: p,
                }
            })
            .collect();
        state.step_viscoelastic(Some(&targets), 1e-4, 1e-8).unwrap();
    }
    state
}
fn probe() -> Matrix {
    [[1.03, 0.01, 0.], [0., 0.99, 0.], [0., 0., 1.]]
}

#[test]
fn inertial_assembly_preserves_histories_masses_moving_supports_and_heat() {
    let mut parts = [region(0., 2, true), region(0.04, 5, true)];
    let original = format!("{parts:?}");
    let mut assembly = InertialBody::assemble_tissues(&parts).unwrap();
    assert_eq!(original, format!("{parts:?}"));
    assert_eq!(assembly.node_ranges, vec![0..4, 4..8]);
    assert_eq!(assembly.cell_ranges, vec![0..1, 1..2]);
    let heat = assembly.body.maxwell_sensible_energy_j().unwrap();
    let temp = assembly.body.maxwell_temperatures_kelvin().unwrap();
    for (i, part) in parts.iter().enumerate() {
        let range = assembly.node_ranges[i].clone();
        assert_eq!(
            &assembly.body.body().positions()[range.clone()],
            part.body().positions()
        );
        assert_eq!(
            &assembly.body.body().rest_positions()[range.clone()],
            part.body().rest_positions()
        );
        assert_eq!(
            &assembly.body.velocities()[range.clone()],
            part.velocities()
        );
        assert_eq!(&assembly.body.masses()[range], part.masses());
        assert_eq!(heat[i], part.maxwell_sensible_energy_j().unwrap()[0]);
        assert_eq!(temp[i], part.maxwell_temperatures_kelvin().unwrap()[0]);
        assert!(heat[i] > 0.);
        let old = part.body().elements()[0].response(probe()).unwrap();
        let new = assembly.body.body().elements()[i]
            .response(probe())
            .unwrap();
        assert_eq!(old.energy_density, new.energy_density);
        assert_eq!(old.first_piola, new.first_piola);
    }
    assert_ne!(
        parts[0].body().elements()[0]
            .response(probe())
            .unwrap()
            .first_piola,
        parts[1].body().elements()[0]
            .response(probe())
            .unwrap()
            .first_piola
    );
    let diag = assembly.body.diagnostics().unwrap();
    let source: Vec<_> = parts.iter().map(|p| p.diagnostics().unwrap()).collect();
    assert!((diag.kinetic_j - source.iter().map(|d| d.kinetic_j).sum::<f64>()).abs() < 1e-16);
    assert!((diag.potential_j - source.iter().map(|d| d.potential_j).sum::<f64>()).abs() < 1e-14);
    let mut global = Vec::new();
    for (i, part) in parts.iter_mut().enumerate() {
        let targets: Vec<_> = [0, 2, 3]
            .into_iter()
            .map(|node| {
                let mut p = part.body().positions()[node];
                p[2] += 1e-7;
                SupportTarget {
                    node,
                    position_m: p,
                }
            })
            .collect();
        global.extend(targets.iter().map(|t| SupportTarget {
            node: t.node + assembly.node_ranges[i].start,
            position_m: t.position_m,
        }));
        part.step_viscoelastic(Some(&targets), 1e-4, 1e-8).unwrap();
    }
    assembly
        .body
        .step_viscoelastic(Some(&global), 1e-4, 2e-8)
        .unwrap();
    for (i, part) in parts.iter().enumerate() {
        for (a, b) in assembly.body.body().positions()[assembly.node_ranges[i].clone()]
            .iter()
            .zip(part.body().positions())
        {
            for axis in 0..3 {
                assert!((a[axis] - b[axis]).abs() < 1e-13);
            }
        }
        let ha = assembly.body.maxwell_sensible_energy_j().unwrap()[i];
        let hb = part.maxwell_sensible_energy_j().unwrap()[0];
        assert!((ha - hb).abs() < 1e-14);
    }
}

#[test]
fn incompatible_inertial_assembly_preserves_all_source_owners() {
    assert!(InertialBody::assemble_tissues(&[]).is_err());
    let original = [region(0., 2, true), region(0.04, 3, true)];
    let mut cases = Vec::new();
    let mut a = original.clone();
    a[1].set_uniform_acceleration([0., 0., 0.]).unwrap();
    cases.push(a);
    let a = [original[0].clone(), region(0.04, 3, false)];
    cases.push(a);
    let mut a = original.clone();
    a[1].set_plane_contact(Some(PlaneContact::new([0., 1., 0.], -1., 100.).unwrap()))
        .unwrap();
    cases.push(a);
    for parts in cases {
        let saved = format!("{parts:?}");
        assert!(InertialBody::assemble_tissues(&parts).is_err());
        assert_eq!(saved, format!("{parts:?}"));
    }
}

#[test]
fn common_native_surface_is_retained_but_distinct_poses_are_rejected() {
    use std::sync::Arc;
    let surface = Arc::new(
        PrescribedTriangleSurface::new(
            vec![[-1., -1., -0.001], [1., -1., -0.001], [0., 1., -0.001]],
            vec![[0, 1, 2]],
            0.0001,
            0.003,
            100.,
        )
        .unwrap(),
    );
    let mut parts = [region(0., 2, true), region(0.04, 3, true)];
    for p in &mut parts {
        p.set_prescribed_surface(Some(surface.clone())).unwrap();
    }
    let assembly = InertialBody::assemble_tissues(&parts).unwrap();
    assert!(std::ptr::eq(
        assembly.body.prescribed_surface().unwrap(),
        surface.as_ref()
    ));
    let contact = assembly.body.diagnostics().unwrap().contact_j;
    let sum: f64 = parts
        .iter()
        .map(|p| p.diagnostics().unwrap().contact_j)
        .sum();
    assert!(sum > 0.);
    assert!((contact - sum).abs() < 1e-12);
    let moved = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] - 0.0001])
                    .collect(),
            )
            .unwrap(),
    );
    parts[1].set_prescribed_surface(Some(moved)).unwrap();
    let saved = format!("{parts:?}");
    assert!(InertialBody::assemble_tissues(&parts).is_err());
    assert_eq!(saved, format!("{parts:?}"));
}

#[test]
fn assembly_does_not_silently_drop_installed_embedded_contact() {
    use std::sync::Arc;
    let mut state = region(0., 2, true);
    let skin = [
        [0.001, 0.001, 0.001],
        [0.002, 0.001, 0.001],
        [0.001, 0.002, 0.001],
    ];
    let contact = Arc::new(
        EmbeddedTriangleContact::new(
            state.body().rest_positions(),
            &[[0, 1, 2, 3]],
            &skin,
            vec![[0, 1, 2]],
        )
        .unwrap(),
    );
    let obstacle = Arc::new(
        PrescribedTriangleSurface::new(
            vec![[-1., -1., -0.001], [1., -1., -0.001], [0., 1., -0.001]],
            vec![[0, 1, 2]],
            0.0001,
            0.003,
            100.,
        )
        .unwrap(),
    );
    state
        .set_stationary_embedded_contact(Some(
            StationaryEmbeddedContact::new(
                contact,
                state.body().rest_positions().to_vec(),
                skin.to_vec(),
                obstacle,
            )
            .unwrap(),
        ))
        .unwrap();
    let saved = format!("{state:?}");
    let error = InertialBody::assemble_tissues(&[state.clone()]).unwrap_err();
    assert_eq!(
        error,
        "tissue assembly requires rebinding embedded skin contact"
    );
    assert!(Body::assemble_tissues(&[state.body().clone()]).is_err());
    assert_eq!(saved, format!("{state:?}"));
}

#[test]
fn inertial_assembly_remaps_distinct_regional_contact_masks_without_face_loss() {
    use std::sync::Arc;
    let source = PrescribedTriangleSurface::new(
        vec![
            [-1., -1., -0.001],
            [1., -1., -0.001],
            [0., 1., -0.001],
            [-1., -1., -0.0014],
            [1., -1., -0.0014],
            [0., 1., -0.0014],
        ],
        vec![[0, 1, 2], [3, 4, 5]],
        0.0001,
        0.003,
        100.,
    )
    .unwrap();
    let mut parts = [region(0., 2, true), region(0.04, 3, true)];
    let local_face = parts[0].body().surface()[0];
    let first = source
        .with_contact_faces(vec![true, false])
        .unwrap()
        .with_body_contact_domains(vec![(vec![local_face], vec![false, false])])
        .unwrap();
    let second = source.with_contact_faces(vec![false, true]).unwrap();
    parts[0]
        .set_prescribed_surface(Some(Arc::new(first)))
        .unwrap();
    parts[1]
        .set_prescribed_surface(Some(Arc::new(second)))
        .unwrap();
    let saved = format!("{parts:?}");
    let assembly = InertialBody::assemble_tissues(&parts).unwrap();
    assert_eq!(saved, format!("{parts:?}"));
    let global = assembly.body.prescribed_surface().unwrap();
    assert!(std::ptr::eq(global.faces(), source.faces()));
    assert_eq!(global.positions(), source.positions());
    assert_eq!(
        global.body_contact_faces(local_face),
        Some([false, false].as_slice())
    );
    assert_eq!(
        global.body_contact_faces(local_face.map(|i| i + 4)),
        Some([false, true].as_slice())
    );
    let all = global
        .response(
            assembly.body.body().positions(),
            &assembly.body.body().surface(),
        )
        .unwrap();
    let mut sum = 0.;
    let mut obstacle_gradient = vec![[0.; 3]; source.positions().len()];
    for (i, part) in parts.iter().enumerate() {
        let local = part
            .prescribed_surface()
            .unwrap()
            .response(part.body().positions(), &part.body().surface())
            .unwrap();
        sum += local.potential_j;
        assert_eq!(
            &all.body_gradient_n[assembly.node_ranges[i].clone()],
            local.body_gradient_n
        );
        for (a, b) in obstacle_gradient.iter_mut().zip(local.obstacle_gradient_n) {
            for axis in 0..3 {
                a[axis] += b[axis];
            }
        }
        for face in part.body().surface() {
            let expected = if i == 0 {
                if face == local_face {
                    vec![false, false]
                } else {
                    vec![true, false]
                }
            } else {
                vec![false, true]
            };
            assert_eq!(
                global
                    .body_contact_faces(face.map(|n| n + assembly.node_ranges[i].start))
                    .unwrap(),
                expected
            );
        }
    }
    assert!((all.potential_j - sum).abs() < 1e-12);
    assert_eq!(all.obstacle_gradient_n, obstacle_gradient);
    let global_energy = assembly.body.diagnostics().unwrap().potential_j;
    let source_energy: f64 = parts
        .iter()
        .map(|p| p.diagnostics().unwrap().potential_j)
        .sum();
    assert!((global_energy - source_energy).abs() < 1e-12);
    let next = global
        .with_positions(
            global
                .positions()
                .iter()
                .map(|p| [p[0], p[1], p[2] - 0.0001])
                .collect(),
        )
        .unwrap();
    let moving: Vec<_> = assembly
        .body
        .body()
        .positions()
        .iter()
        .map(|p| [p[0], p[1], p[2] + 0.0002])
        .collect();
    let path = global
        .path_response(
            &next,
            assembly.body.body().positions(),
            &moving,
            &assembly.body.body().surface(),
        )
        .unwrap();
    let mut separate_work = 0.;
    for (i, part) in parts.iter().enumerate() {
        let native = part.prescribed_surface().unwrap();
        let next = native.with_positions(next.positions().to_vec()).unwrap();
        let end = &moving[assembly.node_ranges[i].clone()];
        let local = native
            .path_response(&next, part.body().positions(), end, &part.body().surface())
            .unwrap();
        assert_eq!(
            &path.body_gradient_n[assembly.node_ranges[i].clone()],
            local.body_gradient_n
        );
        separate_work += local
            .body_gradient_n
            .iter()
            .flatten()
            .skip(2)
            .step_by(3)
            .map(|g| g * 0.0002)
            .sum::<f64>()
            + local
                .obstacle_gradient_n
                .iter()
                .flatten()
                .skip(2)
                .step_by(3)
                .map(|g| g * (-0.0001))
                .sum::<f64>();
    }
    let global_work: f64 = path
        .body_gradient_n
        .iter()
        .flatten()
        .skip(2)
        .step_by(3)
        .map(|g| g * 0.0002)
        .sum::<f64>()
        + path
            .obstacle_gradient_n
            .iter()
            .flatten()
            .skip(2)
            .step_by(3)
            .map(|g| g * (-0.0001))
            .sum::<f64>();
    assert!((global_work - separate_work).abs() < 1e-12);
}
