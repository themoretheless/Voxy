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
        return large(body, dt_s, reservoirs, added);
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
            large(&blocks[id], dt_s, &baths[id], &sources)?
        };
        for (&node, saturation) in nodes.iter().zip(solution) {
            output[node] = saturation;
        }
    }
    Ok(output)
}
// Connected acyclic blocks admit positive leaf elimination without iteration.
fn large(
    body: &Body,
    dt: f64,
    baths: &[Reservoir],
    added: &[f64],
) -> Result<Vec<f64>, &'static str> {
    let n = body.cells.len();
    if body
        .links
        .iter()
        .filter(|l| l.conductance_kg_s > 0.)
        .count()
        != n - 1
    {
        return sparse(body, dt, baths, added);
    }
    let mut graph = vec![Vec::new(); n];
    for l in &body.links {
        if l.conductance_kg_s == 0. {
            continue;
        }
        let w = dt * l.conductance_kg_s;
        if !w.is_finite() {
            return Err("moisture tree operator overflow");
        }
        let [a, b] = l.cells;
        graph[a].push((b, w));
        graph[b].push((a, w));
    }
    let mut parent = vec![usize::MAX; n];
    let mut weight = vec![0.; n];
    let mut order = vec![0];
    parent[0] = 0;
    let mut cursor = 0;
    while cursor < order.len() {
        let u = order[cursor];
        cursor += 1;
        for &(v, w) in &graph[u] {
            if parent[v] == usize::MAX {
                parent[v] = u;
                weight[v] = w;
                order.push(v);
            } else if v != parent[u] {
                return sparse(body, dt, baths, added);
            }
        }
    }
    if order.len() != n {
        return sparse(body, dt, baths, added);
    }
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
    if base.iter().chain(&rhs).any(|v| !v.is_finite()) {
        return Err("moisture tree operator overflow");
    }
    // Store only positive Schur contributions. Avoid subtracting two large
    // diagonal terms, which loses the small material capacity on stiff edges.
    for &u in order.iter().skip(1).rev() {
        let d = base[u] + weight[u];
        if !d.is_finite() {
            return Err("moisture tree operator overflow");
        }
        let ratio = weight[u] / d;
        let p = parent[u];
        base[p] += ratio * base[u];
        rhs[p] += ratio * rhs[u];
        if !base[p].is_finite() || !rhs[p].is_finite() {
            return Err("moisture tree operator overflow");
        }
    }
    let mut solution = vec![0.; n];
    solution[0] = rhs[0] / base[0];
    for &u in order.iter().skip(1) {
        let d = base[u] + weight[u];
        solution[u] = rhs[u] / d + (weight[u] / d) * solution[parent[u]];
    }
    Ok(solution)
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
    let mass = match crate::positive_transport::solve_mass_with_budget(
        &rhs,
        &diagonal,
        &incoming,
        &external_out,
        256,
    ) {
        Ok(mass) => mass,
        Err(_) => return krylov::saturations(body, dt_s, reservoirs, added),
    };
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

mod krylov;
