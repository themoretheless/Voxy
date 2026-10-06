use physics::liquid::{
    Config, ContactConfig, GeometryHit, Liquid, LiquidGeometry, Material, Particle,
};
struct Wall {
    malformed: bool,
}
impl LiquidGeometry for Wall {
    type Error = ();
    fn sweep(
        &self,
        center: [f64; 3],
        radius: f64,
        displacement: [f64; 3],
        _: usize,
    ) -> Result<GeometryHit, ()> {
        let n = [
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
            0.,
        ];
        let gap = -(center[0] + center[1]) * n[0] - radius * 2. * n[0];
        let speed = (displacement[0] + displacement[1]) * n[0];
        if gap < -1e-12 {
            return Ok(GeometryHit::Overlap);
        }
        if speed <= 0. || speed < gap {
            return Ok(GeometryHit::Clear);
        }
        Ok(GeometryHit::Contact {
            fraction: gap / speed,
            normal: if self.malformed {
                [2., 0., 0.]
            } else {
                n.map(|v| -v)
            },
        })
    }
}
fn fluid() -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [-0.1, 0., 0.],
            velocity: [2., 0., 0.],
            mass: 1.,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Config::default()
        },
    )
    .unwrap()
}
#[test]
fn oblique_restitution_preserves_tangent_and_expected_kinetic_loss() {
    for restitution in [0., 0.5, 1.] {
        let mut liquid = fluid();
        liquid
            .step_with_geometry(
                0.1,
                None,
                &Wall { malformed: false },
                ContactConfig {
                    restitution,
                    ..ContactConfig::default()
                },
            )
            .unwrap();
        let v = liquid.particles()[0].velocity;
        assert!((v[0] - (1. - restitution)).abs() < 1e-12);
        assert!((v[1] - (-1. - restitution)).abs() < 1e-12);
        let energy = 0.5 * v.iter().map(|v| v * v).sum::<f64>();
        assert!((energy - (1. + restitution * restitution)).abs() < 1e-12);
    }
}
#[test]
fn malformed_contact_after_advection_rolls_back_entire_fluid() {
    let mut liquid = fluid();
    let before = liquid.clone();
    assert!(
        liquid
            .step_with_geometry(
                0.1,
                None,
                &Wall { malformed: true },
                ContactConfig::default()
            )
            .is_err()
    );
    assert_eq!(liquid, before);
}

#[test]
fn finite_oblique_body_conserves_impulse_and_accounts_contact_loss() {
    use physics::liquid::{ContactConfig, DynamicWorldConfig, TranslatingBody};
    for restitution in [0., 0.5, 1.] {
        for friction in [0., 0.5, 1.] {
            for boost in [[0.; 3], [0.3, -0.4, 0.2]] {
                let mut liquid = Liquid::new(
                    vec![Particle {
                        position: [-0.1, 0., 0.],
                        velocity: [2. + boost[0], boost[1], boost[2]],
                        mass: 1.,
                        material: 0,
                    }],
                    vec![Material::WATER],
                    Config {
                        gravity: [0.; 3],
                        ..Default::default()
                    },
                )
                .unwrap();
                let mut body = TranslatingBody {
                    position: [0.; 3],
                    velocity: boost,
                    mass: 3.,
                };
                let initial_velocity = liquid.particles()[0].velocity;
                let momentum: [f64; 3] =
                    std::array::from_fn(|a| initial_velocity[a] + 3. * boost[a]);
                let initial_energy = 0.5 * initial_velocity.iter().map(|v| v * v).sum::<f64>()
                    + 1.5 * boost.iter().map(|v| v * v).sum::<f64>();
                let report = liquid
                    .step_with_dynamic_geometry(
                        0.1,
                        &mut body,
                        &Wall { malformed: false },
                        DynamicWorldConfig {
                            contact: ContactConfig {
                                restitution,
                                friction,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    )
                    .unwrap();
                assert_eq!(report.contacts, 1);
                let v = liquid.particles()[0].velocity;
                for a in 0..3 {
                    assert!((v[a] + 3. * body.velocity[a] - momentum[a]).abs() < 1e-12);
                }
                let energy = 0.5 * v.iter().map(|v| v * v).sum::<f64>()
                    + 1.5 * body.velocity.iter().map(|v| v * v).sum::<f64>();
                assert!((energy + report.dissipated_energy - initial_energy).abs() < 1e-12);
                let expected_loss =
                    0.75 * (1. - restitution * restitution + friction * (2. - friction));
                assert!((report.dissipated_energy - expected_loss).abs() < 1e-12);
                assert!(
                    body.position
                        .iter()
                        .zip(boost)
                        .any(|(p, b)| (*p - b * 0.1).abs() > 1e-5)
                );
            }
        }
    }
}

#[test]
fn finite_oblique_body_malformed_hit_and_global_budget_roll_back_both_owners() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    for (malformed, max_queries) in [(true, 100000), (false, 1)] {
        let mut liquid = fluid();
        let mut body = TranslatingBody {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 3.,
        };
        let before = liquid.clone();
        let body_before = body;
        assert!(
            liquid
                .step_with_dynamic_geometry(
                    0.1,
                    &mut body,
                    &Wall { malformed },
                    DynamicWorldConfig {
                        max_queries,
                        ..Default::default()
                    }
                )
                .is_err()
        );
        assert_eq!(liquid, before);
        assert_eq!(body, body_before);
    }
}

#[test]
fn irrelevant_large_coordinate_does_not_inflate_oblique_contact_separation() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let mut results = Vec::new();
    for z in [0., 1e12] {
        let mut liquid = Liquid::new(
            vec![Particle {
                position: [-0.1, 0., z],
                velocity: [2., 0., 0.],
                mass: 1.,
                material: 0,
            }],
            vec![Material::WATER],
            Config {
                gravity: [0.; 3],
                ..Default::default()
            },
        )
        .unwrap();
        let mut body = TranslatingBody {
            position: [0., 0., z],
            velocity: [0.; 3],
            mass: 3.,
        };
        liquid
            .step_with_dynamic_geometry(
                0.1,
                &mut body,
                &Wall { malformed: false },
                DynamicWorldConfig::default(),
            )
            .unwrap();
        results.push((liquid.particles()[0], body));
    }
    for axis in 0..2 {
        assert_eq!(results[0].0.position[axis], results[1].0.position[axis]);
        assert_eq!(results[0].1.position[axis], results[1].1.position[axis]);
    }
    assert_eq!(results[0].0.velocity, results[1].0.velocity);
    assert_eq!(results[0].1.velocity, results[1].1.velocity);
}

struct Environment {
    particle_limit: Option<f64>,
    body_limit: Option<f64>,
    fail_after_recoil: bool,
}
fn right_wall(
    position: f64,
    displacement: f64,
    limit: Option<f64>,
) -> physics::liquid::GeometryHit {
    use physics::liquid::GeometryHit;
    let Some(limit) = limit else {
        return GeometryHit::Clear;
    };
    if position > limit + 1e-12 {
        return GeometryHit::Overlap;
    }
    if displacement <= 0. || position + displacement < limit {
        return GeometryHit::Clear;
    }
    GeometryHit::Contact {
        fraction: ((limit - position) / displacement).max(0.),
        normal: [-1., 0., 0.],
    }
}
impl physics::liquid::DynamicLiquidEnvironment for Environment {
    fn sweep_particle(
        &self,
        center: [f64; 3],
        _: f64,
        displacement: [f64; 3],
        _: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        Ok(right_wall(center[0], displacement[0], self.particle_limit))
    }
    fn sweep_body(
        &self,
        body: &physics::liquid::TranslatingBody,
        displacement: [f64; 3],
        _: usize,
    ) -> Result<physics::liquid::GeometryHit, physics::liquid::Error> {
        if self.fail_after_recoil && body.velocity[0] > 0. {
            return Err(physics::liquid::Error::CollisionBackend);
        }
        Ok(right_wall(
            body.position[0],
            displacement[0],
            self.body_limit,
        ))
    }
}
#[test]
fn finite_body_then_static_wall_share_event_time_and_external_impulse() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let mut liquid = fluid();
    let mut body = TranslatingBody {
        position: [0.; 3],
        velocity: [0.; 3],
        mass: 3.,
    };
    let report = liquid
        .step_with_dynamic_geometry_and_environment(
            0.1,
            &mut body,
            &Wall { malformed: false },
            &Environment {
                particle_limit: None,
                body_limit: Some(0.01),
                fail_after_recoil: false,
            },
            DynamicWorldConfig {
                contact: ContactConfig {
                    restitution: 1.,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(report.dynamics.contacts, 2);
    assert!((body.position[0] + 0.0175).abs() < 1e-10);
    assert!((body.position[1] - 0.0375).abs() < 1e-10);
    assert!((body.velocity[0] + 0.5).abs() < 1e-12);
    assert!((body.velocity[1] - 0.5).abs() < 1e-12);
    let p = liquid.particles()[0];
    let initial = [2., 0., 0.];
    for a in 0..3 {
        assert!(
            (p.velocity[a] + 3. * body.velocity[a] + report.environment_impulse[a] - initial[a])
                .abs()
                < 1e-12
        );
    }
    let energy = 0.5 * p.velocity.iter().map(|v| v * v).sum::<f64>()
        + 1.5 * body.velocity.iter().map(|v| v * v).sum::<f64>();
    assert!((energy + report.dynamics.dissipated_energy - 2.).abs() < 1e-12);
}
#[test]
fn static_particle_event_precedes_body_event_and_late_backend_failure_rolls_back() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let mut liquid = fluid();
    let mut body = TranslatingBody {
        position: [0.; 3],
        velocity: [0.; 3],
        mass: 3.,
    };
    let report = liquid
        .step_with_dynamic_geometry_and_environment(
            0.1,
            &mut body,
            &Wall { malformed: false },
            &Environment {
                particle_limit: Some(-0.08),
                body_limit: None,
                fail_after_recoil: false,
            },
            DynamicWorldConfig {
                contact: ContactConfig {
                    restitution: 1.,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    assert_eq!(body.velocity, [0.; 3]);
    assert_eq!(liquid.particles()[0].velocity, [-2., 0., 0.]);
    assert_eq!(report.environment_impulse, [4., 0., 0.]);
    let mut liquid = fluid();
    let before = liquid.clone();
    let body_before = body;
    assert!(
        liquid
            .step_with_dynamic_geometry_and_environment(
                0.1,
                &mut body,
                &Wall { malformed: false },
                &Environment {
                    particle_limit: None,
                    body_limit: None,
                    fail_after_recoil: true
                },
                Default::default()
            )
            .is_err()
    );
    assert_eq!(liquid, before);
    assert_eq!(body, body_before);
}
#[test]
fn empty_fluid_still_sweeps_body_against_environment() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let mut liquid = Liquid::new(
        Vec::new(),
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Default::default()
        },
    )
    .unwrap();
    let mut body = TranslatingBody {
        position: [0.; 3],
        velocity: [1., 0., 0.],
        mass: 3.,
    };
    let report = liquid
        .step_with_dynamic_geometry_and_environment(
            0.05,
            &mut body,
            &Wall { malformed: false },
            &Environment {
                particle_limit: None,
                body_limit: Some(0.01),
                fail_after_recoil: false,
            },
            DynamicWorldConfig {
                contact: ContactConfig {
                    restitution: 1.,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(report.dynamics.contacts, 1);
    assert!((body.position[0] + 0.03).abs() < 1e-12);
    assert_eq!(body.velocity, [-1., 0., 0.]);
    assert_eq!(report.environment_impulse, [6., 0., 0.]);
}

#[test]
fn inelastic_recoil_against_wall_stays_bounded_and_closes_energy_ledger() {
    use physics::liquid::TranslatingBody;
    let mut liquid = fluid();
    let mut body = TranslatingBody {
        position: [0.; 3],
        velocity: [0.; 3],
        mass: 3.,
    };
    let report = liquid
        .step_with_dynamic_geometry_and_environment(
            0.1,
            &mut body,
            &Wall { malformed: false },
            &Environment {
                particle_limit: None,
                body_limit: Some(0.001),
                fail_after_recoil: false,
            },
            Default::default(),
        )
        .unwrap();
    assert!(body.position[0] <= 0.001 + 1e-12);
    let p = liquid.particles()[0];
    for a in 0..3 {
        assert!(
            (p.velocity[a] + 3. * body.velocity[a] + report.environment_impulse[a]
                - [2., 0., 0.][a])
                .abs()
                < 1e-10
        );
    }
    let energy = 0.5 * p.velocity.iter().map(|v| v * v).sum::<f64>()
        + 1.5 * body.velocity.iter().map(|v| v * v).sum::<f64>();
    assert!((energy + report.dynamics.dissipated_energy - 2.).abs() < 1e-10);
}
