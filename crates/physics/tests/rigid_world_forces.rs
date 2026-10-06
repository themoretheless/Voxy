use physics::{
    astrophysics_spin::Spin,
    contact::{ContactBody, ContactWrench},
    gravity::Body,
    liquid::{
        BodyGeometryHit, Config, ContactConfig, ContactWitness, DynamicWorldConfig, Error,
        GeometryHit, Liquid, LiquidBodyWorld, Material, Particle, TranslatingBody,
    },
    rigid_motion::RigidMotion,
    spin_path,
};
struct World {
    acceleration: [f64; 3],
    wall: Option<f64>,
    fail_after_impact: bool,
}
impl LiquidBodyWorld for World {
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
    fn sweep_rigid_environment_contact(
        &self,
        _: usize,
        path: &RigidMotion,
        _: usize,
    ) -> Result<BodyGeometryHit, Error> {
        assert_eq!(path.acceleration(), self.acceleration);
        let initial = path.initial().motion;
        if initial.velocity[0] < 0. && self.fail_after_impact {
            return Err(Error::CollisionBackend);
        }
        let Some(wall) = self.wall else {
            return Ok(GeometryHit::Clear.into());
        };
        let a = path.acceleration()[0];
        let v = initial.velocity[0];
        let time = (-v + (v * v + 2. * a * (wall - initial.position[0])).sqrt()) / a;
        if !(0. ..=path.duration()).contains(&time) {
            return Ok(GeometryHit::Clear.into());
        }
        Ok(BodyGeometryHit {
            geometry: GeometryHit::Contact {
                fraction: time / path.duration(),
                normal: [-1., 0., 0.],
            },
            witness: Some(ContactWitness {
                point: [wall, 0., 0.],
                tolerance_m: 1e-12,
            }),
        })
    }
}
struct Linear;
impl LiquidBodyWorld for Linear {
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
fn rotation() -> spin_path::Config {
    spin_path::Config {
        max_angular_error_rad: 1e-5,
        min_step_s: 1e-9,
        max_arcs: 10000,
        max_trials: 30000,
    }
}
fn fluid(gravity: [f64; 3]) -> Liquid {
    Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity,
            ..Default::default()
        },
    )
    .unwrap()
}
fn rigid() -> ContactBody {
    ContactBody {
        motion: Body {
            mass: 2.,
            position: [0., 2., 0.],
            velocity: [1., 0., 0.],
        },
        spin: Some(Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0., 1.],
            inertia: [1.; 3],
        }),
    }
}
#[test]
fn gravity_and_additional_wrench_follow_analytic_motion_and_work_once() {
    let mut liquid = fluid([0., -2., 0.]);
    let mut bodies = [rigid()];
    let initial_energy = bodies[0].energy().unwrap();
    let world = World {
        acceleration: [2., -2., 0.],
        wall: None,
        fail_after_impact: false,
    };
    let wrench = ContactWrench {
        force: [4., 0., 0.],
        torque: [0., 0., 2.],
    };
    let report = liquid
        .step_with_rigid_body_forces(
            0.1,
            &mut bodies,
            &world,
            Default::default(),
            1,
            rotation(),
            &[wrench],
        )
        .unwrap();
    for (actual, expected) in bodies[0].motion.position.iter().zip([0.11, 1.99, 0.]) {
        assert!((actual - expected).abs() < 1e-14);
    }
    assert_eq!(bodies[0].motion.velocity, [1.2, -0.2, 0.]);
    assert!((bodies[0].spin.unwrap().angular_momentum[2] - 1.2).abs() < 1e-12);
    assert!((bodies[0].spin.unwrap().orientation[2] - (0.11_f64 / 2.).sin()).abs() < 1e-5);
    assert!((report.external_work - 0.7).abs() < 1e-11);
    assert!(report.integration_energy_residual.abs() < 1e-11);
    assert!(
        (bodies[0].energy().unwrap()
            - initial_energy
            - report.external_work
            - report.integration_energy_residual)
            .abs()
            < 1e-12
    );
    assert_eq!(report.world.dynamics.contacts, 0);
    assert_eq!(report.world.dynamics.dissipated_energy, 0.);
    // Splitting the tick retains the same physical load and body state.
    let mut split_fluid = fluid([0., -2., 0.]);
    let mut split = [rigid()];
    let mut work = 0.;
    for dt in [0.04, 0.06] {
        work += split_fluid
            .step_with_rigid_body_forces(
                dt,
                &mut split,
                &world,
                Default::default(),
                1,
                rotation(),
                &[wrench],
            )
            .unwrap()
            .external_work;
    }
    for k in 0..3 {
        assert!((split[0].motion.position[k] - bodies[0].motion.position[k]).abs() < 1e-12);
        assert!((split[0].motion.velocity[k] - bodies[0].motion.velocity[k]).abs() < 1e-12);
    }
    assert!((work - report.external_work).abs() < 1e-11);
}
#[test]
fn continuous_gravity_uses_half_acceleration_displacement_for_nonspinning_body() {
    let mut liquid = fluid([0., -2., 0.]);
    let mut state = rigid();
    state.spin = None;
    let mut bodies = [state];
    let world = World {
        acceleration: [0., -2., 0.],
        wall: None,
        fail_after_impact: false,
    };
    liquid
        .step_with_rigid_body_world(0.1, &mut bodies, &world, Default::default(), 1, rotation())
        .unwrap();
    assert_eq!(bodies[0].motion.position, [0.1, 1.99, 0.]);
    assert_eq!(bodies[0].motion.velocity, [1., -0.2, 0.]);
}
#[test]
fn accelerating_body_impacts_at_correct_time_and_force_continues_after_rebound() {
    let mut liquid = fluid([2., 0., 0.]);
    let mut state = rigid();
    state.spin = None;
    state.motion.position = [0.; 3];
    state.motion.velocity = [0.; 3];
    let mut bodies = [state];
    let world = World {
        acceleration: [2., 0., 0.],
        wall: Some(0.0025),
        fail_after_impact: false,
    };
    let config = DynamicWorldConfig {
        contact: ContactConfig {
            restitution: 1.,
            ..Default::default()
        },
        ..Default::default()
    };
    let result = liquid
        .step_with_rigid_body_forces(
            0.1,
            &mut bodies,
            &world,
            config,
            1,
            rotation(),
            &[ContactWrench::default()],
        )
        .unwrap();
    assert_eq!(result.world.dynamics.contacts, 1);
    assert!((result.world.environment_impulse[0] - 0.4).abs() < 1e-12);
    assert!(bodies[0].motion.position[0].abs() < 1e-12);
    assert!(bodies[0].motion.velocity[0].abs() < 1e-12);
    assert_eq!(result.world.dynamics.dissipated_energy, 0.);
    assert!(result.external_work.abs() < 1e-12);
    assert!(result.integration_energy_residual.abs() < 1e-12);
}
#[test]
fn unsupported_acceleration_bad_wrenches_and_late_geometry_failure_roll_back() {
    let mut liquid = fluid([2., 0., 0.]);
    let before_fluid = liquid.clone();
    let mut initial = rigid();
    initial.spin = None;
    initial.motion.position = [0.; 3];
    initial.motion.velocity = [0.; 3];
    let mut bodies = [initial];
    assert_eq!(
        liquid.step_with_rigid_body_world(
            0.1,
            &mut bodies,
            &Linear,
            Default::default(),
            1,
            rotation()
        ),
        Err(Error::CollisionBackend)
    );
    assert_eq!(bodies, [initial]);
    for bad in [
        vec![],
        vec![ContactWrench {
            force: [f64::NAN, 0., 0.],
            torque: [0.; 3],
        }],
        vec![ContactWrench {
            force: [0.; 3],
            torque: [0., 0., 1.],
        }],
    ] {
        assert_eq!(
            liquid.step_with_rigid_body_forces(
                0.1,
                &mut bodies,
                &Linear,
                Default::default(),
                1,
                rotation(),
                &bad
            ),
            Err(Error::InvalidCollision)
        );
        assert_eq!(bodies, [initial]);
    }
    let world = World {
        acceleration: [2., 0., 0.],
        wall: Some(0.0025),
        fail_after_impact: true,
    };
    let config = DynamicWorldConfig {
        contact: ContactConfig {
            restitution: 1.,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        liquid.step_with_rigid_body_forces(
            0.1,
            &mut bodies,
            &world,
            config,
            1,
            rotation(),
            &[ContactWrench::default()]
        ),
        Err(Error::CollisionBackend)
    );
    assert_eq!(bodies, [initial]);
    assert_eq!(liquid.particles(), before_fluid.particles());
}
