use physics::surface_film::{Material, SurfaceFilm};
fn film() -> SurfaceFilm {
    let mut f = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material {
            surface_tension: 0.0,
            wetting: 0.0,
            ..Material::default()
        },
    )
    .unwrap();
    f.deposit(0, 0.0005).unwrap();
    f
}
#[test]
fn prescribed_layer_advection_refines_to_analytic_donor_drainage() {
    let exact = 0.0005 * (-0.2_f64).exp();
    let mut errors = Vec::new();
    for step in [0.001, 0.0005, 0.00025] {
        let mut f = film();
        f.step_with_advection(0.1, [0.0; 3], &[[-1.0, 0.0, 0.0]; 2], step)
            .unwrap();
        assert!(f.thickness()[1] > 0.0);
        assert!((f.total_volume() - 0.0005).abs() < 1e-18);
        errors.push((f.thickness()[0] * 0.5 - exact).abs());
    }
    assert!(errors[0] / errors[1] > 1.9 && errors[1] / errors[2] > 1.9);
}
#[test]
fn normal_motion_is_removed_and_invalid_advection_rolls_back() {
    let mut f = film();
    let before = f.thickness();
    f.step_with_advection(0.1, [0.0; 3], &[[0.0, 10.0, 0.0]; 2], 0.001)
        .unwrap();
    assert_eq!(f.thickness(), before);
    assert!(
        f.step_with_advection(0.1, [0.0; 3], &[[f64::NAN, 0.0, 0.0]; 2], 0.001)
            .is_err()
    );
    assert_eq!(f.thickness(), before);
    f.step_with_advection(0.1, [0.0; 3], &[[-1e20, 0.0, 0.0]; 2], 0.001)
        .unwrap();
    assert!(f.thickness().iter().all(|h| *h >= 0.0));
    assert!((f.total_volume() - 0.0005).abs() < 1e-18);
}
