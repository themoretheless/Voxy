use physics::biomechanics::*;
#[test]
fn radial_refinement_preserves_layer_regions_boundaries_and_materials() {
    let geometry = UrogenitalWallGeometry {
        radii_m: [0.008, 0.009, 0.010, 0.011, 0.012],
        length_m: 0.03,
        sectors: 16,
        segments: 3,
    };
    let materials: [Material; 4] = std::array::from_fn(|i| {
        Material::from_young_poisson(5000. + 1000. * i as f64, 0.3).unwrap()
    });
    let profiles = vec![materials.clone(); 3];
    let original = geometry.axial_wall([1., 0.3], &profiles).unwrap();
    for n in [1, 2, 3] {
        let body = geometry
            .axial_wall_refined([1., 0.3], &profiles, n)
            .unwrap();
        assert_eq!(body.positions().len(), 4 * (4 * n + 1) * 16);
        assert_eq!(body.elements().len(), original.elements().len() * n);
        for segment in 0..=3 {
            for layer in 0..=4 {
                for angle in 0..16 {
                    assert_eq!(
                        body.positions()[segment * (4 * n + 1) * 16 + layer * n * 16 + angle],
                        original.positions()[segment * 5 * 16 + layer * 16 + angle]
                    );
                }
            }
        }
        for e in body.elements() {
            assert!(e.region < 12);
            assert_eq!(e.material.shear_pa, materials[e.region % 4].shear_pa);
        }
        if n == 1 {
            assert_eq!(body.positions(), original.positions());
        }
    }
    assert!(
        geometry
            .axial_wall_refined([1., 0.3], &profiles, 0)
            .is_err()
    );
    assert!(
        geometry
            .axial_wall_refined([1., 0.3], &profiles, 4)
            .is_err()
    );
}
