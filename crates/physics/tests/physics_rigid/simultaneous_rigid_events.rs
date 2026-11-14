use physics::{
    contact::{ContactBody, NormalContact},
    gravity::Body,
    liquid::{
        BodyGeometryHit, Config, ContactWitness, DynamicWorldConfig, Error, GeometryHit, Liquid,
        LiquidBodyWorld, Material, Particle, RigidGeometryHit, TranslatingBody,
    },
    rigid_motion::RigidMotion,
    rotation::Spin,
    spin_path,
};
struct Faces {
    fail_second_patch: bool,
}
impl LiquidBodyWorld for Faces {
    fn sweep_particle_body(
        &self,
        _: &Particle,
        _: f64,
        _: usize,
        _: &TranslatingBody,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Err(Error::CollisionBackend)
    }
    fn sweep_body_pair(
        &self,
        _: usize,
        _: &TranslatingBody,
        _: usize,
        _: &TranslatingBody,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Err(Error::CollisionBackend)
    }
    fn sweep_particle_environment(
        &self,
        _: &Particle,
        _: f64,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Ok(GeometryHit::Clear)
    }
    fn sweep_body_environment(
        &self,
        _: usize,
        _: &TranslatingBody,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Ok(GeometryHit::Clear)
    }
    fn sweep_rigid_pair_event(
        &self,
        i: usize,
        a: &RigidMotion,
        j: usize,
        b: &RigidMotion,
        _: usize,
    ) -> Result<RigidGeometryHit, Error> {
        let relative = a.initial().motion.velocity[0] - b.initial().motion.velocity[0];
        if j != i + 1 || relative < 1. {
            return Ok(BodyGeometryHit::from(GeometryHit::Clear).into());
        }
        let time =
            (b.initial().motion.position[0] - a.initial().motion.position[0] - 0.1) / relative;
        if !(0. ..=a.duration()).contains(&time) {
            return Ok(BodyGeometryHit::from(GeometryHit::Clear).into());
        }
        let point = a.sample(time).unwrap().motion.position[0] + 0.05;
        Ok(RigidGeometryHit {
            contact: BodyGeometryHit {
                geometry: GeometryHit::Contact {
                    fraction: time / a.duration(),
                    normal: [-1., 0., 0.],
                },
                witness: Some(ContactWitness {
                    point: [point, 0., 0.],
                    tolerance_m: 1e-12,
                }),
            },
            feature: Some(((i as u64) << 32) | j as u64),
        })
    }
    fn rigid_pair_patch_for_feature(
        &self,
        i: usize,
        _: &ContactBody,
        j: usize,
        _: &ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        _: usize,
    ) -> Result<Vec<NormalContact>, Error> {
        assert_eq!(feature, Some(((i as u64) << 32) | j as u64));
        if self.fail_second_patch && i == 1 {
            return Err(Error::CollisionBackend);
        }
        Ok([-0.02, 0.02]
            .into_iter()
            .flat_map(|y| {
                [-0.02, 0.02].map(|z| NormalContact {
                    point: [witness.point[0], y, z],
                    normal,
                })
            })
            .collect())
    }
    fn has_environment(&self) -> bool {
        false
    }
}
fn bodies() -> [ContactBody; 3] {
    [(-0.13, 3.), (0., 0.), (0.13, -3.)].map(|(x, v)| ContactBody {
        motion: Body {
            mass: 1.,
            position: [x, 0., 0.],
            velocity: [v, 0., 0.],
        },
        spin: Some(Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0.; 3],
            inertia: [1.; 3],
        }),
    })
}
fn fluid() -> Liquid {
    Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap()
}
fn rotation() -> spin_path::Config {
    spin_path::Config {
        max_angular_error_rad: 1e-5,
        min_step_s: 1e-9,
        max_arcs: 10000,
        max_trials: 30000,
    }
}
#[test]
fn shared_event_loop_keeps_both_feature_tokens_and_couples_three_bodies() {
    let mut liquid = fluid();
    let mut states = bodies();
    let result = liquid
        .step_with_rigid_body_world(
            0.02,
            &mut states,
            &Faces {
                fail_second_patch: false,
            },
            Default::default(),
            3,
            rotation(),
        )
        .unwrap();
    assert_eq!(result.dynamics.contacts, 2);
    assert!((result.dynamics.dissipated_energy - 9.).abs() < 1e-11);
    assert_eq!(result.environment_impulse, [0.; 3]);
    for (body, x) in states.iter().zip([-0.1, 0., 0.1]) {
        assert!(body.motion.velocity.iter().all(|v| v.abs() < 1e-12));
        assert!(
            body.spin
                .unwrap()
                .angular_momentum
                .iter()
                .all(|v| v.abs() < 1e-12)
        );
        assert!((body.motion.position[0] - x).abs() < 1e-14);
    }
}
#[test]
fn late_patch_failure_and_complete_group_contact_budget_restore_every_body() {
    for (world, config, expected) in [
        (
            Faces {
                fail_second_patch: true,
            },
            DynamicWorldConfig::default(),
            Error::CollisionBackend,
        ),
        (
            Faces {
                fail_second_patch: false,
            },
            DynamicWorldConfig {
                max_contacts: 1,
                ..Default::default()
            },
            Error::CollisionBudget,
        ),
        (
            Faces {
                fail_second_patch: false,
            },
            DynamicWorldConfig {
                max_queries: 4,
                ..Default::default()
            },
            Error::CollisionBudget,
        ),
    ] {
        let mut liquid = fluid();
        let before_fluid = liquid.clone();
        let mut states = bodies();
        let before = states;
        assert_eq!(
            liquid.step_with_rigid_body_world(0.02, &mut states, &world, config, 3, rotation()),
            Err(expected)
        );
        assert_eq!(states, before);
        assert_eq!(liquid.particles(), before_fluid.particles());
    }
}
/// Fixed world face combined with a finite-body event at the same reported time.
/// This fixture qualifies mechanics/ledger wiring, not scene geometry clearance.
struct Wall<'a> {
    moving_queries: Option<&'a std::cell::RefCell<Vec<f64>>>,
}
impl LiquidBodyWorld for Wall<'_> {
    fn sweep_particle_body(
        &self,
        _: &Particle,
        _: f64,
        _: usize,
        _: &TranslatingBody,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Err(Error::CollisionBackend)
    }
    fn sweep_body_pair(
        &self,
        _: usize,
        _: &TranslatingBody,
        _: usize,
        _: &TranslatingBody,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Err(Error::CollisionBackend)
    }
    fn sweep_particle_environment(
        &self,
        _: &Particle,
        _: f64,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Ok(GeometryHit::Clear)
    }
    fn sweep_body_environment(
        &self,
        _: usize,
        _: &TranslatingBody,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Err(Error::CollisionBackend)
    }
    fn sweep_rigid_pair_event(
        &self,
        i: usize,
        a: &RigidMotion,
        j: usize,
        b: &RigidMotion,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
        Faces {
            fail_second_patch: false,
        }
        .sweep_rigid_pair_event(i, a, j, b, budget)
    }
    fn rigid_pair_patch_for_feature(
        &self,
        i: usize,
        a: &ContactBody,
        j: usize,
        b: &ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        budget: usize,
    ) -> Result<Vec<NormalContact>, Error> {
        Faces {
            fail_second_patch: false,
        }
        .rigid_pair_patch_for_feature(i, a, j, b, witness, normal, feature, budget)
    }
    fn sweep_rigid_environment_event(
        &self,
        i: usize,
        a: &RigidMotion,
        _: usize,
    ) -> Result<RigidGeometryHit, Error> {
        if i != 1 || a.initial().motion.velocity[0] < 1. {
            return Ok(BodyGeometryHit::from(GeometryHit::Clear).into());
        }
        let time = (0.13 - 0.1) / 3.;
        Ok(RigidGeometryHit {
            contact: BodyGeometryHit {
                geometry: GeometryHit::Contact {
                    fraction: time / a.duration(),
                    normal: [-1., 0., 0.],
                },
                witness: Some(ContactWitness {
                    point: [a.sample(time).unwrap().motion.position[0] + 0.05, 0., 0.],
                    tolerance_m: 1e-12,
                }),
            },
            feature: Some(99),
        })
    }
    fn sweep_rigid_environment_event_at(
        &self,
        i: usize,
        path: &RigidMotion,
        elapsed_s: f64,
        budget: usize,
    ) -> Result<RigidGeometryHit, Error> {
        let Some(queries) = self.moving_queries else {
            return self.sweep_rigid_environment_event(i, path, budget);
        };
        queries.borrow_mut().push(elapsed_s);
        let start = path.initial().motion;
        let closing_speed = 3. - start.velocity[0];
        let gap = start.position[0] - 0.05 - 3. * elapsed_s;
        if closing_speed <= 1e-12 {
            return Ok(BodyGeometryHit::from(GeometryHit::Clear).into());
        }
        let time = gap / closing_speed;
        if !(0. ..=path.duration()).contains(&time) {
            return Ok(BodyGeometryHit::from(GeometryHit::Clear).into());
        }
        Ok(RigidGeometryHit {
            contact: BodyGeometryHit {
                geometry: GeometryHit::Contact {
                    fraction: time / path.duration(),
                    normal: [1., 0., 0.],
                },
                witness: Some(ContactWitness {
                    point: [
                        3. * (elapsed_s + time),
                        start.position[1],
                        start.position[2],
                    ],
                    tolerance_m: 1e-12,
                }),
            },
            feature: Some(99),
        })
    }

    fn rigid_environment_impact_velocity(
        &self,
        _: usize,
        feature: Option<u64>,
        time_s: f64,
    ) -> Result<[f64; 3], Error> {
        if self.moving_queries.is_some() {
            assert_eq!(feature, Some(99));
            assert!((time_s - 0.05).abs() < 1e-12);
            Ok([3., 0., 0.])
        } else {
            Ok([0.; 3])
        }
    }

    fn rigid_environment_patch_at(
        &self,
        index: usize,
        body: &ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        time_s: f64,
        budget: usize,
    ) -> Result<Vec<NormalContact>, Error> {
        if self.moving_queries.is_some() {
            assert!((time_s - 0.05).abs() < 1e-12);
            assert!((witness.point[0] - 3. * time_s).abs() < 1e-12);
        }
        self.rigid_environment_patch_for_feature(index, body, witness, normal, feature, budget)
    }

    fn rigid_environment_patch_for_feature(
        &self,
        _: usize,
        _: &ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        feature: Option<u64>,
        _: usize,
    ) -> Result<Vec<NormalContact>, Error> {
        assert_eq!(feature, Some(99));
        Ok([-0.02, 0.02]
            .into_iter()
            .flat_map(|y| {
                [-0.02, 0.02].map(|z| NormalContact {
                    point: [witness.point[0], y, z],
                    normal,
                })
            })
            .collect())
    }
}
#[test]
fn simultaneous_body_and_wall_impulses_close_the_boundary_energy_ledger() {
    let mut liquid = fluid();
    let mut states = [bodies()[0], bodies()[1]];
    states[0].motion.velocity[0] = 6.;
    states[1].motion.velocity[0] = 3.;
    let report = liquid
        .step_with_rigid_body_world(
            0.02,
            &mut states,
            &Wall {
                moving_queries: None,
            },
            Default::default(),
            2,
            rotation(),
        )
        .unwrap();
    assert_eq!(report.dynamics.contacts, 2);
    assert!((report.environment_impulse[0] - 9.).abs() < 1e-11);
    assert_eq!(report.environment_impulse[1..], [0.; 2]);
    assert!((report.dynamics.dissipated_energy - 22.5).abs() < 1e-11);
    let tolerance = 128. * f64::EPSILON * 6.;
    // The solver bounds normal point speed. For this narrow patch that bounds
    // residual angular speed by the velocity tolerance divided by its lever arm.
    for state in states {
        assert!(state.motion.velocity.iter().all(|v| v.abs() < 1e-12));
        assert!(
            state
                .spin
                .unwrap()
                .angular_momentum
                .iter()
                .all(|v| v.abs() <= 2. * tolerance / 0.02)
        );
    }
    for y in [-0.02, 0.02] {
        for z in [-0.02, 0.02] {
            let pair = [-0.02, y, z];
            let va = states[0].point_velocity(pair).unwrap()[0];
            let vb = states[1].point_velocity(pair).unwrap()[0];
            assert!((va - vb).abs() <= tolerance);
            assert!(states[1].point_velocity([0.08, y, z]).unwrap()[0].abs() <= tolerance);
        }
    }
}

#[test]
fn moving_boundary_impulse_uses_impact_time_and_advances_remaining_interval() {
    for spin in [None, bodies()[0].spin] {
        let mut liquid = fluid();
        let mut states = [ContactBody {
            motion: Body {
                mass: 2.,
                position: [0.2, 0., 0.],
                velocity: [0.; 3],
            },
            spin,
        }];
        let queries = std::cell::RefCell::new(Vec::new());
        let report = liquid
            .step_with_rigid_body_forces(
                0.1,
                &mut states,
                &Wall {
                    moving_queries: Some(&queries),
                },
                Default::default(),
                1,
                rotation(),
                &[Default::default()],
            )
            .unwrap();
        assert_eq!(report.world.dynamics.contacts, 1);
        assert!((states[0].motion.position[0] - 0.35).abs() < 1e-11);
        assert!((states[0].motion.velocity[0] - 3.).abs() < 1e-11);
        assert!((report.world.environment_impulse[0] + 6.).abs() < 1e-11);
        assert!((report.boundary_impact_work - 18.).abs() < 1e-11);
        assert!((report.world.dynamics.dissipated_energy - 9.).abs() < 1e-11);
        assert_eq!(queries.borrow().len(), 2);
        assert_eq!(queries.borrow()[0], 0.);
        assert!((queries.borrow()[1] - 0.05).abs() < 1e-12);
    }
}

#[test]
fn simultaneous_moving_boundary_contacts_account_for_each_body() {
    let mut liquid = fluid();
    let mut states = [0., 1.].map(|y| ContactBody {
        motion: Body {
            mass: 2.,
            position: [0.2, y, 0.],
            velocity: [0.; 3],
        },
        spin: None,
    });
    let queries = std::cell::RefCell::new(Vec::new());
    let report = liquid
        .step_with_rigid_body_forces(
            0.1,
            &mut states,
            &Wall {
                moving_queries: Some(&queries),
            },
            Default::default(),
            2,
            rotation(),
            &[Default::default(); 2],
        )
        .unwrap();
    assert_eq!(report.world.dynamics.contacts, 2);
    assert!((report.boundary_impact_work - 36.).abs() < 1e-11);
    assert!((report.world.dynamics.dissipated_energy - 18.).abs() < 1e-11);
    assert!((report.world.environment_impulse[0] + 12.).abs() < 1e-11);
    assert!(
        states
            .iter()
            .all(|state| (state.motion.position[0] - 0.35).abs() < 1e-11)
    );
}
