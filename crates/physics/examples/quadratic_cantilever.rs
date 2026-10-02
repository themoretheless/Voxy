//! Accurate small-strain bending followed by undamped load-release vibration.
use physics::plasticity::{
    Material,
    mesh::{QuadraticBody, QuadraticDynamics},
};
fn specimen(nx: u32) -> Result<QuadraticBody, &'static str> {
    let id = |i: u32, j: u32, k: u32| usize::try_from((i * 2 + j) * 2 + k).unwrap_or(usize::MAX);
    let mut points = Vec::new();
    for i in 0..=nx {
        for j in 0..=1 {
            for k in 0..=1 {
                points.push([
                    f64::from(i) / f64::from(nx),
                    0.1 * f64::from(j),
                    0.1 * f64::from(k),
                ]);
            }
        }
    }
    let material = Material::new(1e7, 0., 1e9, 0.)?;
    let mut cells = Vec::new();
    for i in 0..nx {
        let v = [
            id(i, 0, 0),
            id(i + 1, 0, 0),
            id(i, 1, 0),
            id(i + 1, 1, 0),
            id(i, 0, 1),
            id(i + 1, 0, 1),
            id(i, 1, 1),
            id(i + 1, 1, 1),
        ];
        for t in [
            [0, 1, 3, 7],
            [0, 3, 2, 7],
            [0, 2, 6, 7],
            [0, 6, 4, 7],
            [0, 4, 5, 7],
            [0, 5, 1, 7],
        ] {
            cells.push((t.map(|local| v[local]), material));
        }
    }
    QuadraticBody::from_linear(points, cells)
}
fn main() -> Result<(), &'static str> {
    let steps = std::env::args().nth(1).map_or(Ok(2000_u32), |raw| {
        raw.parse::<u32>().map_err(|_| "expected step count")
    })?;
    if steps == 0 || steps > 200_000 {
        return Err("step count must be 1..=200000");
    }
    let mut body = specimen(8)?;
    let rest = body.positions().to_vec();
    let pinned: Vec<_> = rest.iter().map(|p| p[0].abs() < 1e-12).collect();
    let prescribed: Vec<_> = pinned
        .iter()
        .map(|&p| if p { [Some(0.); 3] } else { [None; 3] })
        .collect();
    let traction: Vec<_> = body
        .reference_faces()?
        .iter()
        .map(|face| {
            if face.nodes[..3]
                .iter()
                .all(|&node| (rest[node][0] - 1.).abs() < 1e-12)
            {
                [0., 0., -1.]
            } else {
                [0.; 3]
            }
        })
        .collect();
    let load = body.traction_loads(&traction)?;
    if !body.equilibrate(&load, &prescribed, 20, 1e-8)?.converged {
        return Err("cantilever equilibrium failed");
    }
    let tip = |positions: &[[f64; 3]]| -> f64 {
        load.iter()
            .zip(positions.iter().zip(&rest))
            .map(|(f, (p, x))| f[2] * (p[2] - x[2]))
            .sum::<f64>()
            / 0.01
    };
    let reference = 0.01 / (3. * 1e7 * (0.1_f64.powi(4) / 12.)) + 0.01 / ((5. / 6.) * 5e6 * 0.01);
    eprintln!(
        "nodes={}; static displacement={:.9e} m; beam reference={reference:.9e} m",
        rest.len(),
        tip(body.positions())
    );
    let count = body.states().len();
    let mut dynamic = QuadraticDynamics::new_supported(
        body,
        &vec![1000.; count],
        vec![[0.; 3]; rest.len()],
        &pinned,
    )?;
    let initial = dynamic.energy()?;
    let reference_energy = initial.elastic_j;
    let mut worst = 0_f64;
    println!("time_s,tip_displacement_m,kinetic_j,elastic_j,relative_energy_error");
    for step in 0..=steps {
        if step > 0 {
            dynamic.step(1e-5, [0.; 3], 1e-10)?;
        }
        let e = dynamic.energy()?;
        let error = (e.kinetic_j + e.elastic_j - reference_energy) / reference_energy;
        worst = worst.max(error.abs());
        if step % 100 == 0 {
            println!(
                "{:.6},{:.9e},{:.9e},{:.9e},{error:.3e}",
                f64::from(step) * 1e-5,
                tip(dynamic.body().positions()),
                e.kinetic_j,
                e.elastic_j
            );
        }
    }
    if worst > 1e-3 {
        return Err("cantilever vibration energy drift");
    }
    eprintln!("max relative vibration energy error={worst:.6e}");
    Ok(())
}
