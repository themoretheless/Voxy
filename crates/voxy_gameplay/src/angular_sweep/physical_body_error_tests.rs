use super::*;
#[test]
fn actual_controller_center_and_edges_fit_uniform_whole_body_caps() {
    let path = voxy_animation::RootRigidPath::from_twists(
        &[
            (
                voxy_animation::RootRigidTwist {
                    linear: DVec3::new(1., 0., 0.5),
                    angular: DVec3::Y * 0.7,
                },
                0.5,
            ),
            (
                voxy_animation::RootRigidTwist {
                    linear: DVec3::new(0., 0.25, 0.),
                    angular: DVec3::X * -0.6,
                },
                0.25,
            ),
        ],
        2,
    )
    .unwrap();
    let cache = path.prepare_screw_enclosures(2).unwrap();
    let center = DVec3::new(65536.009765625, 2., -3.);
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.5];
    let basis = DQuat::from_array([0.5, 0.5, 0.5, 0.5]);
    let actor = DQuat::from_array([0.5, -0.5, 0.5, 0.5]);
    let origin = DVec3::new(0.25, -0.125, 0.5);
    let scale = -2.;
    let (axes, radius) = cache
        .physical_body_selection_error_bounds(center, edges, basis, origin, actor, scale)
        .unwrap();
    assert!(
        axes.iter()
            .all(|v| v.is_finite() && *v > 0. && *v <= radius)
    );
    assert!(radius < 1e-7);
    for span in 0..2 {
        for fraction in [0., 0.1, 0.5, 0.875, 1.] {
            let local = cache
                .sample_evaluated_with_errors(span, fraction)
                .unwrap()
                .pose();
            let rotation = reframe_rotation(basis, local.rotation);
            let displacement = rotate_vector(
                actor,
                scale * rotate_vector(basis, local.translation) + origin
                    - rotate_vector(rotation, origin),
            );
            let actual_center = center + displacement;
            let orientation = (actor * rotation).normalize();
            let actual_edges = edges.map(|edge| rotate_vector(orientation, edge));
            for corner in 0..8 {
                let actual = actual_center + corners(actual_edges)[corner];
                println!(
                    "PHYSICAL_BODY_ERROR {:?}",
                    (span, fraction, corner, actual.to_array(), axes, radius)
                );
            }
        }
    }
    assert!(
        cache
            .physical_body_selection_error_bounds(center, edges, basis, origin, actor, 0.)
            .is_err()
    );
}
