use physics::wear::{Column, Material, Stratum};
fn column() -> Column {
    Column::new(
        1.,
        &[
            Stratum {
                thickness_m: 0.1,
                density_kg_m3: 2.,
                material: Material::new(10., 1.).unwrap(),
            },
            Stratum {
                thickness_m: 0.2,
                density_kg_m3: 3.,
                material: Material::new(20., 1.).unwrap(),
            },
        ],
    )
    .unwrap()
}
#[test]
fn layered_surface_recession_and_material_debris() {
    let mut c = column();
    let r = c.advance(1., 3.).unwrap();
    // First layer consumes 1 m; remaining 2 m remove 0.1 m of harder substrate.
    assert!((r.recession_m - 0.2).abs() < 1e-14);
    assert!((r.strata[0].1.mass_kg - 0.2).abs() < 1e-14);
    assert!((r.strata[1].1.mass_kg - 0.3).abs() < 1e-14);
    assert!((c.remaining_mass_kg() + c.debris_mass_kg() - 0.8).abs() < 1e-14);
    let mut split = column();
    for _ in 0..30 {
        split.advance(1., 0.1).unwrap();
    }
    assert!((split.recession_m() - c.recession_m()).abs() < 1e-14);
    let r = c.advance(1., 5.).unwrap();
    assert!((r.remaining_sliding_distance_m - 3.).abs() < 1e-14);
    assert!((c.recession_m() - 0.3).abs() < 1e-14);
    assert_eq!(c.remaining_mass_kg(), 0.);
}
#[test]
fn shielding_and_failed_update_preserve_geometry() {
    let mut c = column();
    assert!(c.advance(1., f64::NAN).is_err());
    assert_eq!(c.recession_m(), 0.);
    assert_eq!(c.debris_mass_kg(), 0.);
    let mut shield = Column::new(
        1.,
        &[Stratum {
            thickness_m: 1.,
            density_kg_m3: 1.,
            material: Material::new(1., 0.).unwrap(),
        }],
    )
    .unwrap();
    let r = shield.advance(1., 100.).unwrap();
    assert_eq!(r.remaining_sliding_distance_m, 0.);
    assert_eq!(shield.recession_m(), 0.);
}

#[test]
fn remaining_geometry_updates_center_and_exact_box_inertia() {
    let mut c = Column::new(
        6.,
        &[Stratum {
            thickness_m: 4.,
            density_kg_m3: 5.,
            material: Material::new(1., 1.).unwrap(),
        }],
    )
    .unwrap();
    let p = c.mass_properties(2.).unwrap().unwrap();
    assert_eq!(p.mass_kg, 120.);
    assert_eq!(p.center_depth_m, 2.);
    for (actual, expected) in p.inertia_kg_m2.into_iter().zip([250., 200., 130.]) {
        assert!((actual - expected).abs() < 1e-12);
    }
    // Remove the front 1 m: remaining box is 2 x 3 x 3, mass 90.
    c.advance(6., 1.).unwrap();
    let p = c.mass_properties(2.).unwrap().unwrap();
    assert_eq!(p.mass_kg, 90.);
    assert_eq!(p.center_depth_m, 2.5);
    for (actual, expected) in p.inertia_kg_m2.into_iter().zip([135., 97.5, 97.5]) {
        assert!((actual - expected).abs() < 1e-12);
    }
    c.advance(6., 3.).unwrap();
    assert!(c.mass_properties(2.).unwrap().is_none());
    assert!(c.mass_properties(0.).is_err());
}
#[test]
fn heterogeneous_centroid_uses_mass_and_parallel_axis_theorem() {
    let c = Column::new(
        1.,
        &[
            Stratum {
                thickness_m: 1.,
                density_kg_m3: 1.,
                material: Material::new(1., 1.).unwrap(),
            },
            Stratum {
                thickness_m: 1.,
                density_kg_m3: 3.,
                material: Material::new(1., 1.).unwrap(),
            },
        ],
    )
    .unwrap();
    let p = c.mass_properties(1.).unwrap().unwrap();
    assert_eq!(p.mass_kg, 4.);
    assert_eq!(p.center_depth_m, 1.25);
    assert!((p.inertia_kg_m2[0] - 17. / 12.).abs() < 1e-14);
    assert!((p.inertia_kg_m2[2] - 2. / 3.).abs() < 1e-14);
}

#[test]
fn inherited_rigid_motion_conserves_linear_angular_and_energy_inventory() {
    use physics::wear::RigidMotion;
    let mut c = column();
    let motion = RigidMotion {
        surface_velocity_m_s: [2., -3., 4.],
        angular_velocity_rad_s: [5., 6., -7.],
    };
    let initial = c
        .mass_properties(2.)
        .unwrap()
        .unwrap()
        .moving(motion)
        .unwrap();
    let r = c.advance_rigid(2., 1., 3., motion).unwrap();
    let inventories: Vec<_> = r
        .debris
        .iter()
        .map(|(_, d)| *d)
        .chain(r.remaining)
        .collect();
    for i in 0..3 {
        let p: f64 = inventories.iter().map(|d| d.momentum_kg_m_s[i]).sum();
        let l: f64 = inventories
            .iter()
            .map(|d| d.angular_momentum_kg_m2_s[i])
            .sum();
        assert!((p - initial.momentum_kg_m_s[i]).abs() < 1e-13);
        assert!((l - initial.angular_momentum_kg_m2_s[i]).abs() < 1e-13);
    }
    let energy: f64 = inventories.iter().map(|d| d.kinetic_j).sum();
    assert!((energy - initial.kinetic_j).abs() < 1e-12);
    let before = c.recession_m();
    let bad = RigidMotion {
        surface_velocity_m_s: [f64::MAX; 3],
        ..motion
    };
    assert!(c.advance_rigid(2., 1., 1., bad).is_err());
    assert_eq!(c.recession_m(), before);
}
