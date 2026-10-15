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
        let length = self.scale.clamp(3.5, 6.0) * (0.017 + self.rng.next() * 0.012);
        let forward = direction.normalize();
        let side = forward
            .cross(Vec3::new(self.rng.signed(), 1., self.rng.signed()))
            .normalize();
        let normal = side.cross(forward).normalize();
        let start = self.leaves.positions.len() as u32;
        for row in 0..=6 {
            let t = row as f32 / 6.;
            let width = (std::f32::consts::PI * t).sin().max(0.0).powf(0.8) * length * 0.30;
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
            5
        } else if depth == 0 {
            16
        } else {
            8
        };
        let sides = if self.world_detail {
            8
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
            self.wood.indices.extend([a, a + 1, tip]);
        }
        if depth < 3 {
            let count = if self.world_detail {
                if depth == 0 { 10 } else { 4 }
            } else if depth == 0 {
                12
            } else {
                5
            };
            let phase = self.rng.next() * TAU;
            for i in 0..count {
                let t = 0.25 + 0.68 * (i as f32 + 0.3 * self.rng.next()) / count as f32;
                let angle = phase + i as f32 * 2.399963;
                let outward = side * angle.cos() + other * angle.sin();
                let dir = (forward * 0.18 + outward * 0.95 + Vec3::Y * 0.30).normalize();
                let size = length
                    * (if depth == 0 { 0.56 } else { 0.46 })
                    * (0.75 + 0.4 * self.rng.next())
                    * (1. - 0.3 * t);
                self.branch(at(t), dir, size, radius * (1. - 0.65 * t) * 0.38, depth + 1);
            }
        }
        if depth >= 2 {
            let count = if self.world_detail { 10 } else { 36 };
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
fn texture(leaf: bool, normal: bool, seed: u64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    const SIZE: u32 = 512;
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let height = |x: u32, y: u32| {
        let u = x as f32 / SIZE as f32;
        let v = y as f32 / SIZE as f32;
        if leaf {
            let mid = (u - 0.5).abs();
            let vein = ((v * 15. + mid * 8.).fract() - 0.5).abs();
            (-(mid * 120.).powi(2)).exp() * 0.7
                + (-(vein * 60.).powi(2)).exp() * 0.25
                + noise(x, y, seed) * 0.05
        } else {
            let ridge = (u * TAU * 23. + (v * TAU * 3.).sin() * 0.9).sin() * 0.5 + 0.5;
            ridge.powi(3) * 0.65 + noise(x / 3, y / 11, seed) * 0.2 + noise(x, y, seed) * 0.15
        }
    };
    for y in 0..SIZE {
        for x in 0..SIZE {
            let h = height(x, y);
            let rgba = if normal {
                let dx = height((x + 1) % SIZE, y) - height((x + SIZE - 1) % SIZE, y);
                let dy = height(x, (y + 1) % SIZE) - height(x, (y + SIZE - 1) % SIZE);
                let n = Vec3::new(-dx * 2., -dy * 2., 1.).normalize();
                [
                    ((n.x * 0.5 + 0.5) * 255.) as u8,
                    ((n.y * 0.5 + 0.5) * 255.) as u8,
                    ((n.z * 0.5 + 0.5) * 255.) as u8,
                    255,
                ]
            } else if leaf {
                let fleck = noise(x, y, seed ^ 91);
                [
                    (58. + h * 23. + fleck * 14.) as u8,
                    (104. + h * 40. + fleck * 22.) as u8,
                    (25. + h * 15.) as u8,
                    255,
                ]
            } else {
                [
                    (67. + h * 54.) as u8,
                    (48. + h * 43.) as u8,
                    (31. + h * 31.) as u8,
                    255,
                ]
            };
            pixels.extend(rgba);
        }
    }
    let mut output = Cursor::new(Vec::new());
    image::RgbaImage::from_raw(SIZE, SIZE, pixels)
        .ok_or("invalid plant texture")?
        .write_to(&mut output, image::ImageFormat::Png)?;
    Ok(output.into_inner())
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
fn generate(
    options: PlantOptions,
    full_material: bool,
    world_detail: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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
    g.wood.normals();
    g.leaves.normals();
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
    for (leaf, normal) in [(false, false), (false, true), (true, false), (true, true)] {
        let png = texture(leaf, normal, options.seed)?;
        let view = append(&mut binary, &mut views, &png);
        images.push(json!({"bufferView":view,"mimeType":"image/png"}));
    }
    let mut root = json!({"asset":{"version":"2.0","generator":"Voxy native botanical generator v1"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0,"name":format!("{:?} seed {}",options.kind,options.seed)}],"meshes":[{"primitives":primitives}],"buffers":[{"byteLength":binary.len()}],"bufferViews":views,"accessors":accessors,"images":images,"samplers":[{"magFilter":9729,"minFilter":9987,"wrapS":10497,"wrapT":10497}],"textures":[{"source":0,"sampler":0},{"source":1,"sampler":0},{"source":2,"sampler":0},{"source":3,"sampler":0}],"materials":[{"name":"Bark","pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicFactor":0.,"roughnessFactor":0.94},"normalTexture":{"index":1,"scale":0.8}},{"name":"Leaves","doubleSided":true,"pbrMetallicRoughness":{"baseColorTexture":{"index":2},"metallicFactor":0.,"roughnessFactor":0.48},"normalTexture":{"index":3,"scale":0.35}}]});
    if !full_material {
        for material in root["materials"].as_array_mut().expect("material array") {
            material
                .as_object_mut()
                .expect("material object")
                .remove("normalTexture");
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
            assert_eq!(gltf.document.images().count(), 4);
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
