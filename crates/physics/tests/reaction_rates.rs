use physics::{
    astrophysics_spin::Spin,
    contact::{
        ContactBody, ContactWrench, NetworkSupport, NormalContact, NormalSupport, ReactionConfig,
        ReactionRateConfig, SupportMotion, SupportPlane, normal_gap_acceleration, normal_gap_jerk,
        resolve_normal_reaction_rate_network,
    },
    gravity::Body,
};
fn body(position: [f64; 3], velocity: [f64; 3], mass: f64) -> ContactBody {
    ContactBody {
        motion: Body {
            position,
            velocity,
            mass,
        },
        spin: Some(Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0.; 3],
            inertia: [1.; 3],
        }),
    }
}
fn config() -> ReactionRateConfig {
    ReactionRateConfig {
        reaction: ReactionConfig {
            max_sweeps: 8192,
            acceleration_tolerance: 1e-11,
            normal_velocity_tolerance: 1e-10,
        },
        jerk_tolerance: 1e-10,
    }
}
fn point(first: usize, second: Option<usize>, p: [f64; 3]) -> NetworkSupport {
    NetworkSupport {
        first,
        second,
        support: NormalSupport {
            contact: NormalContact {
                point: p,
                normal: [0., 1., 0.],
            },
            plane: SupportPlane::World,
        },
    }
}
fn motion(v: [f64; 3]) -> SupportMotion {
    SupportMotion {
        point_velocity: v,
        normal_acceleration: None,
    }
}
#[test]
fn moving_stack_redistributes_floor_pressure_and_cancels_moving_arm_jerk() {
    let states = [
        body([0., 1., 0.], [1., 0., 0.], 2.),
        body([0.; 3], [0.; 3], 3.),
    ];
    let before = states;
    let contacts = [
        point(0, Some(1), [-0.25, 0.5, 0.]),
        point(0, Some(1), [0.25, 0.5, 0.]),
        point(1, None, [-1., -0.5, 0.]),
        point(1, None, [1., -0.5, 0.]),
    ];
    let moving = [
        motion([1., 0., 0.]),
        motion([1., 0., 0.]),
        motion([0.; 3]),
        motion([0.; 3]),
    ];
    let external = states.map(|b| ContactWrench {
        force: [0., -10. * b.motion.mass, 0.],
        torque: [0.; 3],
    });
    let report = resolve_normal_reaction_rate_network(
        &states,
        &contacts,
        &external,
        &[ContactWrench::default(); 2],
        &moving,
        config(),
    )
    .unwrap();
    assert_eq!(states, before);
    assert!((report.baseline.forces[0][1] - 10.).abs() < 1e-9);
    assert!((report.baseline.forces[2][1] - 25.).abs() < 1e-9);
    assert!(report.forces_rate[0][1].abs() < 1e-9);
    assert!((report.forces_rate[2][1] + 10.).abs() < 1e-9);
    assert!((report.forces_rate[3][1] - 10.).abs() < 1e-9);
    for rate in &report.wrenches_rate {
        assert!(
            rate.force
                .iter()
                .chain(&rate.torque)
                .all(|x| x.abs() < 1e-9)
        );
    }
    assert!(report.normal_jerks.iter().all(|x| x.abs() < 1e-10));
    assert!((report.positive_until_s - 2.5).abs() < 1e-8);
    // Reorder bodies and apply a proper cyclic world-frame rotation together.
    let rotate = |v: [f64; 3]| [v[2], v[0], v[1]];
    let mut transformed = [states[1], states[0]];
    for b in &mut transformed {
        b.motion.position = rotate(b.motion.position);
        b.motion.velocity = rotate(b.motion.velocity);
        b.spin.as_mut().unwrap().orientation = [0.5; 4];
    }
    let transformed_contacts = contacts.map(|mut c| {
        c.first = 1 - c.first;
        c.second = c.second.map(|j| 1 - j);
        c.support.contact.point = rotate(c.support.contact.point);
        c.support.contact.normal = rotate(c.support.contact.normal);
        c
    });
    let transformed_loads = [external[1], external[0]].map(|w| ContactWrench {
        force: rotate(w.force),
        torque: rotate(w.torque),
    });
    let transformed_motion = moving.map(|m| motion(rotate(m.point_velocity)));
    let equivalent = resolve_normal_reaction_rate_network(
        &transformed,
        &transformed_contacts,
        &transformed_loads,
        &[ContactWrench::default(); 2],
        &transformed_motion,
        config(),
    )
    .unwrap();
    for i in 0..contacts.len() {
        for k in 0..3 {
            assert!((equivalent.forces_rate[i][k] - rotate(report.forces_rate[i])[k]).abs() < 1e-9);
        }
    }
}
#[test]
fn zero_pressure_tangent_cone_allows_birth_and_release_without_negative_pressure() {
    let states = [body([0.; 3], [0.; 3], 1.)];
    let contacts = [
        point(0, None, [-1., -0.5, 0.]),
        point(0, None, [1., -0.5, 0.]),
    ];
    let external = [ContactWrench {
        force: [0., -2., 0.],
        torque: [0., 0., 2.],
    }];
    for (torque_rate, expected, limit) in [(-2., [-1., 1.], 2.), (2., [1., 0.], f64::INFINITY)] {
        let report = resolve_normal_reaction_rate_network(
            &states,
            &contacts,
            &external,
            &[ContactWrench {
                force: [0.; 3],
                torque: [0., 0., torque_rate],
            }],
            &[motion([0.; 3]); 2],
            config(),
        )
        .unwrap();
        assert_eq!(report.baseline.forces[1], [0.; 3]);
        for k in 0..2 {
            assert!((report.forces_rate[k][1] - expected[k]).abs() < 1e-12);
        }
        assert_eq!(report.positive_until_s, limit);
        if torque_rate > 0. {
            assert!(report.normal_jerks[1] > 0.);
        }
    }
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3])
}
fn rotate(v: [f64; 3], w: [f64; 3], t: f64) -> [f64; 3] {
    let speed = w[0].hypot(w[1]).hypot(w[2]);
    let angle = speed * t;
    if speed == 0. {
        return v;
    }
    let axis = w.map(|x| x / speed);
    let c = cross(axis, v);
    let dot = (0..3).map(|k| axis[k] * v[k]).sum::<f64>();
    std::array::from_fn(|k| {
        v[k] * angle.cos() + c[k] * angle.sin() + axis[k] * dot * (1. - angle.cos())
    })
}
#[test]
fn rotating_anisotropic_gap_jerk_matches_independent_directional_difference() {
    let mut initial = body([0.2, -0.1, 0.3], [0.4, -0.2, 0.1], 2.);
    initial.spin.as_mut().unwrap().inertia = [1., 2., 3.];
    initial.spin.as_mut().unwrap().angular_momentum = [0.3, 0.5, 0.7];
    let load = ContactWrench {
        force: [1., -2., 3.],
        torque: [0.4, -0.3, 0.2],
    };
    let load_rate = ContactWrench {
        force: [-0.3, 0.6, -0.2],
        torque: [0.7, -0.4, 0.5],
    };
    let mut support = point(0, None, [0.7, 0.2, -0.3]).support;
    support.contact.normal = [0., 0., 1.];
    support.plane = SupportPlane::First;
    let moving = motion([0.8, -0.4, 0.2]);
    let omega = initial.spin.unwrap().angular_velocity().unwrap();
    let expected =
        normal_gap_jerk(&initial, None, support, load, None, load_rate, None, moving).unwrap();
    let sample = |t: f64| {
        let mut state = initial;
        state.motion.position =
            std::array::from_fn(|k| initial.motion.position[k] + initial.motion.velocity[k] * t);
        state.motion.velocity = std::array::from_fn(|k| {
            initial.motion.velocity[k] + load.force[k] / initial.motion.mass * t
        });
        let spin = state.spin.as_mut().unwrap();
        let speed = omega[0].hypot(omega[1]).hypot(omega[2]);
        let angle = speed * t * 0.5;
        spin.orientation = [
            omega[0] / speed * angle.sin(),
            omega[1] / speed * angle.sin(),
            omega[2] / speed * angle.sin(),
            angle.cos(),
        ];
        spin.angular_momentum =
            std::array::from_fn(|k| initial.spin.unwrap().angular_momentum[k] + load.torque[k] * t);
        let mut moved = support;
        moved.contact.point =
            std::array::from_fn(|k| support.contact.point[k] + moving.point_velocity[k] * t);
        moved.contact.normal = rotate(support.contact.normal, omega, t);
        normal_gap_acceleration(
            &state,
            None,
            moved,
            ContactWrench {
                force: std::array::from_fn(|k| load.force[k] + load_rate.force[k] * t),
                torque: std::array::from_fn(|k| load.torque[k] + load_rate.torque[k] * t),
            },
            None,
        )
        .unwrap()
    };
    let coarse = (sample(1e-3) - sample(-1e-3)) / 2e-3;
    let fine = (sample(1e-5) - sample(-1e-5)) / 2e-5;
    assert!((fine - expected).abs() < 1e-8, "{fine} vs {expected}");
    assert!((fine - expected).abs() < (coarse - expected).abs());
}
#[test]
fn invalid_geometry_normal_acceleration_and_rate_config_preserve_inputs() {
    let states = [body([0.; 3], [0.; 3], 1.)];
    let before = states;
    let mut support = point(0, None, [0.; 3]).support;
    support.plane = SupportPlane::Rate {
        normal_rate: [1., 0., 0.],
    };
    let load = ContactWrench {
        force: [0., -2., 0.],
        torque: [0.; 3],
    };
    assert!(
        normal_gap_jerk(
            &states[0],
            None,
            support,
            load,
            None,
            ContactWrench::default(),
            None,
            motion([0.; 3])
        )
        .is_err()
    );
    let invalid = SupportMotion {
        point_velocity: [0.; 3],
        normal_acceleration: Some([0.; 3]),
    };
    assert!(
        normal_gap_jerk(
            &states[0],
            None,
            support,
            load,
            None,
            ContactWrench::default(),
            None,
            invalid
        )
        .is_err()
    );
    let valid = SupportMotion {
        point_velocity: [0.; 3],
        normal_acceleration: Some([0., -1., 0.]),
    };
    assert!(
        normal_gap_jerk(
            &states[0],
            None,
            support,
            load,
            None,
            ContactWrench::default(),
            None,
            valid
        )
        .unwrap()
        .is_finite()
    );
    let mut budget = config();
    budget.jerk_tolerance = f64::NAN;
    assert!(
        resolve_normal_reaction_rate_network(
            &states,
            &[point(0, None, [0.; 3])],
            &[load],
            &[ContactWrench::default()],
            &[motion([0.; 3])],
            budget
        )
        .is_err()
    );
    assert_eq!(states, before);
}

#[test]
fn outgoing_support_remains_unloaded_even_with_negative_acceleration_and_jerk() {
    let states = [body([0.; 3], [0., 1., 0.], 1.)];
    let external = [ContactWrench {
        force: [0., -2., 0.],
        torque: [0.; 3],
    }];
    let rate = [ContactWrench {
        force: [0., -1., 0.],
        torque: [0.; 3],
    }];
    let report = resolve_normal_reaction_rate_network(
        &states,
        &[point(0, None, [0.; 3])],
        &external,
        &rate,
        &[motion([0., 1., 0.])],
        config(),
    )
    .unwrap();
    assert_eq!(report.baseline.forces[0], [0.; 3]);
    assert_eq!(report.forces_rate[0], [0.; 3]);
    assert_eq!(report.normal_jerks[0], -1.);
    assert_eq!(report.jerk_residual, 0.);
}

#[test]
fn reciprocal_plane_owner_preserves_jerk_with_omitted_zero_second_load() {
    let mut rotating = body([0.2, -0.1, 0.3], [0.4, -0.2, 0.1], 2.);
    rotating.spin.as_mut().unwrap().inertia = [1., 2., 3.];
    rotating.spin.as_mut().unwrap().angular_momentum = [0.3, 0.5, 0.7];
    let mut support = point(0, None, [0.7, 0.2, -0.3]).support;
    support.contact.normal = [0., 0., 1.];
    support.plane = SupportPlane::First;
    let moving = motion([0.8, -0.4, 0.2]);
    let expected = normal_gap_jerk(
        &rotating,
        None,
        support,
        ContactWrench::default(),
        None,
        ContactWrench::default(),
        None,
        moving,
    )
    .unwrap();
    let mut fixed = body([0.; 3], [0.; 3], 1.);
    fixed.spin = None;
    support.plane = SupportPlane::Second;
    support.contact.normal = [0., 0., -1.];
    let reciprocal = normal_gap_jerk(
        &fixed,
        Some(&rotating),
        support,
        ContactWrench::default(),
        None,
        ContactWrench::default(),
        None,
        moving,
    )
    .unwrap();
    assert!((reciprocal - expected).abs() < 1e-13);
}
