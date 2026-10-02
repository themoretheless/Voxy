use physics::biomechanics::*;
fn asset(name: &str) -> Vec<u8> {
    let source = std::path::PathBuf::from(file!());
    let root = if source.is_absolute() {
        source.ancestors().nth(4).unwrap().to_path_buf()
    } else {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    };
    std::fs::read(root.join("assets/anatomy/hra-female/tetrahedra").join(name)).unwrap()
}
#[test]
fn real_reference_anatomy_imports_into_fem_with_expected_volume() {
    for (name, expected) in [
        ("right-lung-lower-envelope.vxtet", 0.0007138658672533887),
        ("liver-envelope.vxtet", 0.0021381984638402943),
        ("right-lung-middle-envelope.vxtet", 0.000213489017940728),
        ("left-ovary.vxtet", 1.0007896602477584e-6),
        ("right-ovary.vxtet", 1.0166682439629903e-6),
        ("right-lens.vxtet", 1.1279325519542648e-7),
        ("left-lens.vxtet", 1.1279326593517876e-7),
        ("papillary-anterolateral.vxtet", 6.905419802662599e-6),
        ("papillary-posteromedial.vxtet", 4.401498768462441e-6),
    ] {
        let mesh = TetraMesh::from_bytes(&asset(name)).unwrap();
        let pins = vec![true; mesh.points.len()];
        let body = mesh
            .into_body(
                pins,
                &Material {
                    shear_pa: 5000.,
                    bulk_pa: 50_000.,
                    fibers: vec![],
                },
            )
            .unwrap();
        assert!((body.reference_volume() - expected).abs() < 1e-10 * expected);
        let (energy, gradient) = body.evaluate(body.positions()).unwrap();
        assert!(energy.abs() < 1e-15);
        assert!(gradient.iter().flatten().all(|v| v.abs() < 1e-10));
    }
}
#[test]
fn malformed_mesh_payload_geometry_and_boundary_are_rejected() {
    let bytes = asset("left-ovary.vxtet");
    assert!(TetraMesh::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(TetraMesh::from_bytes(&extra).is_err());
    let mut invalid = bytes.clone();
    invalid[4..8].copy_from_slice(&2_u32.to_le_bytes());
    assert!(TetraMesh::from_bytes(&invalid).is_err());
    let mut invalid = bytes.clone();
    invalid[20..28].copy_from_slice(&f64::NAN.to_le_bytes());
    assert!(TetraMesh::from_bytes(&invalid).is_err());
    let n = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let c = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let offset = 20 + 24 * n;
    let mut invalid = bytes.clone();
    invalid[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(TetraMesh::from_bytes(&invalid).is_err());
    let mut inverted = bytes.clone();
    for i in 0..4 {
        inverted.swap(offset + 4 + i, offset + 8 + i);
    }
    assert!(TetraMesh::from_bytes(&inverted).is_err());
    let offset = offset + 16 * c;
    let mut inward = bytes.clone();
    for i in 0..4 {
        inward.swap(offset + 4 + i, offset + 8 + i);
    }
    assert!(TetraMesh::from_bytes(&inward).is_err());
}
