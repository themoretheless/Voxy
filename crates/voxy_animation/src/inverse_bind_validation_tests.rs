use super::*;
fn rig(matrix: Mat4) -> Result<Skeleton, AnimationError> {
    Skeleton::new(vec![Joint {
        name: "root".into(),
        parent: None,
        bind_local: Transform::IDENTITY,
        inverse_bind: matrix,
    }])
}
#[test]
fn bind_preflight_rejects_double_overflow_and_preserves_wide_rigs() {
    let joints = |count: usize| {
        (0..count)
            .map(|i| Joint {
                name: format!("j{i}").into(),
                parent: (i > 0).then(|| (i - 1) as u16),
                bind_local: Transform {
                    scale: Vec3::splat(1e38),
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            })
            .collect::<Vec<_>>()
    };
    assert!(matches!(
        Skeleton::new(joints(9)),
        Err(AnimationError::InvalidJointTransform(8))
    ));
    let mut palette_overflow = joints(8);
    palette_overflow[7].inverse_bind = Mat4::from_scale(Vec3::splat(1e38));
    assert!(matches!(
        Skeleton::new(palette_overflow),
        Err(AnimationError::InvalidJointTransform(7))
    ));
    let wide = Skeleton::new(joints(2)).unwrap();
    assert!(wide.bind_pose64().skin_matrices(&wide).is_ok());
    assert!(wide.bind_pose().skin_matrices(&wide).is_err());
}
#[test]
fn underflowed_bind_chain_rejects_but_tiny_wide_palette_remains_valid() {
    let joints = |count: usize| {
        (0..count)
            .map(|i| Joint {
                name: format!("j{i}").into(),
                parent: (i > 0).then(|| (i - 1) as u16),
                bind_local: Transform {
                    scale: Vec3::splat(1e-30),
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            })
            .collect::<Vec<_>>()
    };
    let wide = Skeleton::new(joints(10)).unwrap();
    let palette = wide.bind_pose64().skin_matrices(&wide).unwrap();
    assert!(palette[9].x_axis.x > 0. && palette[9].x_axis.x < 1e-299);
    assert!(wide.bind_pose().skin_matrices(&wide).is_err());
    assert!(matches!(
        Skeleton::new(joints(11)),
        Err(AnimationError::InvalidJointTransform(10))
    ));
    // Nonzero subnormal columns are still representable; no reciprocal overflow.
    assert!(usable_linear(glam::DMat4::from_scale(glam::DVec3::splat(
        1e-320
    ))));
}
#[test]
fn dependent_integer_columns_remain_singular_after_validation() {
    let matrix = glam::DMat4::from_cols(
        glam::DVec4::new(1., 2., 3., 0.),
        glam::DVec4::new(4., 5., 6., 0.),
        glam::DVec4::new(5., 7., 9., 0.),
        glam::DVec4::W,
    );
    assert!(!usable_linear(matrix));
}
#[test]
fn computed_affine_inverse_binds_are_accepted() {
    for i in 1..=256 {
        let t = i as f32 / 37.;
        let matrix = Mat4::from_scale_rotation_translation(
            Vec3::new(0.3 + t, 1.2, -0.7),
            Quat::from_euler(glam::EulerRot::XYZ, t, t * 0.7, -t * 0.3),
            Vec3::new(t, -2. * t, 0.4),
        )
        .inverse();
        assert!(
            rig(matrix).is_ok(),
            "sample={i} homogeneous_w={}",
            matrix.w_axis.w
        );
    }
}
#[test]
fn homogeneous_scale_preserves_the_represented_affine_map() {
    let affine = Mat4::from_scale_rotation_translation(
        Vec3::new(-2., 3., 0.5),
        Quat::from_rotation_y(0.4),
        Vec3::new(1., 2., 3.),
    );
    for h in [0.5, 2., -2.] {
        let encoded = Mat4::from_cols(
            affine.x_axis * h,
            affine.y_axis * h,
            affine.z_axis * h,
            affine.w_axis * h,
        );
        let skeleton = rig(encoded).unwrap();
        assert_eq!(skeleton.joints()[0].inverse_bind, affine);
        for point in [Vec3::ZERO, Vec3::new(2., -1., 0.3)] {
            assert!(
                skeleton.joints()[0]
                    .inverse_bind
                    .transform_point3(point)
                    .abs_diff_eq(encoded.project_point3(point), 1e-6)
            );
        }
    }
    let mut hidden_perspective = Mat4::IDENTITY;
    hidden_perspective.x_axis.w = f32::MIN_POSITIVE;
    hidden_perspective.w_axis.w = f32::MAX;
    assert!(rig(hidden_perspective).is_err());
    let mut overflow = Mat4::IDENTITY;
    overflow.x_axis.x = f32::MAX;
    overflow.w_axis.w = f32::MIN_POSITIVE;
    assert!(rig(overflow).is_err());
}
#[test]
#[allow(deprecated)]
fn invalid_inverse_bind_is_rejected_before_palette_publication() {
    for matrix in [
        Mat4::ZERO,
        Mat4::from_scale(Vec3::new(1., 0., 1.)),
        Mat4::perspective_rh(1., 1., 0.1, 100.),
    ] {
        assert!(matches!(
            rig(matrix),
            Err(AnimationError::InvalidJointTransform(0))
        ));
    }
}
#[test]
fn small_large_and_reflected_affine_inverse_binds_remain_valid() {
    for scale in [
        Vec3::splat(1e-20),
        Vec3::splat(1e20),
        Vec3::new(-2., 3., 0.5),
    ] {
        let matrix = Mat4::from_scale(scale);
        let skeleton = rig(matrix).unwrap();
        assert_eq!(
            skeleton.bind_pose().skin_matrices(&skeleton).unwrap()[0],
            matrix
        );
    }
}
