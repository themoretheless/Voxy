use physics::biomechanics::*;
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn mean(points: &[[f64; 3]], nodes: &[usize]) -> [f64; 3] {
    std::array::from_fn(|i| nodes.iter().map(|j| points[*j][i]).sum::<f64>() / nodes.len() as f64)
}
fn patch() -> (Vec<[f64; 3]>, Vec<[usize; 4]>) {
    (
        vec![
            [0., 0., 0.],
            [0.01, 0., 0.],
            [0., 0.01, 0.],
            [0.006, 0.002, 0.012],
            [0.001, 0.005, -0.007],
        ],
        vec![[0, 1, 2, 3], [0, 1, 2, 4]],
    )
}
fn tensor() -> Matrix {
    [
        [2e-12, 0.4e-12, 0.2e-12],
        [0.4e-12, 1e-12, -0.1e-12],
        [0.2e-12, -0.1e-12, 3e-12],
    ]
}
#[test]
fn oblique_anisotropic_linear_pressure_patch_has_exact_constant_flux() {
    let (x, cells) = patch();
    let k = tensor();
    let grad = [1000., -600., 800.];
    let pressure = |p: [f64; 3]| 500. + dot(grad, p);
    let mut boundaries = vec![];
    for cell in &cells {
        for i in 0..4 {
            let face = std::array::from_fn::<_, 3, _>(|j| cell[if j < i { j } else { j + 1 }]);
            if face.iter().all(|i| *i < 3) {
                continue;
            }
            boundaries.push((face, pressure(mean(&x, &face))));
        }
    }
    let p: Vec<_> = cells.iter().map(|cell| pressure(mean(&x, cell))).collect();
    let model = MixedDarcy::new(x.clone(), cells.clone(), &[k; 2], 0.001, &boundaries).unwrap();
    let r = model.response(&p).unwrap();
    let expected =
        std::array::from_fn::<_, 3, _>(|i| -(0..3).map(|j| k[i][j] * grad[j]).sum::<f64>() / 0.001);
    for velocity in &r.cell_centroid_velocities_m_per_s {
        for i in 0..3 {
            assert!((velocity[i] - expected[i]).abs() < 1e-18);
        }
    }
    for (face, q) in model.faces().iter().zip(&r.face_flows_m3_per_s) {
        let [a, b, c] = face.nodes.map(|i| x[i]);
        let mut n = cross(sub(b, a), sub(c, a));
        let opposite = cells[face.owner]
            .iter()
            .find(|i| !face.nodes.contains(i))
            .unwrap();
        if dot(n, sub(x[*opposite], a)) > 0. {
            n = n.map(|v| -v);
        }
        assert!((*q - 0.5 * dot(expected, n)).abs() < 1e-23);
    }
    assert!(r.cell_outflows_m3_per_s.iter().all(|v| v.abs() < 1e-23));
    assert!(r.dissipation_w > 0. && r.residual_pa < 1e-12);
    assert!((r.dissipation_w - r.pressure_work_w).abs() < 1e-12 * r.dissipation_w);
}
#[test]
fn sealed_faces_conserve_volume_and_pressure_work_matches_dissipation() {
    let (x, cells) = patch();
    let model = MixedDarcy::new(x, cells, &[tensor(); 2], 0.001, &[]).unwrap();
    assert_eq!(model.faces().len(), 1);
    let r = model.response(&[100., 20.]).unwrap();
    assert!(r.face_flows_m3_per_s[0] > 0.);
    assert_eq!(r.cell_outflows_m3_per_s[0], -r.cell_outflows_m3_per_s[1]);
    assert!((r.pressure_work_w - r.dissipation_w).abs() < 1e-12 * r.dissipation_w);
    let shifted = model.response(&[1e5 + 100., 1e5 + 20.]).unwrap();
    assert_eq!(shifted.face_flows_m3_per_s, r.face_flows_m3_per_s);
    assert_eq!(
        model.response(&[10., 10.]).unwrap().face_flows_m3_per_s,
        vec![0.]
    );
}
#[test]
fn permeability_viscosity_scaling_and_bad_inputs() {
    let (x, cells) = patch();
    let k = tensor();
    let a = MixedDarcy::new(x.clone(), cells.clone(), &[k; 2], 0.001, &[])
        .unwrap()
        .response(&[10., 0.])
        .unwrap();
    let double = k.map(|r| r.map(|v| 2. * v));
    let b = MixedDarcy::new(x.clone(), cells.clone(), &[double; 2], 0.001, &[])
        .unwrap()
        .response(&[10., 0.])
        .unwrap();
    assert!((b.face_flows_m3_per_s[0] / a.face_flows_m3_per_s[0] - 2.).abs() < 1e-12);
    let mut bad = k;
    bad[0][0] = -1.;
    assert!(MixedDarcy::new(x.clone(), cells.clone(), &[bad; 2], 0.001, &[]).is_err());
    let mut bad = k;
    bad[0][1] = 1.;
    assert!(MixedDarcy::new(x.clone(), cells.clone(), &[bad; 2], 0.001, &[]).is_err());
    assert!(MixedDarcy::new(x.clone(), cells.clone(), &[k; 2], 0.001, &[([0, 1, 2], 0.)]).is_err());
    let triple = vec![cells[0], cells[1], cells[0]];
    assert!(MixedDarcy::new(x, triple, &[k; 3], 0.001, &[]).is_err());
}

fn porous_specimen() -> (Body, physics::lymph::LymphNetwork) {
    use physics::lymph::*;
    let (x, cells) = patch();
    let m = Material {
        shear_pa: 1000.,
        bulk_pa: 10_000.,
        fibers: vec![],
    };
    let mut body = Body::new(
        x,
        vec![true, true, true, false, false],
        cells.iter().map(|c| (*c, m.clone())).collect(),
    )
    .unwrap();
    let volumes = body
        .stresses_at(body.positions())
        .unwrap()
        .iter()
        .map(|s| s.reference_volume_m3)
        .collect::<Vec<_>>();
    let stores: Vec<_> = volumes
        .iter()
        .enumerate()
        .map(|(i, v)| PoreFluid {
            reference_fluid_volume_m3: v * 0.5,
            fluid_volume_m3: v * 0.5 + if i == 0 { 1e-9 } else { 0. },
            biot_coefficient: 0.8,
            storage_m3_per_pa: 1e-11,
        })
        .collect();
    let spaces = stores
        .iter()
        .map(|s| FluidSpace {
            reference_volume_m3: s.reference_fluid_volume_m3,
            initial_volume_m3: s.fluid_volume_m3,
            initial_protein_kg: s.fluid_volume_m3 * 10.,
            reference_pressure_pa: 0.,
            compliance_m3_per_pa: s.storage_m3_per_pa,
            oncotic_pa_per_kg_m3: 0.,
        })
        .collect();
    body.set_cell_pore_fluids(stores).unwrap();
    let edge = Exchange {
        from: 0,
        to: 1,
        hydraulic_m3_per_pa_s: 0.,
        reflection: 0.,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: 0.,
        valve: false,
    };
    (body, LymphNetwork::new(spaces, vec![edge]).unwrap())
}
#[test]
fn mixed_flux_deforms_oblique_tissue_and_commits_actual_flow_diagnostics() {
    let (body, mut net) = porous_specimen();
    assert!(
        body.reference_darcy_interface(0, 1, [1e-12; 2], 0.001)
            .is_err()
    );
    let v = net.total_volume();
    let m = net.total_protein();
    let initial = net.volumes().to_vec();
    let mut tissue = CellPoreTissue::new(body, vec![0, 1], 4000, 1e-9).unwrap();
    let report = tissue
        .step_mixed_darcy(&mut net, &[tensor(); 2], 0.001, 0.1, 0.005)
        .unwrap();
    assert!(report.transferred_volume_m3[0] > 0.);
    assert!(net.volumes()[0] < initial[0] && net.volumes()[1] > initial[1]);
    assert!((net.total_volume() - v).abs() < 1e-20 && (net.total_protein() - m).abs() < 1e-18);
    let body = tissue.body();
    assert_eq!(
        net.pressures(),
        body.cell_pore_response_at(body.positions()).unwrap().0
    );
    let flow = body
        .deformed_darcy(&[tensor(); 2], 0.001)
        .unwrap()
        .response(&net.pressures())
        .unwrap()
        .face_flows_m3_per_s[0];
    assert_eq!(net.rates().unwrap()[0].0, flow);
    assert!(body.positions()[3][2] > body.rest_positions()[3][2]);
    assert!(body.positions()[4][2] < body.rest_positions()[4][2]);
}
#[test]
fn mixed_coupling_errors_roll_back_and_bad_flux_callback_is_rejected() {
    let (body, mut net) = porous_specimen();
    let x = body.positions().to_vec();
    let v = net.volumes().to_vec();
    let m = net.protein_masses().to_vec();
    let mut tissue = CellPoreTissue::new(body, vec![0, 1], 1, 1e-15).unwrap();
    assert!(
        tissue
            .step_mixed_darcy(&mut net, &[tensor(); 2], 0.001, 1., 0.1)
            .is_err()
    );
    assert_eq!(tissue.body().positions(), x);
    assert_eq!(net.volumes(), v);
    assert_eq!(net.protein_masses(), m);
    assert!(
        net.step_with_pressure_and_flux_laws(
            1.,
            0.1,
            |_, p| Ok(p.to_vec()),
            |_, _, _, _, _| Ok(vec![])
        )
        .is_err()
    );
    assert_eq!(net.volumes(), v);
}

#[test]
fn flux_and_dissipation_rotate_objectively() {
    let (x, cells) = patch();
    let k = tensor();
    let model = MixedDarcy::new(x.clone(), cells.clone(), &[k; 2], 0.001, &[]).unwrap();
    let a = model.response(&[100., 0.]).unwrap();
    let rotate = |v: [f64; 3]| [-v[1], v[0], v[2]];
    let rotation = [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]];
    let rotated = std::array::from_fn::<_, 3, _>(|i| {
        std::array::from_fn::<_, 3, _>(|j| {
            (0..3)
                .flat_map(|a| (0..3).map(move |b| rotation[i][a] * k[a][b] * rotation[j][b]))
                .sum()
        })
    });
    let model = MixedDarcy::new(
        x.into_iter().map(rotate).collect(),
        cells,
        &[rotated; 2],
        0.001,
        &[],
    )
    .unwrap();
    let b = model.response(&[100., 0.]).unwrap();
    assert!((a.face_flows_m3_per_s[0] - b.face_flows_m3_per_s[0]).abs() < 1e-23);
    assert!((a.dissipation_w - b.dissipation_w).abs() < 1e-22);
    for (a, b) in a
        .cell_centroid_velocities_m_per_s
        .iter()
        .zip(b.cell_centroid_velocities_m_per_s)
    {
        let a = rotate(*a);
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 1e-18);
        }
    }
}
#[test]
fn manufactured_quadratic_pressure_refines_on_nonorthogonal_tetrahedra() {
    use std::collections::BTreeMap;
    fn run(n: usize) -> f64 {
        let id = |i, j, k| i + (n + 1) * (j + (n + 1) * k);
        let mut x: Vec<[f64; 3]> = vec![];
        for k in 0..=n {
            for j in 0..=n {
                for i in 0..=n {
                    x.push([i as f64, j as f64, k as f64].map(|v| v * 0.01 / n as f64));
                }
            }
        }
        let mut cells = vec![];
        for k in 0..n {
            for j in 0..n {
                for i in 0..n {
                    let a = id(i, j, k);
                    let z = id(i + 1, j + 1, k + 1);
                    let ring = [
                        id(i + 1, j, k),
                        id(i + 1, j + 1, k),
                        id(i, j + 1, k),
                        id(i, j + 1, k + 1),
                        id(i, j, k + 1),
                        id(i + 1, j, k + 1),
                    ];
                    for r in 0..6 {
                        cells.push([a, ring[r], ring[(r + 1) % 6], z]);
                    }
                }
            }
        }
        let average = |nodes: &[usize]| {
            let value = |axis: usize| {
                let sum: f64 = nodes.iter().map(|i| x[*i][axis]).sum();
                (sum * sum + nodes.iter().map(|i| x[*i][axis].powi(2)).sum::<f64>())
                    / (nodes.len() * (nodes.len() + 1)) as f64
            };
            1e6 * (value(0) - value(1))
        };
        let mut faces = BTreeMap::new();
        for cell in &cells {
            for opposite in 0..4 {
                let mut face =
                    std::array::from_fn::<_, 3, _>(|j| cell[if j < opposite { j } else { j + 1 }]);
                face.sort_unstable();
                *faces.entry(face).or_insert(0usize) += 1;
            }
        }
        let boundaries = faces
            .into_iter()
            .filter(|(_, count)| *count == 1)
            .map(|(f, _)| (f, average(&f)))
            .collect::<Vec<_>>();
        let pressures = cells.iter().map(|c| average(c)).collect::<Vec<_>>();
        let k = [[1e-12, 0., 0.], [0., 1e-12, 0.], [0., 0., 1e-12]];
        let model = MixedDarcy::new(
            x.clone(),
            cells.clone(),
            &vec![k; cells.len()],
            0.001,
            &boundaries,
        )
        .unwrap();
        let response = model.response(&pressures).unwrap();
        let mut error = 0.;
        for (index, (cell, v)) in cells
            .iter()
            .zip(&response.cell_centroid_velocities_m_per_s)
            .enumerate()
        {
            let center = mean(&x, cell);
            let exact = [-0.002 * center[0], 0.002 * center[1], 0.];
            let [a, b, c, d] = cell.map(|i| x[i]);
            let volume = dot(sub(b, a), cross(sub(c, a), sub(d, a))).abs() / 6.;
            // The centroid alone misses RT0's affine variation. Integrate the
            // complete velocity error exactly using the tetrahedral covariance.
            let beta = response.cell_outflows_m3_per_s[index] / (3. * volume);
            let slopes = [beta + 0.002, beta - 0.002, beta];
            let variation: f64 = (0..3)
                .map(|axis| {
                    slopes[axis].powi(2)
                        * cell
                            .iter()
                            .map(|i| (x[*i][axis] - center[axis]).powi(2))
                            .sum::<f64>()
                        / 20.
                })
                .sum();
            error += volume * (dot(sub(*v, exact), sub(*v, exact)) + variation);
        }
        error.sqrt()
    }
    let coarse = run(1);
    let medium = run(2);
    let fine = run(3);
    assert!(
        medium < coarse * 0.8 && fine < medium * 0.8,
        "{coarse} {medium} {fine}"
    );
}

#[test]
fn mixed_pore_flow_preserves_additional_capillary_exchange_and_conservation() {
    use physics::lymph::*;
    let (body, _) = porous_specimen();
    let mut spaces: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|s| FluidSpace {
            reference_volume_m3: s.reference_fluid_volume_m3,
            initial_volume_m3: s.fluid_volume_m3,
            initial_protein_kg: s.fluid_volume_m3 * 10.,
            reference_pressure_pa: 0.,
            compliance_m3_per_pa: s.storage_m3_per_pa,
            oncotic_pa_per_kg_m3: 0.,
        })
        .collect();
    spaces.push(FluidSpace {
        reference_volume_m3: 1e-6,
        initial_volume_m3: 1e-6,
        initial_protein_kg: 1e-5,
        reference_pressure_pa: 80.,
        compliance_m3_per_pa: 1e-11,
        oncotic_pa_per_kg_m3: 0.,
    });
    let pore = Exchange {
        from: 0,
        to: 1,
        hydraulic_m3_per_pa_s: 0.,
        reflection: 0.,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: 0.,
        valve: false,
    };
    let cap = Exchange {
        from: 2,
        to: 0,
        hydraulic_m3_per_pa_s: 1e-13,
        reflection: 0.8,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: 0.,
        valve: false,
    };
    let mut net = LymphNetwork::new(spaces, vec![pore, cap]).unwrap();
    let v = net.total_volume();
    let mass = net.total_protein();
    let mut tissue = CellPoreTissue::new(body, vec![0, 1], 4000, 1e-9).unwrap();
    let report = tissue
        .step_mixed_darcy(&mut net, &[tensor(); 2], 0.001, 0.1, 0.005)
        .unwrap();
    assert!(report.transferred_volume_m3[0] > 0. && report.transferred_volume_m3[1] > 0.);
    let p = net.pressures();
    let rate = net.rates().unwrap()[1];
    assert!((rate.0 - 1e-13 * (p[2] - p[0])).abs() < 1e-25);
    assert!((rate.1 - 0.2 * rate.0 * net.protein_masses()[2] / net.volumes()[2]).abs() < 1e-24);
    assert!((net.total_volume() - v).abs() < 1e-20 && (net.total_protein() - mass).abs() < 1e-18);
}

#[path = "../examples/support/darcy_block.rs"]
mod scale_fixture;
#[test]
fn thousands_of_cells_preserve_patch_accuracy_with_linear_operator_storage() {
    let data = scale_fixture::block(8);
    let count = data.cells.len();
    assert!(count > 256);
    let model = MixedDarcy::new(
        data.points,
        data.cells,
        &vec![scale_fixture::K; count],
        scale_fixture::MU,
        &data.boundaries,
    )
    .unwrap();
    assert!(model.faces().len() > 512);
    assert!(model.operator_storage_bytes() < count * 1000);
    let response = model.response(&data.pressures).unwrap();
    let expected = scale_fixture::expected_velocity();
    for velocity in response.cell_centroid_velocities_m_per_s {
        for i in 0..3 {
            assert!((velocity[i] - expected[i]).abs() < 1e-16);
        }
    }
    assert!(response.solver_iterations < 2000);
    assert!(
        model
            .response_with_solver(
                &data.pressures,
                DarcySolve {
                    max_iterations: 1,
                    relative_tolerance: 1e-13
                }
            )
            .is_err()
    );
}

#[test]
fn thousands_of_fluid_nodes_and_faces_couple_without_old_network_limits() {
    use physics::lymph::*;
    let data = scale_fixture::block(8);
    let n = data.cells.len();
    let material = Material {
        shear_pa: 1000.,
        bulk_pa: 10_000.,
        fibers: vec![],
    };
    let mut body = Body::new(
        data.points.clone(),
        vec![true; data.points.len()],
        data.cells.iter().map(|c| (*c, material.clone())).collect(),
    )
    .unwrap();
    let volumes = body
        .stresses_at(body.positions())
        .unwrap()
        .into_iter()
        .map(|s| s.reference_volume_m3)
        .collect::<Vec<_>>();
    let stores = volumes
        .iter()
        .zip(&data.pressures)
        .map(|(v, p)| PoreFluid {
            reference_fluid_volume_m3: 0.5 * v,
            fluid_volume_m3: 0.5 * v + v * p / 100_000.,
            biot_coefficient: 0.8,
            storage_m3_per_pa: v / 100_000.,
        })
        .collect::<Vec<_>>();
    let spaces = stores
        .iter()
        .map(|s| FluidSpace {
            reference_volume_m3: s.reference_fluid_volume_m3,
            initial_volume_m3: s.fluid_volume_m3,
            initial_protein_kg: 10. * s.fluid_volume_m3,
            reference_pressure_pa: 0.,
            compliance_m3_per_pa: s.storage_m3_per_pa,
            oncotic_pa_per_kg_m3: 0.,
        })
        .collect();
    body.set_cell_pore_fluids(stores).unwrap();
    let model = body
        .deformed_darcy(&vec![scale_fixture::K; n], scale_fixture::MU)
        .unwrap();
    let edges = model
        .faces()
        .iter()
        .map(|f| Exchange {
            from: f.owner,
            to: f.neighbor.unwrap(),
            hydraulic_m3_per_pa_s: 0.,
            reflection: 0.,
            protein_permeability_m3_per_s: 0.,
            pump_head_pa: 0.,
            valve: false,
        })
        .collect::<Vec<_>>();
    assert!(n > 256 && edges.len() > 4096);
    let mut net = LymphNetwork::new(spaces, edges).unwrap();
    let total = net.total_volume();
    let mass = net.total_protein();
    let mut tissue = CellPoreTissue::new(body, (0..n).collect(), 4000, 1e-9).unwrap();
    let report = tissue
        .step_mixed_darcy(
            &mut net,
            &vec![scale_fixture::K; n],
            scale_fixture::MU,
            0.0001,
            0.00005,
        )
        .unwrap();
    assert!(report.transferred_volume_m3.iter().any(|q| q.abs() > 0.));
    assert!(
        (net.total_volume() - total).abs() < 1e-18 && (net.total_protein() - mass).abs() < 1e-17
    );
    assert_eq!(net.volumes().len(), tissue.body().cell_pore_fluids().len());
    assert_eq!(
        net.pressures(),
        tissue
            .body()
            .cell_pore_response_at(tissue.body().positions())
            .unwrap()
            .0
    );
}

#[test]
fn implicit_storage_matches_two_cell_backward_euler_and_energy_balance() {
    let (x, cells) = patch();
    let model = MixedDarcy::new(x, cells, &[tensor(); 2], 0.001, &[]).unwrap();
    let old = [100., 20.];
    let storage = [1e-11, 3e-11];
    let explicit = model.response(&old).unwrap();
    let conductance = explicit.face_flows_m3_per_s[0] / 80.;
    for dt in [1e-4, 0.1, 1000.] {
        let (next, r) = model.implicit_storage_step(&old, &storage, dt).unwrap();
        let expected = 80. / (1. / conductance + dt * (1. / storage[0] + 1. / storage[1]));
        assert!((r.face_flows_m3_per_s[0] / expected - 1.).abs() < 1e-9);
        for i in 0..2 {
            assert!(
                (next[i] - (old[i] - dt * r.cell_outflows_m3_per_s[i] / storage[i])).abs() < 1e-12
            );
        }
        assert!(next.iter().all(|p| *p >= 20. && *p <= 100.));
        let initial: f64 = old.iter().zip(storage).map(|(p, s)| 0.5 * s * p * p).sum();
        let final_energy: f64 = next.iter().zip(storage).map(|(p, s)| 0.5 * s * p * p).sum();
        let numerical_loss: f64 = next
            .iter()
            .zip(old)
            .zip(storage)
            .map(|((p, o), s)| 0.5 * s * (p - o).powi(2))
            .sum();
        assert!(
            (initial - final_energy - dt * r.dissipation_w - numerical_loss).abs()
                < 1e-12 * initial
        );
        assert!((storage[0] * (next[0] - old[0]) + storage[1] * (next[1] - old[1])).abs() < 1e-20);
    }
    assert!(
        model
            .implicit_storage_step(&old, &[0., 1e-11], 0.1)
            .is_err()
    );
    assert!(
        model
            .implicit_storage_step(&old, &storage, f64::NAN)
            .is_err()
    );
}

#[test]
fn implicit_protein_preserves_uniform_concentration_and_large_step_positivity() {
    let (x, cells) = patch();
    let model = MixedDarcy::new(x, cells, &[tensor(); 2], 0.001, &[]).unwrap();
    // A uniform concentration must remain uniform when the same face flow
    // transfers water and protein, including a 50% donor-volume change.
    let old = [3e-9, 6e-9];
    let volumes = [0.5e-9, 2.5e-9];
    let next = model
        .implicit_protein_step(&old, &volumes, &[0.5e-9], 1.)
        .unwrap();
    assert!((next[0] / volumes[0] - 3.).abs() < 1e-12);
    assert!((next[1] / volumes[1] - 3.).abs() < 1e-12);
    for q in [1e-9, -1e-9, 1e-3, -1e-3] {
        let next = model
            .implicit_protein_step(&old, &[1e-9, 2e-9], &[q], 100.)
            .unwrap();
        assert!(next.iter().all(|m| *m >= 0.));
        assert!((next.iter().sum::<f64>() - 9e-9).abs() < 1e-20);
        let donor = if q > 0. { 0 } else { 1 };
        let expected = old[donor] / (1. + 100. * q.abs() / [1e-9, 2e-9][donor]);
        assert!((next[donor] / expected - 1.).abs() < 1e-12);
    }
    assert_eq!(
        model
            .implicit_protein_step(&[0., 0.], &volumes, &[1e-9], 1.)
            .unwrap(),
        vec![0., 0.]
    );
    assert!(
        model
            .implicit_protein_step(&[-1., 0.], &volumes, &[1e-9], 1.)
            .is_err()
    );
    assert!(
        model
            .implicit_protein_step(&old, &[0., 1.], &[1e-9], 1.)
            .is_err()
    );
}

#[test]
fn implicit_reservoir_exchange_matches_compliance_and_boundary_energy_work() {
    let (x, cells) = patch();
    let cell = cells[0];
    let model = MixedDarcy::new(
        x,
        vec![cell],
        &[tensor()],
        0.001,
        &[([cell[0], cell[1], cell[2]], 100.)],
    )
    .unwrap();
    let conductance = -model.response(&[0.]).unwrap().face_flows_m3_per_s[0] / 100.;
    let storage = 1e-11;
    for dt in [1e-4, 0.1, 1000.] {
        let (p, r) = model.implicit_storage_step(&[20.], &[storage], dt).unwrap();
        let expected =
            (20. + dt * conductance * 100. / storage) / (1. + dt * conductance / storage);
        assert!((p[0] - expected).abs() < 1e-10);
        let q = r.face_flows_m3_per_s[0];
        assert!(q < 0. && p[0] > 20. && p[0] < 100.);
        assert!((storage * (p[0] - 20.) + dt * q).abs() < 1e-20);
        let stored_change = 0.5 * storage * (p[0] * p[0] - 400.);
        let boundary_work = -dt * 100. * q;
        let temporal_loss = 0.5 * storage * (p[0] - 20.).powi(2);
        assert!(
            (boundary_work - stored_change - dt * r.dissipation_w - temporal_loss).abs()
                < 1e-12 * boundary_work
        );
    }
}

#[test]
fn implicit_boundary_protein_accounts_for_inflow_and_outflow() {
    let (x, cells) = patch();
    let c = cells[0];
    let model = MixedDarcy::new(
        x,
        vec![c],
        &[tensor()],
        0.001,
        &[([c[0], c[1], c[2]], 100.)],
    )
    .unwrap();
    let incoming = model
        .implicit_protein_step_with_boundary(&[3e-9], &[1.5e-9], &[-0.5e-9], 1., &[Some(6.)])
        .unwrap();
    assert!((incoming[0] - 6e-9).abs() < 1e-22);
    let outgoing = model
        .implicit_protein_step_with_boundary(&[3e-9], &[0.5e-9], &[0.5e-9], 1., &[None])
        .unwrap();
    assert!((outgoing[0] - 1.5e-9).abs() < 1e-22);
    assert!(
        model
            .implicit_protein_step_with_boundary(&[0.], &[1e-9], &[-1e-9], 1., &[None])
            .is_err()
    );
    let empty = model
        .implicit_protein_step_with_boundary(&[0.], &[1e-9], &[-1e-9], 1., &[Some(2.)])
        .unwrap();
    assert!((empty[0] - 2e-9).abs() < 1e-22);
    assert!(
        model
            .implicit_protein_step(&[3e-9], &[1e-9], &[1e-9], 1.)
            .is_err()
    );
}

#[test]
fn finite_reservoir_multiport_conserves_combined_storage_and_energy() {
    let (x, cells) = patch();
    let c = cells[0];
    let model = MixedDarcy::new(
        x,
        vec![c],
        &[tensor()],
        0.001,
        &[([c[0], c[1], c[2]], 100.), ([c[0], c[1], c[3]], 100.)],
    )
    .unwrap();
    let conductance = -model
        .response(&[0.])
        .unwrap()
        .face_flows_m3_per_s
        .iter()
        .sum::<f64>()
        / 100.;
    let storage = 1e-11;
    let reservoir_storage = 3e-11;
    for dt in [1e-4, 0.1, 1000.] {
        let (next, reservoir, r) = model
            .implicit_reservoir_step(&[20.], &[storage], dt, 100., reservoir_storage)
            .unwrap();
        let q = r.face_flows_m3_per_s.iter().sum::<f64>();
        let expected = -80. / (1. / conductance + dt * (1. / storage + 1. / reservoir_storage));
        assert!((q / expected - 1.).abs() < 1e-9);
        assert!((storage * (next[0] - 20.) + reservoir_storage * (reservoir - 100.)).abs() < 1e-20);
        assert!(next[0] > 20. && reservoir < 100.);
        let old_energy = 0.5 * storage * 400. + 0.5 * reservoir_storage * 10_000.;
        let new_energy =
            0.5 * storage * next[0] * next[0] + 0.5 * reservoir_storage * reservoir * reservoir;
        let time_loss = 0.5 * storage * (next[0] - 20.).powi(2)
            + 0.5 * reservoir_storage * (reservoir - 100.).powi(2);
        assert!(
            (old_energy - new_energy - dt * r.dissipation_w - time_loss).abs() < 1e-12 * old_energy
        );
    }
    assert!(
        model
            .implicit_reservoir_step(&[20.], &[storage], 0.1, 100., 0.)
            .is_err()
    );
}

#[test]
fn finite_reservoir_protein_mixes_bidirectional_ports_and_conserves_mass() {
    let (x, cells) = patch();
    let c = cells[0];
    let model = MixedDarcy::new(
        x,
        vec![c],
        &[tensor()],
        0.001,
        &[([c[0], c[1], c[2]], 100.), ([c[0], c[1], c[3]], 100.)],
    )
    .unwrap();
    for dt in [0.001, 1., 100.] {
        let (tissue, reservoir) = model
            .implicit_protein_reservoir_step(&[1e-9], &[1e-9], &[1e-9, -1e-9], dt, 6e-9, 2e-9)
            .unwrap();
        let a = dt * 1e-9 / 2e-9;
        let b = dt;
        let expected = ((1. + a) * 1e-9 + a * 6e-9) / (1. + a + b);
        assert!((tissue[0] / expected - 1.).abs() < 1e-12);
        assert!((tissue[0] + reservoir - 7e-9).abs() < 1e-21);
        assert!(tissue[0] >= 0. && reservoir >= 0.);
    }
    let (tissue, reservoir) = model
        .implicit_protein_reservoir_step(&[3e-9], &[1.5e-9], &[-0.5e-9, 0.], 1., 6e-9, 1.5e-9)
        .unwrap();
    assert!((tissue[0] / 1.5e-9 - 3.).abs() < 1e-12);
    assert!((reservoir / 1.5e-9 - 3.).abs() < 1e-12);
    assert!(
        model
            .implicit_protein_reservoir_step(&[3e-9], &[1e-9], &[0., 0.], 1., -1., 1e-9)
            .is_err()
    );
}

#[test]
fn persistent_reservoir_steps_conserve_absolute_inventories_and_roll_back() {
    let (x, cells) = patch();
    let c = cells[0];
    let model = MixedDarcy::new(
        x,
        vec![c],
        &[tensor()],
        0.001,
        &[([c[0], c[1], c[2]], 100.)],
    )
    .unwrap();
    let mut pressure = vec![20.];
    let mut fluid = vec![1e-9];
    let mut protein = vec![1e-8];
    let mut reservoir = PoreReservoir {
        reference_volume_m3: 2e-9,
        reference_pressure_pa: 0.,
        compliance_m3_per_pa: 3e-11,
        fluid_volume_m3: 5e-9,
        protein_kg: 1e-7,
    };
    let total_water = fluid[0] + reservoir.fluid_volume_m3;
    let total_protein = protein[0] + reservoir.protein_kg;
    let mut previous_gap = reservoir.pressure_pa().unwrap() - pressure[0];
    for _ in 0..5 {
        model
            .implicit_reservoir_transport_step(
                &mut pressure,
                &mut fluid,
                &mut protein,
                &[1e-11],
                &mut reservoir,
                0.1,
            )
            .unwrap();
        let gap = reservoir.pressure_pa().unwrap() - pressure[0];
        assert!(gap >= 0. && gap < previous_gap);
        previous_gap = gap;
        assert!((fluid[0] + reservoir.fluid_volume_m3 - total_water).abs() < 1e-21);
        assert!((protein[0] + reservoir.protein_kg - total_protein).abs() < 1e-19);
    }
    protein[0] = -1.;
    let before = (pressure.clone(), fluid.clone(), protein.clone(), reservoir);
    assert!(
        model
            .implicit_reservoir_transport_step(
                &mut pressure,
                &mut fluid,
                &mut protein,
                &[1e-11],
                &mut reservoir,
                0.1
            )
            .is_err()
    );
    assert_eq!((pressure, fluid, protein, reservoir), before);
}

#[test]
fn hydraulic_boundary_resistance_matches_series_flow_and_implicit_storage() {
    let (points, cells) = patch();
    let port = [0, 1, 3];
    let model = MixedDarcy::new(
        points,
        cells[..1].to_vec(),
        &[tensor()],
        0.001,
        &[(port, 0.)],
    )
    .unwrap();
    let conductance = model.response(&[1.]).unwrap().cell_outflows_m3_per_s[0];
    for resistance in [0., 0.1 / conductance, 100. / conductance] {
        let limited = model
            .with_added_boundary_resistances(&[(port, resistance)])
            .unwrap();
        let g = 1. / (1. / conductance + resistance);
        for pressure in [-100., 100.] {
            let response = limited.response(&[pressure]).unwrap();
            assert!(
                (response.cell_outflows_m3_per_s[0] - g * pressure).abs()
                    < 1e-10 * g * pressure.abs()
            );
            assert!(
                (response.dissipation_w - g * pressure * pressure).abs()
                    < 1e-10 * g * pressure * pressure
            );
        }
        let dt = 0.1;
        let storage = 1e-11;
        let (p, q) = limited
            .implicit_storage_step(&[100.], &[storage], dt)
            .unwrap();
        let expected = 100. / (1. + dt * g / storage);
        assert!((p[0] - expected).abs() < 1e-8);
        assert!((q.cell_outflows_m3_per_s[0] - g * p[0]).abs() < 1e-10 * g * p[0]);
        let (p, r, q) = limited
            .implicit_reservoir_step(&[100.], &[storage], dt, 0., storage)
            .unwrap();
        let expected_q = 100. / (1. / g + 2. * dt / storage);
        assert!((q.cell_outflows_m3_per_s[0] - expected_q).abs() < 1e-10 * expected_q);
        assert!((p[0] + r - 100.).abs() < 1e-8);
    }
    let original = model.response(&[100.]).unwrap().face_flows_m3_per_s;
    for assignments in [
        vec![(port, -1.)],
        vec![(port, f64::NAN)],
        vec![(port, 1.), ([3, 1, 0], 2.)],
        vec![([0, 1, 2], 1.)],
    ] {
        assert!(model.with_added_boundary_resistances(&assignments).is_err());
    }
    assert_eq!(
        model.response(&[100.]).unwrap().face_flows_m3_per_s,
        original
    );
}

#[test]
fn selective_membrane_advection_and_diffusion_match_discrete_mass_ledger() {
    let (points, cells) = patch();
    let model = MixedDarcy::new(
        points,
        cells[..1].to_vec(),
        &[tensor()],
        0.001,
        &[([0, 1, 3], 0.)],
    )
    .unwrap();
    let volume = 1e-9;
    let dt = 0.01;
    let old_mass = 10. * volume;
    for reflection in [0., 0.8, 1.] {
        for q in [-0.2 * volume / dt, 0., 0.2 * volume / dt] {
            for diffusion in [0., 5. * volume / dt, 1000. * volume / dt] {
                let new_volume = volume - dt * q;
                let exterior = 20.;
                let membrane = ProteinMembrane {
                    concentration_kg_per_m3: exterior,
                    reflection,
                    diffusive_conductance_m3_per_s: diffusion,
                };
                let mass = model
                    .implicit_protein_step_with_membranes(
                        &[old_mass],
                        &[new_volume],
                        &[q],
                        dt,
                        &[Some(membrane)],
                    )
                    .unwrap()[0];
                let expected = (old_mass
                    + dt * ((-q).max(0.) * (1. - reflection) + diffusion) * exterior)
                    / (1. + dt * (q.max(0.) * (1. - reflection) + diffusion) / new_volume);
                assert!((mass - expected).abs() < 1e-12 * old_mass);
                let concentration = mass / new_volume;
                let donor = if q < 0. { exterior } else { concentration };
                let flux = (1. - reflection) * q * donor + diffusion * (concentration - exterior);
                assert!((mass - old_mass + dt * flux).abs() < 1e-10 * old_mass);
                assert!(mass.is_finite() && mass >= 0.);
                if reflection == 1. && diffusion == 0. {
                    assert_eq!(mass, old_mass);
                }
            }
        }
    }
    let membrane = ProteinMembrane {
        concentration_kg_per_m3: 20.,
        reflection: 1.,
        diffusive_conductance_m3_per_s: volume / dt,
    };
    assert!(
        model
            .implicit_protein_step_with_membranes(&[0.], &[volume], &[0.], dt, &[Some(membrane)])
            .unwrap()[0]
            > 0.
    );
    for invalid in [
        ProteinMembrane {
            reflection: 1.1,
            ..membrane
        },
        ProteinMembrane {
            reflection: -0.1,
            ..membrane
        },
        ProteinMembrane {
            diffusive_conductance_m3_per_s: -1.,
            ..membrane
        },
        ProteinMembrane {
            concentration_kg_per_m3: f64::NAN,
            ..membrane
        },
    ] {
        assert!(
            model
                .implicit_protein_step_with_membranes(
                    &[old_mass],
                    &[volume],
                    &[0.],
                    dt,
                    &[Some(invalid)]
                )
                .is_err()
        );
    }
}

#[test]
fn nonlinear_capillary_and_lymph_loop_couples_to_deforming_rt0_tissue() {
    use physics::lymph::*;
    let (mut tissue, mut net, laws) = nonlinear_exchange_fixture();
    let v = net.total_volume();
    let mass = net.total_protein();
    let report = tissue
        .step_mixed_darcy_with_osmotic_laws(
            &mut net,
            &[tensor(); 2],
            0.001,
            0.1,
            0.005,
            Some(&laws),
        )
        .unwrap();
    assert!(report.transferred_volume_m3[0] > 0. && report.transferred_volume_m3[1] > 0.);
    let p = net.pressures();
    let rate = net.rates().unwrap()[1];
    let c0 = net.protein_masses()[0] / net.volumes()[0];
    let c2 = net.protein_masses()[2] / net.volumes()[2];
    let pi0 = c0 + 0.1 * c0 * c0 + 0.001 * c0 * c0 * c0;
    let pi2 = 2. * c2 + 0.2 * c2 * c2 + 0.002 * c2 * c2 * c2;
    assert!((rate.0 - 1e-13 * (p[2] - p[0] - 0.8 * (pi2 - pi0))).abs() < 1e-25);
    assert!((rate.1 - 0.2 * rate.0 * net.protein_masses()[2] / net.volumes()[2]).abs() < 1e-24);
    assert!(report.transferred_volume_m3[2] > 0. && report.transferred_volume_m3[3] > 0.);
    for i in 0..2 {
        assert_eq!(
            tissue.body().cell_pore_fluids()[i].fluid_volume_m3,
            net.volumes()[i]
        );
    }
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-6,
        length_m: 0.01,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: -80.,
        active_tension_n_per_m: 0.01,
    };
    let walls = [None, None, None, Some(wall)];
    tissue
        .step_mixed_darcy_with_lymphatic_walls(
            &mut net,
            &[tensor(); 2],
            0.001,
            0.1,
            0.005,
            Some(&laws),
            Some(&walls),
        )
        .unwrap();
    let r = (net.volumes()[3] / std::f64::consts::PI / 0.01).sqrt();
    let stretch = (net.volumes()[3] / 1e-6).sqrt();
    let expected = -80. + 20. * ((2. * (stretch - 1.)).exp() - stretch.powi(-3)) + 0.01 / r;
    assert!((net.pressures()[3] - expected).abs() < 1e-12);
    assert!((net.total_volume() - v).abs() < 1e-20 && (net.total_protein() - mass).abs() < 1e-18);
    let before = (
        tissue.body().positions().to_vec(),
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
    );
    assert!(
        tissue
            .step_mixed_darcy_with_osmotic_laws(
                &mut net,
                &[tensor(); 2],
                0.001,
                0.1,
                0.005,
                Some(&laws[..3])
            )
            .is_err()
    );
    assert_eq!(
        before,
        (
            tissue.body().positions().to_vec(),
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap()
        )
    );
    assert!((net.total_volume() - v).abs() < 1e-20 && (net.total_protein() - mass).abs() < 1e-18);
    let mut exhausted = CellPoreTissue::new(tissue.body().clone(), vec![0, 1], 1, 1e-20).unwrap();
    let previous_body = exhausted.body().positions().to_vec();
    assert!(
        exhausted
            .step_mixed_darcy_with_osmotic_laws(
                &mut net,
                &[tensor(); 2],
                0.001,
                0.1,
                0.005,
                Some(&laws)
            )
            .is_err()
    );
    assert_eq!(previous_body, exhausted.body().positions());
    assert_eq!(
        before,
        (
            tissue.body().positions().to_vec(),
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap()
        )
    );
}

fn nonlinear_exchange_fixture() -> (
    CellPoreTissue,
    physics::lymph::LymphNetwork,
    [OsmoticPressureLaw; 4],
) {
    use physics::lymph::*;
    let (body, _) = porous_specimen();
    let mut spaces: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|s| FluidSpace {
            reference_volume_m3: s.reference_fluid_volume_m3,
            initial_volume_m3: s.fluid_volume_m3,
            initial_protein_kg: s.fluid_volume_m3 * 10.,
            reference_pressure_pa: 0.,
            compliance_m3_per_pa: s.storage_m3_per_pa,
            oncotic_pa_per_kg_m3: 0.,
        })
        .collect();
    spaces.push(FluidSpace {
        reference_volume_m3: 1e-6,
        initial_volume_m3: 1e-6,
        initial_protein_kg: 1e-5,
        reference_pressure_pa: 80.,
        compliance_m3_per_pa: 1e-11,
        oncotic_pa_per_kg_m3: 0.,
    });
    let pore = Exchange {
        from: 0,
        to: 1,
        hydraulic_m3_per_pa_s: 0.,
        reflection: 0.,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: 0.,
        valve: false,
    };
    let cap = Exchange {
        from: 2,
        to: 0,
        hydraulic_m3_per_pa_s: 1e-13,
        reflection: 0.8,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: 0.,
        valve: false,
    };
    spaces.push(FluidSpace {
        reference_volume_m3: 1e-6,
        initial_volume_m3: 1e-6,
        initial_protein_kg: 5e-6,
        reference_pressure_pa: -80.,
        compliance_m3_per_pa: 1e-11,
        oncotic_pa_per_kg_m3: 0.,
    });
    let inlet = Exchange {
        from: 1,
        to: 3,
        hydraulic_m3_per_pa_s: 1e-13,
        reflection: 0.,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: 0.,
        valve: true,
    };
    let ret = Exchange {
        from: 3,
        to: 2,
        hydraulic_m3_per_pa_s: 1e-14,
        reflection: 0.,
        protein_permeability_m3_per_s: 0.,
        pump_head_pa: 200.,
        valve: true,
    };
    let laws = [
        OsmoticPressureLaw {
            linear: 1.,
            quadratic: 0.1,
            cubic: 0.001,
        },
        OsmoticPressureLaw {
            linear: 1.,
            quadratic: 0.1,
            cubic: 0.001,
        },
        OsmoticPressureLaw {
            linear: 2.,
            quadratic: 0.2,
            cubic: 0.002,
        },
        OsmoticPressureLaw {
            linear: 0.5,
            quadratic: 0.05,
            cubic: 0.,
        },
    ];
    let net = LymphNetwork::new(spaces, vec![pore, cap, inlet, ret]).unwrap();
    let tissue = CellPoreTissue::new(body, vec![0, 1], 4000, 1e-9).unwrap();
    (tissue, net, laws)
}

#[test]
fn adaptive_deforming_tissue_lymph_controls_geometry_and_rolls_back() {
    use physics::lymph::*;
    let (initial, network, laws) = nonlinear_exchange_fixture();
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-6,
        length_m: 0.01,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: -80.,
        active_tension_n_per_m: 0.01,
    };
    let walls = [None, None, None, Some(wall)];
    let fixed = |h: f64| {
        let mut tissue = initial.clone();
        let mut net = network.clone();
        tissue
            .step_mixed_darcy_with_lymphatic_walls(
                &mut net,
                &[tensor(); 2],
                0.001,
                0.1,
                h,
                Some(&laws),
                Some(&walls),
            )
            .unwrap();
        (tissue, net)
    };
    let reference = fixed(0.001);
    let finer = fixed(0.0005);
    let position_difference = |a: &CellPoreTissue, b: &CellPoreTissue| {
        a.body()
            .positions()
            .iter()
            .zip(b.body().positions())
            .map(|(a, b)| dot(sub(*a, *b), sub(*a, *b)).sqrt())
            .fold(0., f64::max)
    };
    assert!(position_difference(&reference.0, &finer.0) < 1e-10);
    let mut geometry_errors = Vec::new();
    let mut accepted_counts = Vec::new();
    for (relative, position_tolerance) in [(1e-3, 1e-8), (1e-3, 1e-10)] {
        let mut tissue = initial.clone();
        let mut net = network.clone();
        let cfg = AdaptiveTissueExchangeConfig {
            exchange: AdaptiveExchangeConfig {
                relative_tolerance: relative,
                absolute_volume_tolerance_m3: 1e-18,
                absolute_protein_tolerance_kg: 1e-17,
                min_step_seconds: 1e-7,
                max_step_seconds: 0.1,
                max_trials: 10_000,
            },
            absolute_position_tolerance_m: position_tolerance,
        };
        let report = tissue
            .step_mixed_darcy_with_lymphatic_walls_adaptive(
                &mut net,
                &[tensor(); 2],
                0.001,
                0.1,
                Some(&laws),
                &walls,
                cfg,
            )
            .unwrap();
        assert!(report.max_accepted_error_ratio <= 1.);
        assert_eq!(report.exchange.substeps, 2 * report.accepted_steps);
        assert!((net.total_volume() - network.total_volume()).abs() < 1e-20);
        assert!((net.total_protein() - network.total_protein()).abs() < 1e-18);
        for i in 0..2 {
            assert_eq!(
                net.volumes()[i],
                tissue.body().cell_pore_fluids()[i].fluid_volume_m3
            );
        }
        let actual_pressure = tissue
            .body()
            .cell_pore_response_at(tissue.body().positions())
            .unwrap()
            .0;
        assert_eq!(&net.pressures()[..2], actual_pressure.as_slice());
        let error = position_difference(&tissue, &reference.0);
        eprintln!(
            "adaptive tissue relative={relative} position_tolerance_m={position_tolerance} accepted={} rejected={} position_error_m={error:.12e}",
            report.accepted_steps, report.rejected_steps
        );
        geometry_errors.push(error);
        accepted_counts.push(report.accepted_steps);
    }
    assert!(
        geometry_errors[1] < geometry_errors[0],
        "{geometry_errors:?}"
    );
    assert!(
        accepted_counts[1] > accepted_counts[0],
        "geometry tolerance must affect time steps"
    );
    let cfg = AdaptiveTissueExchangeConfig {
        exchange: AdaptiveExchangeConfig {
            relative_tolerance: 1e-3,
            absolute_volume_tolerance_m3: 1e-14,
            absolute_protein_tolerance_kg: 1e-13,
            min_step_seconds: 1e-7,
            max_step_seconds: 0.01,
            max_trials: 1,
        },
        absolute_position_tolerance_m: 1e-6,
    };
    let mut tissue = initial.clone();
    let mut net = network.clone();
    let before = (
        tissue.body().positions().to_vec(),
        tissue.body().evaluate(tissue.body().positions()).unwrap(),
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
    );
    let mut proof = initial.clone();
    let mut proof_net = network.clone();
    assert_eq!(
        proof
            .step_mixed_darcy_with_lymphatic_walls_adaptive(
                &mut proof_net,
                &[tensor(); 2],
                0.001,
                0.01,
                Some(&laws),
                &walls,
                cfg
            )
            .unwrap()
            .accepted_steps,
        1
    );
    assert_eq!(
        tissue
            .step_mixed_darcy_with_lymphatic_walls_adaptive(
                &mut net,
                &[tensor(); 2],
                0.001,
                0.02,
                Some(&laws),
                &walls,
                cfg
            )
            .unwrap_err(),
        "adaptive exchange trial limit"
    );
    assert_eq!(
        before,
        (
            tissue.body().positions().to_vec(),
            tissue.body().evaluate(tissue.body().positions()).unwrap(),
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap()
        )
    );
}

#[test]
fn attached_lymphatic_wall_uses_current_interstitial_pressure_and_rejects_bad_links() {
    use physics::lymph::*;
    let (initial, network, laws) = nonlinear_exchange_fixture();
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-6,
        length_m: 0.01,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: -80.,
        active_tension_n_per_m: 0.01,
    };
    let walls = [None, None, None, Some(wall)];
    let cfg = AdaptiveTissueExchangeConfig {
        exchange: AdaptiveExchangeConfig {
            relative_tolerance: 1e-3,
            absolute_volume_tolerance_m3: 1e-18,
            absolute_protein_tolerance_kg: 1e-17,
            min_step_seconds: 1e-7,
            max_step_seconds: 0.1,
            max_trials: 10_000,
        },
        absolute_position_tolerance_m: 1e-8,
    };
    let mut wall_pressures = Vec::new();
    for cell in [0, 1] {
        let mut tissue = initial.clone();
        let mut net = network.clone();
        tissue
            .step_mixed_darcy_with_attached_lymphatic_walls_adaptive(
                &mut net,
                &[tensor(); 2],
                0.001,
                0.1,
                Some(&laws),
                &walls,
                &[LymphaticWallAttachment {
                    compartment: 3,
                    tissue_cell: cell,
                }],
                cfg,
            )
            .unwrap();
        let pressures = tissue
            .body()
            .cell_pore_response_at(tissue.body().positions())
            .unwrap()
            .0;
        let expected = wall.pressure_pa(net.volumes()[3]).unwrap() + pressures[cell];
        assert!((net.pressures()[3] - expected).abs() < 1e-12);
        assert_eq!(&net.pressures()[..2], pressures.as_slice());
        let q = 1e-13 * (net.pressures()[1] - net.pressures()[3]).max(0.);
        assert!((net.rates().unwrap()[2].0 - q).abs() < 1e-25);
        assert!((net.total_volume() - network.total_volume()).abs() < 1e-20);
        assert!((net.total_protein() - network.total_protein()).abs() < 1e-18);
        wall_pressures.push(net.pressures()[3]);
    }
    assert!(
        (wall_pressures[0] - wall_pressures[1]).abs() > 1.,
        "local attachment should affect pressure"
    );
    let mut tissue = initial;
    let mut net = network;
    let before = (
        tissue.body().positions().to_vec(),
        tissue.body().evaluate(tissue.body().positions()).unwrap(),
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
    );
    for bad in [
        vec![LymphaticWallAttachment {
            compartment: usize::MAX,
            tissue_cell: 0,
        }],
        vec![LymphaticWallAttachment {
            compartment: 3,
            tissue_cell: 2,
        }],
        vec![LymphaticWallAttachment {
            compartment: 0,
            tissue_cell: 0,
        }],
        vec![
            LymphaticWallAttachment {
                compartment: 3,
                tissue_cell: 0
            };
            2
        ],
    ] {
        assert!(
            tissue
                .step_mixed_darcy_with_attached_lymphatic_walls_adaptive(
                    &mut net,
                    &[tensor(); 2],
                    0.001,
                    0.1,
                    Some(&laws),
                    &walls,
                    &bad,
                    cfg
                )
                .is_err()
        );
        assert_eq!(
            before,
            (
                tissue.body().positions().to_vec(),
                tissue.body().evaluate(tissue.body().positions()).unwrap(),
                net.volumes().to_vec(),
                net.protein_masses().to_vec(),
                net.pressures(),
                net.rates().unwrap()
            )
        );
    }
}

#[test]
fn current_radius_resistance_couples_to_tissue_and_lymph_ledgers() {
    use physics::lymph::*;
    let (mut tissue, mut net, laws) = nonlinear_exchange_fixture();
    let totals = (net.total_volume(), net.total_protein());
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-6,
        length_m: 0.01,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: -80.,
        active_tension_n_per_m: 0.01,
    };
    let walls = [None, None, None, Some(wall)];
    let links = [2, 3].map(|edge| LymphaticHydraulicAttachment {
        edge,
        compartment: 3,
        segment_length_m: 0.005,
        viscosity_pa_s: 0.001,
    });
    let cfg = AdaptiveTissueExchangeConfig {
        exchange: AdaptiveExchangeConfig {
            relative_tolerance: 1e-3,
            absolute_volume_tolerance_m3: 1e-18,
            absolute_protein_tolerance_kg: 1e-17,
            min_step_seconds: 1e-7,
            max_step_seconds: 0.1,
            max_trials: 10_000,
        },
        absolute_position_tolerance_m: 1e-8,
    };
    tissue
        .step_mixed_darcy_with_lymphatic_geometry_adaptive(
            &mut net,
            &[tensor(); 2],
            0.001,
            0.1,
            Some(&laws),
            &walls,
            &[LymphaticWallAttachment {
                compartment: 3,
                tissue_cell: 1,
            }],
            &links,
            cfg,
        )
        .unwrap();
    let p = net.pressures();
    let radius = (net.volumes()[3] / std::f64::consts::PI / 0.01).sqrt();
    let resistance = 8. * 0.001 * 0.005 / (std::f64::consts::PI * radius.powi(4));
    let rates = net.rates().unwrap();
    for index in [2, 3] {
        let e = net.edges()[index];
        let expected = (p[e.from] - p[e.to] + e.pump_head_pa).max(0.)
            / (1. / e.hydraulic_m3_per_pa_s + resistance);
        assert!((rates[index].0 - expected).abs() < 1e-25);
        let donor = net.protein_masses()[e.from] / net.volumes()[e.from];
        assert!((rates[index].1 - expected * donor).abs() < 1e-24);
    }
    assert!((net.total_volume() - totals.0).abs() < 1e-20);
    assert!((net.total_protein() - totals.1).abs() < 1e-18);
    let before = (
        tissue.body().positions().to_vec(),
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
    );
    let bad = [LymphaticHydraulicAttachment {
        edge: 0,
        compartment: 3,
        ..links[0]
    }];
    assert!(
        tissue
            .step_mixed_darcy_with_lymphatic_geometry_adaptive(
                &mut net,
                &[tensor(); 2],
                0.001,
                0.1,
                Some(&laws),
                &walls,
                &[],
                &bad,
                cfg
            )
            .is_err()
    );
    assert_eq!(
        before,
        (
            tissue.body().positions().to_vec(),
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap()
        )
    );
}

#[test]
fn inertial_lymph_rt0_fem_commits_matching_pressure_geometry_and_momentum() {
    use physics::lymph::*;
    let (mut tissue, mut net, laws) = nonlinear_exchange_fixture();
    let totals = (net.total_volume(), net.total_protein());
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-6,
        length_m: 0.01,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: -80.,
        active_tension_n_per_m: 0.01,
    };
    let walls = [None, None, None, Some(wall)];
    let links = [2, 3].map(|edge| LymphaticHydraulicAttachment {
        edge,
        compartment: 3,
        segment_length_m: 0.005,
        viscosity_pa_s: 0.001,
    });
    let attachments = [LymphaticWallAttachment {
        compartment: 3,
        tissue_cell: 1,
    }];
    let mut history = vec![0.; net.edges().len()];
    let dt = 1e-5;
    tissue
        .step_mixed_darcy_with_inertial_lymphatic_geometry(
            &mut net,
            &[tensor(); 2],
            0.001,
            dt,
            1000.,
            &mut history,
            100,
            1e-18,
            1e-17,
            Some(&laws),
            Some(&walls),
            &attachments,
            &links,
        )
        .unwrap();
    let p = net.pressures();
    let cell_p = tissue
        .body()
        .cell_pore_response_at(tissue.body().positions())
        .unwrap()
        .0;
    assert!((p[0] - cell_p[0]).abs() < 1e-12);
    assert!((p[1] - cell_p[1]).abs() < 1e-12);
    let actualwall = LymphaticWallLaw {
        external_pressure_pa: wall.external_pressure_pa + p[1],
        ..wall
    };
    assert!((p[3] - actualwall.pressure_pa(net.volumes()[3]).unwrap()).abs() < 1e-12);
    let r = wall.radius_m(net.volumes()[3]).unwrap();
    let resistance = 8. * 0.001 * 0.005 / (std::f64::consts::PI * r.powi(4));
    let inertance = 1000. * 0.005 / (std::f64::consts::PI * r * r);
    for i in [2, 3] {
        let e = net.edges()[i];
        let expected = (p[e.from] - p[e.to] + e.pump_head_pa).max(0.)
            / (1. / e.hydraulic_m3_per_pa_s + resistance + inertance / dt);
        assert!((history[i] - expected).abs() < 1e-25);
    }
    assert!((net.total_volume() - totals.0).abs() < 1e-20);
    assert!((net.total_protein() - totals.1).abs() < 1e-18);
    let before = (
        tissue.body().positions().to_vec(),
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        history.clone(),
    );
    assert!(
        tissue
            .step_mixed_darcy_with_inertial_lymphatic_geometry(
                &mut net,
                &[tensor(); 2],
                0.001,
                dt,
                1000.,
                &mut history,
                1,
                1e-30,
                1e-30,
                Some(&laws),
                Some(&walls),
                &attachments,
                &links
            )
            .is_err()
    );
    assert_eq!(
        before,
        (
            tissue.body().positions().to_vec(),
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            history
        )
    );
}

#[test]
fn adaptive_inertial_tissue_preserves_joint_state_and_rolls_back_budget_failure() {
    use physics::lymph::*;
    let (mut tissue, mut net, laws) = nonlinear_exchange_fixture();
    let totals = (net.total_volume(), net.total_protein());
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-6,
        length_m: 0.01,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: -80.,
        active_tension_n_per_m: 0.01,
    };
    let walls = [None, None, None, Some(wall)];
    let links = [2, 3].map(|edge| LymphaticHydraulicAttachment {
        edge,
        compartment: 3,
        segment_length_m: 0.005,
        viscosity_pa_s: 0.001,
    });
    let attachments = [LymphaticWallAttachment {
        compartment: 3,
        tissue_cell: 1,
    }];
    let mut history = vec![0.; net.edges().len()];
    let cfg = AdaptiveTissueExchangeConfig {
        exchange: AdaptiveExchangeConfig {
            relative_tolerance: 1e-3,
            absolute_volume_tolerance_m3: 1e-18,
            absolute_protein_tolerance_kg: 1e-17,
            min_step_seconds: 1e-12,
            max_step_seconds: 5e-5,
            max_trials: 10000,
        },
        absolute_position_tolerance_m: 1e-8,
    };
    let report = tissue
        .step_mixed_darcy_with_inertial_lymphatic_geometry_adaptive(
            &mut net,
            &[tensor(); 2],
            0.001,
            1e-4,
            &laws,
            &walls,
            &attachments,
            &links,
            1000.,
            &mut history,
            cfg,
            1e-12,
            100,
            1e-20,
            1e-19,
        )
        .unwrap();
    eprintln!(
        "adaptive inertial FEM accepted={} rejected={} maxerror={}",
        report.accepted_steps, report.rejected_steps, report.max_accepted_error_ratio
    );
    assert!(report.accepted_steps > 1);
    assert_eq!(report.exchange.substeps, 2 * report.accepted_steps);
    assert!(report.max_accepted_error_ratio <= 1.);
    assert!((net.total_volume() - totals.0).abs() < 1e-20);
    assert!((net.total_protein() - totals.1).abs() < 1e-18);
    let p = net.pressures();
    let cell_p = tissue
        .body()
        .cell_pore_response_at(tissue.body().positions())
        .unwrap()
        .0;
    for i in 0..2 {
        assert!((p[i] - cell_p[i]).abs() < 1e-12);
    }
    let actualwall = LymphaticWallLaw {
        external_pressure_pa: wall.external_pressure_pa + p[1],
        ..wall
    };
    assert!((p[3] - actualwall.pressure_pa(net.volumes()[3]).unwrap()).abs() < 1e-12);
    for (i, q) in history.iter().enumerate() {
        assert_eq!(*q, net.rates().unwrap()[i].0);
    }
    let before = (
        tissue.body().positions().to_vec(),
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
        history.clone(),
    );
    let failed = AdaptiveTissueExchangeConfig {
        exchange: AdaptiveExchangeConfig {
            max_trials: 1,
            ..cfg.exchange
        },
        ..cfg
    };
    assert!(
        tissue
            .step_mixed_darcy_with_inertial_lymphatic_geometry_adaptive(
                &mut net,
                &[tensor(); 2],
                0.001,
                1e-4,
                &laws,
                &walls,
                &attachments,
                &links,
                1000.,
                &mut history,
                failed,
                1e-12,
                100,
                1e-20,
                1e-19
            )
            .is_err()
    );
    assert_eq!(
        before,
        (
            tissue.body().positions().to_vec(),
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap(),
            history
        )
    );
}

#[test]
fn radial_profiles_couple_to_actual_fem_pressure_and_preserve_atomic_state() {
    use physics::lymph::{LymphaticWallLaw, RadialExchange, profile::RadialPipe};
    let (mut tissue, mut net, laws) = nonlinear_exchange_fixture();
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-6,
        length_m: 0.01,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: -80.,
        active_tension_n_per_m: 0.01,
    };
    let walls = [None, None, None, Some(wall)];
    let attachments = [LymphaticWallAttachment {
        compartment: 3,
        tissue_cell: 1,
    }];
    let radius = wall.radius_m(1e-6).unwrap();
    let mut profiles = [2, 3].map(|edge| RadialExchange {
        edge,
        length_m: 0.005,
        pipe: RadialPipe::new(radius, 1000., 0.001, 64).unwrap(),
    });
    let originals = profiles.clone();
    let totals = (net.total_volume(), net.total_protein());
    let dt = 1e-5;
    tissue
        .step_mixed_darcy_with_radial_profiles(
            &mut net,
            &[tensor(); 2],
            0.001,
            dt,
            &mut profiles,
            100,
            1e-20,
            1e-19,
            Some(&laws),
            Some(&walls),
            &attachments,
        )
        .unwrap();
    let p = net.pressures();
    let rates = net.rates().unwrap();
    let cell_p = tissue
        .body()
        .cell_pore_response_at(tissue.body().positions())
        .unwrap()
        .0;
    for i in 0..2 {
        assert!((p[i] - cell_p[i]).abs() < 1e-12);
    }
    let attached = LymphaticWallLaw {
        external_pressure_pa: wall.external_pressure_pa + p[1],
        ..wall
    };
    assert!((p[3] - attached.pressure_pa(net.volumes()[3]).unwrap()).abs() < 1e-12);
    for (old, current) in originals.iter().zip(&profiles) {
        let e = net.edges()[old.edge];
        let c0 = net.protein_masses()[e.from] / net.volumes()[e.from];
        let c1 = net.protein_masses()[e.to] / net.volumes()[e.to];
        let drive = p[e.from] - p[e.to] + e.pump_head_pa
            - e.reflection
                * (laws[e.from].pressure_pa(c0).unwrap() - laws[e.to].pressure_pa(c1).unwrap());
        let mut independent = old.pipe.clone();
        let (r, _, reaction) = independent
            .step_with_ideal_valve(dt, drive, old.length_m, 1. / e.hydraulic_m3_per_pa_s)
            .unwrap();
        let expected = if reaction > 0. { 0. } else { r.flow_m3_per_s };
        assert!((rates[old.edge].0 - expected).abs() < 1e-25);
        for (a, b) in independent
            .velocities_m_per_s()
            .iter()
            .zip(current.pipe.velocities_m_per_s())
        {
            assert!((a - b).abs() < 1e-14);
        }
    }
    assert!((net.total_volume() - totals.0).abs() < 1e-20);
    assert!((net.total_protein() - totals.1).abs() < 1e-18);
    let histories = |profiles: &[RadialExchange]| {
        profiles
            .iter()
            .map(|p| p.pipe.velocities_m_per_s().to_vec())
            .collect::<Vec<_>>()
    };
    let before = (
        tissue.body().positions().to_vec(),
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
        histories(&profiles),
    );
    assert!(
        tissue
            .step_mixed_darcy_with_radial_profiles(
                &mut net,
                &[tensor(); 2],
                0.001,
                dt,
                &mut profiles,
                1,
                1e-30,
                1e-30,
                Some(&laws),
                Some(&walls),
                &attachments
            )
            .is_err()
    );
    assert_eq!(
        before,
        (
            tissue.body().positions().to_vec(),
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap(),
            histories(&profiles)
        )
    );
    profiles[0].edge = 0;
    assert!(
        tissue
            .step_mixed_darcy_with_radial_profiles(
                &mut net,
                &[tensor(); 2],
                0.001,
                dt,
                &mut profiles,
                100,
                1e-20,
                1e-19,
                Some(&laws),
                Some(&walls),
                &attachments
            )
            .is_err()
    );
}

#[test]
fn adaptive_radial_tissue_controls_full_profiles_and_rolls_back() {
    use physics::lymph::{
        AdaptiveExchangeConfig, LymphaticWallLaw, RadialExchange, profile::RadialPipe,
    };
    let (mut tissue, mut net, laws) = nonlinear_exchange_fixture();
    let wall = LymphaticWallLaw {
        reference_volume_m3: 1e-6,
        length_m: 0.01,
        passive_pressure_scale_pa: 20.,
        stiffening: 2.,
        external_pressure_pa: -80.,
        active_tension_n_per_m: 0.01,
    };
    let walls = [None, None, None, Some(wall)];
    let attachments = [LymphaticWallAttachment {
        compartment: 3,
        tissue_cell: 1,
    }];
    let mut profiles = [2, 3].map(|edge| RadialExchange {
        edge,
        length_m: 0.005,
        pipe: RadialPipe::new(wall.radius_m(1e-6).unwrap(), 1000., 0.001, 32).unwrap(),
    });
    let initial = (tissue.clone(), net.clone(), profiles.clone());
    let totals = (net.total_volume(), net.total_protein());
    let cfg = AdaptiveTissueExchangeConfig {
        exchange: AdaptiveExchangeConfig {
            relative_tolerance: 1e-3,
            absolute_volume_tolerance_m3: 1e-18,
            absolute_protein_tolerance_kg: 1e-17,
            min_step_seconds: 1e-12,
            max_step_seconds: 5e-5,
            max_trials: 10000,
        },
        absolute_position_tolerance_m: 1e-8,
    };
    let report = tissue
        .step_mixed_darcy_with_radial_profiles_adaptive(
            &mut net,
            &[tensor(); 2],
            0.001,
            1e-4,
            &laws,
            &walls,
            &attachments,
            &mut profiles,
            cfg,
            1e-8,
            100,
            1e-20,
            1e-19,
        )
        .unwrap();
    eprintln!(
        "adaptive radial FEM accepted={} rejected={} max_error={}",
        report.accepted_steps, report.rejected_steps, report.max_accepted_error_ratio
    );
    assert!(report.accepted_steps > 1);
    assert_eq!(report.exchange.substeps, 2 * report.accepted_steps);
    assert!(report.max_accepted_error_ratio <= 1.);
    assert!((net.total_volume() - totals.0).abs() < 1e-20);
    assert!((net.total_protein() - totals.1).abs() < 1e-18);
    let p = net.pressures();
    let cell_p = tissue
        .body()
        .cell_pore_response_at(tissue.body().positions())
        .unwrap()
        .0;
    for i in 0..2 {
        assert!((p[i] - cell_p[i]).abs() < 1e-12);
    }
    let attached = LymphaticWallLaw {
        external_pressure_pa: wall.external_pressure_pa + p[1],
        ..wall
    };
    assert!((p[3] - attached.pressure_pa(net.volumes()[3]).unwrap()).abs() < 1e-12);
    let histories = |profiles: &[RadialExchange]| {
        profiles
            .iter()
            .map(|p| p.pipe.velocities_m_per_s().to_vec())
            .collect::<Vec<_>>()
    };
    assert!(histories(&profiles).iter().flatten().all(|v| v.is_finite()));
    let before = (
        tissue.body().positions().to_vec(),
        net.volumes().to_vec(),
        net.protein_masses().to_vec(),
        net.pressures(),
        net.rates().unwrap(),
        histories(&profiles),
    );
    let failed = AdaptiveTissueExchangeConfig {
        exchange: AdaptiveExchangeConfig {
            max_trials: 1,
            ..cfg.exchange
        },
        ..cfg
    };
    assert!(
        tissue
            .step_mixed_darcy_with_radial_profiles_adaptive(
                &mut net,
                &[tensor(); 2],
                0.001,
                1e-4,
                &laws,
                &walls,
                &attachments,
                &mut profiles,
                failed,
                1e-8,
                100,
                1e-20,
                1e-19
            )
            .is_err()
    );
    assert_eq!(
        before,
        (
            tissue.body().positions().to_vec(),
            net.volumes().to_vec(),
            net.protein_masses().to_vec(),
            net.pressures(),
            net.rates().unwrap(),
            histories(&profiles)
        )
    );
    let isolated = AdaptiveTissueExchangeConfig {
        exchange: AdaptiveExchangeConfig {
            relative_tolerance: 0.,
            absolute_volume_tolerance_m3: 1e-8,
            absolute_protein_tolerance_kg: 1e-8,
            ..cfg.exchange
        },
        absolute_position_tolerance_m: 1e-3,
    };
    let mut accepted = Vec::new();
    for velocity_tolerance in [1e-8, 1e-12] {
        let (mut t, mut n, mut radial) = initial.clone();
        let r = t
            .step_mixed_darcy_with_radial_profiles_adaptive(
                &mut n,
                &[tensor(); 2],
                0.001,
                1e-4,
                &laws,
                &walls,
                &attachments,
                &mut radial,
                isolated,
                velocity_tolerance,
                100,
                1e-20,
                1e-19,
            )
            .unwrap();
        eprintln!(
            "isolated velocity tolerance={velocity_tolerance} accepted={} rejected={}",
            r.accepted_steps, r.rejected_steps
        );
        accepted.push(r.accepted_steps);
    }
    assert!(accepted[1] > accepted[0]);
}
