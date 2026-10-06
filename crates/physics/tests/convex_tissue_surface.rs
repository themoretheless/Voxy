use physics::biomechanics::{InertialBody, Material, TetraMesh};
use physics::tissue_surface::EmbeddedSurface;

fn cube() -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
    (
        vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ],
        vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    )
}

#[test]
fn complete_source_skin_embeds_and_affine_motion_preserves_exact_cube_mass() {
    let (skin, faces) = cube();
    let mesh = TetraMesh::from_convex_surface(skin.clone(), faces.clone(), [0.5; 3]).unwrap();
    assert_eq!(&mesh.points[..skin.len()], skin);
    assert_eq!(mesh.boundary, faces);
    assert_eq!(mesh.cells.len(), 12);
    assert_eq!(mesh.points.len(), 9);
    let map = EmbeddedSurface::bind(&mesh.points, &mesh.cells, &skin).unwrap();
    let affine = |p: [f64; 3]| {
        [
            2. * p[0] + 0.25 * p[1] + 3.,
            0.5 * p[1] - 2.,
            p[2] + 0.1 * p[0],
        ]
    };
    let nodes: Vec<_> = mesh.points.iter().copied().map(affine).collect();
    let deformed = map.deform(&nodes).unwrap();
    for (&actual, &point) in deformed.iter().zip(&skin) {
        for (a, b) in actual.into_iter().zip(affine(point)) {
            assert!((a - b).abs() < 1e-12);
        }
    }
    let material = Material {
        shear_pa: 500.,
        bulk_pa: 5000.,
        fibers: vec![],
    };
    let body = mesh.into_body(vec![false; 9], &material).unwrap();
    let dynamics = InertialBody::new(body, &[1000.; 12], vec![[0.; 3]; 9]).unwrap();
    assert!((dynamics.masses().iter().sum::<f64>() - 1000.).abs() < 1e-10);
    // Boundary loads are transferred through the exact same complete-volume map.
    let loads = vec![[0., 0., -1.]; skin.len()];
    let mut nodal = vec![[0.; 3]; 9];
    map.accumulate_forces_into(&loads, &mut nodal).unwrap();
    assert_eq!(nodal[8], [0.; 3]);
    assert_eq!(&nodal[..8], loads);
}

#[test]
fn rejects_open_inward_nonconvex_and_unusable_source_geometry() {
    let (points, faces) = cube();
    let build =
        |p: Vec<[f64; 3]>, f: Vec<[usize; 3]>, center| TetraMesh::from_convex_surface(p, f, center);
    assert!(build(points.clone(), faces[..11].to_vec(), [0.5; 3]).is_err());
    let mut duplicate = faces.clone();
    duplicate.push(faces[0]);
    assert!(build(points.clone(), duplicate, [0.5; 3]).is_err());
    let mut inward = faces.clone();
    inward[0].swap(1, 2);
    assert!(build(points.clone(), inward, [0.5; 3]).is_err());
    assert!(build(points.clone(), faces.clone(), [0.; 3]).is_err());
    assert!(build(points.clone(), faces.clone(), [2.; 3]).is_err());
    let mut concave = points.clone();
    concave[6] = [0.4; 3];
    assert!(build(concave, faces.clone(), [0.5; 3]).is_err());
    let mut coincident = points.clone();
    coincident[7] = points[6];
    assert!(build(coincident, faces.clone(), [0.5; 3]).is_err());
    let mut unused = points.clone();
    unused.push([0.5; 3]);
    assert!(build(unused, faces.clone(), [0.4; 3]).is_err());
    let mut nonfinite = points.clone();
    nonfinite[0][0] = f64::NAN;
    assert!(build(nonfinite, faces.clone(), [0.5; 3]).is_err());
    let mut invalid = faces.clone();
    invalid[0][0] = 100;
    assert!(build(points, invalid, [0.5; 3]).is_err());
}

#[test]
fn sheared_source_boundary_refines_conformingly_without_losing_volume() {
    let (mut points, faces) = cube();
    let transform = |p: [f64; 3]| {
        [
            2. * p[0] + p[1] + 10.,
            3. * p[1] - 2. * p[2] - 5.,
            4. * p[2] + 7.,
        ]
    };
    for point in &mut points {
        *point = transform(*point);
    }
    let mesh = TetraMesh::from_convex_surface(points.clone(), faces, transform([0.5; 3])).unwrap();
    let material = Material {
        shear_pa: 500.,
        bulk_pa: 5000.,
        fibers: vec![],
    };
    for mesh in [mesh.clone(), mesh.refined_once().unwrap()] {
        EmbeddedSurface::bind(&mesh.points, &mesh.cells, &points).unwrap();
        let count = mesh.points.len();
        let cells = mesh.cells.len();
        let dynamics = InertialBody::new(
            mesh.into_body(vec![false; count], &material).unwrap(),
            &vec![1000.; cells],
            vec![[0.; 3]; count],
        )
        .unwrap();
        assert!((dynamics.masses().iter().sum::<f64>() - 24_000.).abs() < 1e-8);
    }
}

#[test]
fn authored_volume_round_trips_existing_interchange_and_rejects_invalid_export() {
    let (points, faces) = cube();
    let mesh = TetraMesh::from_convex_surface(points, faces, [0.5; 3]).unwrap();
    let bytes = mesh.to_bytes().unwrap();
    let decoded = TetraMesh::from_bytes(&bytes).unwrap();
    assert_eq!(mesh.points, decoded.points);
    assert_eq!(mesh.cells, decoded.cells);
    assert_eq!(mesh.boundary, decoded.boundary);
    assert_eq!(bytes, decoded.to_bytes().unwrap());
    let mut malformed = mesh.clone();
    malformed.boundary.pop();
    assert!(malformed.to_bytes().is_err());
    let mut nonfinite = mesh;
    nonfinite.points.push([f64::NAN; 3]);
    assert!(nonfinite.to_bytes().is_err());
}
