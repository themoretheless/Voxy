//! Dense compatibility and positive sparse large-network transport paths.
use super::*;
pub(super) fn saturations(
    body: &Body,
    dt_s: f64,
    reservoirs: &[Reservoir],
    added: &[f64],
) -> Result<Vec<f64>, &'static str> {
    if body.cells.len() <= 128 {
        return dense(body, dt_s, reservoirs, added);
    }
    let n = body.cells.len();
    let mut adjacency = vec![Vec::new(); n];
    for link in &body.links {
        if link.conductance_kg_s > 0. {
            let [a, b] = link.cells;
            adjacency[a].push(b);
            adjacency[b].push(a);
        }
    }
    let mut component = vec![usize::MAX; n];
    let mut groups = Vec::new();
    for start in 0..n {
        if component[start] != usize::MAX {
            continue;
        }
        let id = groups.len();
        let mut nodes = Vec::new();
        let mut stack = vec![start];
        component[start] = id;
        while let Some(i) = stack.pop() {
            nodes.push(i);
            for &j in &adjacency[i] {
                if component[j] == usize::MAX {
                    component[j] = id;
                    stack.push(j);
                }
            }
        }
        nodes.sort_unstable();
        groups.push(nodes);
    }
    if groups.len() == 1 {
        return sparse(body, dt_s, reservoirs, added);
    }
    let mut local_index = vec![0; n];
    let mut blocks: Vec<_> = groups
        .iter()
        .map(|nodes| {
            for (i, &node) in nodes.iter().enumerate() {
                local_index[node] = i;
            }
            Body {
                cells: nodes.iter().map(|&i| body.cells[i]).collect(),
                links: Vec::new(),
            }
        })
        .collect();
    for link in &body.links {
        let [a, b] = link.cells;
        if component[a] != component[b] {
            continue;
        } // only zero-conductance links cross blocks
        blocks[component[a]].links.push(Link {
            cells: [local_index[a], local_index[b]],
            conductance_kg_s: link.conductance_kg_s,
        });
    }
    let mut baths = vec![Vec::new(); groups.len()];
    for r in reservoirs {
        baths[component[r.cell]].push(Reservoir {
            cell: local_index[r.cell],
            saturation: r.saturation,
            conductance_kg_s: r.conductance_kg_s,
        });
    }
    let mut output = vec![0.; n];
    for (id, nodes) in groups.iter().enumerate() {
        let sources: Vec<_> = nodes.iter().map(|&i| added[i]).collect();
        let solution = if nodes.len() <= 128 {
            dense(&blocks[id], dt_s, &baths[id], &sources)?
        } else {
            sparse(&blocks[id], dt_s, &baths[id], &sources)?
        };
        for (&node, saturation) in nodes.iter().zip(solution) {
            output[node] = saturation;
        }
    }
    Ok(output)
}
fn sparse(
    body: &Body,
    dt_s: f64,
    reservoirs: &[Reservoir],
    added: &[f64],
) -> Result<Vec<f64>, &'static str> {
    let n = body.cells.len();
    let mut diagonal = vec![1.; n];
    let mut incoming = vec![Vec::new(); n];
    let mut external_out = vec![0.; n];
    let mut rhs: Vec<_> = body
        .cells
        .iter()
        .zip(added)
        .map(|(c, a)| c.water_kg + a)
        .collect();
    for link in &body.links {
        let [a, b] = link.cells;
        let exchange = dt_s * link.conductance_kg_s;
        let ca = exchange / body.cells[a].capacity_kg;
        let cb = exchange / body.cells[b].capacity_kg;
        diagonal[a] += ca;
        diagonal[b] += cb;
        incoming[a].push((b, cb));
        incoming[b].push((a, ca));
    }
    for r in reservoirs {
        let exchange = dt_s * r.conductance_kg_s;
        let coefficient = exchange / body.cells[r.cell].capacity_kg;
        diagonal[r.cell] += coefficient;
        external_out[r.cell] += coefficient;
        rhs[r.cell] += exchange * r.saturation;
    }
    if diagonal.iter().any(|v| !v.is_finite() || *v <= 0.)
        || rhs
            .iter()
            .chain(&external_out)
            .any(|v| !v.is_finite() || *v < 0.)
        || incoming
            .iter()
            .flatten()
            .any(|(_, v)| !v.is_finite() || *v < 0.)
    {
        return Err("moisture sparse operator overflow");
    }
    let mass = crate::positive_transport::solve_mass(&rhs, &diagonal, &incoming, &external_out)
        .map_err(|_| "moisture sparse transport did not converge")?;
    Ok(mass
        .iter()
        .zip(&body.cells)
        .map(|(m, c)| m / c.capacity_kg)
        .collect())
}
fn dense(
    body: &Body,
    dt_s: f64,
    reservoirs: &[Reservoir],
    added: &[f64],
) -> Result<Vec<f64>, &'static str> {
    let n = body.cells.len();
    let mut matrix = vec![vec![0.; n]; n];
    let mut rhs: Vec<_> = body
        .cells
        .iter()
        .zip(added)
        .map(|(c, a)| c.water_kg + a)
        .collect();
    for (i, c) in body.cells.iter().enumerate() {
        matrix[i][i] = c.capacity_kg;
    }
    for link in &body.links {
        let [a, b] = link.cells;
        let exchange = dt_s * link.conductance_kg_s;
        matrix[a][a] += exchange;
        matrix[b][b] += exchange;
        matrix[a][b] -= exchange;
        matrix[b][a] -= exchange;
    }
    for r in reservoirs {
        let exchange = dt_s * r.conductance_kg_s;
        matrix[r.cell][r.cell] += exchange;
        rhs[r.cell] += exchange * r.saturation;
    }
    solve(matrix, rhs)
}
