use physics::tissue::{Material, ellipsoid};
use physics::tissue_surface::EmbeddedSurface;

#[test]
fn refinement_preserves_axes_and_converges_volume_and_mass() {
    let center = [1., -2., 3.];
    let radii = [0.3, 0.36, 0.25];
    let density = 1000.;
    let exact = 4. / 3. * std::f64::consts::PI * radii.iter().product::<f64>();
    let mut previous = 0.;
    let mut coarse_axes = Vec::new();
    for level in 0..=3 {
        let body = ellipsoid(center, radii, density, level, &[], Material::default()).unwrap();
        if level == 0 {
            coarse_axes = body.positions()[..7].to_vec();
        }
        assert_eq!(body.positions()[..7], coarse_axes);
        let volume: f64 = body.volumes().iter().map(|v| v.abs()).sum();
        assert!(volume > previous && volume < exact);
        previous = volume;
        let mass: f64 = body.inverse_masses().iter().map(|w| 1. / w).sum();
        assert!((mass - density * volume).abs() < 1e-11 * mass);
        assert_eq!(body.tetrahedra().count(), 8 * 4_usize.pow(level));
        let mut boundary_edges = std::collections::BTreeMap::new();
        for &[a, b, c] in body.surface_triangles() {
            for (u, v) in [(a, b), (b, c), (c, a)] {
                *boundary_edges.entry((u.min(v), u.max(v))).or_insert(0) += 1;
            }
        }
        assert!(boundary_edges.values().all(|&count| count == 2));
        assert_eq!(
            body.positions().len() - 1 + body.surface_triangles().len(),
            boundary_edges.len() + 2
        );
        for p in &body.positions()[1..] {
            let radius: f64 = (0..3)
                .map(|i| ((p[i] - center[i]) / radii[i]).powi(2))
                .sum();
            assert!((radius - 1.).abs() < 1e-12);
        }
        // Render and collision boundary can use the same physical geometry.
        let surface: Vec<_> = body
            .surface_triangles()
            .iter()
            .flat_map(|f| f.map(|i| body.positions()[i]))
            .collect();
        let bound = EmbeddedSurface::bind(
            body.positions(),
            &body.tetrahedra().collect::<Vec<_>>(),
            &surface,
        )
        .unwrap();
        for (actual, expected) in bound.deform(body.positions()).unwrap().iter().zip(surface) {
            assert!((0..3).all(|i| (actual[i] - expected[i]).abs() < 1e-12));
        }
    }
    assert!((exact - previous) / exact < 0.025);
}

#[test]
fn pinned_axes_and_uniform_free_fall_use_existing_solver() {
    let mut body = ellipsoid([0.; 3], [0.3; 3], 1000., 1, &[], Material::default()).unwrap();
    let before = body.positions().to_vec();
    body.step(1. / 240., [0., -9.81, 0.], &[], 16).unwrap();
    let displacement = body.positions()[0][1] - before[0][1];
    assert!(displacement < 0.);
    for (actual, rest) in body.positions().iter().zip(before) {
        assert!((actual[1] - rest[1] - displacement).abs() < 1e-12);
    }
    let pinned = ellipsoid([0.; 3], [0.3; 3], 1000., 2, &[3, 6], Material::default()).unwrap();
    for (i, weight) in pinned.inverse_masses().iter().enumerate() {
        assert_eq!(*weight == 0., [3, 6].contains(&i));
    }
}

#[test]
fn invalid_and_unrepresentable_geometry_is_rejected() {
    assert_eq!(
        ellipsoid([0.; 3], [1e50; 3], 1.5e158, 0, &[], Material::default()).unwrap_err(),
        "tissue ellipsoid total mass overflow"
    );
    for (radii, density, level, pins) in [
        ([0.; 3], 1000., 1, vec![]),
        ([f64::NAN; 3], 1000., 1, vec![]),
        ([0.3; 3], 0., 1, vec![]),
        ([0.3; 3], 1000., 4, vec![]),
        ([0.3; 3], 1000., 1, vec![0]),
        ([0.3; 3], 1000., 1, vec![7]),
        ([1e-100; 3], 1e-100, 1, vec![]),
        ([f64::MAX; 3], 1000., 1, vec![]),
    ] {
        assert!(ellipsoid([0.; 3], radii, density, level, &pins, Material::default()).is_err());
    }
    assert!(ellipsoid([f64::MAX; 3], [0.3; 3], 1000., 1, &[], Material::default()).is_err());
}
