//! Banded Gauss-Newton solve of the implicit Cosserat rod energy.
//! Each station holds position (3 DOFs) and segment-frame rotation (3 DOFs).
use super::{HairRod, math::*};
const BAND: usize = 9;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_band_solve_recovers_known_solution_and_preserves_fixed_dofs() {
        for n in [12usize, 24, 126] {
            let mut matrix = vec![0.; n * BAND];
            let mut rhs = vec![0.; n];
            let expected: Vec<_> = (0..n)
                .map(|i| {
                    if (6..n - 3).contains(&i) {
                        (i as f64 - 12.) * 0.125
                    } else {
                        0.
                    }
                })
                .collect();
            for i in 0..n {
                matrix[i * BAND] = 1.;
            }
            for i in 6..n - 3 {
                matrix[i * BAND] = 8.;
                rhs[i] = 8. * expected[i];
                for j in 6.max(i.saturating_sub(BAND - 1))..i {
                    let value = 0.125 / (i - j) as f64;
                    matrix[i * BAND + i - j] = value;
                    rhs[i] += value * expected[j];
                    rhs[j] += value * expected[i];
                }
            }
            cholesky(&mut matrix, &mut rhs, 6..n - 3);
            for (actual, expected) in rhs.iter().zip(expected) {
                assert!((actual - expected).abs() < 1e-12);
            }
            for i in (0..6).chain(n - 3..n) {
                assert_eq!(rhs[i], 0.);
                assert_eq!(matrix[i * BAND], 1.);
            }
        }
    }

    #[test]
    fn packed_constraint_matches_analytic_rank_one_matrix_with_fixed_root() {
        let n = 18;
        let mut matrix = vec![0.; n * BAND];
        let mut rhs = vec![0.; n];
        let gradient = [
            (3, 0.25),
            (4, -0.5),
            (5, 0.75),
            (6, -0.125),
            (7, 0.5),
            (8, -0.75),
            (9, 0.125),
            (10, 0.25),
            (11, 0.375),
        ];
        let stiffness = 3.25;
        let residual = 0.175;
        constraint(&mut matrix, &mut rhs, &gradient, residual, stiffness);
        let mut g = vec![0.; n];
        for (index, value) in gradient {
            if index >= 6 {
                g[index] = value;
            }
        }
        for row in 0..n {
            assert_eq!(rhs[row], -stiffness * residual * g[row]);
            for offset in 0..BAND.min(row + 1) {
                let column = row - offset;
                assert_eq!(matrix[row * BAND + offset], stiffness * g[row] * g[column]);
            }
        }
    }
}
fn entry(matrix: &mut [f64], a: usize, b: usize, value: f64) {
    let (a, b) = if a >= b { (a, b) } else { (b, a) };
    if a - b < BAND {
        matrix[a * BAND + a - b] += value;
    }
}
fn constraint(
    matrix: &mut [f64],
    rhs: &mut [f64],
    gradient: &[(usize, f64)],
    value: f64,
    stiffness: f64,
) {
    for &(a, ga) in gradient {
        if a < 6 {
            continue;
        }
        rhs[a] -= stiffness * value * ga;
        for &(b, gb) in gradient {
            if b < 6 || b > a {
                continue;
            }
            entry(matrix, a, b, stiffness * ga * gb);
        }
    }
}
fn cholesky(matrix: &mut [f64], rhs: &mut [f64], active: std::ops::Range<usize>) {
    let first = active.start;
    let end = active.end;
    for i in first..end {
        let start = i.saturating_sub(BAND - 1).max(first);
        for j in start..=i {
            let mut sum = matrix[i * BAND + i - j];
            for k in start.max(j.saturating_sub(BAND - 1))..j {
                sum -= matrix[i * BAND + i - k] * matrix[j * BAND + j - k];
            }
            matrix[i * BAND + i - j] = if i == j {
                sum.max(1e-30).sqrt()
            } else {
                sum / matrix[j * BAND]
            };
        }
    }
    for i in first..end {
        for j in i.saturating_sub(BAND - 1).max(first)..i {
            rhs[i] -= matrix[i * BAND + i - j] * rhs[j];
        }
        rhs[i] /= matrix[i * BAND];
    }
    for i in (first..end).rev() {
        for j in i + 1..(i + BAND).min(end) {
            rhs[i] -= matrix[j * BAND + j - i] * rhs[j];
        }
        rhs[i] /= matrix[i * BAND];
    }
}
pub(super) fn solve(rod: &mut HairRod, dt: f64) {
    let n = rod.x.len() * 6;
    let mut matrix = std::mem::take(&mut rod.solve_matrix);
    matrix.resize(n * BAND, 0.);
    matrix.fill(0.);
    let mut rhs = std::mem::take(&mut rod.solve_rhs);
    rhs.resize(n, 0.);
    rhs.fill(0.);
    for i in 0..rod.x.len() {
        for axis in 0..3 {
            let id = i * 6 + axis;
            if i == 0 {
                matrix[id * BAND] = 1.;
            } else {
                let k = 1. / rod.inv_mass[i] / (dt * dt);
                matrix[id * BAND] = k;
                rhs[id] = -k * (rod.x[i][axis] - rod.predicted_x[i][axis]);
            }
        }
        let rotation_inertia = if i == 0 || i >= rod.q.len() {
            None
        } else {
            Some((
                1. / rod.inv_inertia[i] / (dt * dt),
                log(qm(rod.q[i], conj(rod.predicted_q[i]))),
            ))
        };
        for axis in 0..3 {
            let id = i * 6 + 3 + axis;
            if i == 0 || i >= rod.q.len() {
                matrix[id * BAND] = 1.;
            } else {
                let (k, rotation) = rotation_inertia.expect("free segment inertia");
                matrix[id * BAND] = k;
                rhs[id] = -k * rotation[axis];
            }
        }
    }
    let ea = rod.material.young_modulus * rod.material.area();
    let ga = ea / (2. * (1. + rod.material.poisson_ratio));
    for i in 0..rod.q.len() {
        let l = rod.lengths[i];
        let basis: [V; 3] = std::array::from_fn(|axis| {
            let mut e = [0.; 3];
            e[axis] = 1.;
            rotate(rod.q[i], e)
        });
        let d3 = basis[2];
        let delta = sub(sub(rod.x[i + 1], rod.x[i]), mul(d3, l));
        // Material transverse and longitudinal strain; stiffness is EA/l or GA/l.
        for axis in 0..3 {
            let e = basis[axis];
            let gradient = cross(e, sub(rod.x[i + 1], rod.x[i]));
            let mut jac = [(0usize, 0.0); 9];
            for a in 0..3 {
                jac[a * 3] = (i * 6 + a, -e[a]);
                jac[a * 3 + 1] = ((i + 1) * 6 + a, e[a]);
                jac[a * 3 + 2] = (i * 6 + 3 + a, gradient[a]);
            }
            constraint(
                &mut matrix,
                &mut rhs,
                &jac,
                dot(delta, e),
                (if axis == 2 { ea } else { ga }) / l,
            );
        }
    }
    for i in 0..rod.rest_relative.len() {
        let mut rel = qm(conj(rod.q[i]), rod.q[i + 1]);
        let sign = if rel
            .iter()
            .zip(rod.rest_relative[i])
            .map(|(a, b)| a * b)
            .sum::<f64>()
            < 0.
        {
            -1.
        } else {
            1.
        };
        rel = rel.map(|x| x * sign);
        let l = (rod.lengths[i] + rod.lengths[i + 1]) * 0.5;
        let rotation_gradient: [Q; 3] = std::array::from_fn(|a| {
            let mut e = [0.; 4];
            e[a] = 1.;
            qm(qm(conj(rod.q[i]), e), rod.q[i + 1])
        });
        for axis in 0..3 {
            let value = 2. * (rel[axis] - rod.rest_relative[i][axis]);
            let mut jac = [(0usize, 0.0); 6];
            for a in 0..3 {
                let g = sign * rotation_gradient[a][axis];
                jac[a * 2] = (i * 6 + 3 + a, -g);
                jac[a * 2 + 1] = ((i + 1) * 6 + 3 + a, g);
            }
            let rigidity = if axis == 2 {
                rod.material.twisting_rigidity()
            } else {
                rod.material.bending_rigidity()
            };
            constraint(&mut matrix, &mut rhs, &jac, value, rigidity / l);
        }
    }
    // Unilateral contact planes remain in the coupled solve: resolving one point
    // propagates along the rod instead of simply shortening the adjacent segment.
    for i in 1..rod.x.len() {
        let normal = rod.normals[i];
        if len(normal) > 1e-12 {
            let normal = unit(normal);
            let gap = dot(sub(rod.x[i], rod.contact_targets[i]), normal);
            if gap <= 1e-10 {
                let jac: [(usize, f64); 3] = std::array::from_fn(|a| (i * 6 + a, normal[a]));
                constraint(
                    &mut matrix,
                    &mut rhs,
                    &jac,
                    gap,
                    ea / rod.lengths[i - 1] * 100.,
                );
            }
        }
    }
    // Root DOFs are fixed and uncoupled; the final station has no segment frame.
    cholesky(&mut matrix, &mut rhs, 6..n - 3);
    let max_angle = (1..rod.q.len())
        .map(|i| len([rhs[i * 6 + 3], rhs[i * 6 + 4], rhs[i * 6 + 5]]))
        .fold(0., f64::max);
    let scale = if max_angle > 0.35 {
        0.35 / max_angle
    } else {
        1.
    };
    for i in 1..rod.x.len() {
        rod.x[i] = add(
            rod.x[i],
            mul([rhs[i * 6], rhs[i * 6 + 1], rhs[i * 6 + 2]], scale),
        );
        if i < rod.q.len() {
            apply(
                &mut rod.q[i],
                mul([rhs[i * 6 + 3], rhs[i * 6 + 4], rhs[i * 6 + 5]], scale),
            );
        }
    }
    rod.solve_matrix = matrix;
    rod.solve_rhs = rhs;
}
