//! Voxy-owned breed sculpts and small anatomical features. No external assets.
use super::*;
use std::sync::OnceLock;
use voxy_model::animals::{Breed, BreedAnatomy, Species};
pub struct Surface {
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
    pub fur: Vec<bool>,
}
static DOG: OnceLock<Result<Surface, String>> = OnceLock::new();
static CAT: OnceLock<Result<Surface, String>> = OnceLock::new();
fn selected_breed(cat: bool) -> Breed {
    let flag = if cat { "--cat-breed" } else { "--dog-breed" };
    let args: Vec<_> = std::env::args().collect();
    args.windows(2)
        .find(|a| a[0] == flag)
        .and_then(|a| Breed::parse(&a[1]).ok())
        .filter(|b| (b.species() == Species::Cat) == cat)
        .unwrap_or(if cat {
            Breed::DomesticShorthair
        } else {
            Breed::Labrador
        })
}
pub fn breed(cat: bool) -> Breed {
    static DOG_BREED: OnceLock<Breed> = OnceLock::new();
    static CAT_BREED: OnceLock<Breed> = OnceLock::new();
    *(if cat { &CAT_BREED } else { &DOG_BREED }).get_or_init(|| selected_breed(cat))
}
pub fn validate_selection() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    for (flag, species) in [("--dog-breed", Species::Dog), ("--cat-breed", Species::Cat)] {
        if let Some(index) = args.iter().position(|arg| arg == flag) {
            let id = args
                .get(index + 1)
                .ok_or_else(|| format!("{flag} requires a breed"))?;
            if Breed::parse(id)?.species() != species {
                return Err(format!("{id} does not match {flag}"));
            }
        }
    }
    Ok(())
}
pub fn profile(cat: bool) -> BreedAnatomy {
    breed(cat).anatomy()
}
pub fn surface(cat: bool) -> Result<&'static Surface, String> {
    (if cat { &CAT } else { &DOG })
        .get_or_init(|| build(cat))
        .as_ref()
        .map_err(Clone::clone)
}
pub fn encoded_color(c: Vec3, roughness: f32) -> [f32; 2] {
    let c = (c.clamp(Vec3::ZERO, Vec3::ONE) * 255.).round().as_uvec3();
    [
        (c.x | (c.y << 8) | (c.z << 16)) as f32,
        -10000. - roughness * 1000.,
    ]
}
pub fn skin(p: Vec3, cat: bool) -> ([u16; 4], [u16; 4]) {
    let a = profile(cat);
    let hips = leg_pivots(cat);
    let leg = usize::from(p.x >= 0.) + if p.z > 0. { 0 } else { 2 };
    let hip = hips[leg];
    let region = ((p.x.abs() - a.rib_half_width * 0.22) / (a.rib_half_width * 0.42)).clamp(0., 1.)
        * (1. - (p.z - hip.z).abs() / (a.trunk_length * 0.25)).clamp(0., 1.);
    let limb = ((hip.y + a.shoulder_height * 0.04 - p.y) / (a.shoulder_height * 0.24))
        .clamp(0., 1.)
        * region;
    let foot = ((hip.y * 0.52 - p.y) / (a.shoulder_height * 0.18)).clamp(0., 1.);
    let tail = ((-p.z - a.trunk_length * 0.48) / (a.tail_length * 0.25)).clamp(0., 1.);
    let v = [
        (1. - limb) * (1. - tail),
        limb * (1. - foot) * (1. - tail),
        limb * foot * (1. - tail),
        tail,
    ];
    let mut w = v.map(|x| (x * 65535.).floor() as u16);
    w[0] += (65535 - w.iter().map(|&x| u32::from(x)).sum::<u32>()) as u16;
    ([0, 1 + leg as u16, 5 + leg as u16, 9], w)
}
fn smooth(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
pub fn coat(cat: bool, p: Vec3) -> Vec3 {
    let a = profile(cat);
    let h = a.shoulder_height;
    let front = a.trunk_length * 0.36;
    let white = Vec3::new(0.72, 0.70, 0.66);
    let c = match a.breed {
        Breed::Labrador => Vec3::new(0.27, 0.145, 0.068).lerp(
            Vec3::new(0.45, 0.29, 0.13),
            1. - smooth(h * 0.50, h * 0.92, p.y),
        ),
        Breed::GermanShepherd => {
            let tan = Vec3::new(0.31, 0.19, 0.085);
            let saddle =
                smooth(h * 0.52, h * 0.84, p.y) * (1. - smooth(front * 0.65, front * 1.2, p.z));
            tan.lerp(Vec3::splat(0.018), saddle)
        }
        Breed::SiberianHusky => {
            let dark = Vec3::new(0.075, 0.09, 0.105);
            let back = smooth(h * 0.53, h * 0.86, p.y);
            let face = smooth(front, front + a.head_length, p.z);
            white.lerp(dark, back * (1. - face * 0.65))
        }
        Breed::BritishShorthair => Vec3::new(0.17, 0.195, 0.23),
        Breed::DomesticShorthair | Breed::MaineCoon => {
            let c = if a.breed == Breed::MaineCoon {
                Vec3::new(0.24, 0.18, 0.105)
            } else {
                Vec3::new(0.25, 0.265, 0.27)
            };
            let stripes =
                ((p.z / h * 21. + p.x.abs() / h * 6. + p.y / h * 4.).sin() * 0.5 + 0.5).powi(8);
            c * (1. - stripes * 0.25)
        }
    };
    let grain =
        ((p.x * 731. + p.y * 489. + p.z * 947.).sin() * (p.z * 1271. - p.y * 831.).sin()) * 0.035;
    c * (1. + grain)
}
fn vertex(out: &mut Surface, p: Vec3, n: Vec3, color: Vec3, rough: f32, cat: bool, furry: bool) {
    let (joints, weights) = skin(p, cat);
    out.vertices.push(SkinnedVertex {
        position: p.to_array(),
        normal: n.to_array(),
        uv: encoded_color(color, rough),
        joints,
        weights,
    });
    out.fur.push(furry);
}
fn ellipsoid(out: &mut Surface, p: Vec3, r: Vec3, color: Vec3, rough: f32, cat: bool) {
    let start = out.vertices.len() as u32;
    for y in 0..=12 {
        let v = y as f32 / 12. * std::f32::consts::PI;
        for x in 0..=24 {
            let u = x as f32 / 24. * std::f32::consts::TAU;
            let n = Vec3::new(v.sin() * u.cos(), v.cos(), v.sin() * u.sin());
            vertex(
                out,
                p + n * r,
                (n / r).normalize(),
                color,
                rough,
                cat,
                false,
            );
        }
    }
    for y in 0..12 {
        for x in 0..24 {
            let a = start + y * 25 + x;
            out.indices
                .extend([a, a + 1, a + 25, a + 1, a + 26, a + 25]);
        }
    }
}
fn tube(out: &mut Surface, a: Vec3, b: Vec3, r: f32, color: Vec3, cat: bool) {
    let axis = (b - a).normalize();
    let right = axis
        .cross(if axis.y.abs() > 0.9 { Vec3::X } else { Vec3::Y })
        .normalize();
    let up = axis.cross(right);
    let start = out.vertices.len() as u32;
    for p in [a, b] {
        for k in 0..8 {
            let angle = k as f32 * std::f32::consts::TAU / 8.;
            let n = right * angle.cos() + up * angle.sin();
            vertex(out, p + n * r, n, color, 0.6, cat, false);
        }
    }
    for k in 0..8 {
        let j = (k + 1) % 8;
        out.indices.extend([
            start + k,
            start + j,
            start + k + 8,
            start + j,
            start + j + 8,
            start + k + 8,
        ]);
    }
}
fn eye(out: &mut Surface, center: Vec3, side: f32, r: f32, cat: bool) {
    let n = Vec3::new(side * 0.68, 0.07, 0.73).normalize();
    let u = Vec3::new(n.z, 0., -n.x).normalize();
    let v = n.cross(u);
    let start = out.vertices.len() as u32;
    vertex(
        out,
        center + n * r * 0.2,
        n,
        Vec3::splat(0.003),
        0.12,
        cat,
        false,
    );
    for k in 0..=48 {
        let t = k as f32 * std::f32::consts::TAU / 48.;
        let p = center + u * r * t.cos() + v * r * 0.53 * t.sin();
        vertex(out, p, n, Vec3::splat(0.008), 0.42, cat, false);
        if k > 0 {
            out.indices.extend([start, start + k, start + k + 1]);
        }
    }
    // Iris fits within the almond lid aperture. No exposed white cartoon sclera.
    let iris = if cat {
        Vec3::new(0.25, 0.28, 0.075)
    } else {
        Vec3::new(0.10, 0.052, 0.016)
    };
    for ring in 0..2 {
        let offset = out.vertices.len() as u32;
        let radius = r * if ring == 0 { 0.45 } else { 0.20 };
        vertex(
            out,
            center + n * r * 0.23,
            n,
            if ring == 0 { iris } else { Vec3::splat(0.002) },
            0.15,
            cat,
            false,
        );
        for k in 0..=32 {
            let t = k as f32 * std::f32::consts::TAU / 32.;
            let sx = if cat && ring == 1 { 0.3 } else { 1. };
            vertex(
                out,
                center
                    + n * r * (0.21 + ring as f32 * 0.04)
                    + u * radius * t.cos() * sx
                    + v * radius * t.sin(),
                n,
                if ring == 0 { iris } else { Vec3::splat(0.002) },
                0.15,
                cat,
                false,
            );
            if k > 0 {
                out.indices.extend([offset, offset + k, offset + k + 1]);
            }
        }
    }
}
fn build(cat: bool) -> Result<Surface, String> {
    let a = profile(cat);
    let mesh = voxy_model::animals::build_sculpt(a)?;
    let mut out = Surface {
        vertices: Vec::new(),
        indices: mesh.indices,
        fur: Vec::new(),
    };
    let head_y = a.shoulder_height + a.neck_length * 0.55;
    let head_z = a.trunk_length * 0.36 + a.neck_length * 0.48;
    let skull = a.head_length - a.muzzle_length;
    for (p, n) in mesh.positions.into_iter().zip(mesh.normals) {
        let p = Vec3::from(p);
        vertex(
            &mut out,
            p,
            Vec3::from(n),
            coat(cat, p),
            0.85,
            cat,
            p.y > a.paw_half_width * 0.35,
        );
    }
    for side in [-1., 1.] {
        eye(
            &mut out,
            Vec3::new(
                side * a.skull_width * 0.32,
                head_y + skull * 0.10,
                head_z + skull * 0.85,
            ),
            side,
            a.skull_width * 0.072,
            cat,
        );
        let mouth = [
            Vec3::new(
                side * a.skull_width * 0.17,
                head_y - skull * 0.40,
                head_z + a.head_length * 0.97,
            ),
            Vec3::new(
                side * a.skull_width * 0.24,
                head_y - skull * 0.43,
                head_z + skull * 1.12,
            ),
            Vec3::new(
                side * a.skull_width * 0.31,
                head_y - skull * 0.29,
                head_z + skull * 0.79,
            ),
        ];
        for pair in mouth.windows(2) {
            tube(
                &mut out,
                pair[0],
                pair[1],
                a.skull_width * 0.007,
                Vec3::splat(0.009),
                cat,
            );
        }
        let origin = mouth[0] + Vec3::Y * skull * 0.1;
        for k in 0..if cat { 7 } else { 3 } {
            let end = origin
                + Vec3::new(
                    side * a.skull_width * if cat { 0.8 } else { 0.35 },
                    (k as f32 - 3.) * a.skull_width * 0.035,
                    -a.skull_width * 0.13,
                );
            tube(
                &mut out,
                origin,
                end,
                if cat { 0.00012 } else { 0.00016 },
                Vec3::splat(0.48),
                cat,
            );
        }
        for z in [a.trunk_length * 0.36, -a.trunk_length * 0.36] {
            for toe in 0..4 {
                let x =
                    side * a.rib_half_width * 0.68 + (toe as f32 - 1.5) * a.paw_half_width * 0.43;
                let p = Vec3::new(
                    x,
                    a.paw_half_width * 0.57,
                    z + a.paw_half_width * (1.35 + (1.5 - (toe as f32 - 1.5).abs()) * 0.18),
                );
                ellipsoid(
                    &mut out,
                    p,
                    Vec3::new(
                        a.paw_half_width * 0.23,
                        a.paw_half_width * 0.35,
                        a.paw_half_width * 0.50,
                    ),
                    coat(cat, p) * 0.91,
                    0.87,
                    cat,
                );
            }
        }
    }
    let nose = Vec3::new(
        0.,
        head_y - skull * 0.21,
        head_z + a.head_length + a.skull_width * 0.16,
    );
    let radius = Vec3::new(a.skull_width * 0.165, skull * 0.18, a.skull_width * 0.067);
    ellipsoid(
        &mut out,
        nose,
        radius,
        if cat {
            Vec3::new(0.11, 0.045, 0.035)
        } else {
            Vec3::splat(0.009)
        },
        0.42,
        cat,
    );
    for side in [-1., 1.] {
        ellipsoid(
            &mut out,
            nose + Vec3::new(side * radius.x * 0.55, radius.y * 0.16, radius.z * 0.86),
            radius * Vec3::new(0.24, 0.26, 0.24),
            Vec3::splat(0.001),
            0.65,
            cat,
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nose_and_digit_faces_point_outward() {
        let mut surface = Surface {
            vertices: Vec::new(),
            indices: Vec::new(),
            fur: Vec::new(),
        };
        ellipsoid(
            &mut surface,
            Vec3::ZERO,
            Vec3::new(0.03, 0.02, 0.01),
            Vec3::splat(0.01),
            0.65,
            false,
        );
        for triangle in surface.indices.chunks_exact(3) {
            let p: Vec<_> = triangle
                .iter()
                .map(|&i| Vec3::from(surface.vertices[i as usize].position))
                .collect();
            let normal = (p[1] - p[0]).cross(p[2] - p[0]);
            if normal.length_squared() > 1e-15 {
                assert!(normal.dot(p[0] + p[1] + p[2]) > 0.0);
            }
        }
    }
}
