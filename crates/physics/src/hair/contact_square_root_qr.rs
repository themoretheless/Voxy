//! Minimum-energy equality coordinates without forming a compliance Gram.
pub(super) fn minimum_norm(columns: &[Vec<f64>], bounds: &[f64], tolerance: f64) -> Option<Vec<f64>> {
    equality_with_reactions(columns, bounds, tolerance).map(|solution| solution.0)
}

fn equality_with_reactions(columns: &[Vec<f64>], bounds: &[f64], tolerance: f64) -> Option<(Vec<f64>, Vec<f64>)> {
    let n = columns.len();
    let m = columns.first()?.len();
    if bounds.len() != n
        || n > m
        || columns
            .iter()
            .any(|c| c.len() != m || c.iter().any(|v| !v.is_finite()))
        || bounds.iter().any(|v| !v.is_finite())
        || !tolerance.is_finite() || tolerance <= 0.
    {
        return None;
    }
    let mut q: Vec<Vec<f64>> = Vec::with_capacity(n);
    let mut r = vec![0.; n * n];
    for (k, column) in columns.iter().enumerate() {
        let mut v = column.clone();
        // Twice-orthogonalized modified Gram-Schmidt retains the original
        // column direction rather than subtracting squared dot products.
        for _ in 0..2 {
            for (j, basis) in q.iter().enumerate() {
                let projection = accurate_dot(basis, &v);
                r[j * n + k] += projection;
                for (value, &axis) in v.iter_mut().zip(basis) {
                    *value -= projection * axis;
                }
            }
        }
        let scale = v.iter().map(|x| x.abs()).fold(0., f64::max);
        if scale == 0. || !scale.is_finite() {
            return None;
        }
        let norm = scale
            * v.iter()
                .map(|x| (x / scale) * (x / scale))
                .sum::<f64>()
                .sqrt();
        if norm == 0. || !norm.is_finite() {
            return None;
        }
        r[k * n + k] = norm;
        for value in &mut v {
            *value /= norm;
        }
        q.push(v);
    }
    // W=Q*R, so W^T*y=b gives R^T*z=b and y=Q*z.
    let mut z = bounds.to_vec();
    for i in 0..n {
        for j in 0..i {
            z[i] -= r[j * n + i] * z[j];
        }
        z[i] /= r[i * n + i];
    }
    let mut result = vec![0.; m];
    for (column, &value) in q.iter().zip(&z) {
        for (out, &axis) in result.iter_mut().zip(column) {
            *out += axis * value;
        }
    }
    // y=W*lambda=Q*R*lambda, so the same factor gives R*lambda=z.
    for i in (0..n).rev() {
        for j in i + 1..n {
            z[i] -= r[i * n + j] * z[j];
        }
        z[i] /= r[i * n + i];
    }
    // Correct rounding in Q*z against the original columns using the same
    // factor. Keep reactions consistent with every coordinate correction.
    for _ in 0..8 {
        let mut delta: Vec<f64> = columns.iter().zip(bounds)
            .map(|(column, bound)| bound - accurate_dot(column, &result)).collect();
        // Stop at the caller's unchanged admission tolerance. Continuing
        // after admission can oscillate between neighboring rounded points.
        if delta.iter().all(|v| v.abs() <= tolerance) { break; }
        if delta.iter().any(|v| !v.is_finite()) { return None; }
        for i in 0..n {
            for j in 0..i { delta[i] -= r[j * n + i] * delta[j]; }
            delta[i] /= r[i * n + i];
        }
        for (column, value) in q.iter().zip(&delta) {
            for (out, axis) in result.iter_mut().zip(column) { *out += axis * value; }
        }
        for i in (0..n).rev() {
            for j in i + 1..n { delta[i] -= r[i * n + j] * delta[j]; }
            delta[i] /= r[i * n + i];
        }
        for (value, correction) in z.iter_mut().zip(delta) { *value += correction; }
    }
    (result.iter().chain(&z).all(|v| v.is_finite())).then_some((result, z))
}

pub(super) fn unilateral(
    columns: &[Vec<f64>],
    bounds: &[f64],
    tolerance: f64,
) -> Option<(Vec<f64>, Vec<f64>)> {
    let n = columns.len();
    let m = columns.first()?.len();
    if bounds.len() != n
        || columns
            .iter()
            .any(|c| c.len() != m || c.iter().any(|v| !v.is_finite()))
        || bounds.iter().any(|v| !v.is_finite())
        || !tolerance.is_finite()
        || tolerance <= 0.
    {
        return None;
    }
    let mut state = vec![0.; m];
    let mut multipliers = vec![0.; n];
    let mut active = Vec::new();
    for _ in 0..512 {
        if !active.is_empty() {
            let selected: Vec<_> = active.iter().map(|&i: &usize| columns[i].clone()).collect();
            let targets: Vec<_> = active.iter().map(|&i| bounds[i]).collect();
            let (candidate, reactions) = equality_with_reactions(&selected, &targets, tolerance)?;
            let release = active
                .iter()
                .zip(&reactions)
                .enumerate()
                .filter(|(_, (_, v))| **v < 0.)
                .map(|(k, (&i, &v))| (k, multipliers[i] / (multipliers[i] - v)))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((k, fraction)) = release {
                if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
                    return None;
                }
                for (value, &next) in state.iter_mut().zip(&candidate) {
                    *value += fraction * (next - *value);
                }
                for (&i, &next) in active.iter().zip(&reactions) {
                    multipliers[i] = (multipliers[i] + fraction * (next - multipliers[i])).max(0.);
                }
                multipliers[active[k]] = 0.;
                active.remove(k);
                continue;
            }
            state = candidate;
            multipliers.fill(0.);
            for (&i, &v) in active.iter().zip(&reactions) {
                multipliers[i] = v;
            }
        }
        let gaps: Vec<_> = columns
            .iter()
            .zip(bounds)
            .map(|(c, b)| accurate_dot(c, &state) - b)
            .collect();
        if gaps.iter().any(|v| !v.is_finite()) {
            return None;
        }
        if (0..n).all(|i| {
            if multipliers[i] > 0. {
                gaps[i].abs() <= tolerance
            } else {
                gaps[i] >= -tolerance
            }
        }) {
            return Some((state, multipliers));
        }
        let enter = (0..n)
            .filter(|i| !active.contains(i) && gaps[*i] < -tolerance)
            .min_by(|&a, &b| gaps[a].total_cmp(&gaps[b]))?;
        active.push(enter);
        active.sort_unstable();
    }
    None
}

// A small clearance must survive cancellation between much larger terms.
// Neumaier summation retains addition error; FMA also retains product error.
fn accurate_dot(a: &[f64], b: &[f64]) -> f64 {
    let mut sum = 0f64;
    let mut correction = 0f64;
    for (&x, &y) in a.iter().zip(b) {
        let product = x * y;
        let next = sum + product;
        correction += if sum.abs() >= product.abs() {
            (sum - next) + product
        } else {
            (product - next) + sum
        };
        correction += x.mul_add(y, -product);
        sum = next;
    }
    sum + correction
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a captured VQC1 contact input"]
    fn captured_active_contacts_retain_original_clearance() {
        use std::io::Read;
        let path = std::env::var("VOXY_HAIR_QR_INPUT_FIXTURE").unwrap();
        let mut input = std::io::Cursor::new(std::fs::read(path).unwrap());
        let mut magic = [0; 4]; input.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, b"VQC1");
        fn integer(input: &mut std::io::Cursor<Vec<u8>>) -> usize {
            let mut b = [0; 4]; input.read_exact(&mut b).unwrap();
            u32::from_le_bytes(b) as usize
        }
        fn scalar(input: &mut std::io::Cursor<Vec<u8>>) -> f64 {
            let mut b = [0; 8]; input.read_exact(&mut b).unwrap();
            f64::from_le_bytes(b)
        }
        let rows = integer(&mut input); let width = integer(&mut input);
        let systems = integer(&mut input); let _refinement = integer(&mut input);
        let tolerance = scalar(&mut input);
        assert!(rows <= 512 && width <= 65536);
        let original_bounds: Vec<_> = (0..rows).map(|_| scalar(&mut input)).collect();
        let bounds: Vec<_> = (0..rows).map(|_| scalar(&mut input)).collect();
        let columns: Vec<Vec<_>> = (0..rows).map(|_| (0..width)
            .map(|_| scalar(&mut input)).collect()).collect();
        let (state, reactions) = unilateral(&columns, &bounds, tolerance).unwrap();
        for i in 0..rows {
            let gap = accurate_dot(&columns[i], &state) - bounds[i];
            assert!(reactions[i] >= 0.);
            assert!(if reactions[i] > 0. { gap.abs() <= tolerance } else { gap >= -tolerance });
        }
        // The whitened point alone is insufficient: replay immutable original
        // loads through the canonical factors and physical admission too.
        let mut requests = Vec::new();
        for _ in 0..systems {
            let n = integer(&mut input); let band = integer(&mut input);
            let lo = integer(&mut input); let hi = integer(&mut input);
            assert!(n <= 65536 && band == super::super::direct::BAND && lo <= hi && hi <= n);
            let matrix = (0..n*band).map(|_| scalar(&mut input)).collect();
            let rhs = (0..n).map(|_| scalar(&mut input)).collect();
            let loads = (0..rows).map(|_| (0..n).map(|_| scalar(&mut input)).collect()).collect();
            requests.push(super::super::HairResponseSystem {
                system: super::super::HairLinearSystem { band_width: band, matrix, rhs, active: lo..hi },
                loads,
            });
        }
        assert_eq!(input.position() as usize, input.get_ref().len());
        let (responses, reactions) = super::super::HairResponseSystem::solve_joint_load_inequalities_native(
            &requests, &original_bounds, tolerance).expect("original physical load admission");
        for i in 0..rows {
            let gap = requests.iter().zip(&responses).map(|(r,x)|
                r.loads[i].iter().zip(x).map(|(a,b)| a*b).sum::<f64>()).sum::<f64>() - original_bounds[i];
            assert!(reactions[i] >= 0.);
            assert!(if reactions[i] > 0. { gap.abs() <= tolerance } else { gap >= -tolerance });
        }
    }

    #[test]
    fn opening_contacts_release_and_coupled_closing_contacts_enter() {
        let columns = vec![vec![1., 0.], vec![-0.5, 3f64.sqrt() * 0.5]];
        let (state, reactions) = unilateral(&columns, &[-1., -1.], 1e-14).unwrap();
        assert_eq!(state, vec![0.; 2]);
        assert_eq!(reactions, vec![0.; 2]);
        let (state, reactions) = unilateral(&columns, &[1., 0.], 1e-14).unwrap();
        assert!((state[0] - 1.).abs() < 1e-14);
        assert!((state[1] - 1. / 3f64.sqrt()).abs() < 1e-14);
        assert!((reactions[0] - 4. / 3.).abs() < 1e-14);
        assert!((reactions[1] - 2. / 3.).abs() < 1e-14);
    }
    #[test]
    fn near_parallel_constraint_keeps_one_clearance_budget() {
        let columns = vec![vec![1., 0.], vec![1., 1e-10]];
        let (state, reactions) = unilateral(&columns, &[1., 1. - 1e-6], 1e-14).unwrap();
        assert_eq!(state, vec![1., 0.]);
        assert_eq!(reactions, vec![1., 0.]);
        assert!(
            columns[1]
                .iter()
                .zip(&state)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                >= 1. - 1e-6
        );
        assert!(unilateral(&columns, &[1., f64::NAN], 1e-14).is_none());
    }
    #[test]
    fn two_active_constraints_retain_the_direction_missing_from_gram() {
        let columns = vec![vec![1., 0.], vec![-1., 1e-10]];
        let norm = columns[1].iter().map(|v| v * v).sum::<f64>();
        assert_eq!(
            norm - 1.,
            0.,
            "rounded Gram has lost the independent direction"
        );
        let (state, reactions) = unilateral(&columns, &[1., 1.], 1e-14).unwrap();
        assert_eq!(state, vec![1., 2e10]);
        assert!(reactions.iter().all(|v| v.is_finite() && *v > 0.));
        for column in &columns {
            assert!(
                (column.iter().zip(&state).map(|(a, b)| a * b).sum::<f64>() - 1.).abs() <= 1e-14
            );
        }
    }
}
