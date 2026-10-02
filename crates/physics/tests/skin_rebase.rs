use physics::skin::{Skin, SkinMaterial};
fn patch() -> Skin {
    Skin::new(
        vec![
            [0., 0., 0.],
            [0.01, 0., 0.],
            [0.01, 0.01, 0.],
            [0., 0.01, 0.],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        &[0],
        SkinMaterial::default(),
        vec![[1., 0., 0.]; 2],
    )
    .unwrap()
}
#[test]
fn rest_rebuild_scales_mass_without_introducing_prestrain() {
    let skin = patch();
    let mass: f64 = skin.masses().iter().sum();
    let next = skin
        .rebased(
            skin.rest_positions()
                .iter()
                .map(|p| p.map(|v| 2. * v))
                .collect(),
        )
        .unwrap();
    assert!((next.masses().iter().sum::<f64>() - 4. * mass).abs() < 1e-12);
    assert!(next.stored_energy().unwrap().abs() < 1e-12);
    assert!(next.velocities().iter().flatten().all(|v| *v == 0.));
    assert!(skin.rebased(vec![[0.; 3]; 4]).is_err());
    assert_eq!(skin.rest_positions()[1], [0.01, 0., 0.]);
}
#[test]
fn heterogeneous_properties_change_mass_energy_and_survive_rebase() {
    let mut a = patch();
    let mut b = a.clone();
    a.set_face_properties(&[2., 2.], &[3., 3.]).unwrap();
    let mass: f64 = b.masses().iter().sum();
    assert!((a.masses().iter().sum::<f64>() - 3. * mass).abs() < 1e-12);
    let pose: Vec<_> = a
        .rest_positions()
        .iter()
        .map(|p| [p[0] * 1.1, p[1], p[2]])
        .collect();
    a.set_state(pose.clone(), vec![[0.; 3]; 4]).unwrap();
    b.set_state(pose, vec![[0.; 3]; 4]).unwrap();
    assert!((a.stored_energy().unwrap() - 2. * b.stored_energy().unwrap()).abs() < 1e-10);
    let rebuilt = a.rebased(a.rest_positions().to_vec()).unwrap();
    assert_eq!(rebuilt.masses(), a.masses());
    let before = a.masses().to_vec();
    assert!(a.set_face_properties(&[1., f64::NAN], &[1., 1.]).is_err());
    assert_eq!(a.masses(), before);
}
