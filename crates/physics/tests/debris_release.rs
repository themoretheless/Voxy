//! Plan step A1: rigid debris extraction conserves the consistent-mass
//! fragment integrals and produces valid convex hulls.
use physics::plasticity::Material;
use physics::plasticity::mesh::{
    DebrisPiece, QuadraticBody, QuadraticDynamics, convex_hull, hull_signed_distance,
};

fn two_tets() -> QuadraticBody {
    let material = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    QuadraticBody::from_linear(
        vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., -1.],
        ],
        vec![([0, 1, 2, 3], material), ([4, 5, 6, 7], material)],
    )
    .unwrap()
}

fn velocities(body: &QuadraticBody) -> Vec<[f64; 3]> {
    let groups = body.fragment_nodes();
    let mut velocities = vec![[0.; 3]; body.positions().len()];
    for &i in &groups[0] {
        velocities[i] = [0.02, -0.03, -0.5];
    }
    for &i in &groups[1] {
        velocities[i] = [0.1, 0.2, 0.5];
    }
    // Add a rigid spin to the second fragment about its first node.
    let origin = body.positions()[groups[1][0]];
    for &i in &groups[1] {
        let r = [
            body.positions()[i][0] - origin[0],
            body.positions()[i][1] - origin[1],
            body.positions()[i][2] - origin[2],
        ];
        let omega = [0., 0., 2.];
        velocities[i][0] += omega[1] * r[2] - omega[2] * r[1];
        velocities[i][1] += omega[2] * r[0] - omega[0] * r[2];
        velocities[i][2] += omega[0] * r[1] - omega[1] * r[0];
    }
    velocities
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[test]
fn release_matches_fragment_integrals_and_keeps_everything_inside_hulls() {
    let body = two_tets();
    let velocities = velocities(&body);
    let release = body
        .release_debris(&[1000., 1000.], &velocities, 0.)
        .unwrap();
    assert_eq!(release.pieces.len(), 2);
    assert!(release.retained_nodes.is_empty());
    let dynamics =
        QuadraticDynamics::new(body.clone(), &[1000., 1000.], velocities.clone()).unwrap();
    let fragments = dynamics.fragments().unwrap();
    let mut total_mass = 0.;
    let mut total_momentum = [0.; 3];
    let mut total_angular = [0.; 3];
    for (piece, fragment) in release.pieces.iter().zip(&fragments) {
        assert_eq!(piece.nodes, fragment.nodes);
        assert!((piece.mass_kg - fragment.mass_kg).abs() <= 1e-12 * fragment.mass_kg);
        for axis in 0..3 {
            assert!((piece.center_m[axis] - fragment.center_m[axis]).abs() <= 1e-12);
            assert!((piece.velocity_m_s[axis] - fragment.velocity_m_s[axis]).abs() <= 1e-12);
        }
        total_mass += piece.mass_kg;
        let momentum = piece.velocity_m_s.map(|v| v * piece.mass_kg);
        let orbital = cross(piece.center_m, momentum);
        let spin = {
            let i = piece.inertia_kg_m2;
            let w = piece.angular_velocity_rad_s;
            [
                i[0][0] * w[0] + i[0][1] * w[1] + i[0][2] * w[2],
                i[1][0] * w[0] + i[1][1] * w[1] + i[1][2] * w[2],
                i[2][0] * w[0] + i[2][1] * w[1] + i[2][2] * w[2],
            ]
        };
        for axis in 0..3 {
            total_momentum[axis] += momentum[axis];
            total_angular[axis] += orbital[axis] + spin[axis];
        }
        // Every fragment node lies inside (or on) its hull.
        for &node in &piece.nodes {
            let d = hull_signed_distance(&piece.hull, &piece.hull_faces, body.positions()[node]);
            assert!(d <= 1e-9, "node {node} outside hull by {d}");
        }
        assert_eq!(piece.hull.len(), 4, "a tetrahedron's hull is its corners");
        assert_eq!(piece.hull_faces.len(), 4);
    }
    // The rigid decomposition of each fragment reproduces its total angular momentum.
    let expected_angular: [f64; 3] = fragments.iter().fold([0.; 3], |acc, f| {
        [
            acc[0] + f.angular_momentum_kg_m2_s[0],
            acc[1] + f.angular_momentum_kg_m2_s[1],
            acc[2] + f.angular_momentum_kg_m2_s[2],
        ]
    });
    let expected_momentum: [f64; 3] = fragments.iter().fold([0.; 3], |acc, f| {
        [
            acc[0] + f.momentum_kg_m_s[0],
            acc[1] + f.momentum_kg_m_s[1],
            acc[2] + f.momentum_kg_m_s[2],
        ]
    });
    let expected_mass: f64 = fragments.iter().map(|f| f.mass_kg).sum();
    assert!((total_mass - expected_mass).abs() <= 1e-12 * expected_mass);
    for axis in 0..3 {
        assert!(
            (total_momentum[axis] - expected_momentum[axis]).abs() <= 1e-10,
            "{total_momentum:?} vs {expected_momentum:?}"
        );
        assert!(
            (total_angular[axis] - expected_angular[axis]).abs() <= 1e-10,
            "{total_angular:?} vs {expected_angular:?}"
        );
    }
    // The second fragment was given a pure rigid spin: no deformation energy remains.
    assert!(release.pieces[1].deformation_kinetic_j <= 1e-10 * release.pieces[1].kinetic_j);
    assert!((release.pieces[1].angular_velocity_rad_s[2] - 2.).abs() < 1e-9);
    let total_kinetic: f64 = fragments.iter().map(|f| f.kinetic_j).sum();
    assert!(
        (release.kinetic_j + release.deformation_kinetic_j - total_kinetic).abs()
            <= 1e-10 * total_kinetic
    );
    assert_eq!(
        release.surface_created_m2, 0.,
        "no cohesive faces in this fixture"
    );
}

#[test]
fn mass_threshold_retains_small_fragments_and_input_is_validated() {
    let body = two_tets();
    let velocities = velocities(&body);
    let release = body
        .release_debris(&[1000., 1000.], &velocities, 1e9)
        .unwrap();
    assert!(release.pieces.is_empty());
    assert_eq!(release.retained_nodes.len(), body.positions().len());
    assert!(body.release_debris(&[1000.], &velocities, 0.).is_err());
    assert!(
        body.release_debris(&[1000., 1000.], &velocities[1..], 0.)
            .is_err()
    );
    assert!(
        body.release_debris(&[1000., 1000.], &velocities, f64::NAN)
            .is_err()
    );
}

#[test]
fn convex_hull_of_a_cube_with_interior_points_has_eight_vertices() {
    let mut points = Vec::new();
    for sx in [0., 1.] {
        for sy in [0., 1.] {
            for sz in [0., 1.] {
                points.push([sx, sy, sz]);
            }
        }
    }
    points.push([0.5, 0.5, 0.5]);
    points.push([0.2, 0.7, 0.4]);
    let (hull, faces) = convex_hull(&points).unwrap();
    assert_eq!(hull.len(), 8);
    assert_eq!(faces.len(), 12);
    for p in &points {
        assert!(hull_signed_distance(&hull, &faces, *p) <= 1e-12);
    }
    assert!(hull_signed_distance(&hull, &faces, [2., 0.5, 0.5]) > 0.99);
    // Faces are outward: the centre is behind every face.
    assert!(hull_signed_distance(&hull, &faces, [0.5, 0.5, 0.5]) < -0.49);
    assert!(convex_hull(&[[0.; 3], [1., 0., 0.], [2., 0., 0.], [3., 0., 0.]]).is_err());
    assert!(convex_hull(&[[0.; 3], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]]).is_err());
    let _: Option<DebrisPiece> = None;
}
