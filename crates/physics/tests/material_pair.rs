use physics::{
    astrophysics_spin::Spin,
    contact::ContactBody,
    gravity::Body,
    rigid_motion::{Error, MaterialForcePair},
    spin_path::Config,
};
fn config() -> Config {
    Config {
        max_angular_error_rad: 1e-4,
        min_step_s: 1e-9,
        max_arcs: 120000,
        max_trials: 360000,
    }
}
fn bodies() -> [ContactBody; 2] {
    [
        ContactBody {
            motion: Body {
                mass: 2.,
                position: [1., 2., 3.],
                velocity: [2., -1., 0.],
            },
            spin: Some(Spin {
                orientation: [0., 0., 0., 1.],
                inertia: [2., 3., 4.],
                angular_momentum: [0.7, 0.3, -0.2],
            }),
        },
        ContactBody {
            motion: Body {
                mass: 3.,
                position: [-0.4, 0.3, 2.8],
                velocity: [-1., 2., 0.3],
            },
            spin: Some(Spin {
                orientation: [0., 0., 0., 1.],
                inertia: [0.3, 0.5, 0.7],
                angular_momentum: [0.1, 0.2, 0.05],
            }),
        },
    ]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3])
}
fn rotate(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let axis = [q[0], q[1], q[2]];
    let a = cross(axis, v);
    let b = cross(axis, a);
    std::array::from_fn(|k| v[k] + 2. * (q[3] * a[k] + b[k]))
}
fn distance(a: [f64; 4], b: [f64; 4]) -> f64 {
    let direct = (0..4).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt();
    let opposite = (0..4).map(|k| (a[k] + b[k]).powi(2)).sum::<f64>().sqrt();
    4. * (0.5 * direct.min(opposite)).clamp(0., 1.).asin()
}
fn momentum(bodies: [ContactBody; 2]) -> ([f64; 3], [f64; 3]) {
    let p = bodies.map(|b| b.motion.velocity.map(|v| v * b.motion.mass));
    let orbital = std::array::from_fn::<_, 2, _>(|i| cross(bodies[i].motion.position, p[i]));
    (
        std::array::from_fn(|k| p[0][k] + p[1][k]),
        std::array::from_fn(|k| {
            orbital[0][k]
                + orbital[1][k]
                + bodies[0].spin.map_or(0., |s| s.angular_momentum[k])
                + bodies[1].spin.map_or(0., |s| s.angular_momentum[k])
        }),
    )
}

#[test]
fn whole_pair_bounds_cover_independent_joint_rk4_with_anisotropic_and_light_receivers() {
    let local = [1., -0.5, 1.5];
    let force = [0.2, 0.6, -0.4];
    let rate = [-0.8, 0.4, 0.2];
    let duration = 0.08;
    for light in [false, true] {
        let mut initial = bodies();
        if light {
            initial[1].spin = Some(Spin {
                inertia: [0.01; 3],
                angular_momentum: [0.001, 0.002, 0.003],
                ..initial[1].spin.unwrap()
            });
        }
        let pair = MaterialForcePair::prepare(
            initial[0],
            initial[1],
            local,
            force,
            rate,
            duration,
            config(),
        )
        .unwrap();
        let a = pair.paths()[0].rotation().unwrap();
        let b = pair.paths()[1].rotation().unwrap();
        assert!(a.model_angular_error_rad() <= config().max_angular_error_rad);
        assert!(b.model_angular_error_rad() <= config().max_angular_error_rad);
        assert!(a.model_angular_momentum_error() > 0.);
        assert_eq!(
            a.model_angular_momentum_error(),
            b.model_angular_momentum_error()
        );
        if light {
            assert!(a.model_angular_error_rad() < config().max_angular_error_rad / 16.);
        }
        let conditional = initial[1]
            .prepare_material_point_force_motion(
                &pair.paths()[0],
                local,
                force.map(|x| -x),
                rate.map(|x| -x),
                duration,
                Config {
                    max_angular_error_rad: config().max_angular_error_rad * 0.5,
                    ..config()
                },
            )
            .unwrap();
        assert!(
            b.model_angular_error_rad() > conditional.rotation().unwrap().model_angular_error_rad()
        );
        assert_eq!(pair.paths()[1].end(), conditional.end());
        let derivative = |t: f64, states: [Spin; 2]| {
            let states = states.map(|s| {
                let length = s.orientation.iter().map(|v| v * v).sum::<f64>().sqrt();
                Spin {
                    orientation: s.orientation.map(|v| v / length),
                    ..s
                }
            });
            let centers: [[f64; 3]; 2] = std::array::from_fn(|i| {
                let sign = if i == 0 { 1. } else { -1. };
                std::array::from_fn(|k| {
                    initial[i].motion.position[k]
                        + initial[i].motion.velocity[k] * t
                        + sign * (force[k] * t * t / 2. + rate[k] * t * t * t / 6.)
                            / initial[i].motion.mass
                })
            });
            // The oracle's own first attitude supplies BOTH torques. It never
            // samples the prepared source path or its integrated moment.
            let arm = rotate(states[0].orientation, local);
            let point: [f64; 3] = std::array::from_fn(|k| centers[0][k] + arm[k]);
            let applied: [f64; 3] = std::array::from_fn(|k| force[k] + rate[k] * t);
            let torque = [
                cross(arm, applied),
                cross(
                    std::array::from_fn(|k| point[k] - centers[1][k]),
                    applied.map(|v| -v),
                ),
            ];
            let qdot: [[f64; 4]; 2] = states.map(|s| {
                let w = s.angular_velocity().unwrap();
                let q = s.orientation;
                [
                    0.5 * (w[0] * q[3] + w[1] * q[2] - w[2] * q[1]),
                    0.5 * (-w[0] * q[2] + w[1] * q[3] + w[2] * q[0]),
                    0.5 * (w[0] * q[1] - w[1] * q[0] + w[2] * q[3]),
                    -0.5 * (w[0] * q[0] + w[1] * q[1] + w[2] * q[2]),
                ]
            });
            (qdot, torque)
        };
        let mut oracle = initial.map(|b| b.spin.unwrap());
        let h = duration / 4096.;
        let (p0, l0) = momentum(initial);
        for step in 0..=4096 {
            let t = duration * step as f64 / 4096.;
            if step % 64 == 0 {
                let actual = pair.sample(t).unwrap();
                for i in 0..2 {
                    let rotation = pair.paths()[i].rotation().unwrap();
                    let spin = actual[i].spin.unwrap();
                    assert!(
                        distance(spin.orientation, oracle[i].orientation)
                            <= rotation.model_angular_error_rad() + 1e-10
                    );
                    let delta: [f64; 3] = std::array::from_fn(|k| {
                        spin.angular_momentum[k] - oracle[i].angular_momentum[k]
                    });
                    assert!(
                        delta[0].hypot(delta[1]).hypot(delta[2])
                            <= rotation.model_angular_momentum_error() + 1e-10
                    );
                }
                let (p, l) = momentum(actual);
                for k in 0..3 {
                    assert!((p[k] - p0[k]).abs() < 1e-12);
                    assert!((l[k] - l0[k]).abs() < 1e-9);
                }
            }
            if step < 4096 {
                let mix = |q: [[f64; 4]; 2], l: [[f64; 3]; 2], scale: f64| {
                    std::array::from_fn::<_, 2, _>(|i| Spin {
                        orientation: std::array::from_fn(|k| {
                            oracle[i].orientation[k] + scale * q[i][k]
                        }),
                        angular_momentum: std::array::from_fn(|k| {
                            oracle[i].angular_momentum[k] + scale * l[i][k]
                        }),
                        ..oracle[i]
                    })
                };
                let (q1, l1) = derivative(t, oracle);
                let (q2, l2) = derivative(t + h / 2., mix(q1, l1, h / 2.));
                let (q3, l3) = derivative(t + h / 2., mix(q2, l2, h / 2.));
                let (q4, l4) = derivative(t + h, mix(q3, l3, h));
                for i in 0..2 {
                    let q: [f64; 4] = std::array::from_fn(|k| {
                        oracle[i].orientation[k]
                            + h * (q1[i][k] + 2. * q2[i][k] + 2. * q3[i][k] + q4[i][k]) / 6.
                    });
                    let length = q.iter().map(|v| v * v).sum::<f64>().sqrt();
                    oracle[i].orientation = q.map(|v| v / length);
                    oracle[i].angular_momentum = std::array::from_fn(|k| {
                        oracle[i].angular_momentum[k]
                            + h * (l1[i][k] + 2. * l2[i][k] + 2. * l3[i][k] + l4[i][k]) / 6.
                    });
                }
            }
        }
        assert_eq!(pair.sample(0.).unwrap(), initial);
        assert_eq!(
            pair.sample(duration).unwrap(),
            pair.paths().each_ref().map(|p| p.end())
        );
        let expected = pair.paths().each_ref().map(|p| p.work(duration).unwrap());
        assert_eq!(pair.work(duration).unwrap(), expected);
    }
}

#[test]
fn pair_impact_rebuilds_both_forces_from_the_new_common_material_point() {
    let mut initial = bodies();
    initial[0].spin = Some(Spin {
        inertia: [1.; 3],
        angular_momentum: [0., 0., 2.],
        ..initial[0].spin.unwrap()
    });
    initial[1].motion.position = [-0.2, 0.5, 3.];
    initial[1].motion.velocity = [-1., 0.2, 0.];
    initial[1].spin = Some(Spin {
        inertia: [1.; 3],
        angular_momentum: [0.; 3],
        ..initial[1].spin.unwrap()
    });
    let local = [0.4, 0., 0.];
    let force = [0., 3., 0.];
    let rate = [0., -2., 0.];
    let pair =
        MaterialForcePair::prepare(initial[0], initial[1], local, force, rate, 0.2, config())
            .unwrap();
    let saved = pair.clone();
    let time = 0.07;
    let before = pair.sample(time).unwrap();
    let point = pair.point(time).unwrap().position;
    let event = pair
        .prepare_impact(time, point, [-1., 0., 0.], 0.4, config())
        .unwrap();
    let remaining = event.remainder.as_ref().unwrap();
    assert_eq!(remaining.sample(0.).unwrap(), event.bodies);
    let energy_before: f64 = before.iter().map(|b| b.energy().unwrap()).sum();
    let energy_after: f64 = event.bodies.iter().map(|b| b.energy().unwrap()).sum();
    assert!((energy_after + event.impulse.dissipated_energy - energy_before).abs() < 1e-11);
    let (p0, l0) = momentum(before);
    let (p, l) = momentum(event.bodies);
    for k in 0..3 {
        assert!((p[k] - p0[k]).abs() < 1e-12);
        assert!((l[k] - l0[k]).abs() < 1e-11);
    }
    for t in [1e-9, 0.02, remaining.duration()] {
        let (p, l) = momentum(remaining.sample(t).unwrap());
        for k in 0..3 {
            assert!((p[k] - p0[k]).abs() < 1e-12);
            assert!((l[k] - l0[k]).abs() < 1e-10);
        }
    }
    let changed = remaining.point(0.02).unwrap().position;
    let old = pair.point(time + 0.02).unwrap().position;
    assert!(
        (0..3)
            .map(|k| (changed[k] - old[k]).powi(2))
            .sum::<f64>()
            .sqrt()
            > 1e-4
    );
    for segment in remaining.paths()[1].rotation().unwrap().segments() {
        for dt in [0., segment.arc.duration() * 0.5, segment.arc.duration()] {
            let t = segment.start_s + dt;
            let point = remaining.point(t).unwrap().position;
            let center = remaining.sample(t).unwrap()[1].motion.position;
            let f = std::array::from_fn(|k| -(force[k] + rate[k] * (time + t)));
            let expected = cross(std::array::from_fn(|k| point[k] - center[k]), f);
            let actual = segment.arc.torque_at(dt).unwrap();
            for k in 0..3 {
                assert!((actual[k] - expected[k]).abs() < 1e-10);
            }
        }
    }
    assert_eq!(
        event.endpoints(),
        remaining.sample(remaining.duration()).unwrap()
    );
    assert!(
        pair.prepare_impact(
            time,
            point,
            [-1., 0., 0.],
            0.4,
            Config {
                max_arcs: 1,
                max_trials: 1,
                ..config()
            }
        )
        .is_err()
    );
    let end = pair
        .prepare_impact(
            pair.duration(),
            pair.point(pair.duration()).unwrap().position,
            [-1., 0., 0.],
            0.4,
            config(),
        )
        .unwrap();
    assert!(end.remainder.is_none());
    assert_eq!(end.endpoints(), end.bodies);
    let next_time = 0.03;
    let next_point = remaining.point(next_time).unwrap().position;
    let next_bodies = remaining.sample(next_time).unwrap();
    let velocities = next_bodies.map(|b| b.point_velocity(next_point).unwrap());
    let relative: [f64; 3] = std::array::from_fn(|k| velocities[0][k] - velocities[1][k]);
    let speed = relative[0].hypot(relative[1]).hypot(relative[2]);
    let normal = relative.map(|v| -v / speed);
    let next = remaining
        .prepare_impact(next_time, next_point, normal, 0.2, config())
        .unwrap();
    let next_pair = next.remainder.as_ref().unwrap();
    let (p, l) = momentum(next.endpoints());
    for k in 0..3 {
        assert!((p[k] - p0[k]).abs() < 1e-12);
        assert!((l[k] - l0[k]).abs() < 1e-10);
    }
    for segment in next_pair.paths()[1].rotation().unwrap().segments() {
        let t = segment.start_s + segment.arc.duration() * 0.5;
        let point = next_pair.point(t).unwrap().position;
        let center = next_pair.sample(t).unwrap()[1].motion.position;
        let f = std::array::from_fn(|k| -(force[k] + rate[k] * (time + next_time + t)));
        let expected = cross(std::array::from_fn(|k| point[k] - center[k]), f);
        let actual = segment.arc.torque_at(segment.arc.duration() * 0.5).unwrap();
        for k in 0..3 {
            assert!((actual[k] - expected[k]).abs() < 1e-10);
        }
    }
    assert_eq!(pair, saved);
}

#[test]
fn pair_rejects_invalid_inputs_and_preserves_bodies_on_second_preparation_failure() {
    let [mut a, b] = bodies();
    a.spin = Some(Spin {
        inertia: [1.; 3],
        ..a.spin.unwrap()
    });
    let saved = [a, b];
    let duration = 0.1;
    let limited = Config {
        max_arcs: 1,
        max_trials: 1,
        ..config()
    };
    assert!(
        a.prepare_own_material_point_force_motion(
            [0.; 3],
            [0.; 3],
            [0.; 3],
            duration,
            Config {
                max_angular_error_rad: config().max_angular_error_rad * 0.25,
                ..limited
            }
        )
        .is_ok()
    );
    assert!(
        MaterialForcePair::prepare(a, b, [0.; 3], [0.; 3], [0.; 3], duration, limited).is_err()
    );
    assert_eq!([a, b], saved);
    for duration in [0., -1., f64::NAN, f64::INFINITY] {
        assert_eq!(
            MaterialForcePair::prepare(a, b, [0.; 3], [0.; 3], [0.; 3], duration, config()),
            Err(Error::InvalidInput)
        );
    }
    assert_eq!(
        MaterialForcePair::prepare(
            a,
            b,
            [f64::NAN, 0., 0.],
            [0.; 3],
            [0.; 3],
            duration,
            config()
        ),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        MaterialForcePair::prepare(
            a,
            b,
            [0.; 3],
            [0.; 3],
            [f64::INFINITY; 3],
            duration,
            config()
        ),
        Err(Error::InvalidInput)
    );
    let particle = ContactBody { spin: None, ..a };
    let pair = MaterialForcePair::prepare(
        particle,
        b,
        [0.; 3],
        [0.1, 0., 0.],
        [0.; 3],
        duration,
        config(),
    )
    .unwrap();
    assert!(pair.paths()[0].rotation().is_none());
    assert_eq!(
        pair.paths()[1]
            .rotation()
            .unwrap()
            .model_angular_momentum_error(),
        0.
    );
    assert!(pair.sample(-1.).is_err());
    assert!(pair.work(f64::NAN).is_err());
    assert!(pair.point(duration * 2.).is_err());
    assert_eq!([a, b], saved);
}

#[test]
fn pair_force_and_moment_are_covariant_under_world_frame_rotation_and_origin_shift() {
    let [a, b] = bodies();
    let local = [0.2, -0.1, 0.3];
    let force = [1., 3., -2.];
    let rate = [-4., 2., 1.];
    let duration = 0.08;
    let baseline =
        MaterialForcePair::prepare(a, b, local, force, rate, duration, config()).unwrap();
    let root = 3_f64.sqrt();
    let angle: f64 = 0.7;
    let frame = [
        (angle / 2.).sin() / root,
        (angle / 2.).sin() / root,
        (angle / 2.).sin() / root,
        (angle / 2.).cos(),
    ];
    let shift = [5., -7., 11.];
    let multiply = |a: [f64; 4], b: [f64; 4]| {
        let c = cross([a[0], a[1], a[2]], [b[0], b[1], b[2]]);
        [
            a[3] * b[0] + b[3] * a[0] + c[0],
            a[3] * b[1] + b[3] * a[1] + c[1],
            a[3] * b[2] + b[3] * a[2] + c[2],
            a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
        ]
    };
    let transform = |body: ContactBody| {
        let p = rotate(frame, body.motion.position);
        ContactBody {
            motion: Body {
                position: std::array::from_fn(|k| p[k] + shift[k]),
                velocity: rotate(frame, body.motion.velocity),
                ..body.motion
            },
            spin: body.spin.map(|s| Spin {
                orientation: multiply(frame, s.orientation),
                angular_momentum: rotate(frame, s.angular_momentum),
                ..s
            }),
        }
    };
    let moved = MaterialForcePair::prepare(
        transform(a),
        transform(b),
        local,
        rotate(frame, force),
        rotate(frame, rate),
        duration,
        config(),
    )
    .unwrap();
    for time in [1e-9, 0.03, duration] {
        let original = baseline.sample(time).unwrap();
        let actual = moved.sample(time).unwrap();
        for i in 0..2 {
            let expected = transform(original[i]);
            for k in 0..3 {
                assert!((actual[i].motion.position[k] - expected.motion.position[k]).abs() < 1e-11);
            }
            let bound = baseline.paths()[i]
                .rotation()
                .unwrap()
                .model_angular_error_rad()
                + moved.paths()[i]
                    .rotation()
                    .unwrap()
                    .model_angular_error_rad();
            assert!(
                distance(
                    actual[i].spin.unwrap().orientation,
                    expected.spin.unwrap().orientation
                ) <= bound + 1e-11
            );
            let delta: [f64; 3] = std::array::from_fn(|k| {
                actual[i].spin.unwrap().angular_momentum[k]
                    - expected.spin.unwrap().angular_momentum[k]
            });
            let bound = baseline.paths()[i]
                .rotation()
                .unwrap()
                .model_angular_momentum_error()
                + moved.paths()[i]
                    .rotation()
                    .unwrap()
                    .model_angular_momentum_error();
            assert!(delta[0].hypot(delta[1]).hypot(delta[2]) <= bound + 1e-10);
        }
        let (p, l) = momentum(original);
        let (actual_p, actual_l) = momentum(actual);
        let expected_p = rotate(frame, p);
        let expected_l = rotate(frame, l);
        let offset = cross(shift, expected_p);
        for k in 0..3 {
            assert!((actual_p[k] - expected_p[k]).abs() < 1e-11);
            assert!((actual_l[k] - expected_l[k] - offset[k]).abs() < 1e-9);
        }
    }
}
