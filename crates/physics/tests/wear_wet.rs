use physics::{
    moisture::{Calibration, Cell, Properties},
    wear::{Column, Material, Stratum},
};
fn setup() -> (Column, Vec<Cell>, Vec<Calibration>) {
    let dry = Properties {
        young_pa: 100.,
        poisson: 0.25,
        yield_pa: 10.,
        hardening_pa: 0.,
        hardness_pa: 20.,
        wear_coefficient: 1.,
    };
    let wet = Properties {
        hardness_pa: 10.,
        ..dry
    };
    (
        Column::new(
            1.,
            &[Stratum {
                thickness_m: 1.,
                density_kg_m3: 2.,
                material: Material::new(20., 1.).unwrap(),
            }],
        )
        .unwrap(),
        vec![Cell {
            capacity_kg: 4.,
            water_kg: 4.,
        }],
        vec![Calibration::new(dry, wet).unwrap()],
    )
}
#[test]
fn wet_calibration_removes_material_and_its_water_conservatively() {
    let (mut c, mut water, calibration) = setup();
    let r = c.advance_wet(&mut water, &calibration, 1., 2.).unwrap();
    assert!((c.recession_m() - 0.2).abs() < 1e-14);
    assert!((r.wear.strata[0].1.mass_kg - 0.4).abs() < 1e-14);
    assert!((r.debris_water_kg[0] - 0.8).abs() < 1e-14);
    assert_eq!(water[0].saturation().unwrap(), 1.);
    assert!((water[0].water_kg + r.debris_water_kg[0] - 4.).abs() < 1e-14);
    let r = c.advance_wet(&mut water, &calibration, 1., 10.).unwrap();
    assert_eq!(water[0].water_kg, 0.);
    assert_eq!(water[0].capacity_kg, 0.);
    assert!((r.debris_water_kg[0] - 3.2).abs() < 1e-14);
    c.advance_wet(&mut water, &calibration, 1., 1.).unwrap();
}
#[test]
fn invalid_water_rolls_back_both_inventories() {
    let (mut c, mut water, calibration) = setup();
    water[0].water_kg = 5.;
    assert!(c.advance_wet(&mut water, &calibration, 1., 2.).is_err());
    assert_eq!(c.recession_m(), 0.);
    assert_eq!(water[0].water_kg, 5.);
    assert_eq!(water[0].capacity_kg, 4.);
}

#[test]
fn wet_rigid_partition_conserves_full_mechanical_inventory_and_rolls_back() {
    use physics::wear::RigidMotion;
    let (mut c, mut water, calibration) = setup();
    let motion = RigidMotion {
        surface_velocity_m_s: [1., 2., -3.],
        angular_velocity_rad_s: [4., -5., 6.],
    };
    let initial = c
        .wet_mass_properties(2., &water)
        .unwrap()
        .unwrap()
        .moving(motion)
        .unwrap();
    assert_eq!(initial.geometry.mass_kg, 6.);
    let r = c
        .advance_wet_rigid(2., &mut water, &calibration, 1., 2., motion)
        .unwrap();
    let parts: Vec<_> = r
        .debris
        .iter()
        .map(|(_, d)| *d)
        .chain(r.remaining)
        .collect();
    assert!((parts.iter().map(|p| p.geometry.mass_kg).sum::<f64>() - 6.).abs() < 1e-13);
    for i in 0..3 {
        assert!(
            (parts.iter().map(|p| p.momentum_kg_m_s[i]).sum::<f64>() - initial.momentum_kg_m_s[i])
                .abs()
                < 1e-12
        );
        assert!(
            (parts
                .iter()
                .map(|p| p.angular_momentum_kg_m2_s[i])
                .sum::<f64>()
                - initial.angular_momentum_kg_m2_s[i])
                .abs()
                < 1e-12
        );
    }
    assert!((parts.iter().map(|p| p.kinetic_j).sum::<f64>() - initial.kinetic_j).abs() < 1e-11);
    let recession = c.recession_m();
    let old = water[0];
    let bad = RigidMotion {
        surface_velocity_m_s: [f64::MAX; 3],
        ..motion
    };
    assert!(
        c.advance_wet_rigid(2., &mut water, &calibration, 1., 1., bad)
            .is_err()
    );
    assert_eq!(c.recession_m(), recession);
    assert_eq!(water[0].water_kg, old.water_kg);
    assert_eq!(water[0].capacity_kg, old.capacity_kg);
}

#[test]
fn coupled_network_removes_exhausted_cells_and_rolls_back_bad_graph() {
    use physics::moisture::{Body, Link};
    let (mut c, water, calibration) = setup();
    let mut network = Some(Body::new(water, vec![]).unwrap());
    let mut map = vec![0];
    let r = c
        .advance_wet_network(&mut network, &mut map, &calibration, 1., 2., &[])
        .unwrap();
    assert!((r.debris_water_kg[0] - 0.8).abs() < 1e-14);
    assert_eq!(map, vec![0]);
    assert!((network.as_ref().unwrap().cells()[0].water_kg - 3.2).abs() < 1e-14);
    let before = c.recession_m();
    assert!(
        c.advance_wet_network(
            &mut network,
            &mut map,
            &calibration,
            1.,
            10.,
            &[Link {
                cells: [0, 0],
                conductance_kg_s: 1.
            }]
        )
        .is_err()
    );
    assert_eq!(c.recession_m(), before);
    assert_eq!(map, vec![0]);
    assert!((network.as_ref().unwrap().cells()[0].water_kg - 3.2).abs() < 1e-14);
    c.advance_wet_network(&mut network, &mut map, &calibration, 1., 10., &[])
        .unwrap();
    assert!(network.is_none());
    assert!(map.is_empty());
    c.advance_wet_network(&mut network, &mut map, &calibration, 1., 1., &[])
        .unwrap();
}

#[test]
fn disappearing_top_layer_preserves_substrate_identity_and_calibration() {
    use physics::moisture::Body;
    let (_, _, calibration) = setup();
    let mut c = Column::new(
        1.,
        &[
            Stratum {
                thickness_m: 0.1,
                density_kg_m3: 2.,
                material: Material::new(20., 1.).unwrap(),
            },
            Stratum {
                thickness_m: 0.2,
                density_kg_m3: 3.,
                material: Material::new(20., 1.).unwrap(),
            },
        ],
    )
    .unwrap();
    let mut network = Some(
        Body::new(
            vec![
                Cell {
                    capacity_kg: 0.4,
                    water_kg: 0.4,
                },
                Cell {
                    capacity_kg: 0.8,
                    water_kg: 0.4,
                },
            ],
            vec![],
        )
        .unwrap(),
    );
    let mut map = vec![0, 1];
    let calibrations = vec![calibration[0]; 2];
    // Saturated top H=10 consumes 1 m, half-saturated substrate H=15
    // consumes the remaining 2 m, removing 2/15 m of substrate.
    let r = c
        .advance_wet_network(&mut network, &mut map, &calibrations, 1., 3., &[])
        .unwrap();
    assert_eq!(map, vec![1]);
    assert!((c.recession_m() - (0.1 + 2. / 15.)).abs() < 1e-14);
    assert!((r.debris_water_kg[0] - 0.4).abs() < 1e-14);
    assert!((r.debris_water_kg[1] - 4. / 15.).abs() < 1e-14);
    let cell = network.as_ref().unwrap().cells()[0];
    assert!((cell.saturation().unwrap() - 0.5).abs() < 1e-14);
    let r = c
        .advance_wet_network(&mut network, &mut map, &calibrations, 1., 0.5, &[])
        .unwrap();
    assert_eq!(r.wear.strata[0].0, 1);
    assert!((r.wear.strata[0].1.depth_m - 1. / 30.).abs() < 1e-14);
    assert!((network.as_ref().unwrap().cells()[0].water_kg - 1. / 15.).abs() < 1e-14);
    assert!((c.remaining_mass_kg() + c.debris_mass_kg() - 0.8).abs() < 1e-14);
}
#[test]
fn partial_removal_cannot_silently_underflow_retained_water() {
    let (mut c, mut water, calibration) = setup();
    water[0] = Cell {
        capacity_kg: 1.,
        water_kg: f64::from_bits(1),
    };
    assert!(c.advance_wet(&mut water, &calibration, 1., 10.).is_err());
    assert_eq!(c.recession_m(), 0.);
    assert_eq!(water[0].water_kg, f64::from_bits(1));
}
