use physics::plasticity::{Material, mesh::Body};
fn tet() -> Body {
    Body::new(
        vec![[0.; 3], [2., 0., 0.], [0., 3., 0.], [0., 0., 4.]],
        vec![([0, 1, 2, 3], Material::new(1e9, 0.3, 1e8, 0.).unwrap())],
    )
    .unwrap()
}
#[test]
fn uniform_pressure_is_balanced_and_matches_volume_work() {
    let body = tet();
    let faces = body.exposed_faces().unwrap();
    assert_eq!(faces.len(), 4);
    let pressure = 700.;
    let forces = body.pressure_loads(&[pressure; 4]).unwrap();
    for axis in 0..3 {
        let resultant: f64 = forces.iter().map(|f| f[axis]).sum();
        assert!(resultant.abs() < 1e-10);
        let j = (axis + 1) % 3;
        let k = (axis + 2) % 3;
        let torque: f64 = body
            .positions()
            .iter()
            .zip(&forces)
            .map(|(p, f)| p[j] * f[k] - p[k] * f[j])
            .sum();
        assert!(torque.abs() < 1e-10);
    }
    // dV/dx for axis-aligned tetrahedron: opposite-axis faces are planar.
    let gradient = [
        [-2., -4. / 3., -1.],
        [2., 0., 0.],
        [0., 4. / 3., 0.],
        [0., 0., 1.],
    ];
    for (force, derivative) in forces.iter().zip(gradient) {
        for axis in 0..3 {
            assert!((force[axis] + pressure * derivative[axis]).abs() < 1e-10);
        }
    }
    assert!(body.pressure_loads(&[pressure; 3]).is_err());
    assert!(body.pressure_loads(&[f64::NAN; 4]).is_err());
    assert!(body.pressure_loads(&[f64::MAX; 4]).is_err());
}
#[test]
fn isolated_face_pressure_has_correct_resultant_and_center_of_pressure() {
    let body = tet();
    let faces = body.exposed_faces().unwrap();
    for (selected, face) in faces.iter().enumerate() {
        let mut pressures = vec![0.; 4];
        pressures[selected] = 100.;
        let loads = body.pressure_loads(&pressures).unwrap();
        for axis in 0..3 {
            let force: f64 = loads.iter().map(|f| f[axis]).sum();
            assert!((force + 100. * face.area_m2 * face.normal[axis]).abs() < 1e-10);
            let j = (axis + 1) % 3;
            let k = (axis + 2) % 3;
            let center: [f64; 3] = std::array::from_fn(|a| {
                face.nodes
                    .iter()
                    .map(|&n| body.positions()[n][a] / 3.)
                    .sum()
            });
            let torque: f64 = body
                .positions()
                .iter()
                .zip(&loads)
                .map(|(p, f)| p[j] * f[k] - p[k] * f[j])
                .sum();
            let expected =
                -100. * face.area_m2 * (center[j] * face.normal[k] - center[k] * face.normal[j]);
            assert!((torque - expected).abs() < 1e-10);
        }
    }
}
