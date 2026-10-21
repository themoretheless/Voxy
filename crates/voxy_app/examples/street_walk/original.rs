//! Independently authored deterministic geometry; CC0 surface scans attach separately.
use super::scene::StreetScene;
use glam::{Mat4, Quat, Vec3};
use physics::static_triangle_world::StaticTriangleWorld;
use voxy_render::archive_scene::*;
#[path = "villa.rs"]
mod villa;
struct Builder {
    materials: Vec<ArchiveMaterialSource>,
    meshes: Vec<ArchiveMeshSource>,
    triangles: Vec<[[f64; 3]; 3]>,
    rng: u64,
    /// Separate stream for ageing details, so changing them leaves houses alone.
    age_rng: u64,
    lights: Vec<ArchiveLight>,
}
impl Builder {
    fn random(&mut self) -> f32 {
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.rng >> 40) as f32) / 16777216.
    }
    fn aged(&mut self) -> f32 {
        self.age_rng = self
            .age_rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.age_rng >> 40) as f32) / 16777216.
    }
    fn material(
        &mut self,
        colour: [f32; 3],
        rough: f32,
        metal: f32,
        pattern: f32,
        emission: f32,
        transmission: f32,
    ) -> usize {
        let id = self.materials.len();
        self.materials.push(ArchiveMaterialSource {
            parameters: ArchiveMaterial {
                diffuse: [colour[0], colour[1], colour[2], 1.],
                specular_gloss: [0.04, 0.04, 0.04, 1.],
                emissive_normal: [
                    colour[0] * emission,
                    colour[1] * emission,
                    colour[2] * emission,
                    1.,
                ],
                surface: [
                    metal,
                    rough,
                    transmission,
                    if transmission > 0. { -2. } else { -1. },
                ],
                workflow: [1., 0., pattern, 0.],
            },
            images: [None; 4],
            double_sided: false,
        });
        id
    }
    fn mesh(&mut self, material: usize, vertices: Vec<ArchiveVertex>, indices: Vec<u32>) -> usize {
        let id = self.meshes.len();
        self.meshes.push(ArchiveMeshSource {
            vertices,
            indices,
            instances: Vec::new(),
            material,
            center: Vec3::ZERO,
        });
        id
    }
    fn cube(&mut self, material: usize) -> usize {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for normal in [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::Z,
            Vec3::NEG_Z,
        ] {
            let tangent = if normal.y.abs() > 0.5 {
                Vec3::X
            } else {
                Vec3::Y
            };
            let bitangent = normal.cross(tangent);
            let offset = vertices.len() as u32;
            for (u, v) in [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
                vertices.push(ArchiveVertex {
                    position: (normal * 0.5 + tangent * u + bitangent * v).to_array(),
                    normal: normal.to_array(),
                    tangent: [tangent.x, tangent.y, tangent.z, 1.],
                    uv: [u + 0.5, v + 0.5],
                });
            }
            indices.extend([
                offset,
                offset + 1,
                offset + 2,
                offset,
                offset + 2,
                offset + 3,
            ]);
        }
        self.mesh(material, vertices, indices)
    }
    fn bevel_cube(&mut self, material: usize) -> usize {
        self.bevel_cube_e(material, 0.42)
    }
    /// Bevelled unit cube; a smaller `e` gives a rounder, more worn edge.
    fn bevel_cube_e(&mut self, material: usize, e: f32) -> usize {
        let axes = [Vec3::X, Vec3::Y, Vec3::Z];
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut face = |mut points: Vec<Vec3>, normal: Vec3| {
            if (points[1] - points[0])
                .cross(points[2] - points[0])
                .dot(normal)
                < 0.
            {
                points.reverse();
            }
            let first = vertices.len() as u32;
            let tangent = (points[1] - points[0]).normalize();
            for point in &points {
                vertices.push(ArchiveVertex {
                    position: point.to_array(),
                    normal: normal.to_array(),
                    tangent: [tangent.x, tangent.y, tangent.z, 1.],
                    uv: [0., 0.],
                });
            }
            for i in 1..points.len() - 1 {
                indices.extend([first, first + i as u32, first + i as u32 + 1]);
            }
        };
        for axis in 0..3 {
            for sign in [-1., 1.] {
                let n = axes[axis] * sign;
                let t = axes[(axis + 1) % 3];
                let b = axes[(axis + 2) % 3];
                face(
                    vec![
                        n * 0.5 - t * e - b * e,
                        n * 0.5 + t * e - b * e,
                        n * 0.5 + t * e + b * e,
                        n * 0.5 - t * e + b * e,
                    ],
                    n,
                );
            }
        }
        for a in 0..3 {
            for b in a + 1..3 {
                let c = 3 - a - b;
                for sa in [-1., 1.] {
                    for sb in [-1., 1.] {
                        let na = axes[a] * sa;
                        let nb = axes[b] * sb;
                        let t = axes[c];
                        face(
                            vec![
                                na * 0.5 + nb * e - t * e,
                                na * e + nb * 0.5 - t * e,
                                na * e + nb * 0.5 + t * e,
                                na * 0.5 + nb * e + t * e,
                            ],
                            (na + nb).normalize(),
                        );
                    }
                }
            }
        }
        for x in [-1., 1.] {
            for y in [-1., 1.] {
                for z in [-1., 1.] {
                    face(
                        vec![
                            Vec3::new(x * 0.5, y * e, z * e),
                            Vec3::new(x * e, y * 0.5, z * e),
                            Vec3::new(x * e, y * e, z * 0.5),
                        ],
                        Vec3::new(x, y, z).normalize(),
                    );
                }
            }
        }
        self.mesh(material, vertices, indices)
    }
    fn leaf(&mut self, material: usize) -> usize {
        let outline = [
            (0., -0.5),
            (-0.22, -0.25),
            (-0.32, 0.),
            (-0.22, 0.25),
            (0., 0.5),
            (0.22, 0.25),
            (0.32, 0.),
            (0.22, -0.25),
        ];
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for layer in 0..2 {
            let sign = if layer == 0 { 1. } else { -1. };
            vertices.push(ArchiveVertex {
                position: [0., if layer == 0 { 0.045 } else { -0.005 }, 0.],
                normal: [0., sign, 0.],
                tangent: [1., 0., 0., sign],
                uv: [0.5, 0.5],
            });
            for (x, z) in outline {
                vertices.push(ArchiveVertex {
                    position: [x, if layer == 0 { 0. } else { -0.005 }, z],
                    normal: [0., sign, 0.],
                    tangent: [1., 0., 0., sign],
                    uv: [x + 0.5, z + 0.5],
                });
            }
            let offset = layer * 9;
            for e in 0..8 {
                let a = offset + 1 + e;
                let b = offset + 1 + (e + 1) % 8;
                if layer == 0 {
                    indices.extend([offset, a, b]);
                } else {
                    indices.extend([offset, b, a]);
                }
            }
        }
        for e in 0..8 {
            let a = 1 + e;
            let c = 1 + (e + 1) % 8;
            indices.extend([a, a + 9, c, c, a + 9, c + 9]);
        }
        // Smooth area-weighted normals follow the existing curved membrane.
        // The complete original positions, triangles and collision remain unchanged.
        let mut normals = vec![Vec3::ZERO; vertices.len()];
        for triangle in indices.chunks_exact(3) {
            let p: [Vec3; 3] =
                std::array::from_fn(|i| Vec3::from_array(vertices[triangle[i] as usize].position));
            let normal = (p[1] - p[0]).cross(p[2] - p[0]);
            for &i in triangle {
                normals[i as usize] += normal;
            }
        }
        for (vertex, normal) in vertices.iter_mut().zip(normals) {
            if normal.length_squared() > 0. {
                vertex.normal = normal.normalize().to_array();
            }
        }
        self.mesh(material, vertices, indices)
    }
    fn round(&mut self, material: usize, sphere: bool) -> usize {
        let sides = 20usize;
        let rows = if sphere { 12 } else { 1 };
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for row in 0..=rows {
            let v = row as f32 / rows as f32;
            let latitude = v * std::f32::consts::PI;
            for col in 0..=sides {
                let u = col as f32 / sides as f32;
                let angle = u * std::f32::consts::TAU;
                let normal = if sphere {
                    Vec3::new(
                        angle.cos() * latitude.sin(),
                        latitude.cos(),
                        angle.sin() * latitude.sin(),
                    )
                } else {
                    Vec3::new(angle.cos(), 0., angle.sin())
                };
                let p = if sphere {
                    normal * 0.5
                } else {
                    Vec3::new(normal.x * 0.5, v - 0.5, normal.z * 0.5)
                };
                let t = Vec3::new(-angle.sin(), 0., angle.cos());
                vertices.push(ArchiveVertex {
                    position: p.to_array(),
                    normal: normal.to_array(),
                    tangent: [t.x, t.y, t.z, 1.],
                    uv: [u, v],
                });
            }
        }
        for row in 0..rows {
            for col in 0..sides {
                let a = (row * (sides + 1) + col) as u32;
                let b = a + (sides + 1) as u32;
                indices.extend([a, a + 1, b, a + 1, b + 1, b]);
            }
        }
        // Correct cylinder winding; cylinder rows progress upward, sphere rows downward.
        if !sphere {
            for tri in indices.chunks_exact_mut(3) {
                tri.swap(1, 2);
            }
        }
        self.mesh(material, vertices, indices)
    }
    fn tapered_branch(&mut self, material: usize) -> usize {
        let id = self.round(material, false);
        for vertex in &mut self.meshes[id].vertices {
            let y = vertex.position[1] + 0.5;
            let taper = 1.0 - 0.55 * y;
            vertex.position[0] *= taper;
            vertex.position[2] *= taper;
            let n = Vec3::new(vertex.normal[0], 0.275, vertex.normal[2]).normalize();
            vertex.normal = n.to_array();
        }
        id
    }
    fn transform_new_instances(
        &mut self,
        starts: &[usize],
        triangle_start: usize,
        transform: Mat4,
    ) {
        self.triangles.truncate(triangle_start);
        for (mesh_index, mesh) in self.meshes.iter_mut().enumerate() {
            let start = starts.get(mesh_index).copied().unwrap_or(0);
            for instance in &mut mesh.instances[start..] {
                let world = transform * Mat4::from_cols_array_2d(&instance.world);
                *instance = ArchiveInstance::new(world);
                for ids in mesh.indices.chunks_exact(3) {
                    self.triangles.push(std::array::from_fn(|i| {
                        world
                            .as_dmat4()
                            .transform_point3(
                                Vec3::from_array(mesh.vertices[ids[i] as usize].position)
                                    .as_dvec3(),
                            )
                            .to_array()
                    }));
                }
            }
        }
    }
    fn put(&mut self, id: usize, position: Vec3, scale: Vec3, rotation: Quat) {
        let world = Mat4::from_scale_rotation_translation(scale, rotation, position);
        let mesh = &mut self.meshes[id];
        mesh.instances.push(ArchiveInstance::new(world));
        mesh.center = position;
        for ids in mesh.indices.chunks_exact(3) {
            let t = std::array::from_fn(|i| {
                world
                    .as_dmat4()
                    .transform_point3(
                        Vec3::from_array(mesh.vertices[ids[i] as usize].position).as_dvec3(),
                    )
                    .to_array()
            });
            self.triangles.push(t);
        }
    }
    fn block(&mut self, id: usize, p: [f32; 3], s: [f32; 3]) {
        self.put(id, Vec3::from_array(p), Vec3::from_array(s), Quat::IDENTITY)
    }
    fn beam(&mut self, id: usize, a: Vec3, b: Vec3, r: f32) {
        let delta = b - a;
        self.put(
            id,
            (a + b) * 0.5,
            Vec3::new(r, delta.length(), r),
            Quat::from_rotation_arc(Vec3::Y, delta.normalize()),
        );
    }
}
/// A room's local frame: `p[0]` is depth from the facade's inner face, `p[1]`
/// height above the finished floor, `p[2]` offset along the street from the
/// house centre.
#[derive(Clone, Copy)]
struct Room {
    side: f32,
    z: f32,
    base: f32,
    paint: usize,
    wood: usize,
    fabric: usize,
    accent: usize,
}
impl Room {
    fn put(&self, b: &mut Builder, id: usize, p: [f32; 3], s: [f32; 3]) {
        b.block(
            id,
            [self.side * (7.56 + p[0]), self.base + p[1], self.z + p[2]],
            s,
        );
    }
    fn at(&self, p: [f32; 3]) -> Vec3 {
        Vec3::new(self.side * (7.56 + p[0]), self.base + p[1], self.z + p[2])
    }
}
/// A room measured from the party wall it backs onto: `a` runs from that
/// wall's finished face towards the house centre line, `d` runs inward from
/// `d0` (the facade's inner face for front rooms).
struct Zone {
    r: Room,
    o: f32,
    d0: f32,
}
impl Zone {
    fn put(&self, b: &mut Builder, id: usize, p: [f32; 3], s: [f32; 3]) {
        self.r.put(
            b,
            id,
            [self.d0 + p[2], p[1], self.o * (3.78 - p[0])],
            [s[2], s[1], s[0]],
        );
    }
}
#[derive(Clone)]
struct Kit {
    trim: usize,
    metal: usize,
    glow: usize,
    glass: usize,
    ceramic: usize,
    ceiling: usize,
    tile: usize,
    tile_accent: usize,
    foliage: usize,
    paints: Vec<usize>,
    woods: Vec<usize>,
    fabrics: Vec<usize>,
    books: Vec<usize>,
}
fn pick(b: &mut Builder, ids: &[usize]) -> usize {
    ids[((b.random() * ids.len() as f32) as usize).min(ids.len() - 1)]
}
/// Plan per storey (front = street side, d along depth, u along the street):
///
///   d 0..3.84   two front rooms with all windows, split on a facade pier by a
///               partition with a doorway near the back.
///   d 3.84..3.96 cross wall with doorways to the back rooms and the stairs.
///   d 3.96..6.79 back rooms either side of a central U-shaped stairwell
///               (flight A rises away from the street, a landing at the back,
///               flight B returns to the next floor's cross-wall doorway).
///
/// Furniture is placed per room from the walls that are free of windows and
/// doorways, so nothing blocks a path.
struct Plan {
    side: f32,
    z: f32,
    floors: usize,
    windows: usize,
    spacing: f32,
    opening: f32,
    /// Street door centre (u) and width, if this house has one.
    door: Option<(f32, f32)>,
    door_bay: usize,
}
const RISE: f32 = 2.65 / 16.;
const TREAD: f32 = 1.9 / 7.;
const STOREY: f32 = 2.45;
fn interior(b: &mut Builder, k: &Kit, p: &Plan) {
    let s = -3.75 + p.spacing * (p.windows / 2) as f32;
    let side = p.side;
    let z = p.z;
    let at = |d: f32| side * (7.56 + d);
    // Slabs with the stairwell opening, and a plaster ceiling under each.
    for level in 0..=p.floors {
        let f = level as f32 * 2.65;
        let floor_id = pick(b, &k.woods);
        let hole = level > 0 && level < p.floors;
        let pieces: Vec<(f32, f32, f32, f32)> = if hole {
            vec![
                (0., 3.96, -3.825, 3.825),
                (3.96, 6.79, -3.825, s - 0.95),
                (3.96, 6.79, s + 0.95, 3.825),
            ]
        } else {
            vec![(0., 6.79, -3.825, 3.825)]
        };
        for (d0, d1, u0, u1) in pieces {
            let (dc, uc, dl, ul) = ((d0 + d1) * 0.5, (u0 + u1) * 0.5, d1 - d0, u1 - u0);
            if level == 0 {
                b.block(floor_id, [at(dc), 0.03, z + uc], [dl, 0.06, ul]);
            } else {
                b.block(floor_id, [at(dc), f - 0.04, z + uc], [dl, 0.2, ul]);
                b.block(k.ceiling, [at(dc), f - 0.145, z + uc], [dl, 0.01, ul]);
            }
        }
    }
    for level in 0..p.floors {
        let f = level as f32 * 2.65;
        let base = if level == 0 { 0.06 } else { f + 0.06 };
        let stairs = level + 1 < p.floors;
        let neutral = pick(b, &k.paints);
        let wall = Room {
            side,
            z,
            base,
            paint: neutral,
            wood: pick(b, &k.woods),
            fabric: pick(b, &k.fabrics),
            accent: pick(b, &k.fabrics),
        };
        // Cross wall between front rooms and the back of the house.
        let mut openings: Vec<(f32, f32)> = vec![(s - 2.0, s - 1.2), (s + 1.2, s + 2.0)];
        if stairs {
            openings.push((s - 0.94, s - 0.06));
        }
        if level > 0 {
            openings.push((s + 0.06, s + 0.94));
        }
        openings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut u = -3.825;
        for &(o0, o1) in &openings {
            wall.put(
                b,
                neutral,
                [3.9, STOREY * 0.5, (u + o0) * 0.5],
                [0.12, STOREY, o0 - u],
            );
            wall.put(
                b,
                neutral,
                [3.9, 2.1 + (STOREY - 2.1) * 0.5, (o0 + o1) * 0.5],
                [0.12, STOREY - 2.1, o1 - o0],
            );
            u = o1;
        }
        wall.put(
            b,
            neutral,
            [3.9, STOREY * 0.5, (u + 3.825) * 0.5],
            [0.12, STOREY, 3.825 - u],
        );
        for &(o0, o1) in &openings {
            let back_door = (o0 - (s - 2.0)).abs() < 0.01 || (o0 - (s + 1.2)).abs() < 0.01;
            if !back_door {
                continue; // stair openings stay open and uncased
            }
            casing(b, k, &wall, 3.9, o0, o1, true);
        }
        // Partition between the two front rooms, doorway near the cross wall.
        for (d0, d1) in [(0., 2.6), (3.5, 3.84)] {
            wall.put(
                b,
                neutral,
                [(d0 + d1) * 0.5, STOREY * 0.5, s],
                [d1 - d0, STOREY, 0.1],
            );
        }
        wall.put(
            b,
            neutral,
            [3.05, 2.1 + (STOREY - 2.1) * 0.5, s],
            [0.9, STOREY - 2.1, 0.1],
        );
        casing(b, k, &wall, s, 2.6, 3.5, false);
        // Stairwell side walls and the spine between the flights.
        for e in [-1., 1.] {
            wall.put(
                b,
                neutral,
                [5.375, STOREY * 0.5, s + e * 1.0],
                [2.83, STOREY, 0.1],
            );
        }
        let spine_bottom = if level > 0 { -0.2 } else { 0. };
        let spine_top = if stairs { STOREY } else { 1.0 };
        wall.put(
            b,
            neutral,
            [4.91, (spine_bottom + spine_top) * 0.5, s],
            [1.9, spine_top - spine_bottom, 0.1],
        );
        wall.put(
            b,
            neutral,
            [6.76, (spine_bottom + STOREY) * 0.5, s],
            [0.04, STOREY - spine_bottom, 1.9],
        );
        if stairs {
            for i in 1..=7 {
                let top = i as f32 * RISE;
                let d0 = 3.965 + (i - 1) as f32 * TREAD;
                wall.put(
                    b,
                    wall.wood,
                    [d0 + TREAD * 0.5, top - 0.1, s - 0.5],
                    [TREAD, 0.2, 0.88],
                );
                let top = (8 + i) as f32 * RISE;
                let d1 = 5.86 - (i - 1) as f32 * TREAD;
                let d0 = (d1 - TREAD).max(3.965);
                wall.put(
                    b,
                    wall.wood,
                    [(d0 + d1) * 0.5, top - 0.1, s + 0.5],
                    [d1 - d0, 0.2, 0.88],
                );
            }
            wall.put(b, wall.wood, [6.3, 8. * RISE - 0.1, s], [0.88, 0.2, 1.88]);
            for (e, h0, h1) in [(-1., 0., 8. * RISE), (1., 16. * RISE, 8. * RISE)] {
                let (d0, d1) = (3.96, 5.86);
                b.beam(
                    k.metal,
                    wall.at([d0, h0 + 0.9, s + e * 0.91]),
                    wall.at([d1, h1 + 0.9, s + e * 0.91]),
                    0.04,
                );
            }
            wall.put(b, k.glow, [6.3, STOREY - 0.02, s], [0.3, 0.04, 0.3]);
        }
        // Rooms. Front rooms: left (o = -1) and right (o = +1).
        let w_of = |o: f32| 3.73 - o * s;
        let door_room = if level == 0 {
            p.door
                .map(|(du, dw)| (if du > s { 1. } else { -1. }, du, dw))
        } else {
            None
        };
        let mut kinds = [Kind::Living, Kind::Study];
        let day_first = b.random() < 0.5;
        for (n, o) in [-1f32, 1.].into_iter().enumerate() {
            let narrow = w_of(o) < 3.2;
            let day = (n == 0) == day_first;
            kinds[n] = if narrow {
                if b.random() < 0.5 {
                    Kind::Study
                } else {
                    Kind::SingleBed
                }
            } else if day {
                if b.random() < 0.55 {
                    Kind::Living
                } else {
                    Kind::Kitchen
                }
            } else if b.random() < 0.7 {
                Kind::DoubleBed
            } else {
                Kind::Study
            };
        }
        if let Some((o, _, _)) = door_room {
            kinds[if o < 0. { 0 } else { 1 }] = Kind::Entry;
        }
        let has_kitchen = kinds.contains(&Kind::Kitchen);
        let bath_left = b.random() < 0.5;
        for (n, o) in [-1f32, 1.].into_iter().enumerate() {
            let w = w_of(o);
            let room = Room {
                side,
                z,
                base,
                paint: pick(b, &k.paints),
                wood: pick(b, &k.woods),
                fabric: pick(b, &k.fabrics),
                accent: pick(b, &k.fabrics),
            };
            let front = Zone { r: room, o, d0: 0. };
            linings(b, k, &front, room.paint, w, 3.84, false);
            curtains(b, k, &front, p, level, s, o);
            let door_a = door_room
                .filter(|d| d.0 == o)
                .map(|(_, du, dw)| (3.78 - o * du, dw));
            furnish(b, k, &front, kinds[n], w, door_a);
            ceiling_lamp(b, k, &front, w * 0.5, 1.9);
            // Back room on the same side.
            let wb = w - 1.0;
            let bath = (o < 0.) == bath_left;
            let back_kind = if bath {
                Kind::Bath
            } else if has_kitchen {
                Kind::Laundry
            } else {
                Kind::BackKitchen
            };
            let paint = if bath { k.tile } else { pick(b, &k.paints) };
            let back = Zone {
                r: Room { paint, ..room },
                o,
                d0: 3.96,
            };
            linings(b, k, &back, paint, wb, 2.78, true);
            furnish(b, k, &back, back_kind, wb, None);
            ceiling_lamp(b, k, &back, wb * 0.5, 1.4);
        }
    }
}
fn casing(b: &mut Builder, k: &Kit, r: &Room, at: f32, o0: f32, o1: f32, across_u: bool) {
    // Doorway casing straddling a 0.1-0.12 m wall: jambs and head 0.06 wide.
    let put = |b: &mut Builder, c: f32, y: f32, along: f32, h: f32| {
        if across_u {
            r.put(b, k.trim, [at, y, c], [0.16, h, along]);
        } else {
            r.put(b, k.trim, [c, y, at], [along, h, 0.14]);
        }
    };
    put(b, o0, 1.06, 0.06, 2.12);
    put(b, o1, 1.06, 0.06, 2.12);
    put(b, (o0 + o1) * 0.5, 2.1, o1 - o0 + 0.06, 0.06);
}
fn linings(b: &mut Builder, k: &Kit, zn: &Zone, paint: usize, w: f32, depth: f32, back: bool) {
    zn.put(
        b,
        paint,
        [-0.02, STOREY * 0.5, depth * 0.5],
        [0.04, STOREY, depth],
    );
    zn.put(b, k.trim, [0.0125, 0.05, depth * 0.5], [0.025, 0.1, depth]);
    zn.put(
        b,
        k.trim,
        [0.015, STOREY - 0.03, depth * 0.5],
        [0.03, 0.06, depth],
    );
    if back {
        zn.put(
            b,
            paint,
            [w * 0.5, STOREY * 0.5, depth + 0.02],
            [w, STOREY, 0.04],
        );
        zn.put(
            b,
            k.trim,
            [w * 0.5 + 0.02, 0.05, depth - 0.0125],
            [w - 0.04, 0.1, 0.025],
        );
    }
}
fn curtains(b: &mut Builder, k: &Kit, zn: &Zone, p: &Plan, level: usize, s: f32, o: f32) {
    for window in 0..p.windows {
        let wu = -3.75 + p.spacing * (window as f32 + 0.5);
        if (wu > s) != (o > 0.) {
            continue;
        }
        if level == 0 && p.door.is_some() && window == p.door_bay {
            continue;
        }
        let open = b.random();
        zn.r.put(b, k.metal, [0.12, 2.3, wu], [0.02, 0.02, p.opening + 0.6]);
        for cs in [-1., 1.] {
            let w = 0.2 + open * (p.opening * 0.45);
            let u = wu + cs * (p.opening * 0.5 + 0.3 - w * 0.5);
            for fold in 0..4 {
                let fw = w / 4.;
                zn.r.put(
                    b,
                    zn.r.accent,
                    [
                        0.1 + (fold % 2) as f32 * 0.03,
                        1.2,
                        u - w * 0.5 + fw * (fold as f32 + 0.5),
                    ],
                    [0.02, 2.2, fw * 0.98],
                );
            }
        }
    }
}
fn ceiling_lamp(b: &mut Builder, k: &Kit, zn: &Zone, a: f32, d: f32) {
    let lit = b.random() < 0.55;
    if b.random() < 0.5 {
        zn.put(b, k.metal, [a, STOREY - 0.25, d], [0.012, 0.5, 0.012]);
        zn.put(
            b,
            if lit { k.glow } else { k.metal },
            [a, STOREY - 0.56, d],
            [0.36, 0.14, 0.36],
        );
    } else {
        zn.put(
            b,
            if lit { k.glow } else { k.ceramic },
            [a, STOREY - 0.02, d],
            [0.5, 0.04, 0.5],
        );
    }
}
fn pictures(b: &mut Builder, k: &Kit, zn: &Zone, d: f32, y: f32, n: usize) {
    for i in 0..n {
        let pd = d + (i as f32 - (n - 1) as f32 * 0.5) * 0.7;
        let (pw, ph) = (0.35 + b.random() * 0.25, 0.3 + b.random() * 0.3);
        zn.put(b, zn.r.wood, [0.012, y, pd], [0.02, ph + 0.06, pw + 0.06]);
        let canvas = pick(b, &k.books);
        zn.put(b, canvas, [0.024, y, pd], [0.01, ph, pw]);
    }
}
fn plant(b: &mut Builder, k: &Kit, zn: &Zone, a: f32, d: f32) {
    zn.put(b, k.ceramic, [a, 0.17, d], [0.3, 0.34, 0.3]);
    for n in 0..7 {
        let t = n as f32 * 0.9;
        let c = zn.r.at([
            zn.d0 + d + t.sin() * 0.12,
            0.55 + (n % 3) as f32 * 0.15,
            zn.o * (3.78 - a - t.cos() * 0.12),
        ]);
        b.put(
            k.foliage,
            c,
            Vec3::new(0.08, 0.4, 0.22),
            Quat::from_rotation_y(t) * Quat::from_rotation_x(0.35),
        );
    }
}
fn chair(b: &mut Builder, zn: &Zone, a: f32, d: f32, back: [f32; 2]) {
    let wood = zn.r.wood;
    for (da, dd) in [(-0.17, -0.17), (-0.17, 0.17), (0.17, -0.17), (0.17, 0.17)] {
        zn.put(b, wood, [a + da, 0.22, d + dd], [0.035, 0.44, 0.035]);
    }
    zn.put(b, wood, [a, 0.46, d], [0.4, 0.04, 0.4]);
    let (ba, bd) = (a + back[0] * 0.18, d + back[1] * 0.18);
    let size = if back[0] != 0. {
        [0.035, 0.48, 0.4]
    } else {
        [0.4, 0.48, 0.035]
    };
    zn.put(b, wood, [ba, 0.72, bd], size);
}
fn shelf_unit(b: &mut Builder, k: &Kit, zn: &Zone, d: f32, len: f32) {
    let wood = zn.r.wood;
    for e in [-0.5, 0.5] {
        zn.put(b, wood, [0.19, 0.95, d + e * len], [0.36, 1.9, 0.04]);
    }
    zn.put(b, wood, [0.01, 0.95, d], [0.02, 1.9, len]);
    for shelf in 0..5 {
        let sy = 0.02 + shelf as f32 * 0.46;
        zn.put(b, wood, [0.19, sy, d], [0.36, 0.03, len - 0.04]);
        if shelf == 4 {
            continue;
        }
        let mut bd = d - len * 0.5 + 0.03;
        let end = d + len * 0.5 - 0.05 - b.random() * 0.35;
        while bd < end {
            let w = 0.025 + b.random() * 0.05;
            let h = 0.22 + b.random() * 0.17;
            let id = pick(b, &k.books);
            let inset = b.random() * 0.04;
            zn.put(
                b,
                id,
                [0.15 + inset, sy + 0.015 + h * 0.5, bd + w * 0.5],
                [0.2, h, w],
            );
            bd += w + 0.004;
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Entry,
    Living,
    Kitchen,
    DoubleBed,
    SingleBed,
    Study,
    Bath,
    Laundry,
    BackKitchen,
}
/// `w` is the clear room width from the party-wall lining to the opposite
/// wall face. Front rooms keep the window strip (d < 0.35), the partition
/// doorway (a > w - 0.5, d 2.6..3.5) and the cross-wall doorways
/// (a > w - 2.0, d > 3.3) clear. Back rooms keep their doorway
/// (a w - 0.95..w - 0.15 at d = 0) clear.
fn furnish(b: &mut Builder, k: &Kit, zn: &Zone, kind: Kind, w: f32, door: Option<(f32, f32)>) {
    let r = zn.r;
    let lamp = |b: &mut Builder, a: f32, y: f32, d: f32| {
        zn.put(b, k.metal, [a, y + 0.08, d], [0.05, 0.16, 0.05]);
        let shade = if b.random() < 0.5 { k.glow } else { k.ceramic };
        zn.put(b, shade, [a, y + 0.22, d], [0.2, 0.14, 0.2]);
    };
    match kind {
        Kind::Entry => {
            let (da, dw) = door.unwrap_or((w * 0.5, 1.0));
            zn.put(b, r.accent, [da, 0.006, 1.4], [dw.min(1.0), 0.012, 2.2]);
            zn.put(b, r.wood, [0.03, 1.72, 1.3], [0.04, 0.12, 0.9]);
            for n in 0..4 {
                zn.put(
                    b,
                    k.metal,
                    [0.08, 1.7, 0.95 + n as f32 * 0.24],
                    [0.07, 0.03, 0.03],
                );
            }
            zn.put(b, r.fabric, [0.15, 1.3, 1.05], [0.18, 0.75, 0.32]);
            zn.put(b, r.accent, [0.14, 1.35, 1.55], [0.16, 0.68, 0.3]);
            zn.put(b, r.wood, [0.2, 0.45, 2.45], [0.36, 0.9, 1.0]);
            zn.put(b, k.ceramic, [0.2, 0.96, 2.3], [0.16, 0.12, 0.16]);
            zn.put(b, r.wood, [0.012, 1.55, 2.45], [0.02, 0.9, 0.62]);
            zn.put(b, k.metal, [0.024, 1.55, 2.45], [0.01, 0.8, 0.52]);
            if da + 0.4 < w - 0.5 {
                zn.put(b, r.wood, [w - 0.22, 0.42, 1.4], [0.38, 0.05, 1.2]);
                for dd in [-0.55, 0.55] {
                    zn.put(b, r.wood, [w - 0.22, 0.2, 1.4 + dd], [0.36, 0.4, 0.05]);
                }
            }
            plant(b, k, zn, 0.3, 3.5);
        }
        Kind::Living => {
            zn.put(b, r.accent, [1.5, 0.006, 2.0], [1.9, 0.012, 2.4]);
            zn.put(b, r.fabric, [0.46, 0.22, 2.0], [0.86, 0.32, 2.0]);
            zn.put(b, r.fabric, [0.13, 0.62, 2.0], [0.22, 0.5, 2.0]);
            for d in [0.95, 3.05] {
                zn.put(b, r.fabric, [0.46, 0.48, d], [0.86, 0.3, 0.18]);
            }
            for d in [1.53, 2.47] {
                zn.put(b, r.fabric, [0.55, 0.43, d], [0.68, 0.1, 0.9]);
            }
            for d in [1.3, 2.7] {
                zn.put(b, r.accent, [0.3, 0.62, d], [0.14, 0.34, 0.38]);
            }
            zn.put(b, r.wood, [1.55, 0.4, 2.0], [0.6, 0.05, 1.1]);
            for (da, dd) in [(-0.25, -0.48), (-0.25, 0.48), (0.25, -0.48), (0.25, 0.48)] {
                zn.put(b, k.metal, [1.55 + da, 0.19, 2.0 + dd], [0.03, 0.38, 0.03]);
            }
            zn.put(b, k.ceramic, [1.5, 0.5, 1.8], [0.1, 0.16, 0.1]);
            zn.put(b, r.wood, [w - 0.25, 0.25, 1.5], [0.4, 0.5, 1.6]);
            zn.put(b, k.metal, [w - 0.04, 1.2, 1.5], [0.03, 0.68, 1.2]);
            zn.put(b, k.metal, [0.3, 0.01, 0.5], [0.28, 0.02, 0.28]);
            zn.put(b, k.metal, [0.3, 0.8, 0.5], [0.025, 1.6, 0.025]);
            let shade = if b.random() < 0.6 { k.glow } else { k.ceramic };
            zn.put(b, shade, [0.3, 1.66, 0.5], [0.34, 0.24, 0.34]);
            plant(b, k, zn, 0.3, 3.5);
            let n = 1 + (b.random() * 3.) as usize;
            pictures(b, k, zn, 2.0, 1.45, n);
        }
        Kind::Kitchen => {
            zn.put(b, r.wood, [0.3, 0.43, 1.7], [0.6, 0.86, 2.4]);
            zn.put(b, k.ceramic, [0.32, 0.88, 1.7], [0.64, 0.04, 2.44]);
            for n in 0..4 {
                let d = 0.8 + n as f32 * 0.6;
                zn.put(b, k.metal, [0.605, 0.72, d], [0.01, 0.02, 0.14]);
                zn.put(b, r.wood, [0.17, 1.85, d], [0.34, 0.7, 0.58]);
                zn.put(b, k.metal, [0.345, 1.6, d], [0.01, 0.02, 0.12]);
            }
            zn.put(b, r.accent, [0.003, 1.2, 1.7], [0.006, 0.6, 2.4]);
            zn.put(b, k.metal, [0.3, 0.905, 1.2], [0.4, 0.02, 0.5]);
            zn.put(b, k.metal, [0.08, 1.05, 1.2], [0.03, 0.3, 0.03]);
            zn.put(b, k.metal, [0.3, 0.91, 2.3], [0.55, 0.02, 0.58]);
            zn.put(b, k.ceramic, [0.33, 0.95, 3.25], [0.62, 1.9, 0.66]);
            zn.put(b, k.metal, [0.645, 1.2, 3.45], [0.03, 0.5, 0.03]);
            let ta = w - 1.1;
            zn.put(b, r.wood, [ta, 0.75, 1.6], [0.9, 0.05, 1.4]);
            for (da, dd) in [(-0.4, -0.65), (-0.4, 0.65), (0.4, -0.65), (0.4, 0.65)] {
                zn.put(b, r.wood, [ta + da, 0.36, 1.6 + dd], [0.05, 0.72, 0.05]);
            }
            chair(b, zn, ta, 0.65, [0., -1.]);
            chair(b, zn, ta, 2.55, [0., 1.]);
            chair(b, zn, ta - 0.75, 1.6, [-1., 0.]);
            chair(b, zn, ta + 0.75, 1.6, [1., 0.]);
            for d in [1.3, 1.9] {
                zn.put(b, k.ceramic, [ta, 0.785, d], [0.26, 0.02, 0.26]);
            }
            zn.put(b, k.glass, [ta + 0.2, 0.85, 1.6], [0.07, 0.15, 0.07]);
        }
        Kind::DoubleBed => {
            zn.put(b, r.wood, [1.01, 0.2, 2.0], [2.0, 0.3, 1.6]);
            zn.put(b, r.wood, [0.035, 0.55, 2.0], [0.06, 1.0, 1.7]);
            zn.put(b, k.ceramic, [1.02, 0.45, 2.0], [1.9, 0.2, 1.5]);
            zn.put(b, r.fabric, [1.3, 0.565, 2.0], [1.4, 0.05, 1.56]);
            for d in [1.62, 2.38] {
                zn.put(b, r.accent, [0.3, 0.62, d], [0.36, 0.12, 0.6]);
            }
            for d in [0.92, 3.08] {
                zn.put(b, r.wood, [0.24, 0.27, d], [0.42, 0.54, 0.4]);
                lamp(b, 0.24, 0.54, d);
            }
            zn.put(b, r.accent, [2.3, 0.006, 2.0], [0.9, 0.012, 2.0]);
            zn.put(b, r.wood, [w - 0.3, 1.0, 1.2], [0.58, 2.0, 1.6]);
            for d in [1.15, 1.25] {
                zn.put(b, k.metal, [w - 0.6, 1.1, d], [0.02, 0.4, 0.02]);
            }
            let n = 1 + (b.random() * 2.) as usize;
            pictures(b, k, zn, 2.0, 1.55, n);
        }
        Kind::SingleBed => {
            zn.put(b, r.wood, [0.48, 0.2, 2.0], [0.95, 0.3, 2.0]);
            zn.put(b, r.wood, [0.48, 0.5, 1.0], [0.95, 0.9, 0.06]);
            zn.put(b, k.ceramic, [0.48, 0.45, 2.02], [0.88, 0.2, 1.92]);
            zn.put(b, r.fabric, [0.48, 0.565, 2.3], [0.94, 0.05, 1.3]);
            zn.put(b, r.accent, [0.48, 0.6, 1.25], [0.6, 0.12, 0.36]);
            zn.put(b, r.wood, [0.25, 0.27, 0.6], [0.42, 0.54, 0.4]);
            lamp(b, 0.25, 0.54, 0.6);
            zn.put(b, r.wood, [w - 0.3, 0.74, 1.3], [0.6, 0.04, 1.0]);
            for dd in [-0.47, 0.47] {
                zn.put(b, r.wood, [w - 0.3, 0.36, 1.3 + dd], [0.56, 0.72, 0.04]);
            }
            chair(b, zn, w - 0.85, 1.3, [-1., 0.]);
            pictures(b, k, zn, 2.2, 1.5, 1);
        }
        Kind::Study => {
            zn.put(b, r.wood, [0.35, 0.74, 1.3], [0.7, 0.04, 1.4]);
            for dd in [-0.68, 0.68] {
                zn.put(b, r.wood, [0.35, 0.36, 1.3 + dd], [0.66, 0.72, 0.04]);
            }
            zn.put(b, k.metal, [0.12, 1.0, 1.3], [0.03, 0.36, 0.6]);
            zn.put(b, k.metal, [0.16, 0.8, 1.3], [0.06, 0.08, 0.06]);
            zn.put(b, k.metal, [0.42, 0.765, 1.3], [0.15, 0.012, 0.45]);
            zn.put(b, k.glow, [0.15, 0.82, 1.85], [0.12, 0.12, 0.12]);
            zn.put(b, k.metal, [0.95, 0.2, 1.3], [0.06, 0.4, 0.06]);
            zn.put(b, r.fabric, [0.95, 0.45, 1.3], [0.48, 0.08, 0.48]);
            zn.put(b, r.fabric, [1.18, 0.78, 1.3], [0.06, 0.55, 0.46]);
            for spoke in 0..5 {
                let t = spoke as f32 * std::f32::consts::TAU / 5.;
                zn.put(
                    b,
                    k.metal,
                    [0.95 + t.cos() * 0.18, 0.03, 1.3 + t.sin() * 0.18],
                    [0.05, 0.04, 0.05],
                );
            }
            shelf_unit(b, k, zn, 2.75, 1.2);
            zn.put(b, r.accent, [w - 0.5, 0.24, 0.95], [0.8, 0.36, 0.8]);
            zn.put(b, r.accent, [w - 0.5, 0.62, 0.62], [0.8, 0.5, 0.14]);
            zn.put(b, r.accent, [w - 0.5, 0.006, 1.6], [1.0, 0.012, 1.4]);
        }
        Kind::Bath => {
            zn.put(b, k.tile_accent, [w * 0.5, 0.004, 1.39], [w, 0.008, 2.78]);
            if w >= 1.75 {
                zn.put(b, k.ceramic, [0.87, 0.28, 2.4], [1.7, 0.56, 0.74]);
                zn.put(b, k.tile_accent, [0.87, 0.565, 2.4], [1.5, 0.02, 0.56]);
                zn.put(b, k.metal, [0.1, 0.72, 2.4], [0.12, 0.05, 0.04]);
            } else {
                zn.put(b, k.ceramic, [0.46, 0.04, 2.3], [0.9, 0.08, 0.9]);
                zn.put(b, k.glass, [0.93, 1.0, 2.3], [0.01, 1.9, 0.9]);
                zn.put(b, k.metal, [0.08, 1.95, 2.3], [0.12, 0.03, 0.12]);
            }
            zn.put(b, k.ceramic, [0.1, 0.75, 1.0], [0.18, 0.4, 0.42]);
            zn.put(b, k.ceramic, [0.42, 0.2, 1.0], [0.48, 0.4, 0.38]);
            zn.put(b, k.ceramic, [0.43, 0.415, 1.0], [0.46, 0.03, 0.36]);
            zn.put(b, r.wood, [w - 0.24, 0.4, 1.7], [0.44, 0.8, 0.6]);
            zn.put(b, k.ceramic, [w - 0.26, 0.84, 1.7], [0.36, 0.1, 0.44]);
            zn.put(b, k.metal, [w - 0.06, 0.98, 1.7], [0.04, 0.18, 0.03]);
            zn.put(b, r.wood, [w - 0.01, 1.5, 1.7], [0.02, 0.8, 0.6]);
            zn.put(b, k.metal, [w - 0.025, 1.5, 1.7], [0.01, 0.7, 0.5]);
            zn.put(b, k.metal, [0.04, 1.1, 1.6], [0.05, 0.03, 0.5]);
            zn.put(b, r.accent, [0.065, 0.92, 1.6], [0.03, 0.4, 0.42]);
        }
        Kind::Laundry => {
            zn.put(b, k.ceramic, [0.33, 0.43, 2.45], [0.6, 0.85, 0.6]);
            zn.put(b, k.metal, [0.33, 0.5, 2.14], [0.4, 0.4, 0.02]);
            for shelf in 0..4 {
                let y = 0.3 + shelf as f32 * 0.5;
                zn.put(b, r.wood, [0.16, y, 1.2], [0.3, 0.03, 1.1]);
                if shelf > 0 {
                    let id = pick(b, &k.books);
                    zn.put(b, id, [0.16, y + 0.13, 1.0], [0.26, 0.24, 0.32]);
                    let id = pick(b, &k.books);
                    zn.put(b, id, [0.16, y + 0.1, 1.45], [0.24, 0.18, 0.36]);
                }
            }
            for e in [-0.56, 0.56] {
                zn.put(b, r.wood, [0.16, 0.95, 1.2 + e], [0.3, 1.9, 0.03]);
            }
            zn.put(b, r.accent, [w - 0.4, 0.2, 2.4], [0.4, 0.4, 0.5]);
        }
        Kind::BackKitchen => {
            zn.put(b, r.wood, [w * 0.5, 0.43, 2.47], [w - 0.02, 0.86, 0.6]);
            zn.put(b, k.ceramic, [w * 0.5, 0.88, 2.46], [w - 0.02, 0.04, 0.64]);
            zn.put(b, k.metal, [w * 0.5, 0.905, 2.45], [0.5, 0.02, 0.4]);
            zn.put(b, k.metal, [w * 0.5, 1.05, 2.72], [0.03, 0.3, 0.03]);
            zn.put(b, r.wood, [w * 0.5, 1.85, 2.6], [w - 0.02, 0.7, 0.34]);
            zn.put(b, k.ceramic, [0.33, 0.95, 1.4], [0.62, 1.9, 0.66]);
            zn.put(b, k.metal, [0.645, 1.2, 1.2], [0.03, 0.5, 0.03]);
        }
    }
}
/// How worn the street looks. 1 is the authored default, 0 is pristine.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ageing {
    /// Missing, sunk, cracked and tilted stones and chipped kerbs.
    pub damage: f32,
    /// Spread of stone and kerb tones.
    pub colour: f32,
    /// Share of the carriageway relaid with newer setts.
    pub relaid: f32,
    /// Sidewalk stains, tar repairs, cracks and facade streaks.
    pub dirt: f32,
}
impl Default for Ageing {
    fn default() -> Self {
        Self {
            damage: 1.,
            colour: 1.,
            relaid: 1.,
            dirt: 1.,
        }
    }
}
pub fn generate(seed: u64, ageing: Ageing) -> Result<StreetScene, Box<dyn std::error::Error>> {
    let mut scene = geometry(seed, ageing)?;
    super::materials::apply(&mut scene)?;
    Ok(scene)
}
fn geometry(seed: u64, ageing: Ageing) -> Result<StreetScene, Box<dyn std::error::Error>> {
    let Ageing {
        damage,
        colour: tone_spread,
        relaid: relaid_share,
        dirt,
    } = ageing;
    let mut b = Builder {
        materials: Vec::new(),
        meshes: Vec::new(),
        triangles: Vec::new(),
        rng: seed,
        age_rng: seed ^ 0x4147_4549_4e47_5f31,
        lights: Vec::new(),
    };
    let curb_m = b.material([0.34, 0.33, 0.30], 0.76, 0., 1., 0., 0.);
    let dark_m = b.material([0.035, 0.045, 0.052], 0.35, 0.85, 5., 0., 0.);
    let wood_m = b.material([0.19, 0.075, 0.025], 0.65, 0., 4., 0., 0.);
    let roof_m = b.material([0.085, 0.12, 0.15], 0.7, 0.2, 3., 0., 0.);
    let trim_m = b.material([0.7, 0.66, 0.54], 0.68, 0., 1., 0., 0.);
    let glass_m = b.material([0.9, 0.97, 1.], 0.08, 0., 0., 0., 0.92);
    let warm_m = b.material([1., 0.57, 0.2], 0.4, 0., 0., 4200., 0.);
    let leaf_m = b.material([0.045, 0.125, 0.018], 0.72, 0., 6., 0., 0.);
    let curb = b.bevel_cube(curb_m);
    let metal = b.cube(dark_m);
    let timber = b.cube(wood_m);
    let roof = b.cube(roof_m);
    let trim = b.bevel_cube(trim_m);
    // Shared finish batches keep independent frontage choices instanced.
    let mut frame_finishes = vec![timber];
    for colour in [
        [0.035, 0.065, 0.055],
        [0.64, 0.61, 0.52],
        [0.075, 0.09, 0.11],
    ] {
        let material = b.material(colour, 0.65, 0., 4., 0., 0.);
        frame_finishes.push(b.cube(material));
    }
    let glass = b.cube(glass_m);
    let glow = b.cube(warm_m);
    let pole = b.round(dark_m, false);
    let bark_m = b.material([0.075, 0.058, 0.043], 0.93, 0., 8., 0., 0.);
    let trunk = b.tapered_branch(bark_m);
    let bulb = b.round(warm_m, true);
    let leaf = b.leaf(leaf_m);
    let palette = [
        [0.53, 0.35, 0.22],
        [0.56, 0.57, 0.48],
        [0.4, 0.46, 0.49],
        [0.67, 0.54, 0.37],
        [0.42, 0.33, 0.29],
        [0.6, 0.58, 0.51],
    ];
    let cloth_m = b.material([0.04, 0.18, 0.23], 0.9, 0., 11., 0., 0.);
    let cloth = b.cube(cloth_m);
    let plaster_m = b.material([0.82, 0.79, 0.72], 0.92, 0., 1., 0., 0.);
    let plaster = b.cube(plaster_m);
    let floor_m = b.material([0.36, 0.22, 0.12], 0.6, 0., 4., 0., 0.);
    let floor = b.cube(floor_m);
    let rug_m = b.material([0.32, 0.08, 0.06], 0.95, 0., 11., 0., 0.);
    let rug = b.cube(rug_m);
    let _ = (plaster, floor, rug);
    let paints: Vec<usize> = [
        [0.86, 0.83, 0.76],
        [0.72, 0.8, 0.78],
        [0.88, 0.76, 0.62],
        [0.62, 0.68, 0.76],
        [0.78, 0.66, 0.66],
        [0.55, 0.62, 0.5],
    ]
    .into_iter()
    .map(|c| {
        let m = b.material(c, 0.9, 0., 1., 0., 0.);
        b.cube(m)
    })
    .collect();
    let woods: Vec<usize> = [
        [0.42, 0.27, 0.14],
        [0.2, 0.1, 0.05],
        [0.58, 0.45, 0.3],
        [0.82, 0.8, 0.76],
    ]
    .into_iter()
    .map(|c| {
        let m = b.material(c, 0.55, 0., 4., 0., 0.);
        b.cube(m)
    })
    .collect();
    let fabrics: Vec<usize> = [
        [0.32, 0.08, 0.06],
        [0.12, 0.2, 0.34],
        [0.5, 0.42, 0.3],
        [0.18, 0.3, 0.2],
        [0.55, 0.36, 0.12],
        [0.25, 0.25, 0.27],
        [0.6, 0.55, 0.5],
    ]
    .into_iter()
    .map(|c| {
        let m = b.material(c, 0.95, 0., 11., 0., 0.);
        b.cube(m)
    })
    .collect();
    let books: Vec<usize> = [
        [0.5, 0.1, 0.08],
        [0.1, 0.25, 0.4],
        [0.75, 0.6, 0.2],
        [0.15, 0.35, 0.2],
        [0.85, 0.82, 0.75],
    ]
    .into_iter()
    .map(|c| {
        let m = b.material(c, 0.7, 0., 0., 0., 0.);
        b.cube(m)
    })
    .collect();
    let ceramic_m = b.material([0.9, 0.9, 0.87], 0.3, 0., 0., 0., 0.);
    let ceramic = b.cube(ceramic_m);
    let foliage = b.cube(leaf_m);
    let plinth_m = b.material([0.27, 0.26, 0.24], 0.85, 0., 1., 0., 0.);
    let plinth = b.bevel_cube(plinth_m);
    let bloom_ids: Vec<usize> = [
        [0.75, 0.08, 0.1],
        [0.95, 0.75, 0.15],
        [0.85, 0.4, 0.6],
        [0.95, 0.95, 0.9],
    ]
    .into_iter()
    .map(|c| {
        let m = b.material(c, 0.7, 0., 0., 0., 0.);
        b.cube(m)
    })
    .collect();
    let bin = b.round(dark_m, false);
    let ceiling_m = b.material([0.9, 0.89, 0.86], 0.95, 0., 1., 0., 0.);
    let tile_m = b.material([0.82, 0.86, 0.87], 0.22, 0., 0., 0., 0.);
    let tile_accent_m = b.material([0.27, 0.43, 0.52], 0.25, 0., 0., 0., 0.);
    let kit = Kit {
        trim,
        metal,
        glow,
        glass,
        ceramic,
        ceiling: b.cube(ceiling_m),
        tile: b.cube(tile_m),
        tile_accent: b.cube(tile_accent_m),
        foliage,
        paints,
        woods,
        fabrics,
        books,
    };
    // The individual paving stones and kerbs are actual geometry and collision faces.
    // A century of traffic: stones differ in tone and wear, wheel tracks are polished
    // and rounded, some stones have sunk, cracked or gone, and a few patches were
    // relaid with newer, paler and sharper setts.
    let grout_m = b.material([0.12, 0.11, 0.095], 0.95, 0., 1., 0., 0.);
    let grout = b.cube(grout_m);
    b.block(grout, [0., -0.065, 27.], [28., 0.2, 112.]);
    let stone_tones = [
        [0.32, 0.29, 0.24],
        [0.27, 0.26, 0.25],
        [0.36, 0.32, 0.26],
        [0.23, 0.21, 0.19],
        [0.34, 0.27, 0.23],
        [0.29, 0.28, 0.27],
        [0.4, 0.37, 0.32],
    ];
    // [tone][wear]: wear 0 is crisp, 2 is rounded and polished by wheels.
    let mean = [0.316, 0.286, 0.251];
    let spread = |c: [f32; 3], mean: [f32; 3]| {
        std::array::from_fn(|i| mean[i] + (c[i] - mean[i]) * tone_spread)
    };
    let old_stones: Vec<[usize; 3]> = stone_tones
        .iter()
        .map(|&c| {
            let c = spread(c, mean);
            std::array::from_fn(|wear| {
                let rough = [0.86, 0.78, 0.62][wear];
                let m = b.material(c, rough, 0., 1., 0., 0.);
                b.bevel_cube_e(m, [0.42, 0.36, 0.3][wear])
            })
        })
        .collect();
    let new_stones: Vec<usize> = [[0.46, 0.44, 0.41], [0.43, 0.42, 0.4], [0.5, 0.47, 0.43]]
        .into_iter()
        .map(|c| {
            let m = b.material(c, 0.88, 0., 1., 0., 0.);
            b.bevel_cube_e(m, 0.45)
        })
        .collect();
    let patch_hash = |a: i32, c: i32| {
        let mut h = (a as u32).wrapping_mul(0x9e37_79b1) ^ (c as u32).wrapping_mul(0x85eb_ca6b);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2c1b_3c6d);
        h ^= h >> 12;
        (h & 0xffff) as f32 / 65536.
    };
    for row in 0..360 {
        let z = -27. + row as f32 * 0.3;
        for col in 0..30 {
            let x = -4.35 + col as f32 * 0.3 + if row % 2 == 0 { 0.07 } else { 0. };
            let relaid = patch_hash(row / 9, col / 5) < 0.09 * relaid_share;
            // Wheel tracks sit about 1.4 m either side of the crown.
            let track = (1. - ((x.abs() - 1.4).abs() / 0.55)).clamp(0., 1.);
            let roll = b.aged();
            if !relaid && roll < 0.018 * damage {
                continue; // a stone that has gone, leaving a pit in the bedding
            }
            let mesh = if relaid {
                new_stones[(b.aged() * new_stones.len() as f32) as usize]
            } else {
                let tone = (b.aged() * stone_tones.len() as f32) as usize;
                let wear = if b.aged() < track * 0.9 * damage.min(1.) {
                    2
                } else if b.aged() < 0.45 * damage.min(1.) {
                    1
                } else {
                    0
                };
                old_stones[tone][wear]
            };
            let disorder = if relaid { 0.3 } else { 0.25 + 0.75 * damage };
            let sunk = if !relaid && roll < 0.08 * damage {
                0.008 + b.aged() * 0.016
            } else {
                0.
            };
            let height = 0.075 + b.aged() * 0.012 * disorder - sunk;
            let jitter_x = (b.aged() - 0.5) * 0.008 * disorder;
            let jitter_z = (b.aged() - 0.5) * 0.008 * disorder;
            let yaw = (b.aged() - 0.5) * 0.06 * disorder;
            let tilt = if sunk > 0. { 0.05 } else { 0.018 } * disorder;
            let pitch = (b.aged() - 0.5) * tilt;
            let bank = (b.aged() - 0.5) * tilt;
            let width = 0.262 + b.aged() * 0.022;
            let length = 0.258 + b.aged() * 0.024;
            let rotation = Quat::from_euler(glam::EulerRot::YXZ, yaw, pitch, bank);
            let centre = Vec3::new(x + jitter_x, height * 0.5 - 0.03, z + jitter_z);
            if !relaid && roll > 1. - 0.045 * damage {
                // Cracked through: two pieces, one settled lower than the other.
                let along_x = b.aged() < 0.5;
                let cut = 0.35 + b.aged() * 0.3;
                for (part, f) in [(0, cut), (1, 1. - cut)] {
                    let span = if along_x { width } else { length };
                    let piece = span * f - 0.006;
                    let offset = if part == 0 {
                        -span * 0.5 + piece * 0.5
                    } else {
                        span * 0.5 - piece * 0.5
                    };
                    let drop = if part == 1 {
                        0.004 + b.aged() * 0.01
                    } else {
                        0.
                    };
                    let local = if along_x {
                        Vec3::new(offset, -drop * 0.5, 0.)
                    } else {
                        Vec3::new(0., -drop * 0.5, offset)
                    };
                    let size = if along_x {
                        Vec3::new(piece, height - drop, length)
                    } else {
                        Vec3::new(width, height - drop, piece)
                    };
                    let crack_tilt = Quat::from_rotation_z((b.aged() - 0.5) * 0.04);
                    b.put(mesh, centre + rotation * local, size, rotation * crack_tilt);
                }
                continue;
            }
            b.put(mesh, centre, Vec3::new(width, height, length), rotation);
        }
    }
    let kerb_tones: Vec<[usize; 2]> = [
        [0.7, 0.66, 0.54],
        [0.58, 0.56, 0.5],
        [0.64, 0.6, 0.52],
        [0.5, 0.48, 0.44],
    ]
    .into_iter()
    .map(|c| {
        let c = spread(c, [0.6, 0.575, 0.5]);
        let m = b.material(c, 0.74, 0., 1., 0., 0.);
        [b.bevel_cube_e(m, 0.42), b.bevel_cube_e(m, 0.34)]
    })
    .collect();
    // Sidewalk ageing: tar repairs, stains and cracks lie proud of the slab by a few
    // millimetres so no face is coplanar with it.
    let tar_m = b.material([0.045, 0.045, 0.048], 0.6, 0., 1., 0., 0.);
    let tar = b.cube(tar_m);
    let stain_ms: Vec<usize> = [[0.24, 0.23, 0.21], [0.29, 0.28, 0.25], [0.38, 0.37, 0.34]]
        .into_iter()
        .map(|c| {
            let m = b.material(c, 0.85, 0., 1., 0., 0.);
            b.cube(m)
        })
        .collect();
    let crack_m = b.material([0.06, 0.055, 0.05], 0.95, 0., 0., 0., 0.);
    let crack = b.cube(crack_m);
    for side in [-1., 1.] {
        b.block(curb, [side * 6.68, 0.07, 27.], [4.36, 0.2, 112.]);
        for i in 0..180 {
            let kz = -27. + i as f32 * 0.62;
            let tone = &kerb_tones[(b.aged() * kerb_tones.len() as f32) as usize];
            let chipped = b.aged() < 0.3 * damage;
            let mesh = tone[chipped as usize];
            let drop = (if b.aged() < 0.12 * damage {
                0.01 + b.aged() * 0.025
            } else {
                b.aged() * 0.006
            }) * damage.min(1.5);
            let len = 0.575 + b.aged() * 0.02;
            let lean = Quat::from_euler(
                glam::EulerRot::YXZ,
                (b.aged() - 0.5) * 0.02 * damage,
                (b.aged() - 0.5) * 0.02 * damage,
                side * b.aged() * 0.03 * damage,
            );
            b.put(
                mesh,
                Vec3::new(side * 4.55, 0.12 - drop * 0.5, kz),
                Vec3::new(0.22, 0.25 - drop, len),
                lean,
            );
        }
        for _ in 0..(34. * dirt) as usize {
            let z = -26. + b.aged() * 108.;
            let x = side * (4.95 + b.aged() * 1.9);
            let w = (0.25 + b.aged() * 1.1).min(0.9);
            let l = 0.25 + b.aged() * 1.6;
            let mesh = if b.aged() < 0.35 {
                tar
            } else {
                stain_ms[(b.aged() * stain_ms.len() as f32) as usize]
            };
            let yaw = Quat::from_rotation_y((b.aged() - 0.5) * 0.5);
            let lift = 0.003 + b.aged() * 0.003;
            b.put(
                mesh,
                Vec3::new(x, 0.165 + lift * 0.5, z),
                Vec3::new(w, 0.01 + lift, l),
                yaw,
            );
        }
        for _ in 0..(60. * dirt) as usize {
            // Meandering cracks built from short, slightly turning segments.
            let mut p = Vec3::new(side * (4.8 + b.aged() * 2.3), 0.174, -26. + b.aged() * 108.);
            let mut heading = b.aged() * std::f32::consts::TAU;
            for _ in 0..3 + (b.aged() * 6.) as usize {
                heading += (b.aged() - 0.5) * 1.1;
                let seg = 0.08 + b.aged() * 0.22;
                let dir = Vec3::new(heading.cos(), 0., heading.sin());
                let next = p + dir * seg;
                if next.x.abs() < 4.72 || next.x.abs() > 7.2 {
                    break;
                }
                let crack_w = 0.006 + b.aged() * 0.006;
                b.put(
                    crack,
                    (p + next) * 0.5,
                    Vec3::new(crack_w, 0.006, seg + 0.006),
                    Quat::from_rotation_y(dir.x.atan2(dir.z)),
                );
                p = next;
            }
        }
        let mut frontage = -25.;
        for house in 0..12 {
            let width = 6.3 + b.random() * 3.6;
            let z = frontage + width * 0.5;
            frontage += width;
            let setback = 1.6 + b.random() * 0.75;
            let starts: Vec<_> = b.meshes.iter().map(|m| m.instances.len()).collect();
            let triangle_start = b.triangles.len();
            let floors = 3 + (b.random() * 3.) as usize;
            let height = floors as f32 * 2.65 + 0.65 + b.random() * 0.65;
            let windows = 2 + (b.random() * 3.) as usize;
            let spacing = 7.5 / windows as f32;
            let opening = spacing * (0.48 + b.random() * 0.13);
            // A house has a consistent joinery design, independently chosen on
            // each side of the street instead of repeating a house-index motif.
            let geometry_rng = b.rng;
            b.rng ^= 0x4652_414d_4553_5459;
            let frame = frame_finishes[(b.random() * frame_finishes.len() as f32) as usize];
            let transom_drop = 0.30 + b.random() * 0.48;
            let mullion_offset = (b.random() - 0.5) * opening * 0.22;
            b.rng = geometry_rng;
            let door_bay = windows / 2;
            let door_z = z - 3.75 + spacing * (door_bay as f32 + 0.5);
            let facade_pattern = if house % 4 == 1 { 7. } else { 2. };
            let colour = palette[(b.random() * palette.len() as f32) as usize];
            let tint = 0.78 + b.random() * 0.34;
            let house_material =
                b.material(colour.map(|v| v * tint), 0.88, 0., facade_pattern, 0., 0.);
            let wall = b.cube(house_material);
            let grime_material = b.material(
                colour.map(|v| v * tint * 0.58),
                0.93,
                0.,
                facade_pattern,
                0.,
                0.,
            );
            let grime = b.cube(grime_material);
            b.block(wall, [side * 14.45, height * 0.5, z], [0.2, height, 7.85]);
            for dz in [-3.925, 3.925] {
                b.block(wall, [side * 11., height * 0.5, z + dz], [7., height, 0.2]);
            }
            // Openings are cut cleanly from the facade. Piers, sill spandrels and
            // head spandrels tile the wall without overlapping, so no two exterior
            // faces are coplanar (overlaps z-fight).
            let top = floors as f32 * 2.65;
            let door_w = opening.min(1.15);
            for pier in 0..=windows {
                let half = (spacing - opening) * 0.5;
                let lo = if pier == 0 {
                    -3.925
                } else {
                    -3.75 + pier as f32 * spacing - half
                };
                let hi = if pier == windows {
                    3.925
                } else {
                    -3.75 + pier as f32 * spacing + half
                };
                b.block(
                    wall,
                    [side * 7.42, top * 0.5, z + (lo + hi) * 0.5],
                    [0.28, top, hi - lo],
                );
            }
            for level in 0..floors {
                let f = level as f32 * 2.65;
                for window in 0..windows {
                    let wz = z - 3.75 + spacing * (window as f32 + 0.5);
                    if level == 0 && house % 3 != 0 && window == door_bay {
                        // Narrow the bay down to a real door width.
                        let fill = (opening - door_w) * 0.5;
                        for dz in [-1., 1.] {
                            b.block(
                                wall,
                                [side * 7.42, 1.15, wz + dz * (door_w + fill) * 0.5],
                                [0.28, 2.3, fill],
                            );
                        }
                    } else {
                        b.block(wall, [side * 7.42, f + 0.45, wz], [0.28, 0.9, opening]);
                    }
                    b.block(wall, [side * 7.42, f + 2.475, wz], [0.28, 0.35, opening]);
                    // Rain washes dirt off the sill into darker streaks below it.
                    if level > 0 || house % 3 == 0 || window != door_bay {
                        for _ in 0..((0.5 + b.aged() * 3.) * dirt) as usize {
                            let run = 0.25 + b.aged() * 0.65;
                            let sz = wz + (b.aged() - 0.5) * opening * 0.9;
                            let sw = 0.03 + b.aged() * 0.09;
                            b.block(
                                grime,
                                [side * 7.277, f + 0.9 - run * 0.5, sz],
                                [0.006, run, sw],
                            );
                        }
                    }
                }
            }
            if height > top {
                b.block(
                    wall,
                    [side * 7.42, (height + top) * 0.5, z],
                    [0.28, height - top, 7.85],
                );
            }
            // Separate masonry courses and rainwater hardware give the close street
            // section real silhouette and contact shadows, with full collision geometry.
            // Every house drains its gutter through a downpipe beside the corner
            // pilaster, clear of balconies (which reach at most z + 3.63).
            let pipe_z = z + 3.68;
            b.block(
                metal,
                [side * 7.04, (height + 0.24) * 0.5, pipe_z],
                [0.085, height + 0.24, 0.085],
            );
            b.block(metal, [side * 6.97, 0.24, pipe_z], [0.22, 0.08, 0.085]);
            for bracket in 0..=(height / 2.4) as usize {
                b.block(
                    metal,
                    [side * 7.16, 0.6 + bracket as f32 * 2.4, pipe_z],
                    [0.24, 0.035, 0.13],
                );
            }
            b.block(metal, [side * 7.08, height + 0.24, z], [0.14, 0.12, 8.0]);
            // Paving from the sidewalk edge (world x 8.86) up to this facade, flush
            // with the sidewalk top and abutting it rather than overlapping.
            let apron_start = 8.86 - setback;
            if apron_start < 7.28 {
                b.block(
                    curb,
                    [side * (apron_start + 7.28) * 0.5, 0.07, z],
                    [7.28 - apron_start, 0.2, 7.85],
                );
            }
            // Stone plinth course, interrupted at the entrance.
            let gap = if house % 3 != 0 {
                door_w * 0.5 + 0.21
            } else {
                0.
            };
            let spans: Vec<(f32, f32)> = if gap > 0. {
                vec![(z - 3.775, door_z - gap), (door_z + gap, z + 3.775)]
            } else {
                vec![(z - 3.775, z + 3.775)]
            };
            for (lo, hi) in spans {
                b.block(
                    plinth,
                    [side * 7.2, 0.3, (lo + hi) * 0.5],
                    [0.16, 0.6, hi - lo],
                );
            }
            // Entrance hardware: house number, letter box, bell and a wall lantern.
            if house % 3 != 0 {
                let edge = door_w * 0.5 + 0.35;
                b.block(trim, [side * 7.265, 2.0, door_z + edge], [0.03, 0.2, 0.26]);
                b.block(metal, [side * 7.245, 2.0, door_z + edge], [0.02, 0.1, 0.14]);
                b.block(metal, [side * 7.2, 1.15, door_z + edge], [0.16, 0.32, 0.26]);
                b.block(
                    trim,
                    [side * 7.115, 1.22, door_z + edge],
                    [0.01, 0.02, 0.18],
                );
                b.block(
                    metal,
                    [side * 7.18, 1.3, door_z + door_w * 0.5 + 0.1],
                    [0.02, 0.08, 0.05],
                );
                b.block(metal, [side * 7.16, 2.2, door_z - edge], [0.24, 0.03, 0.03]);
                b.block(
                    metal,
                    [side * 7.04, 2.24, door_z - edge],
                    [0.16, 0.03, 0.16],
                );
                b.block(glow, [side * 7.04, 2.11, door_z - edge], [0.12, 0.22, 0.12]);
            }
            // Cornices, deep reveals and casement frames define facade depth.
            for level in 0..floors {
                let f = level as f32 * 2.65;
                let (y0, y1) = (f + 0.9, f + 2.3);
                let (y, h) = ((y0 + y1) * 0.5, y1 - y0);
                if level > 0 {
                    b.block(trim, [side * 7.24, f + 0.02, z], [0.12, 0.17, 7.9]);
                }
                for window in 0..windows {
                    let wz = z - 3.75 + spacing * (window as f32 + 0.5);
                    if level == 0 && house % 3 != 0 && window == door_bay {
                        // Recessed panelled door with a glazed upper half, frame,
                        // stone surround and a step.
                        let leaf = door_w - 0.16;
                        let dx = side * 7.47;
                        for dz in [-1., 1.] {
                            b.block(
                                timber,
                                [dx, 1.12, wz + dz * (door_w - 0.08) * 0.5],
                                [0.1, 2.24, 0.078],
                            );
                            b.block(
                                trim,
                                [side * 7.24, 1.17, wz + dz * (door_w * 0.5 + 0.08)],
                                [0.1, 2.34, 0.16],
                            );
                        }
                        b.block(timber, [dx, 2.26, wz], [0.1, 0.078, door_w - 0.004]);
                        b.block(trim, [side * 7.24, 2.42, wz], [0.12, 0.2, door_w + 0.36]);
                        b.block(timber, [dx, 0.67, wz], [0.05, 1.22, leaf]);
                        b.block(timber, [dx, 2.17, wz], [0.05, 0.1, leaf]);
                        for pz in [-1., 1.] {
                            b.block(
                                timber,
                                [dx, 1.7, wz + pz * (leaf * 0.5 - 0.04)],
                                [0.05, 0.84, 0.08],
                            );
                            b.block(
                                timber,
                                [side * 7.435, 0.62, wz + pz * leaf * 0.24],
                                [0.02, 0.9, leaf * 0.36],
                            );
                        }
                        b.block(timber, [dx, 1.7, wz], [0.05, 0.84, 0.04]);
                        b.block(glass, [dx, 1.7, wz], [0.015, 0.84, leaf - 0.16]);
                        b.block(
                            metal,
                            [side * 7.41, 1.05, wz + leaf * 0.38],
                            [0.06, 0.03, 0.12],
                        );
                        b.block(trim, [side * 7.24, 0.07, wz], [0.42, 0.14, door_w + 0.4]);
                        continue;
                    }
                    // Casement frame sits 0.19 m back from the facade face.
                    let fx = side * 7.47;
                    for dz in [-1., 1.] {
                        b.block(
                            frame,
                            [fx, y, wz + dz * (opening * 0.5 - 0.037)],
                            [0.09, h - 0.004, 0.07],
                        );
                    }
                    for dy in [-1., 1.] {
                        b.block(
                            frame,
                            [fx, y + dy * (h * 0.5 - 0.037), wz],
                            [0.086, 0.07, opening - 0.004],
                        );
                    }
                    b.block(frame, [fx, y, wz + mullion_offset], [0.07, h - 0.1, 0.05]);
                    b.block(
                        frame,
                        [fx, y1 - transom_drop, wz],
                        [0.07, 0.05, opening - 0.1],
                    );
                    b.block(glass, [fx, y, wz], [0.015, h - 0.1, opening - 0.1]);
                    // Projecting stone sill and lintel.
                    b.block(
                        trim,
                        [side * 7.27, y0 - 0.028, wz],
                        [0.31, 0.08, opening + 0.16],
                    );
                    b.block(
                        trim,
                        [side * 7.26, y1 + 0.08, wz],
                        [0.08, 0.18, opening + 0.2],
                    );
                    let balcony = level > 0 && house % 3 != 1 && (window + house) % 2 == 0;
                    if !balcony && (house + window + level) % 3 == 0 {
                        // Flower box hung on the wall just below the stone sill.
                        let len = opening - 0.1;
                        b.block(timber, [side * 7.18, y0 - 0.17, wz], [0.2, 0.18, len]);
                        for e in [-0.35, 0.35] {
                            b.block(
                                metal,
                                [side * 7.2, y0 - 0.275, wz + e * len],
                                [0.16, 0.03, 0.03],
                            );
                        }
                        let plants = (len / 0.11) as usize;
                        for n in 0..plants {
                            let pz = wz - len * 0.5 + (n as f32 + 0.5) * len / plants as f32;
                            let t = n as f32 * 2.4;
                            b.put(
                                foliage,
                                Vec3::new(
                                    side * (7.15 + (n % 2) as f32 * 0.05),
                                    y0 + 0.02 + (n % 3) as f32 * 0.03,
                                    pz,
                                ),
                                Vec3::new(0.05, 0.2, 0.11),
                                Quat::from_rotation_y(t) * Quat::from_rotation_x(0.3),
                            );
                            if n % 2 == 0 {
                                let bloom =
                                    bloom_ids[(house + window + level + n) % bloom_ids.len()];
                                b.block(
                                    bloom,
                                    [side * 7.12, y0 + 0.1 + (n % 3) as f32 * 0.02, pz],
                                    [0.06, 0.05, 0.06],
                                );
                            }
                        }
                    }
                    if house < 3 && level > 0 && (house + window) % 2 == 0 {
                        for shutter_side in [-1.0, 1.0] {
                            let sz = wz + shutter_side * (opening * 0.5 + 0.3);
                            for slat in 0..12 {
                                b.put(
                                    timber,
                                    Vec3::new(side * 7.2, y0 + 0.1 + slat as f32 * 0.11, sz),
                                    Vec3::new(0.05, 0.09, 0.44),
                                    Quat::from_rotation_z(side * 0.28),
                                );
                            }
                            for edge in [-0.25, 0.25] {
                                b.block(timber, [side * 7.2, y, sz + edge], [0.07, h, 0.06]);
                            }
                        }
                    }
                    if level > 0 && house % 3 != 1 && (window + house) % 2 == 0 {
                        b.block(
                            trim,
                            [side * 6.98, y0 - 0.06, wz],
                            [0.6, 0.12, opening + 0.5],
                        );
                        let rail = opening + 0.4;
                        let posts = (rail / 0.2) as usize;
                        for n in 0..=posts {
                            b.block(
                                metal,
                                [
                                    side * 6.72,
                                    y0 + 0.44,
                                    wz - rail * 0.5 + n as f32 * rail / posts as f32,
                                ],
                                [0.03, 0.88, 0.03],
                            );
                        }
                        for dy in [0.06, 0.88] {
                            b.block(metal, [side * 6.72, y0 + dy, wz], [0.045, 0.045, rail]);
                        }
                        // Potted plants at both ends of the balcony.
                        for e in [-1., 1.] {
                            let pz = wz + e * (rail * 0.5 - 0.22);
                            b.block(timber, [side * 6.88, y0 + 0.15, pz], [0.22, 0.3, 0.22]);
                            for n in 0..6 {
                                let t = n as f32 * 1.05;
                                b.put(
                                    foliage,
                                    Vec3::new(
                                        side * (6.88 + t.cos() * 0.06),
                                        y0 + 0.42 + (n % 2) as f32 * 0.08,
                                        pz + t.sin() * 0.06,
                                    ),
                                    Vec3::new(0.07, 0.32, 0.16),
                                    Quat::from_rotation_y(t) * Quat::from_rotation_x(0.35),
                                );
                            }
                            let bloom = bloom_ids[(house + window + level) % bloom_ids.len()];
                            b.block(bloom, [side * 6.85, y0 + 0.62, pz], [0.07, 0.06, 0.07]);
                        }
                    }
                }
            }
            interior(
                &mut b,
                &kit,
                &Plan {
                    side,
                    z,
                    floors,
                    windows,
                    spacing,
                    opening,
                    door: (house % 3 != 0).then_some((door_z - z, door_w)),
                    door_bay,
                },
            );
            for wz in [-3.9, 3.9] {
                b.block(
                    trim,
                    [side * 7.32, height * 0.5, z + wz],
                    [0.38, height, 0.25],
                );
            }
            b.block(trim, [side * 7.35, height, z], [0.65, 0.35, 8.1]);
            for slope in [-1., 1.] {
                b.put(
                    roof,
                    Vec3::new(side * 11. + slope * 1.7, height + 0.65, z),
                    Vec3::new(3.7, 0.17, 8.2),
                    Quat::from_rotation_z(-slope * 0.35),
                );
            }
            b.block(roof, [side * 11., height + 1.2, z], [0.2, 0.2, 8.2]);
            b.block(wall, [side * 12., height + 1.6, z + 2.3], [0.8, 1.8, 0.9]);
            b.block(
                trim,
                [side * 12., height + 2.55, z + 2.3],
                [0.95, 0.1, 1.05],
            );
            for dz in [-0.2, 0.2] {
                b.block(
                    metal,
                    [side * 12., height + 2.72, z + 2.3 + dz],
                    [0.14, 0.24, 0.14],
                );
            }
            if house % 3 == 0 {
                b.put(
                    cloth,
                    Vec3::new(side * 6.7, 2.65, z),
                    Vec3::new(1.4, 0.10, 5.8),
                    Quat::from_rotation_z(side * 0.16),
                );
                b.block(cloth, [side * 6.02, 2.49, z], [0.1, 0.32, 5.8]);
            }
            // Benches and outdoor tables are built from separate timber and steel parts.
            if house % 2 == 0 {
                // Place the bench in the half of the frontage away from the door.
                let bc = if door_z > z { z - 2.2 } else { z + 2.2 };
                for n in 0..4 {
                    b.block(
                        timber,
                        [side * (5.58 + n as f32 * 0.14), 0.60, bc],
                        [0.115, 0.055, 1.65],
                    );
                }
                for dz in [-0.62, 0.62] {
                    for x in [5.60, 5.98] {
                        b.block(metal, [side * x, 0.37, bc + dz], [0.055, 0.42, 0.055]);
                    }
                    b.block(metal, [side * 5.79, 0.55, bc + dz], [0.52, 0.06, 0.06]);
                    b.block(metal, [side * 6.01, 0.84, bc + dz], [0.055, 0.92, 0.055]);
                }
                for n in 0..3 {
                    b.block(
                        timber,
                        [side * 6.00, 0.82 + n as f32 * 0.16, bc],
                        [0.055, 0.125, 1.65],
                    );
                }
            }
            let transform = Mat4::from_translation(Vec3::new(side * setback, 0., z))
                * Mat4::from_scale(Vec3::new(1., 1., width / 8.))
                * Mat4::from_translation(Vec3::new(0., 0., -z));
            b.transform_new_instances(&starts, triangle_start, transform);
        }
        for i in 0..8 {
            let z = -21. + i as f32 * 13.;
            let x = side * 4.95;
            b.block(metal, [x, 0.22, z], [0.4, 0.45, 0.4]);
            b.block(metal, [x, 0.55, z], [0.23, 0.35, 0.23]);
            b.block(metal, [x, 3., z], [0.12, 4.8, 0.12]);
            b.beam(
                pole,
                Vec3::new(x, 5.35, z),
                Vec3::new(x - side * 0.65, 5.6, z),
                0.09,
            );
            b.block(metal, [x - side * 0.65, 5.55, z], [0.42, 0.13, 0.42]);
            b.put(
                bulb,
                Vec3::new(x - side * 0.65, 5.32, z),
                Vec3::new(0.22, 0.35, 0.22),
                Quat::IDENTITY,
            );
            b.lights.push(ArchiveLight {
                position_kind: [x - side * 0.65, 5.25, z, 20.],
                colour_intensity: [1., 0.63, 0.33, 95.],
            });
            // Litter bin beside the lamp base and a gully grate at the kerb
            // halfway to the next lamp.
            b.put(
                bin,
                Vec3::new(side * 5.05, 0.57, z + 1.0),
                Vec3::new(0.42, 0.8, 0.42),
                Quat::IDENTITY,
            );
            b.block(metal, [side * 5.05, 0.99, z + 1.0], [0.46, 0.04, 0.46]);
            b.block(trim, [side * 5.05, 0.6, z + 1.0], [0.44, 0.06, 0.44]);
            b.block(metal, [side * 4.3, 0.054, z + 6.5], [0.26, 0.012, 0.62]);
            for n in 0..5 {
                b.block(
                    trim,
                    [side * 4.3, 0.062, z + 6.26 + n as f32 * 0.12],
                    [0.22, 0.006, 0.03],
                );
            }
            if side > 0. {
                // Manhole cover on the carriageway centre line with a raised ring.
                let mz = z + 9.;
                b.block(metal, [0., 0.054, mz], [0.68, 0.012, 0.68]);
                for (dx, dz, sx, sz) in [
                    (-0.33, 0., 0.04, 0.7),
                    (0.33, 0., 0.04, 0.7),
                    (0., -0.33, 0.62, 0.04),
                    (0., 0.33, 0.62, 0.04),
                ] {
                    b.block(trim, [dx, 0.062, mz + dz], [sx, 0.008, sz]);
                }
            }
            let tx = side * (5.65 + b.random() * 0.45);
            let tz = z + 4. + b.random() * 2.;
            let tree_height = 4.5 + b.random() * 2.3;
            let crown_radius = 1.05 + b.random() * 0.65;
            let geometry_rng = b.rng;
            b.rng ^= 0x4352_4f57_4e53_5459;
            let crown_orientation = b.random() * std::f32::consts::TAU;
            let crown_aspect = 0.65 + b.random() * 0.7;
            let crown_lean = Vec3::new((b.random() - 0.5) * 0.65, 0., (b.random() - 0.5) * 0.65);
            b.rng = geometry_rng;
            let crown_rotation = Quat::from_rotation_y(crown_orientation);
            let crown_direction = |angle: f32| {
                crown_rotation
                    * Vec3::new(angle.cos() * crown_aspect, 0., angle.sin() / crown_aspect)
            };
            b.block(timber, [tx, 0.2, tz], [0.9, 0.4, 0.9]);
            let root = Vec3::new(tx, 0.2, tz);
            let fork = Vec3::new(
                tx + b.random() * 0.20,
                tree_height * 0.48,
                tz + b.random() * 0.15,
            );
            b.beam(trunk, root, fork, 0.24);
            // Hierarchical, tapered branches: each primary has secondary limbs,
            // and leaves occupy their length as well as the tips.
            for branch in 0..9 {
                let a = branch as f32 * 2.399963 + b.random() * 0.8;
                let elevation = b.random();
                let radial = crown_radius * (0.65 + b.random() * 0.35);
                let direction = crown_direction(a);
                let origin = fork + Vec3::Y * (elevation - 0.3) * 0.8;
                let elbow = origin
                    + Vec3::new(
                        direction.x * radial * 0.50,
                        0.45 + elevation * 0.6,
                        direction.z * radial * 0.50,
                    );
                let tip = Vec3::new(
                    tx + direction.x * radial + crown_lean.x,
                    tree_height * (0.72 + 0.25 * elevation),
                    tz + direction.z * radial + crown_lean.z,
                );
                b.beam(trunk, origin, elbow, 0.085);
                b.beam(trunk, elbow, tip, 0.050);
                for twig in 0..8 {
                    let theta = a + twig as f32 * 2.399963;
                    let t = 0.20 + twig as f32 * 0.10;
                    let twig_direction = crown_direction(theta);
                    let start = elbow.lerp(tip, t);
                    let end = start
                        + Vec3::new(
                            twig_direction.x * 0.55,
                            0.12 + b.random() * 0.38,
                            twig_direction.z * 0.55,
                        );
                    b.beam(trunk, start, end, 0.018);
                    for _ in 0..45 {
                        let along = b.random();
                        let d =
                            Vec3::new(b.random() - 0.5, (b.random() - 0.5) * 0.7, b.random() - 0.5)
                                * 0.78;
                        let size = 0.050 + b.random() * 0.035;
                        let tilt = Quat::from_euler(
                            glam::EulerRot::YXZ,
                            #[allow(clippy::approx_constant)]
                            {
                                b.random() * 6.283185
                            },
                            b.random() * 2.4 - 1.2,
                            b.random() * 2.4 - 1.2,
                        );
                        b.put(
                            leaf,
                            start.lerp(end, along) + d,
                            Vec3::new(size * 1.2, size, size * 2.0),
                            tilt,
                        );
                    }
                }
            }
        }
    }
    // Original end-of-street landmark and fountain.
    b.block(trim, [0., 0.2, 75.], [6., 0.45, 6.]);
    b.put(
        trunk,
        Vec3::new(0., 1.1, 75.),
        Vec3::new(0.7, 1.7, 0.7),
        Quat::IDENTITY,
    );
    b.put(
        bulb,
        Vec3::new(0., 2.1, 75.),
        Vec3::splat(0.6),
        Quat::IDENTITY,
    );
    // The colonnade opens in the middle onto the lane towards the villa.
    for side in [-1., 1.] {
        b.block(trim, [side * 9.02, 2.5, 88.], [13.96, 5., 0.6]);
    }
    b.block(trim, [0., 4.6, 88.], [4.08, 0.8, 0.6]);
    b.block(trim, [0., 4.17, 87.5], [4.2, 0.12, 0.8]);
    for pillar in -7..=7 {
        if pillar != 0 {
            b.block(trim, [pillar as f32 * 2.2, 2.7, 87.5], [0.32, 5.4, 0.8]);
        }
    }
    let villa = villa::build(
        &mut b,
        &villa::Parts {
            curb,
            trim,
            metal,
            timber,
            glass,
            glow,
            trunk,
            leaf,
            foliage,
            bulb,
            floor,
            ceiling: kit.ceiling,
            fabrics: kit.fabrics.clone(),
            ceramic: kit.ceramic,
            wood: kit.woods[3],
        },
    );
    let count = b.triangles.len();
    let collision = StaticTriangleWorld::new(b.triangles)?;
    eprintln!(
        "ORIGINAL STREET seed={seed} source_geometry_assets=0 materials={} mesh_batches={} triangles={count} instances={}",
        b.materials.len(),
        b.meshes.len(),
        b.meshes.iter().map(|m| m.instances.len()).sum::<usize>()
    );
    Ok(StreetScene {
        archive: None,
        images: Vec::new(),
        materials: b.materials,
        geometry: b.meshes,
        lights: b.lights,
        sun: Vec3::new(-0.60, 0.58, -0.55).normalize(),
        sun_colour: [1., 0.93, 0.83, 4800.],
        bounds: (Vec3::new(-18., -2.3, -29.), Vec3::new(18., 18., 154.)),
        collision,
        eye: Vec3::new(0.6, 1.78, -18.),
        direction: Vec3::new(-0.10, 0.025, 1.).normalize(),
        water: villa.water,
        pool_view: Some(villa.view),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Writes axis-aligned boxes of one generated house to `VOXY_PLAN_DUMP`
    /// for plotting floor plans; run with `--ignored`.
    #[test]
    #[ignore = "manual floor-plan export via VOXY_PLAN_DUMP"]
    fn dump_house_plan() {
        let Ok(path) = std::env::var("VOXY_PLAN_DUMP") else {
            return;
        };
        let mut out = String::new();
        for (case, (windows, spacing, opening, door)) in [
            (2usize, 3.75f32, 2.0f32, Some((1.875f32, 1.15f32))),
            (3, 2.5, 1.4, Some((0., 1.15))),
        ]
        .into_iter()
        .enumerate()
        {
            let mut b = Builder {
                materials: Vec::new(),
                meshes: Vec::new(),
                triangles: Vec::new(),
                rng: 11 + case as u64,
                age_rng: (11 + case as u64) ^ 0x4147_4549_4e47_5f31,
                lights: Vec::new(),
            };
            let mut kit_ids = Vec::new();
            for n in 0..13 {
                let m = b.material([0.5, 0.5, 0.5], 0.5, 0., 0., 0., 0.);
                kit_ids.push((b.cube(m), n));
            }
            let id = |n: usize| kit_ids[n].0;
            let kit = Kit {
                trim: id(0),
                metal: id(1),
                glow: id(2),
                glass: id(3),
                ceramic: id(4),
                ceiling: id(5),
                tile: id(6),
                tile_accent: id(7),
                foliage: id(8),
                paints: vec![id(9)],
                woods: vec![id(10)],
                fabrics: vec![id(11)],
                books: vec![id(12)],
            };
            let plan = Plan {
                side: 1.,
                z: 0.,
                floors: 3,
                windows,
                spacing,
                opening,
                door,
                door_bay: windows / 2,
            };
            interior(&mut b, &kit, &plan);
            for (mesh_id, mesh) in b.meshes.iter().enumerate() {
                for inst in &mesh.instances {
                    let w = inst.world;
                    let ext = |c: usize| (w[0][c].abs() + w[1][c].abs() + w[2][c].abs()) * 0.5;
                    out += &format!(
                        "{case},{mesh_id},{},{},{},{},{},{}\n",
                        w[3][0],
                        w[3][1],
                        w[3][2],
                        ext(0),
                        ext(1),
                        ext(2)
                    );
                }
            }
        }
        std::fs::write(path, out).unwrap();
    }
    #[test]
    fn house_stairs_climb_to_the_landing() {
        let mut b = Builder {
            materials: Vec::new(),
            meshes: Vec::new(),
            triangles: Vec::new(),
            rng: 7,
            age_rng: 7 ^ 0x4147_4549_4e47_5f31,
            lights: Vec::new(),
        };
        let m = b.material([0.5, 0.5, 0.5], 0.5, 0., 0., 0., 0.);
        let mut ids = || b.cube(m);
        let kit = Kit {
            trim: ids(),
            metal: ids(),
            glow: ids(),
            glass: ids(),
            ceramic: ids(),
            ceiling: ids(),
            tile: ids(),
            tile_accent: ids(),
            foliage: ids(),
            paints: vec![ids()],
            woods: vec![ids()],
            fabrics: vec![ids()],
            books: vec![ids()],
        };
        let plan = Plan {
            side: 1.,
            z: 0.,
            floors: 3,
            windows: 2,
            spacing: 3.75,
            opening: 2.,
            door: None,
            door_bay: 0,
        };
        interior(&mut b, &kit, &plan);
        let world = StaticTriangleWorld::new(b.triangles).unwrap();
        // Start in the left front room in line with flight A and walk inward.
        let (mut feet, grounded) = world
            .move_capsule([7.56 + 3.2, 0.5, -0.5], 1.8, 0.3, [0., -1., 0.])
            .unwrap();
        assert!(grounded);
        for _ in 0..400 {
            if feet[0] > 7.56 + 5.5 {
                break;
            }
            feet = world
                .move_capsule(feet, 1.8, 0.3, [0.02, -0.004, 0.])
                .unwrap()
                .0;
        }
        assert!(feet[1] > 1.1 && feet[0] > 7.56 + 5.4, "stuck at {feet:?}");
    }
    #[test]
    fn original_street_is_independent_and_walkable() {
        let scene = geometry(0x564f5859, Ageing::default()).unwrap();
        assert!(scene.archive.is_none() && scene.images.is_empty());
        assert!(scene.materials.len() >= 34 && scene.lights.len() == 26);
        let rendered_triangles: usize = scene
            .geometry
            .iter()
            .map(|m| m.indices.len() / 3 * m.instances.len())
            .sum();
        assert_eq!(
            rendered_triangles,
            scene.collision.triangle_count(),
            "all detailed geometry must remain in collision"
        );
        let wall_batches: Vec<_> = scene
            .geometry
            .iter()
            .filter(|m| {
                let kind = scene.materials[m.material].parameters.workflow[2];
                kind == 2. || kind == 7.
            })
            .collect();
        // Each frontage owns a facade and a matching grime batch.
        assert_eq!(
            wall_batches.len(),
            48,
            "every frontage owns a distinct facade"
        );
        let mut widths = std::collections::HashSet::new();
        for mesh in wall_batches {
            for instance in &mesh.instances {
                widths.insert(instance.world[2][2].to_bits());
            }
        }
        assert!(
            widths.len() >= 12,
            "frontage widths must vary geometrically"
        );
        for mesh in &scene.geometry {
            assert!(mesh.material < scene.materials.len());
            assert!(
                mesh.indices
                    .iter()
                    .all(|i| (*i as usize) < mesh.vertices.len())
            );
            assert!(mesh.vertices.iter().all(|v| {
                v.position
                    .iter()
                    .chain(&v.normal)
                    .chain(&v.tangent)
                    .all(|v| v.is_finite())
            }));
        }
        let (mut feet, grounded) = scene
            .collision
            .move_capsule([0.6, 0.5, -18.], 1.8, 0.3, [0., -2., 0.])
            .unwrap();
        assert!(grounded);
        for _ in 0..120 {
            feet = scene
                .collision
                .move_capsule(feet, 1.8, 0.3, [0., -0.001, 0.025])
                .unwrap()
                .0;
        }
        assert!(feet[2] > -15.1 && feet[1] > -0.1, "{feet:?}");
    }
    /// Renders the untextured original street with its water to `VOXY_VILLA_PREVIEW`
    /// (PNG) from `VOXY_VILLA_VIEW=x,y,z,dx,dy,dz` or the pool view; run with `--ignored`.
    #[test]
    #[ignore = "manual villa preview render via VOXY_VILLA_PREVIEW"]
    fn render_villa_preview() {
        let Ok(path) = std::env::var("VOXY_VILLA_PREVIEW") else {
            return;
        };
        let scene = if std::env::var("VOXY_VILLA_SCANS").as_deref() == Ok("1") {
            generate(0x564f5859, Ageing::default()).unwrap()
        } else {
            geometry(0x564f5859, Ageing::default()).unwrap()
        };
        let (mut eye, mut direction) = scene.pool_view.unwrap();
        if let Ok(view) = std::env::var("VOXY_VILLA_VIEW") {
            let v: Vec<f32> = view.split(',').map(|x| x.parse().unwrap()).collect();
            eye = Vec3::new(v[0], v[1], v[2]);
            direction = Vec3::new(v[3], v[4], v[5]).normalize();
        }
        let size: Vec<u32> = std::env::var("VOXY_VILLA_SIZE")
            .unwrap_or_else(|_| "640x360".into())
            .split('x')
            .map(|x| x.parse().unwrap())
            .collect();
        let (width, height) = (size[0], size[1]);
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features: adapter.features() & wgpu::Features::FLOAT32_FILTERABLE,
            required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .unwrap();
        let format = if std::env::var("VOXY_VILLA_ACES").as_deref() == Ok("1") {
            wgpu::TextureFormat::Rgba16Float
        } else {
            wgpu::TextureFormat::Rgba8UnormSrgb
        };
        let environment = if std::env::var("VOXY_VILLA_HDRI").as_deref() == Ok("1") {
            Some(crate::materials::daylight().unwrap())
        } else {
            None
        };
        let (sun, sun_colour, env) = match environment {
            Some((env, sun, sun_colour)) => (sun, sun_colour, Some(env)),
            None => (scene.sun, scene.sun_colour, None),
        };
        let mut renderer = ArchiveSceneRenderer::new_with_environment(
            &device,
            &queue,
            format,
            width,
            height,
            scene.images,
            scene.materials,
            scene.geometry,
            &scene.lights,
            sun,
            sun_colour,
            scene.bounds,
            env,
        )
        .unwrap();
        let aa = if std::env::var("VOXY_VILLA_TAA").as_deref() == Ok("1") {
            voxy_render::ArchiveAntialiasing::Temporal
        } else {
            voxy_render::ArchiveAntialiasing::Off
        };
        renderer.set_antialiasing(aa).unwrap();
        let mut water = scene.water.clone();
        if let Ok(caustics) = std::env::var("VOXY_VILLA_CAUSTICS") {
            water[0].material.caustics = caustics.parse().unwrap();
        }
        renderer.set_water(&voxy_render::voxy_water::WaterWorld::new(
            water,
            voxy_render::WaveSpectrum::pool(),
        ));
        if let Ok(splash) = std::env::var("VOXY_VILLA_SPLASH") {
            let v: Vec<f32> = splash.split(',').map(|x| x.parse().unwrap()).collect();
            renderer.advance_water(1. / 30.);
            renderer.water_splash(glam::Vec2::new(v[0], v[1]), v[2], v[3]);
        }
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("villa preview"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        for frame in 0..std::env::var("VOXY_VILLA_FRAMES").map_or(3, |f| f.parse().unwrap()) {
            renderer.set_weather_time(frame as f32 / 30.);
            renderer.advance_water(1. / 30.);
            renderer.draw(&view, eye, direction);
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        }
        let bpp = if format == wgpu::TextureFormat::Rgba16Float {
            8
        } else {
            4
        };
        let row = (width * bpp).div_ceil(256) * 256;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let mapped = readback.get_mapped_range(..).unwrap();
        let pixels: Vec<u8> = if format == wgpu::TextureFormat::Rgba16Float {
            let mut rgba = Vec::with_capacity((width * height * 4) as usize);
            for r in mapped.chunks_exact(row as usize) {
                for p in r[..(width * 8) as usize].chunks_exact(8) {
                    for c in 0..3 {
                        let x = crate::decode_half(u16::from_le_bytes([p[c * 2], p[c * 2 + 1]]))
                            .max(0.);
                        let x = ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14))
                            .clamp(0., 1.);
                        let enc = if x <= 0.0031308 {
                            12.92 * x
                        } else {
                            1.055 * x.powf(1. / 2.4) - 0.055
                        };
                        rgba.push((enc * 255.).round().clamp(0., 255.) as u8);
                    }
                    rgba.push(255);
                }
            }
            rgba
        } else {
            mapped
                .chunks_exact(row as usize)
                .flat_map(|r| &r[..(width * 4) as usize])
                .copied()
                .collect()
        };
        image::save_buffer(&path, &pixels, width, height, image::ColorType::Rgba8).unwrap();
    }
    #[test]
    fn lane_leads_through_the_colonnade_to_the_pool() {
        let scene = geometry(0x564f5859, Ageing::default()).unwrap();
        let world = &scene.collision;
        // Walk from the end of the street along the centre line through the gate.
        let (mut feet, grounded) = world
            .move_capsule([0., 0.5, 80.], 1.8, 0.3, [0., -2., 0.])
            .unwrap();
        assert!(grounded);
        for _ in 0..1400 {
            feet = world
                .move_capsule(feet, 1.8, 0.3, [0., -0.004, 0.025])
                .unwrap()
                .0;
            if feet[2] > 111.0 {
                break;
            }
        }
        assert!(
            feet[2] > 111.0 && feet[1] > 0.1 && feet[1] < 0.3,
            "stopped at {feet:?}"
        );
        // Stepping off the coping into the deep end lands on its floor below the water.
        let (feet, grounded) = world
            .move_capsule([0., 0.6, 124.], 1.8, 0.3, [0., -4., 0.])
            .unwrap();
        assert!(grounded && (feet[1] + 1.9).abs() < 0.05, "{feet:?}");
        let (min, max) = scene.water[0].bounds();
        assert!(max.y < 0.2 && min.y < -1.8 && max.z - min.z > 13.);
        let world = voxy_render::voxy_water::WaterWorld::new(
            scene.water.clone(),
            voxy_render::WaveSpectrum::pool(),
        );
        assert!(
            world.is_submerged(Vec3::new(0., -1., 124.), 0.)
                && !world.is_submerged(Vec3::new(0., 1., 124.), 0.)
        );
    }
}
