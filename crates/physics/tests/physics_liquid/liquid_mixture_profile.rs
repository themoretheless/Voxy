use physics::liquid::{
    Config, FluidMixtureProfile, Liquid, LiquidField, Material, Particle, TransportMaterial,
};

fn fluid() -> Liquid {
    let mut fluid = Liquid::new(
        (0..3)
            .map(|i| Particle {
                position: [i as f64 * 0.2, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: (i + 1) as f64,
                material: 0,
            })
            .collect(),
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 310.0,
                    concentration: 0.0
                };
                3
            ],
            vec![TransportMaterial {
                specific_heat: 4000.0,
                conductivity: 0.0,
                diffusivity: 20.0,
                mixing_group: 0,
            }],
        )
        .unwrap();
    fluid
}
fn rows() -> Vec<[f64; 3]> {
    vec![[0.97, 0.0, 0.03], [0.80, 0.15, 0.05], [0.50, 0.50, 0.0]]
}
#[test]
fn local_composition_and_structure_change_properties_without_injecting_heat() {
    let mut fluid = fluid();
    let fields = fluid.fields().unwrap().to_vec();
    fluid
        .configure_fluid_mixture(FluidMixtureProfile::DEMO, rows(), vec![1.0; 3])
        .unwrap();
    let properties = fluid.effective_materials().unwrap();
    assert!(properties[0].rest_density < properties[1].rest_density);
    assert!(properties[1].rest_density < properties[2].rest_density);
    assert!(properties[0].viscosity < properties[1].viscosity);
    assert!(properties[1].viscosity < properties[2].viscosity);
    assert_eq!(fluid.fields().unwrap(), fields);
    // A matched composition isolates the structural response from component blending.
    let mut broken = fluid.clone();
    broken
        .configure_thixotropy(
            vec![Some(FluidMixtureProfile::DEMO.thixotropy)],
            vec![0.0; 3],
        )
        .unwrap();
    for (a, b) in properties.iter().zip(broken.effective_materials().unwrap()) {
        assert!((b.viscosity - 0.2 * a.viscosity).abs() < 1e-12);
        assert_eq!(a.rest_density, b.rest_density);
    }
}
#[test]
fn heterogeneous_three_component_flow_conserves_each_mass_and_mixes() {
    let mut fluid = fluid();
    fluid
        .configure_fluid_mixture(FluidMixtureProfile::DEMO, rows(), vec![1.0, 0.6, 0.2])
        .unwrap();
    let before = fluid.species_totals().unwrap().unwrap();
    let initial = fluid.species_fractions().unwrap().to_vec();
    for _ in 0..100 {
        fluid.step(0.01, None).unwrap();
    }
    let after = fluid.species_totals().unwrap().unwrap();
    for (a, b) in before.iter().zip(after) {
        assert!((a - b).abs() < 1e-12);
    }
    assert!(fluid.species_fractions().unwrap()[0][1] > initial[0][1]);
    for row in fluid.species_fractions().unwrap() {
        assert!(row.iter().all(|x| (0.0..=1.0).contains(x)));
        assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }
    assert!(
        fluid
            .structure_fractions()
            .iter()
            .all(|x| (0.0..=1.0).contains(x))
    );
}
#[test]
fn late_configuration_errors_roll_back_species_properties_and_fields() {
    let mut fluid = fluid();
    let before = fluid.clone();
    let mut profile = FluidMixtureProfile::DEMO;
    profile.thixotropy.breakdown = f64::NAN;
    assert!(
        fluid
            .configure_fluid_mixture(profile, rows(), vec![1.0; 3])
            .is_err()
    );
    assert_eq!(fluid, before);
    assert!(
        fluid
            .configure_fluid_mixture(FluidMixtureProfile::DEMO, rows(), vec![1.0; 2])
            .is_err()
    );
    assert_eq!(fluid, before);
    let mut bad = rows();
    bad[0] = [0.9, 0.2, 0.0];
    assert!(
        fluid
            .configure_fluid_mixture(FluidMixtureProfile::DEMO, bad, vec![1.0; 3])
            .is_err()
    );
    assert_eq!(fluid, before);
}
