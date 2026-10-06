use physics::biomechanics::{Material, TetraMesh};
use physics::tissue_surface::EmbeddedSurface;
fn volume(mesh: TetraMesh) -> f64 {
    let pins = vec![false; mesh.points.len()];
    mesh.into_body(
        pins,
        &Material {
            shear_pa: 10.,
            bulk_pa: 1000.,
            fibers: vec![],
        },
    )
    .unwrap()
    .reference_volume()
}
#[test]
fn nonconvex_lattice_union_preserves_cavity_boundary_and_analytic_volume() {
    let occupied = [[0, 0, 0], [1, 0, 0], [0, 1, 0]];
    let mesh = TetraMesh::from_lattice_cells([0.; 3], [1.; 3], &occupied).unwrap();
    assert_eq!(mesh.cells.len(), 18);
    assert_eq!(mesh.boundary.len(), 28);
    assert!((volume(mesh.clone()) - 3.).abs() < 1e-12);
    let skin = [[0.5, 0.5, 0.5], [1.5, 0.5, 0.5], [0.5, 1.5, 0.5]];
    EmbeddedSurface::bind(&mesh.points, &mesh.cells, &skin).unwrap();
    assert!(EmbeddedSurface::bind(&mesh.points, &mesh.cells, &[[1.5, 1.5, 0.5]]).is_err());
    let refined = mesh.refined_once().unwrap();
    assert_eq!(refined.boundary.len(), 4 * mesh.boundary.len());
    assert!((volume(refined) - 3.).abs() < 1e-12);
    let bytes = mesh.to_bytes().unwrap();
    assert_eq!(
        TetraMesh::from_bytes(&bytes).unwrap().to_bytes().unwrap(),
        bytes
    );
    let reversed =
        TetraMesh::from_lattice_cells([0.; 3], [1.; 3], &[[0, 1, 0], [1, 0, 0], [0, 0, 0]])
            .unwrap();
    assert_eq!(reversed.to_bytes().unwrap(), bytes);
}
#[test]
fn anisotropic_shifted_cells_preserve_si_volume() {
    let mesh =
        TetraMesh::from_lattice_cells([10., -20., 30.], [0.1, 0.2, 0.3], &[[0, 0, 0], [1, 0, 0]])
            .unwrap();
    assert_eq!(mesh.cells.len(), 12);
    assert_eq!(mesh.boundary.len(), 20);
    assert!((volume(mesh) - 0.012).abs() < 1e-14);
}
#[test]
fn malformed_lattice_inputs_are_rejected() {
    for cells in [vec![], vec![[0, 0, 0]; 2], vec![[u32::MAX, 0, 0]]] {
        assert!(TetraMesh::from_lattice_cells([0.; 3], [1.; 3], &cells).is_err());
    }
    for spacing in [[0., 1., 1.], [-1., 1., 1.], [f64::NAN, 1., 1.], [1e-6; 3]] {
        assert!(TetraMesh::from_lattice_cells([0.; 3], spacing, &[[0; 3]]).is_err());
    }
    assert!(TetraMesh::from_lattice_cells([f64::MAX; 3], [1.; 3], &[[0; 3]]).is_err());
}

#[test]
fn edge_and_point_welds_are_rejected_but_separate_components_and_cavities_are_valid() {
    for (other, error) in [
        ([1, 1, 0], "nonmanifold tetrahedral boundary edge"),
        ([1, 1, 1], "nonmanifold tetrahedral boundary vertex"),
    ] {
        assert_eq!(
            TetraMesh::from_lattice_cells([0.; 3], [1.; 3], &[[0; 3], other]).unwrap_err(),
            error
        );
    }
    let separated = TetraMesh::from_lattice_cells([0.; 3], [1.; 3], &[[0; 3], [2, 0, 0]]).unwrap();
    assert!((volume(separated) - 2.).abs() < 1e-12);
    let mut shell = Vec::new();
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                if [x, y, z] != [1; 3] {
                    shell.push([x, y, z]);
                }
            }
        }
    }
    let shell = TetraMesh::from_lattice_cells([0.; 3], [1.; 3], &shell).unwrap();
    assert_eq!(shell.boundary.len(), 120);
    assert!((volume(shell.clone()) - 26.).abs() < 1e-12);
    assert!(EmbeddedSurface::bind(&shell.points, &shell.cells, &[[1.5; 3]]).is_err());
}
