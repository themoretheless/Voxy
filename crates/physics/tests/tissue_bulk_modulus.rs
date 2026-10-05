use physics::tissue::{Material, Tissue};

fn column(scale: f64) -> Tissue {
    Tissue::new(
        vec![[0.; 3], [scale, 0., 0.], [0., scale, 0.], [0., 0., scale]],
        vec![0., 0., 0., 1.],
        vec![],
        vec![[0, 1, 2, 3]],
        Material {
            damping: 8.,
            volume_compliance: 0.01,
            ..Material::default()
        },
    )
    .unwrap()
}

#[test]
fn constant_pressure_matches_bulk_strain_at_two_scales_and_timesteps() {
    let pressure = 20.;
    let bulk = 1000.;
    for scale in [0.5, 1.] {
        for dt in [1. / 120., 1. / 240.] {
            let mut body = column(scale);
            body.set_bulk_modulus(Some(bulk)).unwrap();
            // Consistent generalized pressure load: F_z = -p * dV/dz.
            // dV/dz = scale^2 / 6; the free vertex has mass 1 kg.
            let acceleration = [0., 0., -pressure * scale * scale / 6.];
            for _ in 0..(5. / dt) as usize {
                body.step(dt, acceleration, &[], 4).unwrap();
            }
            let strain = 1. - body.positions()[3][2] / scale;
            assert!(
                (strain - pressure / bulk).abs() < 1e-5,
                "scale={scale}, dt={dt}, strain={strain}"
            );
        }
    }
}

#[test]
fn invalid_configuration_is_atomic_and_legacy_mode_can_be_restored() {
    let mut body = column(1.);
    body.set_bulk_modulus(Some(1000.)).unwrap();
    let before = format!("{body:?}");
    for k in [0., -1., f64::NAN, f64::INFINITY, f64::from_bits(1)] {
        assert!(body.set_bulk_modulus(Some(k)).is_err());
        assert_eq!(format!("{body:?}"), before);
    }
    body.set_bulk_modulus(None).unwrap();
    let mut reference = column(1.);
    body.step(1. / 240., [0., 0., -1.], &[], 4).unwrap();
    reference.step(1. / 240., [0., 0., -1.], &[], 4).unwrap();
    assert_eq!(format!("{body:?}"), format!("{reference:?}"));
    let mut no_volume =
        Tissue::new(vec![[0.; 3]], vec![1.], vec![], vec![], Material::default()).unwrap();
    assert!(no_volume.set_bulk_modulus(Some(1000.)).is_err());
}
