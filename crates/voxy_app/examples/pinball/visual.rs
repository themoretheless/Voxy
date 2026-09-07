#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
use super::game::{
    BALL_RADIUS, BUMPER_RADIUS, BUMPERS, FLIPPER_LENGTH, FLIPPER_RADIUS, Game, PIVOTS, WALLS,
};
use glam::{Mat4, Vec3};
use voxy_render::{MaterialLayer, MaterialPack, SkinnedMesh, SkinnedVertex};

#[derive(Default)]
struct Mesh {
    vertices: Vec<SkinnedVertex>,
    indices: Vec<u32>,
}
impl Mesh {
    fn quad(&mut self, points: [Vec3; 4], normal: Vec3, shade: u8, joint: u16) {
        let start = u32::try_from(self.vertices.len()).expect("small example mesh");
        for point in points {
            self.vertices.push(SkinnedVertex {
                position: point.to_array(),
                normal: normal.to_array(),
                uv: [(f32::from(shade) + 0.5) / 8.0, 0.5],
                joints: [joint, 0, 0, 0],
                weights: [u16::MAX, 0, 0, 0],
            });
        }
        if (points[1] - points[0])
            .cross(points[2] - points[0])
            .dot(normal)
            >= 0.0
        {
            self.indices
                .extend([start, start + 1, start + 2, start, start + 2, start + 3]);
        } else {
            self.indices
                .extend([start, start + 2, start + 1, start, start + 3, start + 2]);
        }
    }
    fn slab(&mut self, x: f32, y: f32, w: f32, h: f32, height: f32, shade: u8) {
        self.quad(
            [
                Vec3::new(x, height, -y),
                Vec3::new(x + w, height, -y),
                Vec3::new(x + w, height, -y - h),
                Vec3::new(x, height, -y - h),
            ],
            Vec3::Y,
            shade,
            0,
        );
    }
    fn capsule(
        &mut self,
        a: [f32; 2],
        b: [f32; 2],
        radius: f32,
        height: f32,
        shade: u8,
        joint: u16,
    ) {
        let angle = (b[1] - a[1]).atan2(b[0] - a[0]);
        let mut rim = Vec::new();
        for (center, start) in [
            (b, angle - std::f32::consts::FRAC_PI_2),
            (a, angle + std::f32::consts::FRAC_PI_2),
        ] {
            for i in 0..=12 {
                let theta = start + i as f32 / 12.0 * std::f32::consts::PI;
                rim.push(Vec3::new(
                    center[0] + radius * theta.cos(),
                    height,
                    -center[1] - radius * theta.sin(),
                ));
            }
        }
        let center = Vec3::new((a[0] + b[0]) * 0.5, height, -(a[1] + b[1]) * 0.5);
        for i in 0..rim.len() {
            let p = rim[i];
            let q = rim[(i + 1) % rim.len()];
            self.quad([center, p, q, center], Vec3::Y, shade, joint);
            let normal = Vec3::new(q.z - p.z, 0.0, p.x - q.x).normalize_or_zero();
            self.quad(
                [p, q, Vec3::new(q.x, 0.0, q.z), Vec3::new(p.x, 0.0, p.z)],
                normal,
                shade,
                joint,
            );
        }
    }
    fn sphere(&mut self) {
        let r = BALL_RADIUS as f32;
        for lat in 0..12 {
            for lon in 0..24 {
                let point = |a: i32, b: i32| {
                    let phi = a as f32 / 12.0 * std::f32::consts::PI;
                    let theta = b as f32 / 24.0 * std::f32::consts::TAU;
                    Vec3::new(
                        r * phi.sin() * theta.cos(),
                        r * phi.cos(),
                        r * phi.sin() * theta.sin(),
                    )
                };
                let points = [
                    point(lat, lon),
                    point(lat + 1, lon),
                    point(lat + 1, lon + 1),
                    point(lat, lon + 1),
                ];
                let normal = (points[0] + points[1] + points[2] + points[3]).normalize();
                self.quad(points, normal, 7, 1);
            }
        }
    }
    fn text(&mut self, value: &str, x: f32, y: f32, size: f32, shade: u8) {
        for (column, c) in value.chars().enumerate() {
            let rows = glyph(c);
            for (row, bits) in rows.into_iter().enumerate() {
                for bit in 0..5 {
                    if bits & (1 << (4 - bit)) != 0 {
                        self.slab(
                            x + (column as f32 * 6.0 + bit as f32) * size,
                            y - row as f32 * size,
                            size * 0.8,
                            size * 0.8,
                            0.045,
                            shade,
                        );
                    }
                }
            }
        }
    }
}
fn glyph(c: char) -> [u8; 7] {
    match c {
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        _ => [0; 7],
    }
}
pub fn materials() -> MaterialPack {
    let pixels: Vec<u8> = [38, 65, 92, 120, 150, 185, 220, 255]
        .into_iter()
        .flat_map(|v| [v, v, v, 255])
        .collect();
    MaterialPack::new(
        8,
        1,
        vec![MaterialLayer {
            rgba8_srgb: pixels.into(),
        }],
    )
    .expect("valid palette")
}
pub fn matrices(game: &Game) -> [Mat4; 7] {
    let mut joints = [Mat4::IDENTITY; 7];
    joints[1] = Mat4::from_translation(Vec3::new(
        game.ball.position.x as f32,
        BALL_RADIUS as f32 + 0.06,
        -game.ball.position.y as f32,
    ));
    for (i, pivot) in PIVOTS.into_iter().enumerate() {
        joints[i + 2] = Mat4::from_translation(Vec3::new(pivot.x as f32, 0.0, -pivot.y as f32))
            * Mat4::from_rotation_y(game.angles[i] as f32);
    }
    for (i, p) in BUMPERS.into_iter().enumerate() {
        joints[i + 4] = Mat4::from_translation(Vec3::new(p.x as f32, 0.0, -p.y as f32))
            * Mat4::from_scale(Vec3::new(1.0, 1.0 + game.flashes[i] as f32 * 3.0, 1.0));
    }
    joints
}
pub fn mesh(game: &Game, paused: bool) -> SkinnedMesh {
    let mut m = Mesh::default();
    m.slab(-5.5, -2.2, 11.0, 24.6, -0.12, 3);
    m.slab(-5.18, 0.0, 10.36, 18.25, 0.0, 1);
    m.slab(-5.2, 18.5, 10.4, 3.4, 0.0, 0);
    m.text("VOXY PINBALL", -4.3, 21.5, 0.115, 6);
    m.text(
        &format!("{:07}", game.score.min(9_999_999)),
        -4.3,
        20.15,
        0.13,
        7,
    );
    m.text(&format!("BALLS {}", game.lives), 1.3, 20.15, 0.085, 6);
    m.text("A - L FLIPPERS", -4.5, -0.8, 0.085, 7);
    m.text("SPACE LAUNCH", -4.5, -1.55, 0.075, 6);
    m.text("R RESET", 2.0, -1.55, 0.075, 6);
    for (a, b) in WALLS {
        m.capsule(
            [a.x as f32, a.y as f32],
            [b.x as f32, b.y as f32],
            0.12,
            0.5,
            5,
            0,
        );
    }
    // Inlaid stripes and small square rivets give the table a block-built appearance.
    for i in 0..18 {
        for x in [-5.38, 5.25] {
            m.slab(x, i as f32 + 0.3, 0.12, 0.12, 0.02, 7);
        }
    }
    m.text("100", -1.2, 10.5, 0.12, 4);
    for p in BUMPERS {
        m.capsule(
            [p.x as f32, p.y as f32],
            [p.x as f32, p.y as f32],
            0.9,
            0.04,
            4,
            0,
        );
    }
    for i in 0..3 {
        m.capsule([0.0, 0.0], [0.0, 0.0], BUMPER_RADIUS as f32, 0.55, 7, i + 4);
        m.capsule([0.0, 0.0], [0.0, 0.0], 0.4, 0.59, 3, i + 4);
    }
    for i in 0..2 {
        m.capsule(
            [0.0, 0.0],
            [FLIPPER_LENGTH as f32, 0.0],
            FLIPPER_RADIUS as f32,
            0.27,
            7,
            i + 2,
        );
    }
    let message = if game.lives == 0 {
        "GAME OVER"
    } else if paused {
        "PAUSE"
    } else if game.ready {
        "HOLD SPACE"
    } else {
        ""
    };
    m.text(message, -2.5, 9.2, 0.1, 6);
    m.sphere();
    SkinnedMesh::new(m.vertices, m.indices, 7).expect("valid pinball mesh")
}
