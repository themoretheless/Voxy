use physics::{
    astrophysics_spin::Spin,
    contact::{
        ContactBody, Error, ManifoldConfig, NormalContact, resolve_normal_impact,
        resolve_normal_manifold,
    },
    gravity::Body,
};
fn body(position: [f64; 3], velocity: [f64; 3], z: f64) -> ContactBody {
    ContactBody {
        motion: Body {
            mass: 1.,
            position,
            velocity,
        },
        spin: Some(Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0., z],
            inertia: [1.; 3],
        }),
    }
}
fn config() -> ManifoldConfig {
    ManifoldConfig {
        max_sweeps: 100,
        velocity_tolerance: 1e-11,
    }
}
fn contacts(normal: [f64; 3]) -> [NormalContact; 2] {
    [-0.5, 0.5].map(|y| NormalContact {
        point: [0., y, 0.],
        normal,
    })
}
fn momentum(b: ContactBody) -> [f64; 3] {
    b.motion.velocity.map(|v| v * b.motion.mass)
}
fn angular_z(b: ContactBody) -> f64 {
    b.motion.position[0] * momentum(b)[1] - b.motion.position[1] * momentum(b)[0]
        + b.spin.unwrap().angular_momentum[2]
}
#[test]
fn two_face_points_stop_translation_and_spin_without_energy_gain() {
    let mut a = body([0.; 3], [-2., 0., 0.], 0.5);
    let before = a.energy().unwrap();
    let report = resolve_normal_manifold(&mut a, None, &contacts([1., 0., 0.]), config()).unwrap();
    assert!(a.motion.velocity[0].abs() < 1e-11);
    assert!(a.spin.unwrap().angular_momentum[2].abs() < 2e-11);
    assert!(report.sweeps > 1 && report.velocity_residual <= config().velocity_tolerance);
    assert!((a.energy().unwrap() - before - report.kinetic_energy_change).abs() < 1e-12);
    assert!(report.kinetic_energy_change < 0.);
    assert!(report.impulses.iter().all(|j| j[0] >= 0.));
}
#[test]
fn reciprocal_manifold_closes_world_momenta_and_energy_ledger() {
    let mut a = body([-0.5, 0., 0.], [2., 0.2, 0.], 0.3);
    let mut b = body([0.5, 0., 0.], [0., -0.1, 0.], -0.7);
    let p: [f64; 3] = std::array::from_fn(|k| momentum(a)[k] + momentum(b)[k]);
    let l = angular_z(a) + angular_z(b);
    let energy = a.energy().unwrap() + b.energy().unwrap();
    let points = contacts([-1., 0., 0.]);
    let report = resolve_normal_manifold(&mut a, Some(&mut b), &points, config()).unwrap();
    for k in 0..3 {
        assert!((momentum(a)[k] + momentum(b)[k] - p[k]).abs() < 1e-12);
    }
    assert!((angular_z(a) + angular_z(b) - l).abs() < 1e-12);
    assert!(
        (a.energy().unwrap() + b.energy().unwrap() - energy - report.kinetic_energy_change).abs()
            < 1e-12
    );
    for point in points {
        let va = a.point_velocity(point.point).unwrap();
        let vb = b.point_velocity(point.point).unwrap();
        assert!(-(va[0] - vb[0]) >= -config().velocity_tolerance);
    }
}
#[test]
fn late_nonconvergence_and_invalid_points_preserve_both_bodies() {
    let mut a = body([0.; 3], [-2., 0., 0.], 0.5);
    let mut b = body([1., 0., 0.], [0.; 3], 0.);
    let before = (a, b);
    let mut limited = config();
    limited.max_sweeps = 1;
    assert_eq!(
        resolve_normal_manifold(&mut a, Some(&mut b), &contacts([1., 0., 0.]), limited),
        Err(Error::Budget)
    );
    assert_eq!((a, b), before);
    let mut bad = contacts([1., 0., 0.]);
    bad[1].point[0] = f64::NAN;
    assert_eq!(
        resolve_normal_manifold(&mut a, Some(&mut b), &bad, config()),
        Err(Error::InvalidInput)
    );
    assert_eq!((a, b), before);
}
#[test]
fn duplicate_points_match_single_inelastic_impact_and_separating_points_are_inactive() {
    let initial = body([0.; 3], [-2., 0., 0.], 0.5);
    let mut a = initial;
    let mut single = initial;
    let point = NormalContact {
        point: [0., 0.5, 0.],
        normal: [1., 0., 0.],
    };
    resolve_normal_impact(&mut single, None, point.point, point.normal, 0.).unwrap();
    let report = resolve_normal_manifold(&mut a, None, &[point; 4], config()).unwrap();
    assert_eq!(a, single);
    assert_eq!(
        report.impulses.iter().map(|j| j[0]).sum::<f64>(),
        a.motion.velocity[0] + 2.
    );
    let mut separating = body([0.; 3], [2., 0., 0.], 0.);
    let before = separating;
    let report =
        resolve_normal_manifold(&mut separating, None, &contacts([1., 0., 0.]), config()).unwrap();
    assert_eq!(separating, before);
    assert!(report.impulses.iter().all(|j| *j == [0.; 3]));
}

#[test]
fn contact_order_and_proper_coordinate_permutation_preserve_the_solution() {
    let initial = body([0.; 3], [-2., 0.3, -0.1], 0.5);
    let points = contacts([1., 0., 0.]);
    let mut a = initial;
    let mut reverse = initial;
    resolve_normal_manifold(&mut a, None, &points, config()).unwrap();
    resolve_normal_manifold(&mut reverse, None, &[points[1], points[0]], config()).unwrap();
    for k in 0..3 {
        assert!((a.motion.velocity[k] - reverse.motion.velocity[k]).abs() < 3e-11);
        assert!(
            (a.spin.unwrap().angular_momentum[k] - reverse.spin.unwrap().angular_momentum[k]).abs()
                < 3e-11
        );
    }
    let rotate = |v: [f64; 3]| [v[2], v[0], v[1]];
    let mut reframed = initial;
    reframed.motion.position = rotate(initial.motion.position);
    reframed.motion.velocity = rotate(initial.motion.velocity);
    let spin = reframed.spin.as_mut().unwrap();
    spin.orientation = [0.5; 4];
    spin.angular_momentum = rotate(initial.spin.unwrap().angular_momentum);
    let transformed = points.map(|p| NormalContact {
        point: rotate(p.point),
        normal: rotate(p.normal),
    });
    resolve_normal_manifold(&mut reframed, None, &transformed, config()).unwrap();
    for k in 0..3 {
        assert!((reframed.motion.velocity[k] - rotate(a.motion.velocity)[k]).abs() < 3e-11);
        assert!(
            (reframed.spin.unwrap().angular_momentum[k]
                - rotate(a.spin.unwrap().angular_momentum)[k])
                .abs()
                < 3e-11
        );
    }
}
