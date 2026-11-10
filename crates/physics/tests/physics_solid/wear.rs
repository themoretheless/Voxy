use physics::wear::{Layer, Material};
#[test]
fn archard_depth_load_distance_scaling_and_mass_inventory() {
    let material = Material::new(1e8, 1e-3).unwrap();
    let mut layer = Layer::new(0.01, 0.002, 2500.).unwrap();
    let initial = layer.remaining_mass_kg();
    let report = layer.advance(material, 100., 10.).unwrap();
    assert!((report.volume_m3 - 1e-8).abs() < 1e-20);
    assert!((report.depth_m - 1e-6).abs() < 1e-18);
    assert!((report.mass_kg - 2.5e-5).abs() < 1e-16);
    assert!((layer.remaining_mass_kg() + layer.debris_mass_kg() - initial).abs() < 1e-16);
    assert!(!report.exhausted);
    let mut twice = Layer::new(0.01, 0.002, 2500.).unwrap();
    let r = twice.advance(material, 200., 10.).unwrap();
    assert!((r.volume_m3 - 2. * report.volume_m3).abs() < 1e-20);
    let mut subdivided = Layer::new(0.01, 0.002, 2500.).unwrap();
    for _ in 0..100 {
        subdivided.advance(material, 100., 0.1).unwrap();
    }
    assert!((subdivided.thickness_m() - layer.thickness_m()).abs() < 1e-16);
    // At fixed pressure and distance, different patch areas lose the same depth.
    let mut large = Layer::new(0.02, 0.002, 2500.).unwrap();
    let r = large.advance(material, 200., 10.).unwrap();
    assert!((r.depth_m - report.depth_m).abs() < 1e-18);
}
#[test]
fn exhaustion_preserves_debris_and_unprocessed_distance_for_underlying_layer() {
    let material = Material::new(1000., 0.1).unwrap();
    let mut first = Layer::new(0.5, 0.002, 2000.).unwrap();
    let first_mass = first.remaining_mass_kg();
    // Rate 0.001 m³/m exhausts 0.001 m³ in one metre, leaving two metres.
    let removal = first.advance(material, 10., 3.).unwrap();
    assert!(removal.exhausted);
    assert_eq!(first.thickness_m(), 0.);
    assert!((removal.consumed_sliding_distance_m - 1.).abs() < 1e-12);
    assert!((removal.remaining_sliding_distance_m - 2.).abs() < 1e-12);
    assert!((first.debris_mass_kg() - first_mass).abs() < 1e-12);
    let again = first
        .advance(material, 10., removal.remaining_sliding_distance_m)
        .unwrap();
    assert_eq!(again.volume_m3, 0.);
    assert_eq!(again.consumed_sliding_distance_m, 0.);
    assert_eq!(again.remaining_sliding_distance_m, 2.);
    let mut second = Layer::new(0.5, 0.01, 1000.).unwrap();
    let r = second
        .advance(material, 10., removal.remaining_sliding_distance_m)
        .unwrap();
    assert!((r.volume_m3 - 0.002).abs() < 1e-12);
}
#[test]
fn no_sliding_no_load_and_failures_do_not_remove_material() {
    let material = Material::new(1e8, 1e-3).unwrap();
    let mut layer = Layer::new(0.01, 0.002, 2500.).unwrap();
    for (load, distance) in [(0., 10.), (100., 0.)] {
        assert_eq!(
            layer.advance(material, load, distance).unwrap().volume_m3,
            0.
        );
    }
    assert_eq!(
        layer
            .advance(Material::new(1e8, 0.).unwrap(), 100., 10.)
            .unwrap()
            .volume_m3,
        0.
    );
    let thickness = layer.thickness_m();
    assert!(layer.advance(material, -1., 10.).is_err());
    assert!(layer.advance(material, 100., f64::NAN).is_err());
    assert_eq!(layer.thickness_m(), thickness);
    assert!(Material::new(0., 1.).is_err());
    assert!(Layer::new(0., 1., 1000.).is_err());
    assert!(
        layer
            .advance(Material::new(1e-300, 1e300).unwrap(), 1e300, 1.)
            .is_err()
    );
    assert_eq!(layer.thickness_m(), thickness);
}
