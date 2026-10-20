//! Continuation beyond the end-of-street colonnade: a paved lane between garden walls
//! leading to a walled plot with a modern villa and a swimming pool. The basin is
//! ordinary geometry; the water itself is a `voxy_water` basin rendered by the scene's
//! FFT water pass.
use super::*;
use voxy_render::voxy_water::WaterBody;

/// Mesh ids shared with the street.
pub(super) struct Parts {
    pub curb: usize,
    pub trim: usize,
    pub metal: usize,
    pub timber: usize,
    pub glass: usize,
    pub glow: usize,
    pub trunk: usize,
    pub leaf: usize,
    pub foliage: usize,
    pub bulb: usize,
    pub floor: usize,
    pub ceiling: usize,
    pub fabrics: Vec<usize>,
    pub ceramic: usize,
    pub wood: usize,
}

pub(super) struct Villa {
    pub water: Vec<WaterBody>,
    /// Eye and view direction overlooking the pool from the garden gate.
    pub view: (Vec3, Vec3),
}

/// Basin interior; the water level sits 12 cm below the deck.
const POOL_X: f32 = 2.4;
const POOL_Z: [f32; 2] = [113.0, 127.0];
const DECK: f32 = 0.20;
const WATER: f32 = DECK - 0.12;
const SHALLOW: f32 = -1.0;
const DEEP: f32 = -1.9;
/// Shallow end, ramp and deep end along z.
const RAMP: [f32; 2] = [118.0, 121.0];

impl Builder {
    /// Axis-aligned box from its minimum and maximum corners.
    fn span(&mut self, id: usize, lo: [f32; 3], hi: [f32; 3]) {
        let (lo, hi) = (Vec3::from_array(lo), Vec3::from_array(hi));
        self.put(id, (lo + hi) * 0.5, hi - lo, Quat::IDENTITY);
    }
}

fn tree(b: &mut Builder, p: &Parts, root: Vec3, height: f32, radius: f32, leaves: usize) {
    let fork = root + Vec3::new((b.random() - 0.5) * 0.3, height * 0.45, (b.random() - 0.5) * 0.3);
    b.beam(p.trunk, root, fork, 0.06 + height * 0.025);
    for branch in 0..8 {
        let a = branch as f32 * 2.399963 + b.random() * 0.8;
        let elevation = b.random();
        let direction = Vec3::new(a.cos(), 0., a.sin());
        let reach = radius * (0.6 + 0.4 * b.random());
        let origin = fork + Vec3::Y * (elevation - 0.3) * height * 0.12;
        let elbow = origin + direction * reach * 0.5 + Vec3::Y * height * (0.06 + elevation * 0.08);
        let tip = root + direction * reach + Vec3::Y * height * (0.72 + 0.25 * elevation);
        b.beam(p.trunk, origin, elbow, 0.02 + height * 0.01);
        b.beam(p.trunk, elbow, tip, 0.015 + height * 0.006);
        for twig in 0..6 {
            let theta = a + twig as f32 * 2.399963;
            let start = elbow.lerp(tip, 0.2 + twig as f32 * 0.13);
            let end = start
                + Vec3::new(theta.cos(), 0., theta.sin()) * radius * 0.3
                + Vec3::Y * (0.1 + b.random() * 0.3) * radius * 0.4;
            b.beam(p.trunk, start, end, 0.012);
            for _ in 0..leaves {
                let along = b.random();
                let offset = Vec3::new(b.random() - 0.5, (b.random() - 0.5) * 0.7, b.random() - 0.5) * radius * 0.55;
                let size = 0.045 + b.random() * 0.03;
                let tilt = Quat::from_euler(
                    glam::EulerRot::YXZ,
                    b.random() * std::f32::consts::TAU,
                    b.random() * 2.4 - 1.2,
                    b.random() * 2.4 - 1.2,
                );
                b.put(p.leaf, start.lerp(end, along) + offset, Vec3::new(size * 1.2, size, size * 2.), tilt);
            }
        }
    }
}

/// A clipped hedge built from slightly irregular foliage blocks along x or z.
fn hedge(b: &mut Builder, p: &Parts, from: Vec3, to: Vec3, height: f32, depth: f32) {
    let along = to - from;
    let pieces = (along.length() / 0.9).ceil().max(1.) as usize;
    let yaw = Quat::from_rotation_y(along.x.atan2(along.z));
    for i in 0..pieces {
        let c = from.lerp(to, (i as f32 + 0.5) / pieces as f32);
        let h = height * (0.94 + b.random() * 0.1);
        let d = depth * (0.92 + b.random() * 0.14);
        let len = along.length() / pieces as f32 + 0.12;
        b.put(p.foliage, c + Vec3::Y * h * 0.5, Vec3::new(d, h, len), yaw);
    }
}

fn bollard(b: &mut Builder, p: &Parts, x: f32, ground: f32, z: f32, light: bool) {
    b.span(p.metal, [x - 0.07, ground, z - 0.07], [x + 0.07, ground + 0.62, z + 0.07]);
    b.span(p.glow, [x - 0.055, ground + 0.5, z - 0.055], [x + 0.055, ground + 0.6, z + 0.055]);
    b.span(p.metal, [x - 0.09, ground + 0.62, z - 0.09], [x + 0.09, ground + 0.66, z + 0.09]);
    if light {
        b.lights.push(ArchiveLight {
            position_kind: [x, ground + 0.55, z, 5.],
            colour_intensity: [1., 0.7, 0.42, 6.],
        });
    }
}

/// Sun lounger with its foot towards the pool (at `x0`), head towards `x1`.
fn lounger(b: &mut Builder, p: &Parts, x0: f32, x1: f32, z: f32, cushion: usize) {
    let s = (x1 - x0).signum();
    let flat_end = x0 + (x1 - x0) * 0.68;
    let y = DECK + 0.34;
    for dz in [-0.31, 0.31] {
        b.span(p.timber, [x0.min(flat_end), y - 0.05, z + dz - 0.025], [x0.max(flat_end), y, z + dz + 0.025]);
        for x in [x0 + s * 0.08, flat_end - s * 0.08] {
            b.span(p.timber, [x - 0.03, DECK, z + dz - 0.03], [x + 0.03, y - 0.05, z + dz + 0.03]);
        }
    }
    let slats = ((flat_end - x0).abs() / 0.09) as usize;
    for n in 0..slats {
        let x = x0 + s * (0.045 + n as f32 * 0.09);
        b.span(p.timber, [x - 0.035, y, z - 0.33], [x + 0.035, y + 0.025, z + 0.33]);
    }
    let length = (flat_end - x0).abs();
    b.block(cushion, [(x0 + flat_end) * 0.5, y + 0.06, z], [length - 0.04, 0.07, 0.6]);
    // Backrest raised 40 degrees, hinged at the end of the flat part.
    let back = (x1 - flat_end).abs() + 0.15;
    let tilt = Quat::from_rotation_z(s * 0.7);
    let dir = tilt * Vec3::new(s, 0., 0.);
    let hinge = Vec3::new(flat_end, y + 0.02, z);
    b.put(p.timber, hinge + dir * back * 0.5, Vec3::new(back, 0.03, 0.64), tilt);
    b.put(cushion, hinge + dir * back * 0.5 + tilt * Vec3::Y * 0.05, Vec3::new(back - 0.05, 0.07, 0.6), tilt);
}

pub(super) fn build(b: &mut Builder, p: &Parts) -> Villa {
    let slab_m = b.material([0.47, 0.45, 0.41], 0.8, 0., 1., 0., 0.);
    let slab = b.bevel_cube_e(slab_m, 0.46);
    let travertine_m = b.material([0.76, 0.70, 0.60], 0.68, 0., 1., 0., 0.);
    let travertine = b.bevel_cube_e(travertine_m, 0.47);
    let concrete_m = b.material([0.44, 0.43, 0.41], 0.9, 0., 1., 0., 0.);
    let concrete = b.cube(concrete_m);
    let lawn_m = b.material([0.085, 0.16, 0.045], 0.97, 0., 1., 0., 0.);
    let lawn = b.cube(lawn_m);
    let mosaic_m = b.material([0.52, 0.76, 0.80], 0.22, 0., 9., 0., 0.);
    let mosaic = b.cube(mosaic_m);
    let mosaic_dark_m = b.material([0.05, 0.16, 0.34], 0.22, 0., 9., 0., 0.);
    let mosaic_dark = b.cube(mosaic_dark_m);
    let chrome_m = b.material([0.93, 0.93, 0.95], 0.1, 1., 0., 0., 0.);
    let chrome = b.round(chrome_m, false);
    let chrome_box = b.cube(chrome_m);
    let pool_light_m = b.material([0.75, 0.92, 1.], 0.3, 0., 0., 2600., 0.);
    let pool_light = b.round(pool_light_m, false);
    let render_m = b.material([0.86, 0.85, 0.82], 0.9, 0., 1., 0., 0.);
    let render = b.cube(render_m);
    let cladding_m = b.material([0.34, 0.2, 0.1], 0.6, 0., 4., 0., 0.);
    let cladding = b.cube(cladding_m);
    let wall_m = b.material([0.74, 0.70, 0.62], 0.9, 0., 1., 0., 0.);
    let garden_wall = b.cube(wall_m);
    let canvas_m = b.material([0.88, 0.84, 0.74], 0.9, 0., 4., 0., 0.);
    let canvas = b.round(canvas_m, false);
    let cushion_m = b.material([0.88, 0.87, 0.82], 0.95, 0., 4., 0., 0.);
    let cushion = b.cube(cushion_m);
    let towel_m = b.material([0.08, 0.32, 0.5], 0.95, 0., 4., 0., 0.);
    let towel = b.cube(towel_m);

    // ---- forecourt and lane: large staggered slabs on a mortar bed ----
    b.span(concrete, [-16., -0.13, 83.], [16., 0.04, 104.]);
    for row in 0..6 {
        let z = 83.05 + row as f32 * 0.88 + 0.44;
        let offset = if row % 2 == 0 { 0. } else { 0.3 };
        for col in 0..54 {
            let x = -16.2 + offset + col as f32 * 0.6;
            let h = 0.05 + b.aged() * 0.006;
            let yaw = Quat::from_rotation_y((b.aged() - 0.5) * 0.01);
            b.put(slab, Vec3::new(x, 0.06 - h * 0.5, z), Vec3::new(0.585, h, 0.865), yaw);
        }
    }
    for row in 0..18 {
        let z = 88.32 + row as f32 * 0.87 + 0.435;
        let offset = if row % 2 == 0 { 0. } else { 0.3 };
        for col in 0..8 {
            let x = -2.1 + offset + col as f32 * 0.6;
            if x.abs() > 2.38 {
                continue;
            }
            let h = 0.05 + b.aged() * 0.006;
            b.put(slab, Vec3::new(x, 0.06 - h * 0.5, z), Vec3::new(0.585, h, 0.855), Quat::IDENTITY);
        }
    }
    for side in [-1., 1.] {
        let s = |x: f32| side * x;
        // edging kerb, lawn strip, hedge and a rendered garden wall with coping
        b.block(p.curb, [s(2.55), 0.06, 96.2], [0.3, 0.14, 15.7]);
        b.span(lawn, [s(2.7).min(s(16.)), 0.04, 88.3], [s(2.7).max(s(16.)), 0.1, 104.]);
        hedge(b, p, Vec3::new(s(3.55), 0.1, 88.6), Vec3::new(s(3.55), 0.1, 103.6), 1.15, 0.7);
        b.span(garden_wall, [s(4.05).min(s(4.35)), 0.04, 88.3], [s(4.05).max(s(4.35)), 2.0, 104.]);
        b.block(p.trim, [s(4.2), 2.04, 96.15], [0.42, 0.08, 15.7]);
        for z in [91.5, 99.5] {
            bollard(b, p, s(2.62), 0.06, z, true);
        }
        for z in [95.5, 103.] {
            bollard(b, p, s(2.62), 0.06, z, false);
        }
        let root = Vec3::new(s(8. + b.random() * 2.), 0.1, 92. + b.random() * 2.);
        tree(b, p, root, 6.5, 2.4, 44);
        let root = Vec3::new(s(10. + b.random() * 2.), 0.1, 99.5 + b.random() * 2.);
        tree(b, p, root, 5.8, 2.1, 44);
    }

    // ---- villa plot: walls, gate and lawn around the basin ----
    for side in [-1., 1.] {
        let s = |x: f32| side * x;
        b.span(garden_wall, [s(1.9).min(s(16.)), 0.04, 103.85], [s(1.9).max(s(16.)), 2.2, 104.15]);
        b.span(garden_wall, [s(15.7).min(s(16.)), 0.04, 104.], [s(15.7).max(s(16.)), 2.2, 152.]);
        b.block(p.trim, [s(8.95), 2.24, 104.], [14.1, 0.08, 0.42]);
        b.block(p.trim, [s(15.85), 2.24, 128.], [0.42, 0.08, 48.]);
        // gate piers and an open steel gate leaf folded back against the wall
        b.span(render, [s(1.6).min(s(1.95)), 0.04, 103.75], [s(1.6).max(s(1.95)), 2.45, 104.25]);
        b.block(p.trim, [s(1.775), 2.5, 104.], [0.45, 0.1, 0.6]);
        let (x0, x1) = (s(2.0), s(3.45));
        for y in [0.15, 1.8] {
            b.span(p.metal, [x0.min(x1), y, 104.33], [x0.max(x1), y + 0.05, 104.38]);
        }
        for n in 0..12 {
            let x = x0 + (x1 - x0) * (n as f32 + 0.5) / 12.;
            b.span(p.metal, [x - 0.012, 0.15, 104.343], [x + 0.012, 1.85, 104.367]);
        }
        hedge(b, p, Vec3::new(s(15.05), 0.12, 104.8), Vec3::new(s(15.05), 0.12, 131.), 1.5, 0.9);
    }
    b.span(garden_wall, [-16., 0.04, 151.7], [16., 2.2, 152.]);
    b.block(p.trim, [0., 2.24, 151.85], [32., 0.08, 0.42]);
    let (bx, bz) = (POOL_X + 0.3, [POOL_Z[0] - 0.3, POOL_Z[1] + 0.3]);
    b.span(lawn, [-15.7, -0.08, 104.15], [15.7, 0.12, bz[0]]);
    b.span(lawn, [-15.7, -0.08, bz[1]], [15.7, 0.12, 151.7]);
    b.span(lawn, [-15.7, -0.08, bz[0]], [-bx, 0.12, bz[1]]);
    b.span(lawn, [bx, -0.08, bz[0]], [15.7, 0.12, bz[1]]);
    for n in 0..7 {
        let z = 104.7 + n as f32 * 0.78;
        let x = if n % 2 == 0 { -0.12 } else { 0.12 };
        let yaw = Quat::from_rotation_y((b.random() - 0.5) * 0.12);
        b.put(travertine, Vec3::new(x, 0.135, z), Vec3::new(0.9, 0.05, 0.5), yaw);
    }
    for z in [106., 109.] {
        for x in [-0.95, 0.95] {
            bollard(b, p, x, 0.12, z, z > 108. && x > 0.);
        }
    }

    // ---- pool deck: travertine on a concrete bed, open over the basin ----
    // The tile grid starts at the coping, so every cut edge is hidden under it.
    let tile = 0.7;
    let (cx, cz) = (bx + 0.04, [bz[0] - 0.04, bz[1] + 0.04]);
    let deck = ([-cx - 6. * tile, cz[0] - 4. * tile], [cx + 6. * tile, cz[1] + 7. * tile]);
    let hole = ([-bx, bz[0]], [bx, bz[1]]);
    b.span(concrete, [deck.0[0], 0.08, deck.0[1]], [hole.0[0], 0.17, deck.1[1]]);
    b.span(concrete, [hole.1[0], 0.08, deck.0[1]], [deck.1[0], 0.17, deck.1[1]]);
    b.span(concrete, [hole.0[0], 0.08, deck.0[1]], [hole.1[0], 0.17, hole.0[1]]);
    b.span(concrete, [hole.0[0], 0.08, hole.1[1]], [hole.1[0], 0.17, deck.1[1]]);
    let middle = 2. * cx / 8.;
    let mut columns: Vec<(f32, f32, bool)> = Vec::new();
    for i in 0..6 {
        let offset = cx + (i as f32 + 0.5) * tile;
        columns.push((-offset, tile, false));
        columns.push((offset, tile, false));
    }
    for i in 0..8 {
        columns.push((-cx + (i as f32 + 0.5) * middle, middle, true));
    }
    for row in 0..((deck.1[1] - deck.0[1]) / tile).round() as usize {
        let z = deck.0[1] + (row as f32 + 0.5) * tile;
        let beside_pool = z > cz[0] && z < cz[1];
        for &(x, width, inner) in &columns {
            if inner && beside_pool {
                continue;
            }
            let h = 0.07 + b.aged() * 0.004;
            b.put(travertine, Vec3::new(x, DECK - h * 0.5, z), Vec3::new(width - 0.012, h, tile - 0.012), Quat::IDENTITY);
        }
    }

    // ---- basin: mosaic walls and floors, ramp from the shallow to the deep end ----
    let (z0, z1) = (POOL_Z[0], POOL_Z[1]);
    let x = POOL_X;
    let bottom = DEEP - 0.3;
    b.span(mosaic, [-x - 0.3, bottom, z0 - 0.3], [-x, 0.17, z1 + 0.3]);
    b.span(mosaic, [x, bottom, z0 - 0.3], [x + 0.3, 0.17, z1 + 0.3]);
    b.span(mosaic, [-x, bottom, z0 - 0.3], [x, 0.17, z0]);
    b.span(mosaic, [-x, bottom, z1], [x, 0.17, z1 + 0.3]);
    b.span(mosaic, [-x, SHALLOW - 0.25, z0], [x, SHALLOW, RAMP[0]]);
    b.span(mosaic, [-x, DEEP - 0.25, RAMP[1]], [x, DEEP, z1]);
    let fall = SHALLOW - DEEP;
    let run = RAMP[1] - RAMP[0];
    let angle = (fall / run).atan();
    let ramp_rotation = Quat::from_rotation_x(angle);
    let ramp_normal = ramp_rotation * Vec3::Y;
    let ramp_centre = Vec3::new(0., (SHALLOW + DEEP) * 0.5, (RAMP[0] + RAMP[1]) * 0.5);
    let ramp_length = (fall * fall + run * run).sqrt();
    b.put(mosaic, ramp_centre - ramp_normal * 0.125, Vec3::new(2. * x, 0.25, ramp_length + 0.06), ramp_rotation);
    // waterline band and a dark lane line along the floor
    for (lo, hi) in [
        ([-x, -0.12, z0], [-x + 0.004, 0.17, z1]),
        ([x - 0.004, -0.12, z0], [x, 0.17, z1]),
        ([-x, -0.12, z0], [x, 0.17, z0 + 0.004]),
        ([-x, -0.12, z1 - 0.004], [x, 0.17, z1]),
    ] {
        b.span(mosaic_dark, lo, hi);
    }
    b.span(mosaic_dark, [-0.13, SHALLOW, 114.9], [0.13, SHALLOW + 0.004, RAMP[0]]);
    b.span(mosaic_dark, [-0.13, DEEP, RAMP[1]], [0.13, DEEP + 0.004, z1 - 1.0]);
    b.put(mosaic_dark, ramp_centre + ramp_normal * 0.002, Vec3::new(0.26, 0.004, ramp_length), ramp_rotation);
    b.span(mosaic_dark, [-0.5, DEEP, z1 - 1.0], [0.5, DEEP + 0.004, z1 - 0.74]);
    // full-width entry steps at the shallow end, nosings picked out in dark mosaic
    for (n, top) in [-0.16f32, -0.44, -0.72].into_iter().enumerate() {
        let za = z0 + n as f32 * 0.4;
        b.span(mosaic, [-x, SHALLOW, za], [x, top, za + 0.4]);
        b.span(mosaic_dark, [-x, top, za + 0.34], [x, top + 0.004, za + 0.4]);
    }
    // coping: bevelled travertine overhanging the water by 4 cm
    let coping = |b: &mut Builder, lo: [f32; 2], hi: [f32; 2]| {
        let length = (hi[0] - lo[0]).max(hi[1] - lo[1]);
        let pieces = (length / 0.6).round().max(1.) as usize;
        for i in 0..pieces {
            let t0 = i as f32 / pieces as f32;
            let t1 = (i + 1) as f32 / pieces as f32;
            let (a, c) = if hi[0] - lo[0] > hi[1] - lo[1] {
                ([lo[0] + (hi[0] - lo[0]) * t0, lo[1]], [lo[0] + (hi[0] - lo[0]) * t1, hi[1]])
            } else {
                ([lo[0], lo[1] + (hi[1] - lo[1]) * t0], [hi[0], lo[1] + (hi[1] - lo[1]) * t1])
            };
            let centre = Vec3::new((a[0] + c[0]) * 0.5, DECK, (a[1] + c[1]) * 0.5);
            b.put(travertine, centre, Vec3::new(c[0] - a[0] - 0.008, 0.07, c[1] - a[1] - 0.008), Quat::IDENTITY);
        }
    };
    coping(b, [-x - 0.34, z0 - 0.34], [-x + 0.04, z1 + 0.34]);
    coping(b, [x - 0.04, z0 - 0.34], [x + 0.34, z1 + 0.34]);
    coping(b, [-x + 0.04, z0 - 0.34], [x - 0.04, z0 + 0.04]);
    coping(b, [-x + 0.04, z1 - 0.04], [x - 0.04, z1 + 0.34]);
    // underwater lights in the deep-end wall
    for lx in [-1.2, 1.2] {
        b.put(pool_light, Vec3::new(lx, -0.75, z1 - 0.012), Vec3::new(0.26, 0.024, 0.26), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
        b.lights.push(ArchiveLight {
            position_kind: [lx, -0.8, z1 - 0.35, 6.],
            colour_intensity: [0.72, 0.9, 1., 22.],
        });
    }
    // ladder over the coping at the deep end
    for dz in [-0.24, 0.24] {
        let z = 124. + dz;
        b.beam(chrome, Vec3::new(x - 0.17, -1.3, z), Vec3::new(x - 0.17, 0.52, z), 0.04);
        b.beam(chrome, Vec3::new(x - 0.17, 0.52, z), Vec3::new(x + 0.12, 0.64, z), 0.04);
        b.beam(chrome, Vec3::new(x + 0.12, 0.64, z), Vec3::new(x + 0.42, 0.52, z), 0.04);
        b.beam(chrome, Vec3::new(x + 0.42, 0.52, z), Vec3::new(x + 0.42, DECK, z), 0.04);
        b.span(chrome_box, [x - 0.2, -1.3, z - 0.02], [x, -1.26, z + 0.02]);
    }
    for y in [-0.25f32, -0.55, -0.85] {
        b.span(chrome_box, [x - 0.24, y, 123.73], [x - 0.08, y + 0.03, 124.27]);
    }
    // outdoor shower beside the deep end
    b.beam(chrome, Vec3::new(-3.4, DECK, 128.3), Vec3::new(-3.4, 2.3, 128.3), 0.05);
    b.beam(chrome, Vec3::new(-3.4, 2.3, 128.3), Vec3::new(-3.05, 2.32, 128.3), 0.03);
    b.put(chrome, Vec3::new(-3.0, 2.27, 128.3), Vec3::new(0.2, 0.03, 0.2), Quat::IDENTITY);
    b.span(slab, [-3.75, DECK, 127.95], [-3.05, DECK + 0.02, 128.65]);

    // ---- deck furniture ----
    for (n, z) in [115.2, 117.8, 120.4, 123.0].into_iter().enumerate() {
        lounger(b, p, 3.25, 5.3, z, cushion);
        if n % 2 == 0 {
            b.span(towel, [3.4, DECK + 0.47, z - 0.27], [4.0, DECK + 0.49, z + 0.27]);
        }
    }
    for z in [116.5, 121.7] {
        b.span(p.timber, [5.45, DECK, z - 0.22], [5.85, DECK + 0.42, z + 0.22]);
        b.span(p.ceramic, [5.47, DECK + 0.42, z - 0.2], [5.83, DECK + 0.44, z + 0.2]);
    }
    let (ux, uz) = (-4.9, 120.);
    b.span(concrete, [ux - 0.25, DECK, uz - 0.25], [ux + 0.25, DECK + 0.12, uz + 0.25]);
    b.beam(p.trunk, Vec3::new(ux, DECK, uz), Vec3::new(ux, DECK + 2.55, uz), 0.05);
    b.put(canvas, Vec3::new(ux, DECK + 2.42, uz), Vec3::new(3.0, 0.05, 3.0), Quat::IDENTITY);
    b.put(canvas, Vec3::new(ux, DECK + 2.5, uz), Vec3::new(2.0, 0.08, 2.0), Quat::IDENTITY);
    b.put(canvas, Vec3::new(ux, DECK + 2.58, uz), Vec3::new(0.9, 0.08, 0.9), Quat::IDENTITY);
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::FRAC_PI_4;
        let tip = Vec3::new(ux + a.cos() * 1.45, DECK + 2.37, uz + a.sin() * 1.45);
        b.beam(p.metal, Vec3::new(ux, DECK + 2.6, uz), tip, 0.012);
    }
    // dining set beside the parasol
    let (tx, tz) = (-4.9, 124.6);
    b.span(p.timber, [tx - 0.5, DECK + 0.72, tz - 0.8], [tx + 0.5, DECK + 0.76, tz + 0.8]);
    for (dx, dz) in [(-0.42, -0.72), (0.42, -0.72), (-0.42, 0.72), (0.42, 0.72)] {
        b.span(p.metal, [tx + dx - 0.02, DECK, tz + dz - 0.02], [tx + dx + 0.02, DECK + 0.72, tz + dz + 0.02]);
    }
    for (cx, cz) in [(-0.85, -0.4), (-0.85, 0.4), (0.85, -0.4), (0.85, 0.4)] {
        let (x0, z0c) = (tx + cx, tz + cz);
        b.span(p.timber, [x0 - 0.22, DECK + 0.44, z0c - 0.22], [x0 + 0.22, DECK + 0.47, z0c + 0.22]);
        let back = if cx < 0. { x0 - 0.22 } else { x0 + 0.19 };
        b.span(p.timber, [back, DECK + 0.47, z0c - 0.22], [back + 0.03, DECK + 0.9, z0c + 0.22]);
        for (lx, lz) in [(-0.19, -0.19), (0.19, -0.19), (-0.19, 0.19), (0.19, 0.19)] {
            b.span(p.metal, [x0 + lx - 0.015, DECK, z0c + lz - 0.015], [x0 + lx + 0.015, DECK + 0.44, z0c + lz + 0.015]);
        }
    }
    // planted olive trees in concrete planters at the deck corners
    for (px, pz) in [(-6.3, 110.7), (6.3, 110.7), (-6.3, 131.4), (6.3, 131.4)] {
        b.span(concrete, [px - 0.45, DECK, pz - 0.45], [px + 0.45, DECK + 0.62, pz + 0.45]);
        b.span(lawn, [px - 0.4, DECK + 0.62, pz - 0.4], [px + 0.4, DECK + 0.64, pz + 0.4]);
        tree(b, p, Vec3::new(px, DECK + 0.62, pz), 2.6, 0.95, 30);
    }

    // ---- villa: glazed ground floor, cantilevered upper storey, flat roof ----
    let (vx, vz) = (12.0f32, [deck.1[1], 148.0f32]);
    let floor_top = DECK + 0.02;
    b.span(concrete, [-vx, -0.08, vz[0]], [vx, 0.17, vz[1]]);
    b.span(travertine, [-vx, 0.13, vz[0]], [vx, DECK, 134.]);
    b.span(p.floor, [-vx + 0.3, 0.13, 134.3], [vx - 0.3, floor_top, vz[1] - 0.3]);
    let g = 3.3;
    b.span(render, [-vx, DECK, vz[1] - 0.3], [vx, g, vz[1]]);
    b.span(render, [-vx, DECK, 134.], [-vx + 0.3, g, vz[1]]);
    b.span(render, [vx - 0.3, DECK, 134.], [vx, g, vz[1]]);
    b.span(render, [-vx, DECK, 134.], [-7.2, g, 134.3]);
    b.span(cladding, [-vx, DECK + 0.02, 133.97], [-7.25, g - 0.05, 134.]);
    // sliding glass doors between slim aluminium frames
    let mullions: Vec<f32> = (0..=8).map(|n| -7.2 + n as f32 * 2.4).collect();
    for &mx in &mullions {
        b.span(p.metal, [mx - 0.04, DECK, 134.05], [mx + 0.04, g, 134.25]);
    }
    b.span(p.metal, [-7.2, DECK, 134.05], [vx, DECK + 0.06, 134.25]);
    b.span(p.metal, [-7.2, g - 0.1, 134.05], [vx, g, 134.25]);
    for pair in mullions.windows(2) {
        b.span(p.glass, [pair[0] + 0.04, DECK + 0.06, 134.135], [pair[1] - 0.04, g - 0.1, 134.165]);
    }
    // floor slab and fascia; the upper box overhangs the terrace
    b.span(render, [-vx, g, vz[0]], [vx, g + 0.32, vz[1]]);
    b.span(p.ceiling, [-vx + 0.3, g - 0.02, 134.3], [vx - 0.3, g, vz[1] - 0.3]);
    let (ux0, uz1) = (-9.0f32, 146.5f32);
    let u0 = g + 0.32;
    let u1 = u0 + 3.0;
    b.span(render, [ux0, u0, uz1 - 0.3], [vx, u1, uz1]);
    b.span(render, [ux0, u0, vz[0]], [ux0 + 0.3, u1, uz1]);
    b.span(render, [vx - 0.3, u0, vz[0]], [vx, u1, uz1]);
    b.span(render, [ux0, u0, vz[0]], [vx, u0 + 0.8, vz[0] + 0.3]);
    b.span(render, [ux0, u1 - 0.4, vz[0]], [vx, u1, vz[0] + 0.3]);
    let ribbon: Vec<f32> = (0..=7).map(|n| ux0 + 0.3 + n as f32 * (vx - 0.3 - ux0 - 0.3) / 7.).collect();
    for &mx in &ribbon {
        b.span(p.metal, [mx - 0.035, u0 + 0.8, vz[0] + 0.05], [mx + 0.035, u1 - 0.4, vz[0] + 0.22]);
    }
    for pair in ribbon.windows(2) {
        b.span(p.glass, [pair[0] + 0.035, u0 + 0.8, vz[0] + 0.12], [pair[1] - 0.035, u1 - 0.4, vz[0] + 0.15]);
    }
    b.span(p.floor, [ux0 + 0.3, u0, vz[0] + 0.3], [vx - 0.3, u0 + 0.02, uz1 - 0.3]);
    b.span(p.ceiling, [ux0 + 0.3, u1 - 0.02, vz[0] + 0.3], [vx - 0.3, u1, uz1 - 0.3]);
    b.span(render, [ux0 - 0.25, u1, vz[0] - 0.25], [vx + 0.25, u1 + 0.3, uz1 + 0.25]);
    b.span(p.metal, [ux0 - 0.27, u1 + 0.3, vz[0] - 0.27], [vx + 0.27, u1 + 0.36, uz1 + 0.27]);
    // roof terrace edge over the single-storey west wing
    b.span(p.glass, [-vx, u0, vz[0] + 0.05], [ux0, u0 + 1.0, vz[0] + 0.08]);
    b.span(p.glass, [-vx + 0.05, u0, vz[0]], [-vx + 0.08, u0 + 1.0, vz[1]]);
    b.span(p.metal, [-vx, u0 + 1.0, vz[0]], [ux0, u0 + 1.05, vz[0] + 0.12]);
    // terrace and wall lights
    for wx in [-4.8, 4.8] {
        b.span(p.glow, [wx - 0.12, g - 0.04, 133.2], [wx + 0.12, g, 133.44]);
    }
    b.lights.push(ArchiveLight {
        position_kind: [0., g - 0.3, 133.2, 7.],
        colour_intensity: [1., 0.72, 0.45, 14.],
    });

    // ---- interior seen through the glass ----
    let fabric = |i: usize| p.fabrics[i % p.fabrics.len()];
    // living area: L-shaped sofa, rug, coffee table
    b.span(fabric(1), [-1.2, floor_top, 139.6], [2.6, floor_top + 0.42, 140.5]);
    b.span(fabric(1), [-1.2, floor_top + 0.42, 140.3], [2.6, floor_top + 0.85, 140.55]);
    b.span(fabric(1), [2.6, floor_top, 137.2], [3.5, floor_top + 0.42, 140.55]);
    b.span(fabric(1), [3.3, floor_top + 0.42, 137.2], [3.55, floor_top + 0.85, 140.55]);
    b.span(fabric(2), [-1.4, floor_top, 136.6], [2.4, floor_top + 0.012, 139.3]);
    b.span(p.timber, [-0.2, floor_top, 137.4], [1.4, floor_top + 0.38, 138.4]);
    // dining table with chairs and a pendant
    b.span(p.timber, [-6.0, floor_top + 0.72, 138.6], [-3.6, floor_top + 0.76, 139.6]);
    for (lx, lz) in [(-5.85, 138.7), (-3.75, 138.7), (-5.85, 139.5), (-3.75, 139.5)] {
        b.span(p.metal, [lx - 0.03, floor_top, lz - 0.03], [lx + 0.03, floor_top + 0.72, lz + 0.03]);
    }
    for (n, cx) in [-5.5f32, -4.8, -4.1].into_iter().enumerate() {
        for (cz, back) in [(138.15, 137.95), (140.05, 140.25)] {
            b.span(fabric(3 + n), [cx - 0.22, floor_top + 0.45, cz - 0.2], [cx + 0.22, floor_top + 0.5, cz + 0.2]);
            b.span(fabric(3 + n), [cx - 0.22, floor_top + 0.5, back - 0.03], [cx + 0.22, floor_top + 0.95, back + 0.03]);
        }
    }
    b.span(p.metal, [-4.82, floor_top + 1.6, 139.08], [-4.78, g, 139.12]);
    b.span(p.glow, [-5.2, floor_top + 1.5, 138.95], [-4.4, floor_top + 1.6, 139.25]);
    b.lights.push(ArchiveLight {
        position_kind: [-4.8, floor_top + 1.45, 139.1, 6.],
        colour_intensity: [1., 0.66, 0.38, 10.],
    });
    // kitchen island and tall units along the back wall
    b.span(p.timber, [6.0, floor_top, 141.0], [9.6, floor_top + 0.88, 142.0]);
    b.span(p.ceramic, [5.95, floor_top + 0.88, 140.95], [9.65, floor_top + 0.92, 142.05]);
    b.span(p.wood, [5.0, floor_top, 146.95], [11.4, floor_top + 2.3, 147.7]);
    for n in 0..3 {
        let px = 6.8 + n as f32 * 1.2;
        b.put(p.bulb, Vec3::new(px, floor_top + 1.85, 141.5), Vec3::new(0.22, 0.28, 0.22), Quat::IDENTITY);
        b.span(p.metal, [px - 0.01, floor_top + 1.99, 141.49], [px + 0.01, g, 141.51]);
    }
    b.lights.push(ArchiveLight {
        position_kind: [8.0, floor_top + 1.8, 141.5, 6.],
        colour_intensity: [1., 0.7, 0.42, 10.],
    });
    // straight stair to the upper floor along the west wall
    for n in 0..17 {
        let rise = (u0 - floor_top) / 17.;
        let z = 145.6 - n as f32 * 0.27;
        b.span(p.timber, [-11.7, floor_top + n as f32 * rise, z - 0.27], [-10.6, floor_top + (n + 1) as f32 * rise, z]);
    }
    // upstairs: a bed and a reading light visible through the ribbon window
    b.span(fabric(5), [2.0, u0, 140.0], [3.8, u0 + 0.45, 142.2]);
    b.span(fabric(6), [2.0, u0 + 0.45, 140.05], [3.8, u0 + 0.6, 142.15]);
    b.span(p.timber, [1.9, u0, 142.2], [3.9, u0 + 1.1, 142.3]);
    b.span(p.glow, [4.3, u0 + 0.9, 142.0], [4.5, u0 + 1.05, 142.2]);

    // ---- garden trees ----
    for (tx, tz, h, r) in [(-11.5, 108.5, 6.8, 2.6), (11.0, 109.5, 6.2, 2.3), (-12.4, 126.0, 7.2, 2.6), (12.6, 124.5, 6.5, 2.4)] {
        tree(b, p, Vec3::new(tx, 0.12, tz), h, r, 48);
    }

    Villa {
        water: vec![WaterBody::pool(Vec3::new(-x, DEEP, z0), Vec3::new(x, WATER, z1))],
        view: (Vec3::new(2.9, 1.82, 107.4), Vec3::new(-0.22, -0.2, 1.).normalize()),
    }
}
