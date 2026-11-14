use physics::{
    moisture::{Body, Calibration, Cell, Properties, Reservoir},
    plasticity::State,
    wear::Layer,
};
fn endpoints() -> (Properties, Properties) {
    let dry = Properties {
        young_pa: 1e8,
        poisson: 0.3,
        yield_pa: 1e6,
        hardening_pa: 0.,
        hardness_pa: 1e8,
        wear_coefficient: 1e-3,
    };
    let wet = Properties {
        young_pa: 5e7,
        yield_pa: 1e5,
        hardness_pa: 5e7,
        ..dry
    };
    (dry, wet)
}
#[test]
fn transported_water_changes_calibrated_material_and_actual_mass() {
    let (dry, wet) = endpoints();
    let calibration = Calibration::new(dry, wet).unwrap();
    let mut body = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 0.,
        }],
        vec![],
    )
    .unwrap();
    let bath = Reservoir {
        cell: 0,
        saturation: 1.,
        conductance_kg_s: 1.,
    };
    let strain = [[0.005, 0., 0.], [0., -0.0025, 0.], [0., 0., -0.0025]];
    let (_, response) = body.cells()[0]
        .properties(calibration)
        .unwrap()
        .plastic_material()
        .unwrap()
        .response(&State::default(), strain)
        .unwrap();
    assert_eq!(response.plastic_increment, 0.);
    assert_eq!(body.cells()[0].wet_density_kg_m3(2., 0.001).unwrap(), 2000.);
    body.advance(1., &[bath]).unwrap();
    let cell = body.cells()[0];
    let properties = cell.properties(calibration).unwrap();
    assert!((cell.saturation().unwrap() - 0.5).abs() < 1e-12);
    assert!((properties.young_pa - 7.5e7).abs() < 1e-6);
    assert!((cell.wet_density_kg_m3(2., 0.001).unwrap() - 2500.).abs() < 1e-9);
    body.advance(9., &[bath]).unwrap();
    let (_, response) = body.cells()[0]
        .properties(calibration)
        .unwrap()
        .plastic_material()
        .unwrap()
        .response(&State::default(), strain)
        .unwrap();
    assert!(response.plastic_increment > 0.);
}
#[test]
fn calibrated_wet_hardness_controls_wear_without_assumed_universal_softening() {
    let (dry, wet) = endpoints();
    let calibration = Calibration::new(dry, wet).unwrap();
    let mut first = Layer::new(0.01, 0.002, 2500.).unwrap();
    let mut second = first.clone();
    let a = first
        .advance(
            Cell {
                capacity_kg: 1.,
                water_kg: 0.,
            }
            .properties(calibration)
            .unwrap()
            .wear_material()
            .unwrap(),
            100.,
            10.,
        )
        .unwrap();
    let b = second
        .advance(
            Cell {
                capacity_kg: 1.,
                water_kg: 1.,
            }
            .properties(calibration)
            .unwrap()
            .wear_material()
            .unwrap(),
            100.,
            10.,
        )
        .unwrap();
    assert!((b.volume_m3 - 2. * a.volume_m3).abs() < 1e-20);
    let strengthening = Calibration::new(wet, dry).unwrap();
    assert!(strengthening.at(1.).unwrap().young_pa > strengthening.at(0.).unwrap().young_pa);
}
#[test]
fn invalid_water_and_calibration_are_rejected() {
    let (dry, wet) = endpoints();
    let calibration = Calibration::new(dry, wet).unwrap();
    assert!(calibration.at(1.1).is_err());
    assert!(
        Calibration::new(
            Properties {
                hardness_pa: 0.,
                ..dry
            },
            wet
        )
        .is_err()
    );
    let invalid = Cell {
        capacity_kg: 1.,
        water_kg: 2.,
    };
    assert!(invalid.properties(calibration).is_err());
    assert!(invalid.wet_density_kg_m3(2., 0.001).is_err());
    assert!(
        Cell {
            capacity_kg: 1.,
            water_kg: 0.
        }
        .wet_density_kg_m3(2., 0.)
        .is_err()
    );
}
