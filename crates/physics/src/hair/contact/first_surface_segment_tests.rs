use super::*;
#[test]
fn segment_hits_finite_surface_and_reports_distance_and_orientation() {
    let points = [
        [-1., -1., 0.],
        [1., -1., 0.],
        [0., 1., 0.],
        [-1., -1., -0.002],
        [0., 1., -0.002],
        [1., -1., -0.002],
    ];
    let mesh = TriangleMesh::new(&points, &[[0, 1, 2], [3, 4, 5]]).unwrap();
    let (fraction, point, normal) = mesh
        .first_segment_hit([0., 0., -0.00002], [0., 0., -0.01])
        .unwrap()
        .unwrap();
    assert!((fraction * 0.00998 - 0.00198).abs() < 1e-12);
    assert!((point[2] + 0.002).abs() < 1e-12 && normal[2] < -0.99);
    assert!(
        mesh.first_segment_hit([2., 0., -0.00002], [2., 0., -0.01])
            .unwrap()
            .is_none()
    );
    assert!(mesh.first_segment_hit([0.; 3], [0.; 3]).is_err());
}
