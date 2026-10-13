use super::*;
#[test]
fn world_publication_bounds_cover_body_vertices_signed_scale_and_offset_origin() {
    let path = voxy_animation::RootRigidPath::from_twists(
        &[
            (
                voxy_animation::RootRigidTwist {
                    linear: DVec3::X * 0.5,
                    angular: DVec3::Y * 0.1,
                },
                0.25,
            ),
            (
                voxy_animation::RootRigidTwist {
                    linear: DVec3::Z * 0.25,
                    angular: -DVec3::Y * 0.2,
                },
                0.25,
            ),
        ],
        2,
    )
    .unwrap();
    let cache = path.prepare_screw_enclosures(2).unwrap();
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.0625];
    let center = DVec3::new(16777216., 1., -2.);
    let orientation = DQuat::from_xyzw(0., 1., 0., 0.);
    let basis = DQuat::from_xyzw(1., 0., 0., 0.);
    let origin = DVec3::new(0.25, 0., 0.5);
    for scale in [-2., 0.5, 1.] {
        let (axes, radius) =
            rigid_path_publication_errors(center, edges, orientation, &cache, basis, origin, scale)
                .unwrap();
        for span in path.spans() {
            for step in 0..=32 {
                let motion = span.sample(f64::from(step) / 32.).unwrap();
                for vertex in rigid_vertices(edges, basis, origin, scale).unwrap() {
                    let world = center
                        + orientation
                            * (origin
                                + scale
                                    * (basis * (motion.translation + motion.rotation * vertex)));
                    let error = world - world.as_vec3().as_dvec3();
                    for axis in 0..3 {
                        assert!(error[axis].abs() <= axes[axis]);
                    }
                    assert!(error.length() <= radius);
                }
            }
        }
        assert!(axes[0] >= 1.);
    }
}
