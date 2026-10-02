use physics::{
    biomechanics::Material as Elastic,
    plasticity::{
        Material,
        mesh::{FiniteQuadraticDynamics, QuadraticBody, QuadraticPlaneContact},
    },
};
fn mesh(scale: f64, height: f64) -> QuadraticBody {
    QuadraticBody::from_linear(
        vec![
            [0., height, 0.],
            [scale, height, 0.],
            [0., height + scale, 0.],
            [0., height, scale],
        ],
        vec![([0, 1, 2, 3], Material::new(1e5, 0.3, 1e9, 0.).unwrap())],
    )
    .unwrap()
}
#[test]
fn surface_contact_force_is_energy_gradient_and_integrates_linear_gap() {
    let mesh = mesh(1., 0.);
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 2., 1000.).unwrap();
    let positions = mesh.positions().to_vec();
    let response = mesh.plane_contact_at(&positions, plane).unwrap();
    let faces = mesh.reference_faces().unwrap();
    let expected: f64 = faces
        .iter()
        .map(|face| {
            let mean_y = face.nodes[..3]
                .iter()
                .map(|&i| positions[i][1])
                .sum::<f64>()
                / 3.;
            1000. * face.reference_area_m2 * (2. - mean_y)
        })
        .sum();
    assert!((response.force_n[1] - expected).abs() < 1e-9);
    for node in 0..positions.len() {
        for axis in 0..3 {
            let mut plus = positions.clone();
            let mut minus = positions.clone();
            plus[node][axis] += 1e-6;
            minus[node][axis] -= 1e-6;
            let derivative = (mesh.plane_contact_at(&plus, plane).unwrap().energy_j
                - mesh.plane_contact_at(&minus, plane).unwrap().energy_j)
                / 2e-6;
            assert!(
                (derivative - response.gradient_n[node][axis]).abs() < 1e-5,
                "node={node}, axis={axis}"
            );
        }
    }
}
fn impact(dt: f64, steps: usize) -> f64 {
    let mesh = mesh(0.1, 0.01);
    let mut body = FiniteQuadraticDynamics::new(
        mesh,
        vec![Elastic::from_young_poisson(1e5, 0.3).unwrap()],
        &[1000.],
        vec![[0., -1., 0.]; 10],
        &[false; 10],
    )
    .unwrap();
    assert_eq!(
        body.set_plane_contact(Some(
            QuadraticPlaneContact::new([0., 1., 0.], 0., 1e6).unwrap()
        ))
        .unwrap(),
        0.
    );
    let initial = body.energy().unwrap();
    let mut old = initial.clone();
    let mut impulse = 0.;
    let mut worst = 0_f64;
    let mut peak = 0_f64;
    for _ in 0..steps {
        body.step(dt, [0.; 3], 1e-5).unwrap();
        let new = body.energy().unwrap();
        impulse += 0.5 * dt * (old.contact_force_n[1] + new.contact_force_n[1]);
        worst = worst.max(
            (new.kinetic_j + new.elastic_j + new.contact_j - initial.kinetic_j).abs()
                / initial.kinetic_j,
        );
        peak = peak.max(new.contact_force_n[1]);
        assert!((new.momentum_kg_m_s[1] - initial.momentum_kg_m_s[1] - impulse).abs() < 1e-9);
        assert!(new.momentum_kg_m_s[0].abs() < 1e-10 && new.momentum_kg_m_s[2].abs() < 1e-10);
        old = new;
    }
    println!(
        "surface impact relative energy envelope={worst:e}, peak force={peak:e}, final vertical momentum={:e}",
        old.momentum_kg_m_s[1]
    );
    assert!(worst < 1e-3 && peak > 0. && old.momentum_kg_m_s[1] > 0.);
    let positions = body.positions().to_vec();
    let velocities = body.velocities().to_vec();
    assert!(body.step(0.1, [0.; 3], 1e-15).is_err());
    assert_eq!(body.positions(), positions);
    assert_eq!(body.velocities(), velocities);
    assert!(
        body.set_plane_contact(Some(
            QuadraticPlaneContact::new([0., 1., 0.], 1e300, 1e300).unwrap()
        ))
        .is_err()
    );
    assert_eq!(body.energy().unwrap().contact_j, old.contact_j);
    worst
}
#[test]
fn finite_surface_impact_preserves_energy_and_balances_wall_impulse() {
    let coarse = impact(2e-5, 3000);
    let fine = impact(1e-5, 6000);
    assert!(fine < coarse / 2., "coarse={coarse:e}, fine={fine:e}");
}

#[test]
fn contact_covaries_with_plane_frame_and_parameter_work_is_reversible() {
    let mesh = mesh(1., 0.);
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 0.4, 1000.).unwrap();
    let original = mesh.plane_contact_at(mesh.positions(), plane).unwrap();
    // Rotate 90 degrees about z, then translate; transform the plane too.
    let rotated: Vec<_> = mesh
        .positions()
        .iter()
        .map(|p| [2. - p[1], 3. + p[0], p[2] - 1.])
        .collect();
    let moved_plane = QuadraticPlaneContact::new([-1., 0., 0.], -1.6, 1000.).unwrap();
    let moved = mesh.plane_contact_at(&rotated, moved_plane).unwrap();
    assert!((moved.energy_j - original.energy_j).abs() < 1e-10);
    assert!((moved.minimum_surface_gap_m - original.minimum_surface_gap_m).abs() < 1e-12);
    for (a, b) in original.gradient_n.iter().zip(moved.gradient_n) {
        assert!(
            (b[0] + a[1]).abs() < 1e-10
                && (b[1] - a[0]).abs() < 1e-10
                && (b[2] - a[2]).abs() < 1e-10
        );
    }
    let mut body = FiniteQuadraticDynamics::new(
        mesh,
        vec![Elastic::from_young_poisson(1e5, 0.3).unwrap()],
        &[1000.],
        vec![[0.; 3]; 10],
        &[false; 10],
    )
    .unwrap();
    let installed = body.set_plane_contact(Some(plane)).unwrap();
    assert!((installed - original.energy_j).abs() < 1e-10);
    let removed = body.set_plane_contact(None).unwrap();
    assert_eq!(installed, -removed);
    assert_eq!(body.energy().unwrap().contact_j, 0.);
}

#[test]
fn surface_minimum_detects_penetration_between_contact_samples() {
    let mesh = mesh(1., 0.);
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 0., 1000.).unwrap();
    let positions: Vec<_> = mesh
        .positions()
        .iter()
        .map(|p| {
            [
                p[0],
                (p[0] - 0.2).powi(2) + (p[2] - 0.2).powi(2) - 0.001,
                p[2],
            ]
        })
        .collect();
    let response = mesh
        .plane_contact_at(&positions, plane.with_refinement_depth(0).unwrap())
        .unwrap();
    assert!(response.minimum_sample_gap_m > 0.);
    assert!((response.minimum_surface_gap_m + 0.001).abs() < 1e-14);
    // The diagnostic exposes a real limitation: depth-zero six-point force
    // quadrature misses this active patch. Do not claim collision resolution.
    assert_eq!(response.energy_j, 0.);
}

#[test]
fn refined_contact_resolves_small_patch_and_converges_to_analytic_integral() {
    let mesh = mesh(1., 0.);
    let positions: Vec<_> = mesh
        .positions()
        .iter()
        .map(|p| {
            [
                p[0],
                (p[0] - 0.2).powi(2) + (p[2] - 0.2).powi(2) - 0.001,
                p[2],
            ]
        })
        .collect();
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 0., 1000.).unwrap();
    // Two faces project onto the same x-z triangle, with reference area factors
    // 1 and sqrt(3). Integrate the circular active patch in polar coordinates.
    let expected_energy = 1000. * std::f64::consts::PI * 1e-9 / 6. * (1. + 3_f64.sqrt());
    let expected_force = 1000. * std::f64::consts::PI * 1e-6 / 2. * (1. + 3_f64.sqrt());
    let mut last_error = f64::INFINITY;
    for depth in [4, 6, 8] {
        let response = mesh
            .plane_contact_at(&positions, plane.with_refinement_depth(depth).unwrap())
            .unwrap();
        let error = (response.energy_j - expected_energy).abs();
        println!(
            "depth={depth}, relative energy error={}, bound={}",
            error / expected_energy,
            response.partial_energy_error_bound_j
        );
        assert!(response.energy_j > 0. && response.force_n[1] > 0.);
        assert!(error <= response.partial_energy_error_bound_j + 1e-18);
        assert!(error < last_error);
        last_error = error;
        if depth == 8 {
            for node in 0..positions.len() {
                let mut plus = positions.clone();
                let mut minus = positions.clone();
                plus[node][1] += 1e-7;
                minus[node][1] -= 1e-7;
                let energy_difference = (mesh
                    .plane_contact_at(&plus, plane.with_refinement_depth(8).unwrap())
                    .unwrap()
                    .energy_j
                    - mesh
                        .plane_contact_at(&minus, plane.with_refinement_depth(8).unwrap())
                        .unwrap()
                        .energy_j)
                    / 2e-7;
                assert!(
                    (energy_difference - response.gradient_n[node][1]).abs() < 1e-6,
                    "node={node}"
                );
            }
            assert!(error / expected_energy < 1e-5);
            assert!((response.force_n[1] - expected_force).abs() / expected_force < 1e-4);
        }
    }
    assert!(plane.with_refinement_depth(9).is_err());
}
