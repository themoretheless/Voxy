use super::*;
use crate::biomechanics::{Body, Material, TetraMesh};
struct RigidCoarse {
    basis: Vec<Vec<Vec3>>,
    factor: Vec<Vec<f64>>,
}
impl RigidCoarse {
    fn new(body: &Body, weight: &[f64]) -> Self {
        Self::partitioned(body, weight, false)
    }
    fn partitioned(body: &Body, weight: &[f64], spatial: bool) -> Self {
        let center: Vec3 = std::array::from_fn(|a| {
            body.rest.iter().map(|p| p[a]).sum::<f64>() / body.rest.len() as f64
        });
        let mut groups = std::collections::BTreeMap::new();
        for (i, p) in body.rest.iter().enumerate() {
            if body.pinned[i] {
                continue;
            }
            let key = if spatial {
                (0..3).fold(0usize, |k, a| k | (usize::from(p[a] >= center[a]) << a))
            } else {
                0
            };
            groups.entry(key).or_insert_with(Vec::new).push(i);
        }
        let rotations = 3 * groups.len();
        let mut basis = vec![vec![[0.; 3]; body.positions.len()]; rotations + 3];
        for (group, nodes) in groups.values().enumerate() {
            for &node in nodes {
                for axis in 0..3 {
                    basis[group * 3 + axis][node][axis] = 1.;
                }
            }
        }
        for (i, p) in body.rest.iter().enumerate() {
            if body.pinned[i] {
                continue;
            }
            let q: Vec3 = std::array::from_fn(|a| p[a] - center[a]);
            basis[rotations][i] = [0., -q[2], q[1]];
            basis[rotations + 1][i] = [q[2], 0., -q[0]];
            basis[rotations + 2][i] = [-q[1], q[0], 0.];
        }
        if std::env::var_os("VOXY_TEST_COARSE_SMOOTH").is_some() {
            let metric = AssembledMetric::new(body);
            let mut diagonal = vec![[1.; 3]; body.positions.len()];
            let mut bound = 1_f64;
            for (i, row) in metric.rows.iter().enumerate() {
                if body.pinned[i] {
                    continue;
                }
                let block = &row.iter().find(|(j, _)| *j == i).unwrap().1;
                for axis in 0..3 {
                    diagonal[i][axis] = weight[i] + block[axis][axis];
                    assert!(diagonal[i][axis].is_finite() && diagonal[i][axis] > 0.);
                    let sum = weight[i]
                        + row
                            .iter()
                            .map(|(_, b)| b[axis].iter().map(|x| x.abs()).sum::<f64>())
                            .sum::<f64>();
                    bound = bound.max(sum / diagonal[i][axis]);
                }
            }
            assert!(bound.is_finite());
            let damping = 0.8 / bound;
            for v in &mut basis {
                let action = rest_material_action(body, weight, v);
                for i in 0..v.len() {
                    if !body.pinned[i] {
                        for axis in 0..3 {
                            v[i][axis] -= damping * action[i][axis] / diagonal[i][axis];
                        }
                    }
                }
            }
            eprintln!(
                "TEST_COARSE_SMOOTH modes={} steps=1 damping={damping:.17e} scaled_row_sum_bound={bound:.17e} full_amg=false",
                basis.len()
            );
        }
        for v in &mut basis {
            let norm = v.iter().map(|p| dot(*p, *p)).sum::<f64>().sqrt();
            assert!(norm > 0. && norm.is_finite());
            for p in v {
                for x in p {
                    *x /= norm;
                }
            }
        }
        let actions: Vec<_> = basis
            .iter()
            .map(|v| rest_material_action(body, weight, v))
            .collect();
        let dimension = basis.len();
        let mut matrix = vec![vec![0.; dimension]; dimension];
        for i in 0..dimension {
            for j in 0..dimension {
                matrix[i][j] = basis[i]
                    .iter()
                    .zip(&actions[j])
                    .map(|(a, b)| dot(*a, *b))
                    .sum();
            }
        }
        let mut factor = vec![vec![0.; dimension]; dimension];
        for i in 0..dimension {
            for j in 0..=i {
                let mut value = 0.5 * (matrix[i][j] + matrix[j][i]);
                for k in 0..j {
                    value -= factor[i][k] * factor[j][k];
                }
                if i == j {
                    assert!(value.is_finite() && value > 0.);
                    factor[i][j] = value.sqrt();
                } else {
                    factor[i][j] = value / factor[j][j];
                }
            }
        }
        Self { basis, factor }
    }
    fn apply(&self, r: &[Vec3]) -> Vec<Vec3> {
        let dimension = self.basis.len();
        let mut x = vec![0.; dimension];
        for i in 0..dimension {
            let mut value = self.basis[i]
                .iter()
                .zip(r)
                .map(|(a, b)| dot(*a, *b))
                .sum::<f64>();
            for j in 0..i {
                value -= self.factor[i][j] * x[j];
            }
            x[i] = value / self.factor[i][i];
        }
        for i in (0..dimension).rev() {
            for j in i + 1..dimension {
                x[i] -= self.factor[j][i] * x[j];
            }
            x[i] /= self.factor[i][i];
        }
        (0..r.len())
            .map(|node| {
                std::array::from_fn(|a| (0..dimension).map(|i| self.basis[i][node][a] * x[i]).sum())
            })
            .collect()
    }
}
#[test]
fn rigid_coarse_recovers_its_six_mode_subspace() {
    let mesh = TetraMesh::from_lattice_cells([0.; 3], [0.1; 3], &[[0, 0, 0]]).unwrap();
    let body = mesh
        .into_body(
            vec![false; 8],
            &Material {
                shear_pa: 35000.,
                bulk_pa: 2e6,
                fibers: vec![],
            },
        )
        .unwrap();
    let weight = vec![2.; 8];
    let coarse = RigidCoarse::new(&body, &weight);
    for mode in &coarse.basis {
        let action = rest_material_action(&body, &weight, mode);
        let recovered = coarse.apply(&action);
        for (a, b) in recovered.iter().zip(mode) {
            for axis in 0..3 {
                assert!((a[axis] - b[axis]).abs() < 1e-9);
            }
        }
    }
}
#[test]
fn spatial_coarse_recovers_partition_modes_and_preserves_pins() {
    let cells: Vec<_> = (0..2)
        .flat_map(|x| (0..2).flat_map(move |y| (0..2).map(move |z| [x, y, z])))
        .collect();
    let mesh = TetraMesh::from_lattice_cells([0.; 3], [0.1; 3], &cells).unwrap();
    let count = mesh.points.len();
    let mut pins = vec![false; count];
    pins[0] = true;
    let body = mesh
        .into_body(
            pins,
            &Material {
                shear_pa: 35000.,
                bulk_pa: 2e6,
                fibers: vec![],
            },
        )
        .unwrap();
    let weights = vec![2.; count];
    let coarse = RigidCoarse::partitioned(&body, &weights, true);
    assert!(coarse.basis.len() > 6);
    for mode in &coarse.basis {
        let actual = coarse.apply(&rest_material_action(&body, &weights, mode));
        assert_eq!(actual[0], [0.; 3]);
        for (a, b) in actual.iter().zip(mode) {
            for axis in 0..3 {
                assert!((a[axis] - b[axis]).abs() < 1e-8);
            }
        }
    }
}
struct AssembledMetric {
    rows: Vec<Vec<(usize, [[f64; 3]; 3])>>,
}
impl AssembledMetric {
    fn new(body: &Body) -> Self {
        let mut rows =
            vec![std::collections::BTreeMap::<usize, [[f64; 3]; 3]>::new(); body.positions.len()];
        for e in &body.elements {
            let (mu, bulk) = e
                .viscoelastic
                .as_ref()
                .map_or((e.material.shear_pa, e.material.bulk_pa), |m| {
                    m.rest_moduli()
                });
            let lambda = bulk - 2. * mu / 3.;
            for i in 0..4 {
                if body.pinned[e.nodes[i]] {
                    continue;
                }
                for j in 0..4 {
                    if body.pinned[e.nodes[j]] {
                        continue;
                    }
                    let gi = e.gradients[i];
                    let gj = e.gradients[j];
                    let block = rows[e.nodes[i]].entry(e.nodes[j]).or_insert([[0.; 3]; 3]);
                    for a in 0..3 {
                        for b in 0..3 {
                            block[a][b] += e.volume
                                * (if a == b { mu * dot(gi, gj) } else { 0. }
                                    + mu * gi[b] * gj[a]
                                    + lambda * gi[a] * gj[b]);
                        }
                    }
                }
            }
        }
        Self {
            rows: rows.into_iter().map(|r| r.into_iter().collect()).collect(),
        }
    }
    fn apply(&self, body: &Body, weight: &[f64], v: &[Vec3]) -> Vec<Vec3> {
        self.rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let mut result = if body.pinned[i] {
                    [0.; 3]
                } else {
                    v[i].map(|x| weight[i] * x)
                };
                for &(j, block) in row {
                    for a in 0..3 {
                        result[a] += dot(block[a], v[j]);
                    }
                }
                result
            })
            .collect()
    }
}
struct IncompleteCholesky {
    lower: Vec<Vec<(usize, f64)>>,
    diagonal: Vec<f64>,
}
impl IncompleteCholesky {
    fn new(metric: &AssembledMetric, weight: &[f64], shift: f64) -> Result<Self, &'static str> {
        let n = weight.len() * 3;
        let mut lower: Vec<Vec<(usize, f64)>> = Vec::with_capacity(n);
        let mut diagonal: Vec<f64> = Vec::with_capacity(n);
        for i in 0..n {
            let node = i / 3;
            let axis = i % 3;
            let mut row: Vec<_> = metric.rows[node]
                .iter()
                .flat_map(|&(j, block)| (0..3).map(move |a| (j * 3 + a, block[axis][a])))
                .filter(|&(j, _)| j < i)
                .collect();
            row.sort_by_key(|&(j, _)| j);
            for k in 0..row.len() {
                let j = row[k].0;
                let mut sum = 0.;
                let mut a = 0;
                let mut b = 0;
                while a < k && b < lower[j].len() {
                    match row[a].0.cmp(&lower[j][b].0) {
                        std::cmp::Ordering::Less => a += 1,
                        std::cmp::Ordering::Greater => b += 1,
                        std::cmp::Ordering::Equal => {
                            sum += row[a].1 * lower[j][b].1;
                            a += 1;
                            b += 1;
                        }
                    }
                }
                row[k].1 = (row[k].1 - sum) / diagonal[j];
            }
            let stiffness = metric.rows[node]
                .iter()
                .find(|&&(j, _)| j == node)
                .map_or(0., |(_, m)| m[axis][axis]);
            let pivot = (stiffness + weight[node]) * (1. + shift)
                - row.iter().map(|&(_, v)| v * v).sum::<f64>();
            if !pivot.is_finite() || pivot <= 0. {
                return Err("incomplete Cholesky nonpositive pivot");
            }
            diagonal.push(pivot.sqrt());
            lower.push(row);
        }
        Ok(Self { lower, diagonal })
    }
    fn apply(&self, r: &[Vec3]) -> Vec<Vec3> {
        let mut z: Vec<f64> = r.iter().flatten().copied().collect();
        for i in 0..z.len() {
            z[i] = (z[i] - self.lower[i].iter().map(|&(j, l)| l * z[j]).sum::<f64>())
                / self.diagonal[i];
        }
        for i in (0..z.len()).rev() {
            z[i] /= self.diagonal[i];
            let value = z[i];
            for &(j, l) in &self.lower[i] {
                z[j] -= l * value;
            }
        }
        z.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect()
    }
}
#[test]
fn sparse_factor_matches_independent_coupled_node_inverse_and_rejects_bad_pivot() {
    let metric = AssembledMetric {
        rows: vec![vec![(0, [[4., 1., 0.], [1., 3., 1.], [0., 1., 2.]])]],
    };
    let factor = IncompleteCholesky::new(&metric, &[2.], 0.).unwrap();
    let actual = factor.apply(&[[4., -8.5, 0.]]);
    for (a, b) in actual[0].iter().zip([1., -2., 0.5]) {
        assert!((a - b).abs() < 1e-14);
    }
    let invalid = AssembledMetric {
        rows: vec![vec![(0, [[-10., 0., 0.], [0., -10., 0.], [0., 0., -10.]])]],
    };
    assert!(IncompleteCholesky::new(&invalid, &[2.], 0.).is_err());
}
#[test]
#[ignore = "manual sparse factor search residual diagnosis"]
fn full_mesh_incomplete_factor_search_diagnosis() {
    let mesh = TetraMesh::from_medit_volume(
        &std::fs::read_to_string(std::env::var("VOXY_ASSEMBLED_METRIC_MESH").unwrap()).unwrap(),
    )
    .unwrap();
    let mut pinned = vec![false; mesh.points.len()];
    for i in [1282, 167, 1856] {
        pinned[i] = true;
    }
    let body = mesh
        .into_body(
            pinned,
            &Material {
                shear_pa: 35000.,
                bulk_pa: 2e6,
                fibers: vec![],
            },
        )
        .unwrap();
    let weight = vec![2.; body.positions.len()];
    let metric = AssembledMetric::new(&body);
    let diagonal: Vec<_> = weight
        .iter()
        .zip(&body.diagonal)
        .map(|(w, k)| [w + k; 3])
        .collect();
    for shift in [0., 0.001, 0.01, 0.1, 1.] {
        let factor = match IncompleteCholesky::new(&metric, &weight, shift) {
            Ok(factor) => factor,
            Err(error) => {
                eprintln!("IC0_FACTOR_REJECT shift={shift} error={error}");
                continue;
            }
        };
        for phase in [0., 0.4, 1.3] {
            let residual: Vec<_> = (0..weight.len())
                .map(|i| {
                    [
                        0.001 * (i as f64 * 0.17 + phase).sin(),
                        0.001 * (i as f64 * 0.13 - phase).cos(),
                        0.001 * (i as f64 * 0.11 + phase).sin(),
                    ]
                })
                .collect();
            let calls = std::cell::Cell::new(0);
            let direction = preconditioned_direction(
                &body.pinned,
                &residual,
                |v| {
                    calls.set(calls.get() + 1);
                    rest_material_action(&body, &weight, v)
                },
                |r| factor.apply(r),
            );
            let action = rest_material_action(&body, &weight, &direction);
            let mut initial = 0.;
            let mut remaining = 0.;
            let mut objective = 0.;
            for i in 0..weight.len() {
                if !body.pinned[i] {
                    for a in 0..3 {
                        initial += residual[i][a].powi(2) / diagonal[i][a];
                        remaining += (residual[i][a] + action[i][a]).powi(2) / diagonal[i][a];
                        objective +=
                            0.5 * direction[i][a] * action[i][a] + residual[i][a] * direction[i][a];
                    }
                }
            }
            assert!(remaining.is_finite() && objective < 0.);
            for i in [1282, 167, 1856] {
                assert_eq!(direction[i], [0.; 3]);
            }
            eprintln!(
                "IC0_SEARCH_DIAG shift={shift} phase={phase} actions={} common_true_residual_ratio={:.17e} quadratic_objective={objective:.17e} requested_ratio=1e-12",
                calls.get(),
                remaining / initial
            );
        }
    }
}
#[test]
#[ignore = "manual block-Jacobi rest-search residual qualification on imported mesh"]
fn full_mesh_block_jacobi_search_diagnosis() {
    let path = std::env::var("VOXY_ASSEMBLED_METRIC_MESH").unwrap();
    let mesh = TetraMesh::from_medit_volume(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut pinned = vec![false; mesh.points.len()];
    for i in [1282, 167, 1856] {
        pinned[i] = true;
    }
    let body = mesh
        .into_body(
            pinned,
            &Material {
                shear_pa: 35000.,
                bulk_pa: 2e6,
                fibers: vec![],
            },
        )
        .unwrap();
    let weight = vec![2.; body.positions.len()];
    let metric = AssembledMetric::new(&body);
    let diagonal: Vec<_> = weight
        .iter()
        .zip(&body.diagonal)
        .map(|(w, k)| [w + k; 3])
        .collect();
    let inverses: Vec<_> = metric
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut block = row
                .iter()
                .find(|&&(j, _)| i == j)
                .map_or([[0.; 3]; 3], |(_, b)| *b);
            for a in 0..3 {
                block[a][a] += weight[i];
            }
            super::super::super::inverse(block).unwrap()
        })
        .collect();
    let coarse = std::env::var_os("VOXY_TEST_RIGID_COARSE").map(|_| {
        RigidCoarse::partitioned(
            &body,
            &weight,
            std::env::var_os("VOXY_TEST_SPATIAL_COARSE").is_some(),
        )
    });
    for phase in [0., 0.4, 1.3] {
        let residual: Vec<_> = (0..weight.len())
            .map(|i| {
                [
                    0.001 * (i as f64 * 0.17 + phase).sin(),
                    0.001 * (i as f64 * 0.13 - phase).cos(),
                    0.001 * (i as f64 * 0.11 + phase).sin(),
                ]
            })
            .collect();
        let measure = |direction: &[Vec3]| {
            let action = rest_material_action(&body, &weight, direction);
            let mut initial = 0_f64;
            let mut remaining = 0_f64;
            let mut objective = 0_f64;
            for i in 0..weight.len() {
                if !body.pinned[i] {
                    for a in 0..3 {
                        initial += residual[i][a].powi(2) / diagonal[i][a];
                        remaining += (residual[i][a] + action[i][a]).powi(2) / diagonal[i][a];
                        objective +=
                            0.5 * direction[i][a] * action[i][a] + residual[i][a] * direction[i][a];
                    }
                }
            }
            (remaining / initial, objective)
        };
        for block in [false, true] {
            let calls = std::cell::Cell::new(0);
            let start = std::time::Instant::now();
            let direction = preconditioned_direction(
                &body.pinned,
                &residual,
                |v| {
                    calls.set(calls.get() + 1);
                    rest_material_action(&body, &weight, v)
                },
                |r| {
                    let correction = coarse.as_ref().filter(|_| block).map(|c| c.apply(r));
                    r.iter()
                        .enumerate()
                        .map(|(i, r)| {
                            if block {
                                let local = inverses[i].map(|row| dot(row, *r));
                                std::array::from_fn(|a| {
                                    local[a] + correction.as_ref().map_or(0., |c| c[i][a])
                                })
                            } else {
                                std::array::from_fn(|a| r[a] / diagonal[i][a])
                            }
                        })
                        .collect()
                },
            );
            let elapsed = start.elapsed().as_secs_f64();
            let (rho, objective) = measure(&direction);
            assert!(rho.is_finite() && objective < 0.);
            for i in [1282, 167, 1856] {
                assert_eq!(direction[i], [0.; 3]);
            }
            eprintln!(
                "BLOCK_JACOBI_DIAG phase={phase} block={block} actions={} elapsed_s={elapsed:.9} common_true_residual_ratio={rho:.17e} quadratic_objective={objective:.17e} requested_ratio=1e-12 preparation_timing_included=false",
                calls.get()
            );
        }
    }
}
#[test]
#[ignore = "manual assembled search metric parity and assembly-amortized timing"]
fn full_mesh_assembled_search_metric_profile() {
    let path = std::env::var("VOXY_ASSEMBLED_METRIC_MESH").unwrap();
    let mesh = TetraMesh::from_medit_volume(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut pinned = vec![false; mesh.points.len()];
    for i in [1282, 167, 1856] {
        pinned[i] = true;
    }
    let material = Material {
        shear_pa: 35000.,
        bulk_pa: 2e6,
        fibers: vec![],
    };
    let body = mesh.into_body(pinned, &material).unwrap();
    let weight = vec![2.; body.positions.len()];
    let metric = AssembledMetric::new(&body);
    let mut maximum_relative_error = 0_f64;
    for phase in [0., 0.4, 1.3] {
        let v: Vec<_> = (0..weight.len())
            .map(|i| {
                [
                    (i as f64 * 0.17 + phase).sin(),
                    (i as f64 * 0.13 - phase).cos(),
                    (i as f64 * 0.11 + phase).sin(),
                ]
            })
            .collect();
        let old = rest_material_action(&body, &weight, &v);
        let new = metric.apply(&body, &weight, &v);
        let scale = old.iter().flatten().map(|x| x.abs()).fold(0_f64, f64::max);
        for (a, b) in old.iter().flatten().zip(new.iter().flatten()) {
            maximum_relative_error = maximum_relative_error.max((a - b).abs() / scale);
        }
        for i in [1282, 167, 1856] {
            assert_eq!(new[i], [0.; 3]);
        }
        assert!(v.iter().zip(&new).map(|(a, b)| dot(*a, *b)).sum::<f64>() > 0.);
    }
    assert!(maximum_relative_error < 1e-12);
    let vectors: Vec<_> = (0..weight.len())
        .map(|i| {
            [
                (i as f64 * 0.17).sin(),
                (i as f64 * 0.13).cos(),
                (i as f64 * 0.11).sin(),
            ]
        })
        .collect();
    let residual: Vec<_> = vectors.iter().map(|v| v.map(|x| x * 0.001)).collect();
    let diagonal: Vec<_> = weight
        .iter()
        .zip(&body.diagonal)
        .map(|(w, k)| [w + k; 3])
        .collect();
    let old_direction = preconditioned_direction(
        &body.pinned,
        &residual,
        |v| rest_material_action(&body, &weight, v),
        |r| {
            r.iter()
                .zip(&diagonal)
                .map(|(r, d)| std::array::from_fn(|a| r[a] / d[a]))
                .collect()
        },
    );
    let new_direction = preconditioned_direction(
        &body.pinned,
        &residual,
        |v| metric.apply(&body, &weight, v),
        |r| {
            r.iter()
                .zip(&diagonal)
                .map(|(r, d)| std::array::from_fn(|a| r[a] / d[a]))
                .collect()
        },
    );
    let direction_scale = old_direction
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0_f64, f64::max);
    let direction_error = old_direction
        .iter()
        .flatten()
        .zip(new_direction.iter().flatten())
        .map(|(a, b)| (a - b).abs() / direction_scale)
        .fold(0_f64, f64::max);
    let diagnostics = |direction: &[Vec3]| {
        let action = rest_material_action(&body, &weight, direction);
        let mut true_residual = 0_f64;
        let mut initial = 0_f64;
        let mut objective = 0_f64;
        for i in 0..weight.len() {
            if body.pinned[i] {
                continue;
            }
            for a in 0..3 {
                true_residual += (residual[i][a] + action[i][a]).powi(2) / diagonal[i][a];
                initial += residual[i][a].powi(2) / diagonal[i][a];
                objective +=
                    0.5 * direction[i][a] * action[i][a] + residual[i][a] * direction[i][a];
            }
        }
        (true_residual / initial, objective)
    };
    let (old_rho, old_objective) = diagnostics(&old_direction);
    let (new_rho, new_objective) = diagnostics(&new_direction);
    eprintln!(
        "ASSEMBLED_SEARCH_DIAGNOSTICS baseline_true_preconditioned_residual_ratio={old_rho:.17e} candidate_true_preconditioned_residual_ratio={new_rho:.17e} requested_ratio=1e-12 baseline_quadratic_objective={old_objective:.17e} candidate_quadratic_objective={new_objective:.17e}"
    );
    assert!(
        direction_error < 1e-8,
        "search direction relative discrepancy {direction_error}"
    );
    assert!(
        residual
            .iter()
            .zip(&new_direction)
            .map(|(a, b)| dot(*a, *b))
            .sum::<f64>()
            < 0.
    );
    for i in [1282, 167, 1856] {
        assert_eq!(new_direction[i], [0.; 3]);
    }
    eprintln!(
        "ASSEMBLED_DIRECTION_PARITY maximum_relative_error={direction_error:.17e} finite_descent=true pinned_zero=true"
    );
    let applications: usize = std::env::var("VOXY_ASSEMBLED_METRIC_APPLICATIONS")
        .ok()
        .map(|v| v.parse().unwrap())
        .unwrap_or(64);
    assert!((1..=4096).contains(&applications));
    for (trial, assembled) in [false, true, true, false, true, false, false, true]
        .into_iter()
        .enumerate()
    {
        let start = std::time::Instant::now();
        for _ in 0..16 {
            let prepared = assembled.then(|| AssembledMetric::new(&body));
            for _ in 0..applications {
                let v = std::hint::black_box(&vectors);
                std::hint::black_box(if let Some(prepared) = &prepared {
                    prepared.apply(&body, &weight, v)
                } else {
                    rest_material_action(&body, &weight, v)
                });
            }
        }
        eprintln!(
            "ASSEMBLED_METRIC_PROFILE trial={trial} assembled={assembled} elapsed_s={:.9} nodes={} cells={} solves=16 applications_per_solve={applications} assembly_cost_included=true maximum_relative_error={maximum_relative_error:.17e}",
            start.elapsed().as_secs_f64(),
            weight.len(),
            body.elements.len()
        );
    }
}
