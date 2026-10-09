//! Geometric authoring, before rod material and stress-free lengths exist.
use super::*;

impl TriangleMesh {
    /// Fit a guide outside a closed collider while preserving its root and point count.
    /// This changes authored shape/arc length, never a running rod's rest state.
    /// Failure returns no partially repaired curve. Strand/strand admission is separate.
    pub fn fit_authored_guide(&self, points: &[V], margin: f64) -> Result<Vec<V>, &'static str> {
        if points.len() < 2
            || !points.iter().copied().all(finite)
            || points.windows(2).any(|p| len(sub(p[1],p[0])) <= 1e-12)
            || !margin.is_finite()
            || margin <= 0.
        {
            return Err("invalid authored guide fitting");
        }
        self.feature_normals
            .as_ref()
            .ok_or("guide fitting requires closed feature normals")?;
        let mut curve = points.to_vec();
        for _ in 0..128 {
            let mut delta = vec![[0.; 3]; curve.len()];
            let mut count = vec![0usize; curve.len()];
            let mut unresolved = false;
            let mut constrain = |i: usize, t: f64, normal: V, depth: f64| {
                if depth <= 1e-9 {
                    return;
                }
                unresolved = true;
                let weights = [if i == 0 { 0. } else { 1. - t }, t];
                let denominator = weights.iter().map(|w| w * w).sum::<f64>();
                if denominator <= 0. {
                    return;
                }
                for (point, weight) in [(i, weights[0]), (i + 1, weights[1])] {
                    if weight > 0. {
                        delta[point] = add(delta[point], mul(normal, depth * weight / denominator));
                        count[point] += 1;
                    }
                }
            };
            for i in 1..curve.len() {
                let (signed, normal) = self.signed_distance_closed(curve[i])?;
                constrain(i - 1, 1., normal, margin - signed);
            }
            for i in 0..curve.len() - 1 {
                let start = if i == 0 { 0.15 } else { 0. };
                let a = add(mul(curve[i], 1. - start), mul(curve[i + 1], start));
                let b = curve[i + 1];
                let min = std::array::from_fn(|k| a[k].min(b[k]) - margin);
                let max = std::array::from_fn(|k| a[k].max(b[k]) + margin);
                let mut candidates = Vec::new();
                self.query(min, max, 0, &mut candidates);
                let mut cuts = vec![start, 1.];
                for id in candidates {
                    let (local, p, q) = segment_triangle(a, b, &self.triangles[id]);
                    let t = start + (1. - start) * local;
                    let displacement = sub(p, q);
                    let distance = len(displacement);
                    if distance < 1e-10 {
                        cuts.push(t);
                    }
                    if distance < margin {
                        let fallback = self.contact_normal(id, p);
                        let signed = if dot(displacement, fallback) < 0. {
                            -distance
                        } else {
                            distance
                        };
                        constrain(
                            i,
                            t,
                            distance_gradient(displacement, distance, signed, fallback, 1e-10),
                            margin - signed,
                        );
                    }
                }
                cuts.sort_by(f64::total_cmp);
                cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
                for interval in cuts.windows(2) {
                    let t = (interval[0] + interval[1]) * 0.5;
                    let p = add(mul(curve[i], 1. - t), mul(curve[i + 1], t));
                    let (signed, normal) = self.signed_distance_closed(p)?;
                    if signed < 0. {
                        constrain(i, t, normal, margin - signed);
                    }
                }
            }
            if !unresolved {
                return Ok(curve);
            }
            for i in 1..curve.len() {
                if count[i] > 0 {
                    curve[i] = add(curve[i], mul(delta[i], 1. / count[i] as f64));
                }
                if !finite(curve[i]) {
                    return Err("authored guide fitting overflow");
                }
            }
        }
        Err("authored guide fitting did not converge")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fitting_routes_crossing_curve_before_material_creation() {
        let mut mesh = TriangleMesh::new(
            &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
        )
        .unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        let original = [[-0.5, 0.2, 0.2], [-0.1, 0.2, 0.2], [0.9, 0.2, 0.2]];
        let fitted = mesh.fit_authored_guide(&original, 0.001).unwrap();
        assert_eq!(fitted[0], original[0]);
        assert_eq!(fitted.len(), original.len());
        for p in fitted.windows(2) {
            assert!(mesh.first_segment_hit(p[0], p[1]).unwrap().is_none());
        }
        for p in fitted.iter().skip(1) {
            assert!(mesh.signed_distance_closed(*p).unwrap().0 >= 0.001 - 1e-9);
        }
        assert!(mesh.fit_authored_guide(&original, f64::NAN).is_err());
    }
}
