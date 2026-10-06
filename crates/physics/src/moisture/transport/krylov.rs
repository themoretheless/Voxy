//! Symmetric capacity/activity solve after positive transport fails to converge.
//! PCG follows the SPD formulation in https://www.netlib.org/templates/templates.html.
use super::*;
pub(super) fn saturations(
    body: &Body,
    dt: f64,
    baths: &[Reservoir],
    added: &[f64],
) -> Result<Vec<f64>, &'static str> {
    let n = body.cells.len();
    let mut base: Vec<_> = body.cells.iter().map(|c| c.capacity_kg).collect();
    let mut rhs: Vec<_> = body
        .cells
        .iter()
        .zip(added)
        .map(|(c, a)| c.water_kg + a)
        .collect();
    for r in baths {
        let w = dt * r.conductance_kg_s;
        base[r.cell] += w;
        rhs[r.cell] += w * r.saturation;
    }
    let edges: Vec<_> = body
        .links
        .iter()
        .map(|l| (l.cells, dt * l.conductance_kg_s))
        .collect();
    let mut diagonal = base.clone();
    for &([a, b], w) in &edges {
        diagonal[a] += w;
        diagonal[b] += w;
    }
    if base
        .iter()
        .chain(&rhs)
        .chain(&diagonal)
        .any(|v| !v.is_finite() || *v < 0.)
    {
        return Err("moisture Krylov operator overflow");
    }
    let scale = rhs.iter().copied().fold(0., f64::max);
    if scale == 0. {
        return Ok(vec![0.; n]);
    }
    for b in &mut rhs {
        *b /= scale;
    }
    let total: f64 = rhs.iter().sum();
    let apply = |x: &[f64], out: &mut [f64]| {
        for i in 0..n {
            out[i] = base[i] * x[i];
        }
        for &([a, b], w) in &edges {
            let flux = w * (x[a] - x[b]);
            out[a] += flux;
            out[b] -= flux;
        }
    };
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
    let mut x = vec![0.; n];
    let mut residual = rhs.clone();
    let mut z: Vec<_> = residual.iter().zip(&diagonal).map(|(r, d)| r / d).collect();
    let mut direction = z.clone();
    let mut rz = dot(&residual, &z);
    let mut product = vec![0.; n];
    // Recompute the actual residual before accepting, never trust recurrence alone.
    for _ in 0..(4 * n).min(20_000) {
        apply(&direction, &mut product);
        let denom = dot(&direction, &product);
        if !denom.is_finite() || denom <= 0. || !rz.is_finite() || rz <= 0. {
            break;
        }
        let alpha = rz / denom;
        for i in 0..n {
            x[i] += alpha * direction[i];
            residual[i] -= alpha * product[i];
        }
        if residual.iter().map(|r| r.abs()).sum::<f64>() <= 1e-12 * total {
            apply(&x, &mut product);
            let true_error: f64 = product.iter().zip(&rhs).map(|(a, b)| (a - b).abs()).sum();
            let drift = (base.iter().zip(&x).map(|(b, x)| b * x).sum::<f64>() - total).abs();
            // Difference fluxes inherit rounding in the stored activities. Bound
            // that evaluation error, while retaining a hard relative residual
            // ceiling and the independent, stricter global mass balance gate.
            let roundoff = 8.
                * f64::EPSILON
                * (base.iter().zip(&x).map(|(b, x)| b * x.abs()).sum::<f64>()
                    + edges
                        .iter()
                        .map(|&([a, b], w)| w * (x[a].abs() + x[b].abs()))
                        .sum::<f64>());
            let tolerance = (1e-12 * total + roundoff).min(1e-10 * total);
            if true_error <= tolerance && drift <= 1e-12 * total {
                let result: Vec<_> = x.iter().map(|x| x * scale).collect();
                if result
                    .iter()
                    .all(|s| s.is_finite() && (-1e-12..=1. + 1e-12).contains(s))
                {
                    return Ok(result);
                }
                return Err("moisture Krylov saturation outside bounds");
            }
            // Restart with the true residual if the recursive residual drifted.
            for i in 0..n {
                residual[i] = rhs[i] - product[i];
                z[i] = residual[i] / diagonal[i];
            }
            direction.copy_from_slice(&z);
            rz = dot(&residual, &z);
            continue;
        }
        for i in 0..n {
            z[i] = residual[i] / diagonal[i];
        }
        let next = dot(&residual, &z);
        let beta = next / rz;
        for i in 0..n {
            direction[i] = z[i] + beta * direction[i];
        }
        rz = next;
    }
    Err("moisture Krylov transport did not converge")
}
