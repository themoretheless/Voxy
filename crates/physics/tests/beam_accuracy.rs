//! Spatial convergence against a small-deflection cantilever reference.
use physics::plasticity::{Material, mesh::Body};
fn solve(nx: u32, n: u32) -> f64 {
    let width = usize::try_from(n + 1).unwrap();
    let id = |i: u32, j: u32, k: u32| {
        (usize::try_from(i).unwrap() * width + usize::try_from(j).unwrap()) * width
            + usize::try_from(k).unwrap()
    };
    let mut positions = Vec::new();
    let mut prescribed = Vec::new();
    for i in 0..=nx {
        for j in 0..=n {
            for k in 0..=n {
                positions.push([
                    f64::from(i) / f64::from(nx),
                    0.1 * f64::from(j) / f64::from(n),
                    0.1 * f64::from(k) / f64::from(n),
                ]);
                prescribed.push(if i == 0 { [Some(0.); 3] } else { [None; 3] });
            }
        }
    }
    let material = Material::new(1e7, 0., 1e9, 0.).unwrap();
    let mut cells = Vec::new();
    for i in 0..nx {
        for j in 0..n {
            for k in 0..n {
                let v = [
                    id(i, j, k),
                    id(i + 1, j, k),
                    id(i, j + 1, k),
                    id(i + 1, j + 1, k),
                    id(i, j, k + 1),
                    id(i + 1, j, k + 1),
                    id(i, j + 1, k + 1),
                    id(i + 1, j + 1, k + 1),
                ];
                for t in [
                    [0, 1, 3, 7],
                    [0, 3, 2, 7],
                    [0, 2, 6, 7],
                    [0, 6, 4, 7],
                    [0, 4, 5, 7],
                    [0, 5, 1, 7],
                ] {
                    cells.push((t.map(|a| v[a]), material));
                }
            }
        }
    }
    let mut body = Body::new(positions.clone(), cells).unwrap();
    let faces = body.exposed_faces().unwrap();
    let mut loads = vec![[0.; 3]; positions.len()];
    // Constant transverse traction on the end face, total force -0.01 N.
    for face in faces {
        if face
            .nodes
            .iter()
            .all(|&node| (positions[node][0] - 1.).abs() < 1e-12)
        {
            for node in face.nodes {
                loads[node][2] -= face.area_m2 / 3.;
            }
        }
    }
    let report = body.equilibrate(&loads, &prescribed, 20, 1e-8).unwrap();
    assert!(report.converged, "{report:?}");
    let reaction: f64 = report
        .reactions_n
        .iter()
        .take(width * width)
        .map(|r| r[2])
        .sum();
    assert!((reaction - 0.01).abs() < 1e-8);
    // Work-conjugate end displacement avoids choosing a corner with local warping.
    let displacement: f64 = loads
        .iter()
        .zip(body.positions().iter().zip(&positions))
        .map(|(f, (x, rest))| f[2] * (x[2] - rest[2]))
        .sum::<f64>()
        / 0.01;
    assert!(
        body.states()
            .iter()
            .all(|s| s.equivalent_plastic_strain() == 0.)
    );
    displacement
}
#[test]
fn cantilever_mesh_refinement_reduces_bending_stiffness_error() {
    let coarse = solve(4, 1);
    let medium = solve(8, 2);
    let previous = solve(12, 2);
    let fine = solve(30, 3);
    // Euler-Bernoulli bending plus rectangular-section Timoshenko shear correction.
    let reference = 0.01 / (3. * 1e7 * (0.1_f64.powi(4) / 12.)) + 0.01 / ((5. / 6.) * 5e6 * 0.01);
    println!(
        "cantilever: coarse={coarse:e}, medium={medium:e}, fine={fine:e}, beam reference={reference:e}; fine/reference={}",
        fine / reference
    );
    assert!(coarse > 0. && medium > coarse && previous > medium && fine > previous);
    assert!((fine - reference).abs() < (coarse - reference).abs());
}
