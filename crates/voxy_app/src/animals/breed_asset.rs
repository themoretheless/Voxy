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
    let mut limb = ((hip.y + a.shoulder_height * 0.04 - p.y) / (a.shoulder_height * 0.24))
        .clamp(0., 1.)
        * region;
    let paw = a.paw_half_width;
    let foot_coverage = (1. - smooth(paw * 0.9, paw * 1.3, (p.x - hip.x).abs()))
        * smooth(-paw * 1.5, -paw * 1.25, p.z - hip.z)
        * (1. - smooth(paw * 2.1, paw * 2.7, p.z - hip.z))
        * (1. - smooth(paw * 1.2, paw * 2.0, p.y));
    limb += (1. - limb) * foot_coverage;
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
fn tabby_coat(a: BreedAnatomy, p: Vec3) -> Vec3 {
    let brown = a.breed == Breed::MaineCoon;
    let base = if brown {
        Vec3::new(0.25, 0.17, 0.085)
    } else {
        Vec3::new(0.27, 0.28, 0.29)
    };
    let dark = if brown {
        Vec3::new(0.038, 0.027, 0.018)
    } else {
        Vec3::new(0.035, 0.041, 0.045)
    };
    let pale = if brown {
        Vec3::new(0.46, 0.35, 0.21)
    } else {
        Vec3::new(0.48, 0.48, 0.45)
    };
    let q = (p - a.head_origin()) / a.skull_width;
    let facial = smooth(
        a.head_origin().z - a.skull_width * 0.05,
        a.head_origin().z + a.head_length * 0.35,
        p.z,
    ) * smooth(-0.55, -0.35, q.y);
    let flank_phase = p.z / a.shoulder_height * 34.
        + (p.y / a.shoulder_height * 11.).sin() * 1.4
        + (p.x / a.shoulder_height * 8.).cos() * 0.6;
    let flank = smooth(0.50, 0.86, flank_phase.sin());
    let belly = 1. - smooth(a.shoulder_height * 0.48, a.shoulder_height * 0.85, p.y);
    let mut c = base
        .lerp(pale, belly * 0.55)
        .lerp(dark, flank * (1. - belly * 0.7) * 0.86);
    // Paired broken forehead strokes form the central tabby M. Distance to
    // authored strokes avoids wrapping body stripes indiscriminately over eyes.
    let x = q.x.abs();
    let strokes = [
        ([0.025, 0.34], [0.11, 0.13]),
        ([0.11, 0.13], [0.20, 0.31]),
        ([0.20, 0.31], [0.27, 0.15]),
    ];
    let mut distance = f32::INFINITY;
    for (a, b) in strokes {
        let point = glam::Vec2::new(x, q.y);
        let a = glam::Vec2::from(a);
        let b = glam::Vec2::from(b);
        let v = b - a;
        let t = ((point - a).dot(v) / v.length_squared()).clamp(0., 1.);
        distance = distance.min(point.distance(a + v * t));
    }
    let forehead = (1. - smooth(0.010, 0.035, distance)) * smooth(0.08, 0.13, q.y);
    let cheek = smooth(0.55, 0.88, ((q.y + x * 0.38) * 43.).sin())
        * smooth(0.21, 0.32, x)
        * (1. - smooth(-0.04, 0.04, q.y));
    let muzzle = (1. - smooth(0.17, 0.29, x))
        * (1. - smooth(-0.18, -0.10, q.y))
        * smooth(
            a.head_length * 0.65,
            a.head_length * 0.85,
            p.z - a.head_origin().z,
        );
    let face = base
        .lerp(pale, muzzle * 0.85)
        .lerp(dark, (forehead + cheek).clamp(0., 1.) * 0.90);
    c = c.lerp(face, facial);
    let tail = 1. - smooth(-a.trunk_length * 0.52, -a.trunk_length * 0.45, p.z);
    let rings = smooth(0.20, 0.66, (p.z / a.tail_length * 45.).sin());
    c.lerp(dark, tail * rings * 0.85)
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
        Breed::DomesticShorthair | Breed::MaineCoon => tabby_coat(a, p),
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
fn claw(out: &mut Surface, root: Vec3, paw: f32) {
    let start = out.vertices.len() as u32;
    for row in 0..=5 {
        let t = row as f32 / 5.;
        let center = root + Vec3::new(0., -paw * 0.05 * t * t, paw * 0.12 * t);
        let axis = Vec3::new(0., -0.10 * t, 0.12).normalize();
        let up = axis.cross(Vec3::X);
        let radius = paw * (0.060 * (1. - t).powf(1.2) + 0.004);
        for k in 0..8 {
            let angle = k as f32 * std::f32::consts::TAU / 8.;
            let normal = Vec3::X * angle.cos() + up * angle.sin();
            vertex(
                out,
                center + normal * radius,
                normal,
                Vec3::new(0.055, 0.043, 0.035),
                0.40,
                false,
                false,
            );
        }
    }
    for row in 0..5 {
        for k in 0..8 {
            let a = start + row * 8 + k;
            let b = start + row * 8 + (k + 1) % 8;
            out.indices.extend([a, b, a + 8, b, b + 8, a + 8]);
        }
    }
    for tip in [false, true] {
        let center = out.vertices.len() as u32;
        let t = if tip { 1. } else { 0. };
        let axis = Vec3::new(0., -0.10 * t, 0.12).normalize();
        vertex(
            out,
            root + Vec3::new(0., -paw * 0.05 * t * t, paw * 0.12 * t),
            axis * if tip { 1. } else { -1. },
            Vec3::new(0.055, 0.043, 0.035),
            0.40,
            false,
            false,
        );
        let ring = start + if tip { 40 } else { 0 };
        for k in 0..8 {
            let a = ring + k;
            let b = ring + (k + 1) % 8;
            out.indices
                .extend(if tip { [center, a, b] } else { [center, b, a] });
        }
    }
}
fn eye(out: &mut Surface, center: Vec3, side: f32, r: f32, cat: bool) {
    let n = profile(cat).eye_normal(side);
    let center = center - n * r * 0.22;
    let u = Vec3::new(n.z, 0., -n.x).normalize();
    let v = n.cross(u);
    let start = out.vertices.len() as u32;
    vertex(
        out,
        center + n * r * 0.06,
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
    // A continuous annular iris surrounds the pupil. The old overlapping
    // triangle fans put both centres in the same plane and hid most of the iris.
    let iris = if cat {
        Vec3::new(0.25, 0.28, 0.075)
    } else {
        Vec3::new(0.10, 0.052, 0.016)
    };
    let dome = |x: f32, y: f32| {
        let z = (1.05_f32.powi(2) - x * x - y * y).max(0.001).sqrt();
        (0.02 + z - 0.60, (u * x + v * y + n * z).normalize())
    };
    let start = out.vertices.len() as u32;
    for row in 0..=8 {
        let f = row as f32 / 8.;
        for k in 0..=96 {
            let t = k as f32 * std::f32::consts::TAU / 96.;
            let inner_x = if cat { 0.07 } else { 0.23 };
            let inner_y = if cat { 0.40 } else { 0.23 };
            let aperture = 1. / ((t.cos() / 0.88).powi(2) + (t.sin() / 0.49).powi(2)).sqrt();
            let outer = aperture.min(if cat { 0.84 } else { 0.65 });
            let x = (inner_x + (outer - inner_x) * f) * t.cos();
            let y = (inner_y + (outer - inner_y) * f) * t.sin();
            let (depth, normal) = dome(x, y);
            let radial_fibres =
                0.86 + 0.14 * (t * 37. + f * 4.).sin() + 0.07 * (t * 71. - f * 7.).cos();
            let limbus = 1. - 0.70 * ((f - 0.80) / 0.20).clamp(0., 1.);
            let c = iris * radial_fibres * limbus;
            vertex(
                out,
                center + r * (u * x + v * y + n * depth),
                normal,
                c,
                1.65,
                cat,
                false,
            );
        }
    }
    for row in 0..8 {
        for k in 0..96 {
            let a = start + row * 97 + k;
            out.indices
                .extend([a, a + 97, a + 1, a + 1, a + 97, a + 98]);
        }
    }
    // The pupil has its own curved patch, with a shared boundary and no
    // overlapping iris triangles through its centre.
    let start = out.vertices.len() as u32;
    vertex(
        out,
        center + n * r * dome(0., 0.).0,
        n,
        Vec3::splat(0.001),
        1.65,
        cat,
        false,
    );
    for k in 0..=96 {
        let t = k as f32 * std::f32::consts::TAU / 96.;
        let x = if cat { 0.07 } else { 0.23 } * t.cos();
        let y = if cat { 0.40 } else { 0.23 } * t.sin();
        let (depth, normal) = dome(x, y);
        vertex(
            out,
            center + r * (u * x + v * y + n * depth),
            normal,
            Vec3::splat(0.001),
            1.65,
            cat,
            false,
        );
        if k > 0 {
            out.indices.extend([start, start + k, start + k + 1]);
        }
    }
    // A narrow lid margin connects the aperture to surrounding skin.
    let start = out.vertices.len() as u32;
    for row in 0..=4 {
        let f = row as f32 / 4.;
        for k in 0..=96 {
            let t = k as f32 * std::f32::consts::TAU / 96.;
            let x = t.cos() * (1. + f * 0.20);
            let y = t.sin() * (0.53 + f * 0.13);
            let p = center + r * (u * x + v * y + n * (0.08 - f * 0.12));
            let c = Vec3::new(0.018, 0.012, 0.010).lerp(coat(cat, p), f * f);
            vertex(out, p, n, c, 0.50, cat, false);
        }
    }
    for row in 0..4 {
        for k in 0..96 {
            let a = start + row * 97 + k;
            out.indices
                .extend([a, a + 97, a + 1, a + 1, a + 97, a + 98]);
        }
    }
}
fn nasal_planum(out: &mut Surface, center: Vec3, r: Vec3, cat: bool) {
    // Nostrils are recessed in one nasal surface rather than protruding black
    // spheres. Rounded wings and a shallow central cleft break the flat wedge.
    let point = |u: f32, t: f32| {
        let width = r.x
            * if cat {
                (1.0 - t * 0.83) * (0.86 + 0.14 * (t * 8.).min(1.))
            } else {
                1.0 - t * 0.25
            };
        let nostril = (((u.abs() - 0.62) / 0.19).powi(2) + ((t - 0.36) / 0.14).powi(2)).sqrt();
        let recess = (1. - nostril).clamp(0., 1.);
        let groove = (-u * u / 0.012).exp() * t * 0.10;
        Vec3::new(
            u * width,
            (0.5 - t) * r.y * 2. - r.y * 0.12 * u * u,
            r.z * ((1. - u * u * 0.40) * (0.7 + 0.3 * (t * std::f32::consts::PI).sin())
                - recess * 0.40
                - groove),
        )
    };
    let start = out.vertices.len() as u32;
    for y in 0..=48 {
        let t = y as f32 / 48.;
        for x in 0..=64 {
            let u = x as f32 / 32. - 1.;
            let du = point(u + 0.001, t) - point(u - 0.001, t);
            let dt = point(u, t + 0.001) - point(u, t - 0.001);
            let normal = dt.cross(du).normalize();
            let nostril = ((u.abs() - 0.62) / 0.19).powi(2) + ((t - 0.36) / 0.14).powi(2);
            let base = if cat {
                Vec3::new(0.095, 0.040, 0.033)
            } else {
                Vec3::splat(0.006)
            };
            let darkness = ((nostril - 0.30) / 0.55).clamp(0., 1.);
            vertex(
                out,
                center + point(u, t),
                normal,
                Vec3::splat(0.001).lerp(base, darkness),
                0.48,
                cat,
                false,
            );
        }
    }
    for y in 0..48 {
        for x in 0..64 {
            let a = start + y * 65 + x;
            out.indices
                .extend([a, a + 65, a + 1, a + 1, a + 65, a + 66]);
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
    let head_y = a.head_origin().y;
    let head_z = a.head_origin().z;
    let skull = a.head_length - a.muzzle_length;
    for (index, (p, n)) in mesh.positions.into_iter().zip(mesh.normals).enumerate() {
        let inner_ear = mesh.inner_ear.get(index).copied().unwrap_or(false);
        let ear_weight = mesh.inner_ear_weight.get(index).copied().unwrap_or(0.0);
        let p = Vec3::from(p);
        let pad = smooth(0.35, 0.15, p.y / a.paw_half_width) * smooth(0.20, 0.70, -n[1]);
        let skin_color = coat(cat, p).lerp(Vec3::new(0.025, 0.016, 0.014), pad);
        vertex(
            &mut out,
            p,
            Vec3::from(n),
            skin_color.lerp(Vec3::new(0.22, 0.10, 0.075), ear_weight) * (1. - ear_weight * 0.25),
            0.85,
            cat,
            !inner_ear && p.y > a.paw_half_width * 0.35,
        );
    }
    for side in [-1., 1.] {
        eye(&mut out, a.eye_center(side), side, a.eye_radius(), cat);
        let mouth = [
            Vec3::new(
                side * a.skull_width * if cat { 0.08 } else { 0.17 },
                head_y - skull * if cat { 0.36 } else { 0.40 },
                head_z + a.head_length * 0.97,
            ),
            Vec3::new(
                side * a.skull_width * if cat { 0.16 } else { 0.24 },
                head_y - skull * if cat { 0.38 } else { 0.43 },
                head_z
                    + if cat {
                        a.head_length * 0.92
                    } else {
                        skull * 1.12
                    },
            ),
            Vec3::new(
                side * a.skull_width * if cat { 0.18 } else { 0.31 },
                head_y - skull * 0.43,
                head_z
                    + if cat {
                        a.head_length * 0.80
                    } else {
                        skull * 0.79
                    },
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
        for k in 0..if cat { 12 } else { 4 } {
            let row = k / 4;
            let column = k % 4;
            let root = origin
                + Vec3::new(
                    side * a.skull_width * column as f32 * 0.015,
                    (row as f32 - 1.) * skull * 0.045,
                    -column as f32 * skull * 0.025,
                );
            let span = a.skull_width
                * if cat {
                    0.57 + column as f32 * 0.055
                } else {
                    0.28
                };
            let end = root
                + Vec3::new(
                    side * span,
                    (k as f32 - 5.5) * a.skull_width * 0.021,
                    -a.skull_width * 0.17,
                );
            let control = root
                + Vec3::new(
                    side * span * 0.52,
                    a.skull_width * 0.025,
                    a.skull_width * 0.06,
                );
            let mut previous = root;
            for segment in 1..=12 {
                let t = segment as f32 / 12.;
                let p = root * (1. - t).powi(2) + control * (2. * t * (1. - t)) + end * t * t;
                tube(
                    &mut out,
                    previous,
                    p,
                    (if cat { 0.00010 } else { 0.00013 }) * (1. - t * 0.88),
                    Vec3::splat(0.34),
                    cat,
                );
                previous = p;
            }
        }
    }
    if !cat {
        for side in [-1., 1.] {
            for front in [false, true] {
                for digit in 0..4 {
                    claw(
                        &mut out,
                        a.digit_tip(side, front, digit)
                            + Vec3::new(0., -a.paw_half_width * 0.03, a.paw_half_width * 0.10),
                        a.paw_half_width,
                    );
                }
            }
        }
    }
    let nose = a.nose_center();
    let radius = if cat {
        Vec3::new(a.skull_width * 0.095, skull * 0.065, skull * 0.025)
    } else {
        Vec3::new(a.skull_width * 0.16, skull * 0.11, skull * 0.025)
    };
    nasal_planum(&mut out, nose, radius, cat);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distal_digits_follow_the_foot_joint() {
        for cat in [false, true] {
            let a = profile(cat);
            for side in [-1., 1.] {
                for front in [false, true] {
                    for digit in 0..4 {
                        let p = a.digit_tip(side, front, digit);
                        let (joints, weights) = skin(p, cat);
                        let leg = usize::from(side > 0.) + if front { 0 } else { 2 };
                        assert_eq!(joints[2], (5 + leg) as u16);
                        assert_eq!(weights[2], 65535, "digit detaches from foot: {weights:?}");
                    }
                }
            }
        }
    }
    #[test]
    fn eye_surfaces_face_the_viewer() {
        for cat in [false, true] {
            for side in [-1., 1.] {
                let mut surface = Surface {
                    vertices: Vec::new(),
                    indices: Vec::new(),
                    fur: Vec::new(),
                };
                eye(&mut surface, Vec3::ZERO, side, 0.01, cat);
                let forward = profile(cat).eye_normal(side);
                for triangle in surface.indices.chunks_exact(3) {
                    let p: Vec<_> = triangle
                        .iter()
                        .map(|&i| Vec3::from(surface.vertices[i as usize].position))
                        .collect();
                    let area = (p[1] - p[0]).cross(p[2] - p[0]);
                    assert!(
                        area.dot(forward) > 0.,
                        "back-facing eye triangle: {triangle:?}"
                    );
                }
            }
        }
    }
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
