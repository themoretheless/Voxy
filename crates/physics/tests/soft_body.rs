use physics::soft_body::{SoftBody, Sphere};
fn body() -> SoftBody {
    SoftBody::new(
        vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![1.; 4],
        vec![[0, 1, 2, 3]],
    )
    .unwrap()
}
#[test]
fn contact_and_recovery() {
    let mut b = body();
    let rest = b.volume();
    let s = Sphere {
        center: [-0.05, 0., 0.],
        radius: 0.2,
    };
    for _ in 0..240 {
        b.step(1. / 240., [0.; 3], 1e-6, &[s]).unwrap();
        for p in b.positions() {
            let d: f64 = (0..3).map(|i| (p[i] - s.center[i]).powi(2)).sum();
            assert!(d >= s.radius * s.radius - 1e-10);
        }
    }
    for _ in 0..720 {
        b.step(1. / 240., [0.; 3], 1e-6, &[]).unwrap();
    }
    assert!((b.volume() / rest - 1.).abs() < 0.01);
    let p = b.positions();
    let length: f64 = (0..3).map(|i| (p[0][i] - p[1][i]).powi(2)).sum();
    assert!((length.sqrt() - 1.).abs() < 0.01);
}
#[test]
fn invalid_step_is_atomic() {
    let mut b = body();
    let old = b.positions().to_vec();
    assert!(b.step(f64::NAN, [0.; 3], 0., &[]).is_err());
    assert_eq!(old, b.positions());
}
#[test]
fn pinned_vertex_and_free_fall() {
    let mut b = body();
    b.step(1. / 240., [0., -9.81, 0.], 0., &[]).unwrap();
    assert!((b.positions()[0][1] + 9.81 / 240_f64.powi(2)).abs() < 1e-10);
    let mut pinned = SoftBody::new(
        b.positions().to_vec(),
        vec![0., 1., 1., 1.],
        vec![[0, 1, 2, 3]],
    )
    .unwrap();
    let old = pinned.positions()[0];
    for _ in 0..240 {
        pinned.step(1. / 240., [0., -9.81, 0.], 1e-6, &[]).unwrap();
    }
    assert_eq!(old, pinned.positions()[0]);
}
