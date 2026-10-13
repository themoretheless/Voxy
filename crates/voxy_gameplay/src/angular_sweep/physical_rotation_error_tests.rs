use super::*;
#[test]
fn physical_controller_reframe_and_orientation_update_fit_uniform_rotation_caps() {
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
    let basis = DQuat::from_array([0.5, 0.5, 0.5, 0.5]);
    let orientation = DQuat::from_array([0.5, -0.5, 0.5, 0.5]);
    let caps = cache
        .physical_rotation_selection_error_bounds(basis, orientation)
        .unwrap();
    assert!(caps.iter().all(|v| v.is_finite() && *v > 0. && *v < 1e-9));
    for span in 0..2 {
        for fraction in [0., 0.1, 0.3, 0.5, 0.875, 1.] {
            let local = cache
                .sample_evaluated_with_errors(span, fraction)
                .unwrap()
                .pose();
            let actual = (orientation * reframe_rotation(basis, local.rotation)).normalize();
            println!(
                "PHYSICAL_ROTATION_ERROR {:?}",
                (span, fraction, actual.to_array(), caps)
            );
        }
    }
    assert!(
        cache
            .physical_rotation_selection_error_bounds(DQuat::from_array([f64::NAN; 4]), orientation)
            .is_err()
    );
}
