use physics::gravity::{Body, Gravity};
use physics::gravity_spheres::{Error, Simulation, Sphere};

fn sphere(mass: f64, x: f64, vx: f64) -> Sphere {
    Sphere {
        body: Body {
            mass,
            position: [x, 0.0, 0.0],
            velocity: [vx, 0.0, 0.0],
        },
        radius: 0.5,
        angular_velocity: [0.0; 3],
    }
}
fn simulation() -> Simulation {
    Simulation {
        gravity: Gravity {
            constant: 0.0,
            ..Gravity::default()
        },
        max_step: 1.0,
        ..Simulation::default()
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}

#[test]
fn high_speed_collision_does_not_tunnel() {
    let mut bodies = [sphere(1.0, -5.0, 1000.0), sphere(1.0, 5.0, -1000.0)];
    let report = simulation().step(&mut bodies, 0.01).unwrap();
    assert_eq!(report.contacts, 1);
    near(bodies[0].body.velocity[0], -1000.0);
    near(bodies[1].body.velocity[0], 1000.0);
    near(bodies[0].body.position[0], -6.0);
    near(bodies[1].body.position[0], 6.0);
}

#[test]
fn unequal_mass_elastic_collision_matches_analytic_solution() {
    let mut bodies = [sphere(2.0, -1.0, 3.0), sphere(1.0, 1.0, 0.0)];
    simulation().step(&mut bodies, 0.5).unwrap();
    near(bodies[0].body.velocity[0], 1.0);
    near(bodies[1].body.velocity[0], 4.0);
    let momentum = bodies
        .iter()
        .map(|s| s.body.mass * s.body.velocity[0])
        .sum::<f64>();
    near(momentum, 6.0);
    let energy = bodies
        .iter()
        .map(|s| 0.5 * s.body.mass * s.body.velocity[0].powi(2))
        .sum::<f64>();
    near(energy, 9.0);
}

#[test]
fn inelastic_contact_and_initial_overlap_resolve() {
    let mut bodies = [sphere(1.0, 0.0, 1.0), sphere(1.0, 0.0, -1.0)];
    Simulation {
        restitution: 0.0,
        ..simulation()
    }
    .step(&mut bodies, 0.1)
    .unwrap();
    near(bodies[0].body.position[0], -0.5);
    near(bodies[1].body.position[0], 0.5);
    near(bodies[0].body.velocity[0], 0.0);
    near(bodies[1].body.velocity[0], 0.0);
}

#[test]
fn friction_transfers_spin_and_preserves_total_angular_momentum() {
    let mut bodies = [sphere(1.0, -0.5, 1.0), sphere(1.0, 0.5, -1.0)];
    bodies[0].body.velocity[1] = 1.0;
    bodies[1].body.velocity[1] = -1.0;
    let angular_momentum = |s: &[Sphere]| {
        s.iter()
            .map(|b| {
                b.body.mass
                    * (b.body.position[0] * b.body.velocity[1]
                        - b.body.position[1] * b.body.velocity[0])
                    + 0.4 * b.body.mass * b.radius.powi(2) * b.angular_velocity[2]
            })
            .sum::<f64>()
    };
    let before = angular_momentum(&bodies);
    Simulation {
        friction: 0.5,
        ..simulation()
    }
    .step(&mut bodies, 0.1)
    .unwrap();
    near(angular_momentum(&bodies), before);
    assert!(bodies[0].angular_velocity[2].abs() > 0.0);
    assert!(bodies[0].body.velocity[1].abs() < 1.0);
}

#[test]
fn budget_failure_is_atomic() {
    let mut bodies = [sphere(1.0, -5.0, 1000.0), sphere(1.0, 5.0, -1000.0)];
    let before = bodies;
    assert_eq!(
        Simulation {
            max_contacts: 0,
            ..simulation()
        }
        .step(&mut bodies, 0.01),
        Err(Error::BudgetExceeded)
    );
    assert_eq!(bodies, before);
    assert_eq!(
        Simulation {
            max_substeps: 0,
            ..simulation()
        }
        .step(&mut bodies, 0.01),
        Err(Error::BudgetExceeded)
    );
    assert_eq!(bodies, before);
}

#[test]
fn finite_spheres_orbit_under_mutual_gravity() {
    let mut bodies = [sphere(1.0, -1.0, 0.0), sphere(1.0, 1.0, 0.0)];
    bodies[0].body.velocity[1] = -0.5;
    bodies[1].body.velocity[1] = 0.5;
    let solver = Simulation {
        gravity: Gravity {
            constant: 1.0,
            ..Gravity::default()
        },
        max_step: 0.002,
        ..simulation()
    };
    for _ in 0..10_000 {
        assert_eq!(solver.step(&mut bodies, 0.002).unwrap().contacts, 0);
        let distance = bodies[0]
            .body
            .position
            .iter()
            .zip(bodies[1].body.position)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!((distance - 2.0).abs() < 2e-6);
    }
}

#[test]
fn simultaneous_symmetric_impact_is_independent_of_body_order() {
    let original = [
        sphere(1.0, -2.0, 1.0),
        sphere(1.0, 0.0, 0.0),
        sphere(1.0, 2.0, -1.0),
    ];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut bodies = order.map(|index| original[index]);
        simulation().step(&mut bodies, 1.1).unwrap();
        for (body, index) in bodies.iter().zip(order) {
            near(body.body.velocity[0], [-1.0, 0.0, 1.0][index]);
            near(body.body.position[0], [-1.1, 0.0, 1.1][index]);
        }
    }
}

#[test]
fn simultaneous_contact_iteration_exhaustion_restores_all_bodies() {
    let mut bodies = [
        sphere(1.0, -1.0, 1.0),
        sphere(1.0, 0.0, 0.0),
        sphere(1.0, 1.0, -1.0),
    ];
    let before = bodies;
    assert_eq!(
        Simulation {
            max_contacts: 4,
            ..simulation()
        }
        .step(&mut bodies, 0.1),
        Err(Error::BudgetExceeded)
    );
    assert_eq!(bodies, before);
}

#[test]
fn simultaneous_frictional_impact_preserves_momenta_and_dissipates_energy() {
    let mut original = [
        sphere(2.0, -1.0, 1.0),
        sphere(1.0, 0.0, 0.0),
        sphere(3.0, 1.0, -1.0),
    ];
    original[0].body.velocity[1] = 0.7;
    original[2].body.velocity[1] = -0.4;
    let quantities = |bodies: &[Sphere]| {
        let mut momentum = [0.0; 3];
        let mut angular = 0.0;
        let mut energy = 0.0;
        for s in bodies {
            for k in 0..3 {
                momentum[k] += s.body.mass * s.body.velocity[k];
            }
            angular += s.body.mass
                * (s.body.position[0] * s.body.velocity[1]
                    - s.body.position[1] * s.body.velocity[0])
                + 0.4 * s.body.mass * s.radius.powi(2) * s.angular_velocity[2];
            energy += 0.5 * s.body.mass * s.body.velocity.iter().map(|v| v * v).sum::<f64>()
                + 0.2
                    * s.body.mass
                    * s.radius.powi(2)
                    * s.angular_velocity.iter().map(|v| v * v).sum::<f64>();
        }
        (momentum, angular, energy)
    };
    let before = quantities(&original);
    let mut reference = None;
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut bodies = order.map(|index| original[index]);
        Simulation {
            restitution: 0.5,
            friction: 0.4,
            ..simulation()
        }
        .step(&mut bodies, 0.01)
        .unwrap();
        let after = quantities(&bodies);
        for k in 0..3 {
            near(after.0[k], before.0[k]);
        }
        near(after.1, before.1);
        assert!(after.2 < before.2);
        let mut restored = original;
        for (body, index) in bodies.into_iter().zip(order) {
            restored[index] = body;
        }
        if let Some(expected) = reference {
            let expected: [Sphere; 3] = expected;
            for (a, b) in restored.iter().zip(expected) {
                for k in 0..3 {
                    near(a.body.position[k], b.body.position[k]);
                    near(a.body.velocity[k], b.body.velocity[k]);
                    near(a.angular_velocity[k], b.angular_velocity[k]);
                }
            }
        } else {
            reference = Some(restored);
        }
    }
}

#[test]
fn inelastic_simultaneous_impact_settles_without_repeated_zero_time_hits() {
    let mut bodies = [
        sphere(1.0, -1.0, 1.0),
        sphere(1.0, 0.0, 0.0),
        sphere(1.0, 1.0, -1.0),
    ];
    Simulation {
        restitution: 0.0,
        ..simulation()
    }
    .step(&mut bodies, 0.1)
    .unwrap();
    for body in bodies {
        near(body.body.velocity[0], 0.0);
    }
}

#[test]
fn overlapping_cluster_separates_independently_of_order_and_preserves_center_of_mass() {
    let original = [
        sphere(2.0, -0.7, 0.0),
        sphere(1.0, 0.0, 0.0),
        sphere(3.0, 0.6, 0.0),
    ];
    let center = original
        .iter()
        .map(|s| s.body.mass * s.body.position[0])
        .sum::<f64>();
    let mut reference: Option<[Sphere; 3]> = None;
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut bodies = order.map(|index| original[index]);
        simulation().step(&mut bodies, 0.01).unwrap();
        near(
            bodies
                .iter()
                .map(|s| s.body.mass * s.body.position[0])
                .sum::<f64>(),
            center,
        );
        for i in 0..3 {
            for j in i + 1..3 {
                assert!(
                    (bodies[i].body.position[0] - bodies[j].body.position[0]).abs()
                        >= 1.0 - 1.1e-12
                );
            }
        }
        let mut restored = original;
        for (body, index) in bodies.into_iter().zip(order) {
            restored[index] = body;
        }
        if let Some(expected) = reference {
            for (a, b) in restored.iter().zip(expected) {
                near(a.body.position[0], b.body.position[0]);
            }
        } else {
            reference = Some(restored);
        }
    }
}

#[test]
fn ambiguous_coincident_centers_and_overlap_budget_fail_atomically() {
    let mut bodies = [sphere(1.0, 0.0, 0.0), sphere(1.0, 0.0, 0.0)];
    let before = bodies;
    assert_eq!(
        simulation().step(&mut bodies, 0.1),
        Err(Error::InvalidInput)
    );
    assert_eq!(bodies, before);
    let mut cluster = [
        sphere(1.0, -0.7, 0.0),
        sphere(1.0, 0.0, 0.0),
        sphere(1.0, 0.7, 0.0),
    ];
    let before = cluster;
    assert_eq!(
        Simulation {
            max_contacts: 3,
            ..simulation()
        }
        .step(&mut cluster, 0.1),
        Err(Error::BudgetExceeded)
    );
    assert_eq!(cluster, before);
}

#[test]
fn tetrahedral_contact_group_has_symmetric_elastic_rebound_in_three_dimensions() {
    let a = 1.0 / 8.0_f64.sqrt();
    let positions = [[a, a, a], [a, -a, -a], [-a, a, -a], [-a, -a, a]];
    let original = positions.map(|position| Sphere {
        body: Body {
            mass: 1.0,
            position,
            velocity: position.map(|v| -v),
        },
        radius: 0.5,
        angular_velocity: [0.0; 3],
    });
    for order in [[0, 1, 2, 3], [3, 2, 1, 0], [1, 3, 0, 2], [2, 0, 3, 1]] {
        let mut bodies = order.map(|index| original[index]);
        simulation().step(&mut bodies, 0.1).unwrap();
        let energy = bodies
            .iter()
            .map(|s| 0.5 * s.body.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>();
        near(energy, 0.75);
        for (body, index) in bodies.iter().zip(order) {
            for k in 0..3 {
                near(body.body.velocity[k], positions[index][k]);
                near(body.body.position[k], 1.1 * positions[index][k]);
            }
        }
    }
}
