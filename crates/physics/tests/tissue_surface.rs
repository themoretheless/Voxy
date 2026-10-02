use physics::tissue_surface::EmbeddedSurface;
const REST: [[f64; 3]; 4] = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
#[test]
fn affine_deformation_and_boundary_vertices() {
    let surface = [[0.2, 0.3, 0.1], [0., 0., 0.], [1., 0., 0.], [0.5, 0.5, 0.]];
    let b = EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &surface).unwrap();
    let transform = |p: [f64; 3]| [3. - 2. * p[1], -4. + p[0] + 0.3 * p[2], 2. + 1.7 * p[2]];
    let deformed = b.deform(&REST.map(transform)).unwrap();
    for (actual, rest) in deformed.iter().zip(surface) {
        for (a, e) in actual.iter().zip(transform(rest)) {
            assert!((a - e).abs() < 1e-12);
        }
    }
}
#[test]
fn local_vertex_displacement_uses_barycentric_weight() {
    let b = EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &[[0.25; 3]]).unwrap();
    let mut p = REST;
    p[3][2] += 0.4;
    assert!((b.deform(&p).unwrap()[0][2] - 0.35).abs() < 1e-12);
}
#[test]
fn rejects_bad_topology_outside_and_nonfinite_updates() {
    assert!(EmbeddedSurface::bind(&REST, &[[0, 1, 2, 4]], &[]).is_err());
    assert!(EmbeddedSurface::bind(&REST, &[[0, 0, 2, 3]], &[]).is_err());
    assert!(EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &[[0.5; 3]]).is_err());
    let b = EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &[[0.25; 3]]).unwrap();
    assert!(b.deform(&REST[..3]).is_err());
    let mut p = REST;
    p[0][0] = f64::NAN;
    assert!(b.deform(&p).is_err());
    assert_eq!(b.deform(&REST).unwrap(), vec![[0.25; 3]]);
}
#[test]
fn reusable_buffer_is_atomic_and_matches_allocating_api() {
    let binding = EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &[[0.25; 3], [0.1; 3]]).unwrap();
    let mut output = [[99.0; 3]; 2];
    binding.deform_into(&REST, &mut output).unwrap();
    assert_eq!(output.as_slice(), binding.deform(&REST).unwrap());
    let previous = output;
    let mut invalid = REST;
    invalid[3][2] = f64::INFINITY;
    assert!(binding.deform_into(&invalid, &mut output).is_err());
    assert_eq!(output, previous);
    assert!(binding.deform_into(&REST, &mut output[..1]).is_err());
    assert_eq!(output, previous);
}
