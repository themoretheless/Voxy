use physics::{
    contact::{
        ContactBody, ContactWrench, Error, ManifoldConfig, NetworkContact, NetworkSupport,
        NormalContact, NormalSupport, ReactionConfig, SupportPlane, resolve_normal_contact_network,
        resolve_normal_reaction_network, resolve_normal_reactions,
    },
    gravity::Body,
};
fn body(mass: f64, position: [f64; 3], velocity: [f64; 3]) -> ContactBody {
    ContactBody {
        motion: Body {
            mass,
            position,
            velocity,
        },
        spin: None,
    }
}
fn reaction_config() -> ReactionConfig {
    ReactionConfig {
        max_sweeps: 1000,
        acceleration_tolerance: 1e-11,
        normal_velocity_tolerance: 1e-10,
    }
}
fn stack() -> ([ContactBody; 3], [NetworkSupport; 3], [ContactWrench; 3]) {
    let bodies = [
        body(2., [0., 2., 0.], [0.; 3]),
        body(3., [0., 1., 0.], [0.; 3]),
        body(5., [0.; 3], [0.; 3]),
    ];
    let supports = std::array::from_fn(|i| NetworkSupport {
        first: i,
        second: (i < 2).then_some(i + 1),
        support: NormalSupport {
            contact: NormalContact {
                point: [0., 1.5 - i as f64, 0.],
                normal: [0., 1., 0.],
            },
            plane: if i < 2 {
                SupportPlane::Second
            } else {
                SupportPlane::World
            },
        },
    });
    let external = bodies.map(|b| ContactWrench {
        force: [0., -10. * b.motion.mass, 0.],
        torque: [0.; 3],
    });
    (bodies, supports, external)
}
#[test]
fn stack_transmits_complete_weight_through_all_shared_bodies() {
    let (bodies, contacts, external) = stack();
    let before = bodies;
    let result =
        resolve_normal_reaction_network(&bodies, &contacts, &external, reaction_config()).unwrap();
    assert_eq!(before, bodies);
    for (force, expected) in result.forces.iter().zip([20., 50., 100.]) {
        assert!((force[1] - expected).abs() < 2e-10);
        assert_eq!(force[0], 0.);
        assert_eq!(force[2], 0.);
    }
    for (reaction, load) in result.wrenches.iter().zip(external) {
        assert!((reaction.force[1] + load.force[1]).abs() < 5e-11);
        assert_eq!(reaction.torque, [0.; 3]);
    }
    assert!(result.normal_accelerations.iter().all(|g| g.abs() < 1e-11));
    assert!(result.acceleration_residual <= 1e-11);
    assert_eq!(result.instantaneous_power, 0.);
    // An isolated pair falls freely together and cannot transmit the floor load.
    let isolated = resolve_normal_reactions(
        &bodies[0],
        Some(&bodies[1]),
        &[contacts[0].support],
        external[0],
        Some(external[1]),
        reaction_config(),
    )
    .unwrap();
    assert_eq!(isolated.forces, vec![[0.; 3]]);
}
#[test]
fn body_permutation_contact_order_and_reciprocal_orientation_preserve_stack() {
    let (bodies, contacts, external) = stack();
    let baseline =
        resolve_normal_reaction_network(&bodies, &contacts, &external, reaction_config()).unwrap();
    let order = [2, 0, 1];
    let map = [1, 2, 0];
    let states = order.map(|i| bodies[i]);
    let loads = order.map(|i| external[i]);
    let mut indexed = contacts.map(|entry| NetworkSupport {
        first: map[entry.first],
        second: entry.second.map(|j| map[j]),
        ..entry
    });
    indexed.reverse();
    // Swap the two finite participants of the upper support; the face owner
    // follows its body and force changes sign, while physical wrenches agree.
    let entry = &mut indexed[2];
    let first = entry.first;
    entry.first = entry.second.unwrap();
    entry.second = Some(first);
    entry.support.contact.normal = entry.support.contact.normal.map(|n| -n);
    entry.support.plane = SupportPlane::First;
    let result =
        resolve_normal_reaction_network(&states, &indexed, &loads, reaction_config()).unwrap();
    for (i, original) in order.into_iter().enumerate() {
        for k in 0..3 {
            assert!(
                (result.wrenches[i].force[k] - baseline.wrenches[original].force[k]).abs() < 2e-10
            );
            assert!(
                (result.wrenches[i].torque[k] - baseline.wrenches[original].torque[k]).abs()
                    < 2e-10
            );
        }
    }
}
#[test]
fn simultaneous_inelastic_chain_conserves_momentum_and_measures_energy_loss() {
    let mut bodies = [
        body(1., [-1., 0., 0.], [3., 0., 0.]),
        body(1., [0.; 3], [0.; 3]),
        body(1., [1., 0., 0.], [0.; 3]),
    ];
    let contacts = [0, 1].map(|i| NetworkContact {
        first: i,
        second: Some(i + 1),
        contact: NormalContact {
            point: [-0.5 + i as f64, 0., 0.],
            normal: [-1., 0., 0.],
        },
    });
    let report = resolve_normal_contact_network(
        &mut bodies,
        &contacts,
        ManifoldConfig {
            max_sweeps: 100,
            velocity_tolerance: 1e-11,
        },
    )
    .unwrap();
    for body in bodies {
        assert!((body.motion.velocity[0] - 1.).abs() < 1e-11);
    }
    assert!((report.impulses[0][0] + 2.).abs() < 1e-11);
    assert!((report.impulses[1][0] + 1.).abs() < 1e-11);
    assert!((report.kinetic_energy_change + 3.).abs() < 1e-11);
}
#[test]
fn network_invalid_indices_inputs_and_late_budget_preserve_all_states() {
    let (bodies, contacts, external) = stack();
    let before = bodies;
    let mut config = reaction_config();
    config.max_sweeps = 1;
    assert_eq!(
        resolve_normal_reaction_network(&bodies, &contacts, &external, config),
        Err(Error::Budget)
    );
    assert_eq!(bodies, before);
    for (first, second) in [(99, None), (0, Some(99)), (0, Some(0))] {
        let mut bad = contacts;
        bad[2].first = first;
        bad[2].second = second;
        assert_eq!(
            resolve_normal_reaction_network(&bodies, &bad, &external, reaction_config()),
            Err(Error::InvalidInput)
        );
    }
    let mut bad_load = external;
    bad_load[2].force[1] = f64::NAN;
    assert_eq!(
        resolve_normal_reaction_network(&bodies, &contacts, &bad_load, reaction_config()),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        resolve_normal_reaction_network(&bodies, &contacts, &external[..2], reaction_config()),
        Err(Error::InvalidInput)
    );
    let mut moving = bodies;
    moving[0].motion.velocity[1] = -3.;
    let original = moving;
    let points = contacts.map(|entry| NetworkContact {
        first: entry.first,
        second: entry.second,
        contact: entry.support.contact,
    });
    assert_eq!(
        resolve_normal_contact_network(
            &mut moving,
            &points,
            ManifoldConfig {
                max_sweeps: 1,
                velocity_tolerance: 1e-11
            }
        ),
        Err(Error::Budget)
    );
    assert_eq!(moving, original);
    let mut bad_points = points;
    bad_points[2].first = 99;
    assert_eq!(
        resolve_normal_contact_network(
            &mut moving,
            &bad_points,
            ManifoldConfig {
                max_sweeps: 100,
                velocity_tolerance: 1e-11
            }
        ),
        Err(Error::InvalidInput)
    );
    assert_eq!(moving, original);
}
#[test]
fn disconnected_and_outgoing_supports_do_not_receive_unrelated_loads() {
    let (mut bodies, contacts, mut external) = stack();
    bodies[0].motion.velocity[1] = 1.;
    let result =
        resolve_normal_reaction_network(&bodies, &contacts, &external, reaction_config()).unwrap();
    assert_eq!(result.forces[0], [0.; 3]);
    assert!((result.forces[1][1] - 30.).abs() < 2e-10);
    assert!((result.forces[2][1] - 80.).abs() < 2e-10);
    external[0].force = [0., -1000., 0.];
    let changed =
        resolve_normal_reaction_network(&bodies, &contacts, &external, reaction_config()).unwrap();
    assert_eq!(changed.forces, result.forces);
    assert_eq!(changed.wrenches, result.wrenches);
}

#[test]
fn an_eccentric_load_distributes_force_and_torque_between_two_finite_supports() {
    use physics::astrophysics_spin::Spin;
    let mut beam = body(2., [0., 1., 0.], [0.; 3]);
    let (s, c) = (0.23_f64 / 2.).sin_cos();
    beam.spin = Some(Spin {
        orientation: [0., s, 0., c],
        angular_momentum: [0.; 3],
        inertia: [2., 3., 4.],
    });
    let bodies = [
        beam,
        body(3., [-1., 0., 0.], [0.; 3]),
        body(5., [1., 0., 0.], [0.; 3]),
    ];
    let contacts = [
        (0, Some(1), [-1., 0.5, 0.]),
        (0, Some(2), [1., 0.5, 0.]),
        (1, None, [-1., -0.5, 0.]),
        (2, None, [1., -0.5, 0.]),
    ]
    .map(|(first, second, point)| NetworkSupport {
        first,
        second,
        support: NormalSupport {
            contact: NormalContact {
                point,
                normal: [0., 1., 0.],
            },
            plane: if second.is_some() {
                SupportPlane::Second
            } else {
                SupportPlane::World
            },
        },
    });
    let external = [
        ContactWrench {
            force: [0., -20., 0.],
            torque: [0., 0., 6.],
        },
        ContactWrench {
            force: [0., -30., 0.],
            torque: [0.; 3],
        },
        ContactWrench {
            force: [0., -50., 0.],
            torque: [0.; 3],
        },
    ];
    let result =
        resolve_normal_reaction_network(&bodies, &contacts, &external, reaction_config()).unwrap();
    for (force, expected) in result.forces.iter().zip([13., 7., 43., 57.]) {
        assert!((force[1] - expected).abs() < 3e-10);
    }
    for (reaction, load) in result.wrenches.iter().zip(external) {
        for k in 0..3 {
            assert!((reaction.force[k] + load.force[k]).abs() < 3e-10);
            assert!((reaction.torque[k] + load.torque[k]).abs() < 3e-10);
        }
    }
    assert!(result.normal_accelerations.iter().all(|g| g.abs() < 1e-11));
    // A proper cyclic rotation changes world inertia but not the physical solve.
    let rotate = |v: [f64; 3]| [v[2], v[0], v[1]];
    let rotated = bodies.map(|mut b| {
        b.motion.position = rotate(b.motion.position);
        b.motion.velocity = rotate(b.motion.velocity);
        if let Some(spin) = &mut b.spin {
            spin.orientation = [0.5 * (c - s), 0.5 * (c + s), 0.5 * (c + s), 0.5 * (c - s)];
            spin.angular_momentum = rotate(spin.angular_momentum);
        }
        b
    });
    let supports = contacts.map(|mut entry| {
        entry.support.contact.point = rotate(entry.support.contact.point);
        entry.support.contact.normal = rotate(entry.support.contact.normal);
        entry
    });
    let loads = external.map(|w| ContactWrench {
        force: rotate(w.force),
        torque: rotate(w.torque),
    });
    let transformed =
        resolve_normal_reaction_network(&rotated, &supports, &loads, reaction_config()).unwrap();
    for (actual, original) in transformed.wrenches.iter().zip(&result.wrenches) {
        for k in 0..3 {
            assert!((actual.force[k] - rotate(original.force)[k]).abs() < 4e-10);
            assert!((actual.torque[k] - rotate(original.torque)[k]).abs() < 4e-10);
        }
    }
}
#[test]
fn disconnected_high_energy_body_does_not_erase_measured_collision_loss() {
    let mut bodies = [
        body(1., [0.; 3], [3., 0., 0.]),
        body(1., [1., 0., 0.], [0.; 3]),
        body(1., [100.; 3], [1e100, 0., 0.]),
    ];
    let distant = bodies[2];
    let contact = NetworkContact {
        first: 0,
        second: Some(1),
        contact: NormalContact {
            point: [0.5, 0., 0.],
            normal: [-1., 0., 0.],
        },
    };
    let report = resolve_normal_contact_network(
        &mut bodies,
        &[contact],
        ManifoldConfig {
            max_sweeps: 100,
            velocity_tolerance: 1e-11,
        },
    )
    .unwrap();
    assert_eq!(bodies[2], distant);
    assert_eq!(bodies[0].motion.velocity, [1.5, 0., 0.]);
    assert_eq!(bodies[1].motion.velocity, [1.5, 0., 0.]);
    assert_eq!(report.kinetic_energy_change, -2.25);
}
#[test]
fn explicit_network_limits_and_duplicate_constraints_are_admitted_without_panics() {
    let bodies = vec![body(1., [0.; 3], [0.; 3]); 128];
    let mut external = vec![ContactWrench::default(); 128];
    external[127].force = [0., -10., 0.];
    let entry = NetworkSupport {
        first: 127,
        second: None,
        support: NormalSupport {
            contact: NormalContact {
                point: [0.; 3],
                normal: [0., 1., 0.],
            },
            plane: SupportPlane::World,
        },
    };
    let contacts = vec![entry; 128];
    let result =
        resolve_normal_reaction_network(&bodies, &contacts, &external, reaction_config()).unwrap();
    assert_eq!(result.wrenches[127].force, [0., 10., 0.]);
    assert!(
        result.wrenches[..127]
            .iter()
            .all(|w| *w == ContactWrench::default())
    );
    assert_eq!(result.forces.iter().map(|f| f[1]).sum::<f64>(), 10.);
    assert_eq!(
        resolve_normal_reaction_network(&bodies, &vec![entry; 129], &external, reaction_config()),
        Err(Error::InvalidInput)
    );
    let too_many = vec![bodies[0]; 129];
    assert_eq!(
        resolve_normal_reaction_network(
            &too_many,
            &[entry],
            &vec![ContactWrench::default(); 129],
            reaction_config()
        ),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        resolve_normal_reaction_network(&bodies, &[], &external, reaction_config()),
        Err(Error::InvalidInput)
    );
    let indexed = NetworkContact {
        first: 127,
        second: None,
        contact: entry.support.contact,
    };
    let mut staged = bodies.clone();
    let before = staged.clone();
    assert_eq!(
        resolve_normal_contact_network(
            &mut staged,
            &vec![indexed; 129],
            ManifoldConfig {
                max_sweeps: 100,
                velocity_tolerance: 1e-11
            }
        ),
        Err(Error::InvalidInput)
    );
    assert_eq!(staged, before);
}
