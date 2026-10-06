use physics::{
    astrophysics_spin::Spin,
    contact::ContactBody,
    gravity::Body,
    liquid::{
        BodyGeometryHit, Config, ContactConfig, ContactWitness, DynamicWorldConfig, Error,
        GeometryHit, Liquid, LiquidBodyWorld, Material, Particle, TranslatingBody,
    },
    spin_path,
};
fn rotation() -> spin_path::Config {
    spin_path::Config {
        max_angular_error_rad: 1e-5,
        min_step_s: 1e-9,
        max_arcs: 10000,
        max_trials: 30000,
    }
}
fn config() -> DynamicWorldConfig {
    DynamicWorldConfig {
        contact: ContactConfig {
            restitution: 1.,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn fluid(particles: Vec<Particle>) -> Liquid {
    Liquid::new(
        particles,
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            max_particles: 8,
            ..Default::default()
        },
    )
    .unwrap()
}
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
/// Controlled affine face event: first face at x=center+half; second face at x=0.
/// Initial orientations are identity. After impact the faces separate over this
/// short fixture; the callback checks the new Spin state before declaring clear.
struct Faces {
    late_failure: bool,
    missing_witness: bool,
}
impl Faces {
    fn event(
        &self,
        x: f64,
        v: f64,
        y: f64,
        half: f64,
        duration: f64,
        spin: Option<Spin>,
    ) -> Result<BodyGeometryHit, Error> {
        if v <= 2. {
            if self.late_failure {
                return Err(Error::CollisionBackend);
            }
            assert!(spin.unwrap().angular_momentum[2] < 0.);
            return Ok(GeometryHit::Clear.into());
        }
        let time = (-half - x) / v;
        if time < 0. || time > duration {
            return Ok(GeometryHit::Clear.into());
        }
        Ok(BodyGeometryHit {
            geometry: GeometryHit::Contact {
                fraction: time / duration,
                normal: [-1., 0., 0.],
            },
            witness: (!self.missing_witness).then_some(ContactWitness {
                point: [0., y, 0.],
                tolerance_m: 1e-12,
            }),
        })
    }
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
        Err(Error::CollisionBackend)
    }
    fn sweep_particle_rigid_contact(
        &self,
        p: &Particle,
        r: f64,
        _: usize,
        b: &physics::rigid_motion::RigidMotion,
        _: usize,
    ) -> Result<BodyGeometryHit, Error> {
        self.event(
            p.position[0],
            p.velocity[0],
            p.position[1],
            r,
            b.duration(),
            b.initial().spin,
        )
    }
    fn sweep_rigid_pair_contact(
        &self,
        _: usize,
        a: &physics::rigid_motion::RigidMotion,
        _: usize,
        b: &physics::rigid_motion::RigidMotion,
        _: usize,
    ) -> Result<BodyGeometryHit, Error> {
        self.event(
            a.initial().motion.position[0],
            a.initial().motion.velocity[0],
            a.initial().motion.position[1],
            0.04,
            a.duration(),
            b.initial().spin,
        )
    }
    fn rigid_pair_patch(
        &self,
        _: usize,
        _: &ContactBody,
        _: usize,
        _: &ContactBody,
        witness: ContactWitness,
        normal: [f64; 3],
        _: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, Error> {
        if self.late_failure {
            return Ok(vec![physics::contact::NormalContact {
                point: [f64::NAN, 0., 0.],
                normal,
            }]);
        }
        Ok([(-0.02, -0.02), (0.02, -0.02), (-0.02, 0.02), (0.02, 0.02)]
            .map(|(y, z)| physics::contact::NormalContact {
                point: [witness.point[0], witness.point[1] + y, witness.point[2] + z],
                normal,
            })
            .to_vec())
    }
    fn sweep_rigid_pair_event(
        &self,
        i: usize,
        a: &physics::rigid_motion::RigidMotion,
        j: usize,
        b: &physics::rigid_motion::RigidMotion,
        budget: usize,
    ) -> Result<physics::liquid::RigidGeometryHit, Error> {
        let contact = self.sweep_rigid_pair_contact(i, a, j, b, budget)?;
        let feature = matches!(contact.geometry, GeometryHit::Contact { .. }).then_some(99);
        Ok(physics::liquid::RigidGeometryHit { contact, feature })
    }
    fn rigid_pair_patch_for_feature(
        &self,
        i: usize,
        a: &ContactBody,
        j: usize,
        b: &ContactBody,
        w: ContactWitness,
        n: [f64; 3],
        feature: Option<u64>,
        budget: usize,
    ) -> Result<Vec<physics::contact::NormalContact>, Error> {
        assert_eq!(
            feature,
            Some(99),
            "earliest event must retain its geometry-owned key"
        );
        self.rigid_pair_patch(i, a, j, b, w, n, budget)
    }
    fn has_environment(&self) -> bool {
        false
    }
}
#[test]
fn coupled_body_impact_retains_spin_and_advances_the_post_impact_orientation() {
    let mut liquid = fluid(Vec::new());
    let mut bodies = [
        body([-0.1, 1., 0.], [3., 0., 0.], 0.),
        body([0.; 3], [0.; 3], 0.),
    ];
    let initial = bodies;
    let angular = |b: ContactBody| {
        b.motion.position[0] * b.motion.velocity[1] - b.motion.position[1] * b.motion.velocity[0]
            + b.spin.unwrap().angular_momentum[2]
    };
    let before_l = angular(bodies[0]) + angular(bodies[1]);
    let before_e = bodies.iter().map(|b| b.energy().unwrap()).sum::<f64>();
    let report = liquid
        .step_with_rigid_body_world(
            0.1,
            &mut bodies,
            &Faces {
                late_failure: false,
                missing_witness: false,
            },
            config(),
            2,
            rotation(),
        )
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    assert!((bodies[0].motion.velocity[0] - 1.).abs() < 1e-12);
    assert!((bodies[1].motion.velocity[0] - 2.).abs() < 1e-12);
    assert!((bodies[1].spin.unwrap().angular_momentum[2] + 2.).abs() < 1e-12);
    assert!((bodies[1].motion.position[0] - 0.16).abs() < 1e-12);
    assert!((bodies[1].spin.unwrap().orientation[2] - (-0.08_f64).sin()).abs() < 1e-12);
    assert!((angular(bodies[0]) + angular(bodies[1]) - before_l).abs() < 1e-12);
    assert!(
        (bodies.iter().map(|b| b.energy().unwrap()).sum::<f64>()
            + report.dynamics.dissipated_energy
            - before_e)
            .abs()
            < 1e-12
    );
    assert_ne!(bodies, initial);
}
#[test]
fn liquid_particle_recoil_spins_body_on_the_same_event_timeline() {
    let mut liquid = fluid(vec![Particle {
        position: [-0.07, 1., 0.],
        velocity: [3., 0., 0.],
        mass: 1.,
        material: 0,
    }]);
    let mut bodies = [body([0.; 3], [0.; 3], 0.)];
    let report = liquid
        .step_with_rigid_body_world(
            0.03,
            &mut bodies,
            &Faces {
                late_failure: false,
                missing_witness: false,
            },
            config(),
            1,
            rotation(),
        )
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    assert!((liquid.particles()[0].velocity[0] + bodies[0].motion.velocity[0] - 3.).abs() < 1e-12);
    assert!(bodies[0].spin.unwrap().angular_momentum[2] < 0.);
    assert_ne!(bodies[0].spin.unwrap().orientation, [0., 0., 0., 1.]);
    let particle = liquid.particles()[0];
    let body = bodies[0];
    let angular = particle.position[0] * particle.velocity[1]
        - particle.position[1] * particle.velocity[0]
        + body.motion.position[0] * body.motion.velocity[1]
        - body.motion.position[1] * body.motion.velocity[0]
        + body.spin.unwrap().angular_momentum[2];
    assert!((angular + 3.).abs() < 1e-12);
    let energy =
        0.5 * particle.velocity.iter().map(|v| v * v).sum::<f64>() + body.energy().unwrap();
    assert!((energy + report.dynamics.dissipated_energy - 4.5).abs() < 1e-12);
}
#[test]
fn late_backend_failure_missing_witness_and_invalid_rotation_restore_all_states() {
    let initial = fluid(Vec::new());
    let original = [
        body([-0.1, 1., 0.], [3., 0., 0.], 0.),
        body([0.; 3], [0.; 3], 0.),
    ];
    for (late, missing, error) in [
        (true, false, Error::CollisionBackend),
        (false, true, Error::InvalidCollision),
    ] {
        let mut liquid = initial.clone();
        let mut bodies = original;
        assert_eq!(
            liquid.step_with_rigid_body_world(
                0.1,
                &mut bodies,
                &Faces {
                    late_failure: late,
                    missing_witness: missing
                },
                config(),
                2,
                rotation()
            ),
            Err(error)
        );
        assert_eq!(liquid, initial);
        assert_eq!(bodies, original);
    }
    let mut bad = rotation();
    bad.max_arcs = 0;
    let mut liquid = initial.clone();
    let mut bodies = original;
    assert!(
        liquid
            .step_with_rigid_body_world(
                0.1,
                &mut bodies,
                &Faces {
                    late_failure: false,
                    missing_witness: false
                },
                config(),
                2,
                bad
            )
            .is_err()
    );
    assert_eq!(liquid, initial);
    assert_eq!(bodies, original);
}
#[test]
fn free_spin_persists_between_ticks_and_legacy_tangential_damping_is_rejected() {
    let mut liquid = fluid(Vec::new());
    let mut bodies = [body([0.; 3], [0.; 3], 2.)];
    let world = Faces {
        late_failure: false,
        missing_witness: false,
    };
    for _ in 0..3 {
        liquid
            .step_with_rigid_body_world(0.03, &mut bodies, &world, config(), 1, rotation())
            .unwrap();
    }
    assert!((bodies[0].spin.unwrap().orientation[2] - 0.09_f64.sin()).abs() < 1e-12);
    let before = (liquid.clone(), bodies);
    let mut friction = config();
    friction.contact.friction = 0.1;
    assert_eq!(
        liquid.step_with_rigid_body_world(0.03, &mut bodies, &world, friction, 1, rotation()),
        Err(Error::InvalidCollision)
    );
    assert_eq!((liquid, bodies), before);
}

struct LegacyGeometry;
impl LiquidBodyWorld for LegacyGeometry {
    fn sweep_particle_body(
        &self,
        _: &Particle,
        _: f64,
        _: usize,
        _: &TranslatingBody,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Ok(GeometryHit::Clear)
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
        Ok(GeometryHit::Clear)
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
}
#[test]
fn legacy_geometry_cannot_silently_accept_intrinsic_spin() {
    let mut liquid = fluid(Vec::new());
    let mut bodies = [body([0.; 3], [0.; 3], 2.)];
    let before = (liquid.clone(), bodies);
    assert_eq!(
        liquid.step_with_rigid_body_world(
            0.03,
            &mut bodies,
            &LegacyGeometry,
            config(),
            1,
            rotation()
        ),
        Err(Error::CollisionBackend)
    );
    assert_eq!((liquid, bodies), before);
}

#[test]
fn inelastic_patch_impulses_use_shared_event_ledger_and_malformed_patch_rolls_back() {
    // Controlled callback qualifies event/ledger wiring only. Real geometry
    // separately proves this freely rotating remainder requires sustained contact.
    let initial = [
        body([-0.1, 1., 0.], [3., 0., 0.], 0.),
        body([0.; 3], [0.; 3], 0.),
    ];
    let mut liquid = fluid(Vec::new());
    let original = liquid.clone();
    let mut bodies = initial;
    let mut settings = config();
    settings.contact.restitution = 0.;
    let energy: f64 = initial.iter().map(|b| b.energy().unwrap()).sum();
    let report = liquid
        .step_with_rigid_body_world(
            0.03,
            &mut bodies,
            &Faces {
                late_failure: false,
                missing_witness: false,
            },
            settings,
            2,
            rotation(),
        )
        .unwrap();
    let impulse = 3. / (2. + 0.02 * 0.02 + 0.98 * 0.98);
    assert_eq!(report.dynamics.contacts, 1);
    assert!((bodies[0].motion.velocity[0] - (3. - impulse)).abs() < 1e-10);
    assert!((bodies[1].motion.velocity[0] - impulse).abs() < 1e-10);
    assert!(
        (bodies.iter().map(|b| b.energy().unwrap()).sum::<f64>()
            + report.dynamics.dissipated_energy
            - energy)
            .abs()
            < 1e-12
    );
    assert_eq!(report.environment_impulse, [0.; 3]);
    liquid = original.clone();
    bodies = initial;
    assert_eq!(
        liquid.step_with_rigid_body_world(
            0.03,
            &mut bodies,
            &Faces {
                late_failure: true,
                missing_witness: false
            },
            settings,
            2,
            rotation()
        ),
        Err(Error::InvalidCollision)
    );
    assert_eq!((liquid.clone(), bodies), (original.clone(), initial));
    settings.max_queries = 1;
    assert_eq!(
        liquid.step_with_rigid_body_world(
            0.03,
            &mut bodies,
            &Faces {
                late_failure: false,
                missing_witness: false
            },
            settings,
            2,
            rotation()
        ),
        Err(Error::CollisionBudget)
    );
    assert_eq!((liquid, bodies), (original, initial));
}

struct ClearPointWorld;
impl LiquidBodyWorld for ClearPointWorld {
    fn sweep_particle_body(
        &self,
        _: &Particle,
        _: f64,
        _: usize,
        _: &TranslatingBody,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Ok(GeometryHit::Clear)
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
        Ok(GeometryHit::Clear)
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
    fn sweep_rigid_environment_contact(
        &self,
        _: usize,
        _: &physics::rigid_motion::RigidMotion,
        _: usize,
    ) -> Result<BodyGeometryHit, Error> {
        Ok(GeometryHit::Clear.into())
    }
}
#[test]
fn point_load_world_matches_prepared_motion_and_work_and_rejects_atomically() {
    use physics::rigid_motion::{MaterialPointForce, MotionLoad};
    let initial = body([0.; 3], [0.; 3], 0.3);
    let points = vec![
        MaterialPointForce {
            local: [1., 0., 0.],
            force: [0., 2., 0.],
            force_rate: [0., 0.3, 0.],
        },
        MaterialPointForce {
            local: [-1., 0., 0.],
            force: [0., -2., 0.],
            force_rate: [0., -0.3, 0.],
        },
    ];
    let dt = 0.04;
    let reference = initial
        .prepare_material_load_motion(points.iter().copied(), [MotionLoad::zero()], dt, rotation())
        .unwrap();
    let mut liquid = fluid(Vec::new());
    let mut bodies = [initial];
    let report = liquid
        .step_with_rigid_body_point_forces(
            dt,
            &mut bodies,
            &ClearPointWorld,
            config(),
            1,
            rotation(),
            &[Default::default()],
            &[points.clone()],
        )
        .unwrap();
    assert_eq!(bodies[0], reference.end());
    let work = reference.work(dt).unwrap();
    assert!((report.external_work - work.force_work - work.torque_work).abs() < 1e-12);
    assert!((report.integration_energy_residual - work.energy_residual).abs() < 1e-12);
    let saved = (liquid.clone(), bodies);
    let mut invalid = points;
    invalid[0].force[0] = f64::NAN;
    assert!(
        liquid
            .step_with_rigid_body_point_forces(
                dt,
                &mut bodies,
                &ClearPointWorld,
                config(),
                1,
                rotation(),
                &[Default::default()],
                &[invalid]
            )
            .is_err()
    );
    assert_eq!((liquid, bodies), saved);
}

struct MovingSupportWorld;
impl LiquidBodyWorld for MovingSupportWorld {
    fn sweep_particle_body(
        &self,
        _: &Particle,
        _: f64,
        _: usize,
        _: &TranslatingBody,
        _: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Ok(GeometryHit::Clear)
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
        Ok(GeometryHit::Clear)
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
    fn sweep_rigid_environment_contact(
        &self,
        _: usize,
        _: &physics::rigid_motion::RigidMotion,
        _: usize,
    ) -> Result<BodyGeometryHit, Error> {
        Ok(GeometryHit::Clear.into())
    }
    fn rigid_environment_support_contacts(
        &self,
        _: usize,
        _: &ContactBody,
        _: usize,
    ) -> Result<Vec<physics::liquid::RigidSupportPoint>, Error> {
        Ok(vec![physics::liquid::RigidSupportPoint {
            support: physics::contact::NormalSupport {
                contact: physics::contact::NormalContact {
                    point: [1., 0., 0.],
                    normal: [0., 1., 0.],
                },
                plane: physics::contact::SupportPlane::World,
            },
            feature: Some(1),
            tolerance_m: 1e-12,
            admission_error_m: 0.,
            carrying_reaction: false,
        }])
    }
    fn rigid_support_point_velocity(
        &self,
        bodies: &[ContactBody],
        support: physics::contact::NetworkSupport,
        _: physics::liquid::RigidSupportPoint,
    ) -> Result<[f64; 3], Error> {
        bodies[support.first]
            .point_velocity(support.support.contact.point)
            .map_err(|_| Error::InvalidCollision)
    }
}
#[test]
fn reaction_rate_query_uses_geometry_owned_common_point_velocity() {
    use physics::contact::{
        ContactWrench, ReactionConfig, ReactionRateConfig, SupportMotion,
        resolve_normal_reaction_rate_network,
    };
    let liquid = fluid(Vec::new());
    let state = body([0.; 3], [0., -0.2, 0.], 0.2);
    let load = ContactWrench {
        force: [0., -9.81, 0.],
        torque: [0.; 3],
    };
    let rate = ReactionRateConfig {
        reaction: ReactionConfig {
            max_sweeps: 8192,
            acceleration_tolerance: 1e-11,
            normal_velocity_tolerance: 1e-10,
        },
        jerk_tolerance: 1e-10,
    };
    let saved = liquid.clone();
    let report = liquid
        .rigid_world_reaction_rates(
            &[state],
            &MovingSupportWorld,
            &[load],
            &[ContactWrench::default()],
            config(),
            rate,
        )
        .unwrap();
    let expected = resolve_normal_reaction_rate_network(
        &[state],
        &report.reactions.supports,
        &[load],
        &[ContactWrench::default()],
        &[SupportMotion {
            point_velocity: [0.; 3],
            normal_acceleration: None,
        }],
        rate,
    )
    .unwrap();
    let frozen = resolve_normal_reaction_rate_network(
        &[state],
        &report.reactions.supports,
        &[load],
        &[ContactWrench::default()],
        &[SupportMotion {
            point_velocity: state.motion.velocity,
            normal_acceleration: None,
        }],
        rate,
    )
    .unwrap();
    assert!((expected.forces_rate[0][1] - frozen.forces_rate[0][1]).abs() > 1e-6);
    assert_eq!(report.rate.as_ref().unwrap(), &expected);
    assert_eq!(liquid, saved);
}
