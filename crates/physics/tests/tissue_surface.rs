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

#[test]
fn relative_embedding_preserves_posed_skin_and_adds_only_physical_displacement() {
    let rest_surface = [[0.25; 3], [0., 0., 0.]];
    let binding = EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &rest_surface).unwrap();
    for origin in [0., 1e6] {
        let pose = |p: [f64; 3]| [origin - p[1], -3. + p[0], 2. + p[2]];
        let reference = REST.map(pose);
        // A render skin can have an authored offset from the embedded surface.
        let skin = rest_surface.map(|p| {
            let mut q = pose(p);
            q[2] += 0.07;
            q
        });
        let mut output = [[99.; 3]; 2];
        binding
            .deform_relative_into(&reference, &reference, &skin, &mut output)
            .unwrap();
        assert_eq!(output, skin);
        let mut physical = reference;
        physical[3][2] += 0.4;
        binding
            .deform_relative_into(&reference, &physical, &skin, &mut output)
            .unwrap();
        assert!((output[0][2] - skin[0][2] - 0.1).abs() < 1e-14);
        assert_eq!(output[0][..2], skin[0][..2]);
        assert_eq!(output[1], skin[1]);
    }
}
#[test]
fn relative_embedding_failure_does_not_publish_partial_skin() {
    let binding = EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &[[0.25; 3], [0.1; 3]]).unwrap();
    let mut output = [[99.; 3]; 2];
    let previous = output;
    let mut bad = REST;
    bad[3][2] = f64::NAN;
    assert!(
        binding
            .deform_relative_into(&REST, &bad, &[[0.; 3]; 2], &mut output)
            .is_err()
    );
    assert_eq!(output, previous);
    assert!(
        binding
            .deform_relative_into(&REST[..3], &REST, &[[0.; 3]; 2], &mut output)
            .is_err()
    );
    assert_eq!(output, previous);
    let huge = [[f64::MAX; 3]; 4];
    let negative = [[-f64::MAX; 3]; 4];
    assert!(
        binding
            .deform_relative_into(&negative, &huge, &[[0.; 3]; 2], &mut output)
            .is_err()
    );
    assert_eq!(output, previous);
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[test]
fn embedded_force_transfer_preserves_virtual_work_resultant_and_moment() {
    let skin = [[0.2, 0.3, 0.1], [0.1, 0.2, 0.4], [1., 0., 0.]];
    let binding = EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &skin).unwrap();
    let forces = [[3., -2., 7.], [-5., 11., 2.], [0.2, -0.8, 1.]];
    let mut nodal = [[0.; 3]; 4];
    binding.accumulate_forces_into(&forces, &mut nodal).unwrap();
    let displacement = [
        [0.02, -0.03, 0.04],
        [-0.04, 0.05, 0.01],
        [0.03, 0.02, -0.01],
        [0.01, -0.04, 0.02],
    ];
    let surface_displacement = binding.deform(&displacement).unwrap();
    let surface_work: f64 = forces
        .iter()
        .zip(&surface_displacement)
        .map(|(&f, &d)| dot(f, d))
        .sum();
    let body_work: f64 = nodal
        .iter()
        .zip(&displacement)
        .map(|(&f, &d)| dot(f, d))
        .sum();
    assert!((surface_work - body_work).abs() < 1e-14);
    let deformed = [
        [0.3, 0.1, -0.2],
        [1.2, -0.1, 0.2],
        [-0.2, 1.3, 0.1],
        [0.1, 0.2, 1.4],
    ];
    let surface = binding.deform(&deformed).unwrap();
    for axis in 0..3 {
        let source: f64 = forces.iter().map(|f| f[axis]).sum();
        let transferred: f64 = nodal.iter().map(|f| f[axis]).sum();
        assert!((source - transferred).abs() < 1e-13);
        let source_moment: f64 = surface
            .iter()
            .zip(&forces)
            .map(|(&p, &f)| cross(p, f)[axis])
            .sum();
        let transferred_moment: f64 = deformed
            .iter()
            .zip(&nodal)
            .map(|(&p, &f)| cross(p, f)[axis])
            .sum();
        assert!((source_moment - transferred_moment).abs() < 1e-13);
    }
}
#[test]
fn relative_skin_force_matches_independent_potential_derivative() {
    let skin = [[0.25; 3], [0.1, 0.2, 0.3]];
    let binding = EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &skin).unwrap();
    let reference = REST.map(|p| [4. - p[1], -2. + p[0], 3. + p[2]]);
    let base = skin.map(|p| [4. - p[1], -2. + p[0], 3.07 + p[2]]);
    let mut nodes = reference;
    nodes[2][0] += 0.13;
    nodes[3][2] -= 0.11;
    let mut posed = [[0.; 3]; 2];
    binding
        .deform_relative_into(&reference, &nodes, &base, &mut posed)
        .unwrap();
    let targets = [[4., -1.8, 3.2], [3.9, -1.7, 3.6]];
    let stiffness = 7.;
    let forces: Vec<_> = posed
        .iter()
        .zip(targets)
        .map(|(p, t)| std::array::from_fn(|i| stiffness * (t[i] - p[i])))
        .collect();
    let mut nodal = [[0.; 3]; 4];
    binding.accumulate_forces_into(&forces, &mut nodal).unwrap();
    let energy = |p: &[[f64; 3]; 4]| {
        let mut surface = [[0.; 3]; 2];
        binding
            .deform_relative_into(&reference, p, &base, &mut surface)
            .unwrap();
        surface
            .iter()
            .zip(targets)
            .map(|(x, t)| 0.5 * stiffness * (0..3).map(|i| (x[i] - t[i]).powi(2)).sum::<f64>())
            .sum::<f64>()
    };
    for node in 0..4 {
        for axis in 0..3 {
            let mut plus = nodes;
            let mut minus = nodes;
            let h = 1e-6;
            plus[node][axis] += h;
            minus[node][axis] -= h;
            let derivative = (energy(&plus) - energy(&minus)) / (2. * h);
            assert!((derivative + nodal[node][axis]).abs() < 1e-8);
        }
    }
}
#[test]
fn force_accumulation_preserves_existing_loads_and_is_atomic_on_late_overflow() {
    let binding = EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &[[1., 0., 0.]; 2]).unwrap();
    let mut output = [[2., 3., 4.]; 4];
    let previous = output;
    binding
        .accumulate_forces_into(&[[1., 0., 0.], [2., 0., 0.]], &mut output)
        .unwrap();
    assert_eq!(output[1], [5., 3., 4.]);
    assert_eq!(output[0], previous[0]);
    let previous = output;
    assert!(
        binding
            .accumulate_forces_into(&[[f64::MAX, 0., 0.]; 2], &mut output)
            .is_err()
    );
    assert_eq!(output, previous);
    assert!(
        binding
            .accumulate_forces_into(&[[f64::NAN, 0., 0.]; 2], &mut output)
            .is_err()
    );
    assert_eq!(output, previous);
    assert!(
        binding
            .accumulate_forces_into(&[[0.; 3]], &mut output)
            .is_err()
    );
    assert_eq!(output, previous);
}

#[test]
fn moving_skin_reference_work_closes_independent_spring_energy_and_moment() {
    let skin = [[0.25; 3], [0.1, 0.2, 0.3]];
    let binding = EmbeddedSurface::bind(&REST, &[[0, 1, 2, 3]], &skin).unwrap();
    let nodes0 = REST;
    let reference0 = REST;
    let base0 = skin.map(|p| [p[0], p[1], p[2] + 0.07]);
    let node_delta = [
        [0.02, 0.03, -0.01],
        [-0.03, 0.01, 0.02],
        [0.04, -0.02, 0.03],
        [0.01, 0.02, -0.04],
    ];
    let reference_delta = [[0.01, -0.03, 0.02]; 4];
    let base_delta = [[0.03, -0.01, 0.01], [-0.02, 0.01, 0.03]];
    let nodes1: [[f64; 3]; 4] =
        std::array::from_fn(|i| std::array::from_fn(|a| nodes0[i][a] + node_delta[i][a]));
    let reference1: [[f64; 3]; 4] =
        std::array::from_fn(|i| std::array::from_fn(|a| reference0[i][a] + reference_delta[i][a]));
    let base1: [[f64; 3]; 2] =
        std::array::from_fn(|i| std::array::from_fn(|a| base0[i][a] + base_delta[i][a]));
    let mut start = [[0.; 3]; 2];
    let mut end = [[0.; 3]; 2];
    binding
        .deform_relative_into(&reference0, &nodes0, &base0, &mut start)
        .unwrap();
    binding
        .deform_relative_into(&reference1, &nodes1, &base1, &mut end)
        .unwrap();
    let targets = [[0.3, 0.2, 0.5], [0.2, 0.4, 0.1]];
    let k = 11.;
    let force: Vec<_> = start
        .iter()
        .zip(end)
        .zip(targets)
        .map(|((x, y), target)| std::array::from_fn(|a| k * (target[a] - 0.5 * (x[a] + y[a]))))
        .collect();
    let loads = binding.relative_loads(&force).unwrap();
    let energy = |p: &[[f64; 3]; 2]| {
        p.iter()
            .zip(targets)
            .map(|(x, t)| 0.5 * k * (0..3).map(|a| (x[a] - t[a]).powi(2)).sum::<f64>())
            .sum::<f64>()
    };
    let mechanical_work: f64 = loads
        .nodal_forces_n()
        .iter()
        .zip(node_delta)
        .map(|(&f, d)| dot(f, d))
        .sum();
    let actuator_work = loads
        .actuator_work_j(&reference_delta, &base_delta)
        .unwrap();
    assert!(actuator_work.abs() > 1e-4);
    assert!((energy(&end) - energy(&start) + mechanical_work - actuator_work).abs() < 1e-14);
    // Evaluate moments at the midpoint used for this affine path's exact force average.
    let midpoint_nodes = std::array::from_fn::<_, 4, _>(|i| {
        std::array::from_fn(|a| 0.5 * (nodes0[i][a] + nodes1[i][a]))
    });
    let midpoint_reference = std::array::from_fn::<_, 4, _>(|i| {
        std::array::from_fn(|a| 0.5 * (reference0[i][a] + reference1[i][a]))
    });
    let midpoint_base = std::array::from_fn::<_, 2, _>(|i| {
        std::array::from_fn(|a| 0.5 * (base0[i][a] + base1[i][a]))
    });
    let midpoint_skin = std::array::from_fn::<_, 2, _>(|i| {
        std::array::from_fn(|a| 0.5 * (start[i][a] + end[i][a]))
    });
    let origin = [-0.4, 0.2, -0.1];
    let (rig_force, rig_moment) = loads
        .rig_wrench_about(&midpoint_reference, &midpoint_base, origin)
        .unwrap();
    assert!(rig_moment.iter().any(|x| x.abs() > 1e-4));
    for a in 0..3 {
        let physical_force: f64 = loads.nodal_forces_n().iter().map(|f| f[a]).sum();
        let surface_force: f64 = force.iter().map(|f| f[a]).sum();
        assert!((physical_force + rig_force[a] - surface_force).abs() < 1e-14);
        let moment =
            |p: [f64; 3], f: [f64; 3]| cross(std::array::from_fn(|i| p[i] - origin[i]), f)[a];
        let physical_moment: f64 = midpoint_nodes
            .iter()
            .zip(loads.nodal_forces_n())
            .map(|(&p, &f)| moment(p, f))
            .sum();
        let surface_moment: f64 = midpoint_skin
            .iter()
            .zip(&force)
            .map(|(&p, &f)| moment(p, f))
            .sum();
        assert!((physical_moment + rig_moment[a] - surface_moment).abs() < 1e-14);
    }
    assert!(
        loads
            .actuator_work_j(&reference_delta[..3], &base_delta)
            .is_err()
    );
    assert!(
        loads
            .rig_wrench_about(&reference0, &base0, [f64::NAN; 3])
            .is_err()
    );
}

#[test]
fn mixed_surface_preserves_prescribed_vertices_and_virtual_work() {
    let skin = [[0.25; 3], [2., 3., 4.]];
    let binding =
        EmbeddedSurface::bind_relative(&REST, &[[0, 1, 2, 3]], &skin, &[true, false]).unwrap();
    assert!(EmbeddedSurface::bind_relative(&REST, &[[0, 1, 2, 3]], &skin, &[true, true]).is_err());
    assert!(EmbeddedSurface::bind_relative(&REST, &[[0, 1, 2, 3]], &skin, &[true]).is_err());
    let displacement = [
        [0.1, 0.2, 0.3],
        [-0.2, 0.1, 0.4],
        [0.3, -0.1, 0.2],
        [0.4, 0.3, -0.2],
    ];
    let nodes = std::array::from_fn::<_, 4, _>(|i| {
        std::array::from_fn(|a| REST[i][a] + displacement[i][a])
    });
    let mut output = [[99.; 3]; 2];
    binding
        .deform_relative_into(&REST, &nodes, &skin, &mut output)
        .unwrap();
    assert_eq!(output[1].map(f64::to_bits), skin[1].map(f64::to_bits));
    let directions = binding.deform_displacements(&displacement).unwrap();
    assert_eq!(directions[1], [0.; 3]);
    let forces = [[1., -2., 3.], [4., 5., -6.]];
    let loads = binding.relative_loads(&forces).unwrap();
    let surface_work: f64 = forces
        .iter()
        .zip(&directions)
        .flat_map(|(f, d)| (0..3).map(move |a| f[a] * d[a]))
        .sum();
    let nodal_work: f64 = loads
        .nodal_forces_n()
        .iter()
        .zip(&displacement)
        .flat_map(|(f, d)| (0..3).map(move |a| f[a] * d[a]))
        .sum();
    assert!((surface_work - nodal_work).abs() < 1e-14);
    assert_eq!(loads.base_forces_n(), forces);
    // Without a base, absolute position publication would be ambiguous.
    let saved = output;
    assert!(binding.deform_into(&nodes, &mut output).is_err());
    assert_eq!(output, saved);
    let mut bad = nodes;
    bad[3][2] = f64::NAN;
    assert!(
        binding
            .deform_relative_into(&REST, &bad, &skin, &mut output)
            .is_err()
    );
    assert_eq!(output, saved);
}

#[test]
fn indexed_embedding_preserves_boundary_tolerance_and_original_cell_ties() {
    use physics::tissue_surface::TetrahedralEmbedding;
    let mut rest = REST.to_vec();
    rest.extend(REST);
    // Duplicate geometry with separate nodes makes owner choice observable.
    let cells = [[0, 1, 2, 3], [4, 5, 6, 7]];
    let index = TetrahedralEmbedding::new(&rest, &cells).unwrap();
    assert!(index.contains([-5e-11, 0.2, 0.2]).unwrap());
    assert!(!index.contains([-2e-10, 0.2, 0.2]).unwrap());
    assert!(index.contains([f64::NAN, 0., 0.]).is_err());
    let binding = index.bind_relative(&[[0.25; 3]], &[true]).unwrap();
    for p in &mut rest[..4] {
        p[0] += 1.;
    }
    assert_eq!(binding.deform(&rest).unwrap(), vec![[1.25, 0.25, 0.25]]);
}
