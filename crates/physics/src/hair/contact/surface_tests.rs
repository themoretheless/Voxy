use super::*;

#[test]
fn interior_face_crossing_has_zero_separation_without_projected_roundoff() {
    let p = [[0.13, -0.27, 0.19], [1.17, 0.31, 0.41], [0.23, 1.29, 0.73]];
    let normal = unit(cross(sub(p[1], p[0]), sub(p[2], p[0])));
    let tri = Triangle {
        ids: [0, 1, 2],
        p,
        previous_p: p,
        velocity: [[0.; 3]; 3],
        normal,
        min: [0.; 3],
        max: [0.; 3],
    };
    let center = mul(add(add(p[0], p[1]), p[2]), 1. / 3.);
    for shift in [-1e-15, 0., 1e-15] {
        let center = add(center, [shift, 0., 0.]);
        let a = sub(center, mul(normal, 0.2));
        let b = add(center, mul(normal, 0.2));
        let (fraction, x, y) = segment_triangle(a, b, &tri);
        assert!((fraction - 0.5).abs() < 1e-12);
        assert_eq!(
            x, y,
            "a genuine interior crossing acquired a spurious positive distance"
        );
    }
    // A plane crossing outside the face must retain its real separation.
    let outside = add(p[1], sub(p[1], p[0]));
    let (_, x, y) = segment_triangle(sub(outside, normal), add(outside, normal), &tri);
    assert!(len(sub(x, y)) > 0.1);
}

#[test]
fn flat_face_contact_preserves_both_ends_of_the_supported_segment() {
    let mesh =
        TriangleMesh::new(&[[-1., 0., 0.], [1., 0., 0.], [0., 1., 0.]], &[[0, 1, 2]]).unwrap();
    for tilt in [-1e-12, 0., 1e-12] {
        let radius = 40e-6;
        let mut rod = HairRod::new(
            vec![
                [-3., 0.5, radius],
                [-2., 0.5, radius - tilt],
                [2., 0.5, radius + tilt],
            ],
            Default::default(),
        )
        .unwrap();
        let before = rod.x.clone();
        refresh_mesh_constraints(&mut rod, std::slice::from_ref(&mesh), radius);
        assert_eq!(rod.x, before, "discovery must not move the rod");
        for fraction in [0.375, 0.625] {
            assert!(
                rod.contacts
                    .iter()
                    .any(|c| c.segment == 1 && (c.fraction - fraction).abs() < 1e-12),
                "flat face must retain both support endpoints at tilt {tilt}: {:?}",
                rod.contacts
            );
        }
    }
}

#[test]
fn flat_face_velocity_response_admits_both_support_endpoints() {
    let mesh =
        TriangleMesh::new(&[[-1., 0., 0.], [1., 0., 0.], [0., 1., 0.]], &[[0, 1, 2]]).unwrap();
    let radius = 40e-6;
    let dt = 1. / 240.;
    for tilt in [-1e-12, 0., 1e-12] {
        for direction in [-1., 1.] {
            let mut rod = HairRod::new(
                vec![
                    [-3., 0.5, radius],
                    [-2., 0.5, radius - tilt],
                    [2., 0.5, radius + tilt],
                ],
                Default::default(),
            )
            .unwrap();
            refresh_mesh_constraints(&mut rod, std::slice::from_ref(&mesh), radius);
            rod.velocity[1] = [0., 0., direction];
            rod.velocity[2] = [0., 0., -direction];
            stabilize_contact_velocities(std::slice::from_mut(&mut rod), &[], dt, radius).unwrap();
            // Independently check both geometric support endpoints, including
            // the one the old single-witness discovery omitted.
            for t in [0.375, 0.625] {
                let p = add(mul(rod.x[1], 1. - t), mul(rod.x[2], t));
                let velocity = add(mul(rod.velocity[1], 1. - t), mul(rod.velocity[2], t));
                let bound = (-(p[2] - radius) / dt).min(0.);
                assert!(
                    velocity[2] >= bound - 1e-9,
                    "missing supported endpoint: tilt={tilt} t={t} velocity={velocity:?} bound={bound}"
                );
            }
        }
    }
}

#[test]
fn tessellated_flat_face_retains_support_across_both_triangles() {
    for (u, v, n) in [
        ([1., 0., 0.], [0., 1., 0.], [0., 0., 1.]),
        ([0.6, 0.8, 0.], [-0.48, 0.36, 0.8], [0.64, -0.48, 0.6]),
    ] {
        for scale in [0.01, 1., 100.] {
            let radius = 40e-6;
            let dt = 1. / 240.;
            let origin = [0.13, -0.27, 0.19];
            let point = |x, y, z| {
                add(
                    origin,
                    add(mul(u, x * scale), add(mul(v, y * scale), mul(n, z))),
                )
            };
            let points = [
                point(-1., -1., 0.),
                point(1., -1., 0.),
                point(1., 1., 0.),
                point(-1., 1., 0.),
            ];
            for faces in [[[0, 1, 2], [0, 2, 3]], [[0, 2, 3], [0, 1, 2]]] {
                let mesh = TriangleMesh::new(&points, &faces).unwrap();
                for direction in [-1., 1.] {
                    let mut rod = HairRod::new(
                        vec![
                            point(-3., 0., radius),
                            point(-2., 0., radius),
                            point(2., 0., radius),
                        ],
                        Default::default(),
                    )
                    .unwrap();
                    let before = rod.x.clone();
                    refresh_mesh_constraints(&mut rod, std::slice::from_ref(&mesh), radius);
                    assert_eq!(rod.x, before, "contact discovery changed geometry");
                    for t in [0.25, 0.75] {
                        assert!(
                            rod.contacts
                                .iter()
                                .any(|c| c.segment == 1 && (c.fraction - t).abs() < 1e-12),
                            "lost surface support at {t}, scale={scale}, normal={n:?}: {:?}",
                            rod.contacts
                        );
                    }
                    rod.velocity[1] = mul(n, direction);
                    rod.velocity[2] = mul(n, -direction);
                    stabilize_contact_velocities(std::slice::from_mut(&mut rod), &[], dt, radius)
                        .unwrap();
                    for t in [0.25, 0.75] {
                        let velocity = add(mul(rod.velocity[1], 1. - t), mul(rod.velocity[2], t));
                        let position = add(mul(rod.x[1], 1. - t), mul(rod.x[2], t));
                        let bound = (-(dot(sub(position, origin), n) - radius) / dt).min(0.);
                        assert!(
                            dot(velocity, n) >= bound - 1e-9,
                            "lost velocity support at {t}, scale={scale}: {velocity:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn tessellated_contact_preserves_affine_surface_velocity_on_each_face() {
    let dt = 1. / 240.;
    let radius = 40e-6;
    let current = [[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]];
    let mut previous = current;
    previous[3][2] = -4. * dt;
    for faces in [[[0, 1, 2], [0, 2, 3]], [[0, 2, 3], [0, 1, 2]]] {
        let mut mesh = TriangleMesh::new(&previous, &faces).unwrap();
        mesh.refit_with_timestep(&current, dt).unwrap();
        let mut rod = HairRod::new(
            vec![[-3., 0., radius], [-2., 0., radius], [2., 0., radius]],
            Default::default(),
        )
        .unwrap();
        refresh_mesh_constraints(&mut rod, std::slice::from_ref(&mesh), radius);
        stabilize_contact_velocities(std::slice::from_mut(&mut rod), &[], dt, radius).unwrap();
        for (t, surface_speed) in [(0.25, 2.), (0.5, 0.), (0.75, 0.)] {
            let velocity = add(mul(rod.velocity[1], 1. - t), mul(rod.velocity[2], t));
            assert!(
                velocity[2] >= surface_speed - 1e-9,
                "lost moving face at {t}: {velocity:?}, surface={surface_speed}"
            );
        }
    }
}

#[test]
fn parallel_strand_contact_preserves_tip_velocity_support() {
    let radius = 40e-6;
    let dt = 1. / 240.;
    for (axis, normal) in [
        ([0., 1., 0.], [1., 0., 0.]),
        ([0.6, 0.8, 0.], [0.64, -0.48, 0.6]),
    ] {
        for scale in [0.01, 1., 100.] {
            for offset in [0., 0.005] {
                for reversed in [false, true] {
                    let lengths = if reversed {
                        [0.03, 0.02, 0.01]
                    } else {
                        [0., 0.01, 0.02]
                    };
                    let points_a = [0., 0.01, 0.02].map(|y| mul(axis, y * scale));
                    let points_b = lengths
                        .map(|y| add(mul(axis, (y + offset) * scale), mul(normal, 2. * radius)));
                    let mut rods = vec![
                        HairRod::new(points_a.to_vec(), Default::default()).unwrap(),
                        HairRod::new(points_b.to_vec(), Default::default()).unwrap(),
                    ];
                    let before: Vec<_> = rods.iter().map(|r| r.x.clone()).collect();
                    let responses = refresh_strand_responses(&mut rods, radius, &[]);
                    for (rod, positions) in rods.iter().zip(before) {
                        assert_eq!(rod.x, positions);
                    }
                    rods[0].velocity[2] = normal;
                    rods[1].velocity[1] = mul(normal, -1.);
                    rods[1].velocity[2] = mul(normal, -1.);
                    stabilize_contact_velocities(&mut rods, &responses, dt, radius).unwrap();
                    // The support interval is known independently from the projected
                    // witness list, including partial overlap and reversed segments.
                    for y in [0.01 + offset, 0.02] {
                        let a = (y - 0.01) / 0.01;
                        let b = (y - offset - lengths[1]) / (lengths[2] - lengths[1]);
                        let va = add(
                            mul(rods[0].velocity[1], 1. - a),
                            mul(rods[0].velocity[2], a),
                        );
                        let vb = add(
                            mul(rods[1].velocity[1], 1. - b),
                            mul(rods[1].velocity[2], b),
                        );
                        let pa = add(mul(rods[0].x[1], 1. - a), mul(rods[0].x[2], a));
                        let pb = add(mul(rods[1].x[1], 1. - b), mul(rods[1].x[2], b));
                        let permitted = (len(sub(pa, pb)) - 2. * radius).max(0.) / dt;
                        let closing = dot(sub(va, vb), normal);
                        assert!(
                            closing <= permitted + 1e-9,
                            "parallel support approaches unchecked: closing={closing}, scale={scale}, reversed={reversed}, offset={offset}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn strand_manifold_refresh_preserves_historical_pressure_without_duplication() {
    let radius = 40e-6;
    let mut rods = vec![
        HairRod::new(
            vec![[0., 0., 0.], [0., 0.01, 0.], [0., 0.02, 0.]],
            Default::default(),
        )
        .unwrap(),
        HairRod::new(
            vec![
                [2. * radius, 0., 0.],
                [2. * radius, 0.01, 0.],
                [2. * radius, 0.02, 0.],
            ],
            Default::default(),
        )
        .unwrap(),
    ];
    let mut history = refresh_strand_responses(&mut rods, radius, &[]);
    for (i, response) in history.iter_mut().enumerate() {
        response.impulse = (i + 1) as f64 * 1e-9;
    }
    let current = refresh_strand_responses(&mut rods, radius, &history);
    assert_eq!(current.len(), history.len());
    for (old, new) in history.iter().zip(&current) {
        assert_eq!((old.a, old.b), (new.a, new.b));
        assert!(
            (old.impulse - new.impulse).abs() <= old.impulse * 1e-12,
            "history load was copied between support points: old={} new={}",
            old.impulse,
            new.impulse
        );
    }
}

#[test]
fn coincident_parallel_contacts_use_transverse_material_direction() {
    for axis in [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] {
        let rod = HairRod::new(
            [0., 0.01, 0.02].map(|t| mul(axis, t)).to_vec(),
            Default::default(),
        )
        .unwrap();
        let mut rods = vec![rod.clone(), rod];
        let contacts = refresh_strand_responses(&mut rods, 40e-6, &[]);
        assert!(!contacts.is_empty());
        for contact in contacts {
            assert!((len(contact.normal) - 1.).abs() < 1e-12);
            assert!(
                dot(contact.normal, axis).abs() < 1e-12,
                "coincident contact pushed along the fibre: axis={axis:?}, normal={:?}",
                contact.normal
            );
        }
    }
}

#[test]
fn nonzero_subpicometer_strand_separation_retains_its_geometric_normal() {
    for separation in [1e-14, 1e-200] {
        let mut rods = vec![
            HairRod::new(
                vec![[0., 0., 0.], [0.01, 0., 0.], [0.02, 0., 0.]],
                Default::default(),
            )
            .unwrap(),
            HairRod::new(
                vec![
                    [0., separation, 0.],
                    [0.01, separation, 0.],
                    [0.02, separation, 0.],
                ],
                Default::default(),
            )
            .unwrap(),
        ];
        let contacts = refresh_strand_responses(&mut rods, 40e-6, &[]);
        assert!(!contacts.is_empty());
        for contact in contacts {
            assert!(
                dot(contact.normal, [0., -1., 0.]) > 1. - 1e-12,
                "nonzero separation lost its normal: {:?}",
                contact.normal
            );
        }
    }
}

#[test]
fn coincident_material_contact_direction_follows_rigid_rotation() {
    let rod = HairRod::new(
        vec![[0., 0., 0.], [0.01, 0., 0.], [0.02, 0., 0.]],
        Default::default(),
    )
    .unwrap();
    let mut baseline = vec![rod.clone(), rod];
    let expected = refresh_strand_responses(&mut baseline, 40e-6, &[]);
    let rotation = exp([0.2, -0.4, 0.3]);
    let translation = [0.13, -0.27, 0.19];
    let mut transformed = baseline.clone();
    for rod in &mut transformed {
        for point in &mut rod.x {
            *point = add(rotate(rotation, *point), translation);
        }
        for point in &mut rod.old_x {
            *point = add(rotate(rotation, *point), translation);
        }
        for frame in &mut rod.q {
            *frame = qunit(qm(rotation, *frame));
        }
    }
    let actual = refresh_strand_responses(&mut transformed, 40e-6, &[]);
    assert_eq!(actual.len(), expected.len());
    for (a, b) in actual.iter().zip(expected) {
        assert!(
            len(sub(a.normal, rotate(rotation, b.normal))) < 1e-12,
            "contact direction did not follow the material frame"
        );
    }
}

#[test]
fn coincident_strand_contact_retains_the_previous_side_of_separation() {
    for crossing in [false, true] {
        let a = vec![[-0.02, 0., 0.], [-0.01, 0., 0.], [0.01, 0., 0.]];
        let b = if crossing {
            vec![[0., -0.02, 0.], [0., -0.01, 0.], [0., 0.01, 0.]]
        } else {
            a.clone()
        };
        let mut rods = vec![
            HairRod::new(a, Default::default()).unwrap(),
            HairRod::new(b, Default::default()).unwrap(),
        ];
        for point in &mut rods[1].old_x {
            point[2] += 1e-4;
        }
        let contacts = refresh_strand_responses(&mut rods, 40e-6, &[]);
        assert!(!contacts.is_empty());
        for contact in contacts {
            assert!(
                dot(contact.normal, [0., 0., -1.]) > 1. - 1e-12,
                "contact reversed the historical side: {:?}",
                contact.normal
            );
        }
    }
}

#[test]
fn coincident_free_segments_separate_transversely_without_axial_sliding() {
    let radius = 40e-6;
    for axis in [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] {
        let rod = HairRod::new(
            [0., 0.01, 0.02].map(|t| mul(axis, t)).to_vec(),
            Default::default(),
        )
        .unwrap();
        let mut rods = vec![rod.clone(), rod];
        let before = rods.clone();
        let mut contacts = refresh_strand_responses(&mut rods, radius, &[]);
        assert!(reconcile_contact_positions(&mut rods, &mut contacts, 1. / 240., radius).unwrap());
        for (old, rod) in before.iter().zip(&rods) {
            assert_eq!(rod.x[0], old.x[0]);
            for (a, b) in rod.x.iter().zip(&old.x) {
                assert!(
                    dot(sub(*a, *b), axis).abs() < 1e-12,
                    "contact escaped by sliding along the fibre"
                );
            }
        }
        let (_, _, a, b) = segment_pair(rods[0].x[1], rods[0].x[2], rods[1].x[1], rods[1].x[2]);
        assert!(
            len(sub(a, b)) >= 2. * radius - 1e-10,
            "free segments still overlap after normal projection"
        );
    }
}

#[test]
fn interior_crossing_contact_normal_is_orthogonal_to_both_segments() {
    let u = [0.6, 0.8, 0.];
    let v = [-0.48, 0.36, 0.8];
    let n = [0.64, -0.48, 0.6];
    let center = [0.13, -0.27, 0.19];
    for separation in [0., 1e-14] {
        let mut rods = vec![
            HairRod::new(
                [-0.04, -0.013, 0.017]
                    .map(|t| add(center, mul(u, t)))
                    .to_vec(),
                Default::default(),
            )
            .unwrap(),
            HairRod::new(
                [-0.04, -0.011, 0.019]
                    .map(|t| add(add(center, mul(v, t)), mul(n, separation)))
                    .to_vec(),
                Default::default(),
            )
            .unwrap(),
        ];
        let contacts = refresh_strand_responses(&mut rods, 40e-6, &[]);
        let interior = contacts
            .iter()
            .find(|r| {
                r.a.1 == 1 && r.b.1 == 1 && r.a.2 > 0. && r.a.2 < 1. && r.b.2 > 0. && r.b.2 < 1.
            })
            .unwrap();
        assert!(
            dot(interior.normal, u).abs() < 1e-12 && dot(interior.normal, v).abs() < 1e-12,
            "closest-point rounding created a tangential normal at separation {separation}: {:?}",
            interior.normal
        );
    }
}

#[test]
fn translated_interior_strand_contact_has_no_tangential_force() {
    let u = [0.6, 0.8, 0.];
    let v = [-0.48, 0.36, 0.8];
    let n = [0.64, -0.48, 0.6];
    for center in [[0.13, -0.27, 0.19], [1000., -2000., 3000.]] {
        let mut rods = vec![
            HairRod::new(
                [-0.04, -0.013, 0.017]
                    .map(|t| add(center, mul(u, t)))
                    .to_vec(),
                Default::default(),
            )
            .unwrap(),
            HairRod::new(
                [-0.04, -0.011, 0.019]
                    .map(|t| add(add(center, mul(v, t)), mul(n, 40e-6)))
                    .to_vec(),
                Default::default(),
            )
            .unwrap(),
        ];
        let contacts = refresh_strand_responses(&mut rods, 40e-6, &[]);
        let contact = contacts
            .iter()
            .find(|r| {
                r.a.1 == 1 && r.b.1 == 1 && r.a.2 > 0. && r.a.2 < 1. && r.b.2 > 0. && r.b.2 < 1.
            })
            .unwrap();
        for rod in &rods {
            let tangent = unit(sub(rod.x[2], rod.x[1]));
            assert!(
                dot(contact.normal, tangent).abs() < 1e-12,
                "translated closest-point subtraction introduces tangential force: center={center:?}, normal={:?}, tangent={tangent:?}",
                contact.normal
            );
        }
        assert!(
            dot(contact.normal, n) < -0.999999,
            "contact normal reversed its geometric side"
        );
    }
}

#[test]
fn translated_face_contacts_do_not_push_along_the_surface() {
    let u = [0.6, 0.8, 0.];
    let v = [-0.48, 0.36, 0.8];
    let n = [0.64, -0.48, 0.6];
    for center in [[0.13, -0.27, 0.19], [1000., -2000., 3000.]] {
        let points = [
            add(center, mul(u, -0.1)),
            add(center, mul(u, 0.1)),
            add(center, mul(v, 0.1)),
        ];
        let mesh = TriangleMesh::new(&points, &[[0, 1, 2]]).unwrap();
        let face_normal = mesh.triangles[0].normal;
        let edge = unit(sub(points[1], points[0]));
        for gap in [-20e-6, 20e-6] {
            let mut rod = HairRod::new(
                [-0.01, 0., 0.01]
                    .map(|t| add(add(add(center, mul(v, 0.03)), mul(u, t)), mul(n, gap)))
                    .to_vec(),
                Default::default(),
            )
            .unwrap();
            assert_eq!(
                closest_triangle_feature(rod.x[1], &mesh.triangles[0]).1,
                ClosestFeature::Face
            );
            let before = rod.x.clone();
            refresh_mesh_constraints(&mut rod, std::slice::from_ref(&mesh), 40e-6);
            assert_eq!(rod.x, before, "discovery changed the model");
            assert!(!rod.contacts.is_empty());
            for contact in &rod.contacts {
                assert!(
                    dot(contact.normal, edge).abs() < 1e-12,
                    "translated face query created tangential contact: center={center:?}, normal={:?}",
                    contact.normal
                );
                assert!(
                    dot(contact.normal, face_normal) > 1. - 1e-12,
                    "face recovery reversed the outward direction"
                );
            }
        }
    }
}

#[test]
fn triangle_feature_gradient_matches_face_edge_and_vertex_distance_derivatives() {
    let mesh =
        TriangleMesh::new(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], &[[0, 1, 2]]).unwrap();
    let tri = &mesh.triangles[0];
    for (point, feature, side) in [
        ([0.2, 0.2, 0.1], ClosestFeature::Face, 1.),
        ([0.2, 0.2, -0.1], ClosestFeature::Face, -1.),
        ([0.5, -0.2, 0.1], ClosestFeature::Edge(0, 1), 1.),
        ([-0.2, -0.3, 0.1], ClosestFeature::Vertex(0), 1.),
    ] {
        let (closest, actual) = closest_triangle_feature(point, tri);
        assert_eq!(actual, feature);
        let signed = side * len(sub(point, closest));
        let gradient = triangle_distance_gradient(tri, point, closest, signed, tri.normal, 1e-12);
        for axis in 0..3 {
            let dt = 1e-6;
            let mut lo = point;
            let mut hi = point;
            lo[axis] -= dt;
            hi[axis] += dt;
            let distance = |p| side * len(sub(p, closest_triangle(p, tri)));
            let derivative = (distance(hi) - distance(lo)) / (2. * dt);
            assert!(
                (gradient[axis] - derivative).abs() < 1e-9,
                "feature metric derivative changed: {feature:?}, axis={axis}"
            );
        }
    }
}
