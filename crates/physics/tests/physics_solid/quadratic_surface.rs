use physics::plasticity::{Material, mesh::QuadraticBody};
fn body() -> QuadraticBody {
    QuadraticBody::from_linear(
        vec![[0.; 3], [2., 0., 0.], [0., 3., 0.], [0., 0., 4.]],
        vec![([0, 1, 2, 3], Material::new(1e6, 0., 1e9, 0.).unwrap())],
    )
    .unwrap()
}
#[test]
fn uniform_reference_pressure_balances_force_moment_and_affine_work() {
    let body = body();
    let faces = body.reference_faces().unwrap();
    assert_eq!(faces.len(), 4);
    let pressure = 700.;
    let traction: Vec<_> = faces
        .iter()
        .map(|f| f.normal.map(|n| -pressure * n))
        .collect();
    let loads = body.traction_loads(&traction).unwrap();
    assert!(loads[..4].iter().flatten().all(|v| v.abs() == 0.));
    for axis in 0..3 {
        assert!(loads.iter().map(|f| f[axis]).sum::<f64>().abs() < 1e-10);
        let j = (axis + 1) % 3;
        let k = (axis + 2) % 3;
        let torque: f64 = body
            .positions()
            .iter()
            .zip(&loads)
            .map(|(p, f)| p[j] * f[k] - p[k] * f[j])
            .sum();
        assert!(torque.abs() < 1e-10);
    }
    let eps = 1e-4;
    let work: f64 = body
        .positions()
        .iter()
        .zip(&loads)
        .map(|(p, f)| (0..3).map(|a| f[a] * eps * p[a]).sum::<f64>())
        .sum();
    assert!((work + pressure * 3. * eps * 4.).abs() < 1e-10); // Reference volume=2*3*4/6.
    assert!(body.traction_loads(&traction[..3]).is_err());
    assert!(body.traction_loads(&[[f64::NAN; 3]; 4]).is_err());
}
#[test]
fn selected_face_traction_recovers_resultant_at_face_centroid() {
    let body = body();
    let faces = body.reference_faces().unwrap();
    for (selected, face) in faces.iter().enumerate() {
        let t = [30., -50., 70.];
        let mut traction = vec![[0.; 3]; 4];
        traction[selected] = t;
        let loads = body.traction_loads(&traction).unwrap();
        let center: [f64; 3] = std::array::from_fn(|a| {
            face.nodes[..3]
                .iter()
                .map(|&n| body.positions()[n][a] / 3.)
                .sum()
        });
        for axis in 0..3 {
            assert!(
                (loads.iter().map(|f| f[axis]).sum::<f64>() - face.reference_area_m2 * t[axis])
                    .abs()
                    < 1e-10
            );
            let j = (axis + 1) % 3;
            let k = (axis + 2) % 3;
            let torque: f64 = body
                .positions()
                .iter()
                .zip(&loads)
                .map(|(p, f)| p[j] * f[k] - p[k] * f[j])
                .sum();
            assert!(
                (torque - face.reference_area_m2 * (center[j] * t[k] - center[k] * t[j])).abs()
                    < 1e-10
            );
        }
    }
}
