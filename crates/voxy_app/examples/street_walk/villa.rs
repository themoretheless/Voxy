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
    pub ceramic_cyl: usize,
    pub ceramic_sphere: usize,
    pub foliage_sphere: usize,
    pub metal_cyl: usize,
    pub metal_sphere: usize,
    pub glow_cyl: usize,
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
    let fork = root
        + Vec3::new(
            (b.random() - 0.5) * 0.3,
            height * 0.45,
            (b.random() - 0.5) * 0.3,
        );
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
                let offset =
                    Vec3::new(b.random() - 0.5, (b.random() - 0.5) * 0.7, b.random() - 0.5)
                        * radius
                        * 0.55;
                let size = 0.045 + b.random() * 0.03;
                let tilt = Quat::from_euler(
                    glam::EulerRot::YXZ,
                    b.random() * std::f32::consts::TAU,
                    b.random() * 2.4 - 1.2,
                    b.random() * 2.4 - 1.2,
                );
                b.put(
                    p.leaf,
                    start.lerp(end, along) + offset,
                    Vec3::new(size * 1.2, size, size * 2.),
                    tilt,
                );
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

/// Builds a 4-wall rectangular perimeter surround with wall thickness `t`.
fn rect_perimeter_walls(b: &mut Builder, id: usize, lo: [f32; 3], hi: [f32; 3], t: f32) {
    let [x0, y0, z0] = lo;
    let [x1, y1, z1] = hi;
    b.span(id, [x0, y0, z0], [x1, y1, z0 + t]);
    b.span(id, [x0, y0, z1 - t], [x1, y1, z1]);
    b.span(id, [x0, y0, z0 + t], [x0 + t, y1, z1 - t]);
    b.span(id, [x1 - t, y0, z0 + t], [x1, y1, z1 - t]);
}

/// Places a ceramic planter pot with a slightly wider clipped foliage crown on top.
#[expect(
    clippy::too_many_arguments,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
fn planter_box(
    b: &mut Builder,
    p: &Parts,
    x: f32,
    y0: f32,
    z: f32,
    pot_half: [f32; 2],
    pot_h: f32,
    foliage_h: f32,
) {
    let [hx, hz] = pot_half;
    b.span(
        p.ceramic_cyl,
        [x - hx, y0, z - hz],
        [x + hx, y0 + pot_h, z + hz],
    );
    b.span(
        p.foliage_sphere,
        [x - (hx + 0.05), y0 + pot_h, z - (hz + 0.05)],
        [x + (hx + 0.05), y0 + pot_h + foliage_h, z + (hz + 0.05)],
    );
}

fn bollard(b: &mut Builder, p: &Parts, x: f32, ground: f32, z: f32, light: bool) {
    b.span(
        p.metal_cyl,
        [x - 0.07, ground, z - 0.07],
        [x + 0.07, ground + 0.62, z + 0.07],
    );
    b.span(
        p.glow_cyl,
        [x - 0.06, ground + 0.5, z - 0.06],
        [x + 0.06, ground + 0.6, z + 0.06],
    );
    b.span(
        p.metal_sphere,
        [x - 0.09, ground + 0.62, z - 0.09],
        [x + 0.09, ground + 0.68, z + 0.09],
    );
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
        b.span(
            p.timber,
            [x0.min(flat_end), y - 0.05, z + dz - 0.025],
            [x0.max(flat_end), y, z + dz + 0.025],
        );
        for x in [x0 + s * 0.08, flat_end - s * 0.08] {
            b.span(
                p.timber,
                [x - 0.03, DECK, z + dz - 0.03],
                [x + 0.03, y - 0.05, z + dz + 0.03],
            );
        }
    }
    let slats = ((flat_end - x0).abs() / 0.09) as usize;
    for n in 0..slats {
        let x = x0 + s * (0.045 + n as f32 * 0.09);
        b.span(
            p.timber,
            [x - 0.035, y, z - 0.33],
            [x + 0.035, y + 0.025, z + 0.33],
        );
    }
    let length = (flat_end - x0).abs();
    b.block(
        cushion,
        [(x0 + flat_end) * 0.5, y + 0.06, z],
        [length - 0.04, 0.07, 0.6],
    );
    // Backrest raised 40 degrees, hinged at the end of the flat part.
    let back = (x1 - flat_end).abs() + 0.15;
    let tilt = Quat::from_rotation_z(s * 0.7);
    let dir = tilt * Vec3::new(s, 0., 0.);
    let hinge = Vec3::new(flat_end, y + 0.02, z);
    b.put(
        p.timber,
        hinge + dir * back * 0.5,
        Vec3::new(back, 0.03, 0.64),
        tilt,
    );
    b.put(
        cushion,
        hinge + dir * back * 0.5 + tilt * Vec3::Y * 0.05,
        Vec3::new(back - 0.05, 0.07, 0.6),
        tilt,
    );
}

#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
pub(super) fn build(b: &mut Builder, p: &Parts) -> Villa {
    let slab_m = b.material([0.47, 0.45, 0.41], 0.8, 0., 1., 0., 0.);
    let slab = b.bevel_cube_e(slab_m, 0.46);
    let travertine_m = b.material([0.74, 0.68, 0.58], 0.64, 0., 12., 0., 0.);
    let travertine = b.bevel_cube_e(travertine_m, 0.465);
    let travertine_cyl = b.round(travertine_m, false);
    let concrete_m = b.material([0.34, 0.33, 0.31], 0.9, 0., 1., 0., 0.);
    let concrete = b.cube(concrete_m);
    let lawn_m = b.material([0.085, 0.165, 0.042], 0.96, 0., 10., 0., 0.);
    let lawn = b.cube(lawn_m);
    let mosaic_m = b.material([0.52, 0.76, 0.80], 0.22, 0., 9., 0., 0.);
    let mosaic = b.cube(mosaic_m);
    let mosaic_dark_m = b.material([0.05, 0.16, 0.34], 0.22, 0., 9., 0., 0.);
    let mosaic_dark = b.cube(mosaic_dark_m);
    let chrome_m = b.material([0.93, 0.93, 0.95], 0.1, 1., 0., 0., 0.);
    let chrome = b.round(chrome_m, false);
    let chrome_sphere = b.round(chrome_m, true);
    let chrome_box = b.cube(chrome_m);
    let pool_light_m = b.material([0.75, 0.92, 1.], 0.3, 0., 0., 2600., 0.);
    let pool_light = b.round(pool_light_m, false);
    let render_m = b.material([0.86, 0.85, 0.82], 0.86, 0., 13., 0., 0.);
    let render = b.cube(render_m);
    let cladding_m = b.material([0.34, 0.2, 0.1], 0.58, 0., 4., 0., 0.);
    let cladding = b.cube(cladding_m);
    let cladding_cyl = b.round(cladding_m, false);
    let wall_m = b.material([0.74, 0.70, 0.62], 0.86, 0., 13., 0., 0.);
    let garden_wall = b.cube(wall_m);
    let canvas_m = b.material([0.88, 0.84, 0.74], 0.92, 0., 11., 0., 0.);
    let canvas = b.round(canvas_m, false);
    let cushion_m = b.material([0.88, 0.87, 0.82], 0.94, 0., 11., 0., 0.);
    let cushion = b.cube(cushion_m);
    let towel_m = b.material([0.08, 0.32, 0.5], 0.96, 0., 11., 0., 0.);
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
            b.put(
                slab,
                Vec3::new(x, 0.06 - h * 0.5, z),
                Vec3::new(0.585, h, 0.865),
                yaw,
            );
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
            b.put(
                slab,
                Vec3::new(x, 0.06 - h * 0.5, z),
                Vec3::new(0.585, h, 0.855),
                Quat::IDENTITY,
            );
        }
    }
    for side in [-1., 1.] {
        let s = |x: f32| side * x;
        // edging kerb, lawn strip, hedge and a rendered garden wall with coping
        b.block(p.curb, [s(2.55), 0.06, 96.2], [0.3, 0.14, 15.7]);
        b.span(
            lawn,
            [s(2.7).min(s(16.)), 0.04, 88.3],
            [s(2.7).max(s(16.)), 0.1, 104.],
        );
        hedge(
            b,
            p,
            Vec3::new(s(3.55), 0.1, 88.6),
            Vec3::new(s(3.55), 0.1, 103.6),
            1.15,
            0.7,
        );
        b.span(
            garden_wall,
            [s(4.05).min(s(4.35)), 0.04, 88.3],
            [s(4.05).max(s(4.35)), 2.0, 104.],
        );
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
        b.span(
            garden_wall,
            [s(1.9).min(s(16.)), 0.04, 103.85],
            [s(1.9).max(s(16.)), 2.2, 104.15],
        );
        b.span(
            garden_wall,
            [s(15.7).min(s(16.)), 0.04, 104.],
            [s(15.7).max(s(16.)), 2.2, 152.],
        );
        b.block(p.trim, [s(8.95), 2.24, 104.], [14.1, 0.08, 0.42]);
        b.block(p.trim, [s(15.85), 2.24, 128.], [0.42, 0.08, 48.]);
        // gate piers and an open steel gate leaf folded back against the wall
        b.span(
            render,
            [s(1.6).min(s(1.95)), 0.04, 103.75],
            [s(1.6).max(s(1.95)), 2.45, 104.25],
        );
        b.block(p.trim, [s(1.775), 2.5, 104.], [0.45, 0.1, 0.6]);
        let (x0, x1) = (s(2.0), s(3.45));
        for y in [0.15, 1.8] {
            b.span(
                p.metal,
                [x0.min(x1), y, 104.33],
                [x0.max(x1), y + 0.05, 104.38],
            );
        }
        for n in 0..12 {
            let x = x0 + (x1 - x0) * (n as f32 + 0.5) / 12.;
            b.span(
                p.metal,
                [x - 0.012, 0.15, 104.343],
                [x + 0.012, 1.85, 104.367],
            );
        }
        hedge(
            b,
            p,
            Vec3::new(s(15.05), 0.12, 104.8),
            Vec3::new(s(15.05), 0.12, 131.),
            1.5,
            0.9,
        );
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
        b.put(
            travertine,
            Vec3::new(x, 0.135, z),
            Vec3::new(0.9, 0.05, 0.5),
            yaw,
        );
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
    let deck = (
        [-cx - 6. * tile, cz[0] - 4. * tile],
        [cx + 6. * tile, cz[1] + 7. * tile],
    );
    let hole = ([-bx, bz[0]], [bx, bz[1]]);
    b.span(
        concrete,
        [deck.0[0], 0.08, deck.0[1]],
        [hole.0[0], 0.17, deck.1[1]],
    );
    b.span(
        concrete,
        [hole.1[0], 0.08, deck.0[1]],
        [deck.1[0], 0.17, deck.1[1]],
    );
    b.span(
        concrete,
        [hole.0[0], 0.08, deck.0[1]],
        [hole.1[0], 0.17, hole.0[1]],
    );
    b.span(
        concrete,
        [hole.0[0], 0.08, hole.1[1]],
        [hole.1[0], 0.17, deck.1[1]],
    );
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
            b.put(
                travertine,
                Vec3::new(x, DECK - h * 0.5, z),
                Vec3::new(width - 0.012, h, tile - 0.012),
                Quat::IDENTITY,
            );
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
    b.put(
        mosaic,
        ramp_centre - ramp_normal * 0.125,
        Vec3::new(2. * x, 0.25, ramp_length + 0.06),
        ramp_rotation,
    );
    // waterline band and a dark lane line along the floor
    for (lo, hi) in [
        ([-x, -0.12, z0], [-x + 0.004, 0.17, z1]),
        ([x - 0.004, -0.12, z0], [x, 0.17, z1]),
        ([-x, -0.12, z0], [x, 0.17, z0 + 0.004]),
        ([-x, -0.12, z1 - 0.004], [x, 0.17, z1]),
    ] {
        b.span(mosaic_dark, lo, hi);
    }
    b.span(
        mosaic_dark,
        [-0.13, SHALLOW, 114.9],
        [0.13, SHALLOW + 0.004, RAMP[0]],
    );
    b.span(
        mosaic_dark,
        [-0.13, DEEP, RAMP[1]],
        [0.13, DEEP + 0.004, z1 - 1.0],
    );
    b.put(
        mosaic_dark,
        ramp_centre + ramp_normal * 0.002,
        Vec3::new(0.26, 0.004, ramp_length),
        ramp_rotation,
    );
    b.span(
        mosaic_dark,
        [-0.5, DEEP, z1 - 1.0],
        [0.5, DEEP + 0.004, z1 - 0.74],
    );
    // full-width entry steps at the shallow end, nosings picked out in dark mosaic
    for (n, top) in [-0.16f32, -0.44, -0.72].into_iter().enumerate() {
        let za = z0 + n as f32 * 0.4;
        b.span(mosaic, [-x, SHALLOW, za], [x, top, za + 0.4]);
        b.span(
            mosaic_dark,
            [-x, top, za + 0.34],
            [x, top + 0.004, za + 0.4],
        );
    }
    // stainless/chrome shallow-end entry handrails flanking the steps
    for side in [-1.0f32, 1.0] {
        let hx = side * (x - 0.55);
        b.beam(
            chrome,
            Vec3::new(hx, DECK, z0 - 0.18),
            Vec3::new(hx, DECK + 0.78, z0 - 0.18),
            0.036,
        );
        b.beam(
            chrome,
            Vec3::new(hx, -0.72, z0 + 1.05),
            Vec3::new(hx, 0.16, z0 + 1.05),
            0.036,
        );
        b.beam(
            chrome,
            Vec3::new(hx, DECK + 0.78, z0 - 0.18),
            Vec3::new(hx, 0.16, z0 + 1.05),
            0.038,
        );
        b.beam(
            chrome,
            Vec3::new(hx, DECK + 0.42, z0 - 0.18),
            Vec3::new(hx, -0.20, z0 + 1.05),
            0.028,
        );
    }
    // side-wall underwater chrome/emissive spotlight bezels along the pool walls
    for side in [-1.0f32, 1.0] {
        let wx = side * (x - 0.012);
        for wz in [116.2f32, 121.0, 125.8] {
            b.put(
                chrome,
                Vec3::new(wx, -0.62, wz),
                Vec3::new(0.24, 0.022, 0.24),
                Quat::from_rotation_z(side * std::f32::consts::FRAC_PI_2),
            );
            b.put(
                pool_light,
                Vec3::new(wx - side * 0.006, -0.62, wz),
                Vec3::new(0.18, 0.018, 0.18),
                Quat::from_rotation_z(side * std::f32::consts::FRAC_PI_2),
            );
        }
    }
    // floating ceramic and warm-glow lotus lanterns on the water surface near the pool corners
    for (lx, lz) in [(2.85f32, 114.35f32), (-2.75, 127.85), (2.70, 128.15)] {
        b.put(
            p.ceramic,
            Vec3::new(lx, 0.005, lz),
            Vec3::new(0.28, 0.035, 0.28),
            Quat::IDENTITY,
        );
        b.put(
            p.ceramic,
            Vec3::new(lx, 0.025, lz),
            Vec3::new(0.22, 0.04, 0.22),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
        );
        b.put(
            p.glow,
            Vec3::new(lx, 0.055, lz),
            Vec3::new(0.11, 0.07, 0.11),
            Quat::IDENTITY,
        );
    }
    // coping: bevelled travertine overhanging the water by 4 cm
    let coping = |b: &mut Builder, lo: [f32; 2], hi: [f32; 2]| {
        let length = (hi[0] - lo[0]).max(hi[1] - lo[1]);
        let pieces = (length / 0.6).round().max(1.) as usize;
        for i in 0..pieces {
            let t0 = i as f32 / pieces as f32;
            let t1 = (i + 1) as f32 / pieces as f32;
            let (a, c) = if hi[0] - lo[0] > hi[1] - lo[1] {
                (
                    [lo[0] + (hi[0] - lo[0]) * t0, lo[1]],
                    [lo[0] + (hi[0] - lo[0]) * t1, hi[1]],
                )
            } else {
                (
                    [lo[0], lo[1] + (hi[1] - lo[1]) * t0],
                    [hi[0], lo[1] + (hi[1] - lo[1]) * t1],
                )
            };
            let centre = Vec3::new((a[0] + c[0]) * 0.5, DECK, (a[1] + c[1]) * 0.5);
            b.put(
                travertine,
                centre,
                Vec3::new(c[0] - a[0] - 0.008, 0.07, c[1] - a[1] - 0.008),
                Quat::IDENTITY,
            );
        }
    };
    coping(b, [-x - 0.34, z0 - 0.34], [-x + 0.04, z1 + 0.34]);
    coping(b, [x - 0.04, z0 - 0.34], [x + 0.34, z1 + 0.34]);
    coping(b, [-x + 0.04, z0 - 0.34], [x - 0.04, z0 + 0.04]);
    coping(b, [-x + 0.04, z1 - 0.04], [x - 0.04, z1 + 0.34]);
    // underwater lights in the deep-end wall
    for lx in [-1.2, 1.2] {
        b.put(
            pool_light,
            Vec3::new(lx, -0.75, z1 - 0.012),
            Vec3::new(0.26, 0.024, 0.26),
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        );
        b.lights.push(ArchiveLight {
            position_kind: [lx, -0.8, z1 - 0.35, 6.],
            colour_intensity: [0.72, 0.9, 1., 22.],
        });
    }
    // ladder over the coping at the deep end
    for dz in [-0.24, 0.24] {
        let z = 124. + dz;
        b.beam(
            chrome,
            Vec3::new(x - 0.17, -1.3, z),
            Vec3::new(x - 0.17, 0.52, z),
            0.04,
        );
        b.beam(
            chrome,
            Vec3::new(x - 0.17, 0.52, z),
            Vec3::new(x + 0.12, 0.64, z),
            0.04,
        );
        b.beam(
            chrome,
            Vec3::new(x + 0.12, 0.64, z),
            Vec3::new(x + 0.42, 0.52, z),
            0.04,
        );
        b.beam(
            chrome,
            Vec3::new(x + 0.42, 0.52, z),
            Vec3::new(x + 0.42, DECK, z),
            0.04,
        );
        b.span(chrome_box, [x - 0.2, -1.3, z - 0.02], [x, -1.26, z + 0.02]);
    }
    for y in [-0.25f32, -0.55, -0.85] {
        b.span(
            chrome_box,
            [x - 0.24, y, 123.73],
            [x - 0.08, y + 0.03, 124.27],
        );
    }
    // outdoor shower beside the deep end
    b.beam(
        chrome,
        Vec3::new(-3.4, DECK, 128.3),
        Vec3::new(-3.4, 2.3, 128.3),
        0.05,
    );
    b.beam(
        chrome,
        Vec3::new(-3.4, 2.3, 128.3),
        Vec3::new(-3.05, 2.32, 128.3),
        0.03,
    );
    b.put(
        chrome,
        Vec3::new(-3.0, 2.27, 128.3),
        Vec3::new(0.2, 0.03, 0.2),
        Quat::IDENTITY,
    );
    b.span(slab, [-3.75, DECK, 127.95], [-3.05, DECK + 0.02, 128.65]);

    // ---- deck furniture ----
    for (n, z) in [115.2, 117.8, 120.4, 123.0].into_iter().enumerate() {
        lounger(b, p, 3.25, 5.3, z, cushion);
        if n % 2 == 0 {
            b.span(
                towel,
                [3.4, DECK + 0.47, z - 0.27],
                [4.0, DECK + 0.49, z + 0.27],
            );
        }
    }
    for z in [116.5, 121.7] {
        b.span(
            p.timber,
            [5.45, DECK, z - 0.22],
            [5.85, DECK + 0.42, z + 0.22],
        );
        b.span(
            p.ceramic,
            [5.47, DECK + 0.42, z - 0.2],
            [5.83, DECK + 0.44, z + 0.2],
        );
    }
    let (ux, uz) = (-4.9, 120.);
    b.span(
        concrete,
        [ux - 0.25, DECK, uz - 0.25],
        [ux + 0.25, DECK + 0.12, uz + 0.25],
    );
    b.beam(
        p.trunk,
        Vec3::new(ux, DECK, uz),
        Vec3::new(ux, DECK + 2.55, uz),
        0.05,
    );
    b.put(
        canvas,
        Vec3::new(ux, DECK + 2.42, uz),
        Vec3::new(3.0, 0.05, 3.0),
        Quat::IDENTITY,
    );
    b.put(
        canvas,
        Vec3::new(ux, DECK + 2.5, uz),
        Vec3::new(2.0, 0.08, 2.0),
        Quat::IDENTITY,
    );
    b.put(
        canvas,
        Vec3::new(ux, DECK + 2.58, uz),
        Vec3::new(0.9, 0.08, 0.9),
        Quat::IDENTITY,
    );
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::FRAC_PI_4;
        let tip = Vec3::new(ux + a.cos() * 1.45, DECK + 2.37, uz + a.sin() * 1.45);
        b.beam(p.metal, Vec3::new(ux, DECK + 2.6, uz), tip, 0.012);
    }
    // dining set beside the parasol
    let (tx, tz) = (-4.9, 124.6);
    b.span(
        p.timber,
        [tx - 0.5, DECK + 0.72, tz - 0.8],
        [tx + 0.5, DECK + 0.76, tz + 0.8],
    );
    for (dx, dz) in [(-0.42, -0.72), (0.42, -0.72), (-0.42, 0.72), (0.42, 0.72)] {
        b.span(
            p.metal,
            [tx + dx - 0.02, DECK, tz + dz - 0.02],
            [tx + dx + 0.02, DECK + 0.72, tz + dz + 0.02],
        );
    }
    for (cx, cz) in [(-0.85, -0.4), (-0.85, 0.4), (0.85, -0.4), (0.85, 0.4)] {
        let (x0, z0c) = (tx + cx, tz + cz);
        b.span(
            p.timber,
            [x0 - 0.22, DECK + 0.44, z0c - 0.22],
            [x0 + 0.22, DECK + 0.47, z0c + 0.22],
        );
        let back = if cx < 0. { x0 - 0.22 } else { x0 + 0.19 };
        b.span(
            p.timber,
            [back, DECK + 0.47, z0c - 0.22],
            [back + 0.03, DECK + 0.9, z0c + 0.22],
        );
        for (lx, lz) in [(-0.19, -0.19), (0.19, -0.19), (-0.19, 0.19), (0.19, 0.19)] {
            b.span(
                p.metal,
                [x0 + lx - 0.015, DECK, z0c + lz - 0.015],
                [x0 + lx + 0.015, DECK + 0.44, z0c + lz + 0.015],
            );
        }
    }
    // planted olive trees in concrete planters at the deck corners
    for (px, pz) in [(-6.3, 110.7), (6.3, 110.7), (-6.3, 131.4), (6.3, 131.4)] {
        b.span(
            concrete,
            [px - 0.45, DECK, pz - 0.45],
            [px + 0.45, DECK + 0.62, pz + 0.45],
        );
        b.span(
            lawn,
            [px - 0.4, DECK + 0.62, pz - 0.4],
            [px + 0.4, DECK + 0.64, pz + 0.4],
        );
        tree(b, p, Vec3::new(px, DECK + 0.62, pz), 2.6, 0.95, 30);
    }

    // ---- east poolside pergola & summer bar pavilion ----
    let (px0, px1, pz0, pz1) = (6.8f32, 10.4f32, 114.6f32, 123.8f32);
    let py0 = 0.12f32;
    let py_deck = DECK;
    b.span(
        travertine,
        [px0 - 0.2, py0, pz0 - 0.2],
        [px1 + 0.2, py_deck, pz1 + 0.2],
    );
    // Step stones linking the main pool deck to the pergola terrace
    for z_step in [116.8f32, 119.2, 121.6] {
        b.span(
            travertine,
            [5.85, py0, z_step - 0.35],
            [6.55, py_deck - 0.01, z_step + 0.35],
        );
    }
    let py_top = py_deck + 2.55;
    for &cx in &[px0, px1] {
        for &cz in &[pz0, (pz0 + pz1) * 0.5, pz1] {
            b.span(
                p.metal,
                [cx - 0.06, py_deck, cz - 0.06],
                [cx + 0.06, py_top, cz + 0.06],
            );
        }
        b.span(
            p.metal,
            [cx - 0.06, py_top - 0.12, pz0 - 0.1],
            [cx + 0.06, py_top, pz1 + 0.1],
        );
    }
    for &cz in &[pz0, (pz0 + pz1) * 0.5, pz1] {
        b.span(
            p.metal,
            [px0 - 0.1, py_top - 0.12, cz - 0.06],
            [px1 + 0.1, py_top, cz + 0.06],
        );
    }
    // Timber brise-soleil slats casting sun-dappled shadows across the summer lounge
    let slat_count = 24usize;
    for i in 0..slat_count {
        let sz = pz0 + 0.15 + i as f32 * (pz1 - pz0 - 0.3) / (slat_count - 1) as f32;
        b.span(
            p.timber,
            [px0 - 0.15, py_top, sz - 0.03],
            [px1 + 0.15, py_top + 0.09, sz + 0.03],
        );
    }
    // Warm architectural LED strip along the central pergola beam
    b.span(
        p.glow,
        [px0 + 0.2, py_top - 0.14, (pz0 + pz1) * 0.5 - 0.04],
        [px1 - 0.2, py_top - 0.12, (pz0 + pz1) * 0.5 + 0.04],
    );
    // Summer kitchen bar counter with glazed ceramic countertop and bar stools
    b.span(
        cladding,
        [px1 - 0.85, py_deck, pz0 + 0.8],
        [px1 - 0.15, py_deck + 0.92, pz0 + 3.8],
    );
    b.span(
        p.ceramic,
        [px1 - 0.90, py_deck + 0.92, pz0 + 0.75],
        [px1 - 0.10, py_deck + 0.97, pz0 + 3.85],
    );
    for stool_z in [pz0 + 1.35, pz0 + 2.3, pz0 + 3.25] {
        let sx = px1 - 1.25;
        b.span(
            p.metal,
            [sx - 0.025, py_deck, stool_z - 0.025],
            [sx + 0.025, py_deck + 0.66, stool_z + 0.025],
        );
        b.span(
            p.metal,
            [sx - 0.18, py_deck + 0.01, stool_z - 0.18],
            [sx + 0.18, py_deck + 0.03, stool_z + 0.18],
        );
        b.span(
            cushion,
            [sx - 0.19, py_deck + 0.66, stool_z - 0.19],
            [sx + 0.19, py_deck + 0.72, stool_z + 0.19],
        );
    }
    // Outdoor lounge daybed & low travertine table in the southern half of the pergola
    b.span(
        p.timber,
        [px0 + 0.35, py_deck, pz1 - 3.6],
        [px1 - 0.35, py_deck + 0.28, pz1 - 0.7],
    );
    b.span(
        cushion,
        [px0 + 0.42, py_deck + 0.28, pz1 - 3.52],
        [px1 - 0.42, py_deck + 0.42, pz1 - 0.78],
    );
    b.span(
        cushion,
        [px1 - 0.68, py_deck + 0.42, pz1 - 3.52],
        [px1 - 0.42, py_deck + 0.76, pz1 - 0.78],
    );

    // ---- west garden sunken fire-pit lounge & courtyard benches ----
    let (fx, fz) = (-9.2f32, 118.5f32);
    b.span(
        travertine,
        [fx - 2.3, 0.12, fz - 2.3],
        [fx + 2.3, 0.18, fz + 2.3],
    );
    b.span(
        concrete,
        [fx - 0.65, 0.18, fz - 0.65],
        [fx + 0.65, 0.52, fz + 0.65],
    );
    b.span(
        p.metal,
        [fx - 0.48, 0.52, fz - 0.48],
        [fx + 0.48, 0.56, fz + 0.48],
    );
    b.span(
        p.glow,
        [fx - 0.28, 0.56, fz - 0.28],
        [fx + 0.28, 0.62, fz + 0.28],
    );
    for (bx0, bx1, bz0, bz1) in [
        (fx - 2.0, fx - 1.35, fz - 1.5, fz + 1.5),
        (fx + 1.35, fx + 2.0, fz - 1.5, fz + 1.5),
        (fx - 1.2, fx + 1.2, fz - 2.0, fz - 1.35),
    ] {
        b.span(p.timber, [bx0, 0.18, bz0], [bx1, 0.46, bz1]);
        b.span(
            cushion,
            [bx0 + 0.04, 0.46, bz0 + 0.04],
            [bx1 - 0.04, 0.54, bz1 - 0.04],
        );
    }
    // Forecourt garden benches and glazed ceramic amphora planters at z = 85.8
    for side in [-1.0f32, 1.0] {
        let gx = side * 6.4;
        b.span(p.timber, [gx - 0.9, 0.38, 85.5], [gx + 0.9, 0.44, 86.0]);
        for lx in [gx - 0.75, gx + 0.75] {
            b.span(p.metal, [lx - 0.04, 0.06, 85.54], [lx + 0.04, 0.38, 85.96]);
        }
        let vx = side * 4.5;
        b.span(
            p.ceramic,
            [vx - 0.28, 0.06, 85.45],
            [vx + 0.28, 0.58, 86.05],
        );
        b.span(
            p.foliage,
            [vx - 0.34, 0.58, 85.40],
            [vx + 0.34, 1.05, 86.10],
        );
    }

    // ---- villa: glazed ground floor, cantilevered upper storey, flat roof ----
    let (vx, vz) = (12.0f32, [deck.1[1], 148.0f32]);
    let floor_top = DECK + 0.02;
    b.span(concrete, [-vx, -0.08, vz[0]], [vx, 0.17, vz[1]]);
    b.span(travertine, [-vx, 0.13, vz[0]], [vx, DECK, 134.]);
    b.span(
        p.floor,
        [-vx + 0.3, 0.13, 134.3],
        [vx - 0.3, floor_top, vz[1] - 0.3],
    );
    let g = 3.3;
    b.span(render, [-vx, DECK, vz[1] - 0.3], [vx, g, vz[1]]);
    b.span(render, [-vx, DECK, 134.], [-vx + 0.3, g, vz[1]]);
    b.span(render, [vx - 0.3, DECK, 134.], [vx, g, vz[1]]);
    b.span(render, [-vx, DECK, 134.], [-7.2, g, 134.3]);
    b.span(
        cladding,
        [-vx, DECK + 0.02, 133.97],
        [-7.25, g - 0.05, 134.],
    );
    // sliding glass doors between slim aluminium frames
    let mullions: Vec<f32> = (0..=8).map(|n| -7.2 + n as f32 * 2.4).collect();
    for &mx in &mullions {
        b.span(p.metal, [mx - 0.04, DECK, 134.05], [mx + 0.04, g, 134.25]);
    }
    b.span(p.metal, [-7.2, DECK, 134.05], [vx, DECK + 0.06, 134.25]);
    b.span(p.metal, [-7.2, g - 0.1, 134.05], [vx, g, 134.25]);
    for pair in mullions.windows(2) {
        b.span(
            p.glass,
            [pair[0] + 0.04, DECK + 0.06, 134.135],
            [pair[1] - 0.04, g - 0.1, 134.165],
        );
    }
    // floor slab and fascia; the upper box overhangs the terrace
    b.span(render, [-vx, g, vz[0]], [vx, g + 0.32, vz[1]]);
    b.span(
        p.ceiling,
        [-vx + 0.3, g - 0.02, 134.3],
        [vx - 0.3, g, vz[1] - 0.3],
    );
    let (ux0, uz1) = (-9.0f32, 146.5f32);
    let u0 = g + 0.32;
    let u1 = u0 + 3.0;
    b.span(render, [ux0, u0, uz1 - 0.3], [vx, u1, uz1]);
    b.span(render, [ux0, u0, vz[0]], [ux0 + 0.3, u1, uz1]);
    b.span(render, [vx - 0.3, u0, vz[0]], [vx, u1, uz1]);
    b.span(render, [ux0, u0, vz[0]], [vx, u0 + 0.8, vz[0] + 0.3]);
    b.span(render, [ux0, u1 - 0.4, vz[0]], [vx, u1, vz[0] + 0.3]);
    let ribbon: Vec<f32> = (0..=7)
        .map(|n| ux0 + 0.3 + n as f32 * (vx - 0.3 - ux0 - 0.3) / 7.)
        .collect();
    for &mx in &ribbon {
        b.span(
            p.metal,
            [mx - 0.035, u0 + 0.8, vz[0] + 0.05],
            [mx + 0.035, u1 - 0.4, vz[0] + 0.22],
        );
    }
    for pair in ribbon.windows(2) {
        b.span(
            p.glass,
            [pair[0] + 0.035, u0 + 0.8, vz[0] + 0.12],
            [pair[1] - 0.035, u1 - 0.4, vz[0] + 0.15],
        );
    }
    b.span(
        p.floor,
        [ux0 + 0.3, u0, vz[0] + 0.3],
        [vx - 0.3, u0 + 0.02, uz1 - 0.3],
    );
    b.span(
        p.ceiling,
        [ux0 + 0.3, u1 - 0.02, vz[0] + 0.3],
        [vx - 0.3, u1, uz1 - 0.3],
    );
    b.span(
        render,
        [ux0 - 0.25, u1, vz[0] - 0.25],
        [vx + 0.25, u1 + 0.3, uz1 + 0.25],
    );
    b.span(
        p.metal,
        [ux0 - 0.27, u1 + 0.3, vz[0] - 0.27],
        [vx + 0.27, u1 + 0.36, uz1 + 0.27],
    );
    // roof terrace edge over the single-storey west wing
    b.span(
        p.glass,
        [-vx, u0, vz[0] + 0.05],
        [ux0, u0 + 1.0, vz[0] + 0.08],
    );
    b.span(
        p.glass,
        [-vx + 0.05, u0, vz[0]],
        [-vx + 0.08, u0 + 1.0, vz[1]],
    );
    b.span(
        p.metal,
        [-vx, u0 + 1.0, vz[0]],
        [ux0, u0 + 1.05, vz[0] + 0.12],
    );
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
    b.span(
        fabric(1),
        [-1.2, floor_top, 139.6],
        [2.6, floor_top + 0.42, 140.5],
    );
    b.span(
        fabric(1),
        [-1.2, floor_top + 0.42, 140.3],
        [2.6, floor_top + 0.85, 140.55],
    );
    b.span(
        fabric(1),
        [2.6, floor_top, 137.2],
        [3.5, floor_top + 0.42, 140.55],
    );
    b.span(
        fabric(1),
        [3.3, floor_top + 0.42, 137.2],
        [3.55, floor_top + 0.85, 140.55],
    );
    b.span(
        fabric(2),
        [-1.4, floor_top, 136.6],
        [2.4, floor_top + 0.012, 139.3],
    );
    b.span(
        p.timber,
        [-0.2, floor_top, 137.4],
        [1.4, floor_top + 0.38, 138.4],
    );
    // dining table with chairs and a pendant
    b.span(
        p.timber,
        [-6.0, floor_top + 0.72, 138.6],
        [-3.6, floor_top + 0.76, 139.6],
    );
    for (lx, lz) in [
        (-5.85, 138.7),
        (-3.75, 138.7),
        (-5.85, 139.5),
        (-3.75, 139.5),
    ] {
        b.span(
            p.metal,
            [lx - 0.03, floor_top, lz - 0.03],
            [lx + 0.03, floor_top + 0.72, lz + 0.03],
        );
    }
    for (n, cx) in [-5.5f32, -4.8, -4.1].into_iter().enumerate() {
        for (cz, back) in [(138.15, 137.95), (140.05, 140.25)] {
            b.span(
                fabric(3 + n),
                [cx - 0.22, floor_top + 0.45, cz - 0.2],
                [cx + 0.22, floor_top + 0.5, cz + 0.2],
            );
            b.span(
                fabric(3 + n),
                [cx - 0.22, floor_top + 0.5, back - 0.03],
                [cx + 0.22, floor_top + 0.95, back + 0.03],
            );
        }
    }
    b.span(
        p.metal,
        [-4.82, floor_top + 1.6, 139.08],
        [-4.78, g, 139.12],
    );
    b.span(
        p.glow,
        [-5.2, floor_top + 1.5, 138.95],
        [-4.4, floor_top + 1.6, 139.25],
    );
    b.lights.push(ArchiveLight {
        position_kind: [-4.8, floor_top + 1.45, 139.1, 6.],
        colour_intensity: [1., 0.66, 0.38, 10.],
    });
    // kitchen island and tall units along the back wall
    b.span(
        p.timber,
        [6.0, floor_top, 141.0],
        [9.6, floor_top + 0.88, 142.0],
    );
    b.span(
        p.ceramic,
        [5.95, floor_top + 0.88, 140.95],
        [9.65, floor_top + 0.92, 142.05],
    );
    b.span(
        p.wood,
        [5.0, floor_top, 146.95],
        [11.4, floor_top + 2.3, 147.7],
    );
    for n in 0..3 {
        let px = 6.8 + n as f32 * 1.2;
        b.put(
            p.bulb,
            Vec3::new(px, floor_top + 1.85, 141.5),
            Vec3::new(0.22, 0.28, 0.22),
            Quat::IDENTITY,
        );
        b.span(
            p.metal,
            [px - 0.01, floor_top + 1.99, 141.49],
            [px + 0.01, g, 141.51],
        );
    }
    b.lights.push(ArchiveLight {
        position_kind: [8.0, floor_top + 1.8, 141.5, 6.],
        colour_intensity: [1., 0.7, 0.42, 10.],
    });
    // straight stair to the upper floor along the west wall
    for n in 0..17 {
        let rise = (u0 - floor_top) / 17.;
        let z = 145.6 - n as f32 * 0.27;
        b.span(
            p.timber,
            [-11.7, floor_top + n as f32 * rise, z - 0.27],
            [-10.6, floor_top + (n + 1) as f32 * rise, z],
        );
    }
    // upstairs: a bed and a reading light visible through the ribbon window
    b.span(fabric(5), [2.0, u0, 140.0], [3.8, u0 + 0.45, 142.2]);
    b.span(fabric(6), [2.0, u0 + 0.45, 140.05], [3.8, u0 + 0.6, 142.15]);
    b.span(p.timber, [1.9, u0, 142.2], [3.9, u0 + 1.1, 142.3]);
    b.span(p.glow, [4.3, u0 + 0.9, 142.0], [4.5, u0 + 1.05, 142.2]);
    // upstairs gallery bookshelves, study desk, and west-wing roof terrace lounge
    b.span(p.wood, [-6.5, u0 + 0.02, 145.6], [-2.2, u0 + 2.25, 146.15]);
    b.span(p.timber, [-5.4, u0 + 0.72, 138.8], [-3.4, u0 + 0.76, 139.7]);
    for lx in [-5.3f32, -3.5] {
        b.span(
            p.metal,
            [lx - 0.03, u0 + 0.02, 138.85],
            [lx + 0.03, u0 + 0.72, 139.65],
        );
    }
    b.span(
        p.ceramic,
        [-4.6, u0 + 0.76, 139.1],
        [-4.2, u0 + 0.98, 139.4],
    );
    // Rooftop terrace loungers and glass table over the west wing (x = -11.5 .. -9.3)
    for rz in [136.8f32, 139.6] {
        b.span(
            p.timber,
            [-11.3, u0, rz - 0.32],
            [-9.6, u0 + 0.28, rz + 0.32],
        );
        b.span(
            cushion,
            [-11.25, u0 + 0.28, rz - 0.28],
            [-9.65, u0 + 0.36, rz + 0.28],
        );
    }
    b.span(p.metal, [-10.7, u0, 138.0], [-10.1, u0 + 0.42, 138.4]);
    b.span(
        p.glass,
        [-10.8, u0 + 0.42, 137.9],
        [-10.0, u0 + 0.45, 138.5],
    );
    // Ground-floor living room library console, ceramic sculpture & indoor planter
    b.span(
        p.wood,
        [-11.65, floor_top, 136.5],
        [-11.15, floor_top + 1.85, 140.2],
    );
    b.span(
        p.ceramic,
        [0.4, floor_top + 0.38, 137.75],
        [0.72, floor_top + 0.56, 138.05],
    );
    b.span(
        p.ceramic,
        [3.9, floor_top, 135.2],
        [4.4, floor_top + 0.48, 135.7],
    );
    b.span(
        p.foliage,
        [3.85, floor_top + 0.48, 135.15],
        [4.45, floor_top + 1.25, 135.75],
    );
    // Parlor grand piano, open acoustic lid, brass soundboard, keyboard & bench in the east living room (x = 7.4..9.1, z = 135.2..136.9)
    let (px0, px1, pz0, pz1) = (7.45f32, 8.95f32, 135.35f32, 136.85f32);
    // Soundbox rim & curved tail section
    b.span(
        p.metal,
        [px0, floor_top + 0.68, pz0],
        [px1, floor_top + 0.96, pz0 + 0.85],
    );
    b.span(
        p.metal,
        [px0 + 0.22, floor_top + 0.68, pz0 + 0.85],
        [px1, floor_top + 0.96, pz1],
    );
    // Warm cedar/brass interior soundboard & cast-iron harp frame
    b.span(
        p.timber,
        [px0 + 0.08, floor_top + 0.92, pz0 + 0.08],
        [px1 - 0.08, floor_top + 0.95, pz0 + 0.80],
    );
    b.span(
        p.timber,
        [px0 + 0.28, floor_top + 0.92, pz0 + 0.80],
        [px1 - 0.08, floor_top + 0.95, pz1 - 0.08],
    );
    // Three tapered piano legs with brass casters and lyre pedal assembly
    for (lx, lz) in [
        (px0 + 0.14, pz0 + 0.14),
        (px1 - 0.14, pz0 + 0.14),
        (px0 + 0.62, pz1 - 0.16),
    ] {
        b.span(
            p.metal,
            [lx - 0.045, floor_top, lz - 0.045],
            [lx + 0.045, floor_top + 0.68, lz + 0.045],
        );
    }
    b.span(
        p.metal,
        [8.14, floor_top + 0.05, pz0 + 0.16],
        [8.26, floor_top + 0.68, pz0 + 0.22],
    );
    b.span(
        chrome,
        [8.08, floor_top + 0.03, pz0 + 0.08],
        [8.32, floor_top + 0.06, pz0 + 0.16],
    );
    // Keybed, ivory/porcelain white keys, ebony sharps, and music desk
    b.span(
        p.metal,
        [px0 + 0.04, floor_top + 0.64, pz0 - 0.24],
        [px1 - 0.04, floor_top + 0.74, pz0],
    );
    b.span(
        p.ceramic,
        [px0 + 0.10, floor_top + 0.74, pz0 - 0.21],
        [px1 - 0.10, floor_top + 0.77, pz0 - 0.03],
    );
    b.span(
        p.metal,
        [px0 + 0.12, floor_top + 0.77, pz0 - 0.12],
        [px1 - 0.12, floor_top + 0.79, pz0 - 0.03],
    );
    b.put(
        p.wood,
        Vec3::new(8.20, floor_top + 1.08, pz0 + 0.08),
        Vec3::new(0.72, 0.26, 0.025),
        Quat::from_rotation_x(-0.24),
    );
    // Open angled grand piano lid & brass prop stick
    b.put(
        p.metal,
        Vec3::new(8.22, floor_top + 1.24, 136.05),
        Vec3::new(1.42, 0.03, 1.38),
        Quat::from_rotation_z(0.38),
    );
    b.put(
        chrome,
        Vec3::new(7.62, floor_top + 1.12, 135.75),
        Vec3::new(0.025, 0.42, 0.025),
        Quat::from_rotation_z(-0.18),
    );
    // Tufted upholstered duet piano bench facing the keyboard
    b.span(
        p.metal,
        [7.82, floor_top, pz0 - 0.72],
        [8.58, floor_top + 0.44, pz0 - 0.38],
    );
    b.span(
        fabric(0),
        [7.80, floor_top + 0.44, pz0 - 0.74],
        [8.60, floor_top + 0.50, pz0 - 0.36],
    );

    // ---- garden trees ----
    for (tx, tz, h, r) in [
        (-11.5, 108.5, 6.8, 2.6),
        (11.0, 109.5, 6.2, 2.3),
        (-12.4, 126.0, 7.2, 2.6),
        (12.6, 124.5, 6.5, 2.4),
    ] {
        tree(b, p, Vec3::new(tx, 0.12, tz), h, r, 48);
    }

    // ---- walled lane parterre gardens & stone gazebo alcoves (z = 89..103) ----
    for side in [-1.0f32, 1.0] {
        let s = |x: f32| side * x;
        // Stepped travertine garden path behind the garden wall (x = ±5.0..14.0)
        for step in 0..8 {
            let sz = 90.0 + step as f32 * 1.6;
            b.span(
                travertine,
                [s(5.4).min(s(6.2)), 0.10, sz - 0.28],
                [s(5.4).max(s(6.2)), 0.15, sz + 0.28],
            );
        }
        // Ornamental stone fountain basin & clipped topiary parterre at x = ±7.2, z = 95.8
        let (bx, bz) = (s(7.2), 95.8f32);
        b.span(
            travertine,
            [bx - 1.35, 0.10, bz - 1.85],
            [bx + 1.35, 0.18, bz + 1.85],
        );
        b.span(
            travertine_cyl,
            [bx - 0.92, 0.18, bz - 1.25],
            [bx + 0.92, 0.54, bz + 1.25],
        );
        b.span(
            mosaic,
            [bx - 0.72, 0.48, bz - 1.12],
            [bx + 0.72, 0.52, bz + 1.12],
        );
        b.span(
            p.ceramic_cyl,
            [bx - 0.20, 0.52, bz - 0.20],
            [bx + 0.20, 1.08, bz + 0.20],
        );
        b.span(
            p.ceramic_sphere,
            [bx - 0.42, 0.96, bz - 0.42],
            [bx + 0.42, 1.12, bz + 0.42],
        );
        b.span(
            p.metal_cyl,
            [bx - 0.04, 1.08, bz - 0.04],
            [bx + 0.04, 1.24, bz + 0.04],
        );
        // Four corner topiary shrubs around the parterre court
        for (dx, dz) in [(-1.1, -1.55), (1.1, -1.55), (-1.1, 1.55), (1.1, 1.55)] {
            b.span(
                p.ceramic_cyl,
                [bx + dx - 0.24, 0.18, bz + dz - 0.24],
                [bx + dx + 0.24, 0.52, bz + dz + 0.24],
            );
            b.span(
                p.foliage_sphere,
                [bx + dx - 0.34, 0.52, bz + dz - 0.34],
                [bx + dx + 0.34, 1.18, bz + dz + 0.34],
            );
        }
    }

    // ---- rear sculpture courtyard & zen garden behind the villa (z = 148.2..151.3) ----
    b.span(travertine, [-6.5, 0.12, 148.3], [6.5, 0.18, 151.2]);
    // Modernist bronze/chrome sculpture plinth and interlocking rings
    b.span(concrete, [-0.55, 0.18, 149.35], [0.55, 0.88, 150.45]);
    b.span(travertine, [-0.62, 0.88, 149.28], [0.62, 0.96, 150.52]);
    b.put(
        chrome,
        Vec3::new(0.0, 1.52, 149.9),
        Vec3::new(0.75, 1.10, 0.22),
        Quat::from_rotation_y(0.42),
    );
    b.put(
        p.ceramic,
        Vec3::new(0.0, 1.38, 149.9),
        Vec3::new(0.36, 0.68, 0.36),
        Quat::from_rotation_y(-0.35),
    );
    for side in [-1.0f32, 1.0] {
        let bx = side * 3.8;
        b.span(
            p.timber,
            [bx - 0.85, 0.18, 150.35],
            [bx + 0.85, 0.46, 150.85],
        );
        b.span(
            cushion,
            [bx - 0.80, 0.46, 150.38],
            [bx + 0.80, 0.52, 150.82],
        );
        let px = side * 5.6;
        b.span(
            p.ceramic_cyl,
            [px - 0.34, 0.18, 149.6],
            [px + 0.34, 0.66, 150.2],
        );
        b.span(
            p.foliage_sphere,
            [px - 0.40, 0.66, 149.55],
            [px + 0.40, 1.35, 150.25],
        );
    }

    // ---- additional villa interior & terrace details ----
    // Poolside dining table centerpiece (ceramic fruit bowl & glass carafe)
    b.span(
        p.ceramic_sphere,
        [-5.05, DECK + 0.76, 124.45],
        [-4.75, DECK + 0.84, 124.75],
    );
    b.span(
        p.glass,
        [-4.85, DECK + 0.76, 124.92],
        [-4.73, DECK + 0.98, 125.04],
    );
    // Living room coffee table art books & ceramic tea set, plus bar stools at kitchen island
    b.span(
        p.ceramic_cyl,
        [0.85, floor_top + 0.38, 137.85],
        [1.12, floor_top + 0.44, 138.12],
    );
    for i in 0..3 {
        let sx = 6.5 + i as f32 * 1.1;
        let sz = 140.55;
        b.span(
            p.metal_cyl,
            [sx - 0.025, floor_top, sz - 0.025],
            [sx + 0.025, floor_top + 0.62, sz + 0.025],
        );
        b.span(
            p.metal_cyl,
            [sx - 0.17, floor_top + 0.01, sz - 0.17],
            [sx + 0.17, floor_top + 0.03, sz + 0.17],
        );
        b.span(
            fabric(2 + i),
            [sx - 0.18, floor_top + 0.62, sz - 0.18],
            [sx + 0.18, floor_top + 0.68, sz + 0.18],
        );
    }
    // Upstairs bedroom nightstands, reading lamps & wardrobe along the east wall
    for nz in [139.55f32, 142.65] {
        b.span(
            p.timber,
            [3.25, u0, nz - 0.24],
            [3.75, u0 + 0.48, nz + 0.24],
        );
        b.span(
            p.ceramic_cyl,
            [3.42, u0 + 0.48, nz - 0.08],
            [3.58, u0 + 0.68, nz + 0.08],
        );
    }
    b.span(p.wood, [10.85, u0 + 0.02, 138.5], [11.55, u0 + 2.35, 143.5]);

    // ---- east citrus grove & glass-and-steel greenhouse conservatory (x = 11.2..14.4, z = 113.0..121.0) ----
    let (gx0, gx1, gz0, gz1) = (11.4f32, 14.2f32, 113.8f32, 120.2f32);
    let gy0 = 0.12f32;
    let gy_wall = gy0 + 1.95;
    let gy_ridge = gy0 + 2.55;
    b.span(
        travertine,
        [gx0 - 0.15, gy0, gz0 - 0.15],
        [gx1 + 0.15, gy0 + 0.08, gz1 + 0.15],
    );
    // Low brick/travertine knee wall around the conservatory perimeter
    for (lo, hi) in [
        ([gx0, gy0 + 0.08, gz0], [gx1, gy0 + 0.48, gz0 + 0.12]),
        ([gx0, gy0 + 0.08, gz1 - 0.12], [gx1, gy0 + 0.48, gz1]),
        ([gx1 - 0.12, gy0 + 0.08, gz0], [gx1, gy0 + 0.48, gz1]),
        ([gx0, gy0 + 0.08, gz0], [gx0 + 0.12, gy0 + 0.48, gz0 + 2.2]),
        ([gx0, gy0 + 0.08, gz1 - 2.2], [gx0 + 0.12, gy0 + 0.48, gz1]),
    ] {
        b.span(garden_wall, lo, hi);
    }
    // Slim dark-steel greenhouse columns, eaves beams, and ridge beam
    for &cz in &[gz0, gz0 + 2.1, gz1 - 2.1, gz1] {
        for &cx in &[gx0, gx1] {
            b.span(
                p.metal,
                [cx - 0.035, gy0 + 0.08, cz - 0.035],
                [cx + 0.035, gy_wall, cz + 0.035],
            );
        }
        let gx_mid = (gx0 + gx1) * 0.5;
        b.beam(
            p.metal,
            Vec3::new(gx0, gy_wall, cz),
            Vec3::new(gx_mid, gy_ridge, cz),
            0.035,
        );
        b.beam(
            p.metal,
            Vec3::new(gx1, gy_wall, cz),
            Vec3::new(gx_mid, gy_ridge, cz),
            0.035,
        );
    }
    for &cx in &[gx0, gx1] {
        b.span(
            p.metal,
            [cx - 0.035, gy_wall - 0.04, gz0],
            [cx + 0.035, gy_wall + 0.02, gz1],
        );
    }
    b.span(
        p.metal,
        [(gx0 + gx1) * 0.5 - 0.04, gy_ridge - 0.04, gz0],
        [(gx0 + gx1) * 0.5 + 0.04, gy_ridge + 0.02, gz1],
    );
    // Glazed side walls and pitched glass roof panels
    b.span(
        p.glass,
        [gx1 - 0.05, gy0 + 0.48, gz0 + 0.08],
        [gx1 - 0.02, gy_wall - 0.04, gz1 - 0.08],
    );
    b.span(
        p.glass,
        [gx0 + 0.08, gy0 + 0.48, gz0 + 0.02],
        [gx1 - 0.08, gy_wall - 0.04, gz0 + 0.05],
    );
    b.span(
        p.glass,
        [gx0 + 0.08, gy0 + 0.48, gz1 - 0.05],
        [gx1 - 0.08, gy_wall - 0.04, gz1 - 0.02],
    );
    // Interior botanical tables, terracotta/ceramic planters, and citrus saplings inside the conservatory
    b.span(
        p.timber,
        [gx1 - 0.85, gy0 + 0.08, gz0 + 0.45],
        [gx1 - 0.25, gy0 + 0.78, gz1 - 0.45],
    );
    for pz in [gz0 + 1.1, gz0 + 2.4, gz0 + 3.8, gz0 + 5.1] {
        b.span(
            p.ceramic_cyl,
            [gx1 - 0.72, gy0 + 0.78, pz - 0.22],
            [gx1 - 0.38, gy0 + 1.08, pz + 0.22],
        );
        b.span(
            p.foliage_sphere,
            [gx1 - 0.78, gy0 + 1.08, pz - 0.28],
            [gx1 - 0.32, gy0 + 1.58, pz + 0.28],
        );
    }
    for pz in [gz0 + 1.2, gz1 - 1.2] {
        b.span(
            p.ceramic_cyl,
            [gx0 + 0.35, gy0 + 0.08, pz - 0.28],
            [gx0 + 0.90, gy0 + 0.52, pz + 0.28],
        );
        tree(b, p, Vec3::new(gx0 + 0.62, gy0 + 0.52, pz), 1.45, 0.55, 18);
    }

    // ---- west garden classical stone colonnade & pergola walk (x = -13.6..-11.6, z = 112.0..124.5) ----
    for i in 0..6 {
        let cz = 112.5 + i as f32 * 2.3;
        for cx in [-13.4f32, -11.8] {
            b.span(
                travertine_cyl,
                [cx - 0.16, 0.12, cz - 0.16],
                [cx + 0.16, 2.35, cz + 0.16],
            );
            b.span(
                p.trim,
                [cx - 0.22, 0.12, cz - 0.22],
                [cx + 0.22, 0.24, cz + 0.22],
            );
            b.span(
                p.trim,
                [cx - 0.22, 2.25, cz - 0.22],
                [cx + 0.22, 2.38, cz + 0.22],
            );
        }
        b.span(
            p.timber,
            [-13.65, 2.46, cz - 0.06],
            [-11.55, 2.58, cz + 0.06],
        );
    }
    for cx in [-13.4f32, -11.8] {
        b.span(p.timber, [cx - 0.07, 2.38, 112.1], [cx + 0.07, 2.48, 124.4]);
    }
    b.span(travertine, [-13.2, 0.10, 112.2], [-12.0, 0.15, 124.3]);

    // ---- rooftop solar array & architectural skylight on the upper roof (y = u1 + 0.3) ----
    let roof_top = u1 + 0.3;
    // Framed glass skylight over the upper stairwell/gallery
    b.span(
        p.metal,
        [-7.8, roof_top, 141.0],
        [-4.6, roof_top + 0.18, 143.6],
    );
    b.span(
        p.glass,
        [-7.65, roof_top + 0.18, 141.15],
        [-4.75, roof_top + 0.22, 143.45],
    );
    // Angled photovoltaic solar panels on the east half of the roof
    for row in 0..2 {
        let sz = 139.2 + row as f32 * 3.1;
        for col in 0..4 {
            let sx = -1.5 + col as f32 * 2.6;
            b.span(
                p.metal,
                [sx - 0.9, roof_top, sz - 0.55],
                [sx + 0.9, roof_top + 0.14, sz - 0.45],
            );
            b.span(
                p.metal,
                [sx - 0.9, roof_top, sz + 0.45],
                [sx + 0.9, roof_top + 0.38, sz + 0.55],
            );
            b.put(
                mosaic_dark,
                Vec3::new(sx, roof_top + 0.28, sz),
                Vec3::new(2.1, 0.04, 1.35),
                Quat::from_rotation_x(-0.22),
            );
        }
    }

    // ---- poolside outdoor shower alcove & towel cabana (x = -6.2..-5.2, z = 127.6..130.0) ----
    // Architectural travertine privacy screen, teak duckboard grating, chrome rain-shower column, and towel rack
    b.span(
        travertine,
        [-6.15, DECK, 127.7],
        [-5.92, DECK + 2.25, 129.9],
    );
    b.span(
        cladding,
        [-6.18, DECK + 0.12, 127.85],
        [-6.15, DECK + 2.15, 129.75],
    );
    for slat in 0..7 {
        let sz = 127.95 + slat as f32 * 0.28;
        b.span(
            p.timber,
            [-5.90, DECK, sz - 0.08],
            [-5.25, DECK + 0.035, sz + 0.08],
        );
    }
    b.span(
        chrome,
        [-5.92, DECK + 0.04, 128.76],
        [-5.86, DECK + 2.10, 128.84],
    );
    b.span(
        chrome,
        [-5.86, DECK + 2.06, 128.76],
        [-5.48, DECK + 2.10, 128.84],
    );
    b.span(
        chrome,
        [-5.62, DECK + 2.02, 128.66],
        [-5.38, DECK + 2.06, 128.94],
    );
    b.span(
        chrome,
        [-5.90, DECK + 1.05, 128.72],
        [-5.82, DECK + 1.12, 128.88],
    );
    // Towel rack bench and folded pool towels beside the shower
    b.span(
        p.timber,
        [-5.88, DECK, 129.25],
        [-5.42, DECK + 0.46, 129.82],
    );
    b.span(
        towel,
        [-5.82, DECK + 0.46, 129.32],
        [-5.48, DECK + 0.54, 129.75],
    );

    // ---- southwest zen meditation timber pavilion & bamboo screen (x = -14.1..-12.3, z = 136.0..144.0) ----
    let (mx0, mx1, mz0, mz1) = (-14.1f32, -12.35f32, 136.2f32, 143.8f32);
    let my0 = 0.12f32;
    let my_deck = 0.26f32;
    b.span(
        concrete,
        [mx0 + 0.08, my0, mz0 + 0.08],
        [mx1 - 0.08, my_deck - 0.04, mz1 - 0.08],
    );
    b.span(p.timber, [mx0, my_deck - 0.04, mz0], [mx1, my_deck, mz1]);
    // Stepping stones from the west colonnade/lawn to the meditation deck
    for sz in [133.4f32, 134.8] {
        b.span(
            travertine,
            [-13.4, my0, sz - 0.32],
            [-12.5, my0 + 0.05, sz + 0.32],
        );
    }
    // Low Japanese chabudai tea table and floor zabuton cushions on the timber deck
    let tz_center = (mz0 + mz1) * 0.5;
    let tx_center = (mx0 + mx1) * 0.5;
    b.span(
        p.wood,
        [tx_center - 0.42, my_deck + 0.24, tz_center - 0.65],
        [tx_center + 0.42, my_deck + 0.29, tz_center + 0.65],
    );
    for (lx, lz) in [(-0.34, -0.55), (0.34, -0.55), (-0.34, 0.55), (0.34, 0.55)] {
        b.span(
            p.wood,
            [tx_center + lx - 0.03, my_deck, tz_center + lz - 0.03],
            [tx_center + lx + 0.03, my_deck + 0.24, tz_center + lz + 0.03],
        );
    }
    b.span(
        p.ceramic,
        [tx_center - 0.10, my_deck + 0.29, tz_center - 0.10],
        [tx_center + 0.10, my_deck + 0.40, tz_center + 0.10],
    );
    for (cz, col_idx) in [(tz_center - 1.15, 0usize), (tz_center + 1.15, 1)] {
        b.span(
            fabric(col_idx),
            [tx_center - 0.36, my_deck, cz - 0.36],
            [tx_center + 0.36, my_deck + 0.08, cz + 0.36],
        );
    }
    // Carved stone lantern (Toro) and bamboo/foliage planter screen along the outer edge
    b.span(
        travertine,
        [mx0 + 0.18, my_deck, mz1 - 0.85],
        [mx0 + 0.58, my_deck + 0.12, mz1 - 0.45],
    );
    b.span(
        travertine_cyl,
        [mx0 + 0.28, my_deck + 0.12, mz1 - 0.75],
        [mx0 + 0.48, my_deck + 0.68, mz1 - 0.55],
    );
    b.span(
        p.glow_cyl,
        [mx0 + 0.26, my_deck + 0.68, mz1 - 0.77],
        [mx0 + 0.50, my_deck + 0.90, mz1 - 0.53],
    );
    b.span(
        travertine,
        [mx0 + 0.14, my_deck + 0.90, mz1 - 0.89],
        [mx0 + 0.62, my_deck + 1.04, mz1 - 0.41],
    );
    for i in 0..5 {
        let bz = mz0 + 0.7 + i as f32 * 1.45;
        b.span(
            p.ceramic_cyl,
            [mx0 - 0.35, my0, bz - 0.22],
            [mx0 - 0.04, my0 + 0.38, bz + 0.22],
        );
        b.span(
            p.foliage,
            [mx0 - 0.38, my0 + 0.38, bz - 0.26],
            [mx0 - 0.02, my0 + 1.85, bz + 0.26],
        );
    }

    // ---- southeast wood-fired outdoor pizza oven & alfresco dining terrace (x = 12.35..14.1, z = 136.2..143.8) ----
    let (ox0, ox1, oz0, oz1) = (12.35f32, 14.1f32, 136.2f32, 143.8f32);
    b.span(travertine, [ox0, my0, oz0], [ox1, my0 + 0.10, oz1]);
    // Stepping stones from the east conservatory/pergola lawn to the alfresco terrace
    for sz in [133.4f32, 134.8] {
        b.span(
            travertine,
            [12.5, my0, sz - 0.32],
            [13.4, my0 + 0.05, sz + 0.32],
        );
    }
    // Brick and stucco wood-fired pizza oven with stacked firewood niche, dome, chimney, and glowing hearth
    let oz_oven = oz1 - 1.15;
    b.span(
        render,
        [ox1 - 1.15, my0 + 0.10, oz_oven - 0.75],
        [ox1 - 0.12, my0 + 0.92, oz_oven + 0.75],
    );
    b.span(
        travertine,
        [ox1 - 1.20, my0 + 0.92, oz_oven - 0.80],
        [ox1 - 0.08, my0 + 0.98, oz_oven + 0.80],
    );
    // Firewood logs stacked in the lower base niche
    for log_i in 0..4 {
        let lz = oz_oven - 0.36 + log_i as f32 * 0.24;
        b.span(
            p.trunk,
            [ox1 - 1.10, my0 + 0.16, lz - 0.08],
            [ox1 - 0.35, my0 + 0.34, lz + 0.08],
        );
    }
    // Brick oven dome, glowing ember hearth, and blackened steel flue chimney
    b.span(
        p.ceramic_sphere,
        [ox1 - 1.08, my0 + 0.96, oz_oven - 0.64],
        [ox1 - 0.16, my0 + 1.78, oz_oven + 0.64],
    );
    b.span(
        p.glow,
        [ox1 - 1.06, my0 + 1.04, oz_oven - 0.26],
        [ox1 - 0.88, my0 + 1.34, oz_oven + 0.26],
    );
    b.span(
        p.metal_cyl,
        [ox1 - 0.72, my0 + 1.72, oz_oven - 0.12],
        [ox1 - 0.50, my0 + 2.58, oz_oven + 0.12],
    );
    b.span(
        p.metal_sphere,
        [ox1 - 0.78, my0 + 2.58, oz_oven - 0.18],
        [ox1 - 0.44, my0 + 2.66, oz_oven + 0.18],
    );
    // Alfresco teak dining table, benches, and terracotta herb pots in the northern half of the terrace
    let oz_table = oz0 + 2.25;
    let ox_table = (ox0 + ox1) * 0.5;
    b.span(
        p.timber,
        [ox_table - 0.44, my0 + 0.82, oz_table - 1.05],
        [ox_table + 0.44, my0 + 0.87, oz_table + 1.05],
    );
    for (lx, lz) in [(-0.36, -0.92), (0.36, -0.92), (-0.36, 0.92), (0.36, 0.92)] {
        b.span(
            p.metal,
            [ox_table + lx - 0.03, my0 + 0.10, oz_table + lz - 0.03],
            [ox_table + lx + 0.03, my0 + 0.82, oz_table + lz + 0.03],
        );
    }
    b.span(
        p.ceramic_cyl,
        [ox_table - 0.14, my0 + 0.87, oz_table - 0.14],
        [ox_table + 0.14, my0 + 1.04, oz_table + 0.14],
    );
    for bz_off in [-1.38f32, 1.38] {
        b.span(
            p.timber,
            [ox_table - 0.36, my0 + 0.10, oz_table + bz_off - 0.20],
            [ox_table + 0.36, my0 + 0.52, oz_table + bz_off + 0.20],
        );
        b.span(
            cushion,
            [ox_table - 0.34, my0 + 0.52, oz_table + bz_off - 0.18],
            [ox_table + 0.34, my0 + 0.57, oz_table + bz_off + 0.18],
        );
    }

    // ---- ground-floor living room modern fireplace hearth & minimalist wall art (z = 136.8..141.2) ----
    // Architectural travertine chimney breast along the east interior wall (x = 11.15..11.68) with warm glowing firebox
    b.span(
        travertine,
        [11.15, floor_top, 137.0],
        [11.68, g - 0.02, 139.2],
    );
    b.span(
        concrete,
        [10.85, floor_top, 136.75],
        [11.68, floor_top + 0.28, 139.45],
    );
    b.span(
        p.metal,
        [11.10, floor_top + 0.36, 137.45],
        [11.22, floor_top + 0.98, 138.75],
    );
    b.span(
        p.glow,
        [11.14, floor_top + 0.42, 137.58],
        [11.26, floor_top + 0.68, 138.62],
    );
    // Framed canvas triptych on the rear dining wall (z = 147.62)
    for art_i in 0..3 {
        let ax = -5.8 + art_i as f32 * 1.05;
        b.span(
            p.metal,
            [ax - 0.38, floor_top + 1.25, 147.60],
            [ax + 0.38, floor_top + 2.15, 147.66],
        );
        b.span(
            fabric(art_i + 1),
            [ax - 0.34, floor_top + 1.29, 147.57],
            [ax + 0.34, floor_top + 2.11, 147.61],
        );
    }

    // ---- west garden bocce / pétanque court & spectator bench (x = -14.2..-11.2, z = 127.4..132.8) ----
    let (bx0, bx1, bz0, bz1) = (-14.15f32, -11.35f32, 127.6f32, 132.6f32);
    // Compacted gravel/mortar playing surface and timber bumper surround
    b.span(
        concrete,
        [bx0 + 0.12, 0.10, bz0 + 0.12],
        [bx1 - 0.12, 0.14, bz1 - 0.12],
    );
    rect_perimeter_walls(b, p.timber, [bx0, 0.10, bz0], [bx1, 0.24, bz1], 0.12);
    // Pallino target jack (ceramic) and chrome/ceramic bocce boules clustered near the far end
    b.put(
        p.ceramic_sphere,
        Vec3::new(-12.68, 0.165, 131.15),
        Vec3::splat(0.05),
        Quat::IDENTITY,
    );
    for (bx_ball, bz_ball, mesh_id) in [
        (-12.52f32, 130.92f32, chrome_sphere),
        (-12.88, 131.28, chrome_sphere),
        (-12.42, 131.36, p.ceramic_sphere),
        (-12.95, 130.74, p.ceramic_sphere),
    ] {
        b.put(
            mesh_id,
            Vec3::new(bx_ball, 0.19, bz_ball),
            Vec3::splat(0.10),
            Quat::IDENTITY,
        );
    }
    // Travertine and teak player bench with scoreboard post along the west side of the court
    b.span(
        travertine,
        [bx0 - 0.55, 0.10, 129.3],
        [bx0 - 0.18, 0.44, 130.9],
    );
    b.span(
        p.timber,
        [bx0 - 0.58, 0.44, 129.25],
        [bx0 - 0.15, 0.50, 130.95],
    );
    b.span(
        p.metal,
        [bx0 - 0.36, 0.10, 131.45],
        [bx0 - 0.30, 1.28, 131.51],
    );
    b.span(
        p.wood,
        [bx0 - 0.38, 0.96, 131.30],
        [bx0 - 0.28, 1.26, 131.66],
    );

    // ---- upstairs master suite private terrace planters & interior floor lamp ----
    for px in [1.8f32, 5.8, 9.4] {
        planter_box(b, p, px, u0, 135.60, [0.34, 0.18], 0.38, 0.48);
    }
    // Minimalist brass/dark-steel reading floor lamp beside the master suite lounge
    b.span(
        p.metal_cyl,
        [9.85, u0 + 0.02, 136.85],
        [10.15, u0 + 0.05, 137.15],
    );
    b.span(
        p.metal_cyl,
        [9.98, u0 + 0.05, 136.98],
        [10.02, u0 + 1.58, 137.02],
    );
    b.span(
        p.glow_cyl,
        [9.82, u0 + 1.54, 136.82],
        [10.18, u0 + 1.78, 137.18],
    );

    // ---- east garden Nordic cedar sauna cabin & cold-plunge wellness spa alcove (x = 11.45..14.15, z = 126.8..132.4) ----
    let (sx0, sx1, sz0, sz1) = (11.55f32, 14.05f32, 127.0f32, 132.2f32);
    let sy0 = 0.12f32;
    // Travertine spa deck and stepping stones connecting to the pool pergola
    b.span(
        travertine,
        [sx0 - 0.15, sy0, sz0 - 0.15],
        [sx1 + 0.15, sy0 + 0.08, sz1 + 0.15],
    );
    // Nordic cedar sauna cabin in the northern half (z = 127.15..130.15)
    let sz_cabin1 = 130.15f32;
    let sy_sauna_top = sy0 + 2.38;
    b.span(
        p.timber,
        [sx0 + 0.08, sy0 + 0.08, sz0 + 0.12],
        [sx1 - 0.08, sy0 + 0.14, sz_cabin1 - 0.12],
    );
    // Rear, north, and east cedar walls + overhanging fascia roof
    b.span(
        cladding,
        [sx0, sy0 + 0.08, sz0 + 0.05],
        [sx1, sy_sauna_top, sz0 + 0.22],
    );
    b.span(
        cladding,
        [sx0, sy0 + 0.08, sz_cabin1 - 0.22],
        [sx1, sy_sauna_top, sz_cabin1 - 0.05],
    );
    b.span(
        cladding,
        [sx1 - 0.20, sy0 + 0.08, sz0 + 0.22],
        [sx1, sy_sauna_top, sz_cabin1 - 0.22],
    );
    b.span(
        cladding,
        [sx0 - 0.12, sy_sauna_top, sz0 - 0.05],
        [sx1 + 0.12, sy_sauna_top + 0.16, sz_cabin1 + 0.05],
    );
    b.span(
        p.metal,
        [sx0 - 0.14, sy_sauna_top + 0.16, sz0 - 0.07],
        [sx1 + 0.14, sy_sauna_top + 0.20, sz_cabin1 + 0.07],
    );
    // West panoramic glass facade facing the pool & pergola
    b.span(
        p.metal,
        [sx0 + 0.02, sy0 + 0.08, sz0 + 0.22],
        [sx0 + 0.08, sy_sauna_top, sz0 + 0.28],
    );
    b.span(
        p.metal,
        [sx0 + 0.02, sy0 + 0.08, sz_cabin1 - 0.28],
        [sx0 + 0.08, sy_sauna_top, sz_cabin1 - 0.22],
    );
    b.span(
        p.metal,
        [sx0 + 0.02, sy0 + 0.08, (sz0 + sz_cabin1) * 0.5 - 0.03],
        [sx0 + 0.08, sy_sauna_top, (sz0 + sz_cabin1) * 0.5 + 0.03],
    );
    b.span(
        p.glass,
        [sx0 + 0.04, sy0 + 0.14, sz0 + 0.28],
        [sx0 + 0.06, sy_sauna_top - 0.06, sz_cabin1 - 0.28],
    );
    // Interior two-tier cedar sauna benches and warm under-bench LED strip
    b.span(
        p.timber,
        [sx1 - 1.15, sy0 + 0.14, sz0 + 0.26],
        [sx1 - 0.65, sy0 + 0.52, sz_cabin1 - 0.26],
    );
    b.span(
        p.timber,
        [sx1 - 0.68, sy0 + 0.14, sz0 + 0.26],
        [sx1 - 0.22, sy0 + 0.92, sz_cabin1 - 0.26],
    );
    b.span(
        p.glow,
        [sx1 - 0.72, sy0 + 0.86, sz0 + 0.35],
        [sx1 - 0.68, sy0 + 0.90, sz_cabin1 - 0.35],
    );
    // Finnish sauna stove with basalt stones and warm ember glow in the northwest corner
    b.span(
        p.metal,
        [sx0 + 0.24, sy0 + 0.14, sz0 + 0.34],
        [sx0 + 0.68, sy0 + 0.78, sz0 + 0.78],
    );
    b.span(
        mosaic_dark,
        [sx0 + 0.28, sy0 + 0.78, sz0 + 0.38],
        [sx0 + 0.64, sy0 + 0.88, sz0 + 0.74],
    );
    b.span(
        p.glow,
        [sx0 + 0.32, sy0 + 0.72, sz0 + 0.42],
        [sx0 + 0.60, sy0 + 0.84, sz0 + 0.70],
    );
    // Travertine & cedar cold-plunge tub with chrome filler spout in the southern half (z = 130.55..132.05)
    let (cz0, cz1) = (130.55f32, 132.05f32);
    rect_perimeter_walls(
        b,
        travertine,
        [sx0 + 0.15, sy0 + 0.08, cz0],
        [sx1 - 0.25, sy0 + 0.74, cz1],
        0.14,
    );
    b.span(
        mosaic,
        [sx0 + 0.29, sy0 + 0.08, cz0 + 0.14],
        [sx1 - 0.39, sy0 + 0.62, cz1 - 0.14],
    );
    b.span(
        p.glass,
        [sx0 + 0.29, sy0 + 0.64, cz0 + 0.14],
        [sx1 - 0.39, sy0 + 0.66, cz1 - 0.14],
    );
    b.span(
        p.timber,
        [sx0 - 0.05, sy0 + 0.08, cz0 + 0.25],
        [sx0 + 0.15, sy0 + 0.42, cz1 - 0.25],
    );
    b.span(
        chrome,
        [sx1 - 0.24, sy0 + 0.08, (cz0 + cz1) * 0.5 - 0.03],
        [sx1 - 0.18, sy0 + 0.92, (cz0 + cz1) * 0.5 + 0.03],
    );
    b.span(
        chrome,
        [sx1 - 0.44, sy0 + 0.88, (cz0 + cz1) * 0.5 - 0.03],
        [sx1 - 0.18, sy0 + 0.92, (cz0 + cz1) * 0.5 + 0.03],
    );

    // ---- rooftop astronomical observatory rotunda & refractor telescope (x = -6.8..-4.2, z = 135.2..137.8, y = roof_top) ----
    let (rx_c, rz_c) = (-5.5f32, 136.5f32);
    // Stepped travertine & dark-steel circular/octagonal observatory plinth
    b.span(
        travertine_cyl,
        [rx_c - 1.38, roof_top, rz_c - 1.38],
        [rx_c + 1.38, roof_top + 0.10, rz_c + 1.38],
    );
    b.span(
        cladding_cyl,
        [rx_c - 1.22, roof_top + 0.10, rz_c - 1.22],
        [rx_c + 1.22, roof_top + 0.16, rz_c + 1.22],
    );
    // Open steel & glass observatory pavilion columns and hemispherical cupola dome
    for angle_col in 0..6 {
        let a = angle_col as f32 * std::f32::consts::FRAC_PI_3;
        let dx = a.cos() * 1.12;
        let dz = a.sin() * 1.12;
        b.span(
            p.metal_cyl,
            [rx_c + dx - 0.04, roof_top + 0.16, rz_c + dz - 0.04],
            [rx_c + dx + 0.04, roof_top + 2.18, rz_c + dz + 0.04],
        );
    }
    b.span(
        p.metal_cyl,
        [rx_c - 1.26, roof_top + 2.16, rz_c - 1.26],
        [rx_c + 1.26, roof_top + 2.28, rz_c + 1.26],
    );
    b.span(
        p.metal_sphere,
        [rx_c - 1.18, roof_top + 2.02, rz_c - 1.18],
        [rx_c + 1.18, roof_top + 2.74, rz_c + 1.18],
    );
    // Equatorial mount tripod, counterweight shaft, and brass/chrome refractor telescope tube aimed at the southern sky
    for angle_i in 0..3 {
        let a = angle_i as f32 * 2.094395;
        let lx = rx_c + a.cos() * 0.36;
        let lz = rz_c + a.sin() * 0.36;
        b.beam(
            p.metal_cyl,
            Vec3::new(lx, roof_top + 0.16, lz),
            Vec3::new(rx_c, roof_top + 1.18, rz_c),
            0.03,
        );
    }
    b.span(
        chrome,
        [rx_c - 0.06, roof_top + 1.14, rz_c - 0.06],
        [rx_c + 0.06, roof_top + 1.36, rz_c + 0.06],
    );
    let tele_rot = Quat::from_rotation_y(0.24) * Quat::from_rotation_x(1.05);
    b.put(
        chrome,
        Vec3::new(rx_c, roof_top + 1.48, rz_c - 0.08),
        Vec3::new(0.12, 1.18, 0.12),
        tele_rot,
    );
    b.put(
        p.metal_cyl,
        Vec3::new(rx_c + 0.10, roof_top + 1.18, rz_c + 0.32),
        Vec3::new(0.06, 0.28, 0.06),
        tele_rot,
    );
    // Astronomer's chart table, star-atlas lamp, and upholstered observation stool
    b.span(
        p.timber,
        [rx_c + 0.45, roof_top + 0.16, rz_c - 0.35],
        [rx_c + 0.98, roof_top + 0.78, rz_c + 0.35],
    );
    b.span(
        p.glow_cyl,
        [rx_c + 0.64, roof_top + 0.78, rz_c - 0.08],
        [rx_c + 0.80, roof_top + 0.92, rz_c + 0.08],
    );

    // ---- vineyard pergola & stone wine cellar tasting alcove in the east parterre garden (x = 8.8..12.6, z = 98.6..103.4) ----
    let (wx0, wx1, wz0, wz1) = (9.0f32, 12.2f32, 98.8f32, 103.2f32);
    let wy0 = 0.10f32;
    b.span(
        travertine,
        [wx0 - 0.15, wy0, wz0 - 0.15],
        [wx1 + 0.15, wy0 + 0.08, wz1 + 0.15],
    );
    // Arched stone wine cellar grotto wall along the east edge with oak barrel niche
    b.span(
        garden_wall,
        [wx1 - 0.45, wy0 + 0.08, wz0],
        [wx1, wy0 + 2.25, wz1],
    );
    b.span(
        travertine,
        [wx1 - 0.52, wy0 + 2.25, wz0 - 0.08],
        [wx1 + 0.06, wy0 + 2.38, wz1 + 0.08],
    );
    // Aged oak wine barrels (horizontal cylinders along X) stacked on timber cradles against the cellar wall
    let barrel_rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    for (bz_bar, by_bar) in [
        (wz0 + 0.85, wy0 + 0.38),
        (wz0 + 2.20, wy0 + 0.38),
        (wz0 + 3.55, wy0 + 0.38),
        (wz0 + 1.52, wy0 + 0.92),
        (wz0 + 2.88, wy0 + 0.92),
    ] {
        let bx_bar = wx1 - 0.82;
        b.put(
            cladding_cyl,
            Vec3::new(bx_bar, by_bar, bz_bar),
            Vec3::new(0.54, 0.68, 0.54),
            barrel_rot,
        );
        for dx_hoop in [-0.22f32, 0.22] {
            b.put(
                p.metal_cyl,
                Vec3::new(bx_bar + dx_hoop, by_bar, bz_bar),
                Vec3::new(0.56, 0.04, 0.56),
                barrel_rot,
            );
        }
    }
    // Timber vineyard trellis posts, overhead beams, and hanging grapevine foliage canopy
    for &vz in &[wz0 + 0.2, (wz0 + wz1) * 0.5, wz1 - 0.2] {
        b.span(
            p.timber,
            [wx0 + 0.12, wy0 + 0.08, vz - 0.05],
            [wx0 + 0.22, wy0 + 2.25, vz + 0.05],
        );
        b.span(
            p.timber,
            [wx0 + 0.05, wy0 + 2.25, vz - 0.05],
            [wx1 - 0.45, wy0 + 2.35, vz + 0.05],
        );
    }
    b.span(
        p.foliage,
        [wx0 + 0.15, wy0 + 2.18, wz0 + 0.15],
        [wx1 - 0.50, wy0 + 2.48, wz1 - 0.15],
    );
    b.span(
        p.glow,
        [wx1 - 0.52, wy0 + 1.85, wz0 + 0.6],
        [wx1 - 0.46, wy0 + 1.92, wz1 - 0.6],
    );
    // Sommelier tasting table, wine decanter, and oak stools under the vine trellis
    let wz_mid = (wz0 + wz1) * 0.5;
    b.span(
        p.wood,
        [wx0 + 0.65, wy0 + 0.08, wz_mid - 0.55],
        [wx0 + 1.35, wy0 + 0.88, wz_mid + 0.55],
    );
    b.span(
        travertine,
        [wx0 + 0.60, wy0 + 0.88, wz_mid - 0.60],
        [wx0 + 1.40, wy0 + 0.94, wz_mid + 0.60],
    );
    b.span(
        p.glass,
        [wx0 + 0.92, wy0 + 0.94, wz_mid - 0.12],
        [wx0 + 1.08, wy0 + 1.18, wz_mid + 0.04],
    );
    for sz_st in [wz_mid - 0.85, wz_mid + 0.85] {
        b.span(
            cladding_cyl,
            [wx0 + 0.82, wy0 + 0.08, sz_st - 0.18],
            [wx0 + 1.18, wy0 + 0.58, sz_st + 0.18],
        );
    }

    // ---- northwest parterre artist atelier pavilion & sculpture garden (x = -12.2..-9.0, z = 98.8..103.2) ----
    let (ax0, ax1, az0, az1) = (-12.2f32, -9.0f32, 98.8f32, 103.2f32);
    let ay0 = 0.10f32;
    b.span(
        travertine,
        [ax0 - 0.15, ay0, az0 - 0.15],
        [ax1 + 0.15, ay0 + 0.08, az1 + 0.15],
    );
    // West studio wall, overhanging travertine cornice, and steel/glass north-light studio frame
    b.span(
        garden_wall,
        [ax0, ay0 + 0.08, az0],
        [ax0 + 0.42, ay0 + 2.25, az1],
    );
    b.span(
        travertine,
        [ax0 - 0.06, ay0 + 2.25, az0 - 0.08],
        [ax0 + 0.50, ay0 + 2.38, az1 + 0.08],
    );
    for &az_col in &[az0 + 0.2, (az0 + az1) * 0.5, az1 - 0.2] {
        b.span(
            p.metal_cyl,
            [ax1 - 0.22, ay0 + 0.08, az_col - 0.04],
            [ax1 - 0.14, ay0 + 2.25, az_col + 0.04],
        );
        b.span(
            p.metal,
            [ax0 + 0.42, ay0 + 2.20, az_col - 0.04],
            [ax1 - 0.10, ay0 + 2.28, az_col + 0.04],
        );
    }
    b.span(
        p.glass,
        [ax0 + 0.45, ay0 + 2.28, az0 + 0.18],
        [ax1 - 0.15, ay0 + 2.32, az1 - 0.18],
    );
    b.span(
        p.glow,
        [ax0 + 0.46, ay0 + 2.14, az0 + 0.65],
        [ax0 + 0.52, ay0 + 2.20, az1 - 0.65],
    );
    // Artist's timber easel, framed painting canvas, and painter's palette table
    let (ex_c, ez_c) = (-10.35f32, 100.35f32);
    for (dx_leg, dz_leg) in [(-0.26f32, -0.24f32), (-0.26, 0.24), (0.28, 0.0)] {
        b.beam(
            p.timber,
            Vec3::new(ex_c + dx_leg, ay0 + 0.08, ez_c + dz_leg),
            Vec3::new(ex_c, ay0 + 1.72, ez_c),
            0.032,
        );
    }
    b.span(
        p.wood,
        [ex_c - 0.08, ay0 + 0.88, ez_c - 0.42],
        [ex_c + 0.06, ay0 + 0.92, ez_c + 0.42],
    );
    b.put(
        canvas,
        Vec3::new(ex_c - 0.04, ay0 + 1.24, ez_c),
        Vec3::new(0.04, 0.68, 0.76),
        Quat::from_rotation_z(-0.14),
    );
    b.put(
        fabric(2),
        Vec3::new(ex_c - 0.065, ay0 + 1.24, ez_c),
        Vec3::new(0.015, 0.56, 0.64),
        Quat::from_rotation_z(-0.14),
    );
    // Painter's side table with oval wooden palette, pigment dabs, ceramic brush jar, and studio spotlight
    let (pt_x, pt_z) = (ex_c + 0.52, ez_c + 0.58);
    b.span(
        p.timber,
        [pt_x - 0.24, ay0 + 0.08, pt_z - 0.24],
        [pt_x + 0.24, ay0 + 0.72, pt_z + 0.24],
    );
    b.span(
        p.wood,
        [pt_x - 0.28, ay0 + 0.72, pt_z - 0.28],
        [pt_x + 0.28, ay0 + 0.76, pt_z + 0.28],
    );
    b.put(
        cladding_cyl,
        Vec3::new(pt_x - 0.06, ay0 + 0.77, pt_z - 0.04),
        Vec3::new(0.26, 0.018, 0.18),
        Quat::from_rotation_y(0.4),
    );
    for (dab_dx, dab_dz, dab_m) in [
        (-0.12f32, -0.06f32, fabric(0)),
        (-0.04, -0.09, fabric(1)),
        (0.02, -0.02, p.glow),
    ] {
        b.put(
            dab_m,
            Vec3::new(pt_x + dab_dx, ay0 + 0.785, pt_z + dab_dz),
            Vec3::new(0.035, 0.014, 0.035),
            Quat::IDENTITY,
        );
    }
    b.put(
        p.ceramic_cyl,
        Vec3::new(pt_x + 0.14, ay0 + 0.83, pt_z + 0.10),
        Vec3::new(0.09, 0.14, 0.09),
        Quat::IDENTITY,
    );
    b.beam(
        p.timber,
        Vec3::new(pt_x + 0.14, ay0 + 0.84, pt_z + 0.10),
        Vec3::new(pt_x + 0.19, ay0 + 1.02, pt_z + 0.13),
        0.008,
    );
    b.beam(
        p.timber,
        Vec3::new(pt_x + 0.14, ay0 + 0.84, pt_z + 0.10),
        Vec3::new(pt_x + 0.10, ay0 + 1.01, pt_z + 0.07),
        0.008,
    );
    b.put(
        p.metal_cyl,
        Vec3::new(ex_c + 0.35, ay0 + 2.16, ez_c),
        Vec3::new(0.10, 0.14, 0.10),
        Quat::from_rotation_z(0.55),
    );
    b.put(
        p.bulb,
        Vec3::new(ex_c + 0.31, ay0 + 2.12, ez_c),
        Vec3::splat(0.065),
        Quat::IDENTITY,
    );
    // Classical marble/ceramic sculpture bust on a fluted travertine column pedestal in the southern half
    let (sx_ped, sz_ped) = (-10.85f32, 102.05f32);
    b.span(
        travertine_cyl,
        [sx_ped - 0.28, ay0 + 0.08, sz_ped - 0.28],
        [sx_ped + 0.28, ay0 + 0.96, sz_ped + 0.28],
    );
    b.span(
        travertine_cyl,
        [sx_ped - 0.34, ay0 + 0.96, sz_ped - 0.34],
        [sx_ped + 0.34, ay0 + 1.04, sz_ped + 0.34],
    );
    b.put(
        p.ceramic_sphere,
        Vec3::new(sx_ped, ay0 + 1.26, sz_ped),
        Vec3::new(0.36, 0.42, 0.26),
        Quat::from_rotation_y(0.35),
    );
    b.put(
        p.ceramic_cyl,
        Vec3::new(sx_ped, ay0 + 1.48, sz_ped),
        Vec3::new(0.11, 0.14, 0.11),
        Quat::IDENTITY,
    );
    b.put(
        p.ceramic_sphere,
        Vec3::new(sx_ped, ay0 + 1.62, sz_ped),
        Vec3::new(0.20, 0.26, 0.22),
        Quat::from_rotation_y(0.35),
    );
    planter_box(
        b,
        p,
        ax0 + 0.85,
        ay0 + 0.08,
        az0 + 0.55,
        [0.24, 0.24],
        0.34,
        0.46,
    );

    // ---- upstairs gallery billiards / pool table & overhead pendant (x = -5.8..-3.4, z = 141.8..144.2, y = u0) ----
    let (bx_t0, bx_t1, bz_t0, bz_t1) = (-5.7f32, -3.5f32, 141.9f32, 144.1f32);
    for (lx, lz) in [
        (bx_t0 + 0.16, bz_t0 + 0.16),
        (bx_t1 - 0.16, bz_t0 + 0.16),
        (bx_t0 + 0.16, bz_t1 - 0.16),
        (bx_t1 - 0.16, bz_t1 - 0.16),
    ] {
        b.span(
            cladding_cyl,
            [lx - 0.06, u0 + 0.02, lz - 0.06],
            [lx + 0.06, u0 + 0.74, lz + 0.06],
        );
    }
    b.span(p.wood, [bx_t0, u0 + 0.72, bz_t0], [bx_t1, u0 + 0.82, bz_t1]);
    rect_perimeter_walls(
        b,
        p.wood,
        [bx_t0 - 0.04, u0 + 0.80, bz_t0 - 0.04],
        [bx_t1 + 0.04, u0 + 0.86, bz_t1 + 0.04],
        0.10,
    );
    b.span(
        lawn,
        [bx_t0 + 0.06, u0 + 0.82, bz_t0 + 0.06],
        [bx_t1 - 0.06, u0 + 0.84, bz_t1 - 0.06],
    );
    for (ball_x, ball_z, ball_m) in [
        (-4.60f32, 142.55f32, p.ceramic_sphere),
        (-4.52, 143.35, chrome_sphere),
        (-4.68, 143.42, p.ceramic_sphere),
        (-4.42, 143.52, p.bulb),
    ] {
        b.put(
            ball_m,
            Vec3::new(ball_x, u0 + 0.865, ball_z),
            Vec3::splat(0.055),
            Quat::IDENTITY,
        );
    }
    // 6 billiard pockets (corners and mid-rails), wooden triangle rack, and two tapered cue sticks
    let bz_mid = (bz_t0 + bz_t1) * 0.5;
    for (pk_x, pk_z) in [
        (bx_t0 + 0.05, bz_t0 + 0.05),
        (bx_t1 - 0.05, bz_t0 + 0.05),
        (bx_t0 + 0.04, bz_mid),
        (bx_t1 - 0.04, bz_mid),
        (bx_t0 + 0.05, bz_t1 - 0.05),
        (bx_t1 - 0.05, bz_t1 - 0.05),
    ] {
        b.put(
            p.metal_cyl,
            Vec3::new(pk_x, u0 + 0.845, pk_z),
            Vec3::new(0.11, 0.03, 0.11),
            Quat::IDENTITY,
        );
    }
    b.beam(
        p.timber,
        Vec3::new(-4.82, u0 + 0.855, 143.62),
        Vec3::new(-4.32, u0 + 0.855, 143.62),
        0.016,
    );
    b.beam(
        p.timber,
        Vec3::new(-4.82, u0 + 0.855, 143.62),
        Vec3::new(-4.57, u0 + 0.855, 143.20),
        0.016,
    );
    b.beam(
        p.timber,
        Vec3::new(-4.32, u0 + 0.855, 143.62),
        Vec3::new(-4.57, u0 + 0.855, 143.20),
        0.016,
    );
    b.beam(
        p.timber,
        Vec3::new(bx_t0 - 0.12, u0 + 0.87, bz_t0 + 0.25),
        Vec3::new(bx_t0 + 0.95, u0 + 0.86, bz_t0 + 1.10),
        0.015,
    );
    b.beam(
        p.timber,
        Vec3::new(bx_t1 + 0.12, u0 + 0.87, bz_t1 - 0.30),
        Vec3::new(bx_t1 - 0.92, u0 + 0.86, bz_t1 - 1.05),
        0.015,
    );

    Villa {
        water: vec![WaterBody::pool(
            Vec3::new(-x, DEEP, z0),
            Vec3::new(x, WATER, z1),
        )],
        view: (
            Vec3::new(2.9, 1.82, 107.4),
            Vec3::new(-0.22, -0.2, 1.).normalize(),
        ),
    }
}

/// Estimates normalized steam density in `[0.0, 1.0]` inside the Nordic cedar sauna cabin
/// given stove stone temperature (°C), ladled water volume (mL), cabin volume ($\text{m}^3$),
/// and time elapsed since ladling (seconds).
#[allow(dead_code)]
pub fn sauna_steam_density(
    stove_temp_c: f32,
    water_ladled_ml: f32,
    cabin_volume_m3: f32,
    elapsed_s: f32,
) -> f32 {
    if !stove_temp_c.is_finite()
        || !water_ladled_ml.is_finite()
        || !cabin_volume_m3.is_finite()
        || !elapsed_s.is_finite()
        || stove_temp_c <= 100.0
        || water_ladled_ml <= 0.0
        || cabin_volume_m3 <= 0.5
        || elapsed_s < 0.0
    {
        return 0.0;
    }
    let superheat = ((stove_temp_c - 100.0) / 180.0).clamp(0.0, 1.5);
    let specific_load = (water_ladled_ml / (cabin_volume_m3 * 25.0)).clamp(0.0, 2.0);
    // Rapid flash-evaporation rise over ~4s followed by ventilation decay over ~45s.
    let rise = 1.0 - (-elapsed_s / 3.5).exp();
    let decay = (-elapsed_s / 45.0).exp();
    (superheat * specific_load * rise * decay).clamp(0.0, 1.0)
}

#[cfg(test)]
mod sauna_tests {
    use super::*;

    #[test]
    fn sauna_steam_rises_then_decays_with_ventilation() {
        assert_eq!(sauna_steam_density(85.0, 200.0, 12.0, 5.0), 0.0);
        let early = sauna_steam_density(240.0, 250.0, 12.0, 5.0);
        let late = sauna_steam_density(240.0, 250.0, 12.0, 90.0);
        assert!(early > 0.2 && early <= 1.0);
        assert!(late < early);

        let floor_t = sauna_vertical_temperature_profile_c(0.1, 2.2, 28.0, 95.0);
        let top_bench_t = sauna_vertical_temperature_profile_c(1.8, 2.2, 28.0, 95.0);
        assert!(top_bench_t > floor_t && top_bench_t <= 95.0);

        let (az, shutter) = observatory_dome_azimuth_and_shutter([0.0, 0.8, 0.6], false);
        assert!(az.is_finite() && shutter == 1.0);
        assert_eq!(
            observatory_dome_azimuth_and_shutter([0.0, 0.8, 0.6], true).1,
            0.0
        );

        let cellar_amp = wine_grotto_cellar_stability(15.0, 3.5, 0.08);
        assert!(cellar_amp < 2.0);
    }
}

/// Evaluates the stratified vertical air temperature profile in °C inside the Nordic sauna cabin
/// from `height_m` above the floor (`0..=ceiling_height_m`), `floor_temp_c`, and `ceiling_temp_c`.
#[allow(dead_code)]
pub fn sauna_vertical_temperature_profile_c(
    height_m: f32,
    ceiling_height_m: f32,
    floor_temp_c: f32,
    ceiling_temp_c: f32,
) -> f32 {
    if ![height_m, ceiling_height_m, floor_temp_c, ceiling_temp_c]
        .iter()
        .all(|v| v.is_finite())
        || ceiling_height_m <= 0.1
    {
        return floor_temp_c;
    }
    let h_norm = (height_m / ceiling_height_m).clamp(0.0, 1.0);
    // Thermal buoyancy stratification curve (concave near ceiling):
    let s = h_norm.powf(0.75);
    floor_temp_c + (ceiling_temp_c - floor_temp_c) * s
}

/// Computes the rooftop observatory dome rotation `(azimuth_rad, shutter_open_01)` tracking
/// `target_sky_dir` (`[x, y, z]`), automatically closing the slit (`0.0`) during precipitation (`raining`).
#[allow(dead_code)]
pub fn observatory_dome_azimuth_and_shutter(target_sky_dir: [f32; 3], raining: bool) -> (f32, f32) {
    if target_sky_dir.iter().any(|c| !c.is_finite()) {
        return (0.0, 0.0);
    }
    let azimuth = target_sky_dir[0]
        .atan2(target_sky_dir[2])
        .rem_euclid(std::f32::consts::TAU);
    let shutter = if !raining && target_sky_dir[1] > 0.05 {
        1.0
    } else {
        0.0
    };
    (azimuth, shutter)
}

/// Evaluates the damped diurnal/seasonal temperature swing amplitude at underground `depth_m`
/// inside the limestone wine grotto (`surface_swing_c * exp(-depth_m * sqrt(omega / (2 * thermal_diffusivity)))`).
#[allow(dead_code)]
pub fn wine_grotto_cellar_stability(
    surface_swing_c: f32,
    depth_m: f32,
    soil_damping_inv_m: f32,
) -> f32 {
    if !surface_swing_c.is_finite() || !depth_m.is_finite() || !soil_damping_inv_m.is_finite() {
        return 0.0;
    }
    let amp = surface_swing_c.max(0.0);
    let d = depth_m.max(0.0);
    let k = soil_damping_inv_m.max(0.0);
    amp * (-d * (1.0 + k * 6.0)).exp()
}
