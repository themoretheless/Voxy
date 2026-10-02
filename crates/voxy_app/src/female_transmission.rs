//! Directional thickness from actual posed surface intersections.
//! Artistic diffusion profile; open/ambiguous paths contribute no transmission.
use glam::{Mat4, Vec3};
use voxy_render::SceneVertex;

pub(crate) fn key_direction() -> Vec3 {
    if std::env::var("VOXY_FACE_DIAGNOSTIC_BACKLIGHT").as_deref() == Ok("1") {
        Vec3::new(0.5, 0.3, -1.).normalize()
    } else {
        Vec3::new(-0.5, 0.7, 1.).normalize()
    }
}
fn profile(thickness: f64) -> [f32; 3] {
    let d = thickness / 0.002;
    let terms = [
        ([0.233, 0.455, 0.649], 0.0064),
        ([0.1, 0.336, 0.344], 0.0484),
        ([0.118, 0.198, 0.0], 0.187),
        ([0.113, 0.007, 0.007], 0.567),
        ([0.358, 0.004, 0.0], 1.99),
        ([0.078, 0.0, 0.0], 7.41),
    ];
    std::array::from_fn(|c| {
        terms
            .iter()
            .map(|(rgb, v)| rgb[c] * (-d * d / v).exp())
            .sum::<f64>() as f32
    })
}
#[derive(Debug, Default)]
pub(crate) struct Cache {
    triangles: Vec<[usize; 3]>,
    surface: Option<physics::hair::TriangleMesh>,
}
pub(crate) fn apply(
    vertices: &mut [SceneVertex],
    indices: &[u32],
    rest: &[SceneVertex],
    body: usize,
    head: Mat4,
    cache: &mut Cache,
) {
    // Research prototype: user requested the 500-repository/RAG gate before further adoption.
    if std::env::var("VOXY_FACE_EXPERIMENTAL_TRANSMISSION").as_deref() != Ok("1")
        || std::env::var("VOXY_FACE_DIAGNOSTIC_NO_TRANSMISSION").as_deref() == Ok("1")
    {
        return;
    }
    let inverse = head.inverse();
    let points: Vec<[f64; 3]> = vertices[..body]
        .iter()
        .map(|v| v.position.map(f64::from))
        .collect();
    let triangles: Vec<[usize; 3]> = indices
        .chunks_exact(3)
        .filter(|ids| ids.iter().all(|i| (*i as usize) < body))
        .filter(|ids| {
            ids.iter().all(|i| {
                inverse
                    .transform_point3(Vec3::from_array(vertices[*i as usize].position))
                    .y
                    > 0.59
            })
        })
        .map(|ids| [ids[0] as usize, ids[1] as usize, ids[2] as usize])
        .collect();
    // Conservative bind-space neighbourhoods around both query regions.
    let local_triangles: Vec<_> = triangles
        .iter()
        .copied()
        .filter(|ids| {
            ids.iter().any(|i| {
                let p = Vec3::from_array(rest[*i].position);
                (p.x.abs() > 0.045 && p.y > 0.59 && p.y < 0.79)
                    || (p.x.abs() < 0.045 && p.y > 0.635 && p.y < 0.73 && p.z > 0.09)
            })
        })
        .collect();
    if cache.triangles != local_triangles || cache.surface.is_none() {
        let Ok(surface) = physics::hair::TriangleMesh::new(&points, &local_triangles) else {
            return;
        };
        cache.surface = Some(surface);
        cache.triangles = local_triangles;
    } else if cache.surface.as_mut().unwrap().refit(&points).is_err() {
        return;
    }
    let surface = cache.surface.as_ref().unwrap();
    let mut normals = vec![Vec3::ZERO; body];
    for ids in &triangles {
        let [a, b, c] = ids.map(|i| Vec3::from_array(vertices[i].position));
        let n = (b - a).cross(c - a);
        for &i in ids {
            normals[i] += n;
        }
    }
    let lights = [
        (key_direction(), 0.6),
        (Vec3::new(0.8, 0.2, 0.5).normalize(), 0.2),
    ];
    let mut accepted = 0usize;
    for (vertex, n) in vertices[..body].iter_mut().zip(normals) {
        let Some(normal) = n.try_normalize() else {
            continue;
        };
        let bind = inverse.transform_point3(Vec3::from_array(vertex.position));
        // Initial verified scope: ears and nasal surface. Oral surfaces are
        // open boundaries and need an explicit tissue-volume representation.
        if !((bind.x.abs() > 0.06 && bind.y > 0.62 && bind.y < 0.76)
            || (bind.x.abs() < 0.025 && bind.y > 0.66 && bind.y < 0.705))
        {
            continue;
        }
        for (light, weight) in lights {
            let incidence = normal.dot(light);
            if incidence >= 0. {
                continue;
            }
            let p = Vec3::from_array(vertex.position);
            let a = p - normal * 0.00002;
            let b = a + light * 0.02;
            let Ok(Some((fraction, _, exit_normal))) =
                surface.first_segment_hit(a.to_array().map(f64::from), b.to_array().map(f64::from))
            else {
                continue;
            };
            let exit = Vec3::new(
                exit_normal[0] as f32,
                exit_normal[1] as f32,
                exit_normal[2] as f32,
            );
            if exit.dot(light) < 0.1 {
                continue;
            }
            let thickness = fraction * 0.02 + 0.00002 * f64::from(-incidence);
            if thickness < 0.00005 {
                continue;
            }
            let attenuation = profile(thickness);
            for c in 0..3 {
                vertex.color[c] +=
                    0.18 * weight * (-incidence) * attenuation[c] * [0.72, 0.46, 0.34][c];
            }
            accepted += 1;
        }
    }
    if std::env::var("VOXY_FACE_DIAGNOSTIC_TRANSMISSION").as_deref() == Ok("1") {
        eprintln!("SKIN TRANSMISSION accepted_paths={accepted}");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spectral_profile_is_finite_and_decays_with_thickness() {
        let mut previous = profile(0.);
        for thickness in [0.0001, 0.0005, 0.001, 0.002, 0.005, 0.02] {
            let p = profile(thickness);
            for c in 0..3 {
                assert!(p[c].is_finite() && p[c] >= 0. && p[c] <= previous[c]);
            }
            assert!(p[0] >= p[1] && p[1] >= p[2]);
            previous = p;
        }
    }
}
