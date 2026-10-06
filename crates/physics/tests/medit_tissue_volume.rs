use physics::biomechanics::{Material, TetraMesh};
use physics::tissue_surface::EmbeddedSurface;

const VOLUME: &str = "MeshVersionFormatted 1
Dimension 3
Vertices 5
0 0 0 0
1 0 0 0
0 1 0 0
0 0 1 0
0 0 -1 0
Triangles 0
Tetrahedra 2
1 2 3 4 0
1 2 3 5 0
End";

#[test]
fn offline_volume_import_normalizes_order_and_keeps_boundary_mass_and_skin() {
    // The second input cell is reversed. Normalize ordering, never coordinates.
    let mesh = TetraMesh::from_medit_volume(VOLUME).unwrap();
    assert_eq!(mesh.cells, vec![[0, 1, 2, 3], [0, 2, 1, 4]]);
    assert_eq!(mesh.boundary.len(), 6);
    assert_eq!(mesh.points.len(), 5);
    let embedding = EmbeddedSurface::bind(&mesh.points, &mesh.cells, &mesh.points).unwrap();
    assert_eq!(embedding.deform(&mesh.points).unwrap(), mesh.points);
    let decoded = TetraMesh::from_bytes(&mesh.to_bytes().unwrap()).unwrap();
    assert_eq!(decoded.points, mesh.points);
    assert_eq!(decoded.cells, mesh.cells);
    assert_eq!(decoded.boundary, mesh.boundary);
    let body = mesh
        .into_body(
            vec![false; 5],
            &Material {
                shear_pa: 10.,
                bulk_pa: 1000.,
                fibers: vec![],
            },
        )
        .unwrap();
    assert!((body.reference_volume() - 1. / 3.).abs() < 1e-15);
    // Explicit cells preserve the caller's ordering contract instead of fixing it.
    assert!(TetraMesh::from_tetrahedra(decoded.points, vec![[0, 1, 2, 4]]).is_err());
}

#[test]
fn offline_volume_rejects_labels_unknown_sections_bad_cells_and_resource_overflow() {
    for invalid in [
        VOLUME.replace("Dimension 3", "Dimension 2"),
        VOLUME.replace("Vertices 5", "Vertices 1000001"),
        VOLUME.replace("Tetrahedra 2", "Tetrahedra 250001"),
        VOLUME.replace("0 0 -1 0", "0 0 -1 42"),
        VOLUME.replace("1 2 3 4 0", "1 2 3 4 42"),
        VOLUME.replace("Triangles 0", "Triangles 1\n1 2 3 0"),
        VOLUME.replace("1 2 3 4 0", "0 2 3 4 0"),
        VOLUME.replace("1 2 3 4 0", "6 2 3 4 0"),
        VOLUME.replace("1 2 3 4 0", "1 2 3 3 0"),
        VOLUME.replace("0 0 -1 0", "NaN 0 -1 0"),
        VOLUME.replace("1 2 3 5 0", "1 2 3 4 0"),
        VOLUME.replace("End", "End trailing-data"),
        VOLUME.replace("End", ""),
    ] {
        assert!(TetraMesh::from_medit_volume(&invalid).is_err(), "{invalid}");
    }
    assert!(TetraMesh::from_medit_volume(&format!("# native geometry\n{VOLUME}\n# end")).is_ok());
}
