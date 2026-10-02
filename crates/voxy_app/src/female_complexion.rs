//! Deterministic facial tint atlas and small bind-space blemish relief.
//! Pore micro-shading is baked into the tint, not a physical skin BRDF.
use glam::{Vec2, Vec3};
use std::sync::OnceLock;

/// One in-flight atlas; the caller supplies the latest settings each frame.
#[derive(Debug, Default)]
pub(crate) struct AtlasUpdate {
    job: Option<(Vec<f32>, std::thread::JoinHandle<Vec<u8>>)>,
}
impl AtlasUpdate {
    pub(crate) fn poll(
        &mut self,
        applied: &[f32],
        parameters: &crate::face_parameters::FaceParameters,
    ) -> Result<Option<(Vec<f32>, Vec<u8>)>, String> {
        let latest = parameters.material_signature();
        if self.job.as_ref().is_some_and(|(_, job)| job.is_finished()) {
            let (signature, job) = self.job.take().unwrap();
            let pixels = job
                .join()
                .map_err(|_| "skin atlas worker panicked".to_string())?;
            if signature == latest && signature != applied {
                return Ok(Some((signature, pixels)));
            }
        }
        if self.job.is_none() && latest != applied {
            let parameters = parameters.clone();
            self.job = Some((
                latest,
                std::thread::spawn(move || atlas_for_parameters(&parameters)),
            ));
        }
        Ok(None)
    }
}

pub(crate) const SIZE: u32 = 2048;
pub(crate) const WIDTH: u32 = SIZE * 2;
pub(crate) const WHITE_UV: [f32; 2] = [0., 0.];
#[derive(Clone, Copy, Debug)]
pub(crate) struct SkinLayers {
    pub pigmentation: f32,
    pub pores: f32,
    pub pimples: f32,
    pub scars: f32,
    pub freckles: f32,
    pub makeup: f32,
    pub lipstick: f32,
    pub lipstick_color: Vec3,
    pub blush: f32,
    pub eyeshadow: f32,
    pub moles: f32,
    pub wrinkles: f32,
    pub vessels: f32,
}
impl Default for SkinLayers {
    fn default() -> Self {
        Self {
            pigmentation: 1.,
            pores: 1.,
            pimples: 1.,
            scars: 1.,
            freckles: 1.,
            makeup: 1.,
            lipstick: 1.,
            blush: 1.,
            eyeshadow: 1.,
            lipstick_color: Vec3::new(0.92, 0.43, 0.53),
            moles: 1.,
            wrinkles: 1.,
            vessels: 1.,
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Surface {
    pub tint: Vec3,
    pub height: f32,
    pub microheight: f32,
    pub roughness: f32,
    pub sebum: f32,
}
fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn ellipse(p: Vec2, center: Vec2, radius: Vec2) -> f32 {
    (1. - ((p - center) / radius).length_squared())
        .max(0.)
        .powi(2)
}
#[allow(clippy::cast_precision_loss)] // Hash output has only 24 significant bits.
fn random(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = x.cast_unsigned().wrapping_mul(0x8da6_b343)
        ^ y.cast_unsigned().wrapping_mul(0xd816_3841)
        ^ seed;
    h = (h ^ (h >> 16)).wrapping_mul(0x7feb_352d);
    h = (h ^ (h >> 15)).wrapping_mul(0x846c_a68b);
    ((h ^ (h >> 16)) >> 8) as f32 / 16_777_215.
}
#[allow(clippy::cast_possible_truncation)]
fn pigment_noise(p: Vec2, spacing: f32, seed: u32) -> f32 {
    let grid = p / spacing;
    let cell = grid.floor();
    let f = grid - cell;
    let f = f * f * (Vec2::splat(3.) - 2. * f);
    let x = cell.x as i32;
    let y = cell.y as i32;
    let a = random(x, y, seed) * (1. - f.x) + random(x + 1, y, seed) * f.x;
    let b = random(x, y + 1, seed) * (1. - f.x) + random(x + 1, y + 1, seed) * f.x;
    a * (1. - f.y) + b * f.y
}
/// Full-cell pore jitter with neighboring support preserves continuous borders.
#[allow(clippy::cast_possible_truncation)]
fn spot(p: Vec2, spacing: f32, radius: f32, seed: u32, density: f32) -> f32 {
    let cell = (p / spacing).floor();
    let mut coverage: f32 = 0.;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let x = cell.x as i32 + dx;
            let y = cell.y as i32 + dy;
            if random(x, y, seed) > density {
                continue;
            }
            let center = Vec2::new(
                x as f32 + random(x, y, seed + 1),
                y as f32 + random(x, y, seed + 2),
            ) * spacing;
            let r = radius * (0.6 + 0.4 * random(x, y, seed + 3));
            coverage = coverage.max(1. - smooth(0.35 * r, r, p.distance(center)));
        }
    }
    coverage
}
/// Neighbor search allows full-cell jitter without clipping spots at grid edges.
#[allow(clippy::cast_possible_truncation)]
fn freckles(p: Vec2) -> f32 {
    let spacing = 0.0045;
    let cell = (p / spacing).floor();
    let mut coverage: f32 = 0.;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let x = cell.x as i32 + dx;
            let y = cell.y as i32 + dy;
            let cluster = random(x.div_euclid(4), y.div_euclid(4), 349);
            if random(x, y, 351) > 0.25 + 0.45 * cluster {
                continue;
            }
            let center =
                Vec2::new(x as f32 + random(x, y, 352), y as f32 + random(x, y, 353)) * spacing;
            let r = 0.0003 + 0.001 * random(x, y, 354).powi(2);
            let angle = random(x, y, 355) * std::f32::consts::TAU;
            let offset = p - center;
            let rotated = Vec2::new(
                offset.x * angle.cos() + offset.y * angle.sin(),
                -offset.x * angle.sin() + offset.y * angle.cos(),
            );
            let eccentricity = 0.65 + 0.35 * random(x, y, 356);
            let distance = Vec2::new(rotated.x, rotated.y / eccentricity).length();
            let strength = 0.3 + 0.65 * random(x, y, 357);
            coverage = coverage.max((1. - smooth(0.2 * r, r, distance)) * strength);
        }
    }
    coverage
}
fn tint(surface: &mut Surface, color: Vec3, amount: f32) {
    surface.tint = surface.tint.lerp(color, amount.clamp(0., 1.));
}
pub(crate) fn sample(p: Vec2, layers: SkinLayers) -> Surface {
    let mut out = Surface {
        tint: Vec3::ONE,
        height: 0.,
        microheight: 0.,
        roughness: 0.62,
        sebum: 0.12,
    };
    let face = smooth(0.60, 0.625, p.y)
        * (1. - smooth(0.785, 0.805, p.y))
        * (1. - smooth(0.072, 0.095, p.x.abs()));
    let pore = spot(p, 0.00085, 0.00019, 17, 0.88);
    tint(
        &mut out,
        Vec3::splat(0.82),
        pore * 0.45 * face * layers.pores,
    );
    out.microheight = -0.000_015 * pore * face * layers.pores;
    let nose_oil = ellipse(p, Vec2::new(0., 0.693), Vec2::new(0.022, 0.042));
    let forehead_oil = ellipse(p, Vec2::new(0., 0.759), Vec2::new(0.051, 0.031));
    out.sebum = 0.12 + 0.48 * nose_oil + 0.22 * forehead_oil;
    out.roughness = 0.62 - 0.23 * nose_oil - 0.10 * forehead_oil;
    let cheek = ellipse(p, Vec2::new(0.046, 0.691), Vec2::new(0.035, 0.03))
        + ellipse(p, Vec2::new(-0.046, 0.691), Vec2::new(0.035, 0.03));
    let bridge = ellipse(p, Vec2::new(0., 0.695), Vec2::new(0.023, 0.03));
    // Smooth pigment variation is intrinsic color, independent of lighting.
    let mottling = 0.65 * pigment_noise(p, 0.012, 811) + 0.35 * pigment_noise(p, 0.003, 812);
    let warmth =
        (0.035 + 0.09 * cheek + 0.045 * bridge) * (0.45 + mottling) * face * layers.pigmentation;
    tint(&mut out, Vec3::new(1., 0.65, 0.60), warmth);
    tint(
        &mut out,
        Vec3::new(0.72, 0.67, 0.56),
        0.075 * mottling * face * layers.pigmentation,
    );
    out.roughness += (mottling - 0.5) * 0.055 * face * layers.pigmentation;
    let freckle = freckles(p);
    tint(
        &mut out,
        Vec3::new(0.43, 0.41, 0.32),
        freckle * (cheek + bridge).clamp(0., 1.) * 0.75 * layers.freckles,
    );
    for (center, radius) in [
        (Vec2::new(-0.048, 0.670), 0.0023),
        (Vec2::new(0.043, 0.752), 0.0019),
        (Vec2::new(0.058, 0.678), 0.0016),
    ] {
        let d = p.distance(center) / radius;
        let bump = (1. - d * d).max(0.).powi(2) * layers.pimples;
        out.height += 0.00055 * bump;
        tint(
            &mut out,
            Vec3::new(0.98, 0.44, 0.43),
            (1. - smooth(0.45, 1.7, d)) * 0.65 * layers.pimples,
        );
        tint(
            &mut out,
            Vec3::new(1., 0.92, 0.77),
            (1. - smooth(0.12, 0.28, d)) * 0.7 * layers.pimples,
        );
    }
    for (a, b) in [
        (Vec2::new(0.055, 0.659), Vec2::new(0.062, 0.691)),
        (Vec2::new(-0.056, 0.741), Vec2::new(-0.043, 0.747)),
    ] {
        let edge = b - a;
        let t = ((p - a).dot(edge) / edge.length_squared()).clamp(0., 1.);
        let d = p.distance(a + t * edge);
        let ends = smooth(0., 0.12, t) * (1. - smooth(0.88, 1., t));
        let scar = (1. - smooth(0.00025, 0.0009, d)) * ends * layers.scars;
        tint(&mut out, Vec3::new(1., 0.80, 0.72), scar * 0.65);
        out.height -= 0.00025 * scar;
    }
    for (center, radius) in [
        (Vec2::new(-0.037, 0.661), 0.0015),
        (Vec2::new(0.067, 0.709), 0.0010),
        (Vec2::new(-0.020, 0.767), 0.0008),
    ] {
        let mole = 1. - smooth(0.55 * radius, radius, p.distance(center));
        tint(
            &mut out,
            Vec3::new(0.20, 0.17, 0.13),
            mole * 0.9 * layers.moles,
        );
        out.height += 0.00018 * mole * layers.moles;
    }
    // Tinted balm follows the Cupid's bow; blush and eyeshadow are independent regions.
    let x = p.x.abs();
    let lip_top = 0.654 - 0.005 * smooth(0.007, 0.027, x) - 0.0015 * (1. - smooth(0., 0.005, x));
    let lip_bottom = 0.637 + 0.008 * smooth(0., 0.028, x);
    let lip = smooth(lip_bottom - 0.001, lip_bottom + 0.001, p.y)
        * (1. - smooth(lip_top - 0.001, lip_top + 0.001, p.y))
        * (1. - smooth(0.025, 0.030, x));
    // Anisotropic vermilion microfolds: long along lip height, irregular across
    // its width. Bind-space sampling keeps them attached through expressions.
    let lip_noise = pigment_noise(Vec2::new(p.x, p.y * 0.12), 0.0008, 991);
    let folds = smooth(0.48, 0.82, lip_noise) * lip * layers.wrinkles;
    out.microheight -= 0.000_025 * folds;
    tint(&mut out, Vec3::new(0.88, 0.82, 0.82), folds * 0.16);
    out.roughness += (0.33 - out.roughness) * lip * layers.makeup * layers.lipstick;
    out.roughness += 0.065 * folds;

    out.sebum += (0.48 - out.sebum) * lip * layers.makeup * layers.lipstick;
    tint(
        &mut out,
        layers.lipstick_color,
        lip * 0.78 * layers.makeup * layers.lipstick,
    );
    tint(
        &mut out,
        Vec3::new(1., 0.77, 0.77),
        cheek * 0.24 * layers.makeup * layers.blush,
    );
    for side in [-1., 1.] {
        let shadow = ellipse(p, Vec2::new(side * 0.034, 0.724), Vec2::new(0.026, 0.012))
            * smooth(0.713, 0.719, p.y);
        tint(
            &mut out,
            Vec3::new(0.70, 0.54, 0.61),
            shadow * 0.55 * layers.makeup * layers.eyeshadow,
        );
    }
    fine_lines(&mut out, p, layers);
    out.tint = Vec3::ONE.lerp(out.tint, face);
    out.height *= face;
    out
}
fn fine_lines(out: &mut Surface, p: Vec2, layers: SkinLayers) {
    let forehead = 1. - smooth(0.04, 0.07, p.x.abs());
    for y in [0.761, 0.769, 0.776] {
        let curve = y - 0.9 * p.x * p.x;
        let line = (1. - smooth(0.00012, 0.0006, (p.y - curve).abs())) * forehead;
        tint(
            out,
            Vec3::new(0.80, 0.76, 0.72),
            line * 0.035 * layers.wrinkles,
        );
        out.height -= 0.00005 * line * layers.wrinkles;
    }
    for side in [-1., 1.] {
        for slope in [-0.3, 0., 0.3] {
            let x = side * p.x;
            let line = (1. - smooth(0.00015, 0.0007, (p.y - (0.712 + slope * (x - 0.05))).abs()))
                * smooth(0.05, 0.053, x)
                * (1. - smooth(0.062, 0.069, x));
            tint(
                out,
                Vec3::new(0.79, 0.74, 0.70),
                line * 0.28 * layers.wrinkles,
            );
            out.height -= 0.000_045 * line * layers.wrinkles;
        }
        let fold_x = side * (0.018 + 0.018 * smooth(0.650, 0.686, p.y));
        let fold = (1. - smooth(0.0002, 0.0013, (p.x - fold_x).abs()))
            * smooth(0.645, 0.655, p.y)
            * (1. - smooth(0.683, 0.690, p.y));
        tint(
            out,
            Vec3::new(0.85, 0.80, 0.77),
            fold * 0.3 * layers.wrinkles,
        );
        out.height -= 0.000_075 * fold * layers.wrinkles;
        for branch in [0., 1., 2.] {
            let y = 0.685 + branch * 0.004 + 0.003 * ((side * p.x - 0.058) * 220. + branch).sin();
            let vessel = (1. - smooth(0.00012, 0.00045, (p.y - y).abs()))
                * ellipse(p, Vec2::new(side * 0.06, 0.687), Vec2::new(0.011, 0.011));
            tint(
                out,
                Vec3::new(1., 0.68, 0.68),
                vessel * 0.20 * layers.vessels,
            );
        }
    }
}
pub(crate) fn uv(p: Vec3) -> [f32; 2] {
    if p.z < 0.09 || p.x.abs() >= 0.1 || !(0.59..=0.81).contains(&p.y) {
        return WHITE_UV;
    }
    [
        (0.05 + 0.9 * (p.x + 0.1) / 0.2) * 0.5,
        0.05 + 0.9 * (0.81 - p.y) / 0.22,
    ]
}
/// Bind texture coordinates and relief to the original surface, before animation.
pub(crate) fn apply(vertices: &mut [voxy_render::SceneVertex], normals: &[Vec3]) {
    assert_eq!(vertices.len(), normals.len());
    for (vertex, normal) in vertices.iter_mut().zip(normals) {
        let p = Vec3::from_array(vertex.position);
        vertex.uv = uv(p);
        if vertex.uv != WHITE_UV {
            let height = sample(Vec2::new(p.x, p.y), SkinLayers::default()).height;
            vertex.position = (p + normal * height).to_array();
        }
    }
}
/// White border keeps eyes, hair, body and diagnostic overlays untinted.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // 2048-pixel coordinates and clamped byte colors.
pub(crate) fn atlas() -> &'static [u8] {
    static ATLAS: OnceLock<Vec<u8>> = OnceLock::new();
    ATLAS.get_or_init(|| build_atlas(SkinLayers::default()))
}
fn build_atlas(layers: SkinLayers) -> Vec<u8> {
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(8);
    build_atlas_with_workers(layers, workers)
}
fn build_atlas_with_workers(layers: SkinLayers, workers: usize) -> Vec<u8> {
    let mut pixels = vec![255; (WIDTH * SIZE * 4) as usize];
    let rows = (SIZE as usize).div_ceil(workers.max(1));
    let stride = (WIDTH * 4) as usize;
    std::thread::scope(|scope| {
        for (block, pixels) in pixels.chunks_mut(rows * stride).enumerate() {
            scope.spawn(move || {
                for local_y in 0..pixels.len() / stride {
                    let y = (block * rows + local_y) as u32;
                    for x in 0..SIZE {
                        let u = (x as f32 + 0.5) / SIZE as f32;
                        let v = (y as f32 + 0.5) / SIZE as f32;
                        let eye = crate::female_eyes::is_eye_uv([u * 0.5, v]);
                        if !eye && (!(0.05..0.95).contains(&u) || !(0.05..0.95).contains(&v)) {
                            continue;
                        }
                        let p =
                            Vec2::new((u - 0.05) / 0.9 * 0.2 - 0.1, 0.81 - (v - 0.05) / 0.9 * 0.22);
                        let surface = sample(p, layers);
                        let color = if eye {
                            crate::female_eyes::pigment(Vec2::new(u * 0.5, v))
                        } else {
                            surface.tint
                        };
                        let offset = local_y * stride + (x * 4) as usize;
                        // Renderer decodes sRGB textures; encode linear tint factors here.
                        for (channel, value) in color.to_array().into_iter().enumerate() {
                            let srgb = if value <= 0.003_130_8 {
                                12.92 * value
                            } else {
                                1.055 * value.powf(1. / 2.4) - 0.055
                            };
                            pixels[offset + channel] = (srgb.clamp(0., 1.) * 255.).round() as u8;
                            let parameters = [
                                surface.roughness,
                                surface.sebum,
                                0.5 + surface.microheight / 0.002,
                            ];
                            let value = parameters[channel].clamp(0., 1.);
                            let encoded = if value <= 0.003_130_8 {
                                12.92 * value
                            } else {
                                1.055 * value.powf(1. / 2.4) - 0.055
                            };
                            pixels[offset + (SIZE * 4) as usize + channel] =
                                (encoded * 255.).round() as u8;
                        }
                    }
                }
            });
        }
    });
    pixels
}
/// Adds parameterized crease microheight while retaining the cached pigmentation.
pub(crate) fn atlas_for_parameters(parameters: &crate::face_parameters::FaceParameters) -> Vec<u8> {
    let layers = SkinLayers {
        pores: parameters.value("skin_pores").unwrap_or(1.),
        makeup: parameters.value("skin_makeup").unwrap_or(1.),
        lipstick: parameters.value("makeup_lipstick").unwrap_or(1.),
        blush: parameters.value("makeup_blush").unwrap_or(1.),
        eyeshadow: parameters.value("makeup_eyeshadow").unwrap_or(1.),
        lipstick_color: Vec3::new(
            parameters.value("lipstick_red").unwrap_or(0.92),
            parameters.value("lipstick_green").unwrap_or(0.43),
            parameters.value("lipstick_blue").unwrap_or(0.53),
        ),
        ..Default::default()
    };
    let mut pixels = if layers.pores == 1.
        && layers.makeup == 1.
        && layers.lipstick == 1.
        && layers.blush == 1.
        && layers.eyeshadow == 1.
        && layers.lipstick_color == SkinLayers::default().lipstick_color
    {
        atlas().to_vec()
    } else {
        build_atlas(layers)
    };
    fn decode(v: u8) -> f32 {
        let v = f32::from(v) / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    fn encode(v: f32) -> u8 {
        let v = v.clamp(0., 1.);
        let v = if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1. / 2.4) - 0.055
        };
        (v * 255.).round() as u8
    }
    let roughness_scale = parameters.value("skin_roughness_scale").unwrap_or(1.);
    let oil_scale = parameters.value("skin_oil_scale").unwrap_or(1.);
    if roughness_scale != 1. || oil_scale != 1. {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let u = (x as f32 + 0.5) / SIZE as f32;
                let v = (y as f32 + 0.5) / SIZE as f32;
                if !(0.05..0.95).contains(&u)
                    || !(0.05..0.95).contains(&v)
                    || crate::female_eyes::is_eye_uv([u * 0.5, v])
                {
                    continue;
                }
                let offset = ((y * WIDTH + x + SIZE) * 4) as usize;
                if roughness_scale != 1. {
                    pixels[offset] = encode(decode(pixels[offset]) * roughness_scale);
                }
                if oil_scale != 1. {
                    pixels[offset + 1] = encode(decode(pixels[offset + 1]) * oil_scale);
                }
            }
        }
    }
    for (key, center, radius, amount) in parameters.material_creases() {
        if amount == 0. {
            continue;
        }
        for side in [-1., 1.] {
            if center.x == 0. && side < 0. {
                continue;
            }
            let center = Vec3::new(center.x * side, center.y, center.z);
            let x0 =
                ((0.05 + 0.9 * (center.x - radius.x + 0.1) / 0.2) * SIZE as f32).max(0.) as u32;
            let x1 = ((0.05 + 0.9 * (center.x + radius.x + 0.1) / 0.2) * SIZE as f32)
                .ceil()
                .min(SIZE as f32) as u32;
            let y0 =
                ((0.05 + 0.9 * (0.81 - center.y - radius.y) / 0.22) * SIZE as f32).max(0.) as u32;
            let y1 = ((0.05 + 0.9 * (0.81 - center.y + radius.y) / 0.22) * SIZE as f32)
                .ceil()
                .min(SIZE as f32) as u32;
            for y in y0..y1 {
                for x in x0..x1 {
                    let u = (x as f32 + 0.5) / SIZE as f32;
                    let v = (y as f32 + 0.5) / SIZE as f32;
                    if crate::female_eyes::is_eye_uv([u * 0.5, v]) {
                        continue;
                    }
                    let point = Vec3::new(
                        (u - 0.05) / 0.9 * 0.2 - 0.1,
                        0.81 - (v - 0.05) / 0.9 * 0.22,
                        center.z,
                    );
                    let weight = (1. - ((point - center) / radius).length_squared())
                        .max(0.)
                        .powi(2);
                    let relief =
                        amount * weight * crate::face_parameters::wrinkle_microprofile(key, point);
                    if relief < 0.001 {
                        continue;
                    }
                    let offset = ((y * WIDTH + x) * 4) as usize;
                    let height = offset + (SIZE * 4) as usize + 2;
                    pixels[height] = encode(decode(pixels[height]) - relief.min(1.) * 0.075);
                }
            }
        }
    }
    pixels
}
#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "builds real background skin atlas and measures polling latency"]
    fn real_atlas_background_polling() {
        let base = crate::face_parameters::FaceParameters::default();
        let changed = base.patched(&serde_json::json!({"lipstick_red": 0.3, "lipstick_green": 0.02, "lipstick_blue": 0.07})).unwrap();
        let mut update = super::AtlasUpdate::default();
        let applied = base.material_signature();
        let start = std::time::Instant::now();
        let mut worst_poll = std::time::Duration::ZERO;
        let mut polls = 0;
        let pixels = loop {
            let before = std::time::Instant::now();
            let result = update.poll(&applied, &changed).unwrap();
            worst_poll = worst_poll.max(before.elapsed());
            polls += 1;
            if let Some((signature, pixels)) = result {
                assert_eq!(signature, changed.material_signature());
                break pixels;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(120));
            std::thread::sleep(std::time::Duration::from_millis(1));
        };
        let elapsed = start.elapsed();
        assert_eq!(pixels, super::atlas_for_parameters(&changed));
        println!(
            "REAL BACKGROUND ATLAS elapsed={elapsed:?} polls={polls} worst_poll={worst_poll:?} bytes={}",
            pixels.len()
        );
    }
    #[test]
    fn atlas_update_does_not_wait_and_discards_stale_results() {
        let parameters = crate::face_parameters::FaceParameters::default();
        let signature = parameters.material_signature();
        let (send, receive) = std::sync::mpsc::channel();
        let job = std::thread::spawn(move || {
            receive.recv().unwrap();
            vec![7]
        });
        let mut update = super::AtlasUpdate {
            job: Some((vec![-1.], job)),
        };
        // This call must return while the worker is waiting for our message.
        assert!(update.poll(&signature, &parameters).unwrap().is_none());
        send.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !update.job.as_ref().unwrap().1.is_finished() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(update.poll(&signature, &parameters).unwrap().is_none());
        assert!(update.job.is_none());
        let job = std::thread::spawn(|| vec![9]);
        while !job.is_finished() {
            std::thread::yield_now();
        }
        update.job = Some((signature.clone(), job));
        let (result, pixels) = update.poll(&[], &parameters).unwrap().unwrap();
        assert_eq!(result, signature);
        assert_eq!(pixels, vec![9]);
    }
    #[test]
    #[ignore = "compares full atlas bytes and reports serial/parallel elapsed time"]
    fn atlas_parallel_matches_serial() {
        let layers = SkinLayers {
            lipstick_color: Vec3::new(0.3, 0.02, 0.07),
            ..Default::default()
        };
        let start = std::time::Instant::now();
        let serial = super::build_atlas_with_workers(layers, 1);
        let serial_time = start.elapsed();
        let start = std::time::Instant::now();
        let parallel = super::build_atlas_with_workers(layers, 8);
        let parallel_time = start.elapsed();
        assert_eq!(serial, parallel);
        println!(
            "ATLAS serial={serial_time:?} parallel_8={parallel_time:?}; bytes={}",
            serial.len()
        );
    }
    #[test]
    fn lipstick_color_preserves_relief_and_finish_and_respects_zero_intensity() {
        let base = SkinLayers::default();
        let changed = SkinLayers {
            lipstick_color: Vec3::new(0.3, 0.02, 0.07),
            ..base
        };
        let p = Vec2::new(0., 0.645);
        let (a, b) = (sample(p, base), sample(p, changed));
        assert!((a.tint - b.tint).length() > 0.1);
        assert_eq!(a.microheight, b.microheight);
        assert_eq!(a.height, b.height);
        assert_eq!(a.roughness, b.roughness);
        assert_eq!(a.sebum, b.sebum);
        assert_eq!(
            sample(
                p,
                SkinLayers {
                    lipstick: 0.,
                    ..base
                }
            )
            .tint,
            sample(
                p,
                SkinLayers {
                    lipstick: 0.,
                    ..changed
                }
            )
            .tint
        );
    }
    #[test]
    fn makeup_regions_are_independent() {
        let base = SkinLayers::default();
        let lip = Vec2::new(0., 0.645);
        let cheek = Vec2::new(0.045, 0.687);
        let lid = Vec2::new(0.034, 0.724);
        for (layers, target, untouched) in [
            (
                SkinLayers {
                    lipstick: 0.,
                    ..base
                },
                lip,
                lid,
            ),
            (SkinLayers { blush: 0., ..base }, cheek, lip),
            (
                SkinLayers {
                    eyeshadow: 0.,
                    ..base
                },
                lid,
                lip,
            ),
        ] {
            assert!((sample(target, base).tint - sample(target, layers).tint).length() > 0.01);
            assert!(
                (sample(untouched, base).tint - sample(untouched, layers).tint).length() < 0.00001
            );
            assert_eq!(
                sample(target, base).microheight,
                sample(target, layers).microheight
            );
        }
    }
    #[test]
    fn layer_controls_change_atlas_without_changing_eye_pigment() {
        let base = super::atlas();
        for key in ["skin_pores", "skin_makeup"] {
            let parameters = crate::face_parameters::FaceParameters::default()
                .patched(&serde_json::json!({key: 0.}))
                .unwrap();
            let changed = super::atlas_for_parameters(&parameters);
            assert_ne!(base, changed.as_slice());
            for y in 0..super::SIZE {
                for x in 0..super::SIZE {
                    let uv = [
                        (x as f32 + 0.5) / super::WIDTH as f32,
                        (y as f32 + 0.5) / super::SIZE as f32,
                    ];
                    if crate::female_eyes::is_eye_uv(uv) {
                        let offset = ((y * super::WIDTH + x) * 4) as usize;
                        assert_eq!(&base[offset..offset + 4], &changed[offset..offset + 4]);
                    }
                }
            }
        }
    }
    #[test]
    fn skin_finish_controls_preserve_pigment_relief_and_alpha() {
        let base = super::atlas();
        let parameters = crate::face_parameters::FaceParameters::default()
            .patched(&serde_json::json!({"skin_roughness_scale":1.4,"skin_oil_scale":0.2}))
            .unwrap();
        let changed = super::atlas_for_parameters(&parameters);
        let mut roughness_changes = 0;
        let mut oil_changes = 0;
        for y in 0..super::SIZE {
            let row = (y * super::WIDTH * 4) as usize;
            let split = row + (super::SIZE * 4) as usize;
            assert_eq!(&base[row..split], &changed[row..split]);
            for x in 0..super::SIZE {
                let offset = split + (x * 4) as usize;
                assert_eq!(
                    &base[offset + 2..offset + 4],
                    &changed[offset + 2..offset + 4]
                );
                roughness_changes += usize::from(base[offset] != changed[offset]);
                oil_changes += usize::from(base[offset + 1] != changed[offset + 1]);
            }
        }
        assert!(roughness_changes > 1000 && oil_changes > 1000);
        assert_ne!(
            parameters.material_signature(),
            crate::face_parameters::FaceParameters::default().material_signature()
        );
    }
    #[test]
    fn pores_cross_cell_edges_without_clipping() {
        let mut occupied_borders = 0;
        for x in -20..20 {
            for y in 740..780 {
                let p = glam::Vec2::new(x as f32 * 0.00085, (y as f32 + 0.37) * 0.00085);
                let a = super::spot(p - glam::Vec2::X * 1e-9, 0.00085, 0.00019, 17, 0.88);
                let b = super::spot(p + glam::Vec2::X * 1e-9, 0.00085, 0.00019, 17, 0.88);
                assert!((a - b).abs() < 0.001);
                occupied_borders += usize::from(a > 0.1);
            }
        }
        assert!(
            occupied_borders > 50,
            "pore centers still avoid cell boundaries"
        );
    }
    #[test]
    fn parameterized_creases_preserve_pigmentation_and_change_microheight() {
        let parameters =
            crate::face_parameters::FaceParameters::from_json(r#"{"wrinkles_forehead":0.8}"#)
                .unwrap();
        let base = super::atlas();
        let changed = super::atlas_for_parameters(&parameters);
        let mut heights = 0;
        for y in 0..super::SIZE {
            for x in 0..super::WIDTH {
                let offset = ((y * super::WIDTH + x) * 4) as usize;
                if x < super::SIZE {
                    assert_eq!(&base[offset..offset + 4], &changed[offset..offset + 4]);
                } else {
                    assert_eq!(&base[offset..offset + 2], &changed[offset..offset + 2]);
                    assert_eq!(base[offset + 3], changed[offset + 3]);
                    heights += usize::from(base[offset + 2] != changed[offset + 2]);
                }
            }
        }
        assert!(heights > 1000);
        assert_eq!(base, super::atlas_for_parameters(&Default::default()));
    }
    #[test]
    fn freckle_search_is_continuous_at_cell_boundaries() {
        for cell in -8..8 {
            for row in 145..160 {
                let x = cell as f32 * 0.0045;
                let y = row as f32 * 0.0045 + 0.001;
                assert!(
                    (super::freckles(glam::Vec2::new(x - 1e-8, y))
                        - super::freckles(glam::Vec2::new(x + 1e-8, y)))
                    .abs()
                        < 0.001
                );
            }
        }
    }
    use super::*;
    fn off() -> SkinLayers {
        SkinLayers {
            pigmentation: 0.,
            pores: 0.,
            pimples: 0.,
            scars: 0.,
            freckles: 0.,
            makeup: 0.,
            lipstick: 1.,
            blush: 1.,
            eyeshadow: 1.,
            lipstick_color: Vec3::new(0.92, 0.43, 0.53),
            moles: 0.,
            wrinkles: 0.,
            vessels: 0.,
        }
    }
    #[test]
    fn all_layers_have_independent_visible_effects() {
        for layer in 0..9 {
            let mut layers = off();
            match layer {
                0 => layers.pores = 1.,
                1 => layers.pimples = 1.,
                2 => layers.scars = 1.,
                3 => layers.freckles = 1.,
                4 => layers.makeup = 1.,
                5 => layers.moles = 1.,
                6 => layers.wrinkles = 1.,
                7 => layers.vessels = 1.,
                _ => layers.pigmentation = 1.,
            }
            let mut changed = 0;
            for y in 0..400 {
                for x in 0..400 {
                    let p = Vec2::new(-0.085 + x as f32 * 0.000425, 0.615 + y as f32 * 0.00045);
                    let s = sample(p, layers);
                    assert!(
                        s.tint.is_finite()
                            && s.tint.min_element() >= 0.
                            && s.tint.max_element() <= 1.
                    );
                    assert!(s.height.abs() < 0.0008);
                    if s.tint.distance(Vec3::ONE) > 0.01 {
                        changed += 1;
                    }
                }
            }
            assert!(changed > 20, "layer {layer} had only {changed} samples");
        }
    }
    #[test]
    fn neutral_surface_and_nonface_mapping_are_untouched() {
        let s = sample(Vec2::new(0.04, 0.69), off());
        assert_eq!(s.tint, Vec3::ONE);
        assert_eq!(s.height, 0.);
        assert_eq!(uv(Vec3::ZERO), WHITE_UV);
        assert_eq!(uv(Vec3::new(0., 0.70, -0.03)), WHITE_UV);
        let coordinates = uv(Vec3::new(0., 0.70, 0.14));
        assert!(coordinates.iter().all(|x| (0.05..0.95).contains(x)));
        assert_eq!(
            sample(Vec2::new(0.09, 0.61), SkinLayers::default()).height,
            0.
        );
    }
    #[test]
    fn blemishes_have_signed_relief() {
        assert!(sample(Vec2::new(-0.048, 0.670), SkinLayers::default()).height > 0.0004);
        assert!(sample(Vec2::new(0.0585, 0.675), SkinLayers::default()).height < -0.0001);
    }
}
