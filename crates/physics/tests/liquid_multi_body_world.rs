use physics::liquid::{
    Config, ContactConfig, DynamicWorldConfig, Error, GeometryHit, Liquid, LiquidBodyWorld,
    Material, Particle, TranslatingBody,
};
struct Boxes {
    wall: bool,
    late_failure: bool,
}
fn sweep(center: [f64; 3], half: f64, displacement: [f64; 3], obstacle_half: f64) -> GeometryHit {
    let body = physics::AnchoredAabb {
        anchor: Default::default(),
        min: center.map(|v| v - half),
        max: center.map(|v| v + half),
    };
    match physics::sweep_box(body, displacement, [-obstacle_half; 3], [obstacle_half; 3]) {
        None => GeometryHit::Clear,
        Some((_, [0, 0, 0])) => GeometryHit::Overlap,
        Some((fraction, normal)) => GeometryHit::Contact {
            fraction,
            normal: normal.map(f64::from),
        },
    }
}
impl LiquidBodyWorld for Boxes {
    fn sweep_particle_body(
        &self,
        p: &Particle,
        r: f64,
        _: usize,
        b: &TranslatingBody,
        dt: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Ok(sweep(
            std::array::from_fn(|a| p.position[a] - b.position[a]),
            r,
            std::array::from_fn(|a| (p.velocity[a] - b.velocity[a]) * dt),
            0.05,
        ))
    }
    fn sweep_body_pair(
        &self,
        _: usize,
        a: &TranslatingBody,
        _: usize,
        b: &TranslatingBody,
        dt: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        Ok(sweep(
            std::array::from_fn(|i| a.position[i] - b.position[i]),
            0.05,
            std::array::from_fn(|i| (a.velocity[i] - b.velocity[i]) * dt),
            0.05,
        ))
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
        index: usize,
        b: &TranslatingBody,
        dt: f64,
        _: usize,
    ) -> Result<GeometryHit, Error> {
        if self.late_failure && index == 1 && b.velocity[0] > 0. {
            return Err(Error::CollisionBackend);
        }
        if !self.wall
            || index != 1
            || b.velocity[0] <= 0.
            || b.position[0] + b.velocity[0] * dt < 0.3
        {
            return Ok(GeometryHit::Clear);
        }
        Ok(GeometryHit::Contact {
            fraction: ((0.3 - b.position[0]) / (b.velocity[0] * dt)).max(0.),
            normal: [-1., 0., 0.],
        })
    }
}
fn body(x: f64, v: f64) -> TranslatingBody {
    TranslatingBody {
        position: [x, 0., 0.],
        velocity: [v, 0., 0.],
        mass: 1.,
    }
}
fn fluid(particles: Vec<Particle>) -> Liquid {
    Liquid::new(
        particles,
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap()
}
fn elastic() -> DynamicWorldConfig {
    DynamicWorldConfig {
        contact: ContactConfig {
            restitution: 1.,
            ..Default::default()
        },
        ..Default::default()
    }
}
#[test]
fn three_body_impulse_chain_uses_one_event_timeline_without_fluid() {
    let mut liquid = fluid(Vec::new());
    let mut bodies = [body(-0.2, 3.), body(0., 0.), body(0.2, 0.)];
    let report = liquid
        .step_with_body_world(
            0.1,
            &mut bodies,
            &Boxes {
                wall: false,
                late_failure: false,
            },
            elastic(),
            3,
        )
        .unwrap();
    assert_eq!(report.dynamics.contacts, 2);
    for (b, (x, v)) in bodies.iter().zip([(-0.1, 0.), (0.1, 0.), (0.3, 3.)]) {
        assert!((b.position[0] - x).abs() < 1e-12);
        assert_eq!(b.velocity[0], v);
    }
    assert_eq!(report.environment_impulse, [0.; 3]);
    assert_eq!(report.dynamics.dissipated_energy, 0.);
}
#[test]
fn fluid_recoil_body_pair_and_static_wall_preserve_boundary_balance() {
    let mut liquid = fluid(vec![Particle {
        position: [-0.15, 0., 0.],
        velocity: [3., 0., 0.],
        mass: 1.,
        material: 0,
    }]);
    let mut bodies = [body(0., 0.), body(0.2, 0.)];
    let report = liquid
        .step_with_body_world(
            0.1,
            &mut bodies,
            &Boxes {
                wall: true,
                late_failure: false,
            },
            elastic(),
            2,
        )
        .unwrap();
    assert_eq!(report.dynamics.contacts, 3);
    assert!((bodies[0].position[0] - 0.1).abs() < 1e-12);
    assert!((bodies[1].position[0] - 0.275).abs() < 1e-12);
    assert_eq!(bodies[1].velocity, [-3., 0., 0.]);
    let p = liquid.particles()[0];
    for a in 0..3 {
        assert!(
            (p.velocity[a]
                + bodies.iter().map(|b| b.velocity[a]).sum::<f64>()
                + report.environment_impulse[a]
                - [3., 0., 0.][a])
                .abs()
                < 1e-12
        );
    }
    let energy = 0.5 * p.velocity.iter().map(|v| v * v).sum::<f64>()
        + bodies
            .iter()
            .map(|b| 0.5 * b.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>();
    assert!((energy + report.dynamics.dissipated_energy - 4.5).abs() < 1e-12);
}
#[test]
fn late_second_body_failure_and_admission_budget_restore_all_owners() {
    let mut liquid = fluid(vec![Particle {
        position: [-0.15, 0., 0.],
        velocity: [3., 0., 0.],
        mass: 1.,
        material: 0,
    }]);
    let mut bodies = [body(0., 0.), body(0.2, 0.)];
    let before = liquid.clone();
    let bodies_before = bodies;
    assert!(
        liquid
            .step_with_body_world(
                0.1,
                &mut bodies,
                &Boxes {
                    wall: false,
                    late_failure: true
                },
                elastic(),
                2
            )
            .is_err()
    );
    assert_eq!(liquid, before);
    assert_eq!(bodies, bodies_before);
    assert!(
        liquid
            .step_with_body_world(
                0.1,
                &mut bodies,
                &Boxes {
                    wall: false,
                    late_failure: false
                },
                elastic(),
                1
            )
            .is_err()
    );
    assert_eq!(liquid, before);
    assert_eq!(bodies, bodies_before);
}
