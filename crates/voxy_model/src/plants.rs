//! Seeded botanical surfaces and self-contained glTF assets, in metres.
use glam::Vec3;
use serde_json::json;
use std::f32::consts::TAU;
use std::io::Cursor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlantKind {
    Tree,
    Shrub,
}
#[derive(Clone, Copy, Debug)]
pub struct PlantOptions {
    pub kind: PlantKind,
    pub seed: u64,
    pub height: f32,
}
impl Default for PlantOptions {
    fn default() -> Self {
        Self {
            kind: PlantKind::Tree,
            seed: 42,
            height: 7.0,
        }
    }
}
#[derive(Debug, Default)]
struct Surface {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    indices: Vec<u32>,
}
#[derive(Debug)]
struct Random(u64);
impl Random {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((x ^ (x >> 31)) >> 40) as f32 / 16_777_216.0
    }
    fn signed(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
}
impl Surface {
    fn vertex(&mut self, p: Vec3, uv: [f32; 2]) -> u32 {
        let id = self.positions.len() as u32;
        self.positions.push(p.to_array());
        self.uv.push(uv);
        self.normals.push([0.; 3]);
        id
    }
    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        for triangle in [[a, b, c], [a, c, d]] {
            let p = |i: u32| Vec3::from_array(self.positions[i as usize]);
            if (p(triangle[1]) - p(triangle[0]))
                .cross(p(triangle[2]) - p(triangle[0]))
                .length_squared()
                > 1e-20
            {
                self.indices.extend(triangle);
            }
        }
    }
    fn tangents(&self) -> Vec<[f32; 4]> {
        let mut t = vec![Vec3::ZERO; self.positions.len()];
        let mut b = t.clone();
        for ids in self.indices.chunks_exact(3) {
            let [a, c, d] = [ids[0] as usize, ids[1] as usize, ids[2] as usize];
            let e1 = Vec3::from_array(self.positions[c]) - Vec3::from_array(self.positions[a]);
            let e2 = Vec3::from_array(self.positions[d]) - Vec3::from_array(self.positions[a]);
            let uv1 = glam::Vec2::from_array(self.uv[c]) - glam::Vec2::from_array(self.uv[a]);
            let uv2 = glam::Vec2::from_array(self.uv[d]) - glam::Vec2::from_array(self.uv[a]);
            let det = uv1.x * uv2.y - uv1.y * uv2.x;
            if det.abs() > 1e-12 {
                let tangent = (e1 * uv2.y - e2 * uv1.y) / det;
                let bitangent = (e2 * uv1.x - e1 * uv2.x) / det;
                for i in [a, c, d] {
                    t[i] += tangent;
                    b[i] += bitangent;
                }
            }
        }
        self.normals
            .iter()
            .enumerate()
            .map(|(i, normal)| {
                let n = Vec3::from_array(*normal);
                let tangent = (t[i] - n * n.dot(t[i])).normalize_or(n.any_orthonormal_vector());
                let sign = if n.cross(tangent).dot(b[i]) < 0. {
                    -1.
                } else {
                    1.
                };
                [tangent.x, tangent.y, tangent.z, sign]
            })
            .collect()
    }
    fn normals(&mut self) {
        for t in self.indices.chunks_exact(3) {
            let p = |i: u32| Vec3::from_array(self.positions[i as usize]);
            let n = (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0]));
            for &i in t {
                self.normals[i as usize] =
                    (Vec3::from_array(self.normals[i as usize]) + n).to_array();
            }
        }
        for n in &mut self.normals {
            *n = Vec3::from_array(*n).normalize_or(Vec3::Y).to_array();
        }
    }
}
struct Growth {
    world_detail: bool,
    wood: Surface,
    leaves: Surface,
    rng: Random,
    scale: f32,
}
impl Growth {
    fn leaf(&mut self, base: Vec3, direction: Vec3) {
        let length = self.scale.clamp(3.5, 6.0) * (0.023 + self.rng.next() * 0.015);
        let forward = direction.normalize();
        let side = forward
            .cross(Vec3::new(self.rng.signed(), 1., self.rng.signed()))
            .normalize();
        let normal = side.cross(forward).normalize();
        let start = self.leaves.positions.len() as u32;
        for row in 0..=6 {
            let t = row as f32 / 6.;
            let width = (std::f32::consts::PI * t).sin().max(0.0).powf(0.75) * length * 0.36;
            let centre = base + forward * (length * t) + normal * (length * 0.14 * t * t);
            for col in 0..=2 {
                let s = col as f32 - 1.;
                let p = centre + side * (width * s) + normal * (width * 0.15 * (1. - s.abs()));
                self.leaves.vertex(p, [col as f32 / 2., t]);
            }
        }
        for row in 0..6 {
            for col in 0..2 {
                let a = start + row * 3 + col;
                self.leaves.quad(a, a + 3, a + 4, a + 1);
            }
        }
    }
    fn branch(&mut self, base: Vec3, direction: Vec3, length: f32, radius: f32, depth: u32) {
        let forward = direction.normalize();
        let side = forward
            .cross(if forward.y.abs() > 0.95 {
                Vec3::X
            } else {
                Vec3::Y
            })
            .normalize();
        let other = forward.cross(side);
        let bend =
            side * self.rng.signed() * length * 0.13 + other * self.rng.signed() * length * 0.09;
        let at = |t: f32| {
            base + forward * (length * t) + bend * t * t + Vec3::Y * (length * 0.12 * t * t)
        };
        let start = self.wood.positions.len() as u32;
        let segments = if self.world_detail {
            if depth == 0 { 12 } else { 6 }
        } else if depth == 0 {
            16
        } else {
            8
        };
        let sides = if self.world_detail {
            if depth == 0 { 16 } else { 8 }
        } else if depth == 0 {
            24
        } else {
            10
        };
        for ring in 0..=segments {
            let t = ring as f32 / segments as f32;
            let r = radius * (1. - t * 0.87);
            for col in 0..=sides {
                let angle = col as f32 / sides as f32 * TAU;
                let ridges =
                    1. + 0.06 * (angle * 7. + t * 5.).sin() + 0.025 * (angle * 13. - t * 9.).sin();
                let root = if depth == 0 {
                    1. + 0.5 * (1. - t).powi(8)
                } else {
                    1.
                };
                self.wood.vertex(
                    at(t) + (side * angle.cos() + other * angle.sin()) * r * ridges * root,
                    [col as f32 / sides as f32, t * length / (radius * TAU)],
                );
            }
        }
        for ring in 0..segments {
            for col in 0..sides {
                let a = start + ring * (sides + 1) + col;
                self.wood.quad(a, a + 1, a + sides + 2, a + sides + 1);
            }
        }
        // Close the tip; basal ring stays embedded in its parent or the ground.
        let tip = self.wood.vertex(at(1.), [0.5, 1.]);
        for col in 0..sides {
            let a = start + segments * (sides + 1) + col;
            let p = |i: u32| Vec3::from_array(self.wood.positions[i as usize]);
            if (p(a + 1) - p(a)).cross(p(tip) - p(a)).length_squared() > 1e-20 {
                self.wood.indices.extend([a, a + 1, tip]);
            }
        }
        if depth < 4 {
            let count = if self.world_detail {
                if depth == 0 {
                    8
                } else if depth == 3 {
                    2
                } else if depth == 2 {
                    3
                } else {
                    4
                }
            } else if depth == 0 {
                12
            } else if depth == 3 {
                2
            } else {
                5
            };
            let phase = self.rng.next() * TAU;
            for i in 0..count {
                let t = (if depth == 0 { 0.37 } else { 0.22 })
                    + (if depth == 0 { 0.57 } else { 0.70 }) * (i as f32 + 0.4 * self.rng.next())
                        / count as f32;
                let angle = phase + i as f32 * 2.399963;
                let outward = side * angle.cos() + other * angle.sin();
                let dir =
                    (forward * 0.22 + outward * 0.95 + Vec3::Y * (0.20 + self.rng.next() * 0.28))
                        .normalize();
                let size = length
                    * (if depth == 0 { 0.74 } else { 0.48 })
                    * (0.75 + 0.4 * self.rng.next())
                    * (if depth == 0 {
                        (1. - t).sqrt() + 0.25
                    } else {
                        1. - 0.22 * t
                    });
                self.branch(at(t), dir, size, radius * (1. - 0.65 * t) * 0.38, depth + 1);
            }
        }
        if depth >= 3 {
            let count = if self.world_detail { 30 } else { 36 };
            for i in 0..count {
                let t = 0.18 + 0.8 * i as f32 / count as f32;
                let angle = i as f32 * 2.399963 + self.rng.next() * 0.35;
                let dir = (side * angle.cos() + other * angle.sin() + forward * 0.5).normalize();
                self.leaf(at(t), dir);
            }
        }
    }
}
fn noise(x: u32, y: u32, seed: u64) -> f32 {
    let mut r = Random(
        seed ^ u64::from(x).wrapping_mul(0x517cc1b7) ^ u64::from(y).wrapping_mul(0x9e3779b9),
    );
    r.next()
}
fn tile_noise(u: f32, v: f32, nx: u32, ny: u32, seed: u64) -> f32 {
    let x = u * nx as f32;
    let y = v * ny as f32;
    let ix = x.floor() as u32;
    let iy = y.floor() as u32;
    let sx = x.fract();
    let sy = y.fract();
    let sx = sx * sx * (3. - 2. * sx);
    let sy = sy * sy * (3. - 2. * sy);
    let a = noise(ix % nx, iy % ny, seed);
    let b = noise((ix + 1) % nx, iy % ny, seed);
    let c = noise(ix % nx, (iy + 1) % ny, seed);
    let d = noise((ix + 1) % nx, (iy + 1) % ny, seed);
    (a + (b - a) * sx) * (1. - sy) + (c + (d - c) * sx) * sy
}
fn texture_set(leaf: bool, seed: u64) -> Result<[image::RgbaImage; 3], Box<dyn std::error::Error>> {
    const N: u32 = 512;
    let mut heights = Vec::with_capacity((N * N) as usize);
    let mut colours = Vec::with_capacity((N * N * 4) as usize);
    let mut surface = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let u = x as f32 / N as f32;
            let v = y as f32 / N as f32;
            let macro_n = tile_noise(u, v, 7, 5, seed);
            let fine = tile_noise(u, v, 87, 53, seed ^ 17);
            let (h, rgb, rough) = if leaf {
                let mid = (u - 0.5).abs();
                let rib = (-mid * mid * 18000.).exp();
                let vein_distance =
                    ((v * 9. + mid * 3.2 + tile_noise(u, v, 4, 4, seed ^ 3) * 0.08).fract() - 0.5)
                        .abs();
                let veins = (-vein_distance * vein_distance * 12000.).exp() * (1. - mid * 1.6);
                let h = rib * 0.65 + veins * 0.22 + fine * 0.025;
                let tone = 0.78 + macro_n * 0.27 + fine * 0.07;
                (
                    h,
                    [
                        ((48. + rib * 16. + veins * 10.) * tone) as u8,
                        ((99. + rib * 16. + veins * 13.) * tone) as u8,
                        ((22. + rib * 6.) * tone) as u8,
                    ],
                    0.38 + macro_n * 0.18 + veins * 0.08,
                )
            } else {
                let warp = tile_noise(u, v, 9, 5, seed ^ 7) * 0.34;
                let fissure = ((u * 27. + warp).fract() - 0.5).abs();
                let plate = (fissure * 7.).min(1.);
                let cross = tile_noise(u, v, 27, 19, seed ^ 31);
                let h = plate * (0.52 + macro_n * 0.25) + fine * 0.10 + cross * 0.12;
                let shade = 0.42 + plate * 0.48 + macro_n * 0.22;
                (
                    h,
                    [
                        (108. * shade + fine * 12.) as u8,
                        (88. * shade + fine * 9.) as u8,
                        (65. * shade + fine * 7.) as u8,
                    ],
                    0.77 + fine * 0.15 + (1. - plate) * 0.07,
                )
            };
            heights.push(h);
            colours.extend([rgb[0], rgb[1], rgb[2], 255]);
            surface.extend([255, (rough.clamp(0.05, 1.) * 255.) as u8, 0, 255]);
        }
    }
    let sample = |x: u32, y: u32| heights[((y % N) * N + x % N) as usize];
    let mut normals = Vec::with_capacity(colours.len());
    for y in 0..N {
        for x in 0..N {
            let dx = sample(x + 1, y) - sample(x + N - 1, y);
            let dy = sample(x, y + 1) - sample(x, y + N - 1);
            let n = Vec3::new(-dx * 3., -dy * 3., 1.).normalize();
            normals.extend([
                ((n.x * 0.5 + 0.5) * 255.) as u8,
                ((n.y * 0.5 + 0.5) * 255.) as u8,
                ((n.z * 0.5 + 0.5) * 255.) as u8,
                255,
            ]);
        }
    }
    Ok([
        image::RgbaImage::from_raw(N, N, colours).ok_or("colour image")?,
        image::RgbaImage::from_raw(N, N, normals).ok_or("normal image")?,
        image::RgbaImage::from_raw(N, N, surface).ok_or("roughness image")?,
    ])
}
fn plant_images(seed: u64) -> Result<Vec<image::RgbaImage>, Box<dyn std::error::Error>> {
    let [bark, bark_normal, bark_mr] = texture_set(false, seed)?;
    let [leaf, leaf_normal, leaf_mr] = texture_set(true, seed ^ 0x1eaf)?;
    Ok(vec![bark, bark_normal, leaf, leaf_normal, bark_mr, leaf_mr])
}
/// Native render-ready plant data; no intermediate image encoding, filesystem or import pass.
#[derive(Debug)]
pub struct PlantMeshAsset {
    pub geometry: Vec<voxy_render::archive_scene::ArchiveMeshSource>,
    pub materials: Vec<voxy_render::archive_scene::ArchiveMaterialSource>,
    pub images: Vec<voxy_render::ImageAsset>,
}
/// Generate instanced-world geometry and material pixels directly.
/// # Errors
/// Rejects invalid dimensions and image/geometry limits.
pub fn generate_world_plant_asset(
    options: PlantOptions,
) -> Result<PlantMeshAsset, Box<dyn std::error::Error>> {
    use voxy_render::archive_scene::*;
    let g = grow(options, true)?;
    // Ground dressing stays within the root's supporting voxel footprint.
    let mut grass = Surface::default();
    let mut rng = Random(options.seed ^ 0x6a72_6173_73);
    for _ in 0..180 {
        let angle = rng.next() * TAU;
        let radial = rng.next().sqrt() * 0.35;
        let base = Vec3::new(angle.cos() * radial, 0.004, angle.sin() * radial);
        let bend_angle = rng.next() * TAU;
        let side = Vec3::new(bend_angle.cos(), 0., bend_angle.sin());
        let bend = Vec3::new(-side.z, 0., side.x) * (0.025 + rng.next() * 0.06);
        let height = 0.07 + rng.next() * 0.15;
        let width = 0.003 + rng.next() * 0.004;
        let start = grass.positions.len() as u32;
        for row in 0..=3 {
            let t = row as f32 / 3.;
            let centre = base + Vec3::Y * height * t + bend * t * t;
            for col in 0..=1 {
                grass.vertex(
                    centre + side * ((col as f32 * 2. - 1.) * width * (1. - t)),
                    [col as f32, t],
                );
            }
        }
        for row in 0..3 {
            let a = start + row * 2;
            grass.quad(a, a + 2, a + 3, a + 1);
        }
    }
    grass.normals();
    let mut geometry = Vec::new();
    let mut materials = Vec::new();
    for (id, surface) in [g.wood, g.leaves, grass].into_iter().enumerate() {
        let tangents = surface.tangents();
        let vertices = (0..surface.positions.len())
            .map(|i| ArchiveVertex {
                position: surface.positions[i],
                normal: surface.normals[i],
                tangent: tangents[i],
                uv: surface.uv[i],
            })
            .collect();
        geometry.push(ArchiveMeshSource {
            vertices,
            indices: surface.indices,
            instances: vec![ArchiveInstance::new(glam::Mat4::IDENTITY)],
            material: id,
            center: Vec3::ZERO,
        });
        materials.push(ArchiveMaterialSource {
            parameters: ArchiveMaterial {
                diffuse: if id == 2 {
                    [0.72, 0.9, 0.56, 1.]
                } else {
                    [1.; 4]
                },
                specular_gloss: [1.; 4],
                emissive_normal: [0., 0., 0., if id == 0 { 0.8 } else { 0.35 }],
                surface: [0., 1., 0., -1.],
                workflow: [1., 1., 0., if id > 0 { 0.65 } else { 0. }],
            },
            images: [
                Some(id.min(1) * 2),
                Some(4 + id.min(1)),
                Some(id.min(1) * 2 + 1),
                None,
            ],
            double_sided: id > 0,
        });
    }
    let images = plant_images(options.seed)?
        .into_iter()
        .map(|p| {
            voxy_render::ImageAsset::from_rgba(
                p.width(),
                p.height(),
                p.into_raw(),
                Default::default(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PlantMeshAsset {
        geometry,
        materials,
        images,
    })
}

/// Generate a bounded, deterministic GLB with separate bark/leaf PBR materials,
/// normal maps, metric geometry and double-sided leaf surfaces.
/// # Errors
/// Rejects nonfinite or unsupported heights, and image/serialization failures.
pub fn generate_plant_glb(options: PlantOptions) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    generate(options, true, false)
}

/// Simplified asset for Voxy's current model editor, which rejects normal-map materials.
/// The full PBR asset should be retained separately.
/// # Errors
/// Same input and encoding errors as `generate_plant_glb`.
pub fn generate_plant_editor_glb(
    options: PlantOptions,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    generate(options, false, false)
}
/// Bounded world instance detail, with the same materials and botanical growth rules.
/// # Errors
/// Same validation and encoding errors as `generate_plant_glb`.
pub fn generate_world_plant_glb(
    options: PlantOptions,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    generate(options, true, true)
}
fn grow(options: PlantOptions, world_detail: bool) -> Result<Growth, Box<dyn std::error::Error>> {
    if !options.height.is_finite() || !(0.25..=30.).contains(&options.height) {
        return Err("plant height must be 0.25..=30 metres".into());
    }
    let mut g = Growth {
        world_detail,
        wood: Surface::default(),
        leaves: Surface::default(),
        rng: Random(options.seed),
        scale: options.height,
    };
    match options.kind {
        PlantKind::Tree => g.branch(
            Vec3::ZERO,
            Vec3::Y,
            options.height,
            options.height * 0.027,
            0,
        ),
        PlantKind::Shrub => {
            for i in 0..7 {
                let a = i as f32 * 2.399963;
                let length = options.height * (0.7 + g.rng.next() * 0.3);
                g.branch(
                    Vec3::ZERO,
                    Vec3::new(a.cos() * 0.6, 1., a.sin() * 0.6),
                    length,
                    options.height * 0.018,
                    1,
                );
            }
        }
    }
    if options.kind == PlantKind::Tree {
        for root in 0..6 {
            let angle = root as f32 * TAU / 6. + g.rng.signed() * 0.15;
            let radial = Vec3::new(angle.cos(), 0., angle.sin());
            let side = Vec3::new(-angle.sin(), 0., angle.cos());
            let start = g.wood.positions.len() as u32;
            for ring in 0..=6 {
                let t = ring as f32 / 6.;
                let centre = radial * (options.height * 0.085 * t)
                    + Vec3::Y * (options.height * 0.055 * (1. - t).powi(2) - 0.02);
                let radius = options.height * 0.018 * (1. - t).powi(2) + 0.002;
                for j in 0..=8 {
                    let a = j as f32 * TAU / 8.;
                    g.wood.vertex(
                        centre + (side * a.cos() + Vec3::Y * a.sin()) * radius,
                        [j as f32 / 8., t * 2.],
                    );
                }
            }
            for ring in 0..6 {
                for j in 0..8 {
                    let a = start + ring * 9 + j;
                    g.wood.quad(a, a + 1, a + 10, a + 9);
                }
            }
        }
    }
    g.wood.normals();
    g.leaves.normals();
    Ok(g)
}
fn generate(
    options: PlantOptions,
    full_material: bool,
    world_detail: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let g = grow(options, world_detail)?;
    let mut binary = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut primitives = Vec::new();
    fn append(binary: &mut Vec<u8>, views: &mut Vec<serde_json::Value>, bytes: &[u8]) -> usize {
        while binary.len() % 4 != 0 {
            binary.push(0);
        }
        let index = views.len();
        views.push(json!({"buffer":0,"byteOffset":binary.len(),"byteLength":bytes.len()}));
        binary.extend_from_slice(bytes);
        index
    }
    for (material, surface) in [g.wood, g.leaves].into_iter().enumerate() {
        let mut attrs = serde_json::Map::new();
        for (name, width, values) in [
            (
                "POSITION",
                3,
                surface
                    .positions
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>(),
            ),
            (
                "NORMAL",
                3,
                surface.normals.iter().flatten().copied().collect(),
            ),
            (
                "TANGENT",
                4,
                surface.tangents().iter().flatten().copied().collect(),
            ),
            (
                "TEXCOORD_0",
                2,
                surface.uv.iter().flatten().copied().collect(),
            ),
        ] {
            let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
            let view = append(&mut binary, &mut views, &bytes);
            let mut accessor = json!({"bufferView":view,"componentType":5126,"count":values.len()/width,"type":match width { 4=>"VEC4",3=>"VEC3",_=>"VEC2" }});
            if name == "POSITION" {
                let min = surface
                    .positions
                    .iter()
                    .fold(Vec3::splat(f32::INFINITY), |a, p| {
                        a.min(Vec3::from_array(*p))
                    });
                let max = surface
                    .positions
                    .iter()
                    .fold(Vec3::splat(f32::NEG_INFINITY), |a, p| {
                        a.max(Vec3::from_array(*p))
                    });
                accessor["min"] = json!(min.to_array());
                accessor["max"] = json!(max.to_array());
            }
            attrs.insert(name.into(), json!(accessors.len()));
            accessors.push(accessor);
        }
        let bytes: Vec<u8> = surface
            .indices
            .iter()
            .flat_map(|i| i.to_le_bytes())
            .collect();
        let view = append(&mut binary, &mut views, &bytes);
        let indices = accessors.len();
        accessors.push(json!({"bufferView":view,"componentType":5125,"count":surface.indices.len(),"type":"SCALAR"}));
        primitives.push(json!({"attributes":attrs,"indices":indices,"material":material}));
    }
    let mut images = Vec::new();
    for pixels in plant_images(options.seed)? {
        let mut output = Cursor::new(Vec::new());
        pixels.write_to(&mut output, image::ImageFormat::Png)?;
        let view = append(&mut binary, &mut views, &output.into_inner());
        images.push(json!({"bufferView":view,"mimeType":"image/png"}));
    }
    let mut root = json!({"asset":{"version":"2.0","generator":"Voxy native botanical generator v2"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0,"name":format!("{:?} seed {}",options.kind,options.seed)}],"meshes":[{"primitives":primitives}],"buffers":[{"byteLength":binary.len()}],"bufferViews":views,"accessors":accessors,"images":images,"samplers":[{"magFilter":9729,"minFilter":9987,"wrapS":10497,"wrapT":10497}],"textures":[{"source":0,"sampler":0},{"source":1,"sampler":0},{"source":2,"sampler":0},{"source":3,"sampler":0},{"source":4,"sampler":0},{"source":5,"sampler":0}],"materials":[{"name":"Bark","pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicFactor":0.,"roughnessFactor":1.,"metallicRoughnessTexture":{"index":4}},"normalTexture":{"index":1,"scale":0.8}},{"name":"Leaves","doubleSided":true,"pbrMetallicRoughness":{"baseColorTexture":{"index":2},"metallicFactor":0.,"roughnessFactor":1.,"metallicRoughnessTexture":{"index":5}},"normalTexture":{"index":3,"scale":0.35}}]});
    if !full_material {
        for material in root["materials"].as_array_mut().expect("material array") {
            material
                .as_object_mut()
                .expect("material object")
                .remove("normalTexture");
            material["pbrMetallicRoughness"]
                .as_object_mut()
                .expect("PBR material")
                .remove("metallicRoughnessTexture");
        }
    }
    let mut json = serde_json::to_vec(&root)?;
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    while binary.len() % 4 != 0 {
        binary.push(0);
    }
    let mut output = Vec::new();
    for n in [
        0x46546c67,
        2,
        (12 + 8 + json.len() + 8 + binary.len()) as u32,
        json.len() as u32,
        0x4e4f534a,
    ] {
        output.extend(n.to_le_bytes());
    }
    output.extend(json);
    output.extend((binary.len() as u32).to_le_bytes());
    output.extend(0x004e4942u32.to_le_bytes());
    output.extend(binary);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn botanical_assets_round_trip_with_textures_and_finite_geometry() {
        for kind in [PlantKind::Tree, PlantKind::Shrub] {
            let options = PlantOptions {
                kind,
                seed: 7,
                height: 3.,
            };
            let bytes = generate_plant_glb(options).unwrap();
            assert_eq!(bytes, generate_plant_glb(options).unwrap());
            let gltf = gltf::Gltf::from_slice(&bytes).unwrap();
            assert_eq!(gltf.document.materials().count(), 2);
            assert_eq!(gltf.document.images().count(), 6);
            let container = gltf::binary::Glb::from_slice(&bytes).unwrap();
            let archive = crate::SceneArchive::decode(
                &container.json,
                vec![container.bin.unwrap().into_owned().into()],
                Default::default(),
            )
            .unwrap();
            for primitive in &archive.meshes[0] {
                assert!(primitive.positions.iter().flatten().all(|v| v.is_finite()));
                assert!(
                    primitive
                        .normals
                        .as_ref()
                        .unwrap()
                        .iter()
                        .all(|n| (Vec3::from_array(*n).length() - 1.).abs() < 1e-4)
                );
                for ids in primitive.indices.chunks_exact(3) {
                    let p = |i: u32| Vec3::from_array(primitive.positions[i as usize]);
                    assert!(
                        (p(ids[1]) - p(ids[0]))
                            .cross(p(ids[2]) - p(ids[0]))
                            .length_squared()
                            > 1e-20
                    );
                }
            }

            let asset = crate::ModelAsset::parse(
                &generate_plant_editor_glb(options).unwrap(),
                &[],
                crate::ModelLimits::default(),
            )
            .unwrap();
            let meshes = asset.scene_meshes(&asset.skeleton.bind_pose()).unwrap();
            assert_eq!(meshes.len(), 2);
            assert!(meshes.iter().all(|m| m.vertices().len() > 1000));
            assert_ne!(
                bytes,
                generate_plant_glb(PlantOptions { seed: 8, ..options }).unwrap()
            );
        }
    }
    #[test]
    fn native_world_asset_has_real_surface_maps_and_grounded_grass() {
        let asset = generate_world_plant_asset(PlantOptions {
            height: 5.,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(asset.geometry.len(), 3);
        assert_eq!(asset.materials.len(), 3);
        assert_eq!(asset.images.len(), 6);
        for image in &asset.images[4..6] {
            let mut low = 255;
            let mut high = 0;
            for p in image.rgba().chunks_exact(4) {
                assert_eq!(p[2], 0);
                low = low.min(p[1]);
                high = high.max(p[1]);
            }
            assert!(high - low > 20, "roughness must vary across the surface");
        }
        for mesh in &asset.geometry {
            assert!(
                mesh.vertices.len() < 200_000,
                "world detail must remain bounded"
            );
            for v in &mesh.vertices {
                let n = Vec3::from_array(v.normal);
                let t = Vec3::from_array([v.tangent[0], v.tangent[1], v.tangent[2]]);
                assert!(Vec3::from_array(v.position).is_finite());
                assert!((n.length() - 1.).abs() < 1e-4 && (t.length() - 1.).abs() < 1e-4);
                assert!(n.dot(t).abs() < 1e-3);
            }
        }
        for v in asset.geometry[2].vertices.iter().filter(|v| v.uv[1] == 0.) {
            assert!((v.position[1] - 0.004).abs() < 1e-5);
            assert!((v.position[0] * v.position[0] + v.position[2] * v.position[2]).sqrt() < 0.36);
        }
    }

    #[test]
    fn rejects_invalid_dimensions() {
        for height in [f32::NAN, f32::INFINITY, 0., 31.] {
            assert!(
                generate_plant_glb(PlantOptions {
                    height,
                    ..Default::default()
                })
                .is_err()
            );
        }
    }
}
