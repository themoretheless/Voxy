//! Linear sphere sweep against both sides and all features of stationary triangles.
use super::{SurfaceFilm, dot, norm, sub};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphereFilmHit {
    pub cell: usize,
    /// Fraction of the supplied path/query interval, in [0,1].
    pub time: f64,
    /// Closest point on the stationary triangle.
    pub point: [f64; 3],
    /// Unit normal from the triangle toward the sphere center.
    pub normal: [f64; 3],
    /// Initial overlap depth; zero for an ordinary entry contact.
    pub penetration: f64,
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn inside(p: [f64; 3], tri: &[[f64; 3]; 3], n: [f64; 3]) -> Result<bool, &'static str> {
    for k in 0..3 {
        let edge = sub(tri[(k + 1) % 3], tri[k]);
        let value = dot(cross(edge, sub(p, tri[k])), n);
        if !value.is_finite() {
            return Err("sphere film geometry overflow");
        }
        if value < -64.0 * f64::EPSILON * dot(edge, edge) {
            return Ok(false);
        }
    }
    Ok(true)
}
// Stable roots of |w+t*v|²=r², scaled before forming the discriminant.
fn entries(w: [f64; 3], v: [f64; 3], r: f64) -> Result<Vec<f64>, &'static str> {
    let scale = w.iter().chain(&v).fold(r, |s, x| s.max(x.abs()));
    let w = w.map(|x| x / scale);
    let v = v.map(|x| x / scale);
    let r = r / scale;
    let a = dot(v, v);
    let b = dot(w, v);
    let c = dot(w, w) - r * r;
    if ![a, b, c].iter().all(|x| x.is_finite()) {
        return Err("sphere film sweep overflow");
    }
    let mut result = Vec::new();
    if c <= 0.0 {
        result.push(0.0);
    }
    if a == 0.0 {
        return Ok(result);
    }
    let disc = b * b - a * c;
    if disc < -64.0 * f64::EPSILON * (b * b + (a * c).abs()) {
        return Ok(result);
    }
    let q = -b - disc.max(0.0).sqrt().copysign(b);
    let t = if q == 0.0 { -b / a } else { (q / a).min(c / q) };
    if t >= 0.0 && t <= 1.0 {
        result.push(t);
    }
    Ok(result)
}
impl SurfaceFilm {
    /// Earliest contact of a finite sphere following a linear center path.
    /// Faces, edge cylinders and vertex spheres are all tested, on both sides.
    /// Initial touching/overlap returns time zero. Ties select the lowest cell;
    /// initial overlap selects the deepest contact. No fluid state is changed.
    /// This queries the substrate geometry, not the film's changing free surface.
    pub fn first_sphere_hit(
        &self,
        start: [f64; 3],
        end: [f64; 3],
        radius: f64,
    ) -> Result<Option<SphereFilmHit>, &'static str> {
        self.sphere_hit_impl(start, end, radius, true, false)
    }
    pub(crate) fn first_closing_sphere_hit(
        &self,
        start: [f64; 3],
        end: [f64; 3],
        radius: f64,
    ) -> Result<Option<SphereFilmHit>, &'static str> {
        self.sphere_hit_impl(start, end, radius, true, true)
    }
    fn sphere_hit_impl(
        &self,
        start: [f64; 3],
        end: [f64; 3],
        radius: f64,
        prune: bool,
        closing_only: bool,
    ) -> Result<Option<SphereFilmHit>, &'static str> {
        if !radius.is_finite() || radius <= 0.0 || start.iter().chain(&end).any(|x| !x.is_finite())
        {
            return Err("invalid sphere film sweep");
        }
        let direction = sub(end, start);
        if direction.iter().any(|x| !x.is_finite()) {
            return Err("sphere film sweep overflow");
        }
        // Every center on the linear path lies in its endpoint AABB. Expanding
        // that box by radius contains the full sphere sweep, including features.
        // A coordinate-scale margin makes the rejection conservative at roundoff.
        let scale = start.iter().chain(&end).fold(radius, |s, x| s.max(x.abs()));
        let padding = radius + 64.0 * f64::EPSILON * scale;
        let lower: [f64; 3] = std::array::from_fn(|k| start[k].min(end[k]) - padding);
        let upper: [f64; 3] = std::array::from_fn(|k| start[k].max(end[k]) + padding);
        if prune && lower.iter().chain(&upper).any(|x| !x.is_finite()) {
            return Err("sphere film sweep bounds overflow");
        }
        let mut nearest: Option<SphereFilmHit> = None;
        for (cell, tri) in self.geometry.iter().enumerate() {
            if prune
                && (0..3).any(|k| {
                    tri.iter().all(|p| p[k] < lower[k]) || tri.iter().all(|p| p[k] > upper[k])
                })
            {
                continue;
            }
            let n = self.normals[cell];
            let mut consider = |time: f64, point: [f64; 3]| -> Result<(), &'static str> {
                let center = std::array::from_fn(|k| start[k] + time * direction[k]);
                let offset = sub(center, point);
                let distance = norm(offset);
                if !distance.is_finite() || point.iter().any(|x| !x.is_finite()) {
                    return Err("sphere film contact overflow");
                }
                let normal = if distance > 0.0 {
                    offset.map(|x| x / distance)
                } else if dot(direction, n) > 0.0 {
                    n.map(|x| -x)
                } else {
                    n
                };
                let penetration = if time == 0.0 {
                    (radius - distance).max(0.0)
                } else {
                    0.0
                };
                if closing_only
                    && penetration <= 64.0 * f64::EPSILON * radius
                    && dot(direction, normal) >= 0.0
                {
                    return Ok(());
                }
                let hit = SphereFilmHit {
                    cell,
                    time,
                    point,
                    normal,
                    penetration,
                };
                if nearest.is_none_or(|old| {
                    time < old.time || (time == old.time && penetration > old.penetration)
                }) {
                    nearest = Some(hit);
                }
                Ok(())
            };
            let signed = dot(sub(start, tri[0]), n);
            let speed = dot(direction, n);
            if !signed.is_finite() || !speed.is_finite() {
                return Err("sphere film plane overflow");
            }
            let projection = std::array::from_fn(|k| start[k] - signed * n[k]);
            if signed.abs() <= radius && inside(projection, tri, n)? {
                consider(0.0, projection)?;
            }
            for side in [-1.0, 1.0] {
                if side * speed < 0.0 {
                    let time = (side * radius - signed) / speed;
                    if time >= 0.0 && time <= 1.0 {
                        let point = std::array::from_fn(|k| {
                            start[k] + time * direction[k] - side * radius * n[k]
                        });
                        if inside(point, tri, n)? {
                            consider(time, point)?;
                        }
                    }
                }
            }
            for k in 0..3 {
                let a = tri[k];
                for time in entries(sub(start, a), direction, radius)? {
                    consider(time, a)?;
                }
                let edge = sub(tri[(k + 1) % 3], a);
                let length = norm(edge);
                let unit = edge.map(|x| x / length);
                let offset = sub(start, a);
                let along = dot(offset, unit);
                let along_speed = dot(direction, unit);
                let w = std::array::from_fn(|i| offset[i] - along * unit[i]);
                let v = std::array::from_fn(|i| direction[i] - along_speed * unit[i]);
                for time in entries(w, v, radius)? {
                    let position = along + time * along_speed;
                    if position >= 0.0 && position <= length {
                        consider(time, std::array::from_fn(|i| a[i] + position * unit[i]))?;
                    }
                }
            }
        }
        Ok(nearest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface_film::Material;
    #[test]
    fn swept_bounds_match_unfiltered_face_edge_vertex_and_overlap_queries() {
        for scale in [0.001, 1.0, 1000.0] {
            for translation in [[0.0; 3], [1e6, -1e6, 1e6]] {
                let transform = |p: [f64; 3]| {
                    [
                        translation[0] + scale * p[2],
                        translation[1] + scale * p[0],
                        translation[2] + scale * p[1],
                    ]
                };
                let mut points = Vec::new();
                for z in 0..=4 {
                    for x in 0..=16 {
                        points.push(transform([
                            -1.0 + x as f64 / 8.0,
                            0.0,
                            -0.3 + z as f64 * 0.15,
                        ]));
                    }
                }
                let mut triangles = Vec::new();
                for z in 0..4 {
                    for x in 0..16 {
                        let a = z * 17 + x;
                        triangles.extend([[a, a + 1, a + 18], [a, a + 18, a + 17]]);
                    }
                }
                let film = SurfaceFilm::new(&points, triangles, Material::default()).unwrap();
                let mut rng = 17_u64;
                let mut sample = || {
                    rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                    (rng >> 11) as f64 / ((1_u64 << 53) as f64)
                };
                for i in 0..512 {
                    let x = -1.2 + 2.4 * sample();
                    let z = -0.5 + sample();
                    let y = -0.2 + 0.4 * sample();
                    let a = [x, y, z];
                    let b = match i % 4 {
                        0 => [x, -y, z],
                        1 => a,
                        2 => [
                            -1.2 + 2.4 * sample(),
                            -0.2 + 0.4 * sample(),
                            -0.5 + sample(),
                        ],
                        _ => [x, 0.1, z],
                    };
                    let r = (0.005 + 0.08 * sample()) * scale;
                    let a = transform(a);
                    let b = transform(b);
                    assert_eq!(
                        film.sphere_hit_impl(a, b, r, true, false).unwrap(),
                        film.sphere_hit_impl(a, b, r, false, false).unwrap(),
                        "scale={scale}, translation={translation:?}, query={i}"
                    );
                }
            }
        }
    }
}
