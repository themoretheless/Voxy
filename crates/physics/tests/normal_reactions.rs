use physics::{
    astrophysics_spin::Spin,
    contact::{
        ContactBody, ContactWrench, Error, ManifoldConfig, NormalContact, NormalSupport,
        ReactionConfig, SupportPlane, normal_gap_acceleration, resolve_normal_manifold,
        resolve_normal_reactions,
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
fn config() -> ReactionConfig {
    ReactionConfig {
        max_sweeps: 100,
        acceleration_tolerance: 1e-11,
        normal_velocity_tolerance: 1e-10,
    }
}
fn force(value: [f64; 3]) -> ContactWrench {
    ContactWrench {
        force: value,
        torque: [0.; 3],
    }
}
fn support(contact: NormalContact, plane: SupportPlane) -> NormalSupport {
    NormalSupport { contact, plane }
}
#[test]
fn supported_weight_balances_force_and_torque_without_mutating_state() {
    let mut a = body([0.; 3], [0.; 3], 0.);
    a.motion.mass = 2.;
    let before = a;
    let contacts = [-0.5, 0.5].map(|x| NormalContact {
        point: [x, -0.5, 0.],
        normal: [0., 2., 0.],
    });
    let report = resolve_normal_reactions(
        &a,
        None,
        &contacts.map(|c| support(c, SupportPlane::World)),
        force([0., -20., 0.]),
        None,
        config(),
    )
    .unwrap();
    assert_eq!(a, before);
    assert!((report.first_wrench.force[1] - 20.).abs() < 1e-11);
    assert!(report.first_wrench.torque.iter().all(|v| v.abs() < 1e-11));
    assert!(report.normal_accelerations.iter().all(|a| a.abs() < 1e-11));
    assert!(report.forces.iter().all(|f| f[1] >= 0.));
    assert_eq!(report.second_wrench, None);
    assert_eq!(report.instantaneous_power, 0.);
    let detached = resolve_normal_reactions(
        &a,
        None,
        &contacts.map(|c| support(c, SupportPlane::World)),
        force([0., 20., 0.]),
        None,
        config(),
    )
    .unwrap();
    assert!(detached.forces.iter().all(|f| *f == [0.; 3]));
    assert!(
        detached
            .normal_accelerations
            .iter()
            .all(|v| (*v - 10.).abs() < 1e-12)
    );
}
#[test]
fn rotating_patch_reaction_closes_analytic_curvature_and_reciprocal_wrenches() {
    let mut a = body([-0.04, 1., 0.], [3., 0., 0.], 0.);
    let mut b = body([0.; 3], [0.; 3], 0.);
    let contacts =
        [(0.98, -0.02), (1.02, -0.02), (0.98, 0.02), (1.02, 0.02)].map(|(y, z)| NormalContact {
            point: [0., y, z],
            normal: [-1., 0., 0.],
        });
    resolve_normal_manifold(
        &mut a,
        Some(&mut b),
        &contacts,
        ManifoldConfig {
            max_sweeps: 100,
            velocity_tolerance: 1e-11,
        },
    )
    .unwrap();
    let before = (a, b);
    let free = normal_gap_acceleration(
        &a,
        Some(&b),
        support(contacts[0], SupportPlane::Second),
        ContactWrench::default(),
        None,
    )
    .unwrap();
    let impulse = 3. / 2.9608;
    let closing = 0.001552 * impulse * impulse;
    assert!((free + closing).abs() < 1e-10);
    let report = resolve_normal_reactions(
        &a,
        Some(&b),
        &contacts.map(|c| support(c, SupportPlane::Second)),
        ContactWrench::default(),
        None,
        config(),
    )
    .unwrap();
    let reaction = closing / 2.9608;
    assert!((report.first_wrench.force[0] + reaction).abs() < 1e-10);
    let second = report.second_wrench.unwrap();
    for k in 0..3 {
        assert!((report.first_wrench.force[k] + second.force[k]).abs() < 1e-13);
    }
    let angular_z = a.motion.position[0] * report.first_wrench.force[1]
        - a.motion.position[1] * report.first_wrench.force[0]
        + report.first_wrench.torque[2]
        + b.motion.position[0] * second.force[1]
        - b.motion.position[1] * second.force[0]
        + second.torque[2];
    assert!(angular_z.abs() < 1e-13);
    assert!(report.acceleration_residual <= config().acceleration_tolerance);
    assert!(
        report.normal_accelerations[0].abs() < 1e-11
            && report.normal_accelerations[2].abs() < 1e-11
    );
    assert_eq!(report.forces[1], [0.; 3]);
    assert_eq!(report.forces[3], [0.; 3]);
    assert!(report.instantaneous_power.abs() < 1e-12);
    assert_eq!((a, b), before);
}
#[test]
fn gap_acceleration_matches_independent_accelerated_rotating_plane_difference() {
    let a = body([-0.04, 1., 0.], [1.9, 0.2, 0.], 0.4);
    let b = body([0.; 3], [1.1, -0.1, 0.], -0.6);
    let contact = NormalContact {
        point: [0., 0.98, 0.01],
        normal: [-1., 0., 0.],
    };
    let wa = ContactWrench {
        force: [0.3, 0.2, 0.],
        torque: [0., 0., 0.7],
    };
    let wb = ContactWrench {
        force: [0.1, -0.3, 0.],
        torque: [0., 0., -0.3],
    };
    let expected = normal_gap_acceleration(
        &a,
        Some(&b),
        support(contact, SupportPlane::Second),
        wa,
        Some(wb),
    )
    .unwrap();
    let rotate = |v: [f64; 3], angle: f64| {
        let (s, c) = angle.sin_cos();
        [c * v[0] - s * v[1], s * v[0] + c * v[1], v[2]]
    };
    let gap = |t: f64| {
        let ra = rotate([0.04, -0.02, 0.01], 0.4 * t + 0.35 * t * t);
        let normal = rotate([-1., 0., 0.], -0.6 * t - 0.15 * t * t);
        let r: [f64; 3] = std::array::from_fn(|k| {
            a.motion.position[k] - b.motion.position[k]
                + (a.motion.velocity[k] - b.motion.velocity[k]) * t
                + 0.5 * (wa.force[k] - wb.force[k]) * t * t
                + ra[k]
        });
        normal.iter().zip(r).map(|(n, r)| n * r).sum::<f64>()
    };
    let difference = |h: f64| (gap(h) - 2. * gap(0.) + gap(-h)) / (h * h);
    let coarse = (difference(0.002) - expected).abs();
    let fine = (difference(0.001) - expected).abs();
    assert!(fine < coarse * 0.3, "coarse={coarse} fine={fine}");
    assert!(fine < 4e-6);
}
#[test]
fn anisotropic_gyroscopic_acceleration_is_not_dropped() {
    let mut a = body([0.; 3], [0.; 3], 0.);
    a.spin = Some(Spin {
        orientation: [0., 0., 0., 1.],
        angular_momentum: [2., 6., 15.],
        inertia: [2., 3., 5.],
    });
    // omega=(1,2,3); Euler acceleration=(-6,3,-0.4).
    // alpha cross Y + omega cross (omega cross Y) = (2.4,-10,0).
    let value = normal_gap_acceleration(
        &a,
        None,
        support(
            NormalContact {
                point: [0., 1., 0.],
                normal: [1., 0., 0.],
            },
            SupportPlane::World,
        ),
        ContactWrench::default(),
        None,
    )
    .unwrap();
    assert!((value - 2.4).abs() < 1e-12);
}
#[test]
fn outgoing_points_are_inactive_and_approaching_points_require_impact() {
    let contact = NormalContact {
        point: [0.; 3],
        normal: [0., 1., 0.],
    };
    let a = body([0.; 3], [0., 1., 0.], 0.);
    let report = resolve_normal_reactions(
        &a,
        None,
        &[support(contact, SupportPlane::World)],
        force([0., -10., 0.]),
        None,
        config(),
    )
    .unwrap();
    assert_eq!(report.forces, vec![[0.; 3]]);
    assert_eq!(report.sweeps, 0);
    assert_eq!(report.normal_accelerations, vec![-10.]);
    let incoming = body([0.; 3], [0., -1., 0.], 0.);
    assert_eq!(
        resolve_normal_reactions(
            &incoming,
            None,
            &[support(contact, SupportPlane::World)],
            force([0., -10., 0.]),
            None,
            config()
        ),
        Err(Error::InvalidInput)
    );
}
#[test]
fn late_budget_invalid_geometry_and_bad_settings_preserve_inputs() {
    let a = body([0.; 3], [0.; 3], 0.);
    let b = body([1., 0., 0.], [0.; 3], 0.);
    let before = (a, b);
    let points = [
        NormalContact {
            point: [0.; 3],
            normal: [1., 0., 0.],
        },
        NormalContact {
            point: [0.; 3],
            normal: [0.6, 0.8, 0.],
        },
        NormalContact {
            point: [0.; 3],
            normal: [0.4, 0.2, 0.8_f64.sqrt()],
        },
    ];
    let points = points.map(|c| support(c, SupportPlane::Second));
    let mut limited = config();
    limited.max_sweeps = 1;
    assert_eq!(
        resolve_normal_reactions(
            &a,
            Some(&b),
            &points,
            force([-2., -1., -0.5]),
            None,
            limited
        ),
        Err(Error::Budget)
    );
    let mut malformed = points;
    malformed[1].contact.point[0] = f64::NAN;
    assert_eq!(
        resolve_normal_reactions(
            &a,
            Some(&b),
            &malformed,
            ContactWrench::default(),
            None,
            config()
        ),
        Err(Error::InvalidInput)
    );
    let mut invalid = config();
    invalid.acceleration_tolerance = f64::NAN;
    assert_eq!(
        resolve_normal_reactions(
            &a,
            Some(&b),
            &points,
            ContactWrench::default(),
            None,
            invalid
        ),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        resolve_normal_reactions(
            &a,
            None,
            &points,
            ContactWrench::default(),
            Some(ContactWrench::default()),
            config()
        ),
        Err(Error::InvalidInput)
    );
    assert_eq!((a, b), before);
}

#[test]
fn supporting_plane_owner_and_reciprocal_swap_are_explicit() {
    let mut a = body([-0.04, 1., 0.], [3., 0., 0.], 0.);
    let mut b = body([0.; 3], [0.; 3], 0.);
    let points =
        [(0.98, -0.02), (1.02, -0.02), (0.98, 0.02), (1.02, 0.02)].map(|(y, z)| NormalContact {
            point: [0., y, z],
            normal: [-1., 0., 0.],
        });
    resolve_normal_manifold(
        &mut a,
        Some(&mut b),
        &points,
        ManifoldConfig {
            max_sweeps: 100,
            velocity_tolerance: 1e-11,
        },
    )
    .unwrap();
    let moving = points.map(|c| support(c, SupportPlane::Second));
    let world = points.map(|c| support(c, SupportPlane::World));
    let curved =
        normal_gap_acceleration(&a, Some(&b), moving[0], ContactWrench::default(), None).unwrap();
    let fixed =
        normal_gap_acceleration(&a, Some(&b), world[0], ContactWrench::default(), None).unwrap();
    assert!(curved < -0.001 && fixed > 0.);
    let report = resolve_normal_reactions(
        &a,
        Some(&b),
        &moving,
        ContactWrench::default(),
        None,
        config(),
    )
    .unwrap();
    let swapped = points.map(|c| {
        support(
            NormalContact {
                normal: c.normal.map(|n| -n),
                ..c
            },
            SupportPlane::First,
        )
    });
    let reverse = resolve_normal_reactions(
        &b,
        Some(&a),
        &swapped,
        ContactWrench::default(),
        None,
        config(),
    )
    .unwrap();
    for k in 0..3 {
        assert!(
            (report.first_wrench.force[k] - reverse.second_wrench.unwrap().force[k]).abs() < 1e-10
        );
        assert!(
            (report.first_wrench.torque[k] - reverse.second_wrench.unwrap().torque[k]).abs()
                < 1e-10
        );
    }
    for k in 0..points.len() {
        assert!((report.normal_accelerations[k] - reverse.normal_accelerations[k]).abs() < 1e-11);
    }
    let reversed = moving.into_iter().rev().collect::<Vec<_>>();
    let reordered = resolve_normal_reactions(
        &a,
        Some(&b),
        &reversed,
        ContactWrench::default(),
        None,
        config(),
    )
    .unwrap();
    let rotate = |v: [f64; 3]| [v[2], v[0], v[1]];
    let transform = |mut body: ContactBody| {
        body.motion.position = rotate(body.motion.position);
        body.motion.velocity = rotate(body.motion.velocity);
        let spin = body.spin.as_mut().unwrap();
        spin.orientation = [0.5; 4];
        spin.angular_momentum = rotate(spin.angular_momentum);
        body
    };
    let rotated = moving.map(|s| {
        support(
            NormalContact {
                point: rotate(s.contact.point),
                normal: rotate(s.contact.normal),
            },
            s.plane,
        )
    });
    let reoriented = resolve_normal_reactions(
        &transform(a),
        Some(&transform(b)),
        &rotated,
        ContactWrench::default(),
        None,
        config(),
    )
    .unwrap();
    for k in 0..3 {
        assert!((report.first_wrench.force[k] - reordered.first_wrench.force[k]).abs() < 1e-10);
        assert!(
            (rotate(report.first_wrench.force)[k] - reoriented.first_wrench.force[k]).abs() < 1e-10
        );
        assert!(
            (rotate(report.first_wrench.torque)[k] - reoriented.first_wrench.torque[k]).abs()
                < 1e-10
        );
    }
    assert_eq!(
        normal_gap_acceleration(&a, None, moving[0], ContactWrench::default(), None),
        Err(Error::InvalidInput)
    );
}

#[test]
fn a_body_owned_plane_against_a_fixed_vertex_matches_reversed_kinematics() {
    let a = body([0.; 3], [0.; 3], 0.5);
    let contact = NormalContact {
        point: [1., 0., 0.],
        normal: [-1., 0., 0.],
    };
    let actual = normal_gap_acceleration(
        &a,
        None,
        support(contact, SupportPlane::First),
        ContactWrench::default(),
        None,
    )
    .unwrap();
    // n_A(t) dot (COM_A-x_fixed) has second derivative -omega^2 at t=0.
    assert!((actual + 0.25).abs() < 1e-12);
    // Holding the same normal fixed follows the first material point instead,
    // and gives the opposite curvature. The plane owner cannot be guessed.
    let fixed = normal_gap_acceleration(
        &a,
        None,
        support(contact, SupportPlane::World),
        ContactWrench::default(),
        None,
    )
    .unwrap();
    assert!((fixed - 0.25).abs() < 1e-12);
}

#[test]
fn geometry_owned_edge_normal_rate_matches_independent_cross_product_motion() {
    let a = body([-0.04, 1., 0.], [1.9, 0.2, 0.], 0.4);
    let mut b = body([0.; 3], [1.1, -0.1, 0.], 0.);
    b.spin.as_mut().unwrap().angular_momentum = [-0.6, 0., 0.];
    let contact = NormalContact {
        point: [0., 0.98, 0.01],
        normal: [-1., 0., 0.],
    };
    // First Y edge rotates about Z; second Z edge rotates about X.
    // At t=0 their cross product is X. Directed toward the first body, the unit
    // normal is -X and its derivative is -0.4 Y.
    let feature = support(
        contact,
        SupportPlane::Rate {
            normal_rate: [0., -0.4, 0.],
        },
    );
    let expected =
        normal_gap_acceleration(&a, Some(&b), feature, ContactWrench::default(), None).unwrap();
    assert!((expected + 0.2416).abs() < 1e-12);
    let gap = |t: f64| {
        let (sa, ca) = (0.4 * t).sin_cos();
        let (sb, cb) = (0.6 * t).sin_cos();
        let raw = [-ca * cb, -sa * cb, sa * sb];
        let norm = raw.iter().map(|v| v * v).sum::<f64>().sqrt();
        let normal = raw.map(|v| v / norm);
        let ra = [ca * 0.04 + sa * 0.02, sa * 0.04 - ca * 0.02, 0.01];
        let rb = [0., cb * 0.98 + sb * 0.01, -sb * 0.98 + cb * 0.01];
        let separation: [f64; 3] = std::array::from_fn(|k| {
            a.motion.position[k] - b.motion.position[k]
                + (a.motion.velocity[k] - b.motion.velocity[k]) * t
                + ra[k]
                - rb[k]
        });
        normal
            .iter()
            .zip(separation)
            .map(|(n, r)| n * r)
            .sum::<f64>()
    };
    let difference = |h: f64| (gap(h) - 2. * gap(0.) + gap(-h)) / (h * h);
    let coarse = (difference(0.002) - expected).abs();
    let fine = (difference(0.001) - expected).abs();
    assert!(fine < coarse * 0.3, "coarse={coarse} fine={fine}");
    assert!(fine < 4e-6);
    assert_eq!(
        normal_gap_acceleration(
            &a,
            Some(&b),
            support(
                contact,
                SupportPlane::Rate {
                    normal_rate: [1., 0., 0.]
                }
            ),
            ContactWrench::default(),
            None
        ),
        Err(Error::InvalidInput)
    );
}

#[test]
fn point_body_accepts_central_force_and_rejects_unowned_intrinsic_torque() {
    let mut a = body([0.; 3], [0.; 3], 0.);
    a.spin = None;
    let central = support(
        NormalContact {
            point: [0.; 3],
            normal: [1., 0., 0.],
        },
        SupportPlane::World,
    );
    let report =
        resolve_normal_reactions(&a, None, &[central], force([-2., 0., 0.]), None, config())
            .unwrap();
    assert_eq!(report.first_wrench.force, [2., 0., 0.]);
    assert_eq!(report.first_wrench.torque, [0.; 3]);
    let offset = support(
        NormalContact {
            point: [0., 1., 0.],
            normal: [1., 0., 0.],
        },
        SupportPlane::World,
    );
    assert_eq!(
        resolve_normal_reactions(&a, None, &[offset], force([-2., 0., 0.]), None, config()),
        Err(Error::InvalidInput)
    );
}
