use physics::surface_film::{Material, SurfaceFilm};
fn film(viscosity: f64) -> SurfaceFilm {
    SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material {
            density: 1000.0,
            viscosity,
            surface_tension: 0.0,
            wetting: 0.0,
        },
    )
    .unwrap()
}
#[test]
fn shear_smears_to_dry_cell_conserving_volume_and_viscosity_scaling() {
    let mut a = film(1.0);
    let mut b = film(2.0);
    a.deposit(0, 0.0005).unwrap();
    b.deposit(0, 0.0005).unwrap();
    let traction = [[-1.0, 0.0, 0.0]; 2];
    a.step_with_surface_shear(0.001, [0.0; 3], &traction, 0.001)
        .unwrap();
    b.step_with_surface_shear(0.001, [0.0; 3], &traction, 0.001)
        .unwrap();
    assert!(a.thickness()[1] > 0.0);
    assert!((a.total_volume() - 0.0005).abs() < 1e-18);
    assert!((a.thickness()[1] - 2.0 * b.thickness()[1]).abs() < 1e-18);
    // Shared edge length sqrt(2), conormal x projection -1/sqrt(2).
    // One step: volume crossing = dt * 1 * h²/(2*mu).
    assert!((a.thickness()[1] * 0.5 - 0.001 * 0.001_f64.powi(2) / 2.0).abs() < 1e-18);
}
#[test]
fn normal_stress_is_removed_and_zero_traction_matches_existing_flow() {
    let mut normal = film(1.0);
    normal.deposit(0, 0.0005).unwrap();
    let before = normal.thickness();
    normal
        .step_with_surface_shear(0.01, [0.0; 3], &[[0.0, 5.0, 0.0]; 2], 0.001)
        .unwrap();
    assert_eq!(before, normal.thickness());
    let mut old = film(1.0);
    old.deposit(0, 0.0005).unwrap();
    let mut zero = film(1.0);
    zero.deposit(0, 0.0005).unwrap();
    old.step(0.01, [1.0, -9.81, 0.0]).unwrap();
    zero.step_with_surface_shear(0.01, [1.0, -9.81, 0.0], &[[0.0; 3]; 2], 0.001)
        .unwrap();
    assert_eq!(old.thickness(), zero.thickness());
}
#[test]
fn excessive_flow_is_limited_and_invalid_inputs_leave_film_unchanged() {
    let mut f = film(1.0);
    f.deposit(0, 0.0005).unwrap();
    let before = f.thickness();
    assert!(
        f.step_with_surface_shear(0.01, [0.0; 3], &[[f64::NAN, 0.0, 0.0]; 2], 0.001)
            .is_err()
    );
    assert_eq!(before, f.thickness());
    f.step_with_surface_shear(0.1, [0.0; 3], &[[-1e20, 0.0, 0.0]; 2], 0.001)
        .unwrap();
    assert!(f.thickness().iter().all(|h| *h >= 0.0));
    assert!((f.total_volume() - 0.0005).abs() < 1e-18);
}
#[test]
fn temporal_refinement_approaches_integrated_shear_drainage_solution() {
    // dV/dt=-tau*V²/(2*mu*A²), shared-edge projected width=1, A=0.5.
    let exact = 0.0005 / (1.0 + 2000.0 * 0.0005 * 0.1);
    let mut errors = Vec::new();
    for step in [0.001, 0.0005, 0.00025] {
        let mut f = film(1.0);
        f.deposit(0, 0.0005).unwrap();
        f.step_with_surface_shear(0.1, [0.0; 3], &[[-1000.0, 0.0, 0.0]; 2], step)
            .unwrap();
        errors.push((f.thickness()[0] * 0.5 - exact).abs());
    }
    assert!(errors[0] / errors[1] > 1.9 && errors[1] / errors[2] > 1.9);
    assert!(errors[2] < 1e-8);
}
#[test]
fn rotating_surface_and_applied_stress_preserves_smearing() {
    let points = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ];
    // Orthogonal cyclic permutation maps the same sheet into another orientation.
    let rotate = |p: [f64; 3]| [p[2], p[0], p[1]];
    let rotated: Vec<_> = points.into_iter().map(rotate).collect();
    let material = Material {
        density: 1000.0,
        viscosity: 1.0,
        surface_tension: 0.0,
        wetting: 0.0,
    };
    let mut a = film(1.0);
    let mut b = SurfaceFilm::new(&rotated, vec![[0, 1, 2], [0, 2, 3]], material).unwrap();
    a.deposit(0, 0.0005).unwrap();
    b.deposit(0, 0.0005).unwrap();
    a.step_with_surface_shear(0.01, [0.0; 3], &[[-1.0, 0.0, 0.0]; 2], 0.001)
        .unwrap();
    b.step_with_surface_shear(0.01, [0.0; 3], &[rotate([-1.0, 0.0, 0.0]); 2], 0.001)
        .unwrap();
    for (x, y) in a.thickness().iter().zip(b.thickness()) {
        assert!((x - y).abs() < 1e-15);
    }
}
