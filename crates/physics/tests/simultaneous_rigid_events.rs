use physics::{
    astrophysics_spin::Spin,
    contact::{ContactBody, NormalContact},
    gravity::Body,
    liquid::{
        BodyGeometryHit, Config, ContactWitness, DynamicWorldConfig, Error, GeometryHit, Liquid,
        LiquidBodyWorld, Material, Particle, RigidGeometryHit, TranslatingBody,
    },
    rigid_motion::RigidMotion,
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
struct Wall;
impl LiquidBodyWorld for Wall {
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
        .step_with_rigid_body_world(0.02, &mut states, &Wall, Default::default(), 2, rotation())
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
