use physics::biomechanics::{Matrix, Vec3};
use std::collections::BTreeMap;
pub const K: Matrix = [
    [2e-12, 0.4e-12, 0.2e-12],
    [0.4e-12, 1e-12, -0.1e-12],
    [0.2e-12, -0.1e-12, 3e-12],
];
pub const GRAD: Vec3 = [1000., -600., 800.];
pub const MU: f64 = 0.001;
pub struct Fixture {
    pub points: Vec<Vec3>,
    pub cells: Vec<[usize; 4]>,
    pub pressures: Vec<f64>,
    pub boundaries: Vec<([usize; 3], f64)>,
}
pub fn block(n: usize) -> Fixture {
    assert!((1..=32).contains(&n));
    let id = |i, j, k| i + (n + 1) * (j + (n + 1) * k);
    let mut points = vec![];
    for k in 0..=n {
        for j in 0..=n {
            for i in 0..=n {
                points.push([i as f64, j as f64, k as f64].map(|v| 0.01 * v / n as f64));
            }
        }
    }
    let mut cells = vec![];
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                let a = id(i, j, k);
                let z = id(i + 1, j + 1, k + 1);
                let ring = [
                    id(i + 1, j, k),
                    id(i + 1, j + 1, k),
                    id(i, j + 1, k),
                    id(i, j + 1, k + 1),
                    id(i, j, k + 1),
                    id(i + 1, j, k + 1),
                ];
                for r in 0..6 {
                    cells.push([a, ring[r], ring[(r + 1) % 6], z]);
                }
            }
        }
    }
    let pressure = |nodes: &[usize]| {
        500. + (0..3)
            .map(|axis| {
                GRAD[axis] * nodes.iter().map(|i| points[*i][axis]).sum::<f64>()
                    / nodes.len() as f64
            })
            .sum::<f64>()
    };
    let mut faces = BTreeMap::new();
    for cell in &cells {
        for opposite in 0..4 {
            let mut face =
                std::array::from_fn::<_, 3, _>(|j| cell[if j < opposite { j } else { j + 1 }]);
            face.sort_unstable();
            *faces.entry(face).or_insert(0usize) += 1;
        }
    }
    let boundaries = faces
        .into_iter()
        .filter(|(_, n)| *n == 1)
        .map(|(f, _)| (f, pressure(&f)))
        .collect();
    let pressures = cells.iter().map(|c| pressure(c)).collect();
    Fixture {
        points,
        cells,
        pressures,
        boundaries,
    }
}
pub fn expected_velocity() -> Vec3 {
    std::array::from_fn(|i| -(0..3).map(|j| K[i][j] * GRAD[j]).sum::<f64>() / MU)
}
