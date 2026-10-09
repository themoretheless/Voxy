//! Bounded pivoted dual active solve; every inequality remains in admission.
use super::*;

fn linear_solve(mut matrix: Vec<f64>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let n = rhs.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&a, &b| {
            matrix[a * n + col]
                .abs()
                .total_cmp(&matrix[b * n + col].abs())
        })?;
        let value = matrix[pivot * n + col];
        if value == 0. || !value.is_finite() {
            return None;
        }
        for j in col..n {
            matrix.swap(col * n + j, pivot * n + j);
        }
        rhs.swap(col, pivot);
        for row in col + 1..n {
            let factor = matrix[row * n + col] / matrix[col * n + col];
            matrix[row * n + col] = 0.;
            for j in col + 1..n {
                matrix[row * n + j] -= factor * matrix[col * n + j];
            }
            rhs[row] -= factor * rhs[col];
        }
    }
    for row in (0..n).rev() {
        for col in row + 1..n {
            rhs[row] -= matrix[row * n + col] * rhs[col];
        }
        rhs[row] /= matrix[row * n + row];
    }
    rhs.iter().all(|x| x.is_finite()).then_some(rhs)
}

pub(super) fn gram(constraints: &[Constraint]) -> Vec<f64> {
    let n = constraints.len();
    let mut matrix = vec![0.; n * n];
    for (i, row) in constraints.iter().enumerate() {
        for (j, column) in constraints.iter().enumerate() {
            matrix[i * n + j] = row
                .entries
                .iter()
                .map(|entry| {
                    let response = if column.response.is_empty() {
                        column
                            .entries
                            .iter()
                            .filter(|other| other.rod == entry.rod && other.point == entry.point)
                            .fold([0.; 3], |v, other| {
                                add(v, mul(other.gradient, other.mobility))
                            })
                    } else {
                        column
                            .response
                            .iter()
                            .filter(|response| response.rod == entry.rod)
                            .fold([0.; 3], |v, response| add(v, response.linear[entry.point]))
                    };
                    dot(entry.gradient, response)
                })
                .sum();
        }
    }
    matrix
}

fn dual(
    matrix: &[f64],
    rhs: &[f64],
    initial: &[f64],
    scales: &[f64],
    tolerance: f64,
) -> Option<Vec<f64>> {
    let n = rhs.len();
    let mut x = initial.to_vec();
    let mut active: Vec<_> = (0..n).filter(|&i| x[i] > 0.).collect();
    for _ in 0..512 {
        let size = active.len();
        if size > 0 {
            let block = active
                .iter()
                .flat_map(|&i| active.iter().map(move |&j| matrix[i * n + j]))
                .collect();
            let target = active.iter().map(|&i| rhs[i]).collect();
            let solution = linear_solve(block, target)?;
            let release = active
                .iter()
                .zip(&solution)
                .enumerate()
                .filter(|(_, (_, z))| **z < 0.)
                .map(|(k, (&i, &z))| (k, x[i] / (x[i] - z)))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((remove, fraction)) = release {
                if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
                    return None;
                }
                for (&i, &z) in active.iter().zip(&solution) {
                    x[i] = (x[i] + fraction * (z - x[i])).max(0.);
                }
                x[active[remove]] = 0.;
                active.remove(remove);
                continue;
            }
            x.fill(0.);
            for (&i, &z) in active.iter().zip(&solution) {
                x[i] = z;
            }
        } else {
            x.fill(0.);
        }
        let gaps: Vec<_> = matrix
            .chunks_exact(n)
            .zip(rhs)
            .map(|(row, b)| row.iter().zip(&x).map(|(a, v)| a * v).sum::<f64>() - b)
            .collect();
        if gaps.iter().any(|v| !v.is_finite()) {
            return None;
        }
        if (0..n).all(|i| {
            if x[i] > 0. {
                gaps[i].abs() * scales[i] <= tolerance * 0.5
            } else {
                gaps[i] * scales[i] >= -tolerance * 0.5
            }
        }) {
            return Some(x);
        }
        let enter = (0..n)
            .filter(|i| !active.contains(i) && gaps[*i] * scales[*i] < -tolerance * 0.5)
            .min_by(|&a, &b| (gaps[a] * scales[a]).total_cmp(&(gaps[b] * scales[b])))?;
        active.push(enter);
        active.sort_unstable();
    }
    None
}

pub(super) fn solve<S: ProjectionVector + ?Sized>(
    constraints: &mut [Constraint],
    state: &mut S,
    tolerance: f64,
) -> Result<bool, &'static str> {
    let n = constraints.len();
    // Bound dense storage to 2 MiB for the 512-row Gram matrix. The captured
    // 304-row feasible physical component exceeded the former 256-row limit.
    if n == 0 || n > 512 {
        return Ok(false);
    }
    let scales: Vec<_> = constraints.iter().map(|c| c.diagonal.sqrt()).collect();
    let mut matrix = gram(constraints);
    for i in 0..n {
        for j in 0..n {
            matrix[i * n + j] /= scales[i] * scales[j];
        }
    }
    if matrix.iter().any(|v| !v.is_finite()) {
        return Err("contact active matrix overflow");
    }
    let initial: Vec<_> = constraints
        .iter()
        .zip(&scales)
        .map(|(c, s)| c.multiplier * s)
        .collect();
    let rhs: Vec<_> = constraints
        .iter()
        .enumerate()
        .map(|(i, c)| {
            (c.bound - c.speed(state)) / scales[i]
                + matrix[i * n..(i + 1) * n]
                    .iter()
                    .zip(&initial)
                    .map(|(a, x)| a * x)
                    .sum::<f64>()
        })
        .collect();
    let Some(solution) = dual(&matrix, &rhs, &initial, &scales, tolerance) else {
        return Ok(false);
    };
    for (i, c) in constraints.iter_mut().enumerate() {
        let multiplier = solution[i] / scales[i];
        c.apply(state, multiplier - c.multiplier);
        c.multiplier = multiplier;
    }
    Ok(state.finite() && constraints.iter().all(|c| c.residual(state) <= tolerance))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires an exported VQP1 projection fixture"]
    fn captured_projection_retains_all_inequality_gates() {
        let path =
            std::env::var("VOXY_HAIR_PROJECTION_FAILURE_FIXTURE").expect("projection fixture");
        let data = std::fs::read(path).unwrap();
        assert_eq!(&data[..4], b"VQP1");
        let n = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        assert!((1..=512).contains(&n));
        let scalar = |offset| f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        let tolerance = scalar(8);
        let original: Vec<_> = (0..n * n).map(|i| scalar(16 + 8 * i)).collect();
        let bounds: Vec<_> = (0..n).map(|i| scalar(16 + 8 * (n * n + i))).collect();
        let multipliers: Vec<_> = (0..n).map(|i| scalar(16 + 8 * (n * n + n + i))).collect();
        let scales: Vec<_> = (0..n).map(|i| original[i * n + i].sqrt()).collect();
        let mut matrix = original.clone();
        for i in 0..n {
            for j in 0..n {
                matrix[i * n + j] /= scales[i] * scales[j];
            }
        }
        let rhs: Vec<_> = bounds.iter().zip(&scales).map(|(b, s)| b / s).collect();
        let initial: Vec<_> = multipliers
            .iter()
            .zip(&scales)
            .map(|(x, s)| x * s)
            .collect();
        let result =
            dual(&matrix, &rhs, &initial, &scales, tolerance).expect("captured dual solve");
        let lambda: Vec<_> = result.iter().zip(&scales).map(|(x, s)| x / s).collect();
        for i in 0..n {
            let gap = original[i * n..(i + 1) * n]
                .iter()
                .zip(&lambda)
                .map(|(g, x)| g * x)
                .sum::<f64>()
                - bounds[i];
            assert!(lambda[i] >= 0.);
            assert!(
                if lambda[i] > 0. {
                    gap.abs() <= tolerance
                } else {
                    gap >= -tolerance
                },
                "row {i}: {gap}"
            );
        }
    }
    #[test]
    fn dependent_active_plane_releases_without_discarding_inequality() {
        let epsilon = 1e-14;
        let matrix = vec![1., 1., 1., 1. + epsilon];
        let rhs = vec![1., 1. - 1e-6];
        let x = dual(&matrix, &rhs, &[0.5, 0.5], &[1., 1.], 1e-12).unwrap();
        assert!((x[0] - 1.).abs() < 1e-12);
        assert_eq!(x[1], 0.);
        assert!(matrix[2] * x[0] + matrix[3] * x[1] >= rhs[1]);
    }
    #[test]
    fn released_plane_reenters_when_other_reaction_closes_it() {
        let matrix = vec![1., -0.5, -0.5, 1.];
        let rhs = vec![1., 0.];
        let x = dual(&matrix, &rhs, &[1., 0.], &[1., 1.], 1e-12).unwrap();
        assert!((x[0] - 4. / 3.).abs() < 1e-12);
        assert!((x[1] - 2. / 3.).abs() < 1e-12);
    }
}
