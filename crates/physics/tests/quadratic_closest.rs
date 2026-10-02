use physics::plasticity::mesh::{QuadraticClosestLimits, QuadraticFace};
fn face() -> QuadraticFace {
    QuadraticFace {
        nodes: [0, 1, 2, 3, 4, 5],
        normal: [0., 0., 1.],
        reference_area_m2: 0.5,
    }
}
fn surface(f: impl Fn(f64, f64) -> [f64; 3]) -> Vec<[f64; 3]> {
    [
        (0., 0.),
        (1., 0.),
        (0., 1.),
        (0.5, 0.),
        (0.5, 0.5),
        (0., 0.5),
    ]
    .into_iter()
    .map(|(u, v)| f(u, v))
    .collect()
}
fn limits() -> QuadraticClosestLimits {
    QuadraticClosestLimits {
        distance_tolerance_m: 1e-6,
        max_patches: 32768,
        max_depth: 24,
    }
}
#[test]
fn plane_interior_edge_corner_and_observer_rotation() {
    let p = surface(|u, v| [u, v, 0.]);
    for (query, expected) in [
        ([0.2, 0.3, 0.4], [0.2, 0.3, 0.]),
        ([0.6, 0.6, 0.2], [0.5, 0.5, 0.]),
        ([-0.2, -0.3, 0.1], [0., 0., 0.]),
    ] {
        let r = face().closest_point_at(&p, query, limits()).unwrap();
        assert!(
            r.converged,
            "{} {} {}",
            r.distance_m, r.lower_distance_m, r.patches
        );
        for i in 0..3 {
            assert!((r.point_m[i] - expected[i]).abs() < 2e-6);
        }
        let transform = |p: [f64; 3]| {
            [
                3. + 0.6 * p[0] - 0.8 * p[2],
                -2. + p[1],
                1. + 0.8 * p[0] + 0.6 * p[2],
            ]
        };
        let rotated: Vec<_> = p.iter().copied().map(transform).collect();
        let s = face()
            .closest_point_at(&rotated, transform(query), limits())
            .unwrap();
        assert!(
            s.converged,
            "{} {} {}",
            s.distance_m, s.lower_distance_m, s.patches
        );
        assert!((r.distance_m - s.distance_m).abs() < 2e-6);
        for i in 0..3 {
            assert!((s.point_m[i] - transform(expected)[i]).abs() < 2e-6);
        }
    }
}
#[test]
fn curved_interior_and_competing_minima_have_global_distance_bounds() {
    let p = surface(|u, v| [u, v, (u - 0.2).powi(2) + (v - 0.3).powi(2)]);
    let r = face()
        .closest_point_at(&p, [0.2, 0.3, -0.2], limits())
        .unwrap();
    assert!(r.converged);
    assert!(r.lower_distance_m <= 0.2 + 1e-12 && r.distance_m >= 0.2 - 1e-12);
    assert!((r.distance_m - 0.2).abs() < 1e-8);
    // The center is a stationary local maximum of distance along u. There are
    // two minima at u=0.5 +/- sqrt(0.11875), v=0.05.
    let p = surface(|u, v| [u, v, 4. * (u - 0.5).powi(2)]);
    let r = face()
        .closest_point_at(&p, [0.5, 0.05, 0.6], limits())
        .unwrap();
    let exact = 0.134375_f64.sqrt();
    assert!(
        r.converged,
        "{} {} {}",
        r.distance_m, r.lower_distance_m, r.patches
    );
    assert!(r.lower_distance_m <= exact + 1e-12 && r.distance_m >= exact - 1e-12);
    assert!((r.distance_m - exact).abs() < 1e-8);
    assert!(((r.point_m[0] - 0.5).abs() - 0.11875_f64.sqrt()).abs() < 1e-6);
}
#[test]
fn exhausted_budget_is_reported_and_invalid_queries_are_rejected() {
    let p = surface(|u, v| [u, v, 4. * (u - 0.5).powi(2)]);
    let r = face()
        .closest_point_at(
            &p,
            [0.5, 0.05, 0.6],
            QuadraticClosestLimits {
                max_patches: 1,
                max_depth: 0,
                ..limits()
            },
        )
        .unwrap();
    assert!(!r.converged);
    let exact = 0.134375_f64.sqrt();
    assert!(r.lower_distance_m <= exact + 1e-12 && r.distance_m >= exact - 1e-12);
    assert!(
        face()
            .closest_point_at(&p, [f64::NAN, 0., 0.], limits())
            .is_err()
    );
    assert!(
        face()
            .closest_point_at(
                &p,
                [0.; 3],
                QuadraticClosestLimits {
                    distance_tolerance_m: 0.,
                    ..limits()
                }
            )
            .is_err()
    );
    assert!(
        face()
            .closest_point_at(&[[0.; 3]; 6], [0., 0., 1.], limits())
            .is_err()
    );
    assert!(face().closest_point_at(&[], [0.; 3], limits()).is_err());
}

#[test]
fn unique_curved_projection_distance_gradient_matches_virtual_work() {
    let points = surface(|u, v| [u, v, u * u + 0.5 * v * v]);
    let length = 1.25_f64.sqrt();
    let normal = [-0.4 / length, -0.3 / length, 1. / length];
    let expected = [0.2, 0.3, 0.085];
    let query = std::array::from_fn(|i| expected[i] - 0.2 * normal[i]);
    let settings = QuadraticClosestLimits {
        distance_tolerance_m: 1e-9,
        ..limits()
    };
    let r = face().closest_point_at(&points, query, settings).unwrap();
    assert!(r.converged);
    assert!((r.distance_m - 0.2).abs() < 1e-12);
    let h = 1e-6;
    for node in 0..6 {
        for axis in 0..3 {
            let mut plus = points.clone();
            let mut minus = points.clone();
            plus[node][axis] += h;
            minus[node][axis] -= h;
            let a = face().closest_point_at(&plus, query, settings).unwrap();
            let b = face().closest_point_at(&minus, query, settings).unwrap();
            assert!(a.converged && b.converged);
            let derivative = (a.distance_m - b.distance_m) / (2. * h);
            assert!(
                (derivative - r.shape_weights[node] * normal[axis]).abs() < 1e-5,
                "node {node} axis {axis}: {derivative}"
            );
        }
    }
}
