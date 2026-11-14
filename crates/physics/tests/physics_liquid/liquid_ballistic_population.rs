use physics::liquid::{Config, Liquid, LiquidField, Material, Particle, TransportMaterial};
fn particle(x: f64, v: f64, mass: f64) -> Particle {
    Particle {
        position: [x, 0.0, 0.0],
        velocity: [v, 0.0, 0.0],
        mass,
        material: 0,
    }
}
fn config() -> Config {
    Config {
        gravity: [0.0; 3],
        ..Config::default()
    }
}
#[test]
fn massive_neighbor_drop_does_not_change_carrier_motion() {
    let carrier = particle(0.0, 0.0, 0.001);
    let drop = particle(0.01, 3.0, 10.0);
    let mut mixed = Liquid::new(vec![carrier, drop], vec![Material::WATER], config()).unwrap();
    mixed
        .configure_droplet_population(Some(vec![false, true]))
        .unwrap();
    let mut reference = Liquid::new(vec![carrier], vec![Material::WATER], config()).unwrap();
    reference.step(0.01, None).unwrap();
    let stats = mixed.step_carrier_and_droplets(0.01, None).unwrap();
    assert_eq!(stats.neighbor_pairs, 0);
    assert_eq!(mixed.particles()[0], reference.particles()[0]);
    assert_eq!(mixed.particles()[1].velocity, drop.velocity);
    assert!((mixed.particles()[1].position[0] - 0.04).abs() < 1e-14);
}
#[test]
fn gravity_velocity_and_position_are_exact_under_subdivision() {
    for steps in [1, 10] {
        let mut l = Liquid::new(
            vec![Particle {
                position: [0.0; 3],
                velocity: [0.0, 1.0, 0.0],
                mass: 0.001,
                material: 0,
            }],
            vec![Material::WATER],
            Config {
                gravity: [0.0, -2.0, 0.0],
                smoothing_radius: 10.0,
                ..Config::default()
            },
        )
        .unwrap();
        l.configure_droplet_population(Some(vec![true])).unwrap();
        for _ in 0..steps {
            l.step_carrier_and_droplets(0.1 / steps as f64, None)
                .unwrap();
        }
        assert!((l.particles()[0].velocity[1] - 0.8).abs() < 1e-14);
        let error = (l.particles()[0].position[1] - 0.09).abs();
        assert!(error < 1e-14);
    }
}
#[test]
fn carrier_drop_pairs_do_not_diffuse_species_or_sensible_heat() {
    let mut l = Liquid::new(
        vec![particle(0.0, 0.0, 0.001), particle(0.01, 0.0, 0.001)],
        vec![Material::WATER],
        config(),
    )
    .unwrap();
    l.configure_transport(
        vec![
            LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            },
            LiquidField {
                temperature: 400.0,
                concentration: 0.0,
            },
        ],
        vec![TransportMaterial {
            conductivity: 1e6,
            diffusivity: 1e6,
            ..TransportMaterial::default()
        }],
    )
    .unwrap();
    l.configure_species(
        vec!["a".into(), "b".into()],
        vec![vec![1.0, 0.0], vec![0.0, 1.0]],
    )
    .unwrap();
    l.configure_droplet_population(Some(vec![false, true]))
        .unwrap();
    let before = l.clone();
    l.step_carrier_and_droplets(0.01, None).unwrap();
    assert_eq!(l.fields(), before.fields());
    assert_eq!(l.species_fractions(), before.species_fractions());
}
#[test]
fn all_carrier_mode_matches_ordinary_step_and_failure_is_atomic() {
    let mut l = Liquid::new(
        vec![particle(0.0, 1.0, 0.001), particle(0.05, -1.0, 0.001)],
        vec![Material::WATER],
        config(),
    )
    .unwrap();
    l.configure_droplet_population(Some(vec![false, false]))
        .unwrap();
    let mut ordinary = l.clone();
    ordinary.step(0.01, None).unwrap();
    l.step_carrier_and_droplets(0.01, None).unwrap();
    assert_eq!(l, ordinary);
    let before = l.clone();
    assert!(l.step_carrier_and_droplets(0.2, None).is_err());
    assert_eq!(l, before);
}

#[test]
fn drop_does_not_change_carrier_shear_thinning() {
    use physics::liquid::ShearThinning;
    let carriers = vec![particle(0.0, 0.0, 0.001), particle(0.05, 1.0, 0.001)];
    let mut reference = Liquid::new(carriers.clone(), vec![Material::WATER], config()).unwrap();
    let mut particles = carriers;
    particles.push(particle(0.02, 100.0, 10.0));
    let mut mixed = Liquid::new(particles, vec![Material::WATER], config()).unwrap();
    let rheology = Some(ShearThinning {
        reference_rate: 1.0,
        flow_index: 0.5,
        minimum_rate: 0.01,
        minimum_viscosity: 1e-6,
        maximum_viscosity: 0.1,
    });
    reference.configure_shear_thinning(vec![rheology]).unwrap();
    mixed.configure_shear_thinning(vec![rheology]).unwrap();
    mixed
        .configure_droplet_population(Some(vec![false, false, true]))
        .unwrap();
    reference.step(1e-5, None).unwrap();
    mixed.step_carrier_and_droplets(1e-5, None).unwrap();
    for (a, b) in reference.particles().iter().zip(mixed.particles()) {
        for k in 0..3 {
            assert!((a.velocity[k] - b.velocity[k]).abs() < 1e-13);
            assert!((a.position[k] - b.position[k]).abs() < 1e-13);
        }
    }
}
