use physics::plasticity::{Material, mesh::Body};
const E: f64 = 210e9;
const NU: f64 = 0.3;
const Y: f64 = 250e6;
const H: f64 = 1e9;
fn coupon() -> Body {
    Body::new(
        vec![[0.; 3], [0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]],
        vec![([0, 1, 2, 3], Material::new(E, NU, Y, H).unwrap())],
    )
    .unwrap()
}
fn supports() -> [[Option<f64>; 3]; 4] {
    [
        [Some(0.); 3],
        [None, Some(0.), Some(0.)],
        [Some(0.), None, Some(0.)],
        [Some(0.), Some(0.), None],
    ]
}
fn close(a: f64, b: f64, relative: f64) {
    assert!((a - b).abs() <= relative * b.abs().max(1e-12), "{a} != {b}");
}
#[test]
fn uniaxial_coupon_yields_and_keeps_residual_extension_when_force_is_removed() {
    let mut body = coupon();
    let mut constraints = supports();
    let strain = 0.004;
    constraints[1][0] = Some(strain * 0.1);
    let loaded = body
        .equilibrate(&[[0.; 3]; 4], &constraints, 30, 1e-5)
        .unwrap();
    assert!(loaded.converged, "{loaded:?}");
    let alpha = (E * strain - Y) / (E + H);
    let stress = Y + H * alpha;
    let response = body.responses().unwrap()[0];
    close(response.stress.cauchy_pa[0][0], stress, 1e-10);
    assert!(response.stress.cauchy_pa[1][1].abs() < 0.01);
    assert!(response.stress.cauchy_pa[2][2].abs() < 0.01);
    close(body.states()[0].equivalent_plastic_strain(), alpha, 1e-10);
    close(loaded.reactions_n[1][0], stress * 0.1 * 0.1 / 6., 1e-10);
    close(
        body.positions()[2][1] - 0.1,
        0.1 * (-NU * stress / E - alpha / 2.),
        1e-10,
    );
    constraints[1][0] = None;
    let unloaded = body
        .equilibrate(&[[0.; 3]; 4], &constraints, 30, 1e-5)
        .unwrap();
    assert!(unloaded.converged, "{unloaded:?}");
    close(body.positions()[1][0] - 0.1, alpha * 0.1, 1e-10);
    assert!(body.responses().unwrap()[0].stress.von_mises_pa < 0.01);
    close(body.states()[0].equivalent_plastic_strain(), alpha, 1e-10);
    assert!(body.states()[0].dissipated_j_m3() > 0.);
}
#[test]
fn force_driven_equilibrium_and_reactions_match_analytical_solution() {
    let mut body = coupon();
    let stress = 255e6;
    let mut loads = [[0.; 3]; 4];
    loads[1][0] = stress * 0.1 * 0.1 / 6.;
    let result = body.equilibrate(&loads, &supports(), 40, 1e-5).unwrap();
    assert!(result.converged, "{result:?}");
    let alpha = (stress - Y) / H;
    close(
        (body.positions()[1][0] - 0.1) / 0.1,
        stress / E + alpha,
        1e-10,
    );
    close(body.states()[0].equivalent_plastic_strain(), alpha, 1e-10);
    close(result.reactions_n[0][0], -loads[1][0], 1e-10);
}
#[test]
fn rejected_steps_and_missing_supports_never_commit_trial_history() {
    let mut body = coupon();
    let positions = body.positions().to_vec();
    let states = body.states();
    let mut loads = [[0.; 3]; 4];
    loads[1][0] = 255e6 * 0.1 * 0.1 / 6.;
    let result = body.equilibrate(&loads, &supports(), 1, 1e-5).unwrap();
    assert!(!result.converged);
    assert_eq!(body.positions(), positions);
    assert_eq!(body.states(), states);
    assert!(body.equilibrate(&loads, &[[None; 3]; 4], 10, 1e-5).is_err());
    assert_eq!(body.positions(), positions);
    assert_eq!(body.states(), states);
    loads[1][0] = f64::NAN;
    assert!(body.equilibrate(&loads, &supports(), 10, 1e-5).is_err());
    assert_eq!(body.positions(), positions);
    assert_eq!(body.states(), states);
}

#[test]
fn multi_element_affine_patch_and_refinement_preserve_stress_and_total_reaction() {
    for subdivisions in [1_usize, 2] {
        let width = subdivisions + 1;
        let id = |i: usize, j: usize, k: usize| (i * width + j) * width + k;
        let mut points = Vec::new();
        let mut constraints = Vec::new();
        for i in 0..width {
            for j in 0..width {
                for k in 0..width {
                    points.push([i, j, k].map(|v| {
                        f64::from(u32::try_from(v).unwrap()) * 0.1
                            / f64::from(u32::try_from(subdivisions).unwrap())
                    }));
                    constraints.push([
                        if i == 0 {
                            Some(0.)
                        } else if i == subdivisions {
                            Some(0.0004)
                        } else {
                            None
                        },
                        None,
                        None,
                    ]);
                }
            }
        }
        constraints[id(0, 0, 0)][1] = Some(0.);
        constraints[id(0, 0, 0)][2] = Some(0.);
        constraints[id(0, subdivisions, 0)][2] = Some(0.);
        let mut cells = Vec::new();
        let material = Material::new(E, NU, Y, H).unwrap();
        for i in 0..subdivisions {
            for j in 0..subdivisions {
                for k in 0..subdivisions {
                    let vertices = [
                        id(i, j, k),
                        id(i + 1, j, k),
                        id(i, j + 1, k),
                        id(i + 1, j + 1, k),
                        id(i, j, k + 1),
                        id(i + 1, j, k + 1),
                        id(i, j + 1, k + 1),
                        id(i + 1, j + 1, k + 1),
                    ];
                    for tet in [
                        [0, 1, 3, 7],
                        [0, 3, 2, 7],
                        [0, 2, 6, 7],
                        [0, 6, 4, 7],
                        [0, 4, 5, 7],
                        [0, 5, 1, 7],
                    ] {
                        cells.push((tet.map(|v| vertices[v]), material));
                    }
                }
            }
        }
        let mut body = Body::new(points.clone(), cells).unwrap();
        let result = body
            .equilibrate(&vec![[0.; 3]; points.len()], &constraints, 40, 1e-4)
            .unwrap();
        assert!(result.converged, "grid={subdivisions}, {result:?}");
        let alpha = (E * 0.004 - Y) / (E + H);
        let expected = Y + H * alpha;
        for response in body.responses().unwrap() {
            close(response.stress.cauchy_pa[0][0], expected, 1e-9);
            assert!(response.stress.cauchy_pa[1][1].abs() < 0.1);
        }
        let reaction: f64 = result
            .reactions_n
            .iter()
            .enumerate()
            .filter(|(i, _)| *i / (width * width) == subdivisions)
            .map(|(_, r)| r[0])
            .sum();
        close(reaction, expected * 0.01, 1e-9);
    }
}

#[test]
fn microscopic_and_large_mesh_scales_preserve_dimensionless_response() {
    for length in [1e-9, 1e-3, 1e3] {
        let material = Material::new(E, NU, Y, H).unwrap();
        let mut body = Body::new(
            vec![
                [0.; 3],
                [length, 0., 0.],
                [0., length, 0.],
                [0., 0., length],
            ],
            vec![([0, 1, 2, 3], material)],
        )
        .unwrap();
        let mut constraints = supports();
        constraints[1][0] = Some(0.004 * length);
        let result = body
            .equilibrate(&[[0.; 3]; 4], &constraints, 40, Y * length * length * 1e-10)
            .unwrap();
        assert!(result.converged, "length={length}, {result:?}");
        close(
            body.states()[0].equivalent_plastic_strain(),
            (E * 0.004 - Y) / (E + H),
            1e-8,
        );
    }
}
