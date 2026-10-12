use super::*;
#[test]
fn pair_clearance_is_not_two_independent_target_tolerances() {
    let radius = 40e-6;
    let half = (2. * radius - 1.45e-10) * 0.5;
    let mut rods = vec![
        HairRod::new(
            vec![[-0.001, 0., 0.], [-half, 0.01, 0.], [-half, 0.02, 0.]],
            super::super::HairMaterial::default(),
        )
        .unwrap(),
        HairRod::new(
            vec![[0.001, 0., 0.], [half, 0.01, 0.], [half, 0.02, 0.]],
            super::super::HairMaterial::default(),
        )
        .unwrap(),
    ];
    let roots = [rods[0].x[0], rods[1].x[0]];
    let pairs = refresh_strand_responses(&mut rods, radius, &[]);
    assert!(!pairs.is_empty());
    for rod in &rods {
        for contact in &rod.contacts {
            let p = add(
                mul(rod.x[contact.segment], 1. - contact.fraction),
                mul(rod.x[contact.segment + 1], contact.fraction),
            );
            assert!(dot(sub(p, contact.target), contact.normal) >= -1e-10);
        }
    }
    assert!(!strand_geometry_admitted(&rods, &pairs, radius).unwrap());
    super::super::HairSystem::reconcile_positions(
        &mut rods,
        &[],
        1. / 240.,
        radius,
        true,
        &mut Vec::new(),
        None,
    )
    .unwrap();
    let pairs = refresh_strand_responses(&mut rods, radius, &[]);
    assert!(strand_geometry_admitted(&rods, &pairs, radius).unwrap());
    assert_eq!([rods[0].x[0], rods[1].x[0]], roots);
}
#[test]
fn animated_surface_velocity_tracks_material_points_and_invalid_refits_are_atomic() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    let moved = [[0., 0., 0.], [1., 0., 1.], [0., 1., 2.]];
    let mut mesh = TriangleMesh::new(&points, &[[0, 1, 2]]).unwrap();
    mesh.refit_with_timestep(&moved, 0.5).unwrap();
    for (point, expected) in [([0.2, 0.3, 0.8], 1.6), ([0.5, 0., 0.5], 1.), (moved[1], 2.)] {
        assert!(len(sub(mesh.surface_velocity(0, point), [0., 0., expected])) < 1e-12);
    }
    let old = mesh.surface_velocity(0, [0.2, 0.3, 0.8]);
    assert!(mesh.refit_with_timestep(&points, 0.).is_err());
    assert_eq!(mesh.surface_velocity(0, [0.2, 0.3, 0.8]), old);
    assert_eq!(mesh.triangles[0].p, moved);
    mesh.refit(&moved).unwrap();
    assert_eq!(
        mesh.surface_velocity(0, [0.2, 0.3, 0.8]),
        [0.; 3],
        "untimed refit reused stale motion"
    );
}
#[test]
fn sampled_motion_uses_full_interval_velocity_and_original_endpoints() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    let mut source = TriangleMesh::new(&points, &[[0, 1, 2]]).unwrap();
    source
        .refit_with_timestep(&points.map(|p| add(p, [0., 0., 2.])), 0.5)
        .unwrap();
    let mut sample = source.clone();
    for fraction in [0.25, 0.75, 1.] {
        sample.sample_motion(&source, fraction).unwrap();
        assert_eq!(sample.triangles[0].p[0], [0., 0., 2. * fraction]);
        assert_eq!(
            sample.surface_velocity(0, [0.2, 0.3, 2. * fraction]),
            [0., 0., 4.]
        );
    }
    assert_eq!(source.triangles[0].p[0], [0., 0., 2.]);
    assert_eq!(source.triangles[0].previous_p[0], points[0]);
}
#[test]
fn interval_motion_aligns_previous_vertices_duration_and_velocity() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    let mut source = TriangleMesh::new(&points, &[[0, 1, 2]]).unwrap();
    source
        .refit_with_timestep(&points.map(|p| add(p, [0., 0., 2.])), 0.5)
        .unwrap();
    let before = format!("{source:?}");
    let a = source.motion_interval(0.25, 0.5).unwrap();
    let b = source.motion_interval(0.5, 0.75).unwrap();
    assert_eq!(a.triangles[0].previous_p[0], [0., 0., 0.5]);
    assert_eq!(a.triangles[0].p[0], [0., 0., 1.]);
    assert_eq!(a.motion_duration(), Some(0.125));
    assert_eq!(a.triangles[0].p, b.triangles[0].previous_p);
    assert_eq!(a.surface_velocity(0, [0.2, 0.3, 1.]), [0., 0., 4.]);
    for (lo, hi) in [
        (0.5, 0.5),
        (0.75, 0.25),
        (-0.1, 0.5),
        (0., 1.1),
        (f64::NAN, 1.),
    ] {
        assert!(source.motion_interval(lo, hi).is_err());
    }
    assert_eq!(format!("{source:?}"), before);
}
#[test]
fn untimed_refit_is_stationary_in_continuous_queries() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    let mut source = TriangleMesh::new(&points, &[[0, 1, 2]]).unwrap();
    source.refit(&points.map(|p| add(p, [0., 0., 2.]))).unwrap();
    let sample = source.motion_interval(0., 1.).unwrap();
    assert_eq!(sample.triangles[0].previous_p, sample.triangles[0].p);
    assert_eq!(sample.motion_duration(), None);
}
#[test]
fn collapsed_intermediate_motion_is_rejected_atomically() {
    let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    let mut source = TriangleMesh::new(&points, &[[0, 1, 2]]).unwrap();
    source
        .refit_with_timestep(&[[0., 0., 0.], [-1., 0., 0.], [0., -1., 0.]], 0.5)
        .unwrap();
    let mut sample = source.clone();
    assert!(sample.sample_motion(&source, 0.5).is_err());
    assert_eq!(sample.triangles[0].p, source.triangles[0].p);
    assert_eq!(sample.triangles[0].velocity, source.triangles[0].velocity);
}
#[test]
fn moving_surface_response_preserves_common_translation() {
    let mut rod = HairRod::new(
        vec![[0., 0., 0.], [0., 0.01, 0.], [0., 0.02, 0.]],
        super::super::HairMaterial::default(),
    )
    .unwrap();
    let dt = 1. / 240.;
    let velocity = [5., -2., 3.];
    for (old, current) in rod.old_x.iter_mut().zip(&rod.x) {
        *old = sub(*current, mul(velocity, dt));
    }
    rod.record_surface_response(1, [0., 1e-4, 0.], velocity);
    rod.finish(dt);
    assert!(
        len(sub(rod.velocity[1], velocity)) < 1e-12,
        "common surface motion generated friction: {:?}",
        rod.velocity[1]
    );
}
#[test]
fn released_strand_contacts_do_not_apply_historical_velocity_impulses() {
    let make = |x| {
        HairRod::new(
            vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
            super::super::HairMaterial::default(),
        )
        .unwrap()
    };
    let mut rods = vec![make(0.), make(60e-6)];
    let history = self_contacts(&mut rods, 40e-6);
    assert!(!history.is_empty());
    for point in &mut rods[1].x {
        point[0] += 0.01;
    }
    rods[0].velocity.fill([1., 2., 0.]);
    rods[1].velocity.fill([-1., 0., 0.]);
    let before = rods.clone();
    let current = refresh_strand_responses(&mut rods, 40e-6, &history);
    assert!(current.is_empty());
    finish_strand_contacts(&mut rods, &current, 1. / 240.);
    for (actual, expected) in rods.iter().zip(&before) {
        assert_eq!(actual.x, expected.x);
        assert_eq!(actual.velocity, expected.velocity);
        assert!(
            actual
                .contacts
                .iter()
                .all(|c| matches!(c.source, ContactSource::Mesh(_)))
        );
    }
}
#[test]
fn refreshed_strand_contacts_use_current_geometry_without_projecting() {
    let make = |x| {
        HairRod::new(
            vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
            super::super::HairMaterial::default(),
        )
        .unwrap()
    };
    let mut rods = vec![make(0.), make(80e-6)];
    let history = [StrandResponse {
        a: (0, 1, 0.5),
        b: (1, 1, 0.5),
        normal: [0., 1., 0.],
        impulse: 1e-6,
    }];
    let before = rods.clone();
    let current = refresh_strand_responses(&mut rods, 40e-6, &history);
    assert!(!current.is_empty());
    for response in current {
        assert!(response.normal[0] < -0.99);
        assert_eq!(
            response.impulse, 0.,
            "orthogonal historical load must not become friction"
        );
    }
    for (actual, expected) in rods.iter().zip(before) {
        assert_eq!(actual.x, expected.x);
    }
}
#[test]
fn current_strand_manifold_transfers_projection_load_once_per_support() {
    let make = |x| {
        HairRod::new(
            vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
            super::super::HairMaterial::default(),
        )
        .unwrap()
    };
    let mut rods = vec![make(0.), make(80e-6)];
    let history = [
        StrandResponse {
            a: (0, 1, 0.2),
            b: (1, 1, 0.2),
            normal: [-1., 0., 0.],
            impulse: 1e-6,
        },
        StrandResponse {
            a: (0, 1, 0.8),
            b: (1, 1, 0.8),
            normal: [-1., 0., 0.],
            impulse: 2e-6,
        },
    ];
    let current = refresh_strand_responses(&mut rods, 40e-6, &history);
    let pair = current
        .iter()
        .filter(|r| r.a.1 == 1 && r.b.1 == 1)
        .collect::<Vec<_>>();
    assert_eq!(
        pair.len(),
        2,
        "parallel finite segments need both support endpoints"
    );
    for (fraction, pressure) in [(0., 1e-6), (1., 2e-6)] {
        let support = pair
            .iter()
            .find(|r| r.a.2 == fraction && r.b.2 == fraction)
            .unwrap();
        assert!((support.impulse - pressure).abs() < 1e-20);
        assert!(len(sub(support.normal, [-1., 0., 0.])) < 1e-15);
    }
    assert!((pair.iter().map(|r| r.impulse).sum::<f64>() - 3e-6).abs() < 1e-20);
}
#[test]
fn nonlinear_joint_contact_requeries_pairs_created_by_a_solved_increment() {
    let make = |x| {
        HairRod::new(
            vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
            super::super::HairMaterial::default(),
        )
        .unwrap()
    };
    let initial = vec![make(0.), make(20e-6), make(120e-6)];
    let radius = 40e-6;
    let dt = 1. / 240.;
    let penetration = |rods: &[HairRod], responses: &[StrandResponse]| {
        responses
            .iter()
            .map(|r| {
                let (a, i, s) = r.a;
                let (b, j, t) = r.b;
                let pa = add(mul(rods[a].x[i], 1. - s), mul(rods[a].x[i + 1], s));
                let pb = add(mul(rods[b].x[j], 1. - t), mul(rods[b].x[j + 1], t));
                2. * radius - dot(sub(pa, pb), r.normal)
            })
            .fold(0., f64::max)
    };
    let mut once = initial.clone();
    let mut contacts = refresh_strand_responses(&mut once, radius, &[]);
    assert!(reconcile_contact_positions(&mut once, &mut contacts, dt, radius).unwrap());
    let newly_detected = refresh_strand_responses(&mut once, radius, &[]);
    assert!(
        penetration(&once, &newly_detected) > 1e-9,
        "fixture must expose a new contact after a solved tangent step"
    );
    let mut actual = initial.clone();
    let mut history = Vec::new();
    super::super::HairSystem::reconcile_positions(
        &mut actual,
        &[],
        dt,
        radius,
        true,
        &mut history,
        None,
    )
    .unwrap();
    let current = refresh_strand_responses(&mut actual, radius, &[]);
    assert!(penetration(&actual, &current) <= 1e-10);
    for (after, before) in actual.iter().zip(initial) {
        assert_eq!(after.x[0], before.x[0]);
        assert_eq!(after.q[0], before.q[0]);
        assert_eq!(after.velocity, before.velocity);
    }
}
#[test]
fn strand_velocity_response_preserves_common_translation() {
    let make = |x| {
        HairRod::new(
            vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
            super::super::HairMaterial::default(),
        )
        .unwrap()
    };
    let mut rods = vec![make(0.), make(80e-6)];
    for rod in &mut rods {
        rod.velocity.fill([5., -2., 3.]);
    }
    let response = StrandResponse {
        a: (0, 1, 0.5),
        b: (1, 1, 0.5),
        normal: [1., 0., 0.],
        impulse: 1e-8,
    };
    finish_strand_contacts(&mut rods, &[response], 1. / 240.);
    assert!(
        rods.iter()
            .all(|rod| rod.velocity.iter().all(|v| *v == [5., -2., 3.]))
    );
}
#[test]
fn strand_velocity_response_conserves_momentum_dissipates_energy_and_is_galilean_invariant() {
    let make = |length| {
        HairRod::new(
            vec![[0., 0., 0.], [0., length, 0.], [0., 2. * length, 0.]],
            super::super::HairMaterial::default(),
        )
        .unwrap()
    };
    let mut rods = vec![make(0.01), make(0.02)];
    rods[0].velocity.fill([-1., 2., 0.]);
    rods[1].velocity.fill([1., 0., 0.]);
    let before = rods.clone();
    let mut boosted = rods.clone();
    let boost = [7., -3., 2.];
    for rod in &mut boosted {
        for v in &mut rod.velocity {
            *v = add(*v, boost);
        }
    }
    let response = StrandResponse {
        a: (0, 1, 0.5),
        b: (1, 1, 0.5),
        normal: [1., 0., 0.],
        impulse: 0.,
    };
    finish_strand_contacts(&mut rods, std::slice::from_ref(&response), 1. / 240.);
    finish_strand_contacts(&mut boosted, &[response], 1. / 240.);
    let mut momentum = [0.; 3];
    let mut before_energy = 0.;
    let mut after_energy = 0.;
    let mut scale = 0.;
    for ((old, rod), shifted) in before.iter().zip(&rods).zip(&boosted) {
        for i in 1..rod.x.len() {
            let mass = 1. / rod.inv_mass[i];
            let delta = mul(sub(rod.velocity[i], old.velocity[i]), mass);
            momentum = add(momentum, delta);
            scale += len(delta);
            before_energy += 0.5 * mass * dot(old.velocity[i], old.velocity[i]);
            after_energy += 0.5 * mass * dot(rod.velocity[i], rod.velocity[i]);
            assert!(len(sub(sub(shifted.velocity[i], rod.velocity[i]), boost)) < 1e-12);
        }
    }
    assert!(len(momentum) < scale * 1e-12);
    assert!(after_energy < before_energy);
}
#[test]
fn limited_strand_projection_preserves_unresolved_penetration_in_constraints() {
    let material = super::super::HairMaterial::default();
    let short = HairRod::new(
        vec![[0., -0.002, 0.], [0., -0.001, 0.], [0., 0., 0.]],
        material,
    )
    .unwrap();
    let other = HairRod::new(
        vec![[0., -0.002, 0.005], [0., -0.001, 0.005], [0., 0., 0.005]],
        material,
    )
    .unwrap();
    let mut rods = vec![short, other];
    self_contacts(&mut rods, 0.01);
    assert!(
        rods[0].contacts.iter().any(|contact| {
            if !matches!(
                contact.source,
                ContactSource::Strand {
                    other_rod: 1,
                    other_segment: 1
                }
            ) {
                return false;
            }
            let position = add(
                mul(rods[0].x[contact.segment], 1. - contact.fraction),
                mul(rods[0].x[contact.segment + 1], contact.fraction),
            );
            dot(sub(position, contact.target), contact.normal) < -1e-4
        }),
        "limited projection erased its remaining penetration: {:?}",
        rods[0].contacts
    );
}
#[test]
fn capped_pair_projection_preserves_free_particle_momentum() {
    let make = |length| {
        HairRod::new(
            vec![[0., 0., 0.], [0., length, 0.], [0., 2. * length, 0.]],
            super::super::HairMaterial::default(),
        )
        .unwrap()
    };
    let mut rods = vec![make(0.001), make(0.1)];
    let before: Vec<_> = rods.iter().map(|rod| rod.x.clone()).collect();
    let fraction = 0.5;
    let impulse = pair_impulse(&rods, (0, 1, fraction), (1, 1, fraction), 10.);
    for (index, sign) in [(0, 1.), (1, -1.)] {
        let weight = contact_weight(&rods[index], 1, fraction);
        assert!(project(
            &mut rods[index],
            1,
            fraction,
            [sign, 0., 0.],
            impulse * weight,
            None
        ));
    }
    let mut momentum = [0.; 3];
    let mut magnitude = 0.;
    for (rod, positions) in rods.iter().zip(before) {
        for i in 1..rod.x.len() {
            let change = mul(sub(rod.x[i], positions[i]), 1. / rod.inv_mass[i]);
            momentum = add(momentum, change);
            magnitude += len(change);
            assert!(len(sub(rod.x[i], positions[i])) <= 0.2 * rod.lengths[1] * (1. + 1e-12));
        }
    }
    assert!(
        len(momentum) <= magnitude * 1e-12,
        "capped contact created net momentum: {momentum:?}"
    );
    assert!(magnitude > 0.);
}
#[test]
fn near_parallel_segments_preserve_the_interior_minimum_across_scales() {
    for scale in [1e-4, 1., 1e4] {
        let a = [0.; 3];
        let b = [scale, 0., 0.];
        let c = [0., -scale * 5e-11, scale * 1e-4];
        let d = [scale, scale * 5e-11, scale * 1e-4];
        for (a, b, c, d) in [(a, b, c, d), (c, d, a, b), (b, a, c, d), (a, b, d, c)] {
            let (s, t, p, q) = segment_pair(a, b, c, d);
            assert!(
                (s - 0.5).abs() < 1e-12 && (t - 0.5).abs() < 1e-12,
                "lost interior minimum: {s}, {t}"
            );
            assert!((len(sub(p, q)) / scale - 1e-4).abs() < 1e-15);
        }
    }
}
#[test]
fn point_segments_project_onto_the_other_segment_in_both_orders() {
    let a = [0., 0., 0.];
    let b = [1., 0., 0.];
    let p = [0.5, 0.25, 0.];
    assert_eq!(segment_pair(a, b, p, p), (0.5, 0., [0.5, 0., 0.], p));
    assert_eq!(segment_pair(p, p, a, b), (0., 0.5, p, [0.5, 0., 0.]));
    assert_eq!(segment_pair(p, p, a, a), (0., 0., p, a));
}
#[test]
fn separated_strands_retain_boundary_planes_without_projection_or_adhesion() {
    let radius = 40e-6;
    let separation = 2. * radius + 5e-11;
    let make = |x| {
        HairRod::new(
            vec![[x, 0., 0.], [x, 0., 0.01], [x, 0., 0.02]],
            super::super::HairMaterial::default(),
        )
        .unwrap()
    };
    let mut rods = vec![make(0.), make(separation)];
    let before: Vec<_> = rods.iter().map(|rod| rod.x.clone()).collect();
    self_contacts(&mut rods, radius);
    for (rod, positions) in rods.iter().zip(before) {
        assert_eq!(rod.x, positions, "separated strands received a projection");
        assert!(
            rod.normals.iter().all(|normal| *normal == [0.; 3]),
            "separated strands received friction impulses"
        );
        assert!(!rod.contacts.is_empty(), "boundary contact was discarded");
        for contact in &rod.contacts {
            let position = add(
                mul(rod.x[contact.segment], 1. - contact.fraction),
                mul(rod.x[contact.segment + 1], contact.fraction),
            );
            assert!(
                dot(sub(position, contact.target), contact.normal) > 0.,
                "separated plane became penetrating"
            );
        }
    }
}
#[test]
fn nearby_geometric_planes_do_not_replace_distinct_contacts() {
    let mut rod = HairRod::new(
        vec![[0., 0., 0.], [0., 0., 0.01], [0., 0., 0.02]],
        super::super::HairMaterial::default(),
    )
    .unwrap();
    let normal_a = [1., 0., 0.];
    let normal_b = [(1f64 - 1e-14).sqrt(), 1e-7, 0.];
    let target = [0., 0., 0.01];
    rod.record_contact(1, 0., normal_a, target, ContactSource::Mesh(0));
    let original = rod.contacts[0].clone();
    rod.record_contact(1, 0., normal_b, target, ContactSource::Mesh(0));
    let probe = [-5e-10, 0.01, 0.01];
    assert!(original.physical_gap(probe) < -1e-10);
    assert!(
        rod.contacts
            .iter()
            .any(|row| row.physical_gap(probe) < -1e-10),
        "nearby contact replaced a plane that excludes the probe"
    );
    assert_eq!(rod.contacts.len(), 2);
    // A tangentially shifted witness describes exactly the same plane.
    rod.record_contact(1, 0., normal_a, [0., 0.02, 0.01], ContactSource::Mesh(0));
    assert_eq!(rod.contacts.len(), 2, "exact plane alias duplicated");
    // A scaled envelope witness is a distinct physical Jacobian.
    rod.contacts[0].metric_scale = 0.25;
    rod.record_contact(1, 0., normal_a, target, ContactSource::Mesh(0));
    assert_eq!(rod.contacts.len(), 3, "scaled witness was overwritten");
}
#[test]
fn shared_endpoint_contact_is_unique_and_collision_owners_expire_separately() {
    let mut rod = HairRod::new(
        vec![[0., 0., 0.], [0., 0., 1.], [0., 0., 2.]],
        super::super::HairMaterial::default(),
    )
    .unwrap();
    rod.record_point_contact(1, [1., 0., 0.], [0., 0., 1.], ContactSource::Mesh(0));
    rod.record_contact(0, 1., [1., 0., 0.], [0., 0., 1.], ContactSource::Mesh(0));
    assert_eq!(
        rod.contacts.len(),
        1,
        "node and capsule counted the shared endpoint twice"
    );
    rod.record_contact(
        1,
        0.5,
        [0., 1., 0.],
        [0., 0., 1.5],
        ContactSource::Strand {
            other_rod: 1,
            other_segment: 0,
        },
    );
    mesh_contacts(&mut rod, &[], 40e-6);
    assert_eq!(rod.contacts.len(), 1);
    assert!(matches!(
        rod.contacts[0].source,
        ContactSource::Strand { .. }
    ));
    self_contacts(std::slice::from_mut(&mut rod), 40e-6);
    assert!(
        rod.contacts.is_empty(),
        "obsolete strand constraints survived their owner refresh"
    );
}
#[test]
fn closest_point_regions_preserve_face_edge_and_vertex_identity() {
    let mesh =
        TriangleMesh::new(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], &[[0, 1, 2]]).unwrap();
    let tri = &mesh.triangles[0];
    let cases = [
        ([-1., -1., 0.2], [0., 0., 0.], ClosestFeature::Vertex(0)),
        ([2., -1., 0.2], [1., 0., 0.], ClosestFeature::Vertex(1)),
        ([-1., 2., 0.2], [0., 1., 0.], ClosestFeature::Vertex(2)),
        ([0.5, -1., 0.2], [0.5, 0., 0.], ClosestFeature::Edge(0, 1)),
        ([-1., 0.5, 0.2], [0., 0.5, 0.], ClosestFeature::Edge(0, 2)),
        ([1., 1., 0.2], [0.5, 0.5, 0.], ClosestFeature::Edge(1, 2)),
        ([0.25, 0.25, 0.2], [0.25, 0.25, 0.], ClosestFeature::Face),
    ];
    for (query, point, feature) in cases {
        let actual = closest_triangle_feature(query, tri);
        assert_eq!(actual, (point, feature));
        assert_eq!(closest_triangle(query, tri), point);
    }
}
#[test]
fn accumulated_contact_direction_matches_applied_weighted_corrections() {
    let mut rod = HairRod::new(
        vec![[0., 0., 0.], [0., 0., 1.], [0., 0., 2.]],
        super::super::HairMaterial::default(),
    )
    .unwrap();
    let initial = rod.x.clone();
    project(&mut rod, 0, 0.2, [1., 0., 0.], 0.1, Some([0.; 3]));
    project(&mut rod, 1, 0.8, [0., 1., 0.], 0.8, Some([0.; 3]));
    for i in 0..rod.x.len() {
        let applied = sub(rod.x[i], initial[i]);
        assert!(
            len(sub(applied, rod.normals[i])) < 1e-14,
            "point {i}: {:?} != {:?}",
            applied,
            rod.normals[i]
        );
    }
    assert_eq!(rod.normals[0], [0.; 3]);
    assert!(rod.x[1][0] > 0.19 && rod.x[1][1] > 0.);
}
#[test]
fn allowed_follicle_contact_does_not_hide_a_later_surface_crossing() {
    let mesh = TriangleMesh::new(
        &[
            [-2., -2., 0.],
            [2., -2., 0.],
            [0., 2., 0.],
            [-2., -2., 0.5],
            [2., -2., 0.5],
            [0., 2., 0.5],
        ],
        &[[0, 1, 2], [3, 4, 5]],
    )
    .unwrap();
    let mut rod = HairRod::new(
        vec![[0., 0., 0.], [0., 0., 1.], [0., 0., 2.]],
        super::super::HairMaterial::default(),
    )
    .unwrap();
    mesh_contacts(&mut rod, std::slice::from_ref(&mesh), 40e-6);
    assert_eq!(rod.x[0], [0.; 3]);
    assert!(
        (rod.x[1][2] - (1. + 80e-6)).abs() < 1e-12,
        "later contact was hidden: {:?}",
        rod.x
    );
}
#[test]
fn multiple_intersections_choose_first_and_shared_features_use_topology() {
    assert!(segment_contact_precedes(
        5e-16,
        0.2,
        [8, 9, 10],
        0.,
        0.8,
        [0, 1, 2],
        1.
    ));
    assert!(!segment_contact_precedes(
        0.,
        0.8,
        [0, 1, 2],
        5e-16,
        0.2,
        [8, 9, 10],
        1.
    ));
    assert!(segment_contact_precedes(
        1e-16,
        0.2 + 1e-12,
        [0, 1, 2],
        0.,
        0.2,
        [8, 9, 10],
        1.
    ));
    assert!(!segment_contact_precedes(
        0.,
        0.2,
        [8, 9, 10],
        1e-16,
        0.2 + 1e-12,
        [0, 1, 2],
        1.
    ));
    assert!(segment_contact_precedes(
        1e-5,
        0.8,
        [8, 9, 10],
        2e-5,
        0.2,
        [0, 1, 2],
        1.
    ));
}
#[test]
fn clearance_skips_match_full_queries_when_particles_cross_surface() {
    let mesh = TriangleMesh::new(
        &[[-1., 0., -1.], [1., 0., -1.], [1., 0., 1.], [-1., 0., 1.]],
        &[[0, 2, 1], [0, 3, 2]],
    )
    .unwrap();
    let mut cached = HairRod::new(
        vec![[0., 0.05, 0.], [0.01, 0.04, 0.], [0.02, 0.03, 0.]],
        super::super::HairMaterial::default(),
    )
    .unwrap();
    let mut full = cached.clone();
    for i in 0..100 {
        let y = (i as f64 * 0.17).sin() * 0.04;
        for rod in [&mut cached, &mut full] {
            rod.old_x.clone_from(&rod.x);
            rod.x[1][1] = y;
            rod.x[2][1] = y + 0.005;
        }
        // Independent reference starts with fresh query storage on every call.
        full.contact_candidates = Vec::new();
        for bounds in &mut full.clearance_cache {
            bounds.fill(None);
        }
        mesh_contacts(&mut cached, std::slice::from_ref(&mesh), 0.001);
        mesh_contacts(&mut full, std::slice::from_ref(&mesh), 0.001);
        assert_eq!(cached.x, full.x);
        assert_eq!(cached.normals, full.normals);
    }
}
#[test]
fn warm_start_remains_exact_after_surface_refit_and_far_motion() {
    let vertices: Vec<V> = (0..=8)
        .flat_map(|y| (0..=8).map(move |x| [x as f64 * 0.1, y as f64 * 0.1, 0.]))
        .collect();
    let indices: Vec<[usize; 3]> = (0..8)
        .flat_map(|y| {
            (0..8).flat_map(move |x| {
                let a = y * 9 + x;
                [[a, a + 1, a + 9], [a + 1, a + 10, a + 9]]
            })
        })
        .collect();
    let mut mesh = TriangleMesh::new(&vertices, &indices).unwrap();
    let moved: Vec<V> = vertices
        .iter()
        .map(|p| [p[0] + 1., p[1] - 2., p[2] + 0.3 * p[0]])
        .collect();
    mesh.refit(&moved).unwrap();
    let mut cached = 0;
    for sample in 0..100 {
        let p = [
            (sample * 37 % 101) as f64 * 0.03 - 0.5,
            (sample * 19 % 101) as f64 * 0.04 - 3.,
            0.5,
        ];
        let q = closest_triangle(p, &mesh.triangles[cached]);
        let delta = sub(p, q);
        let mut best = (dot(delta, delta), cached, q);
        mesh.nearest(p, 0, &mut best);
        let brute = mesh
            .triangles
            .iter()
            .map(|t| {
                let q = closest_triangle(p, t);
                let d = sub(p, q);
                dot(d, d)
            })
            .fold(f64::INFINITY, f64::min);
        assert!((best.0 - brute).abs() < 1e-12);
        cached = best.1;
    }
}
#[test]
fn capsule_query_detects_interior_contact_and_rejects_distant_plane_extension() {
    let mesh =
        TriangleMesh::new(&[[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]], &[[0, 1, 2]]).unwrap();
    let (_, correction) = mesh
        .capsule_contact([-0.5, 0., 0.05], [0.5, 0., 0.05], 0.1, [0., 0., 1.])
        .unwrap()
        .unwrap();
    assert!((correction[2] - 0.05).abs() < 1e-12);
    assert!(
        mesh.capsule_contact([3., 0., 0.05], [4., 0., 0.05], 0.1, [0., 0., 1.])
            .unwrap()
            .is_none()
    );
    let (t, correction) = mesh
        .capsule_contact([0., 0., -0.2], [0., 0., 0.2], 0.1, [0., 0., 1.])
        .unwrap()
        .unwrap();
    assert!((t - 0.5).abs() < 1e-12 && (correction[2] - 0.1).abs() < 1e-12);
}
#[test]
fn leaf_query_contains_exactly_overlapping_triangle_bounds() {
    let vertices = [
        [0., 0., 0.],
        [1., 0., 0.],
        [0., 1., 0.],
        [4., 4., 0.],
        [5., 4., 0.],
        [4., 5., 0.],
    ];
    let mesh = TriangleMesh::new(&vertices, &[[0, 1, 2], [3, 4, 5]]).unwrap();
    let mut result = Vec::new();
    mesh.query([-0.1, -0.1, -0.1], [1.1, 1.1, 0.1], 0, &mut result);
    assert_eq!(result.len(), 1);
    assert_eq!(mesh.triangles[result[0]].ids, [0, 1, 2]);
}
