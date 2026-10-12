use super::*;

#[test]
fn interior_peak_near_endpoint_is_not_hidden_by_endpoint_sample() {
    let peak = 0.2 / (3f64.sqrt() + 1.);
    for scale in [1e-3, 1., 1e3] {
        let points =
            [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]].map(|p| mul(p, scale));
        let mut mesh =
            TriangleMesh::new(&points, &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]).unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        let a = mul([0., 0.4, 0.4], scale);
        let b = mul([peak / 0.99, 0.4, 0.4], scale);
        for (a, b, expected_t) in [(a, b, 0.99), (b, a, 0.01)] {
            let (t, signed, _) = deepest_interval_witness(&mesh, a, b, 0., 1.).unwrap();
            assert!(
                (signed / scale + peak).abs() < 1e-8,
                "missed analytic interior depth: {signed} at {t}"
            );
            assert!(
                (signed + peak * scale).abs() < 2e-12,
                "witness precision is insufficient for the 1e-10 m contact gate"
            );
            assert!(
                (t - expected_t).abs() < 1e-6,
                "interior ridge was replaced by endpoint"
            );
        }
    }
}

#[test]
fn medial_envelope_gradient_matches_analytic_tetrahedron_derivative() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let mut mesh =
        TriangleMesh::new(&points, &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]).unwrap();
    mesh.enable_closed_feature_normals().unwrap();
    let a = [0., 0.4, 0.4];
    let b = [0.2, 0.4, 0.4];
    let (t, _, normal) = deepest_interval_witness(&mesh, a, b, 0., 1.).unwrap();
    let (direction, scale, _) = interior_envelope_gradient(&mesh, a, b, t, 0., 1., normal, [0.; 3]);
    let gradient = mul(direction, scale);
    let expected = 1. / (3f64.sqrt() + 1.);
    assert!(gradient[0].abs() < 1e-10);
    assert!((gradient[1] - expected).abs() < 1e-8 && (gradient[2] - expected).abs() < 1e-8);
    let h = 1e-5;
    let shifted = |dy| {
        deepest_interval_witness(&mesh, add(a, [0., dy, 0.]), add(b, [0., dy, 0.]), 0., 1.)
            .unwrap()
            .1
    };
    let derivative = (shifted(h) - shifted(-h)) / (2. * h);
    assert!((derivative - gradient[1]).abs() < 1e-4);
}

#[test]
fn interior_witness_finds_deeper_contact_than_crossing_midpoint() {
    for scale in [1e-3, 1., 1e3] {
        let points =
            [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]].map(|p| mul(p, scale));
        let mut mesh =
            TriangleMesh::new(&points, &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]).unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        let a = mul([0., 0.2, 0.2], scale);
        let b = mul([0.6, 0.2, 0.2], scale);
        let (_, depth, _) = deepest_interval_witness(&mesh, a, b, 0., 1.).unwrap();
        let midpoint = mesh.signed_distance_closed(mul(add(a, b), 0.5)).unwrap().0;
        assert!(
            (depth / scale + 0.2).abs() < 1e-8,
            "tetrahedron has analytic maximum interior depth 0.2"
        );
        assert!(depth < midpoint - 0.02 * scale);
    }
}

#[test]
fn fully_interior_segment_discovers_depth_without_boundary_crossings() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let mut mesh =
        TriangleMesh::new(&points, &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]).unwrap();
    mesh.enable_closed_feature_normals().unwrap();
    let mut rod = HairRod::new(
        vec![[0.05, 0.2, 0.2], [0.1, 0.2, 0.2], [0.5, 0.2, 0.2]],
        Default::default(),
    )
    .unwrap();
    let before = rod.x.clone();
    refresh_mesh_constraints(&mut rod, &[mesh], 40e-6);
    assert_eq!(rod.x, before);
    assert!(
        rod.contacts.iter().any(|c| {
            let p = add(
                mul(rod.x[c.segment], 1. - c.fraction),
                mul(rod.x[c.segment + 1], c.fraction),
            );
            c.segment == 1
                && c.fraction > 0.
                && c.fraction < 1.
                && dot(sub(p, c.target), c.normal) < -0.199
        }),
        "wholly interior segment must expose analytic interior depth 0.2"
    );
}

#[test]
fn crossing_segment_recovers_interior_depth_with_exterior_endpoints() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let mut mesh =
        TriangleMesh::new(&points, &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]).unwrap();
    mesh.enable_closed_feature_normals().unwrap();
    let mut rod = HairRod::new(
        vec![[-0.5, 0.2, 0.2], [-0.1, 0.2, 0.2], [0.9, 0.2, 0.2]],
        Default::default(),
    )
    .unwrap();
    let before = rod.x.clone();
    refresh_mesh_constraints(&mut rod, &[mesh], 40e-6);
    assert_eq!(rod.x, before, "constraint discovery must not move the rod");
    assert!(
        rod.contacts.iter().any(|c| {
            let p = add(
                mul(rod.x[c.segment], 1. - c.fraction),
                mul(rod.x[c.segment + 1], c.fraction),
            );
            dot(sub(p, c.target), c.normal) < -0.1
        }),
        "surface intersection must expose interior penetration, not just fiber radius"
    );
}

#[test]
fn feature_normals_classify_faces_edges_vertices_and_refresh_after_motion() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let faces = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
    let mut mesh = TriangleMesh::new(&points, &faces).unwrap();
    assert!(mesh.signed_distance_closed([0.1; 3]).is_err());
    mesh.enable_closed_feature_normals().unwrap();
    for (point, inside) in [
        ([0.1; 3], true),
        ([-0.1; 3], false),
        ([-0.1, -0.1, 0.3], false),
        ([0.3, 0.3, -0.1], false),
    ] {
        let (distance, normal) = mesh.signed_distance_closed(point).unwrap();
        assert_eq!(distance < 0., inside);
        assert!((len(normal) - 1.).abs() < 1e-14);
        assert_eq!(mesh.contains_closed_surface(point).unwrap(), Some(inside));
    }
    let (_, vertex_normal) = mesh.signed_distance_closed([-0.1; 3]).unwrap();
    assert!(len(sub(vertex_normal, unit([-1.; 3]))) < 1e-14);
    let (_, edge_normal) = mesh.signed_distance_closed([-0.1, -0.1, 0.3]).unwrap();
    assert!(len(sub(edge_normal, unit([-1., -1., 0.]))) < 1e-14);
    let moved = points.map(|point| add(point, [2., 3., 4.]));
    mesh.refit(&moved).unwrap();
    assert!(mesh.signed_distance_closed([2.1, 3.1, 4.1]).unwrap().0 < 0.);
    let transformed = mesh
        .transformed([[1., 0., 0., -2.], [0., 1., 0., -3.], [0., 0., 1., -4.]])
        .unwrap();
    assert!(transformed.signed_distance_closed([0.1; 3]).unwrap().0 < 0.);
}

#[test]
fn feature_topology_welds_seams_and_rejects_open_or_split_surfaces() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let faces = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
    let mut seam_points: Vec<V> = faces
        .iter()
        .flat_map(|face| face.map(|i| points[i]))
        .collect();
    let seam_faces = [[0, 1, 2], [3, 4, 5], [6, 7, 8], [9, 10, 11]];
    let mut mesh = TriangleMesh::new(&seam_points, &seam_faces).unwrap();
    mesh.enable_closed_feature_normals().unwrap();
    let previous = mesh.signed_distance_closed([-0.1; 3]).unwrap();
    seam_points[0][0] += 0.01;
    assert!(mesh.refit(&seam_points).is_err());
    assert_eq!(mesh.signed_distance_closed([-0.1; 3]).unwrap(), previous);
    let mut open = TriangleMesh::new(&points, &faces[..3]).unwrap();
    assert!(open.enable_closed_feature_normals().is_err());
    let mut reversed = faces;
    reversed[0].swap(1, 2);
    let mut inconsistent = TriangleMesh::new(&points, &reversed).unwrap();
    assert!(inconsistent.enable_closed_feature_normals().is_err());
}
#[test]
fn concave_edge_recovery_uses_metric_gradient_instead_of_sign_pseudonormal() {
    let outline = [[0., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 2.], [0., 2.]];
    let points: Vec<V> = [-1., 1.]
        .into_iter()
        .flat_map(|z| outline.map(|[x, y]| [x, y, z]))
        .collect();
    let cap = [[0, 1, 3], [1, 2, 3], [0, 3, 5], [3, 4, 5]];
    let mut faces: Vec<[usize; 3]> = cap.into_iter().map(|[a, b, c]| [c, b, a]).collect();
    faces.extend(cap.map(|[a, b, c]| [a + 6, b + 6, c + 6]));
    for a in 0..6 {
        let b = (a + 1) % 6;
        faces.extend([[a, b, b + 6], [a, b + 6, a + 6]]);
    }
    let mut mesh = TriangleMesh::new(&points, &faces).unwrap();
    mesh.enable_closed_feature_normals().unwrap();
    let point = [0.9, 0.8, 0.];
    let (signed, pseudonormal) = mesh.signed_distance_closed(point).unwrap();
    let (closest, _) = mesh.closest_surface(point).unwrap();
    assert!(len(sub(closest, [1., 1., 0.])) < 1e-14);
    assert!(signed < 0.);
    let radius = 0.02;
    let old_projection = add(point, mul(pseudonormal, radius - signed));
    assert!(
        mesh.signed_distance_closed(old_projection).unwrap().0 < 0.,
        "fixture must expose the old inward recovery"
    );
    let gradient = distance_gradient(
        sub(point, closest),
        signed.abs(),
        signed,
        pseudonormal,
        1e-12,
    );
    let projected = add(point, mul(gradient, radius - signed));
    assert!(
        mesh.signed_distance_closed(projected).unwrap().0 > 0.,
        "metric recovery must exit the solid"
    );
    assert!(len(sub(gradient, unit([0.1, 0.2, 0.]))) < 1e-14);
    let (_, witness_signed, witness_normal) =
        deepest_interval_witness(&mesh, point, point, 0., 1.).unwrap();
    assert!((witness_signed - signed).abs() < 1e-14);
    assert!(
        len(sub(witness_normal, gradient)) < 1e-14,
        "interior witness must use the metric gradient too"
    );
}
#[test]
fn feature_admission_rejects_touching_vertex_fans_and_collapsed_refits() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let outer = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
    let mut touching_points = points.to_vec();
    touching_points.extend([[0., 0., 0.], [-1., 0., 0.], [0., -1., 0.], [0., 0., -1.]]);
    let mut faces = outer.to_vec();
    faces.extend(outer.map(|[a, b, c]| [a + 4, c + 4, b + 4]));
    let mut touching = TriangleMesh::new(&touching_points, &faces).unwrap();
    assert_eq!(
        touching.enable_closed_feature_normals(),
        Err("feature normals require manifold vertex fans")
    );
    let mut mesh = TriangleMesh::new(&points, &outer).unwrap();
    mesh.enable_closed_feature_normals().unwrap();
    let before = mesh.signed_distance_closed([-0.1; 3]).unwrap();
    let mut collapsed = points;
    collapsed[1] = collapsed[0];
    assert!(mesh.refit(&collapsed).is_err());
    assert_eq!(mesh.signed_distance_closed([-0.1; 3]).unwrap(), before);
}
#[test]
fn oriented_crossings_preserve_nested_cavities() {
    let mut points = vec![[0., 0., 0.], [3., 0., 0.], [0., 3., 0.], [0., 0., 3.]];
    let outer = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
    let mut faces = outer.to_vec();
    points.extend([
        [0.3, 0.3, 0.3],
        [0.6, 0.3, 0.3],
        [0.3, 0.6, 0.3],
        [0.3, 0.3, 0.6],
    ]);
    faces.extend(outer.map(|[a, b, c]| [a + 4, c + 4, b + 4]));
    let mut mesh = TriangleMesh::new(&points, &faces).unwrap();
    mesh.enable_closed_feature_normals().unwrap();
    assert!(mesh.signed_distance_closed([0.1; 3]).unwrap().0 < 0.);
    assert!(mesh.signed_distance_closed([0.35; 3]).unwrap().0 > 0.);
    assert_eq!(
        mesh.contains_closed_surface([0.1, 0.1, 0.1]).unwrap(),
        Some(true)
    );
    assert_eq!(
        mesh.contains_closed_surface([0.35, 0.35, 0.35]).unwrap(),
        Some(false)
    );
    assert_eq!(
        mesh.contains_closed_surface([2., 2., 2.]).unwrap(),
        Some(false)
    );
}

#[test]
fn surface_vertex_is_ambiguous_and_nonfinite_queries_are_rejected() {
    let mesh = TriangleMesh::new(
        &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
    )
    .unwrap();
    assert_eq!(mesh.contains_closed_surface([0., 0., 0.]).unwrap(), None);
    assert!(mesh.contains_closed_surface([f64::NAN, 0., 0.]).is_err());
}
