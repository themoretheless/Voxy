use physics::biomechanics::{InertialBody, Material, TetraMesh};
use physics::tissue;

fn volume(mesh: &TetraMesh) -> f64 {
    // Independent oriented boundary integral, rather than FEM-private fields.
    mesh.boundary
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|i| mesh.points[i]);
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.
        })
        .sum()
}
fn material() -> Material {
    Material {
        shear_pa: 2000.,
        bulk_pa: 1e6,
        fibers: vec![],
    }
}

#[test]
fn rounded_fem_energy_and_virtual_work_match_uniform_shear() {
    for level in 0..=2 {
        let mesh = TetraMesh::ellipsoid([0.2, -0.1, 0.3], [0.3, 0.36, 0.25], level).unwrap();
        let reference_volume = volume(&mesh);
        let body = mesh
            .clone()
            .into_body(vec![false; mesh.points.len()], &material())
            .unwrap();
        for shear in [0.01, 0.3] {
            let positions: Vec<_> = mesh
                .points
                .iter()
                .map(|p| [p[0] + shear * p[1], p[1], p[2]])
                .collect();
            let (energy, gradient) = body.evaluate(&positions).unwrap();
            // For isochoric neo-Hookean simple shear: J=1 and W=mu*gamma²/2.
            let expected_energy = reference_volume * material().shear_pa * shear * shear / 2.;
            assert!((energy - expected_energy).abs() < expected_energy * 1e-9);
            let work_derivative: f64 = gradient
                .iter()
                .zip(&mesh.points)
                .map(|(g, p)| g[0] * p[1])
                .sum();
            let expected_derivative = reference_volume * material().shear_pa * shear;
            assert!((work_derivative - expected_derivative).abs() < expected_derivative * 1e-10);
            for axis in 0..3 {
                assert!(gradient.iter().map(|g| g[axis]).sum::<f64>().abs() < 1e-9);
            }
        }
    }
}

#[test]
fn continuum_and_compliant_owners_share_geometry_and_lumped_mass() {
    for level in 0..=2 {
        let mesh = TetraMesh::ellipsoid([0.; 3], [0.3, 0.36, 0.25], level).unwrap();
        let tissue = tissue::ellipsoid(
            [0.; 3],
            [0.3, 0.36, 0.25],
            1000.,
            level,
            &[],
            tissue::Material::default(),
        )
        .unwrap();
        assert_eq!(tissue.positions(), mesh.points);
        assert_eq!(tissue.tetrahedra().collect::<Vec<_>>(), mesh.cells);
        let body = mesh
            .clone()
            .into_body(vec![false; mesh.points.len()], &material())
            .unwrap();
        let inertial = InertialBody::new(
            body,
            &vec![1000.; mesh.cells.len()],
            vec![[0.; 3]; mesh.points.len()],
        )
        .unwrap();
        for (mass, inverse_mass) in inertial.masses().iter().zip(tissue.inverse_masses()) {
            assert!((mass - 1. / inverse_mass).abs() < mass * 1e-12);
        }
    }
}

#[test]
fn rounded_mesh_has_oriented_boundary_accepted_by_existing_refinement() {
    let mesh = TetraMesh::ellipsoid([0.; 3], [0.3, 0.36, 0.25], 1).unwrap();
    let initial_volume = volume(&mesh);
    let refined = mesh.refined_once().unwrap();
    assert!((volume(&refined) - initial_volume).abs() < initial_volume * 1e-12);
    assert_eq!(refined.boundary.len(), mesh.boundary.len() * 4);
    assert_eq!(refined.cells.len(), mesh.cells.len() * 8);
}
