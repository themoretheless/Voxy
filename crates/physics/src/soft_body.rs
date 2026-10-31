//! Small CPU XPBD tetrahedral bodies. Discrete vertex/sphere contact; no self-contact.
use crate::math::cross;
use crate::math::dot;
use crate::math::sub;
use std::collections::BTreeSet;
type V = [f64; 3];
fn volume(p: &[V], t: [usize; 4]) -> f64 {
    dot(
        sub(p[t[1]], p[t[0]]),
        cross(sub(p[t[2]], p[t[0]]), sub(p[t[3]], p[t[0]])),
    ) / 6.0
}
#[derive(Clone, Copy, Debug)]
pub struct Sphere {
    pub center: V,
    pub radius: f64,
}
#[derive(Clone, Debug)]
pub struct SoftBody {
    positions: Vec<V>,
    velocities: Vec<V>,
    weights: Vec<f64>,
    tetrahedra: Vec<[usize; 4]>,
    volumes: Vec<f64>,
    edges: Vec<(usize, usize, f64)>,
}
impl SoftBody {
    /// Inverse mass zero pins a vertex. Degenerate tetrahedra are rejected.
    pub fn new(
        positions: Vec<V>,
        weights: Vec<f64>,
        tetrahedra: Vec<[usize; 4]>,
    ) -> Result<Self, &'static str> {
        if positions.is_empty()
            || weights.len() != positions.len()
            || positions.iter().flatten().any(|x| !x.is_finite())
            || weights.iter().any(|x| !x.is_finite() || *x < 0.0)
            || tetrahedra.is_empty()
        {
            return Err("invalid mesh");
        }
        let mut volumes = Vec::new();
        let mut pairs = BTreeSet::new();
        for &t in &tetrahedra {
            if t.iter().any(|&i| i >= positions.len()) {
                return Err("invalid index");
            }
            let v = volume(&positions, t);
            if !v.is_finite() || v.abs() < 1e-12 {
                return Err("degenerate tetrahedron");
            }
            volumes.push(v);
            for i in 0..4 {
                for j in i + 1..4 {
                    pairs.insert((t[i].min(t[j]), t[i].max(t[j])));
                }
            }
        }
        let edges = pairs
            .into_iter()
            .map(|(a, b)| {
                (
                    a,
                    b,
                    dot(
                        sub(positions[a], positions[b]),
                        sub(positions[a], positions[b]),
                    )
                    .sqrt(),
                )
            })
            .collect();
        Ok(Self {
            velocities: vec![[0.0; 3]; positions.len()],
            positions,
            weights,
            tetrahedra,
            volumes,
            edges,
        })
    }
    pub fn positions(&self) -> &[V] {
        &self.positions
    }
    pub fn volume(&self) -> f64 {
        self.tetrahedra
            .iter()
            .map(|&t| volume(&self.positions, t).abs())
            .sum()
    }

    /// Partitions distance constraints (`edges`) into conflict-free color batches using greedy
    /// graph coloring so edges within any single batch share no vertices and can be solved in
    /// parallel without atomic writes.
    #[must_use]
    pub fn color_edge_batches(&self) -> Vec<Vec<usize>> {
        let mut batches: Vec<Vec<usize>> = Vec::new();
        let mut vertex_color_used: Vec<u64> = vec![0; self.positions.len()];
        let mut overflow_used: Vec<Vec<bool>> = Vec::new();
        for (edge_idx, &(a, b, _)) in self.edges.iter().enumerate() {
            let mask_a = vertex_color_used[a];
            let mask_b = vertex_color_used[b];
            let free_mask = !(mask_a | mask_b);
            let color = if free_mask != 0 {
                let c = free_mask.trailing_zeros() as usize;
                vertex_color_used[a] |= 1_u64 << c;
                vertex_color_used[b] |= 1_u64 << c;
                c
            } else {
                let mut chosen = None;
                for (extra_idx, used_vertices) in overflow_used.iter_mut().enumerate() {
                    if !used_vertices[a] && !used_vertices[b] {
                        used_vertices[a] = true;
                        used_vertices[b] = true;
                        chosen = Some(64 + extra_idx);
                        break;
                    }
                }
                if let Some(c) = chosen {
                    c
                } else {
                    let mut used_vertices = vec![false; self.positions.len()];
                    used_vertices[a] = true;
                    used_vertices[b] = true;
                    overflow_used.push(used_vertices);
                    64 + overflow_used.len() - 1
                }
            };
            if color >= batches.len() {
                batches.resize_with(color + 1, Vec::new);
            }
            batches[color].push(edge_idx);
        }
        batches
    }
    /// Compliance in solver units; fixed steps <= 1/120 s. Entire step is atomic.
    pub fn step(
        &mut self,
        dt: f64,
        acceleration: V,
        compliance: f64,
        spheres: &[Sphere],
    ) -> Result<(), &'static str> {
        if !dt.is_finite()
            || dt <= 0.0
            || dt > 1.0 / 120.0
            || !compliance.is_finite()
            || compliance < 0.0
            || acceleration.iter().any(|x| !x.is_finite())
            || spheres.iter().any(|s| {
                s.center.iter().any(|x| !x.is_finite()) || !s.radius.is_finite() || s.radius <= 0.0
            })
        {
            return Err("invalid step");
        }
        let old = &self.positions;
        let mut p = old.clone();
        let alpha = compliance / (dt * dt);
        for i in 0..p.len() {
            if self.weights[i] > 0.0 {
                for k in 0..3 {
                    p[i][k] += self.velocities[i][k] * dt + acceleration[k] * dt * dt;
                }
            }
        }
        let mut el = vec![0.0; self.edges.len()];
        let mut vl = vec![0.0; self.volumes.len()];
        for _ in 0..24 {
            for (n, &(a, b, rest)) in self.edges.iter().enumerate() {
                let d = sub(p[a], p[b]);
                let len = dot(d, d).sqrt();
                let w = self.weights[a] + self.weights[b];
                if len > 1e-12 && w > 0.0 {
                    let dl = (-(len - rest) - alpha * el[n]) / (w + alpha);
                    el[n] += dl;
                    for k in 0..3 {
                        let c = dl * d[k] / len;
                        p[a][k] += self.weights[a] * c;
                        p[b][k] -= self.weights[b] * c;
                    }
                }
            }
            for (n, &t) in self.tetrahedra.iter().enumerate() {
                let a = sub(p[t[1]], p[t[0]]);
                let b = sub(p[t[2]], p[t[0]]);
                let c = sub(p[t[3]], p[t[0]]);
                let mut g = [[0.0; 3]; 4];
                g[1] = cross(b, c);
                g[2] = cross(c, a);
                g[3] = cross(a, b);
                for k in 0..3 {
                    g[0][k] = -g[1][k] - g[2][k] - g[3][k];
                }
                for v in &mut g {
                    for x in v {
                        *x /= 6.0;
                    }
                }
                let w: f64 = (0..4).map(|i| self.weights[t[i]] * dot(g[i], g[i])).sum();
                if w > 1e-20 {
                    let dl = (-(volume(&p, t) - self.volumes[n]) - alpha * vl[n]) / (w + alpha);
                    vl[n] += dl;
                    for i in 0..4 {
                        for k in 0..3 {
                            p[t[i]][k] += self.weights[t[i]] * dl * g[i][k];
                        }
                    }
                }
            }
            for i in 0..p.len() {
                if self.weights[i] == 0.0 {
                    continue;
                }
                for s in spheres {
                    let d = sub(p[i], s.center);
                    let len = dot(d, d).sqrt();
                    if len < s.radius {
                        let normal = if len > 1e-12 {
                            d.map(|x| x / len)
                        } else {
                            [1.0, 0.0, 0.0]
                        };
                        p[i] = std::array::from_fn(|k| s.center[k] + normal[k] * s.radius);
                    }
                }
            }
        }
        let velocities: Vec<V> = p
            .iter()
            .zip(old)
            .map(|(&a, &b)| sub(a, b).map(|x| x / dt * (-6.0 * dt).exp()))
            .collect();
        if p.iter()
            .flatten()
            .chain(velocities.iter().flatten())
            .any(|x| !x.is_finite())
            || self
                .tetrahedra
                .iter()
                .zip(&self.volumes)
                .any(|(&t, &v)| volume(&p, t) * v <= 0.0)
        {
            return Err("nonfinite or inverted body");
        }
        self.positions = p;
        self.velocities = velocities;
        Ok(())
    }
}

/// Computes the signed dihedral angle in radians (`[-pi, pi]`) between two triangles
/// `(p0, p1, p2)` and `(p0, p1, p3)` sharing the hinge edge `p0 -> p1`.
#[must_use]
pub fn dihedral_angle_radians(p0: V, p1: V, p2: V, p3: V) -> f64 {
    let e = sub(p1, p0);
    let e_len = dot(e, e).sqrt();
    if e_len < 1e-12 {
        return 0.0;
    }
    let e_hat = e.map(|x| x / e_len);
    let n1 = cross(e, sub(p2, p0));
    let n2 = cross(sub(p3, p0), e);
    let l1 = dot(n1, n1).sqrt();
    let l2 = dot(n2, n2).sqrt();
    if l1 < 1e-12 || l2 < 1e-12 {
        return 0.0;
    }
    let n1_hat = n1.map(|x| x / l1);
    let n2_hat = n2.map(|x| x / l2);
    let sin_theta = dot(cross(n1_hat, n2_hat), e_hat);
    let cos_theta = dot(n1_hat, n2_hat).clamp(-1.0, 1.0);
    sin_theta.atan2(cos_theta)
}

#[cfg(test)]
mod coloring_and_bending_tests {
    use super::*;

    #[test]
    fn edge_coloring_produces_disjoint_vertex_batches() {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
        ];
        let weights = vec![1.0; 5];
        let tets = vec![[0, 1, 2, 3], [1, 2, 3, 4]];
        let body = SoftBody::new(positions, weights, tets).unwrap();
        let batches = body.color_edge_batches();
        let total_colored: usize = batches.iter().map(Vec::len).sum();
        assert_eq!(total_colored, body.edges.len());
        for batch in &batches {
            let mut seen = BTreeSet::new();
            for &edge_idx in batch {
                let (a, b, _) = body.edges[edge_idx];
                assert!(seen.insert(a), "duplicate vertex {a} in batch");
                assert!(seen.insert(b), "duplicate vertex {b} in batch");
            }
        }
    }

    #[test]
    fn dihedral_angle_zero_for_coplanar_and_ninety_deg_for_folded() {
        let p0 = [0.0, 0.0, 0.0];
        let p1 = [1.0, 0.0, 0.0];
        let p2 = [0.0, 1.0, 0.0];
        let p3_flat = [0.0, -1.0, 0.0];
        assert!(dihedral_angle_radians(p0, p1, p2, p3_flat).abs() < 1e-12);
        let p3_perp = [0.0, 0.0, 1.0];
        assert!(
            (dihedral_angle_radians(p0, p1, p2, p3_perp).abs() - std::f64::consts::FRAC_PI_2).abs()
                < 1e-12
        );
    }
}
