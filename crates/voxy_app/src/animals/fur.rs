use super::*;
use physics::strand::{SphereCollider, Strand, StrandConfig};
use std::sync::{Arc, OnceLock};

pub const GUIDES: usize = 48;
pub const BONES: usize = 10;
pub const PALETTE: usize = BONES + GUIDES * 2;
#[derive(Clone, Copy, Debug)]
pub struct Root {
    pub position: Vec3,
    pub normal: Vec3,
    pub bone: usize,
    pub length: f32,
}
pub struct Groom {
    pub roots: Vec<Root>,
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
}
static DOG: OnceLock<Result<Arc<Groom>, String>> = OnceLock::new();
static CAT: OnceLock<Result<Arc<Groom>, String>> = OnceLock::new();
fn random(state: &mut u64) -> f32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*state >> 40) as u32) as f32 / 16777216.0
}
pub fn groom(cat: bool) -> Result<Arc<Groom>, String> {
    (if cat { &CAT } else { &DOG })
        .get_or_init(|| build(cat).map(Arc::new))
        .clone()
}
fn build(cat: bool) -> Result<Groom, String> {
    let surface = asset::surface(cat)?;
    let mut cdf = Vec::new();
    let mut total = 0.0;
    for face in surface.indices.chunks_exact(3) {
        if !surface.fur[face[0] as usize] {
            continue;
        }
        let p = face.map_indices(surface);
        let area = (p[1] - p[0]).cross(p[2] - p[0]).length() * 0.5;
        if area > 1e-12 {
            total += area;
            cdf.push((total, [face[0], face[1], face[2]]));
        }
    }
    if total <= 0.0 {
        return Err("animal has no groomable surface".into());
    }
    let mut seed = if cat { 7641 } else { 5239 };
    let mut samples = Vec::new();
    for _ in 0..30000 {
        let sample = random(&mut seed) * total;
        let index = cdf
            .partition_point(|(sum, _)| *sum < sample)
            .min(cdf.len() - 1);
        let face = cdf[index].1;
        let u = random(&mut seed).sqrt();
        let v = random(&mut seed);
        let weights = [1.0 - u, u * (1.0 - v), u * v];
        let mut p = Vec3::ZERO;
        let mut n = Vec3::ZERO;
        let mut color = Vec3::ZERO;
        for k in 0..3 {
            let vertex = &surface.vertices[face[k] as usize];
            p += Vec3::from_array(vertex.position) * weights[k];
            n += Vec3::from_array(vertex.normal) * weights[k];
            let packed = vertex.uv[0] as u32;
            color += Vec3::new(
                (packed & 255) as f32,
                ((packed >> 8) & 255) as f32,
                ((packed >> 16) & 255) as f32,
            ) / 255.0
                * weights[k];
        }
        let (j, w) = asset::skin(p, cat);
        let dominant = (0..4).max_by_key(|&k| w[k]).unwrap();
        let face = if cat {
            p.z > 0.24 && p.y > 0.34
        } else {
            p.z > 0.44 && p.y > 0.62
        };
        let limb = p.y < if cat { 0.16 } else { 0.24 };
        let length = if face {
            0.0020
        } else if limb {
            0.0030
        } else if cat {
            0.008
        } else {
            0.009
        } * (0.7 + random(&mut seed) * 0.6);
        samples.push((
            Root {
                position: p,
                normal: n.normalize(),
                bone: j[dominant] as usize,
                length,
            },
            color,
        ));
    }
    // Farthest-point sampling covers the whole surface rather than concentrating
    // guide dynamics on the largest torso triangles.
    let mut roots = vec![samples[0].0];
    let mut distances = vec![f32::INFINITY; samples.len()];
    while roots.len() < GUIDES {
        let last = roots.last().unwrap();
        for (distance, (sample, _)) in distances.iter_mut().zip(&samples) {
            let d = sample.position.distance_squared(last.position)
                + if sample.bone == last.bone { 0.0 } else { 0.15 };
            *distance = distance.min(d);
        }
        let next = distances
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        roots.push(samples[next].0);
    }
    let mut result = Groom {
        roots,
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    for (root, color) in samples {
        let guide = result
            .roots
            .iter()
            .enumerate()
            .filter(|(_, g)| g.bone == root.bone)
            .min_by(|a, b| {
                let score = |guide: &Root| {
                    guide.position.distance_squared(root.position)
                        + 0.02 * (guide.length / root.length).ln().powi(2)
                        + 0.005 * (1.0 - guide.normal.dot(root.normal))
                };
                score(a.1).total_cmp(&score(b.1))
            })
            .map(|(i, _)| i);
        let tangent = root
            .normal
            .cross(if root.normal.y.abs() > 0.9 {
                Vec3::X
            } else {
                Vec3::Y
            })
            .normalize();
        // Groom with the coat, from spine toward the belly and hindquarters.
        let comb = (Vec3::new(0.0, -0.35, -0.6)
            - root.normal * root.normal.dot(Vec3::new(0.0, -0.35, -0.6)))
        .normalize_or_zero();
        let direction = (root.normal * 0.40 + comb * 0.92).normalize();
        let start = result.vertices.len() as u32;
        for segment in 0..=2 {
            let t = segment as f32 * 0.5;
            let center = root.position + direction * root.length * t;
            let width = 0.00020 * (1.0 - t * 0.9);
            let joint = if segment == 0 {
                root.bone
            } else {
                guide.map_or(root.bone, |g| BONES + g * 2 + segment - 1)
            };
            for side in [-1.0, 1.0] {
                let mut vertex = super::super::actor_vertex(
                    (center + tangent * width * side).to_array(),
                    root.normal.to_array(),
                    asset::encoded_color(color * (0.72 + t * 0.35), 0.86),
                    joint as u16,
                );
                if segment > 0 {
                    if let Some(guide) = guide {
                        let influence = ((root.length / result.roots[guide].length).min(1.0)
                            * 65535.0)
                            .round() as u16;
                        vertex.joints = [joint as u16, root.bone as u16, 0, 0];
                        vertex.weights = [influence, 65535 - influence, 0, 0];
                    }
                }
                result.vertices.push(vertex);
            }
        }
        for s in 0..2 {
            let a = start + s * 2;
            result.indices.extend([
                a,
                a + 1,
                a + 2,
                a + 1,
                a + 3,
                a + 2,
                a + 2,
                a + 1,
                a,
                a + 2,
                a + 3,
                a + 1,
            ]);
        }
    }
    Ok(result)
}
trait FacePositions {
    fn map_indices(&self, surface: &asset::Surface) -> [Vec3; 3];
}
impl FacePositions for [u32] {
    fn map_indices(&self, surface: &asset::Surface) -> [Vec3; 3] {
        std::array::from_fn(|k| Vec3::from_array(surface.vertices[self[k] as usize].position))
    }
}
#[derive(Clone, Debug)]
pub struct CoatDynamics {
    pub strands: Vec<Strand>,
    initialized: bool,
}
impl CoatDynamics {
    pub fn new() -> Self {
        Self {
            strands: Vec::new(),
            initialized: false,
        }
    }
    pub fn step(&mut self, groom: &Groom, pose: &[Mat4; BONES], dt: f32) -> Result<(), String> {
        if !dt.is_finite() || dt <= 0.0 || dt > 1.0 / 30.0 {
            return Err("invalid fur timestep".into());
        }
        if !self.initialized {
            self.strands = groom
                .roots
                .iter()
                .map(|root| {
                    let matrix = pose[root.bone];
                    Strand::new_oriented(
                        (0..=2)
                            .map(|i| {
                                matrix
                                    .transform_point3(
                                        root.position
                                            + root.normal * root.length * (i as f32 / 2.0),
                                    )
                                    .as_dvec3()
                                    .to_array()
                            })
                            .collect(),
                        StrandConfig {
                            stiffness: 900.0,
                            damping: 0.0001,
                            iterations: 8,
                            radius: 0.0003,
                        },
                        rest_frame(matrix),
                    )
                })
                .collect::<Result<_, _>>()
                .map_err(str::to_string)?;
            self.initialized = true;
        }
        for (strand, root) in self.strands.iter_mut().zip(&groom.roots) {
            let matrix = pose[root.bone];
            let normal = matrix.transform_vector3(root.normal).normalize();
            let position = matrix.transform_point3(root.position);
            // Local osculating sphere prevents penetration into the attached skin.
            let collider = SphereCollider {
                center: (position - normal * 0.025).as_dvec3().to_array(),
                radius: 0.0245,
            };
            let frame = rest_frame(matrix);
            strand
                .step_oriented(
                    f64::from(dt),
                    position.as_dvec3().to_array(),
                    [0.6, -9.81, 0.2],
                    &[collider],
                    frame,
                )
                .map_err(str::to_string)?;
        }
        Ok(())
    }
    pub fn matrices(&self, groom: &Groom, pose: &[Mat4; BONES]) -> Vec<Mat4> {
        let mut matrices = pose.to_vec();
        for (index, root) in groom.roots.iter().enumerate() {
            let bone = pose[root.bone];
            for segment in 1..=2 {
                let rest = bone.transform_point3(
                    root.position + root.normal * root.length * (segment as f32 / 2.0),
                );
                let delta = self.strands.get(index).map_or(Vec3::ZERO, |s| {
                    Vec3::from_array(s.positions()[segment].map(|v| v as f32)) - rest
                });
                matrices.push(Mat4::from_translation(delta) * bone);
            }
        }
        matrices
    }
}

fn rest_frame(matrix: Mat4) -> [[f64; 3]; 3] {
    let x = matrix.x_axis.truncate().as_dvec3().normalize();
    let z = x.cross(matrix.y_axis.truncate().as_dvec3()).normalize();
    let y = z.cross(x);
    [x.to_array(), y.to_array(), z.to_array()]
}
