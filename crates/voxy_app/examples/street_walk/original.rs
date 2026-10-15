//! Independently authored deterministic geometry; CC0 surface scans attach separately.
use super::scene::StreetScene;
use glam::{Mat4, Quat, Vec3};
use physics::static_triangle_world::StaticTriangleWorld;
use voxy_render::archive_scene::*;
struct Builder {
    materials: Vec<ArchiveMaterialSource>,
    meshes: Vec<ArchiveMeshSource>,
    triangles: Vec<[[f64; 3]; 3]>,
    rng: u64,
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
        let e = 0.42;
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
        b.block(id, [self.side * (7.56 + p[0]), self.base + p[1], self.z + p[2]], s);
    }
}
fn bookcase(b: &mut Builder, r: &Room, u: f32, books: &[usize], wood: usize) {
    for du in [-0.6, 0.6] {
        r.put(b, wood, [6.55, 0.95, u + du], [0.36, 1.9, 0.04]);
    }
    r.put(b, wood, [6.72, 0.95, u], [0.02, 1.9, 1.2]);
    for shelf in 0..5 {
        let sy = 0.02 + shelf as f32 * 0.46;
        r.put(b, wood, [6.55, sy, u], [0.36, 0.03, 1.16]);
        if shelf == 4 {
            continue;
        }
        let mut bu = u - 0.55;
        let end = u + 0.5 - b.random() * 0.4;
        while bu < end {
            let w = 0.025 + b.random() * 0.05;
            let h = 0.22 + b.random() * 0.17;
            let id = books[(b.random() * books.len() as f32) as usize];
            let inset = b.random() * 0.04;
            r.put(b, id, [6.6 - inset, sy + 0.015 + h * 0.5, bu + w * 0.5], [0.2, h, w]);
            bu += w + 0.004;
        }
    }
}
pub fn generate(seed: u64) -> Result<StreetScene, Box<dyn std::error::Error>> {
    let mut scene = geometry(seed)?;
    super::materials::apply(&mut scene)?;
    Ok(scene)
}
fn geometry(seed: u64) -> Result<StreetScene, Box<dyn std::error::Error>> {
    let mut b = Builder {
        materials: Vec::new(),
        meshes: Vec::new(),
        triangles: Vec::new(),
        rng: seed,
        lights: Vec::new(),
    };
    let stone_m = b.material([0.32, 0.29, 0.24], 0.82, 0., 1., 0., 0.);
    let curb_m = b.material([0.48, 0.46, 0.41], 0.76, 0., 1., 0., 0.);
    let dark_m = b.material([0.035, 0.045, 0.052], 0.35, 0.85, 5., 0., 0.);
    let wood_m = b.material([0.19, 0.075, 0.025], 0.65, 0., 4., 0., 0.);
    let roof_m = b.material([0.085, 0.12, 0.15], 0.7, 0.2, 3., 0., 0.);
    let trim_m = b.material([0.7, 0.66, 0.54], 0.68, 0., 1., 0., 0.);
    let glass_m = b.material([0.9, 0.97, 1.], 0.08, 0., 0., 0., 0.92);
    let warm_m = b.material([1., 0.57, 0.2], 0.4, 0., 0., 4200., 0.);
    let leaf_m = b.material([0.045, 0.125, 0.018], 0.72, 0., 6., 0., 0.);
    let stone = b.bevel_cube(stone_m);
    let curb = b.bevel_cube(curb_m);
    let metal = b.cube(dark_m);
    let timber = b.cube(wood_m);
    let roof = b.cube(roof_m);
    let trim = b.bevel_cube(trim_m);
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
    let cloth_m = b.material([0.04, 0.18, 0.23], 0.9, 0., 4., 0., 0.);
    let cloth = b.cube(cloth_m);
    let plaster_m = b.material([0.82, 0.79, 0.72], 0.92, 0., 1., 0., 0.);
    let plaster = b.cube(plaster_m);
    let floor_m = b.material([0.36, 0.22, 0.12], 0.6, 0., 4., 0., 0.);
    let floor = b.cube(floor_m);
    let rug_m = b.material([0.32, 0.08, 0.06], 0.95, 0., 4., 0., 0.);
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
    let woods: Vec<usize> = [[0.42, 0.27, 0.14], [0.2, 0.1, 0.05], [0.58, 0.45, 0.3], [0.82, 0.8, 0.76]]
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
        let m = b.material(c, 0.95, 0., 4., 0., 0.);
        b.cube(m)
    })
    .collect();
    let books: Vec<usize> = [[0.5, 0.1, 0.08], [0.1, 0.25, 0.4], [0.75, 0.6, 0.2], [0.15, 0.35, 0.2], [0.85, 0.82, 0.75]]
        .into_iter()
        .map(|c| {
            let m = b.material(c, 0.7, 0., 0., 0., 0.);
            b.cube(m)
        })
        .collect();
    let ceramic_m = b.material([0.9, 0.9, 0.87], 0.3, 0., 0., 0., 0.);
    let ceramic = b.cube(ceramic_m);
    let foliage = b.cube(leaf_m);
    // The individual paving stones and kerbs are actual geometry and collision faces.
    b.block(stone, [0., -0.065, 27.], [28., 0.2, 112.]);
    for row in 0..360 {
        let z = -27. + row as f32 * 0.3;
        for col in 0..30 {
            let x = -4.35 + col as f32 * 0.3 + if row % 2 == 0 { 0.07 } else { 0. };
            let height = 0.075 + b.random() * 0.009;
            let jitter_x = (b.random() - 0.5) * 0.003;
            let jitter_z = (b.random() - 0.5) * 0.003;
            let yaw = (b.random() - 0.5) * 0.025;
            let width = 0.281 + b.random() * 0.008;
            let length = 0.278 + b.random() * 0.009;
            b.put(
                stone,
                Vec3::new(x + jitter_x, height * 0.5 - 0.03, z + jitter_z),
                Vec3::new(width, height, length),
                Quat::from_rotation_y(yaw),
            );
        }
    }
    for side in [-1., 1.] {
        b.block(curb, [side * 6.68, 0.07, 27.], [4.36, 0.2, 112.]);
        for i in 0..180 {
            b.block(
                trim,
                [side * 4.55, 0.12, -27. + i as f32 * 0.62],
                [0.22, 0.25, 0.6],
            );
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
            let door_bay = windows / 2;
            let door_z = z - 3.75 + spacing * (door_bay as f32 + 0.5);
            let facade_pattern = if house % 4 == 1 { 7. } else { 2. };
            let colour = palette[(b.random() * palette.len() as f32) as usize];
            let tint = 0.78 + b.random() * 0.34;
            let house_material =
                b.material(colour.map(|v| v * tint), 0.88, 0., facade_pattern, 0., 0.);
            let wall = b.cube(house_material);
            b.block(wall, [side * 14.45, height * 0.5, z], [0.2, height, 7.85]);
            for dz in [-3.925, 3.925] {
                b.block(wall, [side * 11., height * 0.5, z + dz], [7., height, 0.2]);
            }
            // Openings are cut cleanly from the facade: piers between bays, a full
            // spandrel below every sill and above every head, so nothing behind the
            // glass is a backing slab and no slit opens between band and trim.
            let top = floors as f32 * 2.65;
            for pier in 0..=windows {
                let edge = pier == 0 || pier == windows;
                let width = if edge {
                    (spacing - opening) * 0.5
                } else {
                    spacing - opening
                };
                let dz = -3.75
                    + pier as f32 * spacing
                    + if pier == 0 {
                        width * 0.5
                    } else if pier == windows {
                        -width * 0.5
                    } else {
                        0.
                    };
                b.block(wall, [side * 7.42, top * 0.5, z + dz], [0.28, top, width]);
            }
            for level in 0..floors {
                let f = level as f32 * 2.65;
                if level == 0 && house % 3 != 0 {
                    // Keep the entrance clear down to the pavement.
                    for (lo, hi) in [
                        (z - 3.925, door_z - opening * 0.5),
                        (door_z + opening * 0.5, z + 3.925),
                    ] {
                        b.block(
                            wall,
                            [side * 7.42, 0.45, (lo + hi) * 0.5],
                            [0.28, 0.9, hi - lo],
                        );
                    }
                } else {
                    b.block(wall, [side * 7.42, f + 0.45, z], [0.28, 0.9, 7.85]);
                }
                b.block(wall, [side * 7.42, f + 2.475, z], [0.28, 0.35, 7.85]);
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
            if house < 3 {
                let pipe_z = z + 3.55;
                b.block(
                    metal,
                    [side * 7.04, height * 0.5, pipe_z],
                    [0.085, height, 0.085],
                );
                for bracket in 0..5 {
                    b.block(
                        metal,
                        [side * 7.12, 0.6 + bracket as f32 * 2.4, pipe_z],
                        [0.24, 0.035, 0.15],
                    );
                }
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
                        let door_width = opening - 0.16;
                        b.block(timber, [side * 7.44, 1.15, wz], [0.07, 2.3, door_width]);
                        for dz in [-1., 1.] {
                            b.block(
                                timber,
                                [side * 7.44, 1.15, wz + dz * (opening - 0.08) * 0.5],
                                [0.12, 2.3, 0.08],
                            );
                            b.block(
                                trim,
                                [side * 7.24, 1.2, wz + dz * (opening * 0.5 + 0.07)],
                                [0.1, 2.4, 0.14],
                            );
                        }
                        b.block(trim, [side * 7.24, 2.39, wz], [0.12, 0.18, opening + 0.32]);
                        for py in [0.6, 1.5] {
                            b.block(
                                timber,
                                [side * 7.39, py, wz],
                                [0.04, 0.7, door_width - 0.24],
                            );
                        }
                        b.block(
                            metal,
                            [side * 7.36, 1.05, wz + door_width * 0.36],
                            [0.07, 0.12, 0.05],
                        );
                        b.block(trim, [side * 7.2, 0.06, wz], [0.5, 0.12, opening + 0.2]);
                        continue;
                    }
                    // Casement frame sits 0.19 m back from the facade face.
                    let fx = side * 7.47;
                    for dz in [-1., 1.] {
                        b.block(
                            timber,
                            [fx, y, wz + dz * (opening * 0.5 - 0.037)],
                            [0.09, h - 0.004, 0.07],
                        );
                    }
                    for dy in [-1., 1.] {
                        b.block(
                            timber,
                            [fx, y + dy * (h * 0.5 - 0.037), wz],
                            [0.086, 0.07, opening - 0.004],
                        );
                    }
                    b.block(timber, [fx, y, wz], [0.07, h - 0.1, 0.05]);
                    b.block(timber, [fx, y1 - 0.45, wz], [0.07, 0.05, opening - 0.1]);
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
                    }
                }
            }
            // Furnished rooms behind every floor. Each room picks a type (living,
            // kitchen, bedroom, study), a wall paint, a wood and a fabric, so the
            // street reads as many different households. All parts are collidable.
            let ix = |d: f32| side * (7.56 + d);
            for level in 0..=floors {
                let f = level as f32 * 2.65;
                let floor_id = woods[(b.random() * woods.len() as f32) as usize];
                if level == 0 {
                    b.block(floor_id, [ix(3.395), 0.03, z], [6.79, 0.06, 7.65]);
                } else {
                    b.block(floor_id, [ix(3.395), f - 0.04, z], [6.79, 0.2, 7.65]);
                }
                if level == floors {
                    continue;
                }
                let base = if level == 0 { 0.06 } else { f + 0.06 };
                let r = Room {
                    side,
                    z,
                    base,
                    paint: paints[(b.random() * paints.len() as f32) as usize],
                    wood: woods[(b.random() * woods.len() as f32) as usize],
                    fabric: fabrics[(b.random() * fabrics.len() as f32) as usize],
                    accent: fabrics[(b.random() * fabrics.len() as f32) as usize],
                };
                r.put(&mut b, r.paint, [6.76, 1.25, 0.], [0.04, 2.5, 7.6]);
                for dz in [-1., 1.] {
                    r.put(&mut b, r.paint, [3.39, 1.25, dz * 3.8], [6.74, 2.5, 0.04]);
                    r.put(&mut b, trim, [3.38, 0.05, dz * 3.768], [6.7, 0.1, 0.02]);
                    r.put(&mut b, trim, [3.38, 2.47, dz * 3.765], [6.7, 0.06, 0.03]);
                }
                r.put(&mut b, trim, [6.725, 0.05, 0.], [0.025, 0.1, 7.5]);
                r.put(&mut b, trim, [6.722, 2.47, 0.], [0.03, 0.06, 7.5]);
                // Curtains just inside the facade at every window of this floor.
                for window in 0..windows {
                    let wu = -3.75 + spacing * (window as f32 + 0.5);
                    if level == 0 && house % 3 != 0 && window == door_bay {
                        continue;
                    }
                    let open = b.random();
                    r.put(&mut b, metal, [0.12, 2.3, wu], [0.02, 0.02, opening + 0.6]);
                    for cs in [-1., 1.] {
                        let w = 0.2 + open * (opening * 0.45);
                        let u = wu + cs * (opening * 0.5 + 0.3 - w * 0.5);
                        for fold in 0..4 {
                            let fw = w / 4.;
                            r.put(
                                &mut b,
                                r.accent,
                                [0.1 + (fold % 2) as f32 * 0.03, 1.2, u - w * 0.5 + fw * (fold as f32 + 0.5)],
                                [0.02, 2.2, fw * 0.98],
                            );
                        }
                    }
                }
                let kind = (b.random() * 4.) as usize;
                let flip = if b.random() < 0.5 { -1. } else { 1. };
                match kind {
                    // Living room: sofa, armchair, coffee table, media unit, rug.
                    0 => {
                        let su = flip * 2.6;
                        r.put(&mut b, r.accent, [3.4, 0.006, flip * 1.4], [2.6, 0.012, 2.0]);
                        r.put(&mut b, r.fabric, [3.4, 0.22, su], [2.1, 0.32, 0.9]);
                        r.put(&mut b, r.fabric, [3.4, 0.62, su + flip * 0.33], [2.1, 0.5, 0.22]);
                        for a in [-1., 1.] {
                            r.put(&mut b, r.fabric, [3.4 + a * 1.0, 0.48, su], [0.2, 0.3, 0.9]);
                        }
                        for c in [-0.45, 0.45] {
                            r.put(&mut b, r.fabric, [3.4 + c, 0.43, su - flip * 0.05], [0.84, 0.1, 0.62]);
                        }
                        r.put(&mut b, r.accent, [3.0, 0.62, su + flip * 0.12], [0.4, 0.34, 0.12]);
                        r.put(&mut b, r.wood, [3.4, 0.4, flip * 1.4], [1.1, 0.05, 0.6]);
                        for (dx, du) in [(-0.5, -0.25), (-0.5, 0.25), (0.5, -0.25), (0.5, 0.25)] {
                            r.put(&mut b, metal, [3.4 + dx, 0.19, flip * 1.4 + du], [0.03, 0.38, 0.03]);
                        }
                        r.put(&mut b, ceramic, [3.3, 0.48, flip * 1.4], [0.12, 0.12, 0.12]);
                        // Media unit on the opposite wall with a screen above it.
                        r.put(&mut b, r.wood, [3.4, 0.25, -flip * 3.55], [1.8, 0.5, 0.4]);
                        r.put(&mut b, metal, [3.4, 1.15, -flip * 3.74], [1.2, 0.7, 0.04]);
                        // Armchair near the window.
                        r.put(&mut b, r.accent, [1.3, 0.24, flip * 0.6], [0.8, 0.36, 0.8]);
                        r.put(&mut b, r.accent, [1.66, 0.62, flip * 0.6], [0.14, 0.5, 0.8]);
                        // Floor lamp.
                        r.put(&mut b, metal, [1.2, 0.01, flip * 1.3], [0.28, 0.02, 0.28]);
                        r.put(&mut b, metal, [1.2, 0.8, flip * 1.3], [0.025, 1.6, 0.025]);
                        let shade = if b.random() < 0.6 { glow } else { ceramic };
                        r.put(&mut b, shade, [1.2, 1.66, flip * 1.3], [0.34, 0.24, 0.34]);
                    }
                    // Kitchen: base and wall cabinets, sink, stove, fridge, dining set.
                    1 => {
                        let wu = flip * 3.47;
                        r.put(&mut b, r.wood, [4.2, 0.43, wu], [4.0, 0.86, 0.6]);
                        r.put(&mut b, ceramic, [4.2, 0.885, wu], [4.04, 0.05, 0.64]);
                        for door in 0..7 {
                            let d = 2.43 + door as f32 * 0.59;
                            r.put(&mut b, metal, [d, 0.7, wu - flip * 0.302], [0.12, 0.02, 0.006]);
                            r.put(&mut b, r.wood, [d, 1.85, flip * 3.6], [0.56, 0.7, 0.34]);
                        }
                        r.put(&mut b, r.accent, [4.2, 1.2, flip * 3.775], [4.0, 0.6, 0.006]);
                        r.put(&mut b, metal, [3.0, 0.9, wu], [0.5, 0.03, 0.4]);
                        r.put(&mut b, metal, [3.0, 1.05, wu + flip * 0.2], [0.03, 0.3, 0.03]);
                        r.put(&mut b, metal, [5.2, 0.92, wu], [0.6, 0.02, 0.55]);
                        r.put(&mut b, ceramic, [6.45, 0.95, wu], [0.6, 1.9, 0.62]);
                        r.put(&mut b, metal, [6.12, 1.2, wu - flip * 0.15], [0.04, 0.5, 0.03]);
                        let tu = -flip * 1.4;
                        r.put(&mut b, r.wood, [3.6, 0.75, tu], [1.6, 0.05, 0.9]);
                        for (dx, du) in [(-0.7, -0.38), (-0.7, 0.38), (0.7, -0.38), (0.7, 0.38)] {
                            r.put(&mut b, r.wood, [3.6 + dx, 0.36, tu + du], [0.05, 0.72, 0.05]);
                        }
                        for (cx, cu) in [(3.15, -0.62), (4.05, -0.62), (3.15, 0.62), (4.05, 0.62)] {
                            let back = if cu < 0. { -1. } else { 1. };
                            for (dx, du) in [(-0.17, -0.17), (-0.17, 0.17), (0.17, -0.17), (0.17, 0.17)] {
                                r.put(&mut b, r.wood, [cx + dx, 0.22, tu + cu + du], [0.035, 0.44, 0.035]);
                            }
                            r.put(&mut b, r.wood, [cx, 0.46, tu + cu], [0.4, 0.04, 0.4]);
                            r.put(&mut b, r.wood, [cx, 0.72, tu + cu + back * 0.18], [0.4, 0.48, 0.035]);
                        }
                        r.put(&mut b, ceramic, [3.4, 0.8, tu], [0.26, 0.04, 0.26]);
                        r.put(&mut b, ceramic, [3.8, 0.8, tu + 0.2], [0.26, 0.04, 0.26]);
                    }
                    // Bedroom: bed with bedding, nightstands, wardrobe, rug.
                    2 => {
                        let bu = flip * 1.6;
                        r.put(&mut b, r.wood, [5.6, 0.2, bu], [2.1, 0.3, 1.6]);
                        r.put(&mut b, r.wood, [6.68, 0.55, bu], [0.06, 1.0, 1.7]);
                        r.put(&mut b, ceramic, [5.6, 0.45, bu], [2.0, 0.2, 1.5]);
                        r.put(&mut b, r.fabric, [5.2, 0.57, bu], [1.4, 0.06, 1.56]);
                        for pu in [-0.38, 0.38] {
                            r.put(&mut b, ceramic, [6.38, 0.6, bu + pu], [0.36, 0.12, 0.6]);
                        }
                        for nu in [-1.08, 1.08] {
                            r.put(&mut b, r.wood, [6.45, 0.27, bu + nu], [0.42, 0.54, 0.45]);
                            r.put(&mut b, metal, [6.45, 0.62, bu + nu], [0.05, 0.16, 0.05]);
                            let shade = if b.random() < 0.5 { glow } else { ceramic };
                            r.put(&mut b, shade, [6.45, 0.76, bu + nu], [0.2, 0.14, 0.2]);
                        }
                        r.put(&mut b, r.accent, [4.4, 0.006, bu], [1.2, 0.012, 2.4]);
                        r.put(&mut b, r.wood, [5.2, 1.0, -flip * 3.48], [1.6, 2.0, 0.58]);
                        r.put(&mut b, metal, [5.2, 1.1, -flip * 3.186], [0.02, 0.4, 0.02]);
                    }
                    // Study: desk with monitor, office chair, bookcases, armchair.
                    _ => {
                        let du = flip * 3.4;
                        r.put(&mut b, r.wood, [2.2, 0.74, du], [1.4, 0.04, 0.7]);
                        for dx in [-0.66, 0.66] {
                            r.put(&mut b, r.wood, [2.2 + dx, 0.36, du], [0.04, 0.72, 0.66]);
                        }
                        r.put(&mut b, metal, [2.2, 1.0, du + flip * 0.2], [0.6, 0.36, 0.03]);
                        r.put(&mut b, metal, [2.2, 0.8, du + flip * 0.22], [0.06, 0.08, 0.06]);
                        r.put(&mut b, metal, [2.2, 0.765, du - flip * 0.08], [0.45, 0.012, 0.15]);
                        r.put(&mut b, glow, [2.7, 0.9, du + flip * 0.15], [0.12, 0.06, 0.12]);
                        r.put(&mut b, metal, [2.2, 0.2, du - flip * 0.65], [0.06, 0.4, 0.06]);
                        r.put(&mut b, r.fabric, [2.2, 0.45, du - flip * 0.65], [0.48, 0.08, 0.48]);
                        r.put(&mut b, r.fabric, [2.2, 0.78, du - flip * 0.88], [0.46, 0.55, 0.06]);
                        for spoke in 0..5 {
                            let a = spoke as f32 * std::f32::consts::TAU / 5.;
                            r.put(&mut b, metal, [2.2 + a.cos() * 0.18, 0.03, du - flip * 0.65 + a.sin() * 0.18], [0.05, 0.04, 0.05]);
                        }
                        for bu in [-1.4, 0.0] {
                            bookcase(&mut b, &r, flip * bu, &books, r.wood);
                        }
                        r.put(&mut b, r.accent, [4.0, 0.24, -flip * 2.6], [0.8, 0.36, 0.8]);
                        r.put(&mut b, r.accent, [4.0, 0.62, -flip * 2.96], [0.8, 0.5, 0.14]);
                    }
                }
                // Shared touches: framed pictures, a plant, a ceiling light.
                let pictures = 1 + (b.random() * 3.) as usize;
                for n in 0..pictures {
                    let pu = (n as f32 - (pictures - 1) as f32 * 0.5) * 0.75 - flip * 0.6;
                    let (pw, ph) = (0.4 + b.random() * 0.3, 0.3 + b.random() * 0.35);
                    r.put(&mut b, r.wood, [6.725, 1.6, pu], [0.02, ph + 0.06, pw + 0.06]);
                    let canvas = books[(b.random() * books.len() as f32) as usize];
                    r.put(&mut b, canvas, [6.712, 1.6, pu], [0.01, ph, pw]);
                }
                if b.random() < 0.7 {
                    let pu = flip * (2.8 + b.random() * 0.7) * if kind == 1 { -1. } else { 1. };
                    let pd = 0.6 + b.random() * 0.4;
                    r.put(&mut b, ceramic, [pd, 0.17, pu], [0.3, 0.34, 0.3]);
                    for leaf_n in 0..7 {
                        let a = leaf_n as f32 * 0.9;
                        b.put(
                            foliage,
                            Vec3::new(ix(pd + a.cos() * 0.12), base + 0.55 + (leaf_n % 3) as f32 * 0.15, z + pu + a.sin() * 0.12),
                            Vec3::new(0.08, 0.4, 0.22),
                            Quat::from_rotation_y(a) * Quat::from_rotation_x(0.35),
                        );
                    }
                }
                let ceiling = 2.65 - 0.2;
                let lit = b.random() < 0.55;
                if b.random() < 0.5 {
                    r.put(&mut b, metal, [3.4, ceiling - 0.25, 0.], [0.012, 0.5, 0.012]);
                    r.put(&mut b, if lit { glow } else { metal }, [3.4, ceiling - 0.56, 0.], [0.36, 0.14, 0.36]);
                } else {
                    r.put(&mut b, if lit { glow } else { ceramic }, [3.4, ceiling - 0.02, 0.], [0.5, 0.04, 0.5]);
                }
            }
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
            let tx = side * (5.65 + b.random() * 0.45);
            let tz = z + 4. + b.random() * 2.;
            let tree_height = 4.5 + b.random() * 2.3;
            let crown_radius = 1.05 + b.random() * 0.65;
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
                let origin = fork + Vec3::Y * (elevation - 0.3) * 0.8;
                let elbow = origin
                    + Vec3::new(
                        a.cos() * radial * 0.50,
                        0.45 + elevation * 0.6,
                        a.sin() * radial * 0.50,
                    );
                let tip = Vec3::new(
                    tx + a.cos() * radial,
                    tree_height * (0.72 + 0.25 * elevation),
                    tz + a.sin() * radial,
                );
                b.beam(trunk, origin, elbow, 0.085);
                b.beam(trunk, elbow, tip, 0.050);
                for twig in 0..8 {
                    let theta = a + twig as f32 * 2.399963;
                    let t = 0.20 + twig as f32 * 0.10;
                    let start = elbow.lerp(tip, t);
                    let end = start
                        + Vec3::new(
                            theta.cos() * 0.55,
                            0.12 + b.random() * 0.38,
                            theta.sin() * 0.55,
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
                            b.random() * 6.283185,
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
    b.block(trim, [0., 2.5, 88.], [32., 5., 0.6]);
    for pillar in -7..=7 {
        b.block(trim, [pillar as f32 * 2.2, 2.7, 87.5], [0.32, 5.4, 0.8]);
    }
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
        bounds: (Vec3::new(-18., -0.3, -29.), Vec3::new(18., 18., 110.)),
        collision,
        eye: Vec3::new(0.6, 1.78, -18.),
        direction: Vec3::new(-0.10, 0.025, 1.).normalize(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_street_is_independent_and_walkable() {
        let scene = geometry(0x564f5859).unwrap();
        assert!(scene.archive.is_none() && scene.images.is_empty());
        assert!(scene.materials.len() >= 34 && scene.lights.len() == 16);
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
        assert_eq!(
            wall_batches.len(),
            24,
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
}
