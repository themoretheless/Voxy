//! Plan step F3: reflecting-box walls as a fixed-temperature reservoir.
use physics::liquid::{
    Config, Formulation, Liquid, LiquidField, Material, Particle, ReflectingBox, TransportMaterial,
    WallThermal,
};

fn fluid() -> Liquid {
    let mut liquid = Liquid::new(
        vec![
            // Surface 0.005 m from the x = 0 wall: touching one face.
            Particle {
                position: [0.03, 1.5, 1.5],
                velocity: [0.; 3],
                mass: 1.0,
                material: 0,
            },
            // Far from every wall.
            Particle {
                position: [1.5, 1.5, 1.5],
                velocity: [0.; 3],
                mass: 1.0,
                material: 0,
            },
            // In a corner: two faces within reach.
            Particle {
                position: [2.97, 0.03, 1.5],
                velocity: [0.; 3],
                mass: 1.0,
                material: 0,
            },
        ],
        vec![Material {
            rest_density: 1.0,
            sound_speed: 1.0,
            viscosity: 0.0,
        }],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.025,
            gravity: [0.; 3],
            ..Config::default()
        },
    )
    .unwrap();
    liquid.set_formulation(Formulation::RestVolume);
    liquid
        .set_reflecting_box(Some(ReflectingBox {
            min: [0.; 3],
            max: [3.; 3],
        }))
        .unwrap();
    liquid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                3
            ],
            vec![TransportMaterial {
                specific_heat: 2.0,
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid
}

#[test]
fn touching_particles_relax_toward_the_wall_temperature_with_face_count_conductance() {
    let mut liquid = fluid();
    let wall = WallThermal {
        temperature: 600.0,
        conductance_w_m2_k: 50.0,
        contact_distance: 0.01,
    };
    let conductances = liquid.box_wall_conductances(wall).unwrap();
    let face = 50.0 * std::f64::consts::PI * 0.025 * 0.025;
    assert!((conductances[0] - face).abs() < 1e-15);
    assert_eq!(conductances[1], 0.0);
    assert!((conductances[2] - 2.0 * face).abs() < 1e-15);
    let initial = liquid.transport_totals().unwrap().unwrap().0;
    let report = liquid.exchange_box_wall_heat(0.5, wall).unwrap();
    assert_eq!(report.touching_particles, 2);
    let fields = liquid.fields().unwrap();
    for (i, g) in [(0, face), (2, 2.0 * face)] {
        let expected = 600.0 + (300.0 - 600.0) * (-g * 0.5 / 2.0).exp();
        assert!(
            (fields[i].temperature - expected).abs() < 1e-9,
            "{i}: {}",
            fields[i].temperature
        );
    }
    assert_eq!(fields[1].temperature, 300.0);
    let after = liquid.transport_totals().unwrap().unwrap().0;
    assert!((after - initial - report.transferred_j).abs() < 1e-9);
    assert!(report.transferred_j > 0.);
    // A cold wall cools them back; without a box the exchange is rejected.
    let cold = liquid
        .exchange_box_wall_heat(
            0.5,
            WallThermal {
                temperature: 250.0,
                ..wall
            },
        )
        .unwrap();
    assert!(cold.transferred_j < 0.);
    let mut boxless = Liquid::new(
        liquid.particles().to_vec(),
        vec![Material {
            rest_density: 1.0,
            sound_speed: 1.0,
            viscosity: 0.0,
        }],
        Config::default(),
    )
    .unwrap();
    assert!(boxless.exchange_box_wall_heat(0.5, wall).is_err());
    assert!(
        liquid
            .exchange_box_wall_heat(
                0.5,
                WallThermal {
                    conductance_w_m2_k: -1.,
                    ..wall
                }
            )
            .is_err()
    );
}
