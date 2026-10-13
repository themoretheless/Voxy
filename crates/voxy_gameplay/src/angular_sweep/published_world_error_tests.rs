use super::*;
#[test]
fn canonical_witness_covers_f32_publication_and_ground_relocation() {
    let center = DVec3::new(65536.009765625, 2., -3.);
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.5];
    let witness = CanonicalWorldPose {
        corners: std::array::from_fn(|i| {
            let point = center + corners(edges)[i];
            point.to_array().map(|x| [x, x])
        }),
    };
    let (_, proposal) = witness.evaluation_error(center, edges).unwrap();
    assert!(proposal < 1e-8);
    let published = center.as_vec3().as_dvec3();
    let (axes, radius) = witness.evaluation_error(published, edges).unwrap();
    assert!(axes[0] >= 1. / 512. && axes[0] < 1. / 512. + 1e-8);
    assert!(radius >= 1. / 512. && radius < 1. / 512. + 1e-8);
    println!("PUBLISHED_WORLD_ERROR {:?}", (axes, radius));
    let relocated = published + DVec3::Y * 0.125;
    let (axes, radius) = witness.evaluation_error(relocated, edges).unwrap();
    assert!(axes[1] >= 0.125 && radius >= 0.125 + 1. / 512.);
    assert!(
        witness
            .evaluation_error(DVec3::splat(f64::NAN), edges)
            .is_err()
    );
}
