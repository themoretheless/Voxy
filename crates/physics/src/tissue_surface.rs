//! Barycentric embedding of a render mesh in a coarse tetrahedral simulation.
//! Binding is O(surface vertices * tetrahedra); deformation is O(surface vertices).
type Point = [f64; 3];
fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
fn determinant(a: Point, b: Point, c: Point) -> f64 {
    a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
}
#[derive(Clone, Debug)]
struct Binding {
    indices: [usize; 4],
    weights: [f64; 4],
}
/// Immutable rest-space attachment. The simulation must retain its vertex order.
#[derive(Clone, Debug)]
pub struct EmbeddedSurface {
    bindings: Vec<Binding>,
    vertex_count: usize,
}
impl EmbeddedSurface {
    /// All render vertices must lie inside the tetrahedral mesh (boundary allowed).
    /// Shared-face ties select the first cell. No extrapolation or nearest-cell fallback.
    /// # Errors
    /// Rejects nonfinite positions, invalid or degenerate cells, and exterior vertices.
    pub fn bind(
        rest: &[Point],
        cells: &[[usize; 4]],
        surface: &[Point],
    ) -> Result<Self, &'static str> {
        if rest.is_empty()
            || cells.is_empty()
            || rest.iter().chain(surface).flatten().any(|x| !x.is_finite())
        {
            return Err("invalid embedding positions");
        }
        let mut prepared = Vec::with_capacity(cells.len());
        for &ids in cells {
            if ids.iter().any(|&i| i >= rest.len()) {
                return Err("invalid embedding index");
            }
            let a = sub(rest[ids[1]], rest[ids[0]]);
            let b = sub(rest[ids[2]], rest[ids[0]]);
            let c = sub(rest[ids[3]], rest[ids[0]]);
            let det = determinant(a, b, c);
            let scale = a
                .iter()
                .chain(&b)
                .chain(&c)
                .fold(0.0_f64, |s, x| s.max(x.abs()));
            if !det.is_finite() || scale == 0.0 || det.abs() <= 1e-12 * scale.powi(3) {
                return Err("degenerate embedding cell");
            }
            prepared.push((ids, a, b, c, det));
        }
        let mut bindings = Vec::with_capacity(surface.len());
        for &point in surface {
            let mut found = None;
            for &(indices, a, b, c, det) in &prepared {
                let q = sub(point, rest[indices[0]]);
                let mut weights = [
                    0.0,
                    determinant(q, b, c) / det,
                    determinant(a, q, c) / det,
                    determinant(a, b, q) / det,
                ];
                weights[0] = 1.0 - weights[1] - weights[2] - weights[3];
                if weights
                    .iter()
                    .all(|&w| w.is_finite() && (-1e-10..=1.0 + 1e-10).contains(&w))
                {
                    // Remove boundary roundoff without permitting visible extrapolation.
                    for w in &mut weights {
                        *w = w.clamp(0.0, 1.0);
                    }
                    let sum: f64 = weights.iter().sum();
                    for w in &mut weights {
                        *w /= sum;
                    }
                    found = Some(Binding { indices, weights });
                    break;
                }
            }
            bindings.push(found.ok_or("surface vertex outside tetrahedral mesh")?);
        }
        Ok(Self {
            bindings,
            vertex_count: rest.len(),
        })
    }
    /// Produces new render positions; caller-owned buffers remain unchanged on failure.
    /// Vertex order/count must match rest geometry. Normals must be recomputed by renderer.
    /// # Errors
    /// Rejects a changed vertex count, nonfinite positions, or arithmetic overflow.
    pub fn deform(&self, positions: &[Point]) -> Result<Vec<Point>, &'static str> {
        let mut output = vec![[0.0; 3]; self.bindings.len()];
        self.deform_into(positions, &mut output)?;
        Ok(output)
    }
    /// Updates an existing render buffer without allocating. Two passes ensure atomicity.
    /// # Errors
    /// Rejects wrong buffer sizes, nonfinite simulation positions or arithmetic overflow.
    /// The output remains unchanged on any error.
    pub fn deform_into(
        &self,
        positions: &[Point],
        output: &mut [Point],
    ) -> Result<(), &'static str> {
        if positions.len() != self.vertex_count
            || output.len() != self.bindings.len()
            || positions.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid deformed embedding positions");
        }
        let evaluate = |binding: &Binding| -> Point {
            std::array::from_fn(|axis| {
                (0..4)
                    .map(|i| positions[binding.indices[i]][axis] * binding.weights[i])
                    .sum()
            })
        };
        for binding in &self.bindings {
            if evaluate(binding).iter().any(|x| !x.is_finite()) {
                return Err("embedding overflow");
            }
        }
        for (binding, point) in self.bindings.iter().zip(output) {
            *point = evaluate(binding);
        }
        Ok(())
    }
}
