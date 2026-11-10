use physics::hair::{HairMaterial, HairRod, HairSystem, RootPose};
#[test]
fn groom_rebase_recalculates_mass_and_rest_geometry() {
    let points = vec![
        [0., 0., 0.],
        [0., 0.01, 0.],
        [0.002, 0.02, 0.],
        [0.004, 0.03, 0.],
    ];
    let rod = HairRod::new(points.clone(), HairMaterial::default()).unwrap();
    let old_mass = rod.mass();
    let mut system = HairSystem::new(vec![rod]).unwrap();
    system.iterations = 6;
    system.substeps = 2;
    system.workers = 1;
    system.self_collision = false;
    let scaled: Vec<_> = points.iter().map(|p| p.map(|x| x * 1.5)).collect();
    let mut rebased = system.rebased(vec![scaled.clone()]).unwrap();
    assert!((rebased.rods()[0].mass() / old_mass - 1.5).abs() < 1e-12);
    assert_eq!(rebased.rods()[0].rest_positions(), scaled);
    assert_eq!(rebased.rods()[0].positions(), scaled);
    assert_eq!(rebased.iterations, 6);
    assert_eq!(rebased.substeps, 2);
    assert!(!rebased.self_collision);
    rebased
        .step(
            1. / 240.,
            &[RootPose {
                position: scaled[0],
                rotation: [0., 0., 0., 1.],
            }],
            [0.; 3],
            [0.; 3],
            &[],
        )
        .unwrap();
    assert!(rebased.rods()[0].max_relative_stretch() < 1e-8);
    let mut invalid = scaled;
    invalid[1] = invalid[0];
    assert!(system.rebased(vec![invalid]).is_err());
    assert_eq!(system.rods()[0].rest_positions(), points);
    assert_eq!(system.rods()[0].mass(), old_mass);
}
