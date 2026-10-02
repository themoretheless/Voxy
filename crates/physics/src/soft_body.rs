//! Small CPU XPBD tetrahedral bodies. Discrete vertex/sphere contact; no self-contact.
use std::collections::BTreeSet;
type V = [f64; 3];
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: V, b: V) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
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
