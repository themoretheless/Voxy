//! Residual-checked dual projection for rank-deficient contact islands.
use super::*;

enum Product {
    Dense(Vec<f64>),
    MatrixFree(Vec<Vec<V>>),
}

impl Product {
    fn apply(&mut self, constraints: &[Constraint], scales: &[f64], x: &[f64], out: &mut [f64]) {
        match self {
            Self::Dense(gram) => {
                for (row, value) in gram.chunks_exact(x.len()).zip(out) {
                    *value = row.iter().zip(x).map(|(a, b)| a * b).sum();
                }
            }
            Self::MatrixFree(scratch) => {
                for points in scratch.iter_mut() {
                    points.fill([0.; 3]);
                }
                for ((column, scale), value) in constraints.iter().zip(scales).zip(x) {
                    let weight = value / scale;
                    if column.response.is_empty() {
                        for entry in column.entries {
                            let point = &mut scratch[entry.rod][entry.point];
                            *point = add(*point, mul(entry.gradient, entry.mobility * weight));
                        }
                    } else {
                        for response in &column.response {
                            for (point, delta) in
                                scratch[response.rod].iter_mut().zip(&response.linear)
                            {
                                *point = add(*point, mul(*delta, weight));
                            }
                        }
                    }
                }
                for ((row, scale), value) in constraints.iter().zip(scales).zip(out) {
                    *value = row
                        .entries
                        .iter()
                        .map(|entry| dot(entry.gradient, scratch[entry.rod][entry.point]))
                        .sum::<f64>()
                        / scale;
                }
            }
        }
    }
}

fn matrix_free_bound(constraints: &[Constraint], scales: &[f64], scratch: &mut [Vec<V>]) -> f64 {
    // Triangle inequality bounds every absolute row sum of the normalized
    // Gram operator, hence its spectral radius, without quadratic storage.
    for (column, scale) in constraints.iter().zip(scales) {
        if column.response.is_empty() {
            for entry in column.entries {
                let point = &mut scratch[entry.rod][entry.point];
                for axis in 0..3 {
                    point[axis] += (entry.gradient[axis] * entry.mobility / scale).abs();
                }
            }
        } else {
            for response in &column.response {
                for (point, delta) in scratch[response.rod].iter_mut().zip(&response.linear) {
                    for axis in 0..3 {
                        point[axis] += (delta[axis] / scale).abs();
                    }
                }
            }
        }
    }
    constraints
        .iter()
        .zip(scales)
        .map(|(row, scale)| {
            row.entries
                .iter()
                .map(|entry| {
                    (0..3)
                        .map(|axis| {
                            entry.gradient[axis].abs() * scratch[entry.rod][entry.point][axis]
                        })
                        .sum::<f64>()
                })
                .sum::<f64>()
                / scale
        })
        .fold(0., f64::max)
}

pub(super) fn solve<S: ProjectionVector + ?Sized>(
    constraints: &mut [Constraint],
    state: &mut S,
    tolerance: f64,
) -> Result<bool, &'static str> {
    let n = constraints.len();
    if n == 0 {
        return Ok(false);
    }
    let scales: Vec<_> = constraints.iter().map(|c| c.diagonal.sqrt()).collect();
    let (mut product, lipschitz) = if n > 256 {
        let mut scratch: Vec<Vec<V>> = state
            .shape()
            .into_iter()
            .map(|n| vec![[0.; 3]; n])
            .collect();
        let bound = matrix_free_bound(constraints, &scales, &mut scratch);
        (Product::MatrixFree(scratch), bound)
    } else {
        let mut gram = vec![0.; n * n];
        for (i, row) in constraints.iter().enumerate() {
            for (j, column) in constraints.iter().enumerate() {
                gram[i * n + j] = row
                    .entries
                    .iter()
                    .map(|entry| {
                        let response = if column.response.is_empty() {
                            column
                                .entries
                                .iter()
                                .filter(|other| {
                                    other.rod == entry.rod && other.point == entry.point
                                })
                                .fold([0.; 3], |sum, other| {
                                    add(sum, mul(other.gradient, other.mobility))
                                })
                        } else {
                            column
                                .response
                                .iter()
                                .find(|r| r.rod == entry.rod)
                                .map_or([0.; 3], |r| r.linear[entry.point])
                        };
                        dot(entry.gradient, response) / (scales[i] * scales[j])
                    })
                    .sum();
            }
        }
        if gram.iter().any(|v| !v.is_finite()) {
            return Err("contact dual matrix overflow");
        }
        let lipschitz = gram
            .chunks_exact(n)
            .map(|row| row.iter().map(|v| v.abs()).sum::<f64>())
            .fold(0., f64::max);
        (Product::Dense(gram), lipschitz)
    };
    if !lipschitz.is_finite() || lipschitz <= 0. {
        return Err("invalid contact dual curvature");
    }
    let mut x: Vec<_> = constraints
        .iter()
        .zip(&scales)
        .map(|(c, s)| c.multiplier * s)
        .collect();
    let mut values = vec![0.; n];
    product.apply(constraints, &scales, &x, &mut values);
    let rhs: Vec<_> = constraints
        .iter()
        .enumerate()
        .map(|(i, c)| (c.bound - c.speed(state)) / scales[i] + values[i])
        .collect();
    let mut extrapolated = x.clone();
    let mut next = vec![0.; n];
    let mut momentum = 1f64;
    for iteration in 0..100_000 {
        product.apply(constraints, &scales, &extrapolated, &mut values);
        for i in 0..n {
            let gradient = values[i] - rhs[i];
            next[i] = (extrapolated[i] - gradient / lipschitz).max(0.);
        }
        if next.iter().any(|v| !v.is_finite()) {
            return Err("contact dual iterate overflow");
        }
        if iteration % 16 == 0 {
            product.apply(constraints, &scales, &next, &mut values);
            let feasible = (0..n).all(|i| {
                let gap = (values[i] - rhs[i]) * scales[i];
                if next[i] > 0. {
                    gap.abs() <= tolerance * 0.5
                } else {
                    gap >= -tolerance * 0.5
                }
            });
            if feasible {
                for (i, c) in constraints.iter_mut().enumerate() {
                    let multiplier = next[i] / scales[i];
                    c.apply(state, multiplier - c.multiplier);
                    c.multiplier = multiplier;
                }
                // Recheck the published state, not merely the dense surrogate.
                return Ok(
                    state.finite() && constraints.iter().all(|c| c.residual(state) <= tolerance)
                );
            }
        }
        let next_momentum = (1. + (1. + 4. * momentum * momentum).sqrt()) * 0.5;
        for i in 0..n {
            extrapolated[i] = next[i] + (momentum - 1.) / next_momentum * (next[i] - x[i]);
        }
        std::mem::swap(&mut x, &mut next);
        momentum = next_momentum;
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hair::{HairLinearSystem, HairMaterial, direct};

    #[test]
    fn matrix_free_product_matches_combined_native_rod_compliance() {
        let rods: Vec<_> = [0., 80e-6]
            .into_iter()
            .map(|x| {
                HairRod::new(
                    vec![[x, 0., 0.], [x, 0.01, 0.], [x, 0.02, 0.]],
                    HairMaterial::default(),
                )
                .unwrap()
            })
            .collect();
        let zero = Entry {
            rod: 0,
            point: 0,
            gradient: [0.; 3],
            mobility: 0.,
        };
        let mut constraints = Vec::new();
        for point in [1, 2] {
            let pair = entries(
                0,
                point.min(1),
                if point == 1 { 0. } else { 1. },
                [1., 0., 0.],
                &rods[0],
            );
            add_constraint(&mut constraints, [pair[0], pair[1], zero, zero], 0.).unwrap();
        }
        let a = entries(0, 1, 0.3, [-1., 0., 0.], &rods[0]);
        let b = entries(1, 1, 0.6, [1., 0., 0.], &rods[1]);
        add_constraint(&mut constraints, [a[0], a[1], b[0], b[1]], 0.).unwrap();
        let dt = 1. / 240.;
        prepare_implicit_response(&mut constraints, &rods, dt).unwrap();
        let scales: Vec<_> = constraints.iter().map(|c| c.diagonal.sqrt()).collect();
        let weights = [0.2, -0.7, 1.3];
        let mut expected_displacements = Vec::new();
        // Assemble one combined load per rod and solve it independently of
        // the cached per-contact response columns used by the matrix-free path.
        for (index, rod) in rods.iter().enumerate() {
            let (mut matrix, mut force) = direct::assemble(&mut rod.clone(), dt).unwrap();
            matrix.iter_mut().for_each(|v| *v *= dt * dt);
            force.fill(0.);
            for ((constraint, scale), weight) in constraints.iter().zip(&scales).zip(weights) {
                for entry in constraint
                    .entries
                    .iter()
                    .filter(|e| e.rod == index && e.point > 0)
                {
                    for axis in 0..3 {
                        force[entry.point * 6 + axis] += entry.gradient[axis] * weight / scale;
                    }
                }
            }
            let end = force.len() - 3;
            expected_displacements.push(
                HairLinearSystem {
                    band_width: direct::BAND,
                    matrix,
                    rhs: force,
                    active: 6..end,
                }
                .solve_native()
                .unwrap(),
            );
        }
        let mut product =
            Product::MatrixFree(rods.iter().map(|r| vec![[0.; 3]; r.x.len()]).collect());
        let mut actual = [0.; 3];
        product.apply(&constraints, &scales, &weights, &mut actual);
        for ((constraint, scale), actual) in constraints.iter().zip(&scales).zip(actual) {
            let expected = constraint
                .entries
                .iter()
                .map(|e| {
                    (0..3)
                        .map(|axis| {
                            e.gradient[axis] * expected_displacements[e.rod][e.point * 6 + axis]
                        })
                        .sum::<f64>()
                })
                .sum::<f64>()
                / scale;
            assert!((actual - expected).abs() < 1e-10 * expected.abs().max(1.));
        }
    }
}
