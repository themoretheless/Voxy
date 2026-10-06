//! Shared cyclic Jacobi decomposition of a normalized symmetric 3x3 matrix.
pub(crate) fn normalized(a: [[f64; 3]; 3]) -> Result<([f64; 3], [[f64; 3]; 3]), &'static str> {
    if a.iter().flatten().any(|v| !v.is_finite()) {
        return Err("nonfinite symmetric matrix");
    }
    let mut eigen = a;
    let mut directions = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..16 {
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            if eigen[i][j].abs() <= 1e-16 {
                continue;
            }
            let angle = 0.5 * (2. * eigen[i][j]).atan2(eigen[j][j] - eigen[i][i]);
            let (sin, cos) = angle.sin_cos();
            let (ii, jj, ij) = (eigen[i][i], eigen[j][j], eigen[i][j]);
            eigen[i][i] = cos * cos * ii - 2. * sin * cos * ij + sin * sin * jj;
            eigen[j][j] = sin * sin * ii + 2. * sin * cos * ij + cos * cos * jj;
            eigen[i][j] = 0.;
            eigen[j][i] = 0.;
            let k = 3 - i - j;
            let (ik, jk) = (eigen[i][k], eigen[j][k]);
            eigen[i][k] = cos * ik - sin * jk;
            eigen[k][i] = eigen[i][k];
            eigen[j][k] = sin * ik + cos * jk;
            eigen[k][j] = eigen[j][k];
            for row in &mut directions {
                let (vi, vj) = (row[i], row[j]);
                row[i] = cos * vi - sin * vj;
                row[j] = sin * vi + cos * vj;
            }
        }
    }
    let mut order = [0, 1, 2];
    order.sort_by(|&i, &j| eigen[j][j].total_cmp(&eigen[i][i]));
    let principal = order.map(|i| eigen[i][i]);
    let directions = directions.map(|row| order.map(|i| row[i]));
    Ok((principal, directions))
}
