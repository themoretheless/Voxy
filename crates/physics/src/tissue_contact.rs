//! Discrete triangle/sphere contact used by volumetric tissues.
type V = [f64; 3];
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: V, b: V) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn closest(p: [V; 3], q: V) -> ([f64; 3], V) {
    let a = sub(p[1], p[0]);
    let b = sub(p[2], p[0]);
    let v = sub(q, p[0]);
    let aa = dot(a, a);
    let ab = dot(a, b);
    let bb = dot(b, b);
    let av = dot(a, v);
    let bv = dot(b, v);
    let det = aa * bb - ab * ab;
    if det > 1e-30 {
        let u = (bb * av - ab * bv) / det;
        let v = (aa * bv - ab * av) / det;
        if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
            let weights = [1.0 - u - v, u, v];
            return (
                weights,
                std::array::from_fn(|k| (0..3).map(|i| p[i][k] * weights[i]).sum()),
            );
        }
    }
    let mut best = ([1., 0., 0.], p[0]);
    let mut distance = dot(sub(q, p[0]), sub(q, p[0]));
    for (i, j) in [(0, 1), (1, 2), (2, 0)] {
        let edge = sub(p[j], p[i]);
        let length = dot(edge, edge);
        let t = if length > 0.0 {
            (dot(sub(q, p[i]), edge) / length).clamp(0., 1.)
        } else {
            0.0
        };
        let point = std::array::from_fn(|k| p[i][k] + edge[k] * t);
        let d = dot(sub(q, point), sub(q, point));
        if d < distance {
            distance = d;
            let mut w = [0.; 3];
            w[i] = 1. - t;
            w[j] = t;
            best = (w, point);
        }
    }
    best
}
pub(crate) fn contact(p: [V; 3], center: V, radius: f64) -> Option<([f64; 3], V, f64)> {
    let (weights, point) = closest(p, center);
    let delta = sub(point, center);
    let distance = dot(delta, delta).sqrt();
    if distance >= radius {
        return None;
    }
    let normal = if distance > 1e-12 {
        delta.map(|x| x / distance)
    } else {
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let length = dot(n, n).sqrt();
        if length <= 1e-15 {
            return None;
        }
        n.map(|x| x / length)
    };
    Some((weights, normal, radius - distance))
}
pub(crate) fn project(
    positions: &mut [V],
    inverse_mass: &[f64],
    faces: &[[usize; 3]],
    spheres: &[crate::strand::SphereCollider],
    margin: f64,
    normal_multipliers: &mut [f64],
) -> Result<(), &'static str> {
    for (face_index, face) in faces.iter().enumerate() {
        for (sphere_index, sphere) in spheres.iter().enumerate() {
            if let Some((weights, normal, penetration)) = contact(
                face.map(|i| positions[i]),
                sphere.center,
                sphere.radius + margin,
            ) {
                if penetration < 1e-10 {
                    continue;
                }
                let denominator: f64 = (0..3)
                    .map(|j| inverse_mass[face[j]] * weights[j] * weights[j])
                    .sum();
                if denominator <= 1e-20 {
                    return Err("surface contact conflicts with pins");
                }
                normal_multipliers[face_index * spheres.len() + sphere_index] +=
                    penetration / denominator;
                for j in 0..3 {
                    for k in 0..3 {
                        positions[face[j]][k] +=
                            normal[k] * penetration * inverse_mass[face[j]] * weights[j]
                                / denominator;
                    }
                }
            }
        }
    }
    Ok(())
}
