//! Closed boundary edge incidence and single-cycle vertex links.
use super::*;
use std::collections::BTreeSet;
impl TetraMesh {
    pub(super) fn validate_boundary_manifold(&self) -> Result<(), &'static str> {
        let mut edges = BTreeMap::<(usize, usize), (usize, i32)>::new();
        let mut links = BTreeMap::<usize, Vec<(usize, usize)>>::new();
        for &[a, b, c] in &self.boundary {
            for (u, v) in [(a, b), (b, c), (c, a)] {
                let incidence = edges.entry((u.min(v), u.max(v))).or_default();
                incidence.0 += 1;
                incidence.1 += if u < v { 1 } else { -1 };
                if incidence.0 > 2 {
                    return Err("nonmanifold tetrahedral boundary edge");
                }
            }
            for (vertex, edge) in [(a, (b, c)), (b, (c, a)), (c, (a, b))] {
                links.entry(vertex).or_default().push(edge);
            }
        }
        if edges
            .values()
            .any(|&(count, orientation)| count != 2 || orientation != 0)
        {
            return Err("nonmanifold tetrahedral boundary edge");
        }
        for link in links.into_values() {
            let mut adjacency = BTreeMap::<usize, Vec<usize>>::new();
            for (a, b) in link {
                adjacency.entry(a).or_default().push(b);
                adjacency.entry(b).or_default().push(a);
            }
            if adjacency.values().any(|neighbours| neighbours.len() != 2) {
                return Err("nonmanifold tetrahedral boundary vertex");
            }
            let Some(&first) = adjacency.keys().next() else {
                continue;
            };
            let mut pending = vec![first];
            let mut seen = BTreeSet::new();
            while let Some(vertex) = pending.pop() {
                if seen.insert(vertex) {
                    pending.extend(&adjacency[&vertex]);
                }
            }
            if seen.len() != adjacency.len() {
                return Err("nonmanifold tetrahedral boundary vertex");
            }
        }
        Ok(())
    }
}
