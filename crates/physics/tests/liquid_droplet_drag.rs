use physics::liquid::{Config, DropletGas, Liquid, Material, Particle};
fn fluid() -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [0.0; 3],
            velocity: [3.0, 0.0, 0.0],
            mass: 2.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap()
}
#[test]
fn analytical_drag_and_moving_gas_energy_momentum_balance() {
    for velocity in [[0.0; 3], [10.0, 0.0, 0.0], [-1.0, 2.0, 0.0]] {
        let mut l = fluid();
        let before = l.particles()[0];
        let gas = DropletGas {
            velocity,
            density: 1.0,
            drag_coefficient: 0.5,
        };
        let r = l.apply_droplet_drag(0.2, gas, &[1.0]).unwrap();
        let after = l.particles()[0];
        let w = std::array::from_fn::<_, 3, _>(|k| before.velocity[k] - velocity[k]);
        let speed = w.iter().map(|v| v * v).sum::<f64>().sqrt();
        let factor = 1.0 + 0.5 * 0.5 * std::f64::consts::PI / 2.0 * speed * 0.2;
        for k in 0..3 {
            assert!((after.velocity[k] - (velocity[k] + w[k] / factor)).abs() < 1e-14);
            assert!(
                (2.0 * after.velocity[k] + r.gas_impulse[k] - 2.0 * before.velocity[k]).abs()
                    < 1e-14
            );
        }
        let ke = |v: [f64; 3]| v.iter().map(|v| v * v).sum::<f64>();
        assert!(
            (ke(before.velocity) - ke(after.velocity) - r.dissipated_heat - r.gas_work).abs()
                < 1e-13
        );
        assert!(r.dissipated_heat >= 0.0);
        assert_eq!(before.position, after.position);
    }
}
#[test]
fn exact_semigroup_and_invalid_controls_are_atomic() {
    let mut single = fluid();
    let mut split = single.clone();
    let gas = DropletGas {
        velocity: [0.0; 3],
        density: 1.0,
        drag_coefficient: 0.5,
    };
    single.apply_droplet_drag(1.0, gas, &[1.0]).unwrap();
    for _ in 0..10 {
        split.apply_droplet_drag(0.1, gas, &[1.0]).unwrap();
    }
    assert!((single.particles()[0].velocity[0] - split.particles()[0].velocity[0]).abs() < 1e-14);
    let before = single.clone();
    assert!(single.apply_droplet_drag(1.0, gas, &[0.0]).is_err());
    assert_eq!(single, before);
    assert!(single.apply_droplet_drag(-1.0, gas, &[1.0]).is_err());
    assert_eq!(single, before);
}

#[test]
fn marked_drag_preserves_carrier_samples_and_reports_only_droplet_exchange() {
    let mut l = Liquid::new(
        vec![
            Particle {
                position: [0.0; 3],
                velocity: [3.0, 0.0, 0.0],
                mass: 2.0,
                material: 0,
            },
            Particle {
                position: [1.0; 3],
                velocity: [7.0, 1.0, 0.0],
                mass: 4.0,
                material: 0,
            },
        ],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    let gas = DropletGas {
        velocity: [1.0, 0.0, 0.0],
        density: 1.0,
        drag_coefficient: 0.5,
    };
    let original = l.clone();
    assert!(l.apply_marked_droplet_drag(0.2, gas, &[1.0, 1.0]).is_err());
    assert_eq!(l, original);
    l.configure_droplet_population(Some(vec![true, false]))
        .unwrap();
    let before = l.clone();
    let r = l.apply_marked_droplet_drag(0.2, gas, &[1.0, 1.0]).unwrap();
    assert_eq!(l.particles()[1], before.particles()[1]);
    let factor = 1.0 + 0.5 * 0.5 * std::f64::consts::PI / 2.0 * 2.0 * 0.2;
    assert!((l.particles()[0].velocity[0] - (1.0 + 2.0 / factor)).abs() < 1e-14);
    assert!(
        (2.0 * (before.particles()[0].velocity[0] - l.particles()[0].velocity[0])
            - r.gas_impulse[0])
            .abs()
            < 1e-14
    );
    let kinetic = |v: f64| v * v; // first droplet has mass 2 kg
    assert!(
        (kinetic(3.0) - kinetic(l.particles()[0].velocity[0]) - r.dissipated_heat - r.gas_work)
            .abs()
            < 1e-13
    );
    assert_eq!(l.droplet_population(), before.droplet_population());
}
#[test]
fn an_empty_marked_population_has_zero_exchange() {
    let mut l = fluid();
    l.configure_droplet_population(Some(vec![false])).unwrap();
    let before = l.clone();
    let r = l
        .apply_marked_droplet_drag(
            0.2,
            DropletGas {
                velocity: [1.0; 3],
                density: 1.0,
                drag_coefficient: 0.5,
            },
            &[1.0],
        )
        .unwrap();
    assert_eq!(r, Default::default());
    assert_eq!(l, before);
}
