//! Linear tetrahedral Dirichlet extension of reference displacements.
use super::{TetraMesh, Vec3, columns, det, dot, inverse, scale, sub};
use std::collections::BTreeMap;

/// Immutable reference Laplacian. This supplies a kinematic reference only;
/// it neither advances physical state nor constrains free dynamic nodes.
#[derive(Clone, Debug)]
pub struct HarmonicReference {
    rest: Vec<Vec3>,
    fixed: Vec<bool>,
    rows: Vec<Vec<(usize, f64)>>,
}
impl HarmonicReference {
    /// Every exterior node must have a prescribed reference position.
    /// # Errors
    /// Rejects invalid topology, missing exterior constraints or singular metrics.
    pub fn new(mesh: &TetraMesh, fixed: Vec<bool>) -> Result<Self, &'static str> {
        mesh.validate()?;
        if fixed.len() != mesh.points.len() || mesh.boundary.iter().flatten().any(|&i| !fixed[i]) {
            return Err("harmonic reference requires every boundary node");
        }
        let mut rows = vec![BTreeMap::<usize, f64>::new(); fixed.len()];
        for cell in &mesh.cells {
            let p = cell.map(|i| mesh.points[i]);
            let shape = columns(sub(p[1], p[0]), sub(p[2], p[0]), sub(p[3], p[0]));
            let volume = det(shape).abs() / 6.;
            let inv = inverse(shape)?;
            let gradients = [
                scale(
                    std::array::from_fn(|i| inv[0][i] + inv[1][i] + inv[2][i]),
                    -1.,
                ),
                inv[0],
                inv[1],
                inv[2],
            ];
            for i in 0..4 {
                for j in 0..4 {
                    let value = volume * dot(gradients[i], gradients[j]);
                    if !value.is_finite() {
                        return Err("harmonic reference metric overflow");
                    }
                    *rows[cell[i]].entry(cell[j]).or_default() += value;
                }
            }
        }
        let rows: Vec<Vec<_>> = rows.into_iter().map(|r| r.into_iter().collect()).collect();
        if rows
            .iter()
            .enumerate()
            .any(|(i, r)| !fixed[i] && !r.iter().any(|&(j, v)| j == i && v.is_finite() && v > 0.))
        {
            return Err("harmonic reference unanchored node");
        }
        Ok(Self {
            rest: mesh.points.clone(),
            fixed,
            rows,
        })
    }
    /// Extend fixed displacements using Jacobi-preconditioned conjugate gradients.
    /// Fixed coordinates are copied exactly. Relative residual is verified against
    /// the original equation after solving; it is not a positional error bound.
    /// # Errors
    /// Rejects invalid input, exhausted iteration budget and nonfinite arithmetic.
    pub fn extend(
        &self,
        prescribed: &[Vec3],
        relative_residual: f64,
        max_iterations: usize,
    ) -> Result<Vec<Vec3>, &'static str> {
        if prescribed.len() != self.rest.len()
            || prescribed.iter().flatten().any(|v| !v.is_finite())
            || !relative_residual.is_finite()
            || relative_residual <= 0.
            || relative_residual >= 1.
            || max_iterations == 0
        {
            return Err("invalid harmonic reference solve input");
        }
        let free: Vec<_> = self
            .fixed
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| (!v).then_some(i))
            .collect();
        let mut output = prescribed.to_vec();
        let scalar_dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        for axis in 0..3 {
            let mut rhs = vec![0.; self.rest.len()];
            for &i in &free {
                rhs[i] = -self.rows[i]
                    .iter()
                    .filter(|&&(j, _)| self.fixed[j])
                    .map(|&(j, k)| k * (prescribed[j][axis] - self.rest[j][axis]))
                    .sum::<f64>();
            }
            let norm = scalar_dot(&rhs, &rhs);
            if !norm.is_finite() {
                return Err("harmonic reference right-hand side overflow");
            }
            let target = relative_residual * relative_residual * norm;
            let apply = |x: &[f64]| {
                let mut result = vec![0.; x.len()];
                for &i in &free {
                    result[i] = self.rows[i]
                        .iter()
                        .filter(|&&(j, _)| !self.fixed[j])
                        .map(|&(j, k)| k * x[j])
                        .sum();
                }
                result
            };
            let precondition = |r: &[f64]| {
                let mut result = vec![0.; r.len()];
                for &i in &free {
                    result[i] = r[i] / self.rows[i].iter().find(|&&(j, _)| j == i).unwrap().1;
                }
                result
            };
            let mut x = vec![0.; rhs.len()];
            let mut residual = rhs.clone();
            let mut z = precondition(&residual);
            let mut direction = z.clone();
            let mut rz = scalar_dot(&residual, &z);
            for _ in 0..max_iterations {
                if scalar_dot(&residual, &residual) <= target {
                    break;
                }
                let action = apply(&direction);
                let denominator = scalar_dot(&direction, &action);
                if !denominator.is_finite() || denominator <= 0. || !rz.is_finite() {
                    return Err("harmonic reference solve breakdown");
                }
                let alpha = rz / denominator;
                for &i in &free {
                    x[i] += alpha * direction[i];
                    residual[i] -= alpha * action[i];
                }
                z = precondition(&residual);
                let next = scalar_dot(&residual, &z);
                let beta = next / rz;
                for &i in &free {
                    direction[i] = z[i] + beta * direction[i];
                }
                rz = next;
            }
            let action = apply(&x);
            let verified = free
                .iter()
                .map(|&i| (rhs[i] - action[i]).powi(2))
                .sum::<f64>();
            if !verified.is_finite() || verified > target {
                return Err("harmonic reference residual budget exceeded");
            }
            for &i in &free {
                output[i][axis] = self.rest[i][axis] + x[i];
            }
        }
        if output.iter().flatten().any(|v| !v.is_finite()) {
            return Err("harmonic reference output overflow");
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn iteration_exhaustion_rejects_without_publishing_partial_reference() {
        let cells: Vec<_> = (0..3)
            .flat_map(|x| (0..3).flat_map(move |y| (0..3).map(move |z| [x, y, z])))
            .collect();
        let mesh = TetraMesh::from_lattice_cells([0.; 3], [0.1; 3], &cells).unwrap();
        let mut fixed = vec![false; mesh.points.len()];
        for &i in mesh.boundary.iter().flatten() {
            fixed[i] = true;
        }
        let reference = HarmonicReference::new(&mesh, fixed.clone()).unwrap();
        let input: Vec<_> = mesh
            .points
            .iter()
            .map(|&p| [p[0] + p[1] * p[1] + p[2] * p[2] * p[2], p[1], p[2]])
            .collect();
        let snapshot = format!("{reference:?}");
        assert!(reference.extend(&input, 1e-12, 1).is_err());
        assert_eq!(format!("{reference:?}"), snapshot);
        let accepted = reference.extend(&input, 1e-12, 128).unwrap();
        for (i, &is_fixed) in fixed.iter().enumerate() {
            if is_fixed {
                assert_eq!(accepted[i], input[i]);
            }
        }
        assert_eq!(format!("{reference:?}"), snapshot);
    }
    #[test]
    fn affine_extension_preserves_boundary_and_interior_and_rejects_missing_constraints() {
        let mesh = TetraMesh::from_tetrahedra(
            vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [0.25; 3]],
            vec![[0, 1, 2, 4], [0, 1, 4, 3], [0, 4, 2, 3], [4, 1, 2, 3]],
        )
        .unwrap();
        let fixed = vec![true, true, true, true, false];
        let reference = HarmonicReference::new(&mesh, fixed).unwrap();
        let transform = |p: Vec3| {
            [
                2. + 0.5 * p[0] - p[1],
                -3. + p[0] + 0.2 * p[1],
                0.4 + 1.3 * p[2],
            ]
        };
        let expected: Vec<_> = mesh.points.iter().copied().map(transform).collect();
        let mut input = expected.clone();
        input[4] = [100.; 3];
        let output = reference.extend(&input, 1e-12, 20).unwrap();
        assert_eq!(&output[..4], &expected[..4]);
        for axis in 0..3 {
            assert!((output[4][axis] - expected[4][axis]).abs() < 1e-14);
        }
        assert!(HarmonicReference::new(&mesh, vec![false; 5]).is_err());
        assert!(reference.extend(&input, 0., 20).is_err());
    }
}
