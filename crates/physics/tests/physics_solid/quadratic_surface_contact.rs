use physics::plasticity::mesh::{QuadraticClosestLimits, QuadraticFace, QuadraticSurfaceContact};
fn pair(curved: bool, gap: f64) -> (Vec<[f64; 3]>, QuadraticFace, QuadraticFace) {
    let mut positions = Vec::new();
    for z in [0., gap] {
        for (u, v) in [
            (0., 0.),
            (1., 0.),
            (0., 1.),
            (0.5, 0.),
            (0.5, 0.5),
            (0., 0.5),
        ] {
            positions.push([
                u,
                v,
                z + if curved {
                    0.1 * u * u + 0.05 * v * v
                } else {
                    0.
                },
            ]);
        }
    }
    (
        positions,
        QuadraticFace {
            nodes: [0, 1, 2, 3, 4, 5],
            normal: [0., 0., 1.],
            reference_area_m2: 0.5,
        },
        QuadraticFace {
            nodes: [6, 7, 8, 9, 10, 11],
            normal: [0., 0., -1.],
            reference_area_m2: 0.5,
        },
    )
}
fn law() -> QuadraticSurfaceContact {
    QuadraticSurfaceContact::new(
        0.02,
        1000.,
        QuadraticClosestLimits {
            distance_tolerance_m: 1e-10,
            max_patches: 32768,
            max_depth: 24,
        },
    )
    .unwrap()
}
#[test]
fn parallel_layers_match_analytic_energy_and_equal_opposite_resultants() {
    let (p, a, b) = pair(false, 0.01);
    let r = law().evaluate(&p, a, b).unwrap();
    assert_eq!(r.active_samples, 12);
    assert!((r.energy_j - 0.025).abs() < 1e-12);
    let bottom: f64 = r.forces_n[..6].iter().map(|f| f[2]).sum();
    let top: f64 = r.forces_n[6..].iter().map(|f| f[2]).sum();
    assert!((bottom + 5.).abs() < 1e-10 && (top - 5.).abs() < 1e-10);
    let (p, a, b) = pair(false, 0.03);
    let r = law().evaluate(&p, a, b).unwrap();
    assert_eq!(r.energy_j, 0.);
    assert_eq!(r.active_samples, 0);
    assert!(r.forces_n.iter().flatten().all(|f| *f == 0.));
}
#[test]
fn curved_pair_forces_match_energy_and_preserve_force_torque_and_objectivity() {
    let (p, a, b) = pair(true, 0.01);
    let r = law().evaluate(&p, a, b).unwrap();
    let h = 1e-6;
    for node in 0..12 {
        for axis in 0..3 {
            let mut plus = p.clone();
            let mut minus = p.clone();
            plus[node][axis] += h;
            minus[node][axis] -= h;
            let gradient = (law().evaluate(&plus, a, b).unwrap().energy_j
                - law().evaluate(&minus, a, b).unwrap().energy_j)
                / (2. * h);
            assert!(
                (gradient + r.forces_n[node][axis]).abs() < 1e-5,
                "node {node} axis {axis}: {gradient} {}",
                r.forces_n[node][axis]
            );
        }
    }
    for axis in 0..3 {
        let force: f64 = r.forces_n.iter().map(|f| f[axis]).sum();
        let u = (axis + 1) % 3;
        let v = (axis + 2) % 3;
        let torque: f64 = p
            .iter()
            .zip(&r.forces_n)
            .map(|(p, f)| p[u] * f[v] - p[v] * f[u])
            .sum();
        assert!(force.abs() < 1e-10 && torque.abs() < 1e-10);
    }
    let swap = law().evaluate(&p, b, a).unwrap();
    assert!((r.energy_j - swap.energy_j).abs() < 1e-12);
    let rotate = |p: [f64; 3]| [0.6 * p[0] - 0.8 * p[2], p[1], 0.8 * p[0] + 0.6 * p[2]];
    let moved: Vec<_> = p
        .iter()
        .copied()
        .map(|p| {
            let q = rotate(p);
            [q[0] + 2., q[1] - 3., q[2] + 4.]
        })
        .collect();
    let s = law().evaluate(&moved, a, b).unwrap();
    assert!((r.energy_j - s.energy_j).abs() < 1e-10);
    for (f, g) in r.forces_n.iter().zip(&s.forces_n) {
        let expected = rotate(*f);
        for i in 0..3 {
            assert!((expected[i] - g[i]).abs() < 1e-6);
        }
    }
}
#[test]
fn singular_intersection_and_unresolved_projection_are_explicit_errors() {
    let (p, a, b) = pair(false, 0.);
    assert!(law().evaluate(&p, a, b).is_err());
    assert!(law().evaluate(&p, a, a).is_err());
    let (p, a, b) = pair(true, 0.01);
    let low = QuadraticSurfaceContact::new(
        0.02,
        1000.,
        QuadraticClosestLimits {
            distance_tolerance_m: 1e-10,
            max_patches: 1,
            max_depth: 0,
        },
    )
    .unwrap();
    assert!(low.evaluate(&p, a, b).is_err());
}

#[test]
fn split_target_surface_does_not_duplicate_contact_pressure() {
    let (mut positions, source, target) = pair(false, 0.01);
    let whole = law().evaluate(&positions, source, target).unwrap();
    // Two T6 subtriangles have the same union as the original target triangle.
    let mut targets = Vec::new();
    for corners in [
        [[0., 0.], [0.5, 0.], [0., 1.]],
        [[0.5, 0.], [1., 0.], [0., 1.]],
    ] {
        let start = positions.len();
        for l in [
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0.5, 0.5, 0.],
            [0., 0.5, 0.5],
            [0.5, 0., 0.5],
        ] {
            positions.push([
                corners.iter().zip(l).map(|(p, w)| p[0] * w).sum(),
                corners.iter().zip(l).map(|(p, w)| p[1] * w).sum(),
                0.01,
            ]);
        }
        targets.push(QuadraticFace {
            nodes: std::array::from_fn(|i| start + i),
            normal: [0., 0., -1.],
            reference_area_m2: 0.25,
        });
    }
    let split = law()
        .evaluate_surfaces(&positions, &[source], &targets)
        .unwrap();
    assert!((split.energy_j - whole.energy_j).abs() < 1e-12);
    let resultant: f64 = split.forces_n[..6].iter().map(|f| f[2]).sum();
    assert!((resultant + 5.).abs() < 1e-10);
    assert_eq!(split.active_samples, 18);
}

fn planar_contact_reference(divisions: u32) -> (f64, f64) {
    let closest = |q: [f64; 3], tilted: bool| {
        let corners = if tilted {
            [[0., 0., 0.007], [1., 0., 0.038], [0., 1., 0.007]]
        } else {
            [[0.; 3], [1., 0., 0.], [0., 1., 0.]]
        };
        let slope = if tilted { 0.031 } else { 0. };
        let offset = if tilted { 0.007 } else { 0. };
        let signed = (q[2] - offset - slope * q[0]) / (1. + slope * slope);
        let projected = [q[0] + slope * signed, q[1], q[2] - signed];
        if projected[0] >= 0. && projected[1] >= 0. && projected[0] + projected[1] <= 1. {
            return projected;
        }
        let mut best = corners[0];
        let mut distance = f64::INFINITY;
        for (a, b) in [(0, 1), (1, 2), (0, 2)] {
            let edge: [f64; 3] = std::array::from_fn(|i| corners[b][i] - corners[a][i]);
            let dot: f64 = (0..3).map(|i| (q[i] - corners[a][i]) * edge[i]).sum();
            let norm: f64 = edge.iter().map(|x| x * x).sum();
            let t = (dot / norm).clamp(0., 1.);
            let p: [f64; 3] = std::array::from_fn(|i| corners[a][i] + t * edge[i]);
            let d: f64 = (0..3).map(|i| (q[i] - p[i]).powi(2)).sum();
            if d < distance {
                distance = d;
                best = p;
            }
        }
        best
    };
    let mut energy = 0.;
    let mut force = 0.;
    let n = f64::from(divisions);
    for i in 0..divisions {
        for j in 0..divisions - i {
            for fraction in [1. / 3., 2. / 3.] {
                if fraction > 0.5 && i + j + 1 >= divisions {
                    continue;
                }
                let u = (f64::from(i) + fraction) / n;
                let v = (f64::from(j) + fraction) / n;
                for source_upper in [false, true] {
                    let q = [u, v, if source_upper { 0.007 + 0.031 * u } else { 0. }];
                    let p = closest(q, !source_upper);
                    let delta: [f64; 3] = std::array::from_fn(|i| q[i] - p[i]);
                    let distance = delta[0].hypot(delta[1]).hypot(delta[2]);
                    let penetration = (0.02 - distance).max(0.);
                    let measure = 0.25 / (n * n);
                    energy += 0.5 * 1000. * measure * penetration * penetration;
                    force += if source_upper { -1. } else { 1. }
                        * 1000.
                        * measure
                        * penetration
                        * delta[2]
                        / distance;
                }
            }
        }
    }
    (energy, force)
}
#[test]
fn partially_active_contact_refines_against_independent_planar_integration() {
    let (mut p, a, b) = pair(false, 0.01);
    for point in &mut p[6..] {
        point[2] = 0.007 + 0.031 * point[0];
    }
    let reference = planar_contact_reference(1024);
    let previous = planar_contact_reference(512);
    assert!((reference.0 - previous.0).abs() < 1e-7);
    assert!((reference.1 - previous.1).abs() < 1e-4);
    let mut errors = Vec::new();
    for depth in [0, 1, 2, 3, 4, 5] {
        let r = law()
            .with_integration_depth(depth)
            .unwrap()
            .evaluate(&p, a, b)
            .unwrap();
        let force: f64 = r.forces_n[..6].iter().map(|f| f[2]).sum();
        errors.push((
            (r.energy_j - reference.0).abs(),
            (force - reference.1).abs(),
        ));
    }
    println!("partial surface quadrature errors: {errors:?}; reference {reference:?}");
    assert!(errors[3].0 < errors[0].0 / 8. && errors[3].0 < 1e-5);
    assert!(errors[3].1 < errors[0].1 / 8. && errors[3].1 < 1e-3);
    assert!(errors[5].0 < 1e-7 && errors[5].1 < 1e-4);
    let refined = law().with_integration_depth(3).unwrap();
    let h = 1e-6;
    let mut plus = p.clone();
    let mut minus = p.clone();
    for point in &mut plus[6..] {
        point[2] += h;
    }
    for point in &mut minus[6..] {
        point[2] -= h;
    }
    let gradient = (refined.evaluate(&plus, a, b).unwrap().energy_j
        - refined.evaluate(&minus, a, b).unwrap().energy_j)
        / (2. * h);
    let r = refined.evaluate(&p, a, b).unwrap();
    let top_force: f64 = r.forces_n[6..].iter().map(|f| f[2]).sum();
    assert!((gradient + top_force).abs() < 1e-6);
    assert!(law().with_integration_depth(6).is_err());
}
