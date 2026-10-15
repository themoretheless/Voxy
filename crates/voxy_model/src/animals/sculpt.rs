//! Original section-volume union. Removes intersecting loft seams before grooming.
use super::{AnatomySurface, BreedAnatomy, surface};
use glam::Vec3;
use std::collections::HashMap;
struct Segment {
    a: Vec3,
    delta: Vec3,
    inverse: f32,
    right: Vec3,
    up: Vec3,
    radii: [f32; 4],
}
struct Volume {
    segments: Vec<Segment>,
    min: Vec3,
    max: Vec3,
}
impl Volume {
    fn distance(&self, p: Vec3) -> f32 {
        self.segments.iter().fold(f32::INFINITY, |best, s| {
            let t = (p - s.a).dot(s.delta) * s.inverse;
            let u = t.clamp(0., 1.);
            let q = p - (s.a + s.delta * u);
            let w = s.radii[0] + (s.radii[2] - s.radii[0]) * u;
            let d = s.radii[1] + (s.radii[3] - s.radii[1]) * u;
            // Rounded elliptical ends overlap neighbouring sections. Capped
            // discs left wedges and internal plates at every curved junction.
            let axial = (t - u).abs() * s.delta.length();
            let v = (Vec3::new(q.dot(s.right) / w, q.dot(s.up) / d, axial / w.min(d)).length()
                - 1.)
                * w.min(d);
            best.min(v)
        })
    }
}
struct Field {
    volumes: Vec<Volume>,
    blend: f32,
    sockets: Vec<(Vec3, Vec3, Vec3)>,
}
impl Field {
    fn value(&self, p: Vec3) -> f32 {
        let body = self.volumes.iter().fold(f32::INFINITY, |best, v| {
            let bound = (v.min - p).max(p - v.max).max(Vec3::ZERO).length();
            if bound > best.max(0.) + self.blend {
                return best;
            }
            let d = v.distance(p);
            if !best.is_finite() {
                return d;
            }
            let h = (0.5 + 0.5 * (d - best) / self.blend).clamp(0., 1.);
            d * (1. - h) + best * h - self.blend * h * (1. - h)
        });
        self.sockets
            .iter()
            .fold(body, |distance, (center, radii, normal)| {
                let right = Vec3::new(normal.z, 0.0, -normal.x).normalize();
                let up = normal.cross(right);
                let q = p - *center;
                let local = Vec3::new(q.dot(right), q.dot(up), q.dot(*normal));
                let cut = (local / *radii).length() - 1.0;
                distance.max(-cut * radii.min_element())
            })
    }
    fn normal(&self, p: Vec3) -> Vec3 {
        let e = 0.00025;
        Vec3::new(
            self.value(p + Vec3::X * e) - self.value(p - Vec3::X * e),
            self.value(p + Vec3::Y * e) - self.value(p - Vec3::Y * e),
            self.value(p + Vec3::Z * e) - self.value(p - Vec3::Z * e),
        )
        .normalize_or_zero()
    }
}
pub(super) fn build(a: BreedAnatomy) -> Result<AnatomySurface, String> {
    let source = surface::build(a)?;
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let volumes = source
        .volumes
        .iter()
        .map(|rings| {
            let mut lo = Vec3::splat(f32::INFINITY);
            let mut hi = Vec3::splat(f32::NEG_INFINITY);
            for r in rings {
                let radius = r.w.max(r.d);
                lo = lo.min(r.p - Vec3::splat(radius));
                hi = hi.max(r.p + Vec3::splat(radius));
            }
            min = min.min(lo);
            max = max.max(hi);
            let segments = rings
                .windows(2)
                .map(|r| {
                    let delta = r[1].p - r[0].p;
                    let axis = delta.normalize();
                    let right = (Vec3::X - axis * axis.x).normalize();
                    Segment {
                        a: r[0].p,
                        delta,
                        inverse: delta.length_squared().recip(),
                        right,
                        up: axis.cross(right),
                        radii: [r[0].w, r[0].d, r[1].w, r[1].d],
                    }
                })
                .collect();
            Volume {
                segments,
                min: lo,
                max: hi,
            }
        })
        .collect();
    let step = a.shoulder_height / 90.;
    let margin = step * 3.;
    min -= Vec3::splat(margin);
    max += Vec3::splat(margin);
    let dims = ((max - min) / step).ceil().as_uvec3() + glam::UVec3::ONE;
    let count = dims.x as usize * dims.y as usize * dims.z as usize;
    if count > 8_000_000 {
        return Err("sculpt grid budget exceeded".into());
    }
    let head_y = a.shoulder_height + a.neck_length * 0.55;
    let head_z = a.trunk_length * 0.36 + a.neck_length * 0.48;
    let skull = a.head_length - a.muzzle_length;
    let sockets = [-1.0, 1.0]
        .into_iter()
        .map(|side| {
            let normal = Vec3::new(side * 0.68, 0.07, 0.73).normalize();
            let center = Vec3::new(
                side * a.skull_width * 0.32,
                head_y + skull * 0.10,
                head_z + skull * 0.85,
            );
            let r = a.skull_width * 0.072;
            (center, Vec3::new(r * 1.08, r * 0.62, r * 0.75), normal)
        })
        .collect();
    let field = Field {
        volumes,
        blend: a.shoulder_height * 0.018,
        sockets,
    };
    let index = |x: u32, y: u32, z: u32| (x + dims.x * (y + dims.y * z)) as usize;
    let point = |i: usize| {
        min + Vec3::new(
            (i % dims.x as usize) as f32,
            ((i / dims.x as usize) % dims.y as usize) as f32,
            (i / (dims.x * dims.y) as usize) as f32,
        ) * step
    };
    let values: Vec<_> = (0..count).map(|i| field.value(point(i))).collect();
    let mut out = AnatomySurface::default();
    let mut edges = HashMap::new();
    let tetra = [
        [0, 1, 3, 7],
        [0, 3, 2, 7],
        [0, 2, 6, 7],
        [0, 6, 4, 7],
        [0, 4, 5, 7],
        [0, 5, 1, 7],
    ];
    for z in 0..dims.z - 1 {
        for y in 0..dims.y - 1 {
            for x in 0..dims.x - 1 {
                let cube = [
                    index(x, y, z),
                    index(x + 1, y, z),
                    index(x, y + 1, z),
                    index(x + 1, y + 1, z),
                    index(x, y, z + 1),
                    index(x + 1, y, z + 1),
                    index(x, y + 1, z + 1),
                    index(x + 1, y + 1, z + 1),
                ];
                if cube.iter().all(|&i| values[i] > 0.) || cube.iter().all(|&i| values[i] <= 0.) {
                    continue;
                }
                for t in tetra {
                    let mut inside = Vec::new();
                    let mut outside = Vec::new();
                    for k in t {
                        let i = cube[k];
                        if values[i] <= 0. {
                            inside.push(i);
                        } else {
                            outside.push(i);
                        }
                    }
                    if inside.is_empty() || outside.is_empty() {
                        continue;
                    }
                    let mut crossing = |a: usize, b: usize| {
                        *edges.entry((a.min(b), a.max(b))).or_insert_with(|| {
                            let p = point(a).lerp(point(b), values[a] / (values[a] - values[b]));
                            let i = out.positions.len() as u32;
                            out.positions.push(p.to_array());
                            out.normals.push(field.normal(p).to_array());
                            i
                        })
                    };
                    let tris = if inside.len() == 1 {
                        vec![[
                            crossing(inside[0], outside[0]),
                            crossing(inside[0], outside[1]),
                            crossing(inside[0], outside[2]),
                        ]]
                    } else if outside.len() == 1 {
                        vec![[
                            crossing(outside[0], inside[0]),
                            crossing(outside[0], inside[1]),
                            crossing(outside[0], inside[2]),
                        ]]
                    } else {
                        let a = crossing(inside[0], outside[0]);
                        let b = crossing(inside[0], outside[1]);
                        let c = crossing(inside[1], outside[1]);
                        let d = crossing(inside[1], outside[0]);
                        vec![[a, b, c], [a, c, d]]
                    };
                    for mut tri in tris {
                        let p = tri.map(|i| Vec3::from(out.positions[i as usize]));
                        let n = tri.map(|i| Vec3::from(out.normals[i as usize]));
                        if (p[1] - p[0]).cross(p[2] - p[0]).dot(n[0] + n[1] + n[2]) < 0. {
                            tri.swap(1, 2);
                        }
                        out.indices.extend(tri);
                    }
                }
            }
        }
    }
    if out.positions.is_empty() {
        return Err("empty sculpt".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounded_section_end_is_continuous_and_overlaps_its_neighbour() {
        let volume = Volume {
            min: Vec3::splat(-1.),
            max: Vec3::splat(2.),
            segments: vec![Segment {
                a: Vec3::ZERO,
                delta: Vec3::Z,
                inverse: 1.,
                right: Vec3::X,
                up: Vec3::Y,
                radii: [1.; 4],
            }],
        };
        let before = volume.distance(Vec3::new(0.5, 0., 1. - 1e-5));
        let after = volume.distance(Vec3::new(0.5, 0., 1. + 1e-5));
        assert!(
            (before - after).abs() < 0.0001,
            "a disc cap would create a surface discontinuity"
        );
        assert!(after < 0.);
        assert!(volume.distance(Vec3::new(0.5, 0., 2.5)) > 0.);
    }
}
