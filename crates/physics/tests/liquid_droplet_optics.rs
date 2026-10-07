use physics::liquid::*;
fn fixture() -> (Liquid, FiniteDropletGasGrid) {
    let particles = [0.25, 1.25, 0.75, 3.].map(|x| Particle {
        position: [x, 0.5, 0.5],
        velocity: [0.; 3],
        mass: 1.,
        material: 0,
    });
    let mut liquid =
        Liquid::new(particles.to_vec(), vec![Material::WATER], Config::default()).unwrap();
    liquid
        .configure_droplet_population(Some(vec![true, true, false, true]))
        .unwrap();
    let gas = FiniteDropletGasGrid::new(
        [0.; 3],
        [1.; 3],
        [2, 1, 1],
        vec![
            VaporCell {
                mass: 0.1,
                volume: 1.,
                temperature: 300.,
                velocity: [0.; 3],
                specific_heat_cv: 718.
            };
            2
        ],
    )
    .unwrap();
    (liquid, gas)
}
#[test]
fn physical_cross_sections_produce_analytic_optical_depth_and_transmission() {
    let (liquid, gas) = fixture();
    let before = format!("{liquid:?}");
    let old_gas = gas.clone();
    let field = liquid
        .droplet_extinction_grid(&gas, &[0.1, 0.2, 0.5, 0.1], 2.)
        .unwrap();
    assert_eq!(field.included_droplets, 2);
    assert_eq!(field.outside_droplets, 1);
    let expected = 2. * std::f64::consts::PI * (0.1_f64.powi(2) + 0.2_f64.powi(2));
    for (a, b) in [
        ([-1., 0.5, 0.5], [3., 0.5, 0.5]),
        ([3., 0.5, 0.5], [-1., 0.5, 0.5]),
    ] {
        assert!((field.optical_depth_segment(a, b).unwrap() - expected).abs() < 1e-14);
        assert!((field.transmittance_segment(a, b).unwrap() - (-expected).exp()).abs() < 1e-14);
    }
    assert_eq!(
        field
            .optical_depth_segment([0., 2., 0.], [2., 2., 0.])
            .unwrap(),
        0.
    );
    assert_eq!(field.optical_depth_segment([0.5; 3], [0.5; 3]).unwrap(), 0.);
    assert_eq!(
        field
            .optical_depth_segment([0., 1., 0.5], [2., 1., 0.5])
            .unwrap(),
        0.
    );
    let along_shared_face = field
        .optical_depth_segment([1., 0., 0.5], [1., 1., 0.5])
        .unwrap();
    assert!((along_shared_face - 2. * std::f64::consts::PI * 0.2_f64.powi(2)).abs() < 1e-14);
    let diagonal = field
        .optical_depth_segment([0., 0., 0.5], [2., 1., 0.5])
        .unwrap();
    assert!((diagonal - expected * 5_f64.sqrt() / 2.).abs() < 1e-14);
    assert_eq!(format!("{liquid:?}"), before);
    assert_eq!(gas, old_gas);
}
#[test]
fn invalid_optical_inputs_reject_and_transparent_efficiency_remains_transparent() {
    let (liquid, gas) = fixture();
    for q in [-1., f64::NAN, f64::INFINITY] {
        assert!(liquid.droplet_extinction_grid(&gas, &[0.1; 4], q).is_err());
    }
    assert!(liquid.droplet_extinction_grid(&gas, &[0.1; 3], 2.).is_err());
    assert!(
        liquid
            .droplet_extinction_grid(&gas, &[f64::MAX; 4], 2.)
            .is_err()
    );
    let field = liquid.droplet_extinction_grid(&gas, &[0.1; 4], 0.).unwrap();
    assert_eq!(
        field
            .transmittance_segment([-1., 0.5, 0.5], [3., 0.5, 0.5])
            .unwrap(),
        1.
    );
    assert!(
        field
            .optical_depth_segment([f64::NAN, 0., 0.], [0.; 3])
            .is_err()
    );
}
