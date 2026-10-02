use physics::plasticity::mesh::{
    QuadraticClosestLimits, QuadraticFace, QuadraticSweep, QuadraticSweepLimits,
};
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
fn limits() -> QuadraticSweepLimits {
    QuadraticSweepLimits {
        closest: QuadraticClosestLimits {
            distance_tolerance_m: 1e-7,
            max_patches: 8192,
            max_depth: 24,
        },
        minimum_time_fraction: 1e-10,
        max_intervals: 2048,
    }
}
#[test]
fn crossing_is_found_between_separated_endpoints() {
    let p = surface(|u, v| [u, v, 0.]);
    let start = [0.2, 0.3, 0.5];
    let end = [0.2, 0.3, -1.5];
    for query in [start, end] {
        assert!(
            face()
                .closest_point_at(&p, query, limits().closest)
                .unwrap()
                .distance_m
                > 0.4
        );
    }
    let r = face()
        .swept_point_clearance_at(&p, &p, start, end, 1e-5, limits())
        .unwrap();
    let QuadraticSweep::WithinClearance {
        time_fraction,
        closest,
        ..
    } = r
    else {
        panic!("missing crossing: {r:?}");
    };
    assert!((time_fraction - 0.25).abs() < 1e-5);
    assert!(closest.distance_m <= 1e-5);
    assert!((closest.point_m[0] - 0.2).abs() < 1e-6);
    // Endpoint target motion, rather than point motion, crosses a stationary point.
    let a = surface(|u, v| [u, v, -0.5]);
    let b = surface(|u, v| [u, v, 0.5]);
    assert!(matches!(
        face()
            .swept_point_clearance_at(&a, &b, [0.2, 0.3, 0.], [0.2, 0.3, 0.], 1e-5, limits())
            .unwrap(),
        QuadraticSweep::WithinClearance { .. }
    ));
}
#[test]
fn curved_nondyadic_crossing_and_common_observer_motion() {
    let p = surface(|u, v| [u, v, 0.1 * u * u + 0.2 * v * v]);
    let start = [0.2, 0.3, 0.2];
    let end = [0.2, 0.3, -0.2];
    let r = face()
        .swept_point_clearance_at(&p, &p, start, end, 1e-5, limits())
        .unwrap();
    let QuadraticSweep::WithinClearance {
        time_fraction,
        closest,
        ..
    } = r
    else {
        panic!("missing curved crossing: {r:?}");
    };
    let exact = (0.2 - 0.022) / 0.4;
    assert!((time_fraction - exact).abs() < 3e-5);
    assert!(closest.distance_m <= 1e-5);
    let transform = |p: [f64; 3], t: f64| {
        [
            0.6 * p[0] - 0.8 * p[2] + 1000. * t,
            p[1] - 5.,
            0.8 * p[0] + 0.6 * p[2] + 2.,
        ]
    };
    let first: Vec<_> = p.iter().copied().map(|p| transform(p, 0.)).collect();
    let last: Vec<_> = p.iter().copied().map(|p| transform(p, 1.)).collect();
    let r = face()
        .swept_point_clearance_at(
            &first,
            &last,
            transform(start, 0.),
            transform(end, 1.),
            1e-5,
            limits(),
        )
        .unwrap();
    let QuadraticSweep::WithinClearance { time_fraction, .. } = r else {
        panic!("observer changed crossing: {r:?}");
    };
    assert!((time_fraction - exact).abs() < 3e-5);
}
#[test]
fn separation_budget_exhaustion_and_invalid_queries_are_distinct() {
    let p = surface(|u, v| [u, v, 0.]);
    assert!(matches!(
        face()
            .swept_point_clearance_at(&p, &p, [2., 2., 0.5], [2., 2., -1.5], 1e-5, limits())
            .unwrap(),
        QuadraticSweep::Separated { .. }
    ));
    let r = face()
        .swept_point_clearance_at(
            &p,
            &p,
            [0.2, 0.3, 0.5],
            [0.2, 0.3, -1.5],
            1e-5,
            QuadraticSweepLimits {
                max_intervals: 1,
                ..limits()
            },
        )
        .unwrap();
    assert!(matches!(r, QuadraticSweep::Unresolved { .. }));
    assert!(
        face()
            .swept_point_clearance_at(&p, &p, [0.; 3], [0.; 3], -1., limits())
            .is_err()
    );
    assert!(
        face()
            .swept_point_clearance_at(&p, &[], [0.; 3], [0.; 3], 1e-5, limits())
            .is_err()
    );
    assert!(
        face()
            .swept_point_clearance_at(
                &p,
                &p,
                [0.; 3],
                [0.; 3],
                1e-5,
                QuadraticSweepLimits {
                    minimum_time_fraction: 0.,
                    ..limits()
                }
            )
            .is_err()
    );
}

#[test]
fn first_clearance_bracket_contains_analytic_entry_not_middle_witness() {
    let p = surface(|u, v| [u, v, 0.]);
    let r = face()
        .first_point_clearance_at(
            &p,
            &p,
            [0.2, 0.3, 0.5],
            [0.2, 0.3, -1.5],
            0.01,
            limits(),
            1e-6,
            128,
        )
        .unwrap()
        .unwrap();
    let entry = (0.5 - 0.01) / 2.;
    assert!(r.converged, "{r:?}");
    assert!(r.time_interval[0] <= entry + 1e-10 && r.time_interval[1] >= entry - 1e-10);
    assert!(r.time_interval[1] - r.time_interval[0] <= 1e-6);
    assert!(r.witness.unwrap().distance_m <= 0.01);
    assert!(
        face()
            .first_point_clearance_at(
                &p,
                &p,
                [0.2, 0.3, 0.5],
                [0.2, 0.3, 1.],
                0.01,
                limits(),
                1e-6,
                128
            )
            .unwrap()
            .is_none()
    );
}
