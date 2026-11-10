use physics::strand::{SphereCollider, Strand, StrandConfig};
fn strand(stiffness: f64) -> Strand {
    Strand::new(
        (0..7).map(|i| [0.0, f64::from(i) * 0.15, 0.0]).collect(),
        StrandConfig {
            stiffness,
            iterations: 64,
            ..Default::default()
        },
    )
    .unwrap()
}
#[test]
fn wind_bends_grass_and_restores_after_release() {
    let mut s = strand(120.0);
    for _ in 0..480 {
        s.step(1.0 / 240.0, [0.0; 3], [15.0, -9.81, 0.0], &[])
            .unwrap();
    }
    let bent = s.positions()[6][0];
    assert!(bent > 0.04);
    for _ in 0..2400 {
        s.step(1.0 / 240.0, [0.0; 3], [0.0, -9.81, 0.0], &[])
            .unwrap();
    }
    assert!(s.positions()[6][0].abs() < bent * 0.1);
    assert_eq!(s.positions()[0], [0.0; 3]);
    for p in s.positions().windows(2) {
        let distance = p[0]
            .iter()
            .zip(p[1])
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!((distance - 0.15).abs() < 0.003);
    }
}
#[test]
fn moving_attachment_has_inertia_and_sphere_contacts_stay_outside() {
    let mut s = strand(2.0);
    s.step(1.0 / 240.0, [0.1, 0.0, 0.0], [0.0, -9.81, 0.0], &[])
        .unwrap();
    assert_eq!(s.positions()[0], [0.1, 0.0, 0.0]);
    assert!(s.positions()[6][0] < 0.05);
    let collider = SphereCollider {
        center: [0.0, 0.6, 0.0],
        radius: 0.2,
    };
    for i in 0..3000 {
        s.step(
            1.0 / 240.0,
            [0.1, 0.0, 0.0],
            [(f64::from(i) * 0.03).sin() * 5.0, -9.81, 0.0],
            &[collider],
        )
        .unwrap();
        for p in &s.positions()[1..] {
            let d = p
                .iter()
                .zip(collider.center)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(d >= 0.21 - 1e-9);
        }
    }
}
#[test]
fn invalid_step_is_atomic() {
    let mut s = strand(10.0);
    let before = s.positions().to_vec();
    assert!(s.step(f64::NAN, [0.0; 3], [0.0; 3], &[]).is_err());
    assert_eq!(before, s.positions());
    assert!(Strand::new(vec![[0.0; 3]; 2], StrandConfig::default()).is_err());
}
